use std::path::PathBuf;

use crate::core::central_repo::{ensure_central_repo, move_central_repo};
use crate::core::skill_store::{SkillRecord, SkillStore};

fn make_store() -> (tempfile::TempDir, SkillStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    (dir, store)
}

fn skill(id: &str, central_path: &std::path::Path) -> SkillRecord {
    SkillRecord {
        id: id.to_string(),
        name: id.to_string(),
        description: None,
        source_type: "local".to_string(),
        source_ref: None,
        source_subpath: None,
        source_revision: None,
        central_path: central_path.to_string_lossy().to_string(),
        content_hash: None,
        created_at: 1,
        updated_at: 1,
        last_sync_at: None,
        last_seen_at: 1,
        status: "active".to_string(),
    }
}

#[test]
fn ensure_central_repo_creates_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p: PathBuf = dir.path().join("a/b/c");
    assert!(!p.exists());
    ensure_central_repo(&p).unwrap();
    assert!(p.exists());
}

#[test]
fn move_central_repo_relocates_dirs_and_records() {
    let (dir, store) = make_store();
    let old_base = dir.path().join("old");
    let new_base = dir.path().join("new");
    for name in ["a", "b"] {
        let p = old_base.join(name);
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("SKILL.md"), name).unwrap();
        store.upsert_skill(&skill(name, &p)).unwrap();
    }
    std::fs::create_dir_all(&new_base).unwrap();

    move_central_repo(&store, &new_base).unwrap();

    for name in ["a", "b"] {
        assert!(new_base.join(name).join("SKILL.md").exists());
        assert!(!old_base.join(name).exists());
        let rec = store.get_skill_by_id(name).unwrap().unwrap();
        assert_eq!(PathBuf::from(rec.central_path), new_base.join(name));
        assert!(rec.updated_at > 1);
    }
}

#[test]
fn move_central_repo_refuses_when_target_exists_without_moving_anything() {
    let (dir, store) = make_store();
    let old_base = dir.path().join("old");
    let new_base = dir.path().join("new");
    for name in ["a", "b"] {
        let p = old_base.join(name);
        std::fs::create_dir_all(&p).unwrap();
        store.upsert_skill(&skill(name, &p)).unwrap();
    }
    // Collision on the second skill only.
    std::fs::create_dir_all(new_base.join("b")).unwrap();

    let err = move_central_repo(&store, &new_base).unwrap_err();
    assert!(err.to_string().contains("already exists"), "{err}");
    // Validation runs before any move, so "a" is untouched.
    assert!(old_base.join("a").exists());
    assert!(!new_base.join("a").exists());
}

#[test]
fn move_central_repo_refuses_when_source_missing() {
    let (dir, store) = make_store();
    let missing = dir.path().join("old").join("ghost");
    store.upsert_skill(&skill("ghost", &missing)).unwrap();
    let err = move_central_repo(&store, &dir.path().join("new")).unwrap_err();
    assert!(err.to_string().contains("not found"), "{err}");
}
