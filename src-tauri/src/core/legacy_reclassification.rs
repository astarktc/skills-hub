//! Legacy reclassification: the once-per-launch pass that turns `local` rows
//! whose "source" was really a Tool's skills directory into `imported` rows
//! (see **Provenance** in `CONTEXT.md`, ADR-0003).
//!
//! Until v1.2.3 Onboarding import recorded the Tool path it copied from as a
//! `local` source — a path the import itself then replaced with a link or
//! removed. Such a row fails every Refresh forever. This pass runs after
//! `ensure_schema` with explicit roots (no environment reads), is idempotent,
//! never touches a `local` row whose folder is the operator's own, and logs
//! every row it changes with the source it discards — nothing is dropped
//! without a trace.

use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::Result;

use super::provenance::Provenance;
use super::skill_store::{SkillRecord, SkillStore};
use super::tool_adapters::{default_tool_adapters, skills_dir_in, tool_holding_path};

/// Reclassify every `local` row whose stored source path was really a Tool's
/// skills directory as `imported`. Returns how many rows changed.
pub fn reclassify_legacy_imports(
    store: &SkillStore,
    home: &Path,
    central_dir: &Path,
) -> Result<usize> {
    // Canonical once: on macOS a temp `/var/...` central dir is really
    // `/private/var/...`, and a resolved path is always canonical.
    let canonical_central = std::fs::canonicalize(central_dir).ok();
    let mut changed = 0;
    for record in store.list_skills()? {
        if Provenance::parse(&record.source_type) != Some(Provenance::Local) {
            continue;
        }
        let Some(source) = record.source_ref.as_deref().map(PathBuf::from) else {
            continue;
        };
        let Some(verdict) = classify(home, canonical_central.as_deref(), &source) else {
            continue;
        };
        log::info!(
            "reclassified {} as imported (rule {}); former source {}",
            record.name,
            verdict.rule,
            source.display()
        );
        store.upsert_skill(&SkillRecord {
            source_type: Provenance::Imported.as_str().to_string(),
            source_ref: None,
            imported_from_tool: verdict.found_in_tool.map(str::to_string),
            ..record
        })?;
        changed += 1;
    }
    Ok(changed)
}

/// Which of the three rules decided that a stored `local` source was really
/// an import. Logged with every changed row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rule {
    /// (i) The path lies inside a Tool's global skills dir under this home.
    InsideToolDir,
    /// (ii) The path resolves into the central repo — the "source" is the
    /// skill's own central copy.
    ResolvesIntoCentral,
    /// (iii) The path is gone and has the shape of a Tool's global skills
    /// dir under some other home.
    ToolDirShape,
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Rule::InsideToolDir => "i",
            Rule::ResolvesIntoCentral => "ii",
            Rule::ToolDirShape => "iii",
        })
    }
}

/// The evidence that a stored `local` source was really an import: the rule
/// that fired, and which Tool it was found in when the path names one.
struct ImportEvidence {
    rule: Rule,
    found_in_tool: Option<&'static str>,
}

/// The three rules, in order. `None` means "leave the row alone".
fn classify(
    home: &Path,
    canonical_central: Option<&Path>,
    source: &Path,
) -> Option<ImportEvidence> {
    // A Tool's skills dir itself names no skill; require a strict descendant.
    if let Some(tool) =
        tool_holding_path(home, source).filter(|adapter| source != skills_dir_in(home, adapter))
    {
        return Some(ImportEvidence {
            rule: Rule::InsideToolDir,
            found_in_tool: Some(tool.key()),
        });
    }
    if resolves_into(source, canonical_central) {
        // The path itself may still name the Tool it sat in (a Tool dir
        // outside this `home`); when it does not, there is no history to keep.
        return Some(ImportEvidence {
            rule: Rule::ResolvesIntoCentral,
            found_in_tool: tool_by_home_shaped_prefix(source),
        });
    }
    // Rule (i) is the whole answer for a path under this home: one that is
    // not inside a Tool's global skills dir is the operator's own, whether or
    // not it exists right now (a removed worktree is not an import).
    if !source.starts_with(home) && !source.exists() {
        if let Some(tool) = tool_by_home_shaped_prefix(source) {
            return Some(ImportEvidence {
                rule: Rule::ToolDirShape,
                found_in_tool: Some(tool),
            });
        }
    }
    None
}

/// Rule (ii): `path` resolves to a location inside `canonical_central` —
/// canonical containment is the rule, so a link anywhere on the path (the
/// leaf or an ancestor) counts. A path that does not resolve (it is gone)
/// never matches.
fn resolves_into(path: &Path, canonical_central: Option<&Path>) -> bool {
    let Some(central) = canonical_central else {
        return false;
    };
    std::fs::canonicalize(path).is_ok_and(|resolved| resolved.starts_with(central))
}

/// Rule (iii): `path` has the shape of a Tool's *global* skills-dir entry
/// under some home — its components contain an adapter's relative global
/// skills dir as a contiguous run followed by exactly one more component
/// (the skill name), and the run is immediately preceded by a home-shaped
/// prefix: `Users/<name>` or `home/<name>` (under any earlier prefix, so
/// `/mnt/c/Users/x` and `C:\Users\x` count), exactly `root`, or exactly `~`.
/// `.claude/skills` and `.agents/skills` are project-scope dirs too, and a
/// run preceded by a project directory is not this rule's business.
/// Separators are normalised so Windows spellings match. First adapter in
/// registry order wins when Tools share a directory.
fn tool_by_home_shaped_prefix(path: &Path) -> Option<&'static str> {
    let normalised = path.to_string_lossy().replace('\\', "/");
    let components: Vec<&str> = normalised.split('/').filter(|c| !c.is_empty()).collect();
    default_tool_adapters()
        .iter()
        .find(|adapter| {
            let dir: Vec<&str> = adapter.relative_skills_dir.split('/').collect();
            // The run must end exactly one component before the path's end.
            if components.len() <= dir.len() {
                return false;
            }
            let run_start = components.len() - 1 - dir.len();
            components[run_start..components.len() - 1] == dir[..]
                && home_shaped_prefix(&components[..run_start])
        })
        .map(|adapter| adapter.key())
}

/// Does `prefix` end the way a home directory is spelled?
fn home_shaped_prefix(prefix: &[&str]) -> bool {
    matches!(prefix, [.., "Users" | "home", _] | ["root"] | ["~"])
}

#[cfg(test)]
#[path = "tests/legacy_reclassification.rs"]
mod tests;
