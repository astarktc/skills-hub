use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};

use super::clock::now_ms;
use super::git_cache::with_idle_entry;
use super::removal_fence::{ensure_clear_of_library, Claim, Fence, OwnedDir, ProtectedLibrary};
use super::skill_store::SkillStore;
use super::sync_engine::remove_path_any;
use serde::{Deserialize, Serialize};
use tauri::Manager;

const CACHE_DIR_NAME: &str = "skills-hub-git-cache";
const CACHE_META_FILE: &str = ".skills-hub-cache.json";

#[derive(Debug, Deserialize)]
struct RepoCacheMeta {
    last_fetched_ms: i64,
}

/// What one cleanup pass did: entries removed, and entries due for removal
/// that were skipped because an acquisition was using them — fetching into
/// one (its key lock held) or reading a checkout of it (`git_cache`'s
/// `with_idle_entry` is the rule).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, specta::Type)]
pub struct GitCacheCleanup {
    pub cleared: usize,
    pub in_use: usize,
}

/// Age out the git cache under the app cache dir. `central_dir` is the
/// configured central repo the command seam resolved: with `store`'s records
/// it is the library no reclaimed entry may reach.
pub fn cleanup_git_cache_dirs<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    store: &SkillStore,
    central_dir: &Path,
    max_age: Duration,
) -> Result<GitCacheCleanup> {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .context("failed to resolve app cache dir")?;
    cleanup_git_cache_dirs_in(
        &cache_dir,
        &ProtectedLibrary::new(store, central_dir),
        max_age,
    )
}

/// The age-out over the cache under `cache_dir` — the trusted boundary. The
/// cache root is the app's only when it is reached from that boundary through
/// real directories ([`OwnedDir`]): an alias anywhere on the way is refused
/// whole, and so is a cache root that overlaps the library (a central repo
/// configured inside or around it) — cleanup refuses rather than deletes.
/// Every entry is then removed as a direct child of that admitted physical
/// root; a refused entry is skipped like any failed one.
///
/// **The in-use rule.** An entry due for removal is removed only under its
/// key lock, taken with a try-lock ([`with_idle_entry`]; the entry's
/// directory name is its key): a held lock — or a checkout of the entry still
/// being read — means an acquisition is using it, so it is skipped and
/// counted `in_use`: never removed, never waited on.
/// The cleanup takes no Mutation guard: acquisitions run outside it.
fn cleanup_git_cache_dirs_in(
    cache_dir: &Path,
    library: &ProtectedLibrary<'_>,
    max_age: Duration,
) -> Result<GitCacheCleanup> {
    let relative = Path::new(CACHE_DIR_NAME);
    let Some(cache_root) = OwnedDir::resolve(cache_dir, relative)? else {
        return Ok(GitCacheCleanup::default());
    };
    ensure_clear_of_library(cache_root.path(), library)?;

    let cutoff_ms = now_ms().saturating_sub(max_age.as_millis().try_into().unwrap_or(i64::MAX));
    let cutoff_time = SystemTime::now()
        .checked_sub(max_age)
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let fence = Fence::new(library).claiming(Claim::ChildOfOwned(&cache_root));
    let mut cleanup = GitCacheCleanup::default();
    let rd = match std::fs::read_dir(cache_root.path()) {
        Ok(v) => v,
        Err(err) => {
            return Err(anyhow::anyhow!(
                "failed to read cache dir {:?}: {}",
                cache_root.path(),
                err
            ));
        }
    };

    let is_due = |path: &Path| is_due(path, cutoff_ms, cutoff_time);
    for entry in rd.flatten() {
        let path: PathBuf = entry.path();
        if !path.is_dir() || !path.join(".git").exists() || !is_due(&path) {
            continue;
        }
        // The directory name is the key (a name that is not UTF-8 is no key
        // the cache wrote, and its lossy spelling is a lock nobody holds).
        // Due-ness is read again under the lock: a fetch that finished
        // between the first read and the lock has refreshed the entry.
        let key = entry.file_name().to_string_lossy().into_owned();
        match with_idle_entry(&key, || {
            is_due(&path) && remove_path_any(&path, &fence).is_ok()
        }) {
            Some(true) => cleanup.cleared += 1,
            Some(false) => {}
            None => cleanup.in_use += 1,
        }
    }

    Ok(cleanup)
}

/// Whether the entry at `path` is old enough to reclaim: its record's last
/// fetch, else its directory's modification time, at or before the cutoff.
fn is_due(path: &Path, cutoff_ms: i64, cutoff_time: SystemTime) -> bool {
    if let Ok(raw) = std::fs::read_to_string(path.join(CACHE_META_FILE)) {
        if let Ok(meta) = serde_json::from_str::<RepoCacheMeta>(&raw) {
            if meta.last_fetched_ms > 0 && meta.last_fetched_ms <= cutoff_ms {
                return true;
            }
        }
    }
    std::fs::metadata(path)
        .map(|meta| meta.modified().unwrap_or(SystemTime::UNIX_EPOCH) <= cutoff_time)
        .unwrap_or(false)
}

#[cfg(test)]
#[path = "tests/cache_cleanup.rs"]
mod tests;
