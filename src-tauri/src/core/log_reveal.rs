//! The rules for revealing a path in the operator's file manager: the app
//! log folder ("Open log folder") and any path a report named ("Reveal in
//! Finder" on a Sync target a Delete could not remove, D16).
//!
//! macOS `open` treats any directory whose name ends in `.app` as an
//! application bundle, and the log dir is named after the bundle identifier
//! (`~/Library/Logs/com.skillshub.app`), so opening it as a folder fails
//! silently in the detached opener. Such a dir is selected inside its parent
//! instead; a plain dir is opened itself; and once the log file exists, the
//! file is what gets selected. The rule is pure — the command resolves the
//! paths, checks the file, and holds the opener.

use std::path::{Path, PathBuf};

/// Where the file manager lands for "Open log folder".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRevealTarget {
    /// The folder the file manager shows.
    pub folder: PathBuf,
    /// The item selected inside `folder` (reveal), or `None` to open
    /// `folder` plainly.
    pub selected: Option<PathBuf>,
}

/// Decide how to reveal `log_dir`; `log_file` is the plugin's log file when
/// it already exists.
pub fn log_reveal_target(log_dir: &Path, log_file: Option<&Path>) -> LogRevealTarget {
    if let Some(file) = log_file {
        return LogRevealTarget {
            folder: log_dir.to_path_buf(),
            selected: Some(file.to_path_buf()),
        };
    }
    if looks_like_app_bundle(log_dir) {
        return LogRevealTarget {
            folder: log_dir
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| log_dir.to_path_buf()),
            selected: Some(log_dir.to_path_buf()),
        };
    }
    LogRevealTarget {
        folder: log_dir.to_path_buf(),
        selected: None,
    }
}

/// Decide how to reveal `path`, a path a report named: the item itself,
/// selected in its folder, while it is there (`exists` is the caller's
/// presence probe — `symlink_metadata`, so a dangling link is still
/// selected); once it is gone, its nearest ancestor that is there, opened
/// plainly. `None` when no ancestor exists (a path on a vanished volume).
pub fn path_reveal_target(path: &Path, exists: impl Fn(&Path) -> bool) -> Option<LogRevealTarget> {
    if exists(path) {
        let folder = path.parent().unwrap_or(path).to_path_buf();
        return Some(LogRevealTarget {
            folder,
            selected: Some(path.to_path_buf()),
        });
    }
    path.ancestors()
        .skip(1)
        .find(|ancestor| !ancestor.as_os_str().is_empty() && exists(ancestor))
        .map(|ancestor| LogRevealTarget {
            folder: ancestor.to_path_buf(),
            selected: None,
        })
}

/// A directory name ending in `.app` (any case) is an app bundle to macOS.
fn looks_like_app_bundle(dir: &Path) -> bool {
    dir.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("app"))
}

#[cfg(test)]
#[path = "tests/log_reveal.rs"]
mod tests;
