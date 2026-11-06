use std::path::PathBuf;

use crate::core::central_repo::ensure_central_repo;

#[test]
fn ensure_central_repo_creates_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p: PathBuf = dir.path().join("a/b/c");
    assert!(!p.exists());
    ensure_central_repo(&p).unwrap();
    assert!(p.exists());
}
