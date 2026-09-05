use crate::core::sync_status::{SyncMode, SyncStatus};
use std::fs;
use std::path::Path;

use crate::core::project_sync;
use crate::core::skill_store::{ProjectRecord, SkillRecord, SkillStore, SkillTargetRecord};

fn make_store() -> (tempfile::TempDir, SkillStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    (dir, store)
}

fn make_skill_dir(base: &Path, name: &str) -> std::path::PathBuf {
    let dir = base.join(name);
    fs::create_dir_all(&dir).expect("create skill dir");
    fs::write(dir.join("SKILL.md"), "# Test Skill\nTest content.").expect("write SKILL.md");
    dir
}

/// A copy-only tool key for the calling thread: no shipped registry entry
/// lacks `supports_symlink` any more, so tests that need copy-mode
/// bookkeeping shadow the Cursor record with the capability flipped.
fn copy_only_tool() -> &'static str {
    let mut adapter = crate::core::tool_adapters::adapter_by_key("cursor")
        .expect("cursor adapter")
        .clone();
    adapter.supports_symlink = false;
    crate::core::tool_adapters::test_overrides::shadow(adapter).key()
}

/// A listing whose reconcile pass actually ran.
///
/// The mutation guard is process-global and `cargo test` runs tests in
/// parallel, so an unlucky call can legitimately return `reconciled: false`
/// (another test's mutation was in flight). Tests asserting on *reconciled*
/// status retry through this helper; the skip path has its own test.
fn list_reconciled(
    store: &SkillStore,
    project_id: &str,
) -> Vec<crate::core::skill_store::ProjectSkillAssignmentRecord> {
    for _ in 0..100 {
        let listing = project_sync::list_assignments_with_staleness(store, project_id)
            .expect("list should succeed");
        if listing.reconciled {
            return listing.assignments;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("reconcile pass never got the guard");
}

fn register_project_and_skill(
    store: &SkillStore,
    project_path: &str,
    skill_name: &str,
    skill_central_path: &str,
) -> (ProjectRecord, SkillRecord) {
    let now = 1000i64;
    let project = ProjectRecord {
        id: uuid::Uuid::new_v4().to_string(),
        path: project_path.to_string(),
        created_at: now,
        updated_at: now,
    };
    store.register_project(&project).unwrap();

    let skill = SkillRecord {
        id: uuid::Uuid::new_v4().to_string(),
        name: skill_name.to_string(),
        description: None,
        source_type: "local".to_string(),
        source_ref: None,
        source_subpath: None,
        source_revision: None,
        central_path: skill_central_path.to_string(),
        content_hash: None,
        created_at: now,
        updated_at: now,
        last_sync_at: None,
        last_seen_at: now,
        status: "ok".to_string(),
    };
    store.upsert_skill(&skill).unwrap();

    (project, skill)
}

#[test]
fn assign_creates_symlink() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let skill_dir = make_skill_dir(tmpdir.path(), "test-skill");
    let project_dir = tmpdir.path().join("my-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "test-skill",
        &skill_dir.to_string_lossy(),
    );

    let result = project_sync::assign_and_sync(&store, &project, &skill, "claude_code", 2000);
    let record = result.expect("assign_and_sync should succeed");

    assert_eq!(record.status, SyncStatus::Synced);
    assert_eq!(record.mode, SyncMode::Symlink);
    assert!(
        record.content_hash.is_none(),
        "symlink mode should not store content_hash"
    );
    assert!(record.synced_at.is_some());

    // Verify filesystem: target should exist and be a symlink
    let target = project_dir.join(".claude/skills/test-skill");
    assert!(target.exists(), "target should exist");
    assert!(
        target.symlink_metadata().unwrap().file_type().is_symlink(),
        "target should be a symlink"
    );
}

#[test]
fn assign_stores_hash_for_copy() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let skill_dir = make_skill_dir(tmpdir.path(), "copy-skill");
    let project_dir = tmpdir.path().join("copy-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "copy-skill",
        &skill_dir.to_string_lossy(),
    );

    let tool = copy_only_tool();
    let result = project_sync::assign_and_sync(&store, &project, &skill, tool, 2000);
    let record = result.expect("assign_and_sync should succeed");

    assert_eq!(record.status, SyncStatus::Synced);
    assert_eq!(record.mode, SyncMode::Copy);
    assert!(
        record.content_hash.is_some(),
        "copy mode should store content_hash"
    );
    let hash = record.content_hash.unwrap();
    assert!(!hash.is_empty(), "content_hash should be non-empty");

    // Verify filesystem: target should exist and NOT be a symlink
    let target = project_dir.join(".agents/skills/copy-skill");
    assert!(target.exists(), "target should exist");
    assert!(
        !target.symlink_metadata().unwrap().file_type().is_symlink(),
        "target should NOT be a symlink for a copy-only tool"
    );
}

#[test]
fn assign_records_error_on_sync_failure() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let project_dir = tmpdir.path().join("err-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    // Use a non-existent source path for the skill: sync fails in every mode.
    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "missing-skill",
        "/nonexistent/path/to/skill",
    );

    let result = project_sync::assign_and_sync(&store, &project, &skill, "cursor", 2000);
    let record = result.expect("assign_and_sync should return Ok even on sync failure");

    assert_eq!(record.status, SyncStatus::Error);
    assert!(record.last_error.is_some(), "should have an error message");
}

#[test]
fn unassign_removes_symlink() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let skill_dir = make_skill_dir(tmpdir.path(), "unsync-skill");
    let project_dir = tmpdir.path().join("unsync-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "unsync-skill",
        &skill_dir.to_string_lossy(),
    );

    // First assign
    project_sync::assign_and_sync(&store, &project, &skill, "claude_code", 2000)
        .expect("assign should succeed");

    let target = project_dir.join(".claude/skills/unsync-skill");
    assert!(target.exists(), "target should exist after assign");

    // Now unassign
    project_sync::unassign_and_remove_artifacts(&store, &project, &skill, "claude_code")
        .expect("unassign should succeed");

    assert!(!target.exists(), "target should not exist after unassign");

    // DB record should be gone
    let assignment = store
        .get_project_skill_assignment(&project.id, &skill.id, "claude_code")
        .unwrap();
    assert!(assignment.is_none(), "DB record should be deleted");
}

#[test]
fn unassign_target_not_found_cleans_db() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let skill_dir = make_skill_dir(tmpdir.path(), "ghost-skill");
    let project_dir = tmpdir.path().join("ghost-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "ghost-skill",
        &skill_dir.to_string_lossy(),
    );

    // Assign to create DB record and symlink
    project_sync::assign_and_sync(&store, &project, &skill, "claude_code", 2000)
        .expect("assign should succeed");

    // Manually delete the target path (simulates external removal)
    let target = project_dir.join(".claude/skills/ghost-skill");
    if target.symlink_metadata().is_ok() {
        fs::remove_file(&target).ok();
        fs::remove_dir_all(&target).ok();
    }
    assert!(
        !target.exists() && target.symlink_metadata().is_err(),
        "target should be deleted"
    );

    // Unassign should gracefully clean up the DB record
    project_sync::unassign_and_remove_artifacts(&store, &project, &skill, "claude_code")
        .expect("unassign should succeed even when target is gone");

    let assignment = store
        .get_project_skill_assignment(&project.id, &skill.id, "claude_code")
        .unwrap();
    assert!(assignment.is_none(), "DB record should be deleted");
}

/// Make the *parent* of `target` read-only so a symlink at `target` cannot
/// be unlinked. Returns `false` when permissions are not enforced (root), so
/// the caller can skip.
#[cfg(unix)]
fn lock_parent(target: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let parent = target.parent().expect("target has a parent");
    fs::set_permissions(parent, fs::Permissions::from_mode(0o555)).unwrap();
    if fs::remove_file(target).is_ok() {
        // Root ignores the mode bits; nothing to test.
        fs::set_permissions(parent, fs::Permissions::from_mode(0o755)).unwrap();
        return false;
    }
    true
}

#[cfg(unix)]
fn unlock_parent(target: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let parent = target.parent().expect("target has a parent");
    fs::set_permissions(parent, fs::Permissions::from_mode(0o755)).unwrap();
}

/// ADR-0002 at project scope: the row that locates a stuck artifact is kept
/// with Sync status `error`, and the caller's final policy turns the report's
/// failures into the typed `DeleteCleanupFailed` naming the path.
#[cfg(unix)]
#[test]
fn unassign_failure_keeps_the_row_as_error_and_reports_the_path() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let skill_dir = make_skill_dir(tmpdir.path(), "stuck-skill");
    let project_dir = tmpdir.path().join("stuck-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "stuck-skill",
        &skill_dir.to_string_lossy(),
    );
    project_sync::assign_and_sync(&store, &project, &skill, "claude_code", 2000)
        .expect("assign should succeed");

    let target = project_dir.join(".claude/skills/stuck-skill");
    if !lock_parent(&target) {
        return; // running as root
    }

    let err = project_sync::unassign_and_remove_artifacts(&store, &project, &skill, "claude_code")
        .expect_err("a stuck artifact must fail the unassign");
    unlock_parent(&target);

    match err.downcast_ref::<crate::core::errors::SignalError>() {
        Some(crate::core::errors::SignalError::DeleteCleanupFailed { failures }) => {
            assert_eq!(failures.len(), 1);
            assert!(
                failures[0].starts_with(&format!("{}: ", target.display())),
                "the report names the path it could not remove: {:?}",
                failures
            );
        }
        other => panic!("expected DeleteCleanupFailed, got {:?}", other),
    }

    let assignment = store
        .get_project_skill_assignment(&project.id, &skill.id, "claude_code")
        .unwrap()
        .expect("row is kept, never deleted blind");
    assert_eq!(assignment.status, SyncStatus::Error);
    assert!(
        assignment.last_error.is_some(),
        "the diagnostic is recorded on the row"
    );
    assert!(
        target.symlink_metadata().is_ok(),
        "the artifact is still there"
    );
}

#[test]
fn resync_updates_all() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let skill1_dir = make_skill_dir(tmpdir.path(), "skill-a");
    let skill2_dir = make_skill_dir(tmpdir.path(), "skill-b");
    let project_dir = tmpdir.path().join("resync-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let (project, skill1) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "skill-a",
        &skill1_dir.to_string_lossy(),
    );

    let skill2 = SkillRecord {
        id: uuid::Uuid::new_v4().to_string(),
        name: "skill-b".to_string(),
        description: None,
        source_type: "local".to_string(),
        source_ref: None,
        source_subpath: None,
        source_revision: None,
        central_path: skill2_dir.to_string_lossy().to_string(),
        content_hash: None,
        created_at: 1000,
        updated_at: 1000,
        last_sync_at: None,
        last_seen_at: 1000,
        status: "ok".to_string(),
    };
    store.upsert_skill(&skill2).unwrap();

    // Assign both skills
    project_sync::assign_and_sync(&store, &project, &skill1, "claude_code", 2000)
        .expect("assign skill1");
    project_sync::assign_and_sync(&store, &project, &skill2, "claude_code", 2000)
        .expect("assign skill2");

    // Modify source of skill1 (add a new file)
    fs::write(skill1_dir.join("extra.txt"), "new content").expect("write extra file");

    // Re-sync the project
    let summary = project_sync::resync_project(&store, &project.id, 3000)
        .expect("resync_project should succeed");

    assert_eq!(summary.synced, 2, "both assignments should be re-synced");
    assert_eq!(summary.failed, 0, "no failures expected");
    assert_eq!(summary.project_id, project.id);

    // Verify both targets still exist
    let target1 = project_dir.join(".claude/skills/skill-a");
    let target2 = project_dir.join(".claude/skills/skill-b");
    assert!(target1.exists(), "skill-a target should exist after resync");
    assert!(target2.exists(), "skill-b target should exist after resync");
}

#[test]
fn resync_continues_on_error() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let skill1_dir = make_skill_dir(tmpdir.path(), "ok-skill");
    let project_dir = tmpdir.path().join("partial-resync-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let (project, skill1) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "ok-skill",
        &skill1_dir.to_string_lossy(),
    );

    // Second skill with a path that will be deleted after assignment
    let bad_skill_dir = make_skill_dir(tmpdir.path(), "bad-skill");
    let bad_skill = SkillRecord {
        id: uuid::Uuid::new_v4().to_string(),
        name: "bad-skill".to_string(),
        description: None,
        source_type: "local".to_string(),
        source_ref: None,
        source_subpath: None,
        source_revision: None,
        central_path: bad_skill_dir.to_string_lossy().to_string(),
        content_hash: None,
        created_at: 1000,
        updated_at: 1000,
        last_sync_at: None,
        last_seen_at: 1000,
        status: "ok".to_string(),
    };
    store.upsert_skill(&bad_skill).unwrap();

    // Assign both; a missing source fails in every sync mode
    project_sync::assign_and_sync(&store, &project, &skill1, "cursor", 2000)
        .expect("assign ok-skill");
    project_sync::assign_and_sync(&store, &project, &bad_skill, "cursor", 2000)
        .expect("assign bad-skill");

    // Delete the source of bad-skill to cause resync failure
    fs::remove_dir_all(&bad_skill_dir).expect("remove bad-skill source");

    // Re-sync should continue despite the error on bad-skill
    let summary = project_sync::resync_project(&store, &project.id, 3000)
        .expect("resync_project should succeed overall");

    assert_eq!(summary.synced, 1, "one assignment should succeed");
    assert_eq!(summary.failed, 1, "one assignment should fail");
    assert_eq!(summary.errors.len(), 1, "one error recorded");

    // Verify the failed assignment has error status in DB
    let bad_assignment = store
        .get_project_skill_assignment(&project.id, &bad_skill.id, "cursor")
        .unwrap()
        .expect("bad assignment should exist");
    assert_eq!(bad_assignment.status, SyncStatus::Error);

    // Verify the successful assignment has synced status
    let ok_assignment = store
        .get_project_skill_assignment(&project.id, &skill1.id, "cursor")
        .unwrap()
        .expect("ok assignment should exist");
    assert_eq!(ok_assignment.status, SyncStatus::Synced);
}

#[test]
fn resync_all_multiple_projects() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    // Project 1
    let skill1_dir = make_skill_dir(tmpdir.path(), "all-skill-1");
    let project1_dir = tmpdir.path().join("all-project-1");
    fs::create_dir_all(&project1_dir).expect("create project1 dir");
    let (project1, skill1) = register_project_and_skill(
        &store,
        &project1_dir.to_string_lossy(),
        "all-skill-1",
        &skill1_dir.to_string_lossy(),
    );
    project_sync::assign_and_sync(&store, &project1, &skill1, "claude_code", 2000)
        .expect("assign to project1");

    // Project 2
    let skill2_dir = make_skill_dir(tmpdir.path(), "all-skill-2");
    let project2_dir = tmpdir.path().join("all-project-2");
    fs::create_dir_all(&project2_dir).expect("create project2 dir");
    let (project2, skill2) = register_project_and_skill(
        &store,
        &project2_dir.to_string_lossy(),
        "all-skill-2",
        &skill2_dir.to_string_lossy(),
    );
    project_sync::assign_and_sync(&store, &project2, &skill2, "claude_code", 2000)
        .expect("assign to project2");

    // Re-sync all
    let summaries = project_sync::resync_all_projects(&store, 3000)
        .expect("resync_all_projects should succeed");

    assert_eq!(summaries.len(), 2, "should have 2 project summaries");
    for s in &summaries {
        assert_eq!(s.synced, 1, "each project should have 1 synced assignment");
        assert_eq!(s.failed, 0, "no failures expected");
    }
}

#[test]
fn staleness_detected_for_copy() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let skill_dir = make_skill_dir(tmpdir.path(), "stale-skill");
    let project_dir = tmpdir.path().join("stale-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "stale-skill",
        &skill_dir.to_string_lossy(),
    );

    // copy mode stores content_hash
    let tool = copy_only_tool();
    project_sync::assign_and_sync(&store, &project, &skill, tool, 2000)
        .expect("assign should succeed");

    // Verify initial status is synced
    let before = store
        .get_project_skill_assignment(&project.id, &skill.id, "cursor")
        .unwrap()
        .expect("assignment exists");
    assert_eq!(before.status, SyncStatus::Synced);
    assert!(before.content_hash.is_some());

    // Modify source to change the hash
    fs::write(skill_dir.join("new-file.txt"), "changed content").expect("write new file");

    // list_assignments_with_staleness should detect the change
    let assignments = list_reconciled(&store, &project.id);
    assert_eq!(assignments.len(), 1);
    assert_eq!(assignments[0].status, SyncStatus::Stale);

    // DB should also be updated to stale
    let after = store
        .get_project_skill_assignment(&project.id, &skill.id, "cursor")
        .unwrap()
        .expect("assignment exists");
    assert_eq!(after.status, SyncStatus::Stale);
}

#[test]
fn staleness_skipped_for_symlink() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let skill_dir = make_skill_dir(tmpdir.path(), "sym-skill");
    let project_dir = tmpdir.path().join("sym-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "sym-skill",
        &skill_dir.to_string_lossy(),
    );

    // claude_code uses symlink mode
    project_sync::assign_and_sync(&store, &project, &skill, "claude_code", 2000)
        .expect("assign should succeed");

    // Modify source
    fs::write(skill_dir.join("new-file.txt"), "changed content").expect("write new file");

    // Staleness check should skip symlink-mode -- status stays synced
    let assignments = list_reconciled(&store, &project.id);
    assert_eq!(assignments.len(), 1);
    assert_eq!(
        assignments[0].status,
        SyncStatus::Synced,
        "symlink-mode should not become stale"
    );
}

#[test]
fn missing_status_when_source_absent() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let skill_dir = make_skill_dir(tmpdir.path(), "vanish-skill");
    let project_dir = tmpdir.path().join("vanish-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "vanish-skill",
        &skill_dir.to_string_lossy(),
    );

    let tool = copy_only_tool();
    project_sync::assign_and_sync(&store, &project, &skill, tool, 2000)
        .expect("assign should succeed");

    // Delete source directory entirely
    fs::remove_dir_all(&skill_dir).expect("remove source");

    // Should detect missing source and mark status as "missing"
    let assignments = list_reconciled(&store, &project.id);
    assert_eq!(assignments.len(), 1);
    assert_eq!(
        assignments[0].status,
        SyncStatus::Missing,
        "source absent should produce missing status"
    );

    // Verify DB persisted the missing status
    let db_record = store
        .get_project_skill_assignment(&project.id, &skill.id, "cursor")
        .unwrap()
        .expect("assignment should exist in DB");
    assert_eq!(
        db_record.status,
        SyncStatus::Missing,
        "missing status should be persisted to DB"
    );
}

#[test]
fn global_and_project_sync_independent() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let skill_dir = make_skill_dir(tmpdir.path(), "shared-skill");
    let project_dir = tmpdir.path().join("indep-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "shared-skill",
        &skill_dir.to_string_lossy(),
    );

    // Global sync: add to skill_targets table (home dir path)
    let global_target = SkillTargetRecord {
        id: uuid::Uuid::new_v4().to_string(),
        skill_id: skill.id.clone(),
        tool: "claude_code".to_string(),
        target_path: "/home/user/.claude/skills/shared-skill".to_string(),
        mode: SyncMode::Symlink,
        status: SyncStatus::Synced,
        last_error: None,
        synced_at: Some(2000),
    };
    store.upsert_skill_target(&global_target).unwrap();

    // Project sync: assign to project (project dir path)
    project_sync::assign_and_sync(&store, &project, &skill, "claude_code", 2000)
        .expect("assign should succeed");

    // Verify both exist independently
    let global_targets = store.list_skill_targets(&skill.id).unwrap();
    assert_eq!(global_targets.len(), 1, "one global target");
    assert_eq!(
        global_targets[0].target_path,
        "/home/user/.claude/skills/shared-skill"
    );

    let project_assignments = store.list_project_skill_assignments(&project.id).unwrap();
    assert_eq!(project_assignments.len(), 1, "one project assignment");
    assert_eq!(project_assignments[0].project_id, project.id);

    // Remove project assignment -- global should remain
    project_sync::unassign_and_remove_artifacts(&store, &project, &skill, "claude_code")
        .expect("unassign should succeed");

    let global_after = store.list_skill_targets(&skill.id).unwrap();
    assert_eq!(global_after.len(), 1, "global target still exists");

    let project_after = store.list_project_skill_assignments(&project.id).unwrap();
    assert_eq!(project_after.len(), 0, "project assignment removed");
}

#[test]
fn bulk_assign_to_multiple_tools() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let skill_dir = make_skill_dir(tmpdir.path(), "bulk-skill");
    let project_dir = tmpdir.path().join("bulk-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "bulk-skill",
        &skill_dir.to_string_lossy(),
    );

    // Configure two tools for the project
    use crate::core::skill_store::ProjectToolRecord;
    store
        .add_project_tool(&ProjectToolRecord {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project.id.clone(),
            tool: "claude_code".to_string(),
        })
        .unwrap();
    store
        .add_project_tool(&ProjectToolRecord {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project.id.clone(),
            tool: "cursor".to_string(),
        })
        .unwrap();

    let tools = store.list_project_tools(&project.id).unwrap();
    assert_eq!(tools.len(), 2);

    // Simulate bulk assign: iterate tools, call assign_and_sync for each
    let now = 3000i64;
    let mut assigned = Vec::new();
    let mut failed = Vec::new();

    for tool_record in &tools {
        // Skip if already assigned
        if store
            .get_project_skill_assignment(&project.id, &skill.id, &tool_record.tool)
            .unwrap()
            .is_some()
        {
            continue;
        }
        match project_sync::assign_and_sync(&store, &project, &skill, &tool_record.tool, now) {
            Ok(record) => assigned.push(record),
            Err(e) => failed.push(format!("{}: {:#}", tool_record.tool, e)),
        }
    }

    assert_eq!(assigned.len(), 2, "both tools should be assigned");
    assert_eq!(failed.len(), 0, "no failures expected");

    // Verify both targets exist
    let target_claude = project_dir.join(".claude/skills/bulk-skill");
    let target_cursor = project_dir.join(".agents/skills/bulk-skill");
    assert!(target_claude.exists(), "claude target should exist");
    assert!(target_cursor.exists(), "cursor target should exist");
}

#[test]
fn bulk_assign_skips_already_assigned() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let skill_dir = make_skill_dir(tmpdir.path(), "skip-skill");
    let project_dir = tmpdir.path().join("skip-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "skip-skill",
        &skill_dir.to_string_lossy(),
    );

    // Configure one tool
    use crate::core::skill_store::ProjectToolRecord;
    store
        .add_project_tool(&ProjectToolRecord {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project.id.clone(),
            tool: "claude_code".to_string(),
        })
        .unwrap();

    // Pre-assign the skill to claude_code
    project_sync::assign_and_sync(&store, &project, &skill, "claude_code", 2000)
        .expect("initial assign");

    // Now simulate bulk assign -- should skip the already-assigned tool
    let tools = store.list_project_tools(&project.id).unwrap();
    let mut assigned_count = 0;
    for tool_record in &tools {
        if store
            .get_project_skill_assignment(&project.id, &skill.id, &tool_record.tool)
            .unwrap()
            .is_some()
        {
            continue; // Already assigned -- skip
        }
        project_sync::assign_and_sync(&store, &project, &skill, &tool_record.tool, 3000)
            .expect("assign");
        assigned_count += 1;
    }

    assert_eq!(assigned_count, 0, "no new assignments -- already assigned");

    // Verify only one assignment exists in DB
    let assignments = store.list_project_skill_assignments(&project.id).unwrap();
    assert_eq!(assignments.len(), 1, "still only one assignment");
}

#[test]
fn bulk_assign_continues_on_error() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let project_dir = tmpdir.path().join("bulk-err-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    // Create a real skill dir for symlink-capable tools
    let skill_dir = make_skill_dir(tmpdir.path(), "partial-skill");

    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "partial-skill",
        &skill_dir.to_string_lossy(),
    );

    // Configure two tools: claude_code and cursor (both symlink)
    use crate::core::skill_store::ProjectToolRecord;
    store
        .add_project_tool(&ProjectToolRecord {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project.id.clone(),
            tool: "claude_code".to_string(),
        })
        .unwrap();
    store
        .add_project_tool(&ProjectToolRecord {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project.id.clone(),
            tool: "cursor".to_string(),
        })
        .unwrap();

    // Assign claude_code first (will succeed via symlink)
    let r1 = project_sync::assign_and_sync(&store, &project, &skill, "claude_code", 3000)
        .expect("claude_code assign");
    assert_eq!(r1.status, SyncStatus::Synced);

    // Delete source so the second assignment fails
    fs::remove_dir_all(&skill_dir).expect("remove source");

    // Now try to assign cursor -- it fails because the source is gone
    let r2 = project_sync::assign_and_sync(&store, &project, &skill, "cursor", 3000);
    let record = r2.expect("assign_and_sync returns Ok even on sync failure");
    assert_eq!(
        record.status,
        SyncStatus::Error,
        "cursor should fail since source is gone"
    );

    // The point: claude_code succeeded first, cursor failed, both have DB records
    let assignments = store.list_project_skill_assignments(&project.id).unwrap();
    assert_eq!(assignments.len(), 2, "both tools have assignment records");
}

#[test]
fn missing_status_when_target_absent() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let skill_dir = make_skill_dir(tmpdir.path(), "target-gone-skill");
    let project_dir = tmpdir.path().join("target-gone-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "target-gone-skill",
        &skill_dir.to_string_lossy(),
    );

    // claude_code uses symlink mode
    project_sync::assign_and_sync(&store, &project, &skill, "claude_code", 2000)
        .expect("assign should succeed");

    // Verify target exists
    let target = project_dir.join(".claude/skills/target-gone-skill");
    assert!(target.exists(), "target should exist after assign");

    // Manually remove the symlink
    fs::remove_file(&target).ok();
    fs::remove_dir_all(&target).ok();
    assert!(
        !target.exists() && target.symlink_metadata().is_err(),
        "target should be deleted"
    );

    // Should detect missing target and mark status as "missing"
    let assignments = list_reconciled(&store, &project.id);
    assert_eq!(assignments.len(), 1);
    assert_eq!(
        assignments[0].status,
        SyncStatus::Missing,
        "target absent should produce missing status"
    );

    // Verify DB persisted
    let db_record = store
        .get_project_skill_assignment(&project.id, &skill.id, "claude_code")
        .unwrap()
        .expect("assignment should exist");
    assert_eq!(
        db_record.status,
        SyncStatus::Missing,
        "missing status should be persisted to DB"
    );
}

#[test]
fn missing_status_recovers_when_source_restored() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let skill_dir = make_skill_dir(tmpdir.path(), "recover-skill");
    let project_dir = tmpdir.path().join("recover-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "recover-skill",
        &skill_dir.to_string_lossy(),
    );

    let tool = copy_only_tool();
    let record = project_sync::assign_and_sync(&store, &project, &skill, tool, 2000)
        .expect("assign should succeed");
    assert_eq!(record.status, SyncStatus::Synced);

    // Delete source directory -> should become missing
    fs::remove_dir_all(&skill_dir).expect("remove source");

    let assignments = list_reconciled(&store, &project.id);
    assert_eq!(
        assignments[0].status,
        SyncStatus::Missing,
        "should be missing after source deleted"
    );

    // Recreate source with same content
    fs::create_dir_all(&skill_dir).expect("recreate skill dir");
    fs::write(skill_dir.join("SKILL.md"), "# Test Skill\nTest content.").expect("write SKILL.md");

    // Also ensure target copy exists (re-sync to restore it)
    let target = project_dir.join(".agents/skills/recover-skill");
    if !target.exists() {
        // Manually recreate the target copy to simulate recovery
        fs::create_dir_all(&target).expect("recreate target");
        fs::write(target.join("SKILL.md"), "# Test Skill\nTest content.")
            .expect("write target SKILL.md");
    }

    // D-07 litmus test: assignment had DB status "missing", source+target reappeared,
    // function should recalculate to "synced" or "stale" -- NOT "missing"
    let assignments = list_reconciled(&store, &project.id);
    assert_eq!(assignments.len(), 1);
    assert_ne!(
        assignments[0].status,
        SyncStatus::Missing,
        "D-07: recovered assignment must not stay missing"
    );
    // Should be either "synced" or "stale" depending on hash match
    assert!(
        assignments[0].status == SyncStatus::Synced || assignments[0].status == SyncStatus::Stale,
        "recovered assignment should be synced or stale, got: {:?}",
        assignments[0].status
    );
}

#[test]
fn missing_status_source_and_target_both_absent() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let skill_dir = make_skill_dir(tmpdir.path(), "both-gone-skill");
    let project_dir = tmpdir.path().join("both-gone-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "both-gone-skill",
        &skill_dir.to_string_lossy(),
    );

    let tool = copy_only_tool();
    project_sync::assign_and_sync(&store, &project, &skill, tool, 2000)
        .expect("assign should succeed");

    // Delete both source and target
    fs::remove_dir_all(&skill_dir).expect("remove source");
    let target = project_dir.join(".agents/skills/both-gone-skill");
    fs::remove_dir_all(&target).ok();
    assert!(!skill_dir.exists(), "source should be gone");
    assert!(!target.exists(), "target should be gone");

    // Should detect missing
    let assignments = list_reconciled(&store, &project.id);
    assert_eq!(assignments.len(), 1);
    assert_eq!(
        assignments[0].status,
        SyncStatus::Missing,
        "both absent should produce missing status"
    );
}

// ---------------------------------------------------------------------------
// assign_skill_to_tools / assign_skill_to_project_tools — the project-scope
// fan-out engine behind `bulk_assign_skill` and the toggle command.
// ---------------------------------------------------------------------------

use crate::core::errors::SignalError;
use crate::core::project_sync::{
    assign_skill_to_project_tool_unlocked, assign_skill_to_project_tools, assign_skill_to_tools,
    AssignTargetStatus,
};
use crate::core::skill_store::ProjectToolRecord;

fn add_tools(store: &SkillStore, project: &ProjectRecord, tools: &[&str]) {
    for tool in tools {
        store
            .add_project_tool(&ProjectToolRecord {
                id: uuid::Uuid::new_v4().to_string(),
                project_id: project.id.clone(),
                tool: tool.to_string(),
            })
            .unwrap();
    }
}

fn keys(strs: &[&str]) -> Vec<String> {
    strs.iter().map(|s| s.to_string()).collect()
}

#[test]
fn fanout_assigns_every_tool_in_caller_order() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().unwrap();
    let skill_dir = make_skill_dir(tmpdir.path(), "fan-skill");
    let project_dir = tmpdir.path().join("fan-project");
    fs::create_dir_all(&project_dir).unwrap();
    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "fan-skill",
        &skill_dir.to_string_lossy(),
    );

    let outcomes = assign_skill_to_tools(
        &store,
        &project,
        &skill,
        &keys(&["claude_code", "cursor"]),
        3000,
    );

    let tool_keys: Vec<&str> = outcomes.iter().map(|o| o.tool_key.as_str()).collect();
    assert_eq!(tool_keys, vec!["claude_code", "cursor"]);
    for o in &outcomes {
        match &o.status {
            AssignTargetStatus::Assigned { record } => {
                assert_eq!(record.status, SyncStatus::Synced, "tool {}", o.tool_key)
            }
            other => panic!("expected Assigned for {}, got {:?}", o.tool_key, other),
        }
    }
    assert!(project_dir.join(".claude/skills/fan-skill").exists());
    assert!(project_dir.join(".agents/skills/fan-skill").exists());
}

#[test]
fn fanout_reports_already_assigned_as_data_and_does_not_duplicate() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().unwrap();
    let skill_dir = make_skill_dir(tmpdir.path(), "dedupe-skill");
    let project_dir = tmpdir.path().join("dedupe-project");
    fs::create_dir_all(&project_dir).unwrap();
    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "dedupe-skill",
        &skill_dir.to_string_lossy(),
    );
    project_sync::assign_and_sync(&store, &project, &skill, "claude_code", 2000).unwrap();

    let outcomes = assign_skill_to_tools(
        &store,
        &project,
        &skill,
        &keys(&["claude_code", "cursor"]),
        3000,
    );

    assert!(matches!(
        outcomes[0].status,
        AssignTargetStatus::AlreadyAssigned
    ));
    assert!(matches!(
        outcomes[1].status,
        AssignTargetStatus::Assigned { .. }
    ));
    assert_eq!(
        store
            .list_project_skill_assignments(&project.id)
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn fanout_isolates_an_unknown_tool_and_continues() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().unwrap();
    let skill_dir = make_skill_dir(tmpdir.path(), "iso-skill");
    let project_dir = tmpdir.path().join("iso-project");
    fs::create_dir_all(&project_dir).unwrap();
    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "iso-skill",
        &skill_dir.to_string_lossy(),
    );

    let outcomes = assign_skill_to_tools(
        &store,
        &project,
        &skill,
        &keys(&["claude_code", "no-such-tool", "cursor"]),
        3000,
    );

    assert!(matches!(
        outcomes[0].status,
        AssignTargetStatus::Assigned { .. }
    ));
    match &outcomes[1].status {
        AssignTargetStatus::Failed { error } => {
            assert!(format!("{:#}", error).contains("unknown tool"));
        }
        other => panic!("expected Failed, got {:?}", other),
    }
    assert!(matches!(
        outcomes[2].status,
        AssignTargetStatus::Assigned { .. }
    ));
    assert_eq!(
        store
            .list_project_skill_assignments(&project.id)
            .unwrap()
            .len(),
        2,
        "the unknown tool leaves no record behind"
    );
}

#[test]
fn fanout_keeps_sync_failures_inside_the_assignment_record() {
    // A sync failure is not a fan-out failure: the assignment row exists with
    // status "error" (what the UI shows per cell), so the outcome is Assigned.
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().unwrap();
    let skill_dir = make_skill_dir(tmpdir.path(), "gone-skill");
    let project_dir = tmpdir.path().join("gone-project");
    fs::create_dir_all(&project_dir).unwrap();
    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "gone-skill",
        &skill_dir.to_string_lossy(),
    );
    fs::remove_dir_all(&skill_dir).unwrap();

    let outcomes = assign_skill_to_tools(&store, &project, &skill, &keys(&["cursor"]), 3000);

    match &outcomes[0].status {
        AssignTargetStatus::Assigned { record } => assert_eq!(record.status, SyncStatus::Error),
        other => panic!("expected Assigned with error status, got {:?}", other),
    }
}

#[test]
fn project_fanout_uses_persisted_project_tools() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().unwrap();
    let skill_dir = make_skill_dir(tmpdir.path(), "pt-skill");
    let project_dir = tmpdir.path().join("pt-project");
    fs::create_dir_all(&project_dir).unwrap();
    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "pt-skill",
        &skill_dir.to_string_lossy(),
    );
    add_tools(&store, &project, &["claude_code", "pi"]);

    let outcomes =
        assign_skill_to_project_tools(&store, &project.id, &skill.id, 3000).expect("fan-out");

    let mut tool_keys: Vec<&str> = outcomes.iter().map(|o| o.tool_key.as_str()).collect();
    tool_keys.sort();
    assert_eq!(tool_keys, vec!["claude_code", "pi"]);
    assert!(outcomes
        .iter()
        .all(|o| matches!(o.status, AssignTargetStatus::Assigned { .. })));
    // pi's project-scope dir, not its global one
    let pi = crate::core::tool_adapters::adapter_by_key("pi").unwrap();
    assert!(
        project_sync::resolve_project_sync_target(&project_dir, pi, "pt-skill")
            .symlink_metadata()
            .is_ok()
    );
}

#[test]
fn project_fanout_raises_typed_not_found_for_project_and_skill() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().unwrap();
    let skill_dir = make_skill_dir(tmpdir.path(), "nf-skill");
    let project_dir = tmpdir.path().join("nf-project");
    fs::create_dir_all(&project_dir).unwrap();
    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "nf-skill",
        &skill_dir.to_string_lossy(),
    );

    let err = assign_skill_to_project_tools(&store, "missing-project", &skill.id, 1)
        .expect_err("project must be missing");
    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::NotFound {
            kind: "project".to_string(),
            id: "missing-project".to_string(),
        })
    );

    let err = assign_skill_to_project_tools(&store, &project.id, "missing-skill", 1)
        .expect_err("skill must be missing");
    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::NotFound {
            kind: "skill".to_string(),
            id: "missing-skill".to_string(),
        })
    );
}

#[test]
fn single_tool_assign_returns_record_and_raises_assignment_exists_on_repeat() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().unwrap();
    let skill_dir = make_skill_dir(tmpdir.path(), "single-skill");
    let project_dir = tmpdir.path().join("single-project");
    fs::create_dir_all(&project_dir).unwrap();
    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "single-skill",
        &skill_dir.to_string_lossy(),
    );

    let record =
        assign_skill_to_project_tool_unlocked(&store, &project.id, &skill.id, "claude_code", 3000)
            .expect("first assign");
    assert_eq!(record.status, SyncStatus::Synced);
    assert_eq!(record.tool, "claude_code");

    let err =
        assign_skill_to_project_tool_unlocked(&store, &project.id, &skill.id, "claude_code", 3001)
            .expect_err("second assign must fail");
    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::AssignmentExists {
            project: project.id.clone(),
            skill: skill.id.clone(),
            tool: "claude_code".to_string(),
        })
    );

    let err =
        assign_skill_to_project_tool_unlocked(&store, &project.id, &skill.id, "no-such-tool", 3002)
            .expect_err("unknown tool must fail");
    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::UnknownTool {
            tool: "no-such-tool".to_string(),
        })
    );
}

#[test]
fn assign_and_sync_raises_typed_unknown_tool() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");
    let skill_dir = make_skill_dir(tmpdir.path(), "test-skill");
    let project_dir = tmpdir.path().join("my-project");
    fs::create_dir_all(&project_dir).expect("create project dir");
    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "test-skill",
        &skill_dir.to_string_lossy(),
    );

    let err = project_sync::assign_and_sync(&store, &project, &skill, "not-a-tool", 2000)
        .expect_err("unknown tool must fail");
    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::UnknownTool {
            tool: "not-a-tool".to_string(),
        })
    );
}

#[test]
fn resync_project_raises_typed_not_found_for_unknown_project() {
    let (_db_dir, store) = make_store();
    let err = match project_sync::resync_project(&store, "missing", 4000) {
        Ok(_) => panic!("unknown project must fail"),
        Err(err) => err,
    };
    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::NotFound {
            kind: "project".to_string(),
            id: "missing".to_string(),
        })
    );
}

// ---------------------------------------------------------------------------
// toggle_skill_assignment — the backend decides add-vs-remove from its rows
// ---------------------------------------------------------------------------

#[test]
fn toggle_assigns_then_unassigns_from_the_stored_state() {
    use crate::core::project_sync::{
        resolve_project_sync_target, toggle_skill_assignment, ToggleOutcome,
    };

    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().unwrap();
    let skill_dir = make_skill_dir(tmpdir.path(), "toggle-skill");
    let project_dir = tmpdir.path().join("toggle-project");
    fs::create_dir_all(&project_dir).unwrap();
    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "toggle-skill",
        &skill_dir.to_string_lossy(),
    );
    let adapter = crate::core::tool_adapters::adapter_by_key("claude_code").expect("adapter");
    let target = resolve_project_sync_target(&project_dir, adapter, "toggle-skill");

    let first = toggle_skill_assignment(&store, &project.id, &skill.id, "claude_code", 4000)
        .expect("first toggle");
    assert_eq!(first, ToggleOutcome::Assigned);
    assert!(target.symlink_metadata().is_ok(), "artifact materialised");
    assert_eq!(
        store
            .list_project_skill_assignments(&project.id)
            .unwrap()
            .len(),
        1
    );

    let second = toggle_skill_assignment(&store, &project.id, &skill.id, "claude_code", 4001)
        .expect("second toggle");
    assert_eq!(second, ToggleOutcome::Unassigned);
    assert!(
        target.symlink_metadata().is_err(),
        "artifact removed: {:?}",
        target
    );
    assert!(store
        .list_project_skill_assignments(&project.id)
        .unwrap()
        .is_empty());
}

#[test]
fn toggle_raises_typed_not_found_for_unknown_project() {
    use crate::core::project_sync::toggle_skill_assignment;

    let (_db_dir, store) = make_store();
    let err = toggle_skill_assignment(&store, "missing", "skill", "claude_code", 1)
        .expect_err("must fail");
    assert_eq!(
        err.downcast_ref::<crate::core::errors::SignalError>(),
        Some(&crate::core::errors::SignalError::NotFound {
            kind: "project".to_string(),
            id: "missing".to_string(),
        })
    );
}

// ---------------------------------------------------------------------------
// One naming rule: the artifact is located by the stored assignment name
// ---------------------------------------------------------------------------

/// Assign a copy, then rename the Managed skill through the store (finalize
/// never renames today, so there is no entry point). Returns the artifact
/// path the assignment was materialised under.
fn assign_copy_then_rename_skill(
    store: &SkillStore,
    tmp: &Path,
) -> (ProjectRecord, SkillRecord, std::path::PathBuf) {
    let skill_dir = make_skill_dir(tmp, "named-skill");
    let project_dir = tmp.join("rename-project");
    fs::create_dir_all(&project_dir).expect("create project dir");
    let (project, skill) = register_project_and_skill(
        store,
        &project_dir.to_string_lossy(),
        "named-skill",
        &skill_dir.to_string_lossy(),
    );
    let tool = copy_only_tool();
    let record = project_sync::assign_and_sync(store, &project, &skill, tool, 2000)
        .expect("assign should succeed");
    assert_eq!(record.status, SyncStatus::Synced);
    let adapter = crate::core::tool_adapters::adapter_by_key(tool).expect("adapter");
    let target = project_sync::resolve_project_sync_target(&project_dir, adapter, "named-skill");
    assert!(target.join("SKILL.md").exists(), "assignment materialised");

    let mut renamed = skill.clone();
    renamed.name = "named-skill-v2".to_string();
    store.upsert_skill(&renamed).expect("rename skill");
    (project, renamed, target)
}

#[test]
fn resync_rematerialises_the_artifact_under_its_stored_name_after_a_rename() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");
    let (project, skill, target) = assign_copy_then_rename_skill(&store, tmpdir.path());
    fs::write(
        Path::new(&skill.central_path).join("extra.txt"),
        "new content",
    )
    .expect("change the central copy");

    let summary = project_sync::resync_project(&store, &project.id, 3000).expect("resync");

    assert_eq!(summary.synced, 1, "errors: {:?}", summary.errors);
    assert!(
        target.join("extra.txt").exists(),
        "the stored-name artifact receives the new bytes"
    );
    let live_name_path = target.with_file_name("named-skill-v2");
    assert!(
        live_name_path.symlink_metadata().is_err(),
        "no second artifact under the live name: {}",
        live_name_path.display()
    );
}

#[test]
fn reconcile_observes_the_artifact_under_its_stored_name_after_a_rename() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");
    let (project, _skill, _target) = assign_copy_then_rename_skill(&store, tmpdir.path());

    let assignments = list_reconciled(&store, &project.id);

    assert_eq!(assignments.len(), 1);
    assert_eq!(
        assignments[0].status,
        SyncStatus::Synced,
        "the artifact is still there under the name it was materialised with"
    );
    assert_eq!(assignments[0].skill_name, "named-skill");
}
