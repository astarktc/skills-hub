use std::fs;
use std::path::{Path, PathBuf};

use crate::core::errors::{RevealPathFailure, SignalError};
use crate::core::log_reveal::{
    admit_reveal_path, log_reveal_target, path_reveal_item, resolve_reveal_item, LogRevealTarget,
};
use crate::core::project_sync;
use crate::core::skill_store::{
    ProjectRecord, ProjectSkillAssignmentRecord, SkillRecord, SkillStore, SkillTargetRecord,
};
use crate::core::sync_status::{SyncMode, SyncStatus};
use crate::core::tool_adapters::adapter_by_key;

/// macOS `open` treats any directory whose name ends in `.app` as an
/// application bundle, so the log dir (`~/Library/Logs/com.skillshub.app`)
/// is never opened itself: it is selected inside its parent.
#[test]
fn a_dot_app_suffixed_log_dir_is_revealed_from_its_parent() {
    let log_dir = Path::new("/Users/u/Library/Logs/com.skillshub.app");

    let target = log_reveal_target(log_dir, None);

    assert_eq!(
        target,
        LogRevealTarget {
            folder: PathBuf::from("/Users/u/Library/Logs"),
            selected: Some(log_dir.to_path_buf()),
        }
    );
}

/// A log dir without the bundle suffix (Linux, Windows) is opened itself.
#[test]
fn a_plain_log_dir_is_opened_itself() {
    let log_dir = Path::new("/home/u/.local/share/com.skillshub.app/logs");

    let target = log_reveal_target(log_dir, None);

    assert_eq!(
        target,
        LogRevealTarget {
            folder: log_dir.to_path_buf(),
            selected: None,
        }
    );
}

/// Once the plugin has written a log file, that file is what gets selected —
/// inside the log dir, whatever the dir is called.
#[test]
fn an_existing_log_file_is_selected_inside_the_log_dir() {
    let log_dir = Path::new("/Users/u/Library/Logs/com.skillshub.app");
    let log_file = log_dir.join("Skills Hub.log");

    let target = log_reveal_target(log_dir, Some(&log_file));

    assert_eq!(
        target,
        LogRevealTarget {
            folder: log_dir.to_path_buf(),
            selected: Some(log_file),
        }
    );
}

/// The suffix rule is about the name, not the case it was typed in.
#[test]
fn the_bundle_suffix_rule_ignores_case() {
    let log_dir = Path::new("/Users/u/Library/Logs/Com.Skillshub.APP");

    let target = log_reveal_target(log_dir, None);

    assert_eq!(target.folder, PathBuf::from("/Users/u/Library/Logs"));
    assert_eq!(target.selected.as_deref(), Some(log_dir));
}

// ── Reveal in Finder: admission (audit A3) ─────────────────────────────

const HOME: &str = "/Users/u";
const CENTRAL: &str = "/Users/u/.skillshub";

fn known() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/Users/u/.claude/skills/release-train"),
        PathBuf::from("/Users/u/code/monorepo/.pi/skills/release-train"),
    ]
}

fn admit(requested: &str) -> Option<PathBuf> {
    admit_reveal_path(requested, Path::new(HOME), Path::new(CENTRAL), &known())
}

/// A path the store records (the identity a Delete report carries) is
/// admitted as itself; so is anything inside the central repo.
#[test]
fn a_recorded_target_or_a_central_repo_path_is_admitted() {
    assert_eq!(
        admit("/Users/u/.claude/skills/release-train"),
        Some(PathBuf::from("/Users/u/.claude/skills/release-train"))
    );
    assert_eq!(
        admit("/Users/u/code/monorepo/.pi/skills/release-train/"),
        Some(PathBuf::from(
            "/Users/u/code/monorepo/.pi/skills/release-train"
        ))
    );
    assert!(admit("/Users/u/.skillshub/release-train/SKILL.md").is_some());
    assert!(admit("/Users/u/.skillshub").is_some());
}

/// Anything else is refused before the filesystem is touched: an app
/// bundle's (made-up) child, an executable's, a system file, a sibling or a
/// child of a recorded target, a sibling of the central repo.
#[test]
fn an_unknown_path_outside_every_managed_root_is_refused() {
    for requested in [
        "/Applications/Calculator.app/not-a-real-child",
        "/tmp/audit.command/not-a-real-child",
        "/etc/passwd",
        "/Users/u/.claude/skills/other-skill",
        "/Users/u/.claude/skills/release-train/SKILL.md",
        "/Users/u/.claude/skills",
        "/Users/u/.skillshub-evil/x",
        "/",
    ] {
        assert_eq!(admit(requested), None, "{requested} must be refused");
    }
}

/// The repo's path rule: a leading `~` expands against home (so does a
/// stored `~` central repo); a relative path, `~user` and an empty string
/// are refused.
#[test]
fn tilde_expands_against_home_and_a_relative_path_is_refused() {
    assert_eq!(
        admit("~/.claude/skills/release-train"),
        Some(PathBuf::from("/Users/u/.claude/skills/release-train"))
    );
    assert!(admit_reveal_path(
        "~/.skillshub/x",
        Path::new(HOME),
        Path::new("~/.skillshub"),
        &[]
    )
    .is_some());
    for requested in [
        ".claude/skills/release-train",
        "release-train",
        "~other/.claude/skills/release-train",
        "",
        "   ",
    ] {
        assert_eq!(admit(requested), None, "{requested:?} must be refused");
    }
}

/// `..` never climbs out of a managed root, even when the string starts
/// with one.
#[test]
fn a_dot_dot_component_is_refused() {
    for requested in [
        "/Users/u/.skillshub/../../../Applications/Calculator.app",
        "/Users/u/.skillshub/../.ssh",
        "/Users/u/.claude/skills/release-train/../../../../../etc",
    ] {
        assert_eq!(admit(requested), None, "{requested} must be refused");
    }
}

// ── Reveal in Finder: what gets selected (never opened) ──────────────────

/// A path still there is itself the item selected in its folder.
#[test]
fn a_present_path_is_selected_itself() {
    let path = Path::new("/Users/u/.claude/skills/release-train");
    assert_eq!(
        path_reveal_item(path, |p| p == path),
        Some(path.to_path_buf())
    );
}

/// Once it is gone, its nearest existing ancestor is **selected** (in its
/// own parent), never opened: with the real probe on a temp dir, an `.app`
/// bundle and an executable file on the way are selected like any folder.
#[cfg(unix)]
#[test]
fn a_gone_path_selects_its_nearest_ancestor_even_a_bundle_or_an_executable() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("Calculator.app");
    fs::create_dir_all(bundle.join("Contents")).unwrap();
    let script = dir.path().join("audit.command");
    fs::write(&script, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let probe = |p: &Path| p.symlink_metadata().is_ok();

    assert_eq!(
        path_reveal_item(&bundle.join("not-a-real-child"), probe),
        Some(bundle.clone())
    );
    assert_eq!(
        path_reveal_item(&script.join("not-a-real-child"), probe),
        Some(script.clone())
    );
    assert_eq!(
        path_reveal_item(&dir.path().join("gone/deeper"), probe),
        Some(dir.path().to_path_buf())
    );
}

/// The presence probe is `symlink_metadata`: a dangling link is still an
/// item to select (the real probe, on a temp dir).
#[cfg(unix)]
#[test]
fn a_dangling_link_is_selected_with_the_real_probe() {
    let dir = tempfile::tempdir().unwrap();
    let link = dir.path().join("release-train");
    std::os::unix::fs::symlink(dir.path().join("gone"), &link).unwrap();

    let item = path_reveal_item(&link, |p| p.symlink_metadata().is_ok());

    assert_eq!(item, Some(link));
}

/// Nothing on the way exists (a vanished volume): there is nothing to show.
#[test]
fn a_path_with_no_existing_ancestor_has_no_item() {
    assert_eq!(
        path_reveal_item(Path::new("/Volumes/gone/x"), |_| false),
        None
    );
}

// ── Reveal in Finder: against the store (temp dirs) ──────────────────────

fn make_store(base: &Path) -> SkillStore {
    let store = SkillStore::new(base.join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    store
}

fn seed_skill(store: &SkillStore, name: &str, central_path: &Path) -> SkillRecord {
    let skill = SkillRecord {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
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
    };
    store.upsert_skill(&skill).expect("upsert skill");
    skill
}

/// A kept (`error`) global row whose artifact is still on disk.
fn seed_kept_target(store: &SkillStore, skill: &SkillRecord, tool: &str, target: &Path) {
    fs::create_dir_all(target).unwrap();
    store
        .upsert_skill_target(&SkillTargetRecord {
            id: uuid::Uuid::new_v4().to_string(),
            skill_id: skill.id.clone(),
            tool: tool.to_string(),
            target_path: target.to_string_lossy().to_string(),
            mode: SyncMode::Copy,
            status: SyncStatus::Error,
            last_error: Some("permission denied".to_string()),
            synced_at: Some(1),
        })
        .expect("upsert target");
}

fn refusal(result: anyhow::Result<PathBuf>) -> RevealPathFailure {
    match result.expect_err("refused").downcast::<SignalError>() {
        Ok(SignalError::RevealPathFailed { reason, .. }) => reason,
        other => panic!("expected RevealPathFailed, got {other:?}"),
    }
}

/// The Delete report's kept paths — a global row and a project assignment,
/// resolved as Delete planned them — are admitted and selected; once the
/// artifact vanished, its nearest existing ancestor is.
#[test]
fn a_reported_target_is_selected_and_a_vanished_one_selects_its_ancestor() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let central = home.join(".skillshub");
    let store = make_store(tmp.path());
    let skill = seed_skill(&store, "release-train", &central.join("release-train"));
    let global = home.join(".claude/skills/release-train");
    seed_kept_target(&store, &skill, "claude_code", &global);
    let project_dir = tmp.path().join("monorepo");
    fs::create_dir_all(&project_dir).unwrap();
    let project = ProjectRecord {
        id: uuid::Uuid::new_v4().to_string(),
        path: project_dir.to_string_lossy().to_string(),
        created_at: 1,
        updated_at: 1,
    };
    store.register_project(&project).unwrap();
    store
        .add_project_skill_assignment(&ProjectSkillAssignmentRecord {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project.id.clone(),
            skill_id: skill.id.clone(),
            skill_name: skill.name.clone(),
            tool: "pi".to_string(),
            mode: SyncMode::Copy,
            status: SyncStatus::Error,
            last_error: None,
            synced_at: Some(1),
            content_hash: None,
            created_at: 1,
        })
        .unwrap();
    let assigned = project_sync::resolve_project_sync_target(
        &project_dir,
        adapter_by_key("pi").unwrap(),
        "release-train",
    );
    fs::create_dir_all(&assigned).unwrap();
    let probe = |p: &Path| p.symlink_metadata().is_ok();
    let reveal = |requested: &Path| {
        resolve_reveal_item(&store, &home, &central, &requested.to_string_lossy(), probe)
    };

    assert_eq!(reveal(&global).unwrap(), global);
    assert_eq!(reveal(&assigned).unwrap(), assigned);

    fs::remove_dir_all(&global).unwrap();
    assert_eq!(reveal(&global).unwrap(), home.join(".claude/skills"));
}

/// An existing path the store does not record — even one inside a Tool's
/// skills dir, an `.app` bundle's child, or a sibling — is refused as
/// `NotManaged`; a recorded path with nothing on the way is `Gone`.
#[cfg(unix)]
#[test]
fn an_unrecorded_path_is_refused_and_a_vanished_volume_is_gone() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let central = home.join(".skillshub");
    let store = make_store(tmp.path());
    let skill = seed_skill(&store, "release-train", &central.join("release-train"));
    let global = home.join(".claude/skills/release-train");
    seed_kept_target(&store, &skill, "claude_code", &global);
    let bundle = tmp.path().join("Calculator.app");
    fs::create_dir_all(&bundle).unwrap();
    let unrelated = home.join(".claude/skills/someone-elses");
    fs::create_dir_all(&unrelated).unwrap();
    let probe = |p: &Path| p.symlink_metadata().is_ok();

    for requested in [
        bundle.join("not-a-real-child"),
        bundle.clone(),
        unrelated,
        global.join("SKILL.md"),
        PathBuf::from("release-train"),
    ] {
        assert_eq!(
            refusal(resolve_reveal_item(
                &store,
                &home,
                &central,
                &requested.to_string_lossy(),
                probe
            )),
            RevealPathFailure::NotManaged,
            "{requested:?}"
        );
    }
    assert_eq!(
        refusal(resolve_reveal_item(
            &store,
            &home,
            &central,
            &global.to_string_lossy(),
            |_| false
        )),
        RevealPathFailure::Gone
    );
}
