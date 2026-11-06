//! Tests for `core::onboarding_import` — Onboarding import as one operation.
//!
//! Every case runs against a temp home / central dir / DB; a Tool is "installed"
//! by creating its detect dir under the temp home, and its pre-existing skills
//! are written straight into its global skills dir. No test touches the
//! operator's real home, central repo, or database.

use crate::core::environment::HostRoots;
use crate::core::removal_fence::central_repo_of;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::installer::InstallerPaths;
use crate::core::onboarding::OnboardingScanScope;
use crate::core::onboarding_import::{
    import_onboarding_selection, ImportGroupStatus, ImportPhase, ImportPolicy, ImportSelection,
    ImportSkipReason, OriginalStatus,
};
use crate::core::skill_store::SkillStore;
use crate::core::sync_status::SyncMode;
use crate::core::tool_adapters::{adapter_by_key, mark_installed_in, skills_dir_in};

struct Fixture {
    _dir: tempfile::TempDir,
    paths: InstallerPaths,
    store: SkillStore,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = InstallerPaths {
        roots: HostRoots::under_home(&dir.path().join("home")),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.roots.home).expect("create home");
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
    mark_installed_in(&f.paths.roots, adapter);
}

/// A pre-existing skill directory inside a Tool's global skills dir.
fn seed_skill_dir(f: &Fixture, key: &str, name: &str, body: &str) -> PathBuf {
    let adapter = adapter_by_key(key).unwrap_or_else(|| panic!("adapter {}", key));
    let dir = skills_dir_in(&f.paths.roots, adapter).join(name);
    fs::create_dir_all(&dir).expect("create skill dir");
    fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {}\n---\n{}\n", name, body),
    )
    .expect("write SKILL.md");
    dir
}

/// A Tool's variant that is a symlink to another Tool's real skill dir —
/// the shape the operator's library has when one Tool was linked into
/// another by hand (or by an earlier tool's sync).
fn seed_skill_link(f: &Fixture, key: &str, name: &str, target: &Path) -> PathBuf {
    let adapter = adapter_by_key(key).unwrap_or_else(|| panic!("adapter {}", key));
    let skills_dir = skills_dir_in(&f.paths.roots, adapter);
    fs::create_dir_all(&skills_dir).expect("create skills dir");
    let link = skills_dir.join(name);
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, &link).expect("symlink");
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(target, &link).expect("symlink");
    link
}

fn assert_links_to_central(f: &Fixture, path: &Path, name: &str) {
    assert!(
        path.symlink_metadata()
            .expect("target present")
            .file_type()
            .is_symlink(),
        "{:?} should be a link",
        path
    );
    assert_eq!(
        fs::read_link(path).expect("link"),
        f.paths.central_dir.join(name),
        "{:?} should point at the central copy",
        path
    );
}

fn selection(group_name: &str, chosen: &Path) -> ImportSelection {
    ImportSelection {
        group_name: group_name.to_string(),
        chosen_path: chosen.to_path_buf(),
        name: None,
        replace: false,
        remove_divergent: false,
    }
}

/// Auto-sync off, with the operator's per-run choice about the originals.
fn policy_off(remove_originals: bool) -> ImportPolicy {
    ImportPolicy {
        auto_sync: false,
        remove_originals,
        tools: None,
        scan_scope: OnboardingScanScope::Installed,
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

/// The record an import writes: provenance `imported`, no source path, and
/// the Tool the chosen variant was found in kept as history (Q1/Q2).
#[test]
fn import_records_imported_provenance_with_the_found_in_tool_as_history() {
    let f = fixture();
    install_tool(&f, "claude_code");
    let chosen = seed_skill_dir(&f, "claude_code", "alpha", "body");

    let report = run(&f, &[selection("alpha", &chosen)], policy_off(true));

    let ImportGroupStatus::Imported { skill_id, .. } = imported(&report, "alpha") else {
        panic!("alpha should import: {:?}", report);
    };
    let record = f
        .store
        .get_skill_by_id(skill_id)
        .expect("query")
        .expect("record");
    assert_eq!(record.source_type, "imported");
    assert_eq!(
        record.source_ref, None,
        "an imported skill has no source path"
    );
    assert_eq!(record.source_subpath, None);
    assert_eq!(record.source_revision, None);
    assert_eq!(record.imported_from_tool.as_deref(), Some("claude_code"));
    assert!(
        !crate::core::provenance::is_refreshable(&record),
        "the central copy is its truth: nothing to refresh from"
    );
}

/// A Tool copy that `.skill-lock.json` identifies as installed by
/// `npx skills add` has a real upstream: it is recorded `git`, not `imported`.
#[test]
fn import_of_a_variant_installed_by_npx_skills_add_records_its_upstream_as_git() {
    let f = fixture();
    install_tool(&f, "claude_code");
    // `npx skills add` keeps the bytes in ~/.agents/skills/<name> and links
    // every Tool at it; the lock file names the upstream.
    let agents_dir = f.paths.roots.home.join(".agents");
    let real = agents_dir.join("skills").join("alpha");
    fs::create_dir_all(&real).expect("agents skills dir");
    fs::write(real.join("SKILL.md"), "---\nname: alpha\n---\nbody\n").expect("SKILL.md");
    fs::write(
        agents_dir.join(".skill-lock.json"),
        r#"{ "version": 3, "skills": { "alpha": {
            "source": "owner/repo", "sourceType": "github",
            "sourceUrl": "https://github.com/owner/repo.git",
            "skillPath": "skills/alpha/SKILL.md", "skillFolderHash": "abc" } } }"#,
    )
    .expect("lock file");
    let chosen = seed_skill_link(&f, "claude_code", "alpha", &real);

    let report = run(&f, &[selection("alpha", &chosen)], policy_off(true));

    let ImportGroupStatus::Imported { skill_id, .. } = imported(&report, "alpha") else {
        panic!("alpha should import: {:?}", report);
    };
    let record = f
        .store
        .get_skill_by_id(skill_id)
        .expect("query")
        .expect("record");
    assert_eq!(record.source_type, "git");
    assert_eq!(
        record.source_ref.as_deref(),
        Some("https://github.com/owner/repo.git")
    );
    assert_eq!(record.source_subpath.as_deref(), Some("skills/alpha"));
    assert_eq!(record.imported_from_tool, None);
    assert!(crate::core::provenance::is_refreshable(&record));
}

#[test]
fn auto_sync_on_overwrites_the_source_tool_in_place_across_its_shared_dir_group() {
    // amp and kimi_cli share `.config/agents/skills` (one detect dir, one
    // artifact): the source variant's own path IS one of the sync targets.
    let f = fixture();
    install_tool(&f, "amp");
    install_tool(&f, "kimi_cli");
    let original = seed_skill_dir(&f, "amp", "alpha", "v1");

    let report = run(
        &f,
        &[selection("alpha", &original)],
        ImportPolicy {
            auto_sync: true,
            remove_originals: true,
            tools: Some(vec!["amp".to_string(), "kimi_cli".to_string()]),
            scan_scope: OnboardingScanScope::Installed,
        },
    );

    let ImportGroupStatus::Imported {
        skill_id,
        targets,
        originals,
        forced_tools,
        ..
    } = imported(&report, "alpha")
    else {
        panic!("alpha should import: {:?}", report);
    };
    assert!(originals.is_empty(), "auto-sync on removes nothing");
    assert!(
        forced_tools.is_empty(),
        "a source Tool the policy already names is not a forced inclusion: {:?}",
        forced_tools
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
            remove_originals: true,
            tools: Some(vec!["cursor".to_string()]),
            scan_scope: OnboardingScanScope::Installed,
        },
    );

    let ImportGroupStatus::Imported {
        skill_id,
        targets,
        originals,
        forced_tools,
        ..
    } = imported(&report, "alpha")
    else {
        panic!("alpha should import: {:?}", report);
    };
    assert!(originals.is_empty(), "auto-sync on removes nothing");
    assert_eq!(
        *forced_tools,
        vec!["claude_code".to_string()],
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
fn auto_sync_on_takes_over_every_identical_original_in_the_group() {
    // The operator's smoke-test shape: pi holds the real dir, claude_code a
    // symlink to it — one group, same fingerprint, no conflict. The policy
    // deselects pi. Both Tools must end holding a link to the central copy;
    // pi is reported as the Tool included beyond the policy.
    let f = fixture();
    install_tool(&f, "pi");
    install_tool(&f, "claude_code");
    let real = seed_skill_dir(&f, "pi", "alpha", "v1");
    let link = seed_skill_link(&f, "claude_code", "alpha", &real);

    let report = run(
        &f,
        &[selection("alpha", &real)],
        ImportPolicy {
            auto_sync: true,
            remove_originals: true,
            tools: Some(vec!["claude_code".to_string()]),
            scan_scope: OnboardingScanScope::Installed,
        },
    );

    let ImportGroupStatus::Imported {
        skill_id,
        targets,
        originals,
        forced_tools,
        ..
    } = imported(&report, "alpha")
    else {
        panic!("alpha should import: {:?}", report);
    };
    assert_eq!(*forced_tools, vec!["pi".to_string()], "{:?}", report);
    assert!(
        originals.is_empty(),
        "no divergent sibling to report: {:?}",
        originals
    );
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
    assert_eq!(synced, vec!["claude_code", "pi"], "{:?}", targets);
    assert_eq!(targets.len(), 2, "{:?}", targets);

    for (key, path) in [("pi", &real), ("claude_code", &link)] {
        let row = f
            .store
            .get_skill_target(skill_id, key)
            .expect("query")
            .unwrap_or_else(|| panic!("row for {}", key));
        assert_eq!(row.target_path, path.to_string_lossy());
        assert_eq!(row.mode, SyncMode::Symlink);
        assert_links_to_central(&f, path, "alpha");
    }
    assert!(f.paths.central_dir.join("alpha").join("SKILL.md").is_file());
}

#[test]
fn auto_sync_on_leaves_a_divergent_sibling_in_place_and_reports_it() {
    // pi (real dir) and claude_code (link to it) are identical; cursor holds
    // a same-named skill with different bytes. Sharing a name never proved
    // it was the same skill: cursor's copy is neither overwritten nor
    // force-included — it is kept and reported, as the auto-sync-off path
    // already does.
    let f = fixture();
    install_tool(&f, "pi");
    install_tool(&f, "claude_code");
    install_tool(&f, "cursor");
    let real = seed_skill_dir(&f, "pi", "alpha", "same");
    let link = seed_skill_link(&f, "claude_code", "alpha", &real);
    let divergent = seed_skill_dir(&f, "cursor", "alpha", "different");
    let divergent_bytes = fs::read(divergent.join("SKILL.md")).expect("read");

    let report = run(
        &f,
        &[selection("alpha", &real)],
        ImportPolicy {
            auto_sync: true,
            remove_originals: true,
            tools: Some(vec!["claude_code".to_string()]),
            scan_scope: OnboardingScanScope::Installed,
        },
    );

    let ImportGroupStatus::Imported {
        targets,
        originals,
        forced_tools,
        ..
    } = imported(&report, "alpha")
    else {
        panic!("alpha should import: {:?}", report);
    };
    assert_eq!(*forced_tools, vec!["pi".to_string()], "{:?}", report);
    assert!(
        !targets.iter().any(|t| t.tool_key == "cursor"),
        "a divergent sibling's Tool is never force-included: {:?}",
        targets
    );
    assert_eq!(originals.len(), 1, "{:?}", originals);
    let kept = &originals[0];
    assert_eq!(kept.path, divergent);
    assert_eq!(kept.tool, "cursor");
    assert!(
        matches!(kept.status, OriginalStatus::KeptDivergent),
        "divergent sibling is kept: {:?}",
        kept.status
    );
    assert!(
        !divergent
            .symlink_metadata()
            .expect("still present")
            .file_type()
            .is_symlink(),
        "the divergent copy stays a real directory"
    );
    assert_eq!(
        fs::read(divergent.join("SKILL.md")).expect("read"),
        divergent_bytes,
        "the divergent copy's bytes are untouched"
    );
    assert_links_to_central(&f, &real, "alpha");
    assert_links_to_central(&f, &link, "alpha");
}

#[test]
fn auto_sync_on_keeps_a_sibling_that_diverged_after_planning() {
    let f = fixture();
    install_tool(&f, "pi");
    install_tool(&f, "cursor");
    let chosen = seed_skill_dir(&f, "pi", "alpha", "chosen");
    let sibling = seed_skill_dir(&f, "cursor", "alpha", "chosen");
    let chosen_bytes = fs::read(chosen.join("SKILL.md")).expect("read chosen");
    let changed_bytes = "---\nname: alpha\n---\nchanged after planning\n";

    let report = import_onboarding_selection(
        &f.paths,
        &f.store,
        &[selection("alpha", &chosen)],
        &ImportPolicy {
            auto_sync: true,
            remove_originals: true,
            tools: Some(vec!["cursor".to_string()]),
            scan_scope: OnboardingScanScope::Installed,
        },
        5000,
        |progress| {
            if progress.phase == ImportPhase::Applying {
                fs::write(sibling.join("SKILL.md"), changed_bytes).expect("external edit");
            }
        },
    )
    .expect("import");

    assert_eq!(
        fs::read_to_string(sibling.join("SKILL.md")).expect("read sibling"),
        changed_bytes,
        "a stale fingerprint must not authorize overwriting an external edit"
    );
    assert_eq!(
        fs::read(f.paths.central_dir.join("alpha/SKILL.md")).expect("read central"),
        chosen_bytes,
        "central retains the chosen bytes"
    );
    assert_links_to_central(&f, &chosen, "alpha");
    let ImportGroupStatus::Imported {
        originals,
        forced_tools,
        targets,
        ..
    } = imported(&report, "alpha")
    else {
        panic!("alpha should import: {:?}", report);
    };
    assert_eq!(*forced_tools, vec!["pi".to_string()]);
    // Reported once (ticket 04): the sibling in the target's way is the
    // pair's askable `TARGET_EXISTS` row, carrying the ask's facts — not
    // also a kept original.
    assert!(originals.is_empty(), "{originals:?}");
    assert!(targets.iter().any(|target| {
        target.tool_key == "cursor"
            && matches!(
                &target.status,
                crate::core::global_sync::BatchTargetStatus::Failed {
                    error: crate::core::errors::CommandError::TargetExists { path, modified_at: Some(_), same_content: false },
                } if Path::new(path) == sibling
            )
    }), "{targets:?}");
}

#[test]
fn auto_sync_on_reports_nothing_beyond_a_policy_naming_every_identical_tool() {
    let f = fixture();
    install_tool(&f, "pi");
    install_tool(&f, "claude_code");
    let real = seed_skill_dir(&f, "pi", "alpha", "v1");
    let link = seed_skill_link(&f, "claude_code", "alpha", &real);

    let report = run(
        &f,
        &[selection("alpha", &real)],
        ImportPolicy {
            auto_sync: true,
            remove_originals: true,
            tools: Some(vec!["claude_code".to_string(), "pi".to_string()]),
            scan_scope: OnboardingScanScope::Installed,
        },
    );

    let ImportGroupStatus::Imported {
        targets,
        originals,
        forced_tools,
        ..
    } = imported(&report, "alpha")
    else {
        panic!("alpha should import: {:?}", report);
    };
    assert!(
        forced_tools.is_empty(),
        "every identical Tool was already in the policy: {:?}",
        forced_tools
    );
    assert!(originals.is_empty(), "{:?}", originals);
    assert_eq!(targets.len(), 2, "{:?}", targets);
    assert!(
        targets.iter().all(|t| matches!(
            t.status,
            crate::core::global_sync::BatchTargetStatus::Synced { .. }
        )),
        "{:?}",
        targets
    );
    assert_links_to_central(&f, &real, "alpha");
    assert_links_to_central(&f, &link, "alpha");
}

#[cfg(unix)]
#[test]
fn chosen_original_with_internal_symlinks_is_same_content() {
    for auto_sync in [false, true] {
        let f = fixture();
        install_tool(&f, "claude_code");
        let chosen = seed_skill_dir(&f, "claude_code", "alpha", "same");
        std::os::unix::fs::symlink("SKILL.md", chosen.join("relative")).unwrap();
        std::os::unix::fs::symlink("missing", chosen.join("dangling")).unwrap();

        let report = run(
            &f,
            &[selection("alpha", &chosen)],
            ImportPolicy {
                auto_sync,
                remove_originals: true,
                tools: Some(vec![]),
                scan_scope: OnboardingScanScope::Installed,
            },
        );
        let ImportGroupStatus::Imported {
            originals,
            forced_tools,
            ..
        } = imported(&report, "alpha")
        else {
            panic!("alpha should import: {:?}", report);
        };
        if auto_sync {
            assert!(
                originals.is_empty(),
                "chosen original must not be KeptDivergent"
            );
            assert!(forced_tools.iter().any(|tool| tool == "claude_code"));
            assert_links_to_central(&f, &chosen, "alpha");
        } else {
            assert_eq!(originals.len(), 1);
            assert_eq!(originals[0].path, chosen);
            assert!(matches!(originals[0].status, OriginalStatus::Removed));
            assert!(!chosen.exists());
        }
        let central = f.paths.central_dir.join("alpha");
        assert!(central.join("SKILL.md").is_file());
        for name in ["relative", "dangling"] {
            assert!(fs::symlink_metadata(central.join(name)).is_err());
        }
    }
}

#[test]
fn auto_sync_off_removes_identical_originals() {
    let f = fixture();
    install_tool(&f, "claude_code");
    install_tool(&f, "cursor");
    let chosen = seed_skill_dir(&f, "claude_code", "alpha", "same");
    let sibling = seed_skill_dir(&f, "cursor", "alpha", "same");

    let report = run(&f, &[selection("alpha", &chosen)], policy_off(true));

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

    let report = run(&f, &[selection("alpha", &chosen)], policy_off(true));

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
        .roots
        .home
        .join(adapter.skills_dir.rel)
        .join("broken");
    fs::create_dir_all(&broken).expect("create broken dir");
    fs::write(broken.join("notes.txt"), "no manifest").expect("write");
    let good = seed_skill_dir(&f, "claude_code", "alpha", "v1");

    let report = run(
        &f,
        &[selection("broken", &broken), selection("alpha", &good)],
        policy_off(true),
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
        policy_off(true),
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
    let elsewhere = f.paths.roots.home.join("Documents").join("alpha");
    fs::create_dir_all(&elsewhere).expect("create dir");
    fs::write(elsewhere.join("SKILL.md"), "---\nname: alpha\n---\n").expect("write");

    let report = run(&f, &[selection("alpha", &elsewhere)], policy_off(true));

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
        &policy_off(true),
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

#[test]
fn import_without_a_policy_selection_follows_the_recorded_global_selection() {
    // The fourth site of the "intent beats detection" rule
    // (`settings::effective_global_tool_targets`): with no per-import
    // selection, an import writes to the operator's recorded global selection,
    // not to every detected Tool.
    let f = fixture();
    install_tool(&f, "claude_code");
    install_tool(&f, "codex");
    crate::core::settings::apply_setting(
        &f.store,
        &crate::core::credentials::MemoryStore::new(),
        &f.paths.roots.home,
        crate::core::settings::SettingUpdate::GlobalToolConfig {
            selected_tools: vec!["claude_code".to_string()],
            scan_selected_only: false,
        },
    )
    .expect("save the selection");
    let alpha = seed_skill_dir(&f, "claude_code", "alpha", "v1");

    let report = run(
        &f,
        &[selection("alpha", &alpha)],
        ImportPolicy {
            auto_sync: true,
            remove_originals: true,
            tools: None,
            scan_scope: OnboardingScanScope::Installed,
        },
    );

    let ImportGroupStatus::Imported { targets, .. } = imported(&report, "alpha") else {
        panic!("alpha should import: {:?}", report);
    };
    let tools: Vec<&str> = targets.iter().map(|t| t.tool_key.as_str()).collect();
    assert_eq!(
        tools,
        vec!["claude_code"],
        "an installed-but-unselected Tool is never written to: {:?}",
        targets
    );
    assert!(
        !f.paths.roots.home.join(".codex/skills/alpha").exists(),
        "codex is detected but not selected, so it gets no artifact"
    );
}

// ── remove_originals × auto-sync ──────────────────────────────────────────

/// Auto-sync off, `remove_originals: false`: nothing on disk is touched and
/// every original is reported per Tool — identical ones `LeftInPlace`, a
/// divergent sibling `KeptDivergent`.
#[test]
fn auto_sync_off_without_remove_originals_leaves_and_reports_every_original() {
    let f = fixture();
    install_tool(&f, "claude_code");
    install_tool(&f, "cursor");
    install_tool(&f, "codex");
    let chosen = seed_skill_dir(&f, "claude_code", "alpha", "same");
    let sibling = seed_skill_dir(&f, "cursor", "alpha", "same");
    let divergent = seed_skill_dir(&f, "codex", "alpha", "different");

    let report = run(&f, &[selection("alpha", &chosen)], policy_off(false));

    let ImportGroupStatus::Imported {
        targets, originals, ..
    } = imported(&report, "alpha")
    else {
        panic!("alpha should import: {:?}", report);
    };
    assert!(targets.is_empty(), "auto-sync off syncs nothing");
    let by_tool: Vec<(&str, &OriginalStatus)> = originals
        .iter()
        .map(|o| (o.tool.as_str(), &o.status))
        .collect();
    assert_eq!(by_tool.len(), 3, "{:?}", originals);
    for (tool, status) in &by_tool {
        match *tool {
            "claude_code" | "cursor" => assert!(
                matches!(status, OriginalStatus::LeftInPlace),
                "{tool}: {status:?}"
            ),
            "codex" => assert!(
                matches!(status, OriginalStatus::KeptDivergent),
                "{tool}: {status:?}"
            ),
            other => panic!("unexpected Tool {other}"),
        }
    }
    for path in [&chosen, &sibling, &divergent] {
        assert!(
            path.join("SKILL.md").is_file(),
            "{:?} is left in place",
            path
        );
        assert!(
            !path.symlink_metadata().unwrap().file_type().is_symlink(),
            "{:?} stays a real directory",
            path
        );
    }
    assert!(f.paths.central_dir.join("alpha/SKILL.md").is_file());
}

/// Auto-sync on ignores `remove_originals`: whatever the value, the sync
/// batch takes the identical originals over, exactly as with `true`.
#[test]
fn auto_sync_on_takes_originals_over_whatever_remove_originals_says() {
    for remove_originals in [false, true] {
        let f = fixture();
        install_tool(&f, "pi");
        install_tool(&f, "claude_code");
        let real = seed_skill_dir(&f, "pi", "alpha", "v1");
        let link = seed_skill_link(&f, "claude_code", "alpha", &real);

        let report = run(
            &f,
            &[selection("alpha", &real)],
            ImportPolicy {
                auto_sync: true,
                remove_originals,
                tools: Some(vec!["claude_code".to_string()]),
                scan_scope: OnboardingScanScope::Installed,
            },
        );

        let ImportGroupStatus::Imported {
            originals,
            forced_tools,
            ..
        } = imported(&report, "alpha")
        else {
            panic!("alpha should import: {:?}", report);
        };
        assert!(originals.is_empty(), "{remove_originals}: {originals:?}");
        assert_eq!(*forced_tools, vec!["pi".to_string()]);
        assert_links_to_central(&f, &real, "alpha");
        assert_links_to_central(&f, &link, "alpha");
    }
}

// ── managed-name groups ───────────────────────────────────────────────────

/// Import `alpha` from claude_code (auto-sync off, originals removed), so a
/// Managed skill holds the name; returns its id.
fn manage_alpha(f: &Fixture, body: &str) -> String {
    install_tool(f, "claude_code");
    let chosen = seed_skill_dir(f, "claude_code", "alpha", body);
    let report = run(f, &[selection("alpha", &chosen)], policy_off(true));
    let ImportGroupStatus::Imported { skill_id, .. } = imported(&report, "alpha") else {
        panic!("alpha should import: {:?}", report);
    };
    skill_id.clone()
}

fn replace(group_name: &str, chosen: &Path) -> ImportSelection {
    ImportSelection {
        replace: true,
        ..selection(group_name, chosen)
    }
}

/// With no decision a managed-name group is skipped with a typed reason —
/// never the finalize collision — and nothing is touched: no applying tick.
#[test]
fn a_managed_name_group_without_a_decision_is_skipped_untouched() {
    let f = fixture();
    let skill_id = manage_alpha(&f, "managed");
    install_tool(&f, "cursor");
    let copy = seed_skill_dir(&f, "cursor", "alpha", "someone's fork");

    let mut phases = Vec::new();
    let report = import_onboarding_selection(
        &f.paths,
        &f.store,
        &[selection("alpha", &copy)],
        &policy_off(true),
        5000,
        |p| phases.push(p.phase),
    )
    .expect("import");

    match imported(&report, "alpha") {
        ImportGroupStatus::Skipped {
            reason: ImportSkipReason::NameManaged { skill_id: id },
        } => assert_eq!(*id, skill_id),
        other => panic!("expected a NameManaged skip: {other:?}"),
    }
    assert_eq!(phases, vec![ImportPhase::Admitting]);
    assert!(copy.join("SKILL.md").is_file(), "the copy is untouched");
    assert!(f
        .store
        .get_skill_target(&skill_id, "cursor")
        .unwrap()
        .is_none());
}

/// Replace over an identical copy: the Managed skill is synced over it by
/// the same-content policy, no ask needed.
#[test]
fn replace_syncs_the_managed_skill_over_an_identical_copy() {
    let f = fixture();
    let skill_id = manage_alpha(&f, "same");
    install_tool(&f, "cursor");
    let copy = seed_skill_dir(&f, "cursor", "alpha", "same");

    let report = run(&f, &[replace("alpha", &copy)], policy_off(true));

    let ImportGroupStatus::Replaced {
        skill_id: id,
        targets,
        ..
    } = imported(&report, "alpha")
    else {
        panic!("alpha should be replaced: {:?}", report);
    };
    assert_eq!(*id, skill_id);
    assert_eq!(targets.len(), 1, "{targets:?}");
    assert_eq!(targets[0].tool_key, "cursor");
    assert!(matches!(
        targets[0].status,
        crate::core::global_sync::BatchTargetStatus::Synced { .. }
    ));
    assert_links_to_central(&f, &copy, "alpha");
    assert!(f
        .store
        .get_skill_target(&skill_id, "cursor")
        .unwrap()
        .is_some());
}

/// Replace over a divergent copy never force-overwrites from the plan: the
/// pair answers an askable `TARGET_EXISTS` row naming (skill, Tool), and the
/// overwrite ask's retry — `sync_skills_to_tools` with an Overwrite override
/// for that pair — replaces the copy.
#[test]
fn replace_over_a_divergent_copy_produces_an_askable_target_exists_row() {
    use crate::core::errors::CommandError;
    use crate::core::global_sync::{
        sync_skills_to_tools, BatchOverride, BatchPolicy, BatchSkill, BatchTargetStatus,
        OverrideDecision,
    };

    let f = fixture();
    let skill_id = manage_alpha(&f, "managed");
    install_tool(&f, "cursor");
    let copy = seed_skill_dir(&f, "cursor", "alpha", "someone's fork");
    let fork_bytes = fs::read(copy.join("SKILL.md")).unwrap();

    let report = run(&f, &[replace("alpha", &copy)], policy_off(false));

    let ImportGroupStatus::Replaced {
        skill_id: id,
        skill_name,
        targets,
        ..
    } = imported(&report, "alpha")
    else {
        panic!("alpha should be replaced: {:?}", report);
    };
    assert_eq!(targets.len(), 1, "{targets:?}");
    let row = &targets[0];
    assert_eq!(
        (row.skill_id.as_str(), row.tool_key.as_str()),
        (id.as_str(), "cursor")
    );
    assert!(
        matches!(
            &row.status,
            BatchTargetStatus::Failed {
                error: CommandError::TargetExists { .. }
            }
        ),
        "askable TARGET_EXISTS row: {:?}",
        row.status
    );
    assert_eq!(
        fs::read(copy.join("SKILL.md")).unwrap(),
        fork_bytes,
        "copy untouched"
    );

    // The ask's confirmed retry: one batch, per-pair Overwrite override.
    let record = f.store.get_skill_by_id(&skill_id).unwrap().unwrap();
    let retry = sync_skills_to_tools(
        &f.paths.roots,
        &f.store,
        &central_repo_of(&f.store),
        &[BatchSkill {
            skill_id: skill_id.clone(),
            skill_name: skill_name.clone(),
            source_path: PathBuf::from(&record.central_path),
        }],
        &["cursor".to_string()],
        &BatchPolicy {
            overwrite: false,
            overwrite_if_same_content: true,
            overrides: vec![BatchOverride {
                skill_id: skill_id.clone(),
                tool_key: "cursor".to_string(),
                decision: OverrideDecision::Overwrite,
            }],
        },
        6000,
        |_| {},
    );
    assert!(
        matches!(retry[0].status, BatchTargetStatus::Synced { .. }),
        "{:?}",
        retry
    );
    assert_links_to_central(&f, &copy, "alpha");
}

/// #46 end to end: a Tool dir entry whose manifest fails the gate is not in
/// the plan, so naming it fails admission as an unknown group.
#[test]
fn an_invalid_manifest_is_never_admitted() {
    let f = fixture();
    install_tool(&f, "cursor");
    let broken = f.paths.roots.home.join(".cursor/skills/broken-skill");
    fs::create_dir_all(&broken).unwrap();
    fs::write(broken.join("SKILL.md"), "no frontmatter at all\n").unwrap();

    let report = run(&f, &[selection("broken-skill", &broken)], policy_off(true));

    match imported(&report, "broken-skill") {
        ImportGroupStatus::Failed {
            error: crate::core::errors::CommandError::NotFound { kind, .. },
        } => assert_eq!(kind, "onboarding_group"),
        other => panic!("expected an unknown-group failure: {other:?}"),
    }
    assert!(broken.join("SKILL.md").is_file());
}

// ---------------------------------------------------------------------------
// The keep/remove choice for a conflict's other copies (ticket 21)
// ---------------------------------------------------------------------------

fn remove_others(group_name: &str, chosen: &Path) -> ImportSelection {
    ImportSelection {
        remove_divergent: true,
        ..selection(group_name, chosen)
    }
}

/// Auto-sync off, originals kept: "remove the other copies" removes the
/// divergent copy alone — the chosen one follows the originals policy.
#[test]
fn remove_divergent_removes_only_the_copies_that_differ() {
    let f = fixture();
    install_tool(&f, "claude_code");
    install_tool(&f, "cursor");
    let chosen = seed_skill_dir(&f, "claude_code", "alpha", "chosen-body");
    let divergent = seed_skill_dir(&f, "cursor", "alpha", "different-body");

    let report = run(&f, &[remove_others("alpha", &chosen)], policy_off(false));

    let ImportGroupStatus::Imported { originals, .. } = imported(&report, "alpha") else {
        panic!("alpha should import: {:?}", report);
    };
    let removed = originals
        .iter()
        .find(|o| o.path == divergent)
        .expect("the divergent copy is reported");
    assert!(
        matches!(removed.status, OriginalStatus::Removed),
        "{:?}",
        removed.status
    );
    assert_eq!(removed.tool, "cursor");
    assert!(!divergent.exists(), "the divergent copy is gone");
    let left = originals
        .iter()
        .find(|o| o.path == chosen)
        .expect("the chosen copy is reported");
    assert!(
        matches!(left.status, OriginalStatus::LeftInPlace),
        "the chosen copy follows remove_originals=false: {:?}",
        left.status
    );
    assert!(chosen.join("SKILL.md").is_file());
    assert_eq!(
        originals.len(),
        2,
        "each copy is reported once: {:?}",
        originals
    );
}

/// Without the choice the divergent copy stays (the default, keep).
#[test]
fn without_remove_divergent_the_other_copies_stay() {
    let f = fixture();
    install_tool(&f, "claude_code");
    install_tool(&f, "cursor");
    let chosen = seed_skill_dir(&f, "claude_code", "alpha", "chosen-body");
    let divergent = seed_skill_dir(&f, "cursor", "alpha", "different-body");

    let report = run(&f, &[selection("alpha", &chosen)], policy_off(false));

    let ImportGroupStatus::Imported { originals, .. } = imported(&report, "alpha") else {
        panic!("alpha should import: {:?}", report);
    };
    let kept = originals
        .iter()
        .find(|o| o.path == divergent)
        .expect("reported");
    assert!(matches!(kept.status, OriginalStatus::KeptDivergent));
    assert!(divergent.join("SKILL.md").is_file());
}

/// Auto-sync on: the removed divergent copy frees its path, so a target Tool
/// gets the link there — no `TARGET_EXISTS` row, no ask.
#[test]
fn remove_divergent_frees_a_target_path_for_the_link() {
    let f = fixture();
    install_tool(&f, "claude_code");
    install_tool(&f, "cursor");
    let chosen = seed_skill_dir(&f, "claude_code", "alpha", "chosen-body");
    let divergent = seed_skill_dir(&f, "cursor", "alpha", "different-body");

    let report = run(
        &f,
        &[remove_others("alpha", &chosen)],
        ImportPolicy {
            auto_sync: true,
            remove_originals: true,
            tools: Some(vec!["cursor".to_string()]),
            scan_scope: OnboardingScanScope::Installed,
        },
    );

    let ImportGroupStatus::Imported {
        targets,
        originals,
        central_path,
        ..
    } = imported(&report, "alpha")
    else {
        panic!("alpha should import: {:?}", report);
    };
    assert_eq!(*central_path, f.paths.central_dir.join("alpha"));
    assert!(
        targets.iter().all(|t| matches!(
            t.status,
            crate::core::global_sync::BatchTargetStatus::Synced { .. }
        )),
        "every target synced, no collision: {:?}",
        targets
    );
    assert_eq!(originals.len(), 1, "{:?}", originals);
    assert_eq!(originals[0].path, divergent);
    assert!(matches!(originals[0].status, OriginalStatus::Removed));
    assert_links_to_central(&f, &divergent, "alpha");
    assert_links_to_central(&f, &chosen, "alpha");
}

/// A managed-name group ignores the choice: Replace's divergent copies stay
/// askable rows, never removed by a plan.
#[test]
fn remove_divergent_never_touches_a_managed_name_group() {
    let f = fixture();
    install_tool(&f, "cursor");
    let skill_id = manage_alpha(&f, "mine");
    let copy = seed_skill_dir(&f, "cursor", "alpha", "a fork");

    let report = run(
        &f,
        &[ImportSelection {
            replace: true,
            ..remove_others("alpha", &copy)
        }],
        policy_off(true),
    );

    let ImportGroupStatus::Replaced {
        skill_id: replaced,
        central_path,
        ..
    } = imported(&report, "alpha")
    else {
        panic!("alpha should be replaced: {:?}", report);
    };
    assert_eq!(*replaced, skill_id);
    assert_eq!(*central_path, f.paths.central_dir.join("alpha"));
    assert!(copy.join("SKILL.md").is_file(), "the fork is still there");
}

// ── remove_divergent: deletion needs two known, unequal identities (audit F1)

/// The status a removal attempt left on the copy at `path`, for `tool`.
fn original_at<'a>(
    originals: &'a [crate::core::onboarding_import::OriginalOutcome],
    path: &Path,
    tool: &str,
) -> &'a OriginalStatus {
    &originals
        .iter()
        .find(|o| o.path == path && o.tool == tool)
        .unwrap_or_else(|| panic!("{tool} {path:?} reported: {originals:?}"))
        .status
}

// Used by the unix-only unreadable-copy tests alone.
#[cfg(unix)]
fn is_identity_unknown(status: &OriginalStatus, path: &Path) -> bool {
    matches!(
        status,
        OriginalStatus::Failed {
            error: crate::core::errors::CommandError::IdentityUnknown { path: reported, .. },
        } if Path::new(reported) == path
    )
}

/// Make one file unreadable; `false` when the platform (or root) still
/// reads it, so a test cannot construct the unknown identity.
#[cfg(unix)]
fn make_unreadable(file: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(file, fs::Permissions::from_mode(0o000)).expect("chmod 000");
    fs::read(file).is_err()
}

#[cfg(unix)]
fn make_readable(file: &Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(file, fs::Permissions::from_mode(0o644)).expect("chmod 644");
}

/// The audit's A/B/C construction: A (chosen) and C are byte-identical, B
/// differs; one of C's files cannot be read, so C's identity is unknown.
/// Only B — two known, unequal identities — is removed. C is retained and
/// reported `Failed` with `IDENTITY_UNKNOWN`; A follows the originals policy.
#[cfg(unix)]
#[test]
fn remove_divergent_retains_a_copy_whose_identity_is_unknown() {
    let f = fixture();
    install_tool(&f, "claude_code");
    install_tool(&f, "cursor");
    install_tool(&f, "codex");
    let chosen = seed_skill_dir(&f, "claude_code", "alpha", "same-body");
    fs::write(chosen.join("notes.txt"), "notes").unwrap();
    let divergent = seed_skill_dir(&f, "cursor", "alpha", "different-body");
    let unknown = seed_skill_dir(&f, "codex", "alpha", "same-body");
    fs::write(unknown.join("notes.txt"), "notes").unwrap();
    if !make_unreadable(&unknown.join("notes.txt")) {
        eprintln!("skipped: this user can read a 000 file");
        return;
    }

    let report = run(&f, &[remove_others("alpha", &chosen)], policy_off(false));
    make_readable(&unknown.join("notes.txt"));

    let ImportGroupStatus::Imported { originals, .. } = imported(&report, "alpha") else {
        panic!("alpha should import: {:?}", report);
    };
    assert!(
        matches!(
            original_at(originals, &divergent, "cursor"),
            OriginalStatus::Removed
        ),
        "{originals:?}"
    );
    assert!(divergent.symlink_metadata().is_err(), "B is gone");
    assert!(
        is_identity_unknown(original_at(originals, &unknown, "codex"), &unknown),
        "C is retained and reported as an unknown comparison: {originals:?}"
    );
    assert_eq!(
        fs::read_to_string(unknown.join("SKILL.md")).unwrap(),
        "---\nname: alpha\n---\nsame-body\n"
    );
    assert_eq!(
        fs::read_to_string(unknown.join("notes.txt")).unwrap(),
        "notes"
    );
    assert!(matches!(
        original_at(originals, &chosen, "claude_code"),
        OriginalStatus::LeftInPlace
    ));
    assert!(chosen.join("SKILL.md").is_file());
    assert_eq!(originals.len(), 3, "each copy once: {originals:?}");
}

/// A Managed identity that cannot be read (no stored hash, central copy
/// unreadable) is just as unknown: the differing copy is retained.
#[cfg(unix)]
#[test]
fn remove_divergent_retains_a_copy_when_the_managed_identity_is_unknown() {
    let f = fixture();
    install_tool(&f, "claude_code");
    install_tool(&f, "cursor");
    let chosen = seed_skill_dir(&f, "claude_code", "alpha", "chosen-body");
    let report = run(&f, &[selection("alpha", &chosen)], policy_off(false));
    let ImportGroupStatus::Imported {
        skill_id,
        central_path,
        ..
    } = imported(&report, "alpha")
    else {
        panic!("alpha should import: {:?}", report);
    };
    let copy = seed_skill_dir(&f, "cursor", "alpha", "different-body");
    let conn = rusqlite::Connection::open(f.store.db_path()).unwrap();
    conn.execute("UPDATE skills SET content_hash = NULL", [])
        .unwrap();
    if !make_unreadable(&central_path.join("SKILL.md")) {
        eprintln!("skipped: this user can read a 000 file");
        return;
    }

    let status = super::remove_divergent_copy(&f.paths, &f.store, skill_id, &chosen, &copy);
    make_readable(&central_path.join("SKILL.md"));

    assert!(
        status
            .as_ref()
            .is_some_and(|s| is_identity_unknown(s, &copy)),
        "{status:?}"
    );
    assert!(copy.join("SKILL.md").is_file(), "the copy is retained");
}

/// A copy whose metadata cannot be read (its Tool dir is not searchable) is
/// not "already gone": it is retained and reported, never dropped.
#[cfg(unix)]
#[test]
fn remove_divergent_retains_and_reports_a_copy_whose_metadata_is_unreadable() {
    use std::os::unix::fs::PermissionsExt;
    let f = fixture();
    install_tool(&f, "claude_code");
    install_tool(&f, "cursor");
    let chosen = seed_skill_dir(&f, "claude_code", "alpha", "chosen-body");
    let report = run(&f, &[selection("alpha", &chosen)], policy_off(false));
    let ImportGroupStatus::Imported { skill_id, .. } = imported(&report, "alpha") else {
        panic!("alpha should import: {:?}", report);
    };
    let copy = seed_skill_dir(&f, "cursor", "alpha", "different-body");
    let tool_dir = copy.parent().unwrap().to_path_buf();
    fs::set_permissions(&tool_dir, fs::Permissions::from_mode(0o000)).unwrap();
    let blocked = copy.symlink_metadata().is_err();

    let status =
        blocked.then(|| super::remove_divergent_copy(&f.paths, &f.store, skill_id, &chosen, &copy));
    fs::set_permissions(&tool_dir, fs::Permissions::from_mode(0o755)).unwrap();
    let Some(status) = status else {
        eprintln!("skipped: this user can stat through a 000 dir");
        return;
    };

    assert!(
        status
            .as_ref()
            .is_some_and(|s| is_identity_unknown(s, &copy)),
        "{status:?}"
    );
    assert!(copy.join("SKILL.md").is_file(), "the copy is retained");
}

/// The chosen source is never a removal candidate, whatever it compares as
/// (here it changed after finalize, so it now differs from the central copy).
#[test]
fn remove_divergent_never_touches_the_chosen_path() {
    let f = fixture();
    install_tool(&f, "claude_code");
    let chosen = seed_skill_dir(&f, "claude_code", "alpha", "chosen-body");
    let report = run(&f, &[selection("alpha", &chosen)], policy_off(false));
    let ImportGroupStatus::Imported { skill_id, .. } = imported(&report, "alpha") else {
        panic!("alpha should import: {:?}", report);
    };
    fs::write(chosen.join("SKILL.md"), "---\nname: alpha\n---\nedited\n").unwrap();

    let status = super::remove_divergent_copy(&f.paths, &f.store, skill_id, &chosen, &chosen);

    assert!(status.is_none(), "{status:?}");
    assert!(chosen.join("SKILL.md").is_file(), "the chosen copy stays");
}

/// A divergent copy that is a link into another Tool's skill: the link is
/// removed, the directory it points at is untouched.
#[test]
fn remove_divergent_removes_a_link_not_the_other_tools_dir_it_points_at() {
    let f = fixture();
    install_tool(&f, "claude_code");
    install_tool(&f, "cursor");
    install_tool(&f, "codex");
    let chosen = seed_skill_dir(&f, "claude_code", "alpha", "chosen-body");
    let beta = seed_skill_dir(&f, "codex", "beta", "beta-body");
    let link = seed_skill_link(&f, "cursor", "alpha", &beta);

    let report = run(&f, &[remove_others("alpha", &chosen)], policy_off(false));

    let ImportGroupStatus::Imported { originals, .. } = imported(&report, "alpha") else {
        panic!("alpha should import: {:?}", report);
    };
    assert!(
        matches!(
            original_at(originals, &link, "cursor"),
            OriginalStatus::Removed
        ),
        "{originals:?}"
    );
    assert!(link.symlink_metadata().is_err(), "the link is gone");
    assert_eq!(
        fs::read_to_string(beta.join("SKILL.md")).unwrap(),
        "---\nname: beta\n---\nbeta-body\n",
        "the linked-to skill is untouched"
    );
}

/// A Tool's link into the central repo is the app's, never a discovered
/// copy: remove_divergent neither removes nor follows it.
#[test]
fn remove_divergent_never_follows_a_link_into_the_central_repo() {
    let f = fixture();
    install_tool(&f, "claude_code");
    install_tool(&f, "cursor");
    let gamma = seed_skill_dir(&f, "claude_code", "gamma", "gamma-body");
    let report = run(&f, &[selection("gamma", &gamma)], policy_off(true));
    let ImportGroupStatus::Imported {
        central_path: gamma_central,
        ..
    } = imported(&report, "gamma")
    else {
        panic!("gamma should import: {:?}", report);
    };
    let chosen = seed_skill_dir(&f, "claude_code", "alpha", "chosen-body");
    let link = seed_skill_link(&f, "cursor", "alpha", gamma_central);

    let report = run(&f, &[remove_others("alpha", &chosen)], policy_off(false));

    let ImportGroupStatus::Imported { originals, .. } = imported(&report, "alpha") else {
        panic!("alpha should import: {:?}", report);
    };
    assert!(
        originals.iter().all(|o| o.path != link),
        "a link into the central repo is not a copy: {originals:?}"
    );
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    assert_eq!(
        fs::read_to_string(gamma_central.join("SKILL.md")).unwrap(),
        "---\nname: gamma\n---\ngamma-body\n"
    );
}

/// Amp and Kimi share `.config/agents/skills`: the plan carries one variant
/// per harness at one path. The path is removed once and both harnesses get
/// the same row (audit F6).
#[test]
fn remove_divergent_settles_a_shared_dir_once_and_reports_every_harness() {
    let f = fixture();
    install_tool(&f, "claude_code");
    install_tool(&f, "amp");
    install_tool(&f, "kimi_cli");
    let chosen = seed_skill_dir(&f, "claude_code", "alpha", "chosen-body");
    let shared = seed_skill_dir(&f, "amp", "alpha", "different-body");
    assert_eq!(
        adapter_by_key("kimi_cli").unwrap().skills_dir,
        adapter_by_key("amp").unwrap().skills_dir,
        "the shared dir this test is about"
    );

    let report = run(&f, &[remove_others("alpha", &chosen)], policy_off(false));

    let ImportGroupStatus::Imported { originals, .. } = imported(&report, "alpha") else {
        panic!("alpha should import: {:?}", report);
    };
    assert!(
        shared.symlink_metadata().is_err(),
        "the shared copy is gone"
    );
    for tool in ["amp", "kimi_cli"] {
        assert!(
            matches!(
                original_at(originals, &shared, tool),
                OriginalStatus::Removed
            ),
            "{tool}: {originals:?}"
        );
    }
    assert_eq!(
        originals.len(),
        3,
        "chosen + one row per harness: {originals:?}"
    );
}

// ── physical admission of an original's removal (BACKLOG #107) ─────────────

/// A Tool's skills dir linked onto a Managed skill's central copy makes an
/// "original" a folder *inside the central library*. The lexical registry
/// fence admits it (the path is spelled under the Tool's dir); physical
/// admission refuses it with the typed refusal, and the central bytes stay.
#[cfg(unix)]
#[test]
fn an_original_inside_the_central_library_through_an_aliased_tool_dir_is_refused() {
    use crate::core::errors::{CommandError, RemovalRefusal};

    let f = fixture();
    install_tool(&f, "claude_code");
    let beta_original = seed_skill_dir(&f, "claude_code", "beta", "beta body");
    let report = run(&f, &[selection("beta", &beta_original)], policy_off(true));
    assert!(matches!(
        imported(&report, "beta"),
        ImportGroupStatus::Imported { .. }
    ));
    let beta_central = f.paths.central_dir.join("beta");
    let bundled = beta_central.join("alpha");
    fs::create_dir_all(&bundled).unwrap();
    let bundled_body = "---\nname: alpha\n---\nsame\n";
    fs::write(bundled.join("SKILL.md"), bundled_body).unwrap();

    install_tool(&f, "cursor");
    let cursor = adapter_by_key("cursor").unwrap();
    let cursor_dir = f.paths.roots.home.join(cursor.skills_dir.rel);
    if cursor_dir.symlink_metadata().is_ok() {
        fs::remove_dir_all(&cursor_dir).unwrap();
    }
    fs::create_dir_all(cursor_dir.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&beta_central, &cursor_dir).unwrap();
    let chosen = seed_skill_dir(&f, "claude_code", "alpha", "same");

    let report = run(&f, &[selection("alpha", &chosen)], policy_off(true));

    let ImportGroupStatus::Imported { originals, .. } = imported(&report, "alpha") else {
        panic!("alpha should import: {:?}", report);
    };
    assert!(
        matches!(
            original_at(originals, &cursor_dir.join("alpha"), "cursor"),
            OriginalStatus::Failed {
                error: CommandError::RemovalRefused {
                    reason: RemovalRefusal::CentralLibrary,
                    ..
                }
            }
        ),
        "{originals:?}"
    );
    assert_eq!(
        fs::read_to_string(bundled.join("SKILL.md")).expect("central bytes survive"),
        bundled_body
    );
    assert!(!chosen.exists(), "the chosen original is still removed");
}
