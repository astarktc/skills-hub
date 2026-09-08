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
        frontmatter_edit::read_invocation_lines(&f.text()).mode(),
        InvocationMode::UserOnly
    );
    assert_ne!(hash_dir(&f.central).unwrap(), hash_dir(&f.source).unwrap());
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
        serde_json::to_string(&frontmatter_edit::read_invocation_lines(upstream)).unwrap()
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
    assert_eq!(entry.skill.content_hash, Some(hash_dir(&f.source).unwrap()));
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
        let user = frontmatter_edit::write_invocation_mode(UPSTREAM, InvocationMode::UserOnly);
        fs::write(f.source.join("SKILL.md"), &user).unwrap();
        f.update();
        f.set(Some(InvocationMode::ModelOnly)).unwrap();
        if previous_conflict {
            fs::write(
                f.source.join("SKILL.md"),
                frontmatter_edit::write_invocation_mode(UPSTREAM, InvocationMode::Neither),
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
        let model = frontmatter_edit::write_invocation_mode(UPSTREAM, InvocationMode::ModelOnly);
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
        frontmatter_edit::read_invocation_lines(&f.text()).mode(),
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
        frontmatter_edit::read_invocation_lines(&f.text()).mode(),
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
        serde_json::to_string(&frontmatter_edit::read_invocation_lines(UPSTREAM)).unwrap()
    );
    let mut record = f.store.get_skill_by_id(&f.id).unwrap().unwrap();
    mutation_guard::serialized(|| replay_unlocked(&f.store, &mut record)).unwrap();
    assert_eq!(
        frontmatter_edit::read_invocation_lines(&f.text()).mode(),
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
