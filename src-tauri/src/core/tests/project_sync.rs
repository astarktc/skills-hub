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

    assert_eq!(record.status, "synced");
    assert_eq!(record.mode, "symlink");
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

    // cursor tool forces copy mode
    let result = project_sync::assign_and_sync(&store, &project, &skill, "cursor", 2000);
    let record = result.expect("assign_and_sync should succeed");

    assert_eq!(record.status, "synced");
    assert_eq!(record.mode, "copy");
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
        "target should NOT be a symlink for cursor"
    );
}

#[test]
fn assign_records_error_on_sync_failure() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let project_dir = tmpdir.path().join("err-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    // Use a non-existent source path for the skill.
    // Use "cursor" tool which forces copy mode -- copy will fail on non-existent source.
    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "missing-skill",
        "/nonexistent/path/to/skill",
    );

    let result = project_sync::assign_and_sync(&store, &project, &skill, "cursor", 2000);
    let record = result.expect("assign_and_sync should return Ok even on sync failure");

    assert_eq!(record.status, "error");
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
    project_sync::unassign_and_cleanup(&store, &project, &skill, "claude_code")
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
    project_sync::unassign_and_cleanup(&store, &project, &skill, "claude_code")
        .expect("unassign should succeed even when target is gone");

    let assignment = store
        .get_project_skill_assignment(&project.id, &skill.id, "claude_code")
        .unwrap();
    assert!(assignment.is_none(), "DB record should be deleted");
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

    // Assign both -- use cursor (copy mode) so missing source fails
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
    assert_eq!(bad_assignment.status, "error");

    // Verify the successful assignment has synced status
    let ok_assignment = store
        .get_project_skill_assignment(&project.id, &skill1.id, "cursor")
        .unwrap()
        .expect("ok assignment should exist");
    assert_eq!(ok_assignment.status, "synced");
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

    // cursor forces copy mode, which stores content_hash
    project_sync::assign_and_sync(&store, &project, &skill, "cursor", 2000)
        .expect("assign should succeed");

    // Verify initial status is synced
    let before = store
        .get_project_skill_assignment(&project.id, &skill.id, "cursor")
        .unwrap()
        .expect("assignment exists");
    assert_eq!(before.status, "synced");
    assert!(before.content_hash.is_some());

    // Modify source to change the hash
    fs::write(skill_dir.join("new-file.txt"), "changed content").expect("write new file");

    // list_assignments_with_staleness should detect the change
    let assignments = project_sync::list_assignments_with_staleness(&store, &project.id)
        .expect("list should succeed");
    assert_eq!(assignments.len(), 1);
    assert_eq!(assignments[0].status, "stale");

    // DB should also be updated to stale
    let after = store
        .get_project_skill_assignment(&project.id, &skill.id, "cursor")
        .unwrap()
        .expect("assignment exists");
    assert_eq!(after.status, "stale");
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
    let assignments = project_sync::list_assignments_with_staleness(&store, &project.id)
        .expect("list should succeed");
    assert_eq!(assignments.len(), 1);
    assert_eq!(
        assignments[0].status, "synced",
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

    // cursor forces copy mode
    project_sync::assign_and_sync(&store, &project, &skill, "cursor", 2000)
        .expect("assign should succeed");

    // Delete source directory entirely
    fs::remove_dir_all(&skill_dir).expect("remove source");

    // Should detect missing source and mark status as "missing"
    let assignments = project_sync::list_assignments_with_staleness(&store, &project.id)
        .expect("list should not crash");
    assert_eq!(assignments.len(), 1);
    assert_eq!(
        assignments[0].status, "missing",
        "source absent should produce missing status"
    );

    // Verify DB persisted the missing status
    let db_record = store
        .get_project_skill_assignment(&project.id, &skill.id, "cursor")
        .unwrap()
        .expect("assignment should exist in DB");
    assert_eq!(
        db_record.status, "missing",
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
        mode: "symlink".to_string(),
        status: "synced".to_string(),
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
    project_sync::unassign_and_cleanup(&store, &project, &skill, "claude_code")
        .expect("unassign should succeed");

    let global_after = store.list_skill_targets(&skill.id).unwrap();
    assert_eq!(global_after.len(), 1, "global target still exists");

    let project_after = store.list_project_skill_assignments(&project.id).unwrap();
    assert_eq!(project_after.len(), 0, "project assignment removed");
}

#[test]
fn sync_serialization() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    // Set up project and skill for both threads
    let skill_dir = make_skill_dir(tmpdir.path(), "serial-skill");
    let project_dir = tmpdir.path().join("serial-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let (project, skill) = register_project_and_skill(
        &store,
        &project_dir.to_string_lossy(),
        "serial-skill",
        &skill_dir.to_string_lossy(),
    );

    // Assign first so resync has something to work with
    project_sync::assign_and_sync(&store, &project, &skill, "claude_code", 2000)
        .expect("assign should succeed");

    // Shared mutex (same type as SyncMutex.0)
    let mutex = Arc::new(std::sync::Mutex::new(()));
    // Counter tracking concurrent executions -- must never exceed 1
    let concurrent = Arc::new(AtomicU32::new(0));

    let store1 = store.clone();
    let pid1 = project.id.clone();
    let mutex1 = mutex.clone();
    let concurrent1 = concurrent.clone();

    let store2 = store.clone();
    let pid2 = project.id.clone();
    let mutex2 = mutex.clone();
    let concurrent2 = concurrent.clone();

    let t1 = std::thread::spawn(move || {
        let _lock = mutex1.lock().unwrap_or_else(|e| e.into_inner());
        let prev = concurrent1.fetch_add(1, Ordering::SeqCst);
        assert_eq!(prev, 0, "thread 1: no concurrent access allowed");
        // Do sync work
        let _ = project_sync::resync_project(&store1, &pid1, 4000);
        concurrent1.fetch_sub(1, Ordering::SeqCst);
    });

    let t2 = std::thread::spawn(move || {
        let _lock = mutex2.lock().unwrap_or_else(|e| e.into_inner());
        let prev = concurrent2.fetch_add(1, Ordering::SeqCst);
        assert_eq!(prev, 0, "thread 2: no concurrent access allowed");
        // Do sync work
        let _ = project_sync::resync_project(&store2, &pid2, 5000);
        concurrent2.fetch_sub(1, Ordering::SeqCst);
    });

    t1.join().expect("thread 1 should complete");
    t2.join().expect("thread 2 should complete");

    // Final counter should be 0
    assert_eq!(concurrent.load(Ordering::SeqCst), 0, "all done");
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

    // Configure two tools: claude_code (symlink, will work) and cursor (copy, will work)
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
    assert_eq!(r1.status, "synced");

    // Delete source to cause cursor (copy mode) to fail on resync
    fs::remove_dir_all(&skill_dir).expect("remove source");

    // Now try to assign cursor -- it will fail because source is gone and cursor uses copy mode
    let r2 = project_sync::assign_and_sync(&store, &project, &skill, "cursor", 3000);
    let record = r2.expect("assign_and_sync returns Ok even on sync failure");
    assert_eq!(
        record.status, "error",
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
    let assignments = project_sync::list_assignments_with_staleness(&store, &project.id)
        .expect("list should succeed");
    assert_eq!(assignments.len(), 1);
    assert_eq!(
        assignments[0].status, "missing",
        "target absent should produce missing status"
    );

    // Verify DB persisted
    let db_record = store
        .get_project_skill_assignment(&project.id, &skill.id, "claude_code")
        .unwrap()
        .expect("assignment should exist");
    assert_eq!(
        db_record.status, "missing",
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

    // cursor forces copy mode
    let record = project_sync::assign_and_sync(&store, &project, &skill, "cursor", 2000)
        .expect("assign should succeed");
    assert_eq!(record.status, "synced");

    // Delete source directory -> should become missing
    fs::remove_dir_all(&skill_dir).expect("remove source");

    let assignments = project_sync::list_assignments_with_staleness(&store, &project.id)
        .expect("list should succeed");
    assert_eq!(
        assignments[0].status, "missing",
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
    let assignments = project_sync::list_assignments_with_staleness(&store, &project.id)
        .expect("list should succeed after recovery");
    assert_eq!(assignments.len(), 1);
    assert_ne!(
        assignments[0].status, "missing",
        "D-07: recovered assignment must not stay missing"
    );
    // Should be either "synced" or "stale" depending on hash match
    assert!(
        assignments[0].status == "synced" || assignments[0].status == "stale",
        "recovered assignment should be synced or stale, got: {}",
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

    // cursor forces copy mode
    project_sync::assign_and_sync(&store, &project, &skill, "cursor", 2000)
        .expect("assign should succeed");

    // Delete both source and target
    fs::remove_dir_all(&skill_dir).expect("remove source");
    let target = project_dir.join(".agents/skills/both-gone-skill");
    fs::remove_dir_all(&target).ok();
    assert!(!skill_dir.exists(), "source should be gone");
    assert!(!target.exists(), "target should be gone");

    // Should detect missing
    let assignments = project_sync::list_assignments_with_staleness(&store, &project.id)
        .expect("list should succeed");
    assert_eq!(assignments.len(), 1);
    assert_eq!(
        assignments[0].status, "missing",
        "both absent should produce missing status"
    );
}
