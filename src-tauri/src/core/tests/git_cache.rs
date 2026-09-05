//! Git-cache tests. Every fetch here targets a **local fixture repository**
//! cloned by path — no test in this file touches the network.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::{fetch_through_cache, key_lock, repo_cache_key, CacheKeyInputs, FetchRequest};
use crate::core::cancel_token::CancelToken;
use crate::core::errors::SignalError;

/// A long TTL: any cached clone written during a test is still fresh.
const FRESH_TTL_MS: i64 = 10 * 60 * 1000;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn git(args: &[&str], cwd: &Path) {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.com")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.com")
        .output()
        .expect("run git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A local repository with one commit, usable as a clone URL by path.
fn fixture_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    git(&["init", "-q", "-b", "main", "."], dir.path());
    fs::write(dir.path().join("SKILL.md"), "---\nname: fixture\n---\n").expect("write");
    git(&["add", "-A"], dir.path());
    git(&["commit", "-q", "-m", "init"], dir.path());
    dir
}

fn commit_more(repo: &Path, file: &str) {
    fs::write(repo.join(file), "more").expect("write");
    git(&["add", "-A"], repo);
    git(&["commit", "-q", "-m", file], repo);
}

fn url_of(repo: &Path) -> String {
    repo.to_string_lossy().to_string()
}

fn request<'a>(clone_url: &'a str, ttl_ms: i64) -> FetchRequest<'a> {
    FetchRequest {
        clone_url,
        branch: None,
        subpath: None,
        ttl_ms,
        cancel: None,
    }
}

fn cache_root(cache_dir: &Path) -> PathBuf {
    cache_dir.join("skills-hub-git-cache")
}

// ---------------------------------------------------------------------------
// The cache key
// ---------------------------------------------------------------------------

/// The key scheme is persisted on disk (directory names) and shared with the
/// explore cache, so it is pinned against an independently computed digest:
/// `printf 'URL\nBRANCH\n' | shasum -a 256`. The digest of a full clone is
/// unchanged from the shipped scheme, so every existing full entry stays
/// valid at its old name.
#[test]
fn cache_key_is_pinned_to_the_shipped_scheme() {
    assert_eq!(
        repo_cache_key(&CacheKeyInputs {
            clone_url: "https://example.com/owner/repo.git",
            branch: Some("main"),
        }),
        "4c2a09a63982c289942fa539948f44244f02350748df81f4bd7b448ed4f876dd"
    );
    assert_eq!(
        repo_cache_key(&CacheKeyInputs {
            clone_url: "https://example.com/owner/repo.git",
            branch: None,
        }),
        "b829837329c3112a34f654c04151145daf5bed54b33cd60e41c3aa365f4b5d87"
    );
}

/// The branch participates in the key: the same URL on two refs is two
/// entries.
#[test]
fn cache_key_separates_branches() {
    let main = repo_cache_key(&CacheKeyInputs {
        clone_url: "u",
        branch: Some("main"),
    });
    let dev = repo_cache_key(&CacheKeyInputs {
        clone_url: "u",
        branch: Some("dev"),
    });
    assert_ne!(main, dev);
}

// ---------------------------------------------------------------------------
// TTL semantics
// ---------------------------------------------------------------------------

/// Within the TTL the cached head is served without refetching: the fixture
/// moves on, the answer does not.
#[test]
fn fetch_within_ttl_serves_the_cached_head() {
    let repo = fixture_repo();
    let cache = tempfile::tempdir().expect("tempdir");
    let url = url_of(repo.path());

    let (dir_a, head_a) =
        fetch_through_cache(cache.path(), &request(&url, FRESH_TTL_MS)).expect("first fetch");

    commit_more(repo.path(), "second.txt");

    let (dir_b, head_b) =
        fetch_through_cache(cache.path(), &request(&url, FRESH_TTL_MS)).expect("second fetch");

    assert_eq!(dir_a, dir_b);
    assert_eq!(head_a, head_b, "a fresh cache entry must not refetch");
}

/// `ttl_ms = 0` disables freshness hits entirely (the shipped "0 = never
/// fresh" semantics), so the second fetch sees the new commit.
#[test]
fn fetch_with_zero_ttl_always_refetches() {
    let repo = fixture_repo();
    let cache = tempfile::tempdir().expect("tempdir");
    let url = url_of(repo.path());

    let (_dir, head_a) = fetch_through_cache(cache.path(), &request(&url, 0)).expect("first fetch");
    commit_more(repo.path(), "second.txt");
    let (_dir, head_b) =
        fetch_through_cache(cache.path(), &request(&url, 0)).expect("second fetch");

    assert_ne!(head_a, head_b, "ttl 0 must never serve a cached head");
}

/// A cache entry whose metadata predates the TTL window is a miss.
#[test]
fn fetch_past_the_ttl_boundary_refetches() {
    let repo = fixture_repo();
    let cache = tempfile::tempdir().expect("tempdir");
    let url = url_of(repo.path());

    let (repo_dir, head_a) =
        fetch_through_cache(cache.path(), &request(&url, FRESH_TTL_MS)).expect("first fetch");

    // Age the metadata past the window rather than sleeping through it.
    let meta_path = repo_dir.join(".skills-hub-cache.json");
    let raw = fs::read_to_string(&meta_path).expect("meta written");
    let mut meta: serde_json::Value = serde_json::from_str(&raw).expect("meta json");
    let aged = crate::core::clock::now_ms() - FRESH_TTL_MS - 1;
    meta["last_fetched_ms"] = serde_json::json!(aged);
    fs::write(&meta_path, meta.to_string()).expect("write aged meta");

    commit_more(repo.path(), "second.txt");

    let (_dir, head_b) =
        fetch_through_cache(cache.path(), &request(&url, FRESH_TTL_MS)).expect("second fetch");
    assert_ne!(head_a, head_b, "an expired entry must refetch");
}

// ---------------------------------------------------------------------------
// Corrupt cache directories
// ---------------------------------------------------------------------------

/// A cache dir holding garbage instead of a clone is rebuilt in place by the
/// one retry policy, and the fetch succeeds.
#[test]
fn corrupt_cache_dir_is_rebuilt_once() {
    let repo = fixture_repo();
    let cache = tempfile::tempdir().expect("tempdir");
    let url = url_of(repo.path());

    let key = repo_cache_key(&CacheKeyInputs {
        clone_url: &url,
        branch: None,
    });
    let repo_dir = cache_root(cache.path()).join(&key);
    fs::create_dir_all(&repo_dir).expect("create corrupt dir");
    fs::write(repo_dir.join("garbage.txt"), "not a repo").expect("write garbage");

    let (dir, head) =
        fetch_through_cache(cache.path(), &request(&url, FRESH_TTL_MS)).expect("fetch rebuilds");

    assert_eq!(dir, repo_dir);
    assert!(
        dir.join(".git").exists(),
        "the dir must be a real clone now"
    );
    assert!(!head.is_empty());
    assert!(
        !dir.join("garbage.txt").exists(),
        "the corrupt contents must be gone, not merged into the clone"
    );
}

// ---------------------------------------------------------------------------
// Cancellation
// ---------------------------------------------------------------------------

/// A pre-cancelled token yields the typed cancellation signal and leaves no
/// entry marked fresh — a later fetch must not answer from it.
#[test]
fn cancelled_fetch_leaves_no_fresh_entry() {
    let repo = fixture_repo();
    let cache = tempfile::tempdir().expect("tempdir");
    let url = url_of(repo.path());

    let token = CancelToken::new();
    token.cancel();
    let err = fetch_through_cache(
        cache.path(),
        &FetchRequest {
            cancel: Some(&token),
            ..request(&url, FRESH_TTL_MS)
        },
    )
    .expect_err("a cancelled fetch fails");
    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::Cancelled)
    );

    let key = repo_cache_key(&CacheKeyInputs {
        clone_url: &url,
        branch: None,
    });
    let repo_dir = cache_root(cache.path()).join(&key);
    assert!(
        !repo_dir.join(".skills-hub-cache.json").exists(),
        "a cancelled fetch must not write freshness metadata"
    );

    // And the cache still works afterwards.
    let (_dir, head) =
        fetch_through_cache(cache.path(), &request(&url, FRESH_TTL_MS)).expect("later fetch");
    assert!(!head.is_empty());
}

// ---------------------------------------------------------------------------
// Per-key locking
// ---------------------------------------------------------------------------

/// Two fetches of the *same* key serialise: the second cannot take the lock
/// while the first holds it.
#[test]
fn the_same_key_serialises() {
    let lock = key_lock("serialise-key");
    let held = lock.lock().expect("first acquisition");
    let again = key_lock("serialise-key");
    assert!(
        again.try_lock().is_err(),
        "a second fetch of the same key must queue behind the first"
    );
    drop(held);
    assert!(
        key_lock("serialise-key").try_lock().is_ok(),
        "the key is free again once the first fetch finishes"
    );
}

/// Two fetches of *different* keys proceed concurrently: holding key A never
/// blocks key B. This is the enabler for parallel Refresh.
#[test]
fn different_keys_do_not_block_each_other() {
    let a = key_lock("concurrent-key-a");
    let held = a.lock().expect("hold key a");

    let entered = Arc::new(AtomicBool::new(false));
    let entered_in_thread = entered.clone();
    let handle = std::thread::spawn(move || {
        let b = key_lock("concurrent-key-b");
        let _guard = b.lock().expect("key b");
        entered_in_thread.store(true, Ordering::SeqCst);
    });

    handle
        .join()
        .expect("key b thread finishes while a is held");
    assert!(
        entered.load(Ordering::SeqCst),
        "a fetch of a different key must not wait for key a"
    );
    drop(held);
}

/// Two concurrent fetches through the real entry point, of two different
/// repositories, both complete — no cross-repository serialisation.
#[test]
fn concurrent_fetches_of_different_repos_both_succeed() {
    let repo_a = fixture_repo();
    let repo_b = fixture_repo();
    let cache = tempfile::tempdir().expect("tempdir");

    let url_a = url_of(repo_a.path());
    let url_b = url_of(repo_b.path());
    let cache_path = cache.path().to_path_buf();

    std::thread::scope(|scope| {
        let one = scope.spawn(|| {
            fetch_through_cache(&cache_path, &request(&url_a, FRESH_TTL_MS)).expect("fetch a")
        });
        let two = scope.spawn(|| {
            fetch_through_cache(&cache_path, &request(&url_b, FRESH_TTL_MS)).expect("fetch b")
        });
        let (dir_a, _) = one.join().expect("thread a");
        let (dir_b, _) = two.join().expect("thread b");
        assert_ne!(dir_a, dir_b, "different repos get different cache dirs");
    });
}

// ---------------------------------------------------------------------------
// Sparse fetches share the entry
// ---------------------------------------------------------------------------

/// The Add flow's repository: two skills under `skills/`, so a listing and
/// an install of one of them are distinguishable on disk.
fn two_skill_fixture() -> tempfile::TempDir {
    let repo = fixture_repo();
    for name in ["a", "b"] {
        let dir = repo.path().join("skills").join(name);
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(dir.join("SKILL.md"), format!("---\nname: {name}\n---\n")).expect("write");
    }
    git(&["add", "-A"], repo.path());
    git(&["commit", "-q", "-m", "add subskills"], repo.path());
    repo
}

/// Every cache entry directory under the cache root — the count is the
/// number of clones the cache has made.
fn cache_entries(cache_dir: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = fs::read_dir(cache_root(cache_dir))
        .expect("cache root")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    entries.sort();
    entries
}

/// The Add flow on a non-GitHub host: the listing's full clone is the
/// install's cache hit. The fixture moves on between the two calls, so a
/// second fetch would show as a different head; the sparse request instead
/// gets the listing's directory, the listing's head, and its skill's bytes.
#[test]
fn a_full_entry_serves_a_later_subpath_request() {
    let repo = two_skill_fixture();
    let cache = tempfile::tempdir().expect("tempdir");
    let url = url_of(repo.path());

    let (listing_dir, listing_head) =
        fetch_through_cache(cache.path(), &request(&url, FRESH_TTL_MS)).expect("listing fetch");

    commit_more(repo.path(), "after-listing.txt");

    let (install_dir, install_head) = fetch_through_cache(
        cache.path(),
        &FetchRequest {
            subpath: Some("skills/a"),
            ..request(&url, FRESH_TTL_MS)
        },
    )
    .expect("install fetch");

    assert_eq!(install_dir, listing_dir, "one key for listing and install");
    assert_eq!(
        install_head, listing_head,
        "the install must be a cache hit, not a second fetch"
    );
    assert!(install_dir.join("skills/a/SKILL.md").exists());
    assert_eq!(
        cache_entries(cache.path()),
        vec![listing_dir],
        "exactly one clone directory exists after listing + install"
    );
}

/// The opposite order — a skill was installed or refreshed sparsely, then the
/// operator lists the repository — must not be fooled by the fresh entry: a
/// sparse tree does not cover a listing, so the full tree is materialised.
#[test]
fn a_sparse_entry_does_not_serve_a_later_full_request() {
    let repo = two_skill_fixture();
    let cache = tempfile::tempdir().expect("tempdir");
    let url = url_of(repo.path());

    let (sparse_dir, _) = fetch_through_cache(
        cache.path(),
        &FetchRequest {
            subpath: Some("skills/a"),
            ..request(&url, FRESH_TTL_MS)
        },
    )
    .expect("sparse fetch");
    assert!(sparse_dir.join("skills/a/SKILL.md").exists());
    assert!(
        !sparse_dir.join("skills/b/SKILL.md").exists(),
        "precondition: the sparse entry holds only skills/a"
    );

    let (listing_dir, _) =
        fetch_through_cache(cache.path(), &request(&url, FRESH_TTL_MS)).expect("listing fetch");

    assert_eq!(listing_dir, sparse_dir, "still one entry per repository");
    assert!(
        listing_dir.join("skills/b/SKILL.md").exists() && listing_dir.join("SKILL.md").exists(),
        "a listing must see the whole tree, not the sparse subset"
    );
}

/// Widening is not a refetch: the head stays the cached one and the entry's
/// fetch time survives, so a widened entry expires exactly when the original
/// one would have. The entry is aged to halfway through the TTL first so a
/// renewed record is distinguishable from a preserved one.
#[test]
fn widening_does_not_renew_freshness() {
    let repo = two_skill_fixture();
    let cache = tempfile::tempdir().expect("tempdir");
    let url = url_of(repo.path());

    let (repo_dir, head_a) = fetch_through_cache(
        cache.path(),
        &FetchRequest {
            subpath: Some("skills/a"),
            ..request(&url, FRESH_TTL_MS)
        },
    )
    .expect("sparse fetch");
    let halfway = crate::core::clock::now_ms() - FRESH_TTL_MS / 2;
    set_entry_fetch_time(&repo_dir, halfway);
    commit_more(repo.path(), "after-sparse.txt");

    let (_dir, head_b) =
        fetch_through_cache(cache.path(), &request(&url, FRESH_TTL_MS)).expect("listing fetch");

    assert_eq!(
        head_a, head_b,
        "widening a fresh entry does not move its head"
    );
    assert_eq!(
        entry_fetch_time(&repo_dir),
        halfway,
        "widening must not renew the entry's fetch time"
    );
}

/// The acceptance case end to end: listing then install is one fetch, and an
/// install after the TTL has expired fetches again.
#[test]
fn a_second_install_after_ttl_expiry_fetches_again() {
    let repo = two_skill_fixture();
    let cache = tempfile::tempdir().expect("tempdir");
    let url = url_of(repo.path());
    let install = |subpath: &'static str| FetchRequest {
        subpath: Some(subpath),
        ..request(&url, FRESH_TTL_MS)
    };

    let (repo_dir, listing_head) =
        fetch_through_cache(cache.path(), &request(&url, FRESH_TTL_MS)).expect("listing fetch");
    let (_dir, install_head) =
        fetch_through_cache(cache.path(), &install("skills/a")).expect("first install");
    assert_eq!(
        install_head, listing_head,
        "the first install is a cache hit"
    );

    age_entry_past_ttl(&repo_dir);
    commit_more(repo.path(), "after-install.txt");

    let (later_dir, later_head) =
        fetch_through_cache(cache.path(), &install("skills/a")).expect("second install");
    assert_eq!(later_dir, repo_dir, "the entry is refreshed in place");
    assert_ne!(
        later_head, listing_head,
        "an expired entry fetches the new commit"
    );
    assert!(later_dir.join("after-install.txt").exists());
}

/// Age an entry's metadata past [`FRESH_TTL_MS`] rather than sleeping.
fn age_entry_past_ttl(repo_dir: &Path) {
    set_entry_fetch_time(repo_dir, crate::core::clock::now_ms() - FRESH_TTL_MS - 1);
}

/// The test's clock lever: rewrite the entry's recorded fetch time.
fn set_entry_fetch_time(repo_dir: &Path, last_fetched_ms: i64) {
    let meta_path = repo_dir.join(".skills-hub-cache.json");
    let raw = fs::read_to_string(&meta_path).expect("meta written");
    let mut meta: serde_json::Value = serde_json::from_str(&raw).expect("meta json");
    meta["last_fetched_ms"] = serde_json::json!(last_fetched_ms);
    fs::write(&meta_path, meta.to_string()).expect("write meta");
}

fn entry_fetch_time(repo_dir: &Path) -> i64 {
    let raw = fs::read_to_string(repo_dir.join(".skills-hub-cache.json")).expect("meta written");
    let meta: serde_json::Value = serde_json::from_str(&raw).expect("meta json");
    meta["last_fetched_ms"].as_i64().expect("last_fetched_ms")
}
