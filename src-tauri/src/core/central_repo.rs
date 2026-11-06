use std::path::Path;

use anyhow::{Context, Result};

/// Create the central repo folder (and its parents) when absent. Moving the
/// Library elsewhere is `core::library_move`.
pub fn ensure_central_repo(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path).with_context(|| format!("create {:?}", path))?;
    Ok(())
}

#[cfg(test)]
#[path = "tests/central_repo.rs"]
mod tests;
