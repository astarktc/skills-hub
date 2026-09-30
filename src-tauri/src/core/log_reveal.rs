//! The rules for revealing a path in the operator's file manager: the app
//! log folder ("Open log folder") and a path a report named ("Reveal in
//! Finder" on a Sync target a Delete could not remove, D16).
//!
//! macOS `open` treats any directory whose name ends in `.app` as an
//! application bundle, and the log dir is named after the bundle identifier
//! (`~/Library/Logs/com.skillshub.app`), so opening it as a folder fails
//! silently in the detached opener. Such a dir is selected inside its parent
//! instead; a plain dir is opened itself; and once the log file exists, the
//! file is what gets selected. The rule is pure — the command resolves the
//! paths, checks the file, and holds the opener.
//!
//! "Reveal in Finder" never opens anything: it admits only a path the app
//! manages (`admit_reveal_path`: a Sync target the store records, or a path
//! inside the central repo) and then **selects** the item — or, once it is
//! gone, its nearest existing ancestor — in its parent folder
//! (`path_reveal_item`). Opening would hand an ancestor to its default
//! handler: an `.app` bundle launches, an executable file runs.

use std::path::{Component, Path, PathBuf};

use anyhow::Result;

use crate::core::artifact_removal::recorded_target_paths;
use crate::core::environment::expand_home_path_in;
use crate::core::errors::{RevealPathFailure, SignalError};
use crate::core::skill_store::SkillStore;

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

/// Admit a path Reveal was asked to show: `requested` (a leading `~`
/// expanded against `home`, the repo's one path rule) must be a plain
/// absolute path — no `.` / `..` component — that is one of `known_targets`
/// (the artifact paths the store records) or lies inside `central_repo`.
/// Anything else is `None`: refused before the filesystem is touched.
pub fn admit_reveal_path(
    requested: &str,
    home: &Path,
    central_repo: &Path,
    known_targets: &[PathBuf],
) -> Option<PathBuf> {
    let path = expand_home_path_in(home, requested).ok()?;
    if !is_plain_absolute(&path) {
        return None;
    }
    let inside_central = expand_home_path_in(home, &central_repo.to_string_lossy())
        .ok()
        .is_some_and(|central| is_plain_absolute(&central) && path.starts_with(&central));
    let known = known_targets.iter().any(|target| target == &path);
    (known || inside_central).then_some(path)
}

/// What the file manager selects for an admitted `path`: the item itself
/// while it is there (`exists` is the caller's presence probe —
/// `symlink_metadata`, so a dangling link is still selected), else its
/// nearest ancestor that is there. Always **selected in its parent, never
/// opened** — whatever it is (a folder, an `.app` bundle, an executable
/// file). `None` when nothing on the way exists (a vanished volume).
pub fn path_reveal_item(path: &Path, exists: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    path.ancestors()
        .find(|ancestor| !ancestor.as_os_str().is_empty() && exists(ancestor))
        .map(Path::to_path_buf)
}

/// The item Reveal selects for `requested`, admitted against every artifact
/// path the store records (`artifact_removal::recorded_target_paths`, the
/// identity a Delete report carries) and the central repo; the command
/// resolves `home` / `central_repo` once and holds the opener. A refusal is
/// `RevealPathFailed { reason: NotManaged }`, nothing on the way `Gone`.
pub fn resolve_reveal_item(
    store: &SkillStore,
    home: &Path,
    central_repo: &Path,
    requested: &str,
    exists: impl Fn(&Path) -> bool,
) -> Result<PathBuf> {
    let failed = |reason, detail: String| SignalError::RevealPathFailed {
        path: requested.to_string(),
        reason,
        detail,
    };
    let known = recorded_target_paths(store)?;
    let Some(path) = admit_reveal_path(requested, home, central_repo, &known) else {
        return Err(failed(
            RevealPathFailure::NotManaged,
            "not a Sync target the app records, nor inside the central repo".to_string(),
        )
        .into());
    };
    path_reveal_item(&path, exists).ok_or_else(|| {
        failed(
            RevealPathFailure::Gone,
            format!("nothing on the way to {path:?} exists"),
        )
        .into()
    })
}

fn is_plain_absolute(path: &Path) -> bool {
    path.is_absolute()
        && path.components().all(|component| {
            matches!(
                component,
                Component::Prefix(_) | Component::RootDir | Component::Normal(_)
            )
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
