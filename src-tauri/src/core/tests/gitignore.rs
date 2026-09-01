//! Tests for `core::gitignore` — managed block add/remove/rewrite semantics,
//! pattern derivation, and file-level orchestration.
//!
//! Replaces the old `src-tauri/tests/gitignore.rs`, which reimplemented the
//! algorithm (then living inside a Tauri command) instead of testing it.

use std::fs;

use crate::core::gitignore::{
    managed_block, patterns_for_tools, project_ignore_status, remove_managed_block,
    set_managed_block, update_project_ignore_files, MARKER,
};
use crate::core::tool_adapters::adapter_by_key;

fn pat(strs: &[&str]) -> Vec<String> {
    strs.iter().map(|s| s.to_string()).collect()
}

// ---------------------------------------------------------------------------
// patterns_for_tools — the dir-mapping decision
// ---------------------------------------------------------------------------

#[test]
fn patterns_use_project_scope_mapping_not_global() {
    // Windsurf global dir is .codeium/windsurf/skills, project dir is .windsurf/skills.
    // The gitignore patterns must match what project sync actually writes.
    let windsurf = adapter_by_key("windsurf").expect("windsurf adapter");
    assert_ne!(
        windsurf.relative_skills_dir, windsurf.project_relative_skills_dir,
        "test premise: windsurf global and project dirs differ"
    );
    let patterns = patterns_for_tools([&windsurf]);
    assert_eq!(patterns, vec!["/.windsurf/skills/".to_string()]);
}

#[test]
fn patterns_for_divergent_tools_all_use_project_dirs() {
    for key in ["pi", "goose", "augment"] {
        let adapter = adapter_by_key(key).expect(key);
        let patterns = patterns_for_tools([&adapter]);
        assert_eq!(
            patterns,
            vec![format!("/{}/", adapter.project_relative_skills_dir)],
            "wrong pattern for {}",
            key
        );
    }
}

#[test]
fn patterns_dedupe_tools_sharing_a_project_dir() {
    // claude_code keeps its own dir; codex and cursor both map to .agents/skills.
    let adapters: Vec<_> = ["claude_code", "codex", "cursor"]
        .iter()
        .map(|k| adapter_by_key(k).expect(k))
        .collect();
    let patterns = patterns_for_tools(adapters.iter());
    assert_eq!(
        patterns,
        vec![
            "/.claude/skills/".to_string(),
            "/.agents/skills/".to_string()
        ]
    );
}

// ---------------------------------------------------------------------------
// set_managed_block — idempotent rewrite
// ---------------------------------------------------------------------------

#[test]
fn set_on_empty_content_appends_block() {
    let patterns = pat(&["/.claude/skills/"]);
    let out = set_managed_block("", &patterns);
    assert_eq!(out, managed_block(&patterns));
    assert!(out.contains(MARKER));
    assert!(out.ends_with('\n'));
}

#[test]
fn set_preserves_existing_content() {
    let existing = "node_modules/\n.env\n";
    let patterns = pat(&["/.claude/skills/"]);
    let out = set_managed_block(existing, &patterns);
    assert!(out.starts_with("node_modules/\n.env\n"));
    assert!(out.contains(&format!("{MARKER} — managed skill directories")));
    assert!(out.contains("/.claude/skills/"));
}

#[test]
fn set_adds_newline_when_content_lacks_trailing_newline() {
    let existing = "node_modules/\n.env";
    let out = set_managed_block(existing, &pat(&["/.claude/skills/"]));
    assert!(out.contains(".env\n\n# Skills Hub"));
}

#[test]
fn set_is_idempotent() {
    let patterns = pat(&["/.claude/skills/", "/.agents/skills/"]);
    let once = set_managed_block("dist/\n", &patterns);
    let twice = set_managed_block(&once, &patterns);
    assert_eq!(once, twice);
    assert_eq!(twice.matches(MARKER).count(), 1);
}

#[test]
fn set_rewrites_stale_wrong_patterns() {
    // Migration case: an earlier version wrote GLOBAL dir patterns
    // (e.g. /.codeium/windsurf/skills/). A rewrite must replace them.
    let existing = "dist/\n\n# Skills Hub — managed skill directories\n/.codeium/windsurf/skills/\n/.pi/agent/skills/\n";
    let patterns = pat(&["/.windsurf/skills/", "/.pi/skills/"]);
    let out = set_managed_block(existing, &patterns);
    assert!(!out.contains("/.codeium/windsurf/skills/"));
    assert!(!out.contains("/.pi/agent/skills/"));
    assert!(out.contains("/.windsurf/skills/"));
    assert!(out.contains("/.pi/skills/"));
    assert_eq!(out.matches(MARKER).count(), 1);
    assert!(out.starts_with("dist/\n"));
}

#[test]
fn set_updates_block_when_tool_list_changes() {
    let v1 = set_managed_block("", &pat(&["/.claude/skills/"]));
    let v2 = set_managed_block(&v1, &pat(&["/.claude/skills/", "/.goose/skills/"]));
    assert!(v2.contains("/.goose/skills/"));
    assert_eq!(v2.matches(MARKER).count(), 1);
}

#[test]
fn set_collapses_duplicate_blocks_from_old_double_writes() {
    let patterns = pat(&["/.claude/skills/"]);
    let block = managed_block(&patterns);
    let doubled = format!("src/\n{}{}", block, block);
    let out = set_managed_block(&doubled, &patterns);
    assert_eq!(out.matches(MARKER).count(), 1);
}

// ---------------------------------------------------------------------------
// remove_managed_block — ported behavior from the old integration tests
// ---------------------------------------------------------------------------

#[test]
fn remove_preserves_unrelated_content_after_block() {
    let content = "node_modules/\n\n# Skills Hub — managed skill directories\n/.claude/skills/\n\n# other stuff\n*.log\n";
    let out = remove_managed_block(content);
    assert_eq!(out, "node_modules/\n\n# other stuff\n*.log\n");
}

#[test]
fn remove_block_at_end_of_file() {
    let content = "dist/\n\n# Skills Hub — managed skill directories\n/.claude/skills/\n";
    let out = remove_managed_block(content);
    assert_eq!(out, "dist/\n");
}

#[test]
fn remove_when_entire_file_is_block() {
    let content = format!("{MARKER} — managed skill directories\n/.claude/skills/\n");
    let content = content.as_str();
    let out = remove_managed_block(content);
    assert_eq!(out, "");
}

#[test]
fn remove_handles_multiple_patterns() {
    let content = "a/\n\n# Skills Hub — managed skill directories\n/.claude/skills/\n/.agents/skills/\n/.goose/skills/\nb/\n";
    let out = remove_managed_block(content);
    assert_eq!(out, "a/\nb/\n");
}

#[test]
fn remove_no_marker_is_untouched() {
    let content = "node_modules/\n.env\n";
    assert_eq!(remove_managed_block(content), content);
}

#[test]
fn set_then_remove_roundtrip() {
    let existing = "node_modules/\n.env\n";
    let patterns = pat(&["/.claude/skills/", "/.agents/skills/"]);
    let with_block = set_managed_block(existing, &patterns);
    let out = remove_managed_block(&with_block);
    assert_eq!(out, existing);
}

#[test]
fn set_then_remove_roundtrip_empty_file() {
    let with_block = set_managed_block("", &pat(&["/.claude/skills/"]));
    assert_eq!(remove_managed_block(&with_block), "");
}

// ---------------------------------------------------------------------------
// update_project_ignore_files — file-level orchestration
// ---------------------------------------------------------------------------

#[test]
fn file_update_writes_gitignore_and_exclude() {
    let dir = tempfile::tempdir().expect("tempdir");
    let patterns = pat(&["/.claude/skills/"]);
    update_project_ignore_files(dir.path(), &patterns, true, true).expect("update");

    let gitignore = fs::read_to_string(dir.path().join(".gitignore")).expect("gitignore");
    assert!(gitignore.contains("/.claude/skills/"));
    // exclude parent dirs are created on demand
    let exclude = fs::read_to_string(dir.path().join(".git/info/exclude")).expect("exclude");
    assert!(exclude.contains("/.claude/skills/"));
}

#[test]
fn file_update_disabled_removes_block_and_keeps_rest() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(dir.path().join(".gitignore"), "dist/\n").expect("seed");
    let patterns = pat(&["/.claude/skills/"]);
    update_project_ignore_files(dir.path(), &patterns, true, false).expect("add");
    update_project_ignore_files(dir.path(), &patterns, false, false).expect("remove");

    let gitignore = fs::read_to_string(dir.path().join(".gitignore")).expect("gitignore");
    assert_eq!(gitignore, "dist/\n");
    assert!(
        !dir.path().join(".git").exists(),
        "disabled exclude must not create .git"
    );
}

#[test]
fn file_update_repairs_stale_wrong_block_on_rewrite() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(
        dir.path().join(".gitignore"),
        "dist/\n\n# Skills Hub — managed skill directories\n/.codeium/windsurf/skills/\n",
    )
    .expect("seed stale block");
    update_project_ignore_files(dir.path(), &pat(&["/.windsurf/skills/"]), true, false)
        .expect("update");

    let gitignore = fs::read_to_string(dir.path().join(".gitignore")).expect("gitignore");
    assert!(!gitignore.contains("/.codeium/windsurf/skills/"));
    assert!(gitignore.contains("/.windsurf/skills/"));
}

#[test]
fn file_update_with_no_patterns_is_a_no_op() {
    let dir = tempfile::tempdir().expect("tempdir");
    update_project_ignore_files(dir.path(), &[], true, true).expect("update");
    assert!(!dir.path().join(".gitignore").exists());
    assert!(!dir.path().join(".git").exists());
}

#[test]
fn file_update_empty_patterns_strips_stale_block() {
    // Ticket 14: removing a project's last tool must let the stale managed
    // block self-heal on the next toggle/edit.
    let dir = tempfile::tempdir().expect("tempdir");
    let patterns = pat(&["/.claude/skills/"]);
    fs::write(dir.path().join(".gitignore"), "node_modules/\n").expect("seed");
    update_project_ignore_files(dir.path(), &patterns, true, true).expect("add");
    assert!(fs::read_to_string(dir.path().join(".gitignore"))
        .expect("read")
        .contains(MARKER));

    // Last tool removed → empty patterns, toggles still true.
    update_project_ignore_files(dir.path(), &[], true, true).expect("strip");
    let gitignore = fs::read_to_string(dir.path().join(".gitignore")).expect("read");
    assert!(!gitignore.contains(MARKER), "stale block must be stripped");
    assert!(
        gitignore.contains("node_modules/"),
        "unrelated content kept"
    );
    let exclude = fs::read_to_string(dir.path().join(".git/info/exclude")).expect("read");
    assert!(!exclude.contains(MARKER));
}

#[test]
fn status_detection_agrees_with_writer() {
    let dir = tempfile::tempdir().expect("tempdir");
    let status = project_ignore_status(dir.path());
    assert!(!status.in_gitignore);
    assert!(!status.in_exclude);

    let patterns = pat(&["/.claude/skills/"]);
    update_project_ignore_files(dir.path(), &patterns, true, false).expect("add gitignore");
    let status = project_ignore_status(dir.path());
    assert!(status.in_gitignore);
    assert!(!status.in_exclude);

    update_project_ignore_files(dir.path(), &patterns, true, true).expect("add exclude");
    let status = project_ignore_status(dir.path());
    assert!(status.in_gitignore);
    assert!(status.in_exclude);

    update_project_ignore_files(dir.path(), &patterns, false, false).expect("remove");
    let status = project_ignore_status(dir.path());
    assert!(!status.in_gitignore);
    assert!(!status.in_exclude);
}

#[test]
fn file_update_skips_write_when_unchanged() {
    let dir = tempfile::tempdir().expect("tempdir");
    let patterns = pat(&["/.claude/skills/"]);
    update_project_ignore_files(dir.path(), &patterns, true, false).expect("first");
    let path = dir.path().join(".gitignore");
    let before = fs::metadata(&path)
        .expect("meta")
        .modified()
        .expect("mtime");
    let content_before = fs::read_to_string(&path).expect("read");
    update_project_ignore_files(dir.path(), &patterns, true, false).expect("second");
    let content_after = fs::read_to_string(&path).expect("read");
    assert_eq!(content_before, content_after);
    let after = fs::metadata(&path)
        .expect("meta")
        .modified()
        .expect("mtime");
    assert_eq!(before, after, "unchanged content must not be rewritten");
}

// ---------------------------------------------------------------------------
// update_for_project — project lookup + pattern derivation from the
// persisted tool list, composed over the file-level writer.
// ---------------------------------------------------------------------------

use crate::core::errors::SignalError;
use crate::core::gitignore::{update_for_project, IgnoreUpdateOptions};
use crate::core::skill_store::{ProjectRecord, ProjectToolRecord, SkillStore};

fn make_store(base: &std::path::Path) -> SkillStore {
    let store = SkillStore::new(base.join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    store
}

fn seed_project(store: &SkillStore, path: &std::path::Path, tools: &[&str]) -> ProjectRecord {
    let project = ProjectRecord {
        id: uuid::Uuid::new_v4().to_string(),
        path: path.to_string_lossy().to_string(),
        created_at: 1,
        updated_at: 1,
    };
    store.register_project(&project).unwrap();
    for tool in tools {
        store
            .add_project_tool(&ProjectToolRecord {
                id: uuid::Uuid::new_v4().to_string(),
                project_id: project.id.clone(),
                tool: tool.to_string(),
            })
            .unwrap();
    }
    project
}

#[test]
fn project_update_derives_patterns_from_persisted_tools() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let project_dir = tmp.path().join("proj");
    fs::create_dir_all(&project_dir).unwrap();
    let project = seed_project(&store, &project_dir, &["claude_code", "windsurf"]);

    update_for_project(
        &store,
        &project.id,
        IgnoreUpdateOptions {
            add_to_gitignore: true,
            add_to_exclude: true,
        },
    )
    .expect("update");

    let gitignore = fs::read_to_string(project_dir.join(".gitignore")).unwrap();
    assert!(gitignore.contains(MARKER));
    assert!(gitignore.contains("/.claude/skills/"));
    assert!(
        gitignore.contains("/.windsurf/skills/"),
        "project-scope mapping"
    );
    let exclude = fs::read_to_string(project_dir.join(".git/info/exclude")).unwrap();
    assert!(exclude.contains("/.windsurf/skills/"));
}

#[test]
fn project_update_honours_toggles() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let project_dir = tmp.path().join("proj");
    fs::create_dir_all(&project_dir).unwrap();
    let project = seed_project(&store, &project_dir, &["claude_code"]);

    update_for_project(
        &store,
        &project.id,
        IgnoreUpdateOptions {
            add_to_gitignore: true,
            add_to_exclude: false,
        },
    )
    .unwrap();
    assert!(project_dir.join(".gitignore").exists());
    assert!(!project_dir.join(".git/info/exclude").exists());

    update_for_project(
        &store,
        &project.id,
        IgnoreUpdateOptions {
            add_to_gitignore: false,
            add_to_exclude: false,
        },
    )
    .unwrap();
    let status = project_ignore_status(&project_dir);
    assert!(!status.in_gitignore && !status.in_exclude);
}

#[test]
fn project_update_ignores_unknown_tool_keys() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let project_dir = tmp.path().join("proj");
    fs::create_dir_all(&project_dir).unwrap();
    let project = seed_project(&store, &project_dir, &["claude_code", "not-a-tool"]);

    update_for_project(
        &store,
        &project.id,
        IgnoreUpdateOptions {
            add_to_gitignore: true,
            add_to_exclude: false,
        },
    )
    .unwrap();

    let gitignore = fs::read_to_string(project_dir.join(".gitignore")).unwrap();
    assert!(gitignore.contains("/.claude/skills/"));
    assert!(!gitignore.contains("not-a-tool"));
}

#[test]
fn project_update_raises_typed_not_found_for_unknown_project() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());

    let err = update_for_project(
        &store,
        "missing",
        IgnoreUpdateOptions {
            add_to_gitignore: true,
            add_to_exclude: true,
        },
    )
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
fn project_update_rejects_a_project_whose_directory_is_gone() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let project_dir = tmp.path().join("vanished");
    let project = seed_project(&store, &project_dir, &["claude_code"]);

    let err = update_for_project(
        &store,
        &project.id,
        IgnoreUpdateOptions {
            add_to_gitignore: true,
            add_to_exclude: true,
        },
    )
    .expect_err("must fail");
    assert!(format!("{:#}", err).contains("project directory does not exist"));
    assert!(
        !project_dir.exists(),
        "nothing is created for a missing project dir"
    );
}
