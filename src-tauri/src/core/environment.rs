//! The one place core reads the operator's real environment.
//!
//! Every other core function takes explicit roots (`home`, `central_dir`,
//! `cache_dir`, ...) so tests can substitute a temp directory; the thin
//! production wrappers here are the only callers of `dirs::*`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// The operator's home directory (production wrapper over `dirs::home_dir()`).
pub fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().context("failed to resolve home directory")
}

/// Which host directory a registry path is relative to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HostRoot {
    /// The operator's home (`~`). Every registry path's default root.
    Home,
    // No production row resolves under an app-data root until the first
    // vendor-documented override lands (Reasonix, round 20 ticket 06).
    /// Roaming application data: `%APPDATA%` on Windows; elsewhere the
    /// platform's config dir (`~/Library/Application Support`, `~/.config`).
    #[cfg_attr(not(test), allow(dead_code, reason = "first override: ticket 06"))]
    RoamingAppData,
    /// Local application data: `%LOCALAPPDATA%` on Windows; elsewhere the
    /// platform's local data dir (`~/Library/Application Support`,
    /// `~/.local/share`).
    #[cfg_attr(not(test), allow(dead_code, reason = "first override: ticket 06"))]
    LocalAppData,
}

/// The operating systems a registry path may carry a per-OS override for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HostOs {
    Windows,
    MacOs,
    /// Linux and every other Unix: the XDG layout.
    Linux,
}

impl HostOs {
    /// The OS this build runs on (a compile-time fact, not an environment read).
    pub const CURRENT: HostOs = if cfg!(windows) {
        HostOs::Windows
    } else if cfg!(target_os = "macos") {
        HostOs::MacOs
    } else {
        HostOs::Linux
    };
}

/// The host directories registry paths resolve under, resolved once at the
/// command seam ([`HostRoots::resolve`]) and handed to core explicitly, like
/// `home` before it, so tests substitute temp dirs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostRoots {
    pub home: PathBuf,
    pub roaming: PathBuf,
    pub local: PathBuf,
}

impl HostRoots {
    /// The operator's real roots (production wrapper over `dirs::*`).
    pub fn resolve() -> Result<HostRoots> {
        Ok(HostRoots {
            home: home_dir()?,
            roaming: dirs::config_dir().context("failed to resolve roaming app data dir")?,
            local: dirs::data_local_dir().context("failed to resolve local app data dir")?,
        })
    }

    /// Test roots under one temp `home`: the app-data roots sit in their
    /// Windows places under it, distinct from home itself, so a path resolved
    /// under the wrong root cannot pass.
    #[cfg(test)]
    pub fn under_home(home: &Path) -> HostRoots {
        HostRoots {
            home: home.to_path_buf(),
            roaming: home.join("AppData").join("Roaming"),
            local: home.join("AppData").join("Local"),
        }
    }

    /// The directory `root` names.
    pub fn root(&self, root: HostRoot) -> &Path {
        match root {
            HostRoot::Home => &self.home,
            HostRoot::RoamingAppData => &self.roaming,
            HostRoot::LocalAppData => &self.local,
        }
    }
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
