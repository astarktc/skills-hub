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

/// `copy_dir_recursive` keeps a file's mode (`fs::copy` copies permissions on
/// Unix): an executable script stays executable in every copy-mode target.
#[cfg(unix)]
#[test]
fn copy_dir_recursive_preserves_executable_bits() {
    use std::os::unix::fs::PermissionsExt;
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    fs::create_dir_all(src.path().join("scripts")).unwrap();
    fs::write(src.path().join("scripts/run.sh"), "#!/bin/sh\n").unwrap();
    fs::set_permissions(
        src.path().join("scripts/run.sh"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    fs::write(src.path().join("SKILL.md"), "plain").unwrap();
    fs::set_permissions(
        src.path().join("SKILL.md"),
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();

    copy_dir_recursive(src.path(), dst.path()).unwrap();

    let mode = |name: &str| {
        fs::metadata(dst.path().join(name))
            .unwrap()
            .permissions()
            .mode()
            & 0o777
    };
    assert_eq!(mode("scripts/run.sh"), 0o755);
    assert_eq!(mode("SKILL.md"), 0o644);
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

/// The Windows junction leg of the sync fallback (symlink → junction → copy).
///
/// Field installs on non-elevated Windows record `mode = junction`, yet a CI
/// runner (and an elevated shell) can create symlinks, so the leg would never
/// run there. These tests force it by calling `try_junction` directly and
/// then drive the production paths a junction target meets: presence,
/// removal (artifact_removal, both settlements), overwrite replacement and
/// content identity. The invariant throughout: removing or replacing a
/// junction target never touches the central skill it points at.
#[cfg(windows)]
mod junction_leg {
    use std::fs;
    use std::path::{Path, PathBuf};

    use crate::core::artifact_removal::{
        execute_settling_unlocked, execute_unlocked, plan, RemovalScope, Settlement,
    };
    use crate::core::content_identity::{self, Comparison, Source};
    use crate::core::skill_store::{SkillRecord, SkillStore, SkillTargetRecord};
    use crate::core::sync_engine::{
        remove_path_any, sync_dir_hybrid_with_overwrite, try_junction, Occupant, SyncMode,
    };
    use crate::core::sync_status::SyncStatus;

    /// A central skill with a nested file, so "intact" covers more than one level.
    fn central_skill(base: &Path, name: &str) -> PathBuf {
        let dir = base.join("central").join(name);
        fs::create_dir_all(dir.join("scripts")).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: {name}\n---\n# {name}\n"),
        )
        .unwrap();
        fs::write(dir.join("scripts").join("run.ps1"), b"Write-Output ok\r\n").unwrap();
        dir
    }

    fn assert_central_intact(central: &Path, name: &str) {
        assert!(central.is_dir(), "central skill dir survives: {central:?}");
        assert_eq!(
            fs::read_to_string(central.join("SKILL.md")).unwrap(),
            format!("---\nname: {name}\n---\n# {name}\n")
        );
        assert_eq!(
            fs::read(central.join("scripts").join("run.ps1")).unwrap(),
            b"Write-Output ok\r\n"
        );
    }

    /// A junction at `<base>/tools/claude/skills/<name>` → `central`, made by
    /// the engine's own junction leg.
    fn junction_target(base: &Path, central: &Path, name: &str) -> PathBuf {
        let parent = base.join("tools").join("claude").join("skills");
        fs::create_dir_all(&parent).unwrap();
        let target = parent.join(name);
        try_junction(central, &target).expect("junction leg");
        assert!(
            junction::exists(&target).unwrap(),
            "a real junction: {target:?}"
        );
        target
    }

    fn store_with_junction_row(
        base: &Path,
        central: &Path,
        name: &str,
        target: &Path,
    ) -> (SkillStore, SkillRecord) {
        let store = SkillStore::new(base.join("test.db"));
        store.ensure_schema().unwrap();
        let skill = SkillRecord {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: None,
            source_type: "local".to_string(),
            source_ref: None,
            source_subpath: None,
            source_revision: None,
            central_path: central.to_string_lossy().to_string(),
            content_hash: None,
            created_at: 1,
            updated_at: 1,
            last_sync_at: None,
            last_seen_at: 1,
            status: "ok".to_string(),
            imported_from_tool: None,
        };
        store.upsert_skill(&skill).unwrap();
        store
            .upsert_skill_target(&SkillTargetRecord {
                id: uuid::Uuid::new_v4().to_string(),
                skill_id: skill.id.clone(),
                tool: "claude_code".to_string(),
                target_path: target.to_string_lossy().to_string(),
                mode: SyncMode::Junction,
                status: SyncStatus::Synced,
                last_error: None,
                synced_at: Some(1),
            })
            .unwrap();
        (store, skill)
    }

    #[test]
    fn a_junction_target_resolves_to_the_central_skill_and_is_present() {
        let tmp = tempfile::tempdir().unwrap();
        let central = central_skill(tmp.path(), "demo");
        let target = junction_target(tmp.path(), &central, "demo");

        // The presence rule (`symlink_metadata`) sees the link itself.
        let meta = fs::symlink_metadata(&target).expect("present");
        assert!(
            meta.file_type().is_symlink(),
            "std reports a junction (a name-surrogate reparse point) as a link"
        );
        // Reading through it reaches the central bytes.
        assert_eq!(
            fs::read_to_string(target.join("SKILL.md")).unwrap(),
            fs::read_to_string(central.join("SKILL.md")).unwrap()
        );
        assert_eq!(
            fs::read(target.join("scripts").join("run.ps1")).unwrap(),
            b"Write-Output ok\r\n"
        );
        assert_eq!(
            fs::canonicalize(&target).unwrap(),
            fs::canonicalize(&central).unwrap()
        );
    }

    #[test]
    fn remove_path_any_removes_the_junction_and_never_the_central_skill() {
        let tmp = tempfile::tempdir().unwrap();
        let central = central_skill(tmp.path(), "demo");
        let target = junction_target(tmp.path(), &central, "demo");

        remove_path_any(&target).expect("remove junction");

        assert_eq!(
            fs::symlink_metadata(&target).unwrap_err().kind(),
            std::io::ErrorKind::NotFound
        );
        assert_central_intact(&central, "demo");
    }

    /// A junction whose target is gone still counts as present and is removed.
    #[test]
    fn a_dangling_junction_is_present_and_removed() {
        let tmp = tempfile::tempdir().unwrap();
        let central = central_skill(tmp.path(), "demo");
        let target = junction_target(tmp.path(), &central, "demo");
        fs::remove_dir_all(&central).unwrap();

        assert!(fs::symlink_metadata(&target).is_ok(), "present");
        assert!(!target.exists(), "dangling");
        remove_path_any(&target).expect("remove dangling junction");
        assert!(fs::symlink_metadata(&target).is_err());
    }

    /// Windows' `ERROR_PRIVILEGE_NOT_HELD`.
    const ERROR_PRIVILEGE_NOT_HELD: i32 = 1314;

    /// Create a symlink, or report that this host may not. Windows creates
    /// symlinks only for a process holding `SeCreateSymbolicLinkPrivilege`
    /// (elevated, or Developer Mode on); without it the shape cannot exist
    /// here at all — the deploy fallback makes a junction instead, which the
    /// junction tests cover — so the calling test has nothing to remove and
    /// passes, saying so on stderr. Any other failure is a real one.
    fn symlink_or_skip(made: std::io::Result<()>, test: &str) -> bool {
        match made {
            Ok(()) => true,
            Err(err) if err.raw_os_error() == Some(ERROR_PRIVILEGE_NOT_HELD) => {
                eprintln!("{test}: skipped, this process may not create symlinks ({err})");
                false
            }
            Err(err) => panic!("{test}: create symlink: {err}"),
        }
    }

    /// A directory symlink (the first leg, where the host allows it) is removed
    /// the same way, as itself.
    #[test]
    fn remove_path_any_removes_a_directory_symlink_and_never_its_target() {
        let tmp = tempfile::tempdir().unwrap();
        let central = central_skill(tmp.path(), "demo");
        let link = tmp.path().join("via-symlink");
        if !symlink_or_skip(
            std::os::windows::fs::symlink_dir(&central, &link),
            "directory symlink",
        ) {
            return;
        }

        remove_path_any(&link).expect("remove directory symlink");

        assert!(fs::symlink_metadata(&link).is_err());
        assert_central_intact(&central, "demo");
    }

    /// A file symlink is a file entry: removed as itself, its target's bytes
    /// untouched.
    #[test]
    fn remove_path_any_removes_a_file_symlink_and_never_its_target() {
        let tmp = tempfile::tempdir().unwrap();
        let central = central_skill(tmp.path(), "demo");
        let link = tmp.path().join("SKILL-link.md");
        if !symlink_or_skip(
            std::os::windows::fs::symlink_file(central.join("SKILL.md"), &link),
            "file symlink",
        ) {
            return;
        }
        assert!(!fs::symlink_metadata(&link).unwrap().file_type().is_dir());

        remove_path_any(&link).expect("remove file symlink");

        assert!(fs::symlink_metadata(&link).is_err());
        assert_central_intact(&central, "demo");
    }

    /// A dangling directory symlink keeps its own directory flag: it is
    /// present, removed with `remove_dir`, and nothing is created at its
    /// missing referent.
    #[test]
    fn a_dangling_directory_symlink_is_present_and_removed() {
        let tmp = tempfile::tempdir().unwrap();
        let gone = tmp.path().join("gone-dir");
        let link = tmp.path().join("dangling-dir-link");
        if !symlink_or_skip(
            std::os::windows::fs::symlink_dir(&gone, &link),
            "dangling directory symlink",
        ) {
            return;
        }
        assert!(fs::symlink_metadata(&link).is_ok(), "present");
        assert!(!link.exists(), "dangling");

        remove_path_any(&link).expect("remove dangling directory symlink");

        assert!(fs::symlink_metadata(&link).is_err());
        assert!(
            fs::symlink_metadata(&gone).is_err(),
            "referent never created"
        );
    }

    /// A dangling file symlink: present, removed, its missing referent left
    /// missing.
    #[test]
    fn a_dangling_file_symlink_is_present_and_removed() {
        let tmp = tempfile::tempdir().unwrap();
        let gone = tmp.path().join("gone.md");
        let link = tmp.path().join("dangling-file-link.md");
        if !symlink_or_skip(
            std::os::windows::fs::symlink_file(&gone, &link),
            "dangling file symlink",
        ) {
            return;
        }
        assert!(fs::symlink_metadata(&link).is_ok(), "present");
        assert!(!link.exists(), "dangling");

        remove_path_any(&link).expect("remove dangling file symlink");

        assert!(fs::symlink_metadata(&link).is_err());
        assert!(
            fs::symlink_metadata(&gone).is_err(),
            "referent never created"
        );
    }

    #[test]
    fn artifact_removal_of_a_junction_target_deletes_the_row_and_keeps_the_central_skill() {
        let tmp = tempfile::tempdir().unwrap();
        let central = central_skill(tmp.path(), "demo");
        let target = junction_target(tmp.path(), &central, "demo");
        let (store, skill) = store_with_junction_row(tmp.path(), &central, "demo", &target);

        let scope = RemovalScope::SkillGlobal {
            skill_id: skill.id.clone(),
        };
        let report = execute_unlocked(&store, plan(&store, &scope).unwrap()).unwrap();

        assert_eq!(
            (report.removed_rows(), report.failed_rows()),
            (1, 0),
            "{report:?}"
        );
        assert!(fs::symlink_metadata(&target).is_err(), "junction removed");
        assert!(store.list_skill_targets(&skill.id).unwrap().is_empty());
        assert_central_intact(&central, "demo");
    }

    #[test]
    fn disabling_a_junction_target_keeps_the_row_disabled_and_the_central_skill() {
        let tmp = tempfile::tempdir().unwrap();
        let central = central_skill(tmp.path(), "demo");
        let target = junction_target(tmp.path(), &central, "demo");
        let (store, skill) = store_with_junction_row(tmp.path(), &central, "demo", &target);

        let scope = RemovalScope::SkillGlobal {
            skill_id: skill.id.clone(),
        };
        let report =
            execute_settling_unlocked(&store, plan(&store, &scope).unwrap(), Settlement::Disable)
                .unwrap();

        assert_eq!(report.failed_rows(), 0, "{report:?}");
        assert!(fs::symlink_metadata(&target).is_err(), "junction removed");
        let rows = store.list_skill_targets(&skill.id).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, SyncStatus::Disabled);
        assert_central_intact(&central, "demo");
    }

    #[test]
    fn overwriting_a_junction_target_replaces_the_link_not_the_central_bytes() {
        let tmp = tempfile::tempdir().unwrap();
        let old_central = central_skill(tmp.path(), "old");
        let target = junction_target(tmp.path(), &old_central, "demo");
        let new_central = central_skill(tmp.path(), "new");

        let out = sync_dir_hybrid_with_overwrite(
            &new_central,
            Source::Directory(&new_central),
            &target,
            true,
        )
        .expect("overwrite");

        assert!(out.replaced);
        assert_central_intact(&old_central, "old");
        assert_eq!(
            fs::read_to_string(target.join("SKILL.md")).unwrap(),
            fs::read_to_string(new_central.join("SKILL.md")).unwrap()
        );
    }

    #[test]
    fn a_junction_collision_without_overwrite_is_target_exists_and_touches_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let old_central = central_skill(tmp.path(), "old");
        let target = junction_target(tmp.path(), &old_central, "demo");
        let new_central = central_skill(tmp.path(), "new");

        let err = sync_dir_hybrid_with_overwrite(
            &new_central,
            Source::Directory(&new_central),
            &target,
            false,
        )
        .unwrap_err();

        assert!(
            err.downcast_ref::<crate::core::sync_engine::TargetExistsError>()
                .is_some(),
            "{err:#}"
        );
        assert!(junction::exists(&target).unwrap());
        assert_central_intact(&old_central, "old");
    }

    /// Content identity reads a junction target as it reads a symlink: the
    /// central bytes behind it.
    #[test]
    fn content_identity_through_a_junction_matches_a_symlink() {
        let tmp = tempfile::tempdir().unwrap();
        let central = central_skill(tmp.path(), "demo");
        let target = junction_target(tmp.path(), &central, "demo");
        let other = central_skill(tmp.path(), "other");

        assert_eq!(
            content_identity::compare(Source::Directory(&central), &target),
            Comparison::Same
        );
        assert!(content_identity::same_content(
            Source::Directory(&central),
            &target
        ));
        assert_eq!(
            content_identity::compare(Source::Directory(&other), &target),
            Comparison::Different
        );
        assert!(Occupant::observe(Source::Directory(&central), &target).same_content);
        assert_eq!(
            content_identity::read(Source::Directory(&target)),
            content_identity::read(Source::Directory(&central))
        );

        // Where this host may create a directory symlink (an elevated shell, a
        // CI runner), the two link kinds must read identically.
        let link = tmp
            .path()
            .join("tools")
            .join("claude")
            .join("skills")
            .join("via-symlink");
        if std::os::windows::fs::symlink_dir(&central, &link).is_ok() {
            assert_eq!(
                content_identity::read(Source::Directory(&link)),
                content_identity::read(Source::Directory(&target))
            );
            assert_eq!(
                content_identity::compare(Source::Directory(&central), &link),
                Comparison::Same
            );
        }
    }
}
