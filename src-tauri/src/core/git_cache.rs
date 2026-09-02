//! The git clone cache under `<cache_dir>/skills-hub-git-cache`.
//!
//! Owns the whole cache: the key scheme, the `.skills-hub-cache.json` freshness
//! metadata, the TTL probe, and the process-wide mutex that serialises work on
//! it. The mutex is **private to this module** — that is the point of the
//! boundary. It is a plain non-reentrant `std::sync::Mutex` acquired only by the
//! two entry points below, which never call each other; no caller outside can
//! hold it across a cache fetch because no caller outside can reach it.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::cancel_token::CancelToken;
use super::clock::now_ms;
use super::git_fetcher::{clone_or_pull, clone_or_pull_sparse};
use super::settings;
use super::skill_store::SkillStore;

/// Freshness record written next to each cached clone.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct RepoCacheMeta {
    last_fetched_ms: i64,
    head: Option<String>,
}

static GIT_CACHE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Clone (or refresh) `clone_url` into the git cache under `cache_dir`.
pub(crate) fn clone_to_cache(
    cache_dir: &Path,
    store: &SkillStore,
    clone_url: &str,
    branch: Option<&str>,
    cancel: Option<&CancelToken>,
) -> Result<(PathBuf, String)> {
    let started = std::time::Instant::now();
    let repo_dir = prepare_repo_dir(cache_dir, clone_url, branch, None)?;
    let meta_path = repo_dir.join(".skills-hub-cache.json");

    let lock = GIT_CACHE_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock.lock().unwrap_or_else(|err| err.into_inner());

    if let Some(head) = fresh_head(store, &repo_dir, &meta_path) {
        log_cache(
            "hit (fresh)",
            &started,
            clone_url,
            branch,
            None,
            &format!("repo_dir={:?}", repo_dir),
        );
        return Ok((repo_dir, head));
    }

    log_cache(
        "miss/stale; fetching",
        &started,
        clone_url,
        branch,
        None,
        &format!("repo_dir={:?}", repo_dir),
    );

    let rev = match clone_or_pull(clone_url, &repo_dir, branch, cancel) {
        Ok(rev) => rev,
        Err(err) => {
            // If cache got corrupted, retry once from a clean state.
            if repo_dir.exists() {
                let _ = std::fs::remove_dir_all(&repo_dir);
            }
            clone_or_pull(clone_url, &repo_dir, branch, cancel)
                .with_context(|| format!("{:#}", err))?
        }
    };

    write_meta(&meta_path, &rev);

    log_cache(
        "ready",
        &started,
        clone_url,
        branch,
        None,
        &format!("head={}", rev),
    );
    Ok((repo_dir, rev))
}

/// Sparse variant of [`clone_to_cache`] fetching only `subpath`.
pub(crate) fn clone_to_cache_subpath(
    cache_dir: &Path,
    store: &SkillStore,
    clone_url: &str,
    branch: Option<&str>,
    subpath: &str,
    cancel: Option<&CancelToken>,
) -> Result<(PathBuf, String)> {
    let started = std::time::Instant::now();
    let repo_dir = prepare_repo_dir(cache_dir, clone_url, branch, Some(subpath))?;
    let meta_path = repo_dir.join(".skills-hub-cache.json");

    let lock = GIT_CACHE_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock.lock().unwrap_or_else(|err| err.into_inner());

    if let Some(head) = fresh_head(store, &repo_dir, &meta_path) {
        log_cache(
            "hit (fresh)",
            &started,
            clone_url,
            branch,
            Some(subpath),
            &format!("repo_dir={:?}", repo_dir),
        );
        return Ok((repo_dir, head));
    }

    log_cache(
        "miss/stale; fetching",
        &started,
        clone_url,
        branch,
        Some(subpath),
        &format!("repo_dir={:?}", repo_dir),
    );

    let rev = match clone_or_pull_sparse(clone_url, &repo_dir, branch, subpath, cancel) {
        Ok(rev) => rev,
        Err(err) => {
            if repo_dir.exists() {
                let _ = std::fs::remove_dir_all(&repo_dir);
            }
            clone_or_pull_sparse(clone_url, &repo_dir, branch, subpath, cancel)
                .with_context(|| format!("{:#}", err))?
        }
    };

    write_meta(&meta_path, &rev);

    log_cache(
        "ready",
        &started,
        clone_url,
        branch,
        Some(subpath),
        &format!("head={}", rev),
    );
    Ok((repo_dir, rev))
}

/// Stable cache-dir name for one (url, branch, subpath) triple. Also used by
/// the explore-cache to key its own preview directories.
pub(crate) fn repo_cache_key(
    clone_url: &str,
    branch: Option<&str>,
    subpath: Option<&str>,
) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(clone_url.as_bytes());
    hasher.update(b"\n");
    if let Some(b) = branch {
        hasher.update(b.as_bytes());
    }
    hasher.update(b"\n");
    if let Some(s) = subpath {
        hasher.update(s.as_bytes());
    }
    hex::encode(hasher.finalize())
}

/// Ensure the cache root exists and resolve this repo's cache dir.
fn prepare_repo_dir(
    cache_dir: &Path,
    clone_url: &str,
    branch: Option<&str>,
    subpath: Option<&str>,
) -> Result<PathBuf> {
    let cache_root = cache_dir.join("skills-hub-git-cache");
    std::fs::create_dir_all(&cache_root)
        .with_context(|| format!("failed to create cache dir {:?}", cache_root))?;
    Ok(cache_root.join(repo_cache_key(clone_url, branch, subpath)))
}

/// The cached head, when the clone exists and its metadata is within TTL.
fn fresh_head(store: &SkillStore, repo_dir: &Path, meta_path: &Path) -> Option<String> {
    if !repo_dir.join(".git").exists() {
        return None;
    }
    let raw = std::fs::read_to_string(meta_path).ok()?;
    let meta: RepoCacheMeta = serde_json::from_str(&raw).ok()?;
    let head = meta.head?;
    let ttl_ms = settings::git_cache_ttl_secs(store).saturating_mul(1000);
    if ttl_ms > 0 && now_ms().saturating_sub(meta.last_fetched_ms) < ttl_ms {
        Some(head)
    } else {
        None
    }
}

fn write_meta(meta_path: &Path, rev: &str) {
    let _ = std::fs::write(
        meta_path,
        serde_json::to_string(&RepoCacheMeta {
            last_fetched_ms: now_ms(),
            head: Some(rev.to_string()),
        })
        .unwrap_or_else(|_| "{}".to_string()),
    );
}

/// One log shape for both variants: the sparse fetch only changes the label and
/// adds the subpath field.
fn log_cache(
    stage: &str,
    started: &std::time::Instant,
    clone_url: &str,
    branch: Option<&str>,
    subpath: Option<&str>,
    tail: &str,
) {
    let label = if subpath.is_some() {
        "sparse git cache"
    } else {
        "git cache"
    };
    let sub = subpath
        .map(|s| format!(" subpath={}", s))
        .unwrap_or_default();
    log::info!(
        "[installer] {} {} {}s url={} branch={:?}{} {}",
        label,
        stage,
        started.elapsed().as_secs_f32(),
        clone_url,
        branch,
        sub,
        tail
    );
}
