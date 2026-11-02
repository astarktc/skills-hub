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
//!    (there is no partial finalize) and every selected skill is reported
//!    [`SkillRefreshStatus::Cancelled`] — its own outcome kind, never a
//!    failure. Staging dirs already acquired are dropped, which deletes them.
//! 2. **Apply** each acquired skill under the mutation guard: finalize, then
//!    Propagation. The guard is taken *per skill*, not once for the batch, so
//!    listings and other mutations are not blocked for the whole run.
//!
//! Auto-sync plays no part here: it decides only where a **new** skill lands
//! (Add, Onboarding import, Enable's first deploy — decision D34). An Update
//! only brings the deployments that already exist into line; it never
//! creates one. (Before D34 the apply phase also "re-asserted" every skill
//! onto every selected Tool; that pass is deleted.)
//!
//! Everything is report data: a skill that fails acquisition is reported and
//! excluded from phase two; a Sync target that fails is reported by
//! Propagation. Two kinds of skill are never dispatched and are reported
//! *skipped* with a typed [`RefreshSkipReason`]: an Unlocatable one (source
//! folder or central copy gone), and one whose Delete was incomplete (D35 —
//! finishing the Delete is the only way forward). Only reading the skill list
//! can fail the operation.

use std::sync::mpsc;
use std::sync::Mutex;

use anyhow::Result;

use super::cancel_token::CancelToken;
use super::credentials::CredentialStore;
use super::errors::{CommandError, SignalError};
use super::git_acquisition::HttpGithubApi;
use super::installer::InstallerPaths;
use super::mutation_guard;
use super::propagation::PropagationOutcome;
use super::provenance::{refresh_eligibility, RefreshEligibility};
use super::skill_health::{self, SkillOutcomeKind};
use super::skill_store::SkillStore;
use super::skill_update::{
    acquire_update, apply_unlocked, ApplyOutcome, UpdateRequest, UpdateSkip,
};
use super::unlocatable::UnlocatableState;

/// Which Managed skills to refresh.
#[derive(Clone, Debug)]
pub enum RefreshSelection {
    All,
    Ids(Vec<String>),
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
type AcquireFn<'a> = dyn Fn(&str, Option<&CancelToken>) -> Result<UpdateRequest> + Sync + 'a;

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

#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SkillRefreshStatus {
    /// Acquired, finalized and propagated. `targets` is Propagation's report:
    /// the deployments that already existed, and nothing else.
    Refreshed {
        content_hash: Option<String>,
        source_revision: Option<String>,
        targets: Vec<PropagationOutcome>,
        edit_conflict: Option<super::skill_edits::InvocationEditConflict>,
    },
    /// Acquisition or finalize failed; this skill's targets were left alone.
    Failed { error: CommandError },
    /// The batch did not dispatch this skill; nothing was acquired and no
    /// target was touched. `reason` says why (see [`RefreshSkipReason`]).
    Skipped { reason: RefreshSkipReason },
    /// Acquired bytes were discarded at admission; no target was touched.
    SkippedAcquisition { reason: UpdateSkip },
    /// The operator cancelled the run before this skill was applied: nothing
    /// was finalized and no target was touched. Its own outcome kind (D10),
    /// never `Failed { Cancelled }` — a cancellation is not a failure of the
    /// skill and records no health. Every batch that honours the cancel token
    /// reports the members it never reached with this kind, on the wire as
    /// `{ "status": "cancelled" }`.
    Cancelled,
}

/// Why a selected skill was set aside before acquisition. Crosses the wire
/// as `{ kind: "unlocatable", state }` or `{ kind: "delete_incomplete" }`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RefreshSkipReason {
    /// The app cannot locate the skill (`provenance::refresh_eligibility`).
    /// Only `All` skips for it — an explicitly named id proceeds to the
    /// acquire step, which answers with the typed condition (or, for a
    /// missing central copy, rebuilds it: that is Restore).
    Unlocatable { state: UnlocatableState },
    /// The skill's stored health is `delete_incomplete` (D35): a Delete that
    /// could not remove every artifact kept it, and an Update must not bring
    /// it back. Skipped for `All` **and** a named id (Update, Re-point): there
    /// is no Restore-like repair here — retrying the Delete is.
    DeleteIncomplete,
}

#[derive(Debug, serde::Serialize, specta::Type)]
pub struct SkillRefreshOutcome {
    pub skill_id: String,
    pub skill_name: String,
    pub status: SkillRefreshStatus,
}

#[derive(Debug, Default, serde::Serialize, specta::Type)]
pub struct RefreshReport {
    pub skills: Vec<SkillRefreshOutcome>,
}

/// Refresh a set of Managed skills (or all of them).
///
/// Mutation entry point: phase two serialises each skill's finalize +
/// Propagation against every other Sync-target mutation. Acquisition is
/// deliberately outside the guard.
#[allow(clippy::too_many_arguments)]
pub fn refresh_managed_skills(
    paths: &InstallerPaths,
    store: &SkillStore,
    credentials: &dyn CredentialStore,
    selection: RefreshSelection,
    cancel: Option<&CancelToken>,
    on_progress: impl FnMut(RefreshProgress),
) -> Result<RefreshReport> {
    // Both acquisition inputs are read once for the batch rather than per
    // skill: the pool must not funnel through the store or the keychain. A
    // token read that fails is not worth failing the batch for — no token is
    // the shipped default.
    let token = super::settings::github_token_or_none(credentials);
    let ttl_ms = super::settings::git_cache_ttl_ms(store);
    refresh_managed_skills_with(
        paths,
        store,
        selection,
        cancel,
        on_progress,
        &|skill_id, cancel| {
            // One adapter per acquisition: `GithubApi` carries no `Send`
            // bound, so nothing is shared between workers.
            acquire_update(
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

/// What the batch is doing to each skill's source, for the Skill health a
/// failure settles as (`core::skill_health`): an Update of the skill's own
/// source (`fetch_failed`, or `restore_failed` when the central copy is gone
/// — that Update is a Restore), or a Re-point to a new one (`repoint_failed`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UpdateIntent {
    Update,
    Repoint,
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
    cancel: Option<&CancelToken>,
    on_progress: impl FnMut(RefreshProgress),
    acquire: &AcquireFn,
) -> Result<RefreshReport> {
    refresh_with_intent(
        paths,
        store,
        selection,
        cancel,
        on_progress,
        acquire,
        UpdateIntent::Update,
    )
}

/// The batch itself, for either [`UpdateIntent`] (Re-point enters here).
#[allow(clippy::too_many_arguments)]
pub(crate) fn refresh_with_intent(
    paths: &InstallerPaths,
    store: &SkillStore,
    selection: RefreshSelection,
    cancel: Option<&CancelToken>,
    mut on_progress: impl FnMut(RefreshProgress),
    acquire: &AcquireFn,
    intent: UpdateIntent,
) -> Result<RefreshReport> {
    let Selection {
        members: selected,
        skipped,
    } = select_skills(store, &selection)?;
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
        let mut report = RefreshReport {
            skills: selected
                .into_iter()
                .map(|(skill_id, skill_name)| SkillRefreshOutcome {
                    skill_id,
                    skill_name,
                    status: SkillRefreshStatus::Cancelled,
                })
                .collect(),
        };
        report.skills.extend(skipped);
        return Ok(report);
    }

    let acquired: Vec<(String, String, Result<UpdateRequest>)> = selected
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
                // Reported and excluded from propagation; the acquisition
                // failure settles here, as the skill's health.
                // An acquisition that answered the cancellation signal is a
                // cancelled row, not a failure, and records nothing.
                let error = CommandError::from_anyhow(error);
                if matches!(error, CommandError::Cancelled) {
                    report.skills.push(SkillRefreshOutcome {
                        skill_id,
                        skill_name,
                        status: SkillRefreshStatus::Cancelled,
                    });
                    continue;
                }
                mutation_guard::serialized(|| {
                    settle_failure_health(store, &skill_id, intent, &error)
                });
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
            mutation_guard::serialized(|| apply_one_unlocked(paths, store, update, intent));
        report.skills.push(SkillRefreshOutcome {
            skill_id,
            skill_name,
            status,
        });
    }
    report.skills.extend(skipped);
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
) -> (Vec<Option<Result<UpdateRequest>>>, bool) {
    let total = selected.len();
    let mut slots: Vec<Option<Result<UpdateRequest>>> = (0..total).map(|_| None).collect();
    if total == 0 {
        return (slots, is_cancelled(cancel));
    }

    let next: Mutex<usize> = Mutex::new(0);
    let (tx, rx) = mpsc::channel::<(usize, Result<UpdateRequest>)>();
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

/// What a selection resolved to: the `(id, name)` members the batch
/// dispatches, and the outcomes already settled for skills it skipped.
struct Selection {
    members: Vec<(String, String)>,
    skipped: Vec<SkillRefreshOutcome>,
}

/// Resolve the selection. An id with no row is dropped rather than failing
/// the batch — the listing that produced it may be stale.
///
/// Membership is `provenance::refresh_eligibility`: `All` means every
/// *refreshable* Managed skill the app can locate — an imported skill is not
/// a member, so it is neither acquired nor reported (not "skipped"), while an
/// Unlocatable one is reported skipped with its state and never dispatched.
/// An id named explicitly is kept as is: the acquire step answers it with
/// the typed condition (`NotRefreshable`, `SourcePathMissing`) or rebuilds a
/// missing central copy (Restore), so a single Update reports what happened
/// instead of an empty batch.
///
/// One gate applies to both selections: a skill whose stored health is
/// `delete_incomplete` is set aside as [`RefreshSkipReason::DeleteIncomplete`]
/// (D35) — for `All` it is checked before eligibility, so an incomplete
/// Delete is what the operator reads even when the skill is also Unlocatable.
fn select_skills(store: &SkillStore, selection: &RefreshSelection) -> Result<Selection> {
    let health = skill_health::all_skill_health(store)?;
    let delete_incomplete = |skill_id: &str| {
        health
            .get(skill_id)
            .is_some_and(|health| health.kind == SkillOutcomeKind::DeleteIncomplete)
    };
    let skip =
        |skill_id: String, skill_name: String, reason: RefreshSkipReason| SkillRefreshOutcome {
            skill_id,
            skill_name,
            status: SkillRefreshStatus::Skipped { reason },
        };
    let mut members = Vec::new();
    let mut skipped = Vec::new();
    match selection {
        RefreshSelection::All => {
            for skill in store.list_skills()? {
                let eligibility = refresh_eligibility(&skill);
                if delete_incomplete(&skill.id)
                    && !matches!(eligibility, RefreshEligibility::NotAMember)
                {
                    skipped.push(skip(
                        skill.id,
                        skill.name,
                        RefreshSkipReason::DeleteIncomplete,
                    ));
                    continue;
                }
                match eligibility {
                    RefreshEligibility::Refreshable => members.push((skill.id, skill.name)),
                    RefreshEligibility::NotAMember => {}
                    RefreshEligibility::Unlocatable(state) => skipped.push(skip(
                        skill.id,
                        skill.name,
                        RefreshSkipReason::Unlocatable { state },
                    )),
                }
            }
        }
        RefreshSelection::Ids(ids) => {
            for id in ids {
                if let Some(skill) = store.get_skill_by_id(id)? {
                    if delete_incomplete(&skill.id) {
                        skipped.push(skip(
                            skill.id,
                            skill.name,
                            RefreshSkipReason::DeleteIncomplete,
                        ));
                    } else {
                        members.push((skill.id, skill.name));
                    }
                }
            }
        }
    }
    Ok(Selection { members, skipped })
}

/// Finalize one acquired skill and bring its Sync targets into line. The
/// caller holds the mutation guard.
fn apply_one_unlocked(
    paths: &InstallerPaths,
    store: &SkillStore,
    update: UpdateRequest,
    intent: UpdateIntent,
) -> SkillRefreshStatus {
    let skill_id = update.skill_id().to_string();
    let outcome = match apply_unlocked(paths, store, update) {
        Ok(ApplyOutcome::Updated(outcome)) => outcome,
        Ok(ApplyOutcome::Skipped { reason }) => {
            return SkillRefreshStatus::SkippedAcquisition { reason }
        }
        Err(error) => {
            let error = CommandError::from_anyhow(error);
            settle_failure_health(store, &skill_id, intent, &error);
            return SkillRefreshStatus::Failed { error };
        }
    };
    SkillRefreshStatus::Refreshed {
        content_hash: outcome.content_hash,
        source_revision: outcome.source_revision,
        targets: outcome.propagation.targets,
        edit_conflict: outcome.edit_conflict,
    }
}

/// A failed Update / Restore / Re-point settles as the skill's health (see
/// `core::skill_health`), with the error its report row carries. A
/// cancellation is not a failure of the skill and records nothing. Restore is
/// told apart by the state it repairs: the central copy is gone (finalize
/// never ran, or rolled back). The caller holds the mutation guard.
fn settle_failure_health(
    store: &SkillStore,
    skill_id: &str,
    intent: UpdateIntent,
    error: &CommandError,
) {
    if matches!(error, CommandError::Cancelled) {
        return;
    }
    let kind = match intent {
        UpdateIntent::Repoint => SkillOutcomeKind::RepointFailed,
        UpdateIntent::Update => match store.get_skill_by_id(skill_id) {
            Ok(Some(skill)) if !std::path::Path::new(&skill.central_path).exists() => {
                SkillOutcomeKind::RestoreFailed
            }
            Ok(Some(_)) => SkillOutcomeKind::FetchFailed,
            // Deleted meanwhile: nothing to record on.
            Ok(None) => return,
            Err(err) => {
                log::warn!("reading skill {skill_id} for its health failed: {err:#}");
                return;
            }
        },
    };
    skill_health::note_outcome(store, skill_id, kind, error);
}

#[cfg(test)]
#[path = "tests/refresh.rs"]
mod tests;
