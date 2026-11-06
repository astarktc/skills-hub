//! Tests for `core::removal_fence` — physical admission at the seam every
//! removal crosses (`sync_engine::remove_path_any`). The callers' own tests
//! (artifact removal, overwrite, Import's originals, the git cache) cover the
//! P0 shapes end to end; these pin the rules those tests do not reach.

use std::fs;
use std::path::{Path, PathBuf};

use super::{admit, admit_central_copy, Claim, Entry, Fence, OwnedDir, ProtectedLibrary};
use crate::core::errors::{RemovalRefusal, SignalError};
use crate::core::skill_store::{SkillRecord, SkillStore};

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
