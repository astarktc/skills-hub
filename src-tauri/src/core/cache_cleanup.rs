use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};

use super::clock::now_ms;
use super::removal_fence::{ensure_clear_of_library, Claim, Fence, OwnedDir, ProtectedLibrary};
use super::skill_store::SkillStore;
use super::sync_engine::remove_path_any;
use serde::Deserialize;
use tauri::Manager;

const CACHE_DIR_NAME: &str = "skills-hub-git-cache";
const CACHE_META_FILE: &str = ".skills-hub-cache.json";

#[derive(Debug, Deserialize)]
struct RepoCacheMeta {
    last_fetched_ms: i64,
}

/// Age out the git cache under the app cache dir. `central_dir` is the
/// configured central repo the command seam resolved: with `store`'s records
/// it is the library no reclaimed entry may reach.
pub fn cleanup_git_cache_dirs<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    store: &SkillStore,
    central_dir: &Path,
    max_age: Duration,
) -> Result<usize> {
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
fn cleanup_git_cache_dirs_in(
    cache_dir: &Path,
    library: &ProtectedLibrary<'_>,
    max_age: Duration,
) -> Result<usize> {
    let relative = Path::new(CACHE_DIR_NAME);
    let Some(cache_root) = OwnedDir::resolve(cache_dir, relative)? else {
        return Ok(0);
    };
    ensure_clear_of_library(cache_root.path(), library)?;

    let cutoff_ms = now_ms().saturating_sub(max_age.as_millis().try_into().unwrap_or(i64::MAX));
    let cutoff_time = SystemTime::now()
        .checked_sub(max_age)
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let fence = Fence::new(library).claiming(Claim::ChildOfOwned(&cache_root));
    let mut removed = 0usize;
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

    for entry in rd.flatten() {
        let path: PathBuf = entry.path();
        if !path.is_dir() {
            continue;
        }

        if !path.join(".git").exists() {
            continue;
        }

        let meta_path = path.join(CACHE_META_FILE);
        let mut should_remove = false;

        if let Ok(raw) = std::fs::read_to_string(&meta_path) {
            if let Ok(meta) = serde_json::from_str::<RepoCacheMeta>(&raw) {
                if meta.last_fetched_ms > 0 && meta.last_fetched_ms <= cutoff_ms {
                    should_remove = true;
                }
            }
        }

        if !should_remove {
            let meta = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            if modified <= cutoff_time {
                should_remove = true;
            }
        }

        if should_remove && remove_path_any(&path, &fence).is_ok() {
            removed += 1;
        }
    }

    Ok(removed)
}

#[cfg(test)]
#[path = "tests/cache_cleanup.rs"]
mod tests;
