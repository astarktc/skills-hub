use crate::core::sync_status::{ProjectSyncStatus, SyncMode, SyncStatus};
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
    let result = project_ops::register_project_path(&store, _dir.path(), &path, now_ms());
    let err = result.expect_err("must fail");
    assert!(
        matches!(
            err.downcast_ref::<crate::core::errors::SignalError>(),
            Some(crate::core::errors::SignalError::InvalidPath { reason, .. }) if reason == "not_a_directory"
        ),
        "expected SignalError::InvalidPath{{not_a_directory}}, got: {:#}",
        err
    );
}

#[test]
fn register_rejects_empty_path() {
    let (_dir, store) = make_store();
    let result = project_ops::register_project_path(&store, _dir.path(), "", now_ms());
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
    let dto = project_ops::register_project_path(&store, _db_dir.path(), &path, now_ms()).unwrap();
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
    project_ops::register_project_path(&store, _db_dir.path(), &path, now_ms()).unwrap();
    let result = project_ops::register_project_path(&store, _db_dir.path(), &path, now_ms());
    let err = result.expect_err("duplicate registration must fail");
    assert!(
        matches!(
            err.downcast_ref::<crate::core::errors::SignalError>(),
            Some(crate::core::errors::SignalError::DuplicateProject { .. })
        ),
        "expected SignalError::DuplicateProject, got: {:#}",
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
        mode: SyncMode::Symlink,
        status: SyncStatus::Error,
        last_error: Some("test error".to_string()),
        synced_at: None,
        content_hash: None,
        created_at: now,
    };
    store.add_project_skill_assignment(&assignment).unwrap();

    let dto = project_ops::to_project_dto(&project, &store).unwrap();
    assert_eq!(dto.sync_status, ProjectSyncStatus::Error);
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
            mode: SyncMode::Symlink,
            status: SyncStatus::Pending,
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

// ---------------------------------------------------------------------------
// Regression: cleanup must use the project-scope path family, not the global
// one. Pi's mappings diverge (`.pi/agent/skills` globally vs `.pi/skills` in a
// project), so a cleanup that joins the global dir onto the project path
// silently leaves the synced skill dir on disk.
// ---------------------------------------------------------------------------

fn pi_adapter() -> &'static crate::core::tool_adapters::ToolAdapter {
    let adapter = crate::core::tool_adapters::adapter_by_key("pi").expect("pi adapter");
    assert_ne!(
        adapter.relative_skills_dir, adapter.project_relative_skills_dir,
        "test precondition: pi's global and project mappings must diverge"
    );
    adapter
}

/// Delete the skill row WITHOUT the FK cascade, leaving the assignment row
/// behind exactly like a legacy database would.
fn orphan_skill_row(store: &SkillStore, skill_id: &str) {
    let conn = rusqlite::Connection::open(store.db_path()).expect("open db");
    conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
    conn.execute(
        "DELETE FROM skills WHERE id = ?1",
        rusqlite::params![skill_id],
    )
    .unwrap();
}

fn setup_pi_assignment(
    tmpdir: &std::path::Path,
    store: &SkillStore,
    name: &str,
) -> (ProjectRecord, SkillRecord, std::path::PathBuf) {
    let skill_dir = make_skill_dir(tmpdir, name);
    let project_dir = tmpdir.join(format!("{name}-project"));
    fs::create_dir_all(&project_dir).expect("create project dir");
    let (project, skill) = register_project_and_skill_at(
        store,
        &project_dir.to_string_lossy(),
        name,
        &skill_dir.to_string_lossy(),
    );
    store
        .add_project_tool(&ProjectToolRecord {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project.id.clone(),
            tool: "pi".to_string(),
        })
        .unwrap();
    project_sync::assign_and_sync(store, &project, &skill, "pi", now_ms()).expect("assign");

    let target = project_sync::resolve_project_sync_target(&project_dir, pi_adapter(), name);
    assert!(
        target.symlink_metadata().is_ok(),
        "precondition: project-scope target should exist at {:?}",
        target
    );
    (project, skill, target)
}

#[test]
fn remove_project_with_cleanup_removes_project_scope_target_for_divergent_tool() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");
    let (project, _skill, target) = setup_pi_assignment(tmpdir.path(), &store, "rpc-pi-skill");

    project_ops::remove_project_with_cleanup(&store, &project.id).expect("remove project");

    assert!(
        target.symlink_metadata().is_err(),
        "project-scope target should be removed: {:?}",
        target
    );
    assert!(store.get_project_by_id(&project.id).unwrap().is_none());
}

#[test]
fn remove_project_with_cleanup_orphan_branch_removes_project_scope_target() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");
    let (project, skill, target) = setup_pi_assignment(tmpdir.path(), &store, "rpc-pi-orphan");

    orphan_skill_row(&store, &skill.id);
    assert!(store.get_skill_by_id(&skill.id).unwrap().is_none());
    assert_eq!(
        store
            .list_project_skill_assignments(&project.id)
            .unwrap()
            .len(),
        1,
        "precondition: orphaned assignment row must survive"
    );

    project_ops::remove_project_with_cleanup(&store, &project.id).expect("remove project");

    assert!(
        target.symlink_metadata().is_err(),
        "orphan branch should remove project-scope target: {:?}",
        target
    );
}

#[test]
fn remove_tool_with_cleanup_removes_project_scope_target_for_divergent_tool() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");
    let (project, _skill, target) = setup_pi_assignment(tmpdir.path(), &store, "rtc-pi-skill");

    project_ops::remove_tool_with_cleanup(&store, &project.id, "pi").expect("remove tool");

    assert!(
        target.symlink_metadata().is_err(),
        "project-scope target should be removed: {:?}",
        target
    );
}

#[test]
fn remove_tool_with_cleanup_orphan_branch_removes_project_scope_target() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");
    let (project, skill, target) = setup_pi_assignment(tmpdir.path(), &store, "rtc-pi-orphan");

    orphan_skill_row(&store, &skill.id);
    assert_eq!(
        store
            .list_project_skill_assignments_for_project_tool(&project.id, "pi")
            .unwrap()
            .len(),
        1,
        "precondition: orphaned assignment row must survive"
    );

    project_ops::remove_tool_with_cleanup(&store, &project.id, "pi").expect("remove tool");

    assert!(
        target.symlink_metadata().is_err(),
        "orphan branch should remove project-scope target: {:?}",
        target
    );
    assert!(store
        .list_project_skill_assignments_for_project_tool(&project.id, "pi")
        .unwrap()
        .is_empty());
}

/// The orphan case as a plan/report assertion: the assignment row names a
/// skill that no longer exists, so the plan locates the artifact from the
/// row's own `skill_name`, removes it, and settles the row.
#[test]
fn remove_tool_with_cleanup_plans_orphan_rows_from_their_stored_skill_name() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");
    let (project, skill, target) = setup_pi_assignment(tmpdir.path(), &store, "rtc-pi-report");

    orphan_skill_row(&store, &skill.id);

    let planned = crate::core::artifact_removal::plan(
        &store,
        &crate::core::artifact_removal::RemovalScope::ProjectTool {
            project_id: project.id.clone(),
            tool_key: "pi".to_string(),
        },
    )
    .expect("plan");
    assert_eq!(planned.targets.len(), 1, "one orphan row, one artifact");
    assert_eq!(
        planned.targets[0].path, target,
        "planned from the row's stored skill name"
    );

    let report =
        project_ops::remove_tool_with_cleanup(&store, &project.id, "pi").expect("remove tool");

    assert!(report.failures().is_empty());
    assert_eq!(report.removed_rows(), 1);
    assert!(target.symlink_metadata().is_err(), "artifact removed");
    assert!(store
        .list_project_skill_assignments_for_project_tool(&project.id, "pi")
        .unwrap()
        .is_empty());
    assert!(store
        .list_project_tools(&project.id)
        .unwrap()
        .iter()
        .all(|t| t.tool != "pi"));
}

/// Make the *parent* of `target` read-only so a symlink at `target` cannot be
/// unlinked. `false` means permissions are not enforced (root) — skip.
#[cfg(unix)]
fn lock_parent(target: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let parent = target.parent().expect("target has a parent");
    fs::set_permissions(parent, fs::Permissions::from_mode(0o555)).unwrap();
    if fs::remove_file(target).is_ok() {
        fs::set_permissions(parent, fs::Permissions::from_mode(0o755)).unwrap();
        return false;
    }
    true
}

#[cfg(unix)]
fn unlock_parent(target: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let parent = target.parent().expect("target has a parent");
    fs::set_permissions(parent, fs::Permissions::from_mode(0o755)).unwrap();
}

/// A tool whose artifact could not be removed keeps its project-tool row, so
/// the operator can retry the same removal against the same plan.
#[cfg(unix)]
#[test]
fn remove_tool_with_cleanup_keeps_the_tool_row_when_an_artifact_stays() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");
    let (project, skill, target) = setup_pi_assignment(tmpdir.path(), &store, "rtc-pi-stuck");
    if !lock_parent(&target) {
        return;
    }

    let report = project_ops::remove_tool_with_cleanup(&store, &project.id, "pi")
        .expect("report, not error");
    unlock_parent(&target);

    assert_eq!(report.failures().len(), 1);
    assert_eq!(report.failed_rows(), 1);
    let assignment = store
        .get_project_skill_assignment(&project.id, &skill.id, "pi")
        .unwrap()
        .expect("row kept");
    assert_eq!(assignment.status, SyncStatus::Error);
    assert!(
        store
            .list_project_tools(&project.id)
            .unwrap()
            .iter()
            .any(|t| t.tool == "pi"),
        "the project tool row is kept for a retry"
    );
}

/// Continue semantics: a stuck tool does not stop the rest of the batch — the
/// added tool is persisted and the failures are raised once, at the end.
#[cfg(unix)]
#[test]
fn configure_tools_applies_the_rest_then_raises_the_removal_failures() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");
    let (project, _skill, target) = setup_pi_assignment(tmpdir.path(), &store, "cfg-pi-stuck");
    if !lock_parent(&target) {
        return;
    }

    let err = project_ops::configure_project_tools(
        &store,
        &project.id,
        &["claude_code".to_string()],
        None,
    )
    .expect_err("the stuck artifact is reported");
    unlock_parent(&target);

    match err.downcast_ref::<SignalError>() {
        Some(SignalError::DeleteCleanupFailed { failures }) => {
            assert_eq!(failures.len(), 1);
            assert!(failures[0].starts_with(&format!("{}: ", target.display())));
        }
        other => panic!("expected DeleteCleanupFailed, got {:?}", other),
    }

    let tools: Vec<String> = store
        .list_project_tools(&project.id)
        .unwrap()
        .into_iter()
        .map(|t| t.tool)
        .collect();
    assert!(
        tools.contains(&"claude_code".to_string()),
        "the requested tool was still added: {:?}",
        tools
    );
    assert!(
        tools.contains(&"pi".to_string()),
        "the stuck tool is kept for a retry: {:?}",
        tools
    );
}

/// The whole-skill rule of ADR-0002 at project scope: a failed artifact keeps
/// the project and its `error` rows, so a retry can still find every path.
#[cfg(unix)]
#[test]
fn remove_project_with_cleanup_keeps_the_project_when_an_artifact_stays() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");
    let (project, skill, target) = setup_pi_assignment(tmpdir.path(), &store, "rpc-pi-stuck");
    if !lock_parent(&target) {
        return;
    }

    let err = project_ops::remove_project_with_cleanup(&store, &project.id)
        .expect_err("a stuck artifact keeps the project");
    unlock_parent(&target);

    match err.downcast_ref::<SignalError>() {
        Some(SignalError::DeleteCleanupFailed { failures }) => {
            assert_eq!(failures.len(), 1);
            assert!(failures[0].starts_with(&format!("{}: ", target.display())));
        }
        other => panic!("expected DeleteCleanupFailed, got {:?}", other),
    }

    assert!(
        store.get_project_by_id(&project.id).unwrap().is_some(),
        "the project is kept so the failure stays retryable"
    );
    let assignment = store
        .get_project_skill_assignment(&project.id, &skill.id, "pi")
        .unwrap()
        .expect("row kept");
    assert_eq!(assignment.status, SyncStatus::Error);
}

#[test]
fn register_expands_tilde_against_home() {
    let home = tempfile::tempdir().expect("home");
    fs::create_dir_all(home.path().join("proj")).unwrap();
    let (_db_dir, store) = make_store();
    let dto = project_ops::register_project_path(&store, home.path(), "~/proj", now_ms()).unwrap();
    let canonical = std::fs::canonicalize(home.path().join("proj"))
        .unwrap()
        .to_string_lossy()
        .to_string();
    assert_eq!(dto.path, canonical);
}

// ---------------------------------------------------------------------------
// configure_project_tools — one batch write owning the persist → derive →
// gitignore ordering. The frontend used to replay the ignore intent after the
// per-tool commands returned; here core sequences both writes.
// ---------------------------------------------------------------------------

use crate::core::errors::SignalError;
use crate::core::gitignore::{project_ignore_status, IgnoreUpdateOptions, MARKER};

fn register_dir_project(store: &SkillStore, dir: &std::path::Path) -> ProjectRecord {
    let project = ProjectRecord {
        id: uuid::Uuid::new_v4().to_string(),
        path: dir.to_string_lossy().to_string(),
        created_at: 1,
        updated_at: 1,
    };
    store.register_project(&project).expect("register_project");
    project
}

fn strs(keys: &[&str]) -> Vec<String> {
    keys.iter().map(|k| k.to_string()).collect()
}

#[test]
fn configure_tools_writes_gitignore_from_the_tools_just_persisted() {
    let (dir, store) = make_store();
    let project_dir = dir.path().join("proj");
    fs::create_dir_all(&project_dir).unwrap();
    // Fresh project: nothing persisted yet. Deriving patterns before the
    // tools are written would yield an empty pattern set and no file.
    let project = register_dir_project(&store, &project_dir);

    let tools = project_ops::configure_project_tools(
        &store,
        &project.id,
        &strs(&["claude_code", "windsurf"]),
        Some(IgnoreUpdateOptions {
            add_to_gitignore: true,
            add_to_exclude: true,
        }),
    )
    .expect("configure");

    let mut keys: Vec<String> = tools.into_iter().map(|t| t.tool).collect();
    keys.sort();
    assert_eq!(keys, strs(&["claude_code", "windsurf"]));
    assert_eq!(store.list_project_tools(&project.id).unwrap().len(), 2);

    let gitignore = fs::read_to_string(project_dir.join(".gitignore")).expect(".gitignore");
    assert!(gitignore.contains(MARKER));
    assert!(gitignore.contains("/.claude/skills/"));
    assert!(gitignore.contains("/.windsurf/skills/"));
    let exclude = fs::read_to_string(project_dir.join(".git/info/exclude")).expect("exclude");
    assert!(exclude.contains("/.windsurf/skills/"));
}

#[test]
fn configure_tools_diffs_against_persisted_tools_and_rewrites_the_block() {
    let (dir, store) = make_store();
    let project_dir = dir.path().join("proj");
    fs::create_dir_all(&project_dir).unwrap();
    let project = register_dir_project(&store, &project_dir);
    project_ops::configure_project_tools(
        &store,
        &project.id,
        &strs(&["claude_code", "windsurf"]),
        Some(IgnoreUpdateOptions {
            add_to_gitignore: true,
            add_to_exclude: false,
        }),
    )
    .unwrap();
    let first_ids: Vec<String> = store
        .list_project_tools(&project.id)
        .unwrap()
        .into_iter()
        .filter(|t| t.tool == "claude_code")
        .map(|t| t.id)
        .collect();

    // Drop windsurf, add pi; claude_code stays (same record id, not re-inserted).
    let tools = project_ops::configure_project_tools(
        &store,
        &project.id,
        &strs(&["claude_code", "pi"]),
        Some(IgnoreUpdateOptions {
            add_to_gitignore: true,
            add_to_exclude: false,
        }),
    )
    .unwrap();

    let mut keys: Vec<String> = tools.iter().map(|t| t.tool.clone()).collect();
    keys.sort();
    assert_eq!(keys, strs(&["claude_code", "pi"]));
    let kept: Vec<String> = tools
        .iter()
        .filter(|t| t.tool == "claude_code")
        .map(|t| t.id.clone())
        .collect();
    assert_eq!(kept, first_ids, "unchanged tool keeps its record");

    let gitignore = fs::read_to_string(project_dir.join(".gitignore")).unwrap();
    assert!(gitignore.contains("/.claude/skills/"));
    assert!(gitignore.contains("/.pi/skills/"), "{gitignore}");
    assert!(!gitignore.contains("/.windsurf/skills/"), "{gitignore}");
    assert!(!project_dir.join(".git/info/exclude").exists());
}

#[test]
fn configure_tools_without_intent_leaves_ignore_files_alone() {
    let (dir, store) = make_store();
    let project_dir = dir.path().join("proj");
    fs::create_dir_all(&project_dir).unwrap();
    let project = register_dir_project(&store, &project_dir);

    project_ops::configure_project_tools(&store, &project.id, &strs(&["claude_code"]), None)
        .unwrap();

    assert_eq!(store.list_project_tools(&project.id).unwrap().len(), 1);
    let status = project_ignore_status(&project_dir);
    assert!(!status.in_gitignore && !status.in_exclude);
    assert!(!project_dir.join(".gitignore").exists());
}

#[test]
fn configure_tools_rejects_unknown_tool_before_writing_anything() {
    let (dir, store) = make_store();
    let project_dir = dir.path().join("proj");
    fs::create_dir_all(&project_dir).unwrap();
    let project = register_dir_project(&store, &project_dir);

    let err = project_ops::configure_project_tools(
        &store,
        &project.id,
        &strs(&["claude_code", "not-a-tool"]),
        Some(IgnoreUpdateOptions {
            add_to_gitignore: true,
            add_to_exclude: true,
        }),
    )
    .expect_err("unknown tool must fail");
    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::UnknownTool {
            tool: "not-a-tool".to_string(),
        })
    );
    assert!(store.list_project_tools(&project.id).unwrap().is_empty());
    assert!(!project_dir.join(".gitignore").exists());
}

#[test]
fn configure_tools_raises_typed_not_found_for_unknown_project() {
    let (_dir, store) = make_store();
    let err =
        project_ops::configure_project_tools(&store, "missing", &strs(&["claude_code"]), None)
            .expect_err("must fail");
    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::NotFound {
            kind: "project".to_string(),
            id: "missing".to_string(),
        })
    );
}

#[test]
fn update_project_path_raises_typed_not_found_for_unknown_project() {
    let (dir, store) = make_store();
    let target = dir.path().join("elsewhere");
    std::fs::create_dir_all(&target).expect("create dir");
    let err = project_ops::update_project_path(
        &store,
        dir.path(),
        "missing",
        &target.to_string_lossy(),
        now_ms(),
    )
    .expect_err("unknown project must fail");
    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::NotFound {
            kind: "project".to_string(),
            id: "missing".to_string(),
        })
    );
}

// ---------------------------------------------------------------------------
// project_view — the one value every project mutation returns
// ---------------------------------------------------------------------------

#[test]
fn project_view_reports_the_projects_row_tools_and_reconciled_assignments() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");
    let (project, _skill, _target) = setup_pi_assignment(tmpdir.path(), &store, "pv-basic");

    let view = project_ops::project_view(&store, &project.id).expect("view");

    assert_eq!(view.project.id, project.id);
    assert_eq!(view.project.tool_count, 1);
    assert_eq!(view.project.skill_count, 1);
    assert_eq!(view.project.assignment_count, 1);
    assert_eq!(view.project.sync_status, ProjectSyncStatus::Synced);
    assert_eq!(
        view.tools
            .iter()
            .map(|t| t.tool.as_str())
            .collect::<Vec<_>>(),
        vec!["pi"]
    );
    assert_eq!(view.assignments.assignments.len(), 1);
    // `reconciled` is not asserted: the flag reports whether the try-lock
    // succeeded, and any other test mutating concurrently can legitimately
    // make it false. Its contract is covered in `project_sync`'s listing
    // tests.
}

/// The cascade case: dropping a tool removes its assignments, and the view
/// the mutation's caller reads already reflects that — no follow-up read can
/// disagree with it.
#[test]
fn view_after_configure_tools_reflects_the_assignment_cascade() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");
    let (project, _skill, target) = setup_pi_assignment(tmpdir.path(), &store, "pv-cascade");

    project_ops::configure_project_tools(&store, &project.id, &strs(&["claude_code"]), None)
        .expect("configure tools");
    let view = project_ops::project_view(&store, &project.id).expect("view");

    assert!(
        target.symlink_metadata().is_err(),
        "the dropped tool's artifact is gone: {:?}",
        target
    );
    assert_eq!(
        view.tools
            .iter()
            .map(|t| t.tool.as_str())
            .collect::<Vec<_>>(),
        vec!["claude_code"]
    );
    assert!(
        view.assignments.assignments.iter().all(|a| a.tool != "pi"),
        "no assignment for the removed tool survives in the view"
    );
    assert_eq!(view.project.tool_count, 1);
    assert_eq!(view.project.assignment_count, 0);
    assert_eq!(view.project.skill_count, 0);
    assert_eq!(view.project.sync_status, ProjectSyncStatus::Empty);
}

#[test]
fn project_view_raises_typed_not_found_for_unknown_project() {
    let (_dir, store) = make_store();
    let err = project_ops::project_view(&store, "missing").expect_err("must fail");
    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::NotFound {
            kind: "project".to_string(),
            id: "missing".to_string(),
        })
    );
}
