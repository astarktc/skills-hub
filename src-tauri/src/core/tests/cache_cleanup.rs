//! Cache-cleanup tests: how an entry nobody fetches any more — a pre-policy
//! entry the checkout-policy key change orphaned — leaves the git cache.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::cleanup_git_cache_dirs_in;
use crate::core::clock::now_ms;
use crate::core::errors::{RemovalRefusal, SignalError};
use crate::core::git_cache::{repo_cache_key, repo_cache_key_under_policy, CacheKeyInputs};
use crate::core::removal_fence::ProtectedLibrary;
use crate::core::skill_store::{SkillRecord, SkillStore};

const DAY: Duration = Duration::from_secs(24 * 60 * 60);

const INPUTS: CacheKeyInputs<'static> = CacheKeyInputs {
    clone_url: "https://example.com/owner/repo.git",
    branch: None,
};

/// A cache entry as the clone cache leaves it: a clone (`.git`) and,
/// optionally, a record carrying its last fetch time.
fn entry(cache_dir: &Path, key: &str, last_fetched_ms: Option<i64>) -> PathBuf {
    let dir = cache_dir.join("skills-hub-git-cache").join(key);
    fs::create_dir_all(dir.join(".git")).expect("mkdir");
    fs::write(dir.join("SKILL.md"), "---\r\nname: a\r\n---\r\n").expect("write");
    if let Some(ms) = last_fetched_ms {
        fs::write(
            dir.join(".skills-hub-cache.json"),
            serde_json::json!({ "last_fetched_ms": ms, "head": "h" }).to_string(),
        )
        .expect("record");
    }
    dir
}

/// The pre-policy entry's record stops at its last fetch under the old key —
/// nothing looks that key up again — so once that fetch is older than the
/// cleanup age it is reclaimed, while the current policy's entry of the same
/// repository, fetched since, stays.
#[test]
fn an_orphaned_pre_policy_entry_ages_out_by_its_record() {
    let cache = tempfile::tempdir().expect("tempdir");
    let old_entry = entry(
        cache.path(),
        &repo_cache_key_under_policy(&INPUTS, 0),
        Some(now_ms() - 31 * DAY.as_millis() as i64),
    );
    let current = entry(cache.path(), &repo_cache_key(&INPUTS), Some(now_ms()));

    let removed =
        cleanup_git_cache_dirs_in(cache.path(), &ProtectedLibrary::unrecorded(), 30 * DAY)
            .expect("cleanup");

    assert_eq!(removed, 1);
    assert!(!old_entry.exists(), "the orphaned entry is reclaimed");
    assert!(current.exists(), "the current policy's entry stays");
}

/// A pre-policy entry without a readable record ages out by its directory's
/// modification time, which nothing touches once its key is retired.
#[test]
fn an_orphaned_recordless_pre_policy_entry_ages_out_by_its_directory_time() {
    let cache = tempfile::tempdir().expect("tempdir");
    let old_entry = entry(cache.path(), &repo_cache_key_under_policy(&INPUTS, 0), None);
    let long_ago = SystemTime::now() - 31 * DAY;
    filetime::set_file_mtime(&old_entry, filetime::FileTime::from_system_time(long_ago))
        .expect("age the directory");
    let current = entry(cache.path(), &repo_cache_key(&INPUTS), Some(now_ms()));

    let removed =
        cleanup_git_cache_dirs_in(cache.path(), &ProtectedLibrary::unrecorded(), 30 * DAY)
            .expect("cleanup");

    assert_eq!(removed, 1);
    assert!(!old_entry.exists(), "the orphaned entry is reclaimed");
    assert!(current.exists(), "the current policy's entry stays");
}

/// The typed refusal `reason` the cleanup ended with.
fn refused(result: anyhow::Result<usize>) -> RemovalRefusal {
    let err = result.expect_err("the cleanup is refused");
    match err.downcast_ref::<SignalError>() {
        Some(SignalError::RemovalRefused { reason, .. }) => *reason,
        _ => panic!("expected a typed refusal, got {err:#}"),
    }
}

/// An old checkout-shaped folder with user bytes, as the age-out would see a
/// stale cache entry. Only the alias tests (Unix links) build one.
#[cfg(unix)]
fn old_checkout(dir: &Path) -> PathBuf {
    fs::create_dir_all(dir.join(".git")).expect("mkdir");
    fs::write(
        dir.join(".skills-hub-cache.json"),
        serde_json::json!({ "last_fetched_ms": 1, "head": "h" }).to_string(),
    )
    .expect("record");
    fs::write(dir.join("keep"), "outside bytes").expect("write");
    dir.join("keep")
}

/// BACKLOG #107, the cleanup's side of R1: a cache root that is a link to a
/// directory outside the cache never makes the age-out delete what lives
/// there — however old its `.git` folders are. The cache is not the app's,
/// so the whole cleanup is refused.
#[cfg(unix)]
#[test]
fn an_aliased_cache_root_is_never_reclaimed_through() {
    let cache = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("tempdir");
    std::os::unix::fs::symlink(outside.path(), cache.path().join("skills-hub-git-cache"))
        .expect("alias root");
    let keep = old_checkout(&outside.path().join("a-real-checkout"));

    let result = cleanup_git_cache_dirs_in(cache.path(), &ProtectedLibrary::unrecorded(), 30 * DAY);

    assert_eq!(refused(result), RemovalRefusal::ResolvesElsewhere);
    assert!(keep.is_file(), "the outside folder survives");
}

/// Review finding 2 (BACKLOG #107): a real `skills-hub-git-cache` reached
/// through an aliased app cache dir (`cache -> outside`) is not the app's
/// either — the trusted boundary is the app cache dir itself, and an alias on
/// any level the app controls refuses the cleanup.
#[cfg(unix)]
#[test]
fn a_cache_reached_through_an_aliased_app_cache_dir_is_never_reclaimed() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let outside = tmp.path().join("outside");
    let keep = old_checkout(
        &outside
            .join("skills-hub-git-cache")
            .join(repo_cache_key(&INPUTS)),
    );
    let cache = tmp.path().join("cache");
    std::os::unix::fs::symlink(&outside, &cache).expect("alias the app cache dir");

    let result = cleanup_git_cache_dirs_in(&cache, &ProtectedLibrary::unrecorded(), 30 * DAY);

    assert_eq!(refused(result), RemovalRefusal::ResolvesElsewhere);
    assert!(keep.is_file(), "the outside bytes survive");
}

/// Review finding 3 (BACKLOG #107): a central repo configured inside the
/// cache (an old checkout reused as the library) overlaps it — the cleanup
/// refuses rather than deletes, and the recorded skill survives.
#[test]
fn a_cache_overlapping_the_configured_library_is_not_cleaned() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let cache = tmp.path().join("cache");
    let configured_library = entry(&cache, &repo_cache_key(&INPUTS), Some(1));
    let central = configured_library.join("alpha");
    fs::create_dir_all(&central).expect("mkdir");
    fs::write(central.join("keep"), "managed library bytes").expect("write");
    let store = SkillStore::new(tmp.path().join("test.db"));
    store.ensure_schema().expect("schema");
    store
        .upsert_skill(&SkillRecord {
            id: "alpha".into(),
            name: "alpha".into(),
            description: None,
            source_type: "local".into(),
            source_ref: None,
            source_subpath: None,
            source_revision: None,
            central_path: central.to_string_lossy().into_owned(),
            content_hash: None,
            created_at: 1,
            updated_at: 1,
            last_sync_at: None,
            last_seen_at: 1,
            status: "ok".into(),
            imported_from_tool: None,
        })
        .expect("record the skill");

    // Either half of the library is enough: the configured repo alone (no
    // record of it yet), or the recorded copy alone (a library left behind
    // by a move).
    let unrecorded = SkillStore::new(tmp.path().join("empty.db"));
    unrecorded.ensure_schema().expect("schema");
    for (store, configured) in [
        (&unrecorded, configured_library.clone()),
        (&store, tmp.path().join("central")),
    ] {
        let library = ProtectedLibrary::new(store, &configured);
        let result = cleanup_git_cache_dirs_in(&cache, &library, 30 * DAY);
        assert_eq!(
            refused(result),
            RemovalRefusal::CentralLibrary,
            "{configured:?}"
        );
        assert!(central.join("keep").is_file(), "the library survives");
    }
}
