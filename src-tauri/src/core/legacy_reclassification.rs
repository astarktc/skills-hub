//! Legacy reclassification: the once-per-launch pass that turns `local` rows
//! whose "source" was really a Tool's skills directory into `imported` rows
//! (see **Provenance** in `CONTEXT.md`, ADR-0003, round-4 spec Q5).
//!
//! Until v1.2.3 Onboarding import recorded the Tool path it copied from as a
//! `local` source — a path the import itself then replaced with a link or
//! removed. Such a row fails every Refresh forever. This pass runs after
//! `ensure_schema` with explicit roots (no environment reads), is idempotent,
//! and never touches a `local` row whose folder is the operator's own.

use std::path::{Path, PathBuf};

use anyhow::Result;

use super::provenance::Provenance;
use super::skill_store::{SkillRecord, SkillStore};
use super::tool_adapters::{default_tool_adapters, skills_dir_in};

/// Reclassify every `local` row whose stored source path was really a Tool's
/// skills directory as `imported`. Returns how many rows changed.
pub fn reclassify_legacy_imports(
    store: &SkillStore,
    home: &Path,
    central_dir: &Path,
) -> Result<usize> {
    // Canonical once: on macOS a temp `/var/...` central dir is really
    // `/private/var/...`, and a link's resolved target is always canonical.
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

/// The evidence that a stored `local` source was really an import: which
/// Tool it was found in, when the path names one.
struct ImportEvidence {
    found_in_tool: Option<&'static str>,
}

/// Spec Q5's three rules, in order. `None` means "leave the row alone".
fn classify(
    home: &Path,
    canonical_central: Option<&Path>,
    source: &Path,
) -> Option<ImportEvidence> {
    if let Some(tool) = tool_owning_path(home, source) {
        return Some(ImportEvidence {
            found_in_tool: Some(tool),
        });
    }
    if links_into(source, canonical_central) {
        return Some(ImportEvidence { found_in_tool: None });
    }
    None
}

/// Rule (ii): `path` is a symlink whose resolved target lies inside
/// `canonical_central` — the "source" is the skill's own central copy.
fn links_into(path: &Path, canonical_central: Option<&Path>) -> bool {
    let Some(central) = canonical_central else {
        return false;
    };
    let is_link = std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink());
    is_link
        && std::fs::canonicalize(path).is_ok_and(|resolved| resolved.starts_with(central))
}

/// Rule (i): the Tool whose global skills dir under `home` strictly contains
/// `path` — the first in registry order when Tools share a directory.
fn tool_owning_path(home: &Path, path: &Path) -> Option<&'static str> {
    default_tool_adapters()
        .iter()
        .find(|adapter| {
            path.strip_prefix(skills_dir_in(home, adapter))
                .is_ok_and(|rest| rest.components().next().is_some())
        })
        .map(|adapter| adapter.key())
}

#[cfg(test)]
#[path = "tests/legacy_reclassification.rs"]
mod tests;
