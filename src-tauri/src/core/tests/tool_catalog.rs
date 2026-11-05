use std::fs;
use std::path::{Path, PathBuf};

use crate::core::tool_adapters::{
    adapter_by_key, constituents_of, default_tool_adapters, installed_global_keys, installed_keys,
    mark_installed_in, project_tool_entries, skills_dir_in, GlobalLayout, ToolCatalogEntry, ToolId,
    VirtualGroup,
};

/// The global catalog with every harness on its own dir and no recorded
/// targets — the shape every pre-#83 assertion here describes.
fn global_tool_entries(home: &Path) -> Vec<ToolCatalogEntry> {
    crate::core::tool_adapters::global_tool_entries(home, GlobalLayout::OWN_DIRS, &[])
}

fn preferring(home: &Path, recorded: &[PathBuf]) -> Vec<ToolCatalogEntry> {
    crate::core::tool_adapters::global_tool_entries(
        home,
        GlobalLayout::PREFER_AGENTS_STANDARD,
        recorded,
    )
}

fn install(home: &Path, key: &str) {
    let adapter = adapter_by_key(key).unwrap_or_else(|| panic!("adapter {key}"));
    mark_installed_in(home, adapter);
}

fn entry<'a>(entries: &'a [ToolCatalogEntry], key: &str) -> &'a ToolCatalogEntry {
    entries
        .iter()
        .find(|e| e.key == key)
        .unwrap_or_else(|| panic!("entry {key}"))
}

#[test]
fn global_catalog_lists_the_virtual_group_alongside_its_constituents() {
    let home = tempfile::tempdir().unwrap();
    let entries = global_tool_entries(home.path());

    let agents_key = ToolId::AgentsStandard.as_key();
    // `~/.agents/skills` is a real global location, so it is selectable
    // — as an independent target, not an aggregate.
    let agents = entry(&entries, agents_key);
    assert!(
        agents.constituents.is_empty(),
        "globally the group absorbs nothing, so it advertises no roster"
    );
    assert!(entries.iter().all(|e| e.constituents.is_empty()));
    // Constituent tools are real global tools and stay listed individually.
    for a in constituents_of(VirtualGroup::AgentsStandard) {
        entry(&entries, a.key());
    }
    assert_eq!(entries.len(), default_tool_adapters().len());
    let keys: std::collections::HashSet<_> = entries.iter().map(|e| e.key).collect();
    assert_eq!(keys.len(), entries.len(), "keys are unique");
}

#[test]
fn the_group_label_carries_the_roster_count_at_project_scope_only() {
    let home = tempfile::tempdir().unwrap();
    let agents_key = ToolId::AgentsStandard.as_key();

    let global = entry(&global_tool_entries(home.path()), agents_key).label;
    let project = entry(&project_tool_entries(home.path()), agents_key).label;

    assert_eq!(global, ".agents/skills");
    assert_eq!(project, ".agents/skills (9 harnesses)");
    assert!(
        !global.contains("harnesses"),
        "a global roster claim would make a user skip the real Cursor checkbox"
    );
}

#[test]
fn global_group_installedness_is_its_own_detect_dir() {
    let home = tempfile::tempdir().unwrap();
    let agents_key = ToolId::AgentsStandard.as_key();
    assert!(!entry(&global_tool_entries(home.path()), agents_key).installed);

    // A constituent being installed says nothing about `~/.agents`.
    install(home.path(), "codex");
    assert!(!entry(&global_tool_entries(home.path()), agents_key).installed);

    install(home.path(), agents_key);
    let entries = global_tool_entries(home.path());
    assert!(entry(&entries, agents_key).installed);
    assert!(installed_keys(&entries).contains(&agents_key.to_string()));
}

#[test]
fn global_catalog_groups_shared_skills_dirs_and_resolves_paths_under_home() {
    let home = tempfile::tempdir().unwrap();
    let entries = global_tool_entries(home.path());

    // Codex, OpenHands and Goose deploy to `~/.agents/skills` (brief I2, I8):
    // they share the AGENTS-standard entry's dir, in registry order.
    let agents = entry(&entries, ToolId::AgentsStandard.as_key());
    assert_eq!(
        agents.shared_with,
        vec!["agents_skills", "codex", "openhands", "goose"]
    );
    for key in ["codex", "openhands", "goose"] {
        assert_eq!(entry(&entries, key).shared_with, agents.shared_with);
    }
    // Kimi left Amp's dir (brief I4): Amp is alone there now.
    assert_eq!(entry(&entries, "amp").shared_with, vec!["amp"]);
    assert_eq!(entry(&entries, "kimi_cli").shared_with, vec!["kimi_cli"]);

    let claude = entry(&entries, "claude_code");
    assert_eq!(claude.shared_with, vec!["claude_code"]);
    assert!(entries.iter().all(|e| e.shared_with.contains(&e.key)));
    // Preference off: a reader such as Cursor keeps its own dir.
    assert_eq!(entry(&entries, "cursor").shared_with, vec!["cursor"]);

    // The catalog carries no dir (see `ToolCatalogEntry`); the grouping it
    // reports must still match what the registry resolves under this home.
    assert_eq!(
        skills_dir_in(home.path(), adapter_by_key("codex").unwrap()),
        home.path().join(".agents/skills")
    );
    assert_eq!(
        skills_dir_in(home.path(), adapter_by_key("kimi_cli").unwrap()),
        home.path().join(".kimi-code/skills")
    );
    assert_eq!(
        skills_dir_in(home.path(), adapter_by_key("claude_code").unwrap()),
        home.path().join(".claude/skills")
    );
}

/// #83 on: every VERIFIED AGENTS-standard reader deploys to
/// `~/.agents/skills`, so all of them share the AGENTS-standard entry —
/// the collapsed "AGENTS standard — Cursor, Codex, OpenCode +N" row's roster.
#[test]
fn the_preference_groups_every_agents_standard_reader_with_the_entry() {
    let home = tempfile::tempdir().unwrap();
    let entries = preferring(home.path(), &[]);
    let agents = entry(&entries, ToolId::AgentsStandard.as_key());
    let expected: Vec<&str> = std::iter::once("agents_skills")
        .chain(
            default_tool_adapters()
                .iter()
                .filter(|a| a.reads_agents_standard_globally.is_some())
                .map(|a| a.key()),
        )
        .collect();
    assert_eq!(agents.shared_with, expected);
    assert!(agents.shared_with.contains(&"cursor"));
    assert!(agents.shared_with.contains(&"amp"));
    // Non-readers (VERIFIED absence, LIKELY, UNKNOWN) keep their own dir.
    for key in [
        "claude_code",
        "antigravity",
        "hermes-agent",
        "copaw",
        "workbuddy",
    ] {
        assert_eq!(entry(&entries, key).shared_with, vec![key], "{key}");
    }
}

#[test]
fn the_installed_global_keys_are_the_installed_catalog_entries() {
    let home = tempfile::tempdir().unwrap();
    install(home.path(), "pi");
    install(home.path(), "claude_code");
    assert_eq!(
        installed_global_keys(home.path()),
        installed_keys(&global_tool_entries(home.path()))
    );
}

/// Cross-read (brief I1): Cursor reads Claude Code's and Codex's dirs. Only
/// *installed* harnesses are named, and with the preference off the
/// AGENTS-standard dir's deployers are cross-reads too (Cursor reads it).
#[test]
fn cross_reads_resolve_to_installed_harness_keys() {
    let home = tempfile::tempdir().unwrap();
    assert!(entry(&global_tool_entries(home.path()), "cursor")
        .also_reads
        .is_empty());

    install(home.path(), "claude_code");
    install(home.path(), "codex");
    install(home.path(), "agents_skills");
    let entries = global_tool_entries(home.path());
    assert_eq!(
        entry(&entries, "cursor").also_reads,
        vec!["agents_skills", "claude_code", "codex"],
        "Claude Code by `also_reads`, the AGENTS-standard sharers by the VERIFIED read"
    );
    // Claude Code reads nobody else's dir.
    assert!(entry(&entries, "claude_code").also_reads.is_empty());
    // A cross-read never names the entry itself or its own sharers.
    let codex = entry(&entries, "codex");
    assert!(!codex.also_reads.contains(&"agents_skills"));

    // Preference on: Cursor deploys to `~/.agents/skills` itself, so those
    // are sharers, and only Claude Code is still a cross-read.
    let entries = preferring(home.path(), &[]);
    assert_eq!(entry(&entries, "cursor").also_reads, vec!["claude_code"]);
}

/// An `OwnWins` harness moved to `~/.agents/skills` whose own dir holds a
/// copy we did not record of a skill we deployed there: the copy shadows ours.
#[test]
fn an_own_wins_harness_with_a_foreign_copy_is_flagged_shadowed() {
    let home = tempfile::tempdir().unwrap();
    install(home.path(), "amp"); // OwnWins (brief § 1 row 7)
    install(home.path(), "cursor"); // Unknown precedence
    let deployed = home.path().join(".agents/skills/skill");
    let recorded = vec![deployed.clone()];
    assert!(!entry(&preferring(home.path(), &recorded), "amp").shadowed_by_own_dir);

    let foreign = home.path().join(".config/agents/skills/skill");
    fs::create_dir_all(&foreign).unwrap();
    fs::create_dir_all(home.path().join(".cursor/skills/skill")).unwrap();
    let entries = preferring(home.path(), &recorded);
    assert!(entry(&entries, "amp").shadowed_by_own_dir);
    assert!(
        !entry(&entries, "cursor").shadowed_by_own_dir,
        "only an OwnWins harness is known to prefer the foreign copy"
    );
    // Our own row there is no foreign copy.
    let ours = vec![deployed, foreign];
    assert!(!entry(&preferring(home.path(), &ours), "amp").shadowed_by_own_dir);
    // Preference off: Amp deploys to its own dir, nothing to shadow.
    assert!(
        !entry(
            &crate::core::tool_adapters::global_tool_entries(
                home.path(),
                GlobalLayout::OWN_DIRS,
                &recorded
            ),
            "amp"
        )
        .shadowed_by_own_dir
    );
}

#[test]
fn a_global_only_tool_is_absent_from_the_project_catalog() {
    let home = tempfile::tempdir().unwrap();
    assert!(global_tool_entries(home.path())
        .iter()
        .any(|e| e.key == "workbuddy"));
    assert!(project_tool_entries(home.path())
        .iter()
        .all(|e| e.key != "workbuddy"));
}

#[test]
fn global_catalog_installedness_comes_from_home() {
    let home = tempfile::tempdir().unwrap();
    assert!(installed_keys(&global_tool_entries(home.path())).is_empty());

    install(home.path(), "claude_code");
    install(home.path(), "codex");
    let entries = global_tool_entries(home.path());
    assert!(entry(&entries, "claude_code").installed);
    assert!(entry(&entries, "codex").installed);
    // A shared skills dir does not make Goose installed: it has its own root.
    assert!(!entry(&entries, "goose").installed);
    assert!(!entry(&entries, "cursor").installed);
    assert_eq!(
        installed_keys(&entries),
        vec!["claude_code", "codex"],
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
