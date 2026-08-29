use super::*;
use crate::core::errors::SignalError;
use crate::core::global_sync::GlobalSyncError;
use crate::core::skill_store::{SkillRecord, SkillTargetRecord};
use error::GitCloneFailureKind;

fn make_store() -> (tempfile::TempDir, SkillStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    (dir, store)
}

#[test]
fn from_anyhow_recovers_signal_errors_through_context() {
    let err = anyhow::Error::new(SignalError::MultiSkills).context("install skill");
    assert!(matches!(
        CommandError::from_anyhow(err),
        CommandError::MultiSkills
    ));

    let err = anyhow::anyhow!(SignalError::RateLimited { reset_minutes: 7 });
    assert!(matches!(
        CommandError::from_anyhow(err),
        CommandError::RateLimited { reset_minutes: 7 }
    ));

    let err = anyhow::anyhow!(SignalError::NotFound {
        kind: "project".to_string(),
        id: "abc-123".to_string(),
    });
    match CommandError::from_anyhow(err) {
        CommandError::NotFound { kind, id } => {
            assert_eq!(kind, "project");
            assert_eq!(id, "abc-123");
        }
        other => panic!("expected NotFound, got {other}"),
    }
}

#[test]
fn from_anyhow_recovers_global_sync_errors() {
    let err = anyhow::Error::new(GlobalSyncError::ToolNotWritable {
        tool_display_name: "Cursor".to_string(),
        skills_dir: std::path::PathBuf::from("/tmp/skills"),
    });
    match CommandError::from_anyhow(err) {
        CommandError::ToolNotWritable { tool, path } => {
            assert_eq!(tool, "Cursor");
            assert_eq!(path, "/tmp/skills");
        }
        other => panic!("expected ToolNotWritable, got {other}"),
    }
}

#[test]
fn from_anyhow_classifies_github_clone_failures() {
    let err = anyhow::anyhow!("git clone https://github.com/a/b failed: authentication failed");
    match CommandError::from_anyhow(err) {
        CommandError::GitCloneFailed { kind, detail } => {
            assert_eq!(kind, GitCloneFailureKind::Auth);
            assert!(detail.contains("authentication failed"));
        }
        other => panic!("expected GitCloneFailed, got {other}"),
    }

    let err = anyhow::anyhow!("fetch https://github.com/a/b: connection timed out");
    assert!(matches!(
        CommandError::from_anyhow(err),
        CommandError::GitCloneFailed {
            kind: GitCloneFailureKind::Timeout,
            ..
        }
    ));
}

#[test]
fn from_anyhow_redacts_clone_temp_path_in_other() {
    let err = anyhow::anyhow!("clone https://example.com/a/b into /tmp/skills-hub-git-123");
    match CommandError::from_anyhow(err) {
        CommandError::Other { message } => {
            assert!(
                !message.contains("/tmp/skills-hub-git-123"),
                "got: {message}"
            );
            assert!(message.contains("clone https://example.com/a/b"));
        }
        other => panic!("expected Other, got {other}"),
    }
}

#[test]
fn command_error_wire_shape_is_internally_tagged() {
    let json = serde_json::to_value(CommandError::ToolNotWritable {
        tool: "Cursor".to_string(),
        path: "/tmp/skills".to_string(),
    })
    .unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "code": "TOOL_NOT_WRITABLE",
            "tool": "Cursor",
            "path": "/tmp/skills",
        })
    );

    let json = serde_json::to_value(CommandError::Cancelled).unwrap();
    assert_eq!(json, serde_json::json!({ "code": "CANCELLED" }));

    let json = serde_json::to_value(CommandError::RateLimited { reset_minutes: 5 }).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "code": "RATE_LIMITED", "resetMinutes": 5 })
    );

    let json = serde_json::to_value(CommandError::GitCloneFailed {
        kind: GitCloneFailureKind::NotFound,
        detail: "404".to_string(),
    })
    .unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "code": "GIT_CLONE_FAILED", "kind": "notFound", "detail": "404" })
    );
}

#[test]
fn expand_home_path_basic() {
    let home = dirs::home_dir().expect("home");
    assert_eq!(expand_home_path("~").unwrap(), home);
    assert_eq!(expand_home_path("~/abc").unwrap(), home.join("abc"));
}

#[test]
fn expand_home_path_empty_is_error() {
    let err = expand_home_path("  ").unwrap_err().to_string();
    assert!(err.contains("storage path is empty"));
}

#[test]
fn remove_path_any_handles_file_dir_and_missing() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("f.txt");
    std::fs::write(&file, b"1").unwrap();
    remove_path_any(&file).unwrap();
    assert!(!file.exists());

    let sub = dir.path().join("d");
    std::fs::create_dir_all(&sub).unwrap();
    remove_path_any(&sub).unwrap();
    assert!(!sub.exists());

    remove_path_any(&dir.path().join("missing")).unwrap();
}

#[test]
#[cfg(unix)]
fn remove_path_any_removes_symlink_only() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("real");
    std::fs::create_dir_all(&target).unwrap();
    let link = dir.path().join("link");
    symlink(&target, &link).unwrap();

    remove_path_any(&link).unwrap();
    assert!(!link.exists());
    assert!(target.exists());
}

#[test]
fn get_managed_skills_impl_maps_targets() {
    let (_dir, store) = make_store();
    let skill = SkillRecord {
        id: "s1".to_string(),
        name: "S1".to_string(),
        description: None,
        source_type: "local".to_string(),
        source_ref: Some("/tmp/src".to_string()),
        source_subpath: None,
        source_revision: None,
        central_path: "/tmp/central".to_string(),
        content_hash: None,
        created_at: 1,
        updated_at: 2,
        last_sync_at: None,
        last_seen_at: 1,
        status: "ok".to_string(),
    };
    store.upsert_skill(&skill).unwrap();

    let target = SkillTargetRecord {
        id: "t1".to_string(),
        skill_id: "s1".to_string(),
        tool: "cursor".to_string(),
        target_path: "/tmp/target".to_string(),
        mode: "copy".to_string(),
        status: "ok".to_string(),
        last_error: None,
        synced_at: None,
    };
    store.upsert_skill_target(&target).unwrap();

    let out = get_managed_skills_impl(&store).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].targets.len(), 1);
    assert_eq!(out[0].targets[0].tool, "cursor");
}

#[test]
fn project_signal_errors_serialize_with_payload_fields() {
    // The exact wire shapes the projects frontend discriminates on.
    let json = serde_json::to_value(CommandError::from(SignalError::DuplicateProject {
        path: "/home/user/my-project".to_string(),
    }))
    .unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "code": "DUPLICATE_PROJECT", "path": "/home/user/my-project" })
    );

    let json = serde_json::to_value(CommandError::from(SignalError::AssignmentExists {
        project: "proj1".to_string(),
        skill: "skill1".to_string(),
        tool: "claude_code".to_string(),
    }))
    .unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "code": "ASSIGNMENT_EXISTS",
            "project": "proj1",
            "skill": "skill1",
            "tool": "claude_code",
        })
    );

    let json = serde_json::to_value(CommandError::from(SignalError::NotFound {
        kind: "skill".to_string(),
        id: "nonexistent-uuid".to_string(),
    }))
    .unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "code": "NOT_FOUND", "kind": "skill", "id": "nonexistent-uuid" })
    );
}

#[test]
fn global_tool_config_defaults() {
    let (_dir, store) = make_store();
    let cfg = get_global_tool_config_impl(&store).expect("get config");
    assert_eq!(cfg.selected_tools, None);
    assert!(cfg.scan_selected_only);
}

#[test]
fn global_tool_config_roundtrip() {
    let (_dir, store) = make_store();
    let selected = vec!["claude_code".to_string(), "cursor".to_string()];
    set_global_tool_config_impl(&store, &selected, false).expect("set config");
    let cfg = get_global_tool_config_impl(&store).expect("get config");
    assert_eq!(cfg.selected_tools, Some(selected));
    assert!(!cfg.scan_selected_only);
}

#[test]
fn global_tool_config_empty_selection_persists() {
    let (_dir, store) = make_store();
    set_global_tool_config_impl(&store, &[], true).expect("set config");
    let cfg = get_global_tool_config_impl(&store).expect("get config");
    // Empty selection is a deliberate choice, distinct from "never configured".
    assert_eq!(cfg.selected_tools, Some(vec![]));
    assert!(cfg.scan_selected_only);
}
