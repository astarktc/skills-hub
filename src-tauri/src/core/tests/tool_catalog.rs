use std::path::Path;

use crate::core::environment::HostRoots;
use crate::core::tool_adapters::catalog::{DuplicateReads, ReaderEntry};
use crate::core::tool_adapters::reads::DuplicateBehaviour;
use crate::core::tool_adapters::{
    adapter_by_key, constituents_of, default_tool_adapters, global_tool_entries, installed_keys,
    mark_installed_in, mark_reader_installed_in, project_tool_entries, skills_dir_in,
    test_overrides, ToolCatalogEntry, ToolId, VirtualGroup,
};

fn install(home: &Path, key: &str) {
    let adapter = adapter_by_key(key).unwrap_or_else(|| panic!("adapter {key}"));
    mark_installed_in(&HostRoots::under_home(home), adapter);
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
    let entries = global_tool_entries(&HostRoots::under_home(home.path()));

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

/// The group's label is the plain dir at both scopes; only the project entry
/// carries the roster its "(N harnesses)" count is derived from (the
/// frontend composes that label through i18n — no count is written anywhere).
#[test]
fn the_group_roster_is_carried_at_project_scope_only() {
    let home = tempfile::tempdir().unwrap();
    let roots = HostRoots::under_home(home.path());
    let agents_key = ToolId::AgentsStandard.as_key();

    let global = global_tool_entries(&roots);
    let project = project_tool_entries(&roots);
    let global = entry(&global, agents_key);
    let project = entry(&project, agents_key);

    assert_eq!(global.label, ".agents/skills");
    assert_eq!(project.label, ".agents/skills");
    assert!(
        global.constituents.is_empty(),
        "a global roster claim would make a user skip the real Cursor checkbox"
    );
    assert_eq!(
        project.constituents,
        vec![
            "Cursor",
            "Codex",
            "OpenCode",
            "Antigravity",
            "Amp",
            "Kimi Code CLI",
            "Cline",
            "Gemini CLI",
            "GitHub Copilot",
        ]
    );
}

#[test]
fn global_group_installedness_is_its_own_detect_dir() {
    let home = tempfile::tempdir().unwrap();
    let agents_key = ToolId::AgentsStandard.as_key();
    assert!(
        !entry(
            &global_tool_entries(&HostRoots::under_home(home.path())),
            agents_key
        )
        .installed
    );

    // A constituent being installed says nothing about `~/.agents`.
    install(home.path(), "codex");
    assert!(
        !entry(
            &global_tool_entries(&HostRoots::under_home(home.path())),
            agents_key
        )
        .installed
    );

    install(home.path(), agents_key);
    let entries = global_tool_entries(&HostRoots::under_home(home.path()));
    assert!(entry(&entries, agents_key).installed);
    assert!(installed_keys(&entries).contains(&agents_key.to_string()));
}

#[test]
fn global_catalog_groups_shared_skills_dirs_and_resolves_paths_under_home() {
    let home = tempfile::tempdir().unwrap();
    let entries = global_tool_entries(&HostRoots::under_home(home.path()));

    let amp = entry(&entries, "amp");
    let kimi = entry(&entries, "kimi_cli");
    assert_eq!(amp.shared_with, vec!["amp", "kimi_cli"]);
    assert_eq!(kimi.shared_with, amp.shared_with);

    let claude = entry(&entries, "claude_code");
    assert_eq!(claude.shared_with, vec!["claude_code"]);
    assert!(entries.iter().all(|e| e.shared_with.contains(&e.key)));

    // Honest grouping: the group's global dir (`~/.agents/skills`) is its own,
    // not the union of what its project-scope constituents read.
    let agents = entry(&entries, ToolId::AgentsStandard.as_key());
    assert_eq!(agents.shared_with, vec![agents.key]);
    assert_eq!(entry(&entries, "cursor").shared_with, vec!["cursor"]);

    // Grouping is `shared_with`, never the displayed `placement` dir (see
    // `ToolCatalogEntry`); it must match what the registry resolves here.
    assert_eq!(
        skills_dir_in(
            &HostRoots::under_home(home.path()),
            adapter_by_key("amp").unwrap()
        ),
        home.path().join(".config/agents/skills")
    );
    assert_eq!(
        skills_dir_in(
            &HostRoots::under_home(home.path()),
            adapter_by_key("kimi_cli").unwrap()
        ),
        skills_dir_in(
            &HostRoots::under_home(home.path()),
            adapter_by_key("amp").unwrap()
        )
    );
    assert_eq!(
        skills_dir_in(
            &HostRoots::under_home(home.path()),
            adapter_by_key("claude_code").unwrap()
        ),
        home.path().join(".claude/skills")
    );
}

#[test]
fn global_catalog_installedness_comes_from_home() {
    let home = tempfile::tempdir().unwrap();
    assert!(installed_keys(&global_tool_entries(&HostRoots::under_home(home.path()))).is_empty());

    install(home.path(), "claude_code");
    install(home.path(), "amp");
    let entries = global_tool_entries(&HostRoots::under_home(home.path()));
    assert!(entry(&entries, "claude_code").installed);
    assert!(entry(&entries, "amp").installed);
    // A shared skills dir does not make kimi_cli installed: it has its own root.
    assert!(!entry(&entries, "kimi_cli").installed);
    assert!(!entry(&entries, "cursor").installed);
    assert_eq!(
        installed_keys(&entries),
        vec!["claude_code", "amp"],
        "catalog order, not alphabetical"
    );
}

#[test]
fn project_catalog_absorbs_constituents_into_the_virtual_group() {
    let home = tempfile::tempdir().unwrap();
    let entries = project_tool_entries(&HostRoots::under_home(home.path()));

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
    assert!(
        !entry(
            &project_tool_entries(&HostRoots::under_home(home.path())),
            agents_key
        )
        .installed
    );

    install(home.path(), "codex");
    let entries = project_tool_entries(&HostRoots::under_home(home.path()));
    assert!(entry(&entries, agents_key).installed);
    assert!(!entry(&entries, "claude_code").installed);
    assert_eq!(installed_keys(&entries), vec![agents_key]);

    // Only constituents decide: the group entry's own detect dir (`~/.agents`)
    // is not a tool installation.
    let home2 = tempfile::tempdir().unwrap();
    install(home2.path(), agents_key);
    assert!(
        !entry(
            &project_tool_entries(&HostRoots::under_home(home2.path())),
            agents_key
        )
        .installed
    );
}

#[test]
fn project_catalog_groups_by_project_skills_dir() {
    let home = tempfile::tempdir().unwrap();
    let entries = project_tool_entries(&HostRoots::under_home(home.path()));

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

#[test]
fn only_the_global_catalog_carries_placement_straight_from_the_registry() {
    let home = tempfile::tempdir().unwrap();
    for entry in global_tool_entries(&HostRoots::under_home(home.path())) {
        let adapter = adapter_by_key(entry.key).unwrap();
        let placement = entry.placement.expect("every global entry is placed");
        assert_eq!(
            placement.skills_dir,
            format!("~/{}", adapter.skills_dir.rel)
        );
        assert_eq!(
            placement.detect_dirs,
            adapter
                .detect_dirs
                .iter()
                .map(|dir| format!("~/{}", dir.rel))
                .collect::<Vec<_>>()
        );
        assert_eq!(placement.supports_symlink, adapter.supports_symlink);
        // Spelled under `~`: the catalog never shows the operator's real home.
        assert!(placement.skills_dir.starts_with("~/"));
    }
    // Project dirs are project-relative and a group's detection is a union:
    // one field could not say both honestly, so the project list carries none.
    assert!(project_tool_entries(&HostRoots::under_home(home.path()))
        .iter()
        .all(|entry| entry.placement.is_none()));
}

// ---------------------------------------------------------------------------
// Reader-only harnesses (round 20 ticket 01)
// ---------------------------------------------------------------------------

/// A reader-only harness reads the `.agents/skills` entry's folder and has
/// no folder of its own: it is listed as a reader of that entry, with its
/// detected state, and is never an entry — never selectable, never in the
/// installed set the new-harness ask diffs, never resolvable as a target.
#[test]
fn a_reader_only_harness_is_listed_under_the_agents_entry_never_as_a_target() {
    let home = tempfile::tempdir().unwrap();
    let roots = HostRoots::under_home(home.path());
    let reader = test_overrides::sample_reader();
    let agents_key = ToolId::AgentsStandard.as_key();

    let entries = global_tool_entries(&roots);
    assert!(entries.iter().all(|e| e.key != reader.key), "not an entry");
    assert_eq!(
        entry(&entries, agents_key).readers,
        vec![ReaderEntry {
            key: reader.key,
            label: reader.display_name,
            installed: false,
        }]
    );
    assert!(
        entries
            .iter()
            .filter(|e| e.key != agents_key)
            .all(|e| e.readers.is_empty()),
        "only the group entry has readers"
    );

    assert!(
        !entry(&entries, agents_key).installed,
        "no reader, no `~/.agents`"
    );
    mark_reader_installed_in(&roots, reader);
    let entries = global_tool_entries(&roots);
    assert!(entry(&entries, agents_key).readers[0].installed, "detected");
    assert!(
        entry(&entries, agents_key).installed,
        "a detected reader makes its group installed: the group is its only way in"
    );
    assert!(
        !installed_keys(&entries).contains(&reader.key.to_string()),
        "never in the installed set the new-harness ask diffs"
    );
    assert!(adapter_by_key(reader.key).is_none(), "never a target");
}

/// At project scope the AGENTS group absorbs its reader-only members like its
/// constituents: they join its roster (the "(N harnesses)" count) and its
/// installedness, and are never entries of their own.
#[test]
fn the_project_agents_group_absorbs_reader_only_harnesses() {
    let home = tempfile::tempdir().unwrap();
    let roots = HostRoots::under_home(home.path());
    let reader = test_overrides::sample_reader();
    let agents_key = ToolId::AgentsStandard.as_key();

    let project = project_tool_entries(&roots);
    let agents = entry(&project, agents_key);
    assert_eq!(agents.constituents.last(), Some(&reader.display_name));
    assert_eq!(
        agents.constituents.len(),
        constituents_of(VirtualGroup::AgentsStandard).count() + 1
    );
    assert!(!agents.installed);
    assert!(agents.readers.is_empty(), "absorbed, not listed apart");
    assert!(project.iter().all(|e| e.key != reader.key));

    mark_reader_installed_in(&roots, reader);
    let project = project_tool_entries(&roots);
    assert!(
        entry(&project, agents_key).installed,
        "a detected reader makes the project folder worth deploying to"
    );
}

// ---------------------------------------------------------------------------
// The cross-read warning's pairs (round 20 ticket 02)
// ---------------------------------------------------------------------------

fn duplicate_readers_of(roots: &HostRoots, key: &str) -> Option<DuplicateReads> {
    entry(&global_tool_entries(roots), key)
        .duplicate_readers
        .clone()
}

/// Cursor reads `~/.claude/skills` and `~/.codex/skills` by default beside
/// its own folder: with Claude Code and Codex installed, a skill deployed to
/// Cursor and either one shows twice in Cursor.
#[test]
fn cursor_with_claude_code_and_codex_installed_sees_both() {
    let home = tempfile::tempdir().unwrap();
    let roots = HostRoots::under_home(home.path());
    for key in ["cursor", "claude_code", "codex"] {
        install(home.path(), key);
    }

    assert_eq!(
        duplicate_readers_of(&roots, "cursor"),
        Some(DuplicateReads {
            keys: vec!["claude_code", "codex"],
            behaviour: DuplicateBehaviour::ShowsAll,
        })
    );
    assert_eq!(
        duplicate_readers_of(&roots, "claude_code"),
        None,
        "Claude Code reads only its own folder"
    );
    assert_eq!(
        duplicate_readers_of(&roots, "codex"),
        None,
        "Codex reads ~/.agents/skills, but nothing deploys there here"
    );
}

/// Kimi Code reads `~/.agents/skills`, but its own deploy dir and its
/// `~/.claude` / `~/.codex` reads are legacy `kimi-cli`'s only: no pair.
/// Codex, which reads `~/.agents/skills` and its own folder by default, does.
#[test]
fn kimis_legacy_only_reads_make_no_pair() {
    let home = tempfile::tempdir().unwrap();
    let roots = HostRoots::under_home(home.path());
    for key in ["kimi_cli", "claude_code", "codex", "agents_skills"] {
        install(home.path(), key);
    }

    assert_eq!(duplicate_readers_of(&roots, "kimi_cli"), None);
    assert_eq!(
        duplicate_readers_of(&roots, "codex"),
        Some(DuplicateReads {
            keys: vec!["agents_skills"],
            behaviour: DuplicateBehaviour::ShowsAll,
        })
    );
}

/// A harness that is not installed deploys nothing: no pair names it.
#[test]
fn an_uninstalled_harness_makes_no_pair() {
    let home = tempfile::tempdir().unwrap();
    let roots = HostRoots::under_home(home.path());
    install(home.path(), "cursor");
    assert_eq!(duplicate_readers_of(&roots, "cursor"), None);

    install(home.path(), "claude_code");
    assert_eq!(
        duplicate_readers_of(&roots, "cursor").map(|d| d.keys),
        Some(vec!["claude_code"])
    );
}

/// An uninstalled reader sees nothing, so it carries no pair either.
#[test]
fn an_uninstalled_reader_carries_no_pair() {
    let home = tempfile::tempdir().unwrap();
    let roots = HostRoots::under_home(home.path());
    install(home.path(), "claude_code");
    install(home.path(), "codex");
    assert_eq!(duplicate_readers_of(&roots, "cursor"), None);
}

/// Amp reads `~/.config/agents/skills`, which is also Kimi's deploy dir:
/// they share one folder, deployed to once (the shared-dir dedupe), so it is
/// not a pair. Crush reads the same folder beside its own: a pair with both.
#[test]
fn shared_dir_siblings_are_not_a_pair() {
    let home = tempfile::tempdir().unwrap();
    let roots = HostRoots::under_home(home.path());
    for key in ["amp", "kimi_cli"] {
        install(home.path(), key);
    }
    assert_eq!(duplicate_readers_of(&roots, "amp"), None);
    assert_eq!(duplicate_readers_of(&roots, "kimi_cli"), None);

    install(home.path(), "crush");
    assert_eq!(
        duplicate_readers_of(&roots, "crush"),
        Some(DuplicateReads {
            keys: vec!["amp", "kimi_cli"],
            behaviour: DuplicateBehaviour::Unknown,
        })
    );
}

/// The pairs are a global fact: the project catalog carries none.
#[test]
fn the_project_catalog_carries_no_pairs() {
    let home = tempfile::tempdir().unwrap();
    let roots = HostRoots::under_home(home.path());
    for key in ["cursor", "claude_code", "codex"] {
        install(home.path(), key);
    }
    assert!(project_tool_entries(&roots)
        .iter()
        .all(|e| e.duplicate_readers.is_none()));
}
