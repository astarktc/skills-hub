//! Tests for `core::skill_health` — the stored last outcome of a skill — and
//! for every settlement site that writes it (Update / Restore / Re-point,
//! Edit replay, kept Delete), plus D9's target-row half: a failed first sync
//! records an `error` row that owns nothing, a declined pair records
//! `disabled`, an unanswered `TARGET_EXISTS` records nothing, and a project
//! collision is typed `TARGET_EXISTS`.

use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::core::artifact_removal::{remove_skill, RemovalTargetStatus};
use crate::core::errors::{CommandError, SignalError};
use crate::core::global_sync::{
    sync_skills_to_planned_tools, BatchOverride, BatchPolicy, BatchSkill, BatchTargetStatus,
    OverrideDecision, PlannedToolTarget,
};
use crate::core::installer::{install_local_skill, InstallerPaths};
use crate::core::manifest::InvocationMode;
use crate::core::refresh::{
    refresh_managed_skills, refresh_with_intent, RefreshPolicy, RefreshReport, RefreshSelection,
    SkillRefreshStatus, UpdateIntent,
};
use crate::core::skill_catalog::managed_skill_catalog;
use crate::core::sync_status::SyncStatus;
use crate::core::tool_adapters::{adapter_by_key, ToolAdapter};

struct Fixture {
    dir: tempfile::TempDir,
    paths: InstallerPaths,
    store: SkillStore,
    source: PathBuf,
    central: PathBuf,
    id: String,
}

/// One local-source Managed skill (`alpha`) in a temp central repo.
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = InstallerPaths {
        home: dir.path().join("home"),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.home).unwrap();
    let source = dir.path().join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("SKILL.md"), "---\nname: alpha\n---\nBody\n").unwrap();
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().unwrap();
    let installed = install_local_skill(&paths, &store, &source, Some("alpha".into())).unwrap();
    Fixture {
        dir,
        paths,
        store,
        source,
        central: installed.central_path,
        id: installed.skill_id,
    }
}

impl Fixture {
    fn update(&self) -> RefreshReport {
        refresh_managed_skills(
            &self.paths,
            &self.store,
            &crate::core::credentials::MemoryStore::new(),
            RefreshSelection::Ids(vec![self.id.clone()]),
            RefreshPolicy::default(),
            None,
            5000,
            |_| {},
        )
        .unwrap()
    }

    fn health(&self) -> Option<SkillHealth> {
        skill_health(&self.store, &self.id).unwrap()
    }

    fn batch_skill(&self) -> BatchSkill {
        BatchSkill {
            skill_id: self.id.clone(),
            skill_name: "alpha".into(),
            source_path: self.central.clone(),
        }
    }
}

fn failed_error(report: &RefreshReport) -> CommandError {
    match &report.skills[0].status {
        SkillRefreshStatus::Failed { error } => error.clone(),
        other => panic!("expected a failed row, got {other:?}"),
    }
}

fn json(error: &CommandError) -> serde_json::Value {
    serde_json::to_value(error).unwrap()
}

fn claude() -> &'static ToolAdapter {
    adapter_by_key("claude_code").expect("claude_code adapter")
}

fn planned(adapter: &'static ToolAdapter, root: &Path) -> PlannedToolTarget {
    PlannedToolTarget {
        adapter,
        root: root.to_path_buf(),
        installed: true,
        record_tools: vec![adapter],
    }
}

fn policy(overrides: Vec<BatchOverride>) -> BatchPolicy {
    BatchPolicy {
        overwrite: false,
        overwrite_if_same_content: true,
        overrides,
    }
}

// ---------------------------------------------------------------------------
// The store slot
// ---------------------------------------------------------------------------

#[test]
fn health_round_trips_the_tagged_error_and_crosses_the_wire_as_itself() {
    let f = fixture();
    assert!(f.health().is_none());
    let error = CommandError::GitCloneFailed {
        kind: crate::core::errors::GitCloneFailureKind::Timeout,
        detail: "timed out".into(),
    };
    record_outcome(&f.store, &f.id, SkillOutcomeKind::FetchFailed, &error).unwrap();
    let health = f.health().expect("recorded");
    assert_eq!(health.kind, SkillOutcomeKind::FetchFailed);
    assert_eq!(json(&health.error), json(&error));
    assert!(health.at > 0);
    let wire = serde_json::to_value(&health).unwrap();
    assert_eq!(wire["kind"], "fetch_failed");
    assert_eq!(wire["error"]["code"], "GIT_CLONE_FAILED");
    assert_eq!(wire["error"]["kind"], "timeout");

    // The catalog carries it; a whole-record upsert never touches it.
    let record = f.store.get_skill_by_id(&f.id).unwrap().unwrap();
    f.store.upsert_skill(&record).unwrap();
    let catalog = managed_skill_catalog(&f.store).unwrap();
    assert_eq!(
        catalog[0].health.as_ref().map(|h| h.kind),
        Some(SkillOutcomeKind::FetchFailed)
    );
}

#[test]
fn a_success_clears_only_its_own_family() {
    let f = fixture();
    let error = CommandError::Other {
        message: "x".into(),
    };
    record_outcome(&f.store, &f.id, SkillOutcomeKind::EditConflict, &error).unwrap();
    clear_outcome(&f.store, &f.id, OutcomeFamily::Update).unwrap();
    assert_eq!(
        f.health().map(|h| h.kind),
        Some(SkillOutcomeKind::EditConflict)
    );
    clear_outcome(&f.store, &f.id, OutcomeFamily::Edit).unwrap();
    assert!(f.health().is_none());

    for kind in [
        SkillOutcomeKind::FetchFailed,
        SkillOutcomeKind::RepointFailed,
        SkillOutcomeKind::RestoreFailed,
    ] {
        record_outcome(&f.store, &f.id, kind, &error).unwrap();
        clear_outcome(&f.store, &f.id, OutcomeFamily::Edit).unwrap();
        assert_eq!(f.health().map(|h| h.kind), Some(kind));
        clear_outcome(&f.store, &f.id, OutcomeFamily::Update).unwrap();
        assert!(f.health().is_none(), "{kind:?}");
    }
}

#[test]
fn an_unknown_kind_is_dropped_and_an_unparsable_error_reads_as_other() {
    let f = fixture();
    f.store
        .set_skill_outcome(
            &f.id,
            Some(&StoredOutcome {
                kind: "from_the_future".into(),
                error: "{}".into(),
                at: 1,
            }),
        )
        .unwrap();
    assert!(f.health().is_none());
    assert!(all_skill_health(&f.store).unwrap().is_empty());

    f.store
        .set_skill_outcome(
            &f.id,
            Some(&StoredOutcome {
                kind: "delete_incomplete".into(),
                error: "{\"code\":\"NO_SUCH_CODE\"}".into(),
                at: 1,
            }),
        )
        .unwrap();
    let health = f.health().expect("kept");
    assert_eq!(health.kind, SkillOutcomeKind::DeleteIncomplete);
    assert!(matches!(health.error, CommandError::Other { .. }));
}

#[test]
fn v11_migration_adds_the_health_columns_to_a_v10_database() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("v10.db");
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(
            "CREATE TABLE skills (
               id TEXT PRIMARY KEY, name TEXT NOT NULL, source_type TEXT NOT NULL,
               source_ref TEXT NULL, source_revision TEXT NULL,
               central_path TEXT NOT NULL UNIQUE, content_hash TEXT NULL,
               created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
               last_sync_at INTEGER NULL, last_seen_at INTEGER NOT NULL,
               status TEXT NOT NULL, description TEXT NULL, source_subpath TEXT NULL,
               imported_from_tool TEXT NULL, disabled_at INTEGER NULL);
             CREATE TABLE skill_targets (id TEXT PRIMARY KEY, skill_id TEXT NOT NULL,
               tool TEXT NOT NULL, target_path TEXT NOT NULL, mode TEXT NOT NULL,
               status TEXT NOT NULL, last_error TEXT NULL, synced_at INTEGER NULL,
               UNIQUE(skill_id, tool));
             CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE projects (id TEXT PRIMARY KEY, path TEXT NOT NULL UNIQUE,
               created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
             CREATE TABLE project_tools (id TEXT PRIMARY KEY, project_id TEXT NOT NULL,
               tool TEXT NOT NULL, UNIQUE(project_id, tool));
             CREATE TABLE project_skill_assignments (id TEXT PRIMARY KEY,
               project_id TEXT NOT NULL, skill_id TEXT NOT NULL, tool TEXT NOT NULL,
               mode TEXT NOT NULL, status TEXT NOT NULL, last_error TEXT NULL,
               synced_at INTEGER NULL, created_at INTEGER NOT NULL,
               content_hash TEXT NULL, skill_name TEXT NOT NULL DEFAULT '',
               UNIQUE(project_id, skill_id, tool));
             INSERT INTO skills (id, name, source_type, central_path, created_at,
               updated_at, last_seen_at, status) VALUES ('s1', 'alpha', 'local', '/c', 1, 1, 1, 'ok');
             PRAGMA user_version = 10;",
        )
        .unwrap();
    }
    let store = SkillStore::new(db);
    store.ensure_schema().unwrap();
    store.ensure_schema().expect("idempotent");
    assert!(skill_health(&store, "s1").unwrap().is_none());
    record_outcome(
        &store,
        "s1",
        SkillOutcomeKind::ImportFailed,
        &CommandError::Cancelled,
    )
    .unwrap();
    assert_eq!(
        skill_health(&store, "s1").unwrap().map(|h| h.kind),
        Some(SkillOutcomeKind::ImportFailed)
    );
}

// ---------------------------------------------------------------------------
// Update / Restore / Re-point settlement (refresh + skill_update)
// ---------------------------------------------------------------------------

#[test]
fn a_failed_update_records_fetch_failed_with_the_rows_error_and_the_next_success_clears() {
    let f = fixture();
    let moved = f.dir.path().join("moved");
    fs::rename(&f.source, &moved).unwrap();

    let report = f.update();
    let error = failed_error(&report);
    assert!(matches!(error, CommandError::SourcePathMissing { .. }));
    let health = f.health().expect("recorded at settlement");
    assert_eq!(health.kind, SkillOutcomeKind::FetchFailed);
    assert_eq!(json(&health.error), json(&error));

    fs::rename(&moved, &f.source).unwrap();
    assert!(matches!(
        f.update().skills[0].status,
        SkillRefreshStatus::Refreshed { .. }
    ));
    assert!(f.health().is_none(), "an Update success clears its family");
}

#[test]
fn a_failed_restore_records_restore_failed() {
    let f = fixture();
    fs::remove_dir_all(&f.central).unwrap();
    fs::remove_dir_all(&f.source).unwrap();

    let report = f.update();
    let error = failed_error(&report);
    let health = f.health().expect("recorded");
    assert_eq!(health.kind, SkillOutcomeKind::RestoreFailed);
    assert_eq!(json(&health.error), json(&error));
}

#[test]
fn a_failed_repoint_acquisition_records_repoint_failed() {
    let f = fixture();
    let report = refresh_with_intent(
        &f.paths,
        &f.store,
        RefreshSelection::Ids(vec![f.id.clone()]),
        RefreshPolicy::default(),
        None,
        5000,
        |_| {},
        &|_, _| {
            Err(anyhow::anyhow!(SignalError::GithubSkillNotFound {
                url: "https://github.com/o/r/tree/main/alpha".into(),
            }))
        },
        UpdateIntent::Repoint,
    )
    .unwrap();
    let error = failed_error(&report);
    let health = f.health().expect("recorded");
    assert_eq!(health.kind, SkillOutcomeKind::RepointFailed);
    assert_eq!(json(&health.error), json(&error));

    // A plain Update success is the same family: it clears it.
    f.update();
    assert!(f.health().is_none());
}

#[test]
fn a_refused_repoint_target_fails_the_command_and_records_nothing() {
    let f = fixture();
    let result = crate::core::repoint::repoint_skill_source(
        &f.paths,
        &f.store,
        &crate::core::credentials::MemoryStore::new(),
        &f.id,
        crate::core::repoint::RepointTarget::Local {
            path: f.dir.path().join("nowhere").to_string_lossy().into_owned(),
        },
        RefreshPolicy::default(),
        None,
        5000,
        |_| {},
    );
    assert!(result.is_err(), "a refused target is a whole-command error");
    assert!(
        f.health().is_none(),
        "a propagated error is never classified"
    );
}

#[test]
fn a_cancelled_acquisition_records_nothing() {
    let f = fixture();
    let report = refresh_with_intent(
        &f.paths,
        &f.store,
        RefreshSelection::Ids(vec![f.id.clone()]),
        RefreshPolicy::default(),
        None,
        5000,
        |_| {},
        &|_, _| Err(anyhow::anyhow!(SignalError::Cancelled)),
        UpdateIntent::Update,
    )
    .unwrap();
    // A cancellation is its own outcome kind, never a failure (D10).
    assert!(
        matches!(report.skills[0].status, SkillRefreshStatus::Cancelled),
        "{report:?}"
    );
    assert!(f.health().is_none());
}

#[test]
fn an_edit_replay_conflict_records_edit_conflict_and_an_edit_save_clears_it() {
    let f = fixture();
    crate::core::skill_edits::set_invocation_override(
        &f.paths,
        &f.store,
        &f.id,
        Some(InvocationMode::UserOnly),
    )
    .unwrap();
    fs::write(
        f.source.join("SKILL.md"),
        "---\nuser-invocable: false\nname: alpha\n---\nnew body\n",
    )
    .unwrap();
    let report = f.update();
    let SkillRefreshStatus::Refreshed {
        edit_conflict: Some(conflict),
        ..
    } = &report.skills[0].status
    else {
        panic!("{report:?}");
    };
    let health = f.health().expect("recorded");
    assert_eq!(health.kind, SkillOutcomeKind::EditConflict);
    assert_eq!(
        json(&health.error),
        json(&CommandError::EditConflict {
            base_mode: conflict.base_mode,
            upstream_mode: conflict.upstream_mode,
            override_mode: conflict.override_mode,
        })
    );
    assert_eq!(
        serde_json::to_value(&health.error).unwrap()["code"],
        "EDIT_CONFLICT"
    );

    // An unchanged Update keeps the unresolved conflict (the Edit row still
    // conflicts), even though it is an Update-family success.
    f.update();
    assert_eq!(
        f.health().map(|h| h.kind),
        Some(SkillOutcomeKind::EditConflict)
    );

    // Saving (here: clearing) the Edit clears the Edit family.
    let outcome =
        crate::core::skill_edits::set_invocation_override(&f.paths, &f.store, &f.id, None).unwrap();
    assert!(outcome.entry.health.is_none());
    assert!(f.health().is_none());
}

// ---------------------------------------------------------------------------
// Kept Delete (artifact_removal)
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn a_kept_delete_records_delete_incomplete_with_the_failed_targets_error() {
    use std::os::unix::fs::PermissionsExt;
    let f = fixture();
    let target = f.dir.path().join("tools").join("alpha");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("SKILL.md"), "# synced\n").unwrap();
    f.store
        .upsert_skill_target(&crate::core::skill_store::SkillTargetRecord {
            id: "t1".into(),
            skill_id: f.id.clone(),
            tool: "claude_code".into(),
            target_path: target.to_string_lossy().into_owned(),
            mode: crate::core::sync_status::SyncMode::Copy,
            status: SyncStatus::Synced,
            last_error: None,
            synced_at: Some(1),
        })
        .unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o555)).unwrap();
    if fs::remove_file(target.join("SKILL.md")).is_ok() {
        return; // permissions not enforced (root)
    }

    let report = remove_skill(&f.store, &f.id).unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(!report.record_deleted);
    let RemovalTargetStatus::Failed { error } = &report.targets[0].status else {
        panic!("{report:?}");
    };
    let health = f.health().expect("recorded");
    assert_eq!(health.kind, SkillOutcomeKind::DeleteIncomplete);
    assert_eq!(json(&health.error), json(error));

    // The retried Delete completes: the row, and its health, are gone.
    let report = remove_skill(&f.store, &f.id).unwrap();
    assert!(report.record_deleted);
    assert!(f.health().is_none());
}

// ---------------------------------------------------------------------------
// D9's target rows (global_sync, skill_disable, project_sync)
// ---------------------------------------------------------------------------

#[test]
fn a_failed_first_sync_records_an_error_row_that_owns_nothing() {
    let f = fixture();
    crate::core::tool_adapters::mark_installed_in(&f.paths.home, claude());
    // A file where the Tool's skills dir should be: the sync cannot land.
    let root = crate::core::tool_adapters::skills_dir_in(&f.paths.home, claude());
    fs::write(&root, "not a directory").unwrap();

    let outcomes = sync_skills_to_planned_tools(
        &f.store,
        &[f.batch_skill()],
        &[planned(claude(), &root)],
        &policy(Vec::new()),
        1000,
        |_| {},
    );
    assert!(matches!(
        outcomes[0].status,
        BatchTargetStatus::Failed { .. }
    ));
    let row = f
        .store
        .get_skill_target(&f.id, "claude_code")
        .unwrap()
        .expect("a failed first sync records a row");
    assert_eq!(row.status, SyncStatus::Error);
    assert!(row.last_error.is_some());
    assert_eq!(row.synced_at, None);
    assert!(!row.owns_artifact());
    assert_eq!(Path::new(&row.target_path), root.join("alpha"));

    // A foreign folder later at that path is never ours: Propagation reports
    // the collision and leaves it; Delete clears the row alone.
    fs::remove_file(&root).unwrap();
    fs::create_dir_all(root.join("alpha")).unwrap();
    fs::write(root.join("alpha").join("SKILL.md"), "foreign").unwrap();
    let propagation =
        crate::core::propagation::propagate_unlocked(&f.store, &f.paths, &f.id, 2000).unwrap();
    assert!(
        matches!(
            propagation.targets[0].status,
            crate::core::propagation::PropagationStatus::Failed {
                error: CommandError::TargetExists { .. }
            }
        ),
        "{propagation:?}"
    );
    let report = remove_skill(&f.store, &f.id).unwrap();
    assert!(report.record_deleted);
    assert_eq!(report.cleared_markers.len(), 1);
    assert_eq!(
        fs::read_to_string(root.join("alpha").join("SKILL.md")).unwrap(),
        "foreign"
    );
}

#[test]
fn a_target_exists_with_no_decision_records_no_row_and_a_kept_one_records_disabled() {
    let f = fixture();
    let root = f.dir.path().join("tools");
    fs::create_dir_all(root.join("alpha")).unwrap();
    fs::write(root.join("alpha").join("SKILL.md"), "someone else's").unwrap();

    let outcomes = sync_skills_to_planned_tools(
        &f.store,
        &[f.batch_skill()],
        &[planned(claude(), &root)],
        &policy(Vec::new()),
        1000,
        |_| {},
    );
    assert!(matches!(
        outcomes[0].status,
        BatchTargetStatus::Failed {
            error: CommandError::TargetExists { .. }
        }
    ));
    assert!(
        f.store
            .get_skill_target(&f.id, "claude_code")
            .unwrap()
            .is_none(),
        "asked first: no row until the operator decides"
    );

    let keep = vec![BatchOverride {
        skill_id: f.id.clone(),
        tool_key: "claude_code".into(),
        decision: OverrideDecision::Keep,
    }];
    let outcomes = sync_skills_to_planned_tools(
        &f.store,
        &[f.batch_skill()],
        &[planned(claude(), &root)],
        &policy(keep),
        1100,
        |_| {},
    );
    // The report row says what the operator did: declined, not skipped (D10).
    assert!(
        matches!(outcomes[0].status, BatchTargetStatus::Declined),
        "{:?}",
        outcomes[0].status
    );
    let row = f
        .store
        .get_skill_target(&f.id, "claude_code")
        .unwrap()
        .expect("a declined pair is recorded");
    assert_eq!(row.status, SyncStatus::Disabled);
    assert!(!row.owns_artifact());
    assert_eq!(
        fs::read_to_string(root.join("alpha").join("SKILL.md")).unwrap(),
        "someone else's"
    );

    // Every later pass skips it (the batch's Disabled gate) — no re-ask.
    let outcomes = sync_skills_to_planned_tools(
        &f.store,
        &[f.batch_skill()],
        &[planned(claude(), &root)],
        &policy(Vec::new()),
        1200,
        |_| {},
    );
    assert!(matches!(
        outcomes[0].status,
        BatchTargetStatus::Skipped {
            error: CommandError::TargetDisabled { .. }
        }
    ));
    // Delete never takes the kept folder.
    remove_skill(&f.store, &f.id).unwrap();
    assert!(root.join("alpha").join("SKILL.md").exists());
}

#[test]
fn an_overwrite_decision_still_replaces_the_folder() {
    let f = fixture();
    let root = f.dir.path().join("tools");
    fs::create_dir_all(root.join("alpha")).unwrap();
    fs::write(root.join("alpha").join("SKILL.md"), "someone else's").unwrap();
    let overwrite = vec![BatchOverride {
        skill_id: f.id.clone(),
        tool_key: "claude_code".into(),
        decision: OverrideDecision::Overwrite,
    }];
    let outcomes = sync_skills_to_planned_tools(
        &f.store,
        &[f.batch_skill()],
        &[planned(claude(), &root)],
        &policy(overwrite),
        1000,
        |_| {},
    );
    assert!(matches!(
        outcomes[0].status,
        BatchTargetStatus::Synced { .. }
    ));
    let row = f
        .store
        .get_skill_target(&f.id, "claude_code")
        .unwrap()
        .unwrap();
    assert_eq!(row.status, SyncStatus::Synced);
    assert!(row.owns_artifact());
}

#[test]
fn override_decisions_cross_the_wire_as_themselves() {
    assert_eq!(
        serde_json::from_str::<OverrideDecision>("\"keep\"").unwrap(),
        OverrideDecision::Keep
    );
    assert_eq!(
        serde_json::from_str::<OverrideDecision>("\"overwrite\"").unwrap(),
        OverrideDecision::Overwrite
    );
}

#[test]
fn an_enable_meeting_a_folder_in_the_way_keeps_the_pair_disabled() {
    use crate::core::skill_disable::{disable_targets, enable_targets, DisableScope};
    let f = fixture();
    crate::core::tool_adapters::mark_installed_in(&f.paths.home, claude());
    let scope = DisableScope::SkillTool {
        skill_id: f.id.clone(),
        tool: "claude_code".into(),
    };
    disable_targets(&f.store, &f.paths.home, &scope, 1000).unwrap();
    let marker = f
        .store
        .get_skill_target(&f.id, "claude_code")
        .unwrap()
        .unwrap();
    assert_eq!(marker.status, SyncStatus::Disabled);
    let path = PathBuf::from(&marker.target_path);
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("SKILL.md"), "foreign").unwrap();

    let report = enable_targets(&f.store, &f.paths.home, &scope, 2000).unwrap();
    assert!(matches!(
        report.global[0].status,
        BatchTargetStatus::Failed {
            error: CommandError::TargetExists { .. }
        }
    ));
    let row = f
        .store
        .get_skill_target(&f.id, "claude_code")
        .unwrap()
        .expect("the pair keeps its row");
    assert_eq!(row.status, SyncStatus::Disabled);
    assert_eq!(row.synced_at, None);
    assert_eq!(
        fs::read_to_string(path.join("SKILL.md")).unwrap(),
        "foreign"
    );
}

#[test]
fn a_project_first_sync_collision_is_typed_target_exists() {
    use crate::core::project_sync::{assign_skill_to_project_tools, ProjectSyncOutcomeStatus};
    use crate::core::skill_store::{ProjectRecord, ProjectToolRecord};
    let f = fixture();
    let project_dir = f.dir.path().join("proj");
    fs::create_dir_all(&project_dir).unwrap();
    let project = ProjectRecord {
        id: "p1".into(),
        path: project_dir.to_string_lossy().into_owned(),
        created_at: 1,
        updated_at: 1,
    };
    f.store.register_project(&project).unwrap();
    f.store
        .add_project_tool(&ProjectToolRecord {
            id: "pt1".into(),
            project_id: "p1".into(),
            tool: "claude_code".into(),
        })
        .unwrap();
    let target =
        crate::core::project_sync::resolve_project_sync_target(&project_dir, claude(), "alpha");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("SKILL.md"), "foreign").unwrap();

    let report = assign_skill_to_project_tools(&f.store, "p1", &f.id, 1000).unwrap();
    let ProjectSyncOutcomeStatus::Failed {
        error: CommandError::TargetExists { path },
    } = &report.items[0].status
    else {
        panic!("{report:?}");
    };
    assert_eq!(Path::new(path), target);
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "foreign"
    );
}

#[test]
fn onboarding_discovery_sees_past_a_row_that_owns_nothing() {
    let f = fixture();
    let root = f.dir.path().join("tools");
    fs::create_dir_all(root.join("alpha")).unwrap();
    let keep = vec![BatchOverride {
        skill_id: f.id.clone(),
        tool_key: "claude_code".into(),
        decision: OverrideDecision::Keep,
    }];
    sync_skills_to_planned_tools(
        &f.store,
        &[f.batch_skill()],
        &[planned(claude(), &root)],
        &policy(keep),
        1000,
        |_| {},
    );
    assert!(f
        .store
        .get_skill_target(&f.id, "claude_code")
        .unwrap()
        .is_some());
    assert!(
        f.store.list_all_skill_target_paths().unwrap().is_empty(),
        "a marker is not a managed target"
    );
}
