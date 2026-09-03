//! Refresh (all): re-acquire Managed skills from their sources, finalize
//! them, and propagate — as one backend-owned batch over a skill set.
//!
//! Two phases, deliberately separate:
//!
//! 1. **Acquire** every selected skill's bytes into its Staging dir, outside
//!    the mutation guard, over a bounded pool of [`ACQUIRE_POOL_SIZE`] worker
//!    threads. One self-contained result per skill, so phase two is untouched
//!    by the concurrency: it still sees a plain list. Same-repository fetches
//!    stay safe because `core::git_cache` locks per cache key.
//!
//!    Cancellation stops *dispatching* new acquisitions and lets in-flight
//!    ones finish; once it has been observed, phase two runs for **no** skill
//!    (there is no partial finalize) and every selected skill is reported as
//!    [`SignalError::Cancelled`]. Staging dirs already acquired are dropped,
//!    which deletes them.
//! 2. **Apply** each acquired skill under the mutation guard: finalize, then
//!    Propagation. The guard is taken *per skill*, not once for the batch, so
//!    listings and other mutations are not blocked for the whole run.
//!
//! With [`RefreshPolicy::reassert_auto_sync`] the apply phase also re-asserts
//! the auto-sync invariant — every Managed skill is synced to every installed
//! Tool — so a Tool the skill was never on gets it now.
//!
//! Everything is report data: a skill that fails acquisition is reported and
//! excluded from phase two (and from the re-assert); a Sync target that fails
//! is reported by Propagation. Only reading the skill list can fail the
//! operation.

use std::sync::mpsc;
use std::sync::Mutex;

use anyhow::Result;

use super::cancel_token::CancelToken;
use super::errors::SignalError;
use super::git_acquisition::HttpGithubApi;
use super::global_sync::{
    sync_skills_to_tools_unlocked, BatchPolicy, BatchSkill, BatchTargetStatus,
};
use super::installer::{
    acquire_managed_skill_update_with, finalize_and_propagate_unlocked, AcquiredUpdate,
    InstallerPaths,
};
use super::mutation_guard;
use super::propagation::{
    PropagationOutcome, PropagationScope, PropagationSkip, PropagationStatus,
};
use super::skill_store::SkillStore;
use super::tool_adapters::{global_tool_entries, installed_keys};

/// Which Managed skills to refresh.
#[derive(Clone, Debug)]
pub enum RefreshSelection {
    All,
    Ids(Vec<String>),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RefreshPolicy {
    /// Also sync each refreshed skill to installed Tools it is not on yet —
    /// the auto-sync invariant, re-asserted. Off leaves the target set alone.
    pub reassert_auto_sync: bool,
}

/// Which half of the batch a progress tick is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshPhase {
    Acquiring,
    Applying,
}

/// How many acquisitions may be in flight at once. A constant, not a setting:
/// core reads no environment for it (see the module doc of
/// `core::environment`).
const ACQUIRE_POOL_SIZE: usize = 4;

/// One skill's acquisition, as the pool sees it. `Sync` because every worker
/// calls the same `&dyn Fn`.
type AcquireFn<'a> = dyn Fn(&str, Option<&CancelToken>) -> Result<AcquiredUpdate> + Sync + 'a;

/// Progress tick emitted per per-skill step of each phase.
///
/// In the acquire phase the tick is emitted when a skill's acquisition
/// **completes**, so `index` counts completions (1..=total) and reads as
/// completion order — with a pool there is no meaningful dispatch index.
pub struct RefreshProgress<'a> {
    /// 1-based index within the phase.
    pub index: usize,
    pub total: usize,
    pub skill_name: &'a str,
    pub phase: RefreshPhase,
}

#[derive(Debug)]
pub enum SkillRefreshStatus {
    /// Acquired, finalized and propagated. `targets` is Propagation's report.
    /// `reassert_error` carries a store failure inside the auto-sync re-assert:
    /// the skill is still `Refreshed` (finalize and Propagation did succeed),
    /// but the targets the re-assert would have created are unknown, and that
    /// is report data rather than a log line.
    Refreshed {
        content_hash: Option<String>,
        source_revision: Option<String>,
        targets: Vec<PropagationOutcome>,
        reassert_error: Option<anyhow::Error>,
    },
    /// Acquisition or finalize failed; this skill's targets were left alone.
    Failed { error: anyhow::Error },
}

#[derive(Debug)]
pub struct SkillRefreshOutcome {
    pub skill_id: String,
    pub skill_name: String,
    pub status: SkillRefreshStatus,
}

#[derive(Debug, Default)]
pub struct RefreshReport {
    pub skills: Vec<SkillRefreshOutcome>,
}

/// Refresh a set of Managed skills (or all of them).
///
/// Mutation entry point: phase two serialises each skill's finalize +
/// Propagation against every other Sync-target mutation. Acquisition is
/// deliberately outside the guard.
pub fn refresh_managed_skills(
    paths: &InstallerPaths,
    store: &SkillStore,
    selection: RefreshSelection,
    policy: RefreshPolicy,
    cancel: Option<&CancelToken>,
    now: i64,
    on_progress: impl FnMut(RefreshProgress),
) -> Result<RefreshReport> {
    // Both DB-backed acquisition inputs are read once for the batch rather
    // than per skill: the pool must not funnel through the store. A settings
    // read that fails is not worth failing the batch for — no token is the
    // shipped default.
    let token = super::settings::github_token(store).unwrap_or_default();
    let ttl_ms = super::settings::git_cache_ttl_ms(store);
    refresh_managed_skills_with(
        paths,
        store,
        selection,
        policy,
        cancel,
        now,
        on_progress,
        &|skill_id, cancel| {
            // One adapter per acquisition: `GithubApi` carries no `Send`
            // bound, so nothing is shared between workers.
            acquire_managed_skill_update_with(
                paths,
                store,
                skill_id,
                cancel,
                &HttpGithubApi::new(token.clone()),
                ttl_ms,
            )
        },
    )
}

/// [`refresh_managed_skills`] with acquisition injected, so the pool's
/// concurrency, ordering and cancellation behaviour are testable without the
/// network (and without real latency being the only signal).
// One argument more than the public entry point it mirrors: the injected
// acquisition. Grouping the batch's parameters into a struct would change the
// public signature for the seam's benefit alone.
#[allow(clippy::too_many_arguments)]
pub(crate) fn refresh_managed_skills_with(
    paths: &InstallerPaths,
    store: &SkillStore,
    selection: RefreshSelection,
    policy: RefreshPolicy,
    cancel: Option<&CancelToken>,
    now: i64,
    mut on_progress: impl FnMut(RefreshProgress),
    acquire: &AcquireFn,
) -> Result<RefreshReport> {
    let selected = select_skills(store, &selection)?;
    let total = selected.len();

    // Phase 1 — acquire (unlocked, slow I/O) over the bounded pool.
    let (results, cancelled) = acquire_all(&selected, cancel, acquire, |done, name| {
        on_progress(RefreshProgress {
            index: done,
            total,
            skill_name: name,
            phase: RefreshPhase::Acquiring,
        })
    });

    if cancelled {
        // No partial finalize: dropping `results` drops every Staging dir,
        // which removes it.
        drop(results);
        return Ok(RefreshReport {
            skills: selected
                .into_iter()
                .map(|(skill_id, skill_name)| SkillRefreshOutcome {
                    skill_id,
                    skill_name,
                    status: SkillRefreshStatus::Failed {
                        error: anyhow::anyhow!(SignalError::Cancelled),
                    },
                })
                .collect(),
        });
    }

    let acquired: Vec<(String, String, Result<AcquiredUpdate>)> = selected
        .into_iter()
        .zip(results)
        .map(|((skill_id, skill_name), result)| {
            (
                skill_id,
                skill_name,
                result.unwrap_or_else(|| Err(anyhow::anyhow!(SignalError::Cancelled))),
            )
        })
        .collect();

    // Phase 2 — apply (finalize + propagate) one skill at a time under the guard.
    let mut report = RefreshReport::default();
    for (index, (skill_id, skill_name, result)) in acquired.into_iter().enumerate() {
        let update = match result {
            Ok(update) => update,
            Err(error) => {
                // Reported and excluded from propagation and the re-assert.
                report.skills.push(SkillRefreshOutcome {
                    skill_id,
                    skill_name,
                    status: SkillRefreshStatus::Failed { error },
                });
                continue;
            }
        };
        on_progress(RefreshProgress {
            index: index + 1,
            total,
            skill_name: &skill_name,
            phase: RefreshPhase::Applying,
        });
        let status =
            mutation_guard::serialized(|| apply_one_unlocked(paths, store, update, policy, now));
        report.skills.push(SkillRefreshOutcome {
            skill_id,
            skill_name,
            status,
        });
    }
    Ok(report)
}

/// Run every selected skill's acquisition over a pool of at most
/// [`ACQUIRE_POOL_SIZE`] threads, in dispatch-order slots.
///
/// `on_done` is called on the **coordinating thread** as each acquisition
/// lands (so the progress callback needs no `Send`), with the running
/// completion count. Returns the per-skill slots (a `None` slot is a skill
/// that was never dispatched) and whether cancellation was observed.
fn acquire_all(
    selected: &[(String, String)],
    cancel: Option<&CancelToken>,
    acquire: &AcquireFn,
    mut on_done: impl FnMut(usize, &str),
) -> (Vec<Option<Result<AcquiredUpdate>>>, bool) {
    let total = selected.len();
    let mut slots: Vec<Option<Result<AcquiredUpdate>>> = (0..total).map(|_| None).collect();
    if total == 0 {
        return (slots, is_cancelled(cancel));
    }

    let next: Mutex<usize> = Mutex::new(0);
    let (tx, rx) = mpsc::channel::<(usize, Result<AcquiredUpdate>)>();
    let mut done = 0usize;

    std::thread::scope(|scope| {
        for _ in 0..total.min(ACQUIRE_POOL_SIZE) {
            let tx = tx.clone();
            let next = &next;
            scope.spawn(move || loop {
                let index = {
                    let mut cursor = next.lock().unwrap_or_else(|err| err.into_inner());
                    // Cancellation stops dispatch, never an in-flight job.
                    if is_cancelled(cancel) || *cursor >= total {
                        break;
                    }
                    let index = *cursor;
                    *cursor += 1;
                    index
                };
                let result = acquire(&selected[index].0, cancel);
                if tx.send((index, result)).is_err() {
                    break;
                }
            });
        }
        // The coordinator holds no sender of its own, so `recv` ends when the
        // last worker is done.
        drop(tx);
        while let Ok((index, result)) = rx.recv() {
            done += 1;
            on_done(done, &selected[index].1);
            slots[index] = Some(result);
        }
    });

    (slots, is_cancelled(cancel) || done < total)
}

fn is_cancelled(cancel: Option<&CancelToken>) -> bool {
    cancel.is_some_and(|token| token.is_cancelled())
}

/// `(id, name)` for every selected skill. An id with no row is dropped rather
/// than failing the batch — the listing that produced it may be stale.
fn select_skills(
    store: &SkillStore,
    selection: &RefreshSelection,
) -> Result<Vec<(String, String)>> {
    match selection {
        RefreshSelection::All => Ok(store
            .list_skills()?
            .into_iter()
            .map(|s| (s.id, s.name))
            .collect()),
        RefreshSelection::Ids(ids) => {
            let mut out = Vec::with_capacity(ids.len());
            for id in ids {
                if let Some(skill) = store.get_skill_by_id(id)? {
                    out.push((skill.id, skill.name));
                }
            }
            Ok(out)
        }
    }
}

/// Finalize one acquired skill and bring its Sync targets into line. The
/// caller holds the mutation guard.
fn apply_one_unlocked(
    paths: &InstallerPaths,
    store: &SkillStore,
    update: AcquiredUpdate,
    policy: RefreshPolicy,
    now: i64,
) -> SkillRefreshStatus {
    let outcome = match finalize_and_propagate_unlocked(paths, store, update) {
        Ok(outcome) => outcome,
        Err(error) => return SkillRefreshStatus::Failed { error },
    };
    let targets = outcome.propagation.targets;
    let (targets, reassert_error) = if policy.reassert_auto_sync {
        let extra = reassert_auto_sync_unlocked(
            paths,
            store,
            &outcome.skill_id,
            &outcome.name,
            now,
            &targets,
        );
        merge_reassert(targets, extra)
    } else {
        (targets, None)
    };
    SkillRefreshStatus::Refreshed {
        content_hash: outcome.content_hash,
        source_revision: outcome.source_revision,
        targets,
        reassert_error,
    }
}

/// Fold the re-assert's result into the skill's target list. The skill stays
/// `Refreshed` either way; an `Err` is carried out as report data instead of
/// being logged away.
pub(crate) fn merge_reassert(
    mut targets: Vec<PropagationOutcome>,
    result: Result<Vec<PropagationOutcome>>,
) -> (Vec<PropagationOutcome>, Option<anyhow::Error>) {
    match result {
        Ok(extra) => {
            targets.extend(extra);
            (targets, None)
        }
        Err(error) => (targets, Some(error)),
    }
}

/// The auto-sync invariant, re-asserted for one skill: sync it to every
/// installed Tool it has no target row for. Existing targets were already
/// brought into line by Propagation, so they are not touched again here;
/// a directory that is in the way without a row is reported, not clobbered
/// (`overwrite_if_same_content`).
fn reassert_auto_sync_unlocked(
    paths: &InstallerPaths,
    store: &SkillStore,
    skill_id: &str,
    skill_name: &str,
    now: i64,
    already: &[PropagationOutcome],
) -> Result<Vec<PropagationOutcome>> {
    let skill = match store.get_skill_by_id(skill_id)? {
        Some(skill) => skill,
        None => return Ok(Vec::new()),
    };
    let existing: Vec<&str> = already
        .iter()
        .filter_map(|outcome| match &outcome.scope {
            PropagationScope::Global { tool } => Some(tool.as_str()),
            PropagationScope::Project { .. } => None,
        })
        .collect();
    let missing: Vec<String> = installed_keys(&global_tool_entries(&paths.home))
        .into_iter()
        .filter(|key| !existing.contains(&key.as_str()))
        .collect();
    if missing.is_empty() {
        return Ok(Vec::new());
    }

    let skills = [BatchSkill {
        skill_id: skill.id.clone(),
        skill_name: skill_name.to_string(),
        source_path: std::path::PathBuf::from(&skill.central_path),
    }];
    let policy = BatchPolicy {
        overwrite: false,
        overwrite_if_same_content: true,
        overrides: Vec::new(),
    };
    let outcomes =
        sync_skills_to_tools_unlocked(&paths.home, store, &skills, &missing, &policy, now, |_| {});

    Ok(outcomes
        .into_iter()
        .map(|outcome| PropagationOutcome {
            scope: PropagationScope::Global {
                tool: outcome.tool_key.clone(),
            },
            status: match outcome.status {
                BatchTargetStatus::Synced { outcome } => PropagationStatus::Synced {
                    mode_used: outcome.mode_used,
                },
                BatchTargetStatus::Skipped { error } => match error {
                    super::global_sync::GlobalSyncError::ToolNotInstalled { tool_key } => {
                        PropagationStatus::Skipped {
                            reason: PropagationSkip::ToolNotInstalled { tool: tool_key },
                        }
                    }
                    // A skips-because-unwritable is still a failure to report:
                    // the operator asked for this Tool to carry the skill.
                    other => PropagationStatus::Failed {
                        error: anyhow::Error::new(other),
                    },
                },
                BatchTargetStatus::Failed { error } => PropagationStatus::Failed {
                    error: anyhow::Error::new(error),
                },
            },
        })
        .collect())
}

#[cfg(test)]
#[path = "tests/refresh.rs"]
mod tests;
