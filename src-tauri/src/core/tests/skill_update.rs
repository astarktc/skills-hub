use super::*;
use crate::core::test_git_api::StubApi;
use std::{fs, path::Path};
fn make_store() -> (tempfile::TempDir, SkillStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    (dir, store)
}

/// Installer roots isolated under one temp dir: an empty home (no tool is
/// installed), a central repo, and a git cache.
fn make_paths() -> (tempfile::TempDir, InstallerPaths) {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = InstallerPaths {
        home: dir.path().join("home"),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.home).unwrap();
    (dir, paths)
}

#[test]
fn slash_branch_install_and_update_keep_url_and_resolved_subpath() {
    let (_dir, store) = make_store();
    let (_roots, paths) = make_paths();
    let url = "https://github.com/owner/repo/tree/feature/x/skills/a";
    let installed = crate::core::installer::install_git_skill_from_selection_with(
        &paths,
        &store,
        url,
        "skills/a",
        None,
        None,
        &StubApi::serving("first"),
    )
    .unwrap();
    let record = store.get_skill_by_id(&installed.skill_id).unwrap().unwrap();
    assert_eq!(record.source_ref.as_deref(), Some(url));
    assert_eq!(record.source_subpath.as_deref(), Some("skills/a"));
    let acquired = acquire_update(
        &paths,
        &store,
        &installed.skill_id,
        None,
        &StubApi::serving("next"),
        0,
        None,
    )
    .unwrap();
    assert_eq!(acquired.record.source_ref.as_deref(), Some(url));
    assert_eq!(acquired.record.source_subpath.as_deref(), Some("skills/a"));
    assert_eq!(staged_parts(&acquired).1.as_deref(), Some("next"));
}

#[test]
fn stale_split_repair_is_acquire_first_and_persisted_only_by_finalize() {
    let (dir, store) = make_store();
    let (_roots, paths) = make_paths();
    let installed = crate::core::installer::install_git_skill_from_selection_with(
        &paths,
        &store,
        "https://github.com/owner/repo/tree/feature/x/skills/a",
        "skills/a",
        None,
        None,
        &StubApi::serving("first"),
    )
    .unwrap();
    let mut before = store.get_skill_by_id(&installed.skill_id).unwrap().unwrap();
    before.source_subpath = Some("x/skills/a".into());
    store.upsert_skill(&before).unwrap();
    let old_bytes = fs::read(installed.central_path.join("SKILL.md")).unwrap();
    let db = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
    db.execute_batch("CREATE TRIGGER reject_skill_write BEFORE INSERT ON skills BEGIN SELECT RAISE(ABORT, 'test upsert failure'); END;").unwrap();
    let api = StubApi {
        missing_branch: Some("feature"),
        ..StubApi::serving("next")
    };

    let acquired =
        acquire_update(&paths, &store, &installed.skill_id, None, &api, 0, None).unwrap();

    assert_eq!(acquired.record.source_subpath.as_deref(), Some("skills/a"));
    assert_eq!(
        format!(
            "{:?}",
            store.get_skill_by_id(&installed.skill_id).unwrap().unwrap()
        ),
        format!("{before:?}")
    );
    assert_eq!(
        fs::read(installed.central_path.join("SKILL.md")).unwrap(),
        old_bytes
    );
    db.execute_batch("DROP TRIGGER reject_skill_write;")
        .unwrap();
    crate::core::mutation_guard::serialized(|| apply_unlocked(&paths, &store, acquired)).unwrap();
    let after = store.get_skill_by_id(&installed.skill_id).unwrap().unwrap();
    assert_eq!(after.source_subpath.as_deref(), Some("skills/a"));
    assert_eq!(after.source_revision.as_deref(), Some("next"));
    assert_eq!(after.id, before.id);
}

#[test]
fn update_acquisition_surfaces_a_typed_not_found() {
    let (_dir, store) = make_store();
    let (_roots, paths) = make_paths();

    let installed = crate::core::installer::install_git_skill_from_selection_with(
        &paths,
        &store,
        "https://github.com/owner/repo/tree/main/skills/a",
        "skills/a",
        None,
        None,
        &StubApi::serving("abc123def4567890123456789012345678901234"),
    )
    .expect("the fast path installs");

    let err = acquire_update(
        &paths,
        &store,
        &installed.skill_id,
        None,
        &StubApi::failing(404),
        0,
        None,
    )
    .err()
    .expect("a removed skill fails its update");

    assert!(
        matches!(
            err.downcast_ref::<SignalError>(),
            Some(SignalError::GithubSkillNotFound { url }) if url.contains("skills/a")
        ),
        "expected GithubSkillNotFound, got: {err:#}"
    );
}

#[test]
fn update_acquisition_uses_the_fast_path() {
    let (_dir, store) = make_store();
    let (_roots, paths) = make_paths();

    let installed = crate::core::installer::install_git_skill_from_selection_with(
        &paths,
        &store,
        "https://github.com/owner/repo/tree/main/skills/a",
        "skills/a",
        None,
        None,
        &StubApi::serving("1111111111111111111111111111111111111111"),
    )
    .expect("install");

    let acquired = acquire_update(
        &paths,
        &store,
        &installed.skill_id,
        None,
        &StubApi::serving("2222222222222222222222222222222222222222"),
        0,
        None,
    )
    .expect("update acquires");

    assert_eq!(
        staged_parts(&acquired).1.as_deref(),
        Some("2222222222222222222222222222222222222222")
    );
    assert!(staged_parts(&acquired).0.path().join("SKILL.md").exists());
    assert!(
        !paths.cache_dir.join("skills-hub-git-cache").exists(),
        "the update fast path must not clone either"
    );
}
fn staged_parts(request: &UpdateRequest) -> (&StagingDir, &Option<String>) {
    match &request.bytes {
        UpdateBytes::GitAcquired { staged, revision }
        | UpdateBytes::RestoreRebuild { staged, revision } => (staged, revision),
        _ => panic!("expected staged bytes"),
    }
}

fn fixture() -> (tempfile::TempDir, InstallerPaths, SkillStore, SkillRecord) {
    let (dir, paths) = make_paths();
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().unwrap();
    let source = dir.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("SKILL.md"), "---\nname: alpha\n---\nold\n").unwrap();
    let installed =
        crate::core::installer::install_local_skill(&paths, &store, &source, None).unwrap();
    let record = store.get_skill_by_id(&installed.skill_id).unwrap().unwrap();
    (dir, paths, store, record)
}

fn staged_request(paths: &InstallerPaths, record: &SkillRecord) -> UpdateRequest {
    let staged = StagingDir::new_in(&paths.central_dir);
    fs::create_dir(staged.path()).unwrap();
    fs::write(
        staged.path().join("SKILL.md"),
        "---\nname: alpha\n---\nnew\n",
    )
    .unwrap();
    UpdateRequest {
        expected: record.clone(),
        record: record.clone(),
        repoint: false,
        bytes: UpdateBytes::GitAcquired {
            staged,
            revision: Some("next".into()),
        },
    }
}

#[test]
fn every_byte_adapter_settles_and_reports_propagation() {
    use crate::core::{
        propagation::PropagationStatus,
        skill_store::SkillTargetRecord,
        sync_status::{SyncMode, SyncStatus},
        tool_adapters,
    };
    for kind in ["git", "local", "edit", "restore"] {
        let (_dir, paths, store, record) = fixture();
        let mut adapter = tool_adapters::adapter_by_key("cursor").unwrap().clone();
        adapter.supports_symlink = false;
        let adapter = tool_adapters::test_overrides::shadow(adapter);
        fs::create_dir_all(paths.home.join(adapter.relative_detect_dir)).unwrap();
        let target = paths.home.join("target");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("SKILL.md"), "old").unwrap();
        store
            .upsert_skill_target(&SkillTargetRecord {
                id: "target".into(),
                skill_id: record.id.clone(),
                tool: "cursor".into(),
                target_path: target.to_string_lossy().into_owned(),
                mode: SyncMode::Copy,
                status: SyncStatus::Synced,
                last_error: None,
                synced_at: Some(1),
            })
            .unwrap();
        let mut request = staged_request(&paths, &record);
        match kind {
            "local" => {
                let path = PathBuf::from(record.source_ref.as_ref().unwrap());
                fs::write(path.join("SKILL.md"), "new local").unwrap();
                request.bytes = UpdateBytes::LocalFolder { path };
            }
            "edit" => {
                fs::write(Path::new(&record.central_path).join("SKILL.md"), "new edit").unwrap();
                request.bytes = UpdateBytes::EditInPlace { clear: false };
            }
            "restore" => {
                fs::remove_dir_all(&record.central_path).unwrap();
                let UpdateBytes::GitAcquired { staged, revision } = request.bytes else {
                    unreachable!()
                };
                request.bytes = UpdateBytes::RestoreRebuild { staged, revision };
            }
            _ => {}
        }
        let outcome =
            crate::core::mutation_guard::serialized(|| apply_unlocked(&paths, &store, request))
                .unwrap();
        let ApplyOutcome::Updated(outcome) = outcome else {
            panic!("unexpected skip")
        };
        assert_eq!(outcome.skill_id, record.id);
        assert_eq!(outcome.name, record.name);
        assert!(outcome.content_hash.is_some());
        assert_eq!(
            outcome.content_hash,
            store
                .get_skill_by_id(&record.id)
                .unwrap()
                .unwrap()
                .content_hash
        );
        assert!(matches!(
            outcome.propagation.targets[0].status,
            PropagationStatus::Synced {
                mode_used: SyncMode::Copy
            }
        ));
        assert_eq!(
            fs::read(target.join("SKILL.md")).unwrap(),
            fs::read(Path::new(&record.central_path).join("SKILL.md")).unwrap()
        );
    }
}

#[test]
fn admission_discards_staging_for_deleted_or_changed_sources() {
    for change in ["gone", "ref", "subpath", "type"] {
        let (_dir, paths, store, record) = fixture();
        let request = staged_request(&paths, &record);
        let staged = staged_parts(&request).0.path().to_path_buf();
        let old = fs::read(Path::new(&record.central_path).join("SKILL.md")).unwrap();
        let reason = if change == "gone" {
            store.delete_skill(&record.id).unwrap();
            UpdateSkip::SkillGone
        } else {
            let mut current = record.clone();
            match change {
                "ref" => current.source_ref = Some("new source".into()),
                "subpath" => current.source_subpath = Some("new subpath".into()),
                _ => current.source_type = "imported".into(),
            }
            store.upsert_skill(&current).unwrap();
            UpdateSkip::StaleAcquisition
        };
        let outcome =
            crate::core::mutation_guard::serialized(|| apply_unlocked(&paths, &store, request))
                .unwrap();
        assert!(matches!(outcome, ApplyOutcome::Skipped { reason: actual } if actual == reason));
        assert!(!staged.exists());
        assert_eq!(
            fs::read(Path::new(&record.central_path).join("SKILL.md")).unwrap(),
            old
        );
        assert!(store.list_skill_targets(&record.id).unwrap().is_empty());
        if change == "gone" {
            assert!(store.get_skill_by_id(&record.id).unwrap().is_none());
        }
    }
}

#[test]
fn failed_local_repoint_preserves_source_and_old_bytes() {
    use crate::core::refresh::{RefreshPhase, RefreshPolicy, SkillRefreshStatus};
    for fault in ["copy", "settle"] {
        let (dir, paths, store, record) = fixture();
        let source = dir.path().join("replacement");
        fs::create_dir(&source).unwrap();
        fs::write(
            source.join("SKILL.md"),
            "---\nname: alpha\n---\nreplacement\n",
        )
        .unwrap();
        let before = fs::read(Path::new(&record.central_path).join("SKILL.md")).unwrap();
        let conn = rusqlite::Connection::open(store.db_path()).unwrap();
        if fault == "settle" {
            conn.execute_batch(
                "CREATE TRIGGER fail_repoint BEFORE INSERT ON skills
                WHEN NEW.source_ref != (SELECT source_ref FROM skills WHERE id = NEW.id)
                BEGIN SELECT RAISE(ABORT, 'test failed repoint'); END;",
            )
            .unwrap();
        }
        let report = crate::core::unlocatable::repoint_and_update(
            &paths,
            &store,
            &record.id,
            &source,
            RefreshPolicy::default(),
            None,
            1234,
            |progress| {
                if fault == "copy" && progress.phase == RefreshPhase::Applying {
                    fs::remove_dir_all(&source).unwrap();
                }
            },
        )
        .unwrap();
        assert!(
            matches!(report.skills[0].status, SkillRefreshStatus::Failed { .. }),
            "{report:?}"
        );
        assert_eq!(
            format!("{:?}", store.get_skill_by_id(&record.id).unwrap().unwrap()),
            format!("{record:?}")
        );
        assert_eq!(
            fs::read(Path::new(&record.central_path).join("SKILL.md")).unwrap(),
            before
        );
        assert!(store.list_skill_targets(&record.id).unwrap().is_empty());
    }
}

#[test]
fn admission_preserves_current_non_source_fields() {
    let (_dir, paths, store, record) = fixture();
    let request = staged_request(&paths, &record);
    let mut current = record.clone();
    current.name = "renamed".into();
    current.last_sync_at = Some(345);
    store.upsert_skill(&current).unwrap();
    let outcome =
        crate::core::mutation_guard::serialized(|| apply_unlocked(&paths, &store, request))
            .unwrap();
    assert!(matches!(outcome, ApplyOutcome::Updated(_)));
    let updated = store.get_skill_by_id(&record.id).unwrap().unwrap();
    assert_eq!(updated.name, current.name);
    assert_eq!(updated.last_sync_at, current.last_sync_at);
}

#[test]
fn restore_rebuilds_the_central_copy_and_its_dangling_link() {
    let (_dir, paths, store, record) = fixture();
    let adapter = crate::core::tool_adapters::adapter_by_key("claude_code").unwrap();
    fs::create_dir_all(paths.home.join(adapter.relative_detect_dir)).unwrap();
    let target = paths.home.join("linked");
    let synced =
        crate::core::sync_engine::sync_dir_hybrid(Path::new(&record.central_path), &target)
            .unwrap();
    store
        .upsert_skill_target(&crate::core::skill_store::SkillTargetRecord {
            id: "linked".into(),
            skill_id: record.id.clone(),
            tool: "claude_code".into(),
            target_path: target.to_string_lossy().into_owned(),
            mode: synced.mode_used,
            status: crate::core::sync_status::SyncStatus::Synced,
            last_error: None,
            synced_at: Some(1),
        })
        .unwrap();
    fs::remove_dir_all(&record.central_path).unwrap();
    let request = acquire_update(
        &paths,
        &store,
        &record.id,
        None,
        &StubApi::serving("next"),
        0,
        None,
    )
    .unwrap();
    assert!(matches!(request.bytes, UpdateBytes::RestoreRebuild { .. }));
    let ApplyOutcome::Updated(outcome) =
        crate::core::mutation_guard::serialized(|| apply_unlocked(&paths, &store, request))
            .unwrap()
    else {
        panic!("restore skipped")
    };
    assert_eq!(outcome.propagation.targets.len(), 1);
    assert!(target.join("SKILL.md").is_file());
    assert_eq!(
        store
            .get_skill_by_id(&record.id)
            .unwrap()
            .unwrap()
            .central_path,
        record.central_path
    );
}
