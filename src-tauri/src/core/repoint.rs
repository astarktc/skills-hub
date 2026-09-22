//! Re-point: replace a Managed skill's source in place, keeping its Sync
//! targets and project assignments (see **Re-point** in `CONTEXT.md`).
//!
//! One operation over one target enum, defined for every Managed skill:
//! `git`, `local` and `imported` may each be re-pointed at a GitHub URL or a
//! local folder. (Becoming `imported` is Detach, not a Re-point.) Whatever
//! the target, the new source is validated first and then carried **only**
//! in the acquired Update request: the single-skill Update (the Refresh
//! batch of one, which takes the Mutation guard itself) settles the new
//! source and bytes together through finalize, replaying Edits inside its
//! window (ADR-0004), propagates, and honours the caller's auto-sync
//! reassert policy. A refused target changes nothing and runs no Update; a
//! failed acquisition or finalize is report data and changes nothing either.
//!
//! Provenance on settle (`skill_update::apply_unlocked`): `→ Local` records
//! `local`, the folder, no subpath and no revision; `→ Git` records `git`,
//! the URL as given, and the subpath and revision acquisition resolved, as
//! Add does. Both clear `imported_from_tool` — a skill with an external
//! source is not imported, so it becomes refreshable.

use std::path::Path;

use anyhow::Result;

use super::cancel_token::CancelToken;
use super::environment::expand_home_path_in;
use super::errors::SignalError;
use super::git_acquisition::{parse_full_github_url, GithubApi, HttpGithubApi};
use super::installer::InstallerPaths;
use super::refresh::{
    refresh_managed_skills_with, RefreshPolicy, RefreshProgress, RefreshReport, RefreshSelection,
};
use super::skill_discovery::require_skill_md;
use super::skill_store::SkillStore;
use super::skill_update::{acquire_git_repoint, UpdateRequest};
use super::tool_adapters::tool_holding_path;

/// Where a Re-point sends a skill. Crosses the wire as itself:
/// `{ kind: "git", url }` or `{ kind: "local", path }`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RepointTarget {
    /// A full GitHub repository or skill URL — HTTP, HTTPS or schemeless
    /// `github.com/…`, tree/blob links and a `.git` suffix; not `owner/repo`
    /// shorthand. Stored as given (trimmed).
    Git { url: String },
    /// A skill folder outside every Tool's skills directory, as the operator
    /// typed or picked it; `~` expands against the operator's home.
    Local { path: String },
}

/// Re-point one Managed skill at `target` and Update from it. Only an
/// unknown skill or a refused target fails the call; the Update's outcome is
/// report data, exactly as for Update.
#[allow(clippy::too_many_arguments)]
pub fn repoint_skill_source(
    paths: &InstallerPaths,
    store: &SkillStore,
    skill_id: &str,
    target: RepointTarget,
    policy: RefreshPolicy,
    cancel: Option<&CancelToken>,
    now: i64,
    on_progress: impl FnMut(RefreshProgress),
) -> Result<RefreshReport> {
    let token = super::settings::github_token_or_none(store);
    repoint_skill_source_with(
        paths,
        store,
        skill_id,
        target,
        policy,
        cancel,
        now,
        on_progress,
        &HttpGithubApi::new(token),
    )
}

/// [`repoint_skill_source`] with the GitHub adapter injected, so the git
/// arm is testable without the network.
#[allow(clippy::too_many_arguments)]
pub(crate) fn repoint_skill_source_with(
    paths: &InstallerPaths,
    store: &SkillStore,
    skill_id: &str,
    target: RepointTarget,
    policy: RefreshPolicy,
    cancel: Option<&CancelToken>,
    now: i64,
    on_progress: impl FnMut(RefreshProgress),
    api: &(dyn GithubApi + Sync),
) -> Result<RefreshReport> {
    // No provenance guard: every Managed skill may be re-pointed.
    let record = store.get_skill_by_id(skill_id)?.ok_or_else(|| {
        anyhow::anyhow!(SignalError::NotFound {
            kind: "skill".to_string(),
            id: skill_id.to_string(),
        })
    })?;
    let selection = RefreshSelection::Ids(vec![skill_id.to_string()]);
    match target {
        RepointTarget::Local { path } => {
            let folder = expand_home_path_in(&paths.home, &path)?;
            validate_local_folder(&paths.home, &folder)?;
            refresh_managed_skills_with(
                paths,
                store,
                selection,
                policy,
                cancel,
                now,
                on_progress,
                &|_, _| UpdateRequest::local(record.clone(), &folder, true),
            )
        }
        RepointTarget::Git { url } => {
            let url = url.trim();
            let source = parse_full_github_url(url)?;
            let ttl_ms = super::settings::git_cache_ttl_ms(store);
            refresh_managed_skills_with(
                paths,
                store,
                selection,
                policy,
                cancel,
                now,
                on_progress,
                &|id, cancel| {
                    acquire_git_repoint(paths, store, id, cancel, api, ttl_ms, url, &source)
                },
            )
        }
    }
}

/// A local Re-point target is validated the way Add validates a folder:
/// present, holds a `SKILL.md`, and not inside a Tool's skills directory (a
/// Tool's copy is a Sync target, never a source — taking one over is
/// Onboarding import's door).
fn validate_local_folder(home: &Path, folder: &Path) -> Result<()> {
    if !folder.exists() {
        anyhow::bail!(SignalError::SourcePathMissing {
            path: folder.to_string_lossy().to_string(),
        });
    }
    require_skill_md(folder)?;
    if let Some(holder) = tool_holding_path(home, folder) {
        anyhow::bail!(SignalError::LocalSourceInsideToolDir {
            path: folder.to_string_lossy().to_string(),
            tool: holder.key().to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/repoint.rs"]
mod tests;
