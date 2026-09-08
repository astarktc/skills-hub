use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result};

use super::cancel_token::CancelToken;
use super::central_repo::ensure_central_repo;
use super::clock::now_ms;
use super::errors::SignalError;
use super::git_acquisition::{
    acquire, parse_github_url, AcquireRequest, GithubApi, HttpGithubApi, SkillIntent,
};
use super::git_cache::{explore_preview_key, fetch_through_cache, FetchRequest};
pub use super::install_finalize::InstallResult;
use super::install_finalize::{
    ensure_name_available, finalize_install, finalize_update, NameIntent, SkillProvenance,
    StagingDir,
};
use super::propagation::{propagate_unlocked, PropagationReport};
use super::provenance::{is_refreshable, Provenance};
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

/// The typed condition for an Update of a skill that has no external source
/// (`provenance::is_refreshable` said no).
fn not_refreshable(record: &super::skill_store::SkillRecord) -> SignalError {
    SignalError::NotRefreshable {
        name: record.name.clone(),
    }
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
    pub edit_conflict: Option<super::skill_edits::InvocationEditConflict>,
}

/// Re-acquire a Managed skill's bytes from its source into a Staging dir.
///
/// Acquisition (git clone / local copy) runs **outside** the mutation guard:
/// it touches no Sync target and can be slow. Only finalize + Propagation,
/// which do, run inside it (see [`finalize_and_propagate_unlocked`]).
/// One self-contained result per skill, which is what lets the Refresh batch
/// run this over a bounded parallel pool (`core::refresh`).
///
/// The git side is one call into `core::git_acquisition`: the fast path, the
/// clone fallback, sparse fetching, cancellation and the legacy subpath
/// backfill all live there. This adapter only picks the destination and
/// records what came back.
///
/// The GitHub adapter and the git-cache freshness window are **parameters**,
/// not per-skill settings reads: the Refresh batch resolves both once and
/// hands every pool worker its own adapter (`GithubApi` carries no `Send`
/// bound).
pub(crate) fn acquire_managed_skill_update_with(
    paths: &InstallerPaths,
    store: &SkillStore,
    skill_id: &str,
    cancel: Option<&CancelToken>,
    api: &dyn GithubApi,
    ttl_ms: i64,
) -> Result<AcquiredUpdate> {
    acquire_managed_skill_update_from(paths, store, skill_id, cancel, api, ttl_ms, None)
}

/// Update's acquisition adapter with an optional, validated git source override.
/// The overridden record stays in memory until the normal finalize step succeeds.
#[allow(clippy::too_many_arguments)]
pub(crate) fn acquire_managed_skill_update_from(
    paths: &InstallerPaths,
    store: &SkillStore,
    skill_id: &str,
    cancel: Option<&CancelToken>,
    api: &dyn GithubApi,
    ttl_ms: i64,
    source_override: Option<(&str, &super::git_acquisition::GitSource)>,
) -> Result<AcquiredUpdate> {
    let mut record = store.get_skill_by_id(skill_id)?.ok_or_else(|| {
        anyhow::anyhow!(SignalError::NotFound {
            kind: "skill".to_string(),
            id: skill_id.to_string(),
        })
    })?;

    // The Provenance rule: a skill with no external source has nothing to
    // acquire, whatever the state of its central copy. The central copy's
    // own presence is deliberately *not* checked: a `git`/`local` skill
    // whose central copy is gone is exactly what Restore re-acquires, and
    // finalize rebuilds it at the recorded path.
    if !is_refreshable(&record) {
        anyhow::bail!(not_refreshable(&record));
    }

    let central_path = PathBuf::from(record.central_path.clone());
    let central_parent = central_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid central path"))?
        .to_path_buf();
    // The central repo itself may be gone too (a Restore after the whole
    // library folder was lost); the staging dir needs its parent.
    ensure_central_repo(&central_parent)?;

    // Build new content in a sibling staging dir; finalize swaps it in.
    let staged = StagingDir::new_in(&central_parent);
    let staging_dir = staged.path().to_path_buf();

    let mut new_revision: Option<String> = None;

    match Provenance::parse(&record.source_type) {
        Some(Provenance::Git) => {
            let repo_url = record
                .source_ref
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("missing source_ref for git skill"))?;
            let source = source_override
                .map(|(_, source)| source.clone())
                .unwrap_or_else(|| parse_github_url(repo_url));

            // Stored paths are explicit selections. URL paths stay on the source
            // so acquisition can correct their branch boundary before using them.
            let intent = if source_override.is_some() {
                SkillIntent::NamedSkill(Some(&record.name))
            } else if let Some(subpath) = record.source_subpath.as_deref() {
                SkillIntent::Subpath(subpath)
            } else {
                SkillIntent::NamedSkillOrWholeRepo(&record.name)
            };

            let acquired = acquire(
                &AcquireRequest {
                    source: &source,
                    intent,
                    stored_subpath: if source_override.is_some() {
                        None
                    } else {
                        record.source_subpath.as_deref()
                    },
                    dest: &staging_dir,
                    cache_dir: &paths.cache_dir,
                    ttl_ms,
                    cancel,
                    allow_fast_path: true,
                },
                api,
            )?;
            new_revision = Some(acquired.revision);

            if let Some((url, _)) = source_override {
                ensure_installable_skill_dir(&staging_dir)?;
                record.source_ref = Some(url.to_string());
            }
            // Acquisition owns the branch/path split. Finalize carries this
            // resolved path into the record, including legacy backfills.
            record.source_subpath = acquired.resolved_subpath.filter(|subpath| subpath != ".");
        }
        Some(Provenance::Local) => {
            let source = record
                .source_ref
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("missing source_ref for local skill"))?;
            let source_path = PathBuf::from(source);
            if !source_path.exists() {
                anyhow::bail!(source_path_missing(&source_path));
            }
            copy_dir_recursive(&source_path, &staging_dir)
                .with_context(|| format!("copy {:?} -> {:?}", source_path, staging_dir))?;
        }
        // Excluded by the predicate above; restated so the match is total.
        Some(Provenance::Imported) | None => anyhow::bail!(not_refreshable(&record)),
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

    let mut updated = finalize_update(store, &record, staged, new_revision.clone())?;
    let edit_conflict = super::skill_edits::replay_unlocked(store, &mut updated)?;
    let content_hash = updated.content_hash.clone();

    let propagation = propagate_unlocked(store, paths, &record.id, content_hash.as_deref(), now)?;

    Ok(UpdateOutcome {
        skill_id: record.id,
        name: record.name,
        content_hash,
        source_revision: new_revision,
        propagation,
        edit_conflict,
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
    let parsed = super::git_acquisition::resolve_tree_source(
        &parse_github_url(repo_url),
        None,
        &HttpGithubApi::new(super::settings::github_token(store)?),
    );
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

/// Install one selected skill from a git source.
///
/// An adapter over `core::git_acquisition`: it chooses the Staging dir as the
/// destination and hands the acquired revision to finalize. The GitHub API
/// fast path (with the real commit SHA), the clone fallback, sparse fetching
/// and cancellation come with the acquisition module.
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
        &HttpGithubApi::new(super::settings::github_token(store)?),
    )
}

/// [`install_git_skill_from_selection`] with the GitHub adapter injected, so
/// the install path's fast-path wiring is testable without HTTP.
pub(crate) fn install_git_skill_from_selection_with(
    paths: &InstallerPaths,
    store: &SkillStore,
    repo_url: &str,
    subpath: &str,
    name: Option<String>,
    cancel: Option<&CancelToken>,
    api: &dyn GithubApi,
) -> Result<InstallResult> {
    let source = parse_github_url(repo_url);
    let name = name_intent(name, || {
        derive_name_from_subpath(&source.clone_url, Some(subpath))
    });

    let central_dir = &paths.central_dir;
    ensure_central_repo(central_dir)?;
    ensure_name_available(central_dir, name.requested())?;

    let staged = StagingDir::new_in(central_dir);
    let acquired = acquire(
        &AcquireRequest {
            source: &source,
            intent: SkillIntent::Subpath(subpath),
            stored_subpath: None,
            dest: staged.path(),
            cache_dir: &paths.cache_dir,
            ttl_ms: super::settings::git_cache_ttl_ms(store),
            cancel,
            allow_fast_path: true,
        },
        api,
    )?;
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
/// probes it and prepares a destination directory. Its own resource, its own
/// lock: the git cache is serialised separately inside `core::git_cache`.
static EXPLORE_CACHE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Acquire a skill into the explore-cache for preview (no DB registration).
///
/// An adapter over `core::git_acquisition`: the only preview-specific logic
/// is the explore-cache hit check and the destination it prepares.
pub fn clone_for_explore_preview(
    paths: &InstallerPaths,
    store: &SkillStore,
    source_url: &str,
    skill_name: Option<&str>,
    cancel: Option<&CancelToken>,
) -> Result<PathBuf> {
    let source = parse_github_url(source_url);

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
    } // _guard dropped — lock released before acquisition

    acquire(
        &AcquireRequest {
            source: &source,
            intent: SkillIntent::NamedSkill(skill_name),
            stored_subpath: None,
            dest: &explore_skill_dir,
            cache_dir: &paths.cache_dir,
            ttl_ms: super::settings::git_cache_ttl_ms(store),
            cancel,
            allow_fast_path: true,
        },
        &HttpGithubApi::new(super::settings::github_token(store)?),
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
