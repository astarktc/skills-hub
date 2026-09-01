use std::fs;
use std::path::Path;

use super::{
    ensure_name_available, finalize_install, finalize_update, NameIntent, SkillProvenance,
    StagingDir,
};
use crate::core::errors::SignalError;
use crate::core::skill_store::SkillStore;

fn make_store() -> (tempfile::TempDir, SkillStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    (dir, store)
}

fn stage_skill(central_dir: &Path, skill_md: &str) -> StagingDir {
    let staged = StagingDir::new_in(central_dir);
    fs::create_dir_all(staged.path()).unwrap();
    fs::write(staged.path().join("SKILL.md"), skill_md).unwrap();
    fs::write(staged.path().join("a.txt"), b"data").unwrap();
    staged
}

fn central_entries(central_dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = fs::read_dir(central_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    out.sort();
    out
}

#[test]
fn ensure_name_available_raises_typed_skill_exists() {
    let central = tempfile::tempdir().unwrap();
    fs::create_dir_all(central.path().join("taken")).unwrap();

    let err = ensure_name_available(central.path(), "taken").unwrap_err();
    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::SkillExists {
            name: "taken".to_string()
        })
    );
    assert_eq!(
        ensure_name_available(central.path(), "free").unwrap(),
        central.path().join("free")
    );
}

#[test]
fn staging_dir_is_removed_on_drop_when_not_consumed() {
    let central = tempfile::tempdir().unwrap();
    let path = {
        let staged = stage_skill(central.path(), "---\nname: x\n---\n");
        assert!(staged.path().exists());
        staged.path().to_path_buf()
    };
    assert!(!path.exists(), "unconsumed staging dir must be cleaned up");
    assert!(central_entries(central.path()).is_empty());
}

#[test]
fn finalize_install_prefers_skill_md_name_for_derived_names() {
    let (_db, store) = make_store();
    let central = tempfile::tempdir().unwrap();
    let staged = stage_skill(
        central.path(),
        "---\nname: real-name\ndescription: A real skill\n---\n",
    );

    let res = finalize_install(
        &store,
        central.path(),
        staged,
        NameIntent::Derived("skills".to_string()),
        SkillProvenance::git("https://github.com/o/r", Some("skills".to_string()), None),
    )
    .unwrap();

    assert_eq!(res.name, "real-name");
    assert_eq!(res.central_path, central.path().join("real-name"));
    assert!(res.central_path.join("a.txt").exists());
    assert_eq!(
        central_entries(central.path()),
        vec!["real-name".to_string()]
    );

    let record = store.get_skill_by_id(&res.skill_id).unwrap().unwrap();
    assert_eq!(record.name, "real-name");
    assert_eq!(record.description.as_deref(), Some("A real skill"));
    assert_eq!(record.source_type, "git");
    assert_eq!(record.source_ref.as_deref(), Some("https://github.com/o/r"));
    assert_eq!(record.source_subpath.as_deref(), Some("skills"));
    assert_eq!(record.source_revision.as_deref(), Some("api-download"));
    assert_eq!(record.central_path, res.central_path.to_string_lossy());
    assert_eq!(record.status, "ok");
}

#[test]
fn finalize_install_keeps_user_provided_name() {
    let (_db, store) = make_store();
    let central = tempfile::tempdir().unwrap();
    let staged = stage_skill(central.path(), "---\nname: real-name\n---\n");

    let res = finalize_install(
        &store,
        central.path(),
        staged,
        NameIntent::UserProvided("mine".to_string()),
        SkillProvenance::local(Path::new("/src/mine")),
    )
    .unwrap();

    assert_eq!(res.name, "mine");
    assert_eq!(central_entries(central.path()), vec!["mine".to_string()]);
    let record = store.get_skill_by_id(&res.skill_id).unwrap().unwrap();
    assert_eq!(record.source_type, "local");
    assert_eq!(record.source_ref.as_deref(), Some("/src/mine"));
    assert_eq!(record.source_revision, None);
}

#[test]
fn finalize_install_falls_back_to_derived_name_when_skill_md_name_is_taken() {
    let (_db, store) = make_store();
    let central = tempfile::tempdir().unwrap();
    fs::create_dir_all(central.path().join("real-name")).unwrap();
    let staged = stage_skill(central.path(), "---\nname: real-name\n---\n");

    let res = finalize_install(
        &store,
        central.path(),
        staged,
        NameIntent::Derived("derived".to_string()),
        SkillProvenance::git("https://github.com/o/r", None, Some("abc".to_string())),
    )
    .unwrap();

    assert_eq!(res.name, "derived");
    assert_eq!(
        central_entries(central.path()),
        vec!["derived".to_string(), "real-name".to_string()]
    );
    let record = store.get_skill_by_id(&res.skill_id).unwrap().unwrap();
    assert_eq!(record.source_revision.as_deref(), Some("abc"));
}

#[test]
fn finalize_install_rejects_collision_and_discards_staging() {
    let (_db, store) = make_store();
    let central = tempfile::tempdir().unwrap();
    fs::create_dir_all(central.path().join("taken")).unwrap();
    let staged = stage_skill(central.path(), "---\nname: taken\n---\n");

    let err = finalize_install(
        &store,
        central.path(),
        staged,
        NameIntent::Derived("taken".to_string()),
        SkillProvenance::local(Path::new("/src/taken")),
    )
    .unwrap_err();

    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::SkillExists {
            name: "taken".to_string()
        })
    );
    assert_eq!(central_entries(central.path()), vec!["taken".to_string()]);
    assert!(store.list_skills().unwrap().is_empty());
}

#[test]
fn finalize_update_swaps_content_and_preserves_identity() {
    let (_db, store) = make_store();
    let central = tempfile::tempdir().unwrap();

    let staged = stage_skill(central.path(), "---\nname: s\ndescription: v1\n---\n");
    let installed = finalize_install(
        &store,
        central.path(),
        staged,
        NameIntent::UserProvided("s".to_string()),
        SkillProvenance::git("https://github.com/o/r", None, Some("rev1".to_string())),
    )
    .unwrap();
    let before = store.get_skill_by_id(&installed.skill_id).unwrap().unwrap();

    let staged = StagingDir::new_in(central.path());
    fs::create_dir_all(staged.path()).unwrap();
    fs::write(staged.path().join("SKILL.md"), "---\nname: s\n---\n").unwrap();
    fs::write(staged.path().join("b.txt"), b"new").unwrap();

    let updated = finalize_update(&store, &before, staged, Some("rev2".to_string())).unwrap();

    assert_eq!(updated.id, before.id);
    assert_eq!(updated.name, "s");
    assert_eq!(updated.created_at, before.created_at);
    assert_eq!(updated.source_revision.as_deref(), Some("rev2"));
    // Description falls back to the previous value when the new SKILL.md has none.
    assert_eq!(updated.description.as_deref(), Some("v1"));
    assert_eq!(central_entries(central.path()), vec!["s".to_string()]);
    let central_path = central.path().join("s");
    assert!(central_path.join("b.txt").exists());
    assert!(!central_path.join("a.txt").exists(), "old content replaced");
    assert_eq!(
        store
            .get_skill_by_id(&before.id)
            .unwrap()
            .unwrap()
            .source_revision,
        updated.source_revision
    );
}
