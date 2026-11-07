use crate::core::sync_status::{ProjectSyncStatus, SyncMode, SyncStatus};
use std::path::PathBuf;

use crate::core::skill_store::{
    AssignmentTransition, ProjectAggregate, ProjectRecord, ProjectSkillAssignmentRecord,
    ProjectToolRecord, SkillRecord, SkillStore, SkillTargetRecord, TargetTransition,
};

fn make_store() -> (tempfile::TempDir, SkillStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("test.db");
    let store = SkillStore::new(db);
    store.ensure_schema().expect("ensure_schema");
    (dir, store)
}

fn make_skill(id: &str, name: &str, central_path: &str, updated_at: i64) -> SkillRecord {
    SkillRecord {
        id: id.to_string(),
        name: name.to_string(),
        description: None,
        source_type: "local".to_string(),
        source_ref: Some("/tmp/source".to_string()),
        source_subpath: None,
        source_revision: None,
        central_path: central_path.to_string(),
        content_hash: None,
        created_at: 1,
        updated_at,
        last_sync_at: None,
        last_seen_at: 1,
        status: "ok".to_string(),
        imported_from_tool: None,
    }
}

/// A database exactly as v1.2.3 left it (schema version 8): every DDL step
/// the upgrade path ran, and nothing this version added.
fn open_v8_database(db: &std::path::Path) {
    let conn = rusqlite::Connection::open(db).expect("open");
    conn.execute_batch(super::SCHEMA_V1).expect("v1");
    conn.execute_batch("ALTER TABLE skills ADD COLUMN description TEXT NULL;")
        .expect("v2");
    conn.execute_batch("ALTER TABLE skills ADD COLUMN source_subpath TEXT NULL;")
        .expect("v3");
    conn.execute_batch(super::MIGRATION_V4).expect("v4");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS hidden_explore_skills (
            source_url TEXT PRIMARY KEY,
            hidden_at INTEGER NOT NULL
        );",
    )
    .expect("v7");
    conn.pragma_update(None, "user_version", 8).expect("v8");
}

#[test]
fn schema_is_idempotent() {
    let (_dir, store) = make_store();
    store.ensure_schema().expect("ensure_schema again");
}

#[test]
fn settings_roundtrip_and_update() {
    let (_dir, store) = make_store();

    assert_eq!(store.get_setting("missing").unwrap(), None);
    store.set_setting("k", "v1").unwrap();
    assert_eq!(store.get_setting("k").unwrap().as_deref(), Some("v1"));
    store.set_setting("k", "v2").unwrap();
    assert_eq!(store.get_setting("k").unwrap().as_deref(), Some("v2"));
}

/// Upgrading a v8 database adds the imported-from column: legacy rows read
/// back with no found-in Tool, and an `imported` row round-trips its own.
#[test]
fn v9_migration_adds_imported_from_tool_to_a_v8_database() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("test.db");
    open_v8_database(&db);
    {
        let conn = rusqlite::Connection::open(&db).expect("open");
        conn.execute(
            "INSERT INTO skills (id, name, source_type, source_ref, central_path,
                                 created_at, updated_at, last_seen_at, status)
             VALUES ('legacy', 'legacy', 'local', '/home/u/.claude/skills/legacy',
                     '/central/legacy', 1, 1, 1, 'ok')",
            [],
        )
        .expect("seed a v8 row");
    }

    let store = SkillStore::new(db.clone());
    store
        .ensure_schema()
        .expect("upgrade to the current schema");
    store
        .ensure_schema()
        .expect("the next launch finds the version recorded and adds nothing twice");

    let version: i32 = rusqlite::Connection::open(&db)
        .expect("open")
        .query_row("PRAGMA user_version;", [], |row| row.get(0))
        .expect("user_version");
    assert_eq!(version, super::SCHEMA_VERSION);

    let legacy = store
        .get_skill_by_id("legacy")
        .unwrap()
        .expect("legacy row");
    assert_eq!(legacy.source_type, "local");
    assert_eq!(legacy.imported_from_tool, None);

    let mut imported = make_skill("imp", "imp", "/central/imp", 2);
    imported.source_type = "imported".to_string();
    imported.source_ref = None;
    imported.imported_from_tool = Some("claude_code".to_string());
    store.upsert_skill(&imported).unwrap();
    let read = store.get_skill_by_id("imp").unwrap().expect("imported row");
    assert_eq!(read.source_type, "imported");
    assert_eq!(read.source_ref, None);
    assert_eq!(read.imported_from_tool.as_deref(), Some("claude_code"));
    assert_eq!(
        store.list_skills().unwrap()[0]
            .imported_from_tool
            .as_deref(),
        Some("claude_code"),
        "the listing carries the found-in Tool too"
    );
}

/// The incremental upgrade is one transaction: when a later step fails, the
/// steps before it are rolled back and `user_version` is untouched, so the
/// next launch starts the same upgrade from the same state instead of
/// meeting a half-applied one. Simulated here with a v7 database whose V8
/// consolidation trips a trigger halfway through its batch.
#[test]
fn a_failing_migration_step_rolls_back_the_whole_upgrade_and_keeps_the_version() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("test.db");
    open_v8_database(&db);
    {
        let conn = rusqlite::Connection::open(&db).expect("open");
        conn.pragma_update(None, "user_version", 7).expect("v7");
        conn.execute_batch(
            "INSERT INTO skills (id, name, source_type, central_path, created_at, updated_at,
                                 last_seen_at, status)
             VALUES ('s', 's', 'local', '/central/s', 1, 1, 1, 'ok');
             INSERT INTO projects (id, path, created_at, updated_at) VALUES ('p', '/p', 1, 1);
             INSERT INTO project_tools (id, project_id, tool) VALUES ('pt', 'p', 'cursor');
             INSERT INTO project_skill_assignments
                 (id, project_id, skill_id, tool, mode, status, created_at)
             VALUES ('a', 'p', 's', 'cursor', 'symlink', 'synced', 1);
             CREATE TRIGGER simulated_failure BEFORE UPDATE ON project_skill_assignments
             BEGIN SELECT RAISE(ABORT, 'simulated migration failure'); END;",
        )
        .expect("seed a v7 database");
    }

    let store = SkillStore::new(db.clone());
    let err = store
        .ensure_schema()
        .expect_err("the failing step fails the upgrade");
    assert!(
        format!("{err:#}").contains("simulated migration failure"),
        "{err:#}"
    );

    let conn = rusqlite::Connection::open(&db).expect("open");
    let version: i32 = conn
        .query_row("PRAGMA user_version;", [], |row| row.get(0))
        .expect("user_version");
    assert_eq!(version, 7, "the version is written only with the upgrade");
    let tool: String = conn
        .query_row(
            "SELECT tool FROM project_tools WHERE id = 'pt'",
            [],
            |row| row.get(0),
        )
        .expect("project tool row");
    assert_eq!(
        tool, "cursor",
        "the step that had already run is rolled back with the failing one"
    );
}

/// A database left with the V9 column present but `user_version` still 8
/// (a crash between the two before they shared a transaction) upgrades
/// cleanly: the column is added only when it is missing.
#[test]
fn a_partially_applied_v9_upgrade_completes_on_the_next_launch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("test.db");
    open_v8_database(&db);
    rusqlite::Connection::open(&db)
        .expect("open")
        .execute_batch("ALTER TABLE skills ADD COLUMN imported_from_tool TEXT NULL;")
        .expect("the column is already there");

    let store = SkillStore::new(db.clone());
    store
        .ensure_schema()
        .expect("the upgrade tolerates the column it was about to add");

    let version: i32 = rusqlite::Connection::open(&db)
        .expect("open")
        .query_row("PRAGMA user_version;", [], |row| row.get(0))
        .expect("user_version");
    assert_eq!(version, super::SCHEMA_VERSION);
    let mut imported = make_skill("imp", "imp", "/central/imp", 2);
    imported.imported_from_tool = Some("pi".to_string());
    store.upsert_skill(&imported).unwrap();
    assert_eq!(
        store
            .get_skill_by_id("imp")
            .unwrap()
            .expect("row")
            .imported_from_tool
            .as_deref(),
        Some("pi")
    );
}

/// Upgrading a v9 database adds the skill-level Disabled marker: legacy rows
/// read back enabled, the step is idempotent, and a partially applied upgrade
/// (column present, version still 9) completes.
#[test]
fn v10_migration_adds_disabled_at_to_a_v9_database() {
    for column_already_there in [false, true] {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = dir.path().join("test.db");
        open_v8_database(&db);
        {
            let conn = rusqlite::Connection::open(&db).expect("open");
            conn.execute_batch("ALTER TABLE skills ADD COLUMN imported_from_tool TEXT NULL;")
                .expect("v9");
            if column_already_there {
                conn.execute_batch("ALTER TABLE skills ADD COLUMN disabled_at INTEGER NULL;")
                    .expect("partial v10");
            }
            conn.pragma_update(None, "user_version", 9).expect("v9");
            conn.execute(
                "INSERT INTO skills (id, name, source_type, central_path,
                                     created_at, updated_at, last_seen_at, status)
                 VALUES ('legacy', 'legacy', 'local', '/central/legacy', 1, 1, 1, 'ok')",
                [],
            )
            .expect("seed a v9 row");
        }

        let store = SkillStore::new(db.clone());
        store.ensure_schema().expect("upgrade");
        store.ensure_schema().expect("idempotent");
        let version: i32 = rusqlite::Connection::open(&db)
            .expect("open")
            .query_row("PRAGMA user_version;", [], |row| row.get(0))
            .expect("user_version");
        assert_eq!(version, super::SCHEMA_VERSION);
        assert_eq!(store.skill_disabled_at("legacy").unwrap(), None);
        store.set_skill_disabled_at("legacy", Some(42)).unwrap();
        assert_eq!(store.skill_disabled_at("legacy").unwrap(), Some(42));
    }
}

/// The marker lives outside `SkillRecord`, so a whole-record upsert (what a
/// finalize or re-point writes) can never re-enable a disabled skill.
#[test]
fn upserting_the_skill_record_keeps_the_disabled_marker() {
    let (_dir, store) = make_store();
    let skill = make_skill("s1", "alpha", "/central/alpha", 1);
    store.upsert_skill(&skill).unwrap();
    store.set_skill_disabled_at("s1", Some(7)).unwrap();

    store
        .upsert_skill(&make_skill("s1", "alpha", "/central/alpha", 2))
        .unwrap();

    assert_eq!(store.skill_disabled_at("s1").unwrap(), Some(7));
    assert_eq!(store.disabled_skills().unwrap().get("s1").copied(), Some(7));
    store.set_skill_disabled_at("s1", None).unwrap();
    assert!(store.disabled_skills().unwrap().is_empty());
    assert_eq!(store.skill_disabled_at("missing").unwrap(), None);
}

/// The two Disabled transitions: an existing row flips status and keeps its
/// path; a pair with no row gets a marker row; a marker over an existing row
/// keeps that row's id and path.
#[test]
fn disabled_target_transitions_flip_or_record_the_row() {
    let (_dir, store) = make_store();
    store
        .upsert_skill(&make_skill("s1", "alpha", "/central/alpha", 1))
        .unwrap();
    store
        .transition_skill_target(
            "t1",
            TargetTransition::Recorded {
                skill_id: "s1",
                tool: "claude_code",
                mode: SyncMode::Symlink,
                target_path: "/home/.claude/skills/alpha",
                synced_at: 5,
                content_hash: None,
            },
        )
        .unwrap();
    store
        .transition_skill_target("t1", TargetTransition::SyncFailed { error: "boom" })
        .unwrap();
    store
        .transition_skill_target("t1", TargetTransition::Disabled)
        .unwrap();
    let row = store
        .get_skill_target("s1", "claude_code")
        .unwrap()
        .unwrap();
    assert_eq!(row.status, SyncStatus::Disabled);
    assert_eq!(row.last_error, None, "a disabled row carries no failure");
    assert_eq!(row.target_path, "/home/.claude/skills/alpha");

    store
        .transition_skill_target(
            "t2",
            TargetTransition::DisabledRecorded {
                skill_id: "s1",
                tool: "codex",
                mode: SyncMode::Copy,
                target_path: "/home/.codex/skills/alpha",
            },
        )
        .unwrap();
    let marker = store.get_skill_target("s1", "codex").unwrap().unwrap();
    assert_eq!(
        (
            marker.id.as_str(),
            marker.status,
            marker.mode,
            marker.synced_at
        ),
        ("t2", SyncStatus::Disabled, SyncMode::Copy, None)
    );

    store
        .transition_skill_target(
            "t3",
            TargetTransition::DisabledRecorded {
                skill_id: "s1",
                tool: "claude_code",
                mode: SyncMode::Copy,
                target_path: "/elsewhere",
            },
        )
        .unwrap();
    let kept = store
        .get_skill_target("s1", "claude_code")
        .unwrap()
        .unwrap();
    assert_eq!(
        (kept.id.as_str(), kept.target_path.as_str()),
        ("t1", "/home/.claude/skills/alpha")
    );
}

#[test]
fn skills_upsert_list_get_delete() {
    let (_dir, store) = make_store();

    let a = make_skill("a", "A", "/central/a", 10);
    let b = make_skill("b", "B", "/central/b", 20);
    store.upsert_skill(&a).unwrap();
    store.upsert_skill(&b).unwrap();

    let listed = store.list_skills().unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].id, "b");
    assert_eq!(listed[1].id, "a");

    let got = store.get_skill_by_id("a").unwrap().unwrap();
    assert_eq!(got.name, "A");

    let mut a2 = a.clone();
    a2.name = "A2".to_string();
    a2.updated_at = 30;
    store.upsert_skill(&a2).unwrap();
    assert_eq!(store.get_skill_by_id("a").unwrap().unwrap().name, "A2");
    assert_eq!(store.list_skills().unwrap()[0].id, "a");

    store.delete_skill("a").unwrap();
    assert!(store.get_skill_by_id("a").unwrap().is_none());
}

#[test]
fn skill_targets_upsert_unique_constraint_and_list_order() {
    let (_dir, store) = make_store();
    let skill = make_skill("s1", "S1", "/central/s1", 1);
    store.upsert_skill(&skill).unwrap();

    let t1 = SkillTargetRecord {
        id: "t1".to_string(),
        skill_id: "s1".to_string(),
        tool: "cursor".to_string(),
        target_path: "/target/1".to_string(),
        mode: SyncMode::Copy,
        status: SyncStatus::Synced,
        last_error: None,
        synced_at: None,
        content_hash: None,
    };
    store.upsert_skill_target(&t1).unwrap();
    assert_eq!(
        store
            .get_skill_target("s1", "cursor")
            .unwrap()
            .unwrap()
            .target_path,
        "/target/1"
    );

    let mut t1b = t1.clone();
    t1b.id = "t2".to_string();
    t1b.target_path = "/target/2".to_string();
    store.upsert_skill_target(&t1b).unwrap();
    assert_eq!(
        store.get_skill_target("s1", "cursor").unwrap().unwrap().id,
        "t1",
        "on unique(skill_id, tool) conflict, the existing row should be updated rather than the id replaced"
    );
    assert_eq!(
        store
            .get_skill_target("s1", "cursor")
            .unwrap()
            .unwrap()
            .target_path,
        "/target/2"
    );

    let t2 = SkillTargetRecord {
        id: "t3".to_string(),
        skill_id: "s1".to_string(),
        tool: "claude_code".to_string(),
        target_path: "/target/cc".to_string(),
        mode: SyncMode::Copy,
        status: SyncStatus::Synced,
        last_error: None,
        synced_at: None,
        content_hash: None,
    };
    store.upsert_skill_target(&t2).unwrap();

    let targets = store.list_skill_targets("s1").unwrap();
    assert_eq!(targets.len(), 2);
    assert_eq!(targets[0].tool, "claude_code");
    assert_eq!(targets[1].tool, "cursor");

    store.delete_skill_target("s1", "cursor").unwrap();
    assert!(store.get_skill_target("s1", "cursor").unwrap().is_none());
}

#[test]
fn recorded_transition_inserts_a_synced_row_then_settles_the_existing_one() {
    let (_dir, store) = make_store();
    store
        .upsert_skill(&make_skill("s1", "S1", "/central/s1", 1))
        .unwrap();

    // No row yet: the transition creates it under the caller's id.
    store
        .transition_skill_target(
            "t1",
            TargetTransition::Recorded {
                skill_id: "s1",
                tool: "cursor",
                mode: SyncMode::Symlink,
                target_path: "/target/1",
                synced_at: 10,
                content_hash: None,
            },
        )
        .unwrap();
    let row = store.get_skill_target("s1", "cursor").unwrap().unwrap();
    assert_eq!(row.id, "t1");
    assert_eq!(row.mode, SyncMode::Symlink);
    assert_eq!(row.status, SyncStatus::Synced);
    assert_eq!(row.last_error, None);
    assert_eq!(row.synced_at, Some(10));

    // Put the row into an error state, then record a fresh sync for the same
    // (skill, tool) pair under a different id.
    store
        .transition_skill_target("t1", TargetTransition::SyncFailed { error: "boom" })
        .unwrap();
    store
        .transition_skill_target(
            "t2",
            TargetTransition::Recorded {
                skill_id: "s1",
                tool: "cursor",
                mode: SyncMode::Copy,
                target_path: "/target/2",
                synced_at: 20,
                content_hash: None,
            },
        )
        .unwrap();
    let row = store.get_skill_target("s1", "cursor").unwrap().unwrap();
    assert_eq!(
        row.id, "t1",
        "on unique(skill_id, tool) conflict the existing row is settled, not re-keyed"
    );
    assert_eq!(row.mode, SyncMode::Copy);
    assert_eq!(row.target_path, "/target/2");
    assert_eq!(row.status, SyncStatus::Synced);
    assert_eq!(
        row.last_error, None,
        "a recorded sync clears the prior error"
    );
    assert_eq!(row.synced_at, Some(20));
    assert_eq!(store.list_skill_targets("s1").unwrap().len(), 1);
}

#[test]
fn deleting_skill_cascades_targets() {
    let (_dir, store) = make_store();
    let skill = make_skill("s1", "S1", "/central/s1", 1);
    store.upsert_skill(&skill).unwrap();

    let t = SkillTargetRecord {
        id: "t1".to_string(),
        skill_id: "s1".to_string(),
        tool: "cursor".to_string(),
        target_path: "/target/1".to_string(),
        mode: SyncMode::Copy,
        status: SyncStatus::Synced,
        last_error: None,
        synced_at: None,
        content_hash: None,
    };
    store.upsert_skill_target(&t).unwrap();
    assert_eq!(store.list_skill_targets("s1").unwrap().len(), 1);

    store.delete_skill("s1").unwrap();
    assert_eq!(store.list_skill_targets("s1").unwrap().len(), 0);
}

#[test]
fn description_stored_and_retrieved() {
    let (_dir, store) = make_store();
    let mut skill = make_skill("d1", "D1", "/central/d1", 1);
    skill.description = Some("A test skill description".to_string());
    store.upsert_skill(&skill).unwrap();

    let got = store.get_skill_by_id("d1").unwrap().unwrap();
    assert_eq!(got.description.as_deref(), Some("A test skill description"));
}

#[test]
fn description_null_by_default() {
    let (_dir, store) = make_store();
    let skill = make_skill("d2", "D2", "/central/d2", 1);
    store.upsert_skill(&skill).unwrap();

    let got = store.get_skill_by_id("d2").unwrap().unwrap();
    assert!(got.description.is_none());
}

#[test]
fn update_skill_description_backfills() {
    let (_dir, store) = make_store();
    let skill = make_skill("d3", "D3", "/central/d3", 1);
    store.upsert_skill(&skill).unwrap();

    assert!(store
        .get_skill_by_id("d3")
        .unwrap()
        .unwrap()
        .description
        .is_none());

    store
        .update_skill_description("d3", Some("backfilled"))
        .unwrap();
    assert_eq!(
        store
            .get_skill_by_id("d3")
            .unwrap()
            .unwrap()
            .description
            .as_deref(),
        Some("backfilled")
    );
}

#[test]
fn list_skills_missing_description_filters_correctly() {
    let (_dir, store) = make_store();

    let s1 = make_skill("m1", "M1", "/central/m1", 1);
    store.upsert_skill(&s1).unwrap();

    let mut s2 = make_skill("m2", "M2", "/central/m2", 2);
    s2.description = Some("has desc".to_string());
    store.upsert_skill(&s2).unwrap();

    let missing = store.list_skills_missing_description().unwrap();
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].id, "m1");
}

#[test]
fn error_context_includes_db_path() {
    let store = SkillStore::new(PathBuf::from("/this/path/should/not/exist/test.db"));
    let err = store.ensure_schema().unwrap_err();
    let msg = format!("{:#}", err);
    assert!(msg.contains("failed to open db at"), "{msg}");
}

#[test]
fn v4_migration_creates_project_tables() {
    let (_dir, store) = make_store();

    let record = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/project1".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    store.register_project(&record).unwrap();

    let projects = store.list_projects().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].id, "p1");
    assert_eq!(projects[0].path, "/home/user/project1");
    assert_eq!(projects[0].created_at, 100);
    assert_eq!(projects[0].updated_at, 100);
}

#[test]
fn v4_migration_preserves_existing_data() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("test.db");

    // Create a store and add data (this creates current schema)
    let store = SkillStore::new(db.clone());
    store.ensure_schema().expect("initial schema");

    let skill = make_skill("s1", "S1", "/central/s1", 1);
    store.upsert_skill(&skill).unwrap();

    let target = SkillTargetRecord {
        id: "t1".to_string(),
        skill_id: "s1".to_string(),
        tool: "cursor".to_string(),
        target_path: "/target/1".to_string(),
        mode: SyncMode::Copy,
        status: SyncStatus::Synced,
        last_error: None,
        synced_at: None,
        content_hash: None,
    };
    store.upsert_skill_target(&target).unwrap();

    // Call ensure_schema again (simulates app restart / upgrade)
    store.ensure_schema().expect("re-ensure schema");

    // Verify existing data survived
    let skills = store.list_skills().unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].id, "s1");

    let targets = store.list_skill_targets("s1").unwrap();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].tool, "cursor");
}

#[test]
fn v4_tables_have_correct_constraints() {
    let (_dir, store) = make_store();

    let p1 = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/project1".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    store.register_project(&p1).unwrap();

    // Duplicate path should fail with UNIQUE constraint
    let p2 = ProjectRecord {
        id: "p2".to_string(),
        path: "/home/user/project1".to_string(),
        created_at: 200,
        updated_at: 200,
    };
    let result = store.register_project(&p2);
    assert!(
        result.is_err(),
        "duplicate path should fail with UNIQUE constraint"
    );
}

// --- Task 2: Project CRUD tests ---

#[test]
fn register_project() {
    let (_dir, store) = make_store();

    let p = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/myproject".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    store.register_project(&p).unwrap();

    let projects = store.list_projects().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].id, "p1");
    assert_eq!(projects[0].path, "/home/user/myproject");
    assert_eq!(projects[0].created_at, 100);
}

#[test]
fn register_project_duplicate_path_fails() {
    let (_dir, store) = make_store();

    let p1 = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/same".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    store.register_project(&p1).unwrap();

    let p2 = ProjectRecord {
        id: "p2".to_string(),
        path: "/home/user/same".to_string(),
        created_at: 200,
        updated_at: 200,
    };
    assert!(store.register_project(&p2).is_err());
}

#[test]
fn get_project_by_path() {
    let (_dir, store) = make_store();

    let p = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/proj".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    store.register_project(&p).unwrap();

    let found = store.get_project_by_path("/home/user/proj").unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, "p1");

    let missing = store.get_project_by_path("/nonexistent").unwrap();
    assert!(missing.is_none());
}

#[test]
fn delete_project() {
    let (_dir, store) = make_store();

    let p = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/proj".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    store.register_project(&p).unwrap();
    assert_eq!(store.list_projects().unwrap().len(), 1);

    store.delete_project("p1").unwrap();
    assert_eq!(store.list_projects().unwrap().len(), 0);
}

#[test]
fn delete_project_cascades_tools_and_assignments() {
    let (_dir, store) = make_store();

    // Create project
    let p = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/proj".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    store.register_project(&p).unwrap();

    // Add tool
    let tool = ProjectToolRecord {
        id: "pt1".to_string(),
        project_id: "p1".to_string(),
        tool: "cursor".to_string(),
    };
    store.add_project_tool(&tool).unwrap();

    // Add skill (needed for FK)
    let skill = make_skill("s1", "S1", "/central/s1", 1);
    store.upsert_skill(&skill).unwrap();

    // Add assignment
    let assign = ProjectSkillAssignmentRecord {
        id: "a1".to_string(),
        project_id: "p1".to_string(),
        skill_id: "s1".to_string(),
        skill_name: "S1".to_string(),
        tool: "cursor".to_string(),
        mode: SyncMode::Symlink,
        status: SyncStatus::Pending,
        last_error: None,
        synced_at: None,
        content_hash: None,
        created_at: 100,
    };
    store.add_project_skill_assignment(&assign).unwrap();

    // Verify data exists
    assert_eq!(store.list_project_tools("p1").unwrap().len(), 1);
    assert_eq!(store.list_project_skill_assignments("p1").unwrap().len(), 1);

    // Delete project -- CASCADE should remove tools and assignments
    store.delete_project("p1").unwrap();
    assert_eq!(store.list_project_tools("p1").unwrap().len(), 0);
    assert_eq!(store.list_project_skill_assignments("p1").unwrap().len(), 0);

    // Skill itself should remain
    assert!(store.get_skill_by_id("s1").unwrap().is_some());
}

#[test]
fn delete_skill_cascades_project_assignments() {
    let (_dir, store) = make_store();

    // Create project + tool
    let p = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/proj".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    store.register_project(&p).unwrap();

    let tool = ProjectToolRecord {
        id: "pt1".to_string(),
        project_id: "p1".to_string(),
        tool: "cursor".to_string(),
    };
    store.add_project_tool(&tool).unwrap();

    // Create skill
    let skill = make_skill("s1", "S1", "/central/s1", 1);
    store.upsert_skill(&skill).unwrap();

    // Assign skill to project
    let assign = ProjectSkillAssignmentRecord {
        id: "a1".to_string(),
        project_id: "p1".to_string(),
        skill_id: "s1".to_string(),
        skill_name: "S1".to_string(),
        tool: "cursor".to_string(),
        mode: SyncMode::Symlink,
        status: SyncStatus::Synced,
        last_error: None,
        synced_at: Some(100),
        content_hash: None,
        created_at: 100,
    };
    store.add_project_skill_assignment(&assign).unwrap();
    assert_eq!(store.list_project_skill_assignments("p1").unwrap().len(), 1);

    // Delete skill -- CASCADE should remove assignment but keep project and tool
    store.delete_skill("s1").unwrap();
    assert_eq!(store.list_project_skill_assignments("p1").unwrap().len(), 0);
    assert!(store
        .get_project_by_path("/home/user/proj")
        .unwrap()
        .is_some());
    assert_eq!(store.list_project_tools("p1").unwrap().len(), 1);
}

#[test]
fn project_tools_crud() {
    let (_dir, store) = make_store();

    let p = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/proj".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    store.register_project(&p).unwrap();

    let tool = ProjectToolRecord {
        id: "pt1".to_string(),
        project_id: "p1".to_string(),
        tool: "cursor".to_string(),
    };
    store.add_project_tool(&tool).unwrap();

    let tools = store.list_project_tools("p1").unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].tool, "cursor");

    store.remove_project_tool("p1", "cursor").unwrap();
    assert_eq!(store.list_project_tools("p1").unwrap().len(), 0);
}

#[test]
fn project_tools_duplicate_ignored() {
    let (_dir, store) = make_store();

    let p = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/proj".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    store.register_project(&p).unwrap();

    let tool1 = ProjectToolRecord {
        id: "pt1".to_string(),
        project_id: "p1".to_string(),
        tool: "cursor".to_string(),
    };
    store.add_project_tool(&tool1).unwrap();

    // Same project+tool, different id -- should be ignored (INSERT OR IGNORE)
    let tool2 = ProjectToolRecord {
        id: "pt2".to_string(),
        project_id: "p1".to_string(),
        tool: "cursor".to_string(),
    };
    store.add_project_tool(&tool2).unwrap();

    let tools = store.list_project_tools("p1").unwrap();
    assert_eq!(tools.len(), 1);
}

#[test]
fn assignment_crud() {
    let (_dir, store) = make_store();

    // Setup: project + skill
    let p = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/proj".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    store.register_project(&p).unwrap();

    let skill = make_skill("s1", "S1", "/central/s1", 1);
    store.upsert_skill(&skill).unwrap();

    let assign = ProjectSkillAssignmentRecord {
        id: "a1".to_string(),
        project_id: "p1".to_string(),
        skill_id: "s1".to_string(),
        skill_name: "S1".to_string(),
        tool: "cursor".to_string(),
        mode: SyncMode::Symlink,
        status: SyncStatus::Pending,
        last_error: None,
        synced_at: None,
        content_hash: None,
        created_at: 100,
    };
    store.add_project_skill_assignment(&assign).unwrap();

    let assignments = store.list_project_skill_assignments("p1").unwrap();
    assert_eq!(assignments.len(), 1);
    assert_eq!(assignments[0].skill_id, "s1");
    assert_eq!(assignments[0].tool, "cursor");
    assert_eq!(assignments[0].mode, SyncMode::Symlink);
    assert_eq!(assignments[0].status, SyncStatus::Pending);

    store
        .remove_project_skill_assignment("p1", "s1", "cursor")
        .unwrap();
    assert_eq!(store.list_project_skill_assignments("p1").unwrap().len(), 0);
}

#[test]
fn assignment_unique_constraint() {
    let (_dir, store) = make_store();

    let p = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/proj".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    store.register_project(&p).unwrap();

    let skill = make_skill("s1", "S1", "/central/s1", 1);
    store.upsert_skill(&skill).unwrap();

    let assign1 = ProjectSkillAssignmentRecord {
        id: "a1".to_string(),
        project_id: "p1".to_string(),
        skill_id: "s1".to_string(),
        skill_name: "S1".to_string(),
        tool: "cursor".to_string(),
        mode: SyncMode::Symlink,
        status: SyncStatus::Pending,
        last_error: None,
        synced_at: None,
        content_hash: None,
        created_at: 100,
    };
    store.add_project_skill_assignment(&assign1).unwrap();

    // Same project+skill+tool should fail
    let assign2 = ProjectSkillAssignmentRecord {
        id: "a2".to_string(),
        project_id: "p1".to_string(),
        skill_id: "s1".to_string(),
        skill_name: "S1".to_string(),
        tool: "cursor".to_string(),
        mode: SyncMode::Copy,
        status: SyncStatus::Pending,
        last_error: None,
        synced_at: None,
        content_hash: None,
        created_at: 200,
    };
    assert!(
        store.add_project_skill_assignment(&assign2).is_err(),
        "duplicate project+skill+tool should fail with UNIQUE constraint"
    );
}

#[test]
fn list_project_assignments_by_project() {
    let (_dir, store) = make_store();

    // Two projects
    let p1 = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/proj1".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    let p2 = ProjectRecord {
        id: "p2".to_string(),
        path: "/home/user/proj2".to_string(),
        created_at: 200,
        updated_at: 200,
    };
    store.register_project(&p1).unwrap();
    store.register_project(&p2).unwrap();

    // Two skills
    let s1 = make_skill("s1", "S1", "/central/s1", 1);
    let s2 = make_skill("s2", "S2", "/central/s2", 2);
    store.upsert_skill(&s1).unwrap();
    store.upsert_skill(&s2).unwrap();

    // Assign s1 to p1, s2 to p2
    let a1 = ProjectSkillAssignmentRecord {
        id: "a1".to_string(),
        project_id: "p1".to_string(),
        skill_id: "s1".to_string(),
        skill_name: "S1".to_string(),
        tool: "cursor".to_string(),
        mode: SyncMode::Symlink,
        status: SyncStatus::Synced,
        last_error: None,
        synced_at: Some(100),
        content_hash: None,
        created_at: 100,
    };
    let a2 = ProjectSkillAssignmentRecord {
        id: "a2".to_string(),
        project_id: "p2".to_string(),
        skill_id: "s2".to_string(),
        skill_name: "S2".to_string(),
        tool: "cursor".to_string(),
        mode: SyncMode::Symlink,
        status: SyncStatus::Synced,
        last_error: None,
        synced_at: Some(200),
        content_hash: None,
        created_at: 200,
    };
    store.add_project_skill_assignment(&a1).unwrap();
    store.add_project_skill_assignment(&a2).unwrap();

    // list by project returns only that project's assignments
    let p1_assignments = store.list_project_skill_assignments("p1").unwrap();
    assert_eq!(p1_assignments.len(), 1);
    assert_eq!(p1_assignments[0].skill_id, "s1");

    let p2_assignments = store.list_project_skill_assignments("p2").unwrap();
    assert_eq!(p2_assignments.len(), 1);
    assert_eq!(p2_assignments[0].skill_id, "s2");
}

#[test]
fn aggregate_sync_status_all_synced() {
    let (_dir, store) = make_store();

    let p = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/proj".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    store.register_project(&p).unwrap();

    let s1 = make_skill("s1", "S1", "/central/s1", 1);
    let s2 = make_skill("s2", "S2", "/central/s2", 2);
    store.upsert_skill(&s1).unwrap();
    store.upsert_skill(&s2).unwrap();

    let a1 = ProjectSkillAssignmentRecord {
        id: "a1".to_string(),
        project_id: "p1".to_string(),
        skill_id: "s1".to_string(),
        skill_name: "S1".to_string(),
        tool: "cursor".to_string(),
        mode: SyncMode::Symlink,
        status: SyncStatus::Synced,
        last_error: None,
        synced_at: Some(100),
        content_hash: None,
        created_at: 100,
    };
    let a2 = ProjectSkillAssignmentRecord {
        id: "a2".to_string(),
        project_id: "p1".to_string(),
        skill_id: "s2".to_string(),
        skill_name: "S2".to_string(),
        tool: "cursor".to_string(),
        mode: SyncMode::Symlink,
        status: SyncStatus::Synced,
        last_error: None,
        synced_at: Some(200),
        content_hash: None,
        created_at: 200,
    };
    store.add_project_skill_assignment(&a1).unwrap();
    store.add_project_skill_assignment(&a2).unwrap();

    assert_eq!(
        store.project_aggregate("p1").unwrap().sync_status,
        ProjectSyncStatus::Synced
    );
}

#[test]
fn aggregate_sync_status_mixed() {
    let (_dir, store) = make_store();

    let p = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/proj".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    store.register_project(&p).unwrap();

    let s1 = make_skill("s1", "S1", "/central/s1", 1);
    let s2 = make_skill("s2", "S2", "/central/s2", 2);
    store.upsert_skill(&s1).unwrap();
    store.upsert_skill(&s2).unwrap();

    let a1 = ProjectSkillAssignmentRecord {
        id: "a1".to_string(),
        project_id: "p1".to_string(),
        skill_id: "s1".to_string(),
        skill_name: "S1".to_string(),
        tool: "cursor".to_string(),
        mode: SyncMode::Symlink,
        status: SyncStatus::Synced,
        last_error: None,
        synced_at: Some(100),
        content_hash: None,
        created_at: 100,
    };
    let a2 = ProjectSkillAssignmentRecord {
        id: "a2".to_string(),
        project_id: "p1".to_string(),
        skill_id: "s2".to_string(),
        skill_name: "S2".to_string(),
        tool: "cursor".to_string(),
        mode: SyncMode::Symlink,
        status: SyncStatus::Error,
        last_error: Some("symlink failed".to_string()),
        synced_at: None,
        content_hash: None,
        created_at: 200,
    };
    store.add_project_skill_assignment(&a1).unwrap();
    store.add_project_skill_assignment(&a2).unwrap();

    assert_eq!(
        store.project_aggregate("p1").unwrap().sync_status,
        ProjectSyncStatus::Error
    );
}

#[test]
fn aggregate_sync_status_no_assignments() {
    let (_dir, store) = make_store();

    let p = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/proj".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    store.register_project(&p).unwrap();

    assert_eq!(
        store.project_aggregate("p1").unwrap().sync_status,
        ProjectSyncStatus::Empty
    );
}

/// The grouped read is what the project listing uses: every project's
/// numbers and status fold arrive from one pass, so three projects with
/// different statuses come back correct without a per-project query.
#[test]
fn project_aggregates_reads_every_project_in_one_pass() {
    let (_dir, store) = make_store();
    for (id, path) in [("p1", "/a"), ("p2", "/b"), ("p3", "/c")] {
        store
            .register_project(&ProjectRecord {
                id: id.to_string(),
                path: path.to_string(),
                created_at: 1,
                updated_at: 1,
            })
            .unwrap();
    }
    for (id, project_id, tool) in [
        ("t1", "p1", "cursor"),
        ("t2", "p1", "claude_code"),
        ("t3", "p2", "cursor"),
    ] {
        store
            .add_project_tool(&ProjectToolRecord {
                id: id.to_string(),
                project_id: project_id.to_string(),
                tool: tool.to_string(),
            })
            .unwrap();
    }
    for skill in ["s1", "s2"] {
        store
            .upsert_skill(&make_skill(skill, skill, &format!("/central/{skill}"), 1))
            .unwrap();
    }
    // p1: one skill on two tools, one stale — stale wins over synced.
    // p2: two skills, one errored — error wins.
    // p3: nothing at all — empty.
    for (id, project_id, skill_id, tool, status) in [
        ("a1", "p1", "s1", "cursor", SyncStatus::Synced),
        ("a2", "p1", "s1", "claude_code", SyncStatus::Stale),
        ("a3", "p2", "s1", "cursor", SyncStatus::Synced),
        ("a4", "p2", "s2", "cursor", SyncStatus::Error),
    ] {
        store
            .add_project_skill_assignment(&ProjectSkillAssignmentRecord {
                id: id.to_string(),
                project_id: project_id.to_string(),
                skill_id: skill_id.to_string(),
                skill_name: skill_id.to_string(),
                tool: tool.to_string(),
                mode: SyncMode::Symlink,
                status,
                last_error: None,
                synced_at: None,
                content_hash: None,
                created_at: 1,
            })
            .unwrap();
    }

    let aggregates = store.project_aggregates().unwrap();

    assert_eq!(
        aggregates.get("p1").cloned().unwrap_or_default(),
        ProjectAggregate {
            tool_count: 2,
            skill_count: 1,
            assignment_count: 2,
            sync_status: ProjectSyncStatus::Stale,
        }
    );
    assert_eq!(
        aggregates.get("p2").cloned().unwrap_or_default(),
        ProjectAggregate {
            tool_count: 1,
            skill_count: 2,
            assignment_count: 2,
            sync_status: ProjectSyncStatus::Error,
        }
    );
    assert_eq!(
        aggregates.get("p3").cloned().unwrap_or_default(),
        ProjectAggregate::default(),
        "a project with nothing configured reads as the default aggregate"
    );
    // Same numbers through the single-project door.
    assert_eq!(
        store.project_aggregate("p1").unwrap(),
        aggregates.get("p1").cloned().unwrap()
    );
}

#[test]
fn project_aggregate_counts_tools_assignments_and_skills() {
    let (_dir, store) = make_store();

    let p = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/proj".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    store.register_project(&p).unwrap();

    assert_eq!(
        store.project_aggregate("p1").unwrap(),
        ProjectAggregate::default()
    );

    let tool = ProjectToolRecord {
        id: "pt1".to_string(),
        project_id: "p1".to_string(),
        tool: "cursor".to_string(),
    };
    store.add_project_tool(&tool).unwrap();
    assert_eq!(store.project_aggregate("p1").unwrap().tool_count, 1);

    let skill = make_skill("s1", "S1", "/central/s1", 1);
    store.upsert_skill(&skill).unwrap();

    let assign = ProjectSkillAssignmentRecord {
        id: "a1".to_string(),
        project_id: "p1".to_string(),
        skill_id: "s1".to_string(),
        skill_name: "S1".to_string(),
        tool: "cursor".to_string(),
        mode: SyncMode::Symlink,
        status: SyncStatus::Pending,
        last_error: None,
        synced_at: None,
        content_hash: None,
        created_at: 100,
    };
    store.add_project_skill_assignment(&assign).unwrap();
    let aggregate = store.project_aggregate("p1").unwrap();
    assert_eq!(aggregate.assignment_count, 1);
    assert_eq!(aggregate.skill_count, 1);
}

#[test]
fn list_project_skill_assignments_for_project_tool_filters() {
    let (_dir, store) = make_store();

    let p = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/proj".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    store.register_project(&p).unwrap();

    let s1 = make_skill("s1", "S1", "/central/s1", 1);
    let s2 = make_skill("s2", "S2", "/central/s2", 2);
    store.upsert_skill(&s1).unwrap();
    store.upsert_skill(&s2).unwrap();

    let a1 = ProjectSkillAssignmentRecord {
        id: "a1".to_string(),
        project_id: "p1".to_string(),
        skill_id: "s1".to_string(),
        skill_name: "S1".to_string(),
        tool: "cursor".to_string(),
        mode: SyncMode::Symlink,
        status: SyncStatus::Synced,
        last_error: None,
        synced_at: Some(100),
        content_hash: None,
        created_at: 100,
    };
    let a2 = ProjectSkillAssignmentRecord {
        id: "a2".to_string(),
        project_id: "p1".to_string(),
        skill_id: "s2".to_string(),
        skill_name: "S2".to_string(),
        tool: "claude_code".to_string(),
        mode: SyncMode::Symlink,
        status: SyncStatus::Synced,
        last_error: None,
        synced_at: Some(200),
        content_hash: None,
        created_at: 200,
    };
    store.add_project_skill_assignment(&a1).unwrap();
    store.add_project_skill_assignment(&a2).unwrap();

    let cursor_assigns = store
        .list_project_skill_assignments_for_project_tool("p1", "cursor")
        .unwrap();
    assert_eq!(cursor_assigns.len(), 1);
    assert_eq!(cursor_assigns[0].skill_id, "s1");

    let cc_assigns = store
        .list_project_skill_assignments_for_project_tool("p1", "claude_code")
        .unwrap();
    assert_eq!(cc_assigns.len(), 1);
    assert_eq!(cc_assigns[0].skill_id, "s2");
}

#[test]
fn get_project_by_id() {
    let (_dir, store) = make_store();

    let p = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/proj".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    store.register_project(&p).unwrap();

    let found = store.get_project_by_id("p1").unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().path, "/home/user/proj");

    let missing = store.get_project_by_id("nonexistent").unwrap();
    assert!(missing.is_none());
}

#[test]
fn v5_migration_adds_content_hash() {
    let (_dir, store) = make_store();

    // Register a project and skill
    let p = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/proj".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    store.register_project(&p).unwrap();

    let skill = make_skill("s1", "S1", "/central/s1", 1);
    store.upsert_skill(&skill).unwrap();

    // Create an assignment
    let assign = ProjectSkillAssignmentRecord {
        id: "a1".to_string(),
        project_id: "p1".to_string(),
        skill_id: "s1".to_string(),
        skill_name: "S1".to_string(),
        tool: "cursor".to_string(),
        mode: SyncMode::Symlink,
        status: SyncStatus::Pending,
        last_error: None,
        synced_at: None,
        content_hash: None,
        created_at: 100,
    };
    store.add_project_skill_assignment(&assign).unwrap();

    // Update with content_hash
    store
        .transition_assignment(
            "a1",
            AssignmentTransition::SyncCompleted {
                mode: SyncMode::Copy,
                synced_at: 1000,
                content_hash: Some("abc123"),
            },
        )
        .unwrap();

    // Read back via get_project_skill_assignment
    let got = store
        .get_project_skill_assignment("p1", "s1", "cursor")
        .unwrap()
        .unwrap();
    assert_eq!(got.content_hash.as_deref(), Some("abc123"));
    assert_eq!(got.status, SyncStatus::Synced);
    assert_eq!(got.synced_at, Some(1000));
    assert_eq!(got.mode, SyncMode::Copy);
}

#[test]
fn update_assignment_status_coalesce() {
    let (_dir, store) = make_store();

    let p = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/proj".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    store.register_project(&p).unwrap();

    let skill = make_skill("s1", "S1", "/central/s1", 1);
    store.upsert_skill(&skill).unwrap();

    let assign = ProjectSkillAssignmentRecord {
        id: "a1".to_string(),
        project_id: "p1".to_string(),
        skill_id: "s1".to_string(),
        skill_name: "S1".to_string(),
        tool: "cursor".to_string(),
        mode: SyncMode::Symlink,
        status: SyncStatus::Pending,
        last_error: None,
        synced_at: None,
        content_hash: None,
        created_at: 100,
    };
    store.add_project_skill_assignment(&assign).unwrap();

    // First update: set synced with synced_at=1000 and mode="symlink"
    store
        .transition_assignment(
            "a1",
            AssignmentTransition::SyncCompleted {
                mode: SyncMode::Symlink,
                synced_at: 1000,
                content_hash: None,
            },
        )
        .unwrap();

    let got = store
        .get_project_skill_assignment("p1", "s1", "cursor")
        .unwrap()
        .unwrap();
    assert_eq!(got.status, SyncStatus::Synced);
    assert_eq!(got.synced_at, Some(1000));
    assert_eq!(got.mode, SyncMode::Symlink);

    // Second update: set error with last_error but synced_at=None (should preserve synced_at=1000)
    store
        .transition_assignment("a1", AssignmentTransition::SyncFailed { error: "fail" })
        .unwrap();

    let got = store
        .get_project_skill_assignment("p1", "s1", "cursor")
        .unwrap()
        .unwrap();
    assert_eq!(got.status, SyncStatus::Error);
    assert_eq!(got.last_error.as_deref(), Some("fail"));
    assert_eq!(
        got.synced_at,
        Some(1000),
        "COALESCE should preserve synced_at"
    );
    assert_eq!(got.mode, SyncMode::Symlink, "COALESCE should preserve mode");
}

#[test]
fn get_project_skill_assignment_returns_none() {
    let (_dir, store) = make_store();

    let result = store
        .get_project_skill_assignment("nonexistent", "nonexistent", "nonexistent")
        .unwrap();
    assert!(result.is_none());
}

#[test]
fn project_aggregate_sync_status_treats_missing_as_error() {
    let (_dir, store) = make_store();
    let project = ProjectRecord {
        id: "p1".to_string(),
        path: "/tmp/proj".to_string(),
        created_at: 1,
        updated_at: 1,
    };
    store.register_project(&project).unwrap();
    let skill = make_skill("s1", "S1", "/central/s1", 1);
    store.upsert_skill(&skill).unwrap();
    let assignment = ProjectSkillAssignmentRecord {
        id: "a1".to_string(),
        project_id: "p1".to_string(),
        skill_id: "s1".to_string(),
        skill_name: "S1".to_string(),
        tool: "claude_code".to_string(),
        mode: SyncMode::Symlink,
        status: SyncStatus::Missing,
        last_error: None,
        synced_at: None,
        content_hash: None,
        created_at: 1,
    };
    store.add_project_skill_assignment(&assignment).unwrap();
    let status = store.project_aggregate("p1").unwrap().sync_status;
    assert_eq!(status, ProjectSyncStatus::Error);
}

#[test]
fn update_project_path_changes_path() {
    let (_dir, store) = make_store();
    let project = ProjectRecord {
        id: "p1".to_string(),
        path: "/tmp/old".to_string(),
        created_at: 1,
        updated_at: 1,
    };
    store.register_project(&project).unwrap();
    store.update_project_path("p1", "/tmp/new", 2).unwrap();
    let updated = store.get_project_by_id("p1").unwrap().unwrap();
    assert_eq!(updated.path, "/tmp/new");
    assert_eq!(updated.updated_at, 2);
}

#[test]
fn list_project_skill_assignments_by_skill_returns_correct_rows() {
    let (_dir, store) = make_store();
    let project = ProjectRecord {
        id: "p1".to_string(),
        path: "/tmp/proj".to_string(),
        created_at: 1,
        updated_at: 1,
    };
    store.register_project(&project).unwrap();
    let s1 = make_skill("s1", "A", "/central/a", 10);
    let s2 = make_skill("s2", "B", "/central/b", 10);
    store.upsert_skill(&s1).unwrap();
    store.upsert_skill(&s2).unwrap();
    store
        .add_project_tool(&ProjectToolRecord {
            id: "pt1".to_string(),
            project_id: "p1".to_string(),
            tool: "claude_code".to_string(),
        })
        .unwrap();
    store
        .add_project_skill_assignment(&ProjectSkillAssignmentRecord {
            id: "a1".to_string(),
            project_id: "p1".to_string(),
            skill_id: "s1".to_string(),
            skill_name: "A".to_string(),
            tool: "claude_code".to_string(),
            mode: SyncMode::Symlink,
            status: SyncStatus::Synced,
            last_error: None,
            synced_at: Some(10),
            content_hash: None,
            created_at: 1,
        })
        .unwrap();
    store
        .add_project_skill_assignment(&ProjectSkillAssignmentRecord {
            id: "a2".to_string(),
            project_id: "p1".to_string(),
            skill_id: "s2".to_string(),
            skill_name: "B".to_string(),
            tool: "claude_code".to_string(),
            mode: SyncMode::Symlink,
            status: SyncStatus::Synced,
            last_error: None,
            synced_at: Some(10),
            content_hash: None,
            created_at: 1,
        })
        .unwrap();
    let results = store.list_project_skill_assignments_by_skill("s1").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].skill_id, "s1");
}

// ---------------------------------------------------------------------------
// Lifecycle store seam: stored strings round-trip; legacy/unknown values are
// surfaced, never panicked on or read as healthy.
// ---------------------------------------------------------------------------

fn raw_conn(store: &SkillStore) -> rusqlite::Connection {
    rusqlite::Connection::open(store.db_path()).expect("open raw connection")
}

fn seed_project_and_skill(store: &SkillStore) {
    store
        .register_project(&ProjectRecord {
            id: "p1".to_string(),
            path: "/tmp/proj".to_string(),
            created_at: 1,
            updated_at: 1,
        })
        .unwrap();
    store
        .upsert_skill(&make_skill("s1", "S1", "/central/s1", 1))
        .unwrap();
}

#[test]
fn lifecycle_round_trips_every_status_and_mode_through_stored_strings() {
    let (_dir, store) = make_store();
    seed_project_and_skill(&store);
    let mut n = 0;
    for status in SyncStatus::ALL {
        for mode in SyncMode::ALL {
            n += 1;
            let tool = format!("tool{n}");
            store
                .add_project_skill_assignment(&ProjectSkillAssignmentRecord {
                    id: format!("a{n}"),
                    project_id: "p1".to_string(),
                    skill_id: "s1".to_string(),
                    skill_name: "S1".to_string(),
                    tool: tool.clone(),
                    mode,
                    status,
                    last_error: None,
                    synced_at: None,
                    content_hash: None,
                    created_at: 1,
                })
                .unwrap();
            let got = store
                .get_project_skill_assignment("p1", "s1", &tool)
                .unwrap()
                .unwrap();
            assert_eq!((got.mode, got.status), (mode, status));
            assert_eq!(got.last_error, None);
            // The column holds exactly the historical spelling (no schema change).
            let raw: (String, String) = raw_conn(&store)
                .query_row(
                    "SELECT mode, status FROM project_skill_assignments WHERE id = ?1",
                    rusqlite::params![format!("a{n}")],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap();
            assert_eq!(
                raw,
                (mode.as_str().to_string(), status.as_str().to_string())
            );
        }
    }
}

#[test]
fn legacy_ok_skill_target_reads_as_synced() {
    let (_dir, store) = make_store();
    store
        .upsert_skill(&make_skill("s1", "S1", "/central/s1", 1))
        .unwrap();
    raw_conn(&store)
        .execute(
            "INSERT INTO skill_targets (id, skill_id, tool, target_path, mode, status, last_error, synced_at)
             VALUES ('t1', 's1', 'claude_code', '/x', 'symlink', 'ok', NULL, 5)",
            [],
        )
        .unwrap();
    let got = store
        .get_skill_target("s1", "claude_code")
        .unwrap()
        .unwrap();
    assert_eq!(got.status, SyncStatus::Synced);
    assert_eq!(got.mode, SyncMode::Symlink);
    assert_eq!(got.last_error, None);
}

#[test]
fn unknown_stored_status_surfaces_as_error_with_diagnostic() {
    let (_dir, store) = make_store();
    seed_project_and_skill(&store);
    raw_conn(&store)
        .execute(
            "INSERT INTO project_skill_assignments
             (id, project_id, skill_id, skill_name, tool, mode, status, last_error, synced_at, content_hash, created_at)
             VALUES ('a1', 'p1', 's1', 'S1', 'claude_code', 'symlink', 'weird', NULL, NULL, NULL, 1)",
            [],
        )
        .unwrap();
    let got = store
        .get_project_skill_assignment("p1", "s1", "claude_code")
        .unwrap()
        .unwrap();
    assert_eq!(got.status, SyncStatus::Error);
    assert_eq!(got.mode, SyncMode::Symlink, "a valid mode is kept");
    let diagnostic = got.last_error.expect("diagnostic recorded");
    assert!(diagnostic.contains("weird"), "got: {diagnostic}");
    // The listing path and the aggregate agree, and the row is untouched on disk.
    assert_eq!(
        store.list_project_skill_assignments("p1").unwrap()[0].status,
        SyncStatus::Error
    );
    assert_eq!(
        store.project_aggregate("p1").unwrap().sync_status,
        ProjectSyncStatus::Error
    );
    let raw: String = raw_conn(&store)
        .query_row(
            "SELECT status FROM project_skill_assignments WHERE id = 'a1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(raw, "weird", "reading must not rewrite the row");
}

#[test]
fn unknown_stored_mode_reads_as_copy_and_error() {
    let (_dir, store) = make_store();
    seed_project_and_skill(&store);
    raw_conn(&store)
        .execute(
            "INSERT INTO project_skill_assignments
             (id, project_id, skill_id, skill_name, tool, mode, status, last_error, synced_at, content_hash, created_at)
             VALUES ('a1', 'p1', 's1', 'S1', 'claude_code', 'auto', 'synced', NULL, NULL, NULL, 1)",
            [],
        )
        .unwrap();
    let got = store
        .get_project_skill_assignment("p1", "s1", "claude_code")
        .unwrap()
        .unwrap();
    assert_eq!(got.mode, SyncMode::Copy);
    assert_eq!(got.status, SyncStatus::Error);
    assert!(got.last_error.unwrap().contains("auto"));
}

#[test]
fn reconciled_transition_writes_status_and_hash_only() {
    let (_dir, store) = make_store();
    seed_project_and_skill(&store);
    store
        .add_project_skill_assignment(&ProjectSkillAssignmentRecord {
            id: "a1".to_string(),
            project_id: "p1".to_string(),
            skill_id: "s1".to_string(),
            skill_name: "S1".to_string(),
            tool: "cursor".to_string(),
            mode: SyncMode::Copy,
            status: SyncStatus::Error,
            last_error: Some("boom".to_string()),
            synced_at: Some(7),
            content_hash: Some("h1".to_string()),
            created_at: 1,
        })
        .unwrap();
    store
        .transition_assignment(
            "a1",
            AssignmentTransition::Reconciled {
                status: SyncStatus::Synced,
            },
        )
        .unwrap();
    let got = store
        .get_project_skill_assignment("p1", "s1", "cursor")
        .unwrap()
        .unwrap();
    assert_eq!(got.status, SyncStatus::Synced);
    assert_eq!(
        got.last_error, None,
        "a reconcile clears the stale diagnostic"
    );
    assert_eq!(
        got.content_hash.as_deref(),
        Some("h1"),
        "the deployed identity is evidence, kept by a reconcile"
    );
    assert_eq!(
        got.synced_at,
        Some(7),
        "nothing was written, so synced_at is kept"
    );
    assert_eq!(got.mode, SyncMode::Copy);
}

// ---------------------------------------------------------------------------
// migrate_legacy_db_if_needed — the legacy-identifier probe root is passed in,
// so the whole adoption rule is testable against a temp data root.
// ---------------------------------------------------------------------------

fn seeded_db(path: &std::path::Path, skill_name: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let store = SkillStore::new(path.to_path_buf());
    store.ensure_schema().unwrap();
    store
        .upsert_skill(&super::SkillRecord {
            id: format!("id-{skill_name}"),
            name: skill_name.to_string(),
            description: None,
            source_type: "local".to_string(),
            source_ref: None,
            source_subpath: None,
            source_revision: None,
            central_path: "/tmp/x".to_string(),
            content_hash: None,
            created_at: 1,
            updated_at: 1,
            last_sync_at: None,
            last_seen_at: 1,
            status: "active".to_string(),
            imported_from_tool: None,
        })
        .unwrap();
}

fn skill_names(path: &std::path::Path) -> Vec<String> {
    SkillStore::new(path.to_path_buf())
        .list_skills()
        .unwrap()
        .into_iter()
        .map(|s| s.name)
        .collect()
}

#[test]
fn legacy_db_is_adopted_when_the_target_has_no_skills() {
    let root = tempfile::tempdir().unwrap();
    let data_root = root.path().join("data");
    let legacy = data_root
        .join("com.qufei1993.skillshub")
        .join("skills_hub.db");
    seeded_db(&legacy, "from-legacy");

    let target = root.path().join("current").join("skills_hub.db");
    super::migrate_legacy_db_if_needed(&data_root, &target).unwrap();

    assert!(target.exists(), "legacy db must be adopted");
    assert_eq!(skill_names(&target), vec!["from-legacy".to_string()]);
}

#[test]
fn a_target_that_already_has_skills_is_left_alone() {
    let root = tempfile::tempdir().unwrap();
    let data_root = root.path().join("data");
    seeded_db(
        &data_root.join("com.tauri.dev").join("skills_hub.db"),
        "from-legacy",
    );

    let target = root.path().join("current").join("skills_hub.db");
    seeded_db(&target, "already-here");
    super::migrate_legacy_db_if_needed(&data_root, &target).unwrap();

    assert_eq!(skill_names(&target), vec!["already-here".to_string()]);
}

#[test]
fn no_legacy_db_under_the_data_root_is_a_no_op() {
    let root = tempfile::tempdir().unwrap();
    let data_root = root.path().join("data");
    std::fs::create_dir_all(&data_root).unwrap();

    let target = root.path().join("current").join("skills_hub.db");
    super::migrate_legacy_db_if_needed(&data_root, &target).unwrap();
    assert!(!target.exists(), "nothing to adopt, nothing created");
}

/// Every call opens a fresh connection, and the Refresh acquire pool runs four
/// of them at once (`refresh::acquire_all` -> `installer::acquire_update`, which
/// reads and then upserts). Without a `busy_timeout` a call landing inside a
/// sibling's write window fails immediately with `SQLITE_BUSY`; with one it
/// waits for the window to close.
#[test]
fn a_call_landing_in_a_write_window_waits_instead_of_failing_busy() {
    let (_dir, store) = make_store();
    store
        .upsert_skill(&make_skill("held", "held", "/tmp/central/held", 1))
        .expect("seed");

    let db_path = store.db_path.clone();
    let (locked_tx, locked_rx) = std::sync::mpsc::channel();
    let holder = std::thread::spawn(move || {
        let conn = rusqlite::Connection::open(&db_path).expect("open competing connection");
        conn.execute_batch("BEGIN EXCLUSIVE;")
            .expect("take the write lock");
        locked_tx.send(()).expect("announce the lock");
        std::thread::sleep(std::time::Duration::from_millis(300));
        conn.execute_batch("COMMIT;")
            .expect("release the write lock");
    });
    locked_rx.recv().expect("lock taken");

    // The window is open right now: without a busy timeout both of these
    // return SQLITE_BUSY at once.
    store
        .get_skill_by_id("held")
        .expect("a read waits out the write window");
    store
        .upsert_skill(&make_skill("held", "held", "/tmp/central/held", 2))
        .expect("a write waits out the write window");

    holder.join().expect("holder thread");
}

/// The Refresh acquire pool's shape: `workers` threads each doing the read +
/// upsert pair `rounds` times against one database, every call `Ok`.
fn read_and_upsert_concurrently(workers: usize, rounds: usize) {
    let (_dir, store) = make_store();
    std::thread::scope(|scope| {
        for worker in 0..workers {
            let store = store.clone();
            scope.spawn(move || {
                let id = format!("skill-{worker}");
                let central = format!("/tmp/central/{id}");
                for round in 0..rounds {
                    let skill = make_skill(&id, &id, &central, round as i64);
                    store
                        .upsert_skill(&skill)
                        .unwrap_or_else(|err| panic!("upsert in worker {worker}: {err:#}"));
                    let found = store
                        .get_skill_by_id(&id)
                        .unwrap_or_else(|err| panic!("read in worker {worker}: {err:#}"));
                    assert!(found.is_some());
                }
            });
        }
    });
}

/// #104's representative probe: the Refresh acquire pool is four workers
/// (`refresh::ACQUIRE_POOL_SIZE`). Under the 1.3.2 policy (rollback journal)
/// its slowest call took 3-5.4 s against the 5 s busy timeout on GitHub's
/// hosted Windows runner.
#[test]
fn four_refresh_workers_reading_and_writing_all_succeed() {
    read_and_upsert_concurrently(4, 50);
}

/// Twice the pool: the headroom check. Under the rollback journal it failed
/// "database is locked" on the hosted Windows runner on every run (BACKLOG
/// #104); under WAL its slowest call there stayed near 1 s.
#[test]
fn concurrent_readers_and_writers_all_succeed() {
    read_and_upsert_concurrently(8, 50);
}

/// #104's policy, pinned on its own: in WAL mode a reader holding a snapshot
/// never holds up a write (and keeps its snapshot). Under the rollback
/// journal the write's commit waits for the reader's lock until the busy
/// timeout and fails "database is locked" — the reader/writer ping-pong
/// behind #104's starvation.
#[test]
fn a_reader_holding_a_snapshot_does_not_hold_up_a_write() {
    let (_dir, store) = make_store();
    store
        .upsert_skill(&make_skill("held", "held", "/tmp/central/held", 1))
        .expect("seed");

    let reader = rusqlite::Connection::open(&store.db_path).expect("open reader");
    reader.execute_batch("BEGIN;").expect("begin");
    let before: i64 = reader
        .query_row(
            "SELECT updated_at FROM skills WHERE id = 'held'",
            [],
            |row| row.get(0),
        )
        .expect("read inside the snapshot");
    assert_eq!(before, 1);

    store
        .upsert_skill(&make_skill("held", "held", "/tmp/central/held", 2))
        .expect("the write does not wait for the reader");

    let inside: i64 = reader
        .query_row(
            "SELECT updated_at FROM skills WHERE id = 'held'",
            [],
            |row| row.get(0),
        )
        .expect("read again inside the snapshot");
    assert_eq!(inside, 1, "the reader keeps its snapshot");
    reader.execute_batch("COMMIT;").expect("end the snapshot");
    assert_eq!(
        store.get_skill_by_id("held").unwrap().unwrap().updated_at,
        2
    );
}

fn journal_mode(db: &std::path::Path) -> String {
    rusqlite::Connection::open(db)
        .expect("open")
        .query_row("PRAGMA journal_mode;", [], |row| row.get(0))
        .expect("journal_mode")
}

/// #104: `ensure_schema` puts the database in WAL mode — a new one, and an
/// existing one as 1.3.2 and earlier left it (rollback journal), data intact.
/// The mode persists in the file; every store connection uses
/// `synchronous = FULL` (an acknowledged write survives a power loss).
#[test]
fn ensure_schema_puts_new_and_existing_databases_in_wal_mode() {
    let (dir, store) = make_store();
    assert_eq!(journal_mode(&dir.path().join("test.db")), "wal", "new");
    let synchronous: i64 = store
        .with_conn(|conn| Ok(conn.query_row("PRAGMA synchronous;", [], |row| row.get(0))?))
        .unwrap();
    assert_eq!(synchronous, 2, "FULL");

    let existing = dir.path().join("existing.db");
    open_v8_database(&existing);
    rusqlite::Connection::open(&existing)
        .unwrap()
        .execute(
            "INSERT INTO skills (id, name, source_type, central_path, created_at,
               updated_at, last_seen_at, status)
             VALUES ('old', 'old', 'local', '/tmp/central/old', 1, 1, 1, 'ok')",
            [],
        )
        .unwrap();
    assert_eq!(journal_mode(&existing), "delete", "precondition: rollback");

    let store = SkillStore::new(existing.clone());
    store.ensure_schema().expect("upgrade");
    assert_eq!(journal_mode(&existing), "wal", "existing");
    assert_eq!(store.get_skill_by_id("old").unwrap().unwrap().name, "old");
    // Closed cleanly, nothing is left beside the file.
    assert!(!dir.path().join("existing.db-wal").exists());
}

/// A WAL database that crashed: its last commit (`only_in_the_wal`) lives
/// only in `<target>-wal`, never checkpointed — the files copied while the
/// writing connection was still open. No `-shm`; SQLite rebuilds it.
fn crash_snapshot(root: &std::path::Path, target: &std::path::Path) {
    let scratch = root.join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    let live = scratch.join("skills_hub.db");
    SkillStore::new(live.clone()).ensure_schema().unwrap();
    let conn = rusqlite::Connection::open(&live).unwrap();
    conn.execute_batch("PRAGMA wal_autocheckpoint = 0;")
        .unwrap();
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('only_in_the_wal', 'yes')",
        [],
    )
    .unwrap();
    std::fs::copy(&live, target).unwrap();
    std::fs::copy(scratch.join("skills_hub.db-wal"), sidecar(target, "-wal")).unwrap();
    drop(conn);
}

fn sidecar(db: &std::path::Path, suffix: &str) -> std::path::PathBuf {
    let mut name = db.as_os_str().to_owned();
    name.push(suffix);
    std::path::PathBuf::from(name)
}

/// The one `*.bak-<secs>` database migration left in `dir`.
fn only_backup(dir: &std::path::Path) -> std::path::PathBuf {
    let backups: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|ext| {
                ext.to_string_lossy()
                    .strip_prefix("bak-")
                    .is_some_and(|secs| secs.chars().all(|c| c.is_ascii_digit()))
            })
        })
        .collect();
    let [backup] = backups.as_slice() else {
        panic!("one backup: {backups:?}")
    };
    backup.clone()
}

/// #104 x legacy adoption: a target that crashed in WAL mode (its last
/// commits only in `-wal`, never checkpointed) and holds no skills is backed
/// up *with* those commits, and the adopted legacy copy is never read through
/// the crashed target's WAL. `db_has_any_skills`' connection recovers and
/// checkpoints the WAL into the file before it is renamed.
#[test]
fn a_crashed_wal_target_is_backed_up_whole_and_its_wal_never_reaches_the_legacy_copy() {
    let root = tempfile::tempdir().unwrap();
    let data_root = root.path().join("data");
    seeded_db(
        &data_root
            .join("com.qufei1993.skillshub")
            .join("skills_hub.db"),
        "from-legacy",
    );

    let target_dir = root.path().join("current");
    let target = target_dir.join("skills_hub.db");
    crash_snapshot(root.path(), &target);

    super::migrate_legacy_db_if_needed(&data_root, &target).unwrap();

    assert_eq!(skill_names(&target), vec!["from-legacy".to_string()]);
    let adopted = SkillStore::new(target.clone());
    assert_eq!(adopted.get_setting("only_in_the_wal").unwrap(), None);
    let integrity: String = rusqlite::Connection::open(&target)
        .unwrap()
        .query_row("PRAGMA integrity_check;", [], |row| row.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");

    let backup = only_backup(&target_dir);
    assert_eq!(
        SkillStore::new(backup)
            .get_setting("only_in_the_wal")
            .unwrap()
            .as_deref(),
        Some("yes"),
        "the backup kept the crashed target's last commit"
    );
}

/// A target migration cannot read (here: its file is damaged, so
/// `db_has_any_skills` fails and nothing checkpointed its WAL) is backed up
/// *with* its sidecars: a `-wal` or hot `-journal` left beside the adopted
/// legacy copy could be replayed onto it (SQLite does not tie either to its
/// database file).
#[test]
fn an_unreadable_target_is_backed_up_with_its_sidecars() {
    let root = tempfile::tempdir().unwrap();
    let data_root = root.path().join("data");
    seeded_db(
        &data_root.join("com.tauri.dev").join("skills_hub.db"),
        "from-legacy",
    );
    let target_dir = root.path().join("current");
    let target = target_dir.join("skills_hub.db");
    crash_snapshot(root.path(), &target);
    std::fs::write(&target, b"not a database, not even a header").unwrap();
    std::fs::write(sidecar(&target, "-journal"), b"a stale journal").unwrap();
    assert!(super::db_has_any_skills(&target).is_err(), "precondition");

    super::migrate_legacy_db_if_needed(&data_root, &target).unwrap();

    // Before anything opens the adopted copy: no old sidecar beside it.
    for suffix in ["-wal", "-shm", "-journal"] {
        assert!(
            !sidecar(&target, suffix).exists(),
            "the old target's {suffix} is beside the adopted copy"
        );
    }
    assert_eq!(skill_names(&target), vec!["from-legacy".to_string()]);
    let integrity: String = rusqlite::Connection::open(&target)
        .unwrap()
        .query_row("PRAGMA integrity_check;", [], |row| row.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
    // The damaged file's WAL is kept with it (SQLite itself discarded the
    // invalid journal when the failed read probed it).
    let backup = only_backup(&target_dir);
    assert!(
        sidecar(&backup, "-wal").exists(),
        "the WAL travelled with the backup"
    );
}

/// 1.3.3 L2 review: a backup that fails part-way leaves the original file set
/// whole — its WAL (holding a committed write) still beside its database —
/// and no partial backup behind. Here the `-journal` "sidecar" is a directory,
/// so copying it fails after the database and its `-wal` were copied.
#[test]
fn a_failed_backup_never_splits_a_database_from_its_wal() {
    let root = tempfile::tempdir().unwrap();
    let data_root = root.path().join("data");
    seeded_db(
        &data_root.join("com.tauri.dev").join("skills_hub.db"),
        "from-legacy",
    );
    let target_dir = root.path().join("current");
    let target = target_dir.join("skills_hub.db");
    crash_snapshot(root.path(), &target);
    // Unreadable to the pre-migration probe, so nothing checkpoints the WAL.
    let header = std::fs::read(&target).unwrap();
    std::fs::write(&target, b"not a database, not even a header").unwrap();
    std::fs::create_dir(sidecar(&target, "-journal")).unwrap();

    assert!(super::migrate_legacy_db_if_needed(&data_root, &target).is_err());

    assert!(
        target.exists() && sidecar(&target, "-wal").exists(),
        "the original set stays whole"
    );
    let leftovers: Vec<_> = std::fs::read_dir(&target_dir)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains(".bak-"))
        .collect();
    assert!(leftovers.is_empty(), "no partial backup: {leftovers:?}");

    // Restore the file and clear the blocker: the committed write is still
    // reachable through the original WAL.
    std::fs::write(&target, header).unwrap();
    std::fs::remove_dir(sidecar(&target, "-journal")).unwrap();
    assert_eq!(
        SkillStore::new(target.clone())
            .get_setting("only_in_the_wal")
            .unwrap()
            .as_deref(),
        Some("yes")
    );
}

/// A database exactly as 1.3.4 left it (schema version 11).
fn open_v11_database(db: &std::path::Path) {
    open_v8_database(db);
    let conn = rusqlite::Connection::open(db).expect("open");
    conn.execute_batch("ALTER TABLE skills ADD COLUMN imported_from_tool TEXT NULL;")
        .expect("v9");
    conn.execute_batch("ALTER TABLE skills ADD COLUMN disabled_at INTEGER NULL;")
        .expect("v10");
    conn.execute_batch(super::MIGRATION_V11).expect("v11");
    conn.pragma_update(None, "user_version", 11).expect("v11");
}

/// V12 (ADR-0007): global rows gain the deployed identity column, recorded
/// as unknown for every existing row (a pre-V12 copy is not provably ours);
/// every already-disabled row — global or project — gives its path up, so
/// none of them can let Delete remove a later occupant. A live owner keeps
/// its claim (and an assignment its recorded identity). The step is
/// idempotent and completes a partially applied upgrade.
#[test]
fn v12_migration_records_identity_and_gives_up_disabled_claims() {
    for column_already_there in [false, true] {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = dir.path().join("test.db");
        open_v11_database(&db);
        {
            let conn = rusqlite::Connection::open(&db).expect("open");
            if column_already_there {
                conn.execute_batch(super::MIGRATION_V12_COLUMN)
                    .expect("partial v12");
            }
            conn.execute_batch(
                "INSERT INTO skills (id, name, source_type, central_path,
                                     created_at, updated_at, last_seen_at, status)
                 VALUES ('s1', 'alpha', 'local', '/central/alpha', 1, 1, 1, 'ok');
                 INSERT INTO skill_targets (id, skill_id, tool, target_path, mode, status,
                                            last_error, synced_at)
                 VALUES ('t-live', 's1', 'claude_code', '/h/.claude/skills/alpha', 'copy',
                         'synced', NULL, 5),
                        ('t-off', 's1', 'codex', '/h/.codex/skills/alpha', 'symlink',
                         'disabled', NULL, 5);
                 INSERT INTO projects (id, path, created_at, updated_at)
                 VALUES ('p1', '/proj', 1, 1);
                 INSERT INTO project_skill_assignments (id, project_id, skill_id, skill_name,
                     tool, mode, status, last_error, synced_at, content_hash, created_at)
                 VALUES ('a-live', 'p1', 's1', 'alpha', 'claude_code', 'copy', 'synced',
                         NULL, 5, 'h-live', 1),
                        ('a-off', 'p1', 's1', 'alpha', 'cursor', 'copy', 'disabled',
                         NULL, 5, 'h-off', 1);",
            )
            .expect("seed v11 rows");
        }

        let store = SkillStore::new(db.clone());
        store.ensure_schema().expect("upgrade");
        store.ensure_schema().expect("idempotent");

        let version: i32 = rusqlite::Connection::open(&db)
            .expect("open")
            .query_row("PRAGMA user_version;", [], |row| row.get(0))
            .expect("user_version");
        assert_eq!(version, super::SCHEMA_VERSION);
        let live = store
            .get_skill_target("s1", "claude_code")
            .unwrap()
            .unwrap();
        assert_eq!(live.synced_at, Some(5), "a live owner keeps its claim");
        assert_eq!(live.content_hash, None, "no identity was ever recorded");
        let off = store.get_skill_target("s1", "codex").unwrap().unwrap();
        assert_eq!(off.status, SyncStatus::Disabled);
        assert!(!off.owns_artifact(), "a disabled row gives its path up");
        let live = store
            .get_project_skill_assignment("p1", "s1", "claude_code")
            .unwrap()
            .unwrap();
        assert_eq!(live.synced_at, Some(5));
        assert_eq!(live.content_hash.as_deref(), Some("h-live"));
        let off = store
            .get_project_skill_assignment("p1", "s1", "cursor")
            .unwrap()
            .unwrap();
        assert!(!off.owns_artifact());
        assert_eq!(off.content_hash, None);
    }
}

/// The claim columns through every transition (ADR-0007): a successful
/// write records the claim and the deployed identity; a failure keeps both
/// (ADR-0002); Disable — and a Keep answer over an existing row — gives both
/// up; a reconcile touches neither.
#[test]
fn transitions_record_keep_and_give_up_the_claim() {
    let (_dir, store) = make_store();
    store
        .upsert_skill(&make_skill("s1", "alpha", "/central/alpha", 1))
        .unwrap();
    let target = |store: &SkillStore| store.get_skill_target("s1", "cursor").unwrap().unwrap();
    store
        .transition_skill_target(
            "t1",
            TargetTransition::Recorded {
                skill_id: "s1",
                tool: "cursor",
                mode: SyncMode::Copy,
                target_path: "/h/.cursor/skills/alpha",
                synced_at: 5,
                content_hash: Some("h1"),
            },
        )
        .unwrap();
    assert_eq!(
        (
            target(&store).synced_at,
            target(&store).content_hash.as_deref()
        ),
        (Some(5), Some("h1"))
    );
    store
        .transition_skill_target("t1", TargetTransition::SyncFailed { error: "boom" })
        .unwrap();
    assert_eq!(
        (
            target(&store).synced_at,
            target(&store).content_hash.as_deref()
        ),
        (Some(5), Some("h1")),
        "a failure keeps the claim and the evidence"
    );
    store
        .transition_skill_target(
            "t1",
            TargetTransition::SyncCompleted {
                mode: SyncMode::Copy,
                target_path: "/h/.cursor/skills/alpha",
                synced_at: 6,
                content_hash: Some("h2"),
            },
        )
        .unwrap();
    assert_eq!(target(&store).content_hash.as_deref(), Some("h2"));
    store
        .transition_skill_target("t1", TargetTransition::Disabled)
        .unwrap();
    assert_eq!(
        (target(&store).synced_at, target(&store).content_hash),
        (None, None),
        "Disable gives the path up"
    );
    store
        .transition_skill_target(
            "t1",
            TargetTransition::SyncCompleted {
                mode: SyncMode::Copy,
                target_path: "/h/.cursor/skills/alpha",
                synced_at: 7,
                content_hash: Some("h3"),
            },
        )
        .unwrap();
    store
        .transition_skill_target(
            "ignored",
            TargetTransition::DisabledRecorded {
                skill_id: "s1",
                tool: "cursor",
                mode: SyncMode::Copy,
                target_path: "/h/.cursor/skills/alpha",
            },
        )
        .unwrap();
    assert_eq!(target(&store).status, SyncStatus::Disabled);
    assert_eq!(
        (target(&store).synced_at, target(&store).content_hash),
        (None, None),
        "a Keep answer over an existing row gives its path up"
    );

    store
        .register_project(&ProjectRecord {
            id: "p1".into(),
            path: "/proj".into(),
            created_at: 1,
            updated_at: 1,
        })
        .unwrap();
    store
        .add_project_skill_assignment(&ProjectSkillAssignmentRecord {
            id: "a1".into(),
            project_id: "p1".into(),
            skill_id: "s1".into(),
            skill_name: "alpha".into(),
            tool: "cursor".into(),
            mode: SyncMode::Copy,
            status: SyncStatus::Synced,
            last_error: None,
            synced_at: Some(5),
            content_hash: Some("h1".into()),
            created_at: 1,
        })
        .unwrap();
    let assignment = |store: &SkillStore| {
        store
            .get_project_skill_assignment("p1", "s1", "cursor")
            .unwrap()
            .unwrap()
    };
    store
        .transition_assignment("a1", AssignmentTransition::SyncFailed { error: "boom" })
        .unwrap();
    assert_eq!(
        (
            assignment(&store).synced_at,
            assignment(&store).content_hash.as_deref()
        ),
        (Some(5), Some("h1"))
    );
    store
        .transition_assignment(
            "a1",
            AssignmentTransition::Reconciled {
                status: SyncStatus::Stale,
            },
        )
        .unwrap();
    assert_eq!(
        assignment(&store).content_hash.as_deref(),
        Some("h1"),
        "a stale copy is still the copy the row deployed"
    );
    store
        .transition_assignment("a1", AssignmentTransition::Disabled)
        .unwrap();
    assert_eq!(
        (
            assignment(&store).synced_at,
            assignment(&store).content_hash
        ),
        (None, None)
    );
}
