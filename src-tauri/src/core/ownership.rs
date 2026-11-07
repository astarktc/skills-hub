//! Ownership of a Deployment's folder: the **occupant check** (ADR-0007 rule 1).
//!
//! A Deployment row (a global `skill_targets` row or a project assignment)
//! records that the app wrote its path. That record is history, never
//! permission: something else can occupy the path later — a folder the
//! operator put where the app's link was, or a copy edited in place. So the
//! app removes or replaces the folder at a row's path only when it can show,
//! **at the moment it acts**, that the folder there is still what it wrote
//! through that row:
//!
//! * a **link** (a junction counts) resolving, by physical identity
//!   (`removal_fence::Place`), to the skill's Library copy — or, when that copy
//!   cannot be resolved, spelling exactly its recorded path: the link the app
//!   writes, which a removal or replacement takes as itself, never its
//!   referent; or
//! * a **copy** (a real folder) whose Content identity equals the identity the
//!   row recorded as deployed (`content_hash`, written at every successful
//!   copy).
//!
//! Anything else is not ours: a real folder when the row recorded no
//! identity (a row from before 1.4.0 that deployed a copy, or a link row), a
//! folder whose content changed, a link elsewhere, a file. A row that never
//! materialised or gave its path up (`owns_artifact` false: a marker, a Keep
//! answer, a successful Disable) has no evidence at all, so nothing at its path
//! is ours.
//!
//! Every destructive seam asks here, right before it acts — never an earlier
//! observation carried forward: `artifact_removal` before each removal
//! (Delete, unassign, project removal, Disable), and `sync_engine` before each
//! replacement ([`crate::core::sync_engine::Replace`]: resync, Propagation, the
//! global batch, Enable's first sync). The physical admission of the removal
//! itself stays `removal_fence`'s: this check decides whether the occupant is
//! ours to act on, the fence decides whether the entry may be touched at all.
//! Physical identity and folder grouping never grant ownership on another
//! row's behalf: each row is checked against its own evidence.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::content_identity::{self, Comparison};
use super::removal_fence::Place;
use super::skill_store::{ProjectSkillAssignmentRecord, SkillRecord, SkillTargetRecord};

/// What one row recorded about what it deployed — the evidence [`check`]
/// verifies the occupant of its path against. Built from the stored row and
/// its skill's record (never from what is on disk).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Deployed(Option<Evidence>);

#[derive(Clone, Debug, PartialEq, Eq)]
struct Evidence {
    /// The skill's Library copy (its recorded `central_path`): a link
    /// resolving to it is ours. `None` when the skill record is gone.
    library_copy: Option<PathBuf>,
    /// The Content identity the row recorded as deployed: a copy whose
    /// identity equals it is ours. `None` for a link row, or a row that
    /// recorded none (a pre-1.4.0 global row).
    content_hash: Option<String>,
}

impl Deployed {
    /// No evidence: nothing at the path is ours.
    pub(crate) fn none() -> Self {
        Self(None)
    }

    /// The evidence of a row that holds its claim on its path (`claimed`:
    /// the app wrote it and never gave it up), else [`Deployed::none`].
    pub(crate) fn recorded(
        claimed: bool,
        library_copy: Option<&Path>,
        content_hash: Option<&str>,
    ) -> Self {
        if !claimed {
            return Self::none();
        }
        Self(Some(Evidence {
            library_copy: library_copy
                .filter(|path| path.is_absolute())
                .map(Path::to_path_buf),
            content_hash: content_hash.map(str::to_string),
        }))
    }

    /// What a global Deployment row recorded; `skill` is its skill's record
    /// (the Library copy a link of ours resolves to).
    pub(crate) fn of_target(row: &SkillTargetRecord, skill: Option<&SkillRecord>) -> Self {
        Self::recorded(
            row.owns_artifact(),
            skill.map(|skill| Path::new(&skill.central_path)),
            row.content_hash.as_deref(),
        )
    }

    /// What a project assignment recorded; `skill` as for [`Self::of_target`].
    pub(crate) fn of_assignment(
        row: &ProjectSkillAssignmentRecord,
        skill: Option<&SkillRecord>,
    ) -> Self {
        Self::recorded(
            row.owns_artifact(),
            skill.map(|skill| Path::new(&skill.central_path)),
            row.content_hash.as_deref(),
        )
    }
}

/// What occupies a row's path, as far as the row's evidence can tell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Occupancy {
    /// Nothing is there (the leaf or its parent is `NotFound`).
    Absent,
    /// What the row wrote, still: the app may remove or replace it.
    Ours,
    /// Something else, or nothing the evidence can vouch for: kept.
    Foreign,
}

/// The occupant check: what occupies `path`, judged against `deployed`.
///
/// Reads the filesystem now — call it at the moment of removal or
/// replacement. An I/O failure that leaves the occupant unknown (a stat
/// other than `NotFound`, an unreadable copy) is an `Err`, never `Ours`: a
/// removal keeps its row `error` (ADR-0002), a write raises the ask.
pub(crate) fn check(path: &Path, deployed: &Deployed) -> Result<Occupancy> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Occupancy::Absent),
        Err(err) => return Err(err).with_context(|| format!("inspect {:?}", path)),
    };
    let Some(evidence) = &deployed.0 else {
        return Ok(Occupancy::Foreign);
    };
    if meta.file_type().is_symlink() {
        let ours = match &evidence.library_copy {
            Some(copy) => links_to(path, copy)?,
            None => false,
        };
        return Ok(if ours {
            Occupancy::Ours
        } else {
            Occupancy::Foreign
        });
    }
    if meta.is_dir() {
        let Some(recorded) = &evidence.content_hash else {
            return Ok(Occupancy::Foreign);
        };
        return match content_identity::compare_recorded(recorded, path) {
            Comparison::Same => Ok(Occupancy::Ours),
            Comparison::Different => Ok(Occupancy::Foreign),
            Comparison::Unknown => {
                anyhow::bail!(
                    "cannot read {:?} to tell whether it is the deployed copy",
                    path
                )
            }
        };
    }
    Ok(Occupancy::Foreign)
}

/// Whether the link at `link` is the one the app writes for `library_copy`:
/// it resolves, by physical identity, to the copy — or, when either side
/// cannot be resolved (a dangling link, a copy gone missing), it spells the
/// copy's recorded path exactly.
fn links_to(link: &Path, library_copy: &Path) -> Result<bool> {
    let at = Place::of(link)?;
    let copy = Place::of(library_copy)?;
    if at.same(&copy) {
        return Ok(true);
    }
    let resolves = link.canonicalize().is_ok() && library_copy.canonicalize().is_ok();
    Ok(!resolves && std::fs::read_link(link).is_ok_and(|target| target == library_copy))
}

#[cfg(test)]
#[path = "tests/ownership.rs"]
mod tests;
