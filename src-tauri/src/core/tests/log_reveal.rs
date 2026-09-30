use std::path::{Path, PathBuf};

use crate::core::log_reveal::{log_reveal_target, path_reveal_target, LogRevealTarget};

/// macOS `open` treats any directory whose name ends in `.app` as an
/// application bundle, so the log dir (`~/Library/Logs/com.skillshub.app`)
/// is never opened itself: it is selected inside its parent.
#[test]
fn a_dot_app_suffixed_log_dir_is_revealed_from_its_parent() {
    let log_dir = Path::new("/Users/u/Library/Logs/com.skillshub.app");

    let target = log_reveal_target(log_dir, None);

    assert_eq!(
        target,
        LogRevealTarget {
            folder: PathBuf::from("/Users/u/Library/Logs"),
            selected: Some(log_dir.to_path_buf()),
        }
    );
}

/// A log dir without the bundle suffix (Linux, Windows) is opened itself.
#[test]
fn a_plain_log_dir_is_opened_itself() {
    let log_dir = Path::new("/home/u/.local/share/com.skillshub.app/logs");

    let target = log_reveal_target(log_dir, None);

    assert_eq!(
        target,
        LogRevealTarget {
            folder: log_dir.to_path_buf(),
            selected: None,
        }
    );
}

/// Once the plugin has written a log file, that file is what gets selected —
/// inside the log dir, whatever the dir is called.
#[test]
fn an_existing_log_file_is_selected_inside_the_log_dir() {
    let log_dir = Path::new("/Users/u/Library/Logs/com.skillshub.app");
    let log_file = log_dir.join("Skills Hub.log");

    let target = log_reveal_target(log_dir, Some(&log_file));

    assert_eq!(
        target,
        LogRevealTarget {
            folder: log_dir.to_path_buf(),
            selected: Some(log_file),
        }
    );
}

/// The suffix rule is about the name, not the case it was typed in.
#[test]
fn the_bundle_suffix_rule_ignores_case() {
    let log_dir = Path::new("/Users/u/Library/Logs/Com.Skillshub.APP");

    let target = log_reveal_target(log_dir, None);

    assert_eq!(target.folder, PathBuf::from("/Users/u/Library/Logs"));
    assert_eq!(target.selected.as_deref(), Some(log_dir));
}

/// A path a report named that is still there (a target a Delete could not
/// remove) is selected inside its folder.
#[test]
fn a_present_reported_path_is_selected_in_its_folder() {
    let path = Path::new("/Users/u/.claude/skills/release-train");

    let target = path_reveal_target(path, |p| p == path);

    assert_eq!(
        target,
        Some(LogRevealTarget {
            folder: PathBuf::from("/Users/u/.claude/skills"),
            selected: Some(path.to_path_buf()),
        })
    );
}

/// Once the path is gone, its nearest ancestor that is there opens plainly.
#[test]
fn a_gone_reported_path_opens_its_nearest_existing_ancestor() {
    let path = Path::new("/Users/u/.claude/skills/release-train");

    let target = path_reveal_target(path, |p| p == Path::new("/Users/u/.claude"));

    assert_eq!(
        target,
        Some(LogRevealTarget {
            folder: PathBuf::from("/Users/u/.claude"),
            selected: None,
        })
    );
}

/// Nothing on the way exists (a vanished volume): there is nothing to show.
#[test]
fn a_path_with_no_existing_ancestor_has_no_target() {
    assert_eq!(
        path_reveal_target(Path::new("/Volumes/gone/x"), |_| false),
        None
    );
}

/// The presence probe is `symlink_metadata`: a dangling link is still an
/// item to select (the real probe, on a temp dir).
#[cfg(unix)]
#[test]
fn a_dangling_link_is_selected_with_the_real_probe() {
    let dir = tempfile::tempdir().unwrap();
    let link = dir.path().join("release-train");
    std::os::unix::fs::symlink(dir.path().join("gone"), &link).unwrap();

    let target = path_reveal_target(&link, |p| p.symlink_metadata().is_ok());

    assert_eq!(target.and_then(|t| t.selected), Some(link));
}
