//! Landing a skill's bytes from a git source in a directory — once, for
//! every flow that needs it.
//!
//! One question is answered here: *given a parsed git source and an intent,
//! put the skill's bytes in `dest` and tell me the revision and how they got
//! there*. Install-from-selection, the update/Refresh acquire phase and the
//! Explore preview are adapters that only choose the destination and pass the
//! answer on; none of them owns a download strategy any more.
//!
//! Two adapters meet at one seam:
//!
//! 1. **The GitHub Contents API fast path** ([`GithubApi`]) — used when the
//!    source carries GitHub coordinates and the intent names a real subpath.
//!    It fetches the branch SHA first, so the recorded revision is the real
//!    commit even though nothing is cloned.
//! 2. **The git clone** ([`super::git_cache::fetch_through_cache`]) — sparse
//!    when a subpath is known, full otherwise; the fallback for everything
//!    else.
//!
//! The fallback is deliberately partial: a GitHub **404** (skill not found)
//! and **403** (rate limited) are answers for the operator, raised as typed
//! [`SignalError`]s and never retried as a clone. Any other API failure is
//! infrastructure noise and falls back.
//!
//! Core resolves no roots here: `cache_dir`, `dest`, `ttl_ms` and the API
//! token are values the caller supplies. Everything the function touches is
//! shared behind `&`, so a bounded parallel pool can call it from worker
//! threads (each with its own [`HttpGithubApi`]).

use std::path::Path;

use anyhow::{Context, Result};

use super::cancel_token::CancelToken;
use super::errors::SignalError;
use super::git_cache::{fetch_through_cache, FetchRequest};
use super::github_download::{
    download_github_directory, fetch_branch_sha, parse_github_repo, GithubApiError,
};
use super::skill_discovery::{discover_skills, DiscoveredSkill};
use super::skill_matching::{match_skill_candidate, SkillMatch};
use super::sync_engine::copy_dir_recursive;

/// The GitHub repository a source lives in, when it lives in one. Presence is
/// the fast path's precondition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GithubRepo {
    pub owner: String,
    pub repo: String,
}

/// A git source as parsed from operator input: what to clone, which branch,
/// and the subpath the URL itself named (a `/tree/<branch>/<path>` link).
#[derive(Clone, Debug)]
pub struct GitSource {
    pub clone_url: String,
    pub branch: Option<String>,
    /// Subpath named by the URL, if any. An intent may override it.
    pub subpath: Option<String>,
    /// GitHub coordinates when the source is a github.com repository.
    pub api: Option<GithubRepo>,
}

/// What the caller wants out of the source.
#[derive(Clone, Copy, Debug)]
pub enum SkillIntent<'a> {
    /// Exactly this repo-relative subpath; `"."` is the repo root.
    Subpath(&'a str),
    /// A skill named in a possibly multi-skill repo. A repo holding several
    /// skills must resolve to exactly one, else [`SignalError::MultiSkills`]
    /// — the caller has to name the skill precisely.
    NamedSkill(Option<&'a str>),
    /// The lenient sibling of [`SkillIntent::NamedSkill`], for the legacy
    /// record whose `source_subpath` was never stored: a name that resolves
    /// backfills the subpath, and one that does not takes the whole repo
    /// rather than failing an update that used to work.
    NamedSkillOrWholeRepo(&'a str),
}

/// Which adapter served the bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcquireStrategy {
    GithubApi,
    GitClone { sparse: bool },
}

/// One acquisition request. `dest` is filled with the skill's bytes.
pub struct AcquireRequest<'a> {
    pub source: &'a GitSource,
    pub intent: SkillIntent<'a>,
    /// Directory the skill's bytes are written into (created as needed).
    pub dest: &'a Path,
    /// App cache root; the git clone cache lives under it.
    pub cache_dir: &'a Path,
    /// Git-cache freshness window, resolved by the caller
    /// (`settings::git_cache_ttl_ms`).
    pub ttl_ms: i64,
    pub cancel: Option<&'a CancelToken>,
    /// Off for callers that need the git tree itself (or want a clone).
    pub allow_fast_path: bool,
}

/// What one acquisition produced.
#[derive(Clone, Debug)]
pub struct Acquired {
    /// The source revision these bytes came from — a real commit SHA on both
    /// paths.
    pub revision: String,
    pub strategy: AcquireStrategy,
    /// The repo-relative subpath the bytes were taken from; `None` when the
    /// whole repo was taken. A named intent reports what it resolved.
    pub resolved_subpath: Option<String>,
}

/// The coordinates one GitHub Contents API request needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GithubCoords {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub subpath: String,
}

impl GithubCoords {
    /// The human-checkable tree URL, used as diagnostic context on a 404.
    pub fn tree_url(&self) -> String {
        format!(
            "https://github.com/{}/{}/tree/{}/{}",
            self.owner, self.repo, self.branch, self.subpath
        )
    }
}

/// The GitHub Contents API as acquisition consumes it. One trait, two impls:
/// [`HttpGithubApi`] in production, a scripted stub in tests — which is why no
/// acquisition test needs the network.
pub trait GithubApi {
    /// HEAD commit SHA of `coords.branch`.
    fn branch_sha(&self, coords: &GithubCoords) -> Result<String>;
    /// Download `coords.subpath` into `dest`.
    fn download_directory(
        &self,
        coords: &GithubCoords,
        dest: &Path,
        cancel: Option<&CancelToken>,
    ) -> Result<()>;
}

/// The production adapter: real HTTP against api.github.com, optionally
/// authenticated with the operator's token (`settings::github_token`).
pub struct HttpGithubApi {
    token: Option<String>,
}

impl HttpGithubApi {
    pub fn new(token: Option<String>) -> Self {
        HttpGithubApi { token }
    }
}

impl GithubApi for HttpGithubApi {
    fn branch_sha(&self, coords: &GithubCoords) -> Result<String> {
        fetch_branch_sha(
            &coords.owner,
            &coords.repo,
            &coords.branch,
            self.token.as_deref(),
        )
    }

    fn download_directory(
        &self,
        coords: &GithubCoords,
        dest: &Path,
        cancel: Option<&CancelToken>,
    ) -> Result<()> {
        download_github_directory(
            &coords.owner,
            &coords.repo,
            &coords.branch,
            &coords.subpath,
            dest,
            cancel,
            self.token.as_deref(),
        )
    }
}

/// Land the requested skill's bytes in `req.dest`.
///
/// Tries the fast path when it applies, falls back to a clone when the API
/// failure is infrastructure, and raises the two GitHub conditions the
/// operator must see. See the module doc for the whole policy.
pub fn acquire(req: &AcquireRequest, api: &dyn GithubApi) -> Result<Acquired> {
    check_cancelled(req.cancel)?;

    // The subpath both adapters key on: the intent's when it names one, else
    // the one the source URL carried. `"."` is the repo root, not a subpath.
    let known_subpath = match req.intent {
        SkillIntent::Subpath(subpath) => Some(subpath),
        SkillIntent::NamedSkill(_) | SkillIntent::NamedSkillOrWholeRepo(_) => {
            req.source.subpath.as_deref()
        }
    }
    .filter(|subpath| !subpath.is_empty() && *subpath != ".");

    if let Some(coords) = fast_path_coords(req, known_subpath) {
        match fast_path(&coords, req, api) {
            Ok(acquired) => return Ok(report(acquired, req)),
            Err(err) => {
                // Whatever the failure, the partial download is not part of
                // the answer.
                let _ = std::fs::remove_dir_all(req.dest);
                classify_fast_path_failure(err, &coords)?;
            }
        }
    }

    clone_path(req, known_subpath).map(|acquired| report(acquired, req))
}

/// One log line per acquisition, naming the adapter that served it.
fn report(acquired: Acquired, req: &AcquireRequest) -> Acquired {
    log::info!(
        "[acquire] {:?} url={} subpath={:?} revision={} dest={:?}",
        acquired.strategy,
        req.source.clone_url,
        acquired.resolved_subpath,
        acquired.revision,
        req.dest
    );
    acquired
}

/// The fast path's precondition: allowed by the caller, a GitHub source, and
/// an intent that names a real subpath.
fn fast_path_coords(req: &AcquireRequest, known_subpath: Option<&str>) -> Option<GithubCoords> {
    if !req.allow_fast_path {
        return None;
    }
    let repo = req.source.api.as_ref()?;
    let subpath = known_subpath?;
    Some(GithubCoords {
        owner: repo.owner.clone(),
        repo: repo.repo.clone(),
        branch: req
            .source
            .branch
            .clone()
            .unwrap_or_else(|| "main".to_string()),
        subpath: subpath.to_string(),
    })
}

/// Adapter one: the GitHub Contents API. The SHA is fetched **before** the
/// download so a served fast path always records the real commit — and so a
/// bad branch fails before any bytes land.
fn fast_path(coords: &GithubCoords, req: &AcquireRequest, api: &dyn GithubApi) -> Result<Acquired> {
    log::info!(
        "[acquire] GitHub API download: {}/{}@{} path={}",
        coords.owner,
        coords.repo,
        coords.branch,
        coords.subpath
    );
    let revision = api.branch_sha(coords)?;
    api.download_directory(coords, req.dest, req.cancel)?;
    Ok(Acquired {
        revision,
        strategy: AcquireStrategy::GithubApi,
        resolved_subpath: Some(coords.subpath.clone()),
    })
}

/// Decide what a failed fast path means: `Ok(())` to fall back to a clone,
/// `Err` for the conditions the operator must see.
///
/// The HTTP layer classifies the status at the origin ([`GithubApiError`]);
/// this maps the two codes acquisition owns to typed signals. No string
/// sniffing, and cancellation is never a "failed strategy".
fn classify_fast_path_failure(err: anyhow::Error, coords: &GithubCoords) -> Result<()> {
    if err.downcast_ref::<SignalError>() == Some(&SignalError::Cancelled) {
        return Err(err);
    }
    match err.downcast_ref::<GithubApiError>() {
        Some(GithubApiError { status: 404, .. }) => {
            anyhow::bail!(SignalError::GithubSkillNotFound {
                url: coords.tree_url(),
            })
        }
        Some(GithubApiError {
            status: 403,
            reset_minutes,
            ..
        }) => {
            // 0 = "no ETA" on the wire.
            anyhow::bail!(SignalError::RateLimited {
                reset_minutes: reset_minutes.unwrap_or(0),
            })
        }
        _ => {
            log::warn!(
                "[acquire] GitHub API download failed, falling back to git clone: {:#}",
                err
            );
            Ok(())
        }
    }
}

/// Adapter two: the git clone cache. Sparse when a subpath is known, then the
/// intent decides which directory of the tree is the skill.
fn clone_path(req: &AcquireRequest, known_subpath: Option<&str>) -> Result<Acquired> {
    let (repo_dir, revision) = fetch_through_cache(
        req.cache_dir,
        &FetchRequest {
            clone_url: &req.source.clone_url,
            branch: req.source.branch.as_deref(),
            subpath: known_subpath,
            ttl_ms: req.ttl_ms,
            cancel: req.cancel,
        },
    )?;
    check_cancelled(req.cancel)?;

    let resolved_subpath = resolve_subpath(&repo_dir, req.intent, known_subpath)?;
    let copy_src = match &resolved_subpath {
        Some(subpath) => repo_dir.join(subpath),
        None => repo_dir.clone(),
    };
    if !copy_src.exists() {
        anyhow::bail!("path not found in repo: {:?}", copy_src);
    }

    copy_dir_recursive(&copy_src, req.dest)
        .with_context(|| format!("copy {:?} -> {:?}", copy_src, req.dest))?;

    Ok(Acquired {
        revision,
        strategy: AcquireStrategy::GitClone {
            sparse: known_subpath.is_some(),
        },
        resolved_subpath,
    })
}

/// The one place multi-skill name matching and subpath backfill live: which
/// repo-relative directory the intent points at (`None` = the whole repo).
fn resolve_subpath(
    repo_dir: &Path,
    intent: SkillIntent,
    known_subpath: Option<&str>,
) -> Result<Option<String>> {
    if let Some(subpath) = known_subpath {
        return Ok(Some(subpath.to_string()));
    }
    let name = match intent {
        // A root subpath is the repo, and there is nothing to match.
        SkillIntent::Subpath(_) => return Ok(None),
        SkillIntent::NamedSkill(name) => name,
        SkillIntent::NamedSkillOrWholeRepo(name) => Some(name),
    };

    // A repo with fewer than two installable skills is the skill: no name is
    // needed and none is required.
    let candidates = installable_skills_in_repo(repo_dir);
    if candidates.len() < 2 {
        return Ok(None);
    }

    let lenient = matches!(intent, SkillIntent::NamedSkillOrWholeRepo(_));
    let matched = name.map(|name| match_skill_candidate(name, &candidates));
    match matched {
        Some(SkillMatch::Resolved(candidate)) => Ok(Some(candidate.subpath.clone())),
        // Anything short of one unambiguous match: the strict intent makes the
        // caller name the skill, the lenient one takes the repo whole.
        _ if lenient => Ok(None),
        _ => Err(anyhow::anyhow!(SignalError::MultiSkills)),
    }
}

/// Skill candidates a git flow may install from a cloned repo: everything
/// discovery found that has skill bytes (a `SKILL.md`, even a broken one, or
/// a `.claude/skills/` child), excluding the repo root itself. The root is
/// never one of the "skills in a multi-skill repo".
pub fn installable_skills_in_repo(repo_dir: &Path) -> Vec<DiscoveredSkill> {
    discover_skills(repo_dir)
        .into_iter()
        .filter(|c| c.validity.is_installable() && c.subpath != ".")
        .collect()
}

fn check_cancelled(cancel: Option<&CancelToken>) -> Result<()> {
    if cancel.is_some_and(|c| c.is_cancelled()) {
        anyhow::bail!(SignalError::Cancelled);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Parsing operator input into a [`GitSource`]
// ---------------------------------------------------------------------------

/// Parse operator input into a [`GitSource`].
///
/// Supports `https://github.com/owner/repo[.git]`, its `/tree/<branch>/<path>`
/// and `/blob/<branch>/<path>` forms, the `owner/repo` shorthand, and passes
/// anything else through as a clone URL (a local path, another remote).
pub fn parse_github_url(input: &str) -> GitSource {
    let trimmed = input.trim().trim_end_matches('/');

    // Convenience: allow GitHub shorthand inputs like `owner/repo` (and `owner/repo/tree/<branch>/...`).
    // This keeps the UI friendly while still allowing local paths or other git remotes.
    let normalized = if trimmed.starts_with("https://github.com/") {
        trimmed.to_string()
    } else if trimmed.starts_with("http://github.com/") {
        trimmed.replacen("http://github.com/", "https://github.com/", 1)
    } else if trimmed.starts_with("github.com/") {
        format!("https://{}", trimmed)
    } else if looks_like_github_shorthand(trimmed) {
        format!("https://github.com/{}", trimmed)
    } else {
        trimmed.to_string()
    };

    let trimmed = normalized.trim_end_matches('/');
    let gh_prefix = "https://github.com/";
    if !trimmed.starts_with(gh_prefix) {
        return GitSource {
            clone_url: trimmed.to_string(),
            branch: None,
            subpath: None,
            api: None,
        };
    }

    let rest = &trimmed[gh_prefix.len()..];
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() < 2 {
        return GitSource {
            clone_url: trimmed.to_string(),
            branch: None,
            subpath: None,
            api: None,
        };
    }

    let owner = parts[0];
    let mut repo = parts[1].to_string();
    if let Some(stripped) = repo.strip_suffix(".git") {
        repo = stripped.to_string();
    }
    let clone_url = format!("https://github.com/{}/{}.git", owner, repo);
    let api = parse_github_repo(&clone_url).map(|(owner, repo)| GithubRepo { owner, repo });

    if parts.len() >= 4 && (parts[2] == "tree" || parts[2] == "blob") {
        let branch = Some(parts[3].to_string());
        let subpath = if parts.len() > 4 {
            Some(normalize_github_skill_subpath(&parts[4..].join("/")))
        } else {
            None
        };
        return GitSource {
            clone_url,
            branch,
            subpath,
            api,
        };
    }

    GitSource {
        clone_url,
        branch: None,
        subpath: None,
        api,
    }
}

fn normalize_github_skill_subpath(subpath: &str) -> String {
    let trimmed = subpath.trim_matches('/');
    if trimmed.eq_ignore_ascii_case("SKILL.md") {
        return ".".to_string();
    }
    trimmed
        .strip_suffix("/SKILL.md")
        .or_else(|| trimmed.strip_suffix("/skill.md"))
        .unwrap_or(trimmed)
        .to_string()
}

fn looks_like_github_shorthand(input: &str) -> bool {
    if input.is_empty() {
        return false;
    }
    if input.starts_with('/') || input.starts_with('~') || input.starts_with('.') {
        return false;
    }
    // Avoid scp-like ssh URLs (git@github.com:owner/repo) and any explicit schemes.
    if input.contains("://") || input.contains('@') || input.contains(':') {
        return false;
    }

    let parts: Vec<&str> = input.split('/').collect();
    if parts.len() < 2 {
        return false;
    }

    let owner = parts[0];
    let repo = parts[1];
    if owner.is_empty()
        || repo.is_empty()
        || owner == "."
        || owner == ".."
        || repo == "."
        || repo == ".."
    {
        return false;
    }

    let is_safe_segment = |s: &str| {
        s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    };
    if !is_safe_segment(owner) || !is_safe_segment(repo.trim_end_matches(".git")) {
        return false;
    }

    // If there are more path parts, only accept the GitHub UI patterns we can parse.
    if parts.len() > 2 {
        matches!(parts[2], "tree" | "blob")
    } else {
        true
    }
}

#[cfg(test)]
#[path = "tests/git_acquisition.rs"]
mod tests;
