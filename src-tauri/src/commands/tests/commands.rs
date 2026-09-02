use super::*;
use crate::core::errors::SignalError;
use crate::core::global_sync::GlobalSyncError;
use error::GitCloneFailureKind;

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
fn from_anyhow_recovers_unknown_tool_and_invalid_path_through_context() {
    let err = anyhow::Error::new(SignalError::UnknownTool {
        tool: "not-a-tool".to_string(),
    })
    .context("configure project tools");
    match CommandError::from_anyhow(err) {
        CommandError::UnknownTool { tool } => assert_eq!(tool, "not-a-tool"),
        other => panic!("expected UnknownTool, got {other}"),
    }

    let err = anyhow::Error::new(SignalError::InvalidPath {
        path: "/tmp/gone".to_string(),
        reason: "missing".to_string(),
    })
    .context("update gitignore");
    match CommandError::from_anyhow(err) {
        CommandError::InvalidPath { path, reason } => {
            assert_eq!(path, "/tmp/gone");
            assert_eq!(reason, "missing");
        }
        other => panic!("expected InvalidPath, got {other}"),
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

    let json = serde_json::to_value(CommandError::from_anyhow(anyhow::anyhow!(
        SignalError::SkillExists {
            name: "react-best-practices".to_string()
        }
    )))
    .unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "code": "SKILL_EXISTS", "name": "react-best-practices" })
    );

    let json = serde_json::to_value(CommandError::from(SignalError::PathOutsideToolDirs {
        path: "/home/user/Documents".to_string(),
    }))
    .unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "code": "PATH_OUTSIDE_TOOL_DIRS", "path": "/home/user/Documents" })
    );

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
