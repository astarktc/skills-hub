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
/// `printf 'URL\nBRANCH\nSUBPATH' | shasum -a 256`.
#[test]
fn cache_key_is_pinned_to_the_shipped_scheme() {
    assert_eq!(
        repo_cache_key(&CacheKeyInputs {
            clone_url: "https://example.com/owner/repo.git",
            branch: Some("main"),
            subpath: Some("skills/a"),
        }),
        "87e6f47a9a465d9c00d01d7330bdaacbfb9f2ebee184825ba61ebf04603d719d"
    );
    assert_eq!(
        repo_cache_key(&CacheKeyInputs {
            clone_url: "https://example.com/owner/repo.git",
            branch: None,
            subpath: None,
        }),
        "b829837329c3112a34f654c04151145daf5bed54b33cd60e41c3aa365f4b5d87"
    );
}

/// Every input participates: no two of the three fields can be swapped
/// without changing the key.
#[test]
fn cache_key_separates_branch_from_subpath() {
    let branch_only = repo_cache_key(&CacheKeyInputs {
        clone_url: "u",
        branch: Some("x"),
        subpath: None,
    });
    let subpath_only = repo_cache_key(&CacheKeyInputs {
        clone_url: "u",
        branch: None,
        subpath: Some("x"),
    });
    assert_ne!(branch_only, subpath_only);
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
        subpath: None,
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
        subpath: None,
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
// Sparse fetches
// ---------------------------------------------------------------------------

/// `subpath` selects the sparse fetcher and gets its own cache dir.
#[test]
fn a_subpath_request_uses_its_own_cache_entry() {
    let repo = fixture_repo();
    fs::create_dir_all(repo.path().join("skills/a")).expect("mkdir");
    fs::write(repo.path().join("skills/a/SKILL.md"), "---\nname: a\n---\n").expect("write");
    git(&["add", "-A"], repo.path());
    git(&["commit", "-q", "-m", "add subskill"], repo.path());

    let cache = tempfile::tempdir().expect("tempdir");
    let url = url_of(repo.path());

    let (full_dir, _) =
        fetch_through_cache(cache.path(), &request(&url, FRESH_TTL_MS)).expect("full fetch");
    let (sparse_dir, _) = fetch_through_cache(
        cache.path(),
        &FetchRequest {
            subpath: Some("skills/a"),
            ..request(&url, FRESH_TTL_MS)
        },
    )
    .expect("sparse fetch");

    assert_ne!(full_dir, sparse_dir);
    assert!(sparse_dir.join("skills/a/SKILL.md").exists());
}
