use std::path::{Path, PathBuf};

use crate::core::environment::expand_home_path_in;

fn home() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(r"C:\Users\fake")
    } else {
        PathBuf::from("/fake/home")
    }
}

#[test]
fn expands_bare_tilde_to_home() {
    assert_eq!(expand_home_path_in(&home(), "~").unwrap(), home());
    assert_eq!(expand_home_path_in(&home(), "  ~  ").unwrap(), home());
}

#[test]
fn expands_tilde_prefix_to_home_join() {
    assert_eq!(
        expand_home_path_in(&home(), "~/abc/def").unwrap(),
        home().join("abc/def")
    );
}

#[test]
fn leaves_other_paths_untouched_but_trimmed() {
    assert_eq!(
        expand_home_path_in(&home(), "  /opt/x  ").unwrap(),
        Path::new("/opt/x")
    );
    // `~user` is not expanded — only `~` and `~/` are.
    assert_eq!(
        expand_home_path_in(&home(), "~alex/x").unwrap(),
        Path::new("~alex/x")
    );
    assert_eq!(
        expand_home_path_in(&home(), "rel/x").unwrap(),
        Path::new("rel/x")
    );
}

#[test]
fn empty_or_whitespace_is_error() {
    let err = expand_home_path_in(&home(), "  ").unwrap_err().to_string();
    assert!(err.contains("storage path is empty"), "got: {err}");
    assert!(expand_home_path_in(&home(), "").is_err());
}
