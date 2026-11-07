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
//!    overwrite path, claimed inside the folder the destination was derived in.
//!    An occupant that is ours (the row recorded there) or byte-identical to
//!    the skill is **adopted**; anything else is a `TARGET_EXISTS` row for the
//!    one overwrite ask. A Keep answer leaves the move undone (`kept`, still
//!    pending); Overwrite replaces the occupant. A Disabled row and a row that
//!    never deployed (no claim) are re-recorded at the new place with nothing
//!    written.
//! 2. **Switch** in one transaction: the folder's row takes the new key, path
//!    and mode, the rows it absorbs are deleted, and a **journal entry** is
//!    written per old artifact still on disk — its path and physical identity,
//!    the row that records the new place, and what the old row recorded as
//!    deployed.
//! 3. **Remove** each old artifact through `artifact_removal` and the removal
//!    fence, only when it is provably ours — a link resolving by physical
//!    identity to the skill's Library copy, or a copy whose Content identity
//!    equals the one its row recorded or the Library's — **and** no other live
//!    row records it (Amp keeps the folder Kimi Code left). A copy that differs
//!    is an **edited copy**: a row of the same ask, delete or leave; leaving it
//!    disowns it and the report names its readers. A link elsewhere is never
//!    ours and is reported left behind. Each settled entry is dropped.
//!
//! **The journal.** Every run and the launch check finish the journal first:
//! an entry is acted on only while the row still records the journaled new
//! place and both artifacts still have their journaled identities (Library
//! move's resume rule); otherwise it is reported left behind and dropped. An
//! edited copy awaiting an answer stays in the journal (`edited`) and is
//! asked again by the next run. A crash at any step leaves at most a
//! duplicate, never a missing skill: the old artifact goes only after the new
//! one is recorded.
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
use super::errors::{CommandError, SignalError};
use super::global_sync::OverrideDecision;
use super::mutation_guard;
use super::ownership::Deployed;
use super::placement::{
    self, capability_entry, managed_global_dirs, readers_of_global_dir, row_reachable, DeployMode,
    InstalledSet, Placement, PlacementScope, PlacementSettings, ScopeKind,
};
use super::removal_fence::{EntryId, ProtectedLibrary};
use super::settings;
use super::skill_store::{
    RedeployJournalRecord, SkillRecord, SkillStore, SkillTargetRecord, SwitchedRow,
    TargetTransition,
};
use super::sync_engine::{self, Replace, TargetExistsError};
use super::sync_status::{SyncMode, SyncStatus};
use super::tool_adapters::installed_set_in;

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
}

impl RedeployReport {
    fn is_empty(&self) -> bool {
        self.moves.is_empty()
            && self.journal.is_empty()
            && self.left_behind.is_empty()
            && self.edited_copies.is_empty()
            && self.lost_disables.is_empty()
    }
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
    /// The row no longer records the journaled new place, or the new artifact
    /// is no longer the one deployed: the old one may be the only copy.
    Stale,
    /// Its removal failed or was refused.
    RemovalFailed,
}

/// An edited copy: a row of the one overwrite ask (delete / leave).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct EditedCopy {
    pub skill_id: String,
    pub skill_name: String,
    /// The key of the row that owned it — the key an answer names.
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

/// The operator's answer to one row of the ask, naming the path it showed:
/// it applies only while that row's place is still that path. Input only.
#[derive(Debug, Clone, Deserialize, Type)]
#[serde(tag = "question", rename_all = "snake_case")]
pub enum RedeployAnswer {
    /// The folder in a move's way (`TARGET_EXISTS`): Overwrite or Keep.
    Occupied {
        skill_id: String,
        folder_key: String,
        path: String,
        decision: OverrideDecision,
    },
    /// An edited old copy: delete it, or leave it (disowned).
    EditedCopy {
        skill_id: String,
        tool: String,
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

    for entry in journal {
        on_progress(RedeployProgress {
            done,
            total,
            name: skill_name_of(store, &entry.skill_id)?,
        });
        done += 1;
        let answer = (entry.state == STATE_EDITED)
            .then(|| edited_answer(answers, &entry))
            .flatten();
        let settled = settle_entry(&ctx, &entry, answer)?;
        let status = record_settled(&ctx, &entry, settled, &mut report)?;
        report.journal.push(JournalOutcome {
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

const STATE_PENDING: &str = "pending";
const STATE_EDITED: &str = "edited";

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

    // The operator's answer for this move, if it is about this place.
    let answer = answers.iter().find_map(|answer| match answer {
        RedeployAnswer::Occupied {
            skill_id,
            folder_key,
            path,
            decision,
        } if *skill_id == skill.id && folder_key == planned.folder_key => {
            Some((path.as_str(), *decision))
        }
        _ => None,
    });
    if let Some((asked, _)) = answer {
        if Path::new(asked) != planned.dest {
            moved.status = RedeployStatus::Failed {
                error: CommandError::from(SignalError::NotFound {
                    kind: "target".to_string(),
                    id: asked.to_string(),
                }),
            };
            return Ok(moved);
        }
    }
    if let Some((_, OverrideDecision::Keep)) = answer {
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
        )?;
        note_lost_disables(planned, report);
        moved.status = RedeployStatus::Unchanged;
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
    let at_dest = planned
        .rows
        .iter()
        .find(|row| Path::new(&row.target_path) == planned.dest && row.owns_artifact());
    let evidence = at_dest
        .map(|row| Deployed::of_target(row, Some(skill)))
        .unwrap_or_default();
    let occupied_before = std::fs::symlink_metadata(&planned.dest).is_ok();
    let replace = match answer {
        Some((_, OverrideDecision::Overwrite)) => Replace::confirmed(),
        _ => Replace::if_ours(&evidence).or_same_content(true),
    };
    let managed = [planned.folder_path.clone()];
    let library = ProtectedLibrary::new(store, ctx.central_dir);
    let outcome = sync_engine::sync_dir_for_tool_with_overwrite(
        capability,
        Path::new(&skill.central_path),
        Source::Managed {
            store,
            skill_id: &skill.id,
        },
        &planned.dest,
        replace.within(&managed),
        &library,
    );
    let outcome = match outcome {
        Ok(outcome) => outcome,
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
    let new_entry = match EntryId::of(&planned.dest)? {
        Some(id) => id,
        None => anyhow::bail!(
            "redeploy: {:?} is gone right after its deploy",
            planned.dest
        ),
    };
    let content_hash = outcome
        .mode_used
        .can_drift()
        .then(|| {
            content_identity::read(Source::Managed {
                store,
                skill_id: &skill.id,
            })
        })
        .flatten();
    let mut journal = Vec::new();
    for row in &live {
        let old = Path::new(&row.target_path);
        if old == planned.dest {
            continue;
        }
        match EntryId::of(old)? {
            Some(id) if id != new_entry => journal.push(RedeployJournalRecord {
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
    store.switch_redeployed_rows(
        &survivor.id,
        SwitchedRow {
            tool: planned.folder_key,
            target_path: &dest,
            mode: outcome.mode_used,
            status: SyncStatus::Synced,
            last_error: None,
            synced_at: Some(ctx.now),
            content_hash: content_hash.as_deref(),
        },
        &absorbed,
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
            Settled::Finished | Settled::Retained => None,
        };
        if let Some(edited) = stays {
            left_old = Some(left_old.unwrap_or(false) || edited);
        }
        record_settled(ctx, entry, settled, report)?;
    }
    let overwrote = matches!(answer, Some((_, OverrideDecision::Overwrite)));
    moved.status = match left_old {
        Some(edited) => RedeployStatus::LeftOld { edited },
        None if occupied_before && !overwrote => RedeployStatus::Adopted,
        None => RedeployStatus::Moved,
    };
    Ok(moved)
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
    /// Left on disk, no longer journaled.
    Left {
        reason: RedeployLeftReason,
        error: Option<CommandError>,
    },
    /// An edited copy awaiting the answer; stays journaled.
    Awaiting,
}

/// Settle `entry` now (the module's journal rule): act only while its row
/// still records the journaled new place and both artifacts keep their
/// journaled identities; remove the old artifact only when it is provably
/// ours and no other live row records it, or the operator answered Delete.
fn settle_entry(
    ctx: &Ctx<'_>,
    entry: &RedeployJournalRecord,
    answer: Option<EditedCopyDecision>,
) -> Result<Settled> {
    let store = ctx.store;
    let left = |reason| Settled::Left {
        reason,
        error: None,
    };
    let (Ok(old_id), Ok(new_id)) = (
        serde_json::from_str::<EntryId>(&entry.old_entry),
        serde_json::from_str::<EntryId>(&entry.new_entry),
    ) else {
        log::warn!(
            "[redeploy] unreadable journal entry {}; nothing was removed",
            entry.id
        );
        return Ok(left(RedeployLeftReason::Stale));
    };
    let old = Path::new(&entry.old_path);
    let new = Path::new(&entry.new_path);
    // The row still records the new place, and the new artifact is still the
    // one deployed: otherwise the old one may be the only copy.
    match store.get_skill_target_by_id(&entry.row_id)? {
        Some(row) if Path::new(&row.target_path) == new && row.owns_artifact() => {}
        _ => return Ok(left(RedeployLeftReason::Stale)),
    }
    if EntryId::of(new)? != Some(new_id) {
        return Ok(left(RedeployLeftReason::Stale));
    }
    match EntryId::of(old)? {
        None => return Ok(Settled::Finished),
        Some(id) if id == new_id => return Ok(Settled::Finished),
        Some(id) if id != old_id => return Ok(left(RedeployLeftReason::Replaced)),
        Some(_) => {}
    }
    // Another live row (any skill, either scope) records it: its Deployment.
    // A path that cannot be read is counted as recording it (kept, never
    // removed blind).
    for path in recorded_target_paths(store, ctx.roots)? {
        match EntryId::of(&path) {
            Ok(Some(id)) if id == old_id => return Ok(Settled::Retained),
            Ok(_) => {}
            Err(_) => return Ok(Settled::Retained),
        }
    }
    let Some(skill) = store.get_skill_by_id(&entry.skill_id)? else {
        return Ok(left(RedeployLeftReason::Stale));
    };
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
    };
    match removal(SupersededAuthority::Evidence(&evidence)) {
        Ok(Superseded::Removed | Superseded::Absent) => return Ok(Settled::Finished),
        Ok(Superseded::NotOurs) => {}
        Err(err) => return Ok(failed(err)),
    }
    let is_copy = std::fs::symlink_metadata(old).is_ok_and(|meta| meta.is_dir());
    if !is_copy {
        return Ok(left(RedeployLeftReason::NotOurs));
    }
    match answer {
        Some(EditedCopyDecision::Delete) => match removal(SupersededAuthority::Answered) {
            Ok(_) => Ok(Settled::Finished),
            Err(err) => Ok(failed(err)),
        },
        Some(EditedCopyDecision::Leave) => Ok(left(RedeployLeftReason::Edited)),
        None => Ok(Settled::Awaiting),
    }
}

/// Record how `entry` settled — drop it, or keep it awaiting the answer —
/// and add what the report owes the operator.
fn record_settled(
    ctx: &Ctx<'_>,
    entry: &RedeployJournalRecord,
    settled: Settled,
    report: &mut RedeployReport,
) -> Result<JournalStatus> {
    let store = ctx.store;
    let old = Path::new(&entry.old_path);
    let readers = || -> Vec<String> {
        old.parent()
            .map(|dir| readers_of_global_dir(ctx.roots, ctx.installed, dir))
            .unwrap_or_default()
            .into_iter()
            .map(str::to_string)
            .collect()
    };
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
        Settled::Left { reason, error } => {
            store.delete_redeploy_journal_entry(&entry.id)?;
            log::info!("[redeploy] left {} on disk ({reason:?})", entry.old_path);
            report.left_behind.push(LeftBehindCopy {
                skill_id: entry.skill_id.clone(),
                skill_name,
                tool: entry.tool.clone(),
                path: entry.old_path.clone(),
                reason,
                readers: readers(),
                error,
            });
            JournalStatus::LeftBehind
        }
        Settled::Awaiting => {
            if entry.state != STATE_EDITED {
                store.set_redeploy_journal_state(&entry.id, STATE_EDITED)?;
            }
            report.edited_copies.push(EditedCopy {
                skill_id: entry.skill_id.clone(),
                skill_name,
                tool: entry.tool.clone(),
                path: entry.old_path.clone(),
                modified_at: modified_at(old),
                readers: readers(),
            });
            JournalStatus::AwaitingAnswer
        }
    })
}

/// The answer to the edited-copy question `entry` raised, naming its row's
/// key and the path it showed.
fn edited_answer(
    answers: &[RedeployAnswer],
    entry: &RedeployJournalRecord,
) -> Option<EditedCopyDecision> {
    answers.iter().find_map(|answer| match answer {
        RedeployAnswer::EditedCopy {
            skill_id,
            tool,
            path,
            decision,
        } if *skill_id == entry.skill_id && *tool == entry.tool && *path == entry.old_path => {
            Some(*decision)
        }
        _ => None,
    })
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
