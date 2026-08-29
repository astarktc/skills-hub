//! Tests for `core::global_sync` — the deterministic half of global-tool
//! sync: overwrite policy, error classification, record fan-out, and unsync.
//!
//! `sync_skill_into_root` / `remove_targets_for_tools` are driven directly so
//! the tests never touch the operator's real home directory or installed
//! tools (the environment probing lives in the thin `*_with_records`
//! wrappers).

use std::fs;
use std::path::Path;

use crate::core::global_sync::{
    remove_targets_for_tools, sync_skill_into_root, target_has_same_content, GlobalSyncError,
    OverwritePolicy,
};
use crate::core::skill_store::{SkillRecord, SkillStore};
use crate::core::tool_adapters::{adapter_by_key, ToolAdapter};

fn make_store(base: &Path) -> SkillStore {
    let store = SkillStore::new(base.join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    store
}

/// skill_targets has a foreign key on skills — seed the parent row.
fn seed_skill(store: &SkillStore, id: &str) {
    let skill = SkillRecord {
        id: id.to_string(),
        name: id.to_string(),
        description: None,
        source_type: "local".to_string(),
        source_ref: None,
        source_subpath: None,
        source_revision: None,
        central_path: format!("/tmp/central/{}", id),
        content_hash: None,
        created_at: 1,
        updated_at: 1,
        last_sync_at: None,
        last_seen_at: 1,
        status: "ok".to_string(),
    };
    store.upsert_skill(&skill).expect("seed skill");
}

fn make_skill_dir(base: &Path, name: &str, content: &str) -> std::path::PathBuf {
    let dir = base.join(name);
    fs::create_dir_all(&dir).expect("create skill dir");
    fs::write(dir.join("SKILL.md"), content).expect("write SKILL.md");
    dir
}

fn no_overwrite() -> OverwritePolicy {
    OverwritePolicy {
        overwrite: false,
        overwrite_if_same_content: false,
    }
}

fn claude() -> ToolAdapter {
    adapter_by_key("claude_code").expect("claude_code adapter")
}

#[test]
fn sync_creates_target_and_records_for_all_group_tools() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = make_store(dir.path());
    seed_skill(&store, "skill-1");
    let source = make_skill_dir(dir.path(), "central-skill", "# Skill");
    let tool_root = dir.path().join("skills-root");

    let adapter = claude();
    // Simulate a shared-dir group of two installed tools.
    let amp = adapter_by_key("amp").expect("amp adapter");
    let record_tools = vec![adapter.clone(), amp];

    let outcome = sync_skill_into_root(
        &store,
        &adapter,
        &tool_root,
        &source,
        "skill-1",
        "my-skill",
        &no_overwrite(),
        &record_tools,
        1000,
    )
    .expect("sync");

    assert_eq!(outcome.target_path, tool_root.join("my-skill"));
    assert!(outcome.target_path.exists());

    for key in ["claude_code", "amp"] {
        let record = store
            .get_skill_target("skill-1", key)
            .expect("query")
            .unwrap_or_else(|| panic!("record for {}", key));
        assert_eq!(record.target_path, outcome.target_path.to_string_lossy());
        assert_eq!(record.mode, outcome.mode_used.as_str());
        assert_eq!(record.status, "ok");
        assert_eq!(record.synced_at, Some(1000));
    }
}

#[test]
fn sync_without_overwrite_fails_with_target_exists() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = make_store(dir.path());
    seed_skill(&store, "skill-1");
    let source = make_skill_dir(dir.path(), "central-skill", "# Skill");
    let tool_root = dir.path().join("skills-root");
    // Pre-existing unrelated dir at the target path.
    fs::create_dir_all(tool_root.join("my-skill")).expect("occupy target");
    fs::write(tool_root.join("my-skill/other.md"), "other").expect("occupy target");

    let adapter = claude();
    let err = sync_skill_into_root(
        &store,
        &adapter,
        &tool_root,
        &source,
        "skill-1",
        "my-skill",
        &no_overwrite(),
        std::slice::from_ref(&adapter),
        1000,
    )
    .expect_err("must fail");

    match err {
        GlobalSyncError::TargetExists { target_path } => {
            assert_eq!(target_path, tool_root.join("my-skill"));
        }
        other => panic!("expected TargetExists, got {:?}", other),
    }
    // No record must be written on failure.
    assert!(store
        .get_skill_target("skill-1", "claude_code")
        .expect("query")
        .is_none());
}

#[test]
fn sync_with_overwrite_replaces_existing_target() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = make_store(dir.path());
    seed_skill(&store, "skill-1");
    let source = make_skill_dir(dir.path(), "central-skill", "# New");
    let tool_root = dir.path().join("skills-root");
    fs::create_dir_all(tool_root.join("my-skill")).expect("occupy target");
    fs::write(tool_root.join("my-skill/SKILL.md"), "# Old").expect("occupy target");

    let adapter = claude();
    let outcome = sync_skill_into_root(
        &store,
        &adapter,
        &tool_root,
        &source,
        "skill-1",
        "my-skill",
        &OverwritePolicy {
            overwrite: true,
            overwrite_if_same_content: false,
        },
        std::slice::from_ref(&adapter),
        1000,
    )
    .expect("sync");
    assert!(outcome.target_path.exists());
}

#[test]
fn overwrite_if_same_content_only_replaces_identical_targets() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = make_store(dir.path());
    seed_skill(&store, "skill-1");
    seed_skill(&store, "skill-2");
    let source = make_skill_dir(dir.path(), "central-skill", "# Same");
    let tool_root = dir.path().join("skills-root");
    let policy = OverwritePolicy {
        overwrite: false,
        overwrite_if_same_content: true,
    };
    let adapter = claude();

    // Target with identical content: allowed to replace.
    make_skill_dir(&tool_root, "my-skill", "# Same");
    assert!(target_has_same_content(
        &source,
        &tool_root.join("my-skill")
    ));
    sync_skill_into_root(
        &store,
        &adapter,
        &tool_root,
        &source,
        "skill-1",
        "my-skill",
        &policy,
        std::slice::from_ref(&adapter),
        1000,
    )
    .expect("same-content sync must succeed");

    // Target with different content: refused.
    make_skill_dir(&tool_root, "other-skill", "# Different");
    let err = sync_skill_into_root(
        &store,
        &adapter,
        &tool_root,
        &source,
        "skill-2",
        "other-skill",
        &policy,
        std::slice::from_ref(&adapter),
        1000,
    )
    .expect_err("different content must not be replaced");
    assert!(matches!(err, GlobalSyncError::TargetExists { .. }));
}

#[test]
fn cursor_gets_copy_mode() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = make_store(dir.path());
    seed_skill(&store, "skill-1");
    let source = make_skill_dir(dir.path(), "central-skill", "# Skill");
    let tool_root = dir.path().join("skills-root");

    let cursor = adapter_by_key("cursor").expect("cursor adapter");
    let outcome = sync_skill_into_root(
        &store,
        &cursor,
        &tool_root,
        &source,
        "skill-1",
        "my-skill",
        &no_overwrite(),
        std::slice::from_ref(&cursor),
        1000,
    )
    .expect("sync");
    assert_eq!(outcome.mode_used.as_str(), "copy");
    let record = store
        .get_skill_target("skill-1", "cursor")
        .expect("query")
        .expect("record");
    assert_eq!(record.mode, "copy");
}

#[test]
fn unsync_removes_filesystem_target_once_and_all_group_records() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = make_store(dir.path());
    seed_skill(&store, "skill-1");
    let source = make_skill_dir(dir.path(), "central-skill", "# Skill");
    let tool_root = dir.path().join("skills-root");

    let adapter = claude();
    let amp = adapter_by_key("amp").expect("amp adapter");
    let outcome = sync_skill_into_root(
        &store,
        &adapter,
        &tool_root,
        &source,
        "skill-1",
        "my-skill",
        &no_overwrite(),
        &[adapter.clone(), amp],
        1000,
    )
    .expect("sync");
    assert!(outcome.target_path.exists());

    remove_targets_for_tools(
        &store,
        "skill-1",
        &["claude_code".to_string(), "amp".to_string()],
    )
    .expect("unsync");

    assert!(
        std::fs::symlink_metadata(&outcome.target_path).is_err(),
        "target must be removed"
    );
    for key in ["claude_code", "amp"] {
        assert!(store
            .get_skill_target("skill-1", key)
            .expect("query")
            .is_none());
    }
}

#[test]
fn unsync_with_no_records_is_a_no_op() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = make_store(dir.path());
    remove_targets_for_tools(&store, "missing-skill", &["claude_code".to_string()])
        .expect("no-op unsync");
}
