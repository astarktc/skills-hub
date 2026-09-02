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
    /// Failure raised instead of serving the directory download.
    download_error: Option<GithubApiError>,
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

/// 404 is the operator's answer, not a reason to clone: the typed condition
/// reaches the caller and nothing is fetched.
#[test]
fn a_not_found_is_typed_and_never_falls_back() {
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
    assert!(
        format!("{err:#}").contains("not found in repo"),
        "unexpected error: {err:#}"
    );
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
