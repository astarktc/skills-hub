use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result};

use super::cancel_token::CancelToken;
use super::central_repo::ensure_central_repo;
use super::clock::now_ms;
use super::errors::SignalError;
use super::git_cache::{explore_preview_key, fetch_through_cache, FetchRequest};
use super::github_download::{download_github_directory, parse_github_api_params, GithubApiError};
pub use super::install_finalize::InstallResult;
use super::install_finalize::{
    ensure_name_available, finalize_install, finalize_update, NameIntent, SkillProvenance,
    StagingDir,
};
use super::propagation::{propagate_unlocked, PropagationReport};
use super::skill_discovery::{
    discover_skills, find_skill_md, is_skill_dir, parse_skill_md, parse_skill_md_with_reason,
    DiscoveredSkill,
};
use super::skill_lock::try_enrich_from_skill_lock_with_home;
use super::skill_matching::{match_skill_candidate, CandidateMatch, MatchableSkill, SkillMatch};
use super::skill_store::SkillStore;
use super::sync_engine::copy_dir_recursive;

/// Filesystem roots the installer reads. Resolved once per command at the
/// wiring seam (home, central repo setting, app cache dir) so core never
/// touches `dirs` or `tauri` and tests substitute temp directories.
#[derive(Clone, Debug)]
pub struct InstallerPaths {
    /// Operator home: decides tool installedness and skill-lock provenance.
    pub home: PathBuf,
    /// Central skills repo root (see `central_repo::resolve_central_repo_path`).
    pub central_dir: PathBuf,
    /// App cache root; the git clone cache lives at `cache_dir/skills-hub-git-cache`.
    pub cache_dir: PathBuf,
}

pub fn install_local_skill(
    paths: &InstallerPaths,
    store: &SkillStore,
    source_path: &Path,
    name: Option<String>,
) -> Result<InstallResult> {
    if !source_path.exists() {
        anyhow::bail!("source path not found: {:?}", source_path);
    }

    let name = name.unwrap_or_else(|| {
        source_path
            .file_name()
            .map(|v| v.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed-skill".to_string())
    });

    let central_dir = &paths.central_dir;
    ensure_central_repo(central_dir)?;
    ensure_name_available(central_dir, &name)?;

    let staged = StagingDir::new_in(central_dir);
    copy_dir_recursive(source_path, staged.path())
        .with_context(|| format!("copy {:?} -> {:?}", source_path, staged.path()))?;

    // Enrich with git provenance from ~/.agents/.skill-lock.json if source is a
    // symlink into ~/.agents/skills/ (skills installed via `npx skills add`).
    let provenance = match try_enrich_from_skill_lock_with_home(source_path, &paths.home) {
        Some(lock_entry) => SkillProvenance {
            source_type: "git".to_string(),
            source_ref: Some(lock_entry.source_url),
            source_subpath: lock_entry.source_subpath,
            source_revision: None,
        },
        None => SkillProvenance::local(source_path),
    };

    // The name is always honored as given: callers either pass the operator's
    // choice or the folder name, and a local folder name is the skill's name.
    finalize_install(
        store,
        central_dir,
        staged,
        NameIntent::UserProvided(name),
        provenance,
    )
}

/// Wrap an optional operator-supplied name as a [`NameIntent`], deriving one
/// when absent.
fn name_intent(name: Option<String>, derive: impl FnOnce() -> String) -> NameIntent {
    match name {
        Some(name) => NameIntent::UserProvided(name),
        None => NameIntent::Derived(derive()),
    }
}

/// Last path segment of `subpath`, or the repo name when the subpath is absent
/// or the repo root (`.`).
fn derive_name_from_subpath(clone_url: &str, subpath: Option<&str>) -> String {
    match subpath {
        Some(".") | None => derive_name_from_repo_url(clone_url),
        Some(subpath) => subpath
            .rsplit('/')
            .next()
            .map(|s| s.to_string())
            .unwrap_or_else(|| derive_name_from_repo_url(clone_url)),
    }
}

#[derive(Clone, Debug)]
struct ParsedGitSource {
    clone_url: String,
    branch: Option<String>,
    subpath: Option<String>,
}

fn parse_github_url(input: &str) -> ParsedGitSource {
    // Supports:
    // - https://github.com/owner/repo
    // - https://github.com/owner/repo.git
    // - https://github.com/owner/repo/tree/<branch>/<path>
    // - https://github.com/owner/repo/blob/<branch>/<path>
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
        return ParsedGitSource {
            clone_url: trimmed.to_string(),
            branch: None,
            subpath: None,
        };
    }

    let rest = &trimmed[gh_prefix.len()..];
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() < 2 {
        return ParsedGitSource {
            clone_url: trimmed.to_string(),
            branch: None,
            subpath: None,
        };
    }

    let owner = parts[0];
    let mut repo = parts[1].to_string();
    if let Some(stripped) = repo.strip_suffix(".git") {
        repo = stripped.to_string();
    }
    let clone_url = format!("https://github.com/{}/{}.git", owner, repo);

    if parts.len() >= 4 && (parts[2] == "tree" || parts[2] == "blob") {
        let branch = Some(parts[3].to_string());
        let subpath = if parts.len() > 4 {
            Some(normalize_github_skill_subpath(&parts[4..].join("/")))
        } else {
            None
        };
        return ParsedGitSource {
            clone_url,
            branch,
            subpath,
        };
    }

    ParsedGitSource {
        clone_url,
        branch: None,
        subpath: None,
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

fn derive_name_from_repo_url(repo_url: &str) -> String {
    let mut name = repo_url
        .split('/')
        .next_back()
        .unwrap_or("skill")
        .to_string();
    if let Some(stripped) = name.strip_suffix(".git") {
        name = stripped.to_string();
    }
    if name.is_empty() {
        "skill".to_string()
    } else {
        name
    }
}

fn ensure_installable_skill_dir(p: &Path) -> Result<()> {
    if is_skill_dir(p) {
        Ok(())
    } else {
        anyhow::bail!(SignalError::SkillInvalid {
            reason: "missing_skill_md".to_string(),
        });
    }
}

/// One Managed skill's freshly acquired bytes, waiting to be finalized.
///
/// Produced by [`acquire_managed_skill_update`] outside the mutation guard
/// and consumed by [`finalize_and_propagate_unlocked`] inside it: the two
/// phases of an update (and of the Refresh (all) batch) meet here.
pub(crate) struct AcquiredUpdate {
    pub record: super::skill_store::SkillRecord,
    pub staged: StagingDir,
    pub new_revision: Option<String>,
}

/// What one finalized-and-propagated skill update produced.
pub(crate) struct UpdateOutcome {
    pub skill_id: String,
    pub name: String,
    pub content_hash: Option<String>,
    pub source_revision: Option<String>,
    pub propagation: PropagationReport,
}

/// Re-acquire a Managed skill's bytes from its source into a Staging dir.
///
/// Acquisition (git clone / local copy) runs **outside** the mutation guard:
/// it touches no Sync target and can be slow. Only finalize + Propagation,
/// which do, run inside it (see [`finalize_and_propagate_unlocked`]).
/// Sequential today, one self-contained result per skill so a bounded
/// parallel pool is a drop-in later.
pub(crate) fn acquire_managed_skill_update(
    paths: &InstallerPaths,
    store: &SkillStore,
    skill_id: &str,
) -> Result<AcquiredUpdate> {
    let mut record = store.get_skill_by_id(skill_id)?.ok_or_else(|| {
        anyhow::anyhow!(SignalError::NotFound {
            kind: "skill".to_string(),
            id: skill_id.to_string(),
        })
    })?;

    let central_path = PathBuf::from(record.central_path.clone());
    if !central_path.exists() {
        anyhow::bail!("central path not found: {:?}", central_path);
    }
    let central_parent = central_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid central path"))?
        .to_path_buf();

    // Build new content in a sibling staging dir; finalize swaps it in.
    let staged = StagingDir::new_in(&central_parent);
    let staging_dir = staged.path().to_path_buf();

    let mut new_revision: Option<String> = None;

    if record.source_type == "git" {
        let repo_url = record
            .source_ref
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("missing source_ref for git skill"))?;
        let parsed = parse_github_url(repo_url);

        let (repo_dir, rev) = fetch_through_cache(
            &paths.cache_dir,
            &FetchRequest {
                clone_url: &parsed.clone_url,
                branch: parsed.branch.as_deref(),
                subpath: record.source_subpath.as_deref(),
                ttl_ms: super::settings::git_cache_ttl_ms(store),
                cancel: None,
            },
        )?;
        new_revision = Some(rev);

        // Prefer stored source_subpath (from install time) over URL-parsed subpath.
        // For legacy records where source_subpath is NULL and URL has no subpath,
        // try to auto-match by skill name in the repo (backfill).
        let mut resolved_subpath = record
            .source_subpath
            .as_deref()
            .or(parsed.subpath.as_deref())
            .map(|s| s.to_string());
        if resolved_subpath.is_none() {
            // Multi-skill repo with no stored subpath: match by name
            let candidates = installable_skills_in_repo(&repo_dir);
            if candidates.len() >= 2 {
                if let SkillMatch::Resolved(matched) =
                    match_skill_candidate(&record.name, &candidates)
                {
                    resolved_subpath = Some(matched.subpath.clone());
                    // Backfill source_subpath for future updates (carried into the
                    // refreshed record by finalize_update as well).
                    record.source_subpath = Some(matched.subpath.clone());
                    let _ = store.upsert_skill(&record);
                }
            }
        }
        let copy_src = if let Some(subpath) = &resolved_subpath {
            repo_dir.join(subpath)
        } else {
            repo_dir.clone()
        };
        if !copy_src.exists() {
            anyhow::bail!("path not found in repo: {:?}", copy_src);
        }

        copy_dir_recursive(&copy_src, &staging_dir)
            .with_context(|| format!("copy {:?} -> {:?}", copy_src, staging_dir))?;
    } else if record.source_type == "local" {
        let source = record
            .source_ref
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("missing source_ref for local skill"))?;
        let source_path = PathBuf::from(source);
        if !source_path.exists() {
            anyhow::bail!("source path not found: {:?}", source_path);
        }
        copy_dir_recursive(&source_path, &staging_dir)
            .with_context(|| format!("copy {:?} -> {:?}", source_path, staging_dir))?;
    } else {
        anyhow::bail!("unsupported source_type for update: {}", record.source_type);
    }

    Ok(AcquiredUpdate {
        record,
        staged,
        new_revision,
    })
}

/// Unlocked internal seam: finalize the Staging dir into the central copy,
/// then hand every Sync target to Propagation. The caller holds the mutation
/// guard. There is no target loop here — bringing targets into line is one
/// rule and it lives in `core::propagation`.
pub(crate) fn finalize_and_propagate_unlocked(
    paths: &InstallerPaths,
    store: &SkillStore,
    acquired: AcquiredUpdate,
) -> Result<UpdateOutcome> {
    let AcquiredUpdate {
        record,
        staged,
        new_revision,
    } = acquired;
    let now = now_ms();

    let updated = finalize_update(store, &record, staged, new_revision.clone())?;
    let content_hash = updated.content_hash.clone();

    let propagation = propagate_unlocked(store, paths, &record.id, content_hash.as_deref(), now)?;

    Ok(UpdateOutcome {
        skill_id: record.id,
        name: record.name,
        content_hash,
        source_revision: new_revision,
        propagation,
    })
}

#[derive(Clone, Debug, serde::Serialize, specta::Type)]
pub struct GitSkillCandidate {
    pub name: String,
    pub description: Option<String>,
    pub subpath: String,
}

#[derive(Clone, Debug, serde::Serialize, specta::Type)]
pub struct LocalSkillCandidate {
    pub name: String,
    pub description: Option<String>,
    pub subpath: String,
    pub valid: bool,
    pub reason: Option<String>,
}

/// Skill candidates a git flow may install from a cloned repo: everything
/// discovery found that has skill bytes (a `SKILL.md`, even a broken one, or
/// a `.claude/skills/` child), excluding the repo root itself. The root is
/// never one of the "skills in a multi-skill repo".
fn installable_skills_in_repo(repo_dir: &Path) -> Vec<DiscoveredSkill> {
    discover_skills(repo_dir)
        .into_iter()
        .filter(|c| c.validity.is_installable() && c.subpath != ".")
        .collect()
}

/// Git listing adapter: the git side admits any installable candidate;
/// validity is not surfaced on this wire shape.
fn git_candidate(c: DiscoveredSkill) -> GitSkillCandidate {
    GitSkillCandidate {
        name: c.name,
        description: c.description,
        subpath: c.subpath,
    }
}

impl MatchableSkill for GitSkillCandidate {
    fn name(&self) -> &str {
        &self.name
    }
    fn subpath(&self) -> &str {
        &self.subpath
    }
}

/// What the git add flow gets back from a listing: the candidates plus, when
/// the caller named the skill it is after (Explore install), that name
/// resolved against them by the one core matching rule.
#[derive(Clone, Debug, serde::Serialize, specta::Type)]
pub struct GitSkillListing {
    pub candidates: Vec<GitSkillCandidate>,
    /// `None` when no `target_name` was given.
    pub target_match: Option<CandidateMatch>,
}

pub fn list_git_skills(
    paths: &InstallerPaths,
    store: &SkillStore,
    repo_url: &str,
    target_name: Option<&str>,
) -> Result<GitSkillListing> {
    let parsed = parse_github_url(repo_url);
    let (repo_dir, _rev) = fetch_through_cache(
        &paths.cache_dir,
        &FetchRequest {
            clone_url: &parsed.clone_url,
            branch: parsed.branch.as_deref(),
            subpath: None,
            ttl_ms: super::settings::git_cache_ttl_ms(store),
            cancel: None,
        },
    )?;

    let candidates = git_candidates_in(&repo_dir, parsed.subpath.as_deref());
    let target_match = target_name.map(|target| match_skill_candidate(target, &candidates).into());
    Ok(GitSkillListing {
        candidates,
        target_match,
    })
}

/// Git listing over a cloned repo. A folder URL (`subpath`) scopes discovery
/// to that folder while subpaths stay repo-relative; when the folder is itself
/// a skill it is the single candidate.
fn git_candidates_in(repo_dir: &Path, subpath: Option<&str>) -> Vec<GitSkillCandidate> {
    let scan_root = match subpath {
        Some(sub) => repo_dir.join(sub),
        None => repo_dir.to_path_buf(),
    };
    if !scan_root.is_dir() {
        return Vec::new();
    }
    let mut found = discover_skills(&scan_root);
    if subpath.is_some() && found.iter().any(|c| c.subpath == ".") {
        found.retain(|c| c.subpath == ".");
    }

    found
        .into_iter()
        .filter(|c| c.validity.is_installable())
        .map(|mut c| {
            if let Some(prefix) = subpath {
                c.subpath = if c.subpath == "." {
                    prefix.to_string()
                } else {
                    format!("{}/{}", prefix.trim_end_matches('/'), c.subpath)
                };
            }
            git_candidate(c)
        })
        .collect()
}

/// Local listing adapter: every discovered candidate is shown, with its
/// validity and reason, so the picker can explain why a folder under a
/// declared skills dir is not selectable.
pub fn list_local_skills(base_path: &Path) -> Result<Vec<LocalSkillCandidate>> {
    if !base_path.exists() {
        anyhow::bail!("source path not found: {:?}", base_path);
    }
    Ok(discover_skills(base_path)
        .into_iter()
        .map(|c| LocalSkillCandidate {
            name: c.name,
            description: c.description,
            subpath: c.subpath,
            valid: c.validity.is_valid(),
            reason: c.validity.reason().map(str::to_string),
        })
        .collect())
}

pub fn install_git_skill_from_selection(
    paths: &InstallerPaths,
    store: &SkillStore,
    repo_url: &str,
    subpath: &str,
    name: Option<String>,
) -> Result<InstallResult> {
    let parsed = parse_github_url(repo_url);
    let name = name_intent(name, || {
        derive_name_from_subpath(&parsed.clone_url, Some(subpath))
    });

    let central_dir = &paths.central_dir;
    ensure_central_repo(central_dir)?;
    ensure_name_available(central_dir, name.requested())?;

    let (repo_dir, revision) = fetch_through_cache(
        &paths.cache_dir,
        &FetchRequest {
            clone_url: &parsed.clone_url,
            branch: parsed.branch.as_deref(),
            subpath: None,
            ttl_ms: super::settings::git_cache_ttl_ms(store),
            cancel: None,
        },
    )?;

    let copy_src = if subpath == "." {
        repo_dir.clone()
    } else {
        repo_dir.join(subpath)
    };
    if !copy_src.exists() {
        anyhow::bail!("path not found in repo: {:?}", copy_src);
    }
    ensure_installable_skill_dir(&copy_src)?;

    let staged = StagingDir::new_in(central_dir);
    copy_dir_recursive(&copy_src, staged.path())
        .with_context(|| format!("copy {:?} -> {:?}", copy_src, staged.path()))?;

    let source_subpath = if subpath == "." {
        None
    } else {
        Some(subpath.to_string())
    };
    finalize_install(
        store,
        central_dir,
        staged,
        name,
        SkillProvenance::git(repo_url, source_subpath, Some(revision)),
    )
}

pub fn install_local_skill_from_selection(
    paths: &InstallerPaths,
    store: &SkillStore,
    base_path: &Path,
    subpath: &str,
    name: Option<String>,
) -> Result<InstallResult> {
    if !base_path.exists() {
        anyhow::bail!("source path not found: {:?}", base_path);
    }

    let selected_dir = if subpath == "." {
        base_path.to_path_buf()
    } else {
        base_path.join(subpath)
    };
    if !selected_dir.exists() {
        anyhow::bail!("source path not found: {:?}", selected_dir);
    }

    let skill_md = find_skill_md(&selected_dir);
    if skill_md.is_none() {
        anyhow::bail!(SignalError::SkillInvalid {
            reason: "missing_skill_md".to_string(),
        });
    }
    let (parsed_name, _desc) =
        parse_skill_md_with_reason(&skill_md.unwrap()).map_err(|reason| {
            anyhow::anyhow!(SignalError::SkillInvalid {
                reason: reason.to_string(),
            })
        })?;

    let display_name = name.unwrap_or(parsed_name);

    install_local_skill(paths, store, &selected_dir, Some(display_name))
}

/// Fetch a single skill's files into `dest_dir`.
///
/// Shared download engine used by both the update and explore-preview paths.
/// Returns the git revision when the bytes came from a clone, `None` when the
/// GitHub API download path served them.
///
/// Handles GitHub API download, git clone fallback, subpath extraction, and
/// multi-skill repo resolution.
///
/// The caller is responsible for:
/// - Choosing and preparing `dest_dir`
/// - Any caching around the destination (explore-cache hit check, etc.)
/// - Post-download processing (DB registration, name renaming, content hash)
fn fetch_skill_files(
    cache_dir: &Path,
    store: &SkillStore,
    parsed: &ParsedGitSource,
    skill_name: Option<&str>,
    dest_dir: &Path,
    cancel: Option<&CancelToken>,
) -> Result<Option<String>> {
    let github_token = super::settings::github_token(store)?;
    let github_token_opt = github_token.as_deref();

    // Path A: GitHub URL with subpath — try API download, fall back to git clone.
    if let Some((owner, repo, branch, subpath)) = parse_github_api_params(
        &parsed.clone_url,
        parsed.branch.as_deref(),
        parsed.subpath.as_deref(),
    ) {
        log::info!(
            "[fetch] GitHub API download: {}/{} path={}",
            owner,
            repo,
            subpath
        );
        match download_github_directory(
            &owner,
            &repo,
            &branch,
            &subpath,
            dest_dir,
            cancel,
            github_token_opt,
        ) {
            Ok(()) => {
                return Ok(None);
            }
            Err(err) => {
                let _ = std::fs::remove_dir_all(dest_dir);
                // Cancellation propagates untouched to the command seam.
                if matches!(
                    err.downcast_ref::<SignalError>(),
                    Some(SignalError::Cancelled)
                ) {
                    return Err(err);
                }
                // The HTTP layer classifies the status at the origin; map the
                // codes this flow owns to typed conditions, no string sniffing.
                match err.downcast_ref::<GithubApiError>() {
                    Some(GithubApiError { status: 404, .. }) => {
                        anyhow::bail!(SignalError::GithubSkillNotFound {
                            url: format!(
                                "{}/tree/{}/{}",
                                parsed.clone_url.trim_end_matches(".git"),
                                branch,
                                subpath
                            ),
                        });
                    }
                    Some(GithubApiError {
                        status: 403,
                        reset_minutes,
                        ..
                    }) => {
                        // 0 = "no ETA" on the wire.
                        anyhow::bail!(SignalError::RateLimited {
                            reset_minutes: reset_minutes.unwrap_or(0),
                        });
                    }
                    _ => {}
                }
                // Fall back to git clone.
                log::warn!(
                    "[fetch] GitHub API download failed, falling back to git clone: {:#}",
                    err
                );
                std::fs::create_dir_all(dest_dir)?;
                let (repo_dir, rev) = fetch_through_cache(
                    cache_dir,
                    &FetchRequest {
                        clone_url: &parsed.clone_url,
                        branch: parsed.branch.as_deref(),
                        subpath: None,
                        ttl_ms: super::settings::git_cache_ttl_ms(store),
                        cancel,
                    },
                )?;
                let sub_src = repo_dir.join(&subpath);
                if !sub_src.exists() {
                    anyhow::bail!("subpath not found in repo: {:?}", sub_src);
                }
                copy_dir_recursive(&sub_src, dest_dir)
                    .with_context(|| format!("copy {:?} -> {:?}", sub_src, dest_dir))?;
                return Ok(Some(rev));
            }
        }
    }

    // Path B: No subpath or non-GitHub URL — full clone, then resolve copy source.
    let (repo_dir, rev) = fetch_through_cache(
        cache_dir,
        &FetchRequest {
            clone_url: &parsed.clone_url,
            branch: parsed.branch.as_deref(),
            subpath: None,
            ttl_ms: super::settings::git_cache_ttl_ms(store),
            cancel,
        },
    )?;

    let copy_src = if let Some(subpath) = &parsed.subpath {
        let sub_src = repo_dir.join(subpath);
        if !sub_src.exists() {
            anyhow::bail!("subpath not found in repo: {:?}", sub_src);
        }
        sub_src
    } else {
        // Multi-skill repo: find the matching skill by name.
        let candidates = installable_skills_in_repo(&repo_dir);
        if candidates.len() >= 2 {
            repo_dir.join(find_skill_by_name(&candidates, skill_name)?)
        } else {
            repo_dir.clone()
        }
    };

    copy_dir_recursive(&copy_src, dest_dir)
        .with_context(|| format!("copy {:?} -> {:?}", copy_src, dest_dir))?;
    Ok(Some(rev))
}

/// Find a skill's subpath by name within a multi-skill repo. Anything short
/// of one unambiguous match (no name, no hit, several hits) is the
/// `MultiSkills` condition: the caller must name the skill precisely.
fn find_skill_by_name<'a>(
    candidates: &'a [DiscoveredSkill],
    skill_name: Option<&str>,
) -> Result<&'a str> {
    let target_name = skill_name.ok_or_else(|| anyhow::anyhow!(SignalError::MultiSkills))?;
    match match_skill_candidate(target_name, candidates) {
        SkillMatch::Resolved(c) => Ok(c.subpath.as_str()),
        SkillMatch::Ambiguous(_) | SkillMatch::None => {
            Err(anyhow::anyhow!(SignalError::MultiSkills))
        }
    }
}

/// Guards the explore cache (`<central_dir>/.explore-cache`) while a preview
/// probes it and prepares a destination directory. Its own resource, its own
/// lock: the git cache is serialised separately inside `core::git_cache`.
static EXPLORE_CACHE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Clone a skill into the explore-cache for preview (no DB registration).
/// Delegates to `fetch_skill_files` for the actual download — the only
/// preview-specific logic is the explore-cache hit check.
pub fn clone_for_explore_preview(
    paths: &InstallerPaths,
    store: &SkillStore,
    source_url: &str,
    skill_name: Option<&str>,
    cancel: Option<&CancelToken>,
) -> Result<PathBuf> {
    let parsed = parse_github_url(source_url);

    let explore_cache_root = paths.central_dir.join(".explore-cache");
    std::fs::create_dir_all(&explore_cache_root).with_context(|| {
        format!(
            "failed to create explore-cache dir {:?}",
            explore_cache_root
        )
    })?;

    let cache_key = explore_preview_key(source_url, skill_name);
    let explore_skill_dir = explore_cache_root.join(&cache_key);

    // Serialise the explore-cache probe/prepare section against itself so two
    // previews of the same skill cannot race on the same directory. This is the
    // explore cache's own lock; the git cache has a separate, private one.
    {
        let lock = EXPLORE_CACHE_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = lock.lock().unwrap_or_else(|err| err.into_inner());

        if explore_skill_dir.exists() {
            let has_content = std::fs::read_dir(&explore_skill_dir)
                .ok()
                .map(|rd| {
                    rd.flatten()
                        .any(|e| e.file_name().to_string_lossy() != ".git")
                })
                .unwrap_or(false);
            if has_content {
                return Ok(explore_skill_dir);
            }
        }

        // Ensure a clean destination.
        if explore_skill_dir.exists() {
            let _ = std::fs::remove_dir_all(&explore_skill_dir);
        }
        std::fs::create_dir_all(&explore_skill_dir).with_context(|| {
            format!("failed to create explore skill dir {:?}", explore_skill_dir)
        })?;
    } // _guard dropped — lock released before download paths

    fetch_skill_files(
        &paths.cache_dir,
        store,
        &parsed,
        skill_name,
        &explore_skill_dir,
        cancel,
    )?;
    Ok(explore_skill_dir)
}

/// Backfill description for skills that have NULL description in the database.
/// Reads SKILL.md from the central_path of each skill.
pub fn backfill_skill_descriptions(store: &SkillStore) {
    let skills = match store.list_skills_missing_description() {
        Ok(s) => s,
        Err(_) => return,
    };
    for skill in skills {
        let central = std::path::Path::new(&skill.central_path);
        if let Some((_, Some(desc))) = find_skill_md(central).and_then(|md| parse_skill_md(&md)) {
            let _ = store.update_skill_description(&skill.id, Some(&desc));
        }
    }
}

#[cfg(test)]
#[path = "tests/installer.rs"]
mod tests;
