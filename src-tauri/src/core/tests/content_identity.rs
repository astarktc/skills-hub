use std::fs;

use super::{read, Source};

fn directory_identity(path: &std::path::Path) -> Option<String> {
    read(Source::Directory(path))
}

#[test]
fn managed_read_backfills_once_and_trusts_the_row() {
    use crate::core::install_finalize::{
        finalize_install, NameIntent, SkillProvenance, StagingDir,
    };
    use crate::core::skill_store::SkillStore;
    let dir = tempfile::tempdir().unwrap();
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().unwrap();
    let staged = StagingDir::new_in(dir.path());
    fs::create_dir(staged.path()).unwrap();
    fs::write(staged.path().join("SKILL.md"), "hello").unwrap();
    let installed = finalize_install(
        &store,
        dir.path(),
        staged,
        NameIntent::UserProvided("skill".into()),
        SkillProvenance::imported(None),
    )
    .unwrap();
    let conn = rusqlite::Connection::open(store.db_path()).unwrap();
    conn.execute_batch(
        "UPDATE skills SET content_hash = NULL;
        CREATE TABLE hash_writes (n INTEGER NOT NULL);
        INSERT INTO hash_writes VALUES (0);
        CREATE TRIGGER count_hash AFTER UPDATE OF content_hash ON skills
        BEGIN UPDATE hash_writes SET n = n + 1; END;",
    )
    .unwrap();
    let first = read(Source::Managed {
        store: &store,
        skill_id: &installed.skill_id,
    });
    assert!(first.is_some());
    fs::remove_dir_all(&installed.central_path).unwrap();
    assert_eq!(
        read(Source::Managed {
            store: &store,
            skill_id: &installed.skill_id
        }),
        first
    );
    assert_eq!(
        conn.query_row("SELECT n FROM hash_writes", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        1
    );
    conn.execute_batch("UPDATE skills SET content_hash = NULL;")
        .unwrap();
    let unknown = read(Source::Managed {
        store: &store,
        skill_id: &installed.skill_id,
    });
    assert_eq!(unknown, None);
    use crate::core::sync_status::{next_status, Observation, SyncMode, SyncStatus};
    for current in [SyncStatus::Synced, SyncStatus::Stale, SyncStatus::Error] {
        assert_eq!(
            next_status(&Observation {
                source_present: true,
                target_present: true,
                mode: SyncMode::Copy,
                current,
                source_hash: unknown.as_deref(),
                recorded_hash: Some("previous")
            }),
            current
        );
    }
}

#[cfg(unix)]
#[test]
fn internal_symlinks_are_not_content() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), b"hello").unwrap();
    let expected = directory_identity(dir.path()).unwrap();

    for (name, target) in [("relative", "a.txt"), ("dangling", "missing")] {
        std::os::unix::fs::symlink(target, dir.path().join(name)).unwrap();
        assert_eq!(directory_identity(dir.path()).unwrap(), expected);
    }
}

#[test]
fn hash_changes_with_content_and_ignores_git_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();

    fs::create_dir_all(root.join("sub")).unwrap();
    fs::write(root.join("a.txt"), b"hello").unwrap();
    fs::write(root.join("sub/b.txt"), b"world").unwrap();

    let h1 = directory_identity(root).unwrap();

    fs::create_dir_all(root.join(".git")).unwrap();
    fs::write(root.join(".git/ignored"), b"ignored").unwrap();
    let h2 = directory_identity(root).unwrap();
    assert_eq!(h1, h2, ".git contents should be ignored");

    fs::write(root.join("a.txt"), b"hello2").unwrap();
    let h3 = directory_identity(root).unwrap();
    assert_ne!(h2, h3);
}

/// The tri-state comparison: only two successfully read identities decide
/// Same / Different; an unreadable or absent side is `Unknown`, never
/// "different" (a caller may only act destructively on `Different`).
#[test]
fn compare_says_unknown_when_either_identity_cannot_be_read() {
    use super::{compare, Comparison};
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    let c = dir.path().join("c");
    for (path, body) in [(&a, "one"), (&b, "one"), (&c, "two")] {
        fs::create_dir(path).unwrap();
        fs::write(path.join("SKILL.md"), body).unwrap();
    }
    assert_eq!(compare(Source::Directory(&a), &b), Comparison::Same);
    assert_eq!(compare(Source::Directory(&a), &c), Comparison::Different);
    let missing = dir.path().join("missing");
    assert_eq!(
        compare(Source::Directory(&a), &missing),
        Comparison::Unknown
    );
    assert_eq!(
        compare(Source::Directory(&missing), &a),
        Comparison::Unknown
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // Byte-identical to `a` except that one file cannot be read.
        fs::write(a.join("notes.txt"), "same").unwrap();
        let unreadable = dir.path().join("unreadable");
        fs::create_dir(&unreadable).unwrap();
        fs::write(unreadable.join("SKILL.md"), "one").unwrap();
        fs::write(unreadable.join("notes.txt"), "same").unwrap();
        fs::set_permissions(
            unreadable.join("notes.txt"),
            fs::Permissions::from_mode(0o000),
        )
        .unwrap();
        if fs::read(unreadable.join("notes.txt")).is_err() {
            assert_eq!(
                compare(Source::Directory(&a), &unreadable),
                Comparison::Unknown
            );
            assert!(!super::same_content(Source::Directory(&a), &unreadable));
        }
        fs::set_permissions(
            unreadable.join("notes.txt"),
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
    }
}
