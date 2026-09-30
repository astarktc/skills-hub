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
//!
//! [`preview_repoint_source`] answers what a Re-point would land — the same
//! validation and acquisition intent, nothing written — so the operator sees
//! the resolved skill (and a name mismatch, or a multi-skill repository's
//! candidates) before committing to it.

use std::path::Path;

use anyhow::Result;

use super::cancel_token::CancelToken;
use super::credentials::CredentialStore;
use super::environment::expand_home_path_in;
use super::errors::SignalError;
use super::git_acquisition::{
    acquire, parse_full_github_url, AcquireRequest, GitSelection, GitSource, GitSourceResolution,
    GithubApi, HttpGithubApi, SkillIntent,
};
use super::git_cache::{fetch_through_cache, FetchRequest};
use super::install_finalize::StagingDir;
use super::installer::{
    ensure_installable_skill_dir, list_git_skills_with, GitSkillCandidate, InstallerPaths,
};
use super::manifest::{find_skill_md, parse_skill_md};
use super::refresh::{
    refresh_with_intent, RefreshPolicy, RefreshProgress, RefreshReport, RefreshSelection,
    UpdateIntent,
};
use super::skill_discovery::require_skill_md;
use super::skill_matching::dir_name_of;
use super::skill_store::{SkillRecord, SkillStore};
use super::skill_update::{acquire_git_repoint, UpdateRequest};
use super::tool_adapters::tool_holding_path;

/// Where a Re-point sends a skill. Crosses the wire as itself:
/// `{ kind: "git", url, selection? }` or `{ kind: "local", path }`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RepointTarget {
    /// A full GitHub repository or skill URL — HTTP, HTTPS or schemeless
    /// `github.com/…`, tree/blob links and a `.git` suffix; not `owner/repo`
    /// shorthand. Stored as given (trimmed). `selection` names the candidate
    /// the operator picked when the URL is a multi-skill repository.
    Git {
        url: String,
        #[serde(default)]
        selection: Option<RepointSelection>,
    },
    /// A skill folder outside every Tool's skills directory, as the operator
    /// typed or picked it; `~` expands against the operator's home.
    Local { path: String },
}

/// A candidate picked from a multi-skill repository's listing (a preview's
/// `candidates`): its repo-relative folder and the listing's branch decision,
/// a deliberate default branch included. A URL cannot carry the latter — a
/// repository cloned on its default branch has no branch name to put in a
/// `/tree/<ref>/` link — so the choice crosses the wire as itself, exactly
/// as Add's install batch carries it (`GitSelection`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, specta::Type)]
pub struct RepointSelection {
    pub subpath: String,
    pub resolution: Option<GitSourceResolution>,
}

impl RepointSelection {
    fn as_git_selection(&self) -> GitSelection<'_> {
        GitSelection {
            subpath: &self.subpath,
            resolution: self.resolution.as_ref(),
        }
    }
}

/// Re-point one Managed skill at `target` and Update from it. Only an
/// unknown skill or a refused target fails the call; the Update's outcome is
/// report data, exactly as for Update.
#[allow(clippy::too_many_arguments)]
pub fn repoint_skill_source(
    paths: &InstallerPaths,
    store: &SkillStore,
    credentials: &dyn CredentialStore,
    skill_id: &str,
    target: RepointTarget,
    policy: RefreshPolicy,
    cancel: Option<&CancelToken>,
    now: i64,
    on_progress: impl FnMut(RefreshProgress),
) -> Result<RefreshReport> {
    let token = super::settings::github_token_or_none(credentials);
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
    let record = require_record(store, skill_id)?;
    let selection = RefreshSelection::Ids(vec![skill_id.to_string()]);
    match target {
        RepointTarget::Local { path } => {
            let folder = expand_home_path_in(&paths.home, &path)?;
            validate_local_folder(&paths.home, &folder)?;
            refresh_with_intent(
                paths,
                store,
                selection,
                policy,
                cancel,
                now,
                on_progress,
                &|_, _| UpdateRequest::local(record.clone(), &folder, true),
                UpdateIntent::Repoint,
            )
        }
        RepointTarget::Git {
            url,
            selection: pick,
        } => {
            let url = url.trim();
            let source = parse_full_github_url(url)?;
            let ttl_ms = super::settings::git_cache_ttl_ms(store);
            let pick = pick.as_ref().map(RepointSelection::as_git_selection);
            refresh_with_intent(
                paths,
                store,
                selection,
                policy,
                cancel,
                now,
                on_progress,
                &|id, cancel| {
                    acquire_git_repoint(paths, store, id, cancel, api, ttl_ms, url, &source, pick)
                },
                UpdateIntent::Repoint,
            )
        }
    }
}

/// What a Re-point at a target would land, answered without landing it
/// (spec D15): the frontend shows it — and asks on a name mismatch — before
/// the real [`repoint_skill_source`] runs.
#[derive(Clone, Debug, serde::Serialize, specta::Type)]
pub struct RepointPreview {
    /// The landing skill's own name (its `SKILL.md` `name`, else its folder's
    /// name). `None` only when the target is a multi-skill repository the
    /// skill's name does not resolve — `candidates` then carries the choice.
    pub resolved_name: Option<String>,
    /// The branch the bytes would come from, as acquisition resolved it;
    /// `None` for a local folder, or a clone of the repository's default branch.
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
    /// The repo-relative folder that would be recorded; `None` for the
    /// repository root and for a local folder.
    pub subpath: Option<String>,
    /// Whether `resolved_name` is the skill's own name (case-insensitive).
    /// A mismatch is never a refusal — the Re-point would keep the skill's
    /// name and take the other skill's bytes.
    pub name_matches: bool,
    /// The repository's installable skills, by listing's admission, when the
    /// target URL names a repository (no skill folder) that holds more than
    /// one; `None` otherwise — always for a local folder.
    pub candidates: Option<Vec<GitSkillCandidate>>,
}

/// Resolve `target` for one Managed skill exactly as [`repoint_skill_source`]
/// would — the same validation, the same acquisition intent (the skill's
/// name), the same typed refusals — and **write nothing** the operator owns:
/// no central copy, no record, no Sync target. Git bytes land in a scratch
/// directory under the app cache that is removed before returning; the clone
/// goes through the git cache, so the real Re-point (or the picker's install)
/// reuses it. The one refusal that is an answer here is
/// [`SignalError::MultiSkills`]: it becomes `candidates`.
pub fn preview_repoint_source(
    paths: &InstallerPaths,
    store: &SkillStore,
    credentials: &dyn CredentialStore,
    skill_id: &str,
    target: RepointTarget,
    cancel: Option<&CancelToken>,
) -> Result<RepointPreview> {
    let token = super::settings::github_token_or_none(credentials);
    preview_repoint_source_with(
        paths,
        store,
        skill_id,
        target,
        cancel,
        &HttpGithubApi::new(token),
    )
}

/// [`preview_repoint_source`] with the GitHub adapter injected.
pub(crate) fn preview_repoint_source_with(
    paths: &InstallerPaths,
    store: &SkillStore,
    skill_id: &str,
    target: RepointTarget,
    cancel: Option<&CancelToken>,
    api: &dyn GithubApi,
) -> Result<RepointPreview> {
    let record = require_record(store, skill_id)?;
    check_cancelled(cancel)?;
    match target {
        RepointTarget::Local { path } => {
            let folder = expand_home_path_in(&paths.home, &path)?;
            validate_local_folder(&paths.home, &folder)?;
            let fallback = folder
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let resolved = landed_name(&folder, &fallback);
            Ok(RepointPreview {
                name_matches: same_name(&resolved, &record),
                resolved_name: Some(resolved),
                git_ref: None,
                subpath: None,
                candidates: None,
            })
        }
        RepointTarget::Git { url, selection } => {
            let url = url.trim();
            let source = parse_full_github_url(url)?;
            let ttl_ms = super::settings::git_cache_ttl_ms(store);
            let selection = selection.as_ref().map(RepointSelection::as_git_selection);
            preview_git(paths, &record, url, &source, selection, ttl_ms, cancel, api)
        }
    }
}

/// Scratch root for a preview's acquired bytes, under the app cache and
/// never inside the central repo.
const PREVIEW_SCRATCH_DIR: &str = "repoint-preview";

#[allow(clippy::too_many_arguments)]
fn preview_git(
    paths: &InstallerPaths,
    record: &SkillRecord,
    url: &str,
    source: &GitSource,
    selection: Option<GitSelection<'_>>,
    ttl_ms: i64,
    cancel: Option<&CancelToken>,
    api: &dyn GithubApi,
) -> Result<RepointPreview> {
    let scratch_root = paths.cache_dir.join(PREVIEW_SCRATCH_DIR);
    std::fs::create_dir_all(&scratch_root)?;
    // Dropped (and removed) on every path out of this function.
    let scratch = StagingDir::new_in(&scratch_root);
    // The acquisition the real Re-point runs (`acquire_git_repoint`).
    let acquired = acquire(
        &AcquireRequest {
            source,
            intent: match selection {
                Some(selection) => SkillIntent::Selection(selection),
                None => SkillIntent::ByName(Some(&record.name)),
            },
            dest: scratch.path(),
            cache_dir: &paths.cache_dir,
            ttl_ms,
            cancel,
            allow_fast_path: true,
        },
        api,
    );
    // A URL naming no skill folder may name a multi-skill repository; a
    // picked candidate is already the choice.
    let names_repository = source.subpath.is_none() && selection.is_none();
    let acquired = match acquired {
        Err(err) if names_repository && is_multi_skills(&err) => {
            let candidates = list_candidates(paths, url, ttl_ms, cancel, api)?;
            return Ok(RepointPreview {
                resolved_name: None,
                git_ref: None,
                subpath: None,
                name_matches: false,
                candidates: Some(candidates),
            });
        }
        other => other?,
    };
    ensure_installable_skill_dir(scratch.path())?;
    let subpath = acquired.resolved_subpath.filter(|path| path != ".");
    let fallback = match &subpath {
        Some(path) => dir_name_of(path).to_string(),
        None => source
            .api
            .as_ref()
            .map(|repo| repo.repo.clone())
            .unwrap_or_default(),
    };
    let resolved = landed_name(scratch.path(), &fallback);
    let candidates = if names_repository {
        Some(list_candidates(paths, url, ttl_ms, cancel, api)?).filter(|list| list.len() > 1)
    } else {
        None
    };
    Ok(RepointPreview {
        name_matches: same_name(&resolved, record),
        resolved_name: Some(resolved),
        git_ref: acquired.branch,
        subpath,
        candidates,
    })
}

/// Listing's candidate admission over the same repository, checked out in
/// full through the git cache — the entry the acquisition just used, and the
/// one the picker's install reuses.
fn list_candidates(
    paths: &InstallerPaths,
    url: &str,
    ttl_ms: i64,
    cancel: Option<&CancelToken>,
    api: &dyn GithubApi,
) -> Result<Vec<GitSkillCandidate>> {
    let listing = list_git_skills_with(url, None, api, |parsed| {
        fetch_through_cache(
            &paths.cache_dir,
            &FetchRequest {
                clone_url: &parsed.clone_url,
                branch: parsed.branch.as_deref(),
                subpath: None,
                ttl_ms,
                cancel,
            },
        )
        .map(|(dir, _revision)| dir)
    })?;
    check_cancelled(cancel)?;
    Ok(listing.candidates)
}

fn is_multi_skills(err: &anyhow::Error) -> bool {
    matches!(
        err.downcast_ref::<SignalError>(),
        Some(SignalError::MultiSkills)
    )
}

/// The name a skill folder introduces itself by: its manifest's `name`, else
/// `fallback` (the folder's own name), as discovery names a candidate.
fn landed_name(dir: &Path, fallback: &str) -> String {
    find_skill_md(dir)
        .and_then(|manifest| parse_skill_md(&manifest))
        .map(|(name, _)| name)
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn same_name(resolved: &str, record: &SkillRecord) -> bool {
    resolved.trim().to_lowercase() == record.name.trim().to_lowercase()
}

fn require_record(store: &SkillStore, skill_id: &str) -> Result<SkillRecord> {
    store.get_skill_by_id(skill_id)?.ok_or_else(|| {
        anyhow::anyhow!(SignalError::NotFound {
            kind: "skill".to_string(),
            id: skill_id.to_string(),
        })
    })
}

fn check_cancelled(cancel: Option<&CancelToken>) -> Result<()> {
    if cancel.is_some_and(|c| c.is_cancelled()) {
        anyhow::bail!(SignalError::Cancelled);
    }
    Ok(())
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
