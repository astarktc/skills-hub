use std::fs;
use std::path::Path;

use std::collections::{HashMap, HashSet};

use super::{build_onboarding_plan_in_home, OnboardingPlan, OnboardingScanScope};
use crate::core::skill_discovery::InvalidManifestReason;
use crate::core::tool_adapters::{adapter_by_key, mark_installed_in};

/// The plan with no Managed skill names.
fn plan_in(
    home: &Path,
    scope: &OnboardingScanScope,
    exclude_root: Option<&Path>,
    exclude_managed_targets: Option<&HashSet<String>>,
) -> anyhow::Result<OnboardingPlan> {
    build_onboarding_plan_in_home(
        home,
        scope,
        exclude_root,
        exclude_managed_targets,
        &HashMap::new(),
    )
}

/// An installed Tool holding one unmanaged skill whose file body is `body`.
fn installed_with_skill(home: &Path, key: &str, name: &str, body: &[u8]) {
    let adapter = adapter_by_key(key).unwrap_or_else(|| panic!("adapter {key}"));
    mark_installed_in(home, adapter);
    let dir = home.join(adapter.relative_skills_dir).join(name);
    fs::create_dir_all(&dir).unwrap();
    let mut manifest = format!("---\nname: {name}\n---\n").into_bytes();
    manifest.extend_from_slice(body);
    fs::write(dir.join("SKILL.md"), manifest).unwrap();
}

#[test]
fn groups_by_name_and_detects_conflicts_by_fingerprint() {
    let home = tempfile::tempdir().unwrap();

    installed_with_skill(home.path(), "cursor", "foo", b"cursor");
    installed_with_skill(home.path(), "codex", "foo", b"codex");

    // Codex .system should be ignored
    fs::create_dir_all(home.path().join(".codex/skills/.system")).unwrap();
    fs::write(home.path().join(".codex/skills/.system/SKILL.md"), b"x").unwrap();

    let plan = plan_in(home.path(), &OnboardingScanScope::Installed, None, None).unwrap();
    assert_eq!(plan.total_tools_scanned, 2);
    assert_eq!(plan.copy_count, 2);
    assert_eq!(plan.groups.len(), 1);
    assert_eq!(plan.groups[0].name, "foo");
    assert!(
        plan.groups[0].has_conflict,
        "same name but different content should conflict"
    );
    assert_eq!(plan.groups[0].variants.len(), 2);
}

#[test]
fn installed_scope_skips_a_skills_only_footprint() {
    let home = tempfile::tempdir().unwrap();

    installed_with_skill(home.path(), "cursor", "foo", b"cursor");
    // A deployer's footprint: the skills path alone, no tool behind it.
    fs::create_dir_all(home.path().join(".kiro/skills/foo")).unwrap();
    fs::write(
        home.path().join(".kiro/skills/foo/SKILL.md"),
        "---\nname: foo\n---\nkiro\n",
    )
    .unwrap();

    let plan = plan_in(home.path(), &OnboardingScanScope::Installed, None, None).unwrap();
    assert_eq!(plan.total_tools_scanned, 1);
    assert_eq!(plan.copy_count, 1);
    assert_eq!(plan.groups[0].variants[0].tool, "cursor");
}

#[test]
fn selected_scope_scans_exactly_the_selection() {
    let home = tempfile::tempdir().unwrap();

    installed_with_skill(home.path(), "cursor", "foo", b"cursor");
    installed_with_skill(home.path(), "codex", "foo", b"codex");
    // Selected but absent: scans to nothing, still counted as visited.
    let scope = OnboardingScanScope::Selected(vec!["codex".into(), "claude_code".into()]);

    let plan = plan_in(home.path(), &scope, None, None).unwrap();
    assert_eq!(plan.total_tools_scanned, 2);
    assert_eq!(plan.copy_count, 1);
    assert_eq!(plan.groups.len(), 1);
    assert_eq!(plan.groups[0].variants.len(), 1);
    assert_eq!(plan.groups[0].variants[0].tool, "codex");
    assert!(!plan.groups[0].has_conflict);
}

#[test]
fn selected_scope_does_not_require_detection() {
    let home = tempfile::tempdir().unwrap();

    // The footprint the Installed scope rejects is scanned when the operator
    // selected that Tool: the selection is their word, not detection's.
    fs::create_dir_all(home.path().join(".kiro/skills/foo")).unwrap();
    fs::write(
        home.path().join(".kiro/skills/foo/SKILL.md"),
        "---\nname: foo\n---\nkiro\n",
    )
    .unwrap();
    let scope = OnboardingScanScope::Selected(vec!["kiro_cli".into()]);

    let plan = plan_in(home.path(), &scope, None, None).unwrap();
    assert_eq!(plan.total_tools_scanned, 1);
    assert_eq!(plan.copy_count, 1);
}

#[test]
#[cfg(unix)]
fn excludes_central_repo_path() {
    use std::os::unix::fs::symlink;

    let home = tempfile::tempdir().unwrap();

    let cursor = adapter_by_key("cursor").unwrap();
    mark_installed_in(home.path(), cursor);
    std::fs::create_dir_all(home.path().join(".cursor/skills")).unwrap();

    let central = home.path().join("central");
    std::fs::create_dir_all(central.join("skill-a")).unwrap();

    let link_path = home.path().join(".cursor/skills/skill-a");
    symlink(central.join("skill-a"), &link_path).unwrap();

    let plan = plan_in(
        home.path(),
        &OnboardingScanScope::Installed,
        Some(&central),
        None,
    )
    .unwrap();
    assert_eq!(plan.copy_count, 0);
}

#[test]
fn excludes_managed_skill_targets() {
    let home = tempfile::tempdir().unwrap();

    installed_with_skill(home.path(), "cursor", "foo", b"cursor");

    let mut exclude = std::collections::HashSet::new();
    exclude.insert(super::managed_target_key(
        "cursor",
        &home.path().join(".cursor/skills/foo"),
    ));

    let plan = plan_in(
        home.path(),
        &OnboardingScanScope::Installed,
        None,
        Some(&exclude),
    )
    .unwrap();
    assert_eq!(plan.copy_count, 0);
}

/// #46: a directory whose `SKILL.md` does not parse is never a group member;
/// it is listed in `invalid` with its Tool, path and typed reason.
#[test]
fn invalid_manifests_are_listed_apart_from_the_groups() {
    let home = tempfile::tempdir().unwrap();
    installed_with_skill(home.path(), "cursor", "foo", b"cursor");
    let broken = home.path().join(".cursor/skills/broken-skill");
    fs::create_dir_all(&broken).unwrap();
    fs::write(broken.join("SKILL.md"), "---\ndescription: x\n---\n").unwrap();
    let bare = home.path().join(".cursor/skills/scratch");
    fs::create_dir_all(&bare).unwrap();

    let plan = plan_in(home.path(), &OnboardingScanScope::Installed, None, None).unwrap();

    assert_eq!(plan.group_count, 1);
    assert_eq!(plan.copy_count, 1);
    assert_eq!(plan.groups[0].name, "foo");
    let mut invalid: Vec<(&str, &str, InvalidManifestReason)> = plan
        .invalid
        .iter()
        .map(|i| (i.name.as_str(), i.tool.as_str(), i.reason))
        .collect();
    invalid.sort_by_key(|(name, _, _)| *name);
    assert_eq!(
        invalid,
        vec![
            ("broken-skill", "cursor", InvalidManifestReason::MissingName),
            ("scratch", "cursor", InvalidManifestReason::MissingSkillMd),
        ]
    );
    assert!(plan.invalid.iter().any(|i| i.path == broken));
}

/// The plan counts name-groups and copies separately.
#[test]
fn plan_counts_groups_and_copies_separately() {
    let home = tempfile::tempdir().unwrap();
    installed_with_skill(home.path(), "cursor", "foo", b"same");
    installed_with_skill(home.path(), "codex", "foo", b"same");
    installed_with_skill(home.path(), "claude_code", "foo", b"other");
    installed_with_skill(home.path(), "claude_code", "bar", b"bar");

    let plan = plan_in(home.path(), &OnboardingScanScope::Installed, None, None).unwrap();

    assert_eq!(plan.group_count, 2);
    assert_eq!(plan.copy_count, 4);
    let names: Vec<&str> = plan.groups.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(names, vec!["bar", "foo"], "groups are sorted by name");
}

/// A group whose name a Managed skill holds carries the flag and the id.
#[test]
fn a_group_whose_name_is_managed_carries_the_flag_and_the_skill_id() {
    let home = tempfile::tempdir().unwrap();
    installed_with_skill(home.path(), "cursor", "tdd", b"fork");
    installed_with_skill(home.path(), "cursor", "fresh", b"new");
    let managed = HashMap::from([("tdd".to_string(), "skill-1".to_string())]);

    let plan = build_onboarding_plan_in_home(
        home.path(),
        &OnboardingScanScope::Installed,
        None,
        None,
        &managed,
    )
    .unwrap();

    let tdd = plan.groups.iter().find(|g| g.name == "tdd").unwrap();
    assert!(tdd.name_is_managed);
    assert_eq!(tdd.managed_skill_id.as_deref(), Some("skill-1"));
    let fresh = plan.groups.iter().find(|g| g.name == "fresh").unwrap();
    assert!(!fresh.name_is_managed);
    assert_eq!(fresh.managed_skill_id, None);
}

/// The store-backed plan reads the Managed names from the library: the
/// record's name and its central directory's name.
#[test]
fn the_store_backed_plan_marks_managed_names_from_the_library() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let central = dir.path().join("central");
    fs::create_dir_all(&home).unwrap();
    let store = crate::core::skill_store::SkillStore::new(dir.path().join("t.db"));
    store.ensure_schema().unwrap();
    installed_with_skill(&home, "cursor", "tdd", b"fork");
    let source = dir.path().join("src/tdd");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("SKILL.md"), "---\nname: tdd\n---\nmine\n").unwrap();
    let installed = crate::core::installer::install_imported_skill(
        &crate::core::installer::InstallerPaths {
            home: home.clone(),
            central_dir: central.clone(),
            cache_dir: dir.path().join("cache"),
        },
        &store,
        &source,
        None,
        None,
    )
    .unwrap();

    let plan =
        super::build_onboarding_plan(&home, &central, &store, &OnboardingScanScope::Installed)
            .unwrap();

    let tdd = plan.groups.iter().find(|g| g.name == "tdd").unwrap();
    assert!(tdd.name_is_managed);
    assert_eq!(
        tdd.managed_skill_id.as_deref(),
        Some(installed.skill_id.as_str())
    );
}
