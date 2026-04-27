use std::fs;

use crate::core::project_ops;
use crate::core::project_sync;
use crate::core::skill_store::{
    ProjectRecord, ProjectSkillAssignmentRecord, ProjectToolRecord, SkillRecord, SkillStore,
};

fn make_store() -> (tempfile::TempDir, SkillStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    (dir, store)
}

fn test_expand_home(input: &str) -> anyhow::Result<std::path::PathBuf> {
    if input.is_empty() {
        anyhow::bail!("path is empty");
    }
    Ok(std::path::PathBuf::from(input))
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn make_skill(store: &SkillStore, name: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_ms();
    let record = SkillRecord {
        id: id.clone(),
        name: name.to_string(),
        description: None,
        source_type: "local".to_string(),
        source_ref: None,
        source_subpath: None,
        source_revision: None,
        central_path: format!("/tmp/central/{}", name),
        content_hash: None,
        created_at: now,
        updated_at: now,
        last_sync_at: None,
        last_seen_at: now,
        status: "ok".to_string(),
    };
    store.upsert_skill(&record).expect("upsert_skill");
    id
}

#[test]
fn register_rejects_non_dir() {
    let (_dir, store) = make_store();
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    let path = tmp.path().to_string_lossy().to_string();
    let result = project_ops::register_project_path(&store, &path, now_ms(), test_expand_home);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("not a directory"),
        "expected 'not a directory', got: {}",
        err
    );
}

#[test]
fn register_rejects_empty_path() {
    let (_dir, store) = make_store();
    let result = project_ops::register_project_path(&store, "", now_ms(), test_expand_home);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("empty"),
        "expected 'empty' in error, got: {}",
        err
    );
}

#[test]
fn register_stores_canonical_path() {
    let tmpdir = tempfile::tempdir().expect("tempdir");
    let (_db_dir, store) = make_store();
    let path = tmpdir.path().to_string_lossy().to_string();
    let dto =
        project_ops::register_project_path(&store, &path, now_ms(), test_expand_home).unwrap();
    let canonical = std::fs::canonicalize(tmpdir.path())
        .unwrap()
        .to_string_lossy()
        .to_string();
    assert_eq!(dto.path, canonical);
}

#[test]
fn register_rejects_duplicate() {
    let tmpdir = tempfile::tempdir().expect("tempdir");
    let (_db_dir, store) = make_store();
    let path = tmpdir.path().to_string_lossy().to_string();
    project_ops::register_project_path(&store, &path, now_ms(), test_expand_home).unwrap();
    let result = project_ops::register_project_path(&store, &path, now_ms(), test_expand_home);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.starts_with("DUPLICATE_PROJECT|"),
        "expected 'DUPLICATE_PROJECT|' prefix, got: {}",
        err
    );
}

#[test]
fn project_name_from_path_derives_basename() {
    assert_eq!(
        project_ops::project_name_from_path("/tmp/my-cool-project"),
        "my-cool-project"
    );
    // Root path edge case
    let root_name = project_ops::project_name_from_path("/");
    assert!(!root_name.is_empty(), "root path should return non-empty");
}

#[test]
fn to_project_dto_includes_sync_status() {
    let (_dir, store) = make_store();
    let now = now_ms();
    let project_id = uuid::Uuid::new_v4().to_string();
    let project = ProjectRecord {
        id: project_id.clone(),
        path: "/tmp/test-project".to_string(),
        created_at: now,
        updated_at: now,
    };
    store.register_project(&project).unwrap();

    let skill_id = make_skill(&store, "test-skill");

    let assignment = ProjectSkillAssignmentRecord {
        id: uuid::Uuid::new_v4().to_string(),
        project_id: project_id.clone(),
        skill_id,
        skill_name: "test-skill".to_string(),
        tool: "claude_code".to_string(),
        mode: "symlink".to_string(),
        status: "error".to_string(),
        last_error: Some("test error".to_string()),
        synced_at: None,
        content_hash: None,
        created_at: now,
    };
    store.add_project_skill_assignment(&assignment).unwrap();

    let dto = project_ops::to_project_dto(&project, &store).unwrap();
    assert_eq!(dto.sync_status, "error");
}

#[test]
fn list_project_dtos_returns_counts() {
    let (_dir, store) = make_store();
    let now = now_ms();
    let project_id = uuid::Uuid::new_v4().to_string();
    let project = ProjectRecord {
        id: project_id.clone(),
        path: "/tmp/test-project-counts".to_string(),
        created_at: now,
        updated_at: now,
    };
    store.register_project(&project).unwrap();

    // Add 2 tools
    store
        .add_project_tool(&ProjectToolRecord {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project_id.clone(),
            tool: "claude_code".to_string(),
        })
        .unwrap();
    store
        .add_project_tool(&ProjectToolRecord {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project_id.clone(),
            tool: "cursor".to_string(),
        })
        .unwrap();

    // Add 1 assignment (needs a skill)
    let skill_id = make_skill(&store, "test-skill-counts");
    store
        .add_project_skill_assignment(&ProjectSkillAssignmentRecord {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project_id.clone(),
            skill_id,
            skill_name: "test-skill-counts".to_string(),
            tool: "claude_code".to_string(),
            mode: "symlink".to_string(),
            status: "pending".to_string(),
            last_error: None,
            synced_at: None,
            content_hash: None,
            created_at: now,
        })
        .unwrap();

    let dtos = project_ops::list_project_dtos(&store).unwrap();
    assert_eq!(dtos.len(), 1);
    let dto = &dtos[0];
    assert_eq!(dto.tool_count, 2);
    assert_eq!(dto.assignment_count, 1);
}

fn make_skill_dir(base: &std::path::Path, name: &str) -> std::path::PathBuf {
    let dir = base.join(name);
    fs::create_dir_all(&dir).expect("create skill dir");
    fs::write(dir.join("SKILL.md"), "# Test Skill\nTest content.").expect("write SKILL.md");
    dir
}

fn register_project_and_skill_at(
    store: &SkillStore,
    project_path: &str,
    skill_name: &str,
    skill_central_path: &str,
) -> (ProjectRecord, SkillRecord) {
    let now = now_ms();
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
fn remove_tool_with_cleanup_deletes_assignments_and_artifacts() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let skill1_dir = make_skill_dir(tmpdir.path(), "rtc-skill-1");
    let skill2_dir = make_skill_dir(tmpdir.path(), "rtc-skill-2");
    let project_dir = tmpdir.path().join("rtc-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let (project, skill1) = register_project_and_skill_at(
        &store,
        &project_dir.to_string_lossy(),
        "rtc-skill-1",
        &skill1_dir.to_string_lossy(),
    );

    let skill2 = SkillRecord {
        id: uuid::Uuid::new_v4().to_string(),
        name: "rtc-skill-2".to_string(),
        description: None,
        source_type: "local".to_string(),
        source_ref: None,
        source_subpath: None,
        source_revision: None,
        central_path: skill2_dir.to_string_lossy().to_string(),
        content_hash: None,
        created_at: now_ms(),
        updated_at: now_ms(),
        last_sync_at: None,
        last_seen_at: now_ms(),
        status: "ok".to_string(),
    };
    store.upsert_skill(&skill2).unwrap();

    // Add tool column
    store
        .add_project_tool(&ProjectToolRecord {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project.id.clone(),
            tool: "claude_code".to_string(),
        })
        .unwrap();

    // Assign both skills to claude_code
    project_sync::assign_and_sync(&store, &project, &skill1, "claude_code", now_ms())
        .expect("assign skill1");
    project_sync::assign_and_sync(&store, &project, &skill2, "claude_code", now_ms())
        .expect("assign skill2");

    // Verify symlinks exist
    let target1 = project_dir.join(".claude/skills/rtc-skill-1");
    let target2 = project_dir.join(".claude/skills/rtc-skill-2");
    assert!(
        target1.exists(),
        "skill1 target should exist before removal"
    );
    assert!(
        target2.exists(),
        "skill2 target should exist before removal"
    );

    // Act: remove the tool
    project_ops::remove_tool_with_cleanup(&store, &project.id, "claude_code")
        .expect("remove_tool_with_cleanup should succeed");

    // Assert: symlinks removed
    assert!(
        !target1.exists() && target1.symlink_metadata().is_err(),
        "skill1 target should be removed"
    );
    assert!(
        !target2.exists() && target2.symlink_metadata().is_err(),
        "skill2 target should be removed"
    );

    // Assert: assignment DB records deleted
    let assignments = store
        .list_project_skill_assignments_for_project_tool(&project.id, "claude_code")
        .unwrap();
    assert_eq!(assignments.len(), 0, "all assignments should be deleted");

    // Assert: tool DB row deleted
    let tools = store.list_project_tools(&project.id).unwrap();
    assert!(
        tools.iter().all(|t| t.tool != "claude_code"),
        "claude_code tool row should be deleted"
    );
}

#[test]
fn remove_tool_with_cleanup_leaves_other_tools_intact() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let skill_dir = make_skill_dir(tmpdir.path(), "multi-tool-skill");
    let project_dir = tmpdir.path().join("multi-tool-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let (project, skill) = register_project_and_skill_at(
        &store,
        &project_dir.to_string_lossy(),
        "multi-tool-skill",
        &skill_dir.to_string_lossy(),
    );

    // Add both tools
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

    // Assign skill to both tools
    project_sync::assign_and_sync(&store, &project, &skill, "claude_code", now_ms())
        .expect("assign claude_code");
    project_sync::assign_and_sync(&store, &project, &skill, "cursor", now_ms())
        .expect("assign cursor");

    // Verify both targets exist
    let claude_target = project_dir.join(".claude/skills/multi-tool-skill");
    let cursor_target = project_dir.join(".agents/skills/multi-tool-skill");
    assert!(claude_target.exists(), "claude target should exist");
    assert!(cursor_target.exists(), "cursor target should exist");

    // Act: remove only claude_code
    project_ops::remove_tool_with_cleanup(&store, &project.id, "claude_code")
        .expect("remove_tool_with_cleanup should succeed");

    // Assert: claude_code target removed
    assert!(
        !claude_target.exists() && claude_target.symlink_metadata().is_err(),
        "claude target should be removed"
    );

    // Assert: cursor target still exists
    assert!(cursor_target.exists(), "cursor target should still exist");

    // Assert: cursor assignment still in DB
    let cursor_assignment = store
        .get_project_skill_assignment(&project.id, &skill.id, "cursor")
        .unwrap();
    assert!(
        cursor_assignment.is_some(),
        "cursor assignment should still exist"
    );

    // Assert: claude_code assignment gone
    let claude_assignment = store
        .get_project_skill_assignment(&project.id, &skill.id, "claude_code")
        .unwrap();
    assert!(
        claude_assignment.is_none(),
        "claude_code assignment should be deleted"
    );
}

#[test]
fn remove_tool_with_cleanup_handles_missing_skill_gracefully() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    let skill_dir = make_skill_dir(tmpdir.path(), "orphan-skill");
    let project_dir = tmpdir.path().join("orphan-project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let (project, skill) = register_project_and_skill_at(
        &store,
        &project_dir.to_string_lossy(),
        "orphan-skill",
        &skill_dir.to_string_lossy(),
    );

    // Add tool and assign
    store
        .add_project_tool(&ProjectToolRecord {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project.id.clone(),
            tool: "claude_code".to_string(),
        })
        .unwrap();

    project_sync::assign_and_sync(&store, &project, &skill, "claude_code", now_ms())
        .expect("assign");

    // Delete the skill record from DB to simulate orphan
    store.delete_skill(&skill.id).unwrap();

    // Verify skill is gone
    assert!(
        store.get_skill_by_id(&skill.id).unwrap().is_none(),
        "skill should be deleted from DB"
    );

    // Act: should not panic
    project_ops::remove_tool_with_cleanup(&store, &project.id, "claude_code")
        .expect("remove_tool_with_cleanup should succeed even with orphaned skill");

    // Assert: tool row deleted
    let tools = store.list_project_tools(&project.id).unwrap();
    assert!(
        tools.iter().all(|t| t.tool != "claude_code"),
        "tool row should be deleted"
    );

    // Assert: assignment DB record cleaned up
    let assignments = store
        .list_project_skill_assignments_for_project_tool(&project.id, "claude_code")
        .unwrap();
    assert_eq!(
        assignments.len(),
        0,
        "assignment should be cleaned up even with missing skill"
    );
}
