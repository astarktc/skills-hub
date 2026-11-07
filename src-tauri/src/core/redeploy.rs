//! Redeploy: move every global Deployment whose recorded place differs from
//! placement's answer, deploy first, then remove the old artifact only when it
//! is provably ours (round 20 #108, ADR-0007 rule 2).
//!
//! Each recorded global Deployment row — its folder key, its path and its
//! mode — is compared with placement's answer for the harness it was written
//! for (`core::placement`, the layout read once per operation). Every
//! difference is a **move**; the module names no cause (a registry
//! correction, a retired key, the shared-folder preference and link/copy are
//! all just a different answer). The rows of one skill that land in one
//! Deployment folder are one move: they collapse into that folder's one row
//! (spec Q24), which is how the per-harness rows written before 1.4.0 are
//! converted explicitly, never reinterpreted in place.
//!
//! Two entry points, plus the journal finish:
//!
//! * [`preview`] — read-only: counts per scope and per cause, and the journal
//!   entries still owed. Feeds the launch offer, the Harnesses pane's
//!   "Move N Deployments" line and (tickets 12, 14) every confirmation's
//!   "would move N".
//! * [`redeploy`] — the mutation, under `mutation_guard::serialized`, with
//!   progress and no cancel; [`redeploy_unlocked`] is the seam a composite
//!   (a setting change that redeploys) calls inside its own guarded section.
//! * [`finish_journal`] — the launch check's first step: finish every journal
//!   entry a crash left, before anything else can mutate.
//!
//! **Per move, in order (spec Q4):**
//!
//! 1. **Deploy** at the new place through `sync_engine`'s capability-aware
//!    path, admitted inside the folder the destination was derived in. A new
//!    place that would nest with a live Deployment (an aliased folder puts
//!    one inside the other) is refused first, nothing written. An occupant
//!    that is ours (the row recorded there, or a link to the Library copy
//!    where links are deployed) or byte-identical to the skill is **adopted**
//!    — recorded as it is, never removed and re-created; anything else is a
//!    `TARGET_EXISTS` row for the one overwrite ask. A Keep answer leaves the
//!    move undone (`kept`, still pending). A genuine replacement — Overwrite,
//!    or ours in the wrong mode — is **staged** ([`staged_replace`]): deployed
//!    beside the occupant, journaled, then swapped in by one atomic exchange.
//!    Where the OS or file system offers none, the occupant is not touched and
//!    the move is refused, still pending (`DESTINATION_REFUSED`,
//!    `cannotReplaceSafely`). A Disabled row and a
//!    row that never deployed (no claim) are re-recorded at the new place with
//!    nothing written.
//! 2. **Switch** in one transaction: the folder's row takes the new key, path
//!    and mode, the rows it absorbs are deleted, and a **journal entry** is
//!    written per old artifact still on disk — its path and physical identity,
//!    the row that records the new place, and what the old row recorded as
//!    deployed.
//! 3. **Remove** each old artifact through `artifact_removal` and the removal
//!    fence, only when it is provably ours — a link resolving by physical
//!    identity to the skill's Library copy, or a copy whose Content identity
//!    equals the one its row recorded or the Library's — **and** no other live
//!    row records it (Amp keeps the folder Kimi Code left) **and** no live
//!    Deployment, the journaled new one included, lies inside it or resolves
//!    into it (`holds_deployment`). A copy that differs is an **edited
//!    copy**: a row of the same ask, delete or leave; leaving it disowns it
//!    and the report names its readers. A link elsewhere is never
//!    ours and is reported left behind. Each settled entry is dropped.
//!
//! **The journal.** Every run and the launch check finish the journal first.
//! An entry is dropped only on positive evidence: the new place verifiably
//! serves the skill (the occupant check returns ours for the moved row's
//! evidence, and it is reachable as a folder), or the skill record is gone.
//! Anything else — the new place absent, unreachable or not ours, a leftover
//! the entry cannot recognise, a removal that failed — keeps it journaled
//! (`unfinished`), counted by the preview and retried by every run and
//! launch; a path that cannot be resolved never authorises forgetting it.
//! Once the new place serves, the old artifact is removed only under the
//! rules of step 3, and what stays is reported left behind. An edited copy
//! awaiting an answer stays in the journal (`edited`) and is asked again by
//! the next run. An answer names exactly its question — an
//! occupied move's rows and place, an edited copy's journal entry and path —
//! and a stale one is refused with nothing touched. A crash at any step
//! leaves at most a duplicate, never a missing skill: the old artifact goes
//! only after the new one is recorded, and a replaced occupant only after the
//! new artifact is in its place.
//!
//! Core never resolves roots: callers pass the host roots and the configured
//! central repo (`commands::installer_paths`).

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use specta::Type;
use uuid::Uuid;

use super::artifact_removal::{
    recorded_target_paths, remove_superseded_artifact_unlocked, Superseded, SupersededAuthority,
};
use super::content_identity::{self, Source};
use super::environment::HostRoots;
use super::errors::{CommandError, DestinationRefusal, SignalError};
use super::global_sync::OverrideDecision;
use super::mutation_guard;
use super::no_replace::{ExchangeError, HeldDir};
use super::ownership::{self, Deployed, Occupancy};
use super::placement::{
    self, capability_entry, managed_global_dirs, readers_of_global_dir, row_reachable, DeployMode,
    InstalledSet, Placement, PlacementScope, PlacementSettings, ScopeKind,
};
use super::removal_fence::{self, Claim, EntryId, Fence, Place, ProtectedLibrary};
use super::settings;
use super::skill_store::{
    RedeployJournalRecord, SkillRecord, SkillStore, SkillTargetRecord, SwitchedRow,
    TargetTransition,
};
use super::sync_engine::{self, Occupant, Replace, TargetExistsError};
use super::sync_status::{SyncMode, SyncStatus};
use super::tool_adapters::{installed_set_in, ToolAdapter};

// ---------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------

/// What a redeploy would do, answered before anything is written.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct RedeployPreview {
    /// Global Deployments whose place differs from placement's answer.
    pub global: RedeployCounts,
    /// Journal entries still owed: an old artifact a crash left, or an edited
    /// copy awaiting the operator's answer.
    pub journal: u32,
    /// The harnesses (registry keys) whose Deployments move, in first-seen
    /// order — what the launch offer names.
    pub harnesses: Vec<String>,
}

/// Moves per cause at one scope.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct RedeployCounts {
    /// The Deployment's place on disk changes (its folder or its mode).
    pub correction: u32,
    /// Only the record changes: rows converted into their folder's one row,
    /// at the path they already have.
    pub migration: u32,
}

/// Why a move moves — counting only; the mechanism is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum RedeployCause {
    Correction,
    Migration,
}

/// A streamed step of the redeploy.
#[derive(Debug, Clone, Serialize, Type)]
pub struct RedeployProgress {
    /// Steps done before this one.
    pub done: u32,
    pub total: u32,
    /// The skill this step is about.
    pub name: String,
}

/// What a redeploy did. Crosses the wire as itself.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct RedeployReport {
    /// This run's moves, one per (skill, Deployment folder).
    pub moves: Vec<RedeployMove>,
    /// Journal entries an earlier run (or a crash) left, finished first.
    pub journal: Vec<JournalOutcome>,
    /// Old artifacts left on disk: duplicates, each with the harnesses that
    /// will show it.
    pub left_behind: Vec<LeftBehindCopy>,
    /// Edited copies awaiting the operator's answer (rows of the one ask).
    pub edited_copies: Vec<EditedCopy>,
    /// Per-harness disables a collapse into one folder row no longer holds
    /// (spec Q24).
    pub lost_disables: Vec<LostDisable>,
    /// Answers that named no question this run asks — an edited copy whose
    /// journal entry is gone or another, a move no longer planned. Refused:
    /// nothing was touched for them. (An answer naming a planned move whose
    /// rows or place changed fails that move instead.)
    pub refused_answers: Vec<RefusedAnswer>,
}

impl RedeployReport {
    fn is_empty(&self) -> bool {
        self.moves.is_empty()
            && self.journal.is_empty()
            && self.left_behind.is_empty()
            && self.edited_copies.is_empty()
            && self.lost_disables.is_empty()
            && self.refused_answers.is_empty()
    }
}

/// An answer refused because the question it answered is no longer asked:
/// `error` is the typed `NOT_FOUND` (`kind` `redeploy_question`, `id` the
/// question's identity — the journal entry, or the move's rows).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct RefusedAnswer {
    /// The path the answer's ask showed.
    pub path: String,
    pub error: CommandError,
}

/// One move: a skill's rows that land in one Deployment folder.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct RedeployMove {
    pub skill_id: String,
    pub skill_name: String,
    /// The Deployment folder the rows land in.
    pub folder_key: String,
    /// The skill's place in it.
    pub path: String,
    pub cause: RedeployCause,
    /// The rows as recorded before the move.
    pub rows: Vec<MovedRow>,
    pub status: RedeployStatus,
}

/// One row of a move, as it was recorded.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MovedRow {
    pub id: String,
    pub tool: String,
    /// Where it recorded its artifact.
    pub from: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RedeployStatus {
    /// Deployed at the new place and every old artifact settled.
    Moved,
    /// Ours or identical content was already at the new place and is kept.
    Adopted,
    /// Only the record changed: the artifact already was at the new place.
    Unchanged,
    /// Deployed and recorded, but an old artifact stays: an edited copy
    /// (`edited`, asked or left by the answer) or something not ours.
    LeftOld { edited: bool },
    /// Not moved: the operator kept the folder in the way. Still pending.
    Kept { reason: RedeployKept },
    /// Re-recorded at the new place with nothing written, or (a skill whose
    /// Disable left an artifact) not touched.
    Skipped { reason: RedeploySkip },
    /// Not moved; nothing was switched. Still pending.
    Failed { error: CommandError },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum RedeployKept {
    Declined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum RedeploySkip {
    /// Disabled: re-recorded with nothing written (Enable deploys it there).
    Disabled,
    /// Never deployed (no claim): re-recorded with nothing written.
    NotDeployed,
}

/// A journal entry an earlier run or a crash left, as this run settled it.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct JournalOutcome {
    /// The journal entry.
    pub entry_id: String,
    pub skill_id: String,
    pub skill_name: String,
    /// The key of the row that owned the old artifact.
    pub tool: String,
    /// The old artifact.
    pub path: String,
    pub status: JournalStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum JournalStatus {
    /// Removed, or already gone.
    Finished,
    /// Another live row records it: it stays as that row's Deployment.
    Retained,
    /// Left on disk and no longer journaled (see `left_behind`).
    LeftBehind,
    /// An edited copy still awaiting the operator's answer (see
    /// `edited_copies`); still journaled.
    AwaitingAnswer,
    /// Not finished: the new place does not verifiably serve the skill
    /// (absent, unreachable, not ours), or what the entry journaled could not
    /// be recognised or settled. Still journaled; every run and launch
    /// retries it.
    Unfinished,
}

/// An old artifact the redeploy left on disk: a duplicate of a moved skill.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LeftBehindCopy {
    pub skill_id: String,
    pub skill_name: String,
    pub tool: String,
    pub path: String,
    pub reason: RedeployLeftReason,
    /// The detected harnesses that read its folder and will show the skill
    /// twice.
    pub readers: Vec<String>,
    /// Why its removal failed (`removal_failed`).
    pub error: Option<CommandError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum RedeployLeftReason {
    /// An edited copy the operator chose to leave: no longer the app's.
    Edited,
    /// Not provably ours (a link elsewhere, a file): never removed.
    NotOurs,
    /// Something else occupies the old path since it was journaled.
    Replaced,
    /// Its skill is gone, or its journal entry could not be read: nothing
    /// was touched.
    Stale,
    /// Its removal failed or was refused.
    RemovalFailed,
    /// A live Deployment (the journaled new one, or another row's) lies
    /// inside it, or resolves into it through an aliased folder: removing it
    /// would remove that Deployment too.
    HoldsDeployment,
}

/// An edited copy: a row of the one overwrite ask (delete / leave).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct EditedCopy {
    /// The journal entry that asks — what an answer names.
    pub entry_id: String,
    pub skill_id: String,
    pub skill_name: String,
    /// The key of the row that owned it.
    pub tool: String,
    pub path: String,
    pub modified_at: Option<i64>,
    /// The detected harnesses that read its folder: who shows it twice if it
    /// is left.
    pub readers: Vec<String>,
}

/// A per-harness Disable that no longer holds once its row collapsed into a
/// folder row where another harness had the skill live (spec Q24).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LostDisable {
    pub skill_id: String,
    pub skill_name: String,
    /// The key of the disabled row.
    pub tool: String,
    pub folder_key: String,
}

/// The operator's answer to one row of the ask, naming exactly the question
/// it was asked — which rows or which journal entry, and the path it showed.
/// It applies only while that question is still the one asked: checked under
/// the guard, and a stale answer is refused with nothing touched. Input only.
#[derive(Debug, Clone, Deserialize, Type)]
#[serde(tag = "question", rename_all = "snake_case")]
pub enum RedeployAnswer {
    /// The folder in a move's way (`TARGET_EXISTS`): Overwrite or Keep.
    /// `row_ids` are the move's rows as the ask's report listed them
    /// (`RedeployMove.rows`); the move must still be exactly those rows.
    Occupied {
        skill_id: String,
        folder_key: String,
        row_ids: Vec<String>,
        path: String,
        decision: OverrideDecision,
    },
    /// An edited old copy: delete it, or leave it (disowned). `entry_id` is
    /// the journal entry that asked (`EditedCopy.entry_id`).
    EditedCopy {
        entry_id: String,
        path: String,
        decision: EditedCopyDecision,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum EditedCopyDecision {
    Delete,
    Leave,
}

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

/// What a redeploy under `settings` would do now. Read-only.
pub fn preview(
    store: &SkillStore,
    roots: &HostRoots,
    settings: &PlacementSettings,
) -> Result<RedeployPreview> {
    let installed = installed_set_in(roots);
    let layout = placement::place(PlacementScope::Global(roots), &installed, settings);
    let mut preview = RedeployPreview {
        journal: store.list_redeploy_journal()?.len() as u32,
        ..RedeployPreview::default()
    };
    for planned in plan(store, &layout, &installed)? {
        match planned.cause {
            RedeployCause::Correction => preview.global.correction += 1,
            RedeployCause::Migration => preview.global.migration += 1,
        }
        for row in &planned.rows {
            if !preview.harnesses.contains(&row.tool) {
                preview.harnesses.push(row.tool.clone());
            }
        }
    }
    Ok(preview)
}

/// Run the redeploy: finish the journal, then every move (see the module
/// docs), with `answers` from the one ask of an earlier run of the same
/// operator action. Per-move outcomes are report data; only a store failure
/// fails the whole operation.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
pub fn redeploy(
    store: &SkillStore,
    roots: &HostRoots,
    central_dir: &Path,
    answers: &[RedeployAnswer],
    now: i64,
    on_progress: impl FnMut(RedeployProgress),
) -> Result<RedeployReport> {
    mutation_guard::serialized(|| {
        redeploy_unlocked(
            store,
            roots,
            central_dir,
            &PlacementSettings::default(),
            answers,
            now,
            on_progress,
        )
    })
}

/// [`redeploy`] under `settings`, for a composite already holding the guard
/// (a setting change that redeploys in the same action). Unlocked seam.
pub(crate) fn redeploy_unlocked(
    store: &SkillStore,
    roots: &HostRoots,
    central_dir: &Path,
    settings: &PlacementSettings,
    answers: &[RedeployAnswer],
    now: i64,
    mut on_progress: impl FnMut(RedeployProgress),
) -> Result<RedeployReport> {
    let installed = installed_set_in(roots);
    let ctx = Ctx {
        store,
        roots,
        central_dir,
        installed: &installed,
        now,
    };
    let journal = store.list_redeploy_journal()?;
    // The layout is read once for the whole operation.
    let layout = placement::place(PlacementScope::Global(roots), &installed, settings);
    let planned = plan(store, &layout, &installed)?;
    let total = (journal.len() + planned.len()) as u32;
    let mut done = 0u32;
    let mut report = RedeployReport::default();

    refuse_unasked_answers(answers, &journal, &planned, &mut report);
    for entry in journal {
        on_progress(RedeployProgress {
            done,
            total,
            name: skill_name_of(store, &entry.skill_id)?,
        });
        done += 1;
        let answer = edited_answer(answers, &entry, &mut report);
        let settled = settle_entry(&ctx, &entry, answer)?;
        let status = record_settled(&ctx, &entry, settled, &mut report)?;
        report.journal.push(JournalOutcome {
            entry_id: entry.id.clone(),
            skill_id: entry.skill_id.clone(),
            skill_name: skill_name_of(store, &entry.skill_id)?,
            tool: entry.tool.clone(),
            path: entry.old_path.clone(),
            status,
        });
    }

    for planned in planned {
        on_progress(RedeployProgress {
            done,
            total,
            name: planned.skill.name.clone(),
        });
        done += 1;
        let moved = execute(&ctx, &planned, answers, &mut report)?;
        report.moves.push(moved);
    }
    Ok(report)
}

/// The launch check's first step: finish every journal entry an interrupted
/// run left (see the module docs), before anything else can mutate. Its
/// report, when it settled anything, is recorded for the frontend to take
/// once ([`take_finished_journal`]). Moves nothing else.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
pub fn finish_journal(
    store: &SkillStore,
    roots: &HostRoots,
    central_dir: &Path,
    now: i64,
) -> Result<Option<RedeployReport>> {
    mutation_guard::serialized(|| finish_journal_unlocked(store, roots, central_dir, now))
}

pub(crate) fn finish_journal_unlocked(
    store: &SkillStore,
    roots: &HostRoots,
    central_dir: &Path,
    now: i64,
) -> Result<Option<RedeployReport>> {
    let installed = installed_set_in(roots);
    let ctx = Ctx {
        store,
        roots,
        central_dir,
        installed: &installed,
        now,
    };
    let mut report = RedeployReport::default();
    for entry in store.list_redeploy_journal()? {
        let settled = settle_entry(&ctx, &entry, None)?;
        let status = record_settled(&ctx, &entry, settled, &mut report)?;
        report.journal.push(JournalOutcome {
            entry_id: entry.id.clone(),
            skill_id: entry.skill_id.clone(),
            skill_name: skill_name_of(store, &entry.skill_id)?,
            tool: entry.tool.clone(),
            path: entry.old_path.clone(),
            status,
        });
    }
    // An entry still awaiting its answer changed nothing: the launch offer
    // (the preview counts it) asks again, so it is not a run of its own.
    let settled_any = report
        .journal
        .iter()
        .any(|entry| entry.status != JournalStatus::AwaitingAnswer);
    if report.is_empty() || !settled_any {
        return Ok(None);
    }
    settings::record_redeploy_report(store, &serde_json::to_string(&report)?)?;
    Ok(Some(report))
}

/// The report of the launch check's journal finish, handed out once: taking
/// it removes it. A stored report that cannot be read is dropped and logged.
pub fn take_finished_journal(store: &SkillStore) -> Result<Option<RedeployReport>> {
    let Some(raw) = settings::take_redeploy_report(store)? else {
        return Ok(None);
    };
    match serde_json::from_str(&raw) {
        Ok(report) => Ok(Some(report)),
        Err(err) => {
            log::warn!("[redeploy] dropped an unreadable journal report: {err}");
            Ok(None)
        }
    }
}

// ---------------------------------------------------------------------------
// Planning
// ---------------------------------------------------------------------------

/// One move as planned: the rows of one skill that land in one folder.
struct Planned {
    skill: SkillRecord,
    folder_key: &'static str,
    folder_path: PathBuf,
    mode: DeployMode,
    /// The skill's place in the folder (joined lexically for a refused name,
    /// for the report only).
    dest: PathBuf,
    /// The refusal of a name that is not one folder name.
    refused: Option<CommandError>,
    rows: Vec<SkillTargetRecord>,
    cause: RedeployCause,
}

/// Every move `layout` asks for: per skill, its reachable rows grouped by the
/// folder that serves each row's harness (`placement::row_reachable` — a
/// legacy row through its own harness, a folder row through its folder; an
/// unknown key has no placement and is left as it is), keeping a group only
/// when some row differs from its place there.
fn plan(store: &SkillStore, layout: &Placement, installed: &InstalledSet) -> Result<Vec<Planned>> {
    let mut planned = Vec::new();
    for skill in store.list_skills()? {
        let mut groups: Vec<(&'static str, Vec<SkillTargetRecord>)> = Vec::new();
        for row in store.list_skill_targets(&skill.id)? {
            if !row_reachable(ScopeKind::Global, &row.tool, installed) {
                continue;
            }
            let Some(folder) = layout.folder_of(&row.tool) else {
                continue;
            };
            match groups.iter_mut().find(|(key, _)| *key == folder.key) {
                Some((_, rows)) => rows.push(row),
                None => groups.push((folder.key, vec![row])),
            }
        }
        for (folder_key, rows) in groups {
            let Some(folder) = layout.folder(folder_key) else {
                continue;
            };
            let (dest, refused) = match placement::deployment_path(&folder.path, &skill.name) {
                Ok(dest) => (dest, None),
                Err(err) => (
                    placement::refused_place(&err).unwrap_or_else(|| folder.path.clone()),
                    Some(CommandError::from_anyhow(err)),
                ),
            };
            let placed = |row: &SkillTargetRecord| {
                Path::new(&row.target_path) == dest && mode_holds(folder.mode, row)
            };
            if rows.iter().all(|row| row.tool == folder_key && placed(row)) {
                continue;
            }
            let cause = if rows.iter().all(placed) {
                RedeployCause::Migration
            } else {
                RedeployCause::Correction
            };
            planned.push(Planned {
                skill: skill.clone(),
                folder_key,
                folder_path: folder.path.clone(),
                mode: folder.mode,
                dest,
                refused,
                rows,
                cause,
            });
        }
    }
    Ok(planned)
}

/// Whether a row's recorded mode is what its folder's mode asks for. Only a
/// copy-mode folder is strict: a link-mode folder's hybrid deploy may itself
/// land on a copy (junction / copy fallback), so a recorded copy there is
/// already its answer. A row that owns nothing has nothing deployed to judge.
fn mode_holds(mode: DeployMode, row: &SkillTargetRecord) -> bool {
    !(mode == DeployMode::Copy && row.owns_artifact() && row.mode != SyncMode::Copy)
}

// ---------------------------------------------------------------------------
// Executing one move
// ---------------------------------------------------------------------------

struct Ctx<'a> {
    store: &'a SkillStore,
    roots: &'a HostRoots,
    central_dir: &'a Path,
    installed: &'a InstalledSet,
    now: i64,
}

/// A journal entry's state (`RedeployJournalRecord::state`): an old
/// artifact's removal is owed (`pending`); an edited copy awaits its answer
/// (`edited`); a folder the operator answered Overwrite for, set aside, is
/// owed its removal under that answer (`overwritten`); a staged replacement
/// is in progress (`replacing`, `replacing_answered` — [`staged_replace`]).
const STATE_PENDING: &str = "pending";
const STATE_EDITED: &str = "edited";
const STATE_OVERWRITTEN: &str = "overwritten";
const STATE_REPLACING: &str = "replacing";
const STATE_REPLACING_ANSWERED: &str = "replacing_answered";

fn execute(
    ctx: &Ctx<'_>,
    planned: &Planned,
    answers: &[RedeployAnswer],
    report: &mut RedeployReport,
) -> Result<RedeployMove> {
    let store = ctx.store;
    let skill = &planned.skill;
    let mut moved = RedeployMove {
        skill_id: skill.id.clone(),
        skill_name: skill.name.clone(),
        folder_key: planned.folder_key.to_string(),
        path: planned.dest.to_string_lossy().into_owned(),
        cause: planned.cause,
        rows: planned
            .rows
            .iter()
            .map(|row| MovedRow {
                id: row.id.clone(),
                tool: row.tool.clone(),
                from: row.target_path.clone(),
            })
            .collect(),
        status: RedeployStatus::Unchanged,
    };
    if let Some(refused) = &planned.refused {
        moved.status = RedeployStatus::Failed {
            error: refused.clone(),
        };
        return Ok(moved);
    }

    let skill_disabled = store.skill_disabled_at(&skill.id)?.is_some();
    let live: Vec<&SkillTargetRecord> = planned
        .rows
        .iter()
        .filter(|row| row.owns_artifact() && row.status != SyncStatus::Disabled)
        .collect();
    // A skill off everywhere whose Disable left an artifact (its rows kept
    // `error`, ADR-0002): nothing is written for it and nothing re-recorded,
    // or the artifact would lose its row. Finishing the Disable or Enable
    // settles it; the move stays pending.
    if skill_disabled && planned.rows.iter().any(SkillTargetRecord::owns_artifact) {
        moved.status = RedeployStatus::Skipped {
            reason: RedeploySkip::Disabled,
        };
        return Ok(moved);
    }
    let survivor = planned
        .rows
        .iter()
        .find(|row| row.tool == planned.folder_key)
        .unwrap_or(&planned.rows[0]);
    let absorbed: Vec<&str> = planned
        .rows
        .iter()
        .filter(|row| row.id != survivor.id)
        .map(|row| row.id.as_str())
        .collect();
    let dest = planned.dest.to_string_lossy().into_owned();

    if live.is_empty() || skill_disabled {
        // Nothing deployed: re-record at the new place, nothing written.
        let all_disabled = planned
            .rows
            .iter()
            .all(|row| row.status == SyncStatus::Disabled);
        let state = if all_disabled {
            survivor
        } else {
            planned
                .rows
                .iter()
                .find(|row| row.status != SyncStatus::Disabled)
                .unwrap_or(survivor)
        };
        store.switch_redeployed_rows(
            &survivor.id,
            SwitchedRow {
                tool: planned.folder_key,
                target_path: &dest,
                mode: planned.mode.first_attempt(),
                status: state.status,
                last_error: state.last_error.as_deref(),
                synced_at: None,
                content_hash: None,
            },
            &absorbed,
            &[],
            &[],
        )?;
        if !all_disabled {
            note_lost_disables(planned, report);
        }
        moved.status = RedeployStatus::Skipped {
            reason: if all_disabled {
                RedeploySkip::Disabled
            } else {
                RedeploySkip::NotDeployed
            },
        };
        return Ok(moved);
    }

    // The operator's answer for this move, if it is about this move: it must
    // name exactly the rows the ask's report listed, and the place it showed.
    // Anything else is a stale answer, refused with nothing touched.
    let answer = answers.iter().find_map(|answer| match answer {
        RedeployAnswer::Occupied {
            skill_id,
            folder_key,
            row_ids,
            path,
            decision,
        } if *skill_id == skill.id && folder_key == planned.folder_key => {
            Some((row_ids, path.as_str(), *decision))
        }
        _ => None,
    });
    if let Some((row_ids, asked, _)) = answer {
        if Path::new(asked) != planned.dest || !same_rows(row_ids, &planned.rows) {
            moved.status = RedeployStatus::Failed {
                error: if Path::new(asked) != planned.dest {
                    CommandError::from(SignalError::NotFound {
                        kind: "target".to_string(),
                        id: asked.to_string(),
                    })
                } else {
                    stale_answer(&row_ids.join(","))
                },
            };
            return Ok(moved);
        }
    }
    let answer = answer.map(|(_, _, decision)| decision);
    if answer == Some(OverrideDecision::Keep) {
        settle_missing_old(ctx, &live, "the operator kept the folder at its new place")?;
        moved.status = RedeployStatus::Kept {
            reason: RedeployKept::Declined,
        };
        return Ok(moved);
    }

    // Every live artifact already is the new place (the record names it, or an
    // aliased folder on the way makes it physically the same entry) and its
    // mode holds: only the record changes. Nothing is written or removed, and
    // the folder's row keeps what the artifact's own row recorded about it.
    let in_place = live.iter().all(|row| {
        (Path::new(&row.target_path) == planned.dest
            || same_entry(Path::new(&row.target_path), &planned.dest))
            && mode_holds(planned.mode, row)
    });
    if in_place {
        let recorded = live
            .iter()
            .find(|row| row.status != SyncStatus::Synced)
            .or_else(|| {
                live.iter()
                    .find(|row| Path::new(&row.target_path) == planned.dest)
            })
            .unwrap_or(&live[0]);
        store.switch_redeployed_rows(
            &survivor.id,
            SwitchedRow {
                tool: planned.folder_key,
                target_path: &dest,
                mode: recorded.mode,
                status: recorded.status,
                last_error: recorded.last_error.as_deref(),
                synced_at: recorded.synced_at,
                content_hash: recorded.content_hash.as_deref(),
            },
            &absorbed,
            &[],
            &[],
        )?;
        note_lost_disables(planned, report);
        moved.status = RedeployStatus::Unchanged;
        return Ok(moved);
    }

    // The new place and a live Deployment must not nest (an aliased folder
    // can put one inside the other): writing there would change that
    // Deployment, and removing the old one later would take the new one with
    // it. Refused before anything is touched; the move stays pending.
    if let Some(blocking) = overlapping_deployment(ctx, &planned.dest)? {
        moved.status = RedeployStatus::Failed {
            error: CommandError::from(SignalError::DestinationRefused {
                path: dest.clone(),
                resolved: blocking.to_string_lossy().into_owned(),
                reason: DestinationRefusal::OverlapsDeployment,
            }),
        };
        return Ok(moved);
    }

    // 1. Deploy at the new place.
    let Some(capability) = capability_entry(ScopeKind::Global, planned.folder_key) else {
        moved.status = RedeployStatus::Failed {
            error: CommandError::from(SignalError::UnknownTool {
                tool: planned.folder_key.to_string(),
            }),
        };
        return Ok(moved);
    };
    let deployment = match deploy(ctx, planned, &survivor.id, capability, answer) {
        Ok(deployment) => deployment,
        Err(err) => {
            let error = match err
                .chain()
                .find_map(|cause| cause.downcast_ref::<TargetExistsError>())
            {
                Some(exists) => CommandError::TargetExists {
                    path: dest.clone(),
                    modified_at: exists.occupant.modified_at,
                    same_content: exists.occupant.same_content,
                },
                None => CommandError::from_anyhow(err),
            };
            // Truthful status (audit 3 F14): a row whose old artifact is gone
            // and that could not be deployed at its new place is not
            // `synced` anywhere — whether the ask is still to come or not.
            settle_missing_old(
                ctx,
                &live,
                "the redeploy could not deploy it at its new place",
            )?;
            moved.status = RedeployStatus::Failed { error };
            return Ok(moved);
        }
    };

    // 2. Switch the rows and journal the old artifacts, in one transaction.
    let new_entry = match JournaledEntry::of(&planned.dest)? {
        Some(entry) => entry,
        None => anyhow::bail!(
            "redeploy: {:?} is gone right after its deploy",
            planned.dest
        ),
    };
    let mut journal: Vec<RedeployJournalRecord> = deployment.set_aside.into_iter().collect();
    for row in &live {
        let old = Path::new(&row.target_path);
        if old == planned.dest {
            continue;
        }
        match JournaledEntry::of(old)? {
            Some(id) if id.id != new_entry.id => journal.push(RedeployJournalRecord {
                id: Uuid::new_v4().to_string(),
                row_id: survivor.id.clone(),
                skill_id: skill.id.clone(),
                tool: row.tool.clone(),
                old_path: row.target_path.clone(),
                old_entry: serde_json::to_string(&id)?,
                old_content_hash: row.content_hash.clone(),
                new_path: dest.clone(),
                new_entry: serde_json::to_string(&new_entry)?,
                state: STATE_PENDING.to_string(),
                created_at: ctx.now,
            }),
            // Gone already, or physically the new place itself.
            _ => {}
        }
    }
    #[cfg(test)]
    test_seams::before_switch();
    let finished: Vec<&str> = deployment.replacing.iter().map(String::as_str).collect();
    store.switch_redeployed_rows(
        &survivor.id,
        SwitchedRow {
            tool: planned.folder_key,
            target_path: &dest,
            mode: deployment.mode,
            status: SyncStatus::Synced,
            last_error: None,
            synced_at: Some(ctx.now),
            content_hash: deployment.content_hash.as_deref(),
        },
        &absorbed,
        &finished,
        &journal,
    )?;
    note_lost_disables(planned, report);
    #[cfg(test)]
    test_seams::after_switch();

    // 3. Remove the old artifacts that are provably ours.
    // An old artifact stays when it is an edited copy (asked, or left by the
    // answer) or anything not provably ours.
    let mut left_old: Option<bool> = None;
    for entry in &journal {
        let settled = settle_entry(ctx, entry, None)?;
        let stays = match &settled {
            Settled::Awaiting => Some(true),
            Settled::Left { reason, .. } => Some(*reason == RedeployLeftReason::Edited),
            Settled::Unfinished => Some(false),
            Settled::Finished | Settled::Retained => None,
        };
        if let Some(edited) = stays {
            left_old = Some(left_old.unwrap_or(false) || edited);
        }
        record_settled(ctx, entry, settled, report)?;
    }
    moved.status = match left_old {
        Some(edited) => RedeployStatus::LeftOld { edited },
        None if deployment.adopted => RedeployStatus::Adopted,
        None => RedeployStatus::Moved,
    };
    Ok(moved)
}

/// Whether an answer's `row_ids` are exactly the move's rows (in any order).
fn same_rows(row_ids: &[String], rows: &[SkillTargetRecord]) -> bool {
    row_ids.len() == rows.len() && rows.iter().all(|row| row_ids.contains(&row.id))
}

/// What step 1 left at the new place, for the switch.
struct Deployment {
    mode: SyncMode,
    /// The Content identity deployed there (a copy).
    content_hash: Option<String>,
    /// Ours or identical content was already there and is recorded as it is.
    adopted: bool,
    /// A staged replacement's journal entry, which the switch drops.
    replacing: Option<String>,
    /// The occupant a staged replacement set aside: an old artifact the
    /// switch journals for removal.
    set_aside: Option<RedeployJournalRecord>,
}

/// What occupies the new place, judged for the deploy (ADR-0007).
enum Judged {
    /// What this move would deploy, or what its own row recorded there: kept
    /// and recorded as it is, never removed and re-created.
    Acceptable {
        mode: SyncMode,
        content_hash: Option<String>,
    },
    /// Ours, in the wrong mode (a link where the folder takes copies): a
    /// genuine replacement, staged.
    Ours,
    /// Not provably ours: the overwrite ask decides.
    Foreign,
}

/// Step 1, the deploy at the new place. Nothing there: a first write. An
/// acceptable occupant (ours, or identical content) is adopted as it is. A
/// replacement — ours in the wrong mode, or a folder the operator answered
/// Overwrite for — is staged ([`staged_replace`]). Anything else is the
/// typed `TARGET_EXISTS` for the ask.
fn deploy(
    ctx: &Ctx<'_>,
    planned: &Planned,
    survivor: &str,
    capability: &ToolAdapter,
    answer: Option<OverrideDecision>,
) -> Result<Deployment> {
    let store = ctx.store;
    let skill = &planned.skill;
    let identity = || Source::Managed {
        store,
        skill_id: &skill.id,
    };
    let managed = [planned.folder_path.clone()];
    let library = ProtectedLibrary::new(store, ctx.central_dir);
    // Admitted like any write — never into the Library, only inside the
    // folder managed for it — before anything is written, replaced or
    // adopted (an adopted Library copy would be recorded as a Deployment).
    sync_engine::admit_destination(&planned.dest, &Replace::never().within(&managed), &library)?;
    let copy_hash = |mode: SyncMode| {
        mode.can_drift()
            .then(|| content_identity::read(identity()))
            .flatten()
    };
    let Some(occupant) = JournaledEntry::of(&planned.dest)? else {
        let outcome = sync_engine::sync_dir_for_tool_with_overwrite(
            capability,
            Path::new(&skill.central_path),
            identity(),
            &planned.dest,
            Replace::never().within(&managed),
            &library,
        )?;
        return Ok(Deployment {
            mode: outcome.mode_used,
            content_hash: copy_hash(outcome.mode_used),
            adopted: false,
            replacing: None,
            set_aside: None,
        });
    };
    let at_dest: Vec<&SkillTargetRecord> = planned
        .rows
        .iter()
        .filter(|row| {
            row.owns_artifact()
                && (Path::new(&row.target_path) == planned.dest
                    || same_entry(Path::new(&row.target_path), &planned.dest))
        })
        .collect();
    let (answered, evidence_hash) = match judge_occupant(ctx, planned, &at_dest, occupant)? {
        Judged::Acceptable { mode, content_hash } => {
            return Ok(Deployment {
                mode,
                content_hash,
                adopted: true,
                replacing: None,
                set_aside: None,
            })
        }
        Judged::Ours => (
            false,
            at_dest.iter().find_map(|row| row.content_hash.clone()),
        ),
        Judged::Foreign if answer == Some(OverrideDecision::Overwrite) => (true, None),
        Judged::Foreign => anyhow::bail!(TargetExistsError {
            target: planned.dest.clone(),
            occupant: Occupant::observe(identity(), &planned.dest),
        }),
    };
    let staged = staged_replace(
        ctx,
        planned,
        survivor,
        capability,
        occupant,
        Authority {
            answered,
            evidence_hash,
        },
        &managed,
        &library,
    )?;
    Ok(Deployment {
        mode: staged.mode,
        content_hash: copy_hash(staged.mode),
        adopted: false,
        replacing: Some(staged.replacing),
        set_aside: Some(staged.set_aside),
    })
}

/// Judge the occupant of the new place, now: a link resolving to the skill's
/// Library copy is acceptable where links are deployed (ours, in the wrong
/// mode where copies are); a real folder is acceptable when its content is
/// the Library's, or what a row of this move recorded deploying there. Any
/// other entry, or one the check cannot read, is foreign.
fn judge_occupant(
    ctx: &Ctx<'_>,
    planned: &Planned,
    at_dest: &[&SkillTargetRecord],
    occupant: JournaledEntry,
) -> Result<Judged> {
    let skill = &planned.skill;
    let dest = &planned.dest;
    let ours = |deployed: &Deployed| match ownership::check(dest, deployed) {
        Ok(occupancy) => occupancy == Occupancy::Ours,
        Err(err) => {
            log::warn!("[redeploy] occupant of {:?} not verified: {err:#}", dest);
            false
        }
    };
    if occupant.link {
        let library_link = Deployed::recorded(true, Some(Path::new(&skill.central_path)), None);
        if !ours(&library_link) {
            return Ok(Judged::Foreign);
        }
        if planned.mode == DeployMode::Copy {
            return Ok(Judged::Ours);
        }
        let mode = at_dest
            .iter()
            .map(|row| row.mode)
            .find(|mode| matches!(mode, SyncMode::Symlink | SyncMode::Junction))
            .unwrap_or(SyncMode::Symlink);
        return Ok(Judged::Acceptable {
            mode,
            content_hash: None,
        });
    }
    if !std::fs::symlink_metadata(dest).is_ok_and(|meta| meta.is_dir()) {
        return Ok(Judged::Foreign);
    }
    let identity = Source::Managed {
        store: ctx.store,
        skill_id: &skill.id,
    };
    if content_identity::same_content(identity, dest) {
        return Ok(Judged::Acceptable {
            mode: SyncMode::Copy,
            content_hash: content_identity::read(Source::Managed {
                store: ctx.store,
                skill_id: &skill.id,
            }),
        });
    }
    for row in at_dest {
        if ours(&Deployed::of_target(row, Some(skill))) {
            return Ok(Judged::Acceptable {
                mode: SyncMode::Copy,
                content_hash: row.content_hash.clone(),
            });
        }
    }
    Ok(Judged::Foreign)
}

/// The live Deployment, if any, that the new place `dest` would nest with:
/// a recorded artifact (any skill, either scope) that is a real folder `dest`
/// lies inside, or one whose parent chain runs through the real folder at
/// `dest`. An artifact physically at `dest` is the destination itself, never
/// a nesting. One that cannot be read counts as nesting (refused, never
/// guessed).
fn overlapping_deployment(ctx: &Ctx<'_>, dest: &Path) -> Result<Option<PathBuf>> {
    let Some(parent) = dest.parent() else {
        return Ok(None);
    };
    // Where the new place is; not knowing refuses (never guessed).
    let placed = (|| -> Result<_> {
        let dest_dir = if real_dir(dest) {
            Some(Place::of(dest)?)
        } else {
            None
        };
        Ok((Place::of(parent)?, EntryId::of(dest)?, dest_dir))
    })();
    let Ok((into, dest_id, dest_dir)) = placed else {
        return Ok(Some(dest.to_path_buf()));
    };
    for path in recorded_target_paths(ctx.store, ctx.roots)? {
        if path == dest {
            continue;
        }
        let id = match EntryId::of(&path) {
            Ok(Some(id)) => id,
            Ok(None) => continue,
            Err(_) => return Ok(Some(path)),
        };
        if Some(id) == dest_id {
            continue;
        }
        let nests = (|| -> Result<bool> {
            if real_dir(&path) && into.within(&Place::of(&path)?) {
                return Ok(true);
            }
            Ok(match (&dest_dir, path.parent()) {
                (Some(dest_dir), Some(parent)) => Place::of(parent)?.within(dest_dir),
                _ => false,
            })
        })();
        if nests.unwrap_or(true) {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

/// Whether `path` names a real folder (not a link, not a file).
fn real_dir(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_dir())
}

// ---------------------------------------------------------------------------
// A staged replacement
// ---------------------------------------------------------------------------

/// Why a staged replacement may remove the occupant it sets aside.
struct Authority {
    /// The operator answered Overwrite for it.
    answered: bool,
    /// What the move's row recorded deploying there (evidence the occupant
    /// is ours, beside the Library link).
    evidence_hash: Option<String>,
}

struct Staged {
    mode: SyncMode,
    replacing: String,
    set_aside: RedeployJournalRecord,
}

const STAGE_PREFIX: &str = ".skills-hub-redeploy-";
const STAGE_NEW: &str = ".new";

/// Replace the occupant at the new place so that an interruption at any
/// step is finished from the journal, never leaving the skill missing:
///
/// 1. admit the occupant's removal (the Library, the managed folder, mounts)
///    before anything is written;
/// 2. deploy the skill beside it, at a staged sibling
///    (`.skills-hub-redeploy-<id>.new`);
/// 3. journal the replacement (`replacing`): the occupant and the staged
///    artifact, by path and physical identity;
/// 4. swap them in one atomic exchange (`no_replace::HeldDir::exchange`),
///    the occupant checked to still be the one judged right before: the new
///    place is never empty, and the occupant lands at the staged name;
/// 5. the switch (the caller) drops the `replacing` entry and journals the
///    occupant, now at the staged name, as an old artifact, removed like any
///    other.
///
/// Where the OS or file system offers no exchange (Windows always; SMB,
/// exFAT, …) the occupant is never touched: what was staged is undone, its
/// journal entry dropped, and the move fails with `DESTINATION_REFUSED`
/// (`cannotReplaceSafely`), still pending until the operator moves the
/// folder aside. Two renames would leave the new place empty between them,
/// and no proof that another Deployment keeps serving meanwhile has held
/// (round 20 t10 review, findings 7–12 and the round 4 disposition).
///
/// A failure before the swap undoes what was staged; what an interruption
/// leaves, the journal finish completes ([`settle_replacing`]).
#[allow(clippy::too_many_arguments)]
fn staged_replace(
    ctx: &Ctx<'_>,
    planned: &Planned,
    survivor: &str,
    capability: &ToolAdapter,
    occupant: JournaledEntry,
    authority: Authority,
    managed: &[PathBuf],
    library: &ProtectedLibrary<'_>,
) -> Result<Staged> {
    let store = ctx.store;
    let skill = &planned.skill;
    let dest = &planned.dest;
    let central = Path::new(&skill.central_path);
    let (Some(folder), Some(name)) = (dest.parent(), dest.file_name()) else {
        anyhow::bail!("redeploy: {:?} names no entry", dest);
    };
    let id = Uuid::new_v4().to_string();
    let stage_name = format!("{STAGE_PREFIX}{id}{STAGE_NEW}");
    let stage = folder.join(&stage_name);

    // 1. The occupant could be removed at all.
    removal_fence::admit(
        dest,
        &Fence::new(library)
            .sparing(central)
            .claiming(Claim::Within(managed)),
    )?;
    // 2. Stage the new artifact.
    let discard_stage = || {
        if let Err(err) = remove_superseded_artifact_unlocked(
            store,
            ctx.central_dir,
            &stage,
            managed,
            SupersededAuthority::Answered,
        ) {
            log::warn!("[redeploy] could not discard {:?}: {err:#}", stage);
        }
    };
    let outcome = match sync_engine::sync_dir_for_tool_with_overwrite(
        capability,
        central,
        Source::Managed {
            store,
            skill_id: &skill.id,
        },
        &stage,
        Replace::never().within(managed),
        library,
    ) {
        Ok(outcome) => outcome,
        Err(err) => {
            discard_stage();
            return Err(err);
        }
    };
    let Some(staged) = JournaledEntry::of(&stage)? else {
        anyhow::bail!("redeploy: {:?} is gone right after its deploy", stage);
    };
    // 3. Journal the replacement before the occupant moves.
    let record = RedeployJournalRecord {
        id: id.clone(),
        row_id: survivor.to_string(),
        skill_id: skill.id.clone(),
        tool: planned.folder_key.to_string(),
        old_path: dest.to_string_lossy().into_owned(),
        old_entry: serde_json::to_string(&occupant)?,
        old_content_hash: authority.evidence_hash.clone(),
        new_path: stage.to_string_lossy().into_owned(),
        new_entry: serde_json::to_string(&staged)?,
        state: if authority.answered {
            STATE_REPLACING_ANSWERED
        } else {
            STATE_REPLACING
        }
        .to_string(),
        created_at: ctx.now,
    };
    if let Err(err) = store.insert_redeploy_journal_entry(&record) {
        discard_stage();
        return Err(err);
    }
    let undo = |err: anyhow::Error| -> Result<Staged> {
        discard_stage();
        store.delete_redeploy_journal_entry(&id)?;
        Err(err)
    };
    #[cfg(test)]
    test_seams::before_swap();
    // 4. The occupant is still the one judged; swap the new one in.
    if JournaledEntry::of(dest)? != Some(occupant) {
        return undo(anyhow::anyhow!(TargetExistsError {
            target: dest.clone(),
            occupant: Occupant::observe(
                Source::Managed {
                    store,
                    skill_id: &skill.id,
                },
                dest,
            ),
        }));
    }
    let held = match HeldDir::open(folder) {
        Ok(held) => held,
        Err(err) => return undo(err.into()),
    };
    #[cfg(test)]
    let exchanged = if test_seams::no_exchange() {
        Err(ExchangeError::Unsupported)
    } else {
        held.exchange(name, stage_name.as_ref())
    };
    #[cfg(not(test))]
    let exchanged = held.exchange(name, stage_name.as_ref());
    match exchanged {
        Ok(()) => {}
        Err(ExchangeError::Failed(err)) => {
            return undo(anyhow::Error::new(err).context(format!("redeploy: swap at {:?}", dest)))
        }
        // No exchange: the occupant is not touched.
        Err(ExchangeError::Unsupported) => return undo(cannot_replace_safely(dest)),
    }
    #[cfg(test)]
    test_seams::after_swap();
    Ok(Staged {
        mode: outcome.mode_used,
        replacing: id,
        set_aside: RedeployJournalRecord {
            id: Uuid::new_v4().to_string(),
            row_id: survivor.to_string(),
            skill_id: skill.id.clone(),
            tool: planned.folder_key.to_string(),
            old_path: stage.to_string_lossy().into_owned(),
            old_entry: serde_json::to_string(&occupant)?,
            old_content_hash: authority.evidence_hash,
            new_path: dest.to_string_lossy().into_owned(),
            new_entry: serde_json::to_string(&staged)?,
            state: if authority.answered {
                STATE_OVERWRITTEN
            } else {
                STATE_PENDING
            }
            .to_string(),
            created_at: ctx.now,
        },
    })
}

/// The refusal to replace the occupant at `dest` (`cannotReplaceSafely`):
/// nothing touched, the move pending until the operator moves it aside.
fn cannot_replace_safely(dest: &Path) -> anyhow::Error {
    let path = dest.to_string_lossy().into_owned();
    anyhow::anyhow!(SignalError::DestinationRefused {
        resolved: path.clone(),
        path,
        reason: DestinationRefusal::CannotReplaceSafely,
    })
}

/// Q24: every Disabled row that collapsed into a folder row where another
/// row had the skill live no longer holds — report data naming it.
fn note_lost_disables(planned: &Planned, report: &mut RedeployReport) {
    for row in planned
        .rows
        .iter()
        .filter(|row| row.status == SyncStatus::Disabled)
    {
        report.lost_disables.push(LostDisable {
            skill_id: planned.skill.id.clone(),
            skill_name: planned.skill.name.clone(),
            tool: row.tool.clone(),
            folder_key: planned.folder_key.to_string(),
        });
    }
}

/// A live row whose old artifact is gone and that did not move is not
/// deployed anywhere: settle it `error` (its claim kept, ADR-0002) so it
/// never reads `synced` over a missing folder (audit 3 F14).
fn settle_missing_old(ctx: &Ctx<'_>, live: &[&SkillTargetRecord], detail: &str) -> Result<()> {
    for row in live {
        if std::fs::symlink_metadata(&row.target_path).is_err() {
            let error = format!("{} is missing; {detail}", row.target_path);
            ctx.store
                .transition_skill_target(&row.id, TargetTransition::SyncFailed { error: &error })?;
        }
    }
    Ok(())
}

/// The modification time of the entry at `path` (ms since the epoch), as
/// the ask shows it.
fn modified_at(path: &Path) -> Option<i64> {
    std::fs::symlink_metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|since| since.as_millis() as i64)
}

/// An artifact as the journal records it: its physical identity and whether
/// it is a link. Identity is a necessary condition, never a sufficient one —
/// a volume may reuse an index once an entry is gone — so the kind is
/// compared too, and the occupant check still decides ownership at the moment
/// of removal (an unproven folder is asked about, never removed blind).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct JournaledEntry {
    #[serde(flatten)]
    id: EntryId,
    link: bool,
}

impl JournaledEntry {
    fn of(path: &Path) -> std::io::Result<Option<Self>> {
        let Some(id) = EntryId::of(path)? else {
            return Ok(None);
        };
        let link = match std::fs::symlink_metadata(path) {
            Ok(meta) => meta.file_type().is_symlink(),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err),
        };
        Ok(Some(Self { id, link }))
    }
}

/// Whether `a` and `b` name one present entry (an aliased folder on the way).
fn same_entry(a: &Path, b: &Path) -> bool {
    matches!((EntryId::of(a), EntryId::of(b)), (Ok(Some(x)), Ok(Some(y))) if x == y)
}

// ---------------------------------------------------------------------------
// Settling one journal entry
// ---------------------------------------------------------------------------

/// How one journal entry settled.
enum Settled {
    /// Removed now, or already gone.
    Finished,
    /// Another live row records it.
    Retained,
    /// Left on disk, no longer journaled (the new place serves the skill,
    /// or the skill is gone).
    Left {
        reason: RedeployLeftReason,
        error: Option<CommandError>,
        /// What stays, when it is not the entry's old path (a staged
        /// replacement's artifacts).
        at: Option<PathBuf>,
    },
    /// An edited copy awaiting the answer; stays journaled.
    Awaiting,
    /// No positive evidence to let it go (the module's journal rule); stays
    /// journaled, retried by every run and launch.
    Unfinished,
}

fn left(reason: RedeployLeftReason) -> Settled {
    Settled::Left {
        reason,
        error: None,
        at: None,
    }
}

fn left_at(reason: RedeployLeftReason, at: &Path) -> Settled {
    Settled::Left {
        reason,
        error: None,
        at: Some(at.to_path_buf()),
    }
}

/// Settle `entry` now (the module's journal rule). The entry goes only on
/// positive evidence: its skill is gone, or the new place verifiably serves
/// the skill ([`serves`]). Then the old artifact is removed when no live
/// Deployment is it or lies inside it, and it is provably ours or the
/// operator answered Delete (or Overwrite, for a folder set aside); what
/// stays is reported left behind. Anything else keeps the entry
/// ([`Settled::Unfinished`]).
fn settle_entry(
    ctx: &Ctx<'_>,
    entry: &RedeployJournalRecord,
    answer: Option<EditedCopyDecision>,
) -> Result<Settled> {
    let store = ctx.store;
    let Some(skill) = store.get_skill_by_id(&entry.skill_id)? else {
        return Ok(skill_gone(entry));
    };
    if matches!(
        entry.state.as_str(),
        STATE_REPLACING | STATE_REPLACING_ANSWERED
    ) {
        return settle_replacing(ctx, entry, &skill);
    }
    let new = Path::new(&entry.new_path);
    // The moved row still records the new place, and it serves the skill
    // there: otherwise the old artifact may be the only copy.
    let row = store.get_skill_target_by_id(&entry.row_id)?;
    let row = row.filter(|row| Path::new(&row.target_path) == new);
    if !row.is_some_and(|row| serves(ctx, &skill, new, Some(&row))) {
        return Ok(Settled::Unfinished);
    }
    let Ok(old_id) = serde_json::from_str::<JournaledEntry>(&entry.old_entry) else {
        log::warn!(
            "[redeploy] unreadable journal entry {}; nothing was removed",
            entry.id
        );
        return Ok(left(RedeployLeftReason::Stale));
    };
    let old = Path::new(&entry.old_path);
    match JournaledEntry::of(old) {
        Ok(None) => return Ok(Settled::Finished),
        Ok(Some(_)) if same_entry(old, new) => return Ok(Settled::Finished),
        Ok(Some(now)) if now != old_id => return Ok(left(RedeployLeftReason::Replaced)),
        Ok(Some(_)) => {}
        Err(err) => {
            log::warn!("[redeploy] cannot read {:?}; kept pending: {err}", old);
            return Ok(Settled::Unfinished);
        }
    }
    if let Some(kept) = protected(ctx, old, old_id, new)? {
        return Ok(kept);
    }
    let authority = if entry.state == STATE_OVERWRITTEN {
        Some(EditedCopyDecision::Delete)
    } else {
        answer
    };
    remove_old(ctx, &skill, entry, old, authority)
}

/// Whether `path` verifiably serves `skill`: reachable as a folder, and the
/// occupant check finds it ours by `row`'s own evidence (the moved row) or by
/// the Library's (a link resolving to the Library copy, or a copy whose
/// Content identity is the Library's). Anything unreadable does not serve.
fn serves(
    ctx: &Ctx<'_>,
    skill: &SkillRecord,
    path: &Path,
    row: Option<&SkillTargetRecord>,
) -> bool {
    if !std::fs::metadata(path).is_ok_and(|meta| meta.is_dir()) {
        return false;
    }
    let central = Path::new(&skill.central_path);
    let library_hash = content_identity::read(Source::Managed {
        store: ctx.store,
        skill_id: &skill.id,
    });
    let library = Deployed::recorded(true, Some(central), library_hash.as_deref());
    row.map(|row| Deployed::of_target(row, Some(skill)))
        .into_iter()
        .chain([library])
        .any(|deployed| matches!(ownership::check(path, &deployed), Ok(Occupancy::Ours)))
}

/// The entry of a skill that is gone (deleted since): nothing is owed to it,
/// so the entry goes. What it journaled and is still on disk — the old
/// artifact, or a replacement's staged sibling — is reported left behind,
/// untouched.
fn skill_gone(entry: &RedeployJournalRecord) -> Settled {
    let stays = if matches!(
        entry.state.as_str(),
        STATE_REPLACING | STATE_REPLACING_ANSWERED
    ) {
        Path::new(&entry.new_path)
    } else {
        Path::new(&entry.old_path)
    };
    if std::fs::symlink_metadata(stays).is_ok() {
        left_at(RedeployLeftReason::Stale, stays)
    } else {
        Settled::Finished
    }
}

/// Whether a live Deployment keeps the old artifact `old` (identity
/// `old_id`): another row (any skill, either scope) records it — it is that
/// row's Deployment (`Retained`) — or, `old` being a real folder, the
/// journaled new place `new` or a recorded Deployment lies inside it or
/// resolves into it through an aliased folder (`HoldsDeployment`): removing
/// it would remove that one too. A path that cannot be read counts as
/// keeping it (never removed blind).
fn protected(
    ctx: &Ctx<'_>,
    old: &Path,
    old_id: JournaledEntry,
    new: &Path,
) -> Result<Option<Settled>> {
    let old_place = if real_dir(old) {
        match Place::of(old) {
            Ok(place) => Some(place),
            Err(_) => return Ok(Some(left(RedeployLeftReason::HoldsDeployment))),
        }
    } else {
        None
    };
    // Its parent chain runs through the old folder, or the entry itself
    // resolves into it.
    let holds = |path: &Path| match &old_place {
        Some(old_place) => {
            let through = |at: &Path| Place::of(at).map_or(true, |place| place.within(old_place));
            path.parent().is_some_and(through) || through(path)
        }
        None => false,
    };
    if holds(new) {
        return Ok(Some(left(RedeployLeftReason::HoldsDeployment)));
    }
    for path in recorded_target_paths(ctx.store, ctx.roots)? {
        match EntryId::of(&path) {
            Ok(Some(id)) if id == old_id.id => return Ok(Some(Settled::Retained)),
            Ok(_) => {}
            Err(_) => return Ok(Some(Settled::Retained)),
        }
        if holds(&path) {
            return Ok(Some(left(RedeployLeftReason::HoldsDeployment)));
        }
    }
    Ok(None)
}

/// Remove the old artifact `old` of `entry`: by evidence (the old row's
/// recorded identity, or the Library's), else as `answer` says for an edited
/// copy — Delete removes it, Leave disowns it, no answer asks.
fn remove_old(
    ctx: &Ctx<'_>,
    skill: &SkillRecord,
    entry: &RedeployJournalRecord,
    old: &Path,
    answer: Option<EditedCopyDecision>,
) -> Result<Settled> {
    let store = ctx.store;
    let central = Path::new(&skill.central_path);
    let library_hash = content_identity::read(Source::Managed {
        store,
        skill_id: &skill.id,
    });
    let evidence = [
        Deployed::recorded(true, Some(central), entry.old_content_hash.as_deref()),
        Deployed::recorded(true, Some(central), library_hash.as_deref()),
    ];
    let managed = managed_global_dirs(ctx.roots, &entry.tool);
    let removal = |authority| {
        remove_superseded_artifact_unlocked(store, ctx.central_dir, old, &managed, authority)
    };
    let failed = |err: anyhow::Error| Settled::Left {
        reason: RedeployLeftReason::RemovalFailed,
        error: Some(CommandError::from_anyhow(err)),
        at: Some(old.to_path_buf()),
    };
    if entry.state == STATE_OVERWRITTEN {
        // The operator answered Overwrite for exactly this folder.
        return Ok(match removal(SupersededAuthority::Answered) {
            Ok(_) => Settled::Finished,
            Err(err) => failed(err),
        });
    }
    match removal(SupersededAuthority::Evidence(&evidence)) {
        Ok(Superseded::Removed | Superseded::Absent) => return Ok(Settled::Finished),
        Ok(Superseded::NotOurs) => {}
        Err(err) => return Ok(failed(err)),
    }
    if !real_dir(old) {
        return Ok(left_at(RedeployLeftReason::NotOurs, old));
    }
    match answer {
        Some(EditedCopyDecision::Delete) => match removal(SupersededAuthority::Answered) {
            Ok(_) => Ok(Settled::Finished),
            Err(err) => Ok(failed(err)),
        },
        Some(EditedCopyDecision::Leave) => Ok(left_at(RedeployLeftReason::Edited, old)),
        None => Ok(Settled::Awaiting),
    }
}

/// Finish a staged replacement an interruption cut short
/// ([`staged_replace`]); its row was never switched, so the next run plans
/// the move again and adopts what is in place. `old_path` is the new place
/// (where the occupant was), `new_path` the staged sibling:
///
/// - the occupant still in place (cut before the swap): the replacement
///   never happened and the old Deployment was never touched; the staged
///   artifact — the app's own, checked by identity — is removed, and the
///   entry goes once nothing staged is left;
/// - the new place verifiably serving the skill ([`serves`] — the swap
///   happened): the occupant, now at the staged name, is removed under the
///   replacement's authority (ours by evidence, or the operator's
///   Overwrite), else reported left behind;
/// - anything else (the new place absent, unreachable or not the skill's, a
///   leftover the entry cannot recognise, a removal that failed): nothing is
///   touched and the entry stays ([`Settled::Unfinished`]).
fn settle_replacing(
    ctx: &Ctx<'_>,
    entry: &RedeployJournalRecord,
    skill: &SkillRecord,
) -> Result<Settled> {
    let dest = Path::new(&entry.old_path);
    let stage = Path::new(&entry.new_path);
    let (Ok(occupant), Ok(staged)) = (
        serde_json::from_str::<JournaledEntry>(&entry.old_entry),
        serde_json::from_str::<JournaledEntry>(&entry.new_entry),
    ) else {
        log::warn!(
            "[redeploy] unreadable journal entry {}; kept, nothing was touched",
            entry.id
        );
        return Ok(Settled::Unfinished);
    };
    let (Ok(in_place), Ok(at_stage)) = (JournaledEntry::of(dest), JournaledEntry::of(stage)) else {
        return Ok(Settled::Unfinished);
    };
    let managed = managed_global_dirs(ctx.roots, &entry.tool);
    let discard_staged = || {
        remove_superseded_artifact_unlocked(
            ctx.store,
            ctx.central_dir,
            stage,
            &managed,
            SupersededAuthority::Answered,
        )
    };
    if in_place == Some(occupant) {
        return Ok(match at_stage {
            None => Settled::Finished,
            Some(now) if now == staged => match discard_staged() {
                Ok(_) => Settled::Finished,
                Err(err) => {
                    log::warn!("[redeploy] could not discard {:?}: {err:#}", stage);
                    Settled::Unfinished
                }
            },
            Some(_) => Settled::Unfinished,
        });
    }
    if !serves(ctx, skill, dest, None) {
        return Ok(Settled::Unfinished);
    }
    match at_stage {
        None => Ok(Settled::Finished),
        // Never swapped in, and the new place serves without it.
        Some(now) if now == staged => Ok(match discard_staged() {
            Ok(_) => Settled::Finished,
            Err(err) => Settled::Left {
                reason: RedeployLeftReason::RemovalFailed,
                error: Some(CommandError::from_anyhow(err)),
                at: Some(stage.to_path_buf()),
            },
        }),
        Some(now) if now != occupant => Ok(left_at(RedeployLeftReason::Replaced, stage)),
        // Swapped: the occupant lies at the staged name, owed its removal
        // under the replacement's authority.
        Some(_) => {
            if let Some(kept) = protected(ctx, stage, occupant, dest)? {
                return Ok(match kept {
                    Settled::Left { reason, error, .. } => Settled::Left {
                        reason,
                        error,
                        at: Some(stage.to_path_buf()),
                    },
                    other => other,
                });
            }
            let set_aside = RedeployJournalRecord {
                state: if entry.state == STATE_REPLACING_ANSWERED {
                    STATE_OVERWRITTEN
                } else {
                    STATE_PENDING
                }
                .to_string(),
                ..entry.clone()
            };
            Ok(match remove_old(ctx, skill, &set_aside, stage, None)? {
                // A set-aside occupant is never asked about: it was ours by
                // evidence when it was replaced.
                Settled::Awaiting => left_at(RedeployLeftReason::NotOurs, stage),
                settled => settled,
            })
        }
    }
}

/// Record how `entry` settled — drop it, or keep it awaiting the answer or
/// unfinished — and add what the report owes the operator.
fn record_settled(
    ctx: &Ctx<'_>,
    entry: &RedeployJournalRecord,
    settled: Settled,
    report: &mut RedeployReport,
) -> Result<JournalStatus> {
    let store = ctx.store;
    let readers = |path: &Path| -> Vec<String> {
        path.parent()
            .map(|dir| readers_of_global_dir(ctx.roots, ctx.installed, dir))
            .unwrap_or_default()
            .into_iter()
            .map(str::to_string)
            .collect()
    };
    let old = Path::new(&entry.old_path);
    let skill_name = skill_name_of(store, &entry.skill_id)?;
    Ok(match settled {
        Settled::Finished => {
            store.delete_redeploy_journal_entry(&entry.id)?;
            JournalStatus::Finished
        }
        Settled::Retained => {
            store.delete_redeploy_journal_entry(&entry.id)?;
            JournalStatus::Retained
        }
        Settled::Left { reason, error, at } => {
            store.delete_redeploy_journal_entry(&entry.id)?;
            let path = at.unwrap_or_else(|| old.to_path_buf());
            log::info!("[redeploy] left {:?} on disk ({reason:?})", path);
            report.left_behind.push(LeftBehindCopy {
                skill_id: entry.skill_id.clone(),
                skill_name,
                tool: entry.tool.clone(),
                path: path.to_string_lossy().into_owned(),
                reason,
                readers: readers(&path),
                error,
            });
            JournalStatus::LeftBehind
        }
        Settled::Awaiting => {
            if entry.state != STATE_EDITED {
                store.set_redeploy_journal_state(&entry.id, STATE_EDITED)?;
            }
            report.edited_copies.push(EditedCopy {
                entry_id: entry.id.clone(),
                skill_id: entry.skill_id.clone(),
                skill_name,
                tool: entry.tool.clone(),
                path: entry.old_path.clone(),
                modified_at: modified_at(old),
                readers: readers(old),
            });
            JournalStatus::AwaitingAnswer
        }
        Settled::Unfinished => JournalStatus::Unfinished,
    })
}

/// The answer to the edited-copy question `entry` raised: one naming this
/// journal entry and the path it showed, while the entry still awaits it.
/// An answer naming the entry with another path, or an entry that is not
/// awaiting an answer, is refused (report data), never applied.
fn edited_answer(
    answers: &[RedeployAnswer],
    entry: &RedeployJournalRecord,
    report: &mut RedeployReport,
) -> Option<EditedCopyDecision> {
    let (path, decision) = answers.iter().find_map(|answer| match answer {
        RedeployAnswer::EditedCopy {
            entry_id,
            path,
            decision,
        } if *entry_id == entry.id => Some((path, *decision)),
        _ => None,
    })?;
    if entry.state == STATE_EDITED && *path == entry.old_path {
        return Some(decision);
    }
    report.refused_answers.push(refused(path, &entry.id));
    None
}

/// Refuse every answer that names no question this run asks: an edited-copy
/// answer whose journal entry is gone (settled, or never this one), and an
/// Overwrite/Keep answer for a move no longer planned. Nothing is touched for
/// them.
fn refuse_unasked_answers(
    answers: &[RedeployAnswer],
    journal: &[RedeployJournalRecord],
    planned: &[Planned],
    report: &mut RedeployReport,
) {
    for answer in answers {
        match answer {
            RedeployAnswer::EditedCopy { entry_id, path, .. } => {
                if !journal.iter().any(|entry| entry.id == *entry_id) {
                    report.refused_answers.push(refused(path, entry_id));
                }
            }
            RedeployAnswer::Occupied {
                skill_id,
                folder_key,
                row_ids,
                path,
                ..
            } => {
                let asked = planned
                    .iter()
                    .any(|move_| move_.skill.id == *skill_id && move_.folder_key == folder_key);
                if !asked {
                    report
                        .refused_answers
                        .push(refused(path, &row_ids.join(",")));
                }
            }
        }
    }
}

/// A stale answer's typed refusal: the question `id` is no longer asked.
fn stale_answer(id: &str) -> CommandError {
    CommandError::from(SignalError::NotFound {
        kind: "redeploy_question".to_string(),
        id: id.to_string(),
    })
}

fn refused(path: &str, id: &str) -> RefusedAnswer {
    RefusedAnswer {
        path: path.to_string(),
        error: stale_answer(id),
    }
}

fn skill_name_of(store: &SkillStore, skill_id: &str) -> Result<String> {
    Ok(store
        .get_skill_by_id(skill_id)?
        .map(|skill| skill.name)
        .unwrap_or_else(|| skill_id.to_string()))
}

/// Test seams: interrupt a redeploy where no progress step marks the boundary.
#[cfg(test)]
pub(crate) mod test_seams {
    use std::cell::Cell;

    thread_local! {
        static CRASH_BEFORE_SWITCH: Cell<bool> = const { Cell::new(false) };
        static CRASH_AFTER_SWITCH: Cell<bool> = const { Cell::new(false) };
        static CRASH_BEFORE_SWAP: Cell<bool> = const { Cell::new(false) };
        static CRASH_AFTER_SWAP: Cell<bool> = const { Cell::new(false) };
        static NO_EXCHANGE: Cell<bool> = const { Cell::new(false) };
    }

    /// Staged replacements on this thread find no atomic exchange, as on an
    /// OS or file system without one (Windows always; SMB, exFAT, …).
    pub(crate) fn force_no_exchange() {
        NO_EXCHANGE.with(|cell| cell.set(true));
    }

    pub(super) fn no_exchange() -> bool {
        NO_EXCHANGE.with(Cell::get)
    }

    /// The next staged replacement on this thread panics right after its
    /// swap: the new artifact in place, the occupant at the staged name,
    /// nothing switched.
    pub(crate) fn crash_after_swap() {
        CRASH_AFTER_SWAP.with(|cell| cell.set(true));
    }

    pub(super) fn after_swap() {
        if CRASH_AFTER_SWAP.with(|cell| cell.replace(false)) {
            panic!("simulated crash right after the swap");
        }
    }

    /// The next staged replacement on this thread panics after its staged
    /// artifact is journaled and before the swap.
    pub(crate) fn crash_before_swap() {
        CRASH_BEFORE_SWAP.with(|cell| cell.set(true));
    }

    pub(super) fn before_swap() {
        if CRASH_BEFORE_SWAP.with(|cell| cell.replace(false)) {
            panic!("simulated crash before the swap");
        }
    }

    /// The next move on this thread panics — as a crash would — after its
    /// deploy and before its row switch.
    pub(crate) fn crash_before_switch() {
        CRASH_BEFORE_SWITCH.with(|cell| cell.set(true));
    }

    /// The next move on this thread panics after its row switch (journal
    /// written) and before the old artifact is removed.
    pub(crate) fn crash_after_switch() {
        CRASH_AFTER_SWITCH.with(|cell| cell.set(true));
    }

    pub(super) fn before_switch() {
        if CRASH_BEFORE_SWITCH.with(|cell| cell.replace(false)) {
            panic!("simulated crash before the row switch");
        }
    }

    pub(super) fn after_switch() {
        if CRASH_AFTER_SWITCH.with(|cell| cell.replace(false)) {
            panic!("simulated crash after the row switch");
        }
    }
}

#[cfg(test)]
#[path = "tests/redeploy.rs"]
mod tests;
