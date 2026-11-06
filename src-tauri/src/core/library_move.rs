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
//!   1. **Copy** every recorded, locatable skill into the destination and
//!      verify each by Content identity. A failure removes what was staged
//!      and changes no record (`LIBRARY_COPY_FAILED`). Scratch in the old
//!      root (`.explore-cache`, `.skills-hub-staging-*`) and anything
//!      unrecorded never travel.
//!   2. **Switch** every moved skill's `central_path` and the Library folder
//!      setting in one DB transaction (`settings::switch_library`).
//!   3. **Re-point** every owned (`synced_at` set) Deployment — global target
//!      rows and project assignments — whose link resolves to the skill's
//!      old copy, through `sync_engine`'s overwrite path. A copy is left
//!      `unchanged`, a link elsewhere (or nothing at the path, or a folder
//!      that is the old copy through an alias) is `not_ours` and untouched,
//!      a project whose folder is gone is `folder_missing`, a disabled row
//!      is `disabled`; a failure is report data and settles its rows `error`.
//!   4. **Clean** each moved skill's old copy through `artifact_removal` and
//!      the removal fence, claimed to its recorded folder — unless something
//!      deployed still reaches it (a failed re-point, an aliased harness
//!      dir), which keeps it. The old root and everything unrecorded stay
//!      and are listed in the report's `left_behind`.
//!
//! An Unlocatable skill (its central copy gone) is skipped and reported; its
//! record keeps its old path.
//!
//! Crash safety: a crash before the switch leaves orphan copies in the
//! destination and the Library untouched. The switch's own transaction also
//! records a **journal** — old and new copy per moved skill, both Library
//! folders — so a crash after it is finished, never forgotten:
//! [`resume_library_move`] (run at startup, and first thing by every
//! [`move_library`]) re-runs Re-point and Clean from the journal — a link
//! reaching a journaled old copy is ours, one already re-pointed is re-made
//! into the same new copy, an old copy already removed is absent — and drops
//! the journal once both settle. Until then a link left pointing at an old
//! copy still resolves: duplicates, never a missing skill.
//!
//! Core never resolves roots: the caller passes `InstallerPaths` (home, the
//! configured central repo, the app cache dir).

use std::collections::{HashMap, HashSet};
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
use super::removal_fence::{Place, ProtectedLibrary};
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
    /// The skill whose old copy this is (`still_linked`, `still_recorded`,
    /// `removal_failed`).
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
}

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

/// The confirmation's facts for moving the Library to `dest`. Read-only:
/// refuses exactly as [`move_library`] would, except "not writable", which
/// only the move's own probe can prove.
pub fn preview_library_move(
    store: &SkillStore,
    paths: &InstallerPaths,
    dest: &Path,
) -> Result<LibraryMovePreview> {
    let skills = store.list_skills()?;
    admit(store, paths, &skills, dest, Probe::ReadOnly)?;
    let (locatable, missing): (Vec<_>, Vec<_>) =
        skills.iter().partition(|skill| is_locatable(skill));
    Ok(LibraryMovePreview {
        destination: dest.to_string_lossy().into_owned(),
        skills: locatable.len() as u32,
        missing: missing.len() as u32,
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
        if let Some(resumed) = resume_unlocked(store, now, &mut on_progress)? {
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

/// Finish a Library move a crash interrupted after its switch: Re-point and
/// Clean every skill its journal names (idempotent — a link already
/// re-pointed is re-made into the same copy, an old copy already removed is
/// absent), then drop the journal. `None` when no move is pending. Run once
/// at startup, before anything else can mutate; [`move_library`] also runs
/// it first.
///
/// Mutation entry point: serialised against every other Sync-target
/// mutation.
pub fn resume_library_move(store: &SkillStore, now: i64) -> Result<Option<LibraryMoveReport>> {
    mutation_guard::serialized(|| resume_unlocked(store, now, &mut |_| {}))
}

/// What a switched move still owes: the Library folders on either side and,
/// per moved skill, its old and new copy. Written in the switch's own
/// transaction (`settings::switch_library`), dropped once Re-point and Clean
/// settle. The Re-point treats a link reaching a journaled old copy as ours.
#[derive(Debug, Serialize, serde::Deserialize)]
struct Journal {
    from: String,
    to: String,
    moves: Vec<JournaledMove>,
}

#[derive(Debug, Serialize, serde::Deserialize)]
struct JournaledMove {
    skill_id: String,
    old: String,
    new: String,
}

fn resume_unlocked(
    store: &SkillStore,
    now: i64,
    on_progress: &mut impl FnMut(LibraryMoveProgress),
) -> Result<Option<LibraryMoveReport>> {
    let Some(raw) = settings::pending_library_move(store)? else {
        return Ok(None);
    };
    let journal: Journal = serde_json::from_str(&raw)
        .with_context(|| "read the journal of an interrupted Library move")?;
    let mut moved = Vec::with_capacity(journal.moves.len());
    for entry in journal.moves {
        // A record the journal names is still the one the switch wrote: no
        // mutation runs before the resume. Anything else is not ours to
        // re-point.
        let Some(mut skill) = store.get_skill_by_id(&entry.skill_id)? else {
            continue;
        };
        if skill.central_path != entry.new {
            continue;
        }
        skill.central_path = entry.old.clone();
        let old = PathBuf::from(&entry.old);
        moved.push(Moved {
            old_place: Place::of(&old)?,
            skill,
            old,
            new: PathBuf::from(&entry.new),
        });
    }
    let (from, to) = (PathBuf::from(&journal.from), PathBuf::from(&journal.to));
    settle(store, &from, &to, &moved, now, on_progress).map(Some)
}

/// After the switch: Re-point, Clean, then drop the journal. Answers the
/// report for `moved` (the caller adds the skills that did not move).
fn settle(
    store: &SkillStore,
    old_root: &Path,
    new_root: &Path,
    moved: &[Moved],
    now: i64,
    on_progress: &mut impl FnMut(LibraryMoveProgress),
) -> Result<LibraryMoveReport> {
    let deployments = relink_phase(store, new_root, moved, now, on_progress)?;
    let left_behind = clean_phase(store, old_root, new_root, moved, on_progress)?;
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
/// physically is, for the re-point and clean checks), its new copy.
struct Moved {
    skill: SkillRecord,
    old: PathBuf,
    old_place: Place,
    new: PathBuf,
}

/// What the copy phase settled: the skills it copied and verified, and the
/// locatable ones it keeps where they are.
struct Copied {
    moved: Vec<Moved>,
    kept: Vec<(SkillRecord, SkillKept)>,
}

fn copy_phase(
    store: &SkillStore,
    paths: &InstallerPaths,
    dest: &Path,
    locatable: Vec<SkillRecord>,
    on_progress: &mut impl FnMut(LibraryMoveProgress),
) -> Result<Copied> {
    let total = locatable.len() as u32;
    // The old Library, physically: its root and every copy about to move.
    // An absolute link into any of them would not follow the move.
    let mut old_library = vec![Place::of(&paths.central_dir)?];
    let mut places = Vec::with_capacity(locatable.len());
    for skill in &locatable {
        old_library.push(Place::of(Path::new(&skill.central_path))?);
        places.push(Place::of(Path::new(&skill.central_path))?);
    }
    // Two records of one physical copy (or of copies one inside the other)
    // alias one set of bytes: copying it twice would split one skill into
    // two, so neither moves.
    let shared_with: Vec<Option<String>> = (0..places.len())
        .map(|i| {
            (0..places.len())
                .find(|&j| j != i && places[i].overlaps(&places[j]))
                .map(|j| locatable[j].id.clone())
        })
        .collect();

    let mut copied = Copied {
        moved: Vec::with_capacity(locatable.len()),
        kept: Vec::new(),
    };
    let skills = locatable.into_iter().zip(places).zip(shared_with);
    for (done, ((skill, old_place), shared_with)) in skills.enumerate() {
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
        let attempt = (|| -> Result<Option<SkillKept>> {
            if let Some(with_skill_id) = shared_with {
                return Ok(Some(SkillKept::SharedCopy { with_skill_id }));
            }
            if let Some((entry, cause)) =
                unrelocatable(&old, &old_place, &paths.central_dir, &old_library)?
            {
                return Ok(Some(SkillKept::Unrelocatable {
                    entry: entry.to_string_lossy().into_owned(),
                    cause,
                }));
            }
            let new = reserved.insert(reserve(dest, &old, &skill)?);
            copy_faithfully(&old, new)?;
            verify_tree(&old, new)?;
            Ok(None)
        })();
        match attempt {
            Ok(Some(reason)) => copied.kept.push((skill, reason)),
            Ok(None) => copied.moved.push(Moved {
                skill,
                old,
                old_place,
                new: reserved.expect("a copied skill reserved its folder"),
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
// entry, dotfiles and `.git` included, recreates each link as a link with
// its text verbatim (never followed), and verifies the result as a whole
// tree: the same entries, the same kinds, the same file bytes (and, on Unix,
// modes) and the same link texts.

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
///
/// Every other link (relative inside the skill, absolute elsewhere, dangling
/// either way) reads the same from the new place.
fn unrelocatable(
    old: &Path,
    old_place: &Place,
    old_root: &Path,
    old_library: &[Place],
) -> Result<Option<(PathBuf, UnrelocatableCause)>> {
    let mut pending: Vec<(PathBuf, Vec<std::ffi::OsString>)> = vec![(old.to_path_buf(), vec![])];
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
            } else if !kind.is_file() {
                return Ok(Some((path, UnrelocatableCause::SpecialFile)));
            }
        }
    }
    Ok(None)
}

/// Whether the relative link text `text`, read from a folder `depth` levels
/// (by name) below the skill's root, climbs out of the skill.
fn escapes(depth: &[std::ffi::OsString], text: &Path) -> bool {
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

/// Copy every entry of `src` into the empty folder `dst`: folders, files
/// (bytes and permissions), and links recreated with their text verbatim.
fn copy_faithfully(src: &Path, dst: &Path) -> Result<()> {
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
        } else if kind.is_dir() {
            std::fs::create_dir(&to).with_context(|| format!("create {:?}", to))?;
            copy_faithfully(&from, &to)?;
        } else if kind.is_file() {
            std::fs::copy(&from, &to).with_context(|| format!("copy {:?} -> {:?}", from, to))?;
        } else {
            anyhow::bail!("{:?} is not a file, folder or link", from);
        }
    }
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

/// Prove `dst` reproduces `src` as a whole tree: the same entry names, each
/// of the same kind, files with the same bytes (and mode, on Unix), links
/// with the same text.
fn verify_tree(src: &Path, dst: &Path) -> Result<()> {
    let listing = |dir: &Path| -> Result<Vec<(std::ffi::OsString, std::fs::FileType)>> {
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
    let names = |list: &[(std::ffi::OsString, std::fs::FileType)]| -> Vec<std::ffi::OsString> {
        list.iter().map(|(name, _)| name.clone()).collect()
    };
    if names(&want) != names(&got) {
        anyhow::bail!("the copy of {:?} does not hold the same entries", src);
    }
    for ((name, want_kind), (_, got_kind)) in want.iter().zip(&got) {
        let (from, to) = (src.join(name), dst.join(name));
        let differs = || anyhow::anyhow!("the copy of {:?} differs", from);
        if want_kind.is_symlink() {
            if !got_kind.is_symlink() || std::fs::read_link(&from)? != std::fs::read_link(&to)? {
                return Err(differs());
            }
        } else if want_kind.is_dir() {
            if !got_kind.is_dir() {
                return Err(differs());
            }
            verify_tree(&from, &to)?;
        } else {
            if !got_kind.is_file() || !same_file_bytes(&from, &to)? {
                return Err(differs());
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = |p: &Path| std::fs::metadata(p).map(|m| m.permissions().mode());
                if mode(&from)? != mode(&to)? {
                    return Err(differs());
                }
            }
        }
    }
    Ok(())
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

fn clean_phase(
    store: &SkillStore,
    old_root: &Path,
    dest: &Path,
    moved: &[Moved],
    on_progress: &mut impl FnMut(LibraryMoveProgress),
) -> Result<Vec<LeftBehind>> {
    // Retention is decided over physical copies, never per skill id: an old
    // copy goes only when no recorded copy of any skill (one kept in place,
    // an Unlocatable one inside it) overlaps it and no Deployment of any
    // skill reaches into it.
    let recorded: Vec<Place> = store
        .list_skills()?
        .iter()
        .filter(|skill| Path::new(&skill.central_path).is_absolute())
        .map(|skill| Place::of(Path::new(&skill.central_path)))
        .collect::<Result<_>>()?;
    let reached = deployment_places(store)?;
    let total = moved.len() as u32;
    let mut kept: Vec<LeftBehind> = Vec::new();
    for (done, m) in moved.iter().enumerate() {
        on_progress(LibraryMoveProgress {
            phase: LibraryMovePhase::Cleaning,
            done: done as u32,
            total,
            name: m.skill.name.clone(),
        });
        let retained = if recorded.iter().any(|copy| copy.overlaps(&m.old_place)) {
            Some(LeftBehindReason::StillRecorded)
        } else if reached.iter().any(|at| at.within(&m.old_place)) {
            Some(LeftBehindReason::StillLinked)
        } else {
            None
        };
        if let Some(reason) = retained {
            kept.push(LeftBehind {
                path: m.old.to_string_lossy().into_owned(),
                reason,
                skill_id: Some(m.skill.id.clone()),
                error: None,
            });
            continue;
        }
        if let Err(err) = remove_former_central_copy_unlocked(store, dest, old_root, &m.old) {
            log::warn!("[library move] old copy {:?} kept: {err:#}", m.old);
            kept.push(LeftBehind {
                path: m.old.to_string_lossy().into_owned(),
                reason: LeftBehindReason::RemovalFailed,
                skill_id: Some(m.skill.id.clone()),
                error: Some(CommandError::from_anyhow(err)),
            });
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
                    .filter(|path| !kept_paths.contains(path) && !recorded.contains(path))
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
