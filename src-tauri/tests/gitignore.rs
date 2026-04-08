//! Integration tests for the gitignore block manipulation logic
//! used by `update_project_gitignore` in commands/projects.rs.
//!
//! These tests reimplement the same algorithm from the implementation
//! to verify correctness of the block add/remove/idempotency behavior
//! without depending on Tauri State or IPC infrastructure.

// ---------------------------------------------------------------------------
// Test helpers — faithful reimplementation of the algorithm from
// commands/projects.rs (lines ~400-446 and ~448-465).
// ---------------------------------------------------------------------------

const MARKER: &str = "# Skills Hub";

/// Build the gitignore block exactly as the implementation does.
/// The block has a leading newline, the marker comment, pattern lines, and
/// a trailing newline.
fn build_gitignore_block(patterns: &[&str]) -> String {
    format!(
        "\n# Skills Hub \u{2014} managed skill directories\n{}\n",
        patterns.join("\n")
    )
}

/// Add the gitignore block to `existing` content.
/// Returns the new file content, or `None` if the block already exists
/// (idempotent — no-op when marker is present).
fn add_block(existing: &str, block: &str) -> Option<String> {
    if existing.contains(MARKER) {
        return None; // already present — no-op
    }
    let mut content = existing.to_string();
    if !content.ends_with('\n') && !content.is_empty() {
        content.push('\n');
    }
    content.push_str(block);
    Some(content)
}

/// Remove the Skills Hub block from content.
/// Exact copy of the `remove_block` closure in commands/projects.rs.
fn remove_block(content: &str) -> String {
    let mut lines: Vec<&str> = content.lines().collect();
    let mut start = None;
    let mut end = None;
    for (i, line) in lines.iter().enumerate() {
        if line.contains(MARKER) {
            // Include preceding blank line if present
            start = Some(if i > 0 && lines[i - 1].trim().is_empty() {
                i - 1
            } else {
                i
            });
        }
        if start.is_some() && end.is_none() && i > start.unwrap() {
            // Block continues while lines are our gitignore patterns (start with '/')
            if line.trim().is_empty() || !line.starts_with('/') {
                end = Some(i);
                break;
            }
        }
    }
    // If we found start but not end, the block runs to EOF
    if start.is_some() && end.is_none() {
        end = Some(lines.len());
    }
    if let (Some(s), Some(e)) = (start, end) {
        lines.drain(s..e);
    }
    let result = lines.join("\n");
    if result.is_empty() {
        result
    } else {
        format!("{}\n", result)
    }
}

// ---------------------------------------------------------------------------
// GAP-1 Required test case 1:
// Adding gitignore block to an empty file.
// ---------------------------------------------------------------------------

#[test]
fn test_add_block_to_empty_file() {
    let existing = "";
    let block = build_gitignore_block(&["/.claude/skills/", "/.cursor/skills/"]);
    let result = add_block(existing, &block).expect("should produce new content");

    assert!(
        result.contains(MARKER),
        "result should contain the marker comment"
    );
    assert!(
        result.contains("/.claude/skills/"),
        "result should contain claude pattern"
    );
    assert!(
        result.contains("/.cursor/skills/"),
        "result should contain cursor pattern"
    );
    // Empty file should not get double leading newlines
    // The block starts with \n, so result starts with \n on an empty file
    assert!(
        result.starts_with('\n'),
        "block starts with newline separator"
    );
}

// ---------------------------------------------------------------------------
// GAP-1 Required test case 2:
// Adding gitignore block to existing .gitignore with other content.
// ---------------------------------------------------------------------------

#[test]
fn test_add_block_to_existing_gitignore_with_content() {
    let existing = "node_modules/\n.env\n";
    let block = build_gitignore_block(&["/.claude/skills/"]);
    let result = add_block(existing, &block).expect("should produce new content");

    // Original content should be preserved
    assert!(
        result.starts_with("node_modules/\n.env\n"),
        "original content should be preserved at the start"
    );
    // New block should follow
    assert!(
        result.contains(MARKER),
        "result should contain the marker comment"
    );
    assert!(
        result.contains("/.claude/skills/"),
        "result should contain the pattern"
    );
}

#[test]
fn test_add_block_to_existing_gitignore_without_trailing_newline() {
    let existing = "node_modules/\n.env";
    let block = build_gitignore_block(&["/.claude/skills/"]);
    let result = add_block(existing, &block).expect("should produce new content");

    // Should add a newline before the block when existing content lacks trailing newline
    assert!(
        result.contains(".env\n\n# Skills Hub"),
        "should have newline separator between existing content and block, got:\n{}",
        result
    );
}

// ---------------------------------------------------------------------------
// GAP-1 Required test case 3:
// Idempotency — adding when block already exists should be a no-op.
// ---------------------------------------------------------------------------

#[test]
fn test_add_block_idempotent_when_marker_exists() {
    let existing = concat!(
        "node_modules/\n",
        "\n",
        "# Skills Hub \u{2014} managed skill directories\n",
        "/.claude/skills/\n",
    );
    let block = build_gitignore_block(&["/.claude/skills/", "/.cursor/skills/"]);
    let result = add_block(existing, &block);

    assert!(
        result.is_none(),
        "should return None (no-op) when marker already exists"
    );
}

#[test]
fn test_add_block_idempotent_with_partial_marker() {
    // Even a partial marker match (e.g. just "# Skills Hub" without the
    // em-dash suffix) should trigger idempotency, because the check is
    // `existing.contains(MARKER)` where MARKER is "# Skills Hub".
    let existing = "# Skills Hub\n/.old/pattern/\n";
    let block = build_gitignore_block(&["/.claude/skills/"]);
    let result = add_block(existing, &block);

    assert!(
        result.is_none(),
        "should return None when any form of the marker is present"
    );
}

// ---------------------------------------------------------------------------
// GAP-1 Required test case 4:
// Removing block when it's followed by unrelated content.
// This is the CR-01 scenario: the fixed algorithm should stop at
// the first non-pattern line (line that does not start with '/').
// ---------------------------------------------------------------------------

#[test]
fn test_remove_block_preserves_unrelated_content_after_block() {
    let content = concat!(
        "node_modules/\n",
        "\n",
        "# Skills Hub \u{2014} managed skill directories\n",
        "/.claude/skills/\n",
        "/.cursor/skills/\n",
        "dist/\n",
        "coverage/\n",
    );
    let result = remove_block(content);

    // The Skills Hub block should be removed (including the preceding blank line)
    assert!(
        !result.contains(MARKER),
        "marker should be removed from result"
    );
    assert!(
        !result.contains("/.claude/skills/"),
        "claude pattern should be removed"
    );
    assert!(
        !result.contains("/.cursor/skills/"),
        "cursor pattern should be removed"
    );
    // Unrelated content that follows (without blank separator) must be preserved
    assert!(
        result.contains("dist/"),
        "unrelated 'dist/' line must be preserved, got:\n{}",
        result
    );
    assert!(
        result.contains("coverage/"),
        "unrelated 'coverage/' line must be preserved, got:\n{}",
        result
    );
    // Original content before the block must be preserved
    assert!(
        result.contains("node_modules/"),
        "original content before block must be preserved"
    );
}

#[test]
fn test_remove_block_with_comment_line_after_block() {
    // A comment line (starts with '#', not '/') immediately after patterns
    // should be preserved.
    let content = concat!(
        "# Skills Hub \u{2014} managed skill directories\n",
        "/.claude/skills/\n",
        "# Some other section\n",
        "build/\n",
    );
    let result = remove_block(content);

    assert!(
        !result.contains(MARKER),
        "marker should be removed"
    );
    assert!(
        !result.contains("/.claude/skills/"),
        "claude pattern should be removed"
    );
    assert!(
        result.contains("# Some other section"),
        "unrelated comment must be preserved, got:\n{}",
        result
    );
    assert!(
        result.contains("build/"),
        "unrelated 'build/' must be preserved, got:\n{}",
        result
    );
}

// ---------------------------------------------------------------------------
// GAP-1 Required test case 5:
// Removing block when it's at end of file.
// ---------------------------------------------------------------------------

#[test]
fn test_remove_block_at_end_of_file() {
    let content = concat!(
        "node_modules/\n",
        ".env\n",
        "\n",
        "# Skills Hub \u{2014} managed skill directories\n",
        "/.claude/skills/\n",
        "/.cursor/skills/\n",
    );
    let result = remove_block(content);

    assert!(
        !result.contains(MARKER),
        "marker should be removed"
    );
    assert!(
        !result.contains("/.claude/skills/"),
        "claude pattern should be removed"
    );
    assert!(
        !result.contains("/.cursor/skills/"),
        "cursor pattern should be removed"
    );
    // Content before the block is preserved
    assert!(
        result.contains("node_modules/"),
        "content before block must be preserved"
    );
    assert!(
        result.contains(".env"),
        "content before block must be preserved"
    );
}

#[test]
fn test_remove_block_entire_file_is_block() {
    // When the file contains only the Skills Hub block
    let content = concat!(
        "# Skills Hub \u{2014} managed skill directories\n",
        "/.claude/skills/\n",
    );
    let result = remove_block(content);

    // Result should be empty (the whole file was the block)
    assert!(
        result.is_empty(),
        "result should be empty when entire file was the block, got: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// GAP-1 Required test case 6:
// Block removal with preceding blank line (should remove the blank line too).
// ---------------------------------------------------------------------------

#[test]
fn test_remove_block_includes_preceding_blank_line() {
    let content = concat!(
        "node_modules/\n",
        "\n",
        "# Skills Hub \u{2014} managed skill directories\n",
        "/.claude/skills/\n",
    );
    let result = remove_block(content);

    // The blank line before the marker should also be removed
    // Result should be just "node_modules/\n" without a trailing blank line
    assert!(
        !result.contains(MARKER),
        "marker should be removed"
    );
    // Check that the result does not have a trailing blank line
    // (the blank line that preceded the marker should have been drained)
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "should have exactly 1 line remaining (node_modules/), got {} lines: {:?}",
        lines.len(),
        lines
    );
    assert_eq!(lines[0], "node_modules/");
}

#[test]
fn test_remove_block_no_preceding_blank_line() {
    // When there is no blank line before the marker, only the marker
    // and patterns should be removed.
    let content = concat!(
        "node_modules/\n",
        "# Skills Hub \u{2014} managed skill directories\n",
        "/.claude/skills/\n",
    );
    let result = remove_block(content);

    assert!(
        !result.contains(MARKER),
        "marker should be removed"
    );
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "should have exactly 1 line remaining, got {} lines: {:?}",
        lines.len(),
        lines
    );
    assert_eq!(lines[0], "node_modules/");
}

// ---------------------------------------------------------------------------
// Additional edge case tests for completeness
// ---------------------------------------------------------------------------

#[test]
fn test_remove_block_with_multiple_patterns() {
    let content = concat!(
        "# existing comment\n",
        "*.log\n",
        "\n",
        "# Skills Hub \u{2014} managed skill directories\n",
        "/.claude/skills/\n",
        "/.cursor/skills/\n",
        "/.windsurf/skills/\n",
        "\n",
        "# build output\n",
        "dist/\n",
    );
    let result = remove_block(content);

    assert!(
        !result.contains(MARKER),
        "marker should be removed"
    );
    assert!(
        !result.contains("/.claude/skills/"),
        "claude pattern should be removed"
    );
    assert!(
        !result.contains("/.cursor/skills/"),
        "cursor pattern should be removed"
    );
    assert!(
        !result.contains("/.windsurf/skills/"),
        "windsurf pattern should be removed"
    );
    // Content around the block should be preserved
    assert!(
        result.contains("# existing comment"),
        "content before block must be preserved"
    );
    assert!(
        result.contains("*.log"),
        "content before block must be preserved"
    );
    assert!(
        result.contains("# build output"),
        "content after block must be preserved, got:\n{}",
        result
    );
    assert!(
        result.contains("dist/"),
        "content after block must be preserved"
    );
}

#[test]
fn test_remove_block_no_marker_present() {
    let content = "node_modules/\n.env\n";
    let result = remove_block(content);

    // When no marker exists, content should be unchanged (modulo trailing newline)
    assert_eq!(
        result, "node_modules/\n.env\n",
        "content without marker should be unchanged"
    );
}

#[test]
fn test_add_then_remove_roundtrip() {
    let original = "node_modules/\n.env\n";
    let block = build_gitignore_block(&["/.claude/skills/", "/.cursor/skills/"]);

    // Add the block
    let with_block = add_block(original, &block).expect("should add");
    assert!(with_block.contains(MARKER));

    // Remove the block
    let after_remove = remove_block(&with_block);

    // Should be back to the original content
    assert_eq!(
        after_remove, original,
        "roundtrip add+remove should restore original content"
    );
}

#[test]
fn test_add_then_remove_roundtrip_empty_file() {
    let original = "";
    let block = build_gitignore_block(&["/.claude/skills/"]);

    // Add the block to empty file
    let with_block = add_block(original, &block).expect("should add");
    assert!(with_block.contains(MARKER));

    // Remove the block
    let after_remove = remove_block(&with_block);

    // Should be back to empty
    assert!(
        after_remove.is_empty(),
        "roundtrip on empty file should return empty, got: {:?}",
        after_remove
    );
}

/// The block that gets added starts with \n, so when added to content
/// that already ends with \n, the result has a blank line separator.
/// This means removal should also eat that blank line.
#[test]
fn test_remove_block_blank_line_separator_from_add() {
    // Simulate what add_block produces when appending to content ending with \n
    let existing = "node_modules/\n";
    let block = build_gitignore_block(&["/.claude/skills/"]);
    let with_block = add_block(existing, &block).expect("should add");

    // The content should look like:
    // "node_modules/\n\n# Skills Hub ...\n/.claude/skills/\n"
    // which when split into lines gives:
    // ["node_modules/", "", "# Skills Hub ...", "/.claude/skills/"]
    assert!(with_block.contains(MARKER));

    let result = remove_block(&with_block);
    assert_eq!(
        result, "node_modules/\n",
        "should restore original with the blank separator also removed"
    );
}
