use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::skill_store::SkillStore;

const CENTRAL_DIR_NAME: &str = ".skillshub";

/// Resolve the central skills repo root: the explicit `central_repo_path`
/// setting wins; otherwise `.skillshub` under `fallback_root` (the operator's
/// home in production, with the app data dir as a last resort — the command
/// seam picks it).
pub fn resolve_central_repo_path(store: &SkillStore, fallback_root: &Path) -> Result<PathBuf> {
    if let Some(path) = store.get_setting("central_repo_path")? {
        return Ok(PathBuf::from(path));
    }
    Ok(fallback_root.join(CENTRAL_DIR_NAME))
}

pub fn ensure_central_repo(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path).with_context(|| format!("create {:?}", path))?;
    Ok(())
}

#[cfg(test)]
#[path = "tests/central_repo.rs"]
mod tests;
