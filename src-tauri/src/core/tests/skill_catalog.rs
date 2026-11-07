//! Tests for `core::skill_catalog` — the Managed-skill catalog: skills with
//! their Sync targets and manifest-derived invocation mode, and the
//! deliberate failure policy (a target query that fails fails the call).

use std::fs;
use std::path::{Path, PathBuf};

use crate::core::skill_catalog::{
    managed_skill_catalog, managed_skill_entry, SkillProject, SkillProjectDeployment,
};
use crate::core::skill_discovery::InvocationMode;
use crate::core::skill_store::{
    ProjectRecord, ProjectSkillAssignmentRecord, SkillRecord, SkillStore, SkillTargetRecord,
};
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
            content_hash: None,
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

/// Detach is offered exactly when it would be accepted: a `local` skill
/// with a central copy to become its truth. With source and central copy
/// both gone the state is still `source_missing`, but Detach is off.
#[test]
fn catalog_offers_detach_only_when_the_central_copy_is_present() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = tmp.path().join("central");

    let mut source_gone = seed_skill(
        &store,
        "s1",
        "source-gone",
        &write_manifest(&central, "source-gone", "---\nname: source-gone\n---\n"),
    );
    source_gone.source_ref = Some(tmp.path().join("moved-away").to_string_lossy().to_string());
    store.upsert_skill(&source_gone).unwrap();

    let mut both_gone = seed_skill(&store, "s2", "both-gone", &central.join("both-gone"));
    both_gone.source_ref = Some(tmp.path().join("moved-away").to_string_lossy().to_string());
    store.upsert_skill(&both_gone).unwrap();

    let mut git = seed_skill(
        &store,
        "s3",
        "git",
        &write_manifest(&central, "git", "---\nname: git\n---\n"),
    );
    git.source_type = "git".to_string();
    git.source_ref = Some("https://github.com/o/r".to_string());
    store.upsert_skill(&git).unwrap();

    let mut catalog = managed_skill_catalog(&store).expect("catalog");
    catalog.sort_by(|a, b| a.skill.id.cmp(&b.skill.id));

    assert_eq!(
        catalog[0].unlocatable,
        Some(UnlocatableState::SourceMissing)
    );
    assert!(catalog[0].detachable, "source gone, central present");
    assert_eq!(
        catalog[1].unlocatable,
        Some(UnlocatableState::SourceMissing)
    );
    assert!(!catalog[1].detachable, "both gone: only Re-point or Remove");
    assert!(
        !catalog[2].detachable,
        "a git skill has nothing to detach from"
    );
}

fn seed_project(store: &SkillStore, id: &str, path: &str) {
    store
        .register_project(&ProjectRecord {
            id: id.to_string(),
            path: path.to_string(),
            created_at: 1,
            updated_at: 1,
        })
        .expect("register project");
}

fn seed_assignment(
    store: &SkillStore,
    project_id: &str,
    skill_id: &str,
    tool: &str,
    status: SyncStatus,
    synced_at: Option<i64>,
) {
    store
        .add_project_skill_assignment(&ProjectSkillAssignmentRecord {
            id: format!("{project_id}-{skill_id}-{tool}"),
            project_id: project_id.to_string(),
            skill_id: skill_id.to_string(),
            skill_name: skill_id.to_string(),
            tool: tool.to_string(),
            mode: SyncMode::Symlink,
            status,
            last_error: None,
            synced_at,
            content_hash: None,
            created_at: 1,
        })
        .expect("add assignment");
}

fn deployment(tool: &str, status: SyncStatus, synced_at: Option<i64>) -> SkillProjectDeployment {
    // A folder of one row: its facts are that row's.
    SkillProjectDeployment {
        tool: tool.to_string(),
        status,
        synced_at,
        disabled: status == SyncStatus::Disabled,
        deployed: status != SyncStatus::Disabled && synced_at.is_some(),
    }
}

/// D41: the listing carries each skill's projects — named by folder, ordered
/// by name, each with the skill's own assignment rows (status and ownership) —
/// read for the whole catalog at once; a skill in no project carries none.
#[test]
fn catalog_carries_each_skills_projects_with_their_deployments() {
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
    seed_skill(
        &store,
        "s3",
        "gamma",
        &write_manifest(&central, "gamma", "---\nname: gamma\n---\n"),
    );
    seed_project(&store, "p-mono", "/work/monorepo");
    seed_project(&store, "p-api", "/work/api");
    seed_assignment(
        &store,
        "p-mono",
        "s1",
        "claude_code",
        SyncStatus::Synced,
        Some(5),
    );
    seed_assignment(
        &store,
        "p-mono",
        "s1",
        "agents_skills",
        SyncStatus::Error,
        None,
    );
    seed_assignment(&store, "p-api", "s1", "pi", SyncStatus::Missing, Some(4));
    seed_assignment(
        &store,
        "p-api",
        "s2",
        "claude_code",
        SyncStatus::Disabled,
        Some(3),
    );

    let mut catalog = managed_skill_catalog(&store).expect("catalog");
    catalog.sort_by(|a, b| a.skill.id.cmp(&b.skill.id));

    assert_eq!(
        catalog[0].projects,
        vec![
            SkillProject {
                project_id: "p-api".to_string(),
                project_name: "api".to_string(),
                deployments: vec![deployment("pi", SyncStatus::Missing, Some(4))],
            },
            SkillProject {
                project_id: "p-mono".to_string(),
                project_name: "monorepo".to_string(),
                deployments: vec![
                    deployment("agents_skills", SyncStatus::Error, None),
                    deployment("claude_code", SyncStatus::Synced, Some(5)),
                ],
            },
        ]
    );
    assert_eq!(
        catalog[1].projects,
        vec![SkillProject {
            project_id: "p-api".to_string(),
            project_name: "api".to_string(),
            deployments: vec![deployment("claude_code", SyncStatus::Disabled, Some(3))],
        }]
    );
    assert!(catalog[2].projects.is_empty());

    // The single-skill entry (Save's catalog read) answers the same projects.
    let entry = managed_skill_entry(&store, "s1")
        .expect("entry")
        .expect("s1");
    assert_eq!(entry.projects, catalog[0].projects);
}

/// The same failure policy as targets: a deployment query that fails fails
/// the listing, never a catalog that says the skill is in no project.
#[test]
fn a_failing_project_query_fails_the_catalog() {
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = tmp.path().join("central");
    seed_skill(
        &store,
        "s1",
        "alpha",
        &write_manifest(&central, "alpha", "---\nname: alpha\n---\n"),
    );
    {
        let conn = rusqlite::Connection::open(store.db_path()).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = OFF; DROP TABLE project_skill_assignments;")
            .unwrap();
    }

    let err = managed_skill_catalog(&store).expect_err("the catalog must fail loudly");
    assert!(format!("{err:#}").contains("list project deployments"));
}

/// Rows written once per shared-dir sibling before rows were per folder (Amp
/// and Kimi Code's `~/.config/agents/skills`) read as one Deployment, keyed by
/// the folder; a lone sibling row comes out keyed by its folder too.
#[test]
fn the_catalog_reads_sibling_rows_as_one_folder() {
    crate::core::tool_adapters::test_overrides::kimi_on_amps_folder();
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = tmp.path().join("central");
    let alpha = write_manifest(&central, "alpha", "---\nname: alpha\n---\n");
    let beta = write_manifest(&central, "beta", "---\nname: beta\n---\n");
    seed_skill(&store, "s1", "alpha", &alpha);
    seed_skill(&store, "s2", "beta", &beta);
    seed_target(&store, "s1", "amp");
    seed_target(&store, "s1", "kimi_cli");
    seed_target(&store, "s1", "claude_code");
    seed_target(&store, "s2", "kimi_cli");

    let catalog = managed_skill_catalog(&store).unwrap();
    let tools = |id: &str| -> Vec<String> {
        let entry = catalog.iter().find(|e| e.skill.id == id).unwrap();
        let mut tools: Vec<String> = entry.targets.iter().map(|t| t.tool.clone()).collect();
        tools.sort();
        tools
    };
    assert_eq!(tools("s1"), vec!["amp", "claude_code"]);
    assert_eq!(tools("s2"), vec!["amp"]);
    let one = managed_skill_entry(&store, "s1").unwrap().unwrap();
    assert_eq!(one.targets.len(), 2);
}

/// A folder's state on the catalog is the one fold over all its rows
/// (`placement::fold_status`): an error on any row of the folder shows,
/// whichever row is keyed by the folder — the global batch's Retry of the
/// folder reaches every one of them.
#[test]
fn the_catalog_shows_a_folders_state_folded_over_every_row() {
    crate::core::tool_adapters::test_overrides::kimi_on_amps_folder();
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = tmp.path().join("central");
    let alpha = write_manifest(&central, "alpha", "---\nname: alpha\n---\n");
    seed_skill(&store, "s1", "alpha", &alpha);
    seed_target(&store, "s1", "amp");
    store
        .upsert_skill_target(&SkillTargetRecord {
            id: "s1-kimi_cli".to_string(),
            skill_id: "s1".to_string(),
            tool: "kimi_cli".to_string(),
            target_path: "/tmp/elsewhere/s1".to_string(),
            mode: SyncMode::Symlink,
            status: SyncStatus::Error,
            last_error: Some("an earlier first sync failed".to_string()),
            synced_at: None,
            content_hash: None,
        })
        .unwrap();

    let entry = managed_skill_entry(&store, "s1").unwrap().unwrap();
    assert_eq!(entry.targets.len(), 1);
    assert_eq!(entry.targets[0].tool, "amp");
    assert_eq!(entry.targets[0].status, SyncStatus::Error);
    assert_eq!(entry.targets[0].id, "s1-kimi_cli", "the row the fold shows");
}

/// Astra 204 / Sol 301–302: the catalog carries whole-folder facts. After a
/// partial Disable — Kimi Code's legacy row settled `disabled`, the folder
/// row's removal failed and it still owns its artifact — the folder shows the
/// failure (failure ahead of muted), carries the disabled intent, and counts
/// the artifact a finished Disable would remove; the row standing for it is
/// the failed one, with its own timestamp.
#[test]
fn the_catalog_carries_a_partially_disabled_folders_facts() {
    crate::core::tool_adapters::test_overrides::kimi_on_amps_folder();
    let tmp = tempfile::tempdir().unwrap();
    let store = make_store(tmp.path());
    let central = tmp.path().join("central");
    let alpha = write_manifest(&central, "alpha", "---\nname: alpha\n---\n");
    seed_skill(&store, "s1", "alpha", &alpha);
    seed_target(&store, "s1", "amp");
    store
        .transition_skill_target(
            "s1-amp",
            crate::core::skill_store::TargetTransition::SyncFailed {
                error: "removal failed",
            },
        )
        .unwrap();
    store
        .upsert_skill_target(&SkillTargetRecord {
            id: "s1-kimi_cli".to_string(),
            skill_id: "s1".to_string(),
            tool: "kimi_cli".to_string(),
            target_path: "/tmp/amp/s1".to_string(),
            mode: SyncMode::Symlink,
            status: SyncStatus::Disabled,
            last_error: None,
            synced_at: Some(3),
            content_hash: None,
        })
        .unwrap();

    let entry = managed_skill_entry(&store, "s1").unwrap().unwrap();
    assert_eq!(entry.targets.len(), 1);
    let folder = &entry.targets[0];
    assert_eq!(
        (folder.id.as_str(), folder.tool.as_str(), folder.synced_at),
        ("s1-amp", "amp", Some(9))
    );
    assert_eq!(
        folder.facts,
        crate::core::placement::FolderFacts {
            status: SyncStatus::Error,
            disabled: true,
            deployed: true,
        }
    );
}
