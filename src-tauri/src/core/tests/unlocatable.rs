//! Tests for `core::unlocatable` — the Unlocatable states and Detach (a
//! `local` skill becomes `imported`). Re-point is tested with
//! `core::repoint`; Restore is the single-skill Update and is tested with
//! `core::refresh`.

use std::fs;
use std::path::PathBuf;

use crate::core::errors::SignalError;
use crate::core::installer::{install_imported_skill, install_local_skill, InstallerPaths};
use crate::core::refresh::{
    refresh_managed_skills, RefreshPolicy, RefreshReport, RefreshSelection, SkillRefreshStatus,
};
use crate::core::skill_store::SkillStore;
use crate::core::unlocatable::{
    detach_from_source, is_detachable, unlocatable_state, UnlocatableState,
};

struct Fixture {
    _dir: tempfile::TempDir,
    paths: InstallerPaths,
    store: SkillStore,
    skill_id: String,
    central_path: PathBuf,
}

/// One `local` skill whose source folder has since been moved: the record
/// still names the old folder, which is gone.
fn fixture_with_moved_source() -> (Fixture, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = InstallerPaths {
        home: dir.path().join("home"),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.home).expect("create home");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");

    let old = dir.path().join("old-place");
    fs::create_dir_all(&old).unwrap();
    fs::write(old.join("SKILL.md"), "---\nname: alpha\n---\n").unwrap();
    fs::write(old.join("a.txt"), "v1").unwrap();
    let installed =
        install_local_skill(&paths, &store, &old, Some("alpha".to_string())).expect("install");

    let new = dir.path().join("new-place");
    fs::rename(&old, &new).expect("move the folder");
    fs::write(new.join("a.txt"), "v2").unwrap();

    (
        Fixture {
            _dir: dir,
            paths,
            store,
            skill_id: installed.skill_id,
            central_path: installed.central_path,
        },
        new,
    )
}

fn update(f: &Fixture) -> RefreshReport {
    refresh_managed_skills(
        &f.paths,
        &f.store,
        RefreshSelection::Ids(vec![f.skill_id.clone()]),
        RefreshPolicy::default(),
        None,
        3000,
        |_| {},
    )
    .expect("refresh")
}

#[test]
fn detach_turns_the_skill_imported_with_no_source_and_no_tool_history() {
    let (f, _new) = fixture_with_moved_source();
    let before = f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap();
    assert_eq!(
        unlocatable_state(&before),
        Some(UnlocatableState::SourceMissing)
    );
    assert!(is_detachable(&before), "a local skill with a central copy");

    let record = detach_from_source(&f.store, &f.skill_id).expect("detach");

    assert_eq!(record.source_type, "imported");
    assert_eq!(record.source_ref, None);
    assert_eq!(
        record.imported_from_tool, None,
        "it came from a folder, not a Tool"
    );
    assert_eq!(
        unlocatable_state(&record),
        None,
        "the central copy is its truth now"
    );
    assert_eq!(
        format!(
            "{:?}",
            f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap()
        ),
        format!("{record:?}"),
        "the store holds the detached record"
    );
    // The single Update is refused typed, as for any imported skill.
    let report = update(&f);
    let [o] = report.skills.as_slice() else {
        panic!("{report:?}")
    };
    assert!(matches!(
        &o.status,
        SkillRefreshStatus::Failed { error }
            if matches!(error, crate::core::errors::CommandError::NotRefreshable { .. })
    ));
}

/// Detach makes the central copy the skill's truth, so there must be one:
/// with the source *and* the central copy gone, Detach would leave a row
/// only Remove can touch. It is refused typed, and the record is untouched.
#[test]
fn detach_is_refused_when_the_central_copy_is_also_gone() {
    let (f, _new) = fixture_with_moved_source();
    fs::remove_dir_all(&f.central_path).expect("lose the central copy");
    let before = f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap();
    assert!(!is_detachable(&before));

    let err = detach_from_source(&f.store, &f.skill_id).expect_err("refused");

    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::CentralPathMissing {
            path: f.central_path.to_string_lossy().to_string(),
        })
    );
    let after = f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap();
    assert_eq!(after.source_type, "local");
    assert_eq!(after.source_ref, before.source_ref);
}

#[test]
fn only_a_local_skill_can_be_detached() {
    let (f, _new) = fixture_with_moved_source();
    let found = f.paths.home.join(".claude/skills/taken-over");
    fs::create_dir_all(&found).unwrap();
    fs::write(found.join("SKILL.md"), "---\nname: taken-over\n---\n").unwrap();
    let imported = install_imported_skill(
        &f.paths,
        &f.store,
        &found,
        Some("taken-over".to_string()),
        Some("claude_code"),
    )
    .unwrap();

    assert!(detach_from_source(&f.store, &imported.skill_id).is_err());
    let err = detach_from_source(&f.store, "no-such-id").expect_err("unknown id");
    assert!(matches!(
        err.downcast_ref::<SignalError>(),
        Some(SignalError::NotFound { .. })
    ));
}
