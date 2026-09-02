//! The process-wide serialisation rule for Sync-target mutations.
//!
//! Every operation that materialises or removes a Sync target — the global
//! sync batch, Artifact removal, assign/unassign, resync, project tool
//! configuration, project removal, gitignore updates, and the Propagation
//! step of a managed-skill update — runs one at a time, by construction:
//! each of those *entry points* wraps its own body in [`serialized`]. No
//! command carries lock state and no caller decides per call site whether to
//! lock; the rule lives here.
//!
//! The mutex is **private to this module** — that is the point of the
//! boundary, and the same shape `git_cache.rs` uses for the git clone cache.
//! It is a plain non-reentrant `std::sync::Mutex`, which imposes one
//! discipline on core:
//!
//! > **An entry point never calls another entry point.** Composite
//! > operations call the unlocked `pub(crate)` internal seam of the
//! > operation they compose (`*_unlocked`, `remove_tool_with_cleanup`,
//! > `plan_skill_removal`, …). Calling the locked twin from inside the
//! > critical section deadlocks.
//!
//! Readers that must not queue behind a mutation use [`try_serialized`]: the
//! reconcile pass run by the project listing takes that door and reports
//! `reconciled: false` when a mutation is in flight, rather than making the
//! listing wait.

use std::sync::Mutex;

/// The one mutation guard. `Mutex::new` is const, so no lazy init is needed.
static GUARD: Mutex<()> = Mutex::new(());

/// Run `op` as the only Sync-target mutation in this process.
///
/// Poisoning is recovered rather than propagated: the guard protects nothing
/// but the right to run, so a panicking mutation must not wedge every later
/// one.
pub(crate) fn serialized<T>(op: impl FnOnce() -> T) -> T {
    let _guard = GUARD.lock().unwrap_or_else(|err| err.into_inner());
    op()
}

/// Run `op` only if no mutation is in flight, otherwise return `None`
/// immediately. For read paths that would rather report un-reconciled data
/// than block behind a mutation.
pub(crate) fn try_serialized<T>(op: impl FnOnce() -> T) -> Option<T> {
    match GUARD.try_lock() {
        Ok(_guard) => Some(op()),
        Err(std::sync::TryLockError::Poisoned(poisoned)) => {
            let _guard = poisoned.into_inner();
            Some(op())
        }
        Err(std::sync::TryLockError::WouldBlock) => None,
    }
}

#[cfg(test)]
#[path = "tests/mutation_guard.rs"]
mod tests;
