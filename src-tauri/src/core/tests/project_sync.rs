use std::fs;
use std::path::Path;

use crate::core::project_sync;
use crate::core::skill_store::{ProjectRecord, SkillRecord, SkillStore};

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
    let target = project_dir.join(".cursor/skills/copy-skill");
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
