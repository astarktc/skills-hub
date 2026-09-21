use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result};

use super::cancel_token::CancelToken;
use super::central_repo::ensure_central_repo;
use super::errors::SignalError;
pub use super::git_acquisition::GitSourceResolution;
use super::git_acquisition::{
    acquire, parse_github_url, AcquireRequest, GitSelection, GithubApi, HttpGithubApi, SkillIntent,
};
use super::git_cache::{explore_preview_key, fetch_through_cache, FetchRequest};
pub use super::install_finalize::InstallResult;
use super::install_finalize::{
    ensure_name_available, finalize_install, NameIntent, SkillProvenance, StagingDir,
};
use super::skill_discovery::{
    discover_skills, find_skill_md, is_skill_dir, parse_skill_md, parse_skill_md_with_reason,
    require_skill_md, DiscoveredSkill,
};
use super::skill_lock::try_enrich_from_skill_lock_with_home;
use super::skill_matching::{match_skill_candidate, CandidateMatch, MatchableSkill};
use super::skill_store::SkillStore;
use super::sync_engine::copy_dir_recursive;
use super::tool_adapters::{tool_holding_path, ToolAdapter};

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

/// Add a skill from an independent local folder the operator maintains.
/// The folder stays the skill's source (`local` provenance), so Update can
/// copy from it again — which is why a folder inside a Tool's skills
/// directory is refused before anything is copied: the app would later
/// overwrite or remove that source, so a Tool's copy is Import's business
/// (`install_imported_skill`).
pub fn install_local_skill(
    paths: &InstallerPaths,
    store: &SkillStore,
    source_path: &Path,
    name: Option<String>,
) -> Result<InstallResult> {
    if let Some(holder) = tool_holding_path(&paths.home, source_path) {
        anyhow::bail!(local_source_inside_tool_dir(source_path, holder));
    }
    install_from_dir(paths, store, source_path, name, || {
        SkillProvenance::local(source_path)
    })
}

/// Take over a skill found in a Tool's skills directory (Onboarding import).
/// Recorded with `imported` provenance: the path it was copied from is the
/// import's own target — about to be overwritten with a link or removed —
/// so it is **not** a source; `found_in_tool` is kept as display-only
/// history (ADR-0003). The `.skill-lock.json` enrichment still wins: a copy
/// `npx skills add` installed has a real upstream and is recorded `git`.
pub fn install_imported_skill(
    paths: &InstallerPaths,
    store: &SkillStore,
    source_path: &Path,
    name: Option<String>,
    found_in_tool: Option<&str>,
) -> Result<InstallResult> {
    install_from_dir(paths, store, source_path, name, || {
        SkillProvenance::imported(found_in_tool)
    })
}

/// Copy a directory into the central repo and record it. `provenance` is
/// what the record says when `~/.agents/.skill-lock.json` has nothing to say
/// about the path (a discovered upstream always wins).
fn install_from_dir(
    paths: &InstallerPaths,
    store: &SkillStore,
    source_path: &Path,
    name: Option<String>,
    provenance: impl FnOnce() -> SkillProvenance,
) -> Result<InstallResult> {
    if !source_path.exists() {
        anyhow::bail!(source_path_missing(source_path));
    }
    // Skill discovery owns the admission rule: no `SKILL.md`, no skill.
    require_skill_md(source_path)?;

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
            imported_from_tool: None,
        },
        None => provenance(),
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

/// The typed condition for an Add → local folder pointed at a Tool's own
/// copy (`tool_adapters::tool_holding_path` said which Tool).
fn local_source_inside_tool_dir(path: &Path, holder: &ToolAdapter) -> SignalError {
    SignalError::LocalSourceInsideToolDir {
        path: path.to_string_lossy().to_string(),
        tool: holder.key().to_string(),
    }
}

/// The typed condition for an external source folder that is not there,
/// raised wherever an install or Update reads one.
fn source_path_missing(path: &Path) -> SignalError {
    SignalError::SourcePathMissing {
        path: path.to_string_lossy().to_string(),
    }
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

/// The single manifest gate for bytes about to become a central copy: install
/// and Re-point both refuse a directory without a `SKILL.md` with this token.
pub(crate) fn ensure_installable_skill_dir(p: &Path) -> Result<()> {
    if is_skill_dir(p) {
        Ok(())
    } else {
        anyhow::bail!(SignalError::SkillInvalid {
            reason: "missing_skill_md".to_string(),
        });
    }
}

#[derive(Clone, Debug, serde::Serialize, specta::Type)]
pub struct GitSkillCandidate {
    pub name: String,
    pub description: Option<String>,
    pub subpath: String,
    pub resolution: Option<GitSourceResolution>,
}

#[derive(Clone, Debug, serde::Serialize, specta::Type)]
pub struct LocalSkillCandidate {
    pub name: String,
    pub description: Option<String>,
    pub subpath: String,
    pub valid: bool,
    pub reason: Option<String>,
}

/// Git listing adapter: the git side admits any installable candidate;
/// validity is not surfaced on this wire shape.
fn git_candidate(c: DiscoveredSkill, resolution: &GitSourceResolution) -> GitSkillCandidate {
    GitSkillCandidate {
        name: c.name,
        description: c.description,
        subpath: c.subpath,
        resolution: Some(resolution.clone()),
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
    list_git_skills_with(
        repo_url,
        target_name,
        &HttpGithubApi::new(super::settings::github_token_or_none(store)),
        |parsed| {
            fetch_through_cache(
                &paths.cache_dir,
                &FetchRequest {
                    clone_url: &parsed.clone_url,
                    branch: parsed.branch.as_deref(),
                    subpath: None,
                    ttl_ms: super::settings::git_cache_ttl_ms(store),
                    cancel: None,
                },
            )
            .map(|(dir, _revision)| dir)
        },
    )
}

fn list_git_skills_with(
    repo_url: &str,
    target_name: Option<&str>,
    api: &dyn GithubApi,
    checkout: impl FnOnce(&super::git_acquisition::GitSource) -> Result<PathBuf>,
) -> Result<GitSkillListing> {
    let (found, resolution) =
        super::git_acquisition::list_candidates_with(&parse_github_url(repo_url), api, checkout)?;
    let candidates: Vec<_> = found
        .into_iter()
        .map(|c| git_candidate(c, &resolution))
        .collect();
    let target_match = target_name.map(|target| match_skill_candidate(target, &candidates).into());
    Ok(GitSkillListing {
        candidates,
        target_match,
    })
}

/// Local listing adapter: every discovered candidate is shown, with its
/// validity and reason, so the picker can explain why a folder under a
/// declared skills dir is not selectable.
pub fn list_local_skills(base_path: &Path) -> Result<Vec<LocalSkillCandidate>> {
    if !base_path.exists() {
        anyhow::bail!(source_path_missing(base_path));
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

/// Legacy null-resolution fixture adapter. Production uses the named
/// selection door below; this keeps existing cross-module fixtures compatible.
#[cfg(test)]
pub fn install_git_skill_from_selection(
    paths: &InstallerPaths,
    store: &SkillStore,
    repo_url: &str,
    subpath: &str,
    name: Option<String>,
    cancel: Option<&CancelToken>,
) -> Result<InstallResult> {
    install_git_skill_from_selection_with(
        paths,
        store,
        repo_url,
        subpath,
        name,
        cancel,
        &HttpGithubApi::new(super::settings::github_token_or_none(store)),
    )
}

/// [`install_git_skill_from_selection`] with the GitHub adapter injected, so
/// the install path's fast-path wiring is testable without HTTP.
#[cfg(test)]
pub(crate) fn install_git_skill_from_selection_with(
    paths: &InstallerPaths,
    store: &SkillStore,
    repo_url: &str,
    subpath: &str,
    name: Option<String>,
    cancel: Option<&CancelToken>,
    api: &dyn GithubApi,
) -> Result<InstallResult> {
    install_git_selection_with(
        paths,
        store,
        repo_url,
        GitSelection {
            subpath: Some(subpath),
            resolution: None,
        },
        name,
        cancel,
        api,
    )
}

/// Install using the listing's exact source decision, without another refs lookup.
pub fn install_git_skill_from_listing(
    paths: &InstallerPaths,
    store: &SkillStore,
    repo_url: &str,
    selection: GitSelection,
    name: Option<String>,
    cancel: Option<&CancelToken>,
) -> Result<InstallResult> {
    install_git_selection_with(
        paths,
        store,
        repo_url,
        selection,
        name,
        cancel,
        &HttpGithubApi::new(super::settings::github_token_or_none(store)),
    )
}

fn install_git_selection_with(
    paths: &InstallerPaths,
    store: &SkillStore,
    repo_url: &str,
    selection: GitSelection,
    name: Option<String>,
    cancel: Option<&CancelToken>,
    api: &dyn GithubApi,
) -> Result<InstallResult> {
    let source = parse_github_url(repo_url);
    let subpath = selection
        .subpath
        .context("install requires an explicit selection")?;
    let name = name_intent(name, || {
        derive_name_from_subpath(&source.clone_url, Some(subpath))
    });

    let central_dir = &paths.central_dir;
    ensure_central_repo(central_dir)?;
    ensure_name_available(central_dir, name.requested())?;

    let staged = StagingDir::new_in(central_dir);
    let request = AcquireRequest {
        source: &source,
        intent: SkillIntent::Selection(selection),
        dest: staged.path(),
        cache_dir: &paths.cache_dir,
        ttl_ms: super::settings::git_cache_ttl_ms(store),
        cancel,
        allow_fast_path: true,
    };
    let acquired = acquire(&request, api)?;
    // The selection has to be a skill, whichever adapter delivered it.
    ensure_installable_skill_dir(staged.path())?;

    let source_subpath = acquired.resolved_subpath.filter(|subpath| subpath != ".");
    finalize_install(
        store,
        central_dir,
        staged,
        name,
        SkillProvenance::git(repo_url, source_subpath, Some(acquired.revision)),
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
        anyhow::bail!(source_path_missing(base_path));
    }

    let selected_dir = if subpath == "." {
        base_path.to_path_buf()
    } else {
        base_path.join(subpath)
    };
    if !selected_dir.exists() {
        anyhow::bail!(source_path_missing(&selected_dir));
    }

    let skill_md = require_skill_md(&selected_dir)?;
    let (parsed_name, _desc) = parse_skill_md_with_reason(&skill_md).map_err(|reason| {
        anyhow::anyhow!(SignalError::SkillInvalid {
            reason: reason.to_string(),
        })
    })?;

    let display_name = name.unwrap_or(parsed_name);

    install_local_skill(paths, store, &selected_dir, Some(display_name))
}

/// Guards the explore cache (`<central_dir>/.explore-cache`) while a preview
/// probes or publishes a completed preview. Never held across acquisition;
/// the git cache is serialised separately inside `core::git_cache`.
static EXPLORE_CACHE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Acquire a skill into the explore-cache for preview (no DB registration).
///
/// An adapter over `core::git_acquisition`: the only preview-specific logic
/// is cache probing and atomic publication of a privately acquired tree.
pub fn clone_for_explore_preview(
    paths: &InstallerPaths,
    store: &SkillStore,
    source_url: &str,
    skill_name: Option<&str>,
    cancel: Option<&CancelToken>,
) -> Result<PathBuf> {
    clone_for_explore_preview_with(paths, source_url, skill_name, cancel, |dest| {
        acquire(
            &AcquireRequest {
                source: &parse_github_url(source_url),
                intent: SkillIntent::ByName(skill_name),
                dest,
                cache_dir: &paths.cache_dir,
                ttl_ms: super::settings::git_cache_ttl_ms(store),
                cancel,
                allow_fast_path: true,
            },
            &HttpGithubApi::new(super::settings::github_token_or_none(store)),
        )
        .map(|_| ())
    })
}

/// Private acquisition adapter: tests may pause or fail while writing bytes.
fn clone_for_explore_preview_with(
    paths: &InstallerPaths,
    source_url: &str,
    skill_name: Option<&str>,
    cancel: Option<&CancelToken>,
    acquire_to: impl FnOnce(&Path) -> Result<()>,
) -> Result<PathBuf> {
    let root = paths.central_dir.join(".explore-cache");
    std::fs::create_dir_all(&root).with_context(|| format!("create explore cache {root:?}"))?;
    let public = root.join(explore_preview_key(source_url, skill_name));
    let lock = EXPLORE_CACHE_LOCK.get_or_init(|| Mutex::new(()));
    {
        let _guard = lock.lock().unwrap_or_else(|err| err.into_inner());
        if preview_has_content(&public) {
            return Ok(public);
        }
    }
    let check_cancelled = || -> Result<()> {
        if cancel.is_some_and(|cancel| cancel.is_cancelled()) {
            anyhow::bail!(SignalError::Cancelled);
        }
        Ok(())
    };
    check_cancelled()?;
    // Sibling means rename stays on the same filesystem. RAII cleans failure,
    // cancellation and losing acquisitions; no public path is used as scratch.
    let private = StagingDir::new_in(&root);
    std::fs::create_dir(private.path())?;
    acquire_to(private.path())?;
    check_cancelled()?;
    // An empty checkout is not a preview and must not become a false cache hit.
    if !preview_has_content(private.path()) {
        anyhow::bail!(SignalError::SkillInvalid {
            reason: "missing_skill_md".into()
        });
    }
    {
        let _guard = lock.lock().unwrap_or_else(|err| err.into_inner());
        check_cancelled()?;
        if preview_has_content(&public) {
            return Ok(public);
        }
        // Only a pre-existing empty/.git-only placeholder can be removed.
        // Completed winners are never replaced, even by a newer acquisition.
        if public.exists() {
            std::fs::remove_dir_all(&public)?;
        }
        std::fs::rename(private.path(), &public)
            .with_context(|| format!("publish explore preview {public:?}"))?;
    }
    Ok(public)
}

fn preview_has_content(path: &Path) -> bool {
    std::fs::read_dir(path)
        .ok()
        .is_some_and(|entries| entries.flatten().any(|entry| entry.file_name() != ".git"))
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
