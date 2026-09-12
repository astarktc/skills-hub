use super::*;
use crate::core::{
    installer::install_local_skill,
    refresh::{
        refresh_managed_skills, RefreshPolicy, RefreshReport, RefreshSelection, SkillRefreshStatus,
    },
    skill_store::SkillTargetRecord,
    sync_status::{SyncMode, SyncStatus},
    unlocatable::repoint_and_update,
};
use std::fs;

const UPSTREAM: &str = "---\nname: alpha\n---\nBody\n";
struct Fixture {
    dir: tempfile::TempDir,
    paths: InstallerPaths,
    store: SkillStore,
    id: String,
    source: PathBuf,
    central: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let paths = InstallerPaths {
            home: dir.path().join("home"),
            central_dir: dir.path().join("central"),
            cache_dir: dir.path().join("cache"),
        };
        fs::create_dir_all(&paths.home).unwrap();
        let source = dir.path().join("source");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), UPSTREAM).unwrap();
        let store = SkillStore::new(dir.path().join("test.db"));
        store.ensure_schema().unwrap();
        let installed = install_local_skill(&paths, &store, &source, None).unwrap();
        Self {
            dir,
            paths,
            store,
            id: installed.skill_id,
            source,
            central: installed.central_path,
        }
    }
    fn set(&self, mode: Option<InvocationMode>) -> Result<ManagedSkillEntry> {
        set_invocation_override(&self.paths, &self.store, &self.id, mode)
            .map(|outcome| outcome.entry)
    }
    fn update(&self) -> RefreshReport {
        refresh_managed_skills(
            &self.paths,
            &self.store,
            RefreshSelection::Ids(vec![self.id.clone()]),
            RefreshPolicy::default(),
            None,
            5000,
            |_| {},
        )
        .unwrap()
    }
    fn text(&self) -> String {
        fs::read_to_string(self.central.join("SKILL.md")).unwrap()
    }
}

#[test]
fn edit_returns_a_failed_copy_target_in_its_propagation_report() {
    let f = Fixture::new();
    let adapter = crate::core::tool_adapters::adapter_by_key("cursor").unwrap();
    fs::create_dir_all(f.paths.home.join(adapter.relative_detect_dir)).unwrap();
    let blocker = f.dir.path().join("blocked");
    fs::write(&blocker, "not a directory").unwrap();
    f.store
        .upsert_skill_target(&SkillTargetRecord {
            id: "blocked-target".into(),
            skill_id: f.id.clone(),
            tool: "cursor".into(),
            target_path: blocker.join("skill").to_string_lossy().into_owned(),
            mode: SyncMode::Copy,
            status: SyncStatus::Synced,
            last_error: None,
            synced_at: Some(1),
        })
        .unwrap();
    let outcome =
        set_invocation_override(&f.paths, &f.store, &f.id, Some(InvocationMode::UserOnly)).unwrap();
    assert_eq!(
        outcome.entry.invocation_override.unwrap().mode,
        InvocationMode::UserOnly
    );
    assert_eq!(outcome.propagation.targets.len(), 1);
    assert!(matches!(
        outcome.propagation.targets[0].status,
        crate::core::propagation::PropagationStatus::Failed { .. }
    ));
    assert_eq!(
        f.store
            .get_skill_target(&f.id, "cursor")
            .unwrap()
            .unwrap()
            .status,
        SyncStatus::Error
    );
    assert_eq!(
        manifest::read_invocation_lines(&f.text()).mode(),
        InvocationMode::UserOnly
    );
}

#[test]
fn update_replays_and_clear_restores_current_upstream_bytes_and_hash() {
    let f = Fixture::new();
    f.set(Some(InvocationMode::UserOnly)).unwrap();
    assert!(matches!(
        f.update().skills[0].status,
        SkillRefreshStatus::Refreshed {
            edit_conflict: None,
            ..
        }
    ));
    assert_eq!(
        manifest::read_invocation_lines(&f.text()).mode(),
        InvocationMode::UserOnly
    );
    assert_ne!(
        directory_identity(&f.central).unwrap(),
        directory_identity(&f.source).unwrap()
    );
    let upstream = "---\nuser-invocable: false  \nname: alpha\n---\nnew body\n";
    fs::write(f.source.join("SKILL.md"), upstream).unwrap();
    let report = f.update();
    let SkillRefreshStatus::Refreshed {
        edit_conflict: Some(conflict),
        ..
    } = &report.skills[0].status
    else {
        panic!("{report:?}");
    };
    assert_eq!(conflict.base_mode, InvocationMode::UserAndModel);
    assert_eq!(conflict.upstream_mode, InvocationMode::ModelOnly);
    assert_eq!(conflict.override_mode, InvocationMode::UserOnly);
    let edit = f
        .store
        .get_skill_edit(&f.id, SkillEditKind::InvocationMode)
        .unwrap()
        .unwrap();
    assert!(edit.conflict);
    assert_eq!(
        edit.base_value,
        serde_json::to_string(&manifest::read_invocation_lines(upstream)).unwrap()
    );
    // Unchanged upstream still disagrees with the override.
    assert!(matches!(
        f.update().skills[0].status,
        SkillRefreshStatus::Refreshed {
            edit_conflict: None,
            ..
        }
    ));
    assert!(
        f.store
            .get_skill_edit(&f.id, SkillEditKind::InvocationMode)
            .unwrap()
            .unwrap()
            .conflict
    );
    let entry = f.set(None).unwrap();
    assert!(entry.invocation_override.is_none());
    assert_eq!(f.text(), upstream);
    assert_eq!(
        entry.skill.content_hash,
        Some(directory_identity(&f.source).unwrap())
    );
    assert!(f
        .store
        .get_skill_edit(&f.id, SkillEditKind::InvocationMode)
        .unwrap()
        .is_none());
}

#[test]
fn upstream_convergence_keeps_override_and_clears_conflict() {
    for previous_conflict in [false, true] {
        let f = Fixture::new();
        let user = manifest::write_invocation_mode(UPSTREAM, InvocationMode::UserOnly);
        fs::write(f.source.join("SKILL.md"), &user).unwrap();
        f.update();
        f.set(Some(InvocationMode::ModelOnly)).unwrap();
        if previous_conflict {
            fs::write(
                f.source.join("SKILL.md"),
                manifest::write_invocation_mode(UPSTREAM, InvocationMode::Neither),
            )
            .unwrap();
            assert!(matches!(
                f.update().skills[0].status,
                SkillRefreshStatus::Refreshed {
                    edit_conflict: Some(_),
                    ..
                }
            ));
            assert!(
                f.store
                    .get_skill_edit(&f.id, SkillEditKind::InvocationMode)
                    .unwrap()
                    .unwrap()
                    .conflict
            );
        }
        let model = manifest::write_invocation_mode(UPSTREAM, InvocationMode::ModelOnly);
        fs::write(f.source.join("SKILL.md"), &model).unwrap();
        assert!(matches!(
            f.update().skills[0].status,
            SkillRefreshStatus::Refreshed {
                edit_conflict: None,
                ..
            }
        ));
        let edit = f
            .store
            .get_skill_edit(&f.id, SkillEditKind::InvocationMode)
            .unwrap()
            .unwrap();
        assert!(!edit.conflict);
        assert_eq!(edit_mode(&edit).unwrap(), InvocationMode::ModelOnly);
        assert_eq!(f.text(), model);
    }
}

#[test]
fn rechoosing_rebases_and_repeated_choices_keep_the_upstream_base() {
    let f = Fixture::new();
    f.set(Some(InvocationMode::UserOnly)).unwrap();
    f.set(Some(InvocationMode::Neither)).unwrap();
    f.set(None).unwrap();
    assert_eq!(f.text(), UPSTREAM);
    f.set(Some(InvocationMode::UserOnly)).unwrap();
    fs::write(
        f.source.join("SKILL.md"),
        "---\nname: alpha\nuser-invocable: false\n---\n",
    )
    .unwrap();
    f.update();
    assert!(
        !f.set(Some(InvocationMode::UserOnly))
            .unwrap()
            .invocation_override
            .unwrap()
            .conflict
    );
    assert!(matches!(
        f.update().skills[0].status,
        SkillRefreshStatus::Refreshed {
            edit_conflict: None,
            ..
        }
    ));
}

#[test]
fn restore_and_repoint_replay_the_edit() {
    let f = Fixture::new();
    f.set(Some(InvocationMode::Neither)).unwrap();
    fs::remove_dir_all(&f.central).unwrap();
    assert!(matches!(
        f.set(Some(InvocationMode::UserOnly))
            .unwrap_err()
            .downcast_ref::<SignalError>(),
        Some(SignalError::CentralPathMissing { .. })
    ));
    assert!(matches!(
        f.update().skills[0].status,
        SkillRefreshStatus::Refreshed { .. }
    ));
    assert_eq!(
        manifest::read_invocation_lines(&f.text()).mode(),
        InvocationMode::Neither
    );
    let new = f.dir.path().join("new");
    fs::rename(&f.source, &new).unwrap();
    let report = repoint_and_update(
        &f.paths,
        &f.store,
        &f.id,
        &new,
        RefreshPolicy::default(),
        None,
        6000,
        |_| {},
    )
    .unwrap();
    assert!(matches!(
        report.skills[0].status,
        SkillRefreshStatus::Refreshed { .. }
    ));
    assert_eq!(
        manifest::read_invocation_lines(&f.text()).mode(),
        InvocationMode::Neither
    );
}

#[test]
fn imported_and_source_missing_skills_are_editable() {
    let f = Fixture::new();
    fs::remove_dir_all(&f.source).unwrap();
    f.set(Some(InvocationMode::UserOnly)).unwrap();
    let mut record = f.store.get_skill_by_id(&f.id).unwrap().unwrap();
    record.source_type = "imported".into();
    record.source_ref = None;
    f.store.upsert_skill(&record).unwrap();
    let entry = f.set(Some(InvocationMode::ModelOnly)).unwrap();
    assert!(!entry.invocation_override.unwrap().conflict);
    f.set(None).unwrap();
    assert_eq!(f.text(), UPSTREAM);
}

#[test]
fn set_propagates_to_copy_fallback_target() {
    let f = Fixture::new();
    let mut adapter = crate::core::tool_adapters::adapter_by_key("cursor")
        .unwrap()
        .clone();
    adapter.supports_symlink = false;
    let adapter = crate::core::tool_adapters::test_overrides::shadow(adapter);
    fs::create_dir_all(f.paths.home.join(adapter.relative_detect_dir)).unwrap();
    let target = f.paths.home.join(adapter.relative_skills_dir).join("alpha");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("SKILL.md"), UPSTREAM).unwrap();
    f.store
        .upsert_skill_target(&SkillTargetRecord {
            id: "target".into(),
            skill_id: f.id.clone(),
            tool: "cursor".into(),
            target_path: target.to_string_lossy().into_owned(),
            mode: SyncMode::Copy,
            status: SyncStatus::Synced,
            last_error: None,
            synced_at: None,
        })
        .unwrap();
    f.set(Some(InvocationMode::Neither)).unwrap();
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        f.text()
    );
    assert!(!fs::symlink_metadata(&target)
        .unwrap()
        .file_type()
        .is_symlink());
}

#[cfg(unix)]
#[test]
fn failed_manifest_write_keeps_base_and_replay_heals_bytes() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    let permissions = fs::metadata(&f.central).unwrap().permissions();
    fs::set_permissions(&f.central, fs::Permissions::from_mode(0o555)).unwrap();
    let result = f.set(Some(InvocationMode::UserOnly));
    fs::set_permissions(&f.central, permissions).unwrap();
    let error = result.unwrap_err();
    assert!(error.downcast_ref::<std::io::Error>().is_some());
    assert!(matches!(
        error.downcast_ref::<SignalError>(),
        Some(SignalError::SkillManifestIo { .. })
    ));
    assert_eq!(f.text(), UPSTREAM);
    assert_eq!(fs::read_dir(&f.central).unwrap().count(), 1);
    let edit = f
        .store
        .get_skill_edit(&f.id, SkillEditKind::InvocationMode)
        .unwrap()
        .unwrap();
    assert_eq!(
        edit.base_value,
        serde_json::to_string(&manifest::read_invocation_lines(UPSTREAM)).unwrap()
    );
    let mut record = f.store.get_skill_by_id(&f.id).unwrap().unwrap();
    mutation_guard::serialized(|| replay_unlocked(&f.store, &mut record)).unwrap();
    assert_eq!(
        manifest::read_invocation_lines(&f.text()).mode(),
        InvocationMode::UserOnly
    );
    f.set(None).unwrap();
    assert_eq!(f.text(), UPSTREAM);
}

#[test]
fn failed_edit_upserts_leave_set_and_replay_bytes_untouched() {
    let f = Fixture::new();
    // Persistent SQLite triggers affect the store's per-operation connections;
    // PRAGMA query_only on this connection would not.
    let conn = rusqlite::Connection::open(f.store.db_path()).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_edit BEFORE INSERT ON skill_edits BEGIN SELECT RAISE(FAIL, 'test edit upsert failure'); END;").unwrap();
    assert!(f.set(Some(InvocationMode::UserOnly)).is_err());
    assert_eq!(f.text(), UPSTREAM);
    assert!(f
        .store
        .get_skill_edit(&f.id, SkillEditKind::InvocationMode)
        .unwrap()
        .is_none());
    conn.execute_batch("DROP TRIGGER fail_edit;").unwrap();
    f.set(Some(InvocationMode::UserOnly)).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_edit BEFORE INSERT ON skill_edits BEGIN SELECT RAISE(FAIL, 'test edit upsert failure'); END;").unwrap();
    fs::write(f.central.join("SKILL.md"), UPSTREAM).unwrap();
    let mut record = f.store.get_skill_by_id(&f.id).unwrap().unwrap();
    assert!(mutation_guard::serialized(|| replay_unlocked(&f.store, &mut record)).is_err());
    assert_eq!(f.text(), UPSTREAM);
}

#[test]
fn failed_update_replay_restores_bytes_skill_and_edit_then_retry_succeeds() {
    for (legacy, fault) in [
        (false, "edit"),
        (false, "hash"),
        (false, "invalid_utf8"),
        (true, "edit"),
        (true, "hash"),
        (true, "invalid_utf8"),
    ] {
        let f = Fixture::new();
        let mode = if legacy {
            seed_legacy_edit(&f);
            InvocationMode::Neither
        } else {
            f.set(Some(InvocationMode::UserOnly)).unwrap();
            InvocationMode::UserOnly
        };
        let before = f.store.get_skill_by_id(&f.id).unwrap().unwrap();
        let edit = f
            .store
            .get_skill_edit(&f.id, SkillEditKind::InvocationMode)
            .unwrap()
            .unwrap();
        let old_bytes = f.text();
        let upstream = "---\nname: alpha\nuser-invocable: false\n---\nnew body\n";
        fs::write(f.source.join("SKILL.md"), upstream).unwrap();
        let conn = rusqlite::Connection::open(f.store.db_path()).unwrap();
        if fault == "edit" {
            conn.execute_batch("CREATE TRIGGER fail_replay BEFORE INSERT ON skill_edits
                WHEN NEW.base_value != (SELECT base_value FROM skill_edits WHERE skill_id = NEW.skill_id)
                BEGIN SELECT RAISE(ABORT, 'test replay edit failure'); END;").unwrap();
        } else if fault == "hash" {
            // Finalize's upstream hash is admitted; replay's post-Edit hash is
            // rejected, after the Edit row and manifest have both changed.
            let replayed = manifest::write_invocation_mode(upstream, mode);
            let expected = tempfile::tempdir().unwrap();
            fs::write(expected.path().join("SKILL.md"), replayed).unwrap();
            let hash = directory_identity(expected.path()).unwrap();
            conn.execute_batch(&format!(
                "CREATE TRIGGER fail_replay BEFORE INSERT ON skills
                WHEN NEW.content_hash = '{hash}'
                BEGIN SELECT RAISE(ABORT, 'test replay hash failure'); END;"
            ))
            .unwrap();
        } else {
            // Optional metadata reads degrade; required replay must fail typed
            // and restore the entire pre-finalize state, not land invalid bytes.
            fs::write(f.source.join("SKILL.md"), [0xff]).unwrap();
        }
        let report = f.update();
        assert!(
            matches!(report.skills[0].status, SkillRefreshStatus::Failed { .. }),
            "{report:?}"
        );
        assert_eq!(f.text(), old_bytes);
        assert_eq!(
            format!("{:?}", f.store.get_skill_by_id(&f.id).unwrap().unwrap()),
            format!("{before:?}")
        );
        assert_eq!(
            format!(
                "{:?}",
                f.store
                    .get_skill_edit(&f.id, SkillEditKind::InvocationMode)
                    .unwrap()
                    .unwrap()
            ),
            format!("{edit:?}")
        );
        assert!(fs::read_dir(&f.paths.central_dir).unwrap().all(|e| !e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".skills-hub-old-")));
        if fault == "invalid_utf8" {
            fs::write(f.source.join("SKILL.md"), upstream).unwrap();
        } else {
            conn.execute_batch("DROP TRIGGER fail_replay;").unwrap();
        }
        let report = f.update();
        assert!(
            matches!(
                report.skills[0].status,
                SkillRefreshStatus::Refreshed {
                    edit_conflict: Some(_),
                    ..
                }
            ),
            "{report:?}"
        );
        let replayed = if legacy {
            "---\nname: alpha\nuser-invocable: false\ndisable-model-invocation: true\n---\nnew body\n"
        } else {
            "---\nname: alpha\nuser-invocable: true\ndisable-model-invocation: true\n---\nnew body\n"
        };
        assert_eq!(f.text(), replayed);
        assert_eq!(
            f.store
                .get_skill_by_id(&f.id)
                .unwrap()
                .unwrap()
                .content_hash,
            Some(directory_identity(&f.central).unwrap())
        );
        assert!(fs::read_dir(&f.paths.central_dir).unwrap().all(|e| !e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".skills-hub-old-")));
    }
}

#[test]
fn replay_names_an_edit_snapshot_restore_failure() {
    let f = Fixture::new();
    f.set(Some(InvocationMode::UserOnly)).unwrap();
    let conn = rusqlite::Connection::open(f.store.db_path()).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_edit BEFORE INSERT ON skill_edits BEGIN SELECT RAISE(ABORT, 'test double fault'); END;").unwrap();
    let mut record = f.store.get_skill_by_id(&f.id).unwrap().unwrap();
    let err = mutation_guard::serialized(|| replay_unlocked(&f.store, &mut record)).unwrap_err();
    assert!(format!("{err:#}").contains("restore pre-replay Edit row"));
    assert!(format!("{err:#}").contains("test double fault"));
}

#[test]
fn invalid_utf8_is_typed_for_set_and_replay() {
    let f = Fixture::new();
    f.set(Some(InvocationMode::UserOnly)).unwrap();
    fs::write(f.central.join("SKILL.md"), [0xff]).unwrap();
    let mut record = f.store.get_skill_by_id(&f.id).unwrap().unwrap();
    for error in [
        f.set(Some(InvocationMode::Neither)).unwrap_err(),
        mutation_guard::serialized(|| replay_unlocked(&f.store, &mut record)).unwrap_err(),
    ] {
        assert!(error.downcast_ref::<std::io::Error>().is_some());
        assert!(matches!(
            error.downcast_ref::<SignalError>(),
            Some(SignalError::SkillManifestIo { .. })
        ));
        assert!(matches!(
            crate::commands::error::CommandError::from_anyhow(error),
            crate::commands::error::CommandError::SkillManifestIo { .. }
        ));
    }
}

#[test]
fn persisted_invocation_base_clears_and_replays_without_a_schema_change() {
    let f = Fixture::new();
    let original = "---\r\nname: alpha\r\nuser-invocable: 'no'  \r\n# keep\r\nuser-invocable: yes\r\ndisable-model-invocation: false\r\n---\r\nBody\r\n---\n";
    let edited = "---\r\nname: alpha\r\nuser-invocable: false\r\n# keep\r\nuser-invocable: false\r\ndisable-model-invocation: true\r\n---\r\nBody\r\n---\n";
    // Literal pre-extraction persisted fields, not serialized from today's type.
    let base = r#"{"disable_model_invocation":"disable-model-invocation: false\r\n","user_invocable":"user-invocable: 'no'  \r\n","had_frontmatter":true,"repeated_lines":[[4,"user-invocable: yes\r\n"]]}"#;
    let row = SkillEditRecord {
        skill_id: f.id.clone(),
        kind: SkillEditKind::InvocationMode,
        value: "neither".into(),
        base_value: base.into(),
        conflict: false,
        applied_at: 1,
    };
    fs::write(f.central.join("SKILL.md"), edited).unwrap();
    f.store.upsert_skill_edit(&row).unwrap();
    assert_eq!(
        invocation_override(&f.store, &f.id)
            .unwrap()
            .unwrap()
            .base_mode,
        InvocationMode::UserAndModel
    );
    f.set(None).unwrap();
    assert_eq!(f.text(), original);

    f.store.upsert_skill_edit(&row).unwrap();
    fs::write(f.source.join("SKILL.md"), original).unwrap();
    assert!(matches!(
        f.update().skills[0].status,
        SkillRefreshStatus::Refreshed {
            edit_conflict: None,
            ..
        }
    ));
    assert_eq!(f.text(), edited);
    let replayed = f
        .store
        .get_skill_edit(&f.id, SkillEditKind::InvocationMode)
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&replayed.base_value).unwrap(),
        serde_json::from_str::<serde_json::Value>(base).unwrap()
    );
    f.set(None).unwrap();
    assert_eq!(f.text(), original);
}

#[test]
fn legacy_indented_edit_clear_restores_bytes_before_deleting_row() {
    let f = Fixture::new();
    // Literal bytes and JSON from the pre-upgrade writer, not today's parser.
    let original = "  ---\nname: alpha\n---\nBody\n";
    let edited =
        "  ---\nname: alpha\ndisable-model-invocation: true\nuser-invocable: true\n---\nBody\n";
    let base = r#"{"disable_model_invocation":null,"user_invocable":null,"had_frontmatter":true,"repeated_lines":[]}"#;
    fs::write(f.central.join("SKILL.md"), edited).unwrap();
    f.store
        .upsert_skill_edit(&SkillEditRecord {
            skill_id: f.id.clone(),
            kind: SkillEditKind::InvocationMode,
            value: "user-only".into(),
            base_value: base.into(),
            conflict: false,
            applied_at: 1,
        })
        .unwrap();
    f.set(None).unwrap();
    assert_eq!(
        f.text(),
        original,
        "clear must restore legacy bytes before forgetting the Edit"
    );
    assert!(f
        .store
        .get_skill_edit(&f.id, SkillEditKind::InvocationMode)
        .unwrap()
        .is_none());
}

// Captured old-writer layout: every duplicate is replaced in place, with CRLF
// retained; the base stores the first lines plus absolute duplicate positions.
const LEGACY_ORIGINAL: &str = "  ---\r\nname: alpha\r\nuser-invocable: 'no'  \r\n# keep\r\nuser-invocable: yes\r\ndisable-model-invocation: false\r\n---\t\r\nBody\r\n---\nlater\n";
const LEGACY_EDITED: &str = "  ---\r\nname: alpha\r\nuser-invocable: false\r\n# keep\r\nuser-invocable: false\r\ndisable-model-invocation: true\r\n---\t\r\nBody\r\n---\nlater\n";
const LEGACY_BASE: &str = r#"{"disable_model_invocation":"disable-model-invocation: false\r\n","user_invocable":"user-invocable: 'no'  \r\n","had_frontmatter":true,"repeated_lines":[[4,"user-invocable: yes\r\n"]]}"#;

fn seed_legacy_edit(f: &Fixture) -> SkillEditRecord {
    let row = SkillEditRecord {
        skill_id: f.id.clone(),
        kind: SkillEditKind::InvocationMode,
        value: "neither".into(),
        base_value: LEGACY_BASE.into(),
        conflict: false,
        applied_at: 1,
    };
    fs::write(f.central.join("SKILL.md"), LEGACY_EDITED).unwrap();
    f.store.upsert_skill_edit(&row).unwrap();
    let mut record = f.store.get_skill_by_id(&f.id).unwrap().unwrap();
    content_identity::record(&f.store, &mut record).unwrap();
    row
}

#[test]
fn legacy_indented_edit_rechoose_and_clear_preserve_comments_duplicates_and_crlf() {
    for (mode, expected) in [
        (None, LEGACY_EDITED),
        (Some(InvocationMode::Neither), LEGACY_EDITED),
        (Some(InvocationMode::UserOnly), "  ---\r\nname: alpha\r\nuser-invocable: true\r\n# keep\r\nuser-invocable: true\r\ndisable-model-invocation: true\r\n---\t\r\nBody\r\n---\nlater\n"),
        (Some(InvocationMode::UserAndModel), "  ---\r\nname: alpha\r\nuser-invocable: true\r\n# keep\r\nuser-invocable: true\r\ndisable-model-invocation: false\r\n---\t\r\nBody\r\n---\nlater\n"),
    ] {
        let f = Fixture::new();
        seed_legacy_edit(&f);
        if let Some(mode) = mode {
            f.set(Some(mode)).unwrap();
        }
        assert_eq!(f.text(), expected, "re-choose must not prepend a synthetic header: {mode:?}");
        let row = f.store.get_skill_edit(&f.id, SkillEditKind::InvocationMode).unwrap().unwrap();
        assert_eq!(row.base_value, LEGACY_BASE);
        // Compatibility is not a general metadata/invocation parser policy.
        assert_eq!(manifest::parse_skill_md_with_reason(&f.central.join("SKILL.md")), Err("invalid_frontmatter"));
        assert_eq!(manifest::parse_invocation_mode(&f.text()), InvocationMode::UserAndModel);
        let entry = f.set(None).unwrap();
        assert_eq!(f.text(), LEGACY_ORIGINAL);
        assert!(entry.invocation_override.is_none());
        assert!(f.store.get_skill_edit(&f.id, SkillEditKind::InvocationMode).unwrap().is_none());
        assert_eq!(entry.skill.content_hash, directory_identity(&f.central));
    }
}

#[test]
fn legacy_indented_edit_update_and_clear_restore_current_upstream_not_legacy_base() {
    for (upstream, expected, base) in [
        (
            "---\r\nname: alpha\r\nuser-invocable: 'yes'  \r\n# current\r\n---\r\nNew body\n",
            "---\r\nname: alpha\r\nuser-invocable: false\r\n# current\r\ndisable-model-invocation: true\r\n---\r\nNew body\n",
            r#"{"disable_model_invocation":null,"user_invocable":"user-invocable: 'yes'  \r\n","had_frontmatter":true,"repeated_lines":[]}"#,
        ),
        (
            "  ---\r\nname: alpha\r\nuser-invocable: 'no'\r\n# current\r\n---\r\nNew body\n",
            "---\ndisable-model-invocation: true\nuser-invocable: false\n---\n  ---\r\nname: alpha\r\nuser-invocable: 'no'\r\n# current\r\n---\r\nNew body\n",
            r#"{"disable_model_invocation":null,"user_invocable":null,"had_frontmatter":false,"repeated_lines":[]}"#,
        ),
    ] {
        let f = Fixture::new();
        seed_legacy_edit(&f);
        fs::write(f.source.join("SKILL.md"), upstream).unwrap();
        // Repeated Updates must not stack synthetic headers or preserve an old base.
        for _ in 0..2 {
            let report = f.update();
            assert!(matches!(report.skills[0].status, SkillRefreshStatus::Refreshed { edit_conflict: None, .. }), "{report:?}");
            assert_eq!(f.text(), expected);
            let row = f.store.get_skill_edit(&f.id, SkillEditKind::InvocationMode).unwrap().unwrap();
            assert_eq!(serde_json::from_str::<serde_json::Value>(&row.base_value).unwrap(), serde_json::from_str::<serde_json::Value>(base).unwrap());
        }
        f.set(None).unwrap();
        assert_eq!(f.text(), upstream, "clear must restore CURRENT upstream bytes");
        assert!(f.store.get_skill_edit(&f.id, SkillEditKind::InvocationMode).unwrap().is_none());
    }
}

#[test]
fn fresh_indented_input_is_body_and_new_edit_round_trips_through_canonical_header() {
    let f = Fixture::new();
    fs::write(f.central.join("SKILL.md"), LEGACY_ORIGINAL).unwrap();
    f.set(Some(InvocationMode::Neither)).unwrap();
    assert_eq!(f.text(), "---\ndisable-model-invocation: true\nuser-invocable: false\n---\n  ---\r\nname: alpha\r\nuser-invocable: 'no'  \r\n# keep\r\nuser-invocable: yes\r\ndisable-model-invocation: false\r\n---\t\r\nBody\r\n---\nlater\n");
    let row = f
        .store
        .get_skill_edit(&f.id, SkillEditKind::InvocationMode)
        .unwrap()
        .unwrap();
    assert_eq!(
        row.base_value,
        r#"{"disable_model_invocation":null,"user_invocable":null,"had_frontmatter":false,"repeated_lines":[]}"#
    );
    f.set(Some(InvocationMode::UserOnly)).unwrap();
    f.set(None).unwrap();
    assert_eq!(f.text(), LEGACY_ORIGINAL);
}

#[cfg(unix)]
#[test]
fn legacy_indented_edit_failed_clear_or_rechoose_retains_base_and_retry_restores_bytes() {
    use std::os::unix::fs::PermissionsExt;
    for mode in [None, Some(InvocationMode::UserOnly)] {
        let f = Fixture::new();
        seed_legacy_edit(&f);
        let permissions = fs::metadata(&f.central).unwrap().permissions();
        fs::set_permissions(&f.central, fs::Permissions::from_mode(0o555)).unwrap();
        let result = f.set(mode);
        fs::set_permissions(&f.central, permissions).unwrap();
        assert!(matches!(
            crate::commands::error::CommandError::from_anyhow(result.unwrap_err()),
            crate::commands::error::CommandError::SkillManifestIo { .. }
        ));
        assert_eq!(f.text(), LEGACY_EDITED);
        let row = f
            .store
            .get_skill_edit(&f.id, SkillEditKind::InvocationMode)
            .unwrap()
            .unwrap();
        assert_eq!(row.base_value, LEGACY_BASE);
        assert_eq!(row.value, mode.unwrap_or(InvocationMode::Neither).as_key());
        assert_eq!(fs::read_dir(&f.central).unwrap().count(), 1);
        f.set(mode).unwrap();
        f.set(None).unwrap();
        assert_eq!(f.text(), LEGACY_ORIGINAL);
        assert!(f
            .store
            .get_skill_edit(&f.id, SkillEditKind::InvocationMode)
            .unwrap()
            .is_none());
    }
}

#[test]
fn legacy_indented_edit_missing_or_invalid_manifest_keeps_row() {
    for invalid in [false, true] {
        let f = Fixture::new();
        let before = seed_legacy_edit(&f);
        if invalid {
            fs::write(f.central.join("SKILL.md"), [0xff]).unwrap();
        } else {
            fs::remove_file(f.central.join("SKILL.md")).unwrap();
        }
        let error = f.set(None).unwrap_err();
        if invalid {
            assert!(matches!(
                error.downcast_ref::<SignalError>(),
                Some(SignalError::SkillManifestIo { .. })
            ));
            assert_eq!(fs::read(f.central.join("SKILL.md")).unwrap(), [0xff]);
        } else {
            assert!(matches!(
                error.downcast_ref::<SignalError>(),
                Some(SignalError::CentralPathMissing { .. })
            ));
            assert!(!f.central.join("SKILL.md").exists());
        }
        let row = f
            .store
            .get_skill_edit(&f.id, SkillEditKind::InvocationMode)
            .unwrap()
            .unwrap();
        assert_eq!(format!("{row:?}"), format!("{before:?}"));
    }
}

#[test]
fn preexisting_database_gets_edit_table_and_skill_deletion_cascades() {
    let f = Fixture::new();
    let conn = rusqlite::Connection::open(f.store.db_path()).unwrap();
    conn.execute_batch("DROP TABLE skill_edits;").unwrap();
    f.store.ensure_schema().unwrap();
    f.set(Some(InvocationMode::Neither)).unwrap();
    f.store.delete_skill(&f.id).unwrap();
    assert!(f
        .store
        .get_skill_edit(&f.id, SkillEditKind::InvocationMode)
        .unwrap()
        .is_none());
}

fn directory_identity(path: &Path) -> Option<String> {
    content_identity::read(content_identity::Source::Directory(path))
}
