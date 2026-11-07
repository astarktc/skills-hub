//! Tests for `core::unlocatable` — the Unlocatable states and Detach (a
//! `local` skill becomes `imported`). Re-point is tested with
//! `core::repoint`; Restore is the single-skill Update and is tested with
//! `core::refresh`.

use crate::core::environment::HostRoots;
use std::fs;
use std::path::PathBuf;

use crate::core::errors::SignalError;
use crate::core::installer::{install_imported_skill, install_local_skill, InstallerPaths};
use crate::core::refresh::{
    refresh_managed_skills, RefreshReport, RefreshSelection, SkillRefreshStatus,
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
        roots: HostRoots::under_home(&dir.path().join("home")),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.roots.home).expect("create home");
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
        &crate::core::credentials::MemoryStore::new(),
        RefreshSelection::Ids(vec![f.skill_id.clone()]),
        None,
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
    let found = f.paths.roots.home.join(".claude/skills/taken-over");
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

/// Round 20 ticket 19: a record written before install checked names, whose
/// name is not one folder name (`../evil`) and whose central copy therefore
/// lies outside the Library. It reads Unlocatable (`not_a_folder_name`),
/// Refresh (all) skips it, an explicit Update is refused before anything is
/// acquired or landed, and no Deployment is ever made from it.
#[test]
fn a_record_whose_name_is_not_one_folder_name_is_never_deployed_from() {
    use crate::core::errors::CommandError;
    use crate::core::global_sync::{
        sync_skills_to_tools, BatchPolicy, BatchSkill, BatchTargetStatus,
    };
    use crate::core::tool_adapters::{adapter_by_key, mark_installed_in, skills_dir_in};

    let dir = tempfile::tempdir().expect("tempdir");
    let paths = InstallerPaths {
        roots: HostRoots::under_home(&dir.path().join("home")),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.roots.home).unwrap();
    let claude = adapter_by_key("claude_code").unwrap();
    mark_installed_in(&paths.roots, claude);
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().unwrap();
    let source = dir.path().join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("SKILL.md"), "---\nname: evil\n---\n").unwrap();
    fs::write(source.join("a.txt"), "v1").unwrap();
    let installed = install_local_skill(&paths, &store, &source, Some("evil".into())).unwrap();

    // What an install of `name: ../evil` wrote before ticket 19: the record's
    // name, and a copy joined from it — lexically under the central repo,
    // physically beside it.
    let outside = dir.path().join("evil");
    fs::rename(&installed.central_path, &outside).unwrap();
    let mut record = store.get_skill_by_id(&installed.skill_id).unwrap().unwrap();
    record.name = "../evil".into();
    record.central_path = paths
        .central_dir
        .join("../evil")
        .to_string_lossy()
        .into_owned();
    store.upsert_skill(&record).unwrap();
    fs::write(source.join("a.txt"), "v2").unwrap();

    assert_eq!(
        unlocatable_state(&record),
        Some(UnlocatableState::NotAFolderName)
    );
    let catalog = crate::core::skill_catalog::managed_skill_catalog(&store).unwrap();
    assert_eq!(
        catalog[0].unlocatable,
        Some(UnlocatableState::NotAFolderName)
    );

    let refresh = |selection| {
        refresh_managed_skills(
            &paths,
            &store,
            &crate::core::credentials::MemoryStore::new(),
            selection,
            None,
            |_| {},
        )
        .expect("refresh")
    };
    let all = refresh(RefreshSelection::All);
    assert!(
        matches!(
            all.skills.as_slice(),
            [o] if matches!(
                &o.status,
                SkillRefreshStatus::Skipped {
                    reason: crate::core::refresh::RefreshSkipReason::Unlocatable {
                        state: UnlocatableState::NotAFolderName
                    }
                }
            )
        ),
        "{all:?}"
    );
    let named = refresh(RefreshSelection::Ids(vec![record.id.clone()]));
    assert!(
        matches!(
            named.skills.as_slice(),
            [o] if matches!(
                &o.status,
                SkillRefreshStatus::Failed { error: CommandError::SkillNameInvalid { name } }
                    if name == "../evil"
            )
        ),
        "{named:?}"
    );
    assert_eq!(
        fs::read_to_string(outside.join("a.txt")).unwrap(),
        "v1",
        "no source bytes landed outside the Library"
    );

    let outcomes = sync_skills_to_tools(
        &paths.roots,
        &store,
        &paths.central_dir,
        &[BatchSkill {
            skill_id: record.id.clone(),
            skill_name: record.name.clone(),
            source_path: PathBuf::from(&record.central_path),
        }],
        &["claude_code".to_string()],
        &BatchPolicy {
            overwrite: false,
            overwrite_if_same_content: true,
            overrides: Vec::new(),
        },
        1000,
        |_| {},
    );
    assert!(
        matches!(
            outcomes.as_slice(),
            [o] if matches!(
                &o.status,
                BatchTargetStatus::Failed { error: CommandError::InvalidPath { reason, .. } }
                    if reason == "not_a_folder_name"
            )
        ),
        "{outcomes:?}"
    );
    let skills_dir = skills_dir_in(&paths.roots, claude);
    assert!(
        !skills_dir.exists() || fs::read_dir(&skills_dir).unwrap().count() == 0,
        "no Deployment in the harness"
    );
    assert!(fs::symlink_metadata(skills_dir.join("../evil")).is_err());
    assert!(store.list_skill_targets(&record.id).unwrap().is_empty());
}
