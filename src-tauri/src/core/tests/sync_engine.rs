use std::fs;

use crate::core::content_identity::Source;
use crate::core::sync_engine::{
    copy_dir_recursive, sync_dir_for_tool_with_overwrite, sync_dir_hybrid,
    sync_dir_hybrid_with_overwrite, SyncMode,
};
use crate::core::tool_adapters::adapter_by_key;

#[cfg(unix)]
#[test]
fn copy_ignores_internal_symlinks_and_preserves_identity() {
    use crate::core::content_identity::{same_content, Source};

    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    fs::write(src.path().join("a.txt"), b"hello").unwrap();
    for (name, target) in [
        ("relative", "a.txt"),
        ("dangling", "missing"),
        ("directory", "."),
    ] {
        std::os::unix::fs::symlink(target, src.path().join(name)).unwrap();
    }

    copy_dir_recursive(src.path(), dst.path()).unwrap();

    assert_eq!(fs::read(dst.path().join("a.txt")).unwrap(), b"hello");
    for name in ["relative", "dangling", "directory"] {
        assert_eq!(
            fs::symlink_metadata(dst.path().join(name))
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::NotFound
        );
    }
    assert!(same_content(Source::Directory(src.path()), dst.path()));
    assert!(same_content(Source::Directory(dst.path()), src.path()));
}

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

    let out = sync_dir_hybrid(src_dir.path(), Source::Directory(src_dir.path()), &target).unwrap();
    assert!(matches!(
        out.mode_used,
        SyncMode::Symlink | SyncMode::Junction | SyncMode::Copy
    ));

    if let Ok(link) = fs::read_link(&target) {
        assert_eq!(link, src_dir.path());
        let out2 =
            sync_dir_hybrid(src_dir.path(), Source::Directory(src_dir.path()), &target).unwrap();
        assert!(matches!(out2.mode_used, SyncMode::Symlink));
    }
}

/// A missing source is a typed failure in symlink mode too — never a
/// dangling link recorded as a successful sync.
#[test]
fn hybrid_sync_refuses_missing_source() {
    let dst_dir = tempfile::tempdir().unwrap();
    let target = dst_dir.path().join("t");
    let source = dst_dir.path().join("no-such-source");

    let err = sync_dir_hybrid_with_overwrite(&source, Source::Directory(&source), &target, true)
        .unwrap_err();
    match err.downcast_ref::<crate::core::errors::SignalError>() {
        Some(crate::core::errors::SignalError::InvalidPath { path, reason }) => {
            assert_eq!(path, &source.to_string_lossy());
            assert_eq!(reason, "missing");
        }
        other => panic!("expected InvalidPath, got {other:?}"),
    }
    assert!(
        fs::symlink_metadata(&target).is_err(),
        "no dangling link may be created"
    );
}

#[test]
fn hybrid_sync_with_overwrite_replaces_existing() {
    let src_dir = tempfile::tempdir().unwrap();
    fs::write(src_dir.path().join("a.txt"), b"src").unwrap();

    let dst_dir = tempfile::tempdir().unwrap();
    let target = dst_dir.path().join("t");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("old.txt"), b"old").unwrap();

    let err = sync_dir_hybrid_with_overwrite(
        src_dir.path(),
        Source::Directory(src_dir.path()),
        &target,
        false,
    )
    .unwrap_err();
    // Typed condition, recovered by downcast — not by message text (ADR 0001).
    let typed = err
        .downcast_ref::<crate::core::sync_engine::TargetExistsError>()
        .expect("TargetExistsError must be downcastable through the anyhow chain");
    assert_eq!(typed.target, target);

    let out = sync_dir_hybrid_with_overwrite(
        src_dir.path(),
        Source::Directory(src_dir.path()),
        &target,
        true,
    )
    .unwrap();
    assert!(out.replaced);
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
    let out = sync_dir_for_tool_with_overwrite(
        claude,
        src_dir.path(),
        Source::Directory(src_dir.path()),
        &target,
        false,
    )
    .unwrap();
    assert!(matches!(out.mode_used, SyncMode::Symlink));
    assert!(fs::symlink_metadata(&target)
        .unwrap()
        .file_type()
        .is_symlink());
}

/// The capability, not the tool's identity, decides the mode: any adapter
/// with `supports_symlink: false` is copied (no current registry entry is,
/// so the record is cloned and flipped locally).
#[test]
fn any_adapter_without_symlink_support_is_copied() {
    let src_dir = tempfile::tempdir().unwrap();
    fs::write(src_dir.path().join("a.txt"), b"ok").unwrap();

    let dst_dir = tempfile::tempdir().unwrap();
    let target = dst_dir.path().join("t");

    // Clone the registry record so the capability can be flipped locally: the
    // registry itself is `static` and immutable by design.
    let mut claude = adapter_by_key("claude_code")
        .expect("claude adapter")
        .clone();
    claude.supports_symlink = false;
    let out = sync_dir_for_tool_with_overwrite(
        &claude,
        src_dir.path(),
        Source::Directory(src_dir.path()),
        &target,
        false,
    )
    .unwrap();
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

    let out = crate::core::sync_engine::sync_dir_copy_with_overwrite(
        src_dir.path(),
        Source::Directory(src_dir.path()),
        &target,
        true,
    )
    .unwrap();

    assert!(matches!(out.mode_used, SyncMode::Copy));
    assert!(target.join("s/a.txt").exists());
    assert_eq!(fs::read(target.join("s/a.txt")).unwrap(), b"ok");
}

// ---------------------------------------------------------------------------
// The existing-root boundary (`create_dirs_within`, `sync_dir_for_tool_within_root`):
// a Project target never recreates a root that is gone at creation time.
// ---------------------------------------------------------------------------

use crate::core::errors::SignalError;
use crate::core::sync_engine::{create_dirs_within, sync_dir_for_tool_within_root, test_hooks};

fn missing_path(err: &anyhow::Error) -> Option<String> {
    err.chain()
        .find_map(|cause| match cause.downcast_ref::<SignalError>() {
            Some(SignalError::InvalidPath { path, reason }) if reason == "missing" => {
                Some(path.clone())
            }
            _ => None,
        })
}

#[test]
fn create_dirs_within_creates_descendants_under_a_present_root() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    fs::create_dir(&root).unwrap();
    fs::create_dir(root.join(".claude")).unwrap();

    create_dirs_within(&root, &root.join(".claude/skills/demo")).unwrap();
    assert!(root.join(".claude/skills/demo").is_dir());
    // Idempotent over existing levels, and the root itself is a no-op.
    create_dirs_within(&root, &root.join(".claude/skills/demo")).unwrap();
    create_dirs_within(&root, &root).unwrap();
}

#[test]
fn create_dirs_within_refuses_a_root_that_is_missing_or_a_file() {
    let tmp = tempfile::tempdir().unwrap();
    let gone = tmp.path().join("gone");
    let err = create_dirs_within(&gone, &gone.join(".claude/skills")).unwrap_err();
    assert_eq!(
        missing_path(&err).as_deref(),
        Some(&*gone.to_string_lossy())
    );
    assert!(!gone.exists(), "a missing root is not created");

    let file = tmp.path().join("file-root");
    fs::write(&file, b"not a dir").unwrap();
    let err = create_dirs_within(&file, &file.join(".claude/skills")).unwrap_err();
    assert_eq!(
        missing_path(&err).as_deref(),
        Some(&*file.to_string_lossy())
    );
    assert!(file.is_file(), "the file is left alone");
}

#[test]
fn create_dirs_within_refuses_a_path_outside_or_escaping_the_root() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    fs::create_dir(&root).unwrap();

    let outside = tmp.path().join("elsewhere/skills");
    assert!(create_dirs_within(&root, &outside).is_err());
    assert!(!tmp.path().join("elsewhere").exists());

    let escaping = root.join("../escaped/skills");
    assert!(create_dirs_within(&root, &escaping).is_err());
    assert!(!tmp.path().join("escaped").exists());
}

/// The race the boundary exists for: the root is present when the walk
/// starts and disappears (a Finder move) just before the first `mkdir`.
#[test]
fn create_dirs_within_never_recreates_a_root_that_vanishes_at_creation() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    let moved = tmp.path().join("project-moved");
    fs::create_dir(&root).unwrap();

    let _hook = test_hooks::before_bounded_mkdir_scoped({
        let (root, moved) = (root.clone(), moved.clone());
        let mut fired = false;
        move |_level| {
            if !fired {
                fired = true;
                fs::rename(&root, &moved).unwrap();
            }
        }
    });
    let err = create_dirs_within(&root, &root.join(".claude/skills")).unwrap_err();

    assert_eq!(
        missing_path(&err).as_deref(),
        Some(&*root.to_string_lossy())
    );
    assert!(moved.is_dir(), "the hook really moved the root");
    assert!(!root.exists(), "the old root is not recreated");
    assert!(
        !moved.join(".claude").exists(),
        "nothing written after the move"
    );
}

#[test]
fn sync_within_root_links_under_a_present_root() {
    let src = tempfile::tempdir().unwrap();
    fs::write(src.path().join("a.txt"), b"ok").unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    fs::create_dir(&root).unwrap();
    let target = root.join(".claude/skills/demo");

    let claude = adapter_by_key("claude_code").unwrap();
    let out = sync_dir_for_tool_within_root(
        claude,
        src.path(),
        Source::Directory(src.path()),
        &target,
        false,
        &root,
    )
    .unwrap();
    assert!(matches!(out.mode_used, SyncMode::Symlink));
    assert!(fs::symlink_metadata(&target)
        .unwrap()
        .file_type()
        .is_symlink());
}

/// The copy fallback creates its own tree (the target dir and every
/// subdir) under the same boundary: the root vanishing exactly when the
/// copy creates the target dir — after the parents were made — is refused
/// and nothing is recreated at the old path.
#[test]
fn copy_within_root_never_recreates_a_root_that_vanishes_mid_copy() {
    let src = tempfile::tempdir().unwrap();
    fs::create_dir(src.path().join("sub")).unwrap();
    fs::write(src.path().join("sub/a.txt"), b"ok").unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    let moved = tmp.path().join("project-moved");
    fs::create_dir(&root).unwrap();
    let target = root.join(".claude/skills/demo");

    let mut copy_only = adapter_by_key("claude_code").unwrap().clone();
    copy_only.supports_symlink = false;

    // Root present: the copy lands under it, subdirs included.
    let out = sync_dir_for_tool_within_root(
        &copy_only,
        src.path(),
        Source::Directory(src.path()),
        &target,
        false,
        &root,
    )
    .unwrap();
    assert!(matches!(out.mode_used, SyncMode::Copy));
    assert_eq!(fs::read(target.join("sub/a.txt")).unwrap(), b"ok");
    fs::remove_dir_all(&target).unwrap();

    let _hook = test_hooks::before_bounded_mkdir_scoped({
        let (root, moved, target) = (root.clone(), moved.clone(), target.clone());
        let mut fired = false;
        move |level| {
            if !fired && level == target {
                fired = true;
                fs::rename(&root, &moved).unwrap();
            }
        }
    });
    let err = sync_dir_for_tool_within_root(
        &copy_only,
        src.path(),
        Source::Directory(src.path()),
        &target,
        false,
        &root,
    )
    .unwrap_err();

    assert_eq!(
        missing_path(&err).as_deref(),
        Some(&*root.to_string_lossy())
    );
    assert!(moved.is_dir(), "the hook really moved the root");
    assert!(!root.exists(), "the old root is not recreated");
    assert!(!moved.join(".claude/skills/demo").exists());
}

/// Global sync keeps creating every missing parent: a Tool's global skills
/// dir may not exist yet.
#[test]
fn global_sync_still_creates_missing_parents() {
    let src = tempfile::tempdir().unwrap();
    fs::write(src.path().join("a.txt"), b"ok").unwrap();
    let home = tempfile::tempdir().unwrap();
    let target = home.path().join("not-yet/.tool/skills/demo");

    for supports_symlink in [true, false] {
        let mut adapter = adapter_by_key("claude_code").unwrap().clone();
        adapter.supports_symlink = supports_symlink;
        let out = sync_dir_for_tool_with_overwrite(
            &adapter,
            src.path(),
            Source::Directory(src.path()),
            &target,
            true,
        )
        .unwrap();
        assert_eq!(matches!(out.mode_used, SyncMode::Copy), !supports_symlink);
        assert_eq!(fs::read(target.join("a.txt")).unwrap(), b"ok");
        fs::remove_dir_all(home.path().join("not-yet")).unwrap();
    }
}
