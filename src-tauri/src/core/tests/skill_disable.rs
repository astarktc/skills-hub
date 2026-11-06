//! Tests for `core::skill_disable` — the durable "not on this Tool" at two
//! grains, and the skip every automatic pass owes it (Propagation, the
//! global sync batch, project sync).
//!
//! Every case runs against a temp home / central dir / DB; installedness is
//! faked through `mark_installed_in`. Nothing touches the operator's home.

use crate::core::environment::HostRoots;
use crate::core::removal_fence::central_repo_of;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::artifact_removal::{remove_skill, RowRef};
use crate::core::errors::CommandError;
use crate::core::global_sync::{
    sync_skills_to_tools, BatchPolicy, BatchSkill, BatchTargetOutcome, BatchTargetStatus,
};
use crate::core::installer::{install_local_skill, InstallerPaths};
use crate::core::onboarding::OnboardingScanScope;
use crate::core::onboarding_import::{
    import_onboarding_selection, ImportGroupStatus, ImportPolicy, ImportSelection,
};
use crate::core::project_sync::{
    self, assign_skill_to_project_tools, resync_all_projects, resync_project,
    unassign_skill_from_project, ProjectSyncOutcomeStatus, ProjectSyncReport, ProjectSyncSkip,
};
use crate::core::propagation::{
    propagate_unlocked, PropagationScope, PropagationSkip, PropagationStatus,
};
use crate::core::refresh::{refresh_managed_skills, RefreshSelection, SkillRefreshStatus};
use crate::core::skill_disable::{disable_targets, enable_targets, DisableScope, EnableReport};
use crate::core::skill_store::{ProjectRecord, ProjectToolRecord, SkillStore};
use crate::core::sync_status::SyncStatus;
use crate::core::tool_adapters::{adapter_by_key, mark_installed_in};

const NOW: i64 = 4000;

struct Fixture {
    _dir: tempfile::TempDir,
    paths: InstallerPaths,
    store: SkillStore,
    _source: tempfile::TempDir,
    skill_id: String,
}

impl Fixture {
    fn home(&self) -> &Path {
        &self.paths.roots.home
    }
    fn status(&self, tool: &str) -> Option<SyncStatus> {
        self.store
            .get_skill_target(&self.skill_id, tool)
            .expect("read target")
            .map(|row| row.status)
    }
    fn global_path(&self, rel: &str) -> PathBuf {
        self.home().join(rel).join("alpha")
    }
    fn pair(&self, tool: &str) -> DisableScope {
        DisableScope::SkillTool {
            skill_id: self.skill_id.clone(),
            tool: tool.to_string(),
        }
    }
    fn skill(&self) -> DisableScope {
        DisableScope::Skill {
            skill_id: self.skill_id.clone(),
        }
    }
    fn sync(&self, tools: &[&str]) -> Vec<BatchTargetOutcome> {
        let skill = self.store.get_skill_by_id(&self.skill_id).unwrap().unwrap();
        sync_skills_to_tools(
            &HostRoots::under_home(self.home()),
            &self.store,
            &central_repo_of(&self.store),
            &[BatchSkill {
                skill_id: skill.id,
                skill_name: skill.name,
                source_path: PathBuf::from(skill.central_path),
            }],
            &tools.iter().map(|t| t.to_string()).collect::<Vec<_>>(),
            &BatchPolicy {
                overwrite: false,
                overwrite_if_same_content: true,
                overrides: Vec::new(),
            },
            NOW,
            |_| {},
        )
    }
    fn refresh_all(&self) -> Vec<(PropagationScope, PropagationStatus)> {
        let report = refresh_managed_skills(
            &self.paths,
            &self.store,
            &crate::core::credentials::MemoryStore::new(),
            RefreshSelection::All,
            None,
            |_| {},
        )
        .expect("refresh");
        match report.skills.into_iter().next().expect("one skill").status {
            SkillRefreshStatus::Refreshed { targets, .. } => targets
                .into_iter()
                .map(|outcome| (outcome.scope, outcome.status))
                .collect(),
            other => panic!("expected refreshed, got {other:?}"),
        }
    }
}

/// One local-source Managed skill `alpha`, with the given Tools installed.
fn fixture(tools: &[&str]) -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = InstallerPaths {
        roots: HostRoots::under_home(&dir.path().join("home")),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.roots.home).expect("home");
    for tool in tools {
        mark_installed_in(&paths.roots, adapter_by_key(tool).expect("adapter"));
    }
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("schema");
    let source = tempfile::tempdir().expect("source");
    fs::write(source.path().join("SKILL.md"), "---\nname: alpha\n---\n").unwrap();
    let installed =
        install_local_skill(&paths, &store, source.path(), Some("alpha".into())).expect("install");
    Fixture {
        _dir: dir,
        paths,
        store,
        _source: source,
        skill_id: installed.skill_id,
    }
}

fn project(f: &Fixture, tool: &str) -> ProjectRecord {
    let path = f._dir.path().join("proj");
    fs::create_dir_all(&path).unwrap();
    let project = ProjectRecord {
        id: "p1".into(),
        path: path.to_string_lossy().into_owned(),
        created_at: 1,
        updated_at: 1,
    };
    f.store.register_project(&project).unwrap();
    f.store
        .add_project_tool(&ProjectToolRecord {
            id: format!("pt-{tool}"),
            project_id: project.id.clone(),
            tool: tool.into(),
        })
        .unwrap();
    project
}

fn skipped_disabled(outcome: &BatchTargetOutcome) -> bool {
    matches!(
        &outcome.status,
        BatchTargetStatus::Skipped { error: CommandError::TargetDisabled { tool } }
            if *tool == outcome.tool_key
    )
}

fn exists_any(path: &Path) -> bool {
    path.symlink_metadata().is_ok()
}

/// Bytes the app never wrote, at `path`.
fn foreign_folder(path: &Path) {
    fs::create_dir_all(path).unwrap();
    fs::write(path.join("SKILL.md"), "someone else's").unwrap();
}

fn foreign_bytes_survive(path: &Path) -> bool {
    fs::read_to_string(path.join("SKILL.md")).ok().as_deref() == Some("someone else's")
}

/// Make `dir` read-only so removing an entry inside it fails. `false` when
/// permissions are not enforced (running as root): the caller returns early.
#[cfg(unix)]
fn lock_dir(dir: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o555)).unwrap();
    let probe = dir.join("probe");
    if fs::create_dir(&probe).is_ok() {
        fs::remove_dir(&probe).unwrap();
        unlock_dir(dir);
        return false;
    }
    true
}

#[cfg(unix)]
fn unlock_dir(dir: &Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o755)).unwrap();
}

fn is_disabled_skip(status: &ProjectSyncOutcomeStatus) -> bool {
    matches!(
        status,
        ProjectSyncOutcomeStatus::Skipped {
            reason: ProjectSyncSkip::Disabled
        }
    )
}

fn item_for<'a>(
    report: &'a ProjectSyncReport,
    skill_id: &str,
) -> &'a project_sync::ProjectSyncOutcome {
    report
        .items
        .iter()
        .find(|item| item.skill_id == skill_id)
        .expect("item for skill")
}

// ---------------------------------------------------------------------------
// Per (skill, Tool)
// ---------------------------------------------------------------------------

#[test]
fn disabling_a_pair_removes_the_artifact_and_keeps_the_row_disabled() {
    let f = fixture(&["claude_code", "codex"]);
    f.sync(&["claude_code", "codex"]);
    let target = f.global_path(".claude/skills");
    assert!(exists_any(&target));

    let report = disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("claude_code"),
        NOW,
    )
    .unwrap();

    assert_eq!((report.removed_rows(), report.failed_rows()), (1, 0));
    assert!(!exists_any(&target), "artifact gone");
    assert_eq!(f.status("claude_code"), Some(SyncStatus::Disabled));
    assert_eq!(
        f.status("codex"),
        Some(SyncStatus::Synced),
        "other Tools untouched"
    );
    assert_eq!(f.store.skill_disabled_at(&f.skill_id).unwrap(), None);
}

#[test]
fn a_disabled_pair_survives_refresh_and_propagation() {
    let f = fixture(&["claude_code", "codex"]);
    f.sync(&["claude_code", "codex"]);
    disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("claude_code"),
        NOW,
    )
    .unwrap();

    let targets = f.refresh_all();

    assert!(
        targets.iter().any(|(scope, status)| {
            *scope
                == PropagationScope::Global {
                    tool: "claude_code".into(),
                }
                && matches!(
                    status,
                    PropagationStatus::Skipped {
                        reason: PropagationSkip::Disabled
                    }
                )
        }),
        "reported skipped as disabled: {targets:?}"
    );
    assert!(
        !targets
            .iter()
            .any(|(_, status)| matches!(status, PropagationStatus::Failed { .. })),
        "never a failure: {targets:?}"
    );
    assert!(!exists_any(&f.global_path(".claude/skills")));
    assert_eq!(f.status("claude_code"), Some(SyncStatus::Disabled));

    // Propagation on its own (the Update / Edit path) says the same.
    let report = propagate_unlocked(&f.store, &f.paths, &f.skill_id, NOW + 2).unwrap();
    assert!(report.targets.iter().any(|t| matches!(
        t.status,
        PropagationStatus::Skipped {
            reason: PropagationSkip::Disabled
        }
    )));
    assert!(!exists_any(&f.global_path(".claude/skills")));
}

#[test]
fn disabling_a_pair_with_no_row_records_a_marker_a_refresh_honours() {
    let f = fixture(&["claude_code"]);
    assert_eq!(f.status("claude_code"), None);

    let report = disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("claude_code"),
        NOW,
    )
    .unwrap();
    assert!(report.targets.is_empty(), "nothing to remove");
    assert_eq!(f.status("claude_code"), Some(SyncStatus::Disabled));
    // #95: the recorded intent is report data, so the fold can say so.
    assert!(
        matches!(
            report.disabled_markers.as_slice(),
            [RowRef::GlobalTarget { skill_id, tool, .. }]
                if *skill_id == f.skill_id && tool == "claude_code"
        ),
        "{:?}",
        report.disabled_markers
    );
    assert!(report.cleared_markers.is_empty());

    let again = disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("claude_code"),
        NOW,
    )
    .unwrap();
    assert!(
        again.targets.is_empty() && again.disabled_markers.is_empty(),
        "a second Disable records nothing new: {:?}",
        again.disabled_markers
    );

    f.refresh_all();
    assert!(!exists_any(&f.global_path(".claude/skills")));
    assert_eq!(f.status("claude_code"), Some(SyncStatus::Disabled));
}

#[test]
fn enabling_a_pair_resyncs_it_as_a_first_sync() {
    let f = fixture(&["claude_code"]);
    f.sync(&["claude_code"]);
    disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("claude_code"),
        NOW,
    )
    .unwrap();

    let report = enable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("claude_code"),
        NOW,
    )
    .unwrap();

    assert!(matches!(
        report.global.as_slice(),
        [outcome] if matches!(outcome.status, BatchTargetStatus::Synced { .. })
    ));
    assert!(report.projects.items.is_empty());
    assert!(exists_any(&f.global_path(".claude/skills")));
    assert_eq!(f.status("claude_code"), Some(SyncStatus::Synced));
}

#[test]
fn enable_reports_a_directory_in_the_way_as_target_exists_data() {
    let f = fixture(&["claude_code"]);
    disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("claude_code"),
        NOW,
    )
    .unwrap();
    let foreign = f.global_path(".claude/skills");
    fs::create_dir_all(&foreign).unwrap();
    fs::write(foreign.join("SKILL.md"), "someone else's").unwrap();

    let report = enable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("claude_code"),
        NOW,
    )
    .unwrap();

    assert!(matches!(
        report.global.as_slice(),
        [outcome] if matches!(
            outcome.status,
            BatchTargetStatus::Failed { error: CommandError::TargetExists { .. } }
        )
    ));
    assert_eq!(
        fs::read_to_string(foreign.join("SKILL.md")).unwrap(),
        "someone else's",
        "never clobbered"
    );
}

#[test]
fn disabling_twice_settles_nothing_new() {
    let f = fixture(&["claude_code"]);
    f.sync(&["claude_code"]);
    disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("claude_code"),
        NOW,
    )
    .unwrap();

    let again = disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("claude_code"),
        NOW,
    )
    .unwrap();
    assert!(again.targets.is_empty());
    assert!(again.disabled_markers.is_empty(), "an owner is no marker");
    assert_eq!(f.status("claude_code"), Some(SyncStatus::Disabled));
}

/// A shared skills dir is one artifact: while one member is disabled, the
/// batch never writes it for a sibling that has no row of its own.
#[test]
fn a_shared_dir_sibling_is_skipped_while_a_member_is_disabled() {
    let f = fixture(&["amp", "kimi_cli"]);
    disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("amp"),
        NOW,
    )
    .unwrap();
    assert_eq!(f.status("kimi_cli"), None);

    let outcomes = f.sync(&["kimi_cli"]);

    assert!(
        matches!(outcomes.as_slice(), [o] if skipped_disabled(o)),
        "{outcomes:?}"
    );
    assert!(!exists_any(&f.global_path(".config/agents/skills")));
}

#[test]
fn an_unknown_tool_with_no_row_is_a_typed_refusal() {
    let f = fixture(&[]);
    let err = disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("no_such_tool"),
        NOW,
    )
    .unwrap_err();
    assert!(matches!(
        CommandError::from_anyhow(err),
        CommandError::UnknownTool { .. }
    ));
}

/// ADR-0002 holds for Disable: a row whose artifact could not be removed is
/// kept `error`, never marked `disabled` with its artifact still there.
#[cfg(unix)]
#[test]
fn a_failed_removal_keeps_the_row_error_not_disabled() {
    use std::os::unix::fs::PermissionsExt;
    let f = fixture(&["claude_code"]);
    f.sync(&["claude_code"]);
    let parent = f.home().join(".claude/skills");
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o555)).unwrap();
    let probe = parent.join("probe");
    if fs::create_dir(&probe).is_ok() {
        // Running as root: permissions are not enforced.
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }

    let report = disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("claude_code"),
        NOW,
    )
    .unwrap();
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o755)).unwrap();

    assert_eq!((report.removed_rows(), report.failed_rows()), (0, 1));
    assert_eq!(f.status("claude_code"), Some(SyncStatus::Error));
    assert!(exists_any(&f.global_path(".claude/skills")));
}

// ---------------------------------------------------------------------------
// Per skill: global targets and project assignments
// ---------------------------------------------------------------------------

#[test]
fn disabling_a_skill_covers_global_targets_and_project_assignments() {
    let f = fixture(&["claude_code"]);
    f.sync(&["claude_code"]);
    let project = project(&f, "claude_code");
    assign_skill_to_project_tools(
        &f.store,
        &central_repo_of(&f.store),
        &project.id,
        &f.skill_id,
        NOW,
    )
    .unwrap();
    let project_target = Path::new(&project.path).join(".claude/skills/alpha");
    assert!(exists_any(&project_target));

    let report = disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.skill(),
        NOW,
    )
    .unwrap();

    assert_eq!((report.removed_rows(), report.failed_rows()), (2, 0));
    assert!(!report.record_deleted && !report.central_removed);
    assert!(!exists_any(&f.global_path(".claude/skills")));
    assert!(!exists_any(&project_target));
    assert_eq!(f.status("claude_code"), Some(SyncStatus::Disabled));
    let assignment = f
        .store
        .get_project_skill_assignment(&project.id, &f.skill_id, "claude_code")
        .unwrap()
        .expect("row kept");
    assert_eq!(assignment.status, SyncStatus::Disabled);
    assert_eq!(f.store.skill_disabled_at(&f.skill_id).unwrap(), Some(NOW));
}

#[test]
fn a_disabled_skill_is_never_rematerialised_by_any_automatic_pass() {
    let f = fixture(&["claude_code", "codex"]);
    f.sync(&["claude_code"]);
    let project = project(&f, "claude_code");
    assign_skill_to_project_tools(
        &f.store,
        &central_repo_of(&f.store),
        &project.id,
        &f.skill_id,
        NOW,
    )
    .unwrap();
    disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.skill(),
        NOW,
    )
    .unwrap();

    // Refresh: codex was never a target; it must not become one.
    let targets = f.refresh_all();
    assert!(
        targets.iter().all(|(_, status)| matches!(
            status,
            PropagationStatus::Skipped {
                reason: PropagationSkip::Disabled
            }
        )),
        "{targets:?}"
    );
    assert_eq!(f.status("codex"), None);
    assert!(!exists_any(&f.global_path(".codex/skills")));

    // The global batch (every operator sync and Onboarding import's first sync).
    let outcomes = f.sync(&["claude_code", "codex"]);
    assert!(outcomes.iter().all(skipped_disabled), "{outcomes:?}");

    // Project re-sync skips the disabled row.
    let resync = resync_project(&f.store, &central_repo_of(&f.store), &project.id, NOW).unwrap();
    assert!(matches!(
        resync.items.as_slice(),
        [item] if matches!(item.status, ProjectSyncOutcomeStatus::Skipped { reason: ProjectSyncSkip::Disabled })
    ));
    assert!(!exists_any(
        &Path::new(&project.path).join(".claude/skills/alpha")
    ));
}

/// Onboarding import's first sync is the gated global batch, and an import
/// always creates a *new* skill (the name must be free), so it can never carry
/// a disabled row of its own. The one way it could meet a Disable is an
/// unmanaged copy of a disabled skill reappearing in the Tool's dir. The
/// disabled row still records that path, so the plan never offers it: naming
/// it fails per group (even with Replace) and leaves the disabled pair and
/// the copy alone.
#[test]
fn onboarding_import_never_rematerialises_a_disabled_pair() {
    for replace in [false, true] {
        onboarding_import_leaves_a_disabled_pair(replace);
    }
}

fn onboarding_import_leaves_a_disabled_pair(replace: bool) {
    let f = fixture(&["claude_code"]);
    f.sync(&["claude_code"]);
    disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("claude_code"),
        NOW,
    )
    .unwrap();
    let reappeared = f.global_path(".claude/skills");
    fs::create_dir_all(&reappeared).unwrap();
    fs::write(
        reappeared.join("SKILL.md"),
        "---\nname: alpha\n---\nforeign\n",
    )
    .unwrap();

    let report = import_onboarding_selection(
        &f.paths,
        &f.store,
        &[ImportSelection {
            group_name: "alpha".into(),
            chosen_path: reappeared.clone(),
            name: None,
            replace,
            remove_divergent: false,
        }],
        &ImportPolicy {
            auto_sync: true,
            remove_originals: true,
            tools: Some(vec!["claude_code".into()]),
            scan_scope: OnboardingScanScope::Installed,
        },
        NOW,
        |_| {},
    )
    .expect("the import reads its own plan");

    assert!(
        matches!(
            report.groups.as_slice(),
            [group] if matches!(group.status, ImportGroupStatus::Failed { .. })
        ),
        "replace={replace}: {report:?}"
    );
    assert_eq!(f.status("claude_code"), Some(SyncStatus::Disabled));
    assert!(
        !reappeared
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink(),
        "the copy is left as found, never replaced by a link"
    );
    assert_eq!(f.store.list_skills().unwrap().len(), 1, "no second skill");
}

#[test]
fn a_new_assignment_of_a_disabled_skill_is_recorded_disabled_not_synced() {
    let f = fixture(&[]);
    disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.skill(),
        NOW,
    )
    .unwrap();
    let project = project(&f, "claude_code");

    let report = assign_skill_to_project_tools(
        &f.store,
        &central_repo_of(&f.store),
        &project.id,
        &f.skill_id,
        NOW,
    )
    .unwrap();

    assert!(matches!(
        report.items.as_slice(),
        [item] if item.assignment_id.is_some()
            && matches!(item.status, ProjectSyncOutcomeStatus::Skipped { reason: ProjectSyncSkip::Disabled })
    ));
    let row = f
        .store
        .get_project_skill_assignment(&project.id, &f.skill_id, "claude_code")
        .unwrap()
        .expect("intent recorded");
    assert_eq!(row.status, SyncStatus::Disabled);
    assert!(!exists_any(
        &Path::new(&project.path).join(".claude/skills/alpha")
    ));

    // The reconcile pass leaves it alone.
    let listing = project_sync::list_assignments_with_staleness(&f.store, &project.id).unwrap();
    assert_eq!(listing.assignments[0].status, SyncStatus::Disabled);
}

#[test]
fn enabling_a_skill_resyncs_every_disabled_row_and_clears_the_marker() {
    let f = fixture(&["claude_code"]);
    f.sync(&["claude_code"]);
    let project = project(&f, "claude_code");
    assign_skill_to_project_tools(
        &f.store,
        &central_repo_of(&f.store),
        &project.id,
        &f.skill_id,
        NOW,
    )
    .unwrap();
    disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.skill(),
        NOW,
    )
    .unwrap();

    let report: EnableReport = enable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.skill(),
        NOW,
    )
    .unwrap();

    assert!(matches!(
        report.global.as_slice(),
        [o] if o.tool_key == "claude_code" && matches!(o.status, BatchTargetStatus::Synced { .. })
    ));
    assert!(matches!(
        report.projects.items.as_slice(),
        [item] if matches!(item.status, ProjectSyncOutcomeStatus::Synced)
    ));
    assert_eq!(f.store.skill_disabled_at(&f.skill_id).unwrap(), None);
    assert_eq!(f.status("claude_code"), Some(SyncStatus::Synced));
    assert!(exists_any(&f.global_path(".claude/skills")));
    assert!(exists_any(
        &Path::new(&project.path).join(".claude/skills/alpha")
    ));
}

/// A project whose folder went away while the skill was disabled: Disable
/// still succeeded on it (removal is never gated by the folder), and Enable
/// skips the project once — no folder recreated, its row left `disabled` —
/// while the global target is re-synced.
#[test]
fn enabling_a_skill_skips_a_project_whose_folder_is_missing() {
    let f = fixture(&["claude_code"]);
    f.sync(&["claude_code"]);
    let project = project(&f, "claude_code");
    assign_skill_to_project_tools(
        &f.store,
        &central_repo_of(&f.store),
        &project.id,
        &f.skill_id,
        NOW,
    )
    .unwrap();
    let project_dir = PathBuf::from(&project.path);
    fs::remove_dir_all(&project_dir).unwrap();

    let disabled = disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.skill(),
        NOW,
    )
    .unwrap();
    assert!(disabled.failures().is_empty(), "{disabled}");
    let before = f
        .store
        .get_project_skill_assignment(&project.id, &f.skill_id, "claude_code")
        .unwrap()
        .expect("row");
    assert_eq!(before.status, SyncStatus::Disabled);

    let report: EnableReport = enable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.skill(),
        NOW + 1,
    )
    .unwrap();

    assert!(report.projects.items.is_empty(), "{}", report.projects);
    assert_eq!(
        report.projects.skipped_projects,
        vec![project_sync::ProjectSkipped {
            project_id: project.id.clone(),
            reason: project_sync::ProjectSkipReason::FolderMissing,
        }]
    );
    assert!(!project_dir.exists(), "the missing folder is not recreated");
    let after = f
        .store
        .get_project_skill_assignment(&project.id, &f.skill_id, "claude_code")
        .unwrap()
        .expect("row");
    assert_eq!(
        (after.status, after.synced_at, after.last_error),
        (before.status, before.synced_at, before.last_error),
        "row untouched (not admitted, not settled error)"
    );
    assert_eq!(f.status("claude_code"), Some(SyncStatus::Synced));

    // The Skills scope folds each skill's project skip once.
    disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.skill(),
        NOW + 2,
    )
    .unwrap();
    let scope = DisableScope::Skills {
        skill_ids: vec![f.skill_id.clone(), f.skill_id.clone()],
    };
    let both = enable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &scope,
        NOW + 3,
    )
    .unwrap();
    assert_eq!(both.projects.skipped_projects.len(), 1);
    assert!(!project_dir.exists());
}

/// Only a skill-grain Enable lifts a skill-grain Disable.
#[test]
fn enabling_a_pair_of_a_disabled_skill_is_skipped() {
    let f = fixture(&["claude_code"]);
    f.sync(&["claude_code"]);
    disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.skill(),
        NOW,
    )
    .unwrap();

    let report = enable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("claude_code"),
        NOW,
    )
    .unwrap();

    assert!(matches!(report.global.as_slice(), [o] if skipped_disabled(o)));
    assert_eq!(f.status("claude_code"), Some(SyncStatus::Disabled));
    assert!(!exists_any(&f.global_path(".claude/skills")));
}

#[test]
fn the_skills_scope_disables_each_named_skill_and_drops_stale_ids() {
    let f = fixture(&["claude_code"]);
    f.sync(&["claude_code"]);
    let scope = DisableScope::Skills {
        skill_ids: vec![f.skill_id.clone(), "gone".into()],
    };

    let report = disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &scope,
        NOW,
    )
    .unwrap();
    assert_eq!(report.removed_rows(), 1);
    assert!(f.store.skill_disabled_at(&f.skill_id).unwrap().is_some());

    let enabled = enable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &scope,
        NOW,
    )
    .unwrap();
    assert_eq!(enabled.global.len(), 1);
    assert_eq!(f.status("claude_code"), Some(SyncStatus::Synced));
}

#[test]
fn an_unknown_skill_is_not_found() {
    let f = fixture(&[]);
    let scope = DisableScope::Skill {
        skill_id: "missing".into(),
    };
    for err in [
        disable_targets(
            &f.store,
            &central_repo_of(&f.store),
            &HostRoots::under_home(f.home()),
            &scope,
            NOW,
        )
        .unwrap_err(),
        enable_targets(
            &f.store,
            &central_repo_of(&f.store),
            &HostRoots::under_home(f.home()),
            &scope,
            NOW,
        )
        .unwrap_err(),
    ] {
        assert!(matches!(
            CommandError::from_anyhow(err),
            CommandError::NotFound { .. }
        ));
    }
}

// ---------------------------------------------------------------------------
// Wire shapes
// ---------------------------------------------------------------------------

#[test]
fn scope_and_skip_reasons_cross_the_wire_as_themselves() {
    let scopes: Vec<DisableScope> = serde_json::from_value(serde_json::json!([
        { "kind": "skill_tool", "skill_id": "s", "tool": "pi" },
        { "kind": "skill", "skill_id": "s" },
        { "kind": "skills", "skill_ids": ["a", "b"] }
    ]))
    .unwrap();
    assert_eq!(
        scopes,
        vec![
            DisableScope::SkillTool {
                skill_id: "s".into(),
                tool: "pi".into()
            },
            DisableScope::Skill {
                skill_id: "s".into()
            },
            DisableScope::Skills {
                skill_ids: vec!["a".into(), "b".into()]
            },
        ]
    );
    assert_eq!(
        serde_json::to_value(PropagationSkip::Disabled).unwrap(),
        serde_json::json!({ "reason": "disabled" })
    );
    assert_eq!(
        serde_json::to_value(ProjectSyncOutcomeStatus::Skipped {
            reason: ProjectSyncSkip::Disabled
        })
        .unwrap(),
        serde_json::json!({ "status": "skipped", "reason": { "reason": "disabled" } })
    );
    assert_eq!(
        serde_json::to_value(CommandError::TargetDisabled { tool: "pi".into() }).unwrap(),
        serde_json::json!({ "code": "TARGET_DISABLED", "tool": "pi" })
    );
    assert_eq!(
        serde_json::to_value(EnableReport::default()).unwrap(),
        serde_json::json!({ "global": [], "projects": { "items": [], "skipped_projects": [] } })
    );
}

// ---------------------------------------------------------------------------
// Review round (2026-09-29): ownership, partial Disable, shared-dir markers
// ---------------------------------------------------------------------------

/// F1: a marker records intent, never ownership. Whether the foreign folder
/// was there before the marker, arrived after it, or blocked an Enable,
/// Delete removes the skill and leaves those bytes alone.
#[test]
fn intent_only_marker_delete_preserves_unmanaged_bytes() {
    type Ordering = fn(&Fixture, &Path);
    let orderings: [(&str, Ordering); 3] = [
        ("marker, then foreign folder", |f, path| {
            disable_targets(
                &f.store,
                &central_repo_of(&f.store),
                &HostRoots::under_home(f.home()),
                &f.pair("claude_code"),
                NOW,
            )
            .unwrap();
            foreign_folder(path);
        }),
        ("foreign folder, then marker", |f, path| {
            foreign_folder(path);
            let report = disable_targets(
                &f.store,
                &central_repo_of(&f.store),
                &HostRoots::under_home(f.home()),
                &f.pair("claude_code"),
                NOW,
            )
            .unwrap();
            assert!(report.targets.is_empty(), "Disable touches no unowned path");
            assert!(foreign_bytes_survive(path));
        }),
        ("marker, foreign folder, Enable collides", |f, path| {
            disable_targets(
                &f.store,
                &central_repo_of(&f.store),
                &HostRoots::under_home(f.home()),
                &f.pair("claude_code"),
                NOW,
            )
            .unwrap();
            foreign_folder(path);
            let enabled = enable_targets(
                &f.store,
                &central_repo_of(&f.store),
                &HostRoots::under_home(f.home()),
                &f.pair("claude_code"),
                NOW,
            )
            .unwrap();
            assert!(matches!(
                enabled.global.as_slice(),
                [o] if matches!(o.status, BatchTargetStatus::Failed { error: CommandError::TargetExists { .. } })
            ));
        }),
    ];
    for (label, arrange) in orderings {
        let f = fixture(&["claude_code"]);
        let path = f.global_path(".claude/skills");
        arrange(&f, &path);

        let report = remove_skill(&f.store, &central_repo_of(&f.store), &f.skill_id).unwrap();

        assert!(report.record_deleted, "{label}: the skill is deleted");
        assert!(
            report.targets.is_empty(),
            "{label}: no path planned: {report}"
        );
        assert!(
            foreign_bytes_survive(&path),
            "{label}: foreign bytes survive"
        );
    }
}

/// F1, project grain: an assignment recorded `disabled` for a disabled skill
/// owns nothing either — bulk unassign clears the row, not the folder.
#[test]
fn an_intent_only_assignment_is_unassigned_without_touching_its_path() {
    let f = fixture(&[]);
    disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.skill(),
        NOW,
    )
    .unwrap();
    let project = project(&f, "claude_code");
    assign_skill_to_project_tools(
        &f.store,
        &central_repo_of(&f.store),
        &project.id,
        &f.skill_id,
        NOW,
    )
    .unwrap();
    let path = Path::new(&project.path).join(".claude/skills/alpha");
    foreign_folder(&path);

    let report = unassign_skill_from_project(
        &f.store,
        &central_repo_of(&f.store),
        &project.id,
        &f.skill_id,
    )
    .unwrap();

    assert!(report.targets.is_empty(), "{report}");
    assert_eq!(report.cleared_markers.len(), 1, "the marker row is cleared");
    assert!(f
        .store
        .get_project_skill_assignment(&project.id, &f.skill_id, "claude_code")
        .unwrap()
        .is_none());
    assert!(foreign_bytes_survive(&path));
}

/// A project first sync that met a directory in the way: typed
/// `TARGET_EXISTS`, the same as the global batch's collision.
fn is_collision(status: &ProjectSyncOutcomeStatus) -> bool {
    matches!(
        status,
        ProjectSyncOutcomeStatus::Failed {
            error: CommandError::TargetExists { .. }
        }
    )
}

/// F1-R1 arrangement: a skill disabled at skill grain, an assignment
/// recorded for it (intent only), a foreign folder at the assignment path,
/// then Enable. The first sync refuses with `TARGET_EXISTS` and settles the
/// row `error` with its diagnostic, while `synced_at` stays NULL: the row
/// never materialised anything, so it still owns nothing.
fn failed_project_enable(f: &Fixture) -> (ProjectRecord, PathBuf) {
    disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.skill(),
        NOW,
    )
    .unwrap();
    let project = project(f, "claude_code");
    assign_skill_to_project_tools(
        &f.store,
        &central_repo_of(&f.store),
        &project.id,
        &f.skill_id,
        NOW,
    )
    .unwrap();
    let path = Path::new(&project.path).join(".claude/skills/alpha");
    foreign_folder(&path);

    let enabled = enable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.skill(),
        NOW,
    )
    .unwrap();

    assert!(
        matches!(enabled.projects.items.as_slice(), [item] if is_collision(&item.status)),
        "{}",
        enabled.projects
    );
    let row = f
        .store
        .get_project_skill_assignment(&project.id, &f.skill_id, "claude_code")
        .unwrap()
        .expect("row kept");
    assert_eq!(row.status, SyncStatus::Error);
    assert_eq!(row.synced_at, None);
    assert!(
        row.last_error
            .as_deref()
            .is_some_and(|e| e.contains("already exists")),
        "the collision stays observable: {:?}",
        row.last_error
    );
    assert!(!row.owns_artifact());
    assert!(foreign_bytes_survive(&path));
    (project, path)
}

/// F1-R1: ownership is independent of lifecycle status. After a failed
/// project Enable turned the intent-only assignment `error`, no removal,
/// re-sync or reconcile touches the foreign folder: unassign (bulk and
/// toggle), project removal, a second Disable and Delete set the row aside
/// (clearing it, or recording it `disabled`), and re-sync is a first sync
/// again that reports the collision instead of overwriting it.
#[test]
fn failed_project_enable_never_makes_a_foreign_folder_ours() {
    type Act = fn(&Fixture, &ProjectRecord, &Path);
    let acts: [(&str, Act); 7] = [
        ("reconcile, then bulk unassign", |f, project, _| {
            // The reconcile pass itself (the listing try-locks the
            // process-wide guard, which a parallel test may hold).
            let mut rows = f.store.list_project_skill_assignments(&project.id).unwrap();
            project_sync::reconcile_listing_unlocked(&f.store, &project.id, &mut rows);
            assert!(rows.iter().all(|a| !a.owns_artifact()));
            let report = unassign_skill_from_project(
                &f.store,
                &central_repo_of(&f.store),
                &project.id,
                &f.skill_id,
            )
            .unwrap();
            assert!(report.targets.is_empty(), "{report}");
            assert_eq!(report.cleared_markers.len(), 1);
        }),
        (
            "toggle-off",
            |f, project, _| match project_sync::toggle_skill_assignment(
                &f.store,
                &central_repo_of(&f.store),
                &project.id,
                &f.skill_id,
                "claude_code",
                NOW,
            )
            .unwrap()
            {
                project_sync::ToggleOutcome::Unassigned { report } => {
                    assert!(report.targets.is_empty(), "{report}");
                    assert_eq!(report.cleared_markers.len(), 1);
                }
                other => panic!("expected unassigned, got {other:?}"),
            },
        ),
        ("remove project", |f, project, _| {
            let report = crate::core::project_ops::remove_project_and_artifacts(
                &f.store,
                &central_repo_of(&f.store),
                &project.id,
            )
            .unwrap();
            assert!(report.targets.is_empty(), "{report}");
            assert_eq!(report.cleared_markers.len(), 1);
            assert!(f.store.get_project_by_id(&project.id).unwrap().is_none());
        }),
        (
            "Disable again, then Enable retries the collision",
            |f, project, _| {
                let report = disable_targets(
                    &f.store,
                    &central_repo_of(&f.store),
                    &HostRoots::under_home(f.home()),
                    &f.skill(),
                    NOW,
                )
                .unwrap();
                assert!(report.targets.is_empty(), "{report}");
                assert!(
                    matches!(
                        report.disabled_markers.as_slice(),
                        [RowRef::Assignment { project_id, tool, .. }]
                            if *project_id == project.id && tool == "claude_code"
                    ),
                    "the never-synced `error` row is recorded `disabled`: {:?}",
                    report.disabled_markers
                );
                let row = f
                    .store
                    .get_project_skill_assignment(&project.id, &f.skill_id, "claude_code")
                    .unwrap()
                    .unwrap();
                assert_eq!(row.status, SyncStatus::Disabled, "intent recorded");
                assert_eq!(row.synced_at, None, "still no owner");
                let enabled = enable_targets(
                    &f.store,
                    &central_repo_of(&f.store),
                    &HostRoots::under_home(f.home()),
                    &f.skill(),
                    NOW,
                )
                .unwrap();
                assert!(matches!(enabled.projects.items.as_slice(), [item]
                if is_collision(&item.status)));
            },
        ),
        ("Delete the skill", |f, _, _| {
            let report = remove_skill(&f.store, &central_repo_of(&f.store), &f.skill_id).unwrap();
            assert!(report.record_deleted, "{report}");
            assert!(report.targets.is_empty(), "{report}");
            assert_eq!(report.cleared_markers.len(), 1);
        }),
        ("resync one", |f, project, _| {
            let report =
                resync_project(&f.store, &central_repo_of(&f.store), &project.id, NOW + 1).unwrap();
            assert!(
                is_collision(&item_for(&report, &f.skill_id).status),
                "{report}"
            );
        }),
        ("resync all", |f, _, _| {
            let report =
                resync_all_projects(&f.store, &central_repo_of(&f.store), NOW + 1).unwrap();
            assert!(
                is_collision(&item_for(&report, &f.skill_id).status),
                "{report}"
            );
        }),
    ];
    for (label, act) in acts {
        let f = fixture(&[]);
        let (project, path) = failed_project_enable(&f);
        act(&f, &project, &path);
        assert!(
            foreign_bytes_survive(&path),
            "{label}: foreign bytes survive"
        );
    }
}

/// F1-R1, Propagation: a row that never materialised is never overwritten by
/// an Update's Propagation, even where its bytes would be rewritten (a copy),
/// at either scope; the collision is a failure the row keeps.
#[test]
fn propagation_never_overwrites_a_path_the_row_does_not_own() {
    use crate::core::skill_store::{ProjectSkillAssignmentRecord, SkillTargetRecord};
    use crate::core::sync_status::SyncMode;

    let f = fixture(&["claude_code"]);
    let global = f.global_path(".claude/skills");
    foreign_folder(&global);
    f.store
        .upsert_skill_target(&SkillTargetRecord {
            id: "unowned".into(),
            skill_id: f.skill_id.clone(),
            tool: "claude_code".into(),
            target_path: global.to_string_lossy().into_owned(),
            mode: SyncMode::Copy,
            status: SyncStatus::Error,
            last_error: Some("target exists".into()),
            synced_at: None,
        })
        .unwrap();
    let project = project(&f, "claude_code");
    let assigned = Path::new(&project.path).join(".claude/skills/alpha");
    foreign_folder(&assigned);
    f.store
        .add_project_skill_assignment(&ProjectSkillAssignmentRecord {
            id: "unowned-assignment".into(),
            project_id: project.id.clone(),
            skill_id: f.skill_id.clone(),
            skill_name: "alpha".into(),
            tool: "claude_code".into(),
            mode: SyncMode::Copy,
            status: SyncStatus::Error,
            last_error: Some("target exists".into()),
            synced_at: None,
            content_hash: None,
            created_at: 1,
        })
        .unwrap();

    let report = propagate_unlocked(&f.store, &f.paths, &f.skill_id, NOW).unwrap();

    assert_eq!(report.targets.len(), 2, "{report:?}");
    assert!(
        report
            .targets
            .iter()
            .all(|t| matches!(t.status, PropagationStatus::Failed { .. })),
        "{report:?}"
    );
    assert!(foreign_bytes_survive(&global));
    assert!(foreign_bytes_survive(&assigned));
    assert_eq!(f.status("claude_code"), Some(SyncStatus::Error));
}

/// F1's other half: a disabled row the app once deployed still owns its
/// path, so an artifact that reappears there is Delete's to remove.
#[test]
fn an_owned_disabled_target_is_still_removed_by_delete() {
    let f = fixture(&["claude_code"]);
    f.sync(&["claude_code"]);
    disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("claude_code"),
        NOW,
    )
    .unwrap();
    let path = f.global_path(".claude/skills");
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("SKILL.md"), "reappeared").unwrap();

    let report = remove_skill(&f.store, &central_repo_of(&f.store), &f.skill_id).unwrap();

    assert!(report.record_deleted);
    assert_eq!(report.removed_rows(), 1);
    assert!(!exists_any(&path), "the owned path is removed");
}

/// Sketch 8: Delete of a disabled skill settles its (owned, disabled) global
/// and project rows, the central copy and the record; a reappeared owned
/// artifact that cannot be removed keeps the skill (ADR-0002) until a retry.
#[cfg(unix)]
#[test]
fn delete_disabled_skill_settles_rows_and_cascade() {
    let f = fixture(&["claude_code"]);
    f.sync(&["claude_code"]);
    let project = project(&f, "claude_code");
    assign_skill_to_project_tools(
        &f.store,
        &central_repo_of(&f.store),
        &project.id,
        &f.skill_id,
        NOW,
    )
    .unwrap();
    disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.skill(),
        NOW,
    )
    .unwrap();
    let central = PathBuf::from(
        f.store
            .get_skill_by_id(&f.skill_id)
            .unwrap()
            .unwrap()
            .central_path,
    );

    // A reappeared owned artifact behind a locked parent: Delete keeps all.
    let global = f.global_path(".claude/skills");
    fs::create_dir_all(&global).unwrap();
    let parent = f.home().join(".claude/skills");
    if !lock_dir(&parent) {
        return;
    }
    let kept = remove_skill(&f.store, &central_repo_of(&f.store), &f.skill_id).unwrap();
    unlock_dir(&parent);
    assert!(!kept.record_deleted && !kept.central_removed);
    // The absent project artifact settled (its row deleted); only the
    // locked global one is kept.
    assert_eq!((kept.removed_rows(), kept.failed_rows()), (1, 1));
    assert_eq!(f.status("claude_code"), Some(SyncStatus::Error));
    assert!(central.is_dir(), "central copy kept for the retry");

    let report = remove_skill(&f.store, &central_repo_of(&f.store), &f.skill_id).unwrap();

    assert!(report.record_deleted && report.central_removed, "{report}");
    assert_eq!((report.removed_rows(), report.failed_rows()), (1, 0));
    assert!(!exists_any(&global));
    assert!(!central.exists());
    assert!(f.store.get_skill_by_id(&f.skill_id).unwrap().is_none());
    assert!(f.store.list_skill_targets(&f.skill_id).unwrap().is_empty());
    assert!(f
        .store
        .list_project_skill_assignments_by_skill(&f.skill_id)
        .unwrap()
        .is_empty());
}

/// F2 (sketch 2): a skill Disable whose project removal failed keeps the
/// assignment `error` while the skill is off everywhere. Re-sync (one and
/// all) and bulk assign must honour the skill marker, not just row status;
/// a healthy skill in the same project is unaffected.
#[cfg(unix)]
#[test]
fn partial_skill_disable_resync_skips_error_assignments() {
    let f = fixture(&["claude_code"]);
    let project = project(&f, "claude_code");
    assign_skill_to_project_tools(
        &f.store,
        &central_repo_of(&f.store),
        &project.id,
        &f.skill_id,
        NOW,
    )
    .unwrap();
    let beta_source = tempfile::tempdir().unwrap();
    fs::write(
        beta_source.path().join("SKILL.md"),
        "---\nname: beta\n---\n",
    )
    .unwrap();
    let beta = install_local_skill(&f.paths, &f.store, beta_source.path(), Some("beta".into()))
        .unwrap()
        .skill_id;
    assign_skill_to_project_tools(
        &f.store,
        &central_repo_of(&f.store),
        &project.id,
        &beta,
        NOW,
    )
    .unwrap();

    let skills_dir = Path::new(&project.path).join(".claude/skills");
    let alpha_target = skills_dir.join("alpha");
    if !lock_dir(&skills_dir) {
        return;
    }
    let report = disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.skill(),
        NOW,
    )
    .unwrap();
    unlock_dir(&skills_dir);
    assert_eq!(report.failed_rows(), 1);
    assert!(f.store.skill_disabled_at(&f.skill_id).unwrap().is_some());
    // The operator clears the artifact by hand; the row still says `error`.
    fs::remove_file(&alpha_target).unwrap();
    let row = || {
        f.store
            .get_project_skill_assignment(&project.id, &f.skill_id, "claude_code")
            .unwrap()
            .expect("row kept")
    };
    let failure = row().last_error.clone();
    assert_eq!(row().status, SyncStatus::Error);
    assert!(failure.is_some());

    for resync in [
        resync_project(&f.store, &central_repo_of(&f.store), &project.id, NOW + 1).unwrap(),
        resync_all_projects(&f.store, &central_repo_of(&f.store), NOW + 2).unwrap(),
    ] {
        assert!(
            is_disabled_skip(&item_for(&resync, &f.skill_id).status),
            "{resync}"
        );
        assert!(
            matches!(
                item_for(&resync, &beta).status,
                ProjectSyncOutcomeStatus::Synced
            ),
            "a healthy skill still re-syncs: {resync}"
        );
        assert!(!exists_any(&alpha_target), "nothing re-materialised");
        assert_eq!(row().status, SyncStatus::Error, "the failure row is kept");
        assert_eq!(row().last_error, failure);
    }

    // Bulk assign of the kept `error` row reports why, not "already assigned".
    let assign = assign_skill_to_project_tools(
        &f.store,
        &central_repo_of(&f.store),
        &project.id,
        &f.skill_id,
        NOW,
    )
    .unwrap();
    assert!(
        matches!(assign.items.as_slice(), [item] if is_disabled_skip(&item.status)),
        "{assign}"
    );
    assert!(!exists_any(&alpha_target));
}

/// F3 (sketch 4): Amp and Kimi share `.config/agents/skills`. Only Kimi has
/// a row; disabling Amp while the shared artifact cannot be removed must not
/// mint an Amp `disabled` row beside it. The retry settles both.
#[cfg(unix)]
/// A row a pre-1.4.0 sibling wrote (Kimi Code's, on Amp's folder) is the
/// folder's record: a pair Disable of the folder settles it — kept `error`
/// while the artifact stays, `disabled` once it is gone — and mints no marker
/// beside it.
#[test]
fn a_legacy_sibling_row_is_the_folders_record_for_a_failed_and_retried_disable() {
    let f = fixture(&["kimi_cli"]);
    f.sync(&["kimi_cli"]);
    // Re-key the folder's row as the pre-1.4.0 per-sharer write left it.
    let row = f
        .store
        .get_skill_target(&f.skill_id, "amp")
        .unwrap()
        .expect("folder row");
    f.store
        .upsert_skill_target(&crate::core::skill_store::SkillTargetRecord {
            id: "legacy-kimi".into(),
            tool: "kimi_cli".into(),
            ..row
        })
        .unwrap();
    f.store.delete_skill_target(&f.skill_id, "amp").unwrap();
    mark_installed_in(
        &HostRoots::under_home(f.home()),
        adapter_by_key("amp").unwrap(),
    );
    let shared = f.global_path(".config/agents/skills");
    assert!(exists_any(&shared));
    assert_eq!(f.status("amp"), None);

    let parent = f.home().join(".config/agents/skills");
    if !lock_dir(&parent) {
        return;
    }
    let failed = disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("amp"),
        NOW,
    )
    .unwrap();
    unlock_dir(&parent);

    assert_eq!(failed.failed_rows(), 1, "{failed}");
    assert_eq!(f.status("kimi_cli"), Some(SyncStatus::Error));
    assert_eq!(f.status("amp"), None, "no marker while the artifact stays");
    assert!(exists_any(&shared));

    let retried = disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("amp"),
        NOW,
    )
    .unwrap();

    assert_eq!((retried.removed_rows(), retried.failed_rows()), (1, 0));
    assert!(!exists_any(&shared));
    assert_eq!(f.status("kimi_cli"), Some(SyncStatus::Disabled));
    assert_eq!(
        f.status("amp"),
        None,
        "the legacy row records the folder; no second row is minted"
    );
}

/// One folder, one row: Kimi Code's Deploy, Disable and Enable all settle the
/// row of the folder serving it (`amp`), and never a row of its own.
#[test]
fn a_harness_on_a_shared_folder_is_disabled_and_enabled_through_the_folders_row() {
    let f = fixture(&["kimi_cli", "amp"]);
    let outcomes = f.sync(&["kimi_cli", "amp"]);
    assert_eq!(outcomes.len(), 1, "one folder, one outcome: {outcomes:?}");
    assert_eq!(outcomes[0].tool_key, "amp");
    assert_eq!(f.status("amp"), Some(SyncStatus::Synced));
    assert_eq!(f.status("kimi_cli"), None);

    let report = disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("kimi_cli"),
        NOW,
    )
    .unwrap();
    assert_eq!(report.failed_rows(), 0, "{report}");
    assert_eq!(f.status("amp"), Some(SyncStatus::Disabled));
    assert_eq!(f.status("kimi_cli"), None);
    assert!(!exists_any(&f.global_path(".config/agents/skills")));

    let enabled = enable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("kimi_cli"),
        NOW,
    )
    .unwrap();
    assert_eq!(enabled.global.len(), 1, "{:?}", enabled.global);
    assert_eq!(enabled.global[0].tool_key, "amp");
    assert_eq!(f.status("amp"), Some(SyncStatus::Synced));
    assert_eq!(f.status("kimi_cli"), None);
    assert!(exists_any(&f.global_path(".config/agents/skills")));
}

/// Sketch 7: a project assignment on the `.agents/skills` virtual group is
/// disabled and enabled once, at its registry path.
#[test]
fn disabled_virtual_group_assignment_enables_once() {
    let f = fixture(&[]);
    let project = project(&f, "agents_skills");
    assign_skill_to_project_tools(
        &f.store,
        &central_repo_of(&f.store),
        &project.id,
        &f.skill_id,
        NOW,
    )
    .unwrap();
    let target = Path::new(&project.path).join(".agents/skills/alpha");
    assert!(exists_any(&target));

    let disabled = disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.skill(),
        NOW,
    )
    .unwrap();
    assert_eq!(disabled.removed_rows(), 1);
    assert!(!exists_any(&target));

    let enabled = enable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.skill(),
        NOW,
    )
    .unwrap();

    assert!(
        matches!(
            enabled.projects.items.as_slice(),
            [item] if item.tool == "agents_skills" && matches!(item.status, ProjectSyncOutcomeStatus::Synced)
        ),
        "{}",
        enabled.projects
    );
    assert!(enabled.global.is_empty());
    assert!(exists_any(&target));
}

// ---------------------------------------------------------------------------
// Ticket 04 (D12): every `TARGET_EXISTS` producer answers an askable row
// carrying the ask's facts, and the ask's retry settles it.
// ---------------------------------------------------------------------------

fn ask_facts(error: &CommandError, expected: &Path) {
    match error {
        CommandError::TargetExists {
            path,
            modified_at,
            same_content,
        } => {
            assert_eq!(Path::new(path), expected);
            assert!(modified_at.is_some(), "modified time observed");
            assert!(!same_content, "a foreign folder differs from the skill");
        }
        other => panic!("expected TARGET_EXISTS, got {other:?}"),
    }
}

fn overwrite_or_keep(
    f: &Fixture,
    tool: &str,
    decision: crate::core::global_sync::OverrideDecision,
) -> Vec<crate::core::global_sync::BatchOverride> {
    vec![crate::core::global_sync::BatchOverride {
        skill_id: f.skill_id.clone(),
        tool_key: tool.to_string(),
        decision,
    }]
}

/// Enable's first sync collides → an askable row (pair identity + facts);
/// the retry is the same scope with the decision: Overwrite replaces.
#[test]
fn an_enable_collision_is_askable_and_its_overwrite_retry_replaces_the_folder() {
    use crate::core::global_sync::OverrideDecision;
    use crate::core::skill_disable::enable_targets_with;
    let f = fixture(&["claude_code"]);
    disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("claude_code"),
        NOW,
    )
    .unwrap();
    let path = f.global_path(".claude/skills");
    foreign_folder(&path);

    let enabled = enable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("claude_code"),
        NOW,
    )
    .unwrap();
    let [row] = enabled.global.as_slice() else {
        panic!("{:?}", enabled.global)
    };
    assert_eq!(
        (row.skill_id.as_str(), row.tool_key.as_str()),
        (f.skill_id.as_str(), "claude_code")
    );
    let BatchTargetStatus::Failed { error } = &row.status else {
        panic!("{row:?}")
    };
    ask_facts(error, &path);
    assert_eq!(f.status("claude_code"), Some(SyncStatus::Disabled));

    let retried = enable_targets_with(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("claude_code"),
        &overwrite_or_keep(&f, "claude_code", OverrideDecision::Overwrite),
        NOW,
    )
    .unwrap();
    assert!(
        matches!(retried.global.as_slice(), [o] if matches!(o.status, BatchTargetStatus::Synced { .. })),
        "{:?}",
        retried.global
    );
    assert!(!foreign_bytes_survive(&path));
    assert_eq!(f.status("claude_code"), Some(SyncStatus::Synced));
}

#[test]
fn an_enable_collision_kept_stays_disabled_and_reads_declined() {
    use crate::core::global_sync::OverrideDecision;
    use crate::core::skill_disable::enable_targets_with;
    let f = fixture(&["claude_code"]);
    disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("claude_code"),
        NOW,
    )
    .unwrap();
    let path = f.global_path(".claude/skills");
    foreign_folder(&path);
    enable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("claude_code"),
        NOW,
    )
    .unwrap();

    let retried = enable_targets_with(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("claude_code"),
        &overwrite_or_keep(&f, "claude_code", OverrideDecision::Keep),
        NOW,
    )
    .unwrap();
    assert!(
        matches!(retried.global.as_slice(), [o] if matches!(o.status, BatchTargetStatus::Declined)),
        "{:?}",
        retried.global
    );
    assert!(foreign_bytes_survive(&path));
    let row = f
        .store
        .get_skill_target(&f.skill_id, "claude_code")
        .unwrap()
        .unwrap();
    assert_eq!(row.status, SyncStatus::Disabled);
    assert!(!row.owns_artifact());
}

/// Review F1 arrangement: an assignment synced (so it owned its artifact),
/// then the skill disabled successfully (artifact removed, row `disabled`
/// with its historical `synced_at`), then a foreign folder at the path and a
/// skill Enable. Enable's admission clears the ownership, so the collision
/// leaves a non-owner the ask can settle.
fn synced_then_disabled_project_enable(
    f: &Fixture,
) -> (ProjectRecord, PathBuf, project_sync::ProjectSyncOutcome) {
    let project = project(f, "claude_code");
    assign_skill_to_project_tools(
        &f.store,
        &central_repo_of(&f.store),
        &project.id,
        &f.skill_id,
        NOW,
    )
    .unwrap();
    let path = Path::new(&project.path).join(".claude/skills/alpha");
    let owned = f
        .store
        .get_project_skill_assignment(&project.id, &f.skill_id, "claude_code")
        .unwrap()
        .unwrap();
    assert!(owned.owns_artifact(), "the first assignment sync owns");
    disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.skill(),
        NOW,
    )
    .unwrap();
    let disabled = f
        .store
        .get_project_skill_assignment(&project.id, &f.skill_id, "claude_code")
        .unwrap()
        .unwrap();
    assert_eq!(disabled.status, SyncStatus::Disabled);
    assert!(!exists_any(&path), "the Disable removed the artifact");
    foreign_folder(&path);

    let enabled = enable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.skill(),
        NOW,
    )
    .unwrap();
    let [item] = enabled.projects.items.as_slice() else {
        panic!("{}", enabled.projects)
    };
    assert!(is_collision(&item.status), "{}", enabled.projects);
    let row = f
        .store
        .get_project_skill_assignment(&project.id, &f.skill_id, "claude_code")
        .unwrap()
        .unwrap();
    assert_eq!(row.status, SyncStatus::Error);
    assert!(
        !row.owns_artifact(),
        "a collided Enable leaves a non-owner, not the historical owner"
    );
    assert!(foreign_bytes_survive(&path));
    (project, path, item.clone())
}

/// A project collision resolves through `project_sync::resolve_collisions`:
/// Overwrite materialises and owns anew, Keep keeps the folder and records
/// the row `disabled`, owning nothing — so no later resync, Propagation or
/// Delete touches it. Two arrangements: the never-synced marker of a failed
/// project Enable (re-asked by resync), and a previously synced assignment
/// whose skill was disabled and re-enabled into a foreign folder (review F1).
#[test]
fn a_project_collision_resolves_with_overwrite_or_keep() {
    use crate::core::global_sync::OverrideDecision;
    use crate::core::project_sync::{resolve_collisions, AssignmentDecision};
    type Setup = fn(&Fixture) -> (ProjectRecord, PathBuf, project_sync::ProjectSyncOutcome);
    let setups: [(&str, Setup); 2] = [
        ("never-synced marker", |f| {
            let (project, path) = failed_project_enable(f);
            let report =
                resync_project(&f.store, &central_repo_of(&f.store), &project.id, NOW).unwrap();
            let [item] = report.items.as_slice() else {
                panic!("{report}")
            };
            (project, path, item.clone())
        }),
        (
            "synced, disabled, re-enabled",
            synced_then_disabled_project_enable,
        ),
    ];
    for (label, setup) in setups {
        for decision in [OverrideDecision::Overwrite, OverrideDecision::Keep] {
            let f = fixture(&[]);
            let (project, path, item) = setup(&f);
            let ProjectSyncOutcomeStatus::Failed { error } = &item.status else {
                panic!("{label}: {item:?}")
            };
            ask_facts(error, &path);

            let resolved = resolve_collisions(
                &f.store,
                &central_repo_of(&f.store),
                &[AssignmentDecision {
                    project_id: item.project_id.clone(),
                    skill_id: item.skill_id.clone(),
                    tool: item.tool.clone(),
                    decision,
                }],
                NOW + 5,
            )
            .unwrap();
            let row = f
                .store
                .get_project_skill_assignment(&project.id, &f.skill_id, "claude_code")
                .unwrap()
                .unwrap();
            match decision {
                OverrideDecision::Overwrite => {
                    assert!(
                        matches!(resolved.items[0].status, ProjectSyncOutcomeStatus::Synced),
                        "{label}: {resolved}"
                    );
                    assert!(!foreign_bytes_survive(&path), "{label}");
                    assert_eq!(row.status, SyncStatus::Synced, "{label}");
                    assert_eq!(row.synced_at, Some(NOW + 5), "{label}: owns anew");
                    // The new artifact is ours: Delete removes it.
                    let report =
                        remove_skill(&f.store, &central_repo_of(&f.store), &f.skill_id).unwrap();
                    assert!(report.record_deleted, "{label}: {report}");
                    assert!(!exists_any(&path), "{label}: the owned artifact is removed");
                }
                OverrideDecision::Keep => {
                    assert!(
                        matches!(resolved.items[0].status, ProjectSyncOutcomeStatus::Declined),
                        "{label}: {resolved}"
                    );
                    assert!(foreign_bytes_survive(&path), "{label}");
                    assert_eq!(row.status, SyncStatus::Disabled, "{label}");
                    assert_eq!(row.synced_at, None, "{label}: a kept row owns nothing");
                    // No later pass re-asks or replaces: resync skips it,
                    // Propagation skips it, Delete sets it aside.
                    let again =
                        resync_project(&f.store, &central_repo_of(&f.store), &project.id, NOW + 6)
                            .unwrap();
                    assert!(is_disabled_skip(&again.items[0].status), "{label}: {again}");
                    assert!(foreign_bytes_survive(&path), "{label}: resync kept it");
                    let propagated =
                        propagate_unlocked(&f.store, &f.paths, &f.skill_id, NOW + 7).unwrap();
                    assert!(
                        propagated.targets.iter().any(|t| matches!(
                            (&t.scope, &t.status),
                            (
                                PropagationScope::Project { .. },
                                PropagationStatus::Skipped {
                                    reason: PropagationSkip::Disabled
                                }
                            )
                        )),
                        "{label}: {propagated:?}"
                    );
                    assert!(foreign_bytes_survive(&path), "{label}: Propagation kept it");
                    let report =
                        remove_skill(&f.store, &central_repo_of(&f.store), &f.skill_id).unwrap();
                    assert!(report.record_deleted, "{label}: {report}");
                    assert!(
                        report.targets.is_empty(),
                        "{label}: no path planned: {report}"
                    );
                    assert!(foreign_bytes_survive(&path), "{label}: Delete kept it");
                }
            }
        }
    }
}

/// Enable's admission clears ownership only for a successfully disabled
/// row: a row whose removal failed (`error`, ADR-0002) keeps its ownership,
/// so a retry still plans and removes the artifact that is ours.
#[test]
fn enable_admission_keeps_the_ownership_of_a_failed_removal() {
    use crate::core::skill_store::AssignmentTransition;
    let f = fixture(&[]);
    let project = project(&f, "claude_code");
    assign_skill_to_project_tools(
        &f.store,
        &central_repo_of(&f.store),
        &project.id,
        &f.skill_id,
        NOW,
    )
    .unwrap();
    let row = |f: &Fixture| {
        f.store
            .get_project_skill_assignment(&project.id, &f.skill_id, "claude_code")
            .unwrap()
            .unwrap()
    };
    f.store
        .transition_assignment(
            &row(&f).id,
            AssignmentTransition::SyncFailed {
                error: "removal failed",
            },
        )
        .unwrap();
    f.store
        .transition_assignment(&row(&f).id, AssignmentTransition::EnableAdmitted)
        .unwrap();
    assert_eq!(row(&f).status, SyncStatus::Error);
    assert!(row(&f).owns_artifact(), "a failed removal stays an owner");

    f.store
        .transition_assignment(&row(&f).id, AssignmentTransition::Disabled)
        .unwrap();
    assert!(
        row(&f).owns_artifact(),
        "Disable keeps the historical record"
    );
    f.store
        .transition_assignment(&row(&f).id, AssignmentTransition::EnableAdmitted)
        .unwrap();
    assert_eq!(row(&f).status, SyncStatus::Disabled);
    assert!(!row(&f).owns_artifact(), "admission clears a disabled row");
}

/// Seed the pre-1.4.0 per-sharer state: the folder's row plus a sibling row
/// of Kimi Code's at the same path.
fn seed_legacy_sibling(f: &Fixture) {
    let row = f
        .store
        .get_skill_target(&f.skill_id, "amp")
        .unwrap()
        .expect("folder row");
    f.store
        .upsert_skill_target(&crate::core::skill_store::SkillTargetRecord {
            id: "legacy-kimi".into(),
            tool: "kimi_cli".into(),
            ..row
        })
        .unwrap();
}

/// Existing per-sharer rows on a dir shared today are each their own row: a
/// pair Disable of either harness's folder settles both rows, the batch skips
/// every row of the folder while one is disabled, and Enable admits each row
/// and syncs it by itself at its own recorded path — no row minted, none lost.
#[test]
fn legacy_sibling_rows_are_disabled_gated_and_enabled_each_by_itself() {
    let f = fixture(&["amp", "kimi_cli"]);
    f.sync(&["amp"]);
    seed_legacy_sibling(&f);
    let shared = f.global_path(".config/agents/skills");

    let report = disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("kimi_cli"),
        NOW,
    )
    .unwrap();
    assert_eq!(
        (report.removed_rows(), report.failed_rows()),
        (2, 0),
        "{report}"
    );
    assert!(!exists_any(&shared));
    assert_eq!(f.status("amp"), Some(SyncStatus::Disabled));
    assert_eq!(f.status("kimi_cli"), Some(SyncStatus::Disabled));

    let outcomes = f.sync(&["amp"]);
    assert_eq!(outcomes.len(), 2, "one outcome per row: {outcomes:?}");
    assert!(outcomes.iter().all(skipped_disabled), "{outcomes:?}");
    assert!(!exists_any(&shared));

    let enabled = enable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("amp"),
        NOW,
    )
    .unwrap();
    assert_eq!(enabled.global.len(), 2, "{:?}", enabled.global);
    assert!(exists_any(&shared));
    for tool in ["amp", "kimi_cli"] {
        let row = f
            .store
            .get_skill_target(&f.skill_id, tool)
            .unwrap()
            .unwrap();
        assert_eq!(row.status, SyncStatus::Synced, "{tool}");
        assert!(row.owns_artifact(), "{tool}");
        assert_eq!(
            std::path::Path::new(&row.target_path),
            shared.as_path(),
            "{tool}"
        );
    }
}

/// A Kimi Code Keep marker recorded under another name over a foreign folder
/// is its own row, and Enable treats it as `main` did: every disabled row of
/// the folder in scope is taken and synced again as a first sync, each under
/// its own key at the skill's place in the folder (Astra 201/202) — never at
/// the stale name, whose foreign folder is left as it is.
#[test]
fn global_enable_re_records_each_taken_row_under_its_own_key_at_the_skills_place() {
    let f = fixture(&["amp", "kimi_cli"]);
    f.sync(&["amp"]);
    let shared = f.global_path(".config/agents/skills");
    let old_name = shared.with_file_name("old-name");
    std::fs::create_dir_all(&old_name).unwrap();
    std::fs::write(old_name.join("SKILL.md"), "foreign").unwrap();
    f.store
        .upsert_skill_target(&crate::core::skill_store::SkillTargetRecord {
            id: "legacy-kimi-keep".into(),
            skill_id: f.skill_id.clone(),
            tool: "kimi_cli".into(),
            target_path: old_name.to_string_lossy().into(),
            mode: crate::core::sync_status::SyncMode::Symlink,
            status: SyncStatus::Disabled,
            last_error: None,
            synced_at: None,
        })
        .unwrap();
    let roots = HostRoots::under_home(f.home());
    disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &roots,
        &f.pair("amp"),
        NOW,
    )
    .unwrap();
    assert!(!exists_any(&shared));

    let enabled = enable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &roots,
        &f.pair("amp"),
        NOW,
    )
    .unwrap();

    assert_eq!(enabled.global.len(), 2, "{:?}", enabled.global);
    assert!(enabled
        .global
        .iter()
        .all(|o| matches!(o.status, BatchTargetStatus::Synced { .. })));
    assert!(exists_any(&shared));
    for tool in ["amp", "kimi_cli"] {
        let row = f
            .store
            .get_skill_target(&f.skill_id, tool)
            .unwrap()
            .unwrap_or_else(|| panic!("{tool} is re-recorded under its own key"));
        assert_eq!(row.status, SyncStatus::Synced, "{tool}");
        assert_eq!(
            std::path::Path::new(&row.target_path),
            shared.as_path(),
            "{tool}"
        );
    }
    assert_eq!(
        std::fs::read_to_string(old_name.join("SKILL.md")).unwrap(),
        "foreign"
    );
}

/// Astra's finding 9 / Sol's 103, the reported key order: the folder key's
/// row (`amp`) records another path, while a pre-1.4.0 `kimi_cli` row owns
/// the folder's artifact.
///
/// - A successful global batch syncs each row by itself, at the skill's place
///   (as on `main`, Astra 201): Amp's row is settled by its own write there,
///   never by Kimi's, and the foreign folder at its stale path is untouched.
/// - Enable acts on exactly the rows it took (its in-memory first-sync plan):
///   a folder-key record outside its scope — Amp's live `error` row at
///   another path — is never rewritten.
#[test]
fn a_global_write_or_enable_never_repurposes_another_rows_record() {
    let f = fixture(&["amp", "kimi_cli"]);
    f.sync(&["amp"]);
    seed_legacy_sibling(&f);
    let shared = f.global_path(".config/agents/skills");
    let independent = shared.with_file_name("independent");
    foreign_folder(&independent);
    // The folder key's row: never synced, recording another path.
    f.store.delete_skill_target(&f.skill_id, "amp").unwrap();
    let seed_amp = |status: SyncStatus| {
        f.store
            .upsert_skill_target(&crate::core::skill_store::SkillTargetRecord {
                id: "amp-record".into(),
                skill_id: f.skill_id.clone(),
                tool: "amp".into(),
                target_path: independent.to_string_lossy().into(),
                mode: crate::core::sync_status::SyncMode::Symlink,
                status,
                last_error: None,
                synced_at: None,
            })
            .unwrap();
    };
    let amp_record = |f: &Fixture| {
        f.store
            .get_skill_target(&f.skill_id, "amp")
            .unwrap()
            .unwrap()
    };
    let kimi_record = |f: &Fixture| {
        f.store
            .get_skill_target(&f.skill_id, "kimi_cli")
            .unwrap()
            .unwrap()
    };

    // A successful global batch (Sol 103): an `error` record is not gated.
    seed_amp(SyncStatus::Error);
    let outcomes = f.sync(&["kimi_cli"]);
    for tool in ["amp", "kimi_cli"] {
        assert!(
            outcomes
                .iter()
                .any(|o| o.tool_key == tool && matches!(o.status, BatchTargetStatus::Synced { .. })),
            "{tool}: {outcomes:?}"
        );
    }
    let amp = amp_record(&f);
    assert_eq!(
        std::path::Path::new(&amp.target_path),
        shared.as_path(),
        "its own write"
    );
    assert!(foreign_bytes_survive(&independent));
    let kimi = kimi_record(&f);
    assert_eq!(std::path::Path::new(&kimi.target_path), shared.as_path());
    assert_eq!(kimi.status, SyncStatus::Synced);

    // Disable the folder, then a live Amp record at another path appears
    // (outside any Enable scope: it is not `disabled`); Enable the folder
    // (Astra 9).
    let roots = HostRoots::under_home(f.home());
    disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &roots,
        &f.pair("kimi_cli"),
        NOW,
    )
    .unwrap();
    assert!(!exists_any(&shared));
    assert_eq!(kimi_record(&f).status, SyncStatus::Disabled);
    seed_amp(SyncStatus::Error);
    let enabled = enable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &roots,
        &f.pair("kimi_cli"),
        NOW,
    )
    .unwrap();

    assert!(
        enabled.global.iter().all(|o| o.tool_key == "kimi_cli"),
        "only the taken row is synced: {:?}",
        enabled.global
    );
    let amp = amp_record(&f);
    assert_eq!(
        (amp.target_path.as_str(), amp.status, amp.synced_at),
        (&*independent.to_string_lossy(), SyncStatus::Error, None),
        "a record outside the Enable's scope is never rewritten"
    );
    assert!(foreign_bytes_survive(&independent));
    let kimi = kimi_record(&f);
    assert_eq!(std::path::Path::new(&kimi.target_path), shared.as_path());
    assert_eq!(kimi.status, SyncStatus::Synced);
    assert!(kimi.owns_artifact());
    assert!(exists_any(&shared));
}

/// Astra 202: Enable leaves no lasting state of its own. A pair whose folder
/// no installed harness reads any more is enabled as `main` did it: its
/// disabled row is taken, the first sync skips `ToolNotInstalled`, and no row
/// remains (no `pending` row is left behind).
#[test]
fn enabling_a_pair_whose_harness_is_gone_leaves_no_row() {
    let f = fixture(&["cursor"]);
    f.sync(&["cursor"]);
    let roots = HostRoots::under_home(f.home());
    disable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &roots,
        &f.pair("cursor"),
        NOW,
    )
    .unwrap();
    assert_eq!(f.status("cursor"), Some(SyncStatus::Disabled));
    // The harness is uninstalled.
    fs::remove_dir_all(f.home().join(".cursor")).unwrap();

    let enabled = enable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &roots,
        &f.pair("cursor"),
        NOW,
    )
    .unwrap();

    assert!(
        matches!(
            enabled.global.as_slice(),
            [BatchTargetOutcome {
                status: BatchTargetStatus::Skipped {
                    error: CommandError::ToolNotInstalled { .. }
                },
                ..
            }]
        ),
        "{:?}",
        enabled.global
    );
    assert_eq!(f.status("cursor"), None, "no row is left behind");
    assert!(f.store.list_skill_targets(&f.skill_id).unwrap().is_empty());
}

/// Astra 202, a legacy row: a disabled Kimi Code row (its own harness gone,
/// Amp still reading the folder) is taken by a skill-grain Enable and skipped
/// `ToolNotInstalled` under its own key, as before rows were per folder — no
/// row is left behind and nothing is written for it.
#[test]
fn enabling_a_legacy_row_whose_own_harness_is_gone_leaves_no_row() {
    let f = fixture(&["amp"]);
    f.store
        .upsert_skill_target(&crate::core::skill_store::SkillTargetRecord {
            id: "legacy-kimi".into(),
            skill_id: f.skill_id.clone(),
            tool: "kimi_cli".into(),
            target_path: f
                .global_path(".config/agents/skills")
                .to_string_lossy()
                .into(),
            mode: crate::core::sync_status::SyncMode::Symlink,
            status: SyncStatus::Disabled,
            last_error: None,
            synced_at: Some(1),
        })
        .unwrap();

    let enabled = enable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.skill(),
        NOW,
    )
    .unwrap();

    assert!(
        matches!(
            enabled.global.as_slice(),
            [BatchTargetOutcome {
                tool_key,
                status: BatchTargetStatus::Skipped {
                    error: CommandError::ToolNotInstalled { .. }
                },
                ..
            }] if tool_key == "kimi_cli"
        ),
        "{:?}",
        enabled.global
    );
    assert!(f.store.list_skill_targets(&f.skill_id).unwrap().is_empty());
    assert!(!exists_any(&f.global_path(".config/agents/skills")));
}

/// The batch's Disabled rule reads a sibling's disabled row as its folder's.
#[test]
fn a_disabled_legacy_sibling_row_gates_the_folder() {
    let f = fixture(&["amp", "kimi_cli"]);
    f.store
        .upsert_skill_target(&crate::core::skill_store::SkillTargetRecord {
            id: "legacy-kimi".into(),
            skill_id: f.skill_id.clone(),
            tool: "kimi_cli".into(),
            target_path: f
                .global_path(".config/agents/skills")
                .to_string_lossy()
                .into(),
            mode: crate::core::sync_status::SyncMode::Symlink,
            status: SyncStatus::Disabled,
            last_error: None,
            synced_at: None,
        })
        .unwrap();

    let outcomes = f.sync(&["amp"]);

    assert!(
        matches!(outcomes.as_slice(), [o] if skipped_disabled(o)),
        "{outcomes:?}"
    );
    assert!(!exists_any(&f.global_path(".config/agents/skills")));
    assert_eq!(f.status("amp"), None);
}

/// Delete takes every row of the folder with its one artifact.
#[test]
fn delete_removes_legacy_sibling_rows_with_their_one_artifact() {
    let f = fixture(&["amp", "kimi_cli"]);
    f.sync(&["amp"]);
    seed_legacy_sibling(&f);

    let report = remove_skill(&f.store, &central_repo_of(&f.store), &f.skill_id).unwrap();

    assert!(report.failures().is_empty(), "{report}");
    assert_eq!(report.targets.len(), 1, "one artifact: {report}");
    assert!(!exists_any(&f.global_path(".config/agents/skills")));
    assert!(f.store.list_skill_targets(&f.skill_id).unwrap().is_empty());
}

/// A pair Enable turns its folder back on, as `main` did for the folder it
/// named: a taken legacy row whose own harness is gone (Kimi Code) is dropped,
/// and the folder — reachable through Amp — gets its one new row.
#[test]
fn a_pair_enable_of_a_reachable_folder_deploys_it_when_only_a_gone_harnesss_row_was_disabled() {
    let f = fixture(&["amp"]);
    f.store
        .upsert_skill_target(&crate::core::skill_store::SkillTargetRecord {
            id: "legacy-kimi".into(),
            skill_id: f.skill_id.clone(),
            tool: "kimi_cli".into(),
            target_path: f
                .global_path(".config/agents/skills")
                .to_string_lossy()
                .into(),
            mode: crate::core::sync_status::SyncMode::Symlink,
            status: SyncStatus::Disabled,
            last_error: None,
            synced_at: Some(1),
        })
        .unwrap();

    let enabled = enable_targets(
        &f.store,
        &central_repo_of(&f.store),
        &HostRoots::under_home(f.home()),
        &f.pair("amp"),
        NOW,
    )
    .unwrap();

    assert!(
        matches!(
            enabled.global.as_slice(),
            [BatchTargetOutcome { tool_key, status: BatchTargetStatus::Synced { .. }, .. }]
                if tool_key == "amp"
        ),
        "{:?}",
        enabled.global
    );
    assert_eq!(f.status("amp"), Some(SyncStatus::Synced));
    assert_eq!(f.status("kimi_cli"), None);
    assert!(exists_any(&f.global_path(".config/agents/skills")));
}
