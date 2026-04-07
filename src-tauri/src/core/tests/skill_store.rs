use std::path::PathBuf;

use crate::core::skill_store::{
    ProjectRecord, SkillRecord, SkillStore, SkillTargetRecord,
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
    }
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

    store.set_onboarding_completed(true).unwrap();
    assert_eq!(
        store
            .get_setting("onboarding_completed")
            .unwrap()
            .as_deref(),
        Some("true")
    );
    store.set_onboarding_completed(false).unwrap();
    assert_eq!(
        store
            .get_setting("onboarding_completed")
            .unwrap()
            .as_deref(),
        Some("false")
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
        mode: "copy".to_string(),
        status: "ok".to_string(),
        last_error: None,
        synced_at: None,
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
        "unique(skill_id, tool) 冲突时应更新现有行而不是替换 id"
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
        mode: "copy".to_string(),
        status: "ok".to_string(),
        last_error: None,
        synced_at: None,
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
fn deleting_skill_cascades_targets() {
    let (_dir, store) = make_store();
    let skill = make_skill("s1", "S1", "/central/s1", 1);
    store.upsert_skill(&skill).unwrap();

    let t = SkillTargetRecord {
        id: "t1".to_string(),
        skill_id: "s1".to_string(),
        tool: "cursor".to_string(),
        target_path: "/target/1".to_string(),
        mode: "copy".to_string(),
        status: "ok".to_string(),
        last_error: None,
        synced_at: None,
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
        mode: "copy".to_string(),
        status: "ok".to_string(),
        last_error: None,
        synced_at: None,
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
    assert!(result.is_err(), "duplicate path should fail with UNIQUE constraint");
}
