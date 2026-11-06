//! Tests for `core::removal_fence` — physical admission at the seam every
//! removal crosses (`sync_engine::remove_path_any`). The callers' own tests
//! (artifact removal, overwrite, Import's originals, the git cache) cover the
//! P0 shapes end to end; these pin the rules those tests do not reach.

use std::fs;
use std::path::{Path, PathBuf};

use super::{admit, Claim, Entry, Fence, ProtectedLibrary};
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
    let library = ProtectedLibrary::recorded(&store);

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

/// A folder beside a central copy is not the library: only the copies and
/// the repo folder itself are fenced.
#[test]
fn a_sibling_of_a_central_copy_is_admitted() {
    let tmp = tempfile::tempdir().unwrap();
    let central = tmp.path().join("central").join("alpha");
    fs::create_dir_all(&central).unwrap();
    let store = store_with_copy(tmp.path(), &central);
    let sibling = tmp.path().join("central").join("leftover");
    fs::create_dir_all(&sibling).unwrap();
    let library = ProtectedLibrary::recorded(&store);

    let physical = admitted_at(admit(&sibling, &Fence::new(&library)));
    assert_eq!(physical, sibling.canonicalize().unwrap());
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
    let root = tmp.path().join("cache-root");
    fs::create_dir_all(root.join("entry").join("inner")).unwrap();
    let library = ProtectedLibrary::unrecorded();
    let fence = Fence::new(&library).claiming(Claim::ChildOfOwned(&root));

    admitted_at(admit(&root.join("entry"), &fence));
    assert_eq!(
        refusal(admit(&root.join("entry").join("inner"), &fence)),
        Some(RemovalRefusal::ResolvesElsewhere)
    );
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
