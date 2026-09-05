//! Provenance: where a Managed skill's bytes come from, and whether Refresh
//! may go back there (see **Provenance** in `CONTEXT.md`, ADR-0003).
//!
//! Three kinds, stored as the `source_type` string on the record:
//!
//! - `git` — an external repository; refreshable.
//! - `local` — an independent folder the operator maintains, outside every
//!   Tool's skills dir; refreshable, and the folder itself is never touched.
//! - `imported` — taken over from a Tool's skills directory. The central copy
//!   is its source of truth: there is nothing to refresh from, `source_ref`
//!   is `None`, and the Tool it was found in is display-only history
//!   (`imported_from_tool`).
//!
//! [`refresh_eligibility`] is the **one** membership rule Refresh (all)
//! consults; [`is_refreshable`] is its provenance half — "is there a source
//! to re-acquire from?" — which the single-skill Update and the Managed-skill
//! listing consult (a Restore *is* an Update of a skill whose central copy is
//! gone). No caller re-derives either from the string.

use super::skill_store::SkillRecord;
use super::unlocatable::{unlocatable_state, UnlocatableState};

/// The three provenances a `source_type` can spell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provenance {
    Git,
    Local,
    Imported,
}

impl Provenance {
    /// Parse the stored `source_type`. `None` for a value this app never
    /// writes.
    pub fn parse(source_type: &str) -> Option<Self> {
        match source_type {
            "git" => Some(Provenance::Git),
            "local" => Some(Provenance::Local),
            "imported" => Some(Provenance::Imported),
            _ => None,
        }
    }

    /// The stored and wire spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Provenance::Git => "git",
            Provenance::Local => "local",
            Provenance::Imported => "imported",
        }
    }

    /// Whether this provenance names an external source Refresh can
    /// re-acquire from. An imported skill's truth is its central copy.
    pub fn has_external_source(self) -> bool {
        match self {
            Provenance::Git | Provenance::Local => true,
            Provenance::Imported => false,
        }
    }
}

/// Whether — and how — a skill takes part in a Refresh (all) batch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshEligibility {
    /// A member: acquired, finalized, propagated.
    Refreshable,
    /// Not a member (no external source: `imported`, or a provenance this
    /// app never wrote). Silently absent from the batch — not "skipped".
    NotAMember,
    /// Would be a member, but the app cannot locate it (see **Unlocatable
    /// skill** in `CONTEXT.md`). Not dispatched; reported as skipped with
    /// its state so the operator sees why.
    Unlocatable(UnlocatableState),
}

/// The Refresh (all) membership rule: provenance first (no external source
/// → not a member), then the Unlocatable states (source folder gone,
/// central copy gone → skipped).
pub fn refresh_eligibility(record: &SkillRecord) -> RefreshEligibility {
    if !is_refreshable(record) {
        return RefreshEligibility::NotAMember;
    }
    match unlocatable_state(record) {
        Some(state) => RefreshEligibility::Unlocatable(state),
        None => RefreshEligibility::Refreshable,
    }
}

/// Does this skill have an external source Update can re-acquire from?
///
/// The provenance alone: a skill this app cannot parse a provenance for is
/// not refreshable either (there is no acquisition path for it). This is
/// deliberately blind to the on-disk state — a `git`/`local` skill whose
/// central copy is gone answers *yes*, and re-acquiring it is exactly what
/// Restore does. Batch membership is [`refresh_eligibility`].
pub fn is_refreshable(record: &SkillRecord) -> bool {
    Provenance::parse(&record.source_type).is_some_and(Provenance::has_external_source)
}

#[cfg(test)]
#[path = "tests/provenance.rs"]
mod tests;
