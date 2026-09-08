//! Unlocatable skill: a Managed skill the app can no longer locate (see
//! **Unlocatable skill** in `CONTEXT.md`).
//!
//! Two states, computed from the recorded paths at listing time and never
//! stored:
//!
//! - `source_missing` — a `local` skill whose folder is gone. Repairs:
//!   Re-point ([`repoint_and_update`] — the single-skill Update lands the new
//!   folder's bytes and source together), Detach
//!   ([`detach_from_source`] — it becomes `imported`, the central copy is
//!   truth from now on), or Remove.
//! - `central_missing` — the central copy is gone, so every Tool's link for
//!   it is dangling. Repairs: Restore (the single-skill Update, which
//!   rebuilds the central copy from a `git`/`local` source) or Remove.
//!
//! [`unlocatable_state`] is the **one** rule; Refresh (all) consults it
//! through `provenance::refresh_eligibility` to skip such a skill instead of
//! failing it, and the Managed-skill listing exposes it on the DTO.

use std::path::Path;

use anyhow::Result;

use super::cancel_token::CancelToken;
use super::errors::SignalError;
use super::installer::InstallerPaths;
use super::provenance::Provenance;
use super::refresh::{
    refresh_managed_skills_with, RefreshPolicy, RefreshProgress, RefreshReport, RefreshSelection,
};
use super::skill_discovery::require_skill_md;
use super::skill_store::{SkillRecord, SkillStore};
use super::tool_adapters::tool_holding_path;

/// Which of a skill's recorded paths is gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum UnlocatableState {
    /// A `local` skill's source folder is not there.
    SourceMissing,
    /// The central copy is not there.
    CentralMissing,
}

/// Is this skill unlocatable, and how? `None` for a skill whose recorded
/// paths are all present.
///
/// Presence is what the Update path would see (`Path::exists`, following
/// links), so the listing and the acquire step agree by construction: a
/// dangling link is missing. Both paths gone reports `source_missing`,
/// because Re-point is the repair that fixes both (its Update rebuilds the
/// central copy), while Restore alone would fail on the missing source.
pub fn unlocatable_state(record: &SkillRecord) -> Option<UnlocatableState> {
    if Provenance::parse(&record.source_type) == Some(Provenance::Local) {
        let source_present = record
            .source_ref
            .as_deref()
            .is_some_and(|source| Path::new(source).exists());
        if !source_present {
            return Some(UnlocatableState::SourceMissing);
        }
    }
    if !Path::new(&record.central_path).exists() {
        return Some(UnlocatableState::CentralMissing);
    }
    None
}

/// Re-point a `local` skill at its folder's new location and update from
/// it — the one Re-point operation. The folder is validated the way Add
/// validates one (present, holds a `SKILL.md`, not inside a Tool's skills
/// directory). The single-skill Update (the Refresh batch of one, which takes
/// the Mutation guard itself) settles the new source and bytes together,
/// propagates, and honours
/// the caller's auto-sync reassert policy. The
/// Update's outcome is report data, exactly as for Update; a refused folder
/// changes nothing and runs no Update.
///
/// Only a `local` skill has a folder to re-point; any other provenance is a
/// caller error, never an operator condition (the card offers Re-point for
/// `source_missing` alone).
#[allow(clippy::too_many_arguments)]
pub fn repoint_and_update(
    paths: &InstallerPaths,
    store: &SkillStore,
    skill_id: &str,
    new_source: &Path,
    policy: RefreshPolicy,
    cancel: Option<&CancelToken>,
    now: i64,
    on_progress: impl FnMut(RefreshProgress),
) -> Result<RefreshReport> {
    let record = validated_local_repoint(store, &paths.home, skill_id, new_source)?;
    refresh_managed_skills_with(
        paths,
        store,
        RefreshSelection::Ids(vec![skill_id.to_string()]),
        policy,
        cancel,
        now,
        on_progress,
        &|_, _| super::skill_update::UpdateRequest::local(record.clone(), new_source, true),
    )
}

/// Validate and propose the new source. Only Update may settle this proposal;
/// a failed copy/finalize leaves the stored source and old bytes untouched.
fn validated_local_repoint(
    store: &SkillStore,
    home: &Path,
    skill_id: &str,
    new_source: &Path,
) -> Result<SkillRecord> {
    let record = require_local(store, skill_id)?;
    if !new_source.exists() {
        anyhow::bail!(SignalError::SourcePathMissing {
            path: new_source.to_string_lossy().to_string(),
        });
    }
    require_skill_md(new_source)?;
    if let Some(holder) = tool_holding_path(home, new_source) {
        anyhow::bail!(SignalError::LocalSourceInsideToolDir {
            path: new_source.to_string_lossy().to_string(),
            tool: holder.key().to_string(),
        });
    }
    Ok(record)
}

/// Detach a `local` skill from its source folder: it becomes `imported` —
/// no external source, the central copy is its truth (ADR-0003) — with no
/// found-in Tool history, because it came from a folder, not a Tool.
/// Store-only: no Sync target changes. Refused ([`require_detachable`])
/// when there is no central copy to become the truth.
pub fn detach_from_source(store: &SkillStore, skill_id: &str) -> Result<SkillRecord> {
    let record = require_local(store, skill_id)?;
    require_detachable(&record)?;
    let updated = SkillRecord {
        source_type: Provenance::Imported.as_str().to_string(),
        source_ref: None,
        source_subpath: None,
        source_revision: None,
        imported_from_tool: None,
        ..record
    };
    store.upsert_skill(&updated)?;
    Ok(updated)
}

/// Whether Detach is open to this skill — the **one** Detach rule, which
/// [`detach_from_source`] enforces and the Managed-skill listing exposes so
/// the card offers Detach exactly when it would be accepted: a `local`
/// skill whose central copy is present. A skill whose source and central
/// copy are both gone is `source_missing` (Re-point rebuilds both), and
/// detaching it would leave a row only Remove can touch.
pub fn is_detachable(record: &SkillRecord) -> bool {
    Provenance::parse(&record.source_type) == Some(Provenance::Local)
        && require_detachable(record).is_ok()
}

/// The typed half of the Detach rule: the central copy must be there to
/// become the truth. Raised as the same condition `move_central_repo` uses.
fn require_detachable(record: &SkillRecord) -> Result<()> {
    if !Path::new(&record.central_path).exists() {
        anyhow::bail!(SignalError::CentralPathMissing {
            path: record.central_path.clone(),
        });
    }
    Ok(())
}

fn require_local(store: &SkillStore, skill_id: &str) -> Result<SkillRecord> {
    let record = store.get_skill_by_id(skill_id)?.ok_or_else(|| {
        anyhow::anyhow!(SignalError::NotFound {
            kind: "skill".to_string(),
            id: skill_id.to_string(),
        })
    })?;
    if Provenance::parse(&record.source_type) != Some(Provenance::Local) {
        anyhow::bail!(
            "only a local skill has a source folder to re-point or detach from: {} is {}",
            record.name,
            record.source_type
        );
    }
    Ok(record)
}

#[cfg(test)]
#[path = "tests/unlocatable.rs"]
mod tests;
