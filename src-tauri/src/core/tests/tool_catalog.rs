use std::path::Path;

use crate::core::environment::{HostOs, HostRoot, HostRoots};
use crate::core::tool_adapters::catalog::{DuplicateReads, ReaderEntry};
use crate::core::tool_adapters::reads::DuplicateBehaviour;
use crate::core::tool_adapters::{
    adapter_by_key, constituents_of, default_tool_adapters, global_tool_entries, installed_keys,
    mark_installed_in, mark_reader_installed_in, project_tool_entries, skills_dir_in,
    test_overrides, RegistryPath, ToolCatalogEntry, ToolId, VirtualGroup,
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
            // round 20 ticket 07 rows
            "Letta Code",
            "Muse Code",
            "Poolside",
            "Dirac",
            // round 20 ticket 05 rows
            "Warp",
            "Zed",
            "Dexto",
            "fx",
            "Sarvam Code",
            "VT Code",
            "DimCode",
            "crow-cli",
            "Corust Agent",
        ],
        "adapters first, then the reader-only members"
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
    assert_eq!(
        agents.constituents.len(),
        constituents_of(VirtualGroup::AgentsStandard).count()
            + crate::core::tool_adapters::reader_only_harnesses().len(),
        "every adapter member plus every reader-only member"
    );
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
    // `HostRoots::under_home` lays the app-data roots out as Windows does, so
    // a path whose effective root on this host is one of them (Reasonix on
    // Windows) is spelled under `~/AppData/…`.
    let shown = |path: &RegistryPath| {
        let (root, rel) = path.effective(HostOs::CURRENT);
        match root {
            HostRoot::Home => format!("~/{rel}"),
            HostRoot::RoamingAppData => format!("~/AppData/Roaming/{rel}"),
            HostRoot::LocalAppData => format!("~/AppData/Local/{rel}"),
        }
    };
    for entry in global_tool_entries(&HostRoots::under_home(home.path())) {
        let adapter = adapter_by_key(entry.key).unwrap();
        let placement = entry.placement.expect("every global entry is placed");
        assert_eq!(placement.skills_dir, shown(&adapter.skills_dir));
        assert_eq!(
            placement.detect_dirs,
            adapter.detect_dirs.iter().map(shown).collect::<Vec<_>>()
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
        entry(&entries, agents_key).readers.last(),
        Some(&ReaderEntry {
            key: reader.key,
            label: reader.display_name,
            installed: false,
            duplicate_readers: None,
        }),
        "the test row follows the shipped readers"
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
    assert!(
        entry(&entries, agents_key)
            .readers
            .last()
            .unwrap()
            .installed,
        "detected"
    );
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
        constituents_of(VirtualGroup::AgentsStandard).count()
            + crate::core::tool_adapters::reader_only_harnesses().len()
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

// ---------------------------------------------------------------------------
// round 20 ticket 05 rows: reader-only harnesses' cross-read pairs
// ---------------------------------------------------------------------------

fn reader_entry_of(roots: &HostRoots, key: &str) -> ReaderEntry {
    entry(&global_tool_entries(roots), ToolId::AgentsStandard.as_key())
        .readers
        .iter()
        .find(|reader| reader.key == key)
        .cloned()
        .unwrap_or_else(|| panic!("reader {key}"))
}

fn install_reader(roots: &HostRoots, key: &str) {
    let reader = crate::core::tool_adapters::reader_only_harnesses()
        .into_iter()
        .find(|reader| reader.key == key)
        .unwrap_or_else(|| panic!("reader {key}"));
    mark_reader_installed_in(roots, reader);
}

/// fx reads `~/.agents/skills` (the folder its group entry deploys to) and
/// also `~/.claude/skills`, `~/.codex/skills` and OpenCode's folder: with
/// those installed, a skill deployed to `.agents/skills` and one of them
/// can't be invoked by name in fx (brief T35) — the pair is carried on fx's
/// reader entry, its own deploy dir being the group entry's.
#[test]
fn fx_reads_the_claude_codex_and_opencode_folders_beside_the_agents_folder() {
    let home = tempfile::tempdir().unwrap();
    let roots = HostRoots::under_home(home.path());
    for key in ["claude_code", "codex", "opencode", "cursor"] {
        install(home.path(), key);
    }
    assert_eq!(
        reader_entry_of(&roots, "fx").duplicate_readers,
        None,
        "an undetected reader sees nothing"
    );

    install_reader(&roots, "fx");
    assert_eq!(
        reader_entry_of(&roots, "fx").duplicate_readers,
        Some(DuplicateReads {
            keys: vec!["claude_code", "codex", "opencode"],
            behaviour: DuplicateBehaviour::LoadsNone,
        })
    );
}

/// Zed reads only `~/.agents/skills`: never a pair. A reader whose read of
/// that folder is not verified (Sarvam Code, LIKELY) carries none either.
#[test]
fn a_reader_of_the_agents_folder_alone_or_unverified_carries_no_pair() {
    let home = tempfile::tempdir().unwrap();
    let roots = HostRoots::under_home(home.path());
    for key in ["claude_code", "codex", "opencode", "cursor"] {
        install(home.path(), key);
    }
    for key in ["zed", "sarvam_code"] {
        install_reader(&roots, key);
        assert_eq!(
            reader_entry_of(&roots, key).duplicate_readers,
            None,
            "{key}"
        );
    }
}

/// Warp lists every copy in every folder it reads (brief T22).
#[test]
fn warp_shows_every_copy_of_the_folders_it_also_reads() {
    let home = tempfile::tempdir().unwrap();
    let roots = HostRoots::under_home(home.path());
    for key in ["claude_code", "codex", "cursor", "droid", "pi"] {
        install(home.path(), key);
    }
    install_reader(&roots, "warp");
    assert_eq!(
        reader_entry_of(&roots, "warp").duplicate_readers,
        Some(DuplicateReads {
            keys: vec!["cursor", "claude_code", "codex", "droid"],
            behaviour: DuplicateBehaviour::ShowsAll,
        })
    );
}

// round 20 ticket 07 rows
/// Kimchi reads another agent's folder "if installed" — the very condition a
/// pair requires, so the read counts: Kimchi with Pi installed sees a Pi
/// deployment beside its own. Its `~/.claude/skills` read is opt-in, so
/// Claude Code makes no pair, and its home reads of the other agents' folders
/// (Cursor, Codex) are only a fallback for a project without its own folder,
/// so they make none either (round 20 rows review, P2).
#[test]
fn kimchi_reads_an_installed_agents_folder_but_not_claudes_by_default() {
    let home = tempfile::tempdir().unwrap();
    let roots = HostRoots::under_home(home.path());
    for key in [
        "kimchi",
        "pi",
        "claude_code",
        "cursor",
        "codex",
        "agents_skills",
    ] {
        install(home.path(), key);
    }

    assert_eq!(
        duplicate_readers_of(&roots, "kimchi"),
        Some(DuplicateReads {
            keys: vec!["pi"],
            behaviour: DuplicateBehaviour::Shadows,
        })
    );
}

// round 20 ticket 06 rows

/// With every harness installed, the ticket 06 rows resolve the pairs their
/// brief facts promise (the fixture's `DUPLICATE_READS` restates them):
/// Cortex Code flags a conflict with Claude Code's copy, Grok Build and
/// Firebender also read Cursor's folder, and the harnesses that read no
/// shared folder (IBM Bob, AiderDesk) make no pair.
#[test]
fn the_ticket_06_rows_resolve_their_cross_read_pairs() {
    let home = tempfile::tempdir().unwrap();
    let roots = HostRoots::under_home(home.path());
    for adapter in default_tool_adapters() {
        mark_installed_in(&roots, adapter);
    }
    let pair = |keys: &[&'static str], behaviour| {
        Some(DuplicateReads {
            keys: keys.to_vec(),
            behaviour,
        })
    };
    use DuplicateBehaviour::{FlagsConflict, Shadows, ShowsAll, Unknown};
    let expected = [
        (
            "grok_build",
            pair(&["agents_skills", "cursor", "claude_code"], Shadows),
        ),
        (
            "deep_agents_code",
            pair(&["agents_skills", "claude_code"], Shadows),
        ),
        ("reasonix", pair(&["agents_skills", "claude_code"], Unknown)),
        ("cortex_code", pair(&["claude_code"], FlagsConflict)),
        ("rovo_dev", pair(&["agents_skills"], Unknown)),
        ("ibm_bob", None),
        ("forgecode", pair(&["agents_skills"], Shadows)),
        (
            "firebender",
            pair(
                &["agents_skills", "cursor", "claude_code", "codex"],
                ShowsAll,
            ),
        ),
        (
            "minimax_code",
            pair(&["agents_skills", "claude_code", "codex"], Shadows),
        ),
        ("posit_assistant", pair(&["agents_skills"], Shadows)),
        ("aider_desk", None),
        (
            "autohand_code",
            pair(&["agents_skills", "claude_code", "codex"], Shadows),
        ),
    ];
    for (key, pairs) in expected {
        assert_eq!(duplicate_readers_of(&roots, key), pairs, "{key}");
    }
}

/// Each catalog entry carries the folder key of the Deployment folder serving
/// it at the catalog's scope — what the frontend matches rows by — and its
/// `shared_with` group is exactly the entries with that folder.
#[test]
fn every_catalog_entry_carries_its_folder_key() {
    let home = tempfile::tempdir().unwrap();
    let roots = HostRoots::under_home(home.path());

    let global = global_tool_entries(&roots);
    assert_eq!(entry(&global, "kimi_cli").folder_key, "amp");
    assert_eq!(entry(&global, "amp").folder_key, "amp");
    assert_eq!(entry(&global, "cursor").folder_key, "cursor");
    assert_eq!(entry(&global, "agents_skills").folder_key, "agents_skills");
    for e in &global {
        let same_folder: Vec<&str> = global
            .iter()
            .filter(|other| other.folder_key == e.folder_key)
            .map(|other| other.key)
            .collect();
        assert_eq!(e.shared_with, same_folder, "{}", e.key);
    }

    let project = project_tool_entries(&roots);
    assert_eq!(entry(&project, "trae_cn").folder_key, "trae");
    assert_eq!(entry(&project, "qoder_cn_cli").folder_key, "qoder");
    assert_eq!(entry(&project, "agents_skills").folder_key, "agents_skills");
    assert_eq!(entry(&project, "claude_code").folder_key, "claude_code");
}
