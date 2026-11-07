//! The install batch (ticket 04, D12/D20): one report per Add action,
//! collisions collected (never asked, never written through), retry through
//! the global sync batch with per-pair decisions, cancel between candidates.

use crate::core::environment::HostRoots;
use crate::core::removal_fence::central_repo_of;
use std::fs;
use std::path::Path;

use super::{
    install_selection_with, InstallBatchReport, InstallCandidate, InstallCandidateStatus,
    InstallOverride, InstallRequest, InstallSource,
};
use crate::core::cancel_token::CancelToken;
use crate::core::errors::CommandError;
use crate::core::git_acquisition::{GitSourceResolution, GithubApi, GithubCoords, GithubRepo};
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
        roots: HostRoots::under_home(&dir.path().join("home")),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.roots.home).unwrap();
    mark_installed_in(&paths.roots, claude());
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
    let target = skills_dir_in(&f.paths.roots, claude()).join("beta");
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
        &f.paths.roots,
        &f.store,
        &central_repo_of(&f.store),
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
                path: None,
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
        !skills_dir_in(&f.paths.roots, claude())
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

/// The GitHub adapter as acquisition calls it, recorded: the stub's bytes,
/// plus every call in order (`refs:` / `sha:` / `dl:`).
struct RecordingApi {
    inner: StubApi,
    calls: std::cell::RefCell<Vec<String>>,
}

impl RecordingApi {
    fn new() -> Self {
        Self {
            inner: StubApi::serving(&"a".repeat(40)),
            calls: Default::default(),
        }
    }
}

impl GithubApi for RecordingApi {
    fn matching_refs(&self, repo: &GithubRepo, prefix: &str) -> anyhow::Result<Vec<String>> {
        self.calls
            .borrow_mut()
            .push(format!("refs:{}/{}:{prefix}", repo.owner, repo.repo));
        self.inner.matching_refs(repo, prefix)
    }

    fn branch_sha(&self, coords: &GithubCoords) -> anyhow::Result<String> {
        self.calls.borrow_mut().push(format!(
            "sha:{}/{}@{}",
            coords.owner, coords.repo, coords.branch
        ));
        self.inner.branch_sha(coords)
    }

    fn download_directory(
        &self,
        coords: &GithubCoords,
        dest: &Path,
        cancel: Option<&CancelToken>,
    ) -> anyhow::Result<()> {
        self.calls
            .borrow_mut()
            .push(format!("dl:{}:{}", coords.branch, coords.subpath));
        self.inner.download_directory(coords, dest, cancel)
    }
}

/// #99: the listing's resolution is the install's source decision. The URL's
/// tree split (`feature/x/skills`) is ambiguous; the listing resolved it, so
/// the install reuses that branch and makes no second refs lookup.
#[test]
fn a_git_source_installs_through_the_listing_resolution() {
    let f = fixture();
    let req = InstallRequest {
        source: InstallSource::Git {
            url: "https://github.com/owner/repo/tree/feature/x/skills".into(),
            resolution: Some(GitSourceResolution {
                branch: Some("feature/x".into()),
                subpath: Some("skills".into()),
            }),
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
    let api = RecordingApi::new();
    let report = install_selection_with(&f.paths, &f.store, &req, &api, None, 1000, |_| {});
    assert!(
        matches!(
            target_status(&report, "fast-path-skill"),
            BatchTargetStatus::Synced { .. }
        ),
        "{report:?}"
    );
    assert_eq!(
        api.calls.borrow().as_slice(),
        [
            "sha:owner/repo@feature/x",
            "dl:feature/x:skills/fast-path-skill"
        ],
        "the resolved branch, no refs lookup"
    );
    let skill = f
        .store
        .get_skill_by_id(&skill_id(&report, "fast-path-skill"))
        .unwrap()
        .unwrap();
    assert_eq!(skill.source_type, "git");
    assert_eq!(
        skill.source_subpath.as_deref(),
        Some("skills/fast-path-skill")
    );
    assert_eq!(skill.source_revision, Some("a".repeat(40)));
}

/// A folder at Claude Code's `alpha` target with exactly `alpha`'s bytes.
fn occupy_alpha_with_its_own_bytes(f: &Fixture) -> std::path::PathBuf {
    let target = skills_dir_in(&f.paths.roots, claude()).join("alpha");
    fs::create_dir_all(&target).unwrap();
    fs::copy(
        f.source.join("alpha").join("SKILL.md"),
        target.join("SKILL.md"),
    )
    .unwrap();
    target
}

/// #99: identical content in the way is replaced silently in the first pass
/// when the request says so (every Add call site does): no ask, the pair
/// synced and owned.
#[test]
fn same_content_in_the_way_is_replaced_in_the_first_pass() {
    let f = fixture();
    let occupied = occupy_alpha_with_its_own_bytes(&f);
    let report = run(&f, &request(&f, &["alpha"]), None);

    assert!(
        matches!(
            target_status(&report, "alpha"),
            BatchTargetStatus::Synced { .. }
        ),
        "{report:?}"
    );
    let row = f
        .store
        .get_skill_target(&skill_id(&report, "alpha"), "claude_code")
        .unwrap()
        .expect("the pair is recorded");
    assert_eq!(row.status, SyncStatus::Synced);
    assert!(row.owns_artifact());
    assert!(fs::read_to_string(occupied.join("SKILL.md"))
        .unwrap()
        .contains("body alpha"));
}

/// The same folder without the same-content policy is the askable
/// collision, and its row says the content is the same.
#[test]
fn same_content_in_the_way_without_the_policy_is_collected_as_same_content() {
    let f = fixture();
    let occupied = occupy_alpha_with_its_own_bytes(&f);
    let mut req = request(&f, &["alpha"]);
    req.overwrite_if_same_content = false;
    let report = run(&f, &req, None);

    match target_status(&report, "alpha") {
        BatchTargetStatus::Failed {
            error: CommandError::TargetExists {
                path, same_content, ..
            },
        } => {
            assert_eq!(Path::new(path), occupied);
            assert!(same_content);
        }
        other => panic!("{other:?}"),
    }
    assert!(
        fs::symlink_metadata(&occupied).unwrap().is_dir(),
        "untouched"
    );
    assert!(f
        .store
        .get_skill_target(&skill_id(&report, "alpha"), "claude_code")
        .unwrap()
        .is_none());
}

/// #99: request overrides that match no collided pair of this pass settle
/// nothing — one for another Tool (not sharing Claude Code's dir) does not
/// overwrite Claude Code's collision, one for a skill outside the batch
/// records nothing, and one for a pair with nothing in the way changes
/// nothing about its first sync.
#[test]
fn extraneous_request_overrides_settle_nothing() {
    let f = fixture();
    let occupied = occupy_beta(&f);
    let mut req = request(&f, &["alpha", "beta"]);
    req.overrides = vec![
        InstallOverride {
            skill_name: "beta".into(),
            tool: "cursor".into(),
            decision: OverrideDecision::Overwrite,
        },
        InstallOverride {
            skill_name: "gamma".into(),
            tool: "claude_code".into(),
            decision: OverrideDecision::Keep,
        },
        InstallOverride {
            skill_name: "alpha".into(),
            tool: "claude_code".into(),
            decision: OverrideDecision::Overwrite,
        },
    ];
    let report = run(&f, &req, None);

    assert!(matches!(
        target_status(&report, "alpha"),
        BatchTargetStatus::Synced { .. }
    ));
    assert!(
        matches!(
            target_status(&report, "beta"),
            BatchTargetStatus::Failed {
                error: CommandError::TargetExists { .. }
            }
        ),
        "{report:?}"
    );
    assert_eq!(
        fs::read_to_string(occupied.join("SKILL.md")).unwrap(),
        "---\nname: beta\n---\nmine\n"
    );
    let beta = skill_id(&report, "beta");
    assert!(f.store.list_skill_targets(&beta).unwrap().is_empty());
    let names: Vec<String> = f
        .store
        .list_skills()
        .unwrap()
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert!(!names.contains(&"gamma".to_string()), "{names:?}");
}

/// #99: a folder that is gone (or unreadable) by the time the collision is
/// observed has no known modification time and is not the same content;
/// the row says so on the wire as `modifiedAt: null`, never a made-up time.
#[test]
fn an_occupant_whose_mtime_cannot_be_read_is_reported_unknown() {
    use crate::core::content_identity::Source;
    use crate::core::sync_engine::Occupant;

    let f = fixture();
    let vanished = skills_dir_in(&f.paths.roots, claude()).join("vanished");
    let occupant = Occupant::observe(Source::Directory(&f.source.join("alpha")), &vanished);
    assert_eq!(
        occupant,
        Occupant {
            modified_at: None,
            same_content: false,
        }
    );

    let row = CommandError::TargetExists {
        path: vanished.to_string_lossy().into_owned(),
        modified_at: occupant.modified_at,
        same_content: occupant.same_content,
    };
    let json = serde_json::to_value(&row).unwrap();
    assert_eq!(json["code"], "TARGET_EXISTS");
    assert_eq!(json["modifiedAt"], serde_json::Value::Null);
    assert!(json.as_object().unwrap().contains_key("modifiedAt"));
    assert_eq!(json["sameContent"], false);
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

/// The stub's bytes with its SKILL.md replaced: a repository whose manifest
/// says what the test needs it to.
struct ManifestApi {
    inner: StubApi,
    manifest: &'static str,
}

impl GithubApi for ManifestApi {
    fn matching_refs(&self, repo: &GithubRepo, prefix: &str) -> anyhow::Result<Vec<String>> {
        self.inner.matching_refs(repo, prefix)
    }

    fn branch_sha(&self, coords: &GithubCoords) -> anyhow::Result<String> {
        self.inner.branch_sha(coords)
    }

    fn download_directory(
        &self,
        coords: &GithubCoords,
        dest: &Path,
        cancel: Option<&CancelToken>,
    ) -> anyhow::Result<()> {
        self.inner.download_directory(coords, dest, cancel)?;
        fs::write(dest.join("SKILL.md"), self.manifest)?;
        Ok(())
    }
}

fn git_request(subpath: &str, name_override: Option<&str>) -> InstallRequest {
    InstallRequest {
        source: InstallSource::Git {
            url: "https://github.com/owner/repo".into(),
            resolution: Some(GitSourceResolution {
                branch: Some("main".into()),
                subpath: None,
            }),
        },
        candidates: vec![InstallCandidate {
            subpath: subpath.into(),
            name_override: name_override.map(str::to_string),
        }],
        tools: vec!["claude_code".into()],
        overwrite_if_same_content: true,
        overrides: Vec::new(),
        deploy: true,
    }
}

/// What the fixture's root holds besides the Library: nothing of a skill
/// ever lands beside it.
fn outside_the_library(f: &Fixture) -> Vec<String> {
    let mut entries: Vec<String> = fs::read_dir(f._dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| !["central", "cache", "home", "source", "test.db"].contains(&name.as_str()))
        .collect();
    entries.sort();
    entries
}

fn refused_name(report: &InstallBatchReport) -> Option<&str> {
    match &report.candidates[0].status {
        InstallCandidateStatus::Failed {
            error: CommandError::SkillNameInvalid { name },
        } => Some(name),
        _ => None,
    }
}

/// Round 20 ticket 19: a name typed in the Add flow that is not one folder
/// name is the typed refusal, raised before any acquisition — no GitHub call,
/// no clone cache, no staging, no record.
#[test]
fn a_requested_name_that_is_not_one_folder_name_is_refused_before_download() {
    // Drive-relative and drive-absolute spellings only mean something there.
    let windows: &[&str] = if cfg!(windows) {
        &["C:x", "C:\\x"]
    } else {
        &[]
    };
    let bad = ["../x", "../../x", "a/b", "a\\b", "..", ""];
    for &name in bad.iter().chain(windows) {
        let f = fixture();
        let api = RecordingApi::new();
        let report = install_selection_with(
            &f.paths,
            &f.store,
            &git_request("skills/alpha", Some(name)),
            &api,
            None,
            1000,
            |_| {},
        );
        assert_eq!(refused_name(&report), Some(name), "{name:?}: {report:?}");
        assert!(api.calls.borrow().is_empty(), "{name:?}: nothing acquired");
        assert!(!f.paths.cache_dir.exists(), "{name:?}: no clone cache");
        assert_eq!(
            fs::read_dir(&f.paths.central_dir).unwrap().count(),
            0,
            "{name:?}: no staging, no copy"
        );
        assert!(outside_the_library(&f).is_empty(), "{name:?}");
        assert!(f.store.list_skills().unwrap().is_empty());

        // A local source: refused before anything is copied.
        let mut local = request(&f, &["alpha"]);
        local.candidates[0].name_override = Some(name.to_string());
        let report = run(&f, &local, None);
        assert_eq!(refused_name(&report), Some(name), "{name:?}: {report:?}");
        assert_eq!(fs::read_dir(&f.paths.central_dir).unwrap().count(), 0);
        assert!(outside_the_library(&f).is_empty(), "{name:?}");
    }
}

/// A repository whose SKILL.md says `name: ../../x` installs under the
/// derived name (the subpath's leaf), inside the Library.
#[test]
fn a_manifest_name_that_is_not_one_folder_name_installs_under_the_derived_name() {
    let f = fixture();
    let api = ManifestApi {
        inner: StubApi::serving(&"a".repeat(40)),
        manifest: "---\nname: ../../x\ndescription: hostile\n---\n",
    };
    let report = install_selection_with(
        &f.paths,
        &f.store,
        &git_request("skills/derived", None),
        &api,
        None,
        1000,
        |_| {},
    );
    let InstallCandidateStatus::Installed { skill, .. } = &report.candidates[0].status else {
        panic!("{report:?}");
    };
    assert_eq!(skill.name, "derived");
    assert_eq!(skill.central_path, f.paths.central_dir.join("derived"));
    assert!(outside_the_library(&f).is_empty());
    assert!(matches!(
        target_status(&report, "derived"),
        BatchTargetStatus::Synced { .. }
    ));
}

/// A local folder whose SKILL.md name is not one folder name installs under
/// the folder's own name, as an unnamed local install does.
#[test]
fn a_local_manifest_name_that_is_not_one_folder_name_installs_under_the_folder_name() {
    let f = fixture();
    let hostile = f.source.join("hostile");
    fs::create_dir_all(&hostile).unwrap();
    fs::write(hostile.join("SKILL.md"), "---\nname: ../../x\n---\nbody\n").unwrap();
    let report = run(&f, &request(&f, &["hostile"]), None);
    let InstallCandidateStatus::Installed { skill, .. } = &report.candidates[0].status else {
        panic!("{report:?}");
    };
    assert_eq!(skill.name, "hostile");
    assert_eq!(skill.central_path, f.paths.central_dir.join("hostile"));
    assert!(outside_the_library(&f).is_empty());
}
