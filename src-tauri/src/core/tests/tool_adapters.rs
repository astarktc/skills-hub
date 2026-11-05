use std::fs;

use crate::core::errors::SignalError;
use crate::core::tool_adapters::{
    adapter_by_key, adapter_succeeding_key, adapters_sharing_skills_dir, constituents_of,
    default_tool_adapters, detect_dirs_in, ensure_path_within_tool_dirs, former_skills_dirs_in,
    global_skills_dir_in, is_installed_in, scan_tool_dir, skills_dir_in, superseded_skills_dirs_in,
    tool_holding_path, AgentsStandardRead, GlobalLayout, Precedence, ToolAdapter, ToolId,
    VirtualGroup, AGENTS_STANDARD_DIR, RETIRED_TOOL_KEYS,
};

#[test]
fn path_resolution_joins_adapter_dirs_onto_home() {
    let home = tempfile::tempdir().unwrap();
    let cases: [(&str, &str, &[&str]); 6] = [
        ("claude_code", ".claude/skills", &[".claude"]),
        // Codex deploys to the AGENTS-standard dir (brief I2) but is detected
        // by its own root.
        ("codex", ".agents/skills", &[".codex"]),
        ("cursor", ".cursor/skills", &[".cursor"]),
        // Amp keeps the `~/.config/agents/skills` convention dir, detected by
        // its own config root; Kimi moved to its own dir (brief I4) and is
        // detected by either root it used across releases.
        ("amp", ".config/agents/skills", &[".config/amp"]),
        ("kimi_cli", ".kimi-code/skills", &[".kimi-code", ".kimi"]),
        ("pi", ".pi/agent/skills", &[".pi"]),
    ];
    for (key, skills, detect) in cases {
        let adapter = adapter_by_key(key).unwrap_or_else(|| panic!("adapter {key}"));
        assert_eq!(
            skills_dir_in(home.path(), adapter),
            home.path().join(skills),
            "skills dir for {key}"
        );
        assert_eq!(
            detect_dirs_in(home.path(), adapter),
            detect
                .iter()
                .map(|d| home.path().join(d))
                .collect::<Vec<_>>(),
            "detect dirs for {key}"
        );
    }
}

#[test]
fn every_adapter_resolves_under_home() {
    let home = tempfile::tempdir().unwrap();
    for adapter in default_tool_adapters() {
        assert!(skills_dir_in(home.path(), adapter).starts_with(home.path()));
        for detect_dir in detect_dirs_in(home.path(), adapter) {
            assert!(detect_dir.starts_with(home.path()));
        }
        assert!(
            !is_installed_in(home.path(), adapter),
            "{} must not be installed in an empty home",
            adapter.id.as_key()
        );
    }
}

#[test]
fn installedness_is_decided_by_detect_dir_not_skills_dir() {
    let home = tempfile::tempdir().unwrap();
    let codex = adapter_by_key("codex").unwrap();
    let claude = adapter_by_key("claude_code").unwrap();

    assert!(!is_installed_in(home.path(), codex));

    // Detect dir present (even without a skills dir) => installed.
    fs::create_dir_all(home.path().join(".codex")).unwrap();
    assert!(is_installed_in(home.path(), codex));
    assert!(!is_installed_in(home.path(), claude));

    // Only the adapter's own detect dir counts — a sibling under the same
    // parent does not.
    let amp = adapter_by_key("amp").unwrap();
    fs::create_dir_all(home.path().join(".config/other")).unwrap();
    assert!(!is_installed_in(home.path(), amp));
    fs::create_dir_all(home.path().join(".config/amp")).unwrap();
    assert!(is_installed_in(home.path(), amp));
    // Shared-skills-dir tools are detected independently, by their own roots.
    let kimi = adapter_by_key("kimi_cli").unwrap();
    assert!(!is_installed_in(home.path(), kimi));
}

#[test]
fn a_tool_whose_skills_dir_is_a_shared_convention_is_detected_by_its_own_root() {
    // The vendor-documented Kimi / Amp layout: config root beside the shared
    // `~/.config/agents/skills` convention, which alone is a footprint.
    let home = tempfile::tempdir().unwrap();
    let kimi = adapter_by_key("kimi_cli").unwrap();
    let amp = adapter_by_key("amp").unwrap();

    fs::create_dir_all(home.path().join(".config/agents/skills/foo")).unwrap();
    assert!(!is_installed_in(home.path(), kimi));
    assert!(!is_installed_in(home.path(), amp));

    fs::create_dir_all(home.path().join(".kimi")).unwrap();
    fs::write(home.path().join(".kimi/config.toml"), b"").unwrap();
    assert!(is_installed_in(home.path(), kimi), "kimi-cli.com layout");
    assert!(!is_installed_in(home.path(), amp));

    fs::remove_dir_all(home.path().join(".kimi")).unwrap();
    fs::create_dir_all(home.path().join(".kimi-code")).unwrap();
    assert!(is_installed_in(home.path(), kimi), "kimi.com/code layout");

    fs::create_dir_all(home.path().join(".config/amp")).unwrap();
    fs::write(home.path().join(".config/amp/settings.json"), b"{}").unwrap();
    assert!(is_installed_in(home.path(), amp));
}

#[test]
fn a_skills_only_footprint_is_not_an_installed_tool() {
    let home = tempfile::tempdir().unwrap();
    let kiro = adapter_by_key("kiro_cli").unwrap();

    // What a skill deployer leaves behind: the skills path and nothing else.
    fs::create_dir_all(home.path().join(".kiro/skills/ego-browser")).unwrap();
    fs::write(home.path().join(".kiro/skills/ego-browser/SKILL.md"), b"x").unwrap();
    assert!(!is_installed_in(home.path(), kiro));

    // An empty skills dir is the same footprint.
    fs::remove_dir_all(home.path().join(".kiro/skills/ego-browser")).unwrap();
    assert!(!is_installed_in(home.path(), kiro));

    // Finder noise does not promote a footprint.
    fs::write(home.path().join(".kiro/.DS_Store"), b"").unwrap();
    assert!(!is_installed_in(home.path(), kiro));

    // Anything else beside the skills path is the tool.
    fs::write(home.path().join(".kiro/settings.json"), b"{}").unwrap();
    assert!(is_installed_in(home.path(), kiro));
}

#[test]
fn footprint_rule_walks_every_level_down_to_the_skills_dir() {
    let home = tempfile::tempdir().unwrap();
    let pi = adapter_by_key("pi").unwrap();

    // `.pi/agent/skills/x` alone: `.pi` holds only `agent`, `agent` only `skills`.
    fs::create_dir_all(home.path().join(".pi/agent/skills/x")).unwrap();
    assert!(!is_installed_in(home.path(), pi));

    // A sibling at an intermediate level is the tool.
    fs::write(home.path().join(".pi/agent/settings.json"), b"{}").unwrap();
    assert!(is_installed_in(home.path(), pi));
}

#[test]
fn a_virtual_group_is_installed_by_its_directory_alone() {
    let home = tempfile::tempdir().unwrap();
    let agents = adapter_by_key("agents_skills").unwrap();
    assert!(agents.as_virtual_group().is_some());

    // `~/.agents` legitimately holds only `skills/`: the convention is the dir.
    fs::create_dir_all(home.path().join(".agents/skills/x")).unwrap();
    assert!(is_installed_in(home.path(), agents));
}

#[test]
fn no_adapter_is_detected_by_a_shared_skills_convention_dir() {
    // The parent of a skills dir several tools share (`~/.config/agents`)
    // is a convention, not a tool: presence there is exactly the footprint
    // the rule rejects. Only a virtual group entry may be detected by one.
    for adapter in default_tool_adapters() {
        assert!(
            !adapter.relative_detect_dirs.is_empty(),
            "{} has no detect dir",
            adapter.id.as_key()
        );
        if adapter.as_virtual_group().is_some() {
            continue;
        }
        for layout in [GlobalLayout::OWN_DIRS, GlobalLayout::PREFER_AGENTS_STANDARD] {
            let shared = adapters_sharing_skills_dir(adapter, layout).len() > 1;
            let convention_root =
                std::path::Path::new(adapter.global_relative_skills_dir(layout)).parent();
            for detect_dir in adapter.relative_detect_dirs {
                assert!(
                    !(shared && convention_root == Some(std::path::Path::new(detect_dir))),
                    "{}: detected by the shared convention dir {}",
                    adapter.id.as_key(),
                    detect_dir
                );
            }
        }
    }
}

#[test]
fn adapter_by_key_finds_known_tool() {
    let a = adapter_by_key("codex").unwrap();
    assert_eq!(a.id, ToolId::Codex);
}

#[test]
fn adapter_by_key_finds_new_tools() {
    assert!(adapter_by_key("kimi_cli").is_some());
    assert!(adapter_by_key("augment").is_some());
    assert!(adapter_by_key("command_code").is_some());
    assert!(adapter_by_key("qwen_code").is_some());
    // Brief § 4 / I13.
    for key in ["deepseek_harness", "zcode", "codewhale", "workbuddy"] {
        assert!(adapter_by_key(key).is_some(), "{key}");
    }
}

#[test]
fn adapters_sharing_skills_dir_groups_the_agents_standard_deployers() {
    let codex = adapter_by_key("codex").unwrap();
    let keys: Vec<&str> = adapters_sharing_skills_dir(codex, GlobalLayout::OWN_DIRS)
        .into_iter()
        .map(|a| a.key())
        .collect();
    assert_eq!(keys, vec!["agents_skills", "codex", "openhands", "goose"]);
    let amp = adapter_by_key("amp").unwrap();
    let keys: Vec<&str> = adapters_sharing_skills_dir(amp, GlobalLayout::OWN_DIRS)
        .into_iter()
        .map(|a| a.key())
        .collect();
    assert_eq!(keys, vec!["amp"], "Kimi left Amp's dir (brief I4)");
}

#[test]
fn retired_rows_are_not_registered() {
    for key in RETIRED_TOOL_KEYS {
        assert!(adapter_by_key(key).is_none(), "{key} is retired");
    }
}

/// Brief I10: Clawdbot and MoltBot are OpenClaw's former names — their keys
/// are OpenClaw's former keys (exactly the retired keys) and their dirs its
/// former dirs; a registered or unknown key has no successor.
#[test]
fn the_retired_keys_are_openclaws_former_names() {
    let mut former: Vec<&str> = default_tool_adapters()
        .iter()
        .flat_map(|a| a.former_keys.iter().copied())
        .collect();
    former.sort_unstable();
    assert_eq!(former, RETIRED_TOOL_KEYS);
    for key in RETIRED_TOOL_KEYS {
        let successor = adapter_succeeding_key(key).expect("a successor");
        assert_eq!(successor.key(), "openclaw", "{key}");
        assert!(
            successor
                .former_relative_skills_dirs
                .contains(&format!(".{key}/skills").as_str()),
            "{key}'s dir is a former dir of OpenClaw"
        );
    }
    assert!(adapter_succeeding_key("openclaw").is_none());
    assert!(adapter_succeeding_key("no_such_tool").is_none());
}

/// A skill folder whose `SKILL.md` passes the gate.
fn valid_skill(dir: &std::path::Path, name: &str) {
    fs::create_dir_all(dir.join(name)).unwrap();
    fs::write(
        dir.join(name).join("SKILL.md"),
        format!("---\nname: {name}\n---\nbody\n"),
    )
    .unwrap();
}

fn plain_adapter(id: ToolId) -> ToolAdapter {
    ToolAdapter {
        id,
        display_name: "Test",
        group_label: None,
        relative_skills_dir: "ignored",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &["ignored"],
        project_relative_skills_dir: Some("ignored"),
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[],
    }
}

#[test]
fn scan_tool_dir_skips_codex_system_and_includes_symlink_dir() {
    let dir = tempfile::tempdir().unwrap();

    valid_skill(dir.path(), "a");
    valid_skill(dir.path(), ".system");
    fs::write(dir.path().join("not-a-dir"), b"x").unwrap();

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(dir.path().join("a"), dir.path().join("link-a")).unwrap();
    }

    let tool = ToolAdapter {
        id: ToolId::Codex,
        display_name: "Codex",
        group_label: None,
        relative_skills_dir: "ignored",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &["ignored"],
        project_relative_skills_dir: Some("ignored"),
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[],
    };

    let out = scan_tool_dir(&tool, dir.path()).unwrap();
    let names: Vec<String> = out.skills.iter().map(|s| s.name.clone()).collect();

    assert!(names.contains(&"a".to_string()));
    assert!(!names.contains(&".system".to_string()));
    assert!(out.invalid.is_empty(), "{:?}", out.invalid);

    #[cfg(unix)]
    {
        let link = out.skills.iter().find(|s| s.name == "link-a").unwrap();
        assert!(link.is_link);
        assert!(link.link_target.is_some());
    }
}

#[test]
fn scan_tool_dir_skips_app_support_path() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir
        .path()
        .join("Library/Application Support/com.tauri.dev/skills");
    valid_skill(&root, "foo");
    std::fs::create_dir_all(root.join("no-manifest")).unwrap();

    let tool = ToolAdapter {
        id: ToolId::Cursor,
        display_name: "Cursor",
        group_label: None,
        relative_skills_dir: "ignored",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &["ignored"],
        project_relative_skills_dir: Some("ignored"),
        group: None,
        supports_symlink: false,
        reads_agents_standard_globally: None,
        also_reads: &[],
    };

    let out = scan_tool_dir(&tool, &root).unwrap();
    assert!(out.skills.is_empty());
    assert!(
        out.invalid.is_empty(),
        "an ignored path is not reported either"
    );
}

/// #46: the scan admits a directory only when its `SKILL.md` parses to a
/// valid manifest; every other visible directory is reported with the typed
/// reason, and a hidden non-skill directory is not a candidate at all.
#[test]
fn scan_tool_dir_admits_only_valid_manifests_and_reports_the_rest() {
    use crate::core::skill_discovery::InvalidManifestReason;

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    valid_skill(root, "good");
    // Case-insensitive manifest lookup, as install admission.
    fs::create_dir_all(root.join("lower")).unwrap();
    fs::write(root.join("lower/skill.md"), "---\nname: lower\n---\n").unwrap();
    fs::create_dir_all(root.join("no-manifest")).unwrap();
    fs::write(root.join("no-manifest/notes.txt"), "x").unwrap();
    fs::create_dir_all(root.join("no-name")).unwrap();
    fs::write(
        root.join("no-name/SKILL.md"),
        "---\ndescription: nameless\n---\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("no-frontmatter")).unwrap();
    fs::write(root.join("no-frontmatter/SKILL.md"), "# just prose\n").unwrap();
    fs::create_dir_all(root.join(".git")).unwrap();

    let out = scan_tool_dir(&plain_adapter(ToolId::ClaudeCode), root).unwrap();

    let mut admitted: Vec<&str> = out.skills.iter().map(|s| s.name.as_str()).collect();
    admitted.sort_unstable();
    assert_eq!(admitted, vec!["good", "lower"]);

    let mut invalid: Vec<(&str, InvalidManifestReason)> = out
        .invalid
        .iter()
        .map(|i| (i.entry.name.as_str(), i.reason))
        .collect();
    invalid.sort_by_key(|(name, _)| *name);
    assert_eq!(
        invalid,
        vec![
            ("no-frontmatter", InvalidManifestReason::InvalidFrontmatter),
            ("no-manifest", InvalidManifestReason::MissingSkillMd),
            ("no-name", InvalidManifestReason::MissingName),
        ]
    );
    assert!(
        out.invalid.iter().all(|i| i.entry.path.starts_with(root)),
        "each invalid entry names its path"
    );
}

/// Every Tool's project-scope dir and project-scope label override, as a
/// table: the registry is the only source, so this pins the mapping against
/// accidental edits. Only the virtual-group entry may carry a `group_label`
/// (a per-scope fact: globally it absorbs nothing).
#[test]
fn project_relative_skills_dir_for_every_tool() {
    let expected: &[(&str, Option<&str>, Option<&str>)] = &[
        (
            "agents_skills",
            Some(".agents/skills"),
            Some(".agents/skills (9 harnesses)"),
        ),
        ("cursor", Some(".agents/skills"), None),
        ("claude_code", Some(".claude/skills"), None),
        ("codex", Some(".agents/skills"), None),
        ("opencode", Some(".agents/skills"), None),
        ("antigravity", Some(".agents/skills"), None),
        ("amp", Some(".agents/skills"), None),
        ("kimi_cli", Some(".agents/skills"), None),
        ("augment", Some(".augment/skills"), None),
        ("openclaw", Some("skills"), None),
        ("copaw", Some(".copaw/skill_pool"), None),
        ("cline", Some(".agents/skills"), None),
        ("codebuddy", Some(".codebuddy/skills"), None),
        ("command_code", Some(".commandcode/skills"), None),
        ("continue", Some(".continue/skills"), None),
        ("crush", Some(".crush/skills"), None),
        ("junie", Some(".junie/skills"), None),
        ("iflow_cli", Some(".iflow/skills"), None),
        ("kiro_cli", Some(".kiro/skills"), None),
        ("kode", Some(".kode/skills"), None),
        ("mcpjam", Some(".mcpjam/skills"), None),
        ("mistral_vibe", Some(".vibe/skills"), None),
        ("mux", Some(".mux/skills"), None),
        ("openclaude", Some(".openclaude/skills"), None),
        ("openhands", Some(".openhands/skills"), None),
        ("pi", Some(".pi/skills"), None),
        ("qoder", Some(".qoder/skills"), None),
        ("qoderwork", Some(".qoderwork/skills"), None),
        ("qwen_code", Some(".qwen/skills"), None),
        ("trae", Some(".trae/skills"), None),
        ("trae_cn", Some(".trae/skills"), None),
        ("zencoder", Some(".zencoder/skills"), None),
        ("neovate", Some(".neovate/skills"), None),
        ("pochi", Some(".pochi/skills"), None),
        ("adal", Some(".adal/skills"), None),
        ("kilo_code", Some(".kilocode/skills"), None),
        ("roo_code", Some(".roo/skills"), None),
        ("goose", Some(".goose/skills"), None),
        ("gemini_cli", Some(".agents/skills"), None),
        ("github_copilot", Some(".agents/skills"), None),
        ("droid", Some(".factory/skills"), None),
        ("windsurf", Some(".windsurf/skills"), None),
        ("hermes-agent", Some(".hermes/skills"), None),
        ("deepseek_harness", Some(".dsh/skills"), None),
        ("zcode", Some(".zcode/skills"), None),
        ("codewhale", Some(".codewhale/skills"), None),
        // Global-only: the project dir is unresolved (brief § 4).
        ("workbuddy", None, None),
    ];
    assert_eq!(
        expected.len(),
        default_tool_adapters().len(),
        "table must cover every registered tool"
    );
    for (key, dir, group_label) in expected {
        let adapter = adapter_by_key(key).unwrap_or_else(|| panic!("adapter {key}"));
        assert_eq!(
            adapter.project_relative_skills_dir, *dir,
            "project dir for {key}"
        );
        assert_eq!(adapter.group_label, *group_label, "group label for {key}");
        assert_eq!(
            adapter.project_display_name(),
            group_label.unwrap_or(adapter.display_name),
            "project label for {key}"
        );
    }
}

#[test]
fn agents_standard_group_has_nine_constituents_and_one_entry() {
    let members: Vec<&str> = constituents_of(VirtualGroup::AgentsStandard)
        .map(|a| a.key())
        .collect();
    assert_eq!(
        members,
        vec![
            "cursor",
            "codex",
            "opencode",
            "antigravity",
            "amp",
            "kimi_cli",
            "cline",
            "gemini_cli",
            "github_copilot",
        ]
    );
    for a in constituents_of(VirtualGroup::AgentsStandard) {
        assert_eq!(a.project_relative_skills_dir, Some(".agents/skills"));
        assert!(a.as_virtual_group().is_none());
    }
    let entries: Vec<&ToolAdapter> = default_tool_adapters()
        .iter()
        .filter(|a| a.as_virtual_group().is_some())
        .collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, ToolId::AgentsStandard);
    // The project-scope label advertises the roster it absorbs; the
    // scope-independent name does not.
    assert_eq!(entries[0].display_name, ".agents/skills");
    assert_eq!(
        entries[0].project_display_name(),
        format!(".agents/skills ({} harnesses)", members.len())
    );
    assert_eq!(
        entries[0].group, None,
        "a group entry is not its own member"
    );
}

/// Every registry entry can take a symlinked skills dir (Cursor was the last
/// copy-only entry; flipped in v-next ticket 38). Pinned so a regression to
/// copy mode is a deliberate registry edit, not drift.
#[test]
fn every_adapter_supports_symlink() {
    let no_symlink: Vec<&str> = default_tool_adapters()
        .iter()
        .filter(|a| !a.supports_symlink)
        .map(|a| a.key())
        .collect();
    assert!(no_symlink.is_empty(), "copy-only entries: {no_symlink:?}");
}

#[test]
fn registry_keys_are_unique() {
    let mut keys: Vec<&str> = default_tool_adapters().iter().map(|a| a.key()).collect();
    let n = keys.len();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), n);
}

/// Augment's global skills dir is `~/.augment/skills` (docs.augmentcode.com/
/// cli/skills, "Skill Locations"; add-skill agrees). `~/.augment/rules` is its
/// *rules* dir; Skills Hub synced there until 1.2.18, so it is recorded as
/// the former dir the startup relocation moves targets out of (#43).
#[test]
fn augment_global_skills_dir_is_augment_skills_and_rules_is_former() {
    let home = tempfile::tempdir().unwrap();
    let augment = adapter_by_key("augment").expect("augment adapter");
    assert_eq!(
        skills_dir_in(home.path(), augment),
        home.path().join(".augment/skills")
    );
    assert_eq!(
        former_skills_dirs_in(home.path(), augment),
        vec![home.path().join(".augment/rules")]
    );
    assert_eq!(
        detect_dirs_in(home.path(), augment),
        vec![home.path().join(".augment")]
    );
    assert_eq!(augment.project_relative_skills_dir, Some(".augment/skills"));
}

/// A former dir is a registry fact about a *correction*: it is never the
/// Tool's own current dir, and it is another Tool's live dir only where the
/// brief says so — Kimi's former `~/.config/agents/skills` is Amp's (I4).
/// Relocation handles that case by moving only Kimi's rows and keeping any
/// artifact another row still records (`core::target_relocation`); this
/// list pins every such overlap so a new one is a deliberate edit.
#[test]
fn a_former_skills_dir_is_a_live_dir_only_where_pinned() {
    let mut overlaps: Vec<(&str, &str, &str)> = Vec::new();
    for adapter in default_tool_adapters() {
        for former in adapter.former_relative_skills_dirs {
            assert_ne!(adapter.relative_skills_dir, *former, "{}", adapter.key());
            for live in default_tool_adapters()
                .iter()
                .filter(|a| a.relative_skills_dir == *former)
            {
                overlaps.push((adapter.key(), former, live.key()));
            }
        }
    }
    assert_eq!(
        overlaps,
        vec![("kimi_cli", ".config/agents/skills", "amp")],
        "former dirs that are another Tool's live dir"
    );
}

// ---------------------------------------------------------------------------
// The deletion safety rule (`ensure_path_within_tool_dirs`)
// ---------------------------------------------------------------------------

#[test]
fn a_path_inside_a_former_tool_skills_dir_is_allowed() {
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join(".augment/rules/some-skill");
    ensure_path_within_tool_dirs(home.path(), &path)
        .unwrap_or_else(|err| panic!("a former dir artifact is ours to remove: {err:#}"));
}

#[test]
fn a_path_inside_any_tool_skills_dir_is_allowed() {
    let home = tempfile::tempdir().unwrap();
    for key in ["claude_code", "pi", "amp"] {
        let adapter = adapter_by_key(key).unwrap();
        let path = skills_dir_in(home.path(), adapter).join("some-skill");
        ensure_path_within_tool_dirs(home.path(), &path)
            .unwrap_or_else(|err| panic!("{key} should be allowed: {err:#}"));
    }
}

#[test]
fn a_path_outside_every_tool_skills_dir_is_refused_with_the_typed_condition() {
    let home = tempfile::tempdir().unwrap();
    for outside in [
        home.path().join("Documents/notes"),
        home.path().join(".claude"), // the tool root, not its skills dir
        std::path::PathBuf::from("/"),
    ] {
        let err = ensure_path_within_tool_dirs(home.path(), &outside)
            .expect_err("must refuse a path outside every tool skills dir");
        match err.downcast_ref::<SignalError>() {
            Some(SignalError::PathOutsideToolDirs { path }) => {
                assert_eq!(path, &outside.to_string_lossy().to_string());
            }
            other => panic!("expected PathOutsideToolDirs, got {other:?}"),
        }
    }
}

// ---------------------------------------------------------------------------
// The inverse: which Tool holds a path (`tool_holding_path`)
// ---------------------------------------------------------------------------

#[test]
fn a_path_inside_a_tool_skills_dir_names_that_tool() {
    let home = tempfile::tempdir().unwrap();
    for key in ["claude_code", "pi", "cursor"] {
        let adapter = adapter_by_key(key).unwrap();
        let path = skills_dir_in(home.path(), adapter).join("some-skill");
        let holder = tool_holding_path(home.path(), &path)
            .unwrap_or_else(|| panic!("{key} should hold {path:?}"));
        assert_eq!(holder.key(), key);
    }
}

#[test]
fn a_path_outside_every_tool_skills_dir_has_no_holder() {
    let home = tempfile::tempdir().unwrap();
    for outside in [
        home.path().join("Documents/my-skill"),
        home.path().join(".claude"), // the tool root, not its skills dir
        std::path::PathBuf::from("/"),
    ] {
        assert_eq!(
            tool_holding_path(home.path(), &outside).map(|a| a.key()),
            None,
            "{outside:?} is outside every tool skills dir"
        );
    }
}

/// A path that only *resolves* into a Tool's skills dir (an alias of the
/// directory, or a link from elsewhere into it) is held by that Tool too:
/// the bytes live in the Tool's directory whichever spelling reaches them.
#[cfg(unix)]
#[test]
fn a_path_resolving_into_a_tool_skills_dir_names_that_tool() {
    let home = tempfile::tempdir().unwrap();
    let claude = adapter_by_key("claude_code").unwrap();
    let real = skills_dir_in(home.path(), claude).join("some-skill");
    fs::create_dir_all(&real).unwrap();
    let elsewhere = home.path().join("Documents");
    fs::create_dir_all(&elsewhere).unwrap();
    let link = elsewhere.join("alias");
    std::os::unix::fs::symlink(&real, &link).unwrap();

    assert_eq!(
        tool_holding_path(home.path(), &link).map(|a| a.key()),
        Some("claude_code")
    );
}

// ---------------------------------------------------------------------------
// Registry truth (ticket 34): the global facts, pinned to the brief
// `docs/design-inputs/2026-11-02-harness-global-skills-dirs.md`.
// ---------------------------------------------------------------------------

/// One row of the brief-derived snapshot. `brief` is the citation — § (a
/// section heading), `#N` (a §1 table row), `IN` (an implication) and an
/// evidence tag in parentheses (`LIKELY` / `UNKNOWN`) — and every part of it
/// must exist in the brief itself ([`every_snapshot_citation_exists_in_the_brief`]),
/// so a row names where to check it and the brief cannot lose that anchor
/// silently.
struct BriefRow {
    key: &'static str,
    display: &'static str,
    global: &'static str,
    former: &'static [&'static str],
    detect: &'static [&'static str],
    reads: Option<(Option<&'static str>, Precedence)>,
    also_reads: &'static [&'static str],
    brief: &'static str,
}

#[allow(clippy::too_many_arguments)]
const fn row(
    key: &'static str,
    display: &'static str,
    global: &'static str,
    former: &'static [&'static str],
    detect: &'static [&'static str],
    reads: Option<(Option<&'static str>, Precedence)>,
    also_reads: &'static [&'static str],
    brief: &'static str,
) -> BriefRow {
    BriefRow {
        key,
        display,
        global,
        former,
        detect,
        reads,
        also_reads,
        brief,
    }
}

use Precedence::{AgentsWins, BothShown, OwnWins, Unknown};

/// The checked-in snapshot: every registry entry's global facts as the brief
/// states them, transcribed and reviewed by hand (not generated from the
/// registry, so not circular). A registry edit without an edit of this table
/// fails; the brief itself is bound only through the citations (see
/// [`every_snapshot_citation_exists_in_the_brief`]), not by a digest.
#[rustfmt::skip]
const BRIEF_SNAPSHOT: &[BriefRow] = &[
    row("agents_skills", ".agents/skills", ".agents/skills", &[], &[".agents"], None, &[], "§1 #1"),
    row("cursor", "Cursor", ".cursor/skills", &[], &[".cursor"], Some((None, Unknown)), &[".claude/skills", ".codex/skills"], "§1 #2, I1"),
    row("claude_code", "Claude Code", ".claude/skills", &[], &[".claude"], None, &[], "§1 #3, I12"),
    row("codex", "Codex", ".agents/skills", &[".codex/skills"], &[".codex"], Some((Some("rust-v0.95.0"), BothShown)), &[], "§3, I2 (a)"),
    row("opencode", "OpenCode", ".config/opencode/skills", &[], &[".config/opencode"], Some((None, Unknown)), &[".claude/skills"], "§1 #5, I1"),
    row("antigravity", "Antigravity", ".gemini/config/skills", &[".gemini/antigravity/global_skills", ".gemini/antigravity/skills"], &[".gemini/antigravity"], None, &[".gemini/skills"], "§2, I3, I12"),
    row("amp", "Amp", ".config/agents/skills", &[], &[".config/amp"], Some((None, OwnWins)), &[".claude/skills"], "§1 #7, I1"),
    row("kimi_cli", "Kimi Code CLI", ".kimi-code/skills", &[".config/agents/skills"], &[".kimi-code", ".kimi"], Some((None, Unknown)), &[".claude/skills", ".codex/skills", ".config/agents/skills"], "§2, I4"),
    row("augment", "Augment", ".augment/skills", &[".augment/rules"], &[".augment"], Some((None, OwnWins)), &[".claude/skills"], "§1 #9, I1"),
    row("openclaw", "OpenClaw", ".openclaw/skills", &[".clawdbot/skills", ".moltbot/skills"], &[".openclaw"], Some((None, AgentsWins)), &[], "§1 #10, #41, #44, I10"),
    row("copaw", "Copaw", ".copaw/skill_pool", &[], &[".copaw"], None, &[], "§1 #11 (LIKELY)"),
    row("cline", "Cline", ".cline/skills", &[], &[".cline"], Some((None, Unknown)), &[], "§2, I5"),
    row("codebuddy", "CodeBuddy", ".codebuddy/skills", &[], &[".codebuddy"], None, &[], "§1 #13, I12"),
    row("command_code", "Command Code", ".commandcode/skills", &[], &[".commandcode"], Some((None, OwnWins)), &[], "§1 #14"),
    row("continue", "Continue", ".continue/skills", &[], &[".continue"], None, &[".claude/skills"], "§1 #15 (LIKELY), I1"),
    row("crush", "Crush", ".config/crush/skills", &[], &[".config/crush"], Some((None, Unknown)), &[".config/agents/skills", ".claude/skills"], "§1 #16, I1"),
    row("junie", "Junie", ".junie/skills", &[], &[".junie"], Some((None, Unknown)), &[], "§1 #17"),
    row("iflow_cli", "iFlow CLI", ".iflow/skills", &[], &[".iflow"], None, &[], "§1 #18, I12"),
    row("kiro_cli", "Kiro CLI", ".kiro/skills", &[], &[".kiro"], None, &[], "§1 #19, I12"),
    row("kode", "Kode", ".kode/skills", &[], &[".kode"], None, &[], "§1 #20 (LIKELY)"),
    row("mcpjam", "MCPJam", ".mcpjam/skills", &[], &[".mcpjam"], None, &[], "§1 #21 (UNKNOWN)"),
    row("mistral_vibe", "Mistral Vibe", ".vibe/skills", &[], &[".vibe"], Some((None, Unknown)), &[], "§1 #22"),
    row("mux", "Xum", ".xum/skills", &[".mux/skills"], &[".xum", ".mux"], Some((None, OwnWins)), &[], "§5, I7"),
    row("openclaude", "OpenClaude IDE", ".openclaude/skills", &[], &[".openclaude"], None, &[], "§1 #24 (UNKNOWN)"),
    row("openhands", "OpenHands", ".agents/skills", &[".openhands/skills"], &[".openhands"], Some((None, AgentsWins)), &[], "§1 #25, I8"),
    row("pi", "Pi", ".pi/agent/skills", &[], &[".pi"], Some((None, Unknown)), &[], "§1 #26"),
    row("qoder", "Qoder", ".qoder/skills", &[], &[".qoder"], None, &[], "§1 #27, I12"),
    row("qoderwork", "QoderWork", ".qoderwork/skills", &[], &[".qoderwork"], None, &[], "§1 #28 (LIKELY)"),
    row("qwen_code", "Qwen Code", ".qwen/skills", &[], &[".qwen"], Some((None, OwnWins)), &[], "§1 #29"),
    row("trae", "Trae", ".trae/skills", &[], &[".trae"], None, &[], "§1 #30 (LIKELY)"),
    row("trae_cn", "Trae CN", ".trae-cn/skills", &[], &[".trae-cn"], None, &[], "§1 #31 (LIKELY)"),
    row("zencoder", "Zencoder", ".zencoder/skills", &[], &[".zencoder"], None, &[], "§1 #32 (LIKELY)"),
    row("neovate", "Neovate", ".neovate/skills", &[], &[".neovate"], None, &[], "§1 #33 (UNKNOWN read)"),
    row("pochi", "Pochi", ".pochi/skills", &[], &[".pochi"], Some((Some("2a9ff73 (#1360)"), OwnWins)), &[], "§1 #34"),
    row("adal", "AdaL", ".adal/skills", &[], &[".adal"], None, &[], "§1 #35 (UNKNOWN)"),
    row("kilo_code", "Kilo Code", ".kilo/skills", &[".kilocode/skills"], &[".kilo", ".kilocode"], Some((None, Unknown)), &[], "§5, I6"),
    row("roo_code", "Roo Code", ".roo/skills", &[], &[".roo"], Some((Some("PR #11181"), OwnWins)), &[], "§1 #37"),
    row("goose", "Goose", ".agents/skills", &[".config/goose/skills"], &[".config/goose"], Some((None, Unknown)), &[".claude/skills"], "§1 #38, I8, I1"),
    row("gemini_cli", "Gemini CLI", ".gemini/skills", &[], &[".gemini"], Some((Some("PR #18151"), AgentsWins)), &[], "§1 #39"),
    row("github_copilot", "GitHub Copilot", ".copilot/skills", &[], &[".copilot"], Some((None, Unknown)), &[], "§1 #40"),
    row("droid", "Droid", ".factory/skills", &[], &[".factory"], Some((None, Unknown)), &[], "§1 #42"),
    row("windsurf", "Windsurf", ".codeium/windsurf/skills", &[], &[".codeium/windsurf"], Some((None, Unknown)), &[], "§1 #43, I9 (held)"),
    row("hermes-agent", "Hermes Agent", ".hermes/skills", &[], &[".hermes"], None, &[], "§1 #45, I12"),
    row("deepseek_harness", "DeepSeek Harness", ".dsh/skills", &[], &[".dsh"], Some((None, OwnWins)), &[], "§4, I13"),
    row("zcode", "ZCode", ".zcode/skills", &[], &[".zcode"], Some((None, OwnWins)), &[], "§4, I13"),
    row("codewhale", "Codewhale", ".codewhale/skills", &[], &[".codewhale"], Some((None, OwnWins)), &[".claude/skills"], "§4, I13, I1"),
    row("workbuddy", "WorkBuddy", ".workbuddy/skills", &[], &[".workbuddy"], None, &[], "§4, I13 (LIKELY)"),
];

/// Every `BRIEF_SNAPSHOT` citation resolves in the brief: each `§N` is a
/// section heading, each `#N` a §1 table row, each `IN` an implication, and
/// an evidence tag (`LIKELY`, `UNKNOWN`) appears on the cited row — the
/// `#N` row, or the row naming the key (§4's new rows).
#[test]
fn every_snapshot_citation_exists_in_the_brief() {
    let brief_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/design-inputs/2026-11-02-harness-global-skills-dirs.md");
    let brief = std::fs::read_to_string(&brief_path).expect("read the brief");
    let lines: Vec<&str> = brief.lines().collect();
    for row in BRIEF_SNAPSHOT {
        let (anchors, tag) = match row.brief.split_once('(') {
            Some((anchors, tag)) => (anchors, Some(tag.trim_end_matches(')'))),
            None => (row.brief, None),
        };
        let mut cited_rows: Vec<&str> = Vec::new();
        for anchor in anchors.split(',').map(str::trim).filter(|a| !a.is_empty()) {
            for part in anchor.split_whitespace() {
                if let Some(section) = part.strip_prefix('§') {
                    let heading = format!("## §{section} ");
                    assert!(
                        lines.iter().any(|l| l.starts_with(&heading)),
                        "{}: no section {part} in the brief",
                        row.key
                    );
                } else if let Some(number) = part.strip_prefix('#') {
                    let prefix = format!("| {number} |");
                    let line = lines
                        .iter()
                        .find(|l| l.starts_with(&prefix))
                        .unwrap_or_else(|| panic!("{}: no §1 row {part} in the brief", row.key));
                    cited_rows.push(line);
                } else if part.starts_with('I') {
                    let implication = format!("- **{part} —");
                    assert!(
                        lines.iter().any(|l| l.starts_with(&implication)),
                        "{}: no implication {part} in the brief",
                        row.key
                    );
                } else {
                    panic!("{}: unreadable citation part {part:?}", row.key);
                }
            }
        }
        if let Some(tag) = tag {
            let word = tag.split_whitespace().next().unwrap_or(tag);
            if word != "LIKELY" && word != "UNKNOWN" {
                continue; // a note, e.g. "(held)"
            }
            if cited_rows.is_empty() {
                let key_cell = format!("| `{}` |", row.key);
                cited_rows.extend(lines.iter().filter(|l| l.starts_with(&key_cell)));
            }
            assert!(
                !cited_rows.is_empty() && cited_rows.iter().all(|l| l.contains(word)),
                "{}: the brief's cited row does not say {word}",
                row.key
            );
        }
    }
}

#[test]
fn registry_matches_the_brief_snapshot() {
    let registry: Vec<&str> = default_tool_adapters().iter().map(|a| a.key()).collect();
    let snapshot: Vec<&str> = BRIEF_SNAPSHOT.iter().map(|r| r.key).collect();
    assert_eq!(registry, snapshot, "registry order and membership");
    for (adapter, expected) in default_tool_adapters().iter().zip(BRIEF_SNAPSHOT) {
        let key = expected.key;
        assert_eq!(
            adapter.display_name, expected.display,
            "display name of {key}"
        );
        assert_eq!(
            adapter.relative_skills_dir, expected.global,
            "global dir of {key}"
        );
        assert_eq!(
            adapter.former_relative_skills_dirs, expected.former,
            "former dirs of {key}"
        );
        assert_eq!(
            adapter.relative_detect_dirs, expected.detect,
            "detect dirs of {key}"
        );
        assert_eq!(
            adapter.reads_agents_standard_globally,
            expected
                .reads
                .map(|(since, precedence)| AgentsStandardRead { since, precedence }),
            "AGENTS-standard read of {key}"
        );
        assert_eq!(
            adapter.also_reads, expected.also_reads,
            "cross-reads of {key}"
        );
    }
}

/// Brief § 1 summary 5 / I11: 24 of the 44 harness rows VERIFIED-read
/// `~/.agents/skills`, and three of the four new rows read it too (DeepSeek
/// Harness, ZCode, Codewhale).
#[test]
fn the_agents_standard_readers_are_the_briefs_verified_rows() {
    let readers: Vec<&str> = default_tool_adapters()
        .iter()
        .filter(|a| a.reads_agents_standard_globally.is_some())
        .map(|a| a.key())
        .collect();
    assert_eq!(readers.len(), 27, "{readers:?}");
    // Since versions are VERIFIED only for these four (I11).
    let with_since: Vec<&str> = default_tool_adapters()
        .iter()
        .filter(|a| {
            a.reads_agents_standard_globally
                .is_some_and(|r| r.since.is_some())
        })
        .map(|a| a.key())
        .collect();
    assert_eq!(with_since, vec!["codex", "pochi", "roo_code", "gemini_cli"]);
    // I12: never group.
    for key in [
        "claude_code",
        "antigravity",
        "kiro_cli",
        "qoder",
        "iflow_cli",
        "codebuddy",
        "hermes-agent",
    ] {
        assert!(
            adapter_by_key(key)
                .unwrap()
                .reads_agents_standard_globally
                .is_none(),
            "{key}"
        );
    }
}

#[test]
fn the_agents_standard_dir_is_the_entry_dir() {
    let entry = adapter_by_key("agents_skills").unwrap();
    assert_eq!(entry.relative_skills_dir, AGENTS_STANDARD_DIR);
    assert!(entry.reads_agents_standard_globally.is_none());
}

/// A cross-read names another harness's dir (own or former) — never the
/// harness's own, and never the AGENTS-standard dir (that read is
/// `reads_agents_standard_globally`).
#[test]
fn every_cross_read_is_another_harness_dir() {
    for adapter in default_tool_adapters() {
        for dir in adapter.also_reads {
            assert_ne!(*dir, AGENTS_STANDARD_DIR, "{}", adapter.key());
            assert_ne!(*dir, adapter.relative_skills_dir, "{}", adapter.key());
            assert!(
                default_tool_adapters()
                    .iter()
                    .any(|other| other.id != adapter.id
                        && (other.relative_skills_dir == *dir
                            || other.former_relative_skills_dirs.contains(dir))),
                "{}: {dir} is no other harness's dir",
                adapter.key()
            );
        }
    }
}

/// I3 / I13: neither Antigravity nor Codewhale is detected on a dir that is
/// a skills-only footprint or another product's fallback.
#[test]
fn detect_dirs_avoid_the_briefs_footprint_hazards() {
    let antigravity = adapter_by_key("antigravity").unwrap();
    assert!(!antigravity.relative_detect_dirs.contains(&".gemini/config"));
    let codewhale = adapter_by_key("codewhale").unwrap();
    assert!(!codewhale.relative_detect_dirs.contains(&".deepseek"));
}

/// `~/.codex` holding only `skills/` is what `gh skill install --agent codex`
/// leaves (brief § 3): the former dir is a skills path, so it is a footprint.
#[test]
fn a_former_dir_alone_is_a_skills_only_footprint() {
    let home = tempfile::tempdir().unwrap();
    let codex = adapter_by_key("codex").unwrap();
    fs::create_dir_all(home.path().join(".codex/skills/x")).unwrap();
    assert!(!is_installed_in(home.path(), codex));
    fs::write(home.path().join(".codex/config.toml"), b"").unwrap();
    assert!(is_installed_in(home.path(), codex));

    let kilo = adapter_by_key("kilo_code").unwrap();
    fs::create_dir_all(home.path().join(".kilocode/skills/x")).unwrap();
    assert!(!is_installed_in(home.path(), kilo));
    fs::create_dir_all(home.path().join(".kilo/skills/x")).unwrap();
    assert!(
        !is_installed_in(home.path(), kilo),
        "both roots hold only skills"
    );
    fs::write(home.path().join(".kilo/settings.json"), b"{}").unwrap();
    assert!(is_installed_in(home.path(), kilo));
}

// ---------------------------------------------------------------------------
// The #83 preference as a layout: where each harness deploys
// ---------------------------------------------------------------------------

/// The effective global dir across the precedence values: with the
/// preference on, every VERIFIED reader resolves to the AGENTS-standard dir
/// whatever its precedence; a `None` row keeps its own dir. Off, everyone
/// keeps their own dir.
#[test]
fn the_layout_moves_every_verified_reader_and_only_them() {
    let home = tempfile::tempdir().unwrap();
    let cases = [
        ("amp", Some(OwnWins), ".config/agents/skills"),
        ("gemini_cli", Some(AgentsWins), ".gemini/skills"),
        ("codex", Some(BothShown), ".agents/skills"),
        ("cursor", Some(Unknown), ".cursor/skills"),
        ("claude_code", None, ".claude/skills"),
        ("antigravity", None, ".gemini/config/skills"),
    ];
    for (key, precedence, own) in cases {
        let adapter = adapter_by_key(key).unwrap();
        assert_eq!(
            adapter.reads_agents_standard_globally.map(|r| r.precedence),
            precedence,
            "{key}"
        );
        assert_eq!(
            global_skills_dir_in(home.path(), adapter, GlobalLayout::OWN_DIRS),
            home.path().join(own),
            "{key} with the preference off"
        );
        let preferred = if precedence.is_some() {
            AGENTS_STANDARD_DIR
        } else {
            own
        };
        assert_eq!(
            global_skills_dir_in(home.path(), adapter, GlobalLayout::PREFER_AGENTS_STANDARD),
            home.path().join(preferred),
            "{key} with the preference on"
        );
    }
}

#[test]
fn superseded_dirs_follow_the_layout() {
    let home = tempfile::tempdir().unwrap();
    let h = |dir: &str| home.path().join(dir);
    let cursor = adapter_by_key("cursor").unwrap();
    assert_eq!(
        superseded_skills_dirs_in(home.path(), cursor, GlobalLayout::OWN_DIRS),
        vec![h(".agents/skills")],
        "preference off: a reader's targets come back from the AGENTS-standard dir"
    );
    assert_eq!(
        superseded_skills_dirs_in(home.path(), cursor, GlobalLayout::PREFER_AGENTS_STANDARD),
        vec![h(".cursor/skills")]
    );
    let kimi = adapter_by_key("kimi_cli").unwrap();
    assert_eq!(
        superseded_skills_dirs_in(home.path(), kimi, GlobalLayout::OWN_DIRS),
        vec![h(".config/agents/skills"), h(".agents/skills")]
    );
    assert_eq!(
        superseded_skills_dirs_in(home.path(), kimi, GlobalLayout::PREFER_AGENTS_STANDARD),
        vec![h(".config/agents/skills"), h(".kimi-code/skills")]
    );
    // Codex deploys to the AGENTS-standard dir either way.
    let codex = adapter_by_key("codex").unwrap();
    for layout in [GlobalLayout::OWN_DIRS, GlobalLayout::PREFER_AGENTS_STANDARD] {
        assert_eq!(
            superseded_skills_dirs_in(home.path(), codex, layout),
            vec![h(".codex/skills")]
        );
    }
    // A non-reader has only its former dirs.
    let augment_like = adapter_by_key("antigravity").unwrap();
    assert_eq!(
        superseded_skills_dirs_in(
            home.path(),
            augment_like,
            GlobalLayout::PREFER_AGENTS_STANDARD
        ),
        former_skills_dirs_in(home.path(), augment_like)
    );
}
