use std::fs;
use std::path::Path;

use super::{build_onboarding_plan_in_home, OnboardingScanScope};
use crate::core::tool_adapters::{adapter_by_key, mark_installed_in};

/// An installed Tool holding one unmanaged skill whose file body is `body`.
fn installed_with_skill(home: &Path, key: &str, name: &str, body: &[u8]) {
    let adapter = adapter_by_key(key).unwrap_or_else(|| panic!("adapter {key}"));
    mark_installed_in(home, adapter);
    let dir = home.join(adapter.relative_skills_dir).join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("a.txt"), body).unwrap();
}

#[test]
fn groups_by_name_and_detects_conflicts_by_fingerprint() {
    let home = tempfile::tempdir().unwrap();

    installed_with_skill(home.path(), "cursor", "foo", b"cursor");
    installed_with_skill(home.path(), "codex", "foo", b"codex");

    // Codex .system should be ignored
    fs::create_dir_all(home.path().join(".codex/skills/.system")).unwrap();
    fs::write(home.path().join(".codex/skills/.system/SKILL.md"), b"x").unwrap();

    let plan =
        build_onboarding_plan_in_home(home.path(), &OnboardingScanScope::Installed, None, None)
            .unwrap();
    assert_eq!(plan.total_tools_scanned, 2);
    assert_eq!(plan.total_skills_found, 2);
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
    fs::write(home.path().join(".kiro/skills/foo/a.txt"), b"kiro").unwrap();

    let plan =
        build_onboarding_plan_in_home(home.path(), &OnboardingScanScope::Installed, None, None)
            .unwrap();
    assert_eq!(plan.total_tools_scanned, 1);
    assert_eq!(plan.total_skills_found, 1);
    assert_eq!(plan.groups[0].variants[0].tool, "cursor");
}

#[test]
fn selected_scope_scans_exactly_the_selection() {
    let home = tempfile::tempdir().unwrap();

    installed_with_skill(home.path(), "cursor", "foo", b"cursor");
    installed_with_skill(home.path(), "codex", "foo", b"codex");
    // Selected but absent: scans to nothing, still counted as visited.
    let scope = OnboardingScanScope::Selected(vec!["codex".into(), "claude_code".into()]);

    let plan = build_onboarding_plan_in_home(home.path(), &scope, None, None).unwrap();
    assert_eq!(plan.total_tools_scanned, 2);
    assert_eq!(plan.total_skills_found, 1);
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
    fs::write(home.path().join(".kiro/skills/foo/a.txt"), b"kiro").unwrap();
    let scope = OnboardingScanScope::Selected(vec!["kiro_cli".into()]);

    let plan = build_onboarding_plan_in_home(home.path(), &scope, None, None).unwrap();
    assert_eq!(plan.total_tools_scanned, 1);
    assert_eq!(plan.total_skills_found, 1);
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

    let plan = build_onboarding_plan_in_home(
        home.path(),
        &OnboardingScanScope::Installed,
        Some(&central),
        None,
    )
    .unwrap();
    assert_eq!(plan.total_skills_found, 0);
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

    let plan = build_onboarding_plan_in_home(
        home.path(),
        &OnboardingScanScope::Installed,
        None,
        Some(&exclude),
    )
    .unwrap();
    assert_eq!(plan.total_skills_found, 0);
}
