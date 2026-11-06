//! Tests for `core::removal_fence` — physical admission at the seam every
//! removal crosses (`sync_engine::remove_path_any`). The callers' own tests
//! (artifact removal, overwrite, Import's originals, the git cache) cover the
//! P0 shapes end to end; these pin the rules those tests do not reach.

use std::fs;
use std::path::{Path, PathBuf};

use super::{
    admit, admit_central_copy, test_seams, Claim, Entry, Fence, OwnedDir, ProtectedLibrary,
};
use crate::core::errors::{RemovalRefusal, SignalError};
use crate::core::skill_store::{SkillRecord, SkillStore};
use crate::core::sync_engine::remove_path_any;

fn store_with_copy(base: &Path, central: &Path) -> SkillStore {
    let store = SkillStore::new(base.join("test.db"));
    store.ensure_schema().unwrap();
    store
        .upsert_skill(&SkillRecord {
            id: "s1".to_string(),
            name: "alpha".to_string(),
            description: None,
            source_type: "local".to_string(),
            source_ref: None,
            source_subpath: None,
            source_revision: None,
            central_path: central.to_string_lossy().into_owned(),
            content_hash: None,
            created_at: 1,
            updated_at: 1,
            last_sync_at: None,
            last_seen_at: 1,
            status: "ok".to_string(),
            imported_from_tool: None,
        })
        .unwrap();
    store
}

fn refusal(result: anyhow::Result<Entry>) -> Option<RemovalRefusal> {
    match result {
        Err(err) => match err.downcast_ref::<SignalError>() {
            Some(SignalError::RemovalRefused { reason, .. }) => Some(*reason),
            _ => panic!("expected a typed refusal, got {err:#}"),
        },
        Ok(entry) => panic!("expected a refusal, got {entry:?}"),
    }
}

fn admitted_at(result: anyhow::Result<Entry>) -> PathBuf {
    match result.expect("admitted") {
        Entry::Present { physical, .. } => physical,
        Entry::Absent => panic!("expected a present entry"),
    }
}

/// The folder holding the central copies — the central repo — is never
/// removed, nor anything around it, however it is reached.
#[cfg(unix)]
#[test]
fn the_folder_around_the_central_repo_is_refused_through_an_alias() {
    let tmp = tempfile::tempdir().unwrap();
    let library_home = tmp.path().join("library-home");
    let central = library_home.join("central").join("alpha");
    fs::create_dir_all(&central).unwrap();
    let store = store_with_copy(tmp.path(), &central);
    let tools = tmp.path().join("tools");
    fs::create_dir_all(&tools).unwrap();
    // `tools/skills` → `library-home`: its `central` entry is the repo.
    std::os::unix::fs::symlink(&library_home, tools.join("skills")).unwrap();
    let central_dir = library_home.join("central");
    let library = ProtectedLibrary::new(&store, &central_dir);

    for path in [
        tools.join("skills").join("central"),
        tmp.path().join("library-home"),
    ] {
        assert_eq!(
            refusal(admit(&path, &Fence::new(&library))),
            Some(RemovalRefusal::CentralLibrary),
            "{path:?}"
        );
    }
    assert!(central.is_dir());
}

/// A folder beside a recorded copy, outside the configured central repo (a
/// library left behind by a move), is not the library: there only the copies
/// and the folder holding them are fenced.
#[test]
fn a_sibling_of_a_recorded_copy_outside_the_configured_repo_is_admitted() {
    let tmp = tempfile::tempdir().unwrap();
    let central = tmp.path().join("old-library").join("alpha");
    fs::create_dir_all(&central).unwrap();
    let store = store_with_copy(tmp.path(), &central);
    let sibling = tmp.path().join("old-library").join("leftover");
    fs::create_dir_all(&sibling).unwrap();
    let configured = tmp.path().join("central");
    let library = ProtectedLibrary::new(&store, &configured);

    let physical = admitted_at(admit(&sibling, &Fence::new(&library)));
    assert_eq!(physical, sibling.canonicalize().unwrap());
}

/// Review finding 3 (BACKLOG #107): the configured central repo is protected
/// with its whole subtree — an unrecorded child (a staging folder, the
/// Explore cache) reached through a Tool dir linked onto the repo is refused,
/// as is the repo itself and anything around it, recorded or not.
#[cfg(unix)]
#[test]
fn the_configured_central_repo_is_protected_whole_through_an_alias() {
    let tmp = tempfile::tempdir().unwrap();
    let central_dir = tmp.path().join("central");
    fs::create_dir_all(central_dir.join("alpha")).unwrap();
    fs::create_dir_all(central_dir.join(".staging-acquisition")).unwrap();
    fs::write(
        central_dir.join(".staging-acquisition/keep"),
        "acquired bytes",
    )
    .unwrap();
    let store = store_with_copy(tmp.path(), &central_dir.join("alpha"));
    let alias = tmp.path().join("skills");
    std::os::unix::fs::symlink(&central_dir, &alias).unwrap();
    let library = ProtectedLibrary::new(&store, &central_dir);

    for path in [
        alias.join(".staging-acquisition"),
        alias.join(".staging-acquisition").join("keep"),
        central_dir.clone(),
        tmp.path().to_path_buf(),
    ] {
        assert_eq!(
            refusal(admit(&path, &Fence::new(&library))),
            Some(RemovalRefusal::CentralLibrary),
            "{path:?}"
        );
    }
    assert!(central_dir.join(".staging-acquisition/keep").is_file());
}

/// A configured central repo that does not exist yet still protects where it
/// would be: nothing around it is removable.
#[test]
fn a_missing_configured_repo_protects_its_existing_ancestors() {
    let tmp = tempfile::tempdir().unwrap();
    let store = store_with_copy(tmp.path(), &tmp.path().join("elsewhere").join("alpha"));
    let parent = tmp.path().join("library-home");
    fs::create_dir_all(&parent).unwrap();
    let central_dir = parent.join("central");
    let library = ProtectedLibrary::new(&store, &central_dir);

    assert_eq!(
        refusal(admit(&parent, &Fence::new(&library))),
        Some(RemovalRefusal::CentralLibrary)
    );
}

/// Review finding 1 (BACKLOG #107): on a case-insensitive volume a leaf
/// spelled in another case under a parent aliased onto the library *is* the
/// central copy. Admission compares physical identity, never spelling, so it
/// is refused; on a case-sensitive volume the variant is a different (here
/// absent) entry.
#[cfg(unix)]
#[test]
fn a_case_variant_leaf_under_an_alias_is_the_copy_it_names() {
    let tmp = tempfile::tempdir().unwrap();
    let central_dir = tmp.path().join("central");
    let central = central_dir.join("alpha");
    fs::create_dir_all(&central).unwrap();
    fs::write(central.join("keep"), "library bytes").unwrap();
    let store = store_with_copy(tmp.path(), &central);
    let alias = tmp.path().join("skills");
    std::os::unix::fs::symlink(&central_dir, &alias).unwrap();
    let case_insensitive = alias.join("ALPHA").join("keep").exists();
    // Not even a historical library: only the recorded copy protects it.
    let elsewhere = tmp.path().join("configured-elsewhere");
    let library = ProtectedLibrary::new(&store, &elsewhere);

    let admitted = admit(&alias.join("ALPHA"), &Fence::new(&library));

    if case_insensitive {
        assert_eq!(refusal(admitted), Some(RemovalRefusal::CentralLibrary));
    } else {
        assert!(matches!(admitted.unwrap(), Entry::Absent));
    }
    assert!(central.join("keep").is_file());
}

/// An operator may link a Tool's skills dir elsewhere (dotfiles): a path
/// spelled under the claimed dir and physically in its referent is admitted,
/// at its physical location.
#[cfg(unix)]
#[test]
fn a_claimed_dir_that_is_itself_an_alias_is_honoured() {
    let tmp = tempfile::tempdir().unwrap();
    let dotfiles = tmp.path().join("dotfiles").join("skills");
    fs::create_dir_all(dotfiles.join("alpha")).unwrap();
    let tool_dir = tmp.path().join("home").join(".claude").join("skills");
    fs::create_dir_all(tool_dir.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&dotfiles, &tool_dir).unwrap();
    let library = ProtectedLibrary::unrecorded();
    let dirs = [tool_dir.clone()];

    let physical = admitted_at(admit(
        &tool_dir.join("alpha"),
        &Fence::new(&library).claiming(Claim::Within(&dirs)),
    ));
    assert_eq!(physical, dotfiles.canonicalize().unwrap().join("alpha"));
}

/// Spelled inside a claimed dir, physically elsewhere — an alias *below* the
/// claimed dir — is refused as `ResolvesElsewhere`.
#[cfg(unix)]
#[test]
fn an_alias_below_a_claimed_dir_resolves_elsewhere_and_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let tool_dir = tmp.path().join("tool").join("skills");
    fs::create_dir_all(&tool_dir).unwrap();
    let outside = tmp.path().join("outside");
    fs::create_dir_all(outside.join("alpha")).unwrap();
    std::os::unix::fs::symlink(&outside, tool_dir.join("nested")).unwrap();
    let library = ProtectedLibrary::unrecorded();
    let dirs = [tool_dir.clone()];

    assert_eq!(
        refusal(admit(
            &tool_dir.join("nested").join("alpha"),
            &Fence::new(&library).claiming(Claim::Within(&dirs)),
        )),
        Some(RemovalRefusal::ResolvesElsewhere)
    );
    assert!(outside.join("alpha").is_dir());
}

/// A direct child of a real, app-owned directory is admitted; a grandchild
/// is not that directory's child.
#[test]
fn an_owned_root_admits_only_its_direct_children() {
    let tmp = tempfile::tempdir().unwrap();
    let boundary = tmp.path().join("app-cache");
    let root_path = boundary.join("cache-root");
    fs::create_dir_all(root_path.join("entry").join("inner")).unwrap();
    let relative = Path::new("cache-root");
    let root = OwnedDir::resolve(&boundary, relative)
        .unwrap()
        .expect("present");
    let library = ProtectedLibrary::unrecorded();
    let fence = Fence::new(&library).claiming(Claim::ChildOfOwned(&root));

    admitted_at(admit(&root_path.join("entry"), &fence));
    assert_eq!(
        refusal(admit(&root_path.join("entry").join("inner"), &fence)),
        Some(RemovalRefusal::ResolvesElsewhere)
    );
}

/// Review finding 2 (BACKLOG #107): an owned directory is the app's only
/// when every level from the trusted boundary down is a real directory — a
/// boundary that is itself a link (`cache -> outside`) or a link below it
/// (`skills-hub-git-cache -> outside`) is refused; the boundary's own
/// ancestors may resolve anywhere.
#[cfg(unix)]
#[test]
fn an_owned_dir_reached_through_an_alias_on_the_apps_path_is_refused() {
    use std::os::unix::fs::symlink;

    let tmp = tempfile::tempdir().unwrap();
    let outside = tmp.path().join("outside");
    fs::create_dir_all(outside.join("cache-root")).unwrap();
    let relative = Path::new("cache-root");

    let aliased_boundary = tmp.path().join("cache");
    symlink(&outside, &aliased_boundary).unwrap();
    let real_boundary = tmp.path().join("real-cache");
    fs::create_dir_all(&real_boundary).unwrap();
    symlink(outside.join("cache-root"), real_boundary.join("cache-root")).unwrap();
    for boundary in [&aliased_boundary, &real_boundary] {
        let err = OwnedDir::resolve(boundary, relative).expect_err("refused");
        assert!(
            matches!(
                err.downcast_ref::<SignalError>(),
                Some(SignalError::RemovalRefused {
                    reason: RemovalRefusal::ResolvesElsewhere,
                    ..
                })
            ),
            "{boundary:?}: {err:#}"
        );
    }

    // An aliased ancestor of a real boundary is the operator's layout.
    let home_alias = tmp.path().join("home-alias");
    symlink(tmp.path().join("home"), &home_alias).unwrap();
    fs::create_dir_all(tmp.path().join("home").join("cache").join("cache-root")).unwrap();
    let boundary = home_alias.join("cache");
    let root = OwnedDir::resolve(&boundary, relative)
        .unwrap()
        .expect("present");
    assert_eq!(
        root.path(),
        tmp.path()
            .join("home/cache/cache-root")
            .canonicalize()
            .unwrap()
    );
}

/// The Delete settlement's own central copy is admitted under the record's
/// authority — but a record that reaches the rest of the library (the
/// configured repo itself, or a folder around another skill's copy) is
/// refused.
#[test]
fn a_central_copy_is_admitted_only_for_its_own_bytes() {
    let tmp = tempfile::tempdir().unwrap();
    let central_dir = tmp.path().join("central");
    let alpha = central_dir.join("alpha");
    fs::create_dir_all(&alpha).unwrap();
    let store = store_with_copy(tmp.path(), &alpha);
    let library = ProtectedLibrary::new(&store, &central_dir);

    let physical = admitted_at(admit_central_copy(&alpha, "s1", &library));
    assert_eq!(physical, alpha.canonicalize().unwrap());

    for path in [central_dir.clone(), tmp.path().to_path_buf()] {
        assert_eq!(
            refusal(admit_central_copy(&path, "other", &library)),
            Some(RemovalRefusal::CentralLibrary),
            "{path:?}"
        );
    }
}

/// Absence is only `NotFound` — of the leaf or of its parent — and nothing
/// is refused that is not there.
#[test]
fn an_absent_leaf_or_parent_is_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let library = ProtectedLibrary::unrecorded();
    for path in [
        tmp.path().join("missing"),
        tmp.path().join("gone").join("missing"),
    ] {
        assert!(matches!(
            admit(&path, &Fence::new(&library)).unwrap(),
            Entry::Absent
        ));
    }
}

/// A relative path locates nothing in particular: refused before any read.
#[test]
fn a_relative_path_is_refused() {
    let library = ProtectedLibrary::unrecorded();
    assert!(admit(Path::new("relative/skill"), &Fence::new(&library)).is_err());
}

/// A Tool target with a nested folder and a file in it — the shape a
/// recursive removal walks.
fn target_with_nested(base: &Path) -> (PathBuf, PathBuf) {
    let target = base.join("home/.claude/skills/alpha");
    let attached = target.join("attached-library");
    fs::create_dir_all(&attached).unwrap();
    fs::write(target.join("SKILL.md"), "---\nname: alpha\n---\n").unwrap();
    (target, attached)
}

fn removal_refusal(result: anyhow::Result<()>) -> RemovalRefusal {
    let err = result.expect_err("the removal is refused");
    match err.downcast_ref::<SignalError>() {
        Some(SignalError::RemovalRefused { reason, .. }) => *reason,
        _ => panic!("expected a typed refusal, got {err:#}"),
    }
}

/// Re-check P1 (BACKLOG #107): a folder inside a Tool target that is a bind
/// mount of the library reports the library's identity. A recursive removal
/// of the target would cross it and delete the library's bytes, so the
/// removal is refused before anything is deleted — the target's own files
/// included.
#[test]
fn a_folder_inside_the_entry_with_the_library_identity_refuses_the_removal() {
    let tmp = tempfile::tempdir().unwrap();
    let central_dir = tmp.path().join("central");
    let alpha = central_dir.join("alpha");
    fs::create_dir_all(&alpha).unwrap();
    fs::write(alpha.join("keep"), "library bytes").unwrap();
    let store = store_with_copy(tmp.path(), &alpha);
    let library = ProtectedLibrary::new(&store, &central_dir);
    let (target, attached) = target_with_nested(tmp.path());

    // The library's own root, a recorded copy, and a folder around the
    // library — each one mounted inside the target.
    for mounted in [central_dir.clone(), alpha.clone(), tmp.path().to_path_buf()] {
        let _mount = test_seams::mount_identity_of(&attached, &mounted);

        assert_eq!(
            refusal(admit(&target, &Fence::new(&library))),
            Some(RemovalRefusal::ContainsMount),
            "{mounted:?}"
        );
        assert_eq!(
            removal_refusal(remove_path_any(&target, &Fence::new(&library))),
            RemovalRefusal::ContainsMount
        );
        assert!(target.join("SKILL.md").is_file(), "nothing is deleted");
        assert!(alpha.join("keep").is_file(), "the library survives");
    }

    // Without the mount the same target is an ordinary removal.
    remove_path_any(&target, &Fence::new(&library)).unwrap();
    assert!(!target.exists());
    assert!(alpha.join("keep").is_file());
}

/// A folder inside the entry that is a mount of the entry itself, or of a
/// folder around it (the Tool's skills dir), would loop the recursion or
/// carry it out of the entry into its siblings: refused the same way.
#[test]
fn a_folder_inside_the_entry_with_the_identity_of_the_entry_or_around_it_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let library = ProtectedLibrary::unrecorded();
    let (target, attached) = target_with_nested(tmp.path());
    let sibling = target.with_file_name("beta");
    fs::create_dir_all(&sibling).unwrap();

    for mounted in [target.clone(), target.parent().unwrap().to_path_buf()] {
        let _mount = test_seams::mount_identity_of(&attached, &mounted);
        assert_eq!(
            removal_refusal(remove_path_any(&target, &Fence::new(&library))),
            RemovalRefusal::ContainsMount,
            "{mounted:?}"
        );
    }
    assert!(sibling.is_dir());
    assert!(target.join("SKILL.md").is_file());
}

/// Re-check P1: a folder inside the entry that sits on another device is a
/// mounted filesystem — never part of a skill folder — so a recursive removal
/// never reaches into it, whatever it holds.
#[cfg(unix)]
#[test]
fn a_folder_inside_the_entry_on_another_device_refuses_the_removal() {
    let tmp = tempfile::tempdir().unwrap();
    let library = ProtectedLibrary::unrecorded();
    let (target, attached) = target_with_nested(tmp.path());
    let deeper = attached.join("deeper");
    fs::create_dir_all(&deeper).unwrap();
    fs::write(deeper.join("keep"), "another volume's bytes").unwrap();

    let _mount = test_seams::mount_other_device(&deeper);

    assert_eq!(
        removal_refusal(remove_path_any(&target, &Fence::new(&library))),
        RemovalRefusal::ContainsMount
    );
    assert!(deeper.join("keep").is_file());
    assert!(target.join("SKILL.md").is_file());
}

/// Round-3 re-check P1: a filesystem mounted *at* the entry (a disk image
/// attached on a Tool's skill folder) reports one device throughout — the
/// entry and everything inside it — so comparing the subtree with the entry
/// finds nothing. The entry's own device against its folder's does: removing
/// it would delete the mounted filesystem's contents, so it is refused
/// before anything goes.
#[cfg(unix)]
#[test]
fn a_filesystem_mounted_at_the_entry_itself_refuses_the_removal() {
    let tmp = tempfile::tempdir().unwrap();
    let library = ProtectedLibrary::unrecorded();
    let (target, nested) = target_with_nested(tmp.path());
    fs::write(nested.join("keep"), "the mounted volume's bytes").unwrap();

    // The entry and the folder inside it both on the mounted device.
    let _entry = test_seams::mount_other_device(&target);
    let _nested = test_seams::mount_other_device(&nested);

    assert_eq!(
        refusal(admit(&target, &Fence::new(&library))),
        Some(RemovalRefusal::MountPoint)
    );
    assert_eq!(
        removal_refusal(remove_path_any(&target, &Fence::new(&library))),
        RemovalRefusal::MountPoint
    );
    assert!(target.join("SKILL.md").is_file(), "nothing is deleted");
    assert!(nested.join("keep").is_file(), "the volume survives");
}

/// The Delete settlement's own central copy is walked too: a copy holding a
/// mount of the configured repo (or of another skill's copy) is refused
/// before its own bytes go.
#[test]
fn a_central_copy_holding_a_mount_of_the_library_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let central_dir = tmp.path().join("central");
    let alpha = central_dir.join("alpha");
    let attached = alpha.join("attached");
    fs::create_dir_all(&attached).unwrap();
    fs::write(alpha.join("SKILL.md"), "---\nname: alpha\n---\n").unwrap();
    let store = store_with_copy(tmp.path(), &alpha);
    let library = ProtectedLibrary::new(&store, &central_dir);

    let _mount = test_seams::mount_identity_of(&attached, &central_dir);

    assert_eq!(
        refusal(admit_central_copy(&alpha, "s1", &library)),
        Some(RemovalRefusal::ContainsMount)
    );
    assert!(alpha.join("SKILL.md").is_file());
}

/// The mount table rule (`/proc/self/mountinfo` on Linux), driven through
/// the table seam so it runs on every Unix: read as bytes, compared as
/// paths, and failing closed when the table cannot be read or parsed.
#[cfg(unix)]
mod mount_table {
    use std::ffi::OsString;
    use std::fs;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    use std::path::PathBuf;

    use super::{admit, refusal, removal_refusal, target_with_nested};
    use crate::core::errors::{CommandError, RemovalRefusal};
    use crate::core::removal_fence::{mounts, test_seams, Entry, Fence, ProtectedLibrary};
    use crate::core::sync_engine::remove_path_any;

    /// One mountinfo line (proc(5)) mounting at `mount_point`, escaped as the
    /// kernel writes it.
    fn line(id: u32, mount_point: &[u8]) -> Vec<u8> {
        let mut escaped = Vec::new();
        for &byte in mount_point {
            match byte {
                b' ' | b'\t' | b'\n' | b'\\' => escaped.extend(format!("\\{byte:03o}").bytes()),
                byte => escaped.push(byte),
            }
        }
        let mut line = format!("{id} 22 8:1 /central ").into_bytes();
        line.extend(escaped);
        line.extend(b" rw,relatime shared:1 - ext4 /dev/sda1 rw\n");
        line
    }

    fn table(mount_points: &[&[u8]]) -> Vec<u8> {
        let mut table = b"22 1 8:1 / / rw,relatime shared:1 - ext4 /dev/sda1 rw\n".to_vec();
        for (n, mount_point) in mount_points.iter().enumerate() {
            table.extend(line(40 + n as u32, mount_point));
        }
        table
    }

    /// Every mount point is read as the bytes the kernel wrote, escapes
    /// undone — a space in a name, and a name that is not UTF-8 (round-3
    /// P1: one such mount anywhere used to discard the whole table).
    #[test]
    fn mount_points_are_parsed_as_bytes_including_a_non_utf8_mount_point() {
        let mountinfo = table(&[
            b"/home/me/.claude/skills/my skill/attached",
            b"/srv/invalid-\xff",
            b"/srv/back\\slash",
        ]);
        assert!(std::str::from_utf8(&mountinfo).is_err());

        assert_eq!(
            mounts::mount_points(&mountinfo),
            Some(vec![
                PathBuf::from("/"),
                PathBuf::from("/home/me/.claude/skills/my skill/attached"),
                PathBuf::from(OsString::from_vec(b"/srv/invalid-\xff".to_vec())),
                PathBuf::from("/srv/back\\slash"),
            ])
        );
    }

    /// A table that is not one — empty (it always lists `/`), or a line
    /// without a mount point field — is never read as "no mounts".
    #[test]
    fn a_table_that_lists_nothing_or_a_short_line_is_not_a_mount_table() {
        assert_eq!(mounts::mount_points(b""), None);
        assert_eq!(mounts::mount_points(b"22 1 8:1 /\n"), None);
    }

    /// A mount point at the entry or below it is found — a non-UTF-8 one
    /// elsewhere in the table no longer hides it; one beside the entry is not
    /// in its way.
    #[test]
    fn a_mount_listed_below_or_at_the_entry_refuses_its_removal() {
        let tmp = tempfile::tempdir().unwrap();
        let library = ProtectedLibrary::unrecorded();
        let (target, attached) = target_with_nested(tmp.path());
        let target = target.canonicalize().unwrap();
        let attached = attached.canonicalize().unwrap();
        let beside = target.with_file_name("alphabet");
        fs::create_dir_all(&beside).unwrap();

        {
            let _table = test_seams::mount_table(&table(&[
                b"/srv/invalid-\xff",
                attached.as_os_str().as_bytes(),
            ]));
            assert_eq!(
                removal_refusal(remove_path_any(&target, &Fence::new(&library))),
                RemovalRefusal::ContainsMount
            );
        }
        {
            let _table = test_seams::mount_table(&table(&[target.as_os_str().as_bytes()]));
            assert_eq!(
                refusal(admit(&target, &Fence::new(&library))),
                Some(RemovalRefusal::MountPoint)
            );
        }
        assert!(target.join("SKILL.md").is_file(), "nothing is deleted");

        let _table = test_seams::mount_table(&table(&[target.as_os_str().as_bytes()]));
        remove_path_any(&beside, &Fence::new(&library)).unwrap();
        assert!(!beside.exists());
    }

    /// Round-3 P1: a mount table that cannot be read (procfs masked or
    /// unmounted) or parsed says nothing about the mounts in a directory, so
    /// the directory's removal is refused, typed, before anything goes. A
    /// file has no subtree to cross and is still removed.
    #[test]
    fn an_unreadable_or_malformed_mount_table_refuses_a_directory_removal() {
        let tmp = tempfile::tempdir().unwrap();
        let library = ProtectedLibrary::unrecorded();
        let (target, _) = target_with_nested(tmp.path());
        let file = target.with_file_name("note.md");
        fs::write(&file, "a file").unwrap();

        let tables: [fn() -> test_seams::TableGuard; 2] =
            [test_seams::unreadable_mount_table, || {
                test_seams::mount_table(b"22 1 8:1 /\n")
            }];
        for table in tables {
            let guard = table();
            assert_eq!(
                refusal(admit(&target, &Fence::new(&library))),
                Some(RemovalRefusal::MountTableUnreadable)
            );
            assert_eq!(
                removal_refusal(remove_path_any(&target, &Fence::new(&library))),
                RemovalRefusal::MountTableUnreadable
            );
            assert!(target.join("SKILL.md").is_file(), "nothing is deleted");
            // Crosses the wire as the refusal, never as a permission failure.
            let err = remove_path_any(&target, &Fence::new(&library)).unwrap_err();
            assert!(matches!(
                CommandError::from_anyhow(err),
                CommandError::RemovalRefused {
                    reason: RemovalRefusal::MountTableUnreadable,
                    ..
                }
            ));
            drop(guard);
        }

        let _table = test_seams::unreadable_mount_table();
        assert!(matches!(
            admit(&file, &Fence::new(&library)),
            Ok(Entry::Present { .. })
        ));
        remove_path_any(&file, &Fence::new(&library)).unwrap();
        assert!(!file.exists());
    }
}

/// Linux mounts: real bind mounts.
#[cfg(target_os = "linux")]
mod linux_mounts {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use super::{store_with_copy, target_with_nested};
    use crate::core::errors::{RemovalRefusal, SignalError};
    use crate::core::removal_fence::{Fence, ProtectedLibrary};
    use crate::core::sync_engine::remove_path_any;

    /// Unmounts on drop, so a failed assertion never leaves the test's temp
    /// dir cleanup to recurse into the library.
    struct BindMount(PathBuf);

    impl BindMount {
        /// `mount --bind source at`; `None` when this process may not mount.
        fn new(source: &Path, at: &Path) -> Option<Self> {
            let mounted = Command::new("mount")
                .arg("--bind")
                .arg(source)
                .arg(at)
                .status()
                .is_ok_and(|status| status.success());
            mounted.then(|| Self(at.to_path_buf()))
        }
    }

    impl Drop for BindMount {
        fn drop(&mut self) {
            let _ = Command::new("umount").arg(&self.0).status();
        }
    }

    /// Re-check P1 for real: a bind mount of the library — or of a folder
    /// inside it — nested in a Tool target is never crossed by the target's
    /// removal. Needs mount privileges, so it is ignored by default; run it
    /// with `sudo -E cargo test --lib real_bind_mount -- --ignored`,
    /// unprivileged in a user + mount namespace with
    /// `unshare -rm cargo test --lib real_bind_mount -- --ignored`, or in a
    /// `docker run --privileged` container. Where mounting is refused it
    /// says so and passes.
    #[test]
    #[ignore = "needs mount privileges: sudo, `unshare -rm`, or a privileged container"]
    fn a_real_bind_mount_inside_a_target_is_never_crossed() {
        let tmp = tempfile::tempdir().unwrap();
        let central_dir = tmp.path().join("central");
        let alpha = central_dir.join("alpha");
        let scripts = alpha.join("scripts");
        fs::create_dir_all(&scripts).unwrap();
        fs::write(alpha.join("keep"), "library bytes").unwrap();
        fs::write(scripts.join("run.sh"), "echo ok").unwrap();
        let store = store_with_copy(tmp.path(), &alpha);
        let library = ProtectedLibrary::new(&store, &central_dir);
        let (target, attached) = target_with_nested(tmp.path());

        for source in [&central_dir, &scripts] {
            let Some(_mount) = BindMount::new(source, &attached) else {
                eprintln!(
                    "skipped: this process may not bind-mount (run under sudo or unshare -rm)"
                );
                return;
            };
            match remove_path_any(&target, &Fence::new(&library)) {
                Err(err) => assert!(
                    matches!(
                        err.downcast_ref::<SignalError>(),
                        Some(SignalError::RemovalRefused {
                            reason: RemovalRefusal::ContainsMount,
                            ..
                        })
                    ),
                    "{source:?}: {err:#}"
                ),
                Ok(()) => panic!("{source:?}: the removal crossed the mount"),
            }
            assert!(target.join("SKILL.md").is_file(), "nothing is deleted");
            assert!(alpha.join("keep").is_file(), "the library survives");
            assert!(scripts.join("run.sh").is_file(), "the library survives");
        }

        // A bind mount *at* a target shares its folder's device: only the
        // mount table shows it.
        let mounted_target = target.with_file_name("beta");
        fs::create_dir_all(&mounted_target).unwrap();
        {
            let Some(_mount) = BindMount::new(&scripts, &mounted_target) else {
                panic!("mounted once already, so this process may mount");
            };
            let err = remove_path_any(&mounted_target, &Fence::new(&library))
                .expect_err("a mount at the entry is refused");
            assert!(
                matches!(
                    err.downcast_ref::<SignalError>(),
                    Some(SignalError::RemovalRefused {
                        reason: RemovalRefusal::MountPoint,
                        ..
                    })
                ),
                "{err:#}"
            );
            assert!(scripts.join("run.sh").is_file(), "the library survives");
        }

        remove_path_any(&target, &Fence::new(&library)).unwrap();
        assert!(!target.exists());
        assert!(alpha.join("keep").is_file());
    }
}

/// macOS: a real disk image attached at a Tool target (round-3 P1, where the
/// image's contents were deleted before `EBUSY` on its root).
#[cfg(target_os = "macos")]
mod macos_mounts {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use super::removal_refusal;
    use crate::core::errors::RemovalRefusal;
    use crate::core::removal_fence::{Fence, ProtectedLibrary};
    use crate::core::sync_engine::remove_path_any;

    /// Detaches on drop, so a failed assertion never leaves an image mounted
    /// (or the temp dir's cleanup recursing into it).
    struct Attached(PathBuf);

    impl Attached {
        fn new(image: &Path, at: &Path) -> Self {
            let attached = Command::new("hdiutil")
                .args(["attach", "-nobrowse", "-mountpoint"])
                .arg(at)
                .arg(image)
                .status()
                .expect("run hdiutil");
            assert!(attached.success(), "hdiutil attach");
            Self(at.to_path_buf())
        }
    }

    impl Drop for Attached {
        fn drop(&mut self) {
            let _ = Command::new("hdiutil")
                .args(["detach", "-force"])
                .arg(&self.0)
                .status();
        }
    }

    /// A disk image attached at the target itself, and one attached below
    /// it: neither removal touches the image's bytes. Creates and attaches a
    /// 32 MB APFS image in a temp dir (no privileges, detached on drop), so
    /// it is ignored by default; run it with
    /// `cargo test --lib real_disk_image -- --ignored`.
    #[test]
    #[ignore = "creates and attaches a disk image with hdiutil"]
    fn a_real_disk_image_at_or_inside_a_target_is_never_removed() {
        let tmp = tempfile::tempdir().unwrap();
        let image = tmp.path().join("volume.dmg");
        let created = Command::new("hdiutil")
            .args(["create", "-quiet", "-size", "32m", "-fs", "APFS"])
            .args(["-volname", "SkillsHubFenceTest"])
            .arg(&image)
            .status()
            .expect("run hdiutil");
        assert!(created.success(), "hdiutil create");
        let library = ProtectedLibrary::unrecorded();

        let target = tmp.path().join("skills/alpha");
        fs::create_dir_all(&target).unwrap();
        {
            let _image = Attached::new(&image, &target);
            fs::create_dir_all(target.join("nested")).unwrap();
            fs::write(target.join("keep"), "the image's bytes").unwrap();
            assert_eq!(
                removal_refusal(remove_path_any(&target, &Fence::new(&library))),
                RemovalRefusal::MountPoint
            );
            assert!(target.join("keep").is_file(), "the image survives");
        }

        let attached = target.join("attached");
        fs::create_dir_all(&attached).unwrap();
        fs::write(target.join("SKILL.md"), "---\nname: alpha\n---\n").unwrap();
        {
            let _image = Attached::new(&image, &attached);
            assert!(attached.join("keep").is_file());
            assert_eq!(
                removal_refusal(remove_path_any(&target, &Fence::new(&library))),
                RemovalRefusal::ContainsMount
            );
            assert!(attached.join("keep").is_file(), "the image survives");
            assert!(target.join("SKILL.md").is_file(), "nothing is deleted");
        }

        remove_path_any(&target, &Fence::new(&library)).unwrap();
        assert!(!target.exists());
    }
}
