//! Tests for `core::target_relocation` — moving global Sync targets out of a
//! Tool's former skills dir (Augment's `~/.augment/rules`, #43).
//!
//! Every case runs against a temp home / central dir / DB; installedness is
//! faked through `mark_installed_in`. Nothing touches the operator's home.

use std::fs;
use std::path::{Path, PathBuf};

use crate::core::artifact_removal::remove_superseded_artifact_unlocked;
use crate::core::errors::SignalError;
use crate::core::installer::InstallerPaths;
use crate::core::propagation::propagate_unlocked;
use crate::core::skill_disable::{disable_targets, DisableScope};
use crate::core::skill_store::{SkillRecord, SkillStore, SkillTargetRecord, TargetTransition};
use crate::core::sync_status::{SyncMode, SyncStatus};
use crate::core::target_relocation::{relocate_unlocked, RelocationSummary};
use crate::core::tool_adapters::{adapter_by_key, mark_installed_in, GlobalLayout, ToolAdapter};

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
        relocate_unlocked(&self.store, self.home(), GlobalLayout::OWN_DIRS, NOW)
            .expect("relocation reads its rows")
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
            left_in_place: 0,
            incomplete: Vec::new(),
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
            left_in_place: 1,
            incomplete: Vec::new(),
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
/// a former dir is inside it, so the operator can still disable the pair.
#[test]
fn a_row_left_in_a_former_dir_can_be_disabled_after_the_tool_is_gone() {
    let f = fixture();
    seed_row(&f, &f.former(), SyncMode::Symlink);
    fs::remove_dir_all(f.home().join(".augment")).unwrap();

    let scope = DisableScope::SkillTool {
        skill_id: "skill-1".into(),
        tool: "augment".into(),
    };
    let report =
        disable_targets(&f.store, f.home(), &scope, 1).expect("planning accepts a former-dir path");

    assert_eq!(report.removed_rows(), 1);
    assert_eq!(f.row().expect("row kept").status, SyncStatus::Disabled);
}

/// A Disabled row has no artifact to move and must not gain one.
#[test]
fn relocation_leaves_a_disabled_row_alone() {
    let f = fixture();
    seed_row(&f, &f.former(), SyncMode::Symlink);
    f.store
        .transition_skill_target(&f.row().unwrap().id, TargetTransition::Disabled)
        .unwrap();

    assert_eq!(f.relocate(), RelocationSummary::default());
    assert!(!f.current().exists(), "nothing materialised");
    assert_eq!(f.row().unwrap().status, SyncStatus::Disabled);
}

/// F1-R1: a row the app never materialised (`synced_at` NULL, here `error`
/// after a failed first sync) owns nothing in the former dir. Relocation must
/// neither materialise the skill for it nor remove the foreign former folder.
#[test]
fn relocation_leaves_a_row_that_owns_nothing_alone() {
    let f = fixture();
    fs::create_dir_all(f.former()).unwrap();
    fs::write(f.former().join("SKILL.md"), "someone else's").unwrap();
    f.store
        .upsert_skill_target(&SkillTargetRecord {
            id: "target-1".to_string(),
            skill_id: "skill-1".to_string(),
            tool: "augment".to_string(),
            target_path: f.former().to_string_lossy().to_string(),
            mode: SyncMode::Symlink,
            status: SyncStatus::Error,
            last_error: Some("target already exists".to_string()),
            synced_at: None,
        })
        .unwrap();

    assert_eq!(f.relocate(), RelocationSummary::default());
    assert!(
        f.current().symlink_metadata().is_err(),
        "nothing materialised"
    );
    assert_eq!(
        fs::read_to_string(f.former().join("SKILL.md")).unwrap(),
        "someone else's"
    );
    let row = f.row().unwrap();
    assert_eq!(row.status, SyncStatus::Error);
    assert_eq!(row.target_path, f.former().to_string_lossy());
}

/// A skill Disable whose removal failed keeps the former-dir row `error`
/// (ADR-0002) while `disabled_at` says "off everywhere": relocation must not
/// materialise the skill in the current dir for it.
#[cfg(unix)]
#[test]
fn partial_skill_disable_relocation_does_not_materialise() {
    use std::os::unix::fs::PermissionsExt;
    let f = fixture();
    legacy_link_row(&f);
    let rules = f.home().join(".augment/rules");
    fs::set_permissions(&rules, fs::Permissions::from_mode(0o555)).unwrap();
    if fs::create_dir(rules.join("probe")).is_ok() {
        // Running as root: permissions are not enforced.
        fs::set_permissions(&rules, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    let scope = DisableScope::Skill {
        skill_id: "skill-1".into(),
    };
    let report = disable_targets(&f.store, f.home(), &scope, 1).expect("disable reports");
    fs::set_permissions(&rules, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(report.failed_rows(), 1);
    assert!(f.store.skill_disabled_at("skill-1").unwrap().is_some());

    assert_eq!(
        f.relocate(),
        RelocationSummary {
            relocated: 0,
            left_in_place: 1,
            incomplete: Vec::new(),
        }
    );
    assert!(
        f.current().symlink_metadata().is_err(),
        "nothing materialised"
    );
    let row = f.row().expect("row kept");
    assert_eq!(row.status, SyncStatus::Error);
    assert_eq!(Path::new(&row.target_path), f.former());
    assert!(
        f.former().symlink_metadata().is_ok(),
        "former artifact untouched"
    );
}

#[test]
fn the_superseded_artifact_seam_refuses_anything_outside_a_former_dir() {
    let f = fixture();
    copy_of_central(&f, &f.current());

    for path in [f.current(), f.home().join(".augment/rules/nested/skill")] {
        let err =
            remove_superseded_artifact_unlocked(f.home(), augment(), GlobalLayout::OWN_DIRS, &path)
                .expect_err("refused");
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

// ---------------------------------------------------------------------------
// Ticket 34: corrected rows (brief § 5), the Kimi / Amp hazard (I4), the #83
// preference, and rows of retired Tools.
// ---------------------------------------------------------------------------

fn seed_tool_row(f: &Fixture, id: &str, tool: &str, target_path: &Path) {
    f.store
        .upsert_skill_target(&SkillTargetRecord {
            id: id.to_string(),
            skill_id: "skill-1".to_string(),
            tool: tool.to_string(),
            target_path: target_path.to_string_lossy().to_string(),
            mode: SyncMode::Symlink,
            status: SyncStatus::Synced,
            last_error: None,
            synced_at: Some(1),
        })
        .expect("seed target");
}

fn row_of(f: &Fixture, tool: &str) -> SkillTargetRecord {
    f.store
        .get_skill_target("skill-1", tool)
        .expect("read row")
        .unwrap_or_else(|| panic!("{tool} row"))
}

fn install(f: &Fixture, key: &str) {
    mark_installed_in(f.home(), adapter_by_key(key).expect("adapter"));
}

fn relocate_under(f: &Fixture, layout: GlobalLayout) -> RelocationSummary {
    relocate_unlocked(&f.store, f.home(), layout, NOW).expect("relocation reads its rows")
}

/// Brief I4: Kimi's former dir is Amp's live dir. With Amp installed and
/// both rows on the shared artifact, only Kimi's row moves — and Amp's
/// artifact stays where Amp reads it.
#[cfg(unix)]
#[test]
fn kimi_relocates_out_of_amps_shared_dir_and_amps_artifact_stays() {
    let f = fixture();
    install(&f, "amp");
    install(&f, "kimi_cli");
    let shared = f.home().join(".config/agents/skills/skill");
    link(&f.central_path, &shared);
    seed_tool_row(&f, "t-amp", "amp", &shared);
    seed_tool_row(&f, "t-kimi", "kimi_cli", &shared);

    let summary = relocate_under(&f, GlobalLayout::OWN_DIRS);

    assert_eq!(summary.relocated, 1, "only Kimi's row moves: {summary:?}");
    let kimi = f.home().join(".kimi-code/skills/skill");
    assert_eq!(Path::new(&row_of(&f, "kimi_cli").target_path), kimi);
    assert_eq!(fs::read_link(&kimi).expect("kimi link"), f.central_path);
    assert_eq!(Path::new(&row_of(&f, "amp").target_path), shared);
    assert_eq!(
        fs::read_link(&shared).expect("Amp's artifact is kept"),
        f.central_path
    );
    assert_eq!(
        relocate_under(&f, GlobalLayout::OWN_DIRS),
        RelocationSummary::default()
    );
}

/// Kimi alone in the former dir: its row moves and the old artifact goes.
#[cfg(unix)]
#[test]
fn kimi_alone_relocates_and_its_former_artifact_is_removed() {
    let f = fixture();
    install(&f, "kimi_cli");
    let old = f.home().join(".config/agents/skills/skill");
    link(&f.central_path, &old);
    seed_tool_row(&f, "t-kimi", "kimi_cli", &old);

    assert_eq!(relocate_under(&f, GlobalLayout::OWN_DIRS).relocated, 1);
    assert_eq!(
        Path::new(&row_of(&f, "kimi_cli").target_path),
        f.home().join(".kimi-code/skills/skill")
    );
    assert!(old.symlink_metadata().is_err(), "former artifact removed");
}

/// Brief I2: Codex's `~/.codex/skills` targets move into `~/.agents/skills`
/// in either layout — Codex deploys there regardless of the preference.
#[cfg(unix)]
#[test]
fn codex_targets_relocate_from_the_deprecated_dir_to_the_agents_standard_dir() {
    for layout in [GlobalLayout::OWN_DIRS, GlobalLayout::PREFER_AGENTS_STANDARD] {
        let f = fixture();
        install(&f, "codex");
        let old = f.home().join(".codex/skills/skill");
        link(&f.central_path, &old);
        seed_tool_row(&f, "t-codex", "codex", &old);

        assert_eq!(relocate_under(&f, layout).relocated, 1, "{layout:?}");
        let new = f.home().join(".agents/skills/skill");
        assert_eq!(Path::new(&row_of(&f, "codex").target_path), new);
        assert_eq!(fs::read_link(&new).unwrap(), f.central_path);
        assert!(old.symlink_metadata().is_err());
    }
}

/// Every corrected row's former dir relocates into its current one (§ 5).
#[cfg(unix)]
#[test]
fn corrected_rows_relocate_from_their_former_dirs() {
    let cases = [
        (
            "antigravity",
            ".gemini/antigravity/global_skills",
            ".gemini/config/skills",
        ),
        ("kilo_code", ".kilocode/skills", ".kilo/skills"),
        ("mux", ".mux/skills", ".xum/skills"),
        ("goose", ".config/goose/skills", ".agents/skills"),
        ("openhands", ".openhands/skills", ".agents/skills"),
        // OpenClaw deployed to `~/.moltbot/skills` before its rename.
        ("openclaw", ".moltbot/skills", ".openclaw/skills"),
    ];
    for (tool, former, current) in cases {
        let f = fixture();
        install(&f, tool);
        let old = f.home().join(former).join("skill");
        link(&f.central_path, &old);
        seed_tool_row(&f, "t", tool, &old);
        assert_eq!(
            relocate_under(&f, GlobalLayout::OWN_DIRS).relocated,
            1,
            "{tool}"
        );
        assert_eq!(
            Path::new(&row_of(&f, tool).target_path),
            f.home().join(current).join("skill"),
            "{tool}"
        );
        assert!(old.symlink_metadata().is_err(), "{tool}");
    }
}

/// #83 on save: a reader's own-dir target moves into `~/.agents/skills`
/// (the preview counts it first); turning the preference off moves it back.
#[cfg(unix)]
#[test]
fn the_preference_relocates_a_readers_targets_both_ways_on_save() {
    use crate::core::settings::{global_layout, load_settings};
    use crate::core::target_relocation::{apply_global_layout, preview_relocation};

    let f = fixture();
    install(&f, "cursor");
    let own = f.home().join(".cursor/skills/skill");
    let agents = f.home().join(".agents/skills/skill");
    link(&f.central_path, &own);
    seed_tool_row(&f, "t-cursor", "cursor", &own);

    let on = GlobalLayout::PREFER_AGENTS_STANDARD;
    assert_eq!(preview_relocation(&f.store, f.home(), on).unwrap(), 1);
    assert_eq!(
        preview_relocation(&f.store, f.home(), GlobalLayout::OWN_DIRS).unwrap(),
        0
    );

    let summary = apply_global_layout(&f.store, f.home(), on, NOW).unwrap();
    assert_eq!(summary.relocated, 1);
    assert_eq!(global_layout(&f.store), on, "the preference is saved");
    assert_eq!(Path::new(&row_of(&f, "cursor").target_path), agents);
    assert_eq!(fs::read_link(&agents).unwrap(), f.central_path);
    assert!(
        own.symlink_metadata().is_err(),
        "no harness-specific copy left"
    );

    let back = apply_global_layout(&f.store, f.home(), GlobalLayout::OWN_DIRS, NOW).unwrap();
    assert_eq!(back.relocated, 1);
    assert_eq!(Path::new(&row_of(&f, "cursor").target_path), own);
    assert!(agents.symlink_metadata().is_err());
    let settings = load_settings(
        &f.store,
        &crate::core::credentials::MemoryStore::new(),
        f.home(),
    )
    .unwrap();
    assert!(!settings.prefer_agents_standard_dir);
}

/// Turning the preference off moves a reader back to its own dir, but the
/// shared `~/.agents/skills` artifact stays while a structural sharer's row
/// (Codex) still records it.
#[cfg(unix)]
#[test]
fn turning_the_preference_off_keeps_the_artifact_a_sharer_still_records() {
    let f = fixture();
    install(&f, "cursor");
    install(&f, "codex");
    let agents = f.home().join(".agents/skills/skill");
    link(&f.central_path, &agents);
    seed_tool_row(&f, "t-cursor", "cursor", &agents);
    seed_tool_row(&f, "t-codex", "codex", &agents);

    assert_eq!(relocate_under(&f, GlobalLayout::OWN_DIRS).relocated, 1);
    assert_eq!(
        Path::new(&row_of(&f, "cursor").target_path),
        f.home().join(".cursor/skills/skill")
    );
    assert_eq!(Path::new(&row_of(&f, "codex").target_path), agents);
    assert!(agents.symlink_metadata().is_ok(), "Codex's artifact stays");
}

/// Preference on, Amp and Kimi rows on Amp's dir: both readers deploy to
/// `~/.agents/skills`, both rows move, and with nothing left recording it
/// the old artifact goes.
#[cfg(unix)]
#[test]
fn under_the_preference_every_reader_on_a_shared_dir_moves_together() {
    let f = fixture();
    install(&f, "amp");
    install(&f, "kimi_cli");
    let shared = f.home().join(".config/agents/skills/skill");
    link(&f.central_path, &shared);
    seed_tool_row(&f, "t-amp", "amp", &shared);
    seed_tool_row(&f, "t-kimi", "kimi_cli", &shared);

    let summary = relocate_under(&f, GlobalLayout::PREFER_AGENTS_STANDARD);
    assert_eq!(summary.relocated, 2, "{summary:?}");
    let agents = f.home().join(".agents/skills/skill");
    for tool in ["amp", "kimi_cli"] {
        assert_eq!(Path::new(&row_of(&f, tool).target_path), agents, "{tool}");
    }
    assert!(shared.symlink_metadata().is_err(), "nothing records it now");
}

/// A non-reader never moves under the preference (Claude Code, brief I12).
#[cfg(unix)]
#[test]
fn the_preference_leaves_a_non_reader_alone() {
    let f = fixture();
    install(&f, "claude_code");
    let own = f.home().join(".claude/skills/skill");
    link(&f.central_path, &own);
    seed_tool_row(&f, "t-claude", "claude_code", &own);
    assert_eq!(
        relocate_under(&f, GlobalLayout::PREFER_AGENTS_STANDARD),
        RelocationSummary::default()
    );
    assert!(own.symlink_metadata().is_ok());
}

/// Brief I10: a row stored under OpenClaw's former name (Clawdbot, MoltBot)
/// in that name's dir is re-keyed to OpenClaw as relocation moves it — into
/// `~/.openclaw/skills`, or `~/.agents/skills` under the preference
/// (OpenClaw is an AGENTS-wins reader) — and the old artifact goes.
#[cfg(unix)]
#[test]
fn a_former_names_row_is_rekeyed_to_openclaw_and_relocates() {
    use crate::core::target_relocation::preview_relocation;
    for key in crate::core::tool_adapters::RETIRED_TOOL_KEYS {
        for (layout, current) in [
            (GlobalLayout::OWN_DIRS, ".openclaw/skills"),
            (GlobalLayout::PREFER_AGENTS_STANDARD, ".agents/skills"),
        ] {
            let f = fixture();
            install(&f, "openclaw");
            let old = f.home().join(format!(".{key}/skills/skill"));
            link(&f.central_path, &old);
            seed_tool_row(&f, "t-former", key, &old);

            assert_eq!(
                preview_relocation(&f.store, f.home(), layout).unwrap(),
                1,
                "{key} {layout:?}"
            );
            assert_eq!(
                relocate_under(&f, layout),
                RelocationSummary {
                    relocated: 1,
                    left_in_place: 0,
                    incomplete: Vec::new(),
                },
                "{key} {layout:?}"
            );
            assert!(f.store.get_skill_target("skill-1", key).unwrap().is_none());
            let row = row_of(&f, "openclaw");
            assert_eq!(row.id, "t-former", "the same row, re-keyed");
            let new = f.home().join(current).join("skill");
            assert_eq!(Path::new(&row.target_path), new, "{key} {layout:?}");
            assert_eq!(fs::read_link(&new).unwrap(), f.central_path);
            assert!(
                old.symlink_metadata().is_err(),
                "{key}: former artifact removed"
            );
            assert_eq!(
                relocate_under(&f, layout),
                RelocationSummary::default(),
                "idempotent"
            );
        }
    }
}

/// A former name's row beside an existing OpenClaw row cannot be re-keyed
/// (`UNIQUE(skill_id, tool)`): relocation leaves it in place, Propagation
/// skips it as an unknown key, and Delete still removes its artifact by the
/// row's stored path.
#[cfg(unix)]
#[test]
fn a_retired_tools_stored_row_is_tolerated_and_still_deletable() {
    use crate::core::artifact_removal::remove_skill;
    use crate::core::propagation::{PropagationScope, PropagationSkip, PropagationStatus};
    use crate::core::target_relocation::preview_relocation;

    for key in crate::core::tool_adapters::RETIRED_TOOL_KEYS {
        let f = fixture();
        install(&f, "openclaw");
        let current = f.home().join(".openclaw/skills/skill");
        link(&f.central_path, &current);
        seed_tool_row(&f, "t-openclaw", "openclaw", &current);
        let path = f.home().join(format!(".{key}/skills/skill"));
        link(&f.central_path, &path);
        seed_tool_row(&f, "t-retired", key, &path);

        assert_eq!(
            preview_relocation(&f.store, f.home(), GlobalLayout::OWN_DIRS).unwrap(),
            0,
            "{key}: a blocked re-key is not a move"
        );
        assert_eq!(
            relocate_under(&f, GlobalLayout::OWN_DIRS),
            RelocationSummary {
                relocated: 0,
                left_in_place: 1,
                incomplete: Vec::new(),
            },
            "{key}"
        );
        assert_eq!(Path::new(&row_of(&f, key).target_path), path, "{key}");
        assert!(path.symlink_metadata().is_ok(), "{key}: left in place");
        let report = propagate_unlocked(&f.store, &f.paths, "skill-1", NOW).expect("propagate");
        let outcome = report
            .targets
            .iter()
            .find(|t| {
                t.scope
                    == PropagationScope::Global {
                        tool: key.to_string(),
                    }
            })
            .unwrap_or_else(|| panic!("{key} reported"));
        assert!(
            matches!(
                &outcome.status,
                PropagationStatus::Skipped {
                    reason: PropagationSkip::UnknownTool { .. }
                }
            ),
            "{key}: {:?}",
            outcome.status
        );

        let removal = remove_skill(&f.store, "skill-1").expect("delete");
        assert_eq!(removal.failed_rows(), 0, "{key}: {removal}");
        assert!(path.symlink_metadata().is_err(), "{key}: artifact removed");
        assert!(f.store.get_skill_target("skill-1", key).unwrap().is_none());
    }
}

// ---------------------------------------------------------------------------
// The launch-time pass and the #83 preference's unset default (fix pass)
// ---------------------------------------------------------------------------

/// An existing install (deployments recorded) launching with the preference
/// never saved: the launch pass moves only registry-truth relocations
/// (Codex's deprecated dir) — never a reader's own-dir target (Cursor) — and
/// writes no preference; the settings offer the one-time choice.
#[cfg(unix)]
#[test]
fn the_launch_pass_never_moves_a_readers_target_on_the_unset_default() {
    use crate::core::credentials::MemoryStore;
    use crate::core::settings::{global_layout, load_settings};
    use crate::core::target_relocation::relocate_former_global_targets;

    let f = fixture();
    install(&f, "cursor");
    install(&f, "codex");
    let cursor = f.home().join(".cursor/skills/skill");
    link(&f.central_path, &cursor);
    seed_tool_row(&f, "t-cursor", "cursor", &cursor);
    let codex_old = f.home().join(".codex/skills/skill");
    link(&f.central_path, &codex_old);
    seed_tool_row(&f, "t-codex", "codex", &codex_old);

    let summary = relocate_former_global_targets(&f.store, f.home(), NOW).unwrap();
    assert_eq!(
        summary,
        RelocationSummary {
            relocated: 1,
            left_in_place: 0,
            incomplete: Vec::new(),
        },
        "only Codex's registry-truth move"
    );
    assert_eq!(Path::new(&row_of(&f, "cursor").target_path), cursor);
    assert!(cursor.symlink_metadata().is_ok(), "Cursor's artifact stays");
    assert_eq!(
        Path::new(&row_of(&f, "codex").target_path),
        f.home().join(".agents/skills/skill")
    );
    assert_eq!(
        f.store.get_setting("prefer_agents_standard_dir").unwrap(),
        None,
        "the launch pass writes no preference"
    );
    assert_eq!(global_layout(&f.store), GlobalLayout::OWN_DIRS);
    let settings = load_settings(&f.store, &MemoryStore::new(), f.home()).unwrap();
    assert!(!settings.prefer_agents_standard_dir);
    assert!(settings.agents_standard_offer_pending);
}

/// The offer answered "on" (`apply_global_layout`): Cursor's row moves under
/// the guard, the summary counts it, the offer is no longer pending, and the
/// next launch pass finds nothing left to move.
#[cfg(unix)]
#[test]
fn saving_the_preference_on_an_existing_install_relocates_and_ends_the_offer() {
    use crate::core::credentials::MemoryStore;
    use crate::core::settings::load_settings;
    use crate::core::target_relocation::{
        apply_global_layout, preview_relocation, relocate_former_global_targets,
    };

    let f = fixture();
    install(&f, "cursor");
    let own = f.home().join(".cursor/skills/skill");
    link(&f.central_path, &own);
    seed_tool_row(&f, "t-cursor", "cursor", &own);
    let on = GlobalLayout::PREFER_AGENTS_STANDARD;

    assert_eq!(preview_relocation(&f.store, f.home(), on).unwrap(), 1);
    let summary = apply_global_layout(&f.store, f.home(), on, NOW).unwrap();
    assert_eq!(
        summary,
        RelocationSummary {
            relocated: 1,
            left_in_place: 0,
            incomplete: Vec::new(),
        }
    );
    let agents = f.home().join(".agents/skills/skill");
    assert_eq!(Path::new(&row_of(&f, "cursor").target_path), agents);
    assert!(own.symlink_metadata().is_err());
    let settings = load_settings(&f.store, &MemoryStore::new(), f.home()).unwrap();
    assert!(settings.prefer_agents_standard_dir);
    assert!(!settings.agents_standard_offer_pending);
    assert_eq!(
        relocate_former_global_targets(&f.store, f.home(), NOW).unwrap(),
        RelocationSummary::default()
    );
}

/// The offer answered "off": nothing moves, and it is no longer pending.
#[test]
fn declining_the_offer_saves_own_dirs_and_ends_it() {
    use crate::core::credentials::MemoryStore;
    use crate::core::settings::load_settings;
    use crate::core::target_relocation::apply_global_layout;

    let f = fixture();
    let own = f.home().join(".cursor/skills/skill");
    seed_tool_row(&f, "t-cursor", "cursor", &own);
    assert!(
        load_settings(&f.store, &MemoryStore::new(), f.home())
            .unwrap()
            .agents_standard_offer_pending
    );
    let summary = apply_global_layout(&f.store, f.home(), GlobalLayout::OWN_DIRS, NOW).unwrap();
    assert_eq!(summary, RelocationSummary::default());
    let settings = load_settings(&f.store, &MemoryStore::new(), f.home()).unwrap();
    assert!(!settings.prefer_agents_standard_dir);
    assert!(!settings.agents_standard_offer_pending);
}

// ---------------------------------------------------------------------------
// Fix pass 2 (ticket-34 backend audit): the destination group's Disabled
// admission (F1), ownership of the old path across every row (F2), and a
// first sync never replacing a different owned path (F3).
// ---------------------------------------------------------------------------

/// A never-synced Disabled marker (a `Keep` answer, or a pair Disable that
/// had nothing to remove): it owns nothing, but it says "not on this Tool".
fn seed_disabled_marker(f: &Fixture, id: &str, tool: &str, target_path: &Path) {
    f.store
        .upsert_skill_target(&SkillTargetRecord {
            id: id.to_string(),
            skill_id: "skill-1".to_string(),
            tool: tool.to_string(),
            target_path: target_path.to_string_lossy().to_string(),
            mode: SyncMode::Symlink,
            status: SyncStatus::Disabled,
            last_error: None,
            synced_at: None,
        })
        .expect("seed disabled marker");
}

/// F1: Cursor is live in its own dir, Codex is disabled for the skill (its
/// marker records `~/.agents/skills/skill`, absent). Turning the preference
/// on would move Cursor into the dir Codex reads — re-materialising a skill
/// the operator switched off there. The move is refused exactly as the
/// global batch's Disabled gate refuses a deploy: Cursor stays, the preview
/// and the pass both count no move, and the marker is never cleared.
#[cfg(unix)]
#[test]
fn the_preference_never_moves_a_target_into_a_dir_a_disabled_sharer_reads() {
    use crate::core::target_relocation::{apply_global_layout, preview_relocation};

    let f = fixture();
    install(&f, "cursor");
    install(&f, "codex");
    let own = f.home().join(".cursor/skills/skill");
    let agents = f.home().join(".agents/skills/skill");
    link(&f.central_path, &own);
    seed_tool_row(&f, "t-cursor", "cursor", &own);
    seed_disabled_marker(&f, "t-codex", "codex", &agents);
    let on = GlobalLayout::PREFER_AGENTS_STANDARD;

    assert_eq!(preview_relocation(&f.store, f.home(), on).unwrap(), 0);
    let summary = apply_global_layout(&f.store, f.home(), on, NOW).unwrap();
    assert_eq!(
        summary,
        RelocationSummary {
            relocated: 0,
            left_in_place: 1,
            incomplete: Vec::new(),
        }
    );
    assert_eq!(Path::new(&row_of(&f, "cursor").target_path), own);
    assert_eq!(fs::read_link(&own).unwrap(), f.central_path, "Cursor stays");
    assert!(
        agents.symlink_metadata().is_err(),
        "nothing where Codex reads"
    );
    let marker = row_of(&f, "codex");
    assert_eq!(marker.status, SyncStatus::Disabled, "the marker is intact");
    assert!(!marker.owns_artifact());
    assert_eq!(Path::new(&marker.target_path), agents);
}

/// F1, the registry-truth move: Codex's deprecated-dir target would move
/// into `~/.agents/skills`, but the AGENTS-standard entry is disabled for the
/// skill there — the row and its artifact stay.
#[cfg(unix)]
#[test]
fn codexs_registry_move_waits_while_a_destination_sharer_is_disabled() {
    use crate::core::target_relocation::preview_relocation;

    let f = fixture();
    install(&f, "codex");
    let old = f.home().join(".codex/skills/skill");
    let agents = f.home().join(".agents/skills/skill");
    link(&f.central_path, &old);
    seed_tool_row(&f, "t-codex", "codex", &old);
    seed_disabled_marker(&f, "t-agents", "agents_skills", &agents);

    for layout in [GlobalLayout::OWN_DIRS, GlobalLayout::PREFER_AGENTS_STANDARD] {
        assert_eq!(
            preview_relocation(&f.store, f.home(), layout).unwrap(),
            0,
            "{layout:?}"
        );
        assert_eq!(
            relocate_under(&f, layout),
            RelocationSummary {
                relocated: 0,
                left_in_place: 1,
                incomplete: Vec::new(),
            },
            "{layout:?}"
        );
        assert_eq!(Path::new(&row_of(&f, "codex").target_path), old);
        assert!(old.symlink_metadata().is_ok(), "the artifact stays");
        assert!(agents.symlink_metadata().is_err(), "{layout:?}");
        assert_eq!(row_of(&f, "agents_skills").status, SyncStatus::Disabled);
    }
}

/// Register `dir` as a project (canonicalised, as the app registers it) and
/// record an owned assignment of the skill for `tool` there.
fn assign_in_project(f: &Fixture, dir: &Path, tool: &str) -> String {
    use crate::core::project_ops::register_project_path;
    use crate::core::skill_store::ProjectSkillAssignmentRecord;

    let project = register_project_path(&f.store, f.home(), &dir.to_string_lossy(), 1)
        .expect("register project");
    f.store
        .add_project_skill_assignment(&ProjectSkillAssignmentRecord {
            id: format!("a-{tool}"),
            project_id: project.id.clone(),
            skill_id: "skill-1".to_string(),
            skill_name: "skill".to_string(),
            tool: tool.to_string(),
            mode: SyncMode::Symlink,
            status: SyncStatus::Synced,
            last_error: None,
            synced_at: Some(1),
            content_hash: None,
            created_at: 1,
        })
        .expect("seed assignment");
    project.id
}

fn assignment_owned(f: &Fixture, project_id: &str, tool: &str) -> bool {
    f.store
        .get_project_skill_assignment(project_id, "skill-1", tool)
        .expect("read assignment")
        .is_some_and(|a| a.status == SyncStatus::Synced && a.owns_artifact())
}

/// F2: the home directory registered as a project, with an Augment
/// assignment whose artifact is `home/.augment/skills/skill` — the very path
/// the global Augment row records. The preference moves the global row, both
/// ways, but the project's artifact is never removed under it: the old path
/// is still owned (by the assignment, compared through the canonicalised
/// project root).
#[cfg(unix)]
#[test]
fn relocation_never_removes_an_artifact_a_project_assignment_owns() {
    use crate::core::target_relocation::apply_global_layout;

    let f = fixture();
    install(&f, "augment");
    let shared = f.home().join(".augment/skills/skill");
    link(&f.central_path, &shared);
    seed_tool_row(&f, "t-augment", "augment", &shared);
    let project_id = assign_in_project(&f, f.home(), "augment");
    let agents = f.home().join(".agents/skills/skill");

    let on = apply_global_layout(
        &f.store,
        f.home(),
        GlobalLayout::PREFER_AGENTS_STANDARD,
        NOW,
    )
    .unwrap();
    assert_eq!(on.relocated, 1, "{on:?}");
    assert_eq!(Path::new(&row_of(&f, "augment").target_path), agents);
    assert_eq!(
        fs::read_link(&shared).expect("the project's artifact stays"),
        f.central_path
    );
    assert!(assignment_owned(&f, &project_id, "augment"));

    let off = apply_global_layout(&f.store, f.home(), GlobalLayout::OWN_DIRS, NOW).unwrap();
    assert_eq!(off.relocated, 1, "{off:?}");
    assert_eq!(Path::new(&row_of(&f, "augment").target_path), shared);
    assert_eq!(fs::read_link(&shared).unwrap(), f.central_path);
    assert!(agents.symlink_metadata().is_err(), "nothing records it now");
    assert!(assignment_owned(&f, &project_id, "augment"));
}

/// F2: `~/.config/agents` registered as a project, OpenClaw assigned there
/// (project dir `skills`) — the artifact is `~/.config/agents/skills/skill`,
/// Kimi's former global dir. Kimi's row relocates; the project's artifact
/// stays.
#[cfg(unix)]
#[test]
fn kimis_relocation_keeps_an_artifact_a_config_agents_project_owns() {
    let f = fixture();
    install(&f, "kimi_cli");
    let shared = f.home().join(".config/agents/skills/skill");
    link(&f.central_path, &shared);
    seed_tool_row(&f, "t-kimi", "kimi_cli", &shared);
    let project_id = assign_in_project(&f, &f.home().join(".config/agents"), "openclaw");

    assert_eq!(relocate_under(&f, GlobalLayout::OWN_DIRS).relocated, 1);
    assert_eq!(
        Path::new(&row_of(&f, "kimi_cli").target_path),
        f.home().join(".kimi-code/skills/skill")
    );
    assert_eq!(
        fs::read_link(&shared).expect("the project's artifact stays"),
        f.central_path
    );
    assert!(assignment_owned(&f, &project_id, "openclaw"));
}

/// F3: a relocation that could not finish (a foreign folder in
/// `~/.agents/skills` kept Cursor in its own dir), then the operator deploys
/// Cursor and confirms the overwrite. The first sync writes the shared dir,
/// but it never silently re-points Cursor's row away from the artifact it
/// still owns: the move goes through the relocation seam, so the old copy is
/// removed with its row moved — and Delete leaves nothing behind.
#[cfg(unix)]
#[test]
fn a_first_sync_never_replaces_a_different_owned_target_path() {
    use crate::core::artifact_removal::remove_skill;
    use crate::core::global_sync::{
        sync_skills_to_tools_unlocked, BatchOverride, BatchPolicy, BatchSkill, BatchTargetStatus,
        OverrideDecision,
    };
    use crate::core::settings::write_prefer_agents_standard_dir;

    let f = fixture();
    install(&f, "cursor");
    let own = f.home().join(".cursor/skills/skill");
    let agents = f.home().join(".agents/skills/skill");
    link(&f.central_path, &own);
    seed_tool_row(&f, "t-cursor", "cursor", &own);
    fs::create_dir_all(&agents).unwrap();
    fs::write(agents.join("SKILL.md"), "foreign").unwrap();
    write_prefer_agents_standard_dir(&f.store, true).unwrap();
    assert_eq!(
        relocate_under(&f, GlobalLayout::PREFER_AGENTS_STANDARD),
        RelocationSummary {
            relocated: 0,
            left_in_place: 1,
            incomplete: Vec::new(),
        },
        "premise: the relocation could not finish"
    );

    let outcomes = sync_skills_to_tools_unlocked(
        f.home(),
        &f.store,
        &[BatchSkill {
            skill_id: "skill-1".to_string(),
            skill_name: "skill".to_string(),
            source_path: f.central_path.clone(),
        }],
        &["cursor".to_string()],
        &BatchPolicy {
            overwrite: false,
            overwrite_if_same_content: true,
            overrides: vec![BatchOverride {
                skill_id: "skill-1".to_string(),
                tool_key: "cursor".to_string(),
                decision: OverrideDecision::Overwrite,
            }],
        },
        NOW,
        |_| {},
    );
    assert!(
        outcomes
            .iter()
            .all(|o| matches!(o.status, BatchTargetStatus::Synced { .. })),
        "{outcomes:?}"
    );

    // Every artifact of the skill on disk is recorded by a row.
    for path in [&own, &agents] {
        if path.symlink_metadata().is_ok() {
            let recorded = f
                .store
                .list_skill_targets("skill-1")
                .unwrap()
                .iter()
                .any(|row| row.owns_artifact() && Path::new(&row.target_path) == path.as_path());
            assert!(recorded, "{} is on disk with no owner", path.display());
        }
    }
    assert_eq!(Path::new(&row_of(&f, "cursor").target_path), agents);
    assert!(
        own.symlink_metadata().is_err(),
        "the old copy went with its row"
    );

    let removal = remove_skill(&f.store, "skill-1").expect("delete");
    assert_eq!(removal.failed_rows(), 0, "{removal}");
    for path in [&own, &agents] {
        assert!(
            path.symlink_metadata().is_err(),
            "{} left behind",
            path.display()
        );
    }
}

// ---------------------------------------------------------------------------
// Fix pass 3 (audit 2): F10–F13, ported from the auditor's executed
// reproductions (`.scratch/round19/review/ticket-34-backend-audit-2.md`).
// ---------------------------------------------------------------------------

/// F10: a move whose source removal fails is rolled back (ADR-0002) — the
/// just-created destination goes, the row stays at the source as `error`
/// (ownership kept) and is reported left in place with its reason — so the
/// next launch's admission sees it and retries.
#[cfg(unix)]
#[test]
fn f10_removal_failure_is_retried_or_reported_next_launch() {
    use crate::core::target_relocation::relocate_former_global_targets;
    use std::os::unix::fs::PermissionsExt;

    let f = fixture();
    legacy_link_row(&f);
    let parent = f.former().parent().unwrap().to_path_buf();
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o500)).unwrap();
    let first = relocate_former_global_targets(&f.store, f.home(), NOW).unwrap();
    let old_survived = f.former().symlink_metadata().is_ok();
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(old_survived, "fault injection must actually deny removal");

    assert_eq!(first.relocated, 0, "{first:?}");
    assert_eq!(first.left_in_place, 1, "{first:?}");
    assert_eq!(
        first.incomplete.len(),
        1,
        "the reason travels in the summary"
    );
    assert_eq!(first.incomplete[0].tool, "augment");
    assert_eq!(Path::new(&first.incomplete[0].path), f.former());
    let row = f.row().expect("row kept");
    assert_eq!(
        Path::new(&row.target_path),
        f.former(),
        "row stays at the source"
    );
    assert_eq!(row.status, SyncStatus::Error);
    assert!(row.last_error.is_some());
    assert_eq!(row.synced_at, Some(1), "ownership kept");
    assert!(
        f.current().symlink_metadata().is_err(),
        "the just-created destination was rolled back"
    );

    let second = relocate_former_global_targets(&f.store, f.home(), NOW + 1).unwrap();
    assert!(
        !f.former().exists() || second.left_in_place > 0,
        "source is neither retried nor reported on next launch"
    );
    assert_eq!(
        second,
        RelocationSummary {
            relocated: 1,
            left_in_place: 0,
            incomplete: Vec::new(),
        }
    );
    let row = f.row().unwrap();
    assert_eq!(Path::new(&row.target_path), f.current());
    assert_eq!(row.status, SyncStatus::Synced);
    assert!(f.former().symlink_metadata().is_err());
}

/// F10 crash boundary: a crash after the source was removed but before the
/// row was settled leaves the row at an absent source with our bytes at the
/// destination. The next launch adopts the destination instead of leaving it
/// unowned.
#[cfg(unix)]
#[test]
fn f10_a_crash_after_source_removal_is_completed_next_launch() {
    let f = fixture();
    seed_row(&f, &f.former(), SyncMode::Symlink);
    link(&f.central_path, &f.current());

    assert_eq!(f.relocate().relocated, 1);
    let row = f.row().unwrap();
    assert_eq!(Path::new(&row.target_path), f.current());
    assert_eq!(row.status, SyncStatus::Synced);
}

/// F10 crash boundary: a crash after the destination was written but before
/// the source was touched leaves the old row and artifact intact; the next
/// launch replaces the identical destination and finishes the move.
#[cfg(unix)]
#[test]
fn f10_a_crash_after_materialising_is_completed_next_launch() {
    let f = fixture();
    legacy_link_row(&f);
    link(&f.central_path, &f.current());

    assert_eq!(f.relocate().relocated, 1);
    assert_eq!(Path::new(&f.row().unwrap().target_path), f.current());
    assert!(f.former().symlink_metadata().is_err(), "old link removed");
}

/// F10: the rollback never removes a destination another row owns — the
/// failed mover keeps its source, the sharer keeps its artifact.
#[cfg(unix)]
#[test]
fn f10_rollback_keeps_a_destination_another_row_owns() {
    use std::os::unix::fs::PermissionsExt;

    let f = fixture();
    install(&f, "cursor");
    install(&f, "codex");
    let own = f.home().join(".cursor/skills/skill");
    let agents = f.home().join(".agents/skills/skill");
    link(&f.central_path, &own);
    link(&f.central_path, &agents);
    seed_tool_row(&f, "t-cursor", "cursor", &own);
    seed_tool_row(&f, "t-codex", "codex", &agents);
    let parent = own.parent().unwrap().to_path_buf();
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o500)).unwrap();
    let summary = relocate_under(&f, GlobalLayout::PREFER_AGENTS_STANDARD);
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();

    assert_eq!(summary.left_in_place, 1, "{summary:?}");
    assert!(agents.symlink_metadata().is_ok(), "Codex's artifact stays");
    assert_eq!(Path::new(&row_of(&f, "codex").target_path), agents);
    assert_eq!(row_of(&f, "cursor").status, SyncStatus::Error);
    assert_eq!(Path::new(&row_of(&f, "cursor").target_path), own);
}

fn keep_cursor(f: &Fixture) -> Vec<crate::core::global_sync::BatchTargetOutcome> {
    use crate::core::global_sync::{
        sync_skills_to_tools, BatchOverride, BatchPolicy, BatchSkill, OverrideDecision,
    };
    sync_skills_to_tools(
        f.home(),
        &f.store,
        &[BatchSkill {
            skill_id: "skill-1".into(),
            skill_name: "skill".into(),
            source_path: f.central_path.clone(),
        }],
        &["cursor".into()],
        &BatchPolicy {
            overwrite_if_same_content: true,
            overrides: vec![BatchOverride {
                skill_id: "skill-1".into(),
                tool_key: "cursor".into(),
                decision: OverrideDecision::Keep,
            }],
            ..Default::default()
        },
        NOW,
        |_| {},
    )
}

/// F11: refused move → Keep → Enable → Delete. Keep settles the old owned
/// artifact through Artifact removal before recording the pair `disabled`,
/// never touching the foreign folder the operator kept; Enable then starts
/// from a clean state and nothing is orphaned.
#[cfg(unix)]
#[test]
fn f11_keep_then_enable_never_orphans_the_owned_former_copy() {
    use crate::core::artifact_removal::{artifact_is_owned, remove_skill};
    use crate::core::global_sync::BatchTargetStatus;

    let f = fixture();
    install(&f, "cursor");
    let own = f.home().join(".cursor/skills/skill");
    let agents = f.home().join(".agents/skills/skill");
    link(&f.central_path, &own);
    seed_tool_row(&f, "cursor-row", "cursor", &own);
    fs::create_dir_all(&agents).unwrap();
    fs::write(agents.join("SKILL.md"), "foreign").unwrap();
    crate::core::target_relocation::apply_global_layout(
        &f.store,
        f.home(),
        GlobalLayout::PREFER_AGENTS_STANDARD,
        NOW,
    )
    .unwrap();

    let outcomes = keep_cursor(&f);
    assert!(
        matches!(outcomes[0].status, BatchTargetStatus::Declined),
        "{outcomes:?}"
    );
    assert!(
        own.symlink_metadata().is_err(),
        "the owned former copy went"
    );
    let row = row_of(&f, "cursor");
    assert_eq!(row.status, SyncStatus::Disabled);
    assert!(!row.owns_artifact(), "a Keep marker owns nothing");
    assert_eq!(
        fs::read_to_string(agents.join("SKILL.md")).unwrap(),
        "foreign",
        "the kept folder is untouched"
    );

    let enabled = crate::core::skill_disable::enable_targets(
        &f.store,
        f.home(),
        &DisableScope::SkillTool {
            skill_id: "skill-1".into(),
            tool: "cursor".into(),
        },
        NOW + 1,
    )
    .unwrap();
    assert!(
        !own.exists() || artifact_is_owned(&f.store, &own).unwrap(),
        "Keep followed by Enable orphaned a live old deployment: {enabled:?}"
    );

    let removal = remove_skill(&f.store, "skill-1").expect("delete");
    assert_eq!(removal.failed_rows(), 0, "{removal}");
    assert!(own.symlink_metadata().is_err());
    assert_eq!(
        fs::read_to_string(agents.join("SKILL.md")).unwrap(),
        "foreign"
    );
}

/// F11: when the old owned artifact cannot be removed, Keep is a typed
/// failure, not Declined — the row keeps its old path and ownership as
/// `error` (ADR-0002).
#[cfg(unix)]
#[test]
fn f11_keep_whose_former_copy_cannot_be_removed_fails_and_keeps_the_row() {
    use crate::core::global_sync::BatchTargetStatus;
    use std::os::unix::fs::PermissionsExt;

    let f = fixture();
    install(&f, "cursor");
    let own = f.home().join(".cursor/skills/skill");
    let agents = f.home().join(".agents/skills/skill");
    link(&f.central_path, &own);
    seed_tool_row(&f, "cursor-row", "cursor", &own);
    fs::create_dir_all(&agents).unwrap();
    fs::write(agents.join("SKILL.md"), "foreign").unwrap();
    crate::core::settings::write_prefer_agents_standard_dir(&f.store, true).unwrap();

    let parent = own.parent().unwrap().to_path_buf();
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o500)).unwrap();
    let outcomes = keep_cursor(&f);
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();

    assert!(
        matches!(outcomes[0].status, BatchTargetStatus::Failed { .. }),
        "{outcomes:?}"
    );
    let row = row_of(&f, "cursor");
    assert_eq!(Path::new(&row.target_path), own);
    assert_eq!(row.status, SyncStatus::Error);
    assert!(row.owns_artifact());
    assert!(own.symlink_metadata().is_ok());
}

/// F12: an owned copy reached through a Tool skills dir that is a symlink to
/// another Tool's dir is the same physical artifact — never offered.
#[cfg(unix)]
#[test]
fn f12_owned_copy_is_excluded_through_a_symlinked_tool_dir() {
    use crate::core::onboarding::{build_onboarding_plan, OnboardingScanScope};

    for scope in [
        OnboardingScanScope::Selected(vec!["claude_code".into()]),
        OnboardingScanScope::Installed,
    ] {
        let f = fixture();
        install(&f, "cursor");
        install(&f, "claude_code");
        let own = f.home().join(".cursor/skills/skill");
        copy_of_central(&f, &own);
        seed_tool_row(&f, "cursor-row", "cursor", &own);
        // Installedness markers may have created the dir; alias it.
        let alias = f.home().join(".claude/skills");
        let _ = fs::remove_dir_all(&alias);
        std::os::unix::fs::symlink(f.home().join(".cursor/skills"), &alias).unwrap();
        let plan = build_onboarding_plan(f.home(), &f.paths.central_dir, &f.store, &scope).unwrap();
        assert_eq!(
            plan.copy_count, 0,
            "{scope:?}: physical owned copy must not be offered under another Tool alias: {:?}",
            plan.groups
        );
    }
}

/// F12: an unmanaged copy reached through two aliased Tool skills dirs is
/// one variant, its readers the union of both dirs' in-scope readers.
#[cfg(unix)]
#[test]
fn f12_unmanaged_copy_is_deduped_through_a_symlinked_tool_dir() {
    use crate::core::onboarding::{build_onboarding_plan, OnboardingScanScope};

    for scope in [
        OnboardingScanScope::Selected(vec!["cursor".into(), "claude_code".into()]),
        OnboardingScanScope::Installed,
    ] {
        let f = fixture();
        install(&f, "cursor");
        install(&f, "claude_code");
        let own = f.home().join(".cursor/skills/skill");
        copy_of_central(&f, &own);
        let alias = f.home().join(".claude/skills");
        let _ = fs::remove_dir_all(&alias);
        std::os::unix::fs::symlink(f.home().join(".cursor/skills"), &alias).unwrap();
        let plan = build_onboarding_plan(f.home(), &f.paths.central_dir, &f.store, &scope).unwrap();
        assert_eq!(
            plan.copy_count, 1,
            "{scope:?}: one physical copy must be one variant"
        );
        let readers = &plan.groups[0].variants[0].readers;
        for key in ["cursor", "claude_code"] {
            assert!(readers.iter().any(|r| r == key), "{scope:?}: {readers:?}");
        }
    }
}

/// F13: while an owned member of the effective group still sits on a
/// superseded path, Propagation never first-syncs a non-owner partition into
/// the effective root (a silent second copy) — it is a typed
/// `RELOCATION_PENDING` skip; the owned partition still updates in place.
#[cfg(unix)]
#[test]
fn f13_propagation_nonowner_does_not_add_second_cursor_copy() {
    use crate::core::errors::CommandError;
    use crate::core::propagation::{PropagationScope, PropagationSkip, PropagationStatus};

    let f = fixture();
    install(&f, "cursor");
    install(&f, "codex");
    let own = f.home().join(".cursor/skills/skill");
    let agents = f.home().join(".agents/skills/skill");
    copy_of_central(&f, &own);
    seed_tool_row(&f, "cursor-row", "cursor", &own);
    let mut cursor = row_of(&f, "cursor");
    cursor.mode = SyncMode::Copy;
    f.store.upsert_skill_target(&cursor).unwrap();
    let mut marker = cursor.clone();
    marker.id = "codex-row".into();
    marker.tool = "codex".into();
    marker.target_path = agents.to_string_lossy().into_owned();
    marker.status = SyncStatus::Error;
    marker.synced_at = None;
    f.store.upsert_skill_target(&marker).unwrap();
    // A real refused move: the central copy is absent on preference save.
    let backup = f.paths.central_dir.join("backup");
    fs::rename(&f.central_path, &backup).unwrap();
    let refused = crate::core::target_relocation::apply_global_layout(
        &f.store,
        f.home(),
        GlobalLayout::PREFER_AGENTS_STANDARD,
        NOW,
    )
    .unwrap();
    assert_eq!(refused.left_in_place, 1);
    fs::rename(&backup, &f.central_path).unwrap();
    fs::write(f.central_path.join("a.txt"), "v2").unwrap();

    let report = propagate_unlocked(&f.store, &f.paths, "skill-1", NOW).unwrap();
    assert!(
        !(own.exists() && agents.exists()),
        "Update created the own-dir plus AGENTS duplicate the preference promises to stop: {report:?}"
    );
    assert_eq!(
        fs::read_to_string(own.join("a.txt")).unwrap(),
        "v2",
        "owned copy updated in place"
    );
    let status_of = |tool: &str| {
        report
            .targets
            .iter()
            .find(|t| t.scope == PropagationScope::Global { tool: tool.into() })
            .map(|t| &t.status)
            .unwrap_or_else(|| panic!("{tool} outcome in {report:?}"))
    };
    assert!(matches!(
        status_of("cursor"),
        PropagationStatus::Synced { .. }
    ));
    match status_of("codex") {
        PropagationStatus::Skipped {
            reason:
                PropagationSkip::RelocationPending {
                    error: CommandError::RelocationPending { tool, path },
                },
        } => {
            assert_eq!(tool, "cursor");
            assert_eq!(Path::new(path), own);
        }
        other => panic!("codex must wait for the relocation: {other:?}"),
    }
    let codex = row_of(&f, "codex");
    assert!(!codex.owns_artifact(), "the skipped marker is not written");
    assert!(
        f.store
            .get_skill_target("skill-1", "augment")
            .unwrap()
            .is_none(),
        "no sibling is recorded by a skipped first sync"
    );
}

/// F10 through a first sync: the batch writes the shared dir and hands
/// Cursor's owned row to relocation; when the old copy cannot be removed the
/// move rolls back — the row stays at its source as `error`, the pair reports
/// `Failed`, never `Synced`, and the new copy stays only if a sharer records
/// it (Augment, when installed); otherwise it is removed.
#[cfg(unix)]
#[test]
fn f10_a_first_sync_whose_old_copy_cannot_be_removed_fails_and_rolls_back() {
    use crate::core::artifact_removal::artifact_is_owned;
    use crate::core::global_sync::{
        sync_skills_to_tools, BatchPolicy, BatchSkill, BatchTargetStatus,
    };
    use std::os::unix::fs::PermissionsExt;

    for augment_installed in [true, false] {
        let f = fixture();
        if !augment_installed {
            fs::remove_dir_all(f.home().join(".augment")).unwrap();
        }
        install(&f, "cursor");
        let own = f.home().join(".cursor/skills/skill");
        let agents = f.home().join(".agents/skills/skill");
        link(&f.central_path, &own);
        seed_tool_row(&f, "cursor-row", "cursor", &own);
        crate::core::settings::write_prefer_agents_standard_dir(&f.store, true).unwrap();

        let parent = own.parent().unwrap().to_path_buf();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o500)).unwrap();
        let outcomes = sync_skills_to_tools(
            f.home(),
            &f.store,
            &[BatchSkill {
                skill_id: "skill-1".into(),
                skill_name: "skill".into(),
                source_path: f.central_path.clone(),
            }],
            &["cursor".into()],
            &BatchPolicy {
                overwrite_if_same_content: true,
                ..Default::default()
            },
            NOW,
            |_| {},
        );
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();

        let cursor = outcomes.iter().find(|o| o.tool_key == "cursor").unwrap();
        assert!(
            matches!(cursor.status, BatchTargetStatus::Failed { .. }),
            "{outcomes:?}"
        );
        let row = row_of(&f, "cursor");
        assert_eq!(Path::new(&row.target_path), own);
        assert_eq!(row.status, SyncStatus::Error);
        assert!(row.owns_artifact());
        assert!(own.symlink_metadata().is_ok());
        if augment_installed {
            assert_eq!(Path::new(&row_of(&f, "augment").target_path), agents);
            assert!(artifact_is_owned(&f.store, &agents).unwrap());
        } else {
            assert!(
                agents.symlink_metadata().is_err(),
                "the shared-dir copy no row records was rolled back"
            );
        }
    }
}
