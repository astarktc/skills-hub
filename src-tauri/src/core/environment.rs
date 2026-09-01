//! The one place core reads the operator's real environment.
//!
//! Every other core function takes explicit roots (`home`, `central_dir`,
//! `cache_dir`, ...) so tests can substitute a temp directory; the thin
//! production wrappers here are the only callers of `dirs::home_dir()`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// The operator's home directory (production wrapper over `dirs::home_dir()`).
pub fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().context("failed to resolve home directory")
}

/// Expand a leading `~` / `~/` in user input against `home`.
///
/// Input is trimmed; empty input is an error. Other paths (including
/// `~user/...` forms) are returned as-is.
pub fn expand_home_path_in(home: &Path, input: &str) -> Result<PathBuf> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        anyhow::bail!("storage path is empty");
    }
    if trimmed == "~" {
        return Ok(home.to_path_buf());
    }
    if let Some(stripped) = trimmed.strip_prefix("~/") {
        return Ok(home.join(stripped));
    }
    Ok(PathBuf::from(trimmed))
}

/// [`expand_home_path_in`] against the real home directory.
pub fn expand_home_path(input: &str) -> Result<PathBuf> {
    expand_home_path_in(&home_dir()?, input)
}

#[cfg(test)]
#[path = "tests/environment.rs"]
mod tests;
