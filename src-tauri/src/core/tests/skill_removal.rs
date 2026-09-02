//! Tests for `core::skill_removal` — the plan/execute split behind
//! `delete_managed_skill`: global tool targets, project-scope artifacts, the
//! central copy, and the DB record, with per-target outcomes as data and the
//! typed `DeleteCleanupFailed` raised only by the composed entry point.

use crate::core::sync_status::{SyncMode, SyncStatus};
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::errors::SignalError;
use crate::core::project_sync;
use crate::core::skill_removal::{
    execute_skill_removal, plan_skill_removal, remove_skill, RemovalScope, RemovalTargetStatus,
};
use crate::core::skill_store::{
    ProjectRecord, ProjectSkillAssignmentRecord, SkillRecord, SkillStore, SkillTargetRecord,
};
use crate::core::tool_adapters::adapter_by_key;

fn make_store(base: &Path) -> SkillStore {
    let store = SkillStore::new(base.join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    store
}

fn make_skill_dir(base: &Path, name: &str) -> PathBuf {
    let dir = base.join(name);
    fs::create_dir_all(&dir).expect("create skill dir");
    fs::write(dir.join("SKILL.md"), "# Test Skill\n").expect("write SKILL.md");
    dir
}

fn seed_skill(store: &SkillStore, name: &str, central_path: &Path) -> SkillRecord {
    let skill = SkillRecord {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        description: None,
        source_type: "local".to_string(),
        source_ref: None,
        source_subpath: None,
        source_revision: None,
        central_path: central_path.to_string_lossy().to_string(),
        content_hash: None,
        created_at: 1,
        updated_at: 1,
        last_sync_at: None,
        last_seen_at: 1,
        status: "ok".to_string(),
    };
    store.upsert_skill(&skill).expect("upsert skill");
    skill
}

/// Seed a global target row pointing at a real directory on disk.
fn seed_global_target(store: &SkillStore, skill: &SkillRecord, tool: &str, root: &Path) -> PathBuf {
    let target = root.join(tool).join(&skill.name);
    fs::create_dir_all(&target).expect("create global target");
    fs::write(target.join("SKILL.md"), "# synced\n").unwrap();
    store
        .upsert_skill_target(&SkillTargetRecord {
            id: uuid::Uuid::new_v4().to_string(),
            skill_id: skill.id.clone(),
            tool: tool.to_string(),
            target_path: target.to_string_lossy().to_string(),
            mode: SyncMode::Copy,
            status: SyncStatus::Synced,
            last_error: None,
            synced_at: Some(1),
        })
        .expect("upsert target");
    target
}

fn seed_project(store: &SkillStore, base: &Path, name: &str) -> ProjectRecord {
    let dir = base.join(name);
    fs::create_dir_all(&dir).expect("create project dir");
    let project = ProjectRecord {
        id: uuid::Uuid::new_v4().to_string(),
        path: dir.to_string_lossy().to_string(),
        created_at: 1,
        updated_at: 1,
    };
    store.register_project(&project).expect("register project");
    project
}

/// Seed an assignment row with the given status and materialize its
/// project-scope target on disk (a real dir, so removal is observable).
fn seed_assignment(
    store: &SkillStore,
    project: &ProjectRecord,
    skill: &SkillRecord,
    tool: &str,
    status: SyncStatus,
) -> PathBuf {
    store
        .add_project_skill_assignment(&ProjectSkillAssignmentRecord {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project.id.clone(),
            skill_id: skill.id.clone(),
            skill_name: skill.name.clone(),
            tool: tool.to_string(),
            mode: SyncMode::Copy,
            status,
            last_error: None,
            synced_at: None,
            content_hash: None,
            created_at: 1,
        })
        .expect("add assignment");
    let adapter = adapter_by_key(tool).expect("adapter");
    let target =
        project_sync::resolve_project_sync_target(Path::new(&project.path), adapter, &skill.name);
    fs::create_dir_all(&target).expect("create project target");
    fs::write(target.join("SKILL.md"), "# synced\n").unwrap();
    target
}

fn exists_any(path: &Path) -> bool {
    path.symlink_metadata().is_ok()
}

// ---------------------------------------------------------------------------
// plan_skill_removal — pure DB reads
// ---------------------------------------------------------------------------

#[test]
fn plan_collects_global_and_project_targets_using_project_scope_mapping() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "alpha");
    let skill = seed_skill(&store, "alpha", &central);
    let global = seed_global_target(&store, &skill, "claude_code", &tmp.path().join("home"));
    let project = seed_project(&store, tmp.path(), "proj");
    // pi's global and project mappings diverge — the plan must use the project one.
    let pi = adapter_by_key("pi").unwrap();
    assert_ne!(pi.relative_skills_dir, pi.project_relative_skills_dir);
    let project_target = seed_assignment(&store, &project, &skill, "pi", SyncStatus::Synced);

    let plan = plan_skill_removal(&store, &skill.id).expect("plan");

    assert_eq!(
        plan.skill.as_ref().map(|s| s.id.as_str()),
        Some(skill.id.as_str())
    );
    let global_paths: Vec<&Path> = plan
        .targets
        .iter()
        .filter(|t| matches!(t.scope, RemovalScope::Global))
        .map(|t| t.path.as_path())
        .collect();
    assert_eq!(global_paths, vec![global.as_path()]);
    let project_paths: Vec<(&str, &Path)> = plan
        .targets
        .iter()
        .filter_map(|t| match &t.scope {
            RemovalScope::Project { project_id } => Some((project_id.as_str(), t.path.as_path())),
            _ => None,
        })
        .collect();
    assert_eq!(
        project_paths,
        vec![(project.id.as_str(), project_target.as_path())]
    );
}

#[test]
fn plan_includes_only_deployed_assignment_statuses() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "beta");
    let skill = seed_skill(&store, "beta", &central);
    let p_synced = seed_project(&store, tmp.path(), "p-synced");
    let p_stale = seed_project(&store, tmp.path(), "p-stale");
    let p_error = seed_project(&store, tmp.path(), "p-error");
    let p_pending = seed_project(&store, tmp.path(), "p-pending");
    let p_missing = seed_project(&store, tmp.path(), "p-missing");
    seed_assignment(&store, &p_synced, &skill, "claude_code", SyncStatus::Synced);
    seed_assignment(&store, &p_stale, &skill, "claude_code", SyncStatus::Stale);
    seed_assignment(&store, &p_error, &skill, "claude_code", SyncStatus::Error);
    seed_assignment(
        &store,
        &p_pending,
        &skill,
        "claude_code",
        SyncStatus::Pending,
    );
    seed_assignment(
        &store,
        &p_missing,
        &skill,
        "claude_code",
        SyncStatus::Missing,
    );

    let plan = plan_skill_removal(&store, &skill.id).expect("plan");

    let mut planned: Vec<&str> = plan
        .targets
        .iter()
        .filter_map(|t| match &t.scope {
            RemovalScope::Project { project_id } => Some(project_id.as_str()),
            _ => None,
        })
        .collect();
    planned.sort();
    let mut expected = vec![
        p_synced.id.as_str(),
        p_stale.id.as_str(),
        p_error.id.as_str(),
    ];
    expected.sort();
    assert_eq!(planned, expected);
}

#[test]
fn plan_for_unknown_skill_still_lists_orphaned_global_targets() {
    // Legacy shape: skill row gone but a skill_targets row survives. The old
    // command still swept those targets; the plan preserves that.
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "gamma");
    let skill = seed_skill(&store, "gamma", &central);
    let global = seed_global_target(&store, &skill, "claude_code", &tmp.path().join("home"));
    {
        let conn = rusqlite::Connection::open(store.db_path()).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
        conn.execute(
            "DELETE FROM skills WHERE id = ?1",
            rusqlite::params![skill.id],
        )
        .unwrap();
    }

    let plan = plan_skill_removal(&store, &skill.id).expect("plan");
    assert!(plan.skill.is_none());
    assert_eq!(plan.targets.len(), 1);
    assert_eq!(plan.targets[0].path, global);
}

// ---------------------------------------------------------------------------
// execute_skill_removal — per-target outcomes as data
// ---------------------------------------------------------------------------

#[test]
fn execute_removes_global_project_central_and_record() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "delta");
    let skill = seed_skill(&store, "delta", &central);
    let global = seed_global_target(&store, &skill, "claude_code", &tmp.path().join("home"));
    let project = seed_project(&store, tmp.path(), "proj");
    let project_target = seed_assignment(&store, &project, &skill, "pi", SyncStatus::Synced);

    let plan = plan_skill_removal(&store, &skill.id).unwrap();
    let report = execute_skill_removal(&store, &skill.id, plan).expect("execute");

    assert!(!exists_any(&global), "global target should be gone");
    assert!(
        !exists_any(&project_target),
        "project target should be gone"
    );
    assert!(!central.exists(), "central copy should be gone");
    assert!(report.central_removed);
    assert!(report.record_deleted);
    assert!(store.get_skill_by_id(&skill.id).unwrap().is_none());
    assert!(
        store.list_skill_targets(&skill.id).unwrap().is_empty(),
        "skill_targets rows cascade with the skill row"
    );
    assert!(
        store
            .list_project_skill_assignments_by_skill(&skill.id)
            .unwrap()
            .is_empty(),
        "assignment rows cascade with the skill row"
    );
    assert_eq!(report.targets.len(), 2);
    assert!(report
        .targets
        .iter()
        .all(|t| matches!(t.status, RemovalTargetStatus::Removed)));
    assert!(report.failures().is_empty());
}

#[test]
fn execute_with_no_skill_row_sweeps_targets_without_touching_central() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "eps");
    let skill = seed_skill(&store, "eps", &central);
    let global = seed_global_target(&store, &skill, "claude_code", &tmp.path().join("home"));
    {
        let conn = rusqlite::Connection::open(store.db_path()).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
        conn.execute(
            "DELETE FROM skills WHERE id = ?1",
            rusqlite::params![skill.id],
        )
        .unwrap();
    }

    let plan = plan_skill_removal(&store, &skill.id).unwrap();
    let report = execute_skill_removal(&store, &skill.id, plan).expect("execute");

    assert!(!exists_any(&global));
    assert!(
        central.exists(),
        "no skill row ⇒ central path unknown ⇒ untouched"
    );
    assert!(!report.central_removed);
    assert!(!report.record_deleted);
}

#[test]
fn execute_tolerates_already_missing_targets() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "zeta");
    let skill = seed_skill(&store, "zeta", &central);
    let global = seed_global_target(&store, &skill, "claude_code", &tmp.path().join("home"));
    fs::remove_dir_all(&global).unwrap();

    let report = remove_skill(&store, &skill.id).expect("remove");
    assert!(matches!(
        report.targets[0].status,
        RemovalTargetStatus::Removed
    ));
    assert!(report.record_deleted);
}

#[cfg(unix)]
#[test]
fn execute_isolates_a_failing_target_and_still_deletes_the_record() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "eta");
    let skill = seed_skill(&store, "eta", &central);
    let home = tmp.path().join("home");
    let good = seed_global_target(&store, &skill, "claude_code", &home);
    let bad = seed_global_target(&store, &skill, "codex", &home);
    // A read-only directory with content cannot have its entries unlinked.
    fs::set_permissions(&bad, fs::Permissions::from_mode(0o555)).unwrap();
    if fs::remove_file(bad.join("SKILL.md")).is_ok() {
        // Running as root: permissions are not enforced; nothing to test here.
        return;
    }

    let plan = plan_skill_removal(&store, &skill.id).unwrap();
    let report = execute_skill_removal(&store, &skill.id, plan).expect("execute");

    // Restore permissions so tempdir cleanup succeeds.
    fs::set_permissions(&bad, fs::Permissions::from_mode(0o755)).unwrap();

    assert!(!exists_any(&good), "the healthy target is still removed");
    assert!(report.central_removed && report.record_deleted);
    let failures = report.failures();
    assert_eq!(failures.len(), 1);
    assert!(
        failures[0].starts_with(&format!("{}: ", bad.display())),
        "failure entry is `<path>: <error>`, got {:?}",
        failures[0]
    );
    let failed = report
        .targets
        .iter()
        .find(|t| matches!(t.status, RemovalTargetStatus::Failed { .. }))
        .expect("one failed outcome");
    assert_eq!(failed.path, bad);
    assert_eq!(failed.tool_key, "codex");
}

// ---------------------------------------------------------------------------
// remove_skill — composed entry point with the typed failure
// ---------------------------------------------------------------------------

#[test]
fn remove_skill_happy_path_returns_clean_report() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "theta");
    let skill = seed_skill(&store, "theta", &central);
    let project = seed_project(&store, tmp.path(), "proj");
    let project_target =
        seed_assignment(&store, &project, &skill, "claude_code", SyncStatus::Stale);

    let report = remove_skill(&store, &skill.id).expect("remove");

    assert!(report.failures().is_empty());
    assert!(!exists_any(&project_target));
    assert!(!central.exists());
    assert!(store.get_skill_by_id(&skill.id).unwrap().is_none());
}

#[cfg(unix)]
#[test]
fn remove_skill_raises_typed_delete_cleanup_failed_on_partial_failure() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "iota");
    let skill = seed_skill(&store, "iota", &central);
    let bad = seed_global_target(&store, &skill, "claude_code", &tmp.path().join("home"));
    fs::set_permissions(&bad, fs::Permissions::from_mode(0o555)).unwrap();
    if fs::remove_file(bad.join("SKILL.md")).is_ok() {
        return; // root: permissions not enforced
    }

    let err = remove_skill(&store, &skill.id).expect_err("must fail");
    fs::set_permissions(&bad, fs::Permissions::from_mode(0o755)).unwrap();

    match err.downcast_ref::<SignalError>() {
        Some(SignalError::DeleteCleanupFailed { failures }) => {
            assert_eq!(failures.len(), 1);
            assert!(failures[0].starts_with(&format!("{}: ", bad.display())));
        }
        other => panic!("expected DeleteCleanupFailed, got {:?}", other),
    }
    // The record is still deleted (matches the pre-existing command contract).
    assert!(store.get_skill_by_id(&skill.id).unwrap().is_none());
    assert!(!central.exists());
}

#[test]
fn remove_skill_cleans_project_artifacts_before_the_cascade() {
    // If the DB delete ran first, the FK cascade would drop the assignment
    // rows and the project artifact would be unreachable — it must be gone.
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "kappa");
    let skill = seed_skill(&store, "kappa", &central);
    let project = seed_project(&store, tmp.path(), "proj");
    let target = seed_assignment(&store, &project, &skill, "windsurf", SyncStatus::Synced);

    remove_skill(&store, &skill.id).expect("remove");

    assert!(!exists_any(&target));
    assert!(store
        .list_project_skill_assignments(&project.id)
        .unwrap()
        .is_empty());
}
