use std::ffi::OsStr;
use std::fs;

use super::{ExchangeError, HeldDir, RenameError};

fn held() -> (tempfile::TempDir, HeldDir) {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("from")).unwrap();
    fs::write(dir.path().join("from/f"), b"ours").unwrap();
    let held = HeldDir::open(dir.path()).unwrap();
    (dir, held)
}

#[test]
fn renames_onto_a_free_name() {
    let (dir, held) = held();
    held.rename_no_replace(OsStr::new("from"), OsStr::new("to"))
        .unwrap();
    assert_eq!(fs::read(dir.path().join("to/f")).unwrap(), b"ours");
    assert!(!dir.path().join("from").exists());
}

#[test]
fn never_replaces_a_folder_empty_or_not_or_a_file() {
    let (dir, held) = held();
    fs::create_dir(dir.path().join("empty")).unwrap();
    fs::create_dir(dir.path().join("full")).unwrap();
    fs::write(dir.path().join("full/g"), b"theirs").unwrap();
    fs::write(dir.path().join("file"), b"theirs").unwrap();
    for name in ["empty", "full", "file"] {
        let err = held
            .rename_no_replace(OsStr::new("from"), OsStr::new(name))
            .unwrap_err();
        assert!(matches!(err, RenameError::Occupied), "{name}: {err:?}");
    }
    assert_eq!(fs::read_dir(dir.path().join("empty")).unwrap().count(), 0);
    assert_eq!(fs::read(dir.path().join("full/g")).unwrap(), b"theirs");
    assert_eq!(fs::read(dir.path().join("file")).unwrap(), b"theirs");
    assert_eq!(fs::read(dir.path().join("from/f")).unwrap(), b"ours");
}

#[cfg(unix)]
#[test]
fn never_follows_or_replaces_a_link_dangling_or_not() {
    let (dir, held) = held();
    let outside = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(outside.path(), dir.path().join("live")).unwrap();
    std::os::unix::fs::symlink(outside.path().join("gone"), dir.path().join("dangling")).unwrap();
    for name in ["live", "dangling"] {
        let err = held
            .rename_no_replace(OsStr::new("from"), OsStr::new(name))
            .unwrap_err();
        assert!(matches!(err, RenameError::Occupied), "{name}: {err:?}");
        assert!(fs::symlink_metadata(dir.path().join(name))
            .unwrap()
            .file_type()
            .is_symlink());
    }
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
}

#[test]
fn refuses_names_that_are_not_one_entry() {
    let (dir, held) = held();
    for name in ["../escape", "a/b", "..", "."] {
        let err = held
            .rename_no_replace(OsStr::new("from"), OsStr::new(name))
            .unwrap_err();
        assert!(matches!(err, RenameError::Failed(_)), "{name}: {err:?}");
    }
    assert!(dir.path().join("from/f").exists());
}

/// The fallback for a file system without an exclusive rename, driven
/// directly: it renames onto its own reservation, and refuses what is
/// already there — a link included, never followed — removing nothing.
#[cfg(unix)]
#[test]
fn the_reservation_fallback_refuses_occupants_and_publishes_otherwise() {
    use std::os::unix::io::AsRawFd;

    use super::sys::{c_name, rename_onto_reservation};

    let (dir, held) = held();
    let outside = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("full")).unwrap();
    fs::write(dir.path().join("full/g"), b"theirs").unwrap();
    std::os::unix::fs::symlink(outside.path(), dir.path().join("link")).unwrap();
    let fd = held.dir.as_raw_fd();
    let from = c_name(OsStr::new("from")).unwrap();
    for name in ["full", "link"] {
        let to = c_name(OsStr::new(name)).unwrap();
        let err = rename_onto_reservation(fd, &from, &to).unwrap_err();
        assert!(matches!(err, RenameError::Occupied), "{name}: {err:?}");
    }
    assert_eq!(fs::read(dir.path().join("full/g")).unwrap(), b"theirs");
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);

    let to = c_name(OsStr::new("to")).unwrap();
    rename_onto_reservation(fd, &from, &to).unwrap();
    assert_eq!(fs::read(dir.path().join("to/f")).unwrap(), b"ours");

    // A failed rename removes its own reservation, and only that.
    let missing = c_name(OsStr::new("missing")).unwrap();
    let again = c_name(OsStr::new("again")).unwrap();
    assert!(matches!(
        rename_onto_reservation(fd, &missing, &again),
        Err(RenameError::Failed(_))
    ));
    assert!(!dir.path().join("again").exists());
}

/// An exchange swaps a folder and a link in one step where the OS offers it
/// (macOS and Linux on their usual file systems), and says so elsewhere —
/// Windows always — touching nothing.
#[test]
fn exchanges_two_entries_or_says_it_cannot() {
    let (dir, held) = held();
    fs::create_dir(dir.path().join("other")).unwrap();
    fs::write(dir.path().join("other/g"), b"theirs").unwrap();
    match held.exchange(OsStr::new("from"), OsStr::new("other")) {
        Ok(()) => {
            assert_eq!(fs::read(dir.path().join("other/f")).unwrap(), b"ours");
            assert_eq!(fs::read(dir.path().join("from/g")).unwrap(), b"theirs");
        }
        Err(ExchangeError::Unsupported) => {
            if cfg!(target_vendor = "apple") {
                panic!("macOS exchanges");
            }
            assert_eq!(fs::read(dir.path().join("from/f")).unwrap(), b"ours");
            assert_eq!(fs::read(dir.path().join("other/g")).unwrap(), b"theirs");
        }
        Err(err) => panic!("{err:?}"),
    }
}

/// An exchange with a name that is absent changes nothing.
#[test]
fn an_exchange_with_an_absent_name_changes_nothing() {
    let (dir, held) = held();
    let err = held
        .exchange(OsStr::new("from"), OsStr::new("absent"))
        .unwrap_err();
    assert!(matches!(
        err,
        ExchangeError::Failed(_) | ExchangeError::Unsupported
    ));
    assert_eq!(fs::read(dir.path().join("from/f")).unwrap(), b"ours");
    assert!(!dir.path().join("absent").exists());
}
