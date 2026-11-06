//! Tests for `core::placement` — folder identity, the serving rule and the
//! per-folder answer, at both scopes, over substituted installed sets. Pure:
//! nothing here touches a filesystem except the temp roots paths are joined to.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::{
    capability_entry, fold_folder_rows, fold_status, folder_mode, folder_reachable, is_folder_row,
    own_folder, place, row_folder, row_reachable, serving_folder, DeployMode, FolderFacts,
    InstalledSet, PlacementScope, PlacementSettings, RowState, ScopeKind,
};
use crate::core::environment::HostRoots;
use crate::core::skill_store::SkillTargetRecord;
use crate::core::sync_status::{SyncMode, SyncStatus};
use crate::core::tool_adapters::{
    adapter_by_key, constituents_of, default_tool_adapters, test_overrides, ToolAdapter,
    VirtualGroup,
};

fn settings() -> PlacementSettings {
    PlacementSettings::default()
}

fn roots() -> HostRoots {
    HostRoots::under_home(Path::new("/home/op"))
}

/// Every dir two or more registry entries share at `scope`, keyed by the dir,
/// with its members in registry order — computed from the registry, so a new
/// shared dir fails the table test below until it is added there.
fn shared_dirs(scope: ScopeKind) -> BTreeMap<String, Vec<&'static str>> {
    let mut dirs: BTreeMap<String, Vec<&'static str>> = BTreeMap::new();
    for adapter in default_tool_adapters() {
        let dir = match scope {
            ScopeKind::Global => adapter.skills_dir.resolve(&roots()).display().to_string(),
            ScopeKind::Project => match adapter.project_skills_dir {
                Some(dir) => dir.to_string(),
                None => continue,
            },
        };
        dirs.entry(dir).or_default().push(adapter.key());
    }
    dirs.retain(|_, members| members.len() > 1);
    dirs
}

/// The folder-key rule over every dir the registry shares: the virtual group
/// entry when one owns the dir, else the first entry in registry order.
#[test]
fn every_shared_dir_has_its_folder_key() {
    let agents_members: Vec<&str> = std::iter::once("agents_skills")
        .chain(constituents_of(VirtualGroup::AgentsStandard).map(ToolAdapter::key))
        .collect();
    let expected: Vec<(ScopeKind, &str, &str)> = vec![
        // Amp and Kimi Code share `~/.config/agents/skills`; no group owns it.
        (ScopeKind::Global, "/home/op/.config/agents/skills", "amp"),
        // The AGENTS group owns `<project>/.agents/skills`.
        (ScopeKind::Project, ".agents/skills", "agents_skills"),
        (ScopeKind::Project, ".qoder/skills", "qoder"),
        (ScopeKind::Project, ".trae/skills", "trae"),
    ];

    for scope in [ScopeKind::Global, ScopeKind::Project] {
        let shared = shared_dirs(scope);
        let table: Vec<&(ScopeKind, &str, &str)> =
            expected.iter().filter(|(s, _, _)| *s == scope).collect();
        assert_eq!(
            shared.keys().map(String::as_str).collect::<Vec<_>>(),
            {
                let mut dirs: Vec<&str> = table.iter().map(|(_, dir, _)| *dir).collect();
                dirs.sort();
                dirs
            },
            "{scope:?}: every shared dir is in the table"
        );
        for (_, dir, folder) in table {
            for member in &shared[*dir] {
                assert_eq!(
                    own_folder(scope, member),
                    Some(*folder),
                    "{scope:?} {dir}: {member} reads as folder {folder}"
                );
            }
        }
    }
    // The project `.agents` folder is every constituent's.
    assert_eq!(
        shared_dirs(ScopeKind::Project)[".agents/skills"],
        agents_members
    );
}

#[test]
fn the_two_shared_folders_take_their_convention_keys() {
    assert_eq!(
        own_folder(ScopeKind::Global, "agents_skills"),
        Some("agents_skills")
    );
    assert_eq!(
        own_folder(ScopeKind::Global, "claude_code"),
        Some("claude_code")
    );
    assert_eq!(
        own_folder(ScopeKind::Project, "agents_skills"),
        Some("agents_skills")
    );
    assert_eq!(
        own_folder(ScopeKind::Project, "claude_code"),
        Some("claude_code")
    );
    // A constituent's own global folder is its own; at project scope it is the group's.
    assert_eq!(own_folder(ScopeKind::Global, "cursor"), Some("cursor"));
    assert_eq!(
        own_folder(ScopeKind::Project, "cursor"),
        Some("agents_skills")
    );
}

/// Today every harness is served by its own folder, at both scopes.
#[test]
fn every_harness_is_served_by_its_own_folder() {
    for adapter in default_tool_adapters() {
        for scope in [ScopeKind::Global, ScopeKind::Project] {
            assert_eq!(
                serving_folder(scope, adapter.key(), &settings()),
                own_folder(scope, adapter.key()),
                "{scope:?} {}",
                adapter.key()
            );
        }
        assert!(serving_folder(ScopeKind::Global, adapter.key(), &settings()).is_some());
    }
}

#[test]
fn unknown_keys_and_project_less_harnesses_have_no_folder() {
    assert_eq!(own_folder(ScopeKind::Global, "no_such_tool"), None);
    // A stored row of an unknown key stands for itself.
    assert_eq!(
        row_folder(ScopeKind::Global, "no_such_tool"),
        "no_such_tool"
    );
    // WorkBuddy reads no project skills dir.
    assert_eq!(own_folder(ScopeKind::Project, "workbuddy"), None);
    assert_eq!(row_folder(ScopeKind::Global, "kimi_cli"), "amp");
    assert_eq!(row_folder(ScopeKind::Project, "trae_cn"), "trae");
}

/// The operator's machine (Claude Code, Codex and Pi installed): every
/// harness its own folder; the `.agents` folder is read by Codex and Pi.
#[test]
fn the_operators_machine_places_three_own_folders() {
    let roots = roots();
    let installed = InstalledSet::from_keys(["claude_code", "codex", "pi"]);
    let layout = place(PlacementScope::Global(&roots), &installed, &settings());

    for key in ["claude_code", "codex", "pi"] {
        let folder = layout.folder_of(key).expect("placed");
        assert_eq!(folder.key, key);
        assert_eq!(folder.served, vec![key]);
        assert_eq!(folder.mode, DeployMode::Link);
        assert_eq!(
            folder.path,
            adapter_by_key(key).unwrap().skills_dir.resolve(&roots)
        );
    }
    let agents = layout.folder("agents_skills").expect("agents folder");
    assert!(agents.served.is_empty(), "nothing detects `~/.agents`");
    assert_eq!(agents.undetected, {
        let mut undetected = vec!["agents_skills"];
        undetected.extend(
            crate::core::tool_adapters::reader_only_harnesses()
                .iter()
                .map(|r| r.key),
        );
        undetected
    });
    assert_eq!(agents.readers, vec!["codex", "pi"]);
    let claude = layout.folder("claude_code").unwrap();
    assert!(
        claude.readers.is_empty(),
        "neither Codex nor Pi reads `~/.claude/skills`"
    );
    // Every registry entry is placed, each harness once.
    assert_eq!(layout.harnesses.len(), default_tool_adapters().len());
    // Only detected members are served; the rest are kept apart.
    let cursor = layout.folder("cursor").unwrap();
    assert!(cursor.served.is_empty());
    assert_eq!(cursor.undetected, vec!["cursor"]);
}

#[test]
fn amp_and_kimi_are_served_by_one_folder() {
    let roots = roots();
    let both = place(
        PlacementScope::Global(&roots),
        &InstalledSet::from_keys(["amp", "kimi_cli"]),
        &settings(),
    );
    assert_eq!(both.folder_of("kimi_cli").unwrap().key, "amp");
    assert_eq!(both.folder("amp").unwrap().served, vec!["amp", "kimi_cli"]);
    assert!(both.folder("kimi_cli").is_none(), "no folder of its own");

    let kimi_only = place(
        PlacementScope::Global(&roots),
        &InstalledSet::from_keys(["kimi_cli"]),
        &settings(),
    );
    let folder = kimi_only.folder("amp").unwrap();
    assert_eq!(folder.served, vec!["kimi_cli"]);
    assert_eq!(folder.undetected, vec!["amp"]);
}

#[test]
fn a_detected_reader_only_harness_is_listed_under_the_folder_serving_it() {
    let roots = roots();
    let layout = place(
        PlacementScope::Global(&roots),
        &InstalledSet::from_keys(["zed", "fx", "claude_code"]),
        &settings(),
    );
    let agents = layout.folder("agents_skills").unwrap();
    assert_eq!(agents.reader_only, vec!["zed", "fx"]);
    assert!(!agents.undetected.contains(&"zed"));
    assert!(agents.undetected.contains(&"warp"));
    // fx also reads `~/.claude/skills` by default: a reader there.
    assert_eq!(layout.folder("claude_code").unwrap().readers, vec!["fx"]);
}

#[test]
fn the_deploy_mode_is_the_registry_capability() {
    assert_eq!(
        folder_mode(ScopeKind::Global, "deep_agents_code"),
        DeployMode::Copy
    );
    assert_eq!(
        folder_mode(ScopeKind::Global, "claude_code"),
        DeployMode::Link
    );
    assert_eq!(DeployMode::Copy.first_attempt(), SyncMode::Copy);
    assert_eq!(DeployMode::Link.first_attempt(), SyncMode::Symlink);
}

/// A plain shared folder is a copy when any entry sharing it is copy-only; a
/// group's folder keeps the group's own capability (never its members' AND).
#[test]
fn a_copy_only_member_makes_a_plain_shared_folder_a_copy_but_never_a_groups() {
    let mut kimi = adapter_by_key("kimi_cli").unwrap().clone();
    kimi.supports_symlink = false;
    test_overrides::shadow(kimi);
    assert_eq!(folder_mode(ScopeKind::Global, "amp"), DeployMode::Copy);
    assert_eq!(
        capability_entry(ScopeKind::Global, "amp").unwrap().key(),
        "kimi_cli"
    );

    let mut cursor = adapter_by_key("cursor").unwrap().clone();
    cursor.supports_symlink = false;
    test_overrides::shadow(cursor);
    assert_eq!(
        folder_mode(ScopeKind::Project, "agents_skills"),
        DeployMode::Link
    );
}

#[test]
fn project_scope_places_folders_under_the_project_root() {
    let root = PathBuf::from("/work/app");
    let layout = place(
        PlacementScope::Project(&root),
        &InstalledSet::from_keys(["cursor", "codex", "trae_cn", "claude_code"]),
        &settings(),
    );
    let agents = layout.folder_of("cursor").unwrap();
    assert_eq!(agents.key, "agents_skills");
    assert_eq!(agents.path, root.join(".agents").join("skills"));
    assert_eq!(agents.served, vec!["cursor", "codex"]);
    assert_eq!(layout.folder_of("trae_cn").unwrap().key, "trae");
    assert_eq!(layout.folder("trae").unwrap().served, vec!["trae_cn"]);
    // Cursor reads the project `.claude/skills` too.
    assert_eq!(
        layout.folder("claude_code").unwrap().readers,
        vec!["cursor"]
    );
    assert!(layout.folder_of("workbuddy").is_none(), "no project dir");
}

fn row(id: &str, skill: &str, tool: &str, status: SyncStatus) -> SkillTargetRecord {
    SkillTargetRecord {
        id: id.to_string(),
        skill_id: skill.to_string(),
        tool: tool.to_string(),
        target_path: format!("/home/op/.config/agents/skills/{skill}"),
        mode: SyncMode::Symlink,
        status,
        last_error: None,
        synced_at: Some(1),
    }
}

/// Rows written once per shared-dir sibling read as one folder, keyed by it.
#[test]
fn sibling_rows_read_as_one_folder() {
    let rows = vec![
        row("1", "alpha", "kimi_cli", SyncStatus::Synced),
        row("2", "alpha", "amp", SyncStatus::Error),
        row("3", "alpha", "claude_code", SyncStatus::Synced),
        row("4", "beta", "kimi_cli", SyncStatus::Synced),
    ];
    let folded = fold_folder_rows(ScopeKind::Global, rows);
    let seen: Vec<(&str, &str, &str)> = folded
        .iter()
        .map(|r| (r.id.as_str(), r.skill_id.as_str(), r.tool.as_str()))
        .collect();
    assert_eq!(
        seen,
        vec![
            // The row already keyed by the folder represents it.
            ("2", "alpha", "amp"),
            ("3", "alpha", "claude_code"),
            // A lone sibling row comes out keyed by its folder.
            ("4", "beta", "amp"),
        ]
    );
}

/// A disabled sibling never hides the folder's other rows: a synced row
/// beside it shows the folder muted (`disabled`), the intent is carried, and
/// the synced row's artifact still counts as deployed.
#[test]
fn a_disabled_sibling_mutes_the_folder_and_keeps_its_live_artifact_counted() {
    let rows = vec![
        row("1", "alpha", "amp", SyncStatus::Synced),
        row("2", "alpha", "kimi_cli", SyncStatus::Disabled),
    ];
    let folded = fold_folder_rows(ScopeKind::Global, rows);
    assert_eq!(folded.len(), 1);
    assert_eq!(folded[0].id, "2");
    assert_eq!(folded[0].tool, "amp");
    assert_eq!(
        folded[0].facts,
        FolderFacts {
            status: SyncStatus::Disabled,
            disabled: true,
            deployed: true,
        }
    );
    assert_eq!(folded[0].facts.total_status(), SyncStatus::Synced);
}

/// Astra 204 / Sol 301: a partial Disable (one row's removal failed, the
/// sibling's settled) shows the failure, failure ahead of muted, with the
/// disabled intent kept apart — and the row standing for the folder is the
/// failed one, with its own timestamp.
#[test]
fn a_failed_removal_shows_beside_a_disabled_sibling() {
    let mut failed = row("1", "alpha", "amp", SyncStatus::Error);
    failed.synced_at = Some(7);
    let mut disabled = row("2", "alpha", "kimi_cli", SyncStatus::Disabled);
    disabled.synced_at = Some(3);
    let folded = fold_folder_rows(ScopeKind::Global, vec![disabled, failed]);
    assert_eq!(folded.len(), 1);
    assert_eq!((folded[0].id.as_str(), folded[0].synced_at), ("1", Some(7)));
    assert_eq!(
        folded[0].facts,
        FolderFacts {
            status: SyncStatus::Error,
            disabled: true,
            deployed: true,
        }
    );
}

/// The one fold every surface reads a folder's state from: failure ahead of
/// muted (`error` › `missing` › `stale` › `disabled` › `pending` › `synced`),
/// the disabled intent from any row, and "deployed" only from a live row that
/// owns its artifact.
#[test]
fn a_folders_status_is_one_fold_over_all_its_rows() {
    use SyncStatus::*;
    let owned = |status| RowState {
        status,
        owns_artifact: true,
    };
    let cases: &[(&[SyncStatus], Option<SyncStatus>)] = &[
        (&[], None),
        (&[Synced], Some(Synced)),
        (&[Synced, Pending], Some(Pending)),
        (&[Pending, Stale], Some(Stale)),
        (&[Stale, Missing], Some(Missing)),
        (&[Missing, Error, Synced], Some(Error)),
        (&[Error, Disabled], Some(Error)),
        (&[Missing, Disabled], Some(Missing)),
        (&[Stale, Disabled], Some(Stale)),
        (&[Disabled, Synced], Some(Disabled)),
        (&[Pending, Disabled], Some(Disabled)),
    ];
    for (statuses, folded) in cases {
        assert_eq!(
            fold_status(statuses.iter().copied().map(owned)).map(|f| f.status),
            *folded,
            "{statuses:?}"
        );
    }
    let facts = |rows: &[(SyncStatus, bool)]| {
        fold_status(rows.iter().map(|&(status, owns_artifact)| RowState {
            status,
            owns_artifact,
        }))
        .unwrap()
    };
    // Only a live owner counts as deployed: a disabled row that still
    // records `synced_at`, or a never-synced error row, does not.
    assert!(!facts(&[(Disabled, true)]).deployed);
    assert!(!facts(&[(Error, false), (Disabled, true)]).deployed);
    assert!(facts(&[(Error, true), (Disabled, true)]).deployed);
    assert!(facts(&[(Error, false), (Disabled, false)]).disabled);
    assert!(!facts(&[(Error, false), (Synced, true)]).disabled);
    // A folder all of whose rows are disabled adds nothing to a total.
    assert_eq!(facts(&[(Disabled, true)]).total_status(), Disabled);
}

/// The one rule mutations read (ADR-0007 rule 2): a row keyed by its own
/// folder key is a folder row; any other key — a pre-1.4.0 sibling's, an
/// unknown one — is a legacy row.
#[test]
fn a_row_keyed_by_its_folder_is_a_folder_row_and_any_other_is_legacy() {
    for (scope, key, folder_row) in [
        (ScopeKind::Global, "amp", true),
        (ScopeKind::Global, "kimi_cli", false),
        (ScopeKind::Global, "claude_code", true),
        (ScopeKind::Global, "agents_skills", true),
        (ScopeKind::Global, "cursor", true),
        (ScopeKind::Global, "no_such_tool", false),
        (ScopeKind::Project, "trae", true),
        (ScopeKind::Project, "trae_cn", false),
        (ScopeKind::Project, "agents_skills", true),
        (ScopeKind::Project, "no_such_tool", false),
    ] {
        assert_eq!(is_folder_row(scope, key), folder_row, "{scope:?} {key}");
    }
}

/// A folder row is reachable through any installed harness its folder
/// serves; a legacy row only through its own harness (Astra 203).
#[test]
fn a_folder_row_is_reached_through_its_folder_and_a_legacy_row_through_its_harness() {
    let amp_only = InstalledSet::from_keys(["amp"]);
    let kimi_only = InstalledSet::from_keys(["kimi_cli"]);
    assert!(folder_reachable(ScopeKind::Global, "amp", &kimi_only));
    assert!(row_reachable(ScopeKind::Global, "amp", &kimi_only));
    assert!(row_reachable(ScopeKind::Global, "amp", &amp_only));
    assert!(!row_reachable(ScopeKind::Global, "kimi_cli", &amp_only));
    assert!(row_reachable(ScopeKind::Global, "kimi_cli", &kimi_only));
    assert!(!folder_reachable(
        ScopeKind::Global,
        "amp",
        &InstalledSet::from_keys(["claude_code"])
    ));
}

/// The fold reads every row of the folder, whatever path it records: a row
/// recording another name is not set aside, so an error on it shows — and
/// the per-row resync of the folder is what settles it.
#[test]
fn a_row_recording_another_path_still_folds_into_its_folder() {
    let mut renamed = row("2", "alpha", "kimi_cli", SyncStatus::Error);
    renamed.target_path = "/home/op/.config/agents/skills/old-name".to_string();
    renamed.synced_at = None;
    let folded = fold_folder_rows(
        ScopeKind::Global,
        vec![row("1", "alpha", "amp", SyncStatus::Synced), renamed],
    );
    assert_eq!(folded.len(), 1);
    assert_eq!(
        (
            folded[0].id.as_str(),
            folded[0].tool.as_str(),
            folded[0].status
        ),
        ("2", "amp", SyncStatus::Error)
    );
}
