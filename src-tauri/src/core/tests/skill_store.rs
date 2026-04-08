use std::path::PathBuf;

use crate::core::skill_store::{
    ProjectRecord, ProjectSkillAssignmentRecord, ProjectToolRecord, SkillRecord, SkillStore,
    SkillTargetRecord,
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
        tool: "cursor".to_string(),
        mode: "symlink".to_string(),
        status: "pending".to_string(),
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
        tool: "cursor".to_string(),
        mode: "symlink".to_string(),
        status: "synced".to_string(),
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
        tool: "cursor".to_string(),
        mode: "symlink".to_string(),
        status: "pending".to_string(),
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
    assert_eq!(assignments[0].mode, "symlink");
    assert_eq!(assignments[0].status, "pending");

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
        tool: "cursor".to_string(),
        mode: "symlink".to_string(),
        status: "pending".to_string(),
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
        tool: "cursor".to_string(),
        mode: "copy".to_string(),
        status: "pending".to_string(),
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
        tool: "cursor".to_string(),
        mode: "symlink".to_string(),
        status: "synced".to_string(),
        last_error: None,
        synced_at: Some(100),
        content_hash: None,
        created_at: 100,
    };
    let a2 = ProjectSkillAssignmentRecord {
        id: "a2".to_string(),
        project_id: "p2".to_string(),
        skill_id: "s2".to_string(),
        tool: "cursor".to_string(),
        mode: "symlink".to_string(),
        status: "synced".to_string(),
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
        tool: "cursor".to_string(),
        mode: "symlink".to_string(),
        status: "synced".to_string(),
        last_error: None,
        synced_at: Some(100),
        content_hash: None,
        created_at: 100,
    };
    let a2 = ProjectSkillAssignmentRecord {
        id: "a2".to_string(),
        project_id: "p1".to_string(),
        skill_id: "s2".to_string(),
        tool: "cursor".to_string(),
        mode: "symlink".to_string(),
        status: "synced".to_string(),
        last_error: None,
        synced_at: Some(200),
        content_hash: None,
        created_at: 200,
    };
    store.add_project_skill_assignment(&a1).unwrap();
    store.add_project_skill_assignment(&a2).unwrap();

    assert_eq!(store.aggregate_project_sync_status("p1").unwrap(), "synced");
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
        tool: "cursor".to_string(),
        mode: "symlink".to_string(),
        status: "synced".to_string(),
        last_error: None,
        synced_at: Some(100),
        content_hash: None,
        created_at: 100,
    };
    let a2 = ProjectSkillAssignmentRecord {
        id: "a2".to_string(),
        project_id: "p1".to_string(),
        skill_id: "s2".to_string(),
        tool: "cursor".to_string(),
        mode: "symlink".to_string(),
        status: "error".to_string(),
        last_error: Some("symlink failed".to_string()),
        synced_at: None,
        content_hash: None,
        created_at: 200,
    };
    store.add_project_skill_assignment(&a1).unwrap();
    store.add_project_skill_assignment(&a2).unwrap();

    assert_eq!(store.aggregate_project_sync_status("p1").unwrap(), "error");
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

    assert_eq!(store.aggregate_project_sync_status("p1").unwrap(), "none");
}

#[test]
fn count_project_assignments_and_tools() {
    let (_dir, store) = make_store();

    let p = ProjectRecord {
        id: "p1".to_string(),
        path: "/home/user/proj".to_string(),
        created_at: 100,
        updated_at: 100,
    };
    store.register_project(&p).unwrap();

    assert_eq!(store.count_project_assignments("p1").unwrap(), 0);
    assert_eq!(store.count_project_tools("p1").unwrap(), 0);

    let tool = ProjectToolRecord {
        id: "pt1".to_string(),
        project_id: "p1".to_string(),
        tool: "cursor".to_string(),
    };
    store.add_project_tool(&tool).unwrap();
    assert_eq!(store.count_project_tools("p1").unwrap(), 1);

    let skill = make_skill("s1", "S1", "/central/s1", 1);
    store.upsert_skill(&skill).unwrap();

    let assign = ProjectSkillAssignmentRecord {
        id: "a1".to_string(),
        project_id: "p1".to_string(),
        skill_id: "s1".to_string(),
        tool: "cursor".to_string(),
        mode: "symlink".to_string(),
        status: "pending".to_string(),
        last_error: None,
        synced_at: None,
        content_hash: None,
        created_at: 100,
    };
    store.add_project_skill_assignment(&assign).unwrap();
    assert_eq!(store.count_project_assignments("p1").unwrap(), 1);
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
        tool: "cursor".to_string(),
        mode: "symlink".to_string(),
        status: "synced".to_string(),
        last_error: None,
        synced_at: Some(100),
        content_hash: None,
        created_at: 100,
    };
    let a2 = ProjectSkillAssignmentRecord {
        id: "a2".to_string(),
        project_id: "p1".to_string(),
        skill_id: "s2".to_string(),
        tool: "claude_code".to_string(),
        mode: "symlink".to_string(),
        status: "synced".to_string(),
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
        tool: "cursor".to_string(),
        mode: "symlink".to_string(),
        status: "pending".to_string(),
        last_error: None,
        synced_at: None,
        content_hash: None,
        created_at: 100,
    };
    store.add_project_skill_assignment(&assign).unwrap();

    // Update with content_hash
    store
        .update_assignment_status("a1", "synced", None, Some(1000), Some("copy"), Some("abc123"))
        .unwrap();

    // Read back via get_project_skill_assignment
    let got = store
        .get_project_skill_assignment("p1", "s1", "cursor")
        .unwrap()
        .unwrap();
    assert_eq!(got.content_hash.as_deref(), Some("abc123"));
    assert_eq!(got.status, "synced");
    assert_eq!(got.synced_at, Some(1000));
    assert_eq!(got.mode, "copy");
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
        tool: "cursor".to_string(),
        mode: "symlink".to_string(),
        status: "pending".to_string(),
        last_error: None,
        synced_at: None,
        content_hash: None,
        created_at: 100,
    };
    store.add_project_skill_assignment(&assign).unwrap();

    // First update: set synced with synced_at=1000 and mode="symlink"
    store
        .update_assignment_status("a1", "synced", None, Some(1000), Some("symlink"), None)
        .unwrap();

    let got = store
        .get_project_skill_assignment("p1", "s1", "cursor")
        .unwrap()
        .unwrap();
    assert_eq!(got.status, "synced");
    assert_eq!(got.synced_at, Some(1000));
    assert_eq!(got.mode, "symlink");

    // Second update: set error with last_error but synced_at=None (should preserve synced_at=1000)
    store
        .update_assignment_status("a1", "error", Some("fail"), None, None, None)
        .unwrap();

    let got = store
        .get_project_skill_assignment("p1", "s1", "cursor")
        .unwrap()
        .unwrap();
    assert_eq!(got.status, "error");
    assert_eq!(got.last_error.as_deref(), Some("fail"));
    assert_eq!(got.synced_at, Some(1000), "COALESCE should preserve synced_at");
    assert_eq!(got.mode, "symlink", "COALESCE should preserve mode");
}

#[test]
fn get_project_skill_assignment_returns_none() {
    let (_dir, store) = make_store();

    let result = store
        .get_project_skill_assignment("nonexistent", "nonexistent", "nonexistent")
        .unwrap();
    assert!(result.is_none());
}
