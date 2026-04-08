use crate::core::project_ops;
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
        err.contains("already registered"),
        "expected 'already registered', got: {}",
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
