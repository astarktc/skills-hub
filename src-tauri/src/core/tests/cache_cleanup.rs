//! Cache-cleanup tests: how an entry nobody fetches any more — a pre-policy
//! entry the checkout-policy key change orphaned — leaves the git cache.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::cleanup_git_cache_dirs_in;
use crate::core::clock::now_ms;
use crate::core::git_cache::{repo_cache_key, repo_cache_key_under_policy, CacheKeyInputs};

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

    let removed = cleanup_git_cache_dirs_in(cache.path(), 30 * DAY).expect("cleanup");

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

    let removed = cleanup_git_cache_dirs_in(cache.path(), 30 * DAY).expect("cleanup");

    assert_eq!(removed, 1);
    assert!(!old_entry.exists(), "the orphaned entry is reclaimed");
    assert!(current.exists(), "the current policy's entry stays");
}
