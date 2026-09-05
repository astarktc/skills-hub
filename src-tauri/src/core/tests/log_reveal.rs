use std::path::{Path, PathBuf};

use crate::core::log_reveal::{log_reveal_target, LogRevealTarget};

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
