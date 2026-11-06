//! Move Library folder: relocate the central repo — every recorded skill's
//! central copy — to a new folder, and keep every harness working.
//!
//! One operator action, two entry points:
//!
//! * [`preview_library_move`] — read-only: admits the destination (every
//!   refusal but "not writable", which needs a write to prove) and answers
//!   what the confirmation states: how many skills move, how many are
//!   Unlocatable, and whether the destination is cloud-synced (a warning,
//!   never a refusal).
//! * [`move_library`] — the mutation, one `mutation_guard::serialized`
//!   section in four phases after admission:
//!   1. **Copy** every recorded, locatable skill into the destination with
//!      a relocation-fidelity copy and verify it over every compared facet
//!      (see § Relocation fidelity). A skill that cannot be copied
//!      faithfully, or whose copy another record shares, stays where it is
//!      (`kept`). A failure removes what was staged and changes no record
//!      (`LIBRARY_COPY_FAILED`). Scratch in the old root (`.explore-cache`,
//!      `.skills-hub-staging-*`) and anything unrecorded never travel.
//!   2. **Switch** every moved skill's `central_path` and the Library folder
//!      setting in one DB transaction (`settings::switch_library`).
//!   3. **Re-point** every owned (`synced_at` set) Deployment — global target
//!      rows and project assignments — whose link resolves to the skill's
//!      old copy, through `sync_engine`'s overwrite path. A copy is left
//!      `unchanged`, a link elsewhere (or nothing at the path, or a folder
//!      that is the old copy through an alias) is `not_ours` and untouched,
//!      a project whose folder is gone is `folder_missing`, a disabled row
//!      is `disabled`; a failure is report data and settles its rows `error`.
//!   4. **Clean** deletes only what is provably duplicated, at the moment of
//!      deletion, and only inside the old Library folder. An old copy is
//!      removed only when it is a direct child of the old Library folder (a
//!      copy recorded elsewhere — a split Library — is copied and switched
//!      but left in place), nothing retains it (a recorded copy, a
//!      Deployment of any skill in either direction, a link inside any
//!      recorded skill), the skill's record still names the new copy, and
//!      both copies are still the entries the copy phase saw (physical
//!      identity). It is then renamed aside inside the old folder, verified
//!      again against the new copy over every compared facet, and removed
//!      through `artifact_removal` and the removal fence — or renamed back
//!      and kept when anything differs. The old root and everything
//!      unrecorded stay and are listed in the report's `left_behind`.
//!
//! An Unlocatable skill (its central copy gone) is skipped and reported; its
//! record keeps its old path.
//!
//! Crash safety: a crash before the switch leaves orphan copies in the
//! destination and the Library untouched. The switch's own transaction also
//! records a **journal** — per moved skill its old and new copy, by path and
//! by physical identity, and both Library folders — so a crash after it is
//! finished, never forgotten: [`resume_library_move`] (run at startup, and
//! first thing by every [`move_library`]) re-runs Re-point and Clean from
//! the journal and drops it once both settle. The journal is a record of
//! what was copied, never an authority to remove what its paths name now:
//! the resume acts only while the configured Library is the journal's `to`,
//! and per skill only while its record names the journaled new copy and
//! both copies still have their journaled identities; anything else is
//! reported left behind and nothing of it is touched. A journal that cannot
//! be read is dropped and logged. Until the resume runs, a link left
//! pointing at an old copy still resolves: duplicates, never a missing skill.
//!
//! Core never resolves roots: the caller passes `InstallerPaths` (home, the
//! configured central repo, the app cache dir).

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;
use specta::Type;

use super::artifact_removal::{
    remove_former_central_copy_unlocked, remove_staged_library_copy_unlocked, RowRef,
};
use super::content_identity::{self, Source};
use super::errors::{CommandError, LibraryDestinationRefusal, SignalError};
use super::git_cache;
use super::installer::InstallerPaths;
use super::mutation_guard;
use super::project_ops;
use super::project_sync::resolve_assignment_artifact;
use super::removal_fence::{EntryId, Place, ProtectedLibrary};
use super::settings;
use super::skill_store::{
    AssignmentTransition, SkillRecord, SkillStore, SkillTargetRecord, TargetTransition,
};
use super::sync_engine::{
    sync_dir_for_tool_with_overwrite, sync_dir_for_tool_within_root, SyncOutcome,
};
use super::sync_status::{SyncMode, SyncStatus};
use super::tool_adapters::{
    adapter_by_key, default_tool_adapters, former_skills_dirs_in, project_skills_dir_in,
    skills_dir_in, ToolAdapter,
};

// ---------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------

/// What the move confirmation states, answered before anything is written.
#[derive(Debug, Clone, Serialize, Type)]
pub struct LibraryMovePreview {
    /// The admitted destination, as it will be recorded.
    pub destination: String,
    /// Recorded skills whose central copy is there: the ones that move.
    pub skills: u32,
    /// Unlocatable skills (central copy gone): skipped, never block the move.
    pub missing: u32,
    /// Locatable skills the move keeps where they are (a copy that cannot be
    /// relocated faithfully, or one another record shares) — not counted in
    /// `skills`.
    pub kept: u32,
    /// Set when the destination is under a known cloud-sync root.
    pub cloud_sync: Option<CloudSync>,
}

/// A cloud-sync root the destination lies under. Online-only eviction there
/// can break harness links, so the confirmation warns (never refuses).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct CloudSync {
    pub provider: CloudSyncProvider,
    /// The cloud-sync root the destination is inside.
    pub root: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum CloudSyncProvider {
    /// `~/Library/Mobile Documents` (iCloud Drive).
    IcloudDrive,
    /// A File Provider root under `~/Library/CloudStorage` (Dropbox, Google
    /// Drive, OneDrive on macOS, …); `root` names which.
    CloudStorage,
    /// A OneDrive root in the home folder (`~/OneDrive`, `~/OneDrive - Org`).
    OneDrive,
}

/// A streamed step of the move.
#[derive(Debug, Clone, Serialize, Type)]
pub struct LibraryMoveProgress {
    pub phase: LibraryMovePhase,
    /// Steps done in this phase before this one.
    pub done: u32,
    pub total: u32,
    /// The skill this step is about.
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum LibraryMovePhase {
    Copying,
    Relinking,
    Cleaning,
}

/// What the move did, per skill, per Deployment, and what it left.
#[derive(Debug, Serialize, Type)]
pub struct LibraryMoveReport {
    /// The Library folder before the move.
    pub from: String,
    /// The Library folder now (the setting's new value).
    pub to: String,
    pub skills: Vec<SkillMoveOutcome>,
    /// Every owned Deployment of a moved skill, one per path (a shared
    /// skills dir is one artifact with several rows).
    pub deployments: Vec<RelinkOutcome>,
    /// What stays in the old place: the old root, anything unrecorded in it,
    /// and an old copy kept because it is still reached or could not be
    /// removed.
    pub left_behind: Vec<LeftBehind>,
}

#[derive(Debug, Serialize, Type)]
pub struct SkillMoveOutcome {
    pub skill_id: String,
    pub name: String,
    pub status: SkillMoveStatus,
}

#[derive(Debug, Serialize, Type)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SkillMoveStatus {
    Moved {
        from: String,
        to: String,
    },
    Skipped {
        reason: SkillMoveSkip,
    },
    /// Not moved: its copy stays where it is recorded, untouched, and so do
    /// its Deployments.
    Kept {
        reason: SkillKept,
    },
}

/// Why a locatable skill was kept where it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SkillKept {
    /// Something in its copy cannot be relocated faithfully: `entry` (the
    /// first one found, as it sits in the old copy) and why.
    Unrelocatable {
        entry: String,
        cause: UnrelocatableCause,
    },
    /// Its recorded copy is physically the same folder as another skill's,
    /// or holds or lies inside it (`with_skill_id`): the records alias one
    /// set of bytes, so neither is moved.
    SharedCopy { with_skill_id: String },
}

/// Why an entry cannot be relocated faithfully.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum UnrelocatableCause {
    /// An absolute link into the old Library (its own copy included): after
    /// the move it would still point at the old place.
    LinkIntoOldLibrary,
    /// A relative link that reaches outside the skill's own folder: from the
    /// new place it would point somewhere else.
    LinkEscapes,
    /// Not a file, folder or link (a FIFO, socket or device): it cannot be
    /// copied.
    SpecialFile,
    /// A file with another name (a hard link) outside the skill's folder:
    /// the copy would split one file in two.
    HardlinkOutside,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SkillMoveSkip {
    /// Unlocatable: the central copy is gone; the record keeps its path.
    Missing,
}

#[derive(Debug, Serialize, Type)]
pub struct RelinkOutcome {
    pub skill_id: String,
    pub name: String,
    pub path: String,
    pub rows: Vec<RowRef>,
    pub status: RelinkStatus,
}

#[derive(Debug, Serialize, Type)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RelinkStatus {
    /// Re-made into the new copy (`mode` as the engine wrote it).
    Relinked { mode: SyncMode },
    /// Left as it was.
    Unchanged { reason: RelinkUnchanged },
    /// Not touched.
    Skipped { reason: RelinkSkip },
    /// The re-point failed; its rows are kept `error` (report data).
    Failed { error: CommandError },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum RelinkUnchanged {
    /// A copy-mode Deployment: its bytes do not follow the Library.
    Copy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum RelinkSkip {
    /// Nothing at the path reaches the old copy through a link of ours
    /// (the operator changed it, removed it, or aliased the harness dir).
    NotOurs,
    /// The project's folder is gone; its next resync re-links it.
    FolderMissing,
    Disabled,
}

#[derive(Debug, Serialize, Type)]
pub struct LeftBehind {
    pub path: String,
    pub reason: LeftBehindReason,
    /// The skill whose old copy this is (every reason but `old_library` and
    /// `unrecorded`).
    pub skill_id: Option<String>,
    /// Why its removal failed (`removal_failed`).
    pub error: Option<CommandError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum LeftBehindReason {
    /// The old Library folder itself — never removed.
    OldLibrary,
    /// Something in the old Library folder no record names — never removed.
    Unrecorded,
    /// A moved skill's old copy that a Deployment (of any skill) still
    /// reaches.
    StillLinked,
    /// A moved skill's old copy that a skill which stays still records (an
    /// Unlocatable skill recorded inside it).
    StillRecorded,
    /// A moved skill's old copy whose removal failed or was refused.
    RemovalFailed,
    /// A moved skill's old copy that is not a folder directly inside the old
    /// Library folder (a split Library): copied and switched, never removed.
    OutsideLibraryFolder,
    /// A moved skill's old copy that no longer matches its new copy (edited
    /// after the copy): kept, never removed.
    ChangedSinceCopy,
    /// A moved skill's old copy whose new copy is gone, or is not the folder
    /// the move made: kept — it may be the only copy.
    DestinationMissing,
    /// What is at a moved skill's old path is not the folder the move copied
    /// (replaced since): never removed.
    Replaced,
    /// An interrupted move's journal no longer describes the Library (the
    /// Library folder or the skill's record changed since): nothing of it
    /// is touched.
    StaleJournal,
    /// A moved skill's old copy that an interrupted clean had renamed aside
    /// (into a `.skills-hub-clean-*` folder in the old Library folder) and
    /// that could not be put back — its old path is taken: kept where it is,
    /// at `path`, never removed.
    SetAside,
}

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

/// The confirmation's facts for moving the Library to `dest`. Read-only:
/// refuses exactly as [`move_library`] would, except "not writable", which
/// only the move's own probe can prove. Classifies the skills the move would
/// keep in place the way the move does ([`classify`]), so "Move N skills"
/// counts exactly the ones that move.
pub fn preview_library_move(
    store: &SkillStore,
    paths: &InstallerPaths,
    dest: &Path,
) -> Result<LibraryMovePreview> {
    let skills = store.list_skills()?;
    admit(store, paths, &skills, dest, Probe::ReadOnly)?;
    let (locatable, missing): (Vec<SkillRecord>, Vec<SkillRecord>) =
        skills.into_iter().partition(is_locatable);
    let kept = classify(&paths.central_dir, &locatable)?
        .iter()
        // A copy that cannot be read counts as moving: the move reports it.
        .filter(|skill| matches!(skill.kept, Ok(Some(_))))
        .count();
    Ok(LibraryMovePreview {
        destination: dest.to_string_lossy().into_owned(),
        skills: (locatable.len() - kept) as u32,
        missing: missing.len() as u32,
        kept: kept as u32,
        cloud_sync: cloud_sync_of(&paths.roots.home, dest),
    })
}

/// Move the Library (`paths.central_dir`) to `dest`. See the module docs for
/// the phases. A refusal or a copy failure is an error and changes nothing;
/// from the switch on, per-Deployment and per-copy outcomes are report data.
/// A move interrupted after its switch is finished first (see
/// [`resume_library_move`]); `paths.central_dir` is then the Library it
/// switched to.
///
/// Mutation entry point: serialised against every other Sync-target
/// mutation.
pub fn move_library(
    store: &SkillStore,
    paths: &InstallerPaths,
    dest: &Path,
    now: i64,
    mut on_progress: impl FnMut(LibraryMoveProgress),
) -> Result<LibraryMoveReport> {
    mutation_guard::serialized(|| {
        if let Some(resumed) = resume_unlocked(store, &paths.central_dir, now, &mut on_progress)? {
            log_resumed(&resumed);
        }
        let skills = store.list_skills()?;
        admit(store, paths, &skills, dest, Probe::Write)?;

        let (locatable, missing): (Vec<SkillRecord>, Vec<SkillRecord>) =
            skills.into_iter().partition(is_locatable);
        let Copied { moved, kept } = copy_phase(store, paths, dest, locatable, &mut on_progress)?;

        let switch: Vec<(String, String)> = moved
            .iter()
            .map(|m| (m.skill.id.clone(), m.new.to_string_lossy().into_owned()))
            .collect();
        let journal = Journal {
            from: paths.central_dir.to_string_lossy().into_owned(),
            to: dest.to_string_lossy().into_owned(),
            moves: moved
                .iter()
                .map(|m| JournaledMove {
                    skill_id: m.skill.id.clone(),
                    old: m.old.to_string_lossy().into_owned(),
                    new: m.new.to_string_lossy().into_owned(),
                    old_id: m.old_id,
                    new_id: m.new_id,
                })
                .collect(),
        };
        let switched = serde_json::to_string(&journal)
            .map_err(anyhow::Error::from)
            .and_then(|journal| settings::switch_library(store, &switch, dest, &journal, now));
        if let Err(err) = switched {
            rollback(store, paths, dest, moved.iter().map(|m| m.new.as_path()));
            return Err(err);
        }

        let mut report = settle(
            store,
            &paths.central_dir,
            dest,
            &moved,
            Vec::new(),
            now,
            &mut on_progress,
        )?;
        report
            .skills
            .extend(kept.into_iter().map(|(skill, reason)| SkillMoveOutcome {
                skill_id: skill.id,
                name: skill.name,
                status: SkillMoveStatus::Kept { reason },
            }));
        report
            .skills
            .extend(missing.into_iter().map(|skill| SkillMoveOutcome {
                skill_id: skill.id,
                name: skill.name,
                status: SkillMoveStatus::Skipped {
                    reason: SkillMoveSkip::Missing,
                },
            }));
        Ok(report)
    })
}

/// Finish a Library move a crash interrupted after its switch, under the
/// Library configured now (`central_dir`): Re-point and Clean every skill
/// its journal still describes (idempotent — a link already re-pointed is
/// re-made into the same copy, an old copy already removed is absent, one a
/// crash left renamed aside is put back first — or, when its old path is
/// taken, reported where it is with its skill, `set_aside`), then drop the
/// journal.
/// `None` when no move is pending, or its journal could not be read (it is
/// dropped and logged; nothing is removed). Run once at startup, before
/// anything else can mutate; [`move_library`] also runs it first.
///
/// The journal authorises nothing by its paths alone: unless `central_dir`
/// is its `to`, nothing of it is touched; per skill, unless the record names
/// its `new` and both copies still have their journaled identities, that
/// skill is neither re-pointed nor cleaned. Each is reported left behind.
///
/// Mutation entry point: serialised against every other Sync-target
/// mutation.
pub fn resume_library_move(
    store: &SkillStore,
    central_dir: &Path,
    now: i64,
) -> Result<Option<LibraryMoveReport>> {
    mutation_guard::serialized(|| resume_unlocked(store, central_dir, now, &mut |_| {}))
}

/// What a switched move still owes: the Library folders on either side and,
/// per moved skill, its old and new copy by path and by physical identity.
/// Written in the switch's own transaction (`settings::switch_library`),
/// dropped once Re-point and Clean settle.
#[derive(Debug, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    from: String,
    to: String,
    moves: Vec<JournaledMove>,
}

#[derive(Debug, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct JournaledMove {
    skill_id: String,
    old: String,
    new: String,
    /// The old copy's identity when the copy phase read it.
    old_id: EntryId,
    /// The new copy's identity when the copy phase created it.
    new_id: EntryId,
}

fn resume_unlocked(
    store: &SkillStore,
    central_dir: &Path,
    now: i64,
    on_progress: &mut impl FnMut(LibraryMoveProgress),
) -> Result<Option<LibraryMoveReport>> {
    let Some(raw) = settings::pending_library_move(store)? else {
        return Ok(None);
    };
    let journal: Journal = match serde_json::from_str(&raw) {
        Ok(journal) => journal,
        Err(err) => {
            log::warn!(
                "[library move] dropped the unreadable journal of an interrupted move; nothing was removed: {err}"
            );
            settings::finish_library_move(store)?;
            return Ok(None);
        }
    };
    let (from, to) = (PathBuf::from(&journal.from), PathBuf::from(&journal.to));
    let left = |entry: &JournaledMove, reason| {
        let old = Path::new(&entry.old);
        present(old).then(|| LeftBehind {
            path: entry.old.clone(),
            reason,
            skill_id: Some(entry.skill_id.clone()),
            error: None,
        })
    };
    // Before anything is decided, every old copy an interrupted clean had
    // renamed aside is found by its identity: put back at its old path when
    // that is free; otherwise it stays where it is and is reported there,
    // with its skill, beside what took its path — and that skill is settled
    // no further. An app-made recovery copy is never left anonymous.
    let set_aside = |entry: &JournaledMove, left_behind: &mut Vec<LeftBehind>| -> Result<bool> {
        let Some(aside) = locate_old_copy(&from, entry)? else {
            return Ok(false);
        };
        left_behind.push(LeftBehind {
            path: aside.to_string_lossy().into_owned(),
            reason: LeftBehindReason::SetAside,
            skill_id: Some(entry.skill_id.clone()),
            error: None,
        });
        left_behind.extend(left(entry, LeftBehindReason::Replaced));
        Ok(true)
    };

    // A journal of another Library (the setting changed since) describes
    // nothing that is true now.
    if to != central_dir {
        log::warn!(
            "[library move] dropped the journal of a move to {:?}: the Library is {:?} now; nothing was touched",
            to,
            central_dir
        );
        let mut left_behind = Vec::new();
        for entry in &journal.moves {
            if !set_aside(entry, &mut left_behind)? {
                left_behind.extend(left(entry, LeftBehindReason::StaleJournal));
            }
        }
        settings::finish_library_move(store)?;
        return Ok(Some(LibraryMoveReport {
            from: journal.from,
            to: journal.to,
            skills: Vec::new(),
            deployments: Vec::new(),
            left_behind,
        }));
    }

    let mut moved = Vec::with_capacity(journal.moves.len());
    let mut left_behind = Vec::new();
    for entry in &journal.moves {
        let (old, new) = (PathBuf::from(&entry.old), PathBuf::from(&entry.new));
        if set_aside(entry, &mut left_behind)? {
            continue;
        }
        // The record is still the one the switch wrote.
        let mut skill = match store.get_skill_by_id(&entry.skill_id)? {
            Some(skill) if Path::new(&skill.central_path) == new => skill,
            _ => {
                left_behind.extend(left(entry, LeftBehindReason::StaleJournal));
                continue;
            }
        };
        // The new copy is still the folder the move made.
        if EntryId::of(&new)? != Some(entry.new_id) {
            left_behind.extend(left(entry, LeftBehindReason::DestinationMissing));
            continue;
        }
        // The old copy, when still there, is still the folder the move
        // copied (absent: already cleaned).
        match EntryId::of(&old)? {
            Some(id) if id != entry.old_id => {
                left_behind.extend(left(entry, LeftBehindReason::Replaced));
                continue;
            }
            _ => {}
        }
        skill.central_path = entry.old.clone();
        moved.push(Moved {
            old_place: Place::of(&old)?,
            skill,
            old,
            new,
            old_id: entry.old_id,
            new_id: entry.new_id,
        });
    }
    settle(store, &from, &to, &moved, left_behind, now, on_progress).map(Some)
}

/// After the switch: Re-point, Clean, then drop the journal. Answers the
/// report for `moved` (the caller adds the skills that did not move), with
/// `left` — what the caller already decided to leave — among what stays.
#[allow(clippy::too_many_arguments)]
fn settle(
    store: &SkillStore,
    old_root: &Path,
    new_root: &Path,
    moved: &[Moved],
    left: Vec<LeftBehind>,
    now: i64,
    on_progress: &mut impl FnMut(LibraryMoveProgress),
) -> Result<LibraryMoveReport> {
    let deployments = relink_phase(store, new_root, moved, now, on_progress)?;
    let left_behind = clean_phase(store, old_root, new_root, moved, left, on_progress)?;
    #[cfg(test)]
    test_seams::before_journal_clear();
    settings::finish_library_move(store)?;
    Ok(LibraryMoveReport {
        from: old_root.to_string_lossy().into_owned(),
        to: new_root.to_string_lossy().into_owned(),
        skills: moved
            .iter()
            .map(|m| SkillMoveOutcome {
                skill_id: m.skill.id.clone(),
                name: m.skill.name.clone(),
                status: SkillMoveStatus::Moved {
                    from: m.old.to_string_lossy().into_owned(),
                    to: m.new.to_string_lossy().into_owned(),
                },
            })
            .collect(),
        deployments,
        left_behind,
    })
}

fn log_resumed(report: &LibraryMoveReport) {
    let relinked = report
        .deployments
        .iter()
        .filter(|d| matches!(d.status, RelinkStatus::Relinked { .. }))
        .count();
    let failed = report
        .deployments
        .iter()
        .filter(|d| matches!(d.status, RelinkStatus::Failed { .. }))
        .count();
    log::info!(
        "[library move] finished an interrupted move from {} to {}: {} skill(s), {} link(s) re-pointed, {} failed, {} item(s) left behind",
        report.from,
        report.to,
        report.skills.len(),
        relinked,
        failed,
        report.left_behind.len()
    );
}

/// Test seams: interrupt a move where no progress step marks the boundary.
#[cfg(test)]
pub(crate) mod test_seams {
    use std::cell::Cell;

    thread_local! {
        static CRASH_BEFORE_JOURNAL_CLEAR: Cell<bool> = const { Cell::new(false) };
        static CRASH_AFTER_SET_ASIDE: Cell<bool> = const { Cell::new(false) };
    }

    /// The next clean on this thread panics — as a crash would — right after
    /// it renamed an old copy aside, before verifying and removing it.
    pub(crate) fn crash_after_set_aside() {
        CRASH_AFTER_SET_ASIDE.with(|cell| cell.set(true));
    }

    pub(super) fn after_set_aside() {
        if CRASH_AFTER_SET_ASIDE.with(|cell| cell.replace(false)) {
            panic!("simulated crash after an old copy was renamed aside");
        }
    }

    /// The next settle on this thread panics — as a crash would — after
    /// Clean and before its journal is dropped.
    pub(crate) fn crash_before_journal_clear() {
        CRASH_BEFORE_JOURNAL_CLEAR.with(|cell| cell.set(true));
    }

    pub(super) fn before_journal_clear() {
        if CRASH_BEFORE_JOURNAL_CLEAR.with(|cell| cell.replace(false)) {
            panic!("simulated crash before the journal is dropped");
        }
    }
}

// ---------------------------------------------------------------------------
// Admission
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Probe {
    /// Decide on reads alone (the preview).
    ReadOnly,
    /// Also create the destination and prove a write (the move).
    Write,
}

/// Admit `dest` as the new Library, or refuse with the typed
/// `LibraryDestinationRefused`. Under [`Probe::Write`] the destination is
/// created (when absent) and written to once, as the last check.
fn admit(
    store: &SkillStore,
    paths: &InstallerPaths,
    skills: &[SkillRecord],
    dest: &Path,
    probe: Probe,
) -> Result<()> {
    let refuse = |reason, conflict: Option<&Path>| {
        anyhow::anyhow!(SignalError::LibraryDestinationRefused {
            path: dest.to_string_lossy().into_owned(),
            reason,
            conflict: conflict.map(|p| p.to_string_lossy().into_owned()),
        })
    };
    let plain = dest
        .components()
        .all(|c| !matches!(c, Component::CurDir | Component::ParentDir));
    if !dest.is_absolute() || !plain {
        return Err(refuse(LibraryDestinationRefusal::NotAbsolute, None));
    }
    let physical_dest = physical(dest);

    // The current Library: the configured folder and every recorded copy.
    let library = std::iter::once(paths.central_dir.clone()).chain(
        skills
            .iter()
            .map(|skill| PathBuf::from(&skill.central_path))
            .filter(|path| path.is_absolute() && path.parent().is_some()),
    );
    for root in library {
        let physical_root = physical(&root);
        if physical_dest.starts_with(&physical_root) || physical_root.starts_with(&physical_dest) {
            return Err(refuse(
                LibraryDestinationRefusal::OverlapsLibrary,
                Some(&root),
            ));
        }
    }

    let cache_root = git_cache::root_in(&paths.cache_dir);
    if physical_dest.starts_with(physical(&cache_root)) {
        return Err(refuse(
            LibraryDestinationRefusal::InsideGitCache,
            Some(&cache_root),
        ));
    }

    for adapter in default_tool_adapters() {
        for dir in std::iter::once(skills_dir_in(&paths.roots, adapter))
            .chain(former_skills_dirs_in(&paths.roots, adapter))
        {
            if physical_dest.starts_with(physical(&dir)) {
                return Err(refuse(
                    LibraryDestinationRefusal::InsideHarnessDir,
                    Some(&dir),
                ));
            }
        }
    }

    for project in store.list_projects()? {
        let root = Path::new(&project.path);
        for adapter in default_tool_adapters() {
            let Some(dir) = project_skills_dir_in(root, adapter) else {
                continue;
            };
            if physical_dest.starts_with(physical(&dir)) {
                return Err(refuse(
                    LibraryDestinationRefusal::InsideProjectDir,
                    Some(&dir),
                ));
            }
        }
    }

    match std::fs::read_dir(dest) {
        Ok(entries) => {
            for entry in entries {
                let entry =
                    entry.map_err(|_| refuse(LibraryDestinationRefusal::NotWritable, None))?;
                // Finder's view-state file is not content.
                if entry.file_name() != ".DS_Store" {
                    return Err(refuse(LibraryDestinationRefusal::NotEmpty, None));
                }
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
            return Err(refuse(LibraryDestinationRefusal::NotWritable, None));
        }
        // Something that is not a folder (a file) is in the way.
        Err(_) => return Err(refuse(LibraryDestinationRefusal::NotEmpty, None)),
    }

    if probe == Probe::Write {
        let created = create_destination(dest)
            .map_err(|_| refuse(LibraryDestinationRefusal::NotWritable, None))?;
        let admitted = admit_physically(store, paths, skills, dest, &refuse).and_then(|()| {
            let probe_file = dest.join(format!(".skills-hub-write-probe-{}", uuid::Uuid::new_v4()));
            std::fs::write(&probe_file, b"")
                .map_err(|_| refuse(LibraryDestinationRefusal::NotWritable, None))?;
            // The probe is the file this call just created in an admitted,
            // empty destination; it is no artifact, so no fence is involved.
            std::fs::remove_file(&probe_file)
                .map_err(|_| refuse(LibraryDestinationRefusal::NotWritable, None))
        });
        if let Err(err) = admitted {
            undo_created(&created);
            return Err(err);
        }
    }
    Ok(())
}

/// Create `dest` and every missing folder above it, one level at a time.
/// Answers the folders this call created, outermost first — a level that
/// turns out to exist already (a case alias of one that does) is not ours.
fn create_destination(dest: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut missing: Vec<&Path> = dest
        .ancestors()
        .take_while(|level| std::fs::symlink_metadata(level).is_err())
        .collect();
    missing.reverse();
    let mut created = Vec::new();
    for level in missing {
        match std::fs::create_dir(level) {
            Ok(()) => created.push(level.to_path_buf()),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(err) => {
                undo_created(&created);
                return Err(err);
            }
        }
    }
    Ok(created)
}

/// Remove the folders [`create_destination`] created, innermost first. Each
/// is removed only while empty (`remove_dir`): this undoes this call's own
/// `mkdir` and can never take content with it, so no fence is involved.
fn undo_created(created: &[PathBuf]) {
    for level in created.iter().rev() {
        if let Err(err) = std::fs::remove_dir(level) {
            log::warn!("[library move] could not remove the folder it created at {level:?}: {err}");
        }
    }
}

/// The refusals decided again once the destination exists, by physical
/// identity (device + inode; the final path on Windows) rather than by
/// spelling: every protected folder is re-resolved now, so one the creation
/// itself materialised through a case alias (`~/.CLAUDE/SKILLS` is
/// `~/.claude/skills` on a case-insensitive volume) is the destination's own
/// identity, and an existing alias the spelling pass missed (macOS keeps the
/// caller's case through `realpath`) compares equal.
fn admit_physically(
    store: &SkillStore,
    paths: &InstallerPaths,
    skills: &[SkillRecord],
    dest: &Path,
    refuse: &impl Fn(LibraryDestinationRefusal, Option<&Path>) -> anyhow::Error,
) -> Result<()> {
    let at = Place::of(dest)?;
    let library = std::iter::once(paths.central_dir.clone()).chain(
        skills
            .iter()
            .map(|skill| PathBuf::from(&skill.central_path))
            .filter(|path| path.is_absolute() && path.parent().is_some()),
    );
    for root in library {
        let place = Place::of(&root)?;
        if at.within(&place) || place.within(&at) {
            return Err(refuse(
                LibraryDestinationRefusal::OverlapsLibrary,
                Some(&root),
            ));
        }
    }
    let cache_root = git_cache::root_in(&paths.cache_dir);
    if at.within(&Place::of(&cache_root)?) {
        return Err(refuse(
            LibraryDestinationRefusal::InsideGitCache,
            Some(&cache_root),
        ));
    }
    for adapter in default_tool_adapters() {
        for dir in std::iter::once(skills_dir_in(&paths.roots, adapter))
            .chain(former_skills_dirs_in(&paths.roots, adapter))
        {
            if at.within(&Place::of(&dir)?) {
                return Err(refuse(
                    LibraryDestinationRefusal::InsideHarnessDir,
                    Some(&dir),
                ));
            }
        }
    }
    for project in store.list_projects()? {
        let root = Path::new(&project.path);
        for adapter in default_tool_adapters() {
            let Some(dir) = project_skills_dir_in(root, adapter) else {
                continue;
            };
            if at.within(&Place::of(&dir)?) {
                return Err(refuse(
                    LibraryDestinationRefusal::InsideProjectDir,
                    Some(&dir),
                ));
            }
        }
    }
    Ok(())
}

/// `path` as the filesystem places it: its deepest existing ancestor
/// resolved through every alias (real spelling), with the rest appended as
/// written. Comparisons between two such forms are physical as far as the
/// disk can say.
fn physical(path: &Path) -> PathBuf {
    let mut existing = path;
    let mut rest = Vec::new();
    loop {
        if let Ok(resolved) = existing.canonicalize() {
            let mut out = resolved;
            for part in rest.iter().rev() {
                out.push(part);
            }
            return out;
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_os_string());
                existing = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// The cloud-sync root `dest` lies under, if any — classified here so the
/// frontend reads no paths.
fn cloud_sync_of(home: &Path, dest: &Path) -> Option<CloudSync> {
    let dest = physical(dest);
    let home = physical(home);
    let found = |provider, root: PathBuf| {
        Some(CloudSync {
            provider,
            root: root.to_string_lossy().into_owned(),
        })
    };
    let icloud = home.join("Library").join("Mobile Documents");
    if dest.starts_with(&icloud) {
        return found(CloudSyncProvider::IcloudDrive, icloud);
    }
    let cloud_storage = home.join("Library").join("CloudStorage");
    if let Ok(rest) = dest.strip_prefix(&cloud_storage) {
        if let Some(Component::Normal(first)) = rest.components().next() {
            return found(CloudSyncProvider::CloudStorage, cloud_storage.join(first));
        }
    }
    if let Ok(rest) = dest.strip_prefix(&home) {
        if let Some(Component::Normal(first)) = rest.components().next() {
            if first.to_string_lossy().starts_with("OneDrive") {
                return found(CloudSyncProvider::OneDrive, home.join(first));
            }
        }
    }
    None
}

/// A skill moves when its central copy is there (a dangling link is gone).
fn is_locatable(skill: &SkillRecord) -> bool {
    Path::new(&skill.central_path).is_dir()
}

// ---------------------------------------------------------------------------
// Copy
// ---------------------------------------------------------------------------

/// One moved skill: its record as it was, its old copy (and where that
/// physically is, for the re-point and clean checks), its new copy, and both
/// copies' physical identities as the copy phase saw them — what the clean
/// requires them still to be before it removes anything.
struct Moved {
    skill: SkillRecord,
    old: PathBuf,
    old_place: Place,
    new: PathBuf,
    old_id: EntryId,
    new_id: EntryId,
}

/// What the copy phase settled: the skills it copied and verified, and the
/// locatable ones it keeps where they are.
struct Copied {
    moved: Vec<Moved>,
    kept: Vec<(SkillRecord, SkillKept)>,
}

/// A locatable skill as the move sees it before anything is copied: where
/// its copy physically is, and whether it stays where it is (`Ok(None)`: it
/// moves; an error: its copy could not be read, which fails the move).
struct Classified {
    place: Place,
    kept: Result<Option<SkillKept>>,
}

/// Decide, read-only, which locatable skills stay where they are — the one
/// rule the preview counts and the copy phase applies:
///
/// * two records of one physical copy (or of copies one inside the other)
///   alias one set of bytes: copying it twice would split one skill into
///   two, so neither moves (`shared_copy`);
/// * a copy holding something a faithful copy elsewhere would not reproduce
///   stays (`unrelocatable`, see [`unrelocatable`]).
fn classify(central_dir: &Path, locatable: &[SkillRecord]) -> Result<Vec<Classified>> {
    let places = locatable
        .iter()
        .map(|skill| Place::of(Path::new(&skill.central_path)))
        .collect::<Result<Vec<_>>>()?;
    // The old Library, physically: its root and every copy about to move.
    // An absolute link into any of them would not follow the move.
    let mut old_library = vec![Place::of(central_dir)?];
    for skill in locatable {
        old_library.push(Place::of(Path::new(&skill.central_path))?);
    }
    let shared_with: Vec<Option<String>> = (0..places.len())
        .map(|i| {
            (0..places.len())
                .find(|&j| j != i && places[i].overlaps(&places[j]))
                .map(|j| locatable[j].id.clone())
        })
        .collect();
    Ok(locatable
        .iter()
        .zip(places)
        .zip(shared_with)
        .map(|((skill, place), shared_with)| {
            let kept = match shared_with {
                Some(with_skill_id) => Ok(Some(SkillKept::SharedCopy { with_skill_id })),
                None => unrelocatable(
                    Path::new(&skill.central_path),
                    &place,
                    central_dir,
                    &old_library,
                )
                .map(|found| {
                    found.map(|(entry, cause)| SkillKept::Unrelocatable {
                        entry: entry.to_string_lossy().into_owned(),
                        cause,
                    })
                }),
            };
            Classified { place, kept }
        })
        .collect())
}

fn copy_phase(
    store: &SkillStore,
    paths: &InstallerPaths,
    dest: &Path,
    locatable: Vec<SkillRecord>,
    on_progress: &mut impl FnMut(LibraryMoveProgress),
) -> Result<Copied> {
    let total = locatable.len() as u32;
    let classified = classify(&paths.central_dir, &locatable)?;
    let mut copied = Copied {
        moved: Vec::with_capacity(locatable.len()),
        kept: Vec::new(),
    };
    for (done, (skill, Classified { place, kept })) in
        locatable.into_iter().zip(classified).enumerate()
    {
        on_progress(LibraryMoveProgress {
            phase: LibraryMovePhase::Copying,
            done: done as u32,
            total,
            name: skill.name.clone(),
        });
        let old = PathBuf::from(&skill.central_path);
        // The folder this skill reserved, once it has one: staged, so a
        // failure removes it with the copies before it.
        let mut reserved: Option<PathBuf> = None;
        let attempt = (|| -> Result<std::result::Result<(EntryId, EntryId), SkillKept>> {
            if let Some(reason) = kept? {
                return Ok(Err(reason));
            }
            let old_id = EntryId::of(&old)?.with_context(|| format!("{:?} is gone", old))?;
            let new = reserved.insert(reserve(dest, &old, &skill)?);
            let new_id = EntryId::of(new)?.with_context(|| format!("{:?} is gone", new))?;
            copy_faithfully(&old, new)?;
            if !same_tree(&old, new)? {
                anyhow::bail!("the copy of {:?} does not reproduce it", old);
            }
            Ok(Ok((old_id, new_id)))
        })();
        match attempt {
            Ok(Err(reason)) => copied.kept.push((skill, reason)),
            Ok(Ok((old_id, new_id))) => copied.moved.push(Moved {
                skill,
                old,
                old_place: place,
                new: reserved.expect("a copied skill reserved its folder"),
                old_id,
                new_id,
            }),
            Err(err) => {
                let staged = copied
                    .moved
                    .iter()
                    .map(|m| m.new.as_path())
                    .chain(reserved.as_deref());
                let cleanup = rollback(store, paths, dest, staged);
                let detail = match cleanup {
                    Some(problem) => format!("{err:#}; {problem}"),
                    None => format!("{err:#}"),
                };
                anyhow::bail!(SignalError::LibraryCopyFailed {
                    name: skill.name.clone(),
                    path: skill.central_path.clone(),
                    detail,
                });
            }
        }
    }
    Ok(copied)
}
/// Reserve the folder a copy takes in the new Library: its current name, or
/// `<name>-2`, `<name>-3`, … when that is taken. Taken is the destination
/// filesystem's answer — an exclusive `create_dir` that reports
/// `AlreadyExists` — never a comparison of spellings: on a case-insensitive
/// volume `alpha` and `ALPHA` (a split Library may hold both) are one folder.
/// The reserved folder is new and empty; a copy never merges into another.
fn reserve(dest: &Path, old: &Path, skill: &SkillRecord) -> Result<PathBuf> {
    let base = old
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_else(|| skill.id.clone().into());
    for n in 1u32.. {
        let mut candidate = base.clone();
        if n > 1 {
            candidate.push(format!("-{n}"));
        }
        let path = dest.join(&candidate);
        match std::fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(anyhow::Error::new(err).context(format!("create {:?}", path))),
        }
    }
    unreachable!("a free folder name exists")
}

/// Remove every staged copy (before the switch). Answers a description of
/// what could not be removed, for the failure's detail.
fn rollback<'p>(
    store: &SkillStore,
    paths: &InstallerPaths,
    dest: &Path,
    staged: impl Iterator<Item = &'p Path>,
) -> Option<String> {
    let problems: Vec<String> = staged
        .filter_map(|path| {
            remove_staged_library_copy_unlocked(store, &paths.central_dir, dest, path)
                .err()
                .map(|err| format!("staged copy {:?} left: {err:#}", path))
        })
        .collect();
    for problem in &problems {
        log::warn!("[library move] {problem}");
    }
    (!problems.is_empty()).then(|| problems.join("; "))
}

// ---------------------------------------------------------------------------
// Relocation fidelity
// ---------------------------------------------------------------------------
//
// The move relocates a skill's copy; it does not deploy it. The deploy copy
// (`sync_engine::copy_dir_recursive`) leaves out `.git` and internal links,
// and Content identity ignores them too — right for a harness, wrong for the
// one copy of a skill the clean is about to remove. So the move copies every
// entry with [`copy_faithfully`] and proves the result with [`same_tree`] —
// which the clean runs again, on the old copy renamed aside, right before it
// removes it.
//
// **The compared facets.** Two trees are the same skill when all of these
// hold:
//
// * the tree: in every folder the same entry names, each of the same kind
//   (folder, file, link) — dotfiles and `.git` included, links never
//   followed;
// * files: the same bytes;
// * links: the same text (recreated verbatim);
// * permissions: the same mode bits (`0o7777`) on every file and folder, the
//   skill's own folder included (Unix); the same read-only flag on every
//   file (Windows);
// * extended attributes (Unix): the same names and values on every file,
//   folder and link, the skill's own folder included. On Linux that is the
//   `user.*` namespace, the one an unprivileged owner reads and writes
//   (`security.*`, `trusted.*` and `system.*` are the system's); on macOS
//   every name, `com.apple.*` included (Finder info, resource forks,
//   quarantine, Spotlight metadata), except the two the system alone owns:
//   `com.apple.provenance`, which the kernel assigns to every file a process
//   creates and silently ignores writes to, and `com.apple.rootless` (System
//   Integrity Protection), which no process may write;
// * hard links: names of one file inside the skill are names of one file in
//   the copy, with the same link count. A file with another name outside the
//   skill cannot be relocated as one file (`hardlink_outside`).
//
// Preserved best-effort, never compared: modification times. Not preserved
// (the residuals): ACLs, BSD file flags, Windows alternate data streams,
// sparse allocation (a sparse file's holes are written out), ownership and
// creation times.

/// The first entry of the skill's old copy `old` (physically `old_place`)
/// that a verbatim copy elsewhere would not reproduce, and why — `None` when
/// the whole tree relocates faithfully. `old_root` is the old Library folder
/// as configured; `old_library` the old Library physically (its root and
/// every copy that moves).
///
/// * An absolute link into the old Library would still point there after
///   the move (and dangle once the clean removes its target).
/// * A relative link that reaches outside the skill's folder — lexically, or
///   as the filesystem resolves it — would point elsewhere from the new place.
/// * A FIFO, socket or device cannot be copied.
/// * A file with a name outside the skill (its link count exceeds the names
///   it has inside) would become two files.
///
/// Every other link (relative inside the skill, absolute elsewhere, dangling
/// either way) reads the same from the new place.
fn unrelocatable(
    old: &Path,
    old_place: &Place,
    old_root: &Path,
    old_library: &[Place],
) -> Result<Option<(PathBuf, UnrelocatableCause)>> {
    // Every multiply-linked file: its link count, the names found inside the
    // skill, and the first of them.
    let mut linked: HashMap<EntryId, (u64, u64, PathBuf)> = HashMap::new();
    let mut pending: Vec<(PathBuf, Vec<OsString>)> = vec![(old.to_path_buf(), vec![])];
    while let Some((dir, depth)) = pending.pop() {
        let entries = std::fs::read_dir(&dir).with_context(|| format!("read {:?}", dir))?;
        for entry in entries {
            let entry = entry.with_context(|| format!("read {:?}", dir))?;
            let path = entry.path();
            let kind = entry
                .file_type()
                .with_context(|| format!("stat {:?}", path))?;
            if kind.is_dir() {
                let mut below = depth.clone();
                below.push(entry.file_name());
                pending.push((path, below));
            } else if kind.is_symlink() {
                let text = std::fs::read_link(&path).with_context(|| format!("read {:?}", path))?;
                let resolved = path
                    .canonicalize()
                    .ok()
                    .map(|p| Place::of(&p))
                    .transpose()?;
                let cause = if text.is_absolute() {
                    let lexically_old = text.starts_with(old) || text.starts_with(old_root);
                    let physically_old = resolved
                        .as_ref()
                        .is_some_and(|at| old_library.iter().any(|lib| at.within(lib)));
                    (lexically_old || physically_old)
                        .then_some(UnrelocatableCause::LinkIntoOldLibrary)
                } else {
                    let escapes_lexically = escapes(&depth, &text);
                    let escapes_physically =
                        resolved.as_ref().is_some_and(|at| !at.within(old_place));
                    (escapes_lexically || escapes_physically)
                        .then_some(UnrelocatableCause::LinkEscapes)
                };
                if let Some(cause) = cause {
                    return Ok(Some((path, cause)));
                }
            } else if kind.is_file() {
                let (id, links) =
                    EntryId::with_links(&path).with_context(|| format!("stat {:?}", path))?;
                if links > 1 {
                    let names = linked.entry(id).or_insert((links, 0, path.clone()));
                    names.1 += 1;
                    if path < names.2 {
                        names.2 = path;
                    }
                }
            } else {
                return Ok(Some((path, UnrelocatableCause::SpecialFile)));
            }
        }
    }
    Ok(linked
        .into_values()
        .filter(|(links, inside, _)| inside < links)
        .map(|(_, _, first)| first)
        .min()
        .map(|first| (first, UnrelocatableCause::HardlinkOutside)))
}

/// Whether the relative link text `text`, read from a folder `depth` levels
/// (by name) below the skill's root, climbs out of the skill.
fn escapes(depth: &[OsString], text: &Path) -> bool {
    let mut level = depth.len();
    for component in text.components() {
        match component {
            Component::ParentDir => match level.checked_sub(1) {
                Some(up) => level = up,
                None => return true,
            },
            Component::Normal(_) => level += 1,
            Component::CurDir => {}
            // A rooted or prefixed text is not relative.
            Component::RootDir | Component::Prefix(_) => return true,
        }
    }
    false
}

/// Copy the skill at `src` (a link to its folder is followed) into the
/// empty folder `dst` over every compared facet: folders, files (bytes,
/// permissions, extended attributes), links recreated with their text
/// verbatim, and hard-link groups recreated as one file. Modification times
/// are kept best-effort. A folder's permissions are set once its contents
/// are in, so a read-only folder is filled first.
fn copy_faithfully(src: &Path, dst: &Path) -> Result<()> {
    let src = src
        .canonicalize()
        .with_context(|| format!("resolve {:?}", src))?;
    let mut linked = HashMap::new();
    copy_contents(&src, dst, &mut linked)?;
    finish_dir(&src, dst)
}

/// Copy every entry of the folder `src` into the folder `dst`. `linked`
/// maps each multiply-linked source file already copied to its copy.
fn copy_contents(src: &Path, dst: &Path, linked: &mut HashMap<EntryId, PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(src).with_context(|| format!("read {:?}", src))? {
        let entry = entry.with_context(|| format!("read {:?}", src))?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let kind = entry
            .file_type()
            .with_context(|| format!("stat {:?}", from))?;
        if kind.is_symlink() {
            let text = std::fs::read_link(&from).with_context(|| format!("read {:?}", from))?;
            make_link(&kind, &text, &to).with_context(|| format!("link {:?} -> {:?}", to, text))?;
            copy_xattrs(&from, &to)?;
        } else if kind.is_dir() {
            std::fs::create_dir(&to).with_context(|| format!("create {:?}", to))?;
            copy_contents(&from, &to, linked)?;
            finish_dir(&from, &to)?;
        } else if kind.is_file() {
            let (id, links) =
                EntryId::with_links(&from).with_context(|| format!("stat {:?}", from))?;
            if links > 1 {
                if let Some(first) = linked.get(&id) {
                    std::fs::hard_link(first, &to)
                        .with_context(|| format!("link {:?} to {:?}", to, first))?;
                    continue;
                }
            }
            copy_file(&from, &to)?;
            if links > 1 {
                linked.insert(id, to);
            }
        } else {
            anyhow::bail!("{:?} is not a file, folder or link", from);
        }
    }
    Ok(())
}

/// Copy one file: bytes, extended attributes, then its exact permissions
/// (an attribute is written while the copy is still writable), then its
/// modification time.
fn copy_file(from: &Path, to: &Path) -> Result<()> {
    let meta = std::fs::symlink_metadata(from).with_context(|| format!("stat {:?}", from))?;
    std::fs::copy(from, to).with_context(|| format!("copy {:?} -> {:?}", from, to))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let writable = std::fs::Permissions::from_mode(meta.permissions().mode() | 0o600);
        std::fs::set_permissions(to, writable).with_context(|| format!("chmod {:?}", to))?;
        copy_xattrs(from, to)?;
        std::fs::set_permissions(to, meta.permissions())
            .with_context(|| format!("chmod {:?}", to))?;
    }
    preserve_modified(to, &meta);
    Ok(())
}

/// A copied folder's own facets, once its contents are in: extended
/// attributes, permissions (Unix), modification time.
fn finish_dir(from: &Path, to: &Path) -> Result<()> {
    let meta = std::fs::metadata(from).with_context(|| format!("stat {:?}", from))?;
    copy_xattrs(from, to)?;
    #[cfg(unix)]
    std::fs::set_permissions(to, meta.permissions()).with_context(|| format!("chmod {:?}", to))?;
    preserve_modified(to, &meta);
    Ok(())
}

/// Give `to` the modification time `from` reports, if the platform lets
/// this process (best-effort: never compared, never a failure).
fn preserve_modified(to: &Path, from: &std::fs::Metadata) {
    let Ok(modified) = from.modified() else {
        return;
    };
    #[cfg(unix)]
    let opened = std::fs::File::open(to);
    #[cfg(windows)]
    let opened = {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_WRITE_ATTRIBUTES: u32 = 0x0100;
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        std::fs::OpenOptions::new()
            .access_mode(FILE_WRITE_ATTRIBUTES)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(to)
    };
    if let Ok(file) = opened {
        let _ = file.set_modified(modified);
    }
}

/// Whether an extended attribute is one the copy carries and compares (see
/// § The compared facets).
#[cfg(unix)]
fn compared_xattr(name: &std::ffi::OsStr) -> bool {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::ffi::OsStrExt;
        name.as_bytes().starts_with(b"user.")
    }
    #[cfg(not(target_os = "linux"))]
    {
        name != "com.apple.provenance" && name != "com.apple.rootless"
    }
}

/// The compared extended attributes of the entry at `path` (a link is read
/// as itself), sorted by name. A volume that supports none has none.
#[cfg(unix)]
fn xattrs(path: &Path) -> Result<Vec<(OsString, Vec<u8>)>> {
    let unsupported = |err: &std::io::Error| matches!(err.raw_os_error(), Some(code) if code == libc::ENOTSUP || code == libc::EOPNOTSUPP);
    let names = match xattr::list(path) {
        Ok(names) => names,
        Err(err) if unsupported(&err) => return Ok(Vec::new()),
        Err(err) => {
            return Err(err).with_context(|| format!("list the extended attributes of {:?}", path))
        }
    };
    let mut out = Vec::new();
    for name in names.filter(|name| compared_xattr(name)) {
        let value = xattr::get(path, &name)
            .with_context(|| format!("read the extended attribute {:?} of {:?}", name, path))?;
        if let Some(value) = value {
            out.push((name, value));
        }
    }
    out.sort();
    Ok(out)
}

/// Write `from`'s compared extended attributes onto `to` (links as
/// themselves).
fn copy_xattrs(from: &Path, to: &Path) -> Result<()> {
    #[cfg(unix)]
    for (name, value) in xattrs(from)? {
        xattr::set(to, &name, &value)
            .with_context(|| format!("write the extended attribute {:?} onto {:?}", name, to))?;
    }
    #[cfg(not(unix))]
    let _ = (from, to);
    Ok(())
}

/// Recreate a link of type `kind` reading `text` at `at`.
fn make_link(kind: &std::fs::FileType, text: &Path, at: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let _ = kind;
        std::os::unix::fs::symlink(text, at)
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileTypeExt;
        if kind.is_symlink_dir() {
            std::os::windows::fs::symlink_dir(text, at)
        } else {
            std::os::windows::fs::symlink_file(text, at)
        }
    }
}

/// Whether `dst` reproduces the skill at `src` over every compared facet
/// (§ The compared facets). Either root may be a link to its folder. An
/// error is a tree that could not be read — never a "same".
fn same_tree(src: &Path, dst: &Path) -> Result<bool> {
    let resolve = |p: &Path| p.canonicalize().with_context(|| format!("resolve {:?}", p));
    let (src, dst) = (resolve(src)?, resolve(dst)?);
    let root = std::fs::symlink_metadata(&src)
        .with_context(|| format!("stat {:?}", src))?
        .file_type();
    let mut links = HardLinks::default();
    Ok(same_facets(&src, &dst, &root)? && same_contents(&src, &dst, &mut links)?)
}

/// Whether the folders `src` and `dst` hold the same entries, each the same
/// over every compared facet.
fn same_contents(src: &Path, dst: &Path, links: &mut HardLinks) -> Result<bool> {
    let listing = |dir: &Path| -> Result<Vec<(OsString, std::fs::FileType)>> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(dir).with_context(|| format!("read {:?}", dir))? {
            let entry = entry.with_context(|| format!("read {:?}", dir))?;
            let kind = entry
                .file_type()
                .with_context(|| format!("stat {:?}", entry.path()))?;
            out.push((entry.file_name(), kind));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    };
    let (want, got) = (listing(src)?, listing(dst)?);
    if want.len() != got.len() || want.iter().zip(&got).any(|(a, b)| a.0 != b.0) {
        return Ok(false);
    }
    for ((name, want_kind), (_, got_kind)) in want.iter().zip(&got) {
        let (from, to) = (src.join(name), dst.join(name));
        let same = if want_kind.is_symlink() {
            got_kind.is_symlink() && std::fs::read_link(&from)? == std::fs::read_link(&to)?
        } else if want_kind.is_dir() {
            got_kind.is_dir() && same_contents(&from, &to, links)?
        } else {
            got_kind.is_file() && same_file_bytes(&from, &to)? && links.pair(&from, &to)?
        };
        if !same || !same_facets(&from, &to, want_kind)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Whether two entries of kind `kind` carry the same permissions and
/// extended attributes (§ The compared facets).
fn same_facets(a: &Path, b: &Path, kind: &std::fs::FileType) -> Result<bool> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if !kind.is_symlink() {
            let mode = |p: &Path| {
                std::fs::symlink_metadata(p)
                    .map(|m| m.permissions().mode() & 0o7777)
                    .with_context(|| format!("stat {:?}", p))
            };
            if mode(a)? != mode(b)? {
                return Ok(false);
            }
        }
        Ok(xattrs(a)? == xattrs(b)?)
    }
    #[cfg(windows)]
    {
        if !kind.is_file() {
            return Ok(true);
        }
        let readonly = |p: &Path| {
            std::fs::symlink_metadata(p)
                .map(|m| m.permissions().readonly())
                .with_context(|| format!("stat {:?}", p))
        };
        Ok(readonly(a)? == readonly(b)?)
    }
}

/// The hard-link structure seen so far while comparing two trees: which
/// file of the copy each multiply-linked source file corresponds to, both
/// ways.
#[derive(Default)]
struct HardLinks {
    forward: HashMap<EntryId, EntryId>,
    back: HashMap<EntryId, EntryId>,
}

impl HardLinks {
    /// Whether the files `a` (source) and `b` (copy) have the same link
    /// count and pair up consistently with every pair seen before: names of
    /// one source file are names of one copied file, and no other.
    fn pair(&mut self, a: &Path, b: &Path) -> Result<bool> {
        let (a_id, a_links) = EntryId::with_links(a).with_context(|| format!("stat {:?}", a))?;
        let (b_id, b_links) = EntryId::with_links(b).with_context(|| format!("stat {:?}", b))?;
        if a_links != b_links {
            return Ok(false);
        }
        if a_links < 2 {
            return Ok(true);
        }
        Ok(match (self.forward.get(&a_id), self.back.get(&b_id)) {
            (None, None) => {
                self.forward.insert(a_id, b_id);
                self.back.insert(b_id, a_id);
                true
            }
            (Some(seen_b), Some(seen_a)) => *seen_b == b_id && *seen_a == a_id,
            _ => false,
        })
    }
}

/// Whether two files hold the same bytes, read in chunks.
fn same_file_bytes(a: &Path, b: &Path) -> Result<bool> {
    use std::io::Read;
    let open = |p: &Path| {
        std::fs::File::open(p)
            .map(std::io::BufReader::new)
            .with_context(|| format!("open {:?}", p))
    };
    let (mut a, mut b) = (open(a)?, open(b)?);
    let (mut x, mut y) = (vec![0u8; 64 * 1024], vec![0u8; 64 * 1024]);
    loop {
        let n = read_full(&mut a, &mut x)?;
        let m = read_full(&mut b, &mut y)?;
        if n != m || x[..n] != y[..m] {
            return Ok(false);
        }
        if n == 0 {
            return Ok(true);
        }
    }
    fn read_full(r: &mut impl Read, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut filled = 0;
        while filled < buf.len() {
            match r.read(&mut buf[filled..])? {
                0 => break,
                n => filled += n,
            }
        }
        Ok(filled)
    }
}

// ---------------------------------------------------------------------------
// Re-point
// ---------------------------------------------------------------------------

/// What sits at a Deployment's path, against the skill's old copy.
enum Occupant {
    /// A link (a junction counts) that resolves to the old copy — or to the
    /// new one, already re-pointed before a crash: re-made into it again.
    Ours,
    /// A real folder: a copy-mode Deployment.
    Copy,
    /// Anything else: nothing there, a link elsewhere, or a real folder
    /// that *is* the old copy through an aliased harness dir.
    NotOurs,
}

fn occupant(path: &Path, old_place: &Place, new_place: &Place) -> Result<Occupant> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Occupant::NotOurs),
        Err(err) => return Err(err.into()),
    };
    // Physically, by identity: a link that dangles reaches nothing.
    let at = Place::of(path)?;
    let reaches_old = at.same(old_place);
    let reaches_new = at.same(new_place);
    Ok(
        match (meta.file_type().is_symlink(), reaches_old || reaches_new) {
            (true, true) => Occupant::Ours,
            (false, false) => Occupant::Copy,
            _ => Occupant::NotOurs,
        },
    )
}

/// One Deployment path of a moved skill and the owned rows describing it.
enum Rows {
    Global(Vec<SkillTargetRecord>),
    Assignment {
        project_root: PathBuf,
        rows: Vec<super::skill_store::ProjectSkillAssignmentRecord>,
    },
}

fn relink_phase(
    store: &SkillStore,
    dest: &Path,
    moved: &[Moved],
    now: i64,
    on_progress: &mut impl FnMut(LibraryMoveProgress),
) -> Result<Vec<RelinkOutcome>> {
    let library = ProtectedLibrary::new(store, dest);
    let total = moved.len() as u32;
    let mut outcomes = Vec::new();
    for (done, m) in moved.iter().enumerate() {
        on_progress(LibraryMoveProgress {
            phase: LibraryMovePhase::Relinking,
            done: done as u32,
            total,
            name: m.skill.name.clone(),
        });
        let skill_disabled = store.skill_disabled_at(&m.skill.id)?.is_some();
        let mut new_record = m.skill.clone();
        new_record.central_path = m.new.to_string_lossy().into_owned();

        for (path, rows) in deployments_of(store, &m.skill)? {
            let (row_refs, adapter_key, disabled) = match &rows {
                Rows::Global(rows) => (
                    rows.iter()
                        .map(|row| RowRef::GlobalTarget {
                            id: row.id.clone(),
                            skill_id: row.skill_id.clone(),
                            tool: row.tool.clone(),
                        })
                        .collect::<Vec<_>>(),
                    rows[0].tool.clone(),
                    rows.iter().any(|row| row.status == SyncStatus::Disabled),
                ),
                Rows::Assignment { rows, .. } => (
                    rows.iter()
                        .map(|row| RowRef::Assignment {
                            id: row.id.clone(),
                            project_id: row.project_id.clone(),
                            skill_id: row.skill_id.clone(),
                            tool: row.tool.clone(),
                        })
                        .collect(),
                    rows[0].tool.clone(),
                    rows.iter().any(|row| row.status == SyncStatus::Disabled),
                ),
            };
            let status = if skill_disabled || disabled {
                RelinkStatus::Skipped {
                    reason: RelinkSkip::Disabled,
                }
            } else if matches!(&rows, Rows::Assignment { project_root, .. }
                if !project_ops::project_root_present(project_root))
            {
                RelinkStatus::Skipped {
                    reason: RelinkSkip::FolderMissing,
                }
            } else {
                relink_one(
                    store,
                    &library,
                    m,
                    &new_record,
                    &path,
                    &adapter_key,
                    &rows,
                    now,
                )?
            };
            outcomes.push(RelinkOutcome {
                skill_id: m.skill.id.clone(),
                name: m.skill.name.clone(),
                path: path.to_string_lossy().into_owned(),
                rows: row_refs,
                status,
            });
        }
    }
    Ok(outcomes)
}

/// Every owned Deployment of `skill`, by path: global target rows grouped by
/// their recorded path, project assignments by their resolved artifact. Rows
/// that own nothing (never materialised) are not Deployments of ours.
fn deployments_of(store: &SkillStore, skill: &SkillRecord) -> Result<Vec<(PathBuf, Rows)>> {
    let mut out: Vec<(PathBuf, Rows)> = Vec::new();
    for row in store.list_skill_targets(&skill.id)? {
        if !row.owns_artifact() {
            continue;
        }
        let path = PathBuf::from(&row.target_path);
        match out
            .iter_mut()
            .find(|(p, rows)| p == &path && matches!(rows, Rows::Global(_)))
        {
            Some((_, Rows::Global(rows))) => rows.push(row),
            _ => out.push((path, Rows::Global(vec![row]))),
        }
    }
    let mut projects = HashMap::new();
    for row in store.list_project_skill_assignments_by_skill(&skill.id)? {
        if !row.owns_artifact() {
            continue;
        }
        let project = match projects.get(&row.project_id) {
            Some(project) => project,
            None => {
                let project = store.get_project_by_id(&row.project_id)?;
                projects.entry(row.project_id.clone()).or_insert(project)
            }
        };
        let (Some(project), Some(adapter)) = (project, adapter_by_key(&row.tool)) else {
            continue;
        };
        let root = PathBuf::from(&project.path);
        let Some(path) =
            resolve_assignment_artifact(&root, adapter, &row, || Ok(Some(skill.name.clone())))?
        else {
            continue;
        };
        match out.iter_mut().find(|(p, rows)| {
            p == &path
                && matches!(rows, Rows::Assignment { project_root, .. } if project_root == &root)
        }) {
            Some((_, Rows::Assignment { rows, .. })) => rows.push(row),
            _ => out.push((
                path,
                Rows::Assignment {
                    project_root: root,
                    rows: vec![row],
                },
            )),
        }
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn relink_one(
    store: &SkillStore,
    library: &ProtectedLibrary<'_>,
    m: &Moved,
    new_record: &SkillRecord,
    path: &Path,
    tool: &str,
    rows: &Rows,
    now: i64,
) -> Result<RelinkStatus> {
    let attempt = (|| -> Result<Option<SyncOutcome>> {
        match occupant(path, &m.old_place, &Place::of(&m.new)?)? {
            Occupant::Copy => return Ok(None),
            Occupant::NotOurs => anyhow::bail!(NotOurs),
            Occupant::Ours => {}
        }
        let adapter: &ToolAdapter = adapter_by_key(tool).ok_or_else(|| {
            anyhow::anyhow!(SignalError::UnknownTool {
                tool: tool.to_string(),
            })
        })?;
        let identity = Source::Managed {
            store,
            skill_id: &new_record.id,
        };
        let outcome = match rows {
            Rows::Global(_) => {
                sync_dir_for_tool_with_overwrite(adapter, &m.new, identity, path, true, library)?
            }
            Rows::Assignment { project_root, .. } => sync_dir_for_tool_within_root(
                adapter,
                &m.new,
                identity,
                path,
                true,
                project_root,
                library,
            )?,
        };
        Ok(Some(outcome))
    })();

    match attempt {
        Ok(None) => Ok(RelinkStatus::Unchanged {
            reason: RelinkUnchanged::Copy,
        }),
        Ok(Some(outcome)) => {
            let target_path = path.to_string_lossy();
            match rows {
                Rows::Global(rows) => {
                    for row in rows {
                        store.transition_skill_target(
                            &row.id,
                            TargetTransition::SyncCompleted {
                                mode: outcome.mode_used,
                                target_path: &target_path,
                                synced_at: now,
                            },
                        )?;
                    }
                }
                Rows::Assignment { rows, .. } => {
                    let hash = outcome
                        .mode_used
                        .can_drift()
                        .then(|| content_identity::read(Source::Directory(&m.new)))
                        .flatten();
                    for row in rows {
                        store.transition_assignment(
                            &row.id,
                            AssignmentTransition::SyncCompleted {
                                mode: outcome.mode_used,
                                synced_at: now,
                                content_hash: hash.as_deref(),
                            },
                        )?;
                    }
                }
            }
            Ok(RelinkStatus::Relinked {
                mode: outcome.mode_used,
            })
        }
        Err(err) if err.is::<NotOurs>() => Ok(RelinkStatus::Skipped {
            reason: RelinkSkip::NotOurs,
        }),
        Err(err) => {
            let detail = format!("{err:#}");
            match rows {
                Rows::Global(rows) => {
                    for row in rows {
                        store.transition_skill_target(
                            &row.id,
                            TargetTransition::SyncFailed { error: &detail },
                        )?;
                    }
                }
                Rows::Assignment { rows, .. } => {
                    for row in rows {
                        store.transition_assignment(
                            &row.id,
                            AssignmentTransition::SyncFailed { error: &detail },
                        )?;
                    }
                }
            }
            Ok(RelinkStatus::Failed {
                error: CommandError::from_anyhow(err),
            })
        }
    }
}

/// Internal control flow: the occupant is not a link of ours.
#[derive(Debug)]
struct NotOurs;

impl std::fmt::Display for NotOurs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "not a link to the old copy")
    }
}

impl std::error::Error for NotOurs {}

// ---------------------------------------------------------------------------
// Clean
// ---------------------------------------------------------------------------

/// The prefix of the folder the clean renames an old copy into, inside the
/// old Library folder, between proving it a duplicate and removing it.
const SET_ASIDE_PREFIX: &str = ".skills-hub-clean-";

/// Clean every moved skill's old copy that is provably a duplicate of its
/// new one, at the moment of its removal, and only inside the old Library
/// folder `old_root`. Answers what stays.
///
/// An old copy is left in place, and reported, when:
///
/// * it is not a folder directly inside `old_root` (`outside_library_folder`
///   — a split Library's copy elsewhere, or a record naming the root);
/// * a recorded copy of any skill is it, holds it or lies inside it
///   (`still_recorded`);
/// * a Deployment of any skill reaches into it or holds it, or a link
///   inside any skill that did not move resolves into it (`still_linked`);
/// * the record, the new copy or the old copy is no longer what the copy
///   phase saw (`stale_journal`, `destination_missing`, `replaced`);
/// * set aside and compared again, it is not the same as the new copy
///   (`changed_since_copy`), or its removal failed (`removal_failed`).
///
/// Retention is decided over physical copies, never per skill id. `left` —
/// what the caller already decided to leave — is reported among what stays,
/// never again as `unrecorded`.
fn clean_phase(
    store: &SkillStore,
    old_root: &Path,
    dest: &Path,
    moved: &[Moved],
    left: Vec<LeftBehind>,
    on_progress: &mut impl FnMut(LibraryMoveProgress),
) -> Result<Vec<LeftBehind>> {
    let records = store.list_skills()?;
    let recorded: Vec<Place> = records
        .iter()
        .filter(|skill| Path::new(&skill.central_path).is_absolute())
        .map(|skill| Place::of(Path::new(&skill.central_path)))
        .collect::<Result<_>>()?;
    let reached = deployment_places(store)?;
    let moved_ids: HashSet<&str> = moved.iter().map(|m| m.skill.id.as_str()).collect();
    let linked = link_places(
        records
            .iter()
            .filter(|skill| !moved_ids.contains(skill.id.as_str())),
    )?;
    let total = moved.len() as u32;
    let mut kept: Vec<LeftBehind> = left;
    for (done, m) in moved.iter().enumerate() {
        on_progress(LibraryMoveProgress {
            phase: LibraryMovePhase::Cleaning,
            done: done as u32,
            total,
            name: m.skill.name.clone(),
        });
        let mut keep = |reason, at: &Path, error: Option<anyhow::Error>| {
            kept.push(LeftBehind {
                path: at.to_string_lossy().into_owned(),
                reason,
                skill_id: Some(m.skill.id.clone()),
                error: error.map(CommandError::from_anyhow),
            })
        };
        // Already gone: a resumed move whose clean had removed it.
        if !present(&m.old) {
            continue;
        }
        if !in_library_folder(&m.old, old_root) {
            keep(LeftBehindReason::OutsideLibraryFolder, &m.old, None);
            continue;
        }
        let place = Place::of(&m.old)?;
        let retained = if recorded.iter().any(|copy| copy.overlaps(&place)) {
            Some(LeftBehindReason::StillRecorded)
        } else if reached.iter().any(|at| at.overlaps(&place))
            || linked
                .as_ref()
                .is_none_or(|links| links.iter().any(|at| at.within(&place)))
        {
            Some(LeftBehindReason::StillLinked)
        } else {
            None
        };
        if let Some(reason) = retained {
            keep(reason, &m.old, None);
            continue;
        }
        match remove_duplicate(store, old_root, dest, m) {
            Cleaned::Removed => {}
            Cleaned::Kept { reason, at, error } => {
                if let Some(err) = &error {
                    log::warn!("[library move] old copy {:?} kept: {err:#}", at);
                }
                keep(reason, &at, error);
            }
        }
    }

    let mut left = Vec::new();
    if old_root.is_dir() {
        left.push(LeftBehind {
            path: old_root.to_string_lossy().into_owned(),
            reason: LeftBehindReason::OldLibrary,
            skill_id: None,
            error: None,
        });
        let kept_paths: HashSet<PathBuf> = kept.iter().map(|k| PathBuf::from(&k.path)).collect();
        // A set-aside folder holding an old copy reported kept is that copy's.
        let holds_kept = |path: &Path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(SET_ASIDE_PREFIX))
                && kept_paths.iter().any(|kept| kept.starts_with(path))
        };
        let recorded: HashSet<PathBuf> = store
            .list_skills()?
            .into_iter()
            .map(|skill| PathBuf::from(skill.central_path))
            .collect();
        let mut unrecorded: Vec<PathBuf> = std::fs::read_dir(old_root)
            .map(|entries| {
                entries
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| !is_scratch(&entry.file_name().to_string_lossy()))
                    .map(|entry| entry.path())
                    .filter(|path| {
                        !kept_paths.contains(path) && !recorded.contains(path) && !holds_kept(path)
                    })
                    .collect()
            })
            .unwrap_or_default();
        unrecorded.sort();
        left.extend(unrecorded.into_iter().map(|path| LeftBehind {
            path: path.to_string_lossy().into_owned(),
            reason: LeftBehindReason::Unrecorded,
            skill_id: None,
            error: None,
        }));
    }
    left.extend(kept);
    Ok(left)
}

/// How one old copy's clean settled.
enum Cleaned {
    Removed,
    /// Left in place — at `at`, its old path unless it could not be put back
    /// from where it was set aside.
    Kept {
        reason: LeftBehindReason,
        at: PathBuf,
        error: Option<anyhow::Error>,
    },
}

/// Remove `m`'s old copy (a folder directly inside `old_root` that nothing
/// retains) only once it is provably a duplicate, at this moment:
///
/// 1. the record still names the new copy the switch wrote, and both copies
///    still have the identities the copy phase recorded;
/// 2. the old copy is renamed aside inside `old_root`
///    (`.skills-hub-clean-<uuid>/<name>`) — one atomic rename, so nothing can
///    reach it under its old path any more and what is compared next is what
///    is removed;
/// 3. set aside, it is still the entry that was copied, the new copy still
///    is too, and the two are the same over every compared facet
///    ([`same_tree`]);
/// 4. it is removed through `artifact_removal` and the fence, claimed inside
///    the set-aside folder.
///
/// Anything that differs — or a step that fails — renames it back and keeps
/// it. The new Library (`dest`) and every recorded copy stay protected.
fn remove_duplicate(store: &SkillStore, old_root: &Path, dest: &Path, m: &Moved) -> Cleaned {
    let kept = |reason| Cleaned::Kept {
        reason,
        at: m.old.clone(),
        error: None,
    };
    let failed = |at: PathBuf, err: anyhow::Error| Cleaned::Kept {
        reason: LeftBehindReason::RemovalFailed,
        at,
        error: Some(err),
    };
    match store.get_skill_by_id(&m.skill.id) {
        Ok(Some(skill)) if Path::new(&skill.central_path) == m.new => {}
        Ok(_) => return kept(LeftBehindReason::StaleJournal),
        Err(err) => return failed(m.old.clone(), err),
    }
    let identities = (|| -> Result<Option<LeftBehindReason>> {
        if EntryId::of(&m.new)? != Some(m.new_id) {
            return Ok(Some(LeftBehindReason::DestinationMissing));
        }
        if EntryId::of(&m.old)? != Some(m.old_id) {
            return Ok(Some(LeftBehindReason::Replaced));
        }
        Ok(None)
    })();
    match identities {
        Ok(None) => {}
        Ok(Some(reason)) => return kept(reason),
        Err(err) => return failed(m.old.clone(), err),
    }
    let Some(name) = m.old.file_name() else {
        return kept(LeftBehindReason::OutsideLibraryFolder);
    };

    let aside_dir = old_root.join(format!("{SET_ASIDE_PREFIX}{}", uuid::Uuid::new_v4()));
    if let Err(err) = std::fs::create_dir(&aside_dir) {
        return failed(
            m.old.clone(),
            anyhow::Error::new(err).context(format!("create {:?}", aside_dir)),
        );
    }
    let aside = aside_dir.join(name);
    if let Err(err) = std::fs::rename(&m.old, &aside) {
        remove_empty_dir(&aside_dir);
        return failed(
            m.old.clone(),
            anyhow::Error::new(err).context(format!("set {:?} aside", m.old)),
        );
    }
    #[cfg(test)]
    test_seams::after_set_aside();

    let verdict = (|| -> Result<Option<LeftBehindReason>> {
        if EntryId::of(&aside)? != Some(m.old_id) {
            return Ok(Some(LeftBehindReason::Replaced));
        }
        if EntryId::of(&m.new)? != Some(m.new_id) {
            return Ok(Some(LeftBehindReason::DestinationMissing));
        }
        if !same_tree(&aside, &m.new)? {
            return Ok(Some(LeftBehindReason::ChangedSinceCopy));
        }
        // The new copy compared is still the one the record names.
        if EntryId::of(&m.new)? != Some(m.new_id) {
            return Ok(Some(LeftBehindReason::DestinationMissing));
        }
        Ok(None)
    })();
    match verdict {
        Ok(None) => match remove_former_central_copy_unlocked(store, dest, old_root, &aside) {
            Ok(()) => {
                remove_empty_dir(&aside_dir);
                Cleaned::Removed
            }
            Err(err) => failed(put_back(&aside, &aside_dir, &m.old), err),
        },
        Ok(Some(reason)) => Cleaned::Kept {
            reason,
            at: put_back(&aside, &aside_dir, &m.old),
            error: None,
        },
        Err(err) => failed(put_back(&aside, &aside_dir, &m.old), err),
    }
}

/// Rename an old copy set aside at `aside` back to `old` — unless something
/// took that path meanwhile, which is never replaced — and drop the set-aside
/// folder once empty. Answers where the old copy is now.
fn put_back(aside: &Path, aside_dir: &Path, old: &Path) -> PathBuf {
    if !present(aside) {
        remove_empty_dir(aside_dir);
        return old.to_path_buf();
    }
    if present(old) {
        log::warn!(
            "[library move] {:?} was taken while its old copy was set aside; it stays at {:?}",
            old,
            aside
        );
        return aside.to_path_buf();
    }
    match std::fs::rename(aside, old) {
        Ok(()) => {
            remove_empty_dir(aside_dir);
            old.to_path_buf()
        }
        Err(err) => {
            log::warn!(
                "[library move] could not put {:?} back at {:?}: {err}",
                aside,
                old
            );
            aside.to_path_buf()
        }
    }
}

/// Find a journaled old copy a crash left renamed aside — between a clean's
/// rename aside and its removal it sits at
/// `<from>/.skills-hub-clean-<uuid>/<name>`, found by its journaled
/// identity, never by its name alone. Put back at its old path when that is
/// free (`None`: the resume then settles it like any other); answers where
/// it stays when its old path is taken or the rename back fails (never
/// replaced, never removed). `None` too when nothing is set aside: the old
/// copy is at its path, or already cleaned. Only an old copy directly inside
/// the old Library folder `from` is ever set aside, so only that is looked
/// for.
fn locate_old_copy(from: &Path, entry: &JournaledMove) -> Result<Option<PathBuf>> {
    let old = Path::new(&entry.old);
    let Some(name) = old.file_name().filter(|_| in_library_folder(old, from)) else {
        return Ok(None);
    };
    if EntryId::of(old)? == Some(entry.old_id) {
        return Ok(None);
    }
    let entries = match std::fs::read_dir(from) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err).with_context(|| format!("read {:?}", from)),
    };
    let found = entries
        .filter_map(|entry| entry.ok())
        .filter(|dir| {
            dir.file_name()
                .to_string_lossy()
                .starts_with(SET_ASIDE_PREFIX)
        })
        .map(|dir| dir.path().join(name))
        .find(|candidate| EntryId::of(candidate).ok().flatten() == Some(entry.old_id));
    let Some(aside) = found else {
        return Ok(None);
    };
    if present(old) {
        log::warn!(
            "[library move] {:?} is taken; the old copy set aside at {:?} stays there",
            old,
            aside
        );
        return Ok(Some(aside));
    }
    match std::fs::rename(&aside, old) {
        Ok(()) => {
            if let Some(dir) = aside.parent() {
                remove_empty_dir(dir);
            }
            log::info!(
                "[library move] put the old copy set aside at {:?} back",
                aside
            );
            Ok(None)
        }
        Err(err) => {
            log::warn!(
                "[library move] could not put {:?} back at {:?}: {err}",
                aside,
                old
            );
            Ok(Some(aside))
        }
    }
}

/// Remove the set-aside folder `dir` if it is empty. `remove_dir` removes
/// only an empty folder — this undoes the clean's own `mkdir` and can never
/// take content with it, so no fence is involved.
fn remove_empty_dir(dir: &Path) {
    if let Err(err) = std::fs::remove_dir(dir) {
        if present(dir) {
            log::warn!("[library move] left the folder {:?}: {err}", dir);
        }
    }
}

/// Whether `copy` is a folder directly inside the Library folder `root` (by
/// its recorded spelling) — the only old copies the clean may remove.
fn in_library_folder(copy: &Path, root: &Path) -> bool {
    copy.file_name().is_some() && copy.parent() == Some(root)
}

/// Whether anything (a link included) is at `path`.
fn present(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

/// Where every link inside these skills' copies resolves (a link that
/// dangles reaches nothing) — what an old copy's removal must not pull out
/// from under them. `None` when a copy could not be read in full: what it
/// links to is unknown, so every old copy is kept.
fn link_places<'s>(skills: impl Iterator<Item = &'s SkillRecord>) -> Result<Option<Vec<Place>>> {
    let mut places = Vec::new();
    for skill in skills {
        let Ok(root) = Path::new(&skill.central_path).canonicalize() else {
            continue;
        };
        if !root.is_dir() {
            continue;
        }
        for entry in walkdir::WalkDir::new(&root).follow_links(false) {
            let Ok(entry) = entry else {
                log::warn!(
                    "[library move] could not read all of {:?}; every old copy is kept",
                    root
                );
                return Ok(None);
            };
            if entry.depth() > 0 && entry.path_is_symlink() {
                if let Ok(target) = entry.path().canonicalize() {
                    places.push(Place::of(&target)?);
                }
            }
        }
    }
    Ok(Some(places))
}

/// The app's own scratch in a Library folder: neither copied nor reported.
fn is_scratch(name: &str) -> bool {
    name == ".explore-cache" || name.starts_with(".skills-hub-staging-") || name == ".DS_Store"
}

/// Where every recorded Deployment of every skill — any row, any status —
/// physically resolves (a path that resolves to nothing reaches nothing).
fn deployment_places(store: &SkillStore) -> Result<Vec<Place>> {
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut projects = HashMap::new();
    for skill in store.list_skills()? {
        for row in store.list_skill_targets(&skill.id)? {
            paths.push(PathBuf::from(&row.target_path));
        }
        for row in store.list_project_skill_assignments_by_skill(&skill.id)? {
            let project = match projects.get(&row.project_id) {
                Some(project) => project,
                None => {
                    let project = store.get_project_by_id(&row.project_id)?;
                    projects.entry(row.project_id.clone()).or_insert(project)
                }
            };
            let (Some(project), Some(adapter)) = (project, adapter_by_key(&row.tool)) else {
                continue;
            };
            let name = skill.name.clone();
            if let Some(path) =
                resolve_assignment_artifact(Path::new(&project.path), adapter, &row, || {
                    Ok(Some(name))
                })?
            {
                paths.push(path);
            }
        }
    }
    paths
        .iter()
        .filter(|path| path.is_absolute())
        .filter_map(|path| path.canonicalize().ok())
        .map(|resolved| Place::of(&resolved))
        .collect()
}

#[cfg(test)]
#[path = "tests/library_move.rs"]
mod tests;
