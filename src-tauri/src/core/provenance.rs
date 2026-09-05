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
//! [`is_refreshable`] is the **one** predicate Refresh (all), the single-skill
//! Update and the Managed-skill listing consult; no caller re-derives it from
//! the string.

use super::skill_store::SkillRecord;

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

/// Is this skill a member of a Refresh batch — can Update re-acquire it?
///
/// Today the answer is the provenance alone; a skill this app cannot parse a
/// provenance for is not refreshable either (there is no acquisition path
/// for it). Round-4 ticket 09 widens this rule with the Unlocatable states
/// (source folder gone, central copy gone) — extend it here, not at a caller.
pub fn is_refreshable(record: &SkillRecord) -> bool {
    Provenance::parse(&record.source_type).is_some_and(Provenance::has_external_source)
}

#[cfg(test)]
#[path = "tests/provenance.rs"]
mod tests;
