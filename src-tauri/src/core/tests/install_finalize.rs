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

fn install_before_update(central: &Path, store: &SkillStore) -> super::SkillRecord {
    let installed = finalize_install(
        store,
        central,
        stage_skill(central, "---\nname: s\ndescription: old\n---\n"),
        NameIntent::UserProvided("s".to_string()),
        SkillProvenance::git("https://github.com/o/r", None, Some("rev1".to_string())),
    )
    .unwrap();
    store.get_skill_by_id(&installed.skill_id).unwrap().unwrap()
}

fn assert_old_skill(central: &Path, store: &SkillStore, before: &super::SkillRecord) {
    assert_eq!(fs::read(central.join("s/a.txt")).unwrap(), b"data");
    assert_eq!(
        fs::read_to_string(central.join("s/SKILL.md")).unwrap(),
        "---\nname: s\ndescription: old\n---\n"
    );
    // SkillRecord has no PartialEq; compare every field, including timestamps.
    assert_eq!(
        format!("{:?}", store.get_skill_by_id(&before.id).unwrap().unwrap()),
        format!("{before:?}")
    );
}

fn reject_skill_writes(db: &Path) {
    rusqlite::Connection::open(db)
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER reject_skill_write BEFORE INSERT ON skills
             BEGIN SELECT RAISE(ABORT, 'test upsert failure'); END;",
        )
        .unwrap();
}

#[test]
fn finalize_update_restores_old_bytes_when_upsert_fails() {
    let (db, store) = make_store();
    let central = tempfile::tempdir().unwrap();
    let before = install_before_update(central.path(), &store);
    let staged = stage_skill(central.path(), "---\nname: s\ndescription: new\n---\n");
    reject_skill_writes(&db.path().join("test.db"));

    let err = finalize_update(&store, &before, staged, Some("rev2".to_string())).unwrap_err();

    assert!(format!("{err:#}").contains("test upsert failure"));
    assert_old_skill(central.path(), &store, &before);
    assert_eq!(central_entries(central.path()), vec!["s"]);
}

#[test]
fn finalize_update_restores_old_bytes_when_staging_is_missing() {
    let (_db, store) = make_store();
    let central = tempfile::tempdir().unwrap();
    let before = install_before_update(central.path(), &store);
    // Rename and fallback copy both fail, after the old bytes were moved aside.
    let staged = StagingDir::new_in(central.path());

    let err = finalize_update(&store, &before, staged, None).unwrap_err();

    assert!(format!("{err:#}").contains("fallback copy"));
    assert_old_skill(central.path(), &store, &before);
    assert_eq!(central_entries(central.path()), vec!["s"]);
}

#[test]
fn finalize_update_failed_restore_leaves_central_missing() {
    let (db, store) = make_store();
    let central = tempfile::tempdir().unwrap();
    let before = install_before_update(central.path(), &store);
    fs::remove_dir_all(&before.central_path).unwrap();
    let staged = stage_skill(central.path(), "---\nname: s\ndescription: new\n---\n");
    reject_skill_writes(&db.path().join("test.db"));

    let err = finalize_update(&store, &before, staged, None).unwrap_err();

    assert!(format!("{err:#}").contains("test upsert failure"));
    assert!(central_entries(central.path()).is_empty());
    assert_eq!(
        format!("{:?}", store.get_skill_by_id(&before.id).unwrap().unwrap()),
        format!("{before:?}")
    );
}

#[test]
fn finalize_update_restores_missing_central_on_success() {
    let (_db, store) = make_store();
    let central = tempfile::tempdir().unwrap();
    let before = install_before_update(central.path(), &store);
    fs::remove_dir_all(&before.central_path).unwrap();
    let staged = stage_skill(central.path(), "---\nname: s\ndescription: restored\n---\n");

    let updated = finalize_update(&store, &before, staged, None).unwrap();

    assert_eq!(updated.description.as_deref(), Some("restored"));
    assert_eq!(central_entries(central.path()), vec!["s"]);
    assert_eq!(fs::read(central.path().join("s/a.txt")).unwrap(), b"data");
}

#[test]
fn rollback_cleanup_failure_retains_backup_and_original_error() {
    let (_db, store) = make_store();
    let central = tempfile::tempdir().unwrap();
    let before = install_before_update(central.path(), &store);
    let path = Path::new(&before.central_path);
    let backup = super::move_old_central_aside(path).unwrap().unwrap();
    // A non-directory replacement cannot be cleaned by remove_dir_all.
    fs::write(path, b"obstruction").unwrap();

    let err = super::rollback_update(path, Some(&backup), anyhow::anyhow!("original failure"));

    assert_eq!(err.root_cause().to_string(), "original failure");
    assert!(format!("{err:#}").contains(backup.to_str().unwrap()));
    assert_eq!(fs::read(backup.join("a.txt")).unwrap(), b"data");
    assert_eq!(fs::read(path).unwrap(), b"obstruction");
    // Recovery siblings are hidden from the ordinary root/recursive scan ladder.
    assert!(crate::core::skill_discovery::discover_skills(central.path()).is_empty());
}

#[cfg(unix)]
struct RestorePermissions(std::path::PathBuf, fs::Permissions);

#[cfg(unix)]
impl Drop for RestorePermissions {
    fn drop(&mut self) {
        fs::set_permissions(&self.0, self.1.clone()).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn finalize_update_readonly_parent_keeps_old_bytes_and_row() {
    use std::os::unix::fs::PermissionsExt;

    let (_db, store) = make_store();
    let central = tempfile::tempdir().unwrap();
    let before = install_before_update(central.path(), &store);
    // Stage elsewhere so Drop can clean up even while the central parent is locked.
    let staging_root = tempfile::tempdir().unwrap();
    let staged = stage_skill(staging_root.path(), "---\nname: s\n---\n");
    let _permissions = RestorePermissions(
        central.path().to_path_buf(),
        fs::metadata(central.path()).unwrap().permissions(),
    );
    fs::set_permissions(central.path(), fs::Permissions::from_mode(0o555)).unwrap();

    // The parent also forbids rename-aside: fail before touching the old bytes.
    finalize_update(&store, &before, staged, None).expect_err("parent is read-only");

    assert_old_skill(central.path(), &store, &before);
    assert_eq!(central_entries(central.path()), vec!["s"]);
}

#[cfg(unix)]
#[test]
fn rollback_rename_failure_names_and_preserves_backup() {
    use std::os::unix::fs::PermissionsExt;

    let (_db, store) = make_store();
    let central = tempfile::tempdir().unwrap();
    let before = install_before_update(central.path(), &store);
    let path = Path::new(&before.central_path);
    let backup = super::move_old_central_aside(path).unwrap().unwrap();
    let _permissions = RestorePermissions(
        central.path().to_path_buf(),
        fs::metadata(central.path()).unwrap().permissions(),
    );
    fs::set_permissions(central.path(), fs::Permissions::from_mode(0o555)).unwrap();

    let err = super::rollback_update(path, Some(&backup), anyhow::anyhow!("move failed"));

    assert_eq!(err.root_cause().to_string(), "move failed");
    let message = format!("{err:#}");
    assert!(message.contains("restore old central dir"), "{message}");
    assert!(message.contains(backup.to_str().unwrap()), "{message}");
    assert_eq!(fs::read(backup.join("a.txt")).unwrap(), b"data");
    assert!(!path.exists());
}

/// A failed fallback copy must not leave a partial directory at the skill's
/// final name inside the central repo: `Drop` only cleans the staging path, so
/// `move_into` removes `dest` itself before propagating. A leftover would read
/// as a name collision on the operator's next install attempt.
#[cfg(unix)]
#[test]
fn move_into_removes_partial_dest_when_the_fallback_copy_fails() {
    use std::os::unix::fs::PermissionsExt;

    let central = tempfile::tempdir().unwrap();
    let staged = StagingDir::new_in(central.path());
    let staged_path = staged.path().to_path_buf();
    fs::create_dir_all(&staged_path).unwrap();
    fs::write(staged_path.join("SKILL.md"), "---\nname: s\n---\n").unwrap();
    // An unreadable subdirectory makes the recursive copy fail part-way, after
    // it has already created `dest` and copied at least one entry.
    let blocked = staged_path.join("blocked");
    fs::create_dir_all(blocked.join("inner")).unwrap();
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o000)).unwrap();

    // Force the rename to fail so the copy fallback runs: renaming onto a
    // non-empty directory is ENOTEMPTY.
    let dest = central.path().join("s");
    fs::create_dir_all(dest.join("occupied")).unwrap();

    let err = staged.move_into(&dest).expect_err("copy must fail");
    assert!(
        format!("{:#}", err).contains("fallback copy"),
        "unexpected error: {:#}",
        err
    );
    assert!(
        !dest.exists(),
        "partial dest must be removed, found {:?}",
        fs::read_dir(&dest).map(|rd| rd.count())
    );

    // Restore permissions so the tempdir can be cleaned up.
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o755)).unwrap();
}
