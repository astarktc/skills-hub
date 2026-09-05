//! Tests for `core::artifact_removal` — the plan/execute/report shape behind
//! every removal scope: one presence rule (a broken symlink is present), one
//! settlement rule (rows deleted on success, kept as `error` on failure —
//! ADR-0002), shared-skills-dir dedupe, and the typed `DeleteCleanupFailed`
//! raised only by the composed skill-deletion entry point.

use std::fs;
use std::path::{Path, PathBuf};

use crate::core::artifact_removal::{
    execute_unlocked, plan, remove_skill, unsync_all_skill_targets, unsync_skill_from_tool,
    unsync_skill_targets, RemovalScope, RemovalTargetStatus, RowRef,
};
use crate::core::errors::SignalError;
use crate::core::project_sync;
use crate::core::skill_store::{
    ProjectRecord, ProjectSkillAssignmentRecord, SkillRecord, SkillStore, SkillTargetRecord,
};
use crate::core::sync_status::{SyncMode, SyncStatus};
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
    seed_global_target_at(store, skill, tool, &target);
    target
}

/// Seed a global target row at an explicit path (shared skills dirs put two
/// rows on one path).
fn seed_global_target_at(store: &SkillStore, skill: &SkillRecord, tool: &str, target: &Path) {
    if target.symlink_metadata().is_err() {
        fs::create_dir_all(target).expect("create global target");
        fs::write(target.join("SKILL.md"), "# synced\n").unwrap();
    }
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

/// A home where `tool` is installed, so the shared-dir group probe finds it.
fn home_with(base: &Path, tools: &[&str]) -> PathBuf {
    let home = base.join("home");
    for tool in tools {
        let adapter = adapter_by_key(tool).expect("adapter");
        fs::create_dir_all(home.join(adapter.relative_detect_dir)).expect("detect dir");
    }
    fs::create_dir_all(&home).expect("home");
    home
}

/// Make `dir` unremovable by making its parent read-only. Returns `false`
/// when permissions are not enforced (running as root), so the test can skip.
#[cfg(unix)]
fn make_unremovable(dir: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o555)).unwrap();
    // Root ignores permission bits: if we can still unlink, there is nothing
    // to test here.
    if fs::remove_file(dir.join("SKILL.md")).is_ok() {
        fs::set_permissions(dir, fs::Permissions::from_mode(0o755)).unwrap();
        return false;
    }
    true
}

#[cfg(unix)]
fn restore_permissions(dir: &Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o755)).unwrap();
}

// ---------------------------------------------------------------------------
// Planning — pure store reads, one target per path
// ---------------------------------------------------------------------------

#[test]
fn skill_scope_plans_global_and_project_targets_using_project_scope_mapping() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "alpha");
    let skill = seed_skill(&store, "alpha", &central);
    let global = seed_global_target(&store, &skill, "claude_code", &tmp.path().join("tools"));
    let project = seed_project(&store, tmp.path(), "proj");
    // pi's global and project mappings diverge — the plan must use the project one.
    let pi = adapter_by_key("pi").unwrap();
    assert_ne!(pi.relative_skills_dir, pi.project_relative_skills_dir);
    let project_target = seed_assignment(&store, &project, &skill, "pi", SyncStatus::Synced);

    let planned = plan(
        &store,
        &RemovalScope::Skill {
            skill_id: skill.id.clone(),
        },
    )
    .expect("plan");

    assert_eq!(
        planned.skill.as_ref().map(|s| s.id.as_str()),
        Some(skill.id.as_str())
    );
    let paths: Vec<&Path> = planned.targets.iter().map(|t| t.path.as_path()).collect();
    assert_eq!(paths, vec![global.as_path(), project_target.as_path()]);
    assert!(matches!(
        planned.targets[0].rows[0],
        RowRef::GlobalTarget { .. }
    ));
    match &planned.targets[1].rows[0] {
        RowRef::Assignment { project_id, .. } => assert_eq!(project_id, &project.id),
        other => panic!("expected an assignment row, got {other:?}"),
    }
}

#[test]
fn plan_dedupes_a_shared_skills_dir_into_one_target_with_every_member_row() {
    // amp and kimi_cli share ~/.config/agents/skills: one artifact, two rows.
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let home = home_with(tmp.path(), &["amp"]);
    let central = make_skill_dir(&tmp.path().join("central"), "shared");
    let skill = seed_skill(&store, "shared", &central);
    let shared = home.join(".config/agents/skills/shared");
    seed_global_target_at(&store, &skill, "amp", &shared);
    seed_global_target_at(&store, &skill, "kimi_cli", &shared);

    let planned = plan(
        &store,
        &RemovalScope::SkillGlobal {
            skill_id: skill.id.clone(),
        },
    )
    .expect("plan");

    assert_eq!(planned.targets.len(), 1, "one path ⇒ one target");
    let mut tools: Vec<&str> = planned.targets[0].rows.iter().map(RowRef::tool).collect();
    tools.sort();
    assert_eq!(tools, vec!["amp", "kimi_cli"]);
    assert!(planned.skill.is_none(), "global scope keeps the skill row");
}

#[test]
fn skill_tool_scope_expands_the_shared_dir_group_and_leaves_other_tools_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let home = home_with(tmp.path(), &["amp", "claude_code"]);
    let central = make_skill_dir(&tmp.path().join("central"), "grp");
    let skill = seed_skill(&store, "grp", &central);
    let shared = home.join(".config/agents/skills/grp");
    seed_global_target_at(&store, &skill, "amp", &shared);
    seed_global_target_at(&store, &skill, "kimi_cli", &shared);
    let claude = seed_global_target(&store, &skill, "claude_code", &tmp.path().join("tools"));

    let planned = plan(
        &store,
        &RemovalScope::SkillTool {
            skill_id: skill.id.clone(),
            tool_key: "amp".to_string(),
            home: home.clone(),
        },
    )
    .expect("plan");

    assert_eq!(planned.targets.len(), 1);
    assert_eq!(planned.targets[0].path, shared);
    assert_eq!(planned.targets[0].rows.len(), 2);
    assert!(exists_any(&claude), "another tool's artifact is untouched");
}

#[test]
fn skill_tool_scope_plans_nothing_when_no_group_tool_is_installed() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let home = home_with(tmp.path(), &[]);
    let central = make_skill_dir(&tmp.path().join("central"), "nope");
    let skill = seed_skill(&store, "nope", &central);
    seed_global_target(&store, &skill, "claude_code", &tmp.path().join("tools"));

    let planned = plan(
        &store,
        &RemovalScope::SkillTool {
            skill_id: skill.id.clone(),
            tool_key: "claude_code".to_string(),
            home: home.clone(),
        },
    )
    .expect("plan");

    assert!(planned.targets.is_empty());
}

#[test]
fn project_scopes_plan_the_projects_assignments() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "beta");
    let skill = seed_skill(&store, "beta", &central);
    let project = seed_project(&store, tmp.path(), "proj");
    let other = seed_project(&store, tmp.path(), "other");
    let claude = seed_assignment(&store, &project, &skill, "claude_code", SyncStatus::Synced);
    let pi = seed_assignment(&store, &project, &skill, "pi", SyncStatus::Pending);
    let elsewhere = seed_assignment(&store, &other, &skill, "claude_code", SyncStatus::Synced);

    let whole_project = plan(
        &store,
        &RemovalScope::Project {
            project_id: project.id.clone(),
        },
    )
    .expect("plan");
    let mut paths: Vec<PathBuf> = whole_project
        .targets
        .iter()
        .map(|t| t.path.clone())
        .collect();
    paths.sort();
    let mut expected = vec![claude.clone(), pi];
    expected.sort();
    assert_eq!(paths, expected, "every assignment row, whatever its status");
    assert!(exists_any(&elsewhere), "another project is untouched");

    let one_tool = plan(
        &store,
        &RemovalScope::ProjectTool {
            project_id: project.id.clone(),
            tool_key: "claude_code".to_string(),
        },
    )
    .expect("plan");
    assert_eq!(
        one_tool
            .targets
            .iter()
            .map(|t| t.path.clone())
            .collect::<Vec<_>>(),
        vec![claude]
    );
}

#[test]
fn every_global_target_scope_plans_every_skills_global_rows() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = tmp.path().join("central");
    let a = seed_skill(&store, "a", &make_skill_dir(&central, "a"));
    let b = seed_skill(&store, "b", &make_skill_dir(&central, "b"));
    let tools = tmp.path().join("tools");
    let a_target = seed_global_target(&store, &a, "claude_code", &tools);
    let b_target = seed_global_target(&store, &b, "claude_code", &tools);
    let project = seed_project(&store, tmp.path(), "proj");
    let project_target = seed_assignment(&store, &project, &a, "claude_code", SyncStatus::Synced);

    let planned = plan(&store, &RemovalScope::EveryGlobalTarget).expect("plan");

    let paths: Vec<&Path> = planned.targets.iter().map(|t| t.path.as_path()).collect();
    assert_eq!(paths.len(), 2);
    assert!(paths.contains(&a_target.as_path()));
    assert!(paths.contains(&b_target.as_path()));
    assert!(
        !paths.contains(&project_target.as_path()),
        "global scope never touches project artifacts"
    );
}

#[test]
fn plan_for_a_missing_skill_row_still_lists_orphaned_global_targets() {
    // Legacy shape: skill row gone but a skill_targets row survives.
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "gamma");
    let skill = seed_skill(&store, "gamma", &central);
    let global = seed_global_target(&store, &skill, "claude_code", &tmp.path().join("tools"));
    {
        let conn = rusqlite::Connection::open(store.db_path()).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
        conn.execute(
            "DELETE FROM skills WHERE id = ?1",
            rusqlite::params![skill.id],
        )
        .unwrap();
    }

    let planned = plan(
        &store,
        &RemovalScope::Skill {
            skill_id: skill.id.clone(),
        },
    )
    .expect("plan");
    assert!(planned.skill.is_none());
    assert_eq!(planned.targets.len(), 1);
    assert_eq!(planned.targets[0].path, global);
}

#[test]
fn plan_for_a_skill_with_no_rows_is_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let home = home_with(tmp.path(), &[]);

    for scope in [
        RemovalScope::SkillGlobal {
            skill_id: "ghost".to_string(),
        },
        RemovalScope::SkillTool {
            skill_id: "ghost".to_string(),
            tool_key: "claude_code".to_string(),
            home: home.clone(),
        },
        RemovalScope::Project {
            project_id: "ghost".to_string(),
        },
        RemovalScope::EveryGlobalTarget,
    ] {
        let planned = plan(&store, &scope).expect("plan");
        assert!(planned.targets.is_empty(), "{scope:?} plans nothing");
    }
}

// ---------------------------------------------------------------------------
// Execution — presence rule, settlement rule, per-target outcomes
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn a_broken_symlink_counts_as_present_and_is_removed() {
    use std::os::unix::fs::symlink;

    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "link");
    let skill = seed_skill(&store, "link", &central);
    let link = tmp.path().join("tools").join("broken-link");
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    symlink(tmp.path().join("does-not-exist"), &link).unwrap();
    assert!(!link.exists(), "the link's destination is gone");
    seed_global_target_at(&store, &skill, "claude_code", &link);

    let report = unsync_skill_targets(&store, &skill.id).expect("unsync");

    assert!(!exists_any(&link), "the dangling link itself is removed");
    assert_eq!(report.removed_rows(), 1);
    assert!(store.list_skill_targets(&skill.id).unwrap().is_empty());
}

#[test]
fn an_already_absent_artifact_is_a_successful_removal_and_its_row_goes() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "zeta");
    let skill = seed_skill(&store, "zeta", &central);
    let global = seed_global_target(&store, &skill, "claude_code", &tmp.path().join("tools"));
    fs::remove_dir_all(&global).unwrap();

    let report = unsync_skill_targets(&store, &skill.id).expect("unsync");

    assert!(matches!(
        report.targets[0].status,
        RemovalTargetStatus::Removed
    ));
    assert!(store.list_skill_targets(&skill.id).unwrap().is_empty());
}

#[test]
fn removal_of_a_skill_with_no_rows_reports_nothing_and_still_deletes_the_skill() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "solo");
    let skill = seed_skill(&store, "solo", &central);

    let report = remove_skill(&store, &skill.id).expect("remove");

    assert!(report.targets.is_empty());
    assert_eq!(report.removed_rows(), 0);
    assert!(report.record_deleted && report.central_removed);
    assert!(store.get_skill_by_id(&skill.id).unwrap().is_none());
}

#[test]
fn a_shared_dir_artifact_is_removed_once_and_every_member_row_is_settled() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let home = home_with(tmp.path(), &["amp"]);
    let central = make_skill_dir(&tmp.path().join("central"), "shared");
    let skill = seed_skill(&store, "shared", &central);
    let shared = home.join(".config/agents/skills/shared");
    seed_global_target_at(&store, &skill, "amp", &shared);
    seed_global_target_at(&store, &skill, "kimi_cli", &shared);

    let report = unsync_skill_from_tool(&store, &home, &skill.id, "amp").expect("unsync");

    assert!(!exists_any(&shared));
    assert_eq!(report.targets.len(), 1, "one artifact, one removal");
    assert_eq!(report.removed_rows(), 2, "both member rows settled");
    assert!(store.list_skill_targets(&skill.id).unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn a_failed_removal_keeps_every_attached_row_with_status_error() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let home = home_with(tmp.path(), &["amp"]);
    let central = make_skill_dir(&tmp.path().join("central"), "stuck");
    let skill = seed_skill(&store, "stuck", &central);
    let shared = home.join(".config/agents/skills/stuck");
    seed_global_target_at(&store, &skill, "amp", &shared);
    seed_global_target_at(&store, &skill, "kimi_cli", &shared);
    if !make_unremovable(&shared) {
        return; // running as root
    }

    let report = unsync_skill_targets(&store, &skill.id).expect("unsync reports failures");
    restore_permissions(&shared);

    assert_eq!(report.failed_rows(), 2);
    assert_eq!(report.removed_rows(), 0);
    assert_eq!(report.failures().len(), 1);
    assert!(report.failures()[0].starts_with(&format!("{}: ", shared.display())));

    let rows = store.list_skill_targets(&skill.id).expect("rows");
    assert_eq!(rows.len(), 2, "rows are kept, never deleted blind");
    for row in rows {
        assert_eq!(row.status, SyncStatus::Error);
        assert!(row.last_error.is_some(), "the diagnostic is recorded");
    }
}

/// The report carries the failure as an error value, not rendered text, so
/// the command seam can classify it (a typed `SignalError` becomes its own
/// wire code rather than `OTHER`). Here the chain bottoms out in the io error.
#[cfg(unix)]
#[test]
fn a_failed_removal_reports_the_error_chain_not_its_rendering() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let home = home_with(tmp.path(), &["amp"]);
    let central = make_skill_dir(&tmp.path().join("central"), "stuck");
    let skill = seed_skill(&store, "stuck", &central);
    let shared = home.join(".config/agents/skills/stuck");
    seed_global_target_at(&store, &skill, "amp", &shared);
    if !make_unremovable(&shared) {
        return; // running as root
    }

    let report = unsync_skill_targets(&store, &skill.id).expect("unsync reports failures");
    restore_permissions(&shared);

    assert_eq!(report.targets.len(), 1);
    match &report.targets[0].status {
        RemovalTargetStatus::Failed { error } => {
            assert!(
                error
                    .root_cause()
                    .downcast_ref::<std::io::Error>()
                    .is_some(),
                "the io failure is still downcastable through the chain: {error:#}"
            );
            assert!(error.chain().count() > 1, "context layers are kept");
        }
        RemovalTargetStatus::Removed => panic!("the stuck artifact must report Failed"),
    }
}

#[cfg(unix)]
#[test]
fn a_failed_project_artifact_keeps_its_assignment_row_with_status_error() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "pstuck");
    let skill = seed_skill(&store, "pstuck", &central);
    let project = seed_project(&store, tmp.path(), "proj");
    let target = seed_assignment(&store, &project, &skill, "claude_code", SyncStatus::Synced);
    if !make_unremovable(&target) {
        return;
    }

    let planned = plan(
        &store,
        &RemovalScope::Project {
            project_id: project.id.clone(),
        },
    )
    .expect("plan");
    let report = execute_unlocked(&store, planned).expect("execute");
    restore_permissions(&target);

    assert_eq!(report.failed_rows(), 1);
    let rows = store
        .list_project_skill_assignments(&project.id)
        .expect("rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, SyncStatus::Error);
    assert!(rows[0].last_error.is_some());
}

#[cfg(unix)]
#[test]
fn a_partial_failure_isolates_the_healthy_target() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "eta");
    let skill = seed_skill(&store, "eta", &central);
    let tools = tmp.path().join("tools");
    let good = seed_global_target(&store, &skill, "claude_code", &tools);
    let bad = seed_global_target(&store, &skill, "codex", &tools);
    if !make_unremovable(&bad) {
        return;
    }

    let report = unsync_skill_targets(&store, &skill.id).expect("unsync");
    restore_permissions(&bad);

    assert!(!exists_any(&good), "the healthy target is still removed");
    assert_eq!(report.removed_rows(), 1);
    assert_eq!(report.failed_rows(), 1);
    assert!(store
        .get_skill_target(&skill.id, "claude_code")
        .unwrap()
        .is_none());
    let kept = store
        .get_skill_target(&skill.id, "codex")
        .unwrap()
        .expect("failed row kept");
    assert_eq!(kept.status, SyncStatus::Error);
}

/// A stat error that is not `NotFound` must not read as absence: the target
/// may well still be there, so removal is attempted, fails, and the row is
/// kept with Sync status `error` (ADR-0002).
#[cfg(unix)]
#[test]
fn an_unstattable_target_is_treated_as_present_and_keeps_its_row_with_status_error() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "blind");
    let skill = seed_skill(&store, "blind", &central);
    let tools = tmp.path().join("tools");
    let target = seed_global_target(&store, &skill, "claude_code", &tools);
    let parent = target.parent().expect("parent").to_path_buf();
    // A search-permission-less parent makes `symlink_metadata` fail with
    // EACCES rather than NotFound.
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o000)).unwrap();
    if target.symlink_metadata().is_ok() {
        // Root ignores permission bits: there is nothing to test here.
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }

    let report = unsync_skill_targets(&store, &skill.id).expect("unsync");
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o755)).unwrap();

    assert_eq!(report.failed_rows(), 1, "the target reports Failed");
    assert_eq!(report.removed_rows(), 0);
    assert!(exists_any(&target), "the artifact is in fact still there");
    let kept = store
        .get_skill_target(&skill.id, "claude_code")
        .unwrap()
        .expect("row kept, never deleted on a stat error");
    assert_eq!(kept.status, SyncStatus::Error);
    assert!(kept.last_error.is_some());
}

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

#[test]
fn remove_skill_removes_global_project_central_and_record() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "delta");
    let skill = seed_skill(&store, "delta", &central);
    let global = seed_global_target(&store, &skill, "claude_code", &tmp.path().join("tools"));
    let project = seed_project(&store, tmp.path(), "proj");
    let project_target = seed_assignment(&store, &project, &skill, "pi", SyncStatus::Synced);

    let report = remove_skill(&store, &skill.id).expect("remove");

    assert!(!exists_any(&global));
    assert!(!exists_any(&project_target));
    assert!(!central.exists());
    assert!(report.central_removed && report.record_deleted);
    assert!(store.get_skill_by_id(&skill.id).unwrap().is_none());
    assert!(store.list_skill_targets(&skill.id).unwrap().is_empty());
    assert!(store
        .list_project_skill_assignments(&project.id)
        .unwrap()
        .is_empty());
    assert!(report.failures().is_empty());
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

#[test]
fn remove_skill_with_no_record_sweeps_targets_without_touching_a_central_copy() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "eps");
    let skill = seed_skill(&store, "eps", &central);
    let global = seed_global_target(&store, &skill, "claude_code", &tmp.path().join("tools"));
    {
        let conn = rusqlite::Connection::open(store.db_path()).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
        conn.execute(
            "DELETE FROM skills WHERE id = ?1",
            rusqlite::params![skill.id],
        )
        .unwrap();
    }

    let report = remove_skill(&store, &skill.id).expect("remove");

    assert!(!exists_any(&global));
    assert!(
        central.exists(),
        "no skill row ⇒ central path unknown ⇒ untouched"
    );
    assert!(!report.central_removed && !report.record_deleted);
}

#[cfg(unix)]
#[test]
fn remove_skill_keeps_the_skill_and_raises_delete_cleanup_failed_on_partial_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "iota");
    let skill = seed_skill(&store, "iota", &central);
    let bad = seed_global_target(&store, &skill, "claude_code", &tmp.path().join("tools"));
    if !make_unremovable(&bad) {
        return;
    }

    let err = remove_skill(&store, &skill.id).expect_err("must fail");
    restore_permissions(&bad);

    match err.downcast_ref::<SignalError>() {
        Some(SignalError::DeleteCleanupFailed { failures }) => {
            assert_eq!(failures.len(), 1);
            assert!(failures[0].starts_with(&format!("{}: ", bad.display())));
        }
        other => panic!("expected DeleteCleanupFailed, got {:?}", other),
    }
    // ADR-0002: nothing is deleted blind — the skill, its central copy and
    // its row survive so the operator can retry.
    assert!(store.get_skill_by_id(&skill.id).unwrap().is_some());
    assert!(central.exists());
    let row = store
        .get_skill_target(&skill.id, "claude_code")
        .unwrap()
        .expect("row kept");
    assert_eq!(row.status, SyncStatus::Error);
}

#[test]
fn unsync_skill_targets_leaves_project_artifacts_and_the_skill_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = make_skill_dir(&tmp.path().join("central"), "theta");
    let skill = seed_skill(&store, "theta", &central);
    let global = seed_global_target(&store, &skill, "claude_code", &tmp.path().join("tools"));
    let project = seed_project(&store, tmp.path(), "proj");
    let project_target = seed_assignment(&store, &project, &skill, "pi", SyncStatus::Synced);

    let report = unsync_skill_targets(&store, &skill.id).expect("unsync");

    assert!(!exists_any(&global));
    assert!(exists_any(&project_target), "project artifact untouched");
    assert!(store.get_skill_by_id(&skill.id).unwrap().is_some());
    assert!(central.exists());
    assert_eq!(report.removed_rows(), 1);
    assert!(!report.record_deleted);
}

#[test]
fn unsync_all_skill_targets_sweeps_every_skill() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = tmp.path().join("central");
    let a = seed_skill(&store, "a", &make_skill_dir(&central, "a"));
    let b = seed_skill(&store, "b", &make_skill_dir(&central, "b"));
    let tools = tmp.path().join("tools");
    let a_target = seed_global_target(&store, &a, "claude_code", &tools);
    let b_target = seed_global_target(&store, &b, "codex", &tools);

    let report = unsync_all_skill_targets(&store).expect("unsync all");

    assert!(!exists_any(&a_target) && !exists_any(&b_target));
    assert_eq!(report.removed_rows(), 2);
    assert!(store.list_skill_targets(&a.id).unwrap().is_empty());
    assert!(store.list_skill_targets(&b.id).unwrap().is_empty());
    assert!(store.get_skill_by_id(&a.id).unwrap().is_some());
}

#[test]
fn unsync_skill_from_tool_with_an_uninstalled_group_touches_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let home = home_with(tmp.path(), &[]);
    let central = make_skill_dir(&tmp.path().join("central"), "idle");
    let skill = seed_skill(&store, "idle", &central);
    let target = seed_global_target(&store, &skill, "claude_code", &tmp.path().join("tools"));

    let report = unsync_skill_from_tool(&store, &home, &skill.id, "claude_code").expect("unsync");

    assert!(exists_any(&target), "nothing installed ⇒ nothing touched");
    assert!(report.targets.is_empty());
    assert!(store
        .get_skill_target(&skill.id, "claude_code")
        .unwrap()
        .is_some());
}
