//! The **reported occupants**: every folder the app itself reported standing
//! in a sync target's way — a `TARGET_EXISTS` row of the one overwrite ask
//! (`sync_engine::Occupant::observe`, where the collision is observed) — or
//! asked about as a redeploy's edited old copy (round 20 #70).
//!
//! The ask lets the operator look before answering: Reveal in Finder
//! (`core::log_reveal::admit_reveal_path`) and a read of the occupant's files
//! against the skill (`core::skill_files::list_occupant_files`). Both admit
//! **only** a path recorded here, exactly as reported — never a path inside
//! one, above one or beside one — so the read and the reveal reach nothing
//! the backend did not itself put in front of the operator.
//!
//! The record lives as long as the process (the asks and the Notifications
//! history that can re-raise one are session state) and holds the most
//! recent [`CAPACITY`] occupants: an older one is no longer admitted, and its
//! row says so (`NOT_FOUND` `occupant`). Nothing here is ever acted on: it
//! admits reads and reveals, never a write or a removal.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// How many reported occupants are remembered.
pub(crate) const CAPACITY: usize = 512;

static REPORTED: Mutex<VecDeque<PathBuf>> = Mutex::new(VecDeque::new());

/// The spelling an occupant is recorded and matched by: its components
/// (a trailing separator dropped), never resolved.
fn normal(path: &Path) -> PathBuf {
    path.components().collect()
}

/// Record that the app reported `path` as an occupant in an ask.
pub(crate) fn report(path: &Path) {
    let path = normal(path);
    let mut reported = REPORTED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    reported.retain(|known| *known != path);
    reported.push_back(path);
    while reported.len() > CAPACITY {
        reported.pop_front();
    }
}

/// Every occupant still recorded, oldest first.
pub fn reported() -> Vec<PathBuf> {
    REPORTED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .cloned()
        .collect()
}

/// Whether `path`, spelled as reported, is a recorded occupant.
pub fn is_reported(path: &Path) -> bool {
    let path = normal(path);
    REPORTED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .any(|known| *known == path)
}

#[cfg(test)]
#[path = "tests/occupants.rs"]
mod tests;
