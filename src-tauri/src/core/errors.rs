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
            SignalError::DuplicateProject { path } => {
                write!(f, "project already registered: {path}")
            }
            SignalError::AssignmentExists {
                project,
                skill,
                tool,
            } => write!(f, "assignment already exists: {project}:{skill}:{tool}"),
            SignalError::NotFound { kind, id } => write!(f, "{kind} not found: {id}"),
        }
    }
}

impl std::error::Error for SignalError {}
