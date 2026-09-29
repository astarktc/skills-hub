//! Skill health: the stored last outcome of a skill-level operation (see
//! **Skill health** in `CONTEXT.md`), distinct from a Sync target's own
//! status and `last_error` (target health).
//!
//! One slot per skill — `skills.last_outcome_kind` / `last_outcome_error` /
//! `last_outcome_at` — written **where the outcome settles**, in core, with
//! the `CommandError` that settlement already classified for its report
//! (ADR-0001, round-15 amendment: never an error that is then
//! `?`-propagated, never in `commands/`). A later failure of any kind
//! replaces the slot; a success clears only its own [`OutcomeFamily`].
//!
//! Writers: `refresh` (Update / Restore / Re-point failures, including a
//! failed acquisition), `skill_update::apply_unlocked` (an Update's Edit
//! replay conflict; the Update and Edit success clears),
//! `artifact_removal` (a kept Delete), `onboarding_import` (a group that
//! finalised but could not settle an original). There is no run log: the
//! slot answers "what is wrong with this skill now", nothing more.
//!
//! Health writes are a projection of an outcome that is already report
//! data, so a store failure while writing one is logged, never raised: it
//! must not turn a settled Update into a failed one.

use std::collections::HashMap;

use anyhow::Result;

use super::clock::now_ms;
use super::errors::CommandError;
use super::skill_store::{SkillStore, StoredOutcome};

/// What went wrong with a skill, as stored. Crosses the wire as itself
/// (`"fetch_failed"`, …).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SkillOutcomeKind {
    /// An Update / Refresh could not acquire or settle the skill's bytes.
    FetchFailed,
    /// An Update kept the operator's Edit over disagreeing upstream bytes.
    EditConflict,
    /// A Delete could not remove every artifact; the skill was kept.
    DeleteIncomplete,
    /// A Re-point's new source could not be acquired or settled.
    RepointFailed,
    /// A Restore (the Update of a skill whose central copy is gone) failed.
    RestoreFailed,
    /// An Onboarding import finalised the skill but could not settle an
    /// original afterwards.
    ImportFailed,
}

impl SkillOutcomeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SkillOutcomeKind::FetchFailed => "fetch_failed",
            SkillOutcomeKind::EditConflict => "edit_conflict",
            SkillOutcomeKind::DeleteIncomplete => "delete_incomplete",
            SkillOutcomeKind::RepointFailed => "repoint_failed",
            SkillOutcomeKind::RestoreFailed => "restore_failed",
            SkillOutcomeKind::ImportFailed => "import_failed",
        }
    }

    pub fn from_stored(value: &str) -> Option<Self> {
        ALL_KINDS
            .iter()
            .copied()
            .find(|kind| kind.as_str() == value)
    }
}

const ALL_KINDS: [SkillOutcomeKind; 6] = [
    SkillOutcomeKind::FetchFailed,
    SkillOutcomeKind::EditConflict,
    SkillOutcomeKind::DeleteIncomplete,
    SkillOutcomeKind::RepointFailed,
    SkillOutcomeKind::RestoreFailed,
    SkillOutcomeKind::ImportFailed,
];

/// The set of kinds one kind of success clears.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutcomeFamily {
    /// A settled Update (Update, Restore or Re-point: the skill's bytes and
    /// source are good now) clears every byte-acquisition failure.
    Update,
    /// A saved or cleared Edit, or an Update whose replay converged (the Edit
    /// row no longer conflicts), clears the Edit conflict.
    Edit,
    /// The operator's explicit Dismiss of an incomplete import: nothing can
    /// retry it (the skill is imported; only an original stayed), so the
    /// operator acknowledges it ([`dismiss_outcome`]).
    Import,
}

impl OutcomeFamily {
    pub fn kinds(self) -> &'static [SkillOutcomeKind] {
        match self {
            OutcomeFamily::Update => &[
                SkillOutcomeKind::FetchFailed,
                SkillOutcomeKind::RepointFailed,
                SkillOutcomeKind::RestoreFailed,
            ],
            OutcomeFamily::Edit => &[SkillOutcomeKind::EditConflict],
            OutcomeFamily::Import => &[SkillOutcomeKind::ImportFailed],
        }
    }
}

/// The family an operator may clear by hand, if any. Only an incomplete
/// import is dismissable: every other kind is cleared by the success of its
/// own family (Update, Edit) or by finishing the operation (a retried Delete
/// removes the row), so dismissing it would hide a problem still there.
pub fn dismissable_family(kind: SkillOutcomeKind) -> Option<OutcomeFamily> {
    match kind {
        SkillOutcomeKind::ImportFailed => Some(OutcomeFamily::Import),
        _ => None,
    }
}

/// The operator's Dismiss: clear the skill's health when it is still one
/// of `kind`'s family (a newer outcome is left alone). A kind that is not
/// dismissable is refused — the UI never offers it (a guard, so prose).
pub fn dismiss_outcome(store: &SkillStore, skill_id: &str, kind: SkillOutcomeKind) -> Result<()> {
    let Some(family) = dismissable_family(kind) else {
        anyhow::bail!("skill health {} is not dismissable", kind.as_str());
    };
    clear_outcome(store, skill_id, family)
}

/// A skill's stored health, as the catalog carries it (`ManagedSkill.health`).
#[derive(Clone, Debug, serde::Serialize, specta::Type)]
pub struct SkillHealth {
    pub kind: SkillOutcomeKind,
    /// The settled failure, in the same tagged shape as every report row.
    pub error: CommandError,
    /// When the outcome settled (ms since the epoch).
    pub at: i64,
}

/// Record `kind` with its settled `error` as the skill's health, replacing
/// whatever was stored. A skill with no row is left alone.
pub fn record_outcome(
    store: &SkillStore,
    skill_id: &str,
    kind: SkillOutcomeKind,
    error: &CommandError,
) -> Result<()> {
    let error = serde_json::to_string(error)?;
    store.set_skill_outcome(
        skill_id,
        Some(&StoredOutcome {
            kind: kind.as_str().to_string(),
            error,
            at: now_ms(),
        }),
    )
}

/// Clear the skill's health when (and only when) it is one of `family`'s
/// kinds.
pub fn clear_outcome(store: &SkillStore, skill_id: &str, family: OutcomeFamily) -> Result<()> {
    let kinds: Vec<&str> = family.kinds().iter().map(|kind| kind.as_str()).collect();
    store.clear_skill_outcome_of(skill_id, &kinds)
}

/// [`record_outcome`] for a settlement site whose outcome is already report
/// data: a store failure is logged, never raised (see the module doc).
pub(crate) fn note_outcome(
    store: &SkillStore,
    skill_id: &str,
    kind: SkillOutcomeKind,
    error: &CommandError,
) {
    if let Err(err) = record_outcome(store, skill_id, kind, error) {
        log::warn!(
            "recording {} health for skill {skill_id} failed: {err:#}",
            kind.as_str()
        );
    }
}

/// [`clear_outcome`], logged instead of raised (see the module doc).
pub(crate) fn note_success(store: &SkillStore, skill_id: &str, family: OutcomeFamily) {
    if let Err(err) = clear_outcome(store, skill_id, family) {
        log::warn!("clearing {family:?} health for skill {skill_id} failed: {err:#}");
    }
}

/// One skill's stored health, parsed; `None` when healthy.
pub fn skill_health(store: &SkillStore, skill_id: &str) -> Result<Option<SkillHealth>> {
    Ok(store
        .skill_outcome(skill_id)?
        .and_then(|stored| parse(skill_id, stored)))
}

/// Every stored health, parsed, keyed by skill id — one read for the catalog.
pub fn all_skill_health(store: &SkillStore) -> Result<HashMap<String, SkillHealth>> {
    Ok(store
        .skill_outcomes()?
        .into_iter()
        .filter_map(|(id, stored)| parse(&id, stored).map(|health| (id, health)))
        .collect())
}

/// Read policy: health never fails the catalog. An unknown kind (written by
/// a newer app version) is dropped with a warning; an error payload that no
/// longer parses is kept as `OTHER` carrying the raw JSON, so the operator
/// still sees that something failed.
fn parse(skill_id: &str, stored: StoredOutcome) -> Option<SkillHealth> {
    let Some(kind) = SkillOutcomeKind::from_stored(&stored.kind) else {
        log::warn!(
            "skill {skill_id}: unrecognised stored outcome kind {:?}",
            stored.kind
        );
        return None;
    };
    let error = serde_json::from_str::<CommandError>(&stored.error).unwrap_or_else(|_| {
        CommandError::Other {
            message: stored.error.clone(),
        }
    });
    Some(SkillHealth {
        kind,
        error,
        at: stored.at,
    })
}

#[cfg(test)]
#[path = "tests/skill_health.rs"]
mod tests;
