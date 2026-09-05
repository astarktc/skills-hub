//! Tests for `core::legacy_reclassification` — the once-per-launch pass that
//! turns `local` rows whose "source" was really a Tool's skills directory
//! into `imported` rows (spec Q5).
//!
//! Every case runs against a temp home / central dir / DB; nothing touches
//! the operator's real library.

use std::fs;
use std::path::{Path, PathBuf};

use crate::core::legacy_reclassification::reclassify_legacy_imports;
use crate::core::skill_store::{SkillRecord, SkillStore};
use crate::core::tool_adapters::adapter_by_key;

struct Fixture {
    _dir: tempfile::TempDir,
    home: PathBuf,
    central: PathBuf,
    store: SkillStore,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = dir.path().join("home");
    let central = dir.path().join("central");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&central).expect("create central");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    Fixture {
        _dir: dir,
        home,
        central,
        store,
    }
}

/// A `local` row exactly as v1.2.3's import recorded it: the Tool path as
/// the source, and a central copy.
fn local_row(f: &Fixture, id: &str, source: &Path) -> SkillRecord {
    SkillRecord {
        id: id.to_string(),
        name: id.to_string(),
        description: None,
        source_type: "local".to_string(),
        source_ref: Some(source.to_string_lossy().into_owned()),
        source_subpath: None,
        source_revision: None,
        central_path: f.central.join(id).to_string_lossy().into_owned(),
        content_hash: None,
        created_at: 1,
        updated_at: 2,
        last_sync_at: None,
        last_seen_at: 3,
        status: "ok".to_string(),
        imported_from_tool: None,
    }
}

fn seed(f: &Fixture, record: SkillRecord) -> SkillRecord {
    f.store.upsert_skill(&record).expect("upsert");
    record
}

fn seed_local_row(f: &Fixture, id: &str, source: &Path) -> SkillRecord {
    seed(f, local_row(f, id, source))
}

/// A real skill directory inside a Tool's global skills dir under the temp home.
fn tool_skill_dir(f: &Fixture, key: &str, name: &str) -> PathBuf {
    let adapter = adapter_by_key(key).unwrap_or_else(|| panic!("adapter {}", key));
    let dir = f.home.join(adapter.relative_skills_dir).join(name);
    fs::create_dir_all(&dir).expect("create tool skill dir");
    dir
}

fn symlink_dir(target: &Path, link: &Path) {
    fs::create_dir_all(link.parent().expect("link parent")).expect("create link parent");
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).expect("symlink");
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(target, link).expect("symlink");
}

fn row(f: &Fixture, id: &str) -> SkillRecord {
    f.store
        .get_skill_by_id(id)
        .expect("get")
        .unwrap_or_else(|| panic!("row {}", id))
}

/// Rule (i): the stored source lives inside a Tool's skills dir under home —
/// the row becomes `imported`, its source is cleared, and the Tool is read
/// off the path.
#[test]
fn a_local_row_inside_a_tool_skills_dir_becomes_imported_from_that_tool() {
    let f = fixture();
    let source = tool_skill_dir(&f, "claude_code", "foo");
    seed_local_row(&f, "foo", &source);

    let changed = reclassify_legacy_imports(&f.store, &f.home, &f.central).expect("pass");

    assert_eq!(changed, 1);
    let after = row(&f, "foo");
    assert_eq!(after.source_type, "imported");
    assert_eq!(after.source_ref, None);
    assert_eq!(after.imported_from_tool.as_deref(), Some("claude_code"));
}

/// Rule (ii): the stored source is a symlink that resolves into the central
/// repo — the row's "source" is its own central copy, so it becomes
/// `imported`. The link here lives outside every Tool dir, so rule (i) is
/// not what catches it; without a Tool on the path there is no found-in Tool.
#[test]
fn a_local_row_whose_path_links_into_central_becomes_imported() {
    let f = fixture();
    let central_copy = f.central.join("bar");
    fs::create_dir_all(&central_copy).expect("central copy");
    let link = f.home.join("elsewhere").join("bar");
    symlink_dir(&central_copy, &link);
    seed_local_row(&f, "bar", &link);

    let changed = reclassify_legacy_imports(&f.store, &f.home, &f.central).expect("pass");

    assert_eq!(changed, 1);
    let after = row(&f, "bar");
    assert_eq!(after.source_type, "imported");
    assert_eq!(after.source_ref, None);
    assert_eq!(after.imported_from_tool, None);
}

/// Rule (iii): the stored source does not exist on this machine but has the
/// shape of a Tool skills-dir path under some other home prefix — a row
/// migrated from a Windows/WSL database. The Tool is read off the shape.
#[test]
fn a_missing_path_shaped_like_a_tool_skills_dir_becomes_imported() {
    let f = fixture();
    seed_local_row(&f, "wsl", Path::new("/mnt/c/Users/x/.claude/skills/wsl"));
    seed_local_row(&f, "win", Path::new(r"C:\Users\x\.pi\agent\skills\win"));

    let changed = reclassify_legacy_imports(&f.store, &f.home, &f.central).expect("pass");

    assert_eq!(changed, 2);
    let wsl = row(&f, "wsl");
    assert_eq!(wsl.source_type, "imported");
    assert_eq!(wsl.source_ref, None);
    assert_eq!(wsl.imported_from_tool.as_deref(), Some("claude_code"));
    let win = row(&f, "win");
    assert_eq!(win.source_type, "imported");
    assert_eq!(win.imported_from_tool.as_deref(), Some("pi"));
}

/// Untouched: a `local` row whose folder is the operator's own — outside every
/// Tool dir — whether or not it still exists; and `git` / `imported` rows,
/// whatever their paths say.
#[test]
fn genuine_local_git_and_imported_rows_are_left_exactly_as_they_are() {
    let f = fixture();
    let own_folder = f.home.join("Projects").join("my-skill");
    fs::create_dir_all(&own_folder).expect("own folder");
    let existing = seed_local_row(&f, "existing", &own_folder);
    let vanished = seed_local_row(&f, "vanished", &f.home.join("gone").join("skill"));
    // A folder that exists under a *different* home's Tool dir is not this
    // operator's Tool dir; only the missing-path rule reads shape.
    let other_home = f
        .home
        .join("other-user")
        .join(".claude")
        .join("skills")
        .join("x");
    fs::create_dir_all(&other_home).expect("other home");
    let other = seed_local_row(&f, "other", &other_home);
    let git = seed(
        &f,
        SkillRecord {
            source_type: "git".to_string(),
            source_ref: Some("https://github.com/o/r".to_string()),
            ..local_row(&f, "git", &tool_skill_dir(&f, "claude_code", "git"))
        },
    );
    let imported = seed(
        &f,
        SkillRecord {
            source_type: "imported".to_string(),
            source_ref: None,
            imported_from_tool: Some("pi".to_string()),
            ..local_row(&f, "imported", &tool_skill_dir(&f, "pi", "imported"))
        },
    );

    let changed = reclassify_legacy_imports(&f.store, &f.home, &f.central).expect("pass");

    assert_eq!(changed, 0);
    for expected in [existing, vanished, other, git, imported] {
        let after = row(&f, &expected.id);
        assert_eq!(after.source_type, expected.source_type, "{}", expected.id);
        assert_eq!(after.source_ref, expected.source_ref, "{}", expected.id);
        assert_eq!(
            after.imported_from_tool, expected.imported_from_tool,
            "{}",
            expected.id
        );
    }
}

/// The pass is idempotent: a second run over the same store changes nothing.
#[test]
fn running_the_pass_twice_changes_nothing_the_second_time() {
    let f = fixture();
    seed_local_row(&f, "foo", &tool_skill_dir(&f, "claude_code", "foo"));
    seed_local_row(&f, "wsl", Path::new("/mnt/c/Users/x/.claude/skills/wsl"));
    seed_local_row(&f, "mine", &f.home.join("mine"));
    assert_eq!(
        reclassify_legacy_imports(&f.store, &f.home, &f.central).expect("first"),
        2
    );
    let after_first: Vec<_> = f.store.list_skills().expect("list").into_iter().collect();

    let second = reclassify_legacy_imports(&f.store, &f.home, &f.central).expect("second");

    assert_eq!(second, 0);
    let after_second = f.store.list_skills().expect("list");
    assert_eq!(after_first.len(), after_second.len());
    for (a, b) in after_first.iter().zip(after_second.iter()) {
        assert_eq!(a.id, b.id);
        assert_eq!(a.source_type, b.source_type);
        assert_eq!(a.source_ref, b.source_ref);
        assert_eq!(a.imported_from_tool, b.imported_from_tool);
        assert_eq!(a.updated_at, b.updated_at);
    }
}
