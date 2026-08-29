//! Tests for `core::gitignore` — managed block add/remove/rewrite semantics,
//! pattern derivation, and file-level orchestration.
//!
//! Replaces the old `src-tauri/tests/gitignore.rs`, which reimplemented the
//! algorithm (then living inside a Tauri command) instead of testing it.

use std::fs;

use crate::core::gitignore::{
    managed_block, patterns_for_tools, remove_managed_block, set_managed_block,
    update_project_ignore_files, MARKER,
};
use crate::core::tool_adapters::{adapter_by_key, project_relative_skills_dir};

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
        windsurf.relative_skills_dir,
        project_relative_skills_dir(&windsurf),
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
            vec![format!("/{}/", project_relative_skills_dir(&adapter))],
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
    assert!(out.contains("# Skills Hub — managed skill directories"));
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
    let content = "# Skills Hub — managed skill directories\n/.claude/skills/\n";
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
