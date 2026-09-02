//! Tests for `core::skill_catalog` — the Managed-skill catalog: skills with
//! their Sync targets and manifest-derived invocation mode, and the
//! deliberate failure policy (a target query that fails fails the call).

use std::fs;
use std::path::{Path, PathBuf};

use crate::core::skill_catalog::managed_skill_catalog;
use crate::core::skill_discovery::InvocationMode;
use crate::core::skill_store::{SkillRecord, SkillStore, SkillTargetRecord};
use crate::core::sync_status::{SyncMode, SyncStatus};

fn make_store(base: &Path) -> SkillStore {
    let store = SkillStore::new(base.join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    store
}

fn seed_skill(store: &SkillStore, id: &str, name: &str, central_path: &Path) -> SkillRecord {
    let skill = SkillRecord {
        id: id.to_string(),
        name: name.to_string(),
        description: None,
        source_type: "local".to_string(),
        source_ref: None,
        source_subpath: None,
        source_revision: None,
        central_path: central_path.to_string_lossy().to_string(),
        content_hash: None,
        created_at: 1,
        updated_at: 2,
        last_sync_at: None,
        last_seen_at: 1,
        status: "ok".to_string(),
    };
    store.upsert_skill(&skill).expect("upsert skill");
    skill
}

fn seed_target(store: &SkillStore, skill_id: &str, tool: &str) {
    store
        .upsert_skill_target(&SkillTargetRecord {
            id: format!("{skill_id}-{tool}"),
            skill_id: skill_id.to_string(),
            tool: tool.to_string(),
            target_path: format!("/tmp/{tool}/{skill_id}"),
            mode: SyncMode::Copy,
            status: SyncStatus::Synced,
            last_error: None,
            synced_at: Some(9),
        })
        .expect("upsert target");
}

fn write_manifest(base: &Path, name: &str, body: &str) -> PathBuf {
    let dir = base.join(name);
    fs::create_dir_all(&dir).expect("create central dir");
    fs::write(dir.join("SKILL.md"), body).expect("write SKILL.md");
    dir
}

#[test]
fn catalog_carries_every_skill_with_its_own_targets() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = tmp.path().join("central");
    seed_skill(
        &store,
        "s1",
        "alpha",
        &write_manifest(&central, "alpha", "---\nname: alpha\n---\n"),
    );
    seed_skill(
        &store,
        "s2",
        "beta",
        &write_manifest(&central, "beta", "---\nname: beta\n---\n"),
    );
    seed_target(&store, "s1", "claude_code");
    seed_target(&store, "s1", "cursor");
    seed_target(&store, "s2", "pi");

    let catalog = managed_skill_catalog(&store).expect("catalog");

    assert_eq!(catalog.len(), 2);
    let alpha = catalog
        .iter()
        .find(|e| e.skill.id == "s1")
        .expect("alpha entry");
    let mut tools: Vec<&str> = alpha.targets.iter().map(|t| t.tool.as_str()).collect();
    tools.sort();
    assert_eq!(tools, vec!["claude_code", "cursor"]);
    let beta = catalog
        .iter()
        .find(|e| e.skill.id == "s2")
        .expect("beta entry");
    assert_eq!(
        beta.targets
            .iter()
            .map(|t| t.tool.as_str())
            .collect::<Vec<_>>(),
        vec!["pi"]
    );
    assert_eq!(beta.targets[0].status, SyncStatus::Synced);
}

#[test]
fn invocation_mode_comes_from_the_central_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = tmp.path().join("central");
    seed_skill(
        &store,
        "s1",
        "restricted",
        &write_manifest(
            &central,
            "restricted",
            "---\nname: restricted\ndisable-model-invocation: true\n---\n",
        ),
    );

    let catalog = managed_skill_catalog(&store).expect("catalog");
    assert_eq!(catalog[0].invocation_mode, InvocationMode::UserOnly);
}

#[test]
fn a_missing_central_manifest_yields_the_default_invocation_mode() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    // Central path points nowhere: an ordinary state, not a failure.
    seed_skill(&store, "s1", "gone", &tmp.path().join("central/gone"));

    let catalog = managed_skill_catalog(&store).expect("catalog");
    assert_eq!(catalog.len(), 1);
    assert_eq!(catalog[0].invocation_mode, InvocationMode::default());
    assert!(catalog[0].targets.is_empty());
}

#[test]
fn a_failing_target_query_fails_the_catalog_instead_of_hiding_targets() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = tmp.path().join("central");
    seed_skill(
        &store,
        "s1",
        "alpha",
        &write_manifest(&central, "alpha", "---\nname: alpha\n---\n"),
    );
    seed_target(&store, "s1", "claude_code");
    // Break only the target query: `skills` still reads, `skill_targets` cannot.
    {
        let conn = rusqlite::Connection::open(store.db_path()).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = OFF; DROP TABLE skill_targets;")
            .unwrap();
    }

    let err = managed_skill_catalog(&store).expect_err("the catalog must fail loudly");
    let chain = format!("{err:#}");
    assert!(
        chain.contains("list sync targets for skill s1"),
        "the failure names the skill it could not describe: {chain}"
    );
}
