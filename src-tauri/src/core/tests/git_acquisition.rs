//! Acquisition tests. Every clone here targets a **local fixture
//! repository** by path and every GitHub API call goes through a scripted
//! stub — no test in this file touches the network.

use std::cell::RefCell;
use std::fs;
use std::path::Path;

use super::{
    acquire, AcquireRequest, AcquireStrategy, GitSource, GithubApi, GithubCoords, GithubRepo,
    SkillIntent,
};
use crate::core::cancel_token::CancelToken;
use crate::core::errors::SignalError;
use crate::core::github_download::GithubApiError;

/// A long TTL: a clone written during a test stays fresh.
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

/// A local repository whose files are `(relative path, contents)`.
fn fixture_repo(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    git(&["init", "-q", "-b", "main", "."], dir.path());
    for (path, contents) in files {
        let full = dir.path().join(path);
        fs::create_dir_all(full.parent().expect("parent")).expect("mkdir");
        fs::write(full, contents).expect("write");
    }
    git(&["add", "-A"], dir.path());
    git(&["commit", "-q", "-m", "init"], dir.path());
    dir
}

/// A local repository with regular files plus committed symlink entries
/// (`120000`), each `(link path, raw target)`. The links go straight into
/// the index as blobs, so no symlink is created on the host filesystem and
/// the fixture builds on every platform.
fn fixture_repo_with_links(files: &[(&str, &str)], links: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = fixture_repo(files);
    for (path, target) in links {
        let out = std::process::Command::new("git")
            .args(["hash-object", "-w", "--stdin"])
            .current_dir(dir.path())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .take()
                    .expect("stdin")
                    .write_all(target.as_bytes())?;
                child.wait_with_output()
            })
            .expect("hash link blob");
        assert!(out.status.success(), "git hash-object failed");
        let blob = String::from_utf8_lossy(&out.stdout).trim().to_string();
        git(
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("120000,{blob},{path}"),
            ],
            dir.path(),
        );
    }
    git(&["commit", "-q", "-m", "links"], dir.path());
    dir
}

/// The alias the operator's records hold for the upstream case this models
/// (`tanstack-skills/tanstack-skills`), and where the bytes really live.
const BUNDLE_ALIAS: &str = "plugins/tanstack-all/skills/tanstack-table";
const BUNDLE_TARGET: &str = "plugins/tanstack-table/skills/tanstack-table";
const BUNDLE_SKILL_MD: &str = "---\nname: tanstack-table\n---\nthe real skill\n";

/// An aggregation bundle: each skill lives at `plugins/<name>/skills/<name>`
/// and is published again from `plugins/tanstack-all/skills/<name>` as a
/// link to `../../<name>/skills/<name>` — relative to the link's own
/// directory. The bundle holds a sibling link too (as the real one holds
/// eighteen), so a lookup that confuses a sibling for the requested link
/// is caught.
fn bundle_repo() -> tempfile::TempDir {
    fixture_repo_with_links(
        &[
            ("README.md", "root"),
            (
                "plugins/tanstack-ai/skills/tanstack-ai/SKILL.md",
                "---\nname: tanstack-ai\n---\nthe wrong skill\n",
            ),
            (
                "plugins/tanstack-table/skills/tanstack-table/SKILL.md",
                BUNDLE_SKILL_MD,
            ),
        ],
        &[
            (
                "plugins/tanstack-all/skills/tanstack-ai",
                "../../tanstack-ai/skills/tanstack-ai",
            ),
            (BUNDLE_ALIAS, "../../tanstack-table/skills/tanstack-table"),
        ],
    )
}

/// The sparse coverage the (single) cache entry records.
fn cache_entry_subpaths(cache_dir: &Path) -> Vec<String> {
    let entries: Vec<_> = fs::read_dir(cache_dir.join("skills-hub-git-cache"))
        .expect("cache root")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join(".skills-hub-cache.json").exists())
        .collect();
    assert_eq!(entries.len(), 1, "one cache entry: {entries:?}");
    let raw = fs::read_to_string(entries[0].join(".skills-hub-cache.json")).expect("sidecar");
    let meta: serde_json::Value = serde_json::from_str(&raw).expect("meta json");
    meta["checkout"]["subpaths"]
        .as_array()
        .expect("a sparse entry")
        .iter()
        .map(|v| v.as_str().expect("subpath").to_string())
        .collect()
}

/// One skill at `skills/a`, plus an unrelated file at the root.
fn single_skill_repo() -> tempfile::TempDir {
    fixture_repo(&[
        ("README.md", "root"),
        ("skills/a/SKILL.md", "---\nname: alpha\n---\n"),
    ])
}

/// Two installable skills — the multi-skill repo shape name matching exists
/// for.
fn two_skill_repo() -> tempfile::TempDir {
    fixture_repo(&[
        ("skills/alpha/SKILL.md", "---\nname: alpha\n---\n"),
        ("skills/beta/SKILL.md", "---\nname: beta\n---\n"),
    ])
}

fn head_of(repo: &Path) -> String {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .expect("run git");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A source cloned from a local path that nonetheless carries GitHub
/// coordinates, so the fast path is attempted and its fallback clones the
/// fixture instead of the network.
fn local_source_with_api(repo: &Path) -> GitSource {
    GitSource {
        clone_url: repo.to_string_lossy().to_string(),
        branch: None,
        subpath: None,
        api: Some(GithubRepo {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
        }),
    }
}

fn local_source(repo: &Path) -> GitSource {
    GitSource {
        clone_url: repo.to_string_lossy().to_string(),
        branch: None,
        subpath: None,
        api: None,
    }
}

// ---------------------------------------------------------------------------
// The stubbed GitHub API
// ---------------------------------------------------------------------------

/// A scripted GitHub API adapter: it records every call and serves whatever
/// the script says, so acquisition policy is testable without HTTP.
#[derive(Default)]
struct StubApi<'a> {
    sha: String,
    /// Failure raised instead of serving the branch SHA.
    sha_error: Option<GithubApiError>,
    /// Failure raised instead of serving the directory download.
    download_error: Option<GithubApiError>,
    /// Typed condition raised by the download (an upstream link refused).
    download_signal: Option<SignalError>,
    /// Files the download writes into `dest`.
    files: Vec<(&'a str, &'a str)>,
    /// Cancel this token when the download is entered (mid-acquisition cancel).
    cancel_on_download: Option<&'a CancelToken>,
    calls: RefCell<Vec<String>>,
}

impl StubApi<'_> {
    fn serving(sha: &str) -> Self {
        StubApi {
            sha: sha.to_string(),
            files: vec![("SKILL.md", "---\nname: alpha\n---\n")],
            ..Default::default()
        }
    }

    fn failing(status: u16, reset_minutes: Option<i64>) -> Self {
        StubApi {
            sha: "0".repeat(40),
            download_error: Some(GithubApiError {
                status,
                reset_minutes,
                url: "stub".to_string(),
            }),
            ..Default::default()
        }
    }

    /// The branch itself is the failure: the SHA lookup answers `status` and
    /// the download is never reached.
    fn failing_branch(status: u16) -> Self {
        StubApi {
            sha: "0".repeat(40),
            sha_error: Some(GithubApiError {
                status,
                reset_minutes: None,
                url: "stub".to_string(),
            }),
            ..Default::default()
        }
    }

    fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}

impl GithubApi for StubApi<'_> {
    fn branch_sha(&self, coords: &GithubCoords) -> anyhow::Result<String> {
        self.calls.borrow_mut().push(format!(
            "sha:{}/{}@{}",
            coords.owner, coords.repo, coords.branch
        ));
        if let Some(err) = &self.sha_error {
            return Err(anyhow::Error::new(err.clone()));
        }
        Ok(self.sha.clone())
    }

    fn download_directory(
        &self,
        coords: &GithubCoords,
        dest: &Path,
        cancel: Option<&CancelToken>,
    ) -> anyhow::Result<()> {
        self.calls
            .borrow_mut()
            .push(format!("download:{}", coords.subpath));
        if let Some(token) = self.cancel_on_download {
            token.cancel();
        }
        if cancel.is_some_and(|c| c.is_cancelled()) {
            anyhow::bail!(SignalError::Cancelled);
        }
        if let Some(err) = &self.download_error {
            // Partial bytes on disk are what a real failed download leaves.
            fs::create_dir_all(dest).expect("create dest");
            fs::write(dest.join("partial.txt"), "half a download").expect("write");
            return Err(anyhow::Error::new(err.clone()));
        }
        if let Some(signal) = &self.download_signal {
            return Err(anyhow::Error::new(signal.clone()).context("download skill"));
        }
        fs::create_dir_all(dest).expect("create dest");
        for (path, contents) in &self.files {
            fs::write(dest.join(path), contents).expect("write");
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Request helper
// ---------------------------------------------------------------------------

struct Fixture {
    _cache: tempfile::TempDir,
    _dest: tempfile::TempDir,
}

impl Fixture {
    fn new() -> (Self, std::path::PathBuf, std::path::PathBuf) {
        let cache = tempfile::tempdir().expect("tempdir");
        let dest_root = tempfile::tempdir().expect("tempdir");
        let cache_dir = cache.path().to_path_buf();
        let dest = dest_root.path().join("dest");
        (
            Fixture {
                _cache: cache,
                _dest: dest_root,
            },
            cache_dir,
            dest,
        )
    }
}

fn request<'a>(
    source: &'a GitSource,
    intent: SkillIntent<'a>,
    dest: &'a Path,
    cache_dir: &'a Path,
) -> AcquireRequest<'a> {
    AcquireRequest {
        source,
        intent,
        dest,
        cache_dir,
        ttl_ms: FRESH_TTL_MS,
        cancel: None,
        allow_fast_path: true,
    }
}

fn git_cache_root_exists(cache_dir: &Path) -> bool {
    cache_dir.join("skills-hub-git-cache").exists()
}

// ---------------------------------------------------------------------------
// The fast path
// ---------------------------------------------------------------------------

/// A subpath intent on a GitHub source is served by the API, records the real
/// commit SHA, and never touches the git cache.
#[test]
fn fast_path_serves_a_subpath_and_records_the_commit_sha() {
    let repo = single_skill_repo();
    let source = local_source_with_api(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::serving("abc123def4567890123456789012345678901234");

    let acquired = acquire(
        &request(&source, SkillIntent::Subpath("skills/a"), &dest, &cache_dir),
        &api,
    )
    .expect("fast path acquires");

    assert_eq!(acquired.strategy, AcquireStrategy::GithubApi);
    assert_eq!(
        acquired.revision,
        "abc123def4567890123456789012345678901234"
    );
    assert_eq!(acquired.resolved_subpath.as_deref(), Some("skills/a"));
    assert!(dest.join("SKILL.md").exists());
    assert!(
        !git_cache_root_exists(&cache_dir),
        "a served fast path must not clone"
    );
    assert_eq!(
        api.calls(),
        vec![
            "sha:owner/repo@main".to_string(),
            "download:skills/a".to_string()
        ],
        "the SHA is fetched before any bytes land"
    );
}

/// The fast path is only for a real subpath: the repo root is a clone.
#[test]
fn fast_path_is_skipped_for_the_repo_root() {
    let repo = single_skill_repo();
    let source = local_source_with_api(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::serving("deadbeef");

    let acquired = acquire(
        &request(&source, SkillIntent::Subpath("."), &dest, &cache_dir),
        &api,
    )
    .expect("clone acquires the root");

    assert_eq!(
        acquired.strategy,
        AcquireStrategy::GitClone { sparse: false },
        "the whole repo is a full clone"
    );
    assert_eq!(acquired.revision, head_of(repo.path()));
    assert_eq!(acquired.resolved_subpath, None);
    assert!(dest.join("README.md").exists());
    assert!(api.calls().is_empty(), "no API call for a root acquisition");
}

/// `allow_fast_path: false` keeps the API out of it entirely.
#[test]
fn fast_path_can_be_disallowed_by_the_caller() {
    let repo = single_skill_repo();
    let source = local_source_with_api(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::serving("deadbeef");

    let acquired = acquire(
        &AcquireRequest {
            allow_fast_path: false,
            ..request(&source, SkillIntent::Subpath("skills/a"), &dest, &cache_dir)
        },
        &api,
    )
    .expect("clone acquires");

    assert_eq!(
        acquired.strategy,
        AcquireStrategy::GitClone { sparse: true }
    );
    assert!(api.calls().is_empty());
    assert!(dest.join("SKILL.md").exists());
}

/// A non-GitHub source has no fast path at all.
#[test]
fn a_source_without_github_coordinates_clones() {
    let repo = single_skill_repo();
    let source = local_source(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::serving("deadbeef");

    let acquired = acquire(
        &request(&source, SkillIntent::Subpath("skills/a"), &dest, &cache_dir),
        &api,
    )
    .expect("clone acquires");

    assert_eq!(
        acquired.strategy,
        AcquireStrategy::GitClone { sparse: true }
    );
    assert!(api.calls().is_empty());
}

// ---------------------------------------------------------------------------
// Fallback and typed outcomes
// ---------------------------------------------------------------------------

/// An unclassified API failure (5xx) is a fallback, not an error: the clone
/// serves the same bytes and the partial download is cleaned up first.
#[test]
fn an_api_failure_falls_back_to_a_clone() {
    let repo = single_skill_repo();
    let source = local_source_with_api(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::failing(502, None);

    let acquired = acquire(
        &request(&source, SkillIntent::Subpath("skills/a"), &dest, &cache_dir),
        &api,
    )
    .expect("the clone fallback acquires");

    assert_eq!(
        acquired.strategy,
        AcquireStrategy::GitClone { sparse: true }
    );
    assert_eq!(acquired.revision, head_of(repo.path()));
    assert!(dest.join("SKILL.md").exists());
    assert!(
        !dest.join("partial.txt").exists(),
        "the failed download's bytes must not survive into the fallback"
    );
}

/// A branch the operator never named was only *assumed* to be `main`: when it
/// does not exist the fast path is not applicable, so the clone — which uses
/// the repository's real default branch — serves the skill.
#[test]
fn an_assumed_branch_that_does_not_exist_falls_back_to_a_clone() {
    let repo = single_skill_repo();
    let source = local_source_with_api(repo.path());
    assert!(source.branch.is_none(), "the branch is assumed, not named");
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::failing_branch(404);

    let acquired = acquire(
        &request(&source, SkillIntent::Subpath("skills/a"), &dest, &cache_dir),
        &api,
    )
    .expect("the clone fallback acquires");

    assert_eq!(
        acquired.strategy,
        AcquireStrategy::GitClone { sparse: true }
    );
    assert_eq!(acquired.revision, head_of(repo.path()));
    assert!(dest.join("SKILL.md").exists());
    assert_eq!(
        api.calls(),
        vec!["sha:owner/repo@main".to_string()],
        "the assumed branch was tried and no bytes were requested"
    );
}

/// A branch the URL named is the operator's own coordinate: when it does not
/// exist, that is their answer, not a reason to clone.
#[test]
fn a_named_branch_that_does_not_exist_is_typed_not_found() {
    let repo = single_skill_repo();
    let source = GitSource {
        branch: Some("nope".to_string()),
        ..local_source_with_api(repo.path())
    };
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::failing_branch(404);

    let err = acquire(
        &request(&source, SkillIntent::Subpath("skills/a"), &dest, &cache_dir),
        &api,
    )
    .expect_err("a missing named branch fails");

    assert!(
        matches!(
            err.downcast_ref::<SignalError>(),
            Some(SignalError::GithubSkillNotFound { url }) if url.contains("tree/nope/")
        ),
        "expected GithubSkillNotFound, got: {err:#}"
    );
    assert!(
        !git_cache_root_exists(&cache_dir),
        "a named branch that does not exist must not fall back to a clone"
    );
}

/// 404 on the download is the operator's answer, not a reason to clone: the
/// branch exists, the subpath does not.
#[test]
fn a_missing_subpath_on_an_existing_branch_is_typed_not_found() {
    let repo = single_skill_repo();
    let source = local_source_with_api(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::failing(404, None);

    let err = acquire(
        &request(
            &source,
            SkillIntent::Subpath("skills/missing"),
            &dest,
            &cache_dir,
        ),
        &api,
    )
    .expect_err("a missing skill fails");

    assert!(
        matches!(
            err.downcast_ref::<SignalError>(),
            Some(SignalError::GithubSkillNotFound { url }) if url.contains("skills/missing")
        ),
        "expected GithubSkillNotFound, got: {err:#}"
    );
    assert!(
        !git_cache_root_exists(&cache_dir),
        "a not-found must not fall back to a clone"
    );
}

/// 403 reaches the caller as the rate-limit condition with its ETA.
#[test]
fn a_rate_limit_is_typed_and_never_falls_back() {
    let repo = single_skill_repo();
    let source = local_source_with_api(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::failing(403, Some(7));

    let err = acquire(
        &request(&source, SkillIntent::Subpath("skills/a"), &dest, &cache_dir),
        &api,
    )
    .expect_err("a rate-limited fetch fails");

    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::RateLimited { reset_minutes: 7 })
    );
    assert!(!git_cache_root_exists(&cache_dir));
}

/// A 403 without a reset header still carries the condition; `0` is "no ETA".
#[test]
fn a_rate_limit_without_an_eta_reports_zero() {
    let repo = single_skill_repo();
    let source = local_source_with_api(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::failing(403, None);

    let err = acquire(
        &request(&source, SkillIntent::Subpath("skills/a"), &dest, &cache_dir),
        &api,
    )
    .expect_err("a rate-limited fetch fails");

    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::RateLimited { reset_minutes: 0 })
    );
}

/// A typed refusal raised by the fast path — an upstream link whose target
/// leaves the repository — is the operator's answer: it reaches the caller
/// as-is and is never retried as a clone (which would only refuse again).
#[test]
fn a_typed_refusal_on_the_fast_path_is_not_retried_as_a_clone() {
    let repo = single_skill_repo();
    let source = local_source_with_api(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let refused = SignalError::SymlinkEscapesRepo {
        subpath: "skills/a".to_string(),
        target: "../../outside".to_string(),
    };
    let api = StubApi {
        download_signal: Some(refused.clone()),
        ..StubApi::serving("deadbeef")
    };

    let err = acquire(
        &request(&source, SkillIntent::Subpath("skills/a"), &dest, &cache_dir),
        &api,
    )
    .expect_err("the refusal fails the acquisition");

    assert_eq!(err.downcast_ref::<SignalError>(), Some(&refused));
    assert!(
        !git_cache_root_exists(&cache_dir),
        "a typed refusal must not fall back to a clone"
    );
}

// ---------------------------------------------------------------------------
// Cancellation
// ---------------------------------------------------------------------------

/// A pre-cancelled acquisition stops before anything is fetched or written.
#[test]
fn a_pre_cancelled_acquisition_aborts_cleanly() {
    let repo = single_skill_repo();
    let source = local_source_with_api(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::serving("deadbeef");
    let token = CancelToken::new();
    token.cancel();

    let err = acquire(
        &AcquireRequest {
            cancel: Some(&token),
            ..request(&source, SkillIntent::Subpath("skills/a"), &dest, &cache_dir)
        },
        &api,
    )
    .expect_err("a cancelled acquisition fails");

    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::Cancelled)
    );
    assert!(api.calls().is_empty());
    assert!(!dest.exists(), "nothing is written for a cancelled request");
    assert!(!git_cache_root_exists(&cache_dir));
}

/// Cancelling mid-download aborts instead of falling back to a clone —
/// cancellation is a decision, not a failed strategy.
#[test]
fn a_cancel_during_the_fast_path_aborts_instead_of_cloning() {
    let repo = single_skill_repo();
    let source = local_source_with_api(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let token = CancelToken::new();
    let api = StubApi {
        cancel_on_download: Some(&token),
        ..StubApi::serving("deadbeef")
    };

    let err = acquire(
        &AcquireRequest {
            cancel: Some(&token),
            ..request(&source, SkillIntent::Subpath("skills/a"), &dest, &cache_dir)
        },
        &api,
    )
    .expect_err("a cancelled download fails");

    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::Cancelled)
    );
    assert!(
        !git_cache_root_exists(&cache_dir),
        "a cancelled fast path must not fall back to a clone"
    );
}

/// Cancellation reaches the clone path too.
#[test]
fn a_cancelled_clone_acquisition_aborts() {
    let repo = single_skill_repo();
    let source = local_source(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::serving("deadbeef");
    let token = CancelToken::new();
    token.cancel();

    let err = acquire(
        &AcquireRequest {
            cancel: Some(&token),
            ..request(&source, SkillIntent::Subpath("."), &dest, &cache_dir)
        },
        &api,
    )
    .expect_err("a cancelled acquisition fails");

    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::Cancelled)
    );
    assert!(!dest.exists());
}

// ---------------------------------------------------------------------------
// Intents: subpath, named skill, backfill
// ---------------------------------------------------------------------------

/// A subpath intent fetches sparsely and reports the subpath it took.
#[test]
fn a_subpath_intent_fetches_sparsely() {
    let repo = single_skill_repo();
    let source = local_source(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::serving("unused");

    let acquired = acquire(
        &request(&source, SkillIntent::Subpath("skills/a"), &dest, &cache_dir),
        &api,
    )
    .expect("sparse clone acquires");

    assert_eq!(
        acquired.strategy,
        AcquireStrategy::GitClone { sparse: true }
    );
    assert_eq!(acquired.resolved_subpath.as_deref(), Some("skills/a"));
    assert!(dest.join("SKILL.md").exists());
    assert!(
        !dest.join("README.md").exists(),
        "only the subpath's bytes land in dest"
    );
}

/// A subpath that does not exist in the repo is an error, not an empty skill.
#[test]
fn a_missing_subpath_fails() {
    let repo = single_skill_repo();
    let source = local_source(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::serving("unused");

    let err = acquire(
        &request(
            &source,
            SkillIntent::Subpath("skills/nope"),
            &dest,
            &cache_dir,
        ),
        &api,
    )
    .expect_err("a missing subpath fails");
    // The typed condition carries the subpath the caller asked for, never
    // the cache-internal absolute path of the checkout.
    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::SubpathMissing {
            subpath: "skills/nope".to_string(),
        }),
        "unexpected error: {err:#}"
    );
    assert!(
        !format!("{err:#}").contains(cache_dir.to_str().unwrap()),
        "the cache path must not leak: {err:#}"
    );
}

/// A subpath is a name inside the checkout, never a traversal: one carrying
/// a `..` segment is refused at the acquisition boundary before either
/// adapter is asked for anything — no API call, no clone, nothing read.
#[test]
fn a_subpath_with_a_parent_segment_is_refused_before_anything_is_read() {
    let repo = single_skill_repo();
    let source = local_source_with_api(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::serving("unused");

    for subpath in [
        "../outside",
        "skills/../../outside",
        r"skills\..\..\outside",
    ] {
        let err = acquire(
            &request(&source, SkillIntent::Subpath(subpath), &dest, &cache_dir),
            &api,
        )
        .expect_err("a traversing subpath is refused");
        assert!(
            format!("{err:#}").contains(subpath),
            "names the subpath: {err:#}"
        );
    }
    assert!(api.calls().is_empty(), "the API was never asked");
    assert!(!git_cache_root_exists(&cache_dir), "nothing was cloned");
    assert!(!dest.exists(), "nothing landed");
}

/// The name-matching rule lives here once: a named skill in a multi-skill
/// repo resolves to its subpath.
#[test]
fn a_named_skill_resolves_its_subpath_in_a_multi_skill_repo() {
    let repo = two_skill_repo();
    let source = local_source(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::serving("unused");

    let acquired = acquire(
        &request(
            &source,
            SkillIntent::NamedSkill(Some("beta")),
            &dest,
            &cache_dir,
        ),
        &api,
    )
    .expect("the named skill acquires");

    assert_eq!(acquired.resolved_subpath.as_deref(), Some("skills/beta"));
    assert_eq!(
        fs::read_to_string(dest.join("SKILL.md")).expect("read"),
        "---\nname: beta\n---\n"
    );
}

/// Without a name (or with one that resolves to nothing) a multi-skill repo
/// is the `MultiSkills` condition: the caller must name the skill.
#[test]
fn a_multi_skill_repo_without_a_usable_name_is_typed() {
    let repo = two_skill_repo();
    let source = local_source(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::serving("unused");

    for intent in [
        SkillIntent::NamedSkill(None),
        SkillIntent::NamedSkill(Some("gamma")),
    ] {
        let err = acquire(&request(&source, intent, &dest, &cache_dir), &api)
            .expect_err("an unnamed multi-skill repo fails");
        assert!(
            matches!(
                err.downcast_ref::<SignalError>(),
                Some(SignalError::MultiSkills)
            ),
            "expected MultiSkills, got: {err:#}"
        );
    }
}

/// A single-skill repo needs no name: the repo root is the skill.
#[test]
fn a_named_intent_on_a_single_skill_repo_takes_the_root() {
    let repo = fixture_repo(&[("SKILL.md", "---\nname: solo\n---\n")]);
    let source = local_source(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::serving("unused");

    let acquired = acquire(
        &request(&source, SkillIntent::NamedSkill(None), &dest, &cache_dir),
        &api,
    )
    .expect("the root acquires");

    assert_eq!(acquired.resolved_subpath, None);
    assert!(dest.join("SKILL.md").exists());
}

/// The update backfill is the lenient sibling: a name that resolves backfills
/// the subpath …
#[test]
fn the_backfill_intent_resolves_a_name_to_a_subpath() {
    let repo = two_skill_repo();
    let source = local_source(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::serving("unused");

    let acquired = acquire(
        &request(
            &source,
            SkillIntent::NamedSkillOrWholeRepo("alpha"),
            &dest,
            &cache_dir,
        ),
        &api,
    )
    .expect("the backfill acquires");

    assert_eq!(acquired.resolved_subpath.as_deref(), Some("skills/alpha"));
}

/// … and a name that does not resolve takes the whole repo rather than
/// failing an update that used to work.
#[test]
fn the_backfill_intent_takes_the_whole_repo_when_no_name_matches() {
    let repo = two_skill_repo();
    let source = local_source(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::serving("unused");

    let acquired = acquire(
        &request(
            &source,
            SkillIntent::NamedSkillOrWholeRepo("gamma"),
            &dest,
            &cache_dir,
        ),
        &api,
    )
    .expect("the backfill acquires the whole repo");

    assert_eq!(acquired.resolved_subpath, None);
    assert!(dest.join("skills/alpha/SKILL.md").exists());
}

/// A URL that names a folder supplies the subpath for a named intent, and it
/// is the fast path's coordinate.
#[test]
fn a_url_subpath_supplies_a_named_intent() {
    let repo = single_skill_repo();
    let source = GitSource {
        subpath: Some("skills/a".to_string()),
        ..local_source_with_api(repo.path())
    };
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::serving("cafebabe");

    let acquired = acquire(
        &request(&source, SkillIntent::NamedSkill(None), &dest, &cache_dir),
        &api,
    )
    .expect("the URL subpath acquires");

    assert_eq!(acquired.strategy, AcquireStrategy::GithubApi);
    assert_eq!(acquired.resolved_subpath.as_deref(), Some("skills/a"));
}

// ---------------------------------------------------------------------------
// Upstream in-repo symlinks (clone path)
// ---------------------------------------------------------------------------

/// The operator's case: a skill recorded under its bundle alias, which
/// upstream publishes as a symlink. The sparse clone follows the link — the
/// target joins the entry's coverage — and the real content lands, while
/// the reported subpath stays the alias. A second acquisition (the Refresh)
/// is served the same way from the now-covering entry.
#[test]
fn a_subpath_that_is_an_upstream_symlink_acquires_the_targets_content() {
    let repo = bundle_repo();
    let source = local_source(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::serving("unused");

    let acquired = acquire(
        &request(
            &source,
            SkillIntent::Subpath(BUNDLE_ALIAS),
            &dest,
            &cache_dir,
        ),
        &api,
    )
    .expect("the link is followed");

    assert_eq!(
        fs::read_to_string(dest.join("SKILL.md")).expect("the target's SKILL.md landed"),
        BUNDLE_SKILL_MD
    );
    assert_eq!(
        acquired.resolved_subpath.as_deref(),
        Some(BUNDLE_ALIAS),
        "the record keeps the alias the operator chose"
    );
    assert_eq!(
        acquired.strategy,
        AcquireStrategy::GitClone { sparse: true }
    );
    assert_eq!(acquired.revision, head_of(repo.path()));
    let coverage = cache_entry_subpaths(&cache_dir);
    assert!(
        coverage.contains(&BUNDLE_TARGET.to_string()),
        "the entry's coverage includes the link's target: {coverage:?}"
    );

    // The Refresh: the same alias again, served from the widened entry.
    let refresh_dest = dest.with_file_name("refresh");
    let refreshed = acquire(
        &request(
            &source,
            SkillIntent::Subpath(BUNDLE_ALIAS),
            &refresh_dest,
            &cache_dir,
        ),
        &api,
    )
    .expect("the refresh follows the link too");
    assert_eq!(refreshed.revision, acquired.revision);
    assert_eq!(
        fs::read_to_string(refresh_dest.join("SKILL.md")).expect("refreshed SKILL.md"),
        BUNDLE_SKILL_MD
    );
}

/// A link on the way to the subpath — a bundle directory that is itself a
/// symlink — is followed the same way, even though a sparse checkout of the
/// requested path materialises nothing under it.
#[test]
fn a_symlinked_component_of_the_subpath_is_followed() {
    let repo = fixture_repo_with_links(
        &[("bundles/all/skills/x/SKILL.md", "---\nname: x\n---\n")],
        &[("plugins/tanstack-all", "../bundles/all")],
    );
    let source = local_source(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::serving("unused");

    let acquired = acquire(
        &request(
            &source,
            SkillIntent::Subpath("plugins/tanstack-all/skills/x"),
            &dest,
            &cache_dir,
        ),
        &api,
    )
    .expect("the component link is followed");

    assert_eq!(
        fs::read_to_string(dest.join("SKILL.md")).expect("SKILL.md landed"),
        "---\nname: x\n---\n"
    );
    assert_eq!(
        acquired.resolved_subpath.as_deref(),
        Some("plugins/tanstack-all/skills/x")
    );
    assert!(cache_entry_subpaths(&cache_dir).contains(&"bundles/all/skills/x".to_string()));
}

/// A chain of two links resolves …
#[test]
fn a_chain_of_two_symlinks_resolves() {
    let repo = fixture_repo_with_links(
        &[("plugins/real/skills/t/SKILL.md", "---\nname: t\n---\n")],
        &[
            ("plugins/all/skills/t", "../../mid/skills/t"),
            ("plugins/mid/skills/t", "../../real/skills/t"),
        ],
    );
    let source = local_source(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::serving("unused");

    let acquired = acquire(
        &request(
            &source,
            SkillIntent::Subpath("plugins/all/skills/t"),
            &dest,
            &cache_dir,
        ),
        &api,
    )
    .expect("two hops resolve");

    assert!(dest.join("SKILL.md").exists());
    assert_eq!(
        acquired.resolved_subpath.as_deref(),
        Some("plugins/all/skills/t")
    );
}

/// … and a chain deeper than the bound is refused rather than followed
/// forever; nothing lands.
#[test]
fn a_symlink_chain_deeper_than_the_bound_is_refused() {
    let depth = crate::core::repo_subpath::MAX_LINK_DEPTH + 1;
    let links: Vec<(String, String)> = (0..depth)
        .map(|hop| (format!("l{hop}"), format!("l{}", hop + 1)))
        .collect();
    let links: Vec<(&str, &str)> = links
        .iter()
        .map(|(l, t)| (l.as_str(), t.as_str()))
        .collect();
    let real = format!("l{depth}/SKILL.md");
    let repo = fixture_repo_with_links(&[(&real, "---\nname: deep\n---\n")], &links);
    let source = local_source(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::serving("unused");

    let err = acquire(
        &request(&source, SkillIntent::Subpath("l0"), &dest, &cache_dir),
        &api,
    )
    .expect_err("a chain past the bound is refused");

    assert!(
        format!("{err:#}").contains("exceeds"),
        "unexpected error: {err:#}"
    );
    assert!(!dest.join("SKILL.md").exists(), "nothing lands");
}

/// An absolute target is refused with the typed condition before anything
/// is read: the directory it points at holds a real SKILL.md, and none of
/// it lands; the entry's coverage never widened toward it.
#[test]
fn an_absolute_symlink_target_is_refused_typed_and_never_read() {
    let outside = tempfile::tempdir().expect("tempdir");
    fs::write(outside.path().join("SKILL.md"), "---\nname: outside\n---\n").expect("write");
    let outside_path = outside.path().to_string_lossy().to_string();
    let repo = fixture_repo_with_links(&[("README.md", "root")], &[("skills/x", &outside_path)]);
    let source = local_source(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::serving("unused");

    let err = acquire(
        &request(&source, SkillIntent::Subpath("skills/x"), &dest, &cache_dir),
        &api,
    )
    .expect_err("an absolute target is refused");

    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::SymlinkEscapesRepo {
            subpath: "skills/x".to_string(),
            target: outside_path,
        })
    );
    assert!(!dest.join("SKILL.md").exists(), "nothing outside was read");
    assert_eq!(
        cache_entry_subpaths(&cache_dir),
        vec!["skills/x".to_string()],
        "the sparse set never widened toward the target"
    );
}

/// A relative target that climbs out of the repository is refused the same
/// way: a SKILL.md planted exactly where `../../outside` would land next to
/// the cache entry never reaches dest.
#[test]
fn an_escaping_symlink_target_is_refused_typed_and_never_read() {
    let repo = fixture_repo_with_links(&[("README.md", "root")], &[("skills/x", "../../outside")]);
    let source = local_source(repo.path());
    let (_fx, cache_dir, dest) = Fixture::new();
    let api = StubApi::serving("unused");
    // `skills/x -> ../../outside` is `<repo root>/../outside`; in the cache
    // that is a sibling of the entry directory.
    let planted = cache_dir.join("skills-hub-git-cache").join("outside");
    fs::create_dir_all(&planted).expect("mkdir");
    fs::write(planted.join("SKILL.md"), "---\nname: outside\n---\n").expect("write");

    let err = acquire(
        &request(&source, SkillIntent::Subpath("skills/x"), &dest, &cache_dir),
        &api,
    )
    .expect_err("an escaping target is refused");

    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::SymlinkEscapesRepo {
            subpath: "skills/x".to_string(),
            target: "../../outside".to_string(),
        })
    );
    assert!(!dest.join("SKILL.md").exists(), "nothing outside was read");
    assert_eq!(
        cache_entry_subpaths(&cache_dir),
        vec!["skills/x".to_string()]
    );
}

// ---------------------------------------------------------------------------
// Source parsing
// ---------------------------------------------------------------------------

/// GitHub sources carry API coordinates; anything else does not.
#[test]
fn parsing_records_github_coordinates() {
    let parsed = super::parse_github_url("https://github.com/owner/repo/tree/main/skills/x");
    assert_eq!(parsed.clone_url, "https://github.com/owner/repo.git");
    assert_eq!(parsed.branch.as_deref(), Some("main"));
    assert_eq!(parsed.subpath.as_deref(), Some("skills/x"));
    let api = parsed.api.expect("github coordinates");
    assert_eq!(api.owner, "owner");
    assert_eq!(api.repo, "repo");

    let parsed = super::parse_github_url("https://gitlab.com/owner/repo.git");
    assert!(parsed.api.is_none(), "only github.com has a fast path");

    let parsed = super::parse_github_url("/local/path/to/repo");
    assert!(parsed.api.is_none());
}

/// The multi-skill view the named intents match against: deep hits count, the
/// repo root and dirs without skill bytes do not.
#[test]
fn installable_skills_in_repo_excludes_root_and_missing_skill_md() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    fs::write(base.join("SKILL.md"), "---\nname: Root\n---\n").unwrap();
    fs::create_dir_all(base.join("skills/empty")).unwrap();
    let skills = [
        ("plugins/a/skills/api-design", "API Design"),
        ("plugins/b/skills/tailwind", "Tailwind"),
    ];
    for (path, name) in &skills {
        fs::create_dir_all(base.join(path)).unwrap();
        fs::write(
            base.join(path).join("SKILL.md"),
            format!("---\nname: {}\n---\n", name),
        )
        .unwrap();
    }

    let candidates = super::installable_skills_in_repo(base);
    let names: Vec<&str> = candidates.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["API Design", "Tailwind"]);
    assert_eq!(candidates[0].subpath, "plugins/a/skills/api-design");
}
