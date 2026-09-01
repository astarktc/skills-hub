//! Maintenance of the managed "Skills Hub" block in a project's `.gitignore`
//! and `.git/info/exclude`.
//!
//! The ignore patterns are derived from `ToolAdapter::project_relative_skills_dir` — the
//! same mapping project sync writes with — so the dir-mapping decision is made
//! exactly once, here. (An earlier version derived patterns from the *global*
//! `relative_skills_dir`, producing entries that never matched what project
//! sync actually wrote for tools like Windsurf/Pi/Goose/Augment.)
//!
//! Writes are idempotent rewrites: any existing managed block (including one
//! with stale or wrong patterns) is stripped and replaced by the current
//! block, so previously mis-written entries self-heal on the next update.

use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::core::errors::SignalError;
use crate::core::skill_store::SkillStore;
use crate::core::tool_adapters::{adapter_by_key, ToolAdapter};

/// Marker identifying the managed block. Any line containing this string
/// starts a managed block; the block extends over the following pattern
/// lines (lines starting with `/`).
pub const MARKER: &str = "# Skills Hub";

/// Ignore patterns for a set of configured tools, deduplicated, in input
/// order. Uses the project-scope mapping (`ToolAdapter::project_relative_skills_dir`).
pub fn patterns_for_tools<'a>(adapters: impl IntoIterator<Item = &'a ToolAdapter>) -> Vec<String> {
    let mut patterns: Vec<String> = Vec::new();
    for adapter in adapters {
        let pattern = format!("/{}/", adapter.project_relative_skills_dir);
        if !patterns.contains(&pattern) {
            patterns.push(pattern);
        }
    }
    patterns
}

/// The managed block exactly as written to disk: a leading blank line, the
/// marker comment, one pattern per line, and a trailing newline.
pub fn managed_block(patterns: &[String]) -> String {
    format!(
        "\n{MARKER} — managed skill directories\n{}\n",
        patterns.join("\n")
    )
}

/// Remove ALL managed blocks from `content`.
///
/// A block is: an optional preceding blank line, the marker comment line, and
/// all immediately following lines that start with `/` (our gitignore
/// patterns). Handles multiple blocks if present (e.g. from a double-write).
pub fn remove_managed_block(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result: Vec<&str> = Vec::new();
    let mut in_block = false;
    for (i, line) in lines.iter().enumerate() {
        if line.contains(MARKER) {
            in_block = true;
            // Remove preceding blank line if we just pushed one
            if let Some(last) = result.last() {
                if last.trim().is_empty() {
                    result.pop();
                }
            }
            continue;
        }
        if in_block {
            // Block continues while lines are our gitignore patterns (start with '/')
            // or are blank lines between patterns within the block
            if line.starts_with('/') {
                continue;
            }
            // A trailing blank line right after the last pattern belongs to the block
            if line.trim().is_empty() {
                // Peek ahead: if the next non-empty line is also a pattern or marker, skip
                // Otherwise this blank separates from unrelated content — keep it
                let next_non_empty = lines[i + 1..].iter().find(|l| !l.trim().is_empty());
                if let Some(next) = next_non_empty {
                    if next.starts_with('/') || next.contains(MARKER) {
                        continue;
                    }
                } else {
                    // Blank line at EOF after block — skip it
                    continue;
                }
            }
            in_block = false;
        }
        result.push(line);
    }
    let joined = result.join("\n");
    if joined.is_empty() {
        joined
    } else {
        format!("{}\n", joined)
    }
}

/// Idempotent rewrite: strip any existing managed block(s), then append the
/// current block for `patterns`. Unlike a skip-if-present write, this updates
/// stale blocks whenever the tool list (or the pattern mapping) changes.
pub fn set_managed_block(content: &str, patterns: &[String]) -> String {
    let mut out = remove_managed_block(content);
    if !out.ends_with('\n') && !out.is_empty() {
        out.push('\n');
    }
    out.push_str(&managed_block(patterns));
    out
}

/// Add (`enabled = true`, idempotent rewrite) or remove (`enabled = false`)
/// the managed block in the ignore file at `path`. Writes only when the
/// content actually changes. `create_parents` is for `.git/info/exclude`,
/// whose directory may not exist yet.
fn apply_to_file(
    path: &Path,
    patterns: &[String],
    enabled: bool,
    create_parents: bool,
) -> Result<()> {
    if enabled && !patterns.is_empty() {
        if create_parents {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
        }
        let existing = if path.exists() {
            fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?
        } else {
            String::new()
        };
        let updated = set_managed_block(&existing, patterns);
        if updated != existing {
            fs::write(path, updated)
                .with_context(|| format!("failed to write {}", path.display()))?;
        }
    } else if path.exists() {
        let existing = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if existing.contains(MARKER) {
            let cleaned = remove_managed_block(&existing);
            fs::write(path, cleaned)
                .with_context(|| format!("failed to write {}", path.display()))?;
        }
    }
    Ok(())
}

/// Status of the managed block in a project's ignore files, as detected
/// beside the writer (same `MARKER`).
pub struct IgnoreStatus {
    pub in_gitignore: bool,
    pub in_exclude: bool,
}

/// Whether a managed block is currently present in the project's
/// `.gitignore` and `.git/info/exclude`.
pub fn project_ignore_status(project_path: &Path) -> IgnoreStatus {
    let has_marker = |path: &Path| {
        path.exists()
            && fs::read_to_string(path)
                .unwrap_or_default()
                .contains(MARKER)
    };
    IgnoreStatus {
        in_gitignore: has_marker(&project_path.join(".gitignore")),
        in_exclude: has_marker(&project_path.join(".git").join("info").join("exclude")),
    }
}

/// Update the managed block in a project's `.gitignore` and
/// `.git/info/exclude` according to the two toggles. A project with no
/// resolvable patterns (e.g. its last tool was removed) has any existing
/// managed block stripped, so stale entries self-heal on any toggle/edit;
/// no files are created in that case.
pub fn update_project_ignore_files(
    project_path: &Path,
    patterns: &[String],
    add_to_gitignore: bool,
    add_to_exclude: bool,
) -> Result<()> {
    apply_to_file(
        &project_path.join(".gitignore"),
        patterns,
        add_to_gitignore,
        false,
    )?;
    let exclude_path = project_path.join(".git").join("info").join("exclude");
    apply_to_file(&exclude_path, patterns, add_to_exclude, true)?;
    Ok(())
}

/// The two toggles behind `update_project_gitignore`.
#[derive(Debug, Clone, Copy)]
pub struct IgnoreUpdateOptions {
    pub add_to_gitignore: bool,
    pub add_to_exclude: bool,
}

/// Update a registered project's ignore files from its *persisted* tool
/// list: look the project up (typed `NotFound`), require its directory to
/// exist, derive patterns from the tools' adapters (unknown keys are
/// ignored), and apply [`update_project_ignore_files`].
pub fn update_for_project(
    store: &SkillStore,
    project_id: &str,
    options: IgnoreUpdateOptions,
) -> Result<()> {
    let project = store.get_project_by_id(project_id)?.ok_or_else(|| {
        anyhow::anyhow!(SignalError::NotFound {
            kind: "project".to_string(),
            id: project_id.to_string(),
        })
    })?;

    let project_path = Path::new(&project.path);
    if !project_path.is_dir() {
        bail!("project directory does not exist: {}", project.path);
    }

    let tools = store.list_project_tools(project_id)?;
    let adapters: Vec<ToolAdapter> = tools
        .iter()
        .filter_map(|t| adapter_by_key(&t.tool))
        .collect();
    let patterns = patterns_for_tools(adapters.iter());

    update_project_ignore_files(
        project_path,
        &patterns,
        options.add_to_gitignore,
        options.add_to_exclude,
    )
}

#[cfg(test)]
#[path = "tests/gitignore.rs"]
mod tests;
