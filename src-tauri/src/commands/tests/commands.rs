use super::*;
use crate::core::artifact_removal::{
    RemovalReport, RemovalScope, RemovalTargetOutcome, RemovalTargetStatus, RowRef,
};
use crate::core::errors::SignalError;
use crate::core::global_sync::GlobalSyncError;
use crate::core::sync_engine::remove_path_any;
use error::GitCloneFailureKind;

/// A removal target that failed with a typed condition reaches the wire as
/// that condition's own code — the report carries the error value, so the
/// seam classifies it the way it classifies a thrown error (never `OTHER`).
/// A shared skills dir settles several rows from one failure: every row
/// carries the same classified error.
#[test]
fn removal_report_dto_classifies_a_typed_target_failure_at_the_seam() {
    let refused = "/home/user/Documents/not-a-skill";
    let error = anyhow::Error::new(SignalError::PathOutsideToolDirs {
        path: refused.to_string(),
    })
    .context("remove dir");
    let report = RemovalReport {
        scope: RemovalScope::SkillGlobal {
            skill_id: "s1".to_string(),
        },
        targets: vec![RemovalTargetOutcome {
            path: std::path::PathBuf::from(refused),
            rows: vec![
                RowRef::GlobalTarget {
                    id: "t1".to_string(),
                    skill_id: "s1".to_string(),
                    tool: "amp".to_string(),
                },
                RowRef::GlobalTarget {
                    id: "t2".to_string(),
                    skill_id: "s1".to_string(),
                    tool: "kimi_cli".to_string(),
                },
            ],
            status: RemovalTargetStatus::Failed { error },
        }],
        central_removed: false,
        record_deleted: false,
    };

    let dto = to_removal_report_dto(report);

    assert_eq!(dto.failed, 2);
    assert_eq!(dto.removed, 0);
    assert_eq!(dto.targets.len(), 2, "one DTO row per settled row");
    for target in &dto.targets {
        match &target.status {
            RemovalTargetStatusDto::Failed {
                error: CommandError::PathOutsideToolDirs { path },
            } => assert_eq!(path, refused),
            other => panic!(
                "expected PATH_OUTSIDE_TOOL_DIRS for {}, got {other:?}",
                target.tool
            ),
        }
    }
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

/// An upstream symlink whose target leaves the repository reaches the wire
/// as its own code, carrying the link's subpath and the raw target as
/// diagnostics — never as prose.
#[test]
fn from_anyhow_recovers_symlink_escapes_repo_through_context() {
    let err = anyhow::Error::new(SignalError::SymlinkEscapesRepo {
        subpath: "plugins/all/skills/x".to_string(),
        target: "../../../../etc".to_string(),
    })
    .context("acquire skill");
    match CommandError::from_anyhow(err) {
        CommandError::SymlinkEscapesRepo { subpath, target } => {
            assert_eq!(subpath, "plugins/all/skills/x");
            assert_eq!(target, "../../../../etc");
        }
        other => panic!("expected SymlinkEscapesRepo, got {other}"),
    }
}

#[test]
fn symlink_chain_too_deep_serializes_its_subpath_through_context() {
    let err = anyhow::Error::new(SignalError::SymlinkChainTooDeep {
        subpath: "skills/alias-9".to_string(),
    })
    .context("acquire skill");

    let json = serde_json::to_value(CommandError::from_anyhow(err)).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "code": "SYMLINK_CHAIN_TOO_DEEP",
            "subpath": "skills/alias-9",
        })
    );
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

/// Installer roots isolated under one temp dir, the same shape the commands
/// resolve at the seam (`installer_paths`).
fn missing_path_fixture() -> (
    tempfile::TempDir,
    crate::core::installer::InstallerPaths,
    crate::core::skill_store::SkillStore,
) {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = crate::core::installer::InstallerPaths {
        home: dir.path().join("home"),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    std::fs::create_dir_all(&paths.home).unwrap();
    let store = crate::core::skill_store::SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    (dir, paths, store)
}

/// Run the single-skill Update the way the command does (a Refresh batch of
/// one) and classify its outcome at the seam.
fn update_outcome_at_the_seam(
    paths: &crate::core::installer::InstallerPaths,
    store: &crate::core::skill_store::SkillStore,
    skill_id: &str,
) -> CommandError {
    let report = crate::core::refresh::refresh_managed_skills(
        paths,
        store,
        crate::core::refresh::RefreshSelection::Ids(vec![skill_id.to_string()]),
        crate::core::refresh::RefreshPolicy::default(),
        None,
        0,
        |_| {},
    )
    .expect("a per-skill failure is report data, not a batch error");
    let outcome = report
        .skills
        .into_iter()
        .next()
        .expect("one outcome for the one skill");
    match outcome.status {
        crate::core::refresh::SkillRefreshStatus::Failed { error } => {
            CommandError::from_anyhow(error)
        }
        other => panic!("expected the update to fail, got {other:?}"),
    }
}

/// Update of a `local` skill whose folder is gone reaches the wire as its own
/// code with the path in a structured field — never `OTHER` prose carrying
/// the path.
#[test]
fn update_of_a_local_skill_whose_source_is_gone_is_typed_source_path_missing() {
    let (_dir, paths, store) = missing_path_fixture();
    let source = tempfile::tempdir().unwrap();
    std::fs::write(source.path().join("SKILL.md"), b"---\nname: x\n---\n").unwrap();
    let installed = crate::core::installer::install_local_skill(
        &paths,
        &store,
        source.path(),
        Some("gone-local".to_string()),
    )
    .unwrap();
    let source_path = source.path().to_string_lossy().to_string();
    drop(source);

    let error = update_outcome_at_the_seam(&paths, &store, &installed.skill_id);

    let json = serde_json::to_value(&error).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "code": "SOURCE_PATH_MISSING", "path": source_path }),
        "got {error}"
    );
    assert!(json.get("message").is_none(), "no prose on the wire");
}

/// A missing central copy reaches the wire as its own code with the central
/// path in a structured field. Since round 4 only moving the central repo
/// raises it: an Update of a `git`/`local` skill whose central copy is gone
/// is a Restore and rebuilds it (`core::refresh` tests pin that).
#[test]
fn a_missing_central_copy_is_typed_central_path_missing_on_the_wire() {
    let error = CommandError::from_anyhow(
        anyhow::anyhow!(SignalError::CentralPathMissing {
            path: "/home/u/.skillshub/gone".to_string(),
        })
        .context("move central repo"),
    );

    let json = serde_json::to_value(&error).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "code": "CENTRAL_PATH_MISSING",
            "path": "/home/u/.skillshub/gone",
        }),
        "got {error}"
    );
    assert!(json.get("message").is_none(), "no prose on the wire");
}

/// Update of an `imported` skill reaches the wire as `NOT_REFRESHABLE` with
/// the skill's name — the card hides Update for it, so this is the answer to
/// a stale or hand-built request.
#[test]
fn update_of_an_imported_skill_is_typed_not_refreshable() {
    let (_dir, paths, store) = missing_path_fixture();
    let found = paths.home.join(".claude/skills/taken-over");
    std::fs::create_dir_all(&found).unwrap();
    std::fs::write(found.join("SKILL.md"), b"---\nname: taken-over\n---\n").unwrap();
    let installed = crate::core::installer::install_imported_skill(
        &paths,
        &store,
        &found,
        Some("taken-over".to_string()),
        Some("claude_code"),
    )
    .unwrap();

    let error = update_outcome_at_the_seam(&paths, &store, &installed.skill_id);

    let json = serde_json::to_value(&error).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "code": "NOT_REFRESHABLE", "name": "taken-over" }),
        "got {error}"
    );
    assert!(json.get("message").is_none(), "no prose on the wire");
}

/// Whatever fails on the way to revealing the log folder (resolving the dir,
/// creating it, the opener) reaches the wire as `REVEAL_LOG_FAILED` with
/// the whole chain as diagnostics.
#[test]
fn an_open_log_folder_failure_is_typed_with_its_chain_as_detail() {
    let err = anyhow::anyhow!("launcher exited with code 1")
        .context("failed to reveal log path \"/Users/u/Library/Logs/com.skillshub.app\"");

    let json = serde_json::to_value(reveal_log_failed(err)).unwrap();

    assert_eq!(json["code"], "REVEAL_LOG_FAILED");
    let detail = json["detail"].as_str().expect("detail is a string");
    assert!(detail.contains("failed to reveal log path"), "got {detail}");
    assert!(
        detail.contains("launcher exited with code 1"),
        "got {detail}"
    );
    assert!(json.get("message").is_none(), "no prose on the wire");
}

/// The requested subpath is the only thing a `SUBPATH_MISSING` carries.
/// The Add → local folder refusal crosses the seam as its own code carrying
/// the refused path and the holding Tool's registry key (the frontend
/// localizes the Tool's label and steers to Import).
#[test]
fn local_source_inside_tool_dir_serializes_the_path_and_the_tool_key() {
    let err = anyhow::Error::new(SignalError::LocalSourceInsideToolDir {
        path: "/home/u/.claude/skills/taken".to_string(),
        tool: "claude_code".to_string(),
    })
    .context("install local skill");

    let json = serde_json::to_value(CommandError::from_anyhow(err)).unwrap();

    assert_eq!(
        json,
        serde_json::json!({
            "code": "LOCAL_SOURCE_INSIDE_TOOL_DIR",
            "path": "/home/u/.claude/skills/taken",
            "tool": "claude_code",
        })
    );
}

#[test]
fn subpath_missing_serializes_the_requested_subpath_only() {
    let err = anyhow::Error::new(SignalError::SubpathMissing {
        subpath: "skills/nope".to_string(),
    })
    .context("acquire");

    let json = serde_json::to_value(CommandError::from_anyhow(err)).unwrap();

    assert_eq!(
        json,
        serde_json::json!({ "code": "SUBPATH_MISSING", "subpath": "skills/nope" })
    );
}
