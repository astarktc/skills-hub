//! Tests for `core::onboarding_import` — Onboarding import as one operation.
//!
//! Every case runs against a temp home / central dir / DB; a Tool is "installed"
//! by creating its detect dir under the temp home, and its pre-existing skills
//! are written straight into its global skills dir. No test touches the
//! operator's real home, central repo, or database.

use std::fs;
use std::path::{Path, PathBuf};

use crate::core::installer::InstallerPaths;
use crate::core::onboarding_import::{
    import_onboarding_selection, ImportGroupStatus, ImportPhase, ImportPolicy, ImportSelection,
    OriginalStatus,
};
use crate::core::skill_store::SkillStore;
use crate::core::sync_status::SyncMode;
use crate::core::tool_adapters::adapter_by_key;

struct Fixture {
    _dir: tempfile::TempDir,
    paths: InstallerPaths,
    store: SkillStore,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = InstallerPaths {
        home: dir.path().join("home"),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.home).expect("create home");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    Fixture {
        _dir: dir,
        paths,
        store,
    }
}

/// Mark a Tool installed for this fixture's operator.
fn install_tool(f: &Fixture, key: &str) {
    let adapter = adapter_by_key(key).unwrap_or_else(|| panic!("adapter {}", key));
    fs::create_dir_all(f.paths.home.join(adapter.relative_detect_dir)).expect("install tool");
}

/// A pre-existing skill directory inside a Tool's global skills dir.
fn seed_skill_dir(f: &Fixture, key: &str, name: &str, body: &str) -> PathBuf {
    let adapter = adapter_by_key(key).unwrap_or_else(|| panic!("adapter {}", key));
    let dir = f.paths.home.join(adapter.relative_skills_dir).join(name);
    fs::create_dir_all(&dir).expect("create skill dir");
    fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {}\n---\n{}\n", name, body),
    )
    .expect("write SKILL.md");
    dir
}

fn selection(group_name: &str, chosen: &Path) -> ImportSelection {
    ImportSelection {
        group_name: group_name.to_string(),
        chosen_path: chosen.to_path_buf(),
        name: None,
    }
}

fn run(
    f: &Fixture,
    selections: &[ImportSelection],
    policy: ImportPolicy,
) -> crate::core::onboarding_import::ImportReport {
    import_onboarding_selection(&f.paths, &f.store, selections, &policy, 5000, |_| {})
        .expect("import reads its own plan")
}

fn imported<'a>(
    report: &'a crate::core::onboarding_import::ImportReport,
    group_name: &str,
) -> &'a ImportGroupStatus {
    &report
        .groups
        .iter()
        .find(|g| g.group_name == group_name)
        .unwrap_or_else(|| panic!("group {} in report", group_name))
        .status
}

#[test]
fn auto_sync_on_overwrites_the_source_tool_in_place_across_its_shared_dir_group() {
    // amp and kimi_cli share `.config/agents/skills` (one detect dir, one
    // artifact): the source variant's own path IS one of the sync targets.
    let f = fixture();
    install_tool(&f, "amp");
    let original = seed_skill_dir(&f, "amp", "alpha", "v1");

    let report = run(
        &f,
        &[selection("alpha", &original)],
        ImportPolicy {
            auto_sync: true,
            tools: Some(vec!["amp".to_string(), "kimi_cli".to_string()]),
        },
    );

    let ImportGroupStatus::Imported {
        skill_id,
        targets,
        originals,
        forced_source_tool,
        ..
    } = imported(&report, "alpha")
    else {
        panic!("alpha should import: {:?}", report);
    };
    assert!(originals.is_empty(), "auto-sync on removes nothing");
    assert_eq!(
        *forced_source_tool, None,
        "a source Tool the policy already names is not a forced inclusion"
    );
    // One artifact, one attempted pair (shared dir dedupe), both rows written.
    assert_eq!(targets.len(), 1, "shared dir attempted once: {:?}", targets);
    assert!(
        matches!(
            targets[0].status,
            crate::core::global_sync::BatchTargetStatus::Synced { .. }
        ),
        "source tool overwritten in place: {:?}",
        targets[0].status
    );
    for key in ["amp", "kimi_cli"] {
        let row = f
            .store
            .get_skill_target(skill_id, key)
            .expect("query")
            .unwrap_or_else(|| panic!("row for {}", key));
        assert_eq!(row.target_path, original.to_string_lossy());
    }
    // The central copy holds the bytes and the original path now carries the
    // Sync target (a link on this platform, a copy elsewhere).
    let central = f.paths.central_dir.join("alpha");
    assert!(central.join("SKILL.md").is_file(), "central copy exists");
    assert!(original.join("SKILL.md").is_file(), "target materialised");
}

#[test]
fn auto_sync_on_force_includes_a_source_tool_the_policy_deselected() {
    // The operator's global auto-sync selection excludes claude_code, yet
    // that is where the chosen variant lives. Leaving its original behind
    // would strand an untracked copy: the source Tool joins the target set
    // and its original is overwritten in place, exactly as a selected one.
    let f = fixture();
    install_tool(&f, "claude_code");
    install_tool(&f, "cursor");
    let original = seed_skill_dir(&f, "claude_code", "alpha", "v1");

    let report = run(
        &f,
        &[selection("alpha", &original)],
        ImportPolicy {
            auto_sync: true,
            tools: Some(vec!["cursor".to_string()]),
        },
    );

    let ImportGroupStatus::Imported {
        skill_id,
        targets,
        originals,
        forced_source_tool,
        ..
    } = imported(&report, "alpha")
    else {
        panic!("alpha should import: {:?}", report);
    };
    assert!(originals.is_empty(), "auto-sync on removes nothing");
    assert_eq!(
        forced_source_tool.as_deref(),
        Some("claude_code"),
        "the report names the Tool included beyond the policy"
    );
    // The policy's Tool and the source Tool are both synced, nothing else.
    let mut synced: Vec<&str> = targets
        .iter()
        .filter(|t| {
            matches!(
                t.status,
                crate::core::global_sync::BatchTargetStatus::Synced { .. }
            )
        })
        .map(|t| t.tool_key.as_str())
        .collect();
    synced.sort_unstable();
    assert_eq!(synced, vec!["claude_code", "cursor"], "{:?}", targets);
    assert_eq!(targets.len(), 2, "{:?}", targets);

    // A target row exists for the source Tool and its dir holds the Sync
    // target (a link on this platform), not the untracked original copy.
    let row = f
        .store
        .get_skill_target(skill_id, "claude_code")
        .expect("query")
        .expect("row for the source Tool");
    assert_eq!(row.target_path, original.to_string_lossy());
    assert_eq!(row.mode, SyncMode::Symlink);
    assert!(
        original
            .symlink_metadata()
            .expect("target")
            .file_type()
            .is_symlink(),
        "the original at {:?} was replaced by a link",
        original
    );
    assert_eq!(
        fs::read_link(&original).expect("link"),
        f.paths.central_dir.join("alpha"),
        "the link points at the central copy"
    );
    assert!(f.paths.central_dir.join("alpha").join("SKILL.md").is_file());
}

#[test]
fn auto_sync_off_removes_identical_originals() {
    let f = fixture();
    install_tool(&f, "claude_code");
    install_tool(&f, "cursor");
    let chosen = seed_skill_dir(&f, "claude_code", "alpha", "same");
    let sibling = seed_skill_dir(&f, "cursor", "alpha", "same");

    let report = run(&f, &[selection("alpha", &chosen)], ImportPolicy::default());

    let ImportGroupStatus::Imported {
        targets, originals, ..
    } = imported(&report, "alpha")
    else {
        panic!("alpha should import: {:?}", report);
    };
    assert!(targets.is_empty(), "auto-sync off syncs nothing");
    assert_eq!(originals.len(), 2);
    for outcome in originals {
        assert!(
            matches!(outcome.status, OriginalStatus::Removed),
            "{:?} should be removed: {:?}",
            outcome.path,
            outcome.status
        );
    }
    assert!(!chosen.exists(), "the chosen variant's own path is removed");
    assert!(!sibling.exists(), "an identical sibling is removed");
    assert!(f.paths.central_dir.join("alpha").join("SKILL.md").is_file());
}

#[test]
fn auto_sync_off_keeps_and_reports_a_divergent_sibling() {
    let f = fixture();
    install_tool(&f, "claude_code");
    install_tool(&f, "cursor");
    let chosen = seed_skill_dir(&f, "claude_code", "alpha", "chosen-body");
    let divergent = seed_skill_dir(&f, "cursor", "alpha", "different-body");

    let report = run(&f, &[selection("alpha", &chosen)], ImportPolicy::default());

    let ImportGroupStatus::Imported { originals, .. } = imported(&report, "alpha") else {
        panic!("alpha should import: {:?}", report);
    };
    let kept = originals
        .iter()
        .find(|o| o.path == divergent)
        .expect("the sibling is reported");
    assert!(
        matches!(kept.status, OriginalStatus::KeptDivergent),
        "divergent sibling is kept: {:?}",
        kept.status
    );
    assert_eq!(kept.tool, "cursor");
    assert!(divergent.join("SKILL.md").is_file(), "still on disk");
    assert!(!chosen.exists(), "the identical chosen variant still goes");
}

#[test]
fn a_group_failing_admission_does_not_stop_the_others() {
    let f = fixture();
    install_tool(&f, "claude_code");
    let adapter = adapter_by_key("claude_code").expect("adapter");
    // A directory in the skills dir with no SKILL.md: discovered, not admissible.
    let broken = f
        .paths
        .home
        .join(adapter.relative_skills_dir)
        .join("broken");
    fs::create_dir_all(&broken).expect("create broken dir");
    fs::write(broken.join("notes.txt"), "no manifest").expect("write");
    let good = seed_skill_dir(&f, "claude_code", "alpha", "v1");

    let report = run(
        &f,
        &[selection("broken", &broken), selection("alpha", &good)],
        ImportPolicy::default(),
    );

    assert!(
        matches!(
            imported(&report, "broken"),
            ImportGroupStatus::Failed { .. }
        ),
        "broken fails admission: {:?}",
        report
    );
    assert!(
        matches!(
            imported(&report, "alpha"),
            ImportGroupStatus::Imported { .. }
        ),
        "alpha still imports: {:?}",
        report
    );
    assert!(broken.exists(), "a failed group is left untouched");
    assert!(!f.paths.central_dir.join("broken").exists());
    assert!(f.paths.central_dir.join("alpha").join("SKILL.md").is_file());
}

#[test]
fn a_finalize_failure_mid_batch_is_reported_per_group() {
    let f = fixture();
    install_tool(&f, "claude_code");
    let first = seed_skill_dir(&f, "claude_code", "alpha", "v1");
    let clash = seed_skill_dir(&f, "claude_code", "beta", "v1");
    let last = seed_skill_dir(&f, "claude_code", "gamma", "v1");
    // `beta` is already a Managed skill name: finalize refuses the collision.
    fs::create_dir_all(f.paths.central_dir.join("beta")).expect("occupy central name");

    let report = run(
        &f,
        &[
            selection("alpha", &first),
            selection("beta", &clash),
            selection("gamma", &last),
        ],
        ImportPolicy::default(),
    );

    assert!(matches!(
        imported(&report, "alpha"),
        ImportGroupStatus::Imported { .. }
    ));
    assert!(
        matches!(imported(&report, "beta"), ImportGroupStatus::Failed { .. }),
        "the collision is this group's failure: {:?}",
        report
    );
    assert!(
        matches!(
            imported(&report, "gamma"),
            ImportGroupStatus::Imported { .. }
        ),
        "the batch continues past a failed group: {:?}",
        report
    );
    assert!(clash.exists(), "a failed group's original is left alone");
    assert!(!first.exists());
    assert!(!last.exists());
}

#[test]
fn a_path_the_plan_does_not_own_fails_the_group_without_touching_disk() {
    let f = fixture();
    install_tool(&f, "claude_code");
    let real = seed_skill_dir(&f, "claude_code", "alpha", "v1");
    let elsewhere = f.paths.home.join("Documents").join("alpha");
    fs::create_dir_all(&elsewhere).expect("create dir");
    fs::write(elsewhere.join("SKILL.md"), "---\nname: alpha\n---\n").expect("write");

    let report = run(
        &f,
        &[selection("alpha", &elsewhere)],
        ImportPolicy::default(),
    );

    assert!(
        matches!(imported(&report, "alpha"), ImportGroupStatus::Failed { .. }),
        "a stale UI cannot name a path outside the group: {:?}",
        report
    );
    assert!(elsewhere.exists());
    assert!(real.exists());
}

#[test]
fn progress_reports_both_phases_of_every_group() {
    let f = fixture();
    install_tool(&f, "claude_code");
    let alpha = seed_skill_dir(&f, "claude_code", "alpha", "v1");
    let beta = seed_skill_dir(&f, "claude_code", "beta", "v1");

    let mut ticks: Vec<(usize, usize, String, ImportPhase)> = Vec::new();
    import_onboarding_selection(
        &f.paths,
        &f.store,
        &[selection("alpha", &alpha), selection("beta", &beta)],
        &ImportPolicy::default(),
        5000,
        |p| ticks.push((p.index, p.total, p.group_name.to_string(), p.phase)),
    )
    .expect("import");

    assert_eq!(
        ticks,
        vec![
            (1, 2, "alpha".to_string(), ImportPhase::Admitting),
            (1, 2, "alpha".to_string(), ImportPhase::Applying),
            (2, 2, "beta".to_string(), ImportPhase::Admitting),
            (2, 2, "beta".to_string(), ImportPhase::Applying),
        ]
    );
}
