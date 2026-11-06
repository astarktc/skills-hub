//! Tests for `core::refresh` — the Refresh (all) batch: acquire every
//! selected skill, then finalize + propagate each under the mutation guard.
//!
//! Every case runs against a temp home / central dir / DB; installedness is
//! faked by creating a Tool's detect dir under the temp home.

use crate::core::environment::HostRoots;
use crate::core::removal_fence::central_repo_of;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

use crate::core::cancel_token::CancelToken;
use crate::core::errors::CommandError;

#[test]
fn acquisition_skips_cross_the_wire_without_counters() {
    use crate::core::{
        refresh::{RefreshReport, SkillRefreshOutcome},
        skill_update::UpdateSkip,
    };
    for (reason, wire) in [
        (UpdateSkip::SkillGone, "skill_gone"),
        (UpdateSkip::StaleAcquisition, "stale_acquisition"),
    ] {
        let report = RefreshReport {
            skills: vec![SkillRefreshOutcome {
                skill_id: "s1".into(),
                skill_name: "alpha".into(),
                status: SkillRefreshStatus::SkippedAcquisition { reason },
            }],
        };
        assert_eq!(
            serde_json::to_value(report).unwrap(),
            serde_json::json!({
                "skills": [{ "skill_id": "s1", "skill_name": "alpha", "status": {
                    "status": "skipped_acquisition", "reason": wire
                } }]
            })
        );
    }
}

/// D10: cancelled and declined are their own outcome kinds on the wire.
#[test]
fn cancelled_and_declined_rows_cross_the_wire_as_their_own_kinds() {
    use crate::core::global_sync::{BatchTargetOutcome, BatchTargetStatus};
    use crate::core::refresh::SkillRefreshOutcome;
    let cancelled = SkillRefreshOutcome {
        skill_id: "s1".into(),
        skill_name: "alpha".into(),
        status: SkillRefreshStatus::Cancelled,
    };
    assert_eq!(
        serde_json::to_value(cancelled).unwrap(),
        serde_json::json!({ "skill_id": "s1", "skill_name": "alpha", "status": { "status": "cancelled" } })
    );
    let declined = BatchTargetOutcome {
        skill_id: "s1".into(),
        skill_name: "alpha".into(),
        tool_key: "claude_code".into(),
        folder_key: "claude_code".into(),
        status: BatchTargetStatus::Declined,
    };
    assert_eq!(
        serde_json::to_value(declined).unwrap(),
        serde_json::json!({
            "skill_id": "s1", "skill_name": "alpha", "tool_key": "claude_code",
            "folder_key": "claude_code", "status": { "status": "declined" }
        })
    );
}

use crate::core::installer::{install_imported_skill, install_local_skill, InstallerPaths};
use crate::core::propagation::{PropagationScope, PropagationStatus};
use crate::core::refresh::{
    refresh_managed_skills, refresh_managed_skills_with, RefreshPhase, RefreshSelection,
    SkillRefreshStatus,
};
use crate::core::skill_store::SkillStore;
use crate::core::tool_adapters::{adapter_by_key, mark_installed_in};
use crate::core::unlocatable::UnlocatableState;

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
        roots: HostRoots::under_home(&dir.path().join("home")),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.roots.home).expect("create home");
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

fn refresh(f: &Fixture) -> crate::core::refresh::RefreshReport {
    refresh_managed_skills(
        &f.paths,
        &f.store,
        &crate::core::credentials::MemoryStore::new(),
        RefreshSelection::All,
        None,
        |_| {},
    )
    .expect("refresh reads its own skill list")
}

#[test]
fn a_refreshed_skill_gets_its_new_bytes_and_reports_its_targets() {
    let f = fixture();
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    mark_installed_in(&f.paths.roots, claude);
    fs::write(f.source.path().join("a.txt"), "v2").expect("write a.txt");

    let mut phases: Vec<(RefreshPhase, String)> = Vec::new();
    let report = refresh_managed_skills(
        &f.paths,
        &f.store,
        &crate::core::credentials::MemoryStore::new(),
        RefreshSelection::Ids(vec![f.skill_id.clone()]),
        None,
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
    // The local source is gone: acquisition cannot produce bytes. Named
    // explicitly (a single Update) — Refresh (all) would skip it instead.
    let source_path = f.source.path().to_path_buf();
    fs::remove_dir_all(&source_path).expect("remove source");

    let report = refresh_managed_skills(
        &f.paths,
        &f.store,
        &crate::core::credentials::MemoryStore::new(),
        RefreshSelection::Ids(vec![f.skill_id.clone()]),
        None,
        |_| {},
    )
    .expect("refresh");

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

/// D34: auto-sync decides where a new skill lands, never what an Update
/// does. A skill deployed to 1 of 3 selected, installed Tools stays on that
/// one Tool through Refresh all with auto-sync on: the report carries no
/// global target for the other two and no row or artifact is created.
#[test]
fn refresh_all_with_auto_sync_on_creates_no_deployment() {
    use crate::core::global_sync::{sync_skills_to_tools, BatchPolicy, BatchSkill};
    let f = fixture();
    for key in ["claude_code", "codex", "cursor"] {
        mark_installed_in(&f.paths.roots, adapter_by_key(key).expect("adapter"));
    }
    f.store
        .set_setting(
            "global_selected_tools_v1",
            r#"["claude_code","codex","cursor"]"#,
        )
        .expect("record the selection");
    f.store
        .set_setting("auto_sync_enabled", "true")
        .expect("auto-sync on");
    let skill = f
        .store
        .get_skill_by_id(&f.skill_id)
        .expect("query")
        .expect("skill");
    sync_skills_to_tools(
        &f.paths.roots,
        &f.store,
        &central_repo_of(&f.store),
        &[BatchSkill {
            skill_id: skill.id.clone(),
            skill_name: skill.name.clone(),
            source_path: PathBuf::from(&skill.central_path),
        }],
        &["claude_code".to_string()],
        &BatchPolicy {
            overwrite: false,
            overwrite_if_same_content: true,
            overrides: Vec::new(),
        },
        2000,
        |_| {},
    );
    fs::write(f.source.path().join("a.txt"), "v2").expect("write a.txt");

    let report = refresh(&f);

    let SkillRefreshStatus::Refreshed { targets, .. } = &report.skills[0].status else {
        panic!("expected Refreshed, got {report:?}");
    };
    let tools: Vec<&str> = targets
        .iter()
        .map(|o| match &o.scope {
            PropagationScope::Global { tool } => tool.as_str(),
            other => panic!("expected a global scope, got {other:?}"),
        })
        .collect();
    assert_eq!(
        tools,
        vec!["claude_code"],
        "only the existing deployment is reported"
    );
    // A symlink already follows the central copy (`LinkFollowsSource`).
    assert!(
        !matches!(targets[0].status, PropagationStatus::Failed { .. }),
        "got {:?}",
        targets[0].status
    );
    let rows: Vec<String> = f
        .store
        .list_skill_targets(&f.skill_id)
        .expect("query")
        .into_iter()
        .map(|row| row.tool)
        .collect();
    assert_eq!(rows, vec!["claude_code".to_string()], "no row is created");
    assert!(!f.paths.roots.home.join(".codex/skills/alpha").exists());
    assert!(!f.paths.roots.home.join(".cursor/skills/alpha").exists());
}

/// Record the stored health a kept Delete leaves behind (`artifact_removal`).
fn mark_delete_incomplete(store: &SkillStore, skill_id: &str) {
    crate::core::skill_health::record_outcome(
        store,
        skill_id,
        crate::core::skill_health::SkillOutcomeKind::DeleteIncomplete,
        &CommandError::Other {
            message: "permission denied".into(),
        },
    )
    .expect("record health");
}

/// D35: a skill whose Delete was incomplete is never updated — in Refresh all
/// *and* in a named Update it is reported skipped with the typed reason,
/// never acquired, its targets and its stored health untouched.
#[test]
fn a_delete_incomplete_skill_is_skipped_by_refresh_all_and_by_a_named_update() {
    use crate::core::refresh::RefreshSkipReason;
    let f = fixture();
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    mark_installed_in(&f.paths.roots, claude);
    let skill = f
        .store
        .get_skill_by_id(&f.skill_id)
        .expect("query")
        .expect("skill");
    crate::core::global_sync::sync_skills_to_tools(
        &f.paths.roots,
        &f.store,
        &central_repo_of(&f.store),
        &[crate::core::global_sync::BatchSkill {
            skill_id: skill.id.clone(),
            skill_name: skill.name.clone(),
            source_path: PathBuf::from(&skill.central_path),
        }],
        &["claude_code".to_string()],
        &crate::core::global_sync::BatchPolicy {
            overwrite: false,
            overwrite_if_same_content: true,
            overrides: Vec::new(),
        },
        2000,
        |_| {},
    );
    mark_delete_incomplete(&f.store, &f.skill_id);
    // The records carry no `PartialEq`; their debug form is the snapshot.
    let health_of = || format!("{:?}", f.store.skill_outcome(&f.skill_id).expect("health"));
    let targets_of = || {
        format!(
            "{:?}",
            f.store.list_skill_targets(&f.skill_id).expect("targets")
        )
    };
    let health_before = health_of();
    let targets_before = targets_of();
    fs::write(f.source.path().join("a.txt"), "v2").expect("write a.txt");

    for selection in [
        RefreshSelection::All,
        RefreshSelection::Ids(vec![f.skill_id.clone()]),
    ] {
        let acquired = AtomicUsize::new(0);
        let report = refresh_managed_skills_with(
            &f.paths,
            &f.store,
            selection.clone(),
            None,
            |_| {},
            &|skill_id, cancel| {
                acquired.fetch_add(1, Ordering::SeqCst);
                crate::core::skill_update::acquire_update(
                    &f.paths,
                    &f.store,
                    skill_id,
                    cancel,
                    &crate::core::git_acquisition::HttpGithubApi::new(None),
                    0,
                )
            },
        )
        .expect("refresh");

        let [outcome] = report.skills.as_slice() else {
            panic!("one outcome for {selection:?}: {report:?}");
        };
        assert_eq!(outcome.skill_id, f.skill_id);
        assert!(
            matches!(
                outcome.status,
                SkillRefreshStatus::Skipped {
                    reason: RefreshSkipReason::DeleteIncomplete
                }
            ),
            "{selection:?}: got {:?}",
            outcome.status
        );
        assert_eq!(
            acquired.load(Ordering::SeqCst),
            0,
            "{selection:?}: no acquisition"
        );
        assert_eq!(
            targets_of(),
            targets_before,
            "{selection:?}: no target is touched"
        );
        assert_eq!(
            health_of(),
            health_before,
            "{selection:?}: the health is unchanged"
        );
        let central = PathBuf::from(&skill.central_path);
        assert_eq!(
            fs::read_to_string(central.join("a.txt")).expect("read central"),
            "v1",
            "{selection:?}: the central copy is untouched"
        );
    }
}

/// D35 boundary of `RefreshSelection::All` (ticket 33 audit F2): the
/// delete-incomplete gate takes precedence over Unlocatable for a member
/// (source missing, central missing), and never reaches a non-member — an
/// imported skill carrying the same health stays out of the report. No case
/// acquires anything or touches health, targets or the central copy.
#[test]
fn refresh_all_reports_delete_incomplete_before_unlocatable_and_never_for_a_non_member() {
    use crate::core::refresh::{RefreshReport, RefreshSkipReason};

    /// Refresh all with a counting acquirer; answers (report, acquisitions).
    fn refresh_all_counting(paths: &InstallerPaths, store: &SkillStore) -> (RefreshReport, usize) {
        let acquired = AtomicUsize::new(0);
        let report = refresh_managed_skills_with(
            paths,
            store,
            RefreshSelection::All,
            None,
            |_| {},
            &|skill_id, cancel| {
                acquired.fetch_add(1, Ordering::SeqCst);
                crate::core::skill_update::acquire_update(
                    paths,
                    store,
                    skill_id,
                    cancel,
                    &crate::core::git_acquisition::HttpGithubApi::new(None),
                    0,
                )
            },
        )
        .expect("refresh");
        (report, acquired.load(Ordering::SeqCst))
    }
    /// Health, targets and central bytes; the records carry no `PartialEq`,
    /// so their debug form is the snapshot.
    fn snapshot(store: &SkillStore, skill_id: &str) -> (String, String, Option<String>) {
        let central = store
            .get_skill_by_id(skill_id)
            .expect("query")
            .expect("skill")
            .central_path;
        (
            format!("{:?}", store.skill_outcome(skill_id).expect("health")),
            format!("{:?}", store.list_skill_targets(skill_id).expect("targets")),
            fs::read_to_string(PathBuf::from(central).join("SKILL.md")).ok(),
        )
    }

    // (a) source missing, (b) central missing: a refreshable member that is
    // also Unlocatable reads DeleteIncomplete.
    for (label, state) in [
        ("source missing", UnlocatableState::SourceMissing),
        ("central missing", UnlocatableState::CentralMissing),
    ] {
        let f = fixture();
        mark_installed_in(
            &f.paths.roots,
            adapter_by_key("claude_code").expect("claude_code adapter"),
        );
        let skill = f
            .store
            .get_skill_by_id(&f.skill_id)
            .expect("query")
            .expect("skill");
        crate::core::global_sync::sync_skills_to_tools(
            &f.paths.roots,
            &f.store,
            &central_repo_of(&f.store),
            &[crate::core::global_sync::BatchSkill {
                skill_id: skill.id.clone(),
                skill_name: skill.name.clone(),
                source_path: PathBuf::from(&skill.central_path),
            }],
            &["claude_code".to_string()],
            &crate::core::global_sync::BatchPolicy {
                overwrite: false,
                overwrite_if_same_content: true,
                overrides: Vec::new(),
            },
            2000,
            |_| {},
        );
        match state {
            UnlocatableState::SourceMissing => {
                fs::remove_dir_all(f.source.path()).expect("remove source")
            }
            UnlocatableState::CentralMissing => {
                fs::remove_dir_all(&skill.central_path).expect("remove central copy")
            }
        }
        // Premise: without the health the skill is reported Unlocatable.
        let (report, acquired) = refresh_all_counting(&f.paths, &f.store);
        assert!(
            matches!(
                report.skills.as_slice(),
                [outcome] if matches!(
                    &outcome.status,
                    SkillRefreshStatus::Skipped {
                        reason: RefreshSkipReason::Unlocatable { state: observed }
                    } if *observed == state
                )
            ),
            "{label}: premise {report:?}"
        );
        assert_eq!(acquired, 0, "{label}: premise acquires nothing");

        mark_delete_incomplete(&f.store, &f.skill_id);
        let before = snapshot(&f.store, &f.skill_id);
        let (report, acquired) = refresh_all_counting(&f.paths, &f.store);

        let [outcome] = report.skills.as_slice() else {
            panic!("{label}: one outcome, got {report:?}");
        };
        assert_eq!(outcome.skill_id, f.skill_id);
        assert!(
            matches!(
                outcome.status,
                SkillRefreshStatus::Skipped {
                    reason: RefreshSkipReason::DeleteIncomplete
                }
            ),
            "{label}: DeleteIncomplete takes precedence over Unlocatable, got {:?}",
            outcome.status
        );
        assert_eq!(acquired, 0, "{label}: no acquisition");
        assert_eq!(
            snapshot(&f.store, &f.skill_id),
            before,
            "{label}: health, targets and central copy unchanged"
        );
    }

    // (c) an imported skill is not a member: the same health never puts it
    // in the report.
    let f = pool_fixture(0);
    let imported_id = install_imported(&f.paths, &f.store, "taken-over");
    mark_delete_incomplete(&f.store, &imported_id);
    let before = snapshot(&f.store, &imported_id);

    let (report, acquired) = refresh_all_counting(&f.paths, &f.store);

    assert!(report.skills.is_empty(), "imported: absent, got {report:?}");
    assert_eq!(acquired, 0, "imported: no acquisition");
    assert_eq!(
        snapshot(&f.store, &imported_id),
        before,
        "imported: unchanged"
    );
}

/// The skip crosses the wire as a tagged reason.
#[test]
fn refresh_skip_reasons_cross_the_wire_tagged() {
    use crate::core::refresh::{RefreshSkipReason, SkillRefreshOutcome};
    for (reason, wire) in [
        (
            RefreshSkipReason::DeleteIncomplete,
            serde_json::json!({ "kind": "delete_incomplete" }),
        ),
        (
            RefreshSkipReason::Unlocatable {
                state: UnlocatableState::SourceMissing,
            },
            serde_json::json!({ "kind": "unlocatable", "state": "source_missing" }),
        ),
    ] {
        let outcome = SkillRefreshOutcome {
            skill_id: "s1".into(),
            skill_name: "alpha".into(),
            status: SkillRefreshStatus::Skipped { reason },
        };
        assert_eq!(
            serde_json::to_value(outcome).unwrap(),
            serde_json::json!({ "skill_id": "s1", "skill_name": "alpha", "status": {
                "status": "skipped", "reason": wire
            } })
        );
    }
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
        roots: HostRoots::under_home(&dir.path().join("home")),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.roots.home).expect("create home");
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

/// The real (fast, local-source) acquisition with an artificial per-skill
/// latency, so the pool's completion order and its cancellation window are
/// observable.
fn slow_acquire<'a>(
    f: &'a PoolFixture,
    delay: impl Fn(&str) -> Duration + Sync + 'a,
) -> impl Fn(&str, Option<&CancelToken>) -> anyhow::Result<crate::core::skill_update::UpdateRequest>
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
        crate::core::skill_update::acquire_update(
            &f.paths,
            &f.store,
            skill_id,
            cancel,
            &crate::core::git_acquisition::HttpGithubApi::new(None),
            0,
        )
    }
}

/// A door every acquisition passes through, counting how many are inside at
/// once. Until overlap has been witnessed, an acquisition waits at the door
/// for a second one to arrive — a rendezvous only a pool can complete, so
/// concurrency is proven by what was observed, never by how long the batch
/// took. The wait is bounded so a sequential regression fails the assertion
/// below instead of hanging the gate; once it has timed out nobody waits.
struct OverlapDoor {
    in_flight: Mutex<usize>,
    arrived: Condvar,
    max_in_flight: AtomicUsize,
    gave_up: AtomicBool,
}

impl OverlapDoor {
    const RENDEZVOUS_BOUND: Duration = Duration::from_secs(10);

    fn new() -> Self {
        Self {
            in_flight: Mutex::new(0),
            arrived: Condvar::new(),
            max_in_flight: AtomicUsize::new(0),
            gave_up: AtomicBool::new(false),
        }
    }

    fn overlap_witnessed(&self) -> bool {
        self.max_in_flight.load(Ordering::SeqCst) >= 2
    }

    fn enter(&self) {
        let mut inside = self.in_flight.lock().expect("door lock");
        *inside += 1;
        self.arrived.notify_all();
        while *inside < 2 && !self.overlap_witnessed() && !self.gave_up.load(Ordering::SeqCst) {
            let (guard, timeout) = self
                .arrived
                .wait_timeout(inside, Self::RENDEZVOUS_BOUND)
                .expect("door lock");
            inside = guard;
            if timeout.timed_out() {
                self.gave_up.store(true, Ordering::SeqCst);
            }
        }
        self.max_in_flight.fetch_max(*inside, Ordering::SeqCst);
    }

    fn leave(&self) {
        *self.in_flight.lock().expect("door lock") -= 1;
    }
}

/// Eight skills through a pool of four: at some point two acquisitions are
/// inside the door together. A sequential batch would never have two in
/// flight, whatever the load on the machine.
#[test]
fn acquisitions_overlap_instead_of_running_one_at_a_time() {
    let f = pool_fixture(8);
    let door = OverlapDoor::new();
    let inner = slow_acquire(&f, |_| Duration::ZERO);
    let acquire = |skill_id: &str, cancel: Option<&CancelToken>| {
        door.enter();
        let result = inner(skill_id, cancel);
        door.leave();
        result
    };

    let report = refresh_managed_skills_with(
        &f.paths,
        &f.store,
        RefreshSelection::All,
        None,
        |_| {},
        &acquire,
    )
    .expect("refresh");

    assert_eq!(report.skills.len(), 8);
    assert!(
        report
            .skills
            .iter()
            .all(|o| matches!(o.status, SkillRefreshStatus::Refreshed { .. })),
        "every skill is refreshed: {report:?}"
    );
    let observed = door.max_in_flight.load(Ordering::SeqCst);
    assert!(
        observed >= 2,
        "acquisitions ran one at a time: at most {observed} in flight"
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
        Duration::from_millis(if name == "s0" { 400 } else { 20 })
    });

    let mut ticks: Vec<(usize, String)> = Vec::new();
    refresh_managed_skills_with(
        &f.paths,
        &f.store,
        RefreshSelection::All,
        None,
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
    let inner = slow_acquire(&f, |_| Duration::from_millis(30));
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
        Some(&token),
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
        report
            .skills
            .iter()
            .all(|o| matches!(o.status, SkillRefreshStatus::Cancelled)),
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
        &crate::core::credentials::MemoryStore::new(),
        RefreshSelection::All,
        None,
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

/// An `imported` Managed skill taken over from `claude_code`'s skills dir,
/// installed the way Onboarding import records it.
fn install_imported(paths: &InstallerPaths, store: &SkillStore, name: &str) -> String {
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    let found = paths.roots.home.join(claude.skills_dir.rel).join(name);
    fs::create_dir_all(&found).expect("tool skill dir");
    fs::write(found.join("SKILL.md"), format!("---\nname: {name}\n---\n")).expect("write");
    install_imported_skill(
        paths,
        store,
        &found,
        Some(name.to_string()),
        Some("claude_code"),
    )
    .expect("import")
    .skill_id
}

/// Refresh (all) over one `git` and one `imported` skill: the imported one
/// is not a member of the batch — not acquired, not applied, not reported —
/// so the report counts exactly one skill, not "1 refreshed, 1 skipped".
#[test]
fn refresh_all_never_puts_an_imported_skill_in_the_batch() {
    let f = pool_fixture(0);
    let repo = fixture_repo();
    let url = repo.path().to_string_lossy().to_string();
    let git_skill = crate::core::installer::install_git_skill_from_selection(
        &f.paths,
        &f.store,
        &url,
        "skills/a",
        Some("from-git".to_string()),
        None,
    )
    .expect("install from the fixture repo");
    let imported_id = install_imported(&f.paths, &f.store, "taken-over");
    let imported_before = f
        .store
        .get_skill_by_id(&imported_id)
        .expect("query")
        .expect("imported record");

    let acquired: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let mut progress: Vec<(RefreshPhase, String)> = Vec::new();
    let report = refresh_managed_skills_with(
        &f.paths,
        &f.store,
        RefreshSelection::All,
        None,
        |p| progress.push((p.phase, p.skill_name.to_string())),
        &|skill_id, cancel| {
            acquired.lock().unwrap().push(skill_id.to_string());
            crate::core::skill_update::acquire_update(
                &f.paths,
                &f.store,
                skill_id,
                cancel,
                &crate::core::git_acquisition::HttpGithubApi::new(None),
                0,
            )
        },
    )
    .expect("refresh");

    assert_eq!(
        acquired.into_inner().unwrap(),
        vec![git_skill.skill_id.clone()],
        "only the git skill reaches the acquire pool"
    );
    assert_eq!(report.skills.len(), 1, "got {:?}", report);
    assert_eq!(report.skills[0].skill_id, git_skill.skill_id);
    assert!(matches!(
        report.skills[0].status,
        SkillRefreshStatus::Refreshed { .. }
    ));
    assert!(
        progress
            .iter()
            .all(|(_, name)| name == "from-git" && progress.len() == 2),
        "progress ticks name the batch's one member only: {:?}",
        progress
    );
    // The imported skill's record was not touched.
    let imported_after = f
        .store
        .get_skill_by_id(&imported_id)
        .expect("query")
        .expect("imported record");
    assert_eq!(imported_after.updated_at, imported_before.updated_at);
}

/// A single Update names the skill explicitly; for an imported skill the
/// answer is a typed refusal, not a silent empty report.
#[test]
fn a_single_update_of_an_imported_skill_is_refused_with_a_typed_condition() {
    let f = pool_fixture(0);
    let imported_id = install_imported(&f.paths, &f.store, "taken-over");

    let report = refresh_managed_skills(
        &f.paths,
        &f.store,
        &crate::core::credentials::MemoryStore::new(),
        RefreshSelection::Ids(vec![imported_id.clone()]),
        None,
        |_| {},
    )
    .expect("refresh");

    let [outcome] = report.skills.as_slice() else {
        panic!("one outcome for the one requested skill: {:?}", report);
    };
    let SkillRefreshStatus::Failed { error } = &outcome.status else {
        panic!("an imported skill cannot be updated: {:?}", outcome);
    };
    assert!(
        matches!(error, CommandError::NotRefreshable { name } if name == "taken-over"),
        "got {error:#}"
    );
    assert!(
        f.paths
            .central_dir
            .join("taken-over")
            .join("SKILL.md")
            .is_file(),
        "the central copy is untouched"
    );
}

// ---------------------------------------------------------------------------
// Unlocatable skills
// ---------------------------------------------------------------------------

/// Refresh (all) over one healthy `local` skill, one whose source folder is
/// gone and one whose central copy is gone: only the healthy one reaches the
/// acquire pool; the two unlocatable ones are reported *skipped* by name
/// with their state, and get no Sync target (a central-missing skill's
/// target would be a dangling link).
#[test]
fn refresh_all_skips_unlocatable_skills_and_mints_no_targets_for_them() {
    use crate::core::refresh::RefreshSkipReason;
    let f = pool_fixture(3);
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    mark_installed_in(&f.paths.roots, claude);
    let ids: Vec<String> = f
        .store
        .list_skills()
        .expect("list")
        .into_iter()
        .map(|s| s.id)
        .collect();
    let by_name = |name: &str| {
        f.store
            .list_skills()
            .expect("list")
            .into_iter()
            .find(|s| s.name == name)
            .expect("skill")
    };
    // s1: source folder gone (moved away).
    let mut source_gone = by_name("s1");
    source_gone.source_ref = Some(
        f.paths
            .roots
            .home
            .join("moved-away")
            .to_string_lossy()
            .to_string(),
    );
    f.store.upsert_skill(&source_gone).expect("upsert");
    // s2: central copy gone.
    let central_gone = by_name("s2");
    fs::remove_dir_all(&central_gone.central_path).expect("remove central");

    let acquired: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let mut progress: Vec<(RefreshPhase, String)> = Vec::new();
    let report = refresh_managed_skills_with(
        &f.paths,
        &f.store,
        RefreshSelection::All,
        None,
        |p| progress.push((p.phase, p.skill_name.to_string())),
        &|skill_id, cancel| {
            acquired.lock().unwrap().push(skill_id.to_string());
            crate::core::skill_update::acquire_update(
                &f.paths,
                &f.store,
                skill_id,
                cancel,
                &crate::core::git_acquisition::HttpGithubApi::new(None),
                0,
            )
        },
    )
    .expect("refresh");

    let healthy = by_name("s0");
    assert_eq!(
        acquired.into_inner().unwrap(),
        vec![healthy.id.clone()],
        "only the healthy skill reaches the acquire pool"
    );
    assert!(
        progress.iter().all(|(_, name)| name == "s0"),
        "progress ticks name the batch's members only: {progress:?}"
    );
    assert_eq!(ids.len(), 3);
    assert_eq!(
        report.skills.len(),
        3,
        "every skill is accounted for: {report:?}"
    );
    let status_of = |name: &str| {
        &report
            .skills
            .iter()
            .find(|o| o.skill_name == name)
            .unwrap_or_else(|| panic!("{name} is in the report: {report:?}"))
            .status
    };
    assert!(matches!(
        status_of("s0"),
        SkillRefreshStatus::Refreshed { .. }
    ));
    assert!(matches!(
        status_of("s1"),
        SkillRefreshStatus::Skipped {
            reason: RefreshSkipReason::Unlocatable {
                state: UnlocatableState::SourceMissing
            }
        }
    ));
    assert!(matches!(
        status_of("s2"),
        SkillRefreshStatus::Skipped {
            reason: RefreshSkipReason::Unlocatable {
                state: UnlocatableState::CentralMissing
            }
        }
    ));
    for name in ["s1", "s2"] {
        assert!(
            f.store
                .list_skill_targets(&by_name(name).id)
                .expect("query")
                .is_empty(),
            "no target is minted for a skipped skill ({name})"
        );
    }
    assert!(!f.paths.roots.home.join(".claude/skills/s2").exists());
}
