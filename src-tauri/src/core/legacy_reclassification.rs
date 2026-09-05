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
    _central_dir: &Path,
) -> Result<usize> {
    let mut changed = 0;
    for record in store.list_skills()? {
        if Provenance::parse(&record.source_type) != Some(Provenance::Local) {
            continue;
        }
        let Some(source) = record.source_ref.as_deref().map(PathBuf::from) else {
            continue;
        };
        let Some(tool) = tool_owning_path(home, &source) else {
            continue;
        };
        store.upsert_skill(&SkillRecord {
            source_type: Provenance::Imported.as_str().to_string(),
            source_ref: None,
            imported_from_tool: Some(tool.to_string()),
            ..record
        })?;
        changed += 1;
    }
    Ok(changed)
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
