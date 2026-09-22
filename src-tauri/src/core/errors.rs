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
        }
    }
}

impl std::error::Error for SignalError {}

use serde::Serialize;
use specta::Type;

use crate::core::global_sync::GlobalSyncError;

/// Why a GitHub clone/fetch failed, classified backend-side from the error
/// chain (the backend has the chain; the frontend owns the copy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
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

/// Structured command failure crossing the IPC seam.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(
    tag = "code",
    rename_all = "SCREAMING_SNAKE_CASE",
    rename_all_fields = "camelCase"
)]
pub enum CommandError {
    ToolNotInstalled {
        tool: String,
    },
    TargetExists {
        path: String,
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
        }
    }
}

impl From<GlobalSyncError> for CommandError {
    fn from(err: GlobalSyncError) -> Self {
        match err {
            GlobalSyncError::ToolNotInstalled { tool_key } => {
                CommandError::ToolNotInstalled { tool: tool_key }
            }
            GlobalSyncError::TargetExists { target_path } => CommandError::TargetExists {
                path: target_path.to_string_lossy().to_string(),
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
