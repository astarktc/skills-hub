//! Tests for `core::target_relocation` — moving global Sync targets out of a
//! Tool's former skills dir (Augment's `~/.augment/rules`, #43).
//!
//! Every case runs against a temp home / central dir / DB; installedness is
//! faked through `mark_installed_in`. Nothing touches the operator's home.

use std::fs;
use std::path::{Path, PathBuf};

use crate::core::artifact_removal::{remove_superseded_artifact_unlocked, unsync_skill_from_tool};
use crate::core::errors::SignalError;
use crate::core::installer::InstallerPaths;
use crate::core::propagation::propagate_unlocked;
use crate::core::skill_store::{SkillRecord, SkillStore, SkillTargetRecord};
use crate::core::sync_status::{SyncMode, SyncStatus};
use crate::core::target_relocation::{relocate_unlocked, RelocationSummary};
use crate::core::tool_adapters::{adapter_by_key, mark_installed_in, ToolAdapter};

const NOW: i64 = 5000;

struct Fixture {
    _dir: tempfile::TempDir,
    paths: InstallerPaths,
    store: SkillStore,
    central_path: PathBuf,
}

impl Fixture {
    fn home(&self) -> &Path {
        &self.paths.home
    }
    fn former(&self) -> PathBuf {
        self.home().join(".augment/rules/skill")
    }
    fn current(&self) -> PathBuf {
        self.home().join(".augment/skills/skill")
    }
    fn row(&self) -> Option<SkillTargetRecord> {
        self.store
            .get_skill_target("skill-1", "augment")
            .expect("read target row")
    }
    fn relocate(&self) -> RelocationSummary {
        relocate_unlocked(&self.store, self.home(), NOW).expect("relocation reads its rows")
    }
}

fn augment() -> &'static ToolAdapter {
    adapter_by_key("augment").expect("augment adapter")
}

/// One managed skill `skill` in a temp central repo, Augment installed.
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = InstallerPaths {
        home: dir.path().join("home"),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.home).expect("create home");
    let central_path = paths.central_dir.join("skill");
    fs::create_dir_all(&central_path).expect("create central skill");
    fs::write(central_path.join("SKILL.md"), "---\nname: skill\n---\n").expect("SKILL.md");
    fs::write(central_path.join("a.txt"), "v1").expect("a.txt");

    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    store
        .upsert_skill(&SkillRecord {
            id: "skill-1".to_string(),
            name: "skill".to_string(),
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
            status: "ok".to_string(),
            imported_from_tool: None,
        })
        .expect("seed skill");
    mark_installed_in(&paths.home, augment());
    Fixture {
        _dir: dir,
        paths,
        store,
        central_path,
    }
}

fn seed_row(f: &Fixture, target_path: &Path, mode: SyncMode) {
    f.store
        .upsert_skill_target(&SkillTargetRecord {
            id: "target-1".to_string(),
            skill_id: "skill-1".to_string(),
            tool: "augment".to_string(),
            target_path: target_path.to_string_lossy().to_string(),
            mode,
            status: SyncStatus::Synced,
            last_error: None,
            synced_at: Some(1),
        })
        .expect("seed target");
}

#[cfg(unix)]
fn link(source: &Path, target: &Path) {
    fs::create_dir_all(target.parent().unwrap()).expect("create link parent");
    std::os::unix::fs::symlink(source, target).expect("symlink");
}

fn copy_of_central(f: &Fixture, target: &Path) {
    fs::create_dir_all(target).expect("create copy");
    for file in ["SKILL.md", "a.txt"] {
        fs::copy(f.central_path.join(file), target.join(file)).expect("copy file");
    }
}

/// A legacy link row pre-1.2.18: `~/.augment/rules/skill -> central`.
#[cfg(unix)]
fn legacy_link_row(f: &Fixture) {
    link(&f.central_path, &f.former());
    seed_row(f, &f.former(), SyncMode::Symlink);
}

/// Why the relocation pass exists: the existing update path does not move a
/// legacy row. Propagation writes to a row's stored path and skips a link
/// entirely, so the row and its artifact stay in Augment's rules dir.
#[cfg(unix)]
#[test]
fn propagation_alone_leaves_a_legacy_link_row_in_the_former_dir() {
    let f = fixture();
    legacy_link_row(&f);

    propagate_unlocked(&f.store, &f.paths, "skill-1", NOW).expect("propagate");

    let row = f.row().expect("row kept");
    assert_eq!(Path::new(&row.target_path), f.former());
    assert!(f.former().symlink_metadata().is_ok(), "old link untouched");
    assert!(
        f.current().symlink_metadata().is_err(),
        "nothing lands in the current dir"
    );
}

#[cfg(unix)]
#[test]
fn a_legacy_link_row_moves_to_the_current_dir_and_the_old_link_is_removed() {
    let f = fixture();
    legacy_link_row(&f);

    let summary = f.relocate();

    assert_eq!(
        summary,
        RelocationSummary {
            relocated: 1,
            left_in_place: 0
        }
    );
    let row = f.row().expect("row kept");
    assert_eq!(Path::new(&row.target_path), f.current());
    assert_eq!(row.mode, SyncMode::Symlink);
    assert_eq!(row.status, SyncStatus::Synced);
    assert_eq!(row.synced_at, Some(NOW));
    assert_eq!(
        fs::read_link(f.current()).expect("current is a link"),
        f.central_path
    );
    assert!(f.former().symlink_metadata().is_err(), "old link removed");
    assert!(
        f.home().join(".augment/rules").is_dir(),
        "Augment's rules dir itself is not ours to remove"
    );
}

#[cfg(unix)]
#[test]
fn a_legacy_copy_row_is_rematerialised_from_the_central_copy() {
    let f = fixture();
    copy_of_central(&f, &f.former());
    seed_row(&f, &f.former(), SyncMode::Copy);

    assert_eq!(f.relocate().relocated, 1);

    let row = f.row().expect("row kept");
    assert_eq!(Path::new(&row.target_path), f.current());
    // Augment consumes links, so the capability-aware engine links it.
    assert_eq!(row.mode, SyncMode::Symlink);
    assert!(f.former().symlink_metadata().is_err(), "old copy removed");
}

#[cfg(unix)]
#[test]
fn relocation_is_idempotent() {
    let f = fixture();
    legacy_link_row(&f);

    assert_eq!(f.relocate().relocated, 1);
    assert_eq!(f.relocate(), RelocationSummary::default());
    assert_eq!(Path::new(&f.row().unwrap().target_path), f.current());
}

#[test]
fn a_row_already_in_the_current_dir_is_untouched() {
    let f = fixture();
    copy_of_central(&f, &f.current());
    seed_row(&f, &f.current(), SyncMode::Copy);

    assert_eq!(f.relocate(), RelocationSummary::default());
    let row = f.row().unwrap();
    assert_eq!(row.synced_at, Some(1), "row not rewritten");
    assert_eq!(row.mode, SyncMode::Copy);
}

/// Nothing to move: materialising anyway could create a skills dir for a
/// Tool that is gone. The row stays (and stays removable — see below).
#[test]
fn an_absent_former_artifact_leaves_the_row_in_place() {
    let f = fixture();
    seed_row(&f, &f.former(), SyncMode::Symlink);

    assert_eq!(
        f.relocate(),
        RelocationSummary {
            relocated: 0,
            left_in_place: 1
        }
    );
    assert_eq!(Path::new(&f.row().unwrap().target_path), f.former());
    assert!(f.current().symlink_metadata().is_err());
}

#[cfg(unix)]
#[test]
fn foreign_bytes_in_the_current_dir_keep_the_row_and_both_artifacts() {
    let f = fixture();
    legacy_link_row(&f);
    fs::create_dir_all(f.current()).unwrap();
    fs::write(f.current().join("a.txt"), "someone else's").unwrap();

    assert_eq!(f.relocate().left_in_place, 1);

    assert_eq!(Path::new(&f.row().unwrap().target_path), f.former());
    assert!(f.former().symlink_metadata().is_ok(), "old link kept");
    assert_eq!(
        fs::read_to_string(f.current().join("a.txt")).unwrap(),
        "someone else's",
        "foreign bytes are never overwritten"
    );
}

#[cfg(unix)]
#[test]
fn an_identical_copy_already_in_the_current_dir_is_adopted() {
    let f = fixture();
    legacy_link_row(&f);
    copy_of_central(&f, &f.current());

    assert_eq!(f.relocate().relocated, 1);

    assert_eq!(Path::new(&f.row().unwrap().target_path), f.current());
    assert!(f.former().symlink_metadata().is_err(), "old link removed");
}

#[test]
fn a_missing_central_copy_leaves_the_row_in_place() {
    let f = fixture();
    copy_of_central(&f, &f.former());
    seed_row(&f, &f.former(), SyncMode::Copy);
    fs::remove_dir_all(&f.central_path).unwrap();

    assert_eq!(f.relocate().left_in_place, 1);
    assert_eq!(Path::new(&f.row().unwrap().target_path), f.former());
    assert!(f.former().is_dir(), "old copy kept");
}

/// A row left in a former dir with Augment gone from the machine takes
/// removal's stored-path fallback, which is fenced by the deletion rule —
/// a former dir is inside it, so the operator can still unsync.
#[test]
fn a_row_left_in_a_former_dir_can_be_unsynced_after_the_tool_is_gone() {
    let f = fixture();
    seed_row(&f, &f.former(), SyncMode::Symlink);
    fs::remove_dir_all(f.home().join(".augment")).unwrap();

    let report = unsync_skill_from_tool(&f.store, f.home(), "skill-1", "augment")
        .expect("planning accepts a former-dir path");

    assert_eq!(report.removed_rows(), 1);
    assert!(f.row().is_none());
}

#[test]
fn the_superseded_artifact_seam_refuses_anything_outside_a_former_dir() {
    let f = fixture();
    copy_of_central(&f, &f.current());

    for path in [f.current(), f.home().join(".augment/rules/nested/skill")] {
        let err =
            remove_superseded_artifact_unlocked(f.home(), augment(), &path).expect_err("refused");
        assert!(
            matches!(
                err.downcast_ref::<SignalError>(),
                Some(SignalError::PathOutsideToolDirs { .. })
            ),
            "typed refusal for {}",
            path.display()
        );
    }
    assert!(f.current().is_dir(), "current artifact untouched");
}
