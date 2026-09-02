//! The git clone cache under `<cache_dir>/skills-hub-git-cache`.
//!
//! Owns the whole cache: the key scheme, the `.skills-hub-cache.json`
//! freshness metadata, the TTL probe, the corrupt-entry retry policy, and the
//! locks that serialise work on it. There is exactly one way in —
//! [`fetch_through_cache`] — and `subpath` on the request selects the sparse
//! fetcher.
//!
//! The module takes no database handle: freshness is a **value** the caller
//! resolves (`settings::git_cache_ttl_ms`) and passes as `ttl_ms`, keeping the
//! shipped `0 = never fresh` semantics.
//!
//! The locks are **private to this module** — that is the point of the
//! boundary, the same shape `mutation_guard.rs` uses for Sync-target
//! mutations. They are plain non-reentrant `std::sync::Mutex`es acquired only
//! by the entry point below, which never calls itself; no caller outside can
//! hold one across a cache fetch because no caller outside can reach one.
//!
//! Locking is **per cache key**, not process-wide, so fetches of different
//! repositories run concurrently while fetches of the same repository still
//! serialise. That needs two levels, with one discipline:
//!
//! > **The lock-table mutex is never held across a fetch.** Take the table
//! > only long enough to get or insert the key's `Arc<Mutex<()>>`, release it,
//! > then lock the key.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::cancel_token::CancelToken;
use super::clock::now_ms;
use super::errors::SignalError;
use super::git_fetcher::{clone_or_pull, clone_or_pull_sparse};

/// Freshness record written next to each cached clone.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct RepoCacheMeta {
    last_fetched_ms: i64,
    head: Option<String>,
}

/// One `Arc<Mutex<()>>` per cache key. Entries are never removed: a cache key
/// is a bounded, long-lived identity, and dropping one would let two fetches
/// of the same repository race.
static LOCKS: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> = OnceLock::new();

/// The lock guarding one cache key. Crate-private so the module's own tests
/// can assert the concurrency rule; nothing else may call it.
pub(crate) fn key_lock(key: &str) -> Arc<Mutex<()>> {
    let table = LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = table.lock().unwrap_or_else(|err| err.into_inner());
    map.entry(key.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

/// The inputs that identify one cached clone. Named so a caller cannot put a
/// skill name in the branch slot.
pub(crate) struct CacheKeyInputs<'a> {
    pub clone_url: &'a str,
    pub branch: Option<&'a str>,
    pub subpath: Option<&'a str>,
}

/// One fetch-through-cache request. `subpath.is_some()` selects the sparse
/// fetcher; `ttl_ms` is the freshness window (`0` disables cache hits).
pub(crate) struct FetchRequest<'a> {
    pub clone_url: &'a str,
    pub branch: Option<&'a str>,
    pub subpath: Option<&'a str>,
    pub ttl_ms: i64,
    pub cancel: Option<&'a CancelToken>,
}

impl<'a> FetchRequest<'a> {
    fn key_inputs(&self) -> CacheKeyInputs<'a> {
        CacheKeyInputs {
            clone_url: self.clone_url,
            branch: self.branch,
            subpath: self.subpath,
        }
    }
}

/// Clone (or refresh) `req.clone_url` into the git cache under `cache_dir`,
/// returning the cached clone's directory and head revision.
///
/// Serves the cached head when the entry's metadata is within `req.ttl_ms`;
/// otherwise fetches under this key's lock. A fetch that fails against an
/// existing entry is retried exactly once from a clean directory, which is how
/// a corrupt cache entry heals.
pub(crate) fn fetch_through_cache(
    cache_dir: &Path,
    req: &FetchRequest,
) -> Result<(PathBuf, String)> {
    let started = std::time::Instant::now();
    let key = repo_cache_key(&req.key_inputs());
    let repo_dir = prepare_repo_dir(cache_dir, &key)?;
    let meta_path = repo_dir.join(".skills-hub-cache.json");

    let lock = key_lock(&key);
    let _guard = lock.lock().unwrap_or_else(|err| err.into_inner());

    if let Some(head) = fresh_head(req.ttl_ms, &repo_dir, &meta_path) {
        log_cache(
            "hit (fresh)",
            &started,
            req,
            &format!("repo_dir={:?}", repo_dir),
        );
        return Ok((repo_dir, head));
    }

    if req.cancel.is_some_and(|c| c.is_cancelled()) {
        anyhow::bail!(SignalError::Cancelled);
    }

    log_cache(
        "miss/stale; fetching",
        &started,
        req,
        &format!("repo_dir={:?}", repo_dir),
    );

    let rev = match fetch_into(&repo_dir, req) {
        Ok(rev) => rev,
        Err(err) => {
            // A cancelled fetch is a decision, not a corrupt cache: never
            // wipe the entry and retry on it.
            if err.downcast_ref::<SignalError>() == Some(&SignalError::Cancelled) {
                return Err(err);
            }
            // Otherwise assume the entry is unusable and retry once clean.
            if repo_dir.exists() {
                let _ = std::fs::remove_dir_all(&repo_dir);
            }
            fetch_into(&repo_dir, req).with_context(|| format!("{:#}", err))?
        }
    };

    write_meta(&meta_path, &rev);

    log_cache("ready", &started, req, &format!("head={}", rev));
    Ok((repo_dir, rev))
}

/// The one place that picks a fetcher: sparse when the request names a
/// subpath, full clone otherwise.
fn fetch_into(repo_dir: &Path, req: &FetchRequest) -> Result<String> {
    match req.subpath {
        Some(subpath) => {
            clone_or_pull_sparse(req.clone_url, repo_dir, req.branch, subpath, req.cancel)
        }
        None => clone_or_pull(req.clone_url, repo_dir, req.branch, req.cancel),
    }
}

/// Stable cache-dir name for one set of [`CacheKeyInputs`].
pub(crate) fn repo_cache_key(inputs: &CacheKeyInputs) -> String {
    hash_key_parts(inputs.clone_url, inputs.branch, inputs.subpath)
}

/// The explore-cache's preview-directory key. It shares this module's hash
/// scheme (and therefore its stability guarantee) but not its inputs: a
/// preview is keyed by source URL and skill name, never by branch.
pub(crate) fn explore_preview_key(source_url: &str, skill_name: Option<&str>) -> String {
    hash_key_parts(source_url, skill_name, None)
}

/// The shipped digest: `sha256(url \n second \n third)`, hex-encoded. Kept
/// private so the two named key functions above are the only way to build one.
fn hash_key_parts(first: &str, second: Option<&str>, third: Option<&str>) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(first.as_bytes());
    hasher.update(b"\n");
    if let Some(value) = second {
        hasher.update(value.as_bytes());
    }
    hasher.update(b"\n");
    if let Some(value) = third {
        hasher.update(value.as_bytes());
    }
    hex::encode(hasher.finalize())
}

/// Ensure the cache root exists and resolve this key's cache dir.
fn prepare_repo_dir(cache_dir: &Path, key: &str) -> Result<PathBuf> {
    let cache_root = cache_dir.join("skills-hub-git-cache");
    std::fs::create_dir_all(&cache_root)
        .with_context(|| format!("failed to create cache dir {:?}", cache_root))?;
    Ok(cache_root.join(key))
}

/// The cached head, when the clone exists and its metadata is within TTL.
fn fresh_head(ttl_ms: i64, repo_dir: &Path, meta_path: &Path) -> Option<String> {
    if !repo_dir.join(".git").exists() {
        return None;
    }
    let raw = std::fs::read_to_string(meta_path).ok()?;
    let meta: RepoCacheMeta = serde_json::from_str(&raw).ok()?;
    let head = meta.head?;
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

/// One log shape for both variants: the sparse fetch only changes the label
/// and adds the subpath field.
fn log_cache(stage: &str, started: &std::time::Instant, req: &FetchRequest, tail: &str) {
    let label = if req.subpath.is_some() {
        "sparse git cache"
    } else {
        "git cache"
    };
    let sub = req
        .subpath
        .map(|s| format!(" subpath={}", s))
        .unwrap_or_default();
    log::info!(
        "[installer] {} {} {}s url={} branch={:?}{} {}",
        label,
        stage,
        started.elapsed().as_secs_f32(),
        req.clone_url,
        req.branch,
        sub,
        tail
    );
}

#[cfg(test)]
#[path = "tests/git_cache.rs"]
mod tests;
