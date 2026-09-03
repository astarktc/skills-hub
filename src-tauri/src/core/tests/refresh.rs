//! Tests for `core::refresh` — the Refresh (all) batch: acquire every
//! selected skill, then finalize + propagate each under the mutation guard.
//!
//! Every case runs against a temp home / central dir / DB; installedness is
//! faked by creating a Tool's detect dir under the temp home.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::core::cancel_token::CancelToken;
use crate::core::errors::SignalError;
use crate::core::installer::{install_local_skill, InstallerPaths};
use crate::core::propagation::{PropagationOutcome, PropagationScope, PropagationStatus};
use crate::core::refresh::{
    merge_reassert, refresh_managed_skills, refresh_managed_skills_with, RefreshPhase,
    RefreshPolicy, RefreshSelection, SkillRefreshStatus,
};
use crate::core::skill_store::SkillStore;
use crate::core::tool_adapters::adapter_by_key;

struct Fixture {
    _dir: tempfile::TempDir,
    paths: InstallerPaths,
    store: SkillStore,
    source: tempfile::TempDir,
    skill_id: String,
}

/// One local-source Managed skill installed into a temp central repo.
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = InstallerPaths {
        home: dir.path().join("home"),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.home).expect("create home");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");

    let source = tempfile::tempdir().expect("source tempdir");
    fs::write(source.path().join("SKILL.md"), "---\nname: alpha\n---\n").expect("write SKILL.md");
    fs::write(source.path().join("a.txt"), "v1").expect("write a.txt");
    let installed = install_local_skill(&paths, &store, source.path(), Some("alpha".to_string()))
        .expect("install");

    Fixture {
        _dir: dir,
        paths,
        store,
        source,
        skill_id: installed.skill_id,
    }
}

fn refresh(f: &Fixture, policy: RefreshPolicy) -> crate::core::refresh::RefreshReport {
    refresh_managed_skills(
        &f.paths,
        &f.store,
        RefreshSelection::All,
        policy,
        None,
        3000,
        |_| {},
    )
    .expect("refresh reads its own skill list")
}

#[test]
fn a_refreshed_skill_gets_its_new_bytes_and_reports_its_targets() {
    let f = fixture();
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    fs::create_dir_all(f.paths.home.join(claude.relative_detect_dir)).expect("install tool");
    fs::write(f.source.path().join("a.txt"), "v2").expect("write a.txt");

    let mut phases: Vec<(RefreshPhase, String)> = Vec::new();
    let report = refresh_managed_skills(
        &f.paths,
        &f.store,
        RefreshSelection::Ids(vec![f.skill_id.clone()]),
        RefreshPolicy::default(),
        None,
        3000,
        |p| phases.push((p.phase, p.skill_name.to_string())),
    )
    .expect("refresh");

    assert!(matches!(
        report.skills.as_slice(),
        [outcome] if matches!(outcome.status, SkillRefreshStatus::Refreshed { .. })
    ));
    let central = PathBuf::from(
        f.store
            .get_skill_by_id(&f.skill_id)
            .expect("query")
            .expect("skill")
            .central_path,
    );
    assert_eq!(
        fs::read_to_string(central.join("a.txt")).expect("read central"),
        "v2"
    );
    assert_eq!(
        phases,
        vec![
            (RefreshPhase::Acquiring, "alpha".to_string()),
            (RefreshPhase::Applying, "alpha".to_string()),
        ],
        "progress ticks come from the backend, one per phase step"
    );
}

#[test]
fn a_skill_that_fails_acquisition_is_reported_and_never_finalized() {
    let f = fixture();
    let central_before = f
        .store
        .get_skill_by_id(&f.skill_id)
        .expect("query")
        .expect("skill");
    // The local source is gone: acquisition cannot produce bytes.
    let source_path = f.source.path().to_path_buf();
    fs::remove_dir_all(&source_path).expect("remove source");

    let report = refresh(&f, RefreshPolicy::default());

    assert!(
        matches!(
            report.skills.as_slice(),
            [outcome] if matches!(outcome.status, SkillRefreshStatus::Failed { .. })
        ),
        "got {:?}",
        report
    );
    let after = f
        .store
        .get_skill_by_id(&f.skill_id)
        .expect("query")
        .expect("skill");
    assert_eq!(
        after.updated_at, central_before.updated_at,
        "a skill that failed acquisition must not be finalized"
    );
    assert_eq!(
        fs::read_to_string(PathBuf::from(&after.central_path).join("a.txt")).expect("read central"),
        "v1",
        "the central copy is untouched"
    );
}

#[test]
fn reassert_auto_sync_creates_a_target_the_skill_was_never_on() {
    let f = fixture();
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    fs::create_dir_all(f.paths.home.join(claude.relative_detect_dir)).expect("install tool");
    assert!(
        f.store
            .list_skill_targets(&f.skill_id)
            .expect("query")
            .is_empty(),
        "the skill starts on no Tool"
    );

    refresh(
        &f,
        RefreshPolicy {
            reassert_auto_sync: true,
        },
    );

    let row = f
        .store
        .get_skill_target(&f.skill_id, "claude_code")
        .expect("query")
        .expect("the re-assert creates the missing target");
    assert!(PathBuf::from(&row.target_path).exists());
}

#[test]
fn without_the_reassert_policy_a_missing_target_stays_missing() {
    let f = fixture();
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    fs::create_dir_all(f.paths.home.join(claude.relative_detect_dir)).expect("install tool");

    refresh(&f, RefreshPolicy::default());

    assert!(
        f.store
            .get_skill_target(&f.skill_id, "claude_code")
            .expect("query")
            .is_none(),
        "refresh alone never creates a Sync target"
    );
    assert!(!f.paths.home.join(".claude/skills/alpha").exists());
}

/// A store failure inside the re-assert is report data, not a log line: the
/// skill stays `Refreshed` (finalize and Propagation did succeed) and carries
/// the error so the batch report can count it.
#[test]
fn a_failed_reassert_is_reported_alongside_the_refreshed_status() {
    let targets = vec![outcome_for("claude_code")];

    let (kept, error) = merge_reassert(targets, Err(anyhow::anyhow!("store is gone")));

    assert_eq!(kept.len(), 1, "Propagation's own outcomes survive");
    assert_eq!(
        format!(
            "{:#}",
            error.expect("the failure is carried, never dropped")
        ),
        "store is gone"
    );
}

#[test]
fn a_successful_reassert_extends_the_targets_and_reports_no_error() {
    let (kept, error) = merge_reassert(
        vec![outcome_for("claude_code")],
        Ok(vec![outcome_for("codex")]),
    );

    let tools: Vec<String> = kept
        .iter()
        .map(|o| match &o.scope {
            PropagationScope::Global { tool } => tool.clone(),
            other => panic!("expected a global scope, got {other:?}"),
        })
        .collect();
    assert_eq!(tools, vec!["claude_code".to_string(), "codex".to_string()]);
    assert!(error.is_none());
}

fn outcome_for(tool: &str) -> PropagationOutcome {
    PropagationOutcome {
        scope: PropagationScope::Global {
            tool: tool.to_string(),
        },
        status: PropagationStatus::Synced {
            mode_used: crate::core::sync_status::SyncMode::Symlink,
        },
    }
}

#[test]
fn a_skill_that_fails_acquisition_is_excluded_from_the_reassert() {
    let f = fixture();
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    fs::create_dir_all(f.paths.home.join(claude.relative_detect_dir)).expect("install tool");
    fs::remove_dir_all(f.source.path()).expect("remove source");

    refresh(
        &f,
        RefreshPolicy {
            reassert_auto_sync: true,
        },
    );

    assert!(
        f.store
            .get_skill_target(&f.skill_id, "claude_code")
            .expect("query")
            .is_none(),
        "a skill whose bytes could not be acquired must not be synced anywhere"
    );
}

// ---------------------------------------------------------------------------
// Phase one — the bounded acquisition pool
// ---------------------------------------------------------------------------

/// N local-source Managed skills named `s0..s{n-1}` in one temp fixture.
struct PoolFixture {
    _dir: tempfile::TempDir,
    paths: InstallerPaths,
    store: SkillStore,
    _sources: Vec<tempfile::TempDir>,
    names: Vec<String>,
}

fn pool_fixture(n: usize) -> PoolFixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = InstallerPaths {
        home: dir.path().join("home"),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.home).expect("create home");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");

    let mut sources = Vec::new();
    let mut names = Vec::new();
    for i in 0..n {
        let name = format!("s{i}");
        let source = tempfile::tempdir().expect("source tempdir");
        fs::write(
            source.path().join("SKILL.md"),
            format!("---\nname: {name}\n---\n"),
        )
        .expect("write SKILL.md");
        install_local_skill(&paths, &store, source.path(), Some(name.clone())).expect("install");
        sources.push(source);
        names.push(name);
    }
    PoolFixture {
        _dir: dir,
        paths,
        store,
        _sources: sources,
        names,
    }
}

/// The real (fast, local-source) acquisition with an artificial latency, so a
/// pooled batch is distinguishable from a sequential one by wall clock alone.
fn slow_acquire<'a>(
    f: &'a PoolFixture,
    delay: impl Fn(&str) -> std::time::Duration + Sync + 'a,
) -> impl Fn(&str, Option<&CancelToken>) -> anyhow::Result<crate::core::installer::AcquiredUpdate>
       + Sync
       + 'a {
    move |skill_id, cancel| {
        let name = f
            .store
            .get_skill_by_id(skill_id)
            .expect("query")
            .expect("skill")
            .name;
        std::thread::sleep(delay(&name));
        crate::core::installer::acquire_managed_skill_update_with(
            &f.paths,
            &f.store,
            skill_id,
            cancel,
            &crate::core::git_acquisition::HttpGithubApi::new(None),
            0,
        )
    }
}

const LATENCY_MS: u64 = 150;

/// Eight skills, each acquisition ~150ms: sequential would be ~1.2s, a pool of
/// four is ~0.3s. Overlap is the only way to come in under 60% of sequential.
#[test]
fn acquisitions_overlap_instead_of_running_one_at_a_time() {
    let f = pool_fixture(8);
    let acquire = slow_acquire(&f, |_| std::time::Duration::from_millis(LATENCY_MS));

    let started = std::time::Instant::now();
    let report = refresh_managed_skills_with(
        &f.paths,
        &f.store,
        RefreshSelection::All,
        RefreshPolicy::default(),
        None,
        3000,
        |_| {},
        &acquire,
    )
    .expect("refresh");
    let elapsed = started.elapsed();

    assert_eq!(report.skills.len(), 8);
    assert!(
        report
            .skills
            .iter()
            .all(|o| matches!(o.status, SkillRefreshStatus::Refreshed { .. })),
        "every skill is refreshed: {report:?}"
    );
    let sequential = std::time::Duration::from_millis(LATENCY_MS * 8);
    assert!(
        elapsed < sequential.mul_f32(0.6),
        "pooled acquisition took {elapsed:?}, sequential would be {sequential:?}"
    );
}

/// Progress reads as completion order, not dispatch order: the slowest skill
/// is dispatched first and still ticks last, and the indices count completions
/// 1..n.
#[test]
fn acquire_progress_counts_completions_in_completion_order() {
    let f = pool_fixture(4);
    // s0 is dispatched first and is by far the slowest.
    let acquire = slow_acquire(&f, |name| {
        std::time::Duration::from_millis(if name == "s0" { 400 } else { 20 })
    });

    let mut ticks: Vec<(usize, String)> = Vec::new();
    refresh_managed_skills_with(
        &f.paths,
        &f.store,
        RefreshSelection::All,
        RefreshPolicy::default(),
        None,
        3000,
        |p| {
            if p.phase == RefreshPhase::Acquiring {
                ticks.push((p.index, p.skill_name.to_string()));
            }
        },
        &acquire,
    )
    .expect("refresh");

    assert_eq!(
        ticks.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
        vec![1, 2, 3, 4],
        "acquire ticks count completions"
    );
    assert_eq!(
        ticks.last().map(|(_, name)| name.as_str()),
        Some("s0"),
        "the slowest skill ticks last even though it was dispatched first: {ticks:?}"
    );
    assert_eq!(f.names.len(), 4);
}

/// Cancellation observed mid-batch: nothing is finalized (no `Applying` tick,
/// no central copy rewritten) and every skill is reported as cancelled.
#[test]
fn cancelling_mid_batch_finalizes_nothing() {
    let f = pool_fixture(6);
    let token = CancelToken::new();
    let completed = Mutex::new(0usize);
    let inner = slow_acquire(&f, |_| std::time::Duration::from_millis(30));
    let acquire = |skill_id: &str, cancel: Option<&CancelToken>| {
        let result = inner(skill_id, cancel);
        let mut done = completed.lock().expect("lock");
        *done += 1;
        if *done == 2 {
            token.cancel();
        }
        result
    };
    let before: Vec<(String, i64)> = f
        .store
        .list_skills()
        .expect("list")
        .into_iter()
        .map(|s| (s.id, s.updated_at))
        .collect();

    let mut phases: Vec<RefreshPhase> = Vec::new();
    let report = refresh_managed_skills_with(
        &f.paths,
        &f.store,
        RefreshSelection::All,
        RefreshPolicy::default(),
        Some(&token),
        3000,
        |p| phases.push(p.phase),
        &acquire,
    )
    .expect("a cancelled batch still reports");

    assert_eq!(report.skills.len(), 6, "every selected skill is reported");
    assert!(
        !phases.contains(&RefreshPhase::Applying),
        "a cancelled batch never enters the apply phase: {phases:?}"
    );
    assert!(
        report.skills.iter().all(|o| matches!(
            &o.status,
            SkillRefreshStatus::Failed { error }
                if error.downcast_ref::<SignalError>() == Some(&SignalError::Cancelled)
        )),
        "cancelled skills are reported as cancelled: {report:?}"
    );
    let after: Vec<(String, i64)> = f
        .store
        .list_skills()
        .expect("list")
        .into_iter()
        .map(|s| (s.id, s.updated_at))
        .collect();
    assert_eq!(before, after, "no skill was finalized");
}

// ---------------------------------------------------------------------------
// Same-repository skills under the pool
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

/// A local repository holding one skill under `skills/a`, cloned by path.
fn fixture_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    git(&["init", "-q", "-b", "main", "."], dir.path());
    fs::create_dir_all(dir.path().join("skills/a")).expect("mkdir");
    fs::write(
        dir.path().join("skills/a/SKILL.md"),
        "---\nname: shared\n---\n",
    )
    .expect("write");
    git(&["add", "-A"], dir.path());
    git(&["commit", "-q", "-m", "init"], dir.path());
    dir
}

/// Two Managed skills from the same repository are acquired concurrently by
/// the pool. `git_cache`'s per-key lock is what keeps that safe: both land the
/// same revision and the cache entry's metadata is intact.
#[test]
fn two_skills_from_one_repository_share_the_cache_without_corrupting_it() {
    let f = pool_fixture(0);
    let repo = fixture_repo();
    let url = repo.path().to_string_lossy().to_string();
    for name in ["one", "two"] {
        crate::core::installer::install_git_skill_from_selection(
            &f.paths,
            &f.store,
            &url,
            "skills/a",
            Some(name.to_string()),
            None,
        )
        .expect("install from the fixture repo");
    }

    let report = refresh_managed_skills(
        &f.paths,
        &f.store,
        RefreshSelection::All,
        RefreshPolicy::default(),
        None,
        3000,
        |_| {},
    )
    .expect("refresh");

    let revisions: Vec<String> = report
        .skills
        .iter()
        .map(|o| match &o.status {
            SkillRefreshStatus::Refreshed {
                source_revision, ..
            } => source_revision
                .clone()
                .expect("a git skill records its revision"),
            other => panic!("expected a refreshed skill, got {other:?}"),
        })
        .collect();
    assert_eq!(revisions.len(), 2);
    assert_eq!(
        revisions[0], revisions[1],
        "both skills come from the same commit"
    );

    let cache_root = f.paths.cache_dir.join("skills-hub-git-cache");
    let entries: Vec<PathBuf> = fs::read_dir(&cache_root)
        .expect("read cache root")
        .map(|e| e.expect("entry").path())
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "one clone URL + subpath is one cache entry"
    );
    let meta = fs::read_to_string(entries[0].join(".skills-hub-cache.json"))
        .expect("the cache metadata survives concurrent fetches");
    assert!(
        meta.contains(&revisions[0]),
        "cache metadata records the fetched head: {meta}"
    );
}
