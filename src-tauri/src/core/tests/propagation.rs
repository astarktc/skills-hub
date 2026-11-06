//! Tests for `core::propagation` — bringing every Sync target of one Managed
//! skill into line after its central copy changed.
//!
//! Every case runs against a temp home / central dir / DB: installedness is
//! faked by creating a tool's detect dir under the temp home, so no test
//! touches the operator's real home or skill library. Outcomes are asserted
//! as report data — Propagation never fails the operation because one target
//! failed.

use crate::core::environment::HostRoots;
use crate::core::removal_fence::central_repo_of;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::installer::InstallerPaths;
use crate::core::propagation::{
    propagate_unlocked, PropagationOutcome, PropagationScope, PropagationSkip, PropagationStatus,
};
use crate::core::skill_store::{
    ProjectRecord, ProjectSkillAssignmentRecord, SkillRecord, SkillStore, SkillTargetRecord,
};
use crate::core::sync_status::{SyncMode, SyncStatus};
use crate::core::tool_adapters::{adapter_by_key, mark_installed_in, RegistryPath, ToolAdapter};

#[test]
fn shared_edit_update_report_preserves_classified_failure_on_the_wire() {
    use crate::core::errors::{CommandError, SignalError};
    let target = PropagationOutcome::new(
        PropagationScope::Global {
            tool: "kimi_cli".into(),
        },
        PropagationStatus::Failed {
            error: CommandError::from_anyhow(anyhow::anyhow!(SignalError::CentralPathMissing {
                path: "central".into(),
            })),
            first_deploy: false,
        },
    );
    assert_eq!(
        serde_json::to_value(target).unwrap(),
        serde_json::json!({
            "scope": { "scope": "global", "tool": "kimi_cli" },
            "folder_key": "amp",
            "status": { "status": "failed", "error": { "code": "CENTRAL_PATH_MISSING", "path": "central" }, "first_deploy": false }
        })
    );
    let skipped = PropagationOutcome::new(
        PropagationScope::Project {
            project_id: "p1".into(),
            tool: "trae_cn".into(),
        },
        PropagationStatus::Skipped {
            reason: PropagationSkip::LinkFollowsSource,
        },
    );
    assert_eq!(
        serde_json::to_value(skipped).unwrap(),
        serde_json::json!({
            "scope": { "scope": "project", "project_id": "p1", "tool": "trae_cn" },
            "folder_key": "trae",
            "status": { "status": "skipped", "reason": { "reason": "link_follows_source" } }
        })
    );
}

struct Fixture {
    dir: tempfile::TempDir,
    paths: InstallerPaths,
    store: SkillStore,
    skill_id: String,
    central_path: PathBuf,
}

/// One managed skill named `skill` in a temp central repo, with `a.txt`
/// holding `v2` (the "just refreshed" bytes Propagation must push).
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = InstallerPaths {
        roots: HostRoots::under_home(&dir.path().join("home")),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.roots.home).expect("create home");

    let central_path = paths.central_dir.join("skill");
    fs::create_dir_all(&central_path).expect("create central skill dir");
    fs::write(central_path.join("SKILL.md"), "---\nname: skill\n---\n").expect("write SKILL.md");
    fs::write(central_path.join("a.txt"), "v2").expect("write a.txt");

    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    let skill = SkillRecord {
        id: "skill-1".to_string(),
        name: "skill".to_string(),
        description: None,
        source_type: "local".to_string(),
        source_ref: None,
        source_subpath: None,
        source_revision: None,
        central_path: central_path.to_string_lossy().to_string(),
        content_hash: Some("hash-v2".to_string()),
        created_at: 1,
        updated_at: 1,
        last_sync_at: None,
        last_seen_at: 1,
        status: "ok".to_string(),
        imported_from_tool: None,
    };
    store.upsert_skill(&skill).expect("seed skill");

    Fixture {
        dir,
        paths,
        store,
        skill_id: "skill-1".to_string(),
        central_path,
    }
}

/// Fake `tool` as installed for the fixture's home by creating its detect dir.
fn install_tool(f: &Fixture, adapter: &ToolAdapter) {
    mark_installed_in(&f.paths.roots, adapter);
}

/// A registry shadow of Cursor with the symlink capability flipped off — no
/// shipped entry is copy-only any more, but the capability still decides.
fn copy_only_tool() -> &'static ToolAdapter {
    let mut adapter = adapter_by_key("cursor").expect("cursor adapter").clone();
    adapter.supports_symlink = false;
    crate::core::tool_adapters::test_overrides::shadow(adapter)
}

/// A global target row pointing at `target_path`, in `mode`.
fn seed_global_target(f: &Fixture, id: &str, tool: &str, target_path: &Path, mode: SyncMode) {
    f.store
        .upsert_skill_target(&SkillTargetRecord {
            id: id.to_string(),
            skill_id: f.skill_id.clone(),
            tool: tool.to_string(),
            target_path: target_path.to_string_lossy().to_string(),
            mode,
            status: SyncStatus::Synced,
            last_error: None,
            synced_at: Some(1),
        })
        .expect("seed target");
}

/// A stale copy of the skill at `target_path` (`a.txt` = `v1`).
fn seed_stale_copy(target_path: &Path) {
    fs::create_dir_all(target_path).expect("create target");
    fs::write(target_path.join("a.txt"), "v1").expect("write stale a.txt");
}

fn propagate(f: &Fixture) -> Vec<PropagationOutcome> {
    propagate_unlocked(&f.store, &f.paths, &f.skill_id, 2000)
        .expect("propagation reads its own rows")
        .targets
}

fn outcome_for<'a>(
    outcomes: &'a [PropagationOutcome],
    scope: &PropagationScope,
) -> &'a PropagationStatus {
    &outcomes
        .iter()
        .find(|o| &o.scope == scope)
        .unwrap_or_else(|| panic!("no outcome for {:?} in {:?}", scope, outcomes))
        .status
}

fn global(tool: &str) -> PropagationScope {
    PropagationScope::Global {
        tool: tool.to_string(),
    }
}

#[test]
fn a_copy_target_is_refreshed_from_the_central_copy() {
    let f = fixture();
    let cursor = copy_only_tool();
    install_tool(&f, cursor);
    let target = f.paths.roots.home.join(".cursor/skills/skill");
    seed_stale_copy(&target);
    seed_global_target(&f, "t-copy", "cursor", &target, SyncMode::Copy);

    let outcomes = propagate(&f);

    assert!(
        matches!(
            outcome_for(&outcomes, &global("cursor")),
            PropagationStatus::Synced {
                mode_used: SyncMode::Copy
            }
        ),
        "copy target should be resynced, got {:?}",
        outcomes
    );
    assert_eq!(
        fs::read_to_string(target.join("a.txt")).expect("read target"),
        "v2"
    );
    let row = f
        .store
        .get_skill_target(&f.skill_id, "cursor")
        .expect("query")
        .expect("row");
    assert_eq!(row.status, SyncStatus::Synced);
    assert_eq!(row.mode, SyncMode::Copy);
    assert_eq!(row.synced_at, Some(2000));
}

/// A drifting copy on a symlink-capable Tool is re-materialised through the
/// capability-aware sync entry point, which prefers a link — the row records
/// the mode actually used, so it stays truthful.
#[test]
fn a_drifting_copy_on_a_symlink_capable_tool_becomes_a_link() {
    let f = fixture();
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    install_tool(&f, claude);
    let target = f.paths.roots.home.join(".claude/skills/skill");
    seed_stale_copy(&target);
    seed_global_target(&f, "t-copy", "claude_code", &target, SyncMode::Copy);

    let outcomes = propagate(&f);

    assert!(
        matches!(
            outcome_for(&outcomes, &global("claude_code")),
            PropagationStatus::Synced {
                mode_used: SyncMode::Symlink
            }
        ),
        "got {:?}",
        outcomes
    );
    assert_eq!(
        fs::read_to_string(target.join("a.txt")).expect("read target"),
        "v2"
    );
    let row = f
        .store
        .get_skill_target(&f.skill_id, "claude_code")
        .expect("query")
        .expect("row");
    assert_eq!(row.mode, SyncMode::Symlink);
}

#[test]
fn a_symlink_target_is_left_untouched_because_it_follows_the_central_copy() {
    let f = fixture();
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    install_tool(&f, claude);
    let target = f.paths.roots.home.join(".claude/skills/skill");
    fs::create_dir_all(target.parent().expect("parent")).expect("create skills dir");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&f.central_path, &target).expect("symlink");
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&f.central_path, &target).expect("symlink");
    seed_global_target(&f, "t-link", "claude_code", &target, SyncMode::Symlink);

    let outcomes = propagate(&f);

    assert!(
        matches!(
            outcome_for(&outcomes, &global("claude_code")),
            PropagationStatus::Skipped {
                reason: PropagationSkip::LinkFollowsSource
            }
        ),
        "a link needs no work, got {:?}",
        outcomes
    );
    assert!(
        fs::symlink_metadata(&target)
            .expect("target still there")
            .file_type()
            .is_symlink(),
        "the link must not be replaced by a copy"
    );
}

#[test]
fn an_uninstalled_tool_is_skipped() {
    let f = fixture();
    // No detect dir under this home, and the row points at the skills dir of
    // a home the operator no longer has: claude_code is not installed.
    let target = f.dir.path().join("old-home/.claude/skills/skill");
    seed_stale_copy(&target);
    seed_global_target(&f, "t-gone", "claude_code", &target, SyncMode::Copy);

    let outcomes = propagate(&f);

    assert!(
        matches!(
            outcome_for(&outcomes, &global("claude_code")),
            PropagationStatus::Skipped {
                reason: PropagationSkip::ToolNotInstalled { .. }
            }
        ),
        "got {:?}",
        outcomes
    );
    assert_eq!(
        fs::read_to_string(target.join("a.txt")).expect("read target"),
        "v1",
        "an uninstalled tool's target must not be touched"
    );
}

/// Rows written once per shared-dir sibling before rows were per folder (Amp
/// and Kimi Code's `~/.config/agents/skills`) are each their own row: each is
/// synced at its own recorded path and settles itself, as before rows were
/// per folder.
#[test]
fn legacy_sibling_rows_each_sync_and_settle_by_themselves() {
    let f = fixture();
    let amp = adapter_by_key("amp").expect("amp adapter");
    let kimi = adapter_by_key("kimi_cli").expect("kimi_cli adapter");
    assert_eq!(amp.skills_dir, kimi.skills_dir);
    install_tool(&f, amp);
    install_tool(&f, kimi);
    let target = f.paths.roots.home.join(".config/agents/skills/skill");
    seed_stale_copy(&target);
    seed_global_target(&f, "t-amp", "amp", &target, SyncMode::Copy);
    seed_global_target(&f, "t-kimi", "kimi_cli", &target, SyncMode::Copy);

    let outcomes = propagate(&f);

    for tool in ["amp", "kimi_cli"] {
        assert!(
            matches!(
                outcome_for(&outcomes, &global(tool)),
                PropagationStatus::Synced { .. }
            ),
            "{} should be reported synced, got {:?}",
            tool,
            outcomes
        );
        let row = f
            .store
            .get_skill_target(&f.skill_id, tool)
            .expect("query")
            .unwrap_or_else(|| panic!("row for {}", tool));
        assert_eq!(row.status, SyncStatus::Synced);
        assert_eq!(row.synced_at, Some(2000));
    }
    assert_eq!(
        fs::read_to_string(target.join("a.txt")).expect("read target"),
        "v2"
    );
}

/// A row's own Tool is always a member of its shared skills dir group. The
/// group is the registry's answer, but a row resolves its Tool through
/// `adapter_by_key` — which a test may shadow with an adapter whose skills
/// dir no shipped entry shares. The row must still be written, not dropped
/// from the report because the static registry does not list its Tool.
#[test]
fn a_row_whose_tool_shares_no_dir_with_the_registry_is_still_its_own_group() {
    let f = fixture();
    let mut lone = adapter_by_key("cursor").expect("cursor adapter").clone();
    lone.skills_dir = RegistryPath::home(".lone-tool/skills");
    const LONE_DETECT: &[RegistryPath] = &[RegistryPath::home(".lone-tool")];
    lone.detect_dirs = LONE_DETECT;
    lone.supports_symlink = false;
    let lone = crate::core::tool_adapters::test_overrides::shadow(lone);
    install_tool(&f, lone);
    let target = f.paths.roots.home.join(".lone-tool/skills/skill");
    seed_stale_copy(&target);
    seed_global_target(&f, "t-lone", lone.key(), &target, SyncMode::Copy);

    let outcomes = propagate(&f);

    assert!(
        matches!(
            outcome_for(&outcomes, &global(lone.key())),
            PropagationStatus::Synced {
                mode_used: SyncMode::Copy
            }
        ),
        "the row's own Tool drives its group, got {:?}",
        outcomes
    );
    assert_eq!(
        fs::read_to_string(target.join("a.txt")).expect("read target"),
        "v2"
    );
    let row = f
        .store
        .get_skill_target(&f.skill_id, lone.key())
        .expect("query")
        .expect("row");
    assert_eq!(row.status, SyncStatus::Synced);
    assert_eq!(row.synced_at, Some(2000));
}

#[test]
fn a_missing_central_source_fails_every_row_as_report_data() {
    let f = fixture();
    let amp = adapter_by_key("amp").expect("amp adapter");
    install_tool(&f, amp);
    install_tool(&f, adapter_by_key("kimi_cli").expect("kimi_cli adapter"));
    let target = f.paths.roots.home.join(".config/agents/skills/skill");
    seed_stale_copy(&target);
    seed_global_target(&f, "t-amp", "amp", &target, SyncMode::Copy);
    seed_global_target(&f, "t-kimi", "kimi_cli", &target, SyncMode::Copy);
    fs::remove_dir_all(&f.central_path).expect("remove central copy");

    let outcomes = propagate(&f);

    for tool in ["amp", "kimi_cli"] {
        match outcome_for(&outcomes, &global(tool)) {
            PropagationStatus::Failed {
                error,
                first_deploy,
            } => {
                assert!(!first_deploy, "an owned copy failed to update");
                assert!(
                    matches!(error,
                        crate::core::errors::CommandError::InvalidPath { path, reason }
                        if path == &f.central_path.to_string_lossy() && reason == "missing"
                    ),
                    "a missing central copy is a typed condition, not prose"
                );
            }
            other => panic!("expected a failed target for {}, got {:?}", tool, other),
        }
        let row = f
            .store
            .get_skill_target(&f.skill_id, tool)
            .expect("query")
            .unwrap_or_else(|| panic!("row for {}", tool));
        assert_eq!(row.status, SyncStatus::Error);
    }
}

/// The shipped copy-only harness with a project folder of its own (Deep Agents
/// Code, `.deepagents/skills`): a project folder deploys with placement's
/// capability for it, so a copy-only shadow of a group constituent would be
/// served by the group's link-mode `.agents/skills` folder instead.
fn project_copy_only_tool() -> &'static ToolAdapter {
    let adapter = adapter_by_key("deep_agents_code").expect("deep agents adapter");
    assert!(!adapter.supports_symlink, "test premise: copy-only");
    adapter
}

/// The project folder [`seed_project_copy_assignment`]'s row describes.
fn project_folder_of_project_copy_only_tool() -> String {
    crate::core::placement::row_folder(
        crate::core::placement::ScopeKind::Project,
        project_copy_only_tool().key(),
    )
}

/// A project assignment row in copy mode on a copy-only Tool, with a stale
/// copy already on disk.
fn seed_project_copy_assignment(f: &Fixture, project_path: &Path) -> PathBuf {
    let tool = project_copy_only_tool();
    let project = ProjectRecord {
        id: "p1".to_string(),
        path: project_path.to_string_lossy().to_string(),
        created_at: 1,
        updated_at: 1,
    };
    f.store
        .register_project(&project)
        .expect("register project");
    f.store
        .add_project_skill_assignment(&ProjectSkillAssignmentRecord {
            id: "pa-copy".to_string(),
            project_id: "p1".to_string(),
            skill_id: f.skill_id.clone(),
            skill_name: "skill".to_string(),
            tool: tool.key().to_string(),
            mode: SyncMode::Copy,
            status: SyncStatus::Synced,
            last_error: None,
            synced_at: Some(1),
            content_hash: None,
            created_at: 1,
        })
        .expect("seed assignment");
    project_path
        .join(tool.project_skills_dir.unwrap())
        .join("skill")
}

#[test]
fn a_project_copy_is_refreshed_and_its_row_settled() {
    let f = fixture();
    let project_dir = tempfile::tempdir().expect("project tempdir");
    let target = seed_project_copy_assignment(&f, project_dir.path());
    seed_stale_copy(&target);

    let outcomes = propagate(&f);

    let scope = PropagationScope::Project {
        project_id: "p1".to_string(),
        tool: project_folder_of_project_copy_only_tool(),
    };
    assert!(
        matches!(
            outcome_for(&outcomes, &scope),
            PropagationStatus::Synced {
                mode_used: SyncMode::Copy
            }
        ),
        "got {:?}",
        outcomes
    );
    assert_eq!(
        fs::read_to_string(target.join("a.txt")).expect("read target"),
        "v2"
    );
    let row = f
        .store
        .get_project_skill_assignment("p1", &f.skill_id, project_copy_only_tool().key())
        .expect("query")
        .expect("row");
    assert_eq!(row.status, SyncStatus::Synced);
    assert_eq!(row.content_hash.as_deref(), Some("hash-v2"));
}

#[test]
fn propagation_backfills_a_lost_identity_and_reconcile_keeps_copies_synced() {
    let f = fixture();
    let mut skill = f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap();
    skill.content_hash = None;
    f.store.upsert_skill(&skill).unwrap();
    let project = tempfile::tempdir().unwrap();
    let target = seed_project_copy_assignment(&f, project.path());
    seed_stale_copy(&target);
    crate::core::mutation_guard::serialized(|| {
        propagate(&f);
        let mut rows = f.store.list_project_skill_assignments("p1").unwrap();
        assert!(rows[0].content_hash.is_some());
        crate::core::project_sync::reconcile_listing_unlocked(&f.store, "p1", &mut rows);
        assert_eq!(rows[0].status, SyncStatus::Synced);
    });
}

#[test]
fn update_supplies_a_real_hash_to_copy_assignments_and_reconcile_keeps_synced() {
    use crate::core::{installer, project_sync, refresh};

    let mut f = fixture();
    let source = f.dir.path().join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("SKILL.md"),
        "---\nname: skill\n---\nnew upstream\n",
    )
    .unwrap();
    let installed =
        installer::install_local_skill(&f.paths, &f.store, &source, Some("updated".into()))
            .unwrap();
    f.skill_id = installed.skill_id;
    f.central_path = installed.central_path;
    let project = tempfile::tempdir().unwrap();
    let target = seed_project_copy_assignment(&f, project.path());
    seed_stale_copy(&target);
    fs::write(source.join("a.txt"), "new bytes").unwrap();

    let report = refresh::refresh_managed_skills(
        &f.paths,
        &f.store,
        &crate::core::credentials::MemoryStore::new(),
        refresh::RefreshSelection::Ids(vec![f.skill_id.clone()]),
        None,
        |_| {},
    )
    .unwrap();
    assert!(
        matches!(
            report.skills[0].status,
            refresh::SkillRefreshStatus::Refreshed { .. }
        ),
        "{report:?}"
    );
    let skill = f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap();
    let row = f
        .store
        .get_project_skill_assignment("p1", &f.skill_id, project_copy_only_tool().key())
        .unwrap()
        .unwrap();
    assert_eq!(row.mode, SyncMode::Copy);
    assert_eq!(
        row.content_hash,
        Some(
            crate::core::content_identity::read(crate::core::content_identity::Source::Directory(
                &f.central_path
            ))
            .unwrap()
        )
    );
    assert_eq!(row.content_hash, skill.content_hash);
    assert_eq!(
        fs::read_to_string(target.join("a.txt")).unwrap(),
        "new bytes"
    );
    crate::core::mutation_guard::serialized(|| {
        let mut rows = f.store.list_project_skill_assignments("p1").unwrap();
        project_sync::reconcile_listing_unlocked(&f.store, "p1", &mut rows);
        assert_eq!(rows[0].status, SyncStatus::Synced);
    });
}

#[test]
fn a_project_whose_directory_is_gone_is_skipped() {
    let f = fixture();
    let project_dir = tempfile::tempdir().expect("project tempdir");
    let project_path = project_dir.path().to_path_buf();
    seed_project_copy_assignment(&f, &project_path);
    drop(project_dir); // the operator moved or deleted the project

    let outcomes = propagate(&f);

    let scope = PropagationScope::Project {
        project_id: "p1".to_string(),
        tool: project_folder_of_project_copy_only_tool(),
    };
    assert!(
        matches!(
            outcome_for(&outcomes, &scope),
            PropagationStatus::Skipped {
                reason: PropagationSkip::ProjectUnavailable { .. }
            }
        ),
        "got {:?}",
        outcomes
    );
    let row = f
        .store
        .get_project_skill_assignment("p1", &f.skill_id, project_copy_only_tool().key())
        .expect("query")
        .expect("row");
    assert_eq!(
        row.status,
        SyncStatus::Synced,
        "a skipped row keeps its previous status"
    );
}

/// The project-root rule is `is_dir`, shared with every project sync: a path
/// that exists but is not a folder is unavailable too, and left as it is.
#[test]
fn a_project_path_that_is_a_file_is_skipped() {
    let f = fixture();
    let project_dir = tempfile::tempdir().expect("project tempdir");
    let project_path = project_dir.path().join("moved");
    fs::create_dir_all(&project_path).unwrap();
    seed_project_copy_assignment(&f, &project_path);
    fs::remove_dir_all(&project_path).unwrap();
    fs::write(&project_path, "a file now").unwrap();

    let outcomes = propagate(&f);

    let scope = PropagationScope::Project {
        project_id: "p1".to_string(),
        tool: project_folder_of_project_copy_only_tool(),
    };
    assert!(
        matches!(
            outcome_for(&outcomes, &scope),
            PropagationStatus::Skipped {
                reason: PropagationSkip::ProjectUnavailable { .. }
            }
        ),
        "got {:?}",
        outcomes
    );
    assert_eq!(fs::read_to_string(&project_path).unwrap(), "a file now");
}

/// The artifact is located by the name it was materialised under (the stored
/// `assignment.skill_name`), not by the Managed skill's live name: renaming
/// the skill after assignment must not strand the old artifact and grow a
/// second one under the new name.
#[test]
fn a_project_artifact_is_found_by_its_stored_name_after_the_skill_is_renamed() {
    let f = fixture();
    let project_dir = tempfile::tempdir().expect("project tempdir");
    let target = seed_project_copy_assignment(&f, project_dir.path());
    seed_stale_copy(&target);

    // Finalize never renames today, so the rename is a direct store update.
    let mut skill = f
        .store
        .get_skill_by_id(&f.skill_id)
        .expect("query")
        .expect("skill row");
    skill.name = "skill-renamed".to_string();
    f.store.upsert_skill(&skill).expect("rename skill");

    let outcomes = propagate(&f);

    let scope = PropagationScope::Project {
        project_id: "p1".to_string(),
        tool: project_folder_of_project_copy_only_tool(),
    };
    assert!(
        matches!(
            outcome_for(&outcomes, &scope),
            PropagationStatus::Synced {
                mode_used: SyncMode::Copy
            }
        ),
        "got {:?}",
        outcomes
    );
    assert_eq!(
        fs::read_to_string(target.join("a.txt")).expect("read the artifact under its stored name"),
        "v2",
        "the stored-name artifact receives the new bytes"
    );
    let renamed = target.with_file_name("skill-renamed");
    assert!(
        renamed.symlink_metadata().is_err(),
        "no second artifact under the live name: {}",
        renamed.display()
    );
}

// ---------------------------------------------------------------------------
// A shared skills dir is one Deployment folder, one row (round 20 ticket 09,
// retiring the D34 amendment's per-sharer records): Propagation's first sync
// of a never-materialised row records nothing beyond the folder's row, exactly
// as the global batch does — so the Update's own first sync and its confirmed
// overwrite retry (which goes through the batch) leave the same row set.
// ---------------------------------------------------------------------------

/// Amp's global skills dir, shared with Kimi Code CLI.
fn shared_target(f: &Fixture) -> PathBuf {
    f.paths.roots.home.join(".config/agents/skills/skill")
}

/// A failed first sync's row: `error`, never synced, owning nothing.
fn seed_unowned_target(f: &Fixture, tool: &str, target_path: &Path) {
    f.store
        .upsert_skill_target(&SkillTargetRecord {
            id: format!("t-{tool}"),
            skill_id: f.skill_id.clone(),
            tool: tool.to_string(),
            target_path: target_path.to_string_lossy().to_string(),
            mode: SyncMode::Symlink,
            status: SyncStatus::Error,
            last_error: Some("an earlier first sync failed".to_string()),
            synced_at: None,
        })
        .expect("seed unowned target");
}

/// The skill's row set, comparable across fixtures: tool, status, mode,
/// path relative to the temp home, and whether the row owns its artifact.
fn row_set(f: &Fixture) -> Vec<(String, SyncStatus, SyncMode, PathBuf, bool)> {
    let mut rows: Vec<_> = f
        .store
        .list_skill_targets(&f.skill_id)
        .expect("list targets")
        .into_iter()
        .map(|row| {
            let owns = row.owns_artifact();
            let relative = Path::new(&row.target_path)
                .strip_prefix(&f.paths.roots.home)
                .expect("target under the temp home")
                .to_path_buf();
            (row.tool, row.status, row.mode, relative, owns)
        })
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows
}

fn tools_of(f: &Fixture) -> Vec<String> {
    row_set(f).into_iter().map(|row| row.0).collect()
}

fn foreign_folder(path: &Path) {
    fs::create_dir_all(path).expect("create foreign folder");
    fs::write(path.join("SKILL.md"), "foreign").expect("write foreign SKILL.md");
}

fn amp_and_kimi() -> (&'static ToolAdapter, &'static ToolAdapter) {
    let amp = adapter_by_key("amp").expect("amp adapter");
    let kimi = adapter_by_key("kimi_cli").expect("kimi_cli adapter");
    assert_eq!(
        amp.skills_dir, kimi.skills_dir,
        "test premise: amp and kimi_cli share a skills dir"
    );
    (amp, kimi)
}

#[test]
fn a_first_sync_into_a_shared_folder_records_only_the_folders_row() {
    let f = fixture();
    let (amp, kimi) = amp_and_kimi();
    install_tool(&f, amp);
    install_tool(&f, kimi);
    let target = shared_target(&f);
    seed_unowned_target(&f, "amp", &target);

    let outcomes = propagate(&f);

    assert!(
        matches!(
            outcome_for(&outcomes, &global("amp")),
            PropagationStatus::Synced {
                mode_used: SyncMode::Symlink
            }
        ),
        "the folder is reported synced, got {outcomes:?}"
    );
    assert_eq!(outcomes.len(), 1, "one folder, one outcome: {outcomes:?}");
    // Kimi Code reads the folder's row; no row of its own is minted.
    assert_eq!(tools_of(&f), ["amp"]);
    let (_, status, mode, _, owns) = &row_set(&f)[0];
    assert_eq!(*status, SyncStatus::Synced);
    assert_eq!(*mode, SyncMode::Symlink);
    assert!(*owns, "the folder's row owns the artifact it was synced to");
    // One artifact: the shared dir holds the skill once, as a link.
    let entries: Vec<_> = fs::read_dir(target.parent().unwrap())
        .expect("read shared dir")
        .collect();
    assert_eq!(entries.len(), 1, "one artifact in the shared dir");
    assert!(fs::symlink_metadata(&target)
        .expect("artifact")
        .file_type()
        .is_symlink());
}

#[test]
fn a_first_sync_records_no_sharer_that_is_not_installed() {
    let f = fixture();
    let (amp, _) = amp_and_kimi();
    install_tool(&f, amp);
    let target = shared_target(&f);
    seed_unowned_target(&f, "amp", &target);

    let outcomes = propagate(&f);

    assert_eq!(outcomes.len(), 1, "{outcomes:?}");
    assert!(matches!(
        outcome_for(&outcomes, &global("amp")),
        PropagationStatus::Synced { .. }
    ));
    assert_eq!(tools_of(&f), ["amp"]);
}

#[test]
fn an_update_of_an_owned_artifact_records_no_new_sharer() {
    let f = fixture();
    let (amp, kimi) = amp_and_kimi();
    install_tool(&f, amp);
    install_tool(&f, kimi);
    let target = shared_target(&f);
    seed_stale_copy(&target);
    seed_global_target(&f, "t-amp", "amp", &target, SyncMode::Copy);

    let outcomes = propagate(&f);

    assert_eq!(outcomes.len(), 1, "{outcomes:?}");
    assert!(matches!(
        outcome_for(&outcomes, &global("amp")),
        PropagationStatus::Synced { .. }
    ));
    assert_eq!(tools_of(&f), ["amp"], "an ordinary update adds no row");
}

/// The audit's exact sequence: the Update's first sync meets a foreign folder
/// (`TARGET_EXISTS`, first deploy; nothing is recorded until the ask is
/// answered), then the confirmed retry through the global batch records the
/// folder's one row — the same row set Propagation writes when nothing is in
/// the way.
#[test]
fn a_first_sync_collision_records_no_sharer_and_its_overwrite_retry_matches_propagation() {
    use crate::core::errors::CommandError;
    use crate::core::global_sync::{
        sync_skills_to_tools_unlocked, BatchOverride, BatchPolicy, BatchSkill, BatchTargetStatus,
        OverrideDecision,
    };

    let f = fixture();
    let (amp, kimi) = amp_and_kimi();
    install_tool(&f, amp);
    install_tool(&f, kimi);
    let target = shared_target(&f);
    seed_unowned_target(&f, "amp", &target);
    foreign_folder(&target);

    let outcomes = propagate(&f);

    assert_eq!(outcomes.len(), 1, "only the asked pair: {outcomes:?}");
    match outcome_for(&outcomes, &global("amp")) {
        PropagationStatus::Failed {
            error: CommandError::TargetExists { .. },
            first_deploy: true,
        } => {}
        other => panic!("expected a first-deploy TARGET_EXISTS, got {other:?}"),
    }
    assert_eq!(tools_of(&f), ["amp"], "a collision records no sharer");
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).expect("foreign bytes"),
        "foreign"
    );

    let retry = sync_skills_to_tools_unlocked(
        &f.paths.roots,
        &f.store,
        &central_repo_of(&f.store),
        &[BatchSkill {
            skill_id: f.skill_id.clone(),
            skill_name: "skill".to_string(),
            source_path: f.central_path.clone(),
        }],
        &["amp".to_string()],
        &BatchPolicy {
            overwrite: false,
            overwrite_if_same_content: true,
            overrides: vec![BatchOverride {
                skill_id: f.skill_id.clone(),
                tool_key: "amp".to_string(),
                decision: OverrideDecision::Overwrite,
            }],
        },
        2000,
        |_| {},
    );
    assert!(
        retry
            .iter()
            .all(|o| matches!(o.status, BatchTargetStatus::Synced { .. })),
        "{retry:?}"
    );

    let control = fixture();
    install_tool(&control, amp);
    install_tool(&control, kimi);
    seed_unowned_target(&control, "amp", &shared_target(&control));
    propagate(&control);

    assert_eq!(tools_of(&f), ["amp"]);
    assert_eq!(
        row_set(&f),
        row_set(&control),
        "the batch's retry and Propagation's first sync leave one row set"
    );
}

#[test]
fn a_failed_first_sync_settles_the_folders_row_error_as_the_batch_does() {
    use crate::core::errors::CommandError;

    let f = fixture();
    let (amp, kimi) = amp_and_kimi();
    install_tool(&f, amp);
    install_tool(&f, kimi);
    let target = shared_target(&f);
    seed_unowned_target(&f, "amp", &target);
    fs::remove_dir_all(&f.central_path).expect("remove central copy");

    let outcomes = propagate(&f);

    assert_eq!(outcomes.len(), 1, "{outcomes:?}");
    match outcome_for(&outcomes, &global("amp")) {
        PropagationStatus::Failed {
            error: CommandError::InvalidPath { reason, .. },
            first_deploy: true,
        } if reason == "missing" => {}
        other => panic!("expected a failed first deploy, got {other:?}"),
    }
    assert_eq!(tools_of(&f), ["amp"], "no sibling row is minted");
    let (_, status, _, _, owns) = &row_set(&f)[0];
    assert_eq!(*status, SyncStatus::Error);
    assert!(!owns, "a failed first sync owns nothing");
}

#[test]
fn a_disabled_member_mints_no_sharer_and_skips_the_group() {
    let f = fixture();
    let (amp, kimi) = amp_and_kimi();
    install_tool(&f, amp);
    install_tool(&f, kimi);
    let target = shared_target(&f);
    f.store
        .upsert_skill_target(&SkillTargetRecord {
            id: "t-amp".to_string(),
            skill_id: f.skill_id.clone(),
            tool: "amp".to_string(),
            target_path: target.to_string_lossy().to_string(),
            mode: SyncMode::Symlink,
            status: SyncStatus::Disabled,
            last_error: None,
            synced_at: None,
        })
        .expect("seed disabled marker");

    let outcomes = propagate(&f);

    assert_eq!(outcomes.len(), 1, "{outcomes:?}");
    assert!(matches!(
        outcome_for(&outcomes, &global("amp")),
        PropagationStatus::Skipped {
            reason: PropagationSkip::Disabled
        }
    ));
    assert_eq!(tools_of(&f), ["amp"]);
    assert!(fs::symlink_metadata(&target).is_err(), "nothing written");

    // Skill grain: an unowned `error` row of a skill disabled everywhere.
    let g = fixture();
    install_tool(&g, amp);
    install_tool(&g, kimi);
    seed_unowned_target(&g, "amp", &shared_target(&g));
    g.store
        .set_skill_disabled_at(&g.skill_id, Some(1))
        .expect("disable skill");

    let outcomes = propagate(&g);

    assert!(matches!(
        outcome_for(&outcomes, &global("amp")),
        PropagationStatus::Skipped {
            reason: PropagationSkip::Disabled
        }
    ));
    assert_eq!(tools_of(&g), ["amp"]);
    assert!(fs::symlink_metadata(shared_target(&g)).is_err());
}

// ---------------------------------------------------------------------------
// Every row propagates by itself (round 20 t09 reviews, rounds 1–3): at its
// own recorded path, with its own ownership and its own settlement, and with
// the capability of the folder it describes. Nothing is lent or settled from
// one row to another, however physically close their paths are.
// ---------------------------------------------------------------------------

/// A project with Trae and Trae CN rows, as a pre-1.4.0 project wrote them.
fn seed_trae_assignment(
    f: &Fixture,
    project_path: &Path,
    tool: &str,
    name: &str,
    mode: SyncMode,
    owned: bool,
) {
    if f.store.get_project_by_id("p1").unwrap().is_none() {
        f.store
            .register_project(&ProjectRecord {
                id: "p1".to_string(),
                path: project_path.to_string_lossy().to_string(),
                created_at: 1,
                updated_at: 1,
            })
            .expect("register project");
    }
    f.store
        .add_project_skill_assignment(&ProjectSkillAssignmentRecord {
            id: format!("pa-{tool}"),
            project_id: "p1".to_string(),
            skill_id: f.skill_id.clone(),
            skill_name: name.to_string(),
            tool: tool.to_string(),
            mode,
            status: if owned {
                SyncStatus::Synced
            } else {
                SyncStatus::Error
            },
            last_error: None,
            synced_at: owned.then_some(1),
            content_hash: None,
            created_at: 1,
        })
        .expect("seed assignment");
}

fn trae_scope(tool: &str) -> PropagationScope {
    PropagationScope::Project {
        project_id: "p1".to_string(),
        tool: tool.to_string(),
    }
}

fn is_first_deploy_collision(status: &PropagationStatus) -> bool {
    matches!(
        status,
        PropagationStatus::Failed {
            error: crate::core::errors::CommandError::TargetExists { .. },
            first_deploy: true,
        }
    )
}

/// Review repro 3: the `trae` row records a link, the `trae_cn` row records a
/// copy of the same artifact, and that artifact is a real copy with old bytes.
/// The copy row's own Propagation refreshes it — no representative row's
/// link can skip it.
#[test]
fn a_project_copy_recorded_by_any_row_of_its_artifact_gets_new_bytes() {
    let f = fixture();
    let project = tempfile::tempdir().expect("project tempdir");
    seed_trae_assignment(&f, project.path(), "trae", "skill", SyncMode::Symlink, true);
    seed_trae_assignment(&f, project.path(), "trae_cn", "skill", SyncMode::Copy, true);
    let target = project.path().join(".trae/skills/skill");
    seed_stale_copy(&target);

    let outcomes = propagate(&f);

    assert!(
        matches!(
            outcome_for(&outcomes, &trae_scope("trae_cn")),
            PropagationStatus::Synced { .. }
        ),
        "the stale copy is refreshed by its own row, got {outcomes:?}"
    );
    assert!(
        matches!(
            outcome_for(&outcomes, &trae_scope("trae")),
            PropagationStatus::Skipped {
                reason: PropagationSkip::LinkFollowsSource
            }
        ),
        "{outcomes:?}"
    );
    assert_eq!(
        fs::read_to_string(target.join("a.txt")).expect("read target"),
        "v2",
        "the copy carries the new bytes"
    );
    let row = f
        .store
        .get_project_skill_assignment("p1", &f.skill_id, "trae_cn")
        .unwrap()
        .unwrap();
    assert_eq!(row.synced_at, Some(2000), "the copy row settles itself");
}

/// A project sibling recording another name neither lends its ownership nor
/// settles another row: the owned old-name copy is refreshed in place by its
/// own row; the never-synced row's own first deploy meets the foreign folder
/// at the current name and asks.
#[test]
fn a_project_sibling_naming_another_artifact_lends_no_ownership() {
    let f = fixture();
    let project = tempfile::tempdir().expect("project tempdir");
    seed_trae_assignment(
        &f,
        project.path(),
        "trae",
        "skill",
        SyncMode::Symlink,
        false,
    );
    seed_trae_assignment(
        &f,
        project.path(),
        "trae_cn",
        "old-name",
        SyncMode::Copy,
        true,
    );
    let old = project.path().join(".trae/skills/old-name");
    seed_stale_copy(&old);
    let foreign = project.path().join(".trae/skills/skill");
    foreign_folder(&foreign);

    let outcomes = propagate(&f);

    assert!(
        matches!(
            outcome_for(&outcomes, &trae_scope("trae_cn")),
            PropagationStatus::Synced { .. }
        ),
        "the owner's own copy is refreshed, got {outcomes:?}"
    );
    assert!(
        is_first_deploy_collision(outcome_for(&outcomes, &trae_scope("trae"))),
        "{outcomes:?}"
    );
    assert_eq!(fs::read_to_string(old.join("a.txt")).unwrap(), "v2");
    assert_eq!(
        fs::read_to_string(foreign.join("SKILL.md")).unwrap(),
        "foreign"
    );
    let trae = f
        .store
        .get_project_skill_assignment("p1", &f.skill_id, "trae")
        .unwrap()
        .unwrap();
    assert!(!trae.owns_artifact(), "never lent ownership");
    assert_eq!(trae.status, SyncStatus::Error);
}

/// The global arm: Kimi's row owning a copy under another name is refreshed in
/// place by itself; Amp's never-synced row over a foreign folder at the
/// current name gets its own first deploy, which asks — it is neither
/// authorised by Kimi's ownership nor settled by Kimi's write.
#[test]
fn a_global_sibling_naming_another_artifact_lends_no_ownership() {
    let f = fixture();
    let (amp, kimi) = amp_and_kimi();
    install_tool(&f, amp);
    install_tool(&f, kimi);
    let foreign = shared_target(&f);
    foreign_folder(&foreign);
    seed_unowned_target(&f, "amp", &foreign);
    let old = f.paths.roots.home.join(".config/agents/skills/old-name");
    seed_stale_copy(&old);
    seed_global_target(&f, "t-kimi", "kimi_cli", &old, SyncMode::Copy);

    let outcomes = propagate(&f);

    assert!(
        matches!(
            outcome_for(&outcomes, &global("kimi_cli")),
            PropagationStatus::Synced { .. }
        ),
        "the owner's own copy is refreshed, got {outcomes:?}"
    );
    assert!(
        is_first_deploy_collision(outcome_for(&outcomes, &global("amp"))),
        "{outcomes:?}"
    );
    assert_eq!(fs::read_to_string(old.join("a.txt")).unwrap(), "v2");
    assert_eq!(
        fs::read_to_string(foreign.join("SKILL.md")).unwrap(),
        "foreign"
    );
    let kimi_row = f
        .store
        .get_skill_target(&f.skill_id, "kimi_cli")
        .unwrap()
        .unwrap();
    assert_eq!(std::path::Path::new(&kimi_row.target_path), old.as_path());
    assert_eq!(kimi_row.synced_at, Some(2000), "its own path, written");
    let amp_row = f
        .store
        .get_skill_target(&f.skill_id, "amp")
        .unwrap()
        .unwrap();
    assert_eq!(
        amp_row.target_path,
        foreign.to_string_lossy(),
        "its own path"
    );
    assert!(!amp_row.owns_artifact());
    assert_eq!(amp_row.status, SyncStatus::Error);
}

/// Pre-1.4.0 Amp/Kimi siblings recording one path byte for byte are still two
/// rows: the never-synced row's own first deploy (first in store order) meets
/// the stale copy and asks; the owner then refreshes its own artifact. The
/// next Propagation's first deploy of the never-synced row finds the skill's
/// link there and settles it by its own sync.
#[test]
fn global_siblings_recording_the_same_path_settle_each_by_its_own_sync() {
    let f = fixture();
    let (amp, kimi) = amp_and_kimi();
    install_tool(&f, amp);
    install_tool(&f, kimi);
    let shared = shared_target(&f);
    seed_stale_copy(&shared);
    seed_unowned_target(&f, "amp", &shared);
    seed_global_target(&f, "t-kimi", "kimi_cli", &shared, SyncMode::Copy);

    let outcomes = propagate(&f);

    assert_eq!(outcomes.len(), 2, "one outcome per row: {outcomes:?}");
    assert!(
        is_first_deploy_collision(outcome_for(&outcomes, &global("amp"))),
        "{outcomes:?}"
    );
    assert!(
        matches!(
            outcome_for(&outcomes, &global("kimi_cli")),
            PropagationStatus::Synced { .. }
        ),
        "{outcomes:?}"
    );
    let amp_row = f
        .store
        .get_skill_target(&f.skill_id, "amp")
        .unwrap()
        .unwrap();
    assert!(
        !amp_row.owns_artifact(),
        "nothing settled it from Kimi's write"
    );

    let again = propagate(&f);
    assert!(
        matches!(
            outcome_for(&again, &global("amp")),
            PropagationStatus::Synced { .. }
        ),
        "{again:?}"
    );
    for tool in ["amp", "kimi_cli"] {
        let row = f
            .store
            .get_skill_target(&f.skill_id, tool)
            .unwrap()
            .unwrap();
        assert_eq!(row.status, SyncStatus::Synced, "{tool}");
        assert!(row.owns_artifact(), "{tool}");
    }
}

/// One disabled row of a folder keeps every row of it from being written, as
/// it did before rows were per folder: writing the shared artifact for the
/// other row would re-materialise it where the disabled harness reads. Each
/// row is reported for itself.
#[test]
fn a_disabled_row_keeps_its_folders_other_rows_unwritten() {
    let f = fixture();
    let (amp, kimi) = amp_and_kimi();
    install_tool(&f, amp);
    install_tool(&f, kimi);
    let shared = shared_target(&f);
    seed_stale_copy(&shared);
    seed_global_target(&f, "t-kimi", "kimi_cli", &shared, SyncMode::Copy);
    f.store
        .upsert_skill_target(&SkillTargetRecord {
            id: "t-amp".to_string(),
            skill_id: f.skill_id.clone(),
            tool: "amp".to_string(),
            target_path: shared.to_string_lossy().to_string(),
            mode: SyncMode::Symlink,
            status: SyncStatus::Disabled,
            last_error: None,
            synced_at: None,
        })
        .expect("seed disabled marker");

    let outcomes = propagate(&f);

    for tool in ["amp", "kimi_cli"] {
        assert!(
            matches!(
                outcome_for(&outcomes, &global(tool)),
                PropagationStatus::Skipped {
                    reason: PropagationSkip::Disabled
                }
            ),
            "{tool}: {outcomes:?}"
        );
    }
    assert_eq!(fs::read_to_string(shared.join("a.txt")).unwrap(), "v1");
}

/// Review finding 6: two spellings naming one slot (the folder reached
/// directly and through an aliased home) are two rows with two recorded paths.
/// With a foreign folder in that slot, neither never-synced row gets an
/// overwrite permission from the other: each one's own first deploy asks,
/// and the foreign folder stays.
#[cfg(unix)]
#[test]
fn a_slot_reached_by_two_spellings_lends_nothing() {
    let f = fixture();
    let (amp, kimi) = amp_and_kimi();
    install_tool(&f, amp);
    install_tool(&f, kimi);
    let canonical = shared_target(&f);
    let alias_home = f.paths.roots.home.parent().unwrap().join("alias-home");
    std::os::unix::fs::symlink(&f.paths.roots.home, &alias_home).unwrap();
    let aliased = alias_home.join(".config/agents/skills/skill");
    foreign_folder(&canonical);
    assert!(
        aliased.join("SKILL.md").is_file(),
        "one slot, two spellings"
    );
    seed_unowned_target(&f, "amp", &canonical);
    seed_unowned_target(&f, "kimi_cli", &aliased);

    let outcomes = propagate(&f);

    for tool in ["amp", "kimi_cli"] {
        assert!(
            is_first_deploy_collision(outcome_for(&outcomes, &global(tool))),
            "{tool}: {outcomes:?}"
        );
        let row = f
            .store
            .get_skill_target(&f.skill_id, tool)
            .unwrap()
            .unwrap();
        assert!(!row.owns_artifact(), "{tool}");
    }
    assert_eq!(
        fs::read_to_string(canonical.join("SKILL.md")).unwrap(),
        "foreign",
        "the foreign occupant is not replaced"
    );
}

/// Astra 203, the differential against `main`: an owned legacy Kimi Code copy
/// on Amp's folder, with Amp installed and Kimi Code not. A legacy row is
/// reached through its own harness alone (`placement::row_reachable`), so it
/// is `ToolNotInstalled` and its bytes are left as they are — never written
/// because a sibling harness of its folder is installed.
#[test]
fn a_legacy_row_of_an_uninstalled_harness_is_skipped_whatever_its_folder_serves() {
    let f = fixture();
    let (amp, _) = amp_and_kimi();
    install_tool(&f, amp);
    let target = shared_target(&f);
    seed_stale_copy(&target);
    seed_global_target(&f, "t-kimi", "kimi_cli", &target, SyncMode::Copy);

    let outcomes = propagate(&f);

    assert!(
        matches!(
            outcome_for(&outcomes, &global("kimi_cli")),
            PropagationStatus::Skipped {
                reason: PropagationSkip::ToolNotInstalled { tool }
            } if tool == "kimi_cli"
        ),
        "got {outcomes:?}"
    );
    assert_eq!(
        fs::read_to_string(target.join("a.txt")).expect("read target"),
        "v1",
        "the legacy copy's bytes are untouched"
    );
    let row = f
        .store
        .get_skill_target(&f.skill_id, "kimi_cli")
        .expect("query")
        .expect("row");
    assert_eq!((row.status, row.synced_at), (SyncStatus::Synced, Some(1)));
}

/// The folder row of the same folder is reached through any harness the
/// folder serves: Amp's row is refreshed with only Kimi Code installed.
#[test]
fn a_folder_row_is_reached_through_any_harness_its_folder_serves() {
    let f = fixture();
    let (_, kimi) = amp_and_kimi();
    install_tool(&f, kimi);
    let target = shared_target(&f);
    seed_stale_copy(&target);
    seed_global_target(&f, "t-amp", "amp", &target, SyncMode::Copy);

    let outcomes = propagate(&f);

    assert!(
        matches!(
            outcome_for(&outcomes, &global("amp")),
            PropagationStatus::Synced { .. }
        ),
        "got {outcomes:?}"
    );
    assert_eq!(
        fs::read_to_string(target.join("a.txt")).expect("read target"),
        "v2"
    );
}
