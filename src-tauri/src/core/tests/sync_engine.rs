use std::fs;

use crate::core::sync_engine::{
    copy_dir_recursive, sync_dir_for_tool_with_overwrite, sync_dir_hybrid,
    sync_dir_hybrid_with_overwrite, SyncMode,
};
use crate::core::tool_adapters::adapter_by_key;

#[test]
fn copy_dir_recursive_skips_git_dir() {
    let src_dir = tempfile::tempdir().unwrap();
    let dst_dir = tempfile::tempdir().unwrap();

    fs::create_dir_all(src_dir.path().join(".git")).unwrap();
    fs::create_dir_all(src_dir.path().join("sub")).unwrap();
    fs::write(src_dir.path().join("sub/a.txt"), b"ok").unwrap();
    fs::write(src_dir.path().join(".git/secret"), b"no").unwrap();

    copy_dir_recursive(src_dir.path(), dst_dir.path()).unwrap();
    assert!(dst_dir.path().join("sub/a.txt").exists());
    assert!(!dst_dir.path().join(".git").exists());
}

#[test]
fn hybrid_sync_creates_link_and_is_idempotent_when_same_link() {
    let src_dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(src_dir.path().join("s")).unwrap();
    fs::write(src_dir.path().join("s/a.txt"), b"ok").unwrap();

    let dst_dir = tempfile::tempdir().unwrap();
    let target = dst_dir.path().join("t");

    let out = sync_dir_hybrid(src_dir.path(), &target).unwrap();
    assert!(matches!(
        out.mode_used,
        SyncMode::Symlink | SyncMode::Junction | SyncMode::Copy
    ));

    if let Ok(link) = fs::read_link(&target) {
        assert_eq!(link, src_dir.path());
        let out2 = sync_dir_hybrid(src_dir.path(), &target).unwrap();
        assert!(matches!(out2.mode_used, SyncMode::Symlink));
    }
}

#[test]
fn hybrid_sync_with_overwrite_replaces_existing() {
    let src_dir = tempfile::tempdir().unwrap();
    fs::write(src_dir.path().join("a.txt"), b"src").unwrap();

    let dst_dir = tempfile::tempdir().unwrap();
    let target = dst_dir.path().join("t");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("old.txt"), b"old").unwrap();

    let err = sync_dir_hybrid_with_overwrite(src_dir.path(), &target, false).unwrap_err();
    // Typed condition, recovered by downcast — not by message text (ADR 0001).
    let typed = err
        .downcast_ref::<crate::core::sync_engine::TargetExistsError>()
        .expect("TargetExistsError must be downcastable through the anyhow chain");
    assert_eq!(typed.target, target);

    let out = sync_dir_hybrid_with_overwrite(src_dir.path(), &target, true).unwrap();
    assert!(out.replaced);
}

#[test]
fn cursor_sync_forces_copy() {
    let src_dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(src_dir.path().join("s")).unwrap();
    fs::write(src_dir.path().join("s/a.txt"), b"ok").unwrap();

    let dst_dir = tempfile::tempdir().unwrap();
    let target = dst_dir.path().join("t");

    let cursor = adapter_by_key("cursor").expect("cursor adapter");
    assert!(
        !cursor.supports_symlink,
        "test premise: Cursor cannot symlink"
    );
    let out = sync_dir_for_tool_with_overwrite(&cursor, src_dir.path(), &target, false).unwrap();
    assert!(matches!(out.mode_used, SyncMode::Copy));
    assert!(target.join("s/a.txt").exists());
    assert_eq!(fs::read(target.join("s/a.txt")).unwrap(), b"ok");
    assert!(
        !fs::symlink_metadata(&target)
            .unwrap()
            .file_type()
            .is_symlink(),
        "target must be a real directory"
    );
}

#[cfg(unix)]
#[test]
fn symlink_capable_tool_gets_a_link() {
    let src_dir = tempfile::tempdir().unwrap();
    fs::write(src_dir.path().join("a.txt"), b"ok").unwrap();

    let dst_dir = tempfile::tempdir().unwrap();
    let target = dst_dir.path().join("t");

    let claude = adapter_by_key("claude_code").expect("claude adapter");
    assert!(claude.supports_symlink);
    let out = sync_dir_for_tool_with_overwrite(&claude, src_dir.path(), &target, false).unwrap();
    assert!(matches!(out.mode_used, SyncMode::Symlink));
    assert!(fs::symlink_metadata(&target)
        .unwrap()
        .file_type()
        .is_symlink());
}

/// The capability, not the tool's identity, decides the mode: a hypothetical
/// non-Cursor adapter with `supports_symlink: false` is copied too.
#[test]
fn any_adapter_without_symlink_support_is_copied() {
    let src_dir = tempfile::tempdir().unwrap();
    fs::write(src_dir.path().join("a.txt"), b"ok").unwrap();

    let dst_dir = tempfile::tempdir().unwrap();
    let target = dst_dir.path().join("t");

    // Clone the registry record so the capability can be flipped locally: the
    // registry itself is `static` and immutable by design.
    let mut claude = adapter_by_key("claude_code").expect("claude adapter").clone();
    claude.supports_symlink = false;
    let out = sync_dir_for_tool_with_overwrite(&claude, src_dir.path(), &target, false).unwrap();
    assert!(matches!(out.mode_used, SyncMode::Copy));
    assert!(!fs::symlink_metadata(&target)
        .unwrap()
        .file_type()
        .is_symlink());
}

#[cfg(unix)]
#[test]
fn copy_overwrite_replaces_broken_symlink_target() {
    use std::os::unix::fs::symlink;

    let src_dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(src_dir.path().join("s")).unwrap();
    fs::write(src_dir.path().join("s/a.txt"), b"ok").unwrap();

    let dst_dir = tempfile::tempdir().unwrap();
    let target = dst_dir.path().join("t");

    // Create a broken symlink at the target path.
    symlink(dst_dir.path().join("missing"), &target).unwrap();

    let out = crate::core::sync_engine::sync_dir_copy_with_overwrite(src_dir.path(), &target, true)
        .unwrap();

    assert!(matches!(out.mode_used, SyncMode::Copy));
    assert!(target.join("s/a.txt").exists());
    assert_eq!(fs::read(target.join("s/a.txt")).unwrap(), b"ok");
}
