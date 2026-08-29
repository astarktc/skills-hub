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
    remove_targets_for_tools, sync_skill_into_root, sync_skills_to_planned_tools,
    target_has_same_content, BatchOverride, BatchPolicy, BatchSkill, BatchTargetStatus,
    GlobalSyncError, OverwritePolicy, PlannedToolTarget,
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

// ---------------------------------------------------------------------------
// Batch engine (`sync_skills_to_planned_tools`) — driven with fabricated
// planned targets so no test touches the real environment.
// ---------------------------------------------------------------------------

fn batch_skill(id: &str, name: &str, source: &Path) -> BatchSkill {
    BatchSkill {
        skill_id: id.to_string(),
        skill_name: name.to_string(),
        source_path: source.to_path_buf(),
    }
}

fn planned(adapter: &ToolAdapter, root: &Path, installed: bool) -> PlannedToolTarget {
    PlannedToolTarget {
        adapter: adapter.clone(),
        root: root.to_path_buf(),
        installed,
        record_tools: vec![adapter.clone()],
    }
}

#[test]
fn batch_syncs_each_skill_to_each_installed_tool() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = make_store(dir.path());
    seed_skill(&store, "skill-1");
    seed_skill(&store, "skill-2");
    let source_a = make_skill_dir(dir.path(), "central-a", "# A");
    let source_b = make_skill_dir(dir.path(), "central-b", "# B");

    let claude_root = dir.path().join("claude-root");
    let cursor_root = dir.path().join("cursor-root");
    let cursor = adapter_by_key("cursor").expect("cursor adapter");
    let targets = vec![
        planned(&claude(), &claude_root, true),
        planned(&cursor, &cursor_root, true),
    ];

    let skills = vec![
        batch_skill("skill-1", "skill-a", &source_a),
        batch_skill("skill-2", "skill-b", &source_b),
    ];

    let mut ticks: Vec<(usize, usize, String, String)> = Vec::new();
    let outcomes = sync_skills_to_planned_tools(
        &store,
        &skills,
        &targets,
        &BatchPolicy::default(),
        1000,
        |p| {
            ticks.push((
                p.index,
                p.total,
                p.skill_name.to_string(),
                p.tool_key.to_string(),
            ));
        },
    );

    assert_eq!(outcomes.len(), 4);
    assert!(outcomes
        .iter()
        .all(|o| matches!(o.status, BatchTargetStatus::Synced { .. })));
    assert!(claude_root.join("skill-a").exists());
    assert!(claude_root.join("skill-b").exists());
    assert!(cursor_root.join("skill-a").exists());
    assert!(cursor_root.join("skill-b").exists());

    // Progress: one 1-based tick per attempted pair, shared total.
    assert_eq!(ticks.len(), 4);
    assert_eq!(ticks[0].0, 1);
    assert_eq!(ticks[3].0, 4);
    assert!(ticks.iter().all(|t| t.1 == 4));
}

#[test]
fn batch_dedupes_installed_tools_sharing_a_root_and_fans_out_records() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = make_store(dir.path());
    seed_skill(&store, "skill-1");
    let source = make_skill_dir(dir.path(), "central-skill", "# Skill");

    let shared_root = dir.path().join("shared-root");
    let amp = adapter_by_key("amp").expect("amp adapter");
    // Both targets resolve to the same root; the second must be deduped, but
    // the first target's record_tools covers both keys.
    let mut first = planned(&claude(), &shared_root, true);
    first.record_tools = vec![claude(), amp.clone()];
    let second = planned(&amp, &shared_root, true);

    let skills = vec![batch_skill("skill-1", "my-skill", &source)];
    let outcomes = sync_skills_to_planned_tools(
        &store,
        &skills,
        &[first, second],
        &BatchPolicy::default(),
        1000,
        |_| {},
    );

    // One attempted pair only — the duplicate root produced no second outcome.
    assert_eq!(outcomes.len(), 1);
    assert!(matches!(
        outcomes[0].status,
        BatchTargetStatus::Synced { .. }
    ));
    // …but records exist for every tool sharing the dir.
    for key in ["claude_code", "amp"] {
        assert!(store
            .get_skill_target("skill-1", key)
            .expect("query")
            .is_some());
    }
}

#[test]
fn batch_skips_not_installed_tools_with_typed_reason() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = make_store(dir.path());
    seed_skill(&store, "skill-1");
    let source = make_skill_dir(dir.path(), "central-skill", "# Skill");

    let installed_root = dir.path().join("installed-root");
    let cursor = adapter_by_key("cursor").expect("cursor adapter");
    let targets = vec![
        planned(&claude(), &installed_root, true),
        planned(&cursor, &dir.path().join("absent-root"), false),
    ];

    let skills = vec![batch_skill("skill-1", "my-skill", &source)];
    let outcomes = sync_skills_to_planned_tools(
        &store,
        &skills,
        &targets,
        &BatchPolicy::default(),
        1000,
        |_| {},
    );

    assert_eq!(outcomes.len(), 2);
    let skipped = outcomes
        .iter()
        .find(|o| o.tool_key == "cursor")
        .expect("cursor outcome");
    match &skipped.status {
        BatchTargetStatus::Skipped {
            error: GlobalSyncError::ToolNotInstalled { tool_key },
        } => assert_eq!(tool_key, "cursor"),
        other => panic!("expected Skipped(ToolNotInstalled), got {:?}", other),
    }
    // The not-installed tool got no filesystem write and no record.
    assert!(store
        .get_skill_target("skill-1", "cursor")
        .expect("query")
        .is_none());
}

#[test]
fn batch_isolates_per_target_failures() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = make_store(dir.path());
    seed_skill(&store, "skill-1");
    let source = make_skill_dir(dir.path(), "central-skill", "# New");

    let blocked_root = dir.path().join("blocked-root");
    // Occupy the target path with different content and no overwrite → the
    // first pair fails with TargetExists; the second must still run.
    make_skill_dir(&blocked_root, "my-skill", "# Old");
    let clean_root = dir.path().join("clean-root");
    let cursor = adapter_by_key("cursor").expect("cursor adapter");
    let targets = vec![
        planned(&claude(), &blocked_root, true),
        planned(&cursor, &clean_root, true),
    ];

    let skills = vec![batch_skill("skill-1", "my-skill", &source)];
    let outcomes = sync_skills_to_planned_tools(
        &store,
        &skills,
        &targets,
        &BatchPolicy::default(),
        1000,
        |_| {},
    );

    assert_eq!(outcomes.len(), 2);
    assert!(matches!(
        outcomes[0].status,
        BatchTargetStatus::Failed {
            error: GlobalSyncError::TargetExists { .. }
        }
    ));
    assert!(matches!(
        outcomes[1].status,
        BatchTargetStatus::Synced { .. }
    ));
    assert!(clean_root.join("my-skill").exists());
}

#[test]
fn batch_override_applies_to_named_tool_and_its_shared_dir_group() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = make_store(dir.path());
    seed_skill(&store, "skill-1");
    let source = make_skill_dir(dir.path(), "central-skill", "# New");

    // Target adapter is claude_code; the override names amp. They do NOT
    // share a dir, so the override must not apply → TargetExists.
    let root_a = dir.path().join("root-a");
    make_skill_dir(&root_a, "my-skill", "# Old");
    let policy_neq = BatchPolicy {
        overwrite: false,
        overwrite_if_same_content: false,
        overrides: vec![BatchOverride {
            skill_id: "skill-1".to_string(),
            tool_key: "amp".to_string(),
            overwrite: true,
        }],
    };
    let skills = vec![batch_skill("skill-1", "my-skill", &source)];
    let outcomes = sync_skills_to_planned_tools(
        &store,
        &skills,
        &[planned(&claude(), &root_a, true)],
        &policy_neq,
        1000,
        |_| {},
    );
    assert!(matches!(
        outcomes[0].status,
        BatchTargetStatus::Failed {
            error: GlobalSyncError::TargetExists { .. }
        }
    ));

    // Override naming amp while the target is kimi_cli — they share
    // ~/.config/agents/skills — must apply and replace the target.
    let kimi = adapter_by_key("kimi_cli");
    let amp = adapter_by_key("amp").expect("amp adapter");
    if let Some(kimi) = kimi {
        assert_eq!(
            kimi.relative_skills_dir, amp.relative_skills_dir,
            "test premise: amp and kimi share a skills dir"
        );
        let root_b = dir.path().join("root-b");
        make_skill_dir(&root_b, "my-skill", "# Old");
        let policy_shared = BatchPolicy {
            overwrite: false,
            overwrite_if_same_content: false,
            overrides: vec![BatchOverride {
                skill_id: "skill-1".to_string(),
                tool_key: "amp".to_string(),
                overwrite: true,
            }],
        };
        let outcomes = sync_skills_to_planned_tools(
            &store,
            &skills,
            &[planned(&kimi, &root_b, true)],
            &policy_shared,
            1000,
            |_| {},
        );
        assert!(
            matches!(outcomes[0].status, BatchTargetStatus::Synced { .. }),
            "shared-dir override must apply: {:?}",
            outcomes[0].status
        );
    }
}

#[test]
fn batch_direct_override_forces_overwrite_for_that_skill_only() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = make_store(dir.path());
    seed_skill(&store, "skill-1");
    seed_skill(&store, "skill-2");
    let source_a = make_skill_dir(dir.path(), "central-a", "# New A");
    let source_b = make_skill_dir(dir.path(), "central-b", "# New B");

    let root = dir.path().join("root");
    make_skill_dir(&root, "skill-a", "# Old A");
    make_skill_dir(&root, "skill-b", "# Old B");

    let policy = BatchPolicy {
        overwrite: false,
        overwrite_if_same_content: false,
        overrides: vec![BatchOverride {
            skill_id: "skill-1".to_string(),
            tool_key: "claude_code".to_string(),
            overwrite: true,
        }],
    };
    let skills = vec![
        batch_skill("skill-1", "skill-a", &source_a),
        batch_skill("skill-2", "skill-b", &source_b),
    ];
    let outcomes = sync_skills_to_planned_tools(
        &store,
        &skills,
        &[planned(&claude(), &root, true)],
        &policy,
        1000,
        |_| {},
    );

    assert_eq!(outcomes.len(), 2);
    // skill-1 had the override → replaced; skill-2 did not → TargetExists.
    assert!(matches!(
        outcomes[0].status,
        BatchTargetStatus::Synced { .. }
    ));
    assert!(matches!(
        outcomes[1].status,
        BatchTargetStatus::Failed {
            error: GlobalSyncError::TargetExists { .. }
        }
    ));
}
