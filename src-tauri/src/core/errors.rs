//! Typed signal errors that core raises through `anyhow` chains.
//!
//! Core keeps `anyhow` for plumbing, but conditions the frontend reacts to
//! specially are raised as `SignalError` values (`bail!(SignalError::Cancelled)`)
//! instead of magic string prefixes. `anyhow` preserves downcastability through
//! `.context(...)` layers, so both internal control flow (e.g. the installer
//! checking for cancellation) and the command seam recover the typed value with
//! `err.downcast_ref::<SignalError>()` — no string sniffing.
//!
//! The wire mapping to the frontend lives in `commands::error::CommandError`.

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
    /// The managed record was deleted but some tool directories could not be
    /// cleaned up. Each entry is `"<path>: <io error>"` diagnostics.
    DeleteCleanupFailed { failures: Vec<String> },
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
            SignalError::DeleteCleanupFailed { failures } => write!(
                f,
                "managed record deleted, but cleanup failed for: {}",
                failures.join(", ")
            ),
        }
    }
}

impl std::error::Error for SignalError {}
