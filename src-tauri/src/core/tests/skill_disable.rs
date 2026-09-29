//! Tests for `core::skill_disable` — the durable "not on this Tool" at two
//! grains, and the skip every automatic pass owes it (Propagation, the
//! Refresh re-assert, the global sync batch, project sync).
//!
//! Every case runs against a temp home / central dir / DB; installedness is
//! faked through `mark_installed_in`. Nothing touches the operator's home.

use std::fs;
use std::path::{Path, PathBuf};

use crate::core::artifact_removal::remove_skill;
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
use crate::core::refresh::{
    refresh_managed_skills, RefreshPolicy, RefreshSelection, SkillRefreshStatus,
};
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
        &self.paths.home
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
            self.home(),
            &self.store,
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
    fn refresh_with_reassert(&self) -> Vec<(PropagationScope, PropagationStatus)> {
        let report = refresh_managed_skills(
            &self.paths,
            &self.store,
            &crate::core::credentials::MemoryStore::new(),
            RefreshSelection::All,
            RefreshPolicy {
                reassert_auto_sync: true,
            },
            None,
            NOW + 1,
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
        home: dir.path().join("home"),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.home).expect("home");
    for tool in tools {
        mark_installed_in(&paths.home, adapter_by_key(tool).expect("adapter"));
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

    let report = disable_targets(&f.store, f.home(), &f.pair("claude_code"), NOW).unwrap();

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
fn a_disabled_pair_survives_refresh_reassert_and_propagation() {
    let f = fixture(&["claude_code", "codex"]);
    f.sync(&["claude_code", "codex"]);
    disable_targets(&f.store, f.home(), &f.pair("claude_code"), NOW).unwrap();

    let targets = f.refresh_with_reassert();

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
fn disabling_a_pair_with_no_row_records_a_marker_the_reassert_honours() {
    let f = fixture(&["claude_code"]);
    assert_eq!(f.status("claude_code"), None);

    let report = disable_targets(&f.store, f.home(), &f.pair("claude_code"), NOW).unwrap();
    assert!(report.targets.is_empty(), "nothing to remove");
    assert_eq!(f.status("claude_code"), Some(SyncStatus::Disabled));

    f.refresh_with_reassert();
    assert!(!exists_any(&f.global_path(".claude/skills")));
    assert_eq!(f.status("claude_code"), Some(SyncStatus::Disabled));
}

#[test]
fn enabling_a_pair_resyncs_it_as_a_first_sync() {
    let f = fixture(&["claude_code"]);
    f.sync(&["claude_code"]);
    disable_targets(&f.store, f.home(), &f.pair("claude_code"), NOW).unwrap();

    let report = enable_targets(&f.store, f.home(), &f.pair("claude_code"), NOW).unwrap();

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
    disable_targets(&f.store, f.home(), &f.pair("claude_code"), NOW).unwrap();
    let foreign = f.global_path(".claude/skills");
    fs::create_dir_all(&foreign).unwrap();
    fs::write(foreign.join("SKILL.md"), "someone else's").unwrap();

    let report = enable_targets(&f.store, f.home(), &f.pair("claude_code"), NOW).unwrap();

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
    disable_targets(&f.store, f.home(), &f.pair("claude_code"), NOW).unwrap();

    let again = disable_targets(&f.store, f.home(), &f.pair("claude_code"), NOW).unwrap();
    assert!(again.targets.is_empty());
    assert_eq!(f.status("claude_code"), Some(SyncStatus::Disabled));
}

/// A shared skills dir is one artifact: while one member is disabled, the
/// batch never writes it for a sibling that has no row of its own.
#[test]
fn a_shared_dir_sibling_is_skipped_while_a_member_is_disabled() {
    let f = fixture(&["amp", "kimi_cli"]);
    disable_targets(&f.store, f.home(), &f.pair("amp"), NOW).unwrap();
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
    let err = disable_targets(&f.store, f.home(), &f.pair("no_such_tool"), NOW).unwrap_err();
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

    let report = disable_targets(&f.store, f.home(), &f.pair("claude_code"), NOW).unwrap();
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
    assign_skill_to_project_tools(&f.store, &project.id, &f.skill_id, NOW).unwrap();
    let project_target = Path::new(&project.path).join(".claude/skills/alpha");
    assert!(exists_any(&project_target));

    let report = disable_targets(&f.store, f.home(), &f.skill(), NOW).unwrap();

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
    assign_skill_to_project_tools(&f.store, &project.id, &f.skill_id, NOW).unwrap();
    disable_targets(&f.store, f.home(), &f.skill(), NOW).unwrap();

    // Refresh + re-assert: codex was never a target; it must not become one.
    let targets = f.refresh_with_reassert();
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
    let resync = resync_project(&f.store, &project.id, NOW).unwrap();
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
    disable_targets(&f.store, f.home(), &f.pair("claude_code"), NOW).unwrap();
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
    disable_targets(&f.store, f.home(), &f.skill(), NOW).unwrap();
    let project = project(&f, "claude_code");

    let report = assign_skill_to_project_tools(&f.store, &project.id, &f.skill_id, NOW).unwrap();

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
    assign_skill_to_project_tools(&f.store, &project.id, &f.skill_id, NOW).unwrap();
    disable_targets(&f.store, f.home(), &f.skill(), NOW).unwrap();

    let report: EnableReport = enable_targets(&f.store, f.home(), &f.skill(), NOW).unwrap();

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

/// Only a skill-grain Enable lifts a skill-grain Disable.
#[test]
fn enabling_a_pair_of_a_disabled_skill_is_skipped() {
    let f = fixture(&["claude_code"]);
    f.sync(&["claude_code"]);
    disable_targets(&f.store, f.home(), &f.skill(), NOW).unwrap();

    let report = enable_targets(&f.store, f.home(), &f.pair("claude_code"), NOW).unwrap();

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

    let report = disable_targets(&f.store, f.home(), &scope, NOW).unwrap();
    assert_eq!(report.removed_rows(), 1);
    assert!(f.store.skill_disabled_at(&f.skill_id).unwrap().is_some());

    let enabled = enable_targets(&f.store, f.home(), &scope, NOW).unwrap();
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
        disable_targets(&f.store, f.home(), &scope, NOW).unwrap_err(),
        enable_targets(&f.store, f.home(), &scope, NOW).unwrap_err(),
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
        serde_json::json!({ "global": [], "projects": { "items": [] } })
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
            disable_targets(&f.store, f.home(), &f.pair("claude_code"), NOW).unwrap();
            foreign_folder(path);
        }),
        ("foreign folder, then marker", |f, path| {
            foreign_folder(path);
            let report = disable_targets(&f.store, f.home(), &f.pair("claude_code"), NOW).unwrap();
            assert!(report.targets.is_empty(), "Disable touches no unowned path");
            assert!(foreign_bytes_survive(path));
        }),
        ("marker, foreign folder, Enable collides", |f, path| {
            disable_targets(&f.store, f.home(), &f.pair("claude_code"), NOW).unwrap();
            foreign_folder(path);
            let enabled = enable_targets(&f.store, f.home(), &f.pair("claude_code"), NOW).unwrap();
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

        let report = remove_skill(&f.store, &f.skill_id).unwrap();

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
    disable_targets(&f.store, f.home(), &f.skill(), NOW).unwrap();
    let project = project(&f, "claude_code");
    assign_skill_to_project_tools(&f.store, &project.id, &f.skill_id, NOW).unwrap();
    let path = Path::new(&project.path).join(".claude/skills/alpha");
    foreign_folder(&path);

    let report = unassign_skill_from_project(&f.store, &project.id, &f.skill_id).unwrap();

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
    disable_targets(&f.store, f.home(), &f.skill(), NOW).unwrap();
    let project = project(f, "claude_code");
    assign_skill_to_project_tools(&f.store, &project.id, &f.skill_id, NOW).unwrap();
    let path = Path::new(&project.path).join(".claude/skills/alpha");
    foreign_folder(&path);

    let enabled = enable_targets(&f.store, f.home(), &f.skill(), NOW).unwrap();

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
            let report = unassign_skill_from_project(&f.store, &project.id, &f.skill_id).unwrap();
            assert!(report.targets.is_empty(), "{report}");
            assert_eq!(report.cleared_markers.len(), 1);
        }),
        (
            "toggle-off",
            |f, project, _| match project_sync::toggle_skill_assignment(
                &f.store,
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
            let report =
                crate::core::project_ops::remove_project_and_artifacts(&f.store, &project.id)
                    .unwrap();
            assert!(report.targets.is_empty(), "{report}");
            assert_eq!(report.cleared_markers.len(), 1);
            assert!(f.store.get_project_by_id(&project.id).unwrap().is_none());
        }),
        (
            "Disable again, then Enable retries the collision",
            |f, project, _| {
                let report = disable_targets(&f.store, f.home(), &f.skill(), NOW).unwrap();
                assert!(report.targets.is_empty(), "{report}");
                let row = f
                    .store
                    .get_project_skill_assignment(&project.id, &f.skill_id, "claude_code")
                    .unwrap()
                    .unwrap();
                assert_eq!(row.status, SyncStatus::Disabled, "intent recorded");
                assert_eq!(row.synced_at, None, "still no owner");
                let enabled = enable_targets(&f.store, f.home(), &f.skill(), NOW).unwrap();
                assert!(matches!(enabled.projects.items.as_slice(), [item]
                if is_collision(&item.status)));
            },
        ),
        ("Delete the skill", |f, _, _| {
            let report = remove_skill(&f.store, &f.skill_id).unwrap();
            assert!(report.record_deleted, "{report}");
            assert!(report.targets.is_empty(), "{report}");
            assert_eq!(report.cleared_markers.len(), 1);
        }),
        ("resync one", |f, project, _| {
            let report = resync_project(&f.store, &project.id, NOW + 1).unwrap();
            assert!(
                is_collision(&item_for(&report, &f.skill_id).status),
                "{report}"
            );
        }),
        ("resync all", |f, _, _| {
            let report = resync_all_projects(&f.store, NOW + 1).unwrap();
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
    disable_targets(&f.store, f.home(), &f.pair("claude_code"), NOW).unwrap();
    let path = f.global_path(".claude/skills");
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("SKILL.md"), "reappeared").unwrap();

    let report = remove_skill(&f.store, &f.skill_id).unwrap();

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
    assign_skill_to_project_tools(&f.store, &project.id, &f.skill_id, NOW).unwrap();
    disable_targets(&f.store, f.home(), &f.skill(), NOW).unwrap();
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
    let kept = remove_skill(&f.store, &f.skill_id).unwrap();
    unlock_dir(&parent);
    assert!(!kept.record_deleted && !kept.central_removed);
    // The absent project artifact settled (its row deleted); only the
    // locked global one is kept.
    assert_eq!((kept.removed_rows(), kept.failed_rows()), (1, 1));
    assert_eq!(f.status("claude_code"), Some(SyncStatus::Error));
    assert!(central.is_dir(), "central copy kept for the retry");

    let report = remove_skill(&f.store, &f.skill_id).unwrap();

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
    assign_skill_to_project_tools(&f.store, &project.id, &f.skill_id, NOW).unwrap();
    let beta_source = tempfile::tempdir().unwrap();
    fs::write(
        beta_source.path().join("SKILL.md"),
        "---\nname: beta\n---\n",
    )
    .unwrap();
    let beta = install_local_skill(&f.paths, &f.store, beta_source.path(), Some("beta".into()))
        .unwrap()
        .skill_id;
    assign_skill_to_project_tools(&f.store, &project.id, &beta, NOW).unwrap();

    let skills_dir = Path::new(&project.path).join(".claude/skills");
    let alpha_target = skills_dir.join("alpha");
    if !lock_dir(&skills_dir) {
        return;
    }
    let report = disable_targets(&f.store, f.home(), &f.skill(), NOW).unwrap();
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
        resync_project(&f.store, &project.id, NOW + 1).unwrap(),
        resync_all_projects(&f.store, NOW + 2).unwrap(),
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
    let assign = assign_skill_to_project_tools(&f.store, &project.id, &f.skill_id, NOW).unwrap();
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
#[test]
fn missing_shared_sibling_disable_failure_does_not_mint_disabled() {
    let f = fixture(&["kimi_cli"]);
    f.sync(&["kimi_cli"]);
    mark_installed_in(f.home(), adapter_by_key("amp").unwrap());
    let shared = f.global_path(".config/agents/skills");
    assert!(exists_any(&shared));
    assert_eq!(f.status("amp"), None);

    let parent = f.home().join(".config/agents/skills");
    if !lock_dir(&parent) {
        return;
    }
    let failed = disable_targets(&f.store, f.home(), &f.pair("amp"), NOW).unwrap();
    unlock_dir(&parent);

    assert_eq!(failed.failed_rows(), 1, "{failed}");
    assert_eq!(f.status("kimi_cli"), Some(SyncStatus::Error));
    assert_eq!(f.status("amp"), None, "no marker while the artifact stays");
    assert!(exists_any(&shared));

    let retried = disable_targets(&f.store, f.home(), &f.pair("amp"), NOW).unwrap();

    assert_eq!((retried.removed_rows(), retried.failed_rows()), (1, 0));
    assert!(!exists_any(&shared));
    assert_eq!(f.status("kimi_cli"), Some(SyncStatus::Disabled));
    assert_eq!(f.status("amp"), Some(SyncStatus::Disabled));
}

/// Sketch 7: a project assignment on the `.agents/skills` virtual group is
/// disabled and enabled once, at its registry path.
#[test]
fn disabled_virtual_group_assignment_enables_once() {
    let f = fixture(&[]);
    let project = project(&f, "agents_skills");
    assign_skill_to_project_tools(&f.store, &project.id, &f.skill_id, NOW).unwrap();
    let target = Path::new(&project.path).join(".agents/skills/alpha");
    assert!(exists_any(&target));

    let disabled = disable_targets(&f.store, f.home(), &f.skill(), NOW).unwrap();
    assert_eq!(disabled.removed_rows(), 1);
    assert!(!exists_any(&target));

    let enabled = enable_targets(&f.store, f.home(), &f.skill(), NOW).unwrap();

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
