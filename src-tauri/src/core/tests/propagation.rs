//! Tests for `core::propagation` — bringing every Sync target of one Managed
//! skill into line after its central copy changed.
//!
//! Every case runs against a temp home / central dir / DB: installedness is
//! faked by creating a tool's detect dir under the temp home, so no test
//! touches the operator's real home or skill library. Outcomes are asserted
//! as report data — Propagation never fails the operation because one target
//! failed.

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
use crate::core::tool_adapters::{adapter_by_key, ToolAdapter};

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
        home: dir.path().join("home"),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.home).expect("create home");

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
    fs::create_dir_all(f.paths.home.join(adapter.relative_detect_dir)).expect("create detect dir");
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
    propagate_unlocked(&f.store, &f.paths, &f.skill_id, Some("hash-v2"), 2000)
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
    let target = f.paths.home.join(".cursor/skills/skill");
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
    let target = f.paths.home.join(".claude/skills/skill");
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
    let target = f.paths.home.join(".claude/skills/skill");
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

#[test]
fn a_shared_skills_dir_group_syncs_once_and_settles_every_member_row() {
    let f = fixture();
    let amp = adapter_by_key("amp").expect("amp adapter");
    let kimi = adapter_by_key("kimi_cli").expect("kimi_cli adapter");
    assert_eq!(amp.relative_skills_dir, kimi.relative_skills_dir);
    install_tool(&f, amp);
    let target = f.paths.home.join(".config/agents/skills/skill");
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

#[test]
fn a_missing_central_source_fails_every_row_as_report_data() {
    let f = fixture();
    let amp = adapter_by_key("amp").expect("amp adapter");
    install_tool(&f, amp);
    let target = f.paths.home.join(".config/agents/skills/skill");
    seed_stale_copy(&target);
    seed_global_target(&f, "t-amp", "amp", &target, SyncMode::Copy);
    seed_global_target(&f, "t-kimi", "kimi_cli", &target, SyncMode::Copy);
    fs::remove_dir_all(&f.central_path).expect("remove central copy");

    let outcomes = propagate(&f);

    for tool in ["amp", "kimi_cli"] {
        match outcome_for(&outcomes, &global(tool)) {
            PropagationStatus::Failed { error } => {
                assert_eq!(
                    error.downcast_ref::<crate::core::errors::SignalError>(),
                    Some(&crate::core::errors::SignalError::InvalidPath {
                        path: f.central_path.to_string_lossy().to_string(),
                        reason: "missing".to_string(),
                    }),
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

/// A project assignment row in copy mode on a copy-only Tool, with a stale
/// copy already on disk.
fn seed_project_copy_assignment(f: &Fixture, project_path: &Path) -> PathBuf {
    let cursor = copy_only_tool();
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
            tool: cursor.key().to_string(),
            mode: SyncMode::Copy,
            status: SyncStatus::Synced,
            last_error: None,
            synced_at: Some(1),
            content_hash: None,
            created_at: 1,
        })
        .expect("seed assignment");
    project_path
        .join(cursor.project_relative_skills_dir)
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
        tool: copy_only_tool().key().to_string(),
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
        .get_project_skill_assignment("p1", &f.skill_id, copy_only_tool().key())
        .expect("query")
        .expect("row");
    assert_eq!(row.status, SyncStatus::Synced);
    assert_eq!(row.content_hash.as_deref(), Some("hash-v2"));
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
        tool: copy_only_tool().key().to_string(),
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
        .get_project_skill_assignment("p1", &f.skill_id, copy_only_tool().key())
        .expect("query")
        .expect("row");
    assert_eq!(
        row.status,
        SyncStatus::Synced,
        "a skipped row keeps its previous status"
    );
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
        tool: copy_only_tool().key().to_string(),
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
