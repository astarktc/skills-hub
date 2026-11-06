//! Tests for `core::library_move` — Move Library folder: admission, copy,
//! switch, re-point and clean, through its two entry points.
//!
//! Every case runs against a temp home / central dir / cache dir / DB, with
//! Tool and Project dirs outside the central dir (`central_repo_of`).
//! Installedness is faked through `mark_installed_in`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::core::errors::{LibraryDestinationRefusal, SignalError};
use crate::core::global_sync::{sync_skills_to_tools, BatchPolicy, BatchSkill};
use crate::core::installer::{install_local_skill, InstallerPaths};
use crate::core::library_move::{
    move_library, preview_library_move, CloudSyncProvider, LeftBehindReason, LibraryMoveReport,
    RelinkSkip, RelinkStatus, SkillMoveSkip, SkillMoveStatus,
};
use crate::core::project_sync::assign_skill_to_project_tools;
use crate::core::removal_fence::central_repo_of;
use crate::core::settings::resolve_central_repo_path;
use crate::core::skill_store::{ProjectRecord, ProjectToolRecord, SkillStore};
use crate::core::sync_status::SyncStatus;
use crate::core::tool_adapters::{adapter_by_key, mark_installed_in};

const NOW: i64 = 9_000;

struct Fixture {
    dir: tempfile::TempDir,
    paths: InstallerPaths,
    store: SkillStore,
    _sources: Vec<tempfile::TempDir>,
}

impl Fixture {
    fn home(&self) -> &Path {
        &self.paths.home
    }
    fn dest(&self) -> PathBuf {
        self.dir.path().join("new-library")
    }
    fn install(&mut self, name: &str) -> String {
        let source = tempfile::tempdir().expect("source");
        fs::write(
            source.path().join("SKILL.md"),
            format!("---\nname: {name}\n---\nbody of {name}\n"),
        )
        .unwrap();
        fs::create_dir_all(source.path().join("refs")).unwrap();
        fs::write(source.path().join("refs/notes.md"), "notes").unwrap();
        let installed =
            install_local_skill(&self.paths, &self.store, source.path(), Some(name.into()))
                .expect("install");
        self._sources.push(source);
        installed.skill_id
    }
    fn central(&self, id: &str) -> PathBuf {
        PathBuf::from(
            self.store
                .get_skill_by_id(id)
                .unwrap()
                .unwrap()
                .central_path,
        )
    }
    fn sync(&self, id: &str, tools: &[&str]) {
        let skill = self.store.get_skill_by_id(id).unwrap().unwrap();
        let outcomes = sync_skills_to_tools(
            self.home(),
            &self.store,
            &self.paths.central_dir,
            &[BatchSkill {
                skill_id: skill.id,
                skill_name: skill.name,
                source_path: PathBuf::from(skill.central_path),
            }],
            &tools.iter().map(|t| t.to_string()).collect::<Vec<_>>(),
            &BatchPolicy {
                overwrite: false,
                overwrite_if_same_content: true,
                overrides: Vec::new(),
            },
            NOW,
            |_| {},
        );
        assert!(!outcomes.is_empty());
    }
    fn project(&self, id: &str, tool: &str) -> PathBuf {
        let path = self.dir.path().join(format!("proj-{id}"));
        fs::create_dir_all(&path).unwrap();
        self.store
            .register_project(&ProjectRecord {
                id: id.into(),
                path: path.to_string_lossy().into_owned(),
                created_at: 1,
                updated_at: 1,
            })
            .unwrap();
        self.store
            .add_project_tool(&ProjectToolRecord {
                id: format!("pt-{id}-{tool}"),
                project_id: id.into(),
                tool: tool.into(),
            })
            .unwrap();
        path
    }
    fn assign(&self, project_id: &str, skill_id: &str) {
        assign_skill_to_project_tools(
            &self.store,
            &self.paths.central_dir,
            project_id,
            skill_id,
            NOW,
        )
        .expect("assign");
    }
    fn run(&self) -> LibraryMoveReport {
        move_library(&self.store, &self.paths, &self.dest(), NOW + 1, |_| {}).expect("move")
    }
    fn refusal(&self, dest: &Path) -> (LibraryDestinationRefusal, Option<String>) {
        let err =
            move_library(&self.store, &self.paths, dest, NOW + 1, |_| {}).expect_err("refused");
        match err.downcast_ref::<SignalError>() {
            Some(SignalError::LibraryDestinationRefused {
                reason, conflict, ..
            }) => (*reason, conflict.clone()),
            _ => panic!("expected a destination refusal, got {err:#}"),
        }
    }
}

fn fixture(tools: &[&str]) -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("schema");
    let paths = InstallerPaths {
        home: dir.path().join("home"),
        central_dir: central_repo_of(&store),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.home).unwrap();
    for tool in tools {
        mark_installed_in(&paths.home, adapter_by_key(tool).expect("adapter"));
    }
    Fixture {
        dir,
        paths,
        store,
        _sources: Vec::new(),
    }
}

fn resolves_to(link: &Path, dir: &Path) -> bool {
    match (link.canonicalize(), dir.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Remove a directory link as itself (Windows needs `remove_dir` for one).
fn remove_link(path: &Path) {
    #[cfg(windows)]
    fs::remove_dir(path).unwrap();
    #[cfg(not(windows))]
    fs::remove_file(path).unwrap();
}

fn is_link(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
}

fn left_behind(report: &LibraryMoveReport, reason: LeftBehindReason) -> Vec<PathBuf> {
    report
        .left_behind
        .iter()
        .filter(|item| item.reason == reason)
        .map(|item| PathBuf::from(&item.path))
        .collect()
}

// ---------------------------------------------------------------------------
// Admission: each refusal is typed, and nothing is written.
// ---------------------------------------------------------------------------

/// A refused destination leaves every record, the setting and the disk as
/// they were.
fn assert_untouched(f: &Fixture, id: &str, before: &Path) {
    assert_eq!(f.central(id), before, "record unchanged");
    assert!(before.join("SKILL.md").is_file(), "old copy still there");
    assert!(
        f.store
            .get_skill_by_id(id)
            .unwrap()
            .is_some_and(|s| s.updated_at != NOW + 1),
        "record not rewritten"
    );
}

#[test]
fn refuses_a_relative_destination() {
    let mut f = fixture(&[]);
    let id = f.install("alpha");
    let before = f.central(&id);
    let (reason, _) = f.refusal(Path::new("relative/library"));
    assert_eq!(reason, LibraryDestinationRefusal::NotAbsolute);
    let (reason, _) = f.refusal(&f.dir.path().join("x/../new-library"));
    assert_eq!(reason, LibraryDestinationRefusal::NotAbsolute);
    assert_untouched(&f, &id, &before);
}

#[test]
fn refuses_a_destination_inside_or_containing_the_library() {
    let mut f = fixture(&[]);
    let id = f.install("alpha");
    let before = f.central(&id);
    let central = f.paths.central_dir.clone();

    for dest in [
        central.clone(),
        central.join("nested"),
        before.join("inside-a-copy"),
        f.dir.path().to_path_buf(),
    ] {
        let (reason, conflict) = f.refusal(&dest);
        assert_eq!(
            reason,
            LibraryDestinationRefusal::OverlapsLibrary,
            "{dest:?}"
        );
        assert!(conflict.is_some(), "{dest:?} names what it overlaps");
    }
    assert!(!central.join("nested").exists(), "nothing was created");
    assert_untouched(&f, &id, &before);
}

#[test]
fn refuses_a_destination_inside_a_harness_skills_dir() {
    let mut f = fixture(&["claude_code"]);
    let id = f.install("alpha");
    let before = f.central(&id);
    let harness_dir = f.home().join(".claude/skills");
    let (reason, conflict) = f.refusal(&harness_dir.join("library"));
    assert_eq!(reason, LibraryDestinationRefusal::InsideHarnessDir);
    assert_eq!(
        conflict
            .map(PathBuf::from)
            .and_then(|p| p.canonicalize().ok()),
        harness_dir.canonicalize().ok()
    );
    // Not installed changes nothing: a harness's dir is never a Library.
    let (reason, _) = f.refusal(&f.home().join(".codex/skills/library"));
    assert_eq!(reason, LibraryDestinationRefusal::InsideHarnessDir);
    assert_untouched(&f, &id, &before);
}

#[test]
fn refuses_a_destination_inside_a_project_skills_dir() {
    let mut f = fixture(&["claude_code"]);
    let id = f.install("alpha");
    let before = f.central(&id);
    let project = f.project("p1", "claude_code");
    let (reason, _) = f.refusal(&project.join(".claude/skills/library"));
    assert_eq!(reason, LibraryDestinationRefusal::InsideProjectDir);
    assert_untouched(&f, &id, &before);
}

#[test]
fn refuses_a_destination_inside_the_git_cache() {
    let mut f = fixture(&[]);
    let id = f.install("alpha");
    let before = f.central(&id);
    let cache_root = f.paths.cache_dir.join("skills-hub-git-cache");
    fs::create_dir_all(&cache_root).unwrap();
    let (reason, _) = f.refusal(&cache_root.join("library"));
    assert_eq!(reason, LibraryDestinationRefusal::InsideGitCache);
    assert_untouched(&f, &id, &before);
}

#[test]
fn refuses_a_destination_that_is_not_empty() {
    let mut f = fixture(&[]);
    let id = f.install("alpha");
    let before = f.central(&id);
    let dest = f.dest();
    fs::create_dir_all(&dest).unwrap();
    fs::write(dest.join("taxes.pdf"), "mine").unwrap();
    let (reason, _) = f.refusal(&dest);
    assert_eq!(reason, LibraryDestinationRefusal::NotEmpty);
    assert_eq!(fs::read_to_string(dest.join("taxes.pdf")).unwrap(), "mine");
    assert_eq!(fs::read_dir(&dest).unwrap().count(), 1, "nothing was added");
    // A file is not an empty folder either.
    let file = f.dir.path().join("a-file");
    fs::write(&file, "x").unwrap();
    assert_eq!(f.refusal(&file).0, LibraryDestinationRefusal::NotEmpty);
    assert_untouched(&f, &id, &before);
}

#[cfg(unix)]
#[test]
fn refuses_a_destination_that_is_not_writable() {
    use std::os::unix::fs::PermissionsExt;
    let mut f = fixture(&[]);
    let id = f.install("alpha");
    let before = f.central(&id);
    let parent = f.dir.path().join("locked");
    fs::create_dir_all(&parent).unwrap();
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o555)).unwrap();
    if fs::create_dir(parent.join("probe")).is_ok() {
        return; // running as root: permissions are not enforced
    }
    let (reason, _) = f.refusal(&parent.join("library"));
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(reason, LibraryDestinationRefusal::NotWritable);
    assert_untouched(&f, &id, &before);
}

// ---------------------------------------------------------------------------
// Preview: the confirmation's facts, read-only.
// ---------------------------------------------------------------------------

#[test]
fn preview_counts_the_skills_and_writes_nothing() {
    let mut f = fixture(&[]);
    f.install("alpha");
    let gone = f.install("beta");
    fs::remove_dir_all(f.central(&gone)).unwrap();

    let preview = preview_library_move(&f.store, &f.paths, &f.dest()).expect("preview");

    assert_eq!(preview.skills, 1);
    assert_eq!(preview.missing, 1);
    assert!(preview.cloud_sync.is_none());
    assert!(!f.dest().exists(), "a preview creates nothing");
}

#[test]
fn preview_refuses_like_the_move() {
    let mut f = fixture(&[]);
    f.install("alpha");
    let err = preview_library_move(&f.store, &f.paths, &f.paths.central_dir.join("x"))
        .expect_err("refused");
    assert!(matches!(
        err.downcast_ref::<SignalError>(),
        Some(SignalError::LibraryDestinationRefused {
            reason: LibraryDestinationRefusal::OverlapsLibrary,
            ..
        })
    ));
}

#[test]
fn preview_classifies_cloud_synced_destinations() {
    let f = fixture(&[]);
    let home = f.home().to_path_buf();
    let cases = [
        (
            home.join("Library/Mobile Documents/com~apple~CloudDocs/Skills"),
            CloudSyncProvider::IcloudDrive,
        ),
        (
            home.join("Library/CloudStorage/GoogleDrive-me@x.com/My Drive/Skills"),
            CloudSyncProvider::CloudStorage,
        ),
        (
            home.join("OneDrive - Contoso/Skills"),
            CloudSyncProvider::OneDrive,
        ),
    ];
    for (dest, provider) in cases {
        let preview = preview_library_move(&f.store, &f.paths, &dest).expect("preview");
        let cloud = preview
            .cloud_sync
            .unwrap_or_else(|| panic!("{dest:?} is cloud-synced"));
        assert_eq!(cloud.provider, provider, "{dest:?}");
    }
    let plain = preview_library_move(&f.store, &f.paths, &home.join("Skills")).unwrap();
    assert!(plain.cloud_sync.is_none());
}

// ---------------------------------------------------------------------------
// The move
// ---------------------------------------------------------------------------

#[test]
fn moves_copies_switches_relinks_and_cleans() {
    let mut f = fixture(&["claude_code", "cursor"]);
    let alpha = f.install("alpha");
    let beta = f.install("beta");
    f.sync(&alpha, &["claude_code", "cursor"]);
    f.sync(&beta, &["claude_code"]);
    let project = f.project("p1", "claude_code");
    f.assign("p1", &alpha);
    let old_alpha = f.central(&alpha);
    let old_root = f.paths.central_dir.clone();

    let report = f.run();

    let dest = f.dest();
    for id in [&alpha, &beta] {
        let new = f.central(id);
        assert!(new.starts_with(&dest), "{new:?} is in the new Library");
        assert!(new.join("SKILL.md").is_file());
        assert!(new.join("refs/notes.md").is_file());
    }
    assert_eq!(
        resolve_central_repo_path(&f.store, f.home()).unwrap(),
        dest,
        "the setting switched"
    );
    // Every owned link resolves into the new Library.
    for (link, id) in [
        (f.home().join(".claude/skills/alpha"), &alpha),
        (f.home().join(".cursor/skills/alpha"), &alpha),
        (f.home().join(".claude/skills/beta"), &beta),
        (project.join(".claude/skills/alpha"), &alpha),
    ] {
        assert!(is_link(&link), "{link:?} is still a link");
        assert!(resolves_to(&link, &f.central(id)), "{link:?} re-pointed");
    }
    assert!(report
        .deployments
        .iter()
        .all(|d| matches!(d.status, RelinkStatus::Relinked { .. })));
    assert_eq!(report.deployments.len(), 4);
    assert!(report
        .skills
        .iter()
        .all(|s| matches!(s.status, SkillMoveStatus::Moved { .. })));
    // Rows stay synced and owned.
    let row = f
        .store
        .get_skill_target(&alpha, "claude_code")
        .unwrap()
        .unwrap();
    assert_eq!(row.status, SyncStatus::Synced);
    assert!(row.owns_artifact());
    // The old copies are gone; the old root stays and is named.
    assert!(!old_alpha.exists());
    assert!(old_root.is_dir());
    assert_eq!(
        left_behind(&report, LeftBehindReason::OldLibrary),
        vec![old_root]
    );
    assert_eq!(report.to, dest.to_string_lossy());
}

#[test]
fn a_copy_mode_deployment_is_unchanged() {
    let mut f = fixture(&["claude_code"]);
    let alpha = f.install("alpha");
    f.sync(&alpha, &["claude_code"]);
    let target = f.home().join(".claude/skills/alpha");
    remove_link(&target);
    crate::core::sync_engine::copy_dir_recursive(&f.central(&alpha), &target).unwrap();
    fs::write(target.join("SKILL.md"), "the harness's copy").unwrap();

    let report = f.run();

    assert!(matches!(
        report.deployments[0].status,
        RelinkStatus::Unchanged { .. }
    ));
    assert!(!is_link(&target));
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "the harness's copy"
    );
}

#[test]
fn an_owned_link_pointing_elsewhere_is_not_ours_and_untouched() {
    let mut f = fixture(&["claude_code"]);
    let alpha = f.install("alpha");
    f.sync(&alpha, &["claude_code"]);
    let elsewhere = f.dir.path().join("elsewhere");
    fs::create_dir_all(&elsewhere).unwrap();
    let target = f.home().join(".claude/skills/alpha");
    remove_link(&target);
    #[cfg(unix)]
    std::os::unix::fs::symlink(&elsewhere, &target).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&elsewhere, &target).unwrap();

    let report = f.run();

    assert!(matches!(
        report.deployments[0].status,
        RelinkStatus::Skipped {
            reason: RelinkSkip::NotOurs
        }
    ));
    assert!(resolves_to(&target, &elsewhere), "untouched");
    assert!(elsewhere.is_dir());
}

#[test]
fn a_missing_project_folder_is_reported_and_left_as_is() {
    let mut f = fixture(&["claude_code"]);
    let alpha = f.install("alpha");
    let project = f.project("p1", "claude_code");
    f.assign("p1", &alpha);
    fs::remove_dir_all(&project).unwrap();

    let report = f.run();

    let item = &report.deployments[0];
    assert!(matches!(
        item.status,
        RelinkStatus::Skipped {
            reason: RelinkSkip::FolderMissing
        }
    ));
    assert!(!project.exists(), "the project root is never recreated");
    let row = f
        .store
        .get_project_skill_assignment("p1", &alpha, "claude_code")
        .unwrap()
        .unwrap();
    assert_eq!(row.status, SyncStatus::Synced, "row left as is");
}

#[test]
fn an_unlocatable_skill_is_skipped_and_keeps_its_record() {
    let mut f = fixture(&["claude_code"]);
    let alpha = f.install("alpha");
    let gone = f.install("gone");
    let gone_path = f.central(&gone);
    fs::remove_dir_all(&gone_path).unwrap();

    let report = f.run();

    let skipped = report.skills.iter().find(|s| s.skill_id == gone).unwrap();
    assert!(matches!(
        skipped.status,
        SkillMoveStatus::Skipped {
            reason: SkillMoveSkip::Missing
        }
    ));
    assert_eq!(f.central(&gone), gone_path, "record keeps its old path");
    assert!(f.central(&alpha).starts_with(f.dest()));
}

#[test]
fn unrecorded_content_and_scratch_stay_and_are_not_copied() {
    let mut f = fixture(&[]);
    let alpha = f.install("alpha");
    let root = f.paths.central_dir.clone();
    fs::create_dir_all(root.join("my-notes")).unwrap();
    fs::write(root.join("my-notes/todo.md"), "keep me").unwrap();
    fs::write(root.join("README.txt"), "mine").unwrap();
    fs::create_dir_all(root.join(".explore-cache/preview")).unwrap();
    fs::create_dir_all(root.join(".skills-hub-staging-abc")).unwrap();

    let report = f.run();

    let dest = f.dest();
    assert!(!dest.join(".explore-cache").exists());
    assert!(!dest.join(".skills-hub-staging-abc").exists());
    assert!(
        !dest.join("my-notes").exists(),
        "unrecorded content is not copied"
    );
    // Nothing but the old copy was removed.
    assert_eq!(
        fs::read_to_string(root.join("my-notes/todo.md")).unwrap(),
        "keep me"
    );
    assert_eq!(fs::read_to_string(root.join("README.txt")).unwrap(), "mine");
    assert!(root.join(".explore-cache/preview").is_dir());
    assert!(root.join(".skills-hub-staging-abc").is_dir());
    assert!(!root.join("alpha").exists());
    let mut unrecorded = left_behind(&report, LeftBehindReason::Unrecorded);
    unrecorded.sort();
    assert_eq!(
        unrecorded,
        vec![root.join("README.txt"), root.join("my-notes")]
    );
    assert!(f.central(&alpha).starts_with(&dest));
}

/// An old copy that is itself a link (a hand-made Library) is removed as the
/// link: the folder it points at is the operator's, and survives.
#[test]
fn the_clean_removes_a_linked_old_copy_as_itself_only() {
    let mut f = fixture(&[]);
    let alpha = f.install("alpha");
    let old = f.central(&alpha);
    let elsewhere = f.dir.path().join("operator-folder");
    fs::rename(&old, &elsewhere).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&elsewhere, &old).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&elsewhere, &old).unwrap();

    let report = f.run();

    assert!(old.symlink_metadata().is_err(), "the link is gone");
    assert!(elsewhere.join("SKILL.md").is_file(), "its target survives");
    assert!(f.central(&alpha).join("SKILL.md").is_file());
    assert!(left_behind(&report, LeftBehindReason::RemovalFailed).is_empty());
}

/// A record that names the Library folder itself as a copy (a corrupt
/// layout) moves its bytes, but the old root is never removed.
#[test]
fn the_old_root_is_never_removed_even_when_a_record_names_it() {
    let mut f = fixture(&[]);
    let alpha = f.install("alpha");
    let root = f.paths.central_dir.clone();
    fs::write(root.join("unrecorded.txt"), "keep").unwrap();
    let mut record = f.store.get_skill_by_id(&alpha).unwrap().unwrap();
    record.central_path = root.to_string_lossy().into_owned();
    f.store.upsert_skill(&record).unwrap();

    let report = f.run();

    assert_eq!(
        fs::read_to_string(root.join("unrecorded.txt")).unwrap(),
        "keep"
    );
    assert!(root.join("alpha/SKILL.md").is_file());
    assert_eq!(
        left_behind(&report, LeftBehindReason::RemovalFailed),
        vec![root.clone()]
    );
}

#[test]
fn a_disabled_skill_is_moved_and_its_rows_skipped() {
    let mut f = fixture(&["claude_code"]);
    let alpha = f.install("alpha");
    f.sync(&alpha, &["claude_code"]);
    crate::core::skill_disable::disable_targets(
        &f.store,
        &f.paths.central_dir,
        f.home(),
        &crate::core::skill_disable::DisableScope::Skill {
            skill_id: alpha.clone(),
        },
        NOW,
    )
    .expect("disable");

    let report = f.run();

    assert!(matches!(
        report.deployments[0].status,
        RelinkStatus::Skipped {
            reason: RelinkSkip::Disabled
        }
    ));
    assert!(
        !f.home().join(".claude/skills/alpha").exists(),
        "not redeployed"
    );
    assert!(f.central(&alpha).starts_with(f.dest()));
}

/// A failed copy leaves the Library exactly as it was: staged copies
/// removed, records and the setting unchanged.
#[cfg(unix)]
#[test]
fn a_copy_failure_changes_nothing() {
    use std::os::unix::fs::PermissionsExt;
    let mut f = fixture(&["claude_code"]);
    let alpha = f.install("alpha");
    let beta = f.install("beta");
    f.sync(&alpha, &["claude_code"]);
    let unreadable = f.central(&beta).join("refs/notes.md");
    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read(&unreadable).is_ok() {
        return; // running as root
    }
    let before = [
        (alpha.clone(), f.central(&alpha)),
        (beta.clone(), f.central(&beta)),
    ];

    let err = move_library(&f.store, &f.paths, &f.dest(), NOW + 1, |_| {}).expect_err("fails");
    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o644)).unwrap();

    assert!(matches!(
        err.downcast_ref::<SignalError>(),
        Some(SignalError::LibraryCopyFailed { name, .. }) if name == "beta"
    ));
    for (id, path) in &before {
        assert_eq!(&f.central(id), path);
        assert!(path.join("SKILL.md").is_file());
    }
    assert_eq!(
        resolve_central_repo_path(&f.store, f.home()).unwrap(),
        f.home().join(".skillshub"),
        "the setting is unchanged"
    );
    assert_eq!(
        fs::read_dir(f.dest()).map(|d| d.count()).unwrap_or(0),
        0,
        "the destination is clean"
    );
    assert!(resolves_to(
        &f.home().join(".claude/skills/alpha"),
        &before[0].1
    ));
}

/// A link that could not be re-pointed is a failure of its own, and the old
/// copy it still reaches is kept — a harness never loses its skill.
#[cfg(unix)]
#[test]
fn a_failed_relink_keeps_the_old_copy_it_reaches() {
    use std::os::unix::fs::PermissionsExt;
    let mut f = fixture(&["claude_code"]);
    let alpha = f.install("alpha");
    f.sync(&alpha, &["claude_code"]);
    let old = f.central(&alpha);
    let harness_dir = f.home().join(".claude/skills");
    fs::set_permissions(&harness_dir, fs::Permissions::from_mode(0o555)).unwrap();
    if fs::create_dir(harness_dir.join("probe")).is_ok() {
        fs::remove_dir(harness_dir.join("probe")).unwrap();
        fs::set_permissions(&harness_dir, fs::Permissions::from_mode(0o755)).unwrap();
        return; // running as root
    }

    let report = f.run();
    fs::set_permissions(&harness_dir, fs::Permissions::from_mode(0o755)).unwrap();

    assert!(matches!(
        report.deployments[0].status,
        RelinkStatus::Failed { .. }
    ));
    assert!(old.join("SKILL.md").is_file(), "the old copy stays");
    assert_eq!(
        left_behind(&report, LeftBehindReason::StillLinked),
        vec![old.clone()]
    );
    assert!(resolves_to(&harness_dir.join("alpha"), &old));
    let row = f
        .store
        .get_skill_target(&alpha, "claude_code")
        .unwrap()
        .unwrap();
    assert_eq!(row.status, SyncStatus::Error);
}

/// The move is one Sync-target mutation: it waits for one in flight.
#[test]
fn the_move_is_serialized_with_other_mutations() {
    use std::sync::mpsc;
    use std::time::{Duration, Instant};
    let mut f = fixture(&[]);
    f.install("alpha");
    let (held, wait) = mpsc::channel();
    let released_at = std::thread::spawn(move || {
        crate::core::mutation_guard::serialized(|| {
            held.send(()).unwrap();
            std::thread::sleep(Duration::from_millis(200));
            Instant::now()
        })
    });
    wait.recv().unwrap();
    f.run();
    let finished = Instant::now();
    assert!(finished >= released_at.join().unwrap());
}

// ---------------------------------------------------------------------------
// Round-2 regressions (review `.scratch/reviews/round20-t03-move-library-review.md`)
// ---------------------------------------------------------------------------

/// Whether the temp volume folds case (APFS / NTFS default): `a` and `A`
/// name one entry.
fn case_insensitive_volume(dir: &Path) -> bool {
    let probe = dir.join("case-probe");
    fs::write(&probe, "x").unwrap();
    let folded = dir.join("CASE-PROBE").exists();
    fs::remove_file(&probe).unwrap();
    folded
}

/// Review #1: two recorded copies whose folder names differ only by case
/// (a split Library) each land in a folder of their own — on a
/// case-insensitive volume the second never merges into the first.
#[test]
fn case_colliding_copy_names_preserve_both_skills() {
    let mut f = fixture(&[]);
    let alpha = f.install("alpha");
    let beta = f.install("beta");
    // Split Library: beta's copy lives elsewhere, in a folder named ALPHA.
    let split = f.dir.path().join("split");
    fs::create_dir_all(&split).unwrap();
    let beta_old = split.join("ALPHA");
    fs::rename(f.central(&beta), &beta_old).unwrap();
    let mut record = f.store.get_skill_by_id(&beta).unwrap().unwrap();
    record.central_path = beta_old.to_string_lossy().into_owned();
    f.store.upsert_skill(&record).unwrap();

    f.run();

    let (alpha_new, beta_new) = (f.central(&alpha), f.central(&beta));
    assert!(alpha_new.starts_with(f.dest()) && beta_new.starts_with(f.dest()));
    assert_eq!(
        fs::read_to_string(alpha_new.join("SKILL.md")).unwrap(),
        "---\nname: alpha\n---\nbody of alpha\n"
    );
    assert_eq!(
        fs::read_to_string(beta_new.join("SKILL.md")).unwrap(),
        "---\nname: beta\n---\nbody of beta\n",
        "beta keeps its own bytes"
    );
    assert_ne!(
        alpha_new.canonicalize().unwrap(),
        beta_new.canonicalize().unwrap(),
        "two folders, not one"
    );
}

/// Review #2: the move is a relocation, not a deploy — `.git` and dotfiles
/// travel with the skill, byte for byte, before the original is removed.
#[test]
fn the_move_keeps_git_history_and_dotfiles() {
    let mut f = fixture(&[]);
    let alpha = f.install("alpha");
    let old = f.central(&alpha);
    fs::create_dir_all(old.join(".git/objects/ab")).unwrap();
    fs::write(old.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    fs::write(old.join(".git/objects/ab/cdef"), [0u8, 1, 2, 255]).unwrap();
    fs::write(old.join(".env.example"), "KEY=").unwrap();

    f.run();

    let new = f.central(&alpha);
    assert!(!old.exists(), "the original was removed");
    assert_eq!(
        fs::read_to_string(new.join(".git/HEAD")).unwrap(),
        "ref: refs/heads/main\n",
        "the move must not delete the only git history"
    );
    assert_eq!(
        fs::read(new.join(".git/objects/ab/cdef")).unwrap(),
        vec![0u8, 1, 2, 255]
    );
    assert_eq!(fs::read_to_string(new.join(".env.example")).unwrap(), "KEY=");
}

/// Review #2: a link inside the skill is recreated as a link with its text
/// verbatim — never followed, never dropped.
#[cfg(unix)]
#[test]
fn the_move_keeps_internal_links_as_links() {
    use std::os::unix::fs::symlink;
    let mut f = fixture(&[]);
    let alpha = f.install("alpha");
    let old = f.central(&alpha);
    symlink("refs/notes.md", old.join("notes-link.md")).unwrap();
    symlink("refs", old.join("refs-alias")).unwrap();
    symlink("/nonexistent/outside", old.join("dangling")).unwrap();

    f.run();

    let new = f.central(&alpha);
    for (name, text) in [
        ("notes-link.md", "refs/notes.md"),
        ("refs-alias", "refs"),
        ("dangling", "/nonexistent/outside"),
    ] {
        let link = new.join(name);
        assert!(is_link(&link), "{name} is still a link");
        assert_eq!(fs::read_link(&link).unwrap(), PathBuf::from(text));
    }
    assert_eq!(
        fs::read_to_string(new.join("notes-link.md")).unwrap(),
        "notes"
    );
    assert!(!old.exists());
}

/// What a skill the move kept in place says about it.
fn kept_reason(report: &LibraryMoveReport, id: &str) -> crate::core::library_move::SkillKept {
    match &report.skills.iter().find(|s| s.skill_id == id).unwrap().status {
        SkillMoveStatus::Kept { reason } => reason.clone(),
        other => panic!("expected {id} kept, got {other:?}"),
    }
}

/// Review #2: a skill holding something the move cannot relocate faithfully
/// stays where it is — record, bytes and Deployments untouched — and the
/// report says what and why.
#[cfg(unix)]
#[test]
fn a_skill_that_cannot_be_relocated_faithfully_stays_where_it_is() {
    use crate::core::library_move::{SkillKept, UnrelocatableCause};
    use std::os::unix::fs::symlink;
    let mut f = fixture(&["claude_code"]);
    let into_own = f.install("into-own");
    let escapes = f.install("escapes");
    let special = f.install("special");
    let fine = f.install("fine");
    f.sync(&into_own, &["claude_code"]);
    let into_own_old = f.central(&into_own);
    symlink(into_own_old.join("refs"), into_own_old.join("abs-refs")).unwrap();
    let escapes_old = f.central(&escapes);
    symlink("../fine/refs", escapes_old.join("sibling")).unwrap();
    let special_old = f.central(&special);
    let made = std::process::Command::new("mkfifo")
        .arg(special_old.join("pipe"))
        .status()
        .expect("mkfifo");
    assert!(made.success());

    let report = f.run();

    let cases = [
        (&into_own, &into_own_old, UnrelocatableCause::LinkIntoOldLibrary, "abs-refs"),
        (&escapes, &escapes_old, UnrelocatableCause::LinkEscapes, "sibling"),
        (&special, &special_old, UnrelocatableCause::SpecialFile, "pipe"),
    ];
    for (id, old, expected, entry_name) in cases {
        match kept_reason(&report, id) {
            SkillKept::Unrelocatable { entry, cause } => {
                assert_eq!(cause, expected, "{entry_name}");
                assert_eq!(PathBuf::from(entry), old.join(entry_name));
            }
            other => panic!("expected unrelocatable, got {other:?}"),
        }
        assert_eq!(&f.central(id), old, "record keeps its path");
        assert!(old.join("SKILL.md").is_file(), "original kept");
    }
    let link = f.home().join(".claude/skills/into-own");
    assert!(resolves_to(&link, &into_own_old), "its Deployment untouched");
    assert!(report.deployments.iter().all(|d| d.skill_id != into_own));
    assert!(f.central(&fine).starts_with(f.dest()), "the rest moved");
}

/// Make `link` a directory link to `target`.
fn link_dir(target: &Path, link: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(target, link).unwrap();
}

/// Review #3 (the reviewer's probe): alpha records `central/alpha`, beta
/// records the same folder through an alias of the Library. Beta's harness
/// link cannot be re-pointed. Retention used to be decided per skill id
/// while deletion acted on the physical folder, so alpha's clean deleted the
/// copy beta's link still reached. Two records of one physical copy are now
/// both kept, and nothing is removed under a link.
#[cfg(unix)]
#[test]
fn other_skill_alias_retains_still_linked_old_copy() {
    use crate::core::library_move::SkillKept;
    use std::os::unix::fs::PermissionsExt;
    let mut f = fixture(&["claude_code", "cursor"]);
    let alpha = f.install("alpha");
    let beta = f.install("beta");
    let shared = f.central(&alpha);
    fs::remove_dir_all(f.central(&beta)).unwrap();
    let alias = f.dir.path().join("alias-to-central");
    link_dir(&f.paths.central_dir, &alias);
    let mut record = f.store.get_skill_by_id(&beta).unwrap().unwrap();
    record.central_path = alias.join("alpha").to_string_lossy().into_owned();
    f.store.upsert_skill(&record).unwrap();
    f.sync(&alpha, &["claude_code"]);
    f.sync(&beta, &["cursor"]);
    let beta_link = f.home().join(".cursor/skills/beta");
    assert!(resolves_to(&beta_link, &shared));
    let cursor_dir = f.home().join(".cursor/skills");
    fs::set_permissions(&cursor_dir, fs::Permissions::from_mode(0o555)).unwrap();
    if fs::create_dir(cursor_dir.join("probe")).is_ok() {
        fs::remove_dir(cursor_dir.join("probe")).unwrap();
        fs::set_permissions(&cursor_dir, fs::Permissions::from_mode(0o755)).unwrap();
        return; // running as root
    }

    let report = f.run();
    fs::set_permissions(&cursor_dir, fs::Permissions::from_mode(0o755)).unwrap();

    assert!(shared.join("SKILL.md").is_file(), "the shared copy stays");
    assert!(resolves_to(&beta_link, &shared), "beta's link never dangles");
    assert!(resolves_to(&f.home().join(".claude/skills/alpha"), &shared));
    assert!(matches!(
        kept_reason(&report, &alpha),
        SkillKept::SharedCopy { with_skill_id } if with_skill_id == beta
    ));
    assert!(matches!(
        kept_reason(&report, &beta),
        SkillKept::SharedCopy { with_skill_id } if with_skill_id == alpha
    ));
    assert_eq!(f.central(&alpha), shared, "records keep their paths");
    assert_eq!(f.central(&beta), alias.join("alpha"));
}

/// Review #3: a physical old copy is removed only when no Deployment of
/// *any* skill reaches it — here another skill's owned link that the
/// operator pointed at alpha's copy.
#[test]
fn an_old_copy_another_skills_deployment_reaches_is_kept() {
    let mut f = fixture(&["claude_code"]);
    let alpha = f.install("alpha");
    let gamma = f.install("gamma");
    f.sync(&gamma, &["claude_code"]);
    let alpha_old = f.central(&alpha);
    let gamma_link = f.home().join(".claude/skills/gamma");
    remove_link(&gamma_link);
    link_dir(&alpha_old, &gamma_link);

    let report = f.run();

    assert!(f.central(&alpha).starts_with(f.dest()), "alpha moved");
    assert!(alpha_old.join("SKILL.md").is_file(), "its old copy stays");
    assert!(resolves_to(&gamma_link, &alpha_old), "gamma's link never dangles");
    let kept: Vec<_> = report
        .left_behind
        .iter()
        .filter(|item| item.reason == LeftBehindReason::StillLinked)
        .map(|item| (PathBuf::from(&item.path), item.skill_id.clone()))
        .collect();
    assert_eq!(kept, vec![(alpha_old, Some(alpha))]);
}

/// A move interrupted — as a crash would — at the first progress step of
/// `phase` numbered `at`. Answers whether it was interrupted.
fn crash_move(f: &Fixture, dest: &Path, phase: crate::core::library_move::LibraryMovePhase, at: u32) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        move_library(&f.store, &f.paths, dest, NOW + 1, |step| {
            if step.phase == phase && step.done == at {
                panic!("simulated crash at {phase:?} {at}");
            }
        })
    }))
    .is_err()
}

/// The paths the command seam resolves after a switch: the configured
/// Library is the setting's value now.
fn paths_now(f: &Fixture) -> InstallerPaths {
    InstallerPaths {
        home: f.paths.home.clone(),
        central_dir: resolve_central_repo_path(&f.store, f.home()).unwrap(),
        cache_dir: f.paths.cache_dir.clone(),
    }
}

/// A library of two skills, each deployed by link to Claude Code and one to
/// a project: the shape every crash test interrupts.
fn crash_fixture() -> (Fixture, Vec<(String, PathBuf, PathBuf)>, PathBuf) {
    let mut f = fixture(&["claude_code"]);
    let alpha = f.install("alpha");
    let beta = f.install("beta");
    f.sync(&alpha, &["claude_code"]);
    f.sync(&beta, &["claude_code"]);
    let project = f.project("p1", "claude_code");
    f.assign("p1", &alpha);
    let skills = [alpha, beta]
        .into_iter()
        .map(|id| {
            let old = f.central(&id);
            let name = f.store.get_skill_by_id(&id).unwrap().unwrap().name;
            let link = f.home().join(".claude/skills").join(name);
            (id, old, link)
        })
        .collect();
    (f, skills, project)
}

/// Review #4 (the reviewer's probe): a crash after the switch used to lose
/// which old copies the links still reach, so a move to a third folder read
/// them as `not_ours` and harnesses kept reading stale bytes. The next move
/// first finishes the interrupted one.
#[test]
fn crash_after_switch_is_recovered_by_the_next_move() {
    use crate::core::library_move::LibraryMovePhase;
    let (f, skills, project) = crash_fixture();
    assert!(crash_move(&f, &f.dest(), LibraryMovePhase::Relinking, 0));
    let third = f.dir.path().join("third-library");

    let report = move_library(&f.store, &paths_now(&f), &third, NOW + 2, |_| {}).expect("move");

    assert!(report.deployments.iter().all(|d| matches!(d.status, RelinkStatus::Relinked { .. })));
    for (id, old, link) in &skills {
        let now = f.central(id);
        assert!(now.starts_with(&third), "{id} is in the third Library");
        assert!(resolves_to(link, &now), "{link:?} follows the Library");
        assert!(!old.exists(), "the first old copy was cleaned");
    }
    assert!(resolves_to(&project.join(".claude/skills/alpha"), &f.central(&skills[0].0)));
}

/// Review #4: a crash at any boundary after the switch is finished by the
/// startup resume — every link follows the Library, every old copy is
/// cleaned, nothing reads as `not_ours` — and the journal is dropped.
#[test]
fn a_crash_after_the_switch_is_finished_by_the_resume_at_every_boundary() {
    use crate::core::library_move::{resume_library_move, test_seams, LibraryMovePhase};
    let boundaries: [(&str, Option<(LibraryMovePhase, u32)>); 5] = [
        ("before any re-point", Some((LibraryMovePhase::Relinking, 0))),
        ("mid re-point", Some((LibraryMovePhase::Relinking, 1))),
        ("before any clean", Some((LibraryMovePhase::Cleaning, 0))),
        ("mid clean", Some((LibraryMovePhase::Cleaning, 1))),
        ("before the journal is dropped", None),
    ];
    for (boundary, at) in boundaries {
        let (f, skills, project) = crash_fixture();
        let crashed = match at {
            Some((phase, done)) => crash_move(&f, &f.dest(), phase, done),
            None => {
                test_seams::crash_before_journal_clear();
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f.run())).is_err()
            }
        };
        assert!(crashed, "{boundary}: interrupted");
        for (id, _, _) in &skills {
            assert!(f.central(id).starts_with(f.dest()), "{boundary}: switched");
        }

        let report = resume_library_move(&f.store, NOW + 2)
            .expect("resume")
            .unwrap_or_else(|| panic!("{boundary}: a move is pending"));

        for d in &report.deployments {
            assert!(
                matches!(d.status, RelinkStatus::Relinked { .. }),
                "{boundary}: {:?} {:?}",
                d.path,
                d.status
            );
        }
        for (id, old, link) in &skills {
            assert!(resolves_to(link, &f.central(id)), "{boundary}: {link:?} re-pointed");
            assert!(!old.exists(), "{boundary}: {old:?} cleaned");
        }
        assert!(resolves_to(
            &project.join(".claude/skills/alpha"),
            &f.central(&skills[0].0)
        ));
        assert!(
            resume_library_move(&f.store, NOW + 3).unwrap().is_none(),
            "{boundary}: the journal was dropped"
        );
    }
}

/// A crash before the switch leaves the Library as it was and nothing to
/// resume: at most orphan copies in the destination.
#[test]
fn a_crash_before_the_switch_leaves_the_library_as_it_was() {
    use crate::core::library_move::{resume_library_move, LibraryMovePhase};
    let (f, skills, _) = crash_fixture();
    let setting = resolve_central_repo_path(&f.store, f.home()).unwrap();
    assert!(crash_move(&f, &f.dest(), LibraryMovePhase::Copying, 1));

    assert!(resume_library_move(&f.store, NOW + 2).unwrap().is_none());
    for (id, old, link) in &skills {
        assert_eq!(&f.central(id), old);
        assert!(resolves_to(link, old));
    }
    assert_eq!(resolve_central_repo_path(&f.store, f.home()).unwrap(), setting);
}
