//! The sync-status lifecycle: the typed vocabulary for "how does a synced
//! artifact relate to its source right now", shared by project skill
//! assignments and global skill targets.
//!
//! This module owns the enums, their stored/wire spelling, the pure decision
//! function a reconcile pass applies (`next_status`) and the per-project
//! precedence fold (`aggregate`). It reads no environment and touches no DB:
//! `project_sync::reconcile_assignments` observes the filesystem and applies
//! the decision through `SkillStore::transition_assignment`.
//!
//! Storage contract (no schema change): the `status`/`mode` columns keep the
//! historical strings — `as_str()` is what gets written, `from_stored()` is
//! what the store seam parses. The parsers are `Option`-returning so the store
//! decides the legacy policy (see `skill_store::read_lifecycle`).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Lifecycle of one synced artifact (an assignment row or a skill target row).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SyncStatus {
    /// Row created, sync not yet attempted.
    Pending,
    /// Artifact present and (for copies) content matches the source.
    Synced,
    /// Copy-mode artifact whose content no longer matches the source.
    Stale,
    /// Source or a previously deployed target has disappeared.
    Missing,
    /// The last sync/cleanup attempt failed; `last_error` carries the diagnostic.
    Error,
}

impl SyncStatus {
    #[cfg(test)]
    pub const ALL: [SyncStatus; 5] = [
        SyncStatus::Pending,
        SyncStatus::Synced,
        SyncStatus::Stale,
        SyncStatus::Missing,
        SyncStatus::Error,
    ];

    /// The stored (and wire) spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            SyncStatus::Pending => "pending",
            SyncStatus::Synced => "synced",
            SyncStatus::Stale => "stale",
            SyncStatus::Missing => "missing",
            SyncStatus::Error => "error",
        }
    }

    /// Parse a stored value. Accepts the legacy `"ok"` that global skill
    /// targets wrote before the lifecycle was typed (it meant exactly
    /// `Synced`). Anything else is unknown — the caller decides.
    pub fn from_stored(raw: &str) -> Option<SyncStatus> {
        match raw {
            "pending" => Some(SyncStatus::Pending),
            "synced" => Some(SyncStatus::Synced),
            "stale" => Some(SyncStatus::Stale),
            "missing" => Some(SyncStatus::Missing),
            "error" => Some(SyncStatus::Error),
            LEGACY_TARGET_OK => Some(SyncStatus::Synced),
            _ => None,
        }
    }

    /// Statuses that imply something was written to the target location
    /// (so removal must attempt to delete it). `Error` is included because a
    /// failed cleanup leaves the artifact behind.
    pub fn has_deployed_artifact(self) -> bool {
        matches!(
            self,
            SyncStatus::Synced | SyncStatus::Stale | SyncStatus::Error
        )
    }
}

/// Pre-enum status literal written by global skill target rows.
const LEGACY_TARGET_OK: &str = "ok";

/// How an artifact was materialised. Stored on the row so a later pass knows
/// whether content drift is possible (copies) or not (links).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SyncMode {
    Symlink,
    /// Windows directory junction (the symlink fallback).
    Junction,
    Copy,
}

impl SyncMode {
    #[cfg(test)]
    pub const ALL: [SyncMode; 3] = [SyncMode::Symlink, SyncMode::Junction, SyncMode::Copy];

    /// The stored (and wire) spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            SyncMode::Symlink => "symlink",
            SyncMode::Junction => "junction",
            SyncMode::Copy => "copy",
        }
    }

    pub fn from_stored(raw: &str) -> Option<SyncMode> {
        match raw {
            "symlink" => Some(SyncMode::Symlink),
            "junction" => Some(SyncMode::Junction),
            "copy" => Some(SyncMode::Copy),
            _ => None,
        }
    }

    /// Links follow the source automatically; only copies can drift.
    pub fn can_drift(self) -> bool {
        matches!(self, SyncMode::Copy)
    }
}

/// What a reconcile pass observed for one assignment. Pure data: the
/// observer resolves paths and hashes, `next_status` only decides.
#[derive(Clone, Copy, Debug)]
pub struct Observation<'a> {
    /// The skill's central directory exists.
    pub source_present: bool,
    /// The project target exists (a dangling link counts as present).
    pub target_present: bool,
    pub mode: SyncMode,
    /// The status currently recorded on the row.
    pub current: SyncStatus,
    /// Current content hash of the source (copy mode only; `None` = unknown).
    pub source_hash: Option<&'a str>,
    /// Content hash recorded on the row at the last successful sync.
    pub recorded_hash: Option<&'a str>,
}

/// The status the row should have given `obs`. Returns `obs.current` when
/// nothing observed justifies a change, so callers write only on difference.
///
/// Rules, in precedence order:
/// 1. Source absent → `Missing` (nothing can be in sync with nothing).
/// 2. Target absent → `Missing` if the row was ever deployed
///    (`Synced`/`Stale`/`Missing`); `Pending`/`Error` rows never had a
///    target to lose, so they keep their status.
/// 3. Both present, copy mode → hashes decide (`Synced` on match, `Stale`
///    otherwise); an unknown source hash leaves the row alone. This runs for
///    any current status so `Missing`/`Error` rows recover.
/// 4. Both present, link mode → a `Missing` row recovers to `Synced`;
///    everything else is left alone (links cannot drift).
pub fn next_status(obs: &Observation<'_>) -> SyncStatus {
    if !obs.source_present {
        return SyncStatus::Missing;
    }
    if !obs.target_present {
        return if obs.current.has_deployed_target() {
            SyncStatus::Missing
        } else {
            obs.current
        };
    }
    if obs.mode.can_drift() {
        return match obs.source_hash {
            Some(source) if obs.recorded_hash == Some(source) => SyncStatus::Synced,
            Some(_) => SyncStatus::Stale,
            None => obs.current,
        };
    }
    if obs.current == SyncStatus::Missing {
        SyncStatus::Synced
    } else {
        obs.current
    }
}

impl SyncStatus {
    /// Statuses whose target existed at some point (so its absence is news).
    fn has_deployed_target(self) -> bool {
        matches!(
            self,
            SyncStatus::Synced | SyncStatus::Stale | SyncStatus::Missing
        )
    }
}

/// Roll-up of a project's assignments as shown on the project list.
/// `Missing` folds into `Error` (both need the user's attention).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ProjectSyncStatus {
    /// No assignments at all.
    #[serde(rename = "none")]
    Empty,
    Error,
    Stale,
    Pending,
    Synced,
}

/// Precedence fold: error/missing > stale > pending > synced; no rows → `Empty`.
pub fn aggregate(statuses: impl IntoIterator<Item = SyncStatus>) -> ProjectSyncStatus {
    let mut result: Option<ProjectSyncStatus> = None;
    for status in statuses {
        let rank = match status {
            SyncStatus::Error | SyncStatus::Missing => ProjectSyncStatus::Error,
            SyncStatus::Stale => ProjectSyncStatus::Stale,
            SyncStatus::Pending => ProjectSyncStatus::Pending,
            SyncStatus::Synced => ProjectSyncStatus::Synced,
        };
        result = Some(match result {
            Some(current) if current.severity() >= rank.severity() => current,
            _ => rank,
        });
    }
    result.unwrap_or(ProjectSyncStatus::Empty)
}

impl ProjectSyncStatus {
    fn severity(self) -> u8 {
        match self {
            ProjectSyncStatus::Empty => 0,
            ProjectSyncStatus::Synced => 1,
            ProjectSyncStatus::Pending => 2,
            ProjectSyncStatus::Stale => 3,
            ProjectSyncStatus::Error => 4,
        }
    }
}

#[cfg(test)]
#[path = "tests/sync_status.rs"]
mod tests;
