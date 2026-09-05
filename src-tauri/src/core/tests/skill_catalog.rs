//! Tests for `core::skill_catalog` — the Managed-skill catalog: skills with
//! their Sync targets and manifest-derived invocation mode, and the
//! deliberate failure policy (a target query that fails fails the call).

use std::fs;
use std::path::{Path, PathBuf};

use crate::core::skill_catalog::managed_skill_catalog;
use crate::core::skill_discovery::InvocationMode;
use crate::core::skill_store::{SkillRecord, SkillStore, SkillTargetRecord};
use crate::core::sync_status::{SyncMode, SyncStatus};
use crate::core::unlocatable::UnlocatableState;

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
        imported_from_tool: None,
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

/// The listing answers "can this skill be refreshed?" itself, from the one
/// Provenance predicate, so the UI never re-derives it from `source_type`.
#[test]
fn catalog_marks_an_imported_skill_not_refreshable() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = tmp.path().join("central");
    seed_skill(
        &store,
        "s1",
        "alpha",
        &write_manifest(&central, "alpha", "---\nname: alpha\n---\n"),
    );
    let mut imported = seed_skill(
        &store,
        "s2",
        "beta",
        &write_manifest(&central, "beta", "---\nname: beta\n---\n"),
    );
    imported.source_type = "imported".to_string();
    imported.imported_from_tool = Some("claude_code".to_string());
    store.upsert_skill(&imported).expect("upsert imported");

    let mut catalog = managed_skill_catalog(&store).expect("catalog");
    catalog.sort_by(|a, b| a.skill.id.cmp(&b.skill.id));

    assert!(catalog[0].refreshable, "a local skill is refreshable");
    assert!(!catalog[1].refreshable, "an imported skill is not");
    assert_eq!(
        catalog[1].skill.imported_from_tool.as_deref(),
        Some("claude_code")
    );
}

/// The listing answers "can the app still locate this skill?" from the
/// recorded paths alone (see **Unlocatable skill** in `CONTEXT.md`): a
/// `local` skill whose folder is gone is `source_missing`, a skill whose
/// central copy is gone is `central_missing`, and a healthy row — or an
/// imported row with a healthy central copy — is neither.
#[test]
fn catalog_marks_unlocatable_skills_by_their_recorded_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = tmp.path().join("central");
    let own_folder = tmp.path().join("own-folder");
    fs::create_dir_all(&own_folder).unwrap();
    fs::write(own_folder.join("SKILL.md"), "---\nname: healthy\n---\n").unwrap();

    let mut healthy = seed_skill(
        &store,
        "s1",
        "healthy",
        &write_manifest(&central, "healthy", "---\nname: healthy\n---\n"),
    );
    healthy.source_ref = Some(own_folder.to_string_lossy().to_string());
    store.upsert_skill(&healthy).unwrap();

    let mut source_gone = seed_skill(
        &store,
        "s2",
        "source-gone",
        &write_manifest(&central, "source-gone", "---\nname: source-gone\n---\n"),
    );
    source_gone.source_ref = Some(tmp.path().join("moved-away").to_string_lossy().to_string());
    store.upsert_skill(&source_gone).unwrap();

    let mut central_gone = seed_skill(&store, "s3", "central-gone", &central.join("central-gone"));
    central_gone.source_type = "git".to_string();
    central_gone.source_ref = Some("https://github.com/o/r".to_string());
    store.upsert_skill(&central_gone).unwrap();

    let mut imported = seed_skill(
        &store,
        "s4",
        "taken-over",
        &write_manifest(&central, "taken-over", "---\nname: taken-over\n---\n"),
    );
    imported.source_type = "imported".to_string();
    imported.imported_from_tool = Some("claude_code".to_string());
    store.upsert_skill(&imported).unwrap();

    let mut catalog = managed_skill_catalog(&store).expect("catalog");
    catalog.sort_by(|a, b| a.skill.id.cmp(&b.skill.id));

    assert_eq!(catalog[0].unlocatable, None, "a healthy local skill");
    assert_eq!(
        catalog[1].unlocatable,
        Some(UnlocatableState::SourceMissing)
    );
    assert_eq!(
        catalog[2].unlocatable,
        Some(UnlocatableState::CentralMissing)
    );
    assert_eq!(
        catalog[3].unlocatable, None,
        "an imported skill has no source to be missing"
    );
    // Restore is Update, so a git skill whose central copy is gone still
    // answers "refreshable": it has a source to re-acquire from.
    assert!(catalog[2].refreshable);
}
