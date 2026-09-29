//! The install batch (ticket 04, D12/D20): one report per Add action,
//! collisions collected (never asked, never written through), retry through
//! the global sync batch with per-pair decisions, cancel between candidates.

use std::fs;
use std::path::Path;

use super::{
    install_selection_with, InstallBatchReport, InstallCandidate, InstallCandidateStatus,
    InstallOverride, InstallRequest, InstallSource,
};
use crate::core::cancel_token::CancelToken;
use crate::core::errors::CommandError;
use crate::core::global_sync::{
    sync_skills_to_tools, BatchOverride, BatchPolicy, BatchSkill, BatchTargetStatus,
    OverrideDecision,
};
use crate::core::installer::InstallerPaths;
use crate::core::skill_store::SkillStore;
use crate::core::sync_status::SyncStatus;
use crate::core::test_git_api::StubApi;
use crate::core::tool_adapters::{adapter_by_key, mark_installed_in, skills_dir_in, ToolAdapter};

struct Fixture {
    _dir: tempfile::TempDir,
    paths: InstallerPaths,
    store: SkillStore,
    source: std::path::PathBuf,
}

fn claude() -> &'static ToolAdapter {
    adapter_by_key("claude_code").expect("claude_code adapter")
}

/// A home with Claude Code installed and a local source folder holding
/// `alpha`, `beta` and `gamma`.
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = InstallerPaths {
        home: dir.path().join("home"),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.home).unwrap();
    mark_installed_in(&paths.home, claude());
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().unwrap();
    let source = dir.path().join("source");
    for name in ["alpha", "beta", "gamma"] {
        let skill = source.join(name);
        fs::create_dir_all(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: {name} skill\n---\nbody {name}\n"),
        )
        .unwrap();
    }
    Fixture {
        _dir: dir,
        paths,
        store,
        source,
    }
}

fn request(f: &Fixture, names: &[&str]) -> InstallRequest {
    InstallRequest {
        source: InstallSource::Local {
            root: f.source.to_string_lossy().into_owned(),
        },
        candidates: names
            .iter()
            .map(|name| InstallCandidate {
                subpath: (*name).to_string(),
                name_override: None,
            })
            .collect(),
        tools: vec!["claude_code".to_string()],
        overwrite_if_same_content: true,
        overrides: Vec::new(),
        deploy: true,
    }
}

/// A foreign folder at Claude Code's `beta` target, with other content.
fn occupy_beta(f: &Fixture) -> std::path::PathBuf {
    let target = skills_dir_in(&f.paths.home, claude()).join("beta");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("SKILL.md"), "---\nname: beta\n---\nmine\n").unwrap();
    target
}

fn run(f: &Fixture, request: &InstallRequest, cancel: Option<&CancelToken>) -> InstallBatchReport {
    install_selection_with(
        &f.paths,
        &f.store,
        request,
        &StubApi::serving(&"a".repeat(40)),
        cancel,
        1000,
        |_| {},
    )
}

fn target_status<'a>(report: &'a InstallBatchReport, name: &str) -> &'a BatchTargetStatus {
    let outcome = report.candidates.iter().find(|c| c.name == name).unwrap();
    let InstallCandidateStatus::Installed { targets, .. } = &outcome.status else {
        panic!("{name}: {:?}", outcome.status);
    };
    assert_eq!(targets.len(), 1, "{targets:?}");
    &targets[0].status
}

fn skill_id(report: &InstallBatchReport, name: &str) -> String {
    let outcome = report.candidates.iter().find(|c| c.name == name).unwrap();
    let InstallCandidateStatus::Installed { skill, .. } = &outcome.status else {
        panic!("{name}: {:?}", outcome.status);
    };
    skill.skill_id.clone()
}

#[test]
fn a_batch_of_three_with_one_collision_answers_one_report_and_writes_nothing_through_it() {
    let f = fixture();
    let occupied = occupy_beta(&f);
    let report = run(&f, &request(&f, &["alpha", "beta", "gamma"]), None);

    assert_eq!(report.candidates.len(), 3);
    assert!(matches!(
        target_status(&report, "alpha"),
        BatchTargetStatus::Synced { .. }
    ));
    assert!(matches!(
        target_status(&report, "gamma"),
        BatchTargetStatus::Synced { .. }
    ));
    // The collision is collected with what the ask shows, never asked here.
    match target_status(&report, "beta") {
        BatchTargetStatus::Failed {
            error:
                CommandError::TargetExists {
                    path,
                    modified_at,
                    same_content,
                },
        } => {
            assert_eq!(Path::new(path), occupied);
            assert!(modified_at.is_some());
            assert!(!same_content);
        }
        other => panic!("{other:?}"),
    }
    // Every candidate is installed; the collided target is untouched and
    // has no row (asked first, D9).
    for name in ["alpha", "beta", "gamma"] {
        assert!(f.paths.central_dir.join(name).join("SKILL.md").exists());
    }
    assert!(fs::symlink_metadata(&occupied).unwrap().is_dir());
    assert_eq!(
        fs::read_to_string(occupied.join("SKILL.md")).unwrap(),
        "---\nname: beta\n---\nmine\n"
    );
    let beta = skill_id(&report, "beta");
    assert!(f
        .store
        .get_skill_target(&beta, "claude_code")
        .unwrap()
        .is_none());
}

/// The retry decision: the skills exist after the first pass, so the ask's
/// answer is an ordinary `sync_skills_to_tools` batch with per-pair
/// overrides over the collided pairs only.
fn retry(f: &Fixture, skill_id: &str, name: &str, decision: OverrideDecision) -> BatchTargetStatus {
    let mut rows = sync_skills_to_tools(
        &f.paths.home,
        &f.store,
        &[BatchSkill {
            skill_id: skill_id.to_string(),
            skill_name: name.to_string(),
            source_path: f.paths.central_dir.join(name),
        }],
        &["claude_code".to_string()],
        &BatchPolicy {
            overwrite: false,
            overwrite_if_same_content: true,
            overrides: vec![BatchOverride {
                skill_id: skill_id.to_string(),
                tool_key: "claude_code".to_string(),
                decision,
            }],
        },
        2000,
        |_| {},
    );
    assert_eq!(rows.len(), 1);
    rows.remove(0).status
}

#[test]
fn retrying_the_collision_with_overwrite_replaces_the_folder() {
    let f = fixture();
    let occupied = occupy_beta(&f);
    let report = run(&f, &request(&f, &["alpha", "beta", "gamma"]), None);
    let beta = skill_id(&report, "beta");

    assert!(matches!(
        retry(&f, &beta, "beta", OverrideDecision::Overwrite),
        BatchTargetStatus::Synced { .. }
    ));
    assert!(fs::read_to_string(occupied.join("SKILL.md"))
        .unwrap()
        .contains("body beta"));
    let row = f
        .store
        .get_skill_target(&beta, "claude_code")
        .unwrap()
        .unwrap();
    assert_eq!(row.status, SyncStatus::Synced);
}

#[test]
fn retrying_the_collision_with_keep_records_the_pair_disabled() {
    let f = fixture();
    let occupied = occupy_beta(&f);
    let report = run(&f, &request(&f, &["alpha", "beta", "gamma"]), None);
    let beta = skill_id(&report, "beta");

    assert!(matches!(
        retry(&f, &beta, "beta", OverrideDecision::Keep),
        BatchTargetStatus::Declined
    ));
    assert!(fs::read_to_string(occupied.join("SKILL.md"))
        .unwrap()
        .contains("mine"));
    let row = f
        .store
        .get_skill_target(&beta, "claude_code")
        .unwrap()
        .unwrap();
    assert_eq!(row.status, SyncStatus::Disabled);
    assert!(!row.owns_artifact());
}

#[test]
fn a_request_override_settles_the_collision_in_the_first_pass() {
    let f = fixture();
    let occupied = occupy_beta(&f);
    let mut req = request(&f, &["beta"]);
    req.overrides = vec![InstallOverride {
        skill_name: "beta".into(),
        tool: "claude_code".into(),
        decision: OverrideDecision::Overwrite,
    }];
    let report = run(&f, &req, None);
    assert!(matches!(
        target_status(&report, "beta"),
        BatchTargetStatus::Synced { .. }
    ));
    assert!(fs::read_to_string(occupied.join("SKILL.md"))
        .unwrap()
        .contains("body beta"));
}

#[test]
fn cancel_during_the_second_candidate_reports_the_rest_cancelled_and_finalizes_nothing_of_them() {
    let f = fixture();
    let cancel = CancelToken::new();
    let mut ticks = Vec::new();
    let report = install_selection_with(
        &f.paths,
        &f.store,
        &request(&f, &["alpha", "beta", "gamma"]),
        &StubApi::serving(&"a".repeat(40)),
        Some(&cancel),
        1000,
        |p| {
            ticks.push((p.index, p.total, p.name.to_string()));
            // The operator presses Cancel while candidate 2 is acquired.
            if p.index == 2 {
                cancel.cancel();
            }
        },
    );
    assert_eq!(
        ticks,
        vec![(1, 3, "alpha".to_string()), (2, 3, "beta".to_string())]
    );
    assert!(matches!(
        report.candidates[0].status,
        InstallCandidateStatus::Installed { .. }
    ));
    for outcome in &report.candidates[1..] {
        assert!(
            matches!(outcome.status, InstallCandidateStatus::Cancelled),
            "{outcome:?}"
        );
    }
    assert_eq!(report.candidates[1].name, "beta");
    assert_eq!(report.candidates[2].name, "gamma");
    // No partial finalize: beta's staged bytes were discarded, no record.
    assert!(!f.paths.central_dir.join("beta").exists());
    assert!(!f.paths.central_dir.join("gamma").exists());
    let names: Vec<String> = f
        .store
        .list_skills()
        .unwrap()
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(names, vec!["alpha".to_string()]);
    // Nothing left in the central repo's staging area either.
    let leftovers: Vec<_> = fs::read_dir(&f.paths.central_dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n != "alpha")
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

/// Review F5: a Cancel pressed while an acquired candidate waits for the
/// mutation guard (another mutation holds it) wins once the guard is held:
/// nothing is finalized, the staging is dropped, and that candidate and the
/// rest answer `Cancelled`.
#[test]
fn cancel_while_waiting_for_the_guard_finalizes_nothing() {
    use crate::core::mutation_guard::serialized;
    use std::time::{Duration, Instant};
    let f = fixture();
    let cancel = CancelToken::new();
    let req = request(&f, &["alpha", "beta"]);
    let (tx, rx) = std::sync::mpsc::channel();
    let report = std::thread::scope(|scope| {
        let handle = serialized(|| {
            let handle = scope.spawn(|| {
                install_selection_with(
                    &f.paths,
                    &f.store,
                    &req,
                    &StubApi::serving(&"a".repeat(40)),
                    Some(&cancel),
                    1000,
                    |p| {
                        let _ = tx.send(p.index);
                    },
                )
            });
            assert_eq!(rx.recv().expect("first candidate starts"), 1);
            // Wait until alpha is staged, then give it time to queue on the
            // guard this thread holds.
            let deadline = Instant::now() + Duration::from_secs(5);
            while fs::read_dir(&f.paths.central_dir)
                .map(|entries| entries.count() == 0)
                .unwrap_or(true)
            {
                assert!(Instant::now() < deadline, "alpha was never staged");
                std::thread::sleep(Duration::from_millis(5));
            }
            std::thread::sleep(Duration::from_millis(150));
            assert!(!handle.is_finished(), "the candidate waits for the guard");
            assert!(f.store.list_skills().unwrap().is_empty());
            cancel.cancel();
            handle
        });
        handle.join().expect("batch thread")
    });

    for (outcome, name) in report.candidates.iter().zip(["alpha", "beta"]) {
        assert_eq!(outcome.name, name);
        assert!(
            matches!(outcome.status, InstallCandidateStatus::Cancelled),
            "{outcome:?}"
        );
    }
    assert!(f.store.list_skills().unwrap().is_empty(), "no record");
    let leftovers: Vec<_> = fs::read_dir(&f.paths.central_dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(leftovers.is_empty(), "staging dropped: {leftovers:?}");
    assert!(
        !skills_dir_in(&f.paths.home, claude())
            .join("alpha")
            .exists(),
        "no target"
    );
}

#[test]
fn a_failed_candidate_is_report_data_and_the_rest_proceed() {
    let f = fixture();
    let report = run(&f, &request(&f, &["alpha", "missing", "gamma"]), None);
    assert!(matches!(
        report.candidates[1].status,
        InstallCandidateStatus::Failed {
            error: CommandError::SourcePathMissing { .. }
        }
    ));
    assert_eq!(report.candidates[1].name, "missing");
    assert!(matches!(
        target_status(&report, "alpha"),
        BatchTargetStatus::Synced { .. }
    ));
    assert!(matches!(
        target_status(&report, "gamma"),
        BatchTargetStatus::Synced { .. }
    ));
}

#[test]
fn deploy_off_installs_without_targets() {
    let f = fixture();
    let mut req = request(&f, &["alpha"]);
    req.deploy = false;
    let report = run(&f, &req, None);
    let InstallCandidateStatus::Installed { targets, .. } = &report.candidates[0].status else {
        panic!("{report:?}");
    };
    assert!(targets.is_empty());
    let alpha = skill_id(&report, "alpha");
    assert!(f.store.list_skill_targets(&alpha).unwrap().is_empty());
}

#[test]
fn a_git_source_installs_through_the_listing_resolution() {
    let f = fixture();
    let req = InstallRequest {
        source: InstallSource::Git {
            url: "https://github.com/owner/repo".into(),
            resolution: None,
        },
        candidates: vec![InstallCandidate {
            subpath: "skills/fast-path-skill".into(),
            name_override: None,
        }],
        tools: vec!["claude_code".into()],
        overwrite_if_same_content: true,
        overrides: Vec::new(),
        deploy: true,
    };
    let report = run(&f, &req, None);
    assert!(
        matches!(
            target_status(&report, "fast-path-skill"),
            BatchTargetStatus::Synced { .. }
        ),
        "{report:?}"
    );
    let skill = f
        .store
        .get_skill_by_id(&skill_id(&report, "fast-path-skill"))
        .unwrap()
        .unwrap();
    assert_eq!(skill.source_type, "git");
}

#[test]
fn the_report_crosses_the_wire_with_cancelled_as_its_own_kind() {
    let f = fixture();
    let cancel = CancelToken::new();
    cancel.cancel();
    let report = run(&f, &request(&f, &["alpha"]), Some(&cancel));
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "candidates": [
            { "subpath": "alpha", "name": "alpha", "status": { "status": "cancelled" } }
        ]})
    );
}
