use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use super::{serialized, try_serialized};

use crate::core::gitignore::IgnoreUpdateOptions;
use crate::core::installer::InstallerPaths;
use crate::core::skill_store::{ProjectRecord, SkillRecord, SkillStore};
use crate::core::sync_status::SyncStatus;
use crate::core::{artifact_removal, global_sync, installer, project_ops, project_sync, refresh};

/// How long a mutation-in-flight window is held open while asserting that a
/// second mutation has not started. The guard is process-global, so every
/// other mutation test queues behind this — keep it short.
const HOLD: Duration = Duration::from_millis(300);

// ---------------------------------------------------------------------------
// The guard itself
// ---------------------------------------------------------------------------

/// The guard is process-global: a second `serialized` call cannot start while
/// the first is still running.
#[test]
fn serialized_runs_one_at_a_time() {
    let inside = Arc::new(AtomicBool::new(false));
    let entered_while_held = Arc::new(AtomicBool::new(false));

    let inside_main = inside.clone();
    let entered = entered_while_held.clone();
    serialized(|| {
        inside_main.store(true, Ordering::SeqCst);
        let inside_thread = inside_main.clone();
        let entered_thread = entered.clone();
        let handle = std::thread::spawn(move || {
            serialized(|| {
                if inside_thread.load(Ordering::SeqCst) {
                    entered_thread.store(true, Ordering::SeqCst);
                }
            });
        });
        std::thread::sleep(Duration::from_millis(150));
        inside_main.store(false, Ordering::SeqCst);
        handle
    })
    .join()
    .expect("second thread completes once the guard is released");

    assert!(
        !entered_while_held.load(Ordering::SeqCst),
        "a second mutation must not run while the first holds the guard"
    );
}

/// `try_serialized` is the reconcile pass's probe: `None` while a mutation is
/// in flight, `Some` once it is not.
#[test]
fn try_serialized_yields_none_while_a_mutation_is_in_flight() {
    let observed = serialized(|| {
        std::thread::spawn(|| try_serialized(|| 42u32))
            .join()
            .expect("probe thread")
    });
    assert!(
        observed.is_none(),
        "try_serialized must not block behind an in-flight mutation"
    );

    // The guard is process-global and the test binary runs tests in parallel,
    // so "free" is only ever eventually true; retry briefly rather than assume
    // this thread is the only mutation in the process.
    let mut reopened = None;
    for _ in 0..100 {
        reopened = try_serialized(|| 42u32);
        if reopened.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(
        reopened,
        Some(42),
        "try_serialized runs the operation once no mutation is in flight"
    );
}

// ---------------------------------------------------------------------------
// The rule at the real entry points
// ---------------------------------------------------------------------------

/// Drive `op` from a second thread while this one holds the guard, and assert
/// it could not start until the guard was released.
///
/// This is the whole point of the ticket: no test-owned mutex is involved —
/// `op` is a real core entry point, and the only thing stopping it is the
/// serialisation the entry point wraps itself in.
fn assert_serialized(label: &str, op: impl FnOnce() + Send + 'static) {
    let completed = Arc::new(AtomicBool::new(false));
    let completed_in_thread = completed.clone();
    let completed_probe = completed.clone();

    let handle = serialized(move || {
        let handle = std::thread::spawn(move || {
            op();
            completed_in_thread.store(true, Ordering::SeqCst);
        });
        std::thread::sleep(HOLD);
        assert!(
            !completed_probe.load(Ordering::SeqCst),
            "{label} ran while another Sync-target mutation held the guard"
        );
        handle
    });

    handle.join().expect("the queued mutation thread finishes");
    assert!(
        completed.load(Ordering::SeqCst),
        "{label} must complete once the guard is released"
    );
}

struct Fixture {
    _db_dir: tempfile::TempDir,
    _work_dir: tempfile::TempDir,
    store: SkillStore,
    project: ProjectRecord,
    skill: SkillRecord,
    skill_dir: std::path::PathBuf,
}

fn make_skill_dir(base: &Path, name: &str) -> std::path::PathBuf {
    let dir = base.join(name);
    fs::create_dir_all(&dir).expect("create skill dir");
    fs::write(dir.join("SKILL.md"), "# Test Skill\nTest content.").expect("write SKILL.md");
    dir
}

/// One registered project with one Managed skill assigned to `claude_code`,
/// all inside temp dirs.
fn fixture(name: &str) -> Fixture {
    let db_dir = tempfile::tempdir().expect("tempdir");
    let store = SkillStore::new(db_dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");

    let work_dir = tempfile::tempdir().expect("tempdir");
    let skill_dir = make_skill_dir(work_dir.path(), name);
    let project_dir = work_dir.path().join(format!("{name}-project"));
    fs::create_dir_all(&project_dir).expect("create project dir");

    let now = 1000i64;
    let project = ProjectRecord {
        id: uuid::Uuid::new_v4().to_string(),
        path: project_dir.to_string_lossy().to_string(),
        created_at: now,
        updated_at: now,
    };
    store.register_project(&project).expect("register project");

    let skill = SkillRecord {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        description: None,
        source_type: "local".to_string(),
        source_ref: None,
        source_subpath: None,
        source_revision: None,
        central_path: skill_dir.to_string_lossy().to_string(),
        content_hash: None,
        created_at: now,
        updated_at: now,
        last_sync_at: None,
        last_seen_at: now,
        status: "ok".to_string(),
    };
    store.upsert_skill(&skill).expect("upsert skill");

    project_sync::assign_and_sync(&store, &project, &skill, "claude_code", now)
        .expect("assign should succeed");

    Fixture {
        _db_dir: db_dir,
        _work_dir: work_dir,
        store,
        project,
        skill,
        skill_dir,
    }
}

/// The toggle reads its own rows and acts on them in one critical section,
/// so the decision cannot be raced by another mutation.
#[test]
fn toggle_skill_assignment_is_serialized() {
    let f = fixture("toggle-guard");
    let store = f.store.clone();
    let project_id = f.project.id.clone();
    let skill_id = f.skill.id.clone();
    assert_serialized("toggle_skill_assignment", move || {
        project_sync::toggle_skill_assignment(&store, &project_id, &skill_id, "claude_code", 4000)
            .expect("toggle");
    });
}

#[test]
fn resync_project_is_serialized() {
    let f = fixture("resync-guard");
    let store = f.store.clone();
    let project_id = f.project.id.clone();
    assert_serialized("resync_project", move || {
        project_sync::resync_project(&store, &project_id, 4000).expect("resync");
    });
}

#[test]
fn deleting_a_managed_skill_is_serialized() {
    let f = fixture("delete-guard");
    let store = f.store.clone();
    let skill_id = f.skill.id.clone();
    assert_serialized("remove_skill", move || {
        artifact_removal::remove_skill(&store, &skill_id).expect("remove skill");
    });
}

#[test]
fn updating_a_projects_gitignore_is_serialized() {
    let f = fixture("gitignore-guard");
    let store = f.store.clone();
    let project_id = f.project.id.clone();
    assert_serialized("gitignore::update_for_project", move || {
        crate::core::gitignore::update_for_project(
            &store,
            &project_id,
            IgnoreUpdateOptions {
                add_to_gitignore: true,
                add_to_exclude: false,
            },
        )
        .expect("gitignore update");
    });
}

#[test]
fn configuring_project_tools_is_serialized() {
    let f = fixture("configure-guard");
    let store = f.store.clone();
    let project_id = f.project.id.clone();
    assert_serialized("configure_project_tools", move || {
        project_ops::configure_project_tools(
            &store,
            &project_id,
            &["claude_code".to_string()],
            None,
        )
        .expect("configure tools");
    });
}

#[test]
fn global_sync_batch_is_serialized() {
    let f = fixture("global-sync-guard");
    let store = f.store.clone();
    let home = f._work_dir.path().join("empty-home");
    fs::create_dir_all(&home).expect("create home");
    let skills = vec![global_sync::BatchSkill {
        skill_id: f.skill.id.clone(),
        skill_name: f.skill.name.clone(),
        source_path: f.skill_dir.clone(),
    }];
    assert_serialized("sync_skills_to_tools", move || {
        let _ = global_sync::sync_skills_to_tools(
            &home,
            &store,
            &skills,
            &["claude_code".to_string()],
            &global_sync::BatchPolicy::default(),
            5000,
            |_| {},
        );
    });
}

/// The Refresh batch: acquisition (phase one) happens outside the guard, but
/// finalize + Propagation (phase two) are inside it, so the whole call still
/// cannot complete while another mutation is in flight.
#[test]
fn refresh_finalize_and_propagation_is_serialized() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");

    let roots = tempfile::tempdir().expect("tempdir");
    let paths = InstallerPaths {
        home: roots.path().join("home"),
        central_dir: roots.path().join("central"),
        cache_dir: roots.path().join("cache"),
    };
    fs::create_dir_all(&paths.home).expect("create home");

    let source = tempfile::tempdir().expect("tempdir");
    fs::write(source.path().join("SKILL.md"), b"---\nname: x\n---\n").expect("write SKILL.md");
    fs::write(source.path().join("a.txt"), b"v1").expect("write a.txt");

    let installed =
        installer::install_local_skill(&paths, &store, source.path(), Some("upd".to_string()))
            .expect("install");

    fs::write(source.path().join("a.txt"), b"v2").expect("write a.txt");

    let skill_id = installed.skill_id.clone();
    assert_serialized("refresh_managed_skills", move || {
        refresh::refresh_managed_skills(
            &paths,
            &store,
            refresh::RefreshSelection::Ids(vec![skill_id]),
            refresh::RefreshPolicy::default(),
            None,
            5000,
            |_| {},
        )
        .expect("refresh");
        // Keep the temp roots alive for the whole operation.
        drop((roots, source, dir));
    });
}

// ---------------------------------------------------------------------------
// The reconcile pass's try-lock
// ---------------------------------------------------------------------------

/// A copy-mode row whose source drifted: the reconcile pass would rewrite it
/// to `stale`. Returns `(tool_key, assignment_id)`.
fn make_drifted_copy_assignment(f: &Fixture) -> (&'static str, String) {
    let mut adapter = crate::core::tool_adapters::adapter_by_key("cursor")
        .expect("cursor adapter")
        .clone();
    adapter.supports_symlink = false;
    let tool = crate::core::tool_adapters::test_overrides::shadow(adapter).key();

    let record = project_sync::assign_and_sync(&f.store, &f.project, &f.skill, tool, 2000)
        .expect("assign copy-mode");
    assert_eq!(record.status, SyncStatus::Synced);
    fs::write(f.skill_dir.join("drift.txt"), "changed").expect("write drift");
    (tool, record.id)
}

#[test]
fn listing_skips_reconciliation_while_a_mutation_is_in_flight() {
    let f = fixture("listing-skip-guard");
    let (tool, _assignment_id) = make_drifted_copy_assignment(&f);

    let store = f.store.clone();
    let project_id = f.project.id.clone();
    let listing = serialized(move || {
        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let listing = project_sync::list_assignments_with_staleness(&store, &project_id)
                .expect("listing should not block");
            (listing, started.elapsed())
        })
        .join()
        .expect("listing thread")
    });

    let (listing, elapsed) = listing;
    assert!(
        !listing.reconciled,
        "a listing taken during a mutation must report the reconcile pass was skipped"
    );
    assert!(
        elapsed < HOLD,
        "the listing must return promptly, not queue behind the mutation (took {elapsed:?})"
    );

    let stored = f
        .store
        .get_project_skill_assignment(&f.project.id, &f.skill.id, tool)
        .expect("read row")
        .expect("row exists");
    assert_eq!(
        stored.status,
        SyncStatus::Synced,
        "a skipped reconcile writes nothing: the stale row keeps its stored status"
    );
}

#[test]
fn listing_reconciles_when_no_mutation_is_in_flight() {
    let f = fixture("listing-reconcile-guard");
    let (_tool, assignment_id) = make_drifted_copy_assignment(&f);

    let mut listing = None;
    for _ in 0..100 {
        let attempt = project_sync::list_assignments_with_staleness(&f.store, &f.project.id)
            .expect("listing");
        if attempt.reconciled {
            listing = Some(attempt);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let listing = listing.expect("the reconcile pass eventually gets the guard");

    let row = listing
        .assignments
        .iter()
        .find(|a| a.id == assignment_id)
        .expect("the drifted row is listed");
    assert_eq!(
        row.status,
        SyncStatus::Stale,
        "an un-blocked listing reconciles the drifted copy to stale"
    );
}
