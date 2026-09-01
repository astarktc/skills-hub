use std::fs;
use std::path::Path;

use crate::core::tool_adapters::{
    adapter_by_key, constituents_of, global_tool_entries, installed_keys, project_tool_entries,
    ToolCatalogEntry, ToolId, VirtualGroup,
};

fn install(home: &Path, key: &str) {
    let adapter = adapter_by_key(key).unwrap_or_else(|| panic!("adapter {key}"));
    fs::create_dir_all(home.join(adapter.relative_detect_dir)).unwrap();
}

fn entry<'a>(entries: &'a [ToolCatalogEntry], key: &str) -> &'a ToolCatalogEntry {
    entries
        .iter()
        .find(|e| e.key == key)
        .unwrap_or_else(|| panic!("entry {key}"))
}

#[test]
fn global_catalog_lists_every_real_tool_and_no_virtual_group() {
    let home = tempfile::tempdir().unwrap();
    let entries = global_tool_entries(home.path());

    let agents_key = ToolId::AgentsStandard.as_key();
    assert!(entries.iter().all(|e| e.key != agents_key));
    assert!(entries.iter().all(|e| e.constituents.is_empty()));
    // Constituent tools are real global tools and stay listed.
    for a in constituents_of(VirtualGroup::AgentsStandard) {
        entry(&entries, a.key());
    }
    let keys: std::collections::HashSet<_> = entries.iter().map(|e| e.key).collect();
    assert_eq!(keys.len(), entries.len(), "keys are unique");
}

#[test]
fn global_catalog_groups_shared_skills_dirs_and_resolves_paths_under_home() {
    let home = tempfile::tempdir().unwrap();
    let entries = global_tool_entries(home.path());

    let amp = entry(&entries, "amp");
    let kimi = entry(&entries, "kimi_cli");
    assert_eq!(amp.shared_with, vec!["amp", "kimi_cli"]);
    assert_eq!(kimi.shared_with, amp.shared_with);
    assert_eq!(amp.skills_dir, home.path().join(".config/agents/skills"));

    let claude = entry(&entries, "claude_code");
    assert_eq!(claude.shared_with, vec!["claude_code"]);
    assert_eq!(claude.skills_dir, home.path().join(".claude/skills"));
    assert!(entries.iter().all(|e| e.shared_with.contains(&e.key)));
}

#[test]
fn global_catalog_installedness_comes_from_home() {
    let home = tempfile::tempdir().unwrap();
    assert!(installed_keys(&global_tool_entries(home.path())).is_empty());

    install(home.path(), "claude_code");
    install(home.path(), "amp");
    let entries = global_tool_entries(home.path());
    assert!(entry(&entries, "claude_code").installed);
    assert!(entry(&entries, "amp").installed);
    // Shared detect dir: kimi_cli is detected by the same directory as amp.
    assert!(entry(&entries, "kimi_cli").installed);
    assert!(!entry(&entries, "cursor").installed);
    assert_eq!(
        installed_keys(&entries),
        vec!["claude_code", "amp", "kimi_cli"],
        "catalog order, not alphabetical"
    );
}

#[test]
fn project_catalog_absorbs_constituents_into_the_virtual_group() {
    let home = tempfile::tempdir().unwrap();
    let entries = project_tool_entries(home.path());

    let agents = entry(&entries, ToolId::AgentsStandard.as_key());
    for a in constituents_of(VirtualGroup::AgentsStandard) {
        assert!(
            entries.iter().all(|e| e.key != a.key()),
            "{} must be absorbed into the group entry",
            a.key()
        );
        assert!(agents.constituents.contains(&a.display_name));
    }
    assert_eq!(agents.constituents.len(), 9);
    assert_eq!(agents.skills_dir, home.path().join(".agents/skills"));
    // Real tools keep no constituents.
    assert!(entry(&entries, "claude_code").constituents.is_empty());
}

#[test]
fn project_catalog_group_is_installed_when_any_constituent_is() {
    let home = tempfile::tempdir().unwrap();
    let agents_key = ToolId::AgentsStandard.as_key();
    assert!(!entry(&project_tool_entries(home.path()), agents_key).installed);

    install(home.path(), "codex");
    let entries = project_tool_entries(home.path());
    assert!(entry(&entries, agents_key).installed);
    assert!(!entry(&entries, "claude_code").installed);
    assert_eq!(installed_keys(&entries), vec![agents_key]);

    // Only constituents decide: the group entry's own detect dir (`~/.agents`)
    // is not a tool installation.
    let home2 = tempfile::tempdir().unwrap();
    install(home2.path(), agents_key);
    assert!(!entry(&project_tool_entries(home2.path()), agents_key).installed);
}

#[test]
fn project_catalog_groups_by_project_skills_dir() {
    let home = tempfile::tempdir().unwrap();
    let entries = project_tool_entries(home.path());

    let agents = entry(&entries, ToolId::AgentsStandard.as_key());
    assert_eq!(agents.shared_with, vec![agents.key]);
    // Trae and Trae CN both write <project>/.trae/skills.
    assert_eq!(entry(&entries, "trae").shared_with, vec!["trae", "trae_cn"]);
    assert_eq!(
        entry(&entries, "claude_code").shared_with,
        vec!["claude_code"]
    );
    assert!(entries.iter().all(|e| e.shared_with.contains(&e.key)));
}
