//! Typed signal errors that core raises through `anyhow` chains.
//!
//! Core keeps `anyhow` for plumbing, but conditions the frontend reacts to
//! specially are raised as `SignalError` values (`bail!(SignalError::Cancelled)`)
//! instead of magic string prefixes. `anyhow` preserves downcastability through
//! `.context(...)` layers, so both internal control flow (e.g. the installer
//! checking for cancellation) and the command seam recover the typed value with
//! `err.downcast_ref::<SignalError>()` — no string sniffing.
//!
//! `CommandError` owns wire classification here: report rows classify at
//! settlement, whole-command failures at the command seam.

use std::fmt;

/// A typed condition raised somewhere in core that callers discriminate on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalError {
    /// The user cancelled the operation via the app's cancel token.
    Cancelled,
    /// GitHub API rate limit hit; `reset_minutes` is the rounded-up wait (0 = unknown).
    RateLimited { reset_minutes: i64 },
    /// A path was expected to be an installable skill but is not.
    /// `reason` is a machine token (e.g. `missing_skill_md`) the frontend localizes.
    SkillInvalid { reason: String },
    /// A repository contains multiple skills and no selection was provided.
    MultiSkills,
    /// A skill directory with this name already exists in the central repo.
    SkillExists { name: String },
    /// A skill's name is not one folder name (`placement::is_folder_name`), so
    /// it cannot name its Library copy: refused before anything is acquired
    /// or written.
    SkillNameInvalid { name: String },
    /// Update recovery failed; any backup still holds the previous central bytes.
    FinalizeRollbackFailed {
        central: String,
        backup: Option<String>,
    },
    /// A project with this path is already registered.
    DuplicateProject { path: String },
    /// The project/skill/tool assignment already exists.
    AssignmentExists {
        project: String,
        skill: String,
        tool: String,
    },
    /// An entity referenced by id does not exist. `kind` is e.g. `project`/`skill`.
    NotFound { kind: String, id: String },
    /// A tool key does not match any entry in the tool adapter registry.
    UnknownTool { tool: String },
    /// A filesystem path cannot serve its intended role. `reason` is a machine
    /// token (`missing` / `not_a_directory`) the frontend localizes.
    InvalidPath { path: String, reason: String },
    /// Running the system `git` CLI failed (and the libgit2 fallback is disabled).
    /// `detail` is diagnostic text (error chain + env-var hint), not user copy.
    GitExecFailed { detail: String },
    /// A git CLI operation exceeded the configured timeout.
    /// `detail` is diagnostic text (elapsed seconds, env-var hint, stderr).
    GitTimeout { detail: String },
    /// A GitHub-hosted skill path could not be found (404). `url` is the
    /// human-checkable tree URL the frontend can surface.
    GithubSkillNotFound { url: String },
    /// Re-point requires a full GitHub repository or tree URL.
    InvalidGithubUrl { url: String },
    /// A path a caller asked to delete is not inside any Tool's skills
    /// directory, so Skills Hub refuses to touch it. Owned by the Tool
    /// registry (`tool_adapters::ensure_path_within_tool_dirs`).
    PathOutsideToolDirs { path: String },
    /// A skill's external source folder (a `local` provenance's `source_ref`,
    /// or the folder an Add flow was pointed at) is not there.
    SourcePathMissing { path: String },
    /// A Managed skill's central copy is not there, so nothing can be
    /// updated or moved from it.
    CentralPathMissing { path: String },
    /// Reading or atomically replacing the skill manifest failed.
    SkillManifestIo { path: String, detail: String },
    /// The requested repo-relative subpath is not in the fetched repository.
    /// Carries the subpath the caller asked for, never a cache-internal
    /// absolute path.
    SubpathMissing { subpath: String },
    /// Revealing the app log folder in the file manager failed. `detail` is
    /// diagnostic text (the opener's error chain), not user copy.
    RevealLogFailed { detail: String },
    /// Revealing a path a report named (a target a Delete could not remove)
    /// in the file manager failed or was refused (`reason`). `detail` is the
    /// diagnostic chain, never user copy.
    RevealPathFailed {
        path: String,
        reason: RevealPathFailure,
        detail: String,
    },
    /// An in-repo symlink at `subpath` points at `target`, which is absolute
    /// or resolves to (or above) the repository root, so acquisition refuses
    /// to follow it. Nothing at the target is read. Owned by
    /// `repo_subpath::LinkChain`.
    SymlinkEscapesRepo { subpath: String, target: String },
    /// An upstream symlink chain exceeded acquisition's hop bound at this
    /// repo-relative subpath. Owned by `repo_subpath::LinkChain`.
    SymlinkChainTooDeep { subpath: String },
    /// An Update was asked of a skill that has no external source to
    /// re-acquire from (`imported` provenance: the central copy is its
    /// truth). Owned by `core::provenance::is_refreshable`.
    NotRefreshable { name: String },
    /// Add → local folder was pointed at a folder inside a Tool's global
    /// skills directory. A Tool's copy is not an independent source (the app
    /// would later overwrite or remove it); Onboarding import is how such a
    /// skill is taken over. `tool` is the registry key of the holding Tool.
    /// Owned by the Tool registry (`tool_adapters::tool_holding_path`).
    LocalSourceInsideToolDir { path: String, tool: String },
    /// A stored setting that drives writes (the global Tool selection)
    /// exists but cannot be parsed. Refused rather than defaulted: a default
    /// here would mean "every detected tool". The operator repairs it by
    /// saving the setting again. `key` is the storage key; `detail` is the
    /// parser's diagnostic, not user copy. Owned by `core::settings`.
    SettingCorrupt { key: String, detail: String },
    /// The OS credential store (macOS Keychain, Windows Credential Manager,
    /// Linux Secret Service) refused or could not serve a read/write: a
    /// locked or denied Keychain prompt, no Secret Service running. The
    /// operator can cause and repair it; there is never a plaintext
    /// fallback. `detail` is the store's diagnostic, not user copy. Owned by
    /// `core::credentials`.
    CredentialStoreUnavailable { detail: String },
    /// A copy's content could not be compared with the skill's (either
    /// identity unreadable, or the copy's metadata unreadable), so a step
    /// that needs a known difference — Onboarding import removing a
    /// conflict's other copies — left it in place. `detail` is diagnostic
    /// text, not user copy. Owned by `core::onboarding_import`.
    IdentityUnknown { path: String, detail: String },
    /// A removal (Delete, Disable, unassign, an overwrite's replacement,
    /// Import's original removal, the git cache's clean retry and age-out)
    /// was refused at physical admission: the entry `path` physically names —
    /// its parent resolved through every link or junction, its leaf never —
    /// is `resolved`, which lies in the central library or outside the
    /// directory the removal claims (`reason`) — or, for `ContainsMount`,
    /// `resolved` is the mount found inside the entry. Nothing was touched.
    /// The operator causes it by aliasing a folder on the way (a Tool's skills
    /// dir linked onto the library, a linked cache folder), by configuring the
    /// central repo inside the git cache, or by mounting a folder inside a
    /// skill folder, and repairs it by replacing that link, moving the
    /// library or unmounting. Owned by `core::removal_fence`.
    RemovalRefused {
        path: String,
        resolved: String,
        reason: RemovalRefusal,
    },
    /// The OS refused to remove (or even inspect) `path`: the operator's
    /// account lacks the permission — a folder owned by another user,
    /// read-only, or held open by another process. A retry fails the same way
    /// until that changes. `detail` is the I/O diagnostic chain, not user
    /// copy. Raised by the removal primitive (`sync_engine::remove_path_any`)
    /// as context over the I/O error, which stays in the chain.
    RemovalPermissionDenied { path: String, detail: String },
    /// Move Library folder refused its destination before anything was
    /// written (`reason`); `conflict` names the folder it overlaps (the
    /// current Library, a harness or project skills dir, the git cache) when
    /// there is one. Owned by `core::library_move`.
    LibraryDestinationRefused {
        path: String,
        reason: LibraryDestinationRefusal,
        conflict: Option<String>,
    },
    /// Move Library folder could not copy (or verify by Content identity)
    /// skill `name`'s central copy at `path` into the new folder. Nothing was
    /// switched: the staged copies were removed and every record and the
    /// setting are unchanged. `detail` is the diagnostic chain, not user
    /// copy. Owned by `core::library_move`.
    LibraryCopyFailed {
        name: String,
        path: String,
        detail: String,
    },
    /// A sync write (a first write or a replacement) was refused before
    /// anything was written: the folder it would write into, `path`'s parent
    /// resolved through every link or junction, is `resolved` — inside the
    /// central library, or (for a stored path) outside every skills folder
    /// the row is managed in (`reason`). The operator causes it by aliasing a
    /// harness skills folder onto the Library or elsewhere, or by a record
    /// pointing outside its folder; the redeploy moves a Deployment to its
    /// canonical folder. Owned by `core::sync_engine`.
    DestinationRefused {
        path: String,
        resolved: String,
        reason: DestinationRefusal,
    },
}

impl fmt::Display for SignalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SignalError::Cancelled => write!(f, "operation cancelled by user"),
            SignalError::RateLimited { reset_minutes } => {
                write!(
                    f,
                    "GitHub rate limit reached (resets in ~{reset_minutes} min)"
                )
            }
            SignalError::SkillInvalid { reason } => write!(f, "invalid skill: {reason}"),
            SignalError::MultiSkills => {
                write!(
                    f,
                    "repository contains multiple skills; a selection is required"
                )
            }
            SignalError::SkillExists { name } => {
                write!(f, "skill already installed in central repo: {name}")
            }
            SignalError::SkillNameInvalid { name } => {
                write!(f, "skill name is not one folder name: {name:?}")
            }
            SignalError::FinalizeRollbackFailed { central, backup } => {
                write!(
                    f,
                    "finalize rollback failed: central={central}, backup={backup:?}"
                )
            }
            SignalError::DuplicateProject { path } => {
                write!(f, "project already registered: {path}")
            }
            SignalError::AssignmentExists {
                project,
                skill,
                tool,
            } => write!(f, "assignment already exists: {project}:{skill}:{tool}"),
            SignalError::NotFound { kind, id } => write!(f, "{kind} not found: {id}"),
            SignalError::UnknownTool { tool } => write!(f, "unknown tool: {tool}"),
            SignalError::InvalidPath { path, reason } => {
                write!(f, "invalid path ({reason}): {path}")
            }
            SignalError::GitExecFailed { detail } => {
                write!(f, "git command execution failed: {detail}")
            }
            SignalError::GitTimeout { detail } => write!(f, "git operation timed out: {detail}"),
            SignalError::GithubSkillNotFound { url } => {
                write!(f, "skill not found on GitHub: {url}")
            }
            SignalError::InvalidGithubUrl { url } => write!(f, "invalid GitHub URL: {url}"),
            SignalError::PathOutsideToolDirs { path } => {
                write!(f, "path is not under a known tool skills directory: {path}")
            }
            SignalError::SourcePathMissing { path } => {
                write!(f, "source path is missing: {path}")
            }
            SignalError::CentralPathMissing { path } => {
                write!(f, "central path is missing: {path}")
            }
            SignalError::SkillManifestIo { path, detail } => {
                write!(f, "skill manifest I/O failed: {path}: {detail}")
            }
            SignalError::SubpathMissing { subpath } => {
                write!(f, "subpath is not in the repository: {subpath}")
            }
            SignalError::RevealLogFailed { detail } => {
                write!(f, "revealing the log folder failed: {detail}")
            }
            SignalError::RevealPathFailed {
                path,
                reason,
                detail,
            } => {
                write!(f, "revealing {path} failed ({reason:?}): {detail}")
            }
            SignalError::SymlinkEscapesRepo { subpath, target } => {
                write!(
                    f,
                    "symlink at {subpath} escapes the repository (target: {target})"
                )
            }
            SignalError::SymlinkChainTooDeep { subpath } => {
                write!(
                    f,
                    "symlink chain exceeds the acquisition depth bound at {subpath}"
                )
            }
            SignalError::NotRefreshable { name } => {
                write!(f, "skill has no source to refresh from: {name}")
            }
            SignalError::LocalSourceInsideToolDir { path, tool } => {
                write!(
                    f,
                    "folder is inside the {tool} skills directory (import it instead): {path}"
                )
            }
            SignalError::SettingCorrupt { key, detail } => {
                write!(f, "stored setting {key} is corrupt: {detail}")
            }
            SignalError::CredentialStoreUnavailable { detail } => {
                write!(f, "OS credential store unavailable: {detail}")
            }
            SignalError::IdentityUnknown { path, detail } => {
                write!(f, "content could not be compared: {path}: {detail}")
            }
            SignalError::RemovalRefused {
                path,
                resolved,
                reason,
            } => {
                write!(
                    f,
                    "removal of {path} refused ({reason:?}): it resolves to {resolved}"
                )
            }
            SignalError::RemovalPermissionDenied { path, .. } => {
                write!(f, "not permitted to remove {path}")
            }
            SignalError::LibraryDestinationRefused {
                path,
                reason,
                conflict,
            } => {
                write!(
                    f,
                    "library destination {path} refused ({reason:?}; conflict: {conflict:?})"
                )
            }
            SignalError::LibraryCopyFailed { name, path, detail } => {
                write!(f, "copying skill {name} from {path} failed: {detail}")
            }
            SignalError::DestinationRefused {
                path,
                resolved,
                reason,
            } => {
                write!(
                    f,
                    "write to {path} refused ({reason:?}): its folder resolves to {resolved}"
                )
            }
        }
    }
}

impl std::error::Error for SignalError {}

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::core::global_sync::GlobalSyncError;

/// Why a GitHub clone/fetch failed, classified backend-side from the error
/// chain (the backend has the chain; the frontend owns the copy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum GitCloneFailureKind {
    Tls,
    Auth,
    NotFound,
    Dns,
    Timeout,
    Refused,
    /// Running the system `git` CLI itself failed (fallback disabled).
    ExecFailed,
    Unknown,
}

/// Why Reveal could not show a path (the backend classifies; the frontend
/// owns the copy). Owned by `core::log_reveal::resolve_reveal_item`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum RevealPathFailure {
    /// Neither a Sync target the app recorded nor inside the central repo
    /// (or not a plain absolute path): refused before the filesystem is
    /// touched.
    NotManaged,
    /// Nothing on the way to the path exists (a vanished volume).
    Gone,
    /// The file manager could not show it.
    Opener,
}

/// Why physical admission refused a removal (the backend classifies; the
/// frontend owns the copy). Owned by `core::removal_fence`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum RemovalRefusal {
    /// The entry physically is, lies inside, or contains the central library
    /// (the configured central repo with everything in it, or a recorded
    /// central copy or the folder holding one) — reached through an aliased
    /// folder on the way, or a cache that overlaps the library.
    CentralLibrary,
    /// The entry physically lies outside the directory the removal claims to
    /// act in (an aliased folder on the way leads elsewhere).
    ResolvesElsewhere,
    /// A folder inside the entry is a mount the recursive removal would cross
    /// — a bind mount of the library or of a folder around the entry, another
    /// device, or (Linux) any mount point the kernel lists below the entry;
    /// `resolved` is that folder.
    ContainsMount,
    /// The entry itself is a mount point — a filesystem (a disk image, a
    /// network share, a bind mount) is mounted at it — so removing it would
    /// delete that filesystem's contents; `resolved` is the entry.
    MountPoint,
    /// (Linux) The kernel's mount table (`/proc/self/mountinfo`) could not
    /// be read or parsed, so whether a mount lies at or below the entry is
    /// unknown: admission fails closed. `resolved` is the entry.
    MountTableUnreadable,
}

/// Why a sync write was refused before anything was written (the backend
/// classifies; the frontend owns the copy). Owned by `core::sync_engine`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum DestinationRefusal {
    /// The folder written into physically is, or lies inside, the central
    /// library (an aliased harness skills folder): a Deployment there would
    /// write into the Library itself.
    InLibrary,
    /// The folder written into physically lies outside every skills folder
    /// the row is managed in (a stored path pointing elsewhere, or an aliased
    /// folder on the way leading elsewhere).
    OutsideSkillsFolder,
    /// The place written physically lies inside another live Deployment, or
    /// the folder already there holds one (an aliased folder nests them):
    /// writing or replacing it would change or remove that Deployment.
    /// `resolved` names the Deployment in the way. Raised by the redeploy
    /// (`core::redeploy`).
    OverlapsDeployment,
    /// A folder is at the new place and the redeploy cannot replace it
    /// without a moment in which neither it nor the skill is there: no atomic
    /// exchange here (Windows always, some file systems) while no working old
    /// Deployment of the skill stays in place meanwhile, or no exclusive
    /// rename on this file system. Nothing was touched; the move stays
    /// pending until the operator moves that folder aside. `resolved` is the
    /// new place. Raised by the redeploy (`core::redeploy`).
    CannotReplaceSafely,
}

/// Why Move Library folder refused a destination (the backend classifies;
/// the frontend owns the copy). Owned by `core::library_move`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum LibraryDestinationRefusal {
    /// Not an absolute path (or it climbs with `.` / `..`).
    NotAbsolute,
    /// It is, lies inside, or contains the current Library (the configured
    /// folder or a recorded central copy); `conflict` is that folder.
    OverlapsLibrary,
    /// It lies inside a harness's global skills dir; `conflict` is the dir.
    InsideHarnessDir,
    /// It lies inside a registered project's skills dir; `conflict` is the dir.
    InsideProjectDir,
    /// It lies inside the app's git cache; `conflict` is the cache root.
    InsideGitCache,
    /// It exists and holds something (or is not a folder).
    NotEmpty,
    /// It cannot be created or written to.
    NotWritable,
}

/// Structured command failure crossing the IPC seam. `Deserialize` because a
/// settled failure is also stored as this JSON (`skills.last_outcome_error`,
/// see `core::skill_health`) and read back into the catalog.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(
    tag = "code",
    rename_all = "SCREAMING_SNAKE_CASE",
    rename_all_fields = "camelCase"
)]
pub enum CommandError {
    ToolNotInstalled {
        tool: String,
    },
    /// A sync fan-out skipped this (skill, Tool) pair because the operator
    /// disabled it — at the pair or skill grain (see **Disabled** in
    /// `CONTEXT.md`). A skip reason, never a failure: only Enable lifts it.
    TargetDisabled {
        /// Registry key of the Tool the pair names.
        tool: String,
    },
    /// A sync fan-out skipped this row because its Deployment waits for the
    /// redeploy (round 20 #108): the row owns an artifact at another place
    /// than the one placement gives it now (`path`, as recorded). Writing the
    /// new place here would re-record the row and orphan that artifact; the
    /// redeploy (Settings → Harnesses → Move Deployments) moves it. A skip
    /// reason, never a failure.
    RedeployPending {
        /// The key of the row.
        tool: String,
        /// Where the row records its artifact.
        path: String,
    },
    /// A sync target is occupied by a folder the app does not own. Every
    /// producer's row carries what the one overwrite ask shows (D12): the
    /// path, the folder's modification time and whether its content is
    /// identical to the skill's.
    TargetExists {
        path: String,
        modified_at: Option<i64>,
        same_content: bool,
    },
    ToolNotWritable {
        tool: String,
        path: String,
    },
    SkillInvalid {
        reason: String,
    },
    MultiSkills,
    SkillExists {
        /// Name of the skill directory already present in the central repo.
        name: String,
    },
    /// The name a skill would have is not one folder name — empty, `.` or
    /// `..`, absolute or drive-relative, or holding a separator or NUL — so
    /// its Library copy could land outside the Library. Nothing was acquired
    /// or written.
    SkillNameInvalid {
        name: String,
    },
    FinalizeRollbackFailed {
        central: String,
        backup: Option<String>,
        /// Original failure and rollback error chain, diagnostics only.
        detail: String,
    },
    DuplicateProject {
        path: String,
    },
    AssignmentExists {
        project: String,
        skill: String,
        tool: String,
    },
    NotFound {
        kind: String,
        id: String,
    },
    UnknownTool {
        /// Registry key that matched no tool adapter.
        tool: String,
    },
    InvalidPath {
        path: String,
        /// Machine token (`missing` / `not_a_directory`) the frontend localizes.
        reason: String,
    },
    Cancelled,
    RateLimited {
        /// Rounded-up minutes until the limit resets; 0 = unknown.
        reset_minutes: i64,
    },
    GitCloneFailed {
        kind: GitCloneFailureKind,
        detail: String,
    },
    GithubSkillNotFound {
        /// Human-checkable GitHub tree URL for the missing skill path.
        url: String,
    },
    InvalidGithubUrl {
        url: String,
    },
    PathOutsideToolDirs {
        /// The refused path (not inside any Tool's skills directory).
        path: String,
    },
    SkillManifestIo {
        path: String,
        detail: String,
    },
    SourcePathMissing {
        /// The external source folder that is not there.
        path: String,
    },
    CentralPathMissing {
        /// The Managed skill's central copy that is not there.
        path: String,
    },
    SubpathMissing {
        /// The requested repo-relative subpath (never a cache-internal path).
        subpath: String,
    },
    RevealLogFailed {
        /// Opener error chain, diagnostics only.
        detail: String,
    },
    RevealPathFailed {
        /// The path the operator asked to see.
        path: String,
        /// Refused (not a path the app manages), gone, or the opener failed.
        reason: RevealPathFailure,
        /// Diagnostic chain, diagnostics only.
        detail: String,
    },
    SymlinkEscapesRepo {
        /// Repo-relative path of the symlink that was refused.
        subpath: String,
        /// The link's raw target (absolute, or climbing out of the repository).
        target: String,
    },
    SymlinkChainTooDeep {
        /// Repo-relative path where the upstream chain exceeded the hop bound.
        subpath: String,
    },
    NotRefreshable {
        /// The Managed skill that has no external source (imported provenance).
        name: String,
    },
    LocalSourceInsideToolDir {
        /// The refused folder (inside a Tool's global skills directory).
        path: String,
        /// Registry key of the Tool whose skills directory holds it.
        tool: String,
    },
    SettingCorrupt {
        /// Storage key of the setting that could not be parsed.
        key: String,
        /// Parser diagnostic, not user copy.
        detail: String,
    },
    CredentialStoreUnavailable {
        /// The credential store's diagnostic, not user copy.
        detail: String,
    },
    /// A copy's content could not be compared with the skill's, so it was
    /// left in place (never removed on an unknown comparison).
    IdentityUnknown {
        /// The copy that was left in place.
        path: String,
        /// Which read failed, diagnostics only.
        detail: String,
    },
    /// A removal was refused at physical admission: `path` physically names
    /// `resolved`, which is in the central library or outside the directory
    /// the removal claims. Nothing was touched.
    RemovalRefused {
        /// The path the removal was asked to act on.
        path: String,
        /// Where that path physically leads (parent resolved, leaf not).
        resolved: String,
        /// In the central library, or outside the claimed directory.
        reason: RemovalRefusal,
    },
    /// The OS refused to remove `path` for lack of permission (ownership,
    /// read-only, a process holding it); a retry fails the same way until
    /// that is fixed.
    RemovalPermissionDenied {
        /// The path that could not be removed.
        path: String,
        /// The I/O diagnostic chain, diagnostics only.
        detail: String,
    },
    /// Move Library folder refused its destination before anything was
    /// written.
    LibraryDestinationRefused {
        /// The destination the operator picked.
        path: String,
        reason: LibraryDestinationRefusal,
        /// The folder it overlaps, when there is one.
        conflict: Option<String>,
    },
    /// Move Library folder could not copy or verify one skill; nothing was
    /// switched and the Library is as it was.
    LibraryCopyFailed {
        /// The skill whose copy failed.
        name: String,
        /// Its central copy.
        path: String,
        /// The diagnostic chain, diagnostics only.
        detail: String,
    },
    /// A sync write was refused before anything was written: `path`'s folder
    /// physically resolves to `resolved`, inside the central library or
    /// outside every skills folder the row is managed in.
    DestinationRefused {
        /// The path the write was asked to create.
        path: String,
        /// Where its folder physically leads.
        resolved: String,
        reason: DestinationRefusal,
    },
    /// An Update replayed the operator's invocation Edit onto new upstream
    /// bytes whose own invocation mode disagrees with both the Edit's base
    /// and its override: the Edit was kept and needs review. Settled by
    /// `skill_update::apply_unlocked` as the skill's `edit_conflict` health.
    EditConflict {
        base_mode: crate::core::manifest::InvocationMode,
        upstream_mode: crate::core::manifest::InvocationMode,
        override_mode: crate::core::manifest::InvocationMode,
    },
    Other {
        message: String,
    },
}

impl CommandError {
    /// Wrap an infrastructure failure (e.g. a task join error) as `Other`.
    pub fn internal(err: impl std::fmt::Display) -> Self {
        CommandError::Other {
            message: err.to_string(),
        }
    }

    /// Classify an `anyhow` chain into the wire contract. This is the single
    /// classifier used at report-row settlement and the command seam: typed `SignalError`s
    /// are recovered by downcast, GitHub clone failures are classified by
    /// heuristic, and everything else becomes `Other` with the full chain.
    pub fn from_anyhow(err: anyhow::Error) -> Self {
        let rollback_detail = matches!(
            err.downcast_ref::<SignalError>(),
            Some(SignalError::FinalizeRollbackFailed { .. })
        )
        .then(|| format!("{err:#}"));
        let err = match err.downcast::<SignalError>() {
            Ok(signal) => {
                let mut command = CommandError::from(signal);
                if let CommandError::FinalizeRollbackFailed { detail, .. } = &mut command {
                    *detail = rollback_detail.unwrap_or_default();
                }
                return command;
            }
            Err(err) => err,
        };
        let err = match err.downcast::<GlobalSyncError>() {
            Ok(sync_err) => return CommandError::from(sync_err),
            Err(err) => err,
        };
        // `sync_engine`'s typed collision, wherever it sits in the chain: a
        // project first sync / resync / Enable meeting a folder in the way is
        // the same `TARGET_EXISTS` the global batch reports.
        if let Some(exists) = err
            .chain()
            .find_map(|cause| cause.downcast_ref::<crate::core::sync_engine::TargetExistsError>())
        {
            return CommandError::TargetExists {
                path: exists.target.to_string_lossy().into_owned(),
                modified_at: exists.occupant.modified_at,
                same_content: exists.occupant.same_content,
            };
        }

        let full = format!("{:#}", err);
        let root = err.root_cause().to_string();
        let lower = full.to_lowercase();

        // GitHub clone/fetch failures: classify so the frontend can show a
        // localized hint instead of a raw git error chain.
        if lower.contains("github.com")
            && (lower.contains("clone ") || lower.contains("remote") || lower.contains("fetch"))
        {
            let kind = if lower.contains("securetransport") {
                GitCloneFailureKind::Tls
            } else if lower.contains("authentication")
                || lower.contains("permission denied")
                || lower.contains("credentials")
            {
                GitCloneFailureKind::Auth
            } else if lower.contains("not found") {
                GitCloneFailureKind::NotFound
            } else if lower.contains("failed to resolve")
                || lower.contains("could not resolve")
                || lower.contains("dns")
            {
                GitCloneFailureKind::Dns
            } else if lower.contains("timed out") || lower.contains("timeout") {
                GitCloneFailureKind::Timeout
            } else if lower.contains("connection refused") || lower.contains("connection reset") {
                GitCloneFailureKind::Refused
            } else {
                GitCloneFailureKind::Unknown
            };
            return CommandError::GitCloneFailed { kind, detail: root };
        }

        // Redact noisy temp paths from clone context (we care about the cause,
        // not the destination). Example head line:
        // `clone https://... into "/Users/.../skills-hub-git-<uuid>"`
        let mut message = full;
        if let Some(head) = message.lines().next() {
            if head.starts_with("clone ") {
                if let Some(pos) = head.find(" into ") {
                    let head_redacted = head[..pos].to_string();
                    let rest: String = message.lines().skip(1).collect::<Vec<_>>().join("\n");
                    message = if rest.is_empty() {
                        head_redacted
                    } else {
                        format!("{}\n{}", head_redacted, rest)
                    };
                }
            }
        }

        CommandError::Other { message }
    }
}

impl From<SignalError> for CommandError {
    fn from(signal: SignalError) -> Self {
        match signal {
            SignalError::Cancelled => CommandError::Cancelled,
            SignalError::RateLimited { reset_minutes } => {
                CommandError::RateLimited { reset_minutes }
            }
            SignalError::SkillInvalid { reason } => CommandError::SkillInvalid { reason },
            SignalError::MultiSkills => CommandError::MultiSkills,
            SignalError::SkillExists { name } => CommandError::SkillExists { name },
            SignalError::SkillNameInvalid { name } => CommandError::SkillNameInvalid { name },
            SignalError::FinalizeRollbackFailed { central, backup } => {
                CommandError::FinalizeRollbackFailed {
                    central,
                    backup,
                    detail: String::new(),
                }
            }
            SignalError::DuplicateProject { path } => CommandError::DuplicateProject { path },
            SignalError::AssignmentExists {
                project,
                skill,
                tool,
            } => CommandError::AssignmentExists {
                project,
                skill,
                tool,
            },
            SignalError::NotFound { kind, id } => CommandError::NotFound { kind, id },
            SignalError::UnknownTool { tool } => CommandError::UnknownTool { tool },
            SignalError::InvalidPath { path, reason } => CommandError::InvalidPath { path, reason },
            SignalError::GitExecFailed { detail } => CommandError::GitCloneFailed {
                kind: GitCloneFailureKind::ExecFailed,
                detail,
            },
            SignalError::GitTimeout { detail } => CommandError::GitCloneFailed {
                kind: GitCloneFailureKind::Timeout,
                detail,
            },
            SignalError::GithubSkillNotFound { url } => CommandError::GithubSkillNotFound { url },
            SignalError::InvalidGithubUrl { url } => CommandError::InvalidGithubUrl { url },
            SignalError::PathOutsideToolDirs { path } => CommandError::PathOutsideToolDirs { path },
            SignalError::SkillManifestIo { path, detail } => {
                CommandError::SkillManifestIo { path, detail }
            }
            SignalError::SourcePathMissing { path } => CommandError::SourcePathMissing { path },
            SignalError::CentralPathMissing { path } => CommandError::CentralPathMissing { path },
            SignalError::SubpathMissing { subpath } => CommandError::SubpathMissing { subpath },
            SignalError::RevealLogFailed { detail } => CommandError::RevealLogFailed { detail },
            SignalError::RevealPathFailed {
                path,
                reason,
                detail,
            } => CommandError::RevealPathFailed {
                path,
                reason,
                detail,
            },
            SignalError::SymlinkEscapesRepo { subpath, target } => {
                CommandError::SymlinkEscapesRepo { subpath, target }
            }
            SignalError::SymlinkChainTooDeep { subpath } => {
                CommandError::SymlinkChainTooDeep { subpath }
            }
            SignalError::NotRefreshable { name } => CommandError::NotRefreshable { name },
            SignalError::LocalSourceInsideToolDir { path, tool } => {
                CommandError::LocalSourceInsideToolDir { path, tool }
            }
            SignalError::SettingCorrupt { key, detail } => {
                CommandError::SettingCorrupt { key, detail }
            }
            SignalError::CredentialStoreUnavailable { detail } => {
                CommandError::CredentialStoreUnavailable { detail }
            }
            SignalError::IdentityUnknown { path, detail } => {
                CommandError::IdentityUnknown { path, detail }
            }
            SignalError::RemovalRefused {
                path,
                resolved,
                reason,
            } => CommandError::RemovalRefused {
                path,
                resolved,
                reason,
            },
            SignalError::RemovalPermissionDenied { path, detail } => {
                CommandError::RemovalPermissionDenied { path, detail }
            }
            SignalError::LibraryDestinationRefused {
                path,
                reason,
                conflict,
            } => CommandError::LibraryDestinationRefused {
                path,
                reason,
                conflict,
            },
            SignalError::LibraryCopyFailed { name, path, detail } => {
                CommandError::LibraryCopyFailed { name, path, detail }
            }
            SignalError::DestinationRefused {
                path,
                resolved,
                reason,
            } => CommandError::DestinationRefused {
                path,
                resolved,
                reason,
            },
        }
    }
}

impl From<GlobalSyncError> for CommandError {
    fn from(err: GlobalSyncError) -> Self {
        match err {
            GlobalSyncError::ToolNotInstalled { tool_key } => {
                CommandError::ToolNotInstalled { tool: tool_key }
            }
            GlobalSyncError::TargetDisabled { tool_key } => {
                CommandError::TargetDisabled { tool: tool_key }
            }
            GlobalSyncError::RedeployPending { tool_key, path } => CommandError::RedeployPending {
                tool: tool_key,
                path: path.to_string_lossy().into_owned(),
            },
            GlobalSyncError::TargetExists {
                target_path,
                occupant,
            } => CommandError::TargetExists {
                path: target_path.to_string_lossy().to_string(),
                modified_at: occupant.modified_at,
                same_content: occupant.same_content,
            },
            GlobalSyncError::ToolNotWritable {
                tool_display_name,
                skills_dir,
            } => CommandError::ToolNotWritable {
                tool: tool_display_name,
                path: skills_dir.to_string_lossy().to_string(),
            },
            GlobalSyncError::Other(err) => CommandError::from_anyhow(err),
        }
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Debug-ish display for logs; user-facing copy lives frontend-side.
        match serde_json::to_string(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(f, "{:?}", self),
        }
    }
}
