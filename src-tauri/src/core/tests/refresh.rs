//! Tests for `core::refresh` — the Refresh (all) batch: acquire every
//! selected skill, then finalize + propagate each under the mutation guard.
//!
//! Every case runs against a temp home / central dir / DB; installedness is
//! faked by creating a Tool's detect dir under the temp home.

use std::fs;
use std::path::PathBuf;

use crate::core::installer::{install_local_skill, InstallerPaths};
use crate::core::refresh::{
    refresh_managed_skills, RefreshPhase, RefreshPolicy, RefreshSelection, SkillRefreshStatus,
};
use crate::core::skill_store::SkillStore;
use crate::core::tool_adapters::adapter_by_key;

struct Fixture {
    _dir: tempfile::TempDir,
    paths: InstallerPaths,
    store: SkillStore,
    source: tempfile::TempDir,
    skill_id: String,
}

/// One local-source Managed skill installed into a temp central repo.
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = InstallerPaths {
        home: dir.path().join("home"),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.home).expect("create home");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");

    let source = tempfile::tempdir().expect("source tempdir");
    fs::write(source.path().join("SKILL.md"), "---\nname: alpha\n---\n").expect("write SKILL.md");
    fs::write(source.path().join("a.txt"), "v1").expect("write a.txt");
    let installed = install_local_skill(&paths, &store, source.path(), Some("alpha".to_string()))
        .expect("install");

    Fixture {
        _dir: dir,
        paths,
        store,
        source,
        skill_id: installed.skill_id,
    }
}

fn refresh(f: &Fixture, policy: RefreshPolicy) -> crate::core::refresh::RefreshReport {
    refresh_managed_skills(
        &f.paths,
        &f.store,
        RefreshSelection::All,
        policy,
        3000,
        |_| {},
    )
    .expect("refresh reads its own skill list")
}

#[test]
fn a_refreshed_skill_gets_its_new_bytes_and_reports_its_targets() {
    let f = fixture();
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    fs::create_dir_all(f.paths.home.join(claude.relative_detect_dir)).expect("install tool");
    fs::write(f.source.path().join("a.txt"), "v2").expect("write a.txt");

    let mut phases: Vec<(RefreshPhase, String)> = Vec::new();
    let report = refresh_managed_skills(
        &f.paths,
        &f.store,
        RefreshSelection::Ids(vec![f.skill_id.clone()]),
        RefreshPolicy::default(),
        3000,
        |p| phases.push((p.phase, p.skill_name.to_string())),
    )
    .expect("refresh");

    assert!(matches!(
        report.skills.as_slice(),
        [outcome] if matches!(outcome.status, SkillRefreshStatus::Refreshed { .. })
    ));
    let central = PathBuf::from(
        f.store
            .get_skill_by_id(&f.skill_id)
            .expect("query")
            .expect("skill")
            .central_path,
    );
    assert_eq!(
        fs::read_to_string(central.join("a.txt")).expect("read central"),
        "v2"
    );
    assert_eq!(
        phases,
        vec![
            (RefreshPhase::Acquiring, "alpha".to_string()),
            (RefreshPhase::Applying, "alpha".to_string()),
        ],
        "progress ticks come from the backend, one per phase step"
    );
}

#[test]
fn a_skill_that_fails_acquisition_is_reported_and_never_finalized() {
    let f = fixture();
    let central_before = f
        .store
        .get_skill_by_id(&f.skill_id)
        .expect("query")
        .expect("skill");
    // The local source is gone: acquisition cannot produce bytes.
    let source_path = f.source.path().to_path_buf();
    fs::remove_dir_all(&source_path).expect("remove source");

    let report = refresh(&f, RefreshPolicy::default());

    assert!(
        matches!(
            report.skills.as_slice(),
            [outcome] if matches!(outcome.status, SkillRefreshStatus::Failed { .. })
        ),
        "got {:?}",
        report
    );
    let after = f
        .store
        .get_skill_by_id(&f.skill_id)
        .expect("query")
        .expect("skill");
    assert_eq!(
        after.updated_at, central_before.updated_at,
        "a skill that failed acquisition must not be finalized"
    );
    assert_eq!(
        fs::read_to_string(PathBuf::from(&after.central_path).join("a.txt")).expect("read central"),
        "v1",
        "the central copy is untouched"
    );
}

#[test]
fn reassert_auto_sync_creates_a_target_the_skill_was_never_on() {
    let f = fixture();
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    fs::create_dir_all(f.paths.home.join(claude.relative_detect_dir)).expect("install tool");
    assert!(
        f.store
            .list_skill_targets(&f.skill_id)
            .expect("query")
            .is_empty(),
        "the skill starts on no Tool"
    );

    refresh(
        &f,
        RefreshPolicy {
            reassert_auto_sync: true,
        },
    );

    let row = f
        .store
        .get_skill_target(&f.skill_id, "claude_code")
        .expect("query")
        .expect("the re-assert creates the missing target");
    assert!(PathBuf::from(&row.target_path).exists());
}

#[test]
fn without_the_reassert_policy_a_missing_target_stays_missing() {
    let f = fixture();
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    fs::create_dir_all(f.paths.home.join(claude.relative_detect_dir)).expect("install tool");

    refresh(&f, RefreshPolicy::default());

    assert!(
        f.store
            .get_skill_target(&f.skill_id, "claude_code")
            .expect("query")
            .is_none(),
        "refresh alone never creates a Sync target"
    );
    assert!(!f.paths.home.join(".claude/skills/alpha").exists());
}

#[test]
fn a_skill_that_fails_acquisition_is_excluded_from_the_reassert() {
    let f = fixture();
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    fs::create_dir_all(f.paths.home.join(claude.relative_detect_dir)).expect("install tool");
    fs::remove_dir_all(f.source.path()).expect("remove source");

    refresh(
        &f,
        RefreshPolicy {
            reassert_auto_sync: true,
        },
    );

    assert!(
        f.store
            .get_skill_target(&f.skill_id, "claude_code")
            .expect("query")
            .is_none(),
        "a skill whose bytes could not be acquired must not be synced anywhere"
    );
}
