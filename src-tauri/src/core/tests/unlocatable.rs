//! Tests for `core::unlocatable` — the two repairs of an Unlocatable skill
//! that are store-only: Re-point (new source folder) and Detach (becomes
//! `imported`). Restore is the single-skill Update and is tested with
//! `core::refresh`.

use std::fs;
use std::path::PathBuf;

use crate::core::errors::SignalError;
use crate::core::installer::{install_imported_skill, install_local_skill, InstallerPaths};
use crate::core::refresh::{
    refresh_managed_skills, RefreshPolicy, RefreshSelection, SkillRefreshStatus,
};
use crate::core::skill_store::SkillStore;
use crate::core::unlocatable::{
    detach_from_source, repoint_local_source, unlocatable_state, UnlocatableState,
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

fn update(f: &Fixture) -> crate::core::refresh::RefreshReport {
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
fn repoint_rewrites_the_source_and_the_next_update_lands_the_new_folders_bytes() {
    let (f, new) = fixture_with_moved_source();
    let before = f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap();
    assert_eq!(
        unlocatable_state(&before),
        Some(UnlocatableState::SourceMissing)
    );

    let record =
        repoint_local_source(&f.store, &f.paths.home, &f.skill_id, &new).expect("re-point");

    assert_eq!(record.source_ref.as_deref(), Some(new.to_str().unwrap()));
    assert_eq!(record.source_type, "local", "still a local skill");
    assert_eq!(unlocatable_state(&record), None);
    assert_eq!(
        fs::read_to_string(f.central_path.join("a.txt")).unwrap(),
        "v1",
        "re-pointing alone moves no bytes"
    );

    let report = update(&f);
    assert!(
        matches!(
            report.skills.as_slice(),
            [o] if matches!(o.status, SkillRefreshStatus::Refreshed { .. })
        ),
        "{report:?}"
    );
    assert_eq!(
        fs::read_to_string(f.central_path.join("a.txt")).unwrap(),
        "v2",
        "the Update copies from the new folder"
    );
}

#[test]
fn repoint_validates_the_new_folder_the_way_add_does() {
    let (f, _new) = fixture_with_moved_source();
    let before = f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap();

    // Not there.
    let err = repoint_local_source(
        &f.store,
        &f.paths.home,
        &f.skill_id,
        &f.paths.home.join("nowhere"),
    )
    .expect_err("a missing folder is refused");
    assert!(matches!(
        err.downcast_ref::<SignalError>(),
        Some(SignalError::SourcePathMissing { .. })
    ));

    // No SKILL.md.
    let empty = f.paths.home.join("Documents/empty");
    fs::create_dir_all(&empty).unwrap();
    let err = repoint_local_source(&f.store, &f.paths.home, &f.skill_id, &empty)
        .expect_err("a folder without SKILL.md is refused");
    assert!(matches!(
        err.downcast_ref::<SignalError>(),
        Some(SignalError::SkillInvalid { .. })
    ));

    // Inside a Tool's skills dir: a Tool's copy is not a source.
    let in_tool = f.paths.home.join(".claude/skills/alpha");
    fs::create_dir_all(&in_tool).unwrap();
    fs::write(in_tool.join("SKILL.md"), "---\nname: alpha\n---\n").unwrap();
    let err = repoint_local_source(&f.store, &f.paths.home, &f.skill_id, &in_tool)
        .expect_err("a Tool-dir folder is refused");
    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::LocalSourceInsideToolDir {
            path: in_tool.to_string_lossy().to_string(),
            tool: "claude_code".to_string(),
        })
    );

    let after = f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap();
    assert_eq!(
        format!("{after:?}"),
        format!("{before:?}"),
        "a refused re-point changes nothing"
    );
}

#[test]
fn detach_turns_the_skill_imported_with_no_source_and_no_tool_history() {
    let (f, _new) = fixture_with_moved_source();

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
            if matches!(error.downcast_ref::<SignalError>(), Some(SignalError::NotRefreshable { .. }))
    ));
}

#[test]
fn only_a_local_skill_can_be_repointed_or_detached() {
    let (f, new) = fixture_with_moved_source();
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

    assert!(repoint_local_source(&f.store, &f.paths.home, &imported.skill_id, &new).is_err());
    assert!(detach_from_source(&f.store, &imported.skill_id).is_err());
    let err = detach_from_source(&f.store, "no-such-id").expect_err("unknown id");
    assert!(matches!(
        err.downcast_ref::<SignalError>(),
        Some(SignalError::NotFound { .. })
    ));
}
