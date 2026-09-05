//! The git clone cache under `<cache_dir>/skills-hub-git-cache`.
//!
//! Owns the whole cache: the key scheme, the `.skills-hub-cache.json`
//! freshness metadata, the TTL probe, the corrupt-entry retry policy, and the
//! locks that serialise work on it. There is exactly one way in —
//! [`fetch_through_cache`] — and `subpath` on the request selects the sparse
//! fetcher.
//!
//! **One entry per repository and ref.** The key is the normalised clone URL
//! and branch — what identifies the bytes — never the subpath. How much of
//! the tree is checked out is a property of the entry's metadata
//! ([`Checkout`]), and a hit needs both freshness *and* a checkout that
//! covers the request: a full clone answers any later sparse request (the
//! Add flow's listing then install), a sparse one never answers a full
//! listing. An entry is only ever **widened** (union of sparse paths, or up
//! to the full tree), never narrowed — a fresh-but-uncovered request widens
//! the working tree in place without moving HEAD, a stale one refetches with
//! the widened shape. Never narrowing is what lets two skills of one
//! repository share an entry while a parallel Refresh copies them out.
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
use super::git_fetcher::{clone_or_pull, clone_or_pull_sparse, reshape_checkout};

/// Freshness record written next to each cached clone.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct RepoCacheMeta {
    last_fetched_ms: i64,
    head: Option<String>,
    /// How much of the tree the entry has checked out. A record without the
    /// field predates the field and was written by the full fetcher (sparse
    /// entries used to live under their own key), so it defaults to `Full`.
    #[serde(default)]
    checkout: Checkout,
}

/// What an entry's working tree contains.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Checkout {
    /// The whole tree.
    #[default]
    Full,
    /// Exactly these repo-relative paths (a sparse checkout).
    Sparse { subpaths: Vec<String> },
}

impl Checkout {
    fn requested(subpath: Option<&str>) -> Self {
        match subpath {
            Some(subpath) => Checkout::Sparse {
                subpaths: vec![normalize_subpath(subpath)],
            },
            None => Checkout::Full,
        }
    }

    /// Whether this checkout already contains what the request wants: the
    /// full tree contains everything, a sparse one contains a subpath at or
    /// below one of its own.
    fn covers(&self, subpath: Option<&str>) -> bool {
        match (self, subpath) {
            (Checkout::Full, _) => true,
            (Checkout::Sparse { .. }, None) => false,
            (Checkout::Sparse { subpaths }, Some(wanted)) => {
                let wanted = normalize_subpath(wanted);
                subpaths
                    .iter()
                    .any(|have| wanted == *have || wanted.starts_with(&format!("{have}/")))
            }
        }
    }

    /// The smallest checkout containing both this one and the request.
    fn widened_to(&self, subpath: Option<&str>) -> Self {
        match (self, subpath) {
            (Checkout::Full, _) | (Checkout::Sparse { .. }, None) => Checkout::Full,
            (Checkout::Sparse { subpaths }, Some(wanted)) => {
                let wanted = normalize_subpath(wanted);
                let mut union = subpaths.clone();
                if !union.contains(&wanted) {
                    union.push(wanted);
                }
                Checkout::Sparse { subpaths: union }
            }
        }
    }

    /// The sparse pattern set to hand the fetcher; `None` is the full tree.
    fn sparse_subpaths(&self) -> Option<Vec<&str>> {
        match self {
            Checkout::Full => None,
            Checkout::Sparse { subpaths } => Some(subpaths.iter().map(String::as_str).collect()),
        }
    }
}

fn normalize_subpath(subpath: &str) -> String {
    subpath.trim_matches('/').to_string()
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

/// The inputs that identify one cached clone: the bytes' origin, never the
/// subpath being asked for. Named so a caller cannot put a skill name in the
/// branch slot.
pub(crate) struct CacheKeyInputs<'a> {
    pub clone_url: &'a str,
    pub branch: Option<&'a str>,
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
        }
    }
}

/// Clone (or refresh) `req.clone_url` into the git cache under `cache_dir`,
/// returning the cached clone's directory and head revision.
///
/// Serves the cached head when the entry's metadata is within `req.ttl_ms`
/// and its checkout covers `req.subpath` (widening the working tree in place
/// first when it does not); otherwise fetches under this key's lock. A fetch
/// that fails against an existing entry is retried exactly once from a clean
/// directory, which is how a corrupt cache entry heals.
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

    let existing = read_meta(&repo_dir, &meta_path);
    let checkout = existing
        .as_ref()
        .map(|meta| meta.checkout.widened_to(req.subpath))
        .unwrap_or_else(|| Checkout::requested(req.subpath));

    if let Some((meta, head)) = existing
        .as_ref()
        .and_then(|meta| fresh_head(req.ttl_ms, meta).map(|head| (meta, head)))
    {
        if meta.checkout.covers(req.subpath) {
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

        // Fresh head, narrower tree: widen without moving HEAD. The record
        // keeps its fetch time — nothing was refetched, so nothing gets
        // fresher. A widening that fails takes the same road as a failed
        // fetch below.
        match reshape_checkout(&repo_dir, checkout.sparse_subpaths().as_deref(), req.cancel) {
            Ok(()) => {
                write_meta(
                    &meta_path,
                    &RepoCacheMeta {
                        last_fetched_ms: meta.last_fetched_ms,
                        head: Some(head.clone()),
                        checkout: checkout.clone(),
                    },
                );
                log_cache(
                    "hit (fresh, widened)",
                    &started,
                    req,
                    &format!("repo_dir={:?} checkout={:?}", repo_dir, checkout),
                );
                return Ok((repo_dir, head));
            }
            Err(err) if err.downcast_ref::<SignalError>() == Some(&SignalError::Cancelled) => {
                return Err(err);
            }
            Err(err) => {
                log::warn!("[installer] git cache widen failed, refetching: {:#}", err);
            }
        }
    }

    if req.cancel.is_some_and(|c| c.is_cancelled()) {
        anyhow::bail!(SignalError::Cancelled);
    }

    log_cache(
        "miss/stale; fetching",
        &started,
        req,
        &format!("repo_dir={:?} checkout={:?}", repo_dir, checkout),
    );

    let rev = match fetch_into(&repo_dir, req, &checkout) {
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
            fetch_into(&repo_dir, req, &checkout).with_context(|| format!("{:#}", err))?
        }
    };

    write_meta(
        &meta_path,
        &RepoCacheMeta {
            last_fetched_ms: now_ms(),
            head: Some(rev.clone()),
            checkout,
        },
    );

    log_cache("ready", &started, req, &format!("head={}", rev));
    Ok((repo_dir, rev))
}

/// The one place that picks a fetcher: sparse for a sparse checkout, full
/// clone otherwise. `checkout` is the shape the entry ends up with — the
/// request's own subpath widened by whatever the entry already held.
fn fetch_into(repo_dir: &Path, req: &FetchRequest, checkout: &Checkout) -> Result<String> {
    match checkout.sparse_subpaths() {
        Some(subpaths) => {
            clone_or_pull_sparse(req.clone_url, repo_dir, req.branch, &subpaths, req.cancel)
        }
        None => clone_or_pull(req.clone_url, repo_dir, req.branch, req.cancel),
    }
}

/// Stable cache-dir name for one set of [`CacheKeyInputs`].
pub(crate) fn repo_cache_key(inputs: &CacheKeyInputs) -> String {
    hash_key_parts(inputs.clone_url, inputs.branch, None)
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

/// The entry's metadata, when the clone exists and the record parses.
fn read_meta(repo_dir: &Path, meta_path: &Path) -> Option<RepoCacheMeta> {
    if !repo_dir.join(".git").exists() {
        return None;
    }
    let raw = std::fs::read_to_string(meta_path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// The cached head, when the metadata is within TTL.
fn fresh_head(ttl_ms: i64, meta: &RepoCacheMeta) -> Option<String> {
    let head = meta.head.clone()?;
    if ttl_ms > 0 && now_ms().saturating_sub(meta.last_fetched_ms) < ttl_ms {
        Some(head)
    } else {
        None
    }
}

fn write_meta(meta_path: &Path, meta: &RepoCacheMeta) {
    let _ = std::fs::write(
        meta_path,
        serde_json::to_string(meta).unwrap_or_else(|_| "{}".to_string()),
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
