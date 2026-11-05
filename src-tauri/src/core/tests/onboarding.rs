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
    exclude_managed_targets: Option<&HashSet<std::path::PathBuf>>,
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
    installed_with_skill(home.path(), "pi", "foo", b"pi");

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
    installed_with_skill(home.path(), "pi", "foo", b"same");
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

/// `~/.agents/skills` is shared by the AGENTS-standard entry, Codex, Goose
/// and OpenHands: it is scanned once, and its copy is one variant.
#[test]
fn a_shared_skills_dir_is_scanned_once() {
    let home = tempfile::tempdir().unwrap();
    installed_with_skill(home.path(), "codex", "foo", b"x");
    let goose = adapter_by_key("goose").unwrap();
    mark_installed_in(home.path(), goose);

    let plan = plan_in(home.path(), &OnboardingScanScope::Installed, None, None).unwrap();
    assert_eq!(plan.copy_count, 1);
    assert_eq!(plan.groups[0].variants.len(), 1);
    assert_eq!(
        plan.groups[0].variants[0].tool, "agents_skills",
        "attributed to the first sharer in scope, in registry order"
    );
}

/// A store holding one Managed skill `foo` with an owned global row for
/// `tool` at `path` — the exclusion the production plan reads.
fn store_owning(dir: &Path, tool: &str, path: &Path) -> crate::core::skill_store::SkillStore {
    use crate::core::skill_store::{SkillRecord, SkillStore, SkillTargetRecord};
    use crate::core::sync_status::{SyncMode, SyncStatus};

    let store = SkillStore::new(dir.join("test.db"));
    store.ensure_schema().unwrap();
    store
        .upsert_skill(&SkillRecord {
            id: "skill-1".to_string(),
            name: "foo".to_string(),
            description: None,
            source_type: "local".to_string(),
            source_ref: None,
            source_subpath: None,
            source_revision: None,
            central_path: dir.join("central/foo").to_string_lossy().into_owned(),
            content_hash: None,
            created_at: 1,
            updated_at: 1,
            last_sync_at: None,
            last_seen_at: 1,
            status: "ok".to_string(),
            imported_from_tool: None,
        })
        .unwrap();
    store
        .upsert_skill_target(&SkillTargetRecord {
            id: "t-1".to_string(),
            skill_id: "skill-1".to_string(),
            tool: tool.to_string(),
            target_path: path.to_string_lossy().into_owned(),
            mode: SyncMode::Copy,
            status: SyncStatus::Synced,
            last_error: None,
            synced_at: Some(1),
        })
        .unwrap();
    store
}

/// Ticket-34 audit F4: a managed **copy** in `~/.agents/skills` owned only by
/// Codex's row (a copy-fallback relocation) is the app's own artifact. The
/// shared dir is scanned once and attributed to the first sharer in scope —
/// under Installed scope the AGENTS-standard entry — but the exclusion is by
/// physical path, so the copy is never re-offered, whatever the attribution.
#[test]
fn an_owned_copy_in_a_shared_dir_is_never_offered_whatever_its_attribution() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    installed_with_skill(&home, "codex", "foo", b"managed");
    let path = home.join(".agents/skills/foo");
    let store = store_owning(dir.path(), "codex", &path);
    let central = dir.path().join("central");

    for scope in [
        OnboardingScanScope::Installed,
        OnboardingScanScope::Selected(vec!["codex".to_string()]),
    ] {
        let plan = super::build_onboarding_plan(&home, &central, &store, &scope).unwrap();
        assert_eq!(plan.copy_count, 0, "{scope:?}: {:?}", plan.groups);
        assert!(plan.groups.is_empty(), "{scope:?}");
    }
}

/// F4: an unmanaged copy in the shared `~/.agents/skills` is one variant whose
/// roster names every in-scope harness loading it — the dir's sharers and a
/// VERIFIED AGENTS-standard reader (Cursor) — never one that is not
/// installed (Pi is a reader too) or not selected.
#[test]
fn a_shared_copy_names_every_in_scope_reader() {
    let home = tempfile::tempdir().unwrap();
    installed_with_skill(home.path(), "codex", "foo", b"x");
    for key in ["goose", "openhands", "cursor", "claude_code"] {
        mark_installed_in(home.path(), adapter_by_key(key).unwrap());
    }

    let plan = plan_in(home.path(), &OnboardingScanScope::Installed, None, None).unwrap();
    assert_eq!(plan.copy_count, 1, "one physical copy, one variant");
    let variant = &plan.groups[0].variants[0];
    assert_eq!(variant.tool, "agents_skills");
    assert_eq!(
        variant.readers,
        ["agents_skills", "cursor", "codex", "openhands", "goose"],
        "registry order; Claude Code reads its own dir only, Pi is not installed"
    );

    let selected = OnboardingScanScope::Selected(vec!["goose".to_string(), "codex".to_string()]);
    let plan = plan_in(home.path(), &selected, None, None).unwrap();
    let variant = &plan.groups[0].variants[0];
    assert_eq!(variant.tool, "codex", "the first selected sharer");
    assert_eq!(variant.readers, ["codex", "goose"]);

    // An own-dir copy is read by its own Tool and by every installed
    // cross-reader of that dir (Cursor and Goose read `~/.claude/skills`).
    installed_with_skill(home.path(), "claude_code", "bar", b"y");
    let plan = plan_in(home.path(), &OnboardingScanScope::Installed, None, None).unwrap();
    let bar = plan.groups.iter().find(|g| g.name == "bar").unwrap();
    assert_eq!(bar.variants[0].readers, ["cursor", "claude_code", "goose"]);
}
