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
//! destination and the Library untouched; after it, old copies the next
//! clean would have removed — duplicates, never a missing skill. A link left
//! pointing at an old copy still resolves, because that copy stays until
//! nothing deployed reaches it.
//!
//! Core never resolves roots: the caller passes `InstallerPaths` (home, the
//! configured central repo, the app cache dir).

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use anyhow::Result;
use serde::Serialize;
use specta::Type;

use super::artifact_removal::{
    remove_former_central_copy_unlocked, remove_staged_library_copy_unlocked, RowRef,
};
use super::content_identity::{self, Comparison, Source};
use super::errors::{CommandError, LibraryDestinationRefusal, SignalError};
use super::git_cache;
use super::installer::InstallerPaths;
use super::mutation_guard;
use super::project_ops;
use super::project_sync::resolve_assignment_artifact;
use super::removal_fence::ProtectedLibrary;
use super::settings;
use super::skill_store::{
    AssignmentTransition, SkillRecord, SkillStore, SkillTargetRecord, TargetTransition,
};
use super::sync_engine::{
    copy_dir_recursive, sync_dir_for_tool_with_overwrite, sync_dir_for_tool_within_root,
    SyncOutcome,
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
    Moved { from: String, to: String },
    Skipped { reason: SkillMoveSkip },
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
    /// The skill whose old copy this is (`still_linked`, `removal_failed`).
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
    /// A moved skill's old copy that a Deployment still reaches.
    StillLinked,
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
        cloud_sync: cloud_sync_of(&paths.home, dest),
    })
}

/// Move the Library (`paths.central_dir`) to `dest`. See the module docs for
/// the phases. A refusal or a copy failure is an error and changes nothing;
/// from the switch on, per-Deployment and per-copy outcomes are report data.
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
        let skills = store.list_skills()?;
        admit(store, paths, &skills, dest, Probe::Write)?;

        let (locatable, missing): (Vec<SkillRecord>, Vec<SkillRecord>) =
            skills.into_iter().partition(is_locatable);
        let moved = copy_phase(store, paths, dest, locatable, &mut on_progress)?;

        let switch: Vec<(String, String)> = moved
            .iter()
            .map(|m| (m.skill.id.clone(), m.new.to_string_lossy().into_owned()))
            .collect();
        if let Err(err) = settings::switch_library(store, &switch, dest, now) {
            rollback(store, paths, dest, moved.iter().map(|m| m.new.as_path()));
            return Err(err);
        }

        let deployments = relink_phase(store, dest, &moved, now, &mut on_progress)?;
        let left_behind = clean_phase(store, paths, dest, &moved, &mut on_progress)?;

        let mut outcomes: Vec<SkillMoveOutcome> = moved
            .iter()
            .map(|m| SkillMoveOutcome {
                skill_id: m.skill.id.clone(),
                name: m.skill.name.clone(),
                status: SkillMoveStatus::Moved {
                    from: m.old.to_string_lossy().into_owned(),
                    to: m.new.to_string_lossy().into_owned(),
                },
            })
            .collect();
        outcomes.extend(missing.into_iter().map(|skill| SkillMoveOutcome {
            skill_id: skill.id,
            name: skill.name,
            status: SkillMoveStatus::Skipped {
                reason: SkillMoveSkip::Missing,
            },
        }));
        Ok(LibraryMoveReport {
            from: paths.central_dir.to_string_lossy().into_owned(),
            to: dest.to_string_lossy().into_owned(),
            skills: outcomes,
            deployments,
            left_behind,
        })
    })
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
        for dir in std::iter::once(skills_dir_in(&paths.home, adapter))
            .chain(former_skills_dirs_in(&paths.home, adapter))
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
            let dir = project_skills_dir_in(root, adapter);
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
        std::fs::create_dir_all(dest)
            .map_err(|_| refuse(LibraryDestinationRefusal::NotWritable, None))?;
        let probe_file = dest.join(format!(".skills-hub-write-probe-{}", uuid::Uuid::new_v4()));
        std::fs::write(&probe_file, b"")
            .map_err(|_| refuse(LibraryDestinationRefusal::NotWritable, None))?;
        // The probe is the file this call just created in an admitted, empty
        // destination; it is no artifact, so no fence is involved.
        std::fs::remove_file(&probe_file)
            .map_err(|_| refuse(LibraryDestinationRefusal::NotWritable, None))?;
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
    old_physical: PathBuf,
    new: PathBuf,
}

fn copy_phase(
    store: &SkillStore,
    paths: &InstallerPaths,
    dest: &Path,
    locatable: Vec<SkillRecord>,
    on_progress: &mut impl FnMut(LibraryMoveProgress),
) -> Result<Vec<Moved>> {
    let total = locatable.len() as u32;
    let mut taken: HashSet<std::ffi::OsString> = HashSet::new();
    let mut moved: Vec<Moved> = Vec::with_capacity(locatable.len());
    for (done, skill) in locatable.into_iter().enumerate() {
        on_progress(LibraryMoveProgress {
            phase: LibraryMovePhase::Copying,
            done: done as u32,
            total,
            name: skill.name.clone(),
        });
        let old = PathBuf::from(&skill.central_path);
        let new = dest.join(unique_name(&old, &skill, &mut taken));
        if let Err(err) = copy_verified(&old, &new) {
            let staged = moved
                .iter()
                .map(|m| m.new.as_path())
                .chain(std::iter::once(new.as_path()));
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
        let old_physical = old.canonicalize().unwrap_or_else(|_| old.clone());
        moved.push(Moved {
            skill,
            old,
            old_physical,
            new,
        });
    }
    Ok(moved)
}

/// The folder name a copy takes in the new Library: its current one, made
/// unique among this move's copies (a split Library may repeat a name).
fn unique_name(
    old: &Path,
    skill: &SkillRecord,
    taken: &mut HashSet<std::ffi::OsString>,
) -> std::ffi::OsString {
    let base = old
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_else(|| skill.id.clone().into());
    let mut candidate = base.clone();
    let mut n = 2;
    while !taken.insert(candidate.clone()) {
        let mut next = base.clone();
        next.push(format!("-{n}"));
        candidate = next;
        n += 1;
    }
    candidate
}

/// Copy `old` to `new` (never `.git`, never an internal link) and prove the
/// bytes arrived: the two must have the same Content identity.
fn copy_verified(old: &Path, new: &Path) -> Result<()> {
    copy_dir_recursive(old, new)?;
    match content_identity::compare(Source::Directory(old), new) {
        Comparison::Same => Ok(()),
        Comparison::Different => anyhow::bail!("the copy's content differs from {:?}", old),
        Comparison::Unknown => anyhow::bail!("the copy's content could not be verified"),
    }
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
// Re-point
// ---------------------------------------------------------------------------

/// What sits at a Deployment's path, against the skill's old copy.
enum Occupant {
    /// A link (a junction counts) that resolves to the old copy.
    LinkToOld,
    /// A real folder: a copy-mode Deployment.
    Copy,
    /// Anything else: nothing there, a link elsewhere, or a real folder
    /// that *is* the old copy through an aliased harness dir.
    NotOurs,
}

fn occupant(path: &Path, old_physical: &Path) -> Result<Occupant> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Occupant::NotOurs),
        Err(err) => return Err(err.into()),
    };
    let reaches_old = path
        .canonicalize()
        .is_ok_and(|resolved| resolved == old_physical);
    Ok(match (meta.file_type().is_symlink(), reaches_old) {
        (true, true) => Occupant::LinkToOld,
        (false, false) => Occupant::Copy,
        _ => Occupant::NotOurs,
    })
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
        match occupant(path, &m.old_physical)? {
            Occupant::Copy => return Ok(None),
            Occupant::NotOurs => anyhow::bail!(NotOurs),
            Occupant::LinkToOld => {}
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
    paths: &InstallerPaths,
    dest: &Path,
    moved: &[Moved],
    on_progress: &mut impl FnMut(LibraryMoveProgress),
) -> Result<Vec<LeftBehind>> {
    let total = moved.len() as u32;
    let mut kept: Vec<LeftBehind> = Vec::new();
    for (done, m) in moved.iter().enumerate() {
        on_progress(LibraryMoveProgress {
            phase: LibraryMovePhase::Cleaning,
            done: done as u32,
            total,
            name: m.skill.name.clone(),
        });
        if still_reached(store, m)? {
            kept.push(LeftBehind {
                path: m.old.to_string_lossy().into_owned(),
                reason: LeftBehindReason::StillLinked,
                skill_id: Some(m.skill.id.clone()),
                error: None,
            });
            continue;
        }
        if let Err(err) =
            remove_former_central_copy_unlocked(store, dest, &paths.central_dir, &m.old)
        {
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
    let old_root = &paths.central_dir;
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

/// Whether any recorded Deployment of the skill — any row, any status —
/// still physically reaches its old copy: removing it would take a
/// harness's skill away.
fn still_reached(store: &SkillStore, m: &Moved) -> Result<bool> {
    let reaches = |path: &Path| {
        path.canonicalize()
            .is_ok_and(|resolved| resolved.starts_with(&m.old_physical))
    };
    for row in store.list_skill_targets(&m.skill.id)? {
        if reaches(Path::new(&row.target_path)) {
            return Ok(true);
        }
    }
    for row in store.list_project_skill_assignments_by_skill(&m.skill.id)? {
        let Some(project) = store.get_project_by_id(&row.project_id)? else {
            continue;
        };
        let Some(adapter) = adapter_by_key(&row.tool) else {
            continue;
        };
        let path = resolve_assignment_artifact(Path::new(&project.path), adapter, &row, || {
            Ok(Some(m.skill.name.clone()))
        })?;
        if path.as_deref().is_some_and(reaches) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
#[path = "tests/library_move.rs"]
mod tests;
