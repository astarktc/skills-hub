use std::fs;

use crate::core::tool_adapters::{
    adapter_by_key, adapters_sharing_skills_dir, constituents_of, default_tool_adapters,
    detect_dir_in, is_installed_in, scan_tool_dir, skills_dir_in, ToolAdapter, ToolId,
    VirtualGroup,
};

#[test]
fn path_resolution_joins_adapter_dirs_onto_home() {
    let home = tempfile::tempdir().unwrap();
    let cases = [
        ("claude_code", ".claude/skills", ".claude"),
        ("codex", ".codex/skills", ".codex"),
        ("cursor", ".cursor/skills", ".cursor"),
        ("amp", ".config/agents/skills", ".config/agents"),
        ("kimi_cli", ".config/agents/skills", ".config/agents"),
        ("pi", ".pi/agent/skills", ".pi"),
    ];
    for (key, skills, detect) in cases {
        let adapter = adapter_by_key(key).unwrap_or_else(|| panic!("adapter {key}"));
        assert_eq!(
            skills_dir_in(home.path(), &adapter),
            home.path().join(skills),
            "skills dir for {key}"
        );
        assert_eq!(
            detect_dir_in(home.path(), &adapter),
            home.path().join(detect),
            "detect dir for {key}"
        );
    }
}

#[test]
fn every_adapter_resolves_under_home() {
    let home = tempfile::tempdir().unwrap();
    for adapter in default_tool_adapters() {
        assert!(skills_dir_in(home.path(), adapter).starts_with(home.path()));
        assert!(detect_dir_in(home.path(), adapter).starts_with(home.path()));
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

    assert!(!is_installed_in(home.path(), &codex));

    // Detect dir present (even without a skills dir) => installed.
    fs::create_dir_all(home.path().join(".codex")).unwrap();
    assert!(is_installed_in(home.path(), &codex));
    assert!(!is_installed_in(home.path(), &claude));

    // Only the adapter's own detect dir counts — a sibling under the same
    // parent does not.
    let amp = adapter_by_key("amp").unwrap();
    fs::create_dir_all(home.path().join(".config/other")).unwrap();
    assert!(!is_installed_in(home.path(), &amp));
    fs::create_dir_all(home.path().join(".config/agents")).unwrap();
    assert!(is_installed_in(home.path(), &amp));
    // Shared-dir tools are detected independently by the same dir.
    let kimi = adapter_by_key("kimi_cli").unwrap();
    assert!(is_installed_in(home.path(), &kimi));
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
    assert!(adapter_by_key("openclaw").is_some());
    assert!(adapter_by_key("command_code").is_some());
    assert!(adapter_by_key("qwen_code").is_some());
}

#[test]
fn adapters_sharing_skills_dir_groups_amp_and_kimi() {
    let amp = adapter_by_key("amp").unwrap();
    let group = adapters_sharing_skills_dir(&amp);
    let keys: std::collections::HashSet<&'static str> =
        group.into_iter().map(|a| a.id.as_key()).collect();
    assert!(keys.contains("amp"));
    assert!(keys.contains("kimi_cli"));
}

#[test]
fn scan_tool_dir_skips_codex_system_and_includes_symlink_dir() {
    let dir = tempfile::tempdir().unwrap();

    fs::create_dir_all(dir.path().join("a")).unwrap();
    fs::create_dir_all(dir.path().join(".system")).unwrap();
    fs::write(dir.path().join("not-a-dir"), b"x").unwrap();

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(dir.path().join("a"), dir.path().join("link-a")).unwrap();
    }

    let tool = ToolAdapter {
        id: ToolId::Codex,
        display_name: "Codex",
        relative_skills_dir: "ignored",
        relative_detect_dir: "ignored",
        project_relative_skills_dir: "ignored",
        group: None,
        supports_symlink: true,
    };

    let out = scan_tool_dir(&tool, dir.path()).unwrap();
    let names: Vec<String> = out.iter().map(|s| s.name.clone()).collect();

    assert!(names.contains(&"a".to_string()));
    assert!(!names.contains(&".system".to_string()));

    #[cfg(unix)]
    {
        let link = out.iter().find(|s| s.name == "link-a").unwrap();
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
    std::fs::create_dir_all(root.join("foo")).unwrap();

    let tool = ToolAdapter {
        id: ToolId::Cursor,
        display_name: "Cursor",
        relative_skills_dir: "ignored",
        relative_detect_dir: "ignored",
        project_relative_skills_dir: "ignored",
        group: None,
        supports_symlink: false,
    };

    let out = scan_tool_dir(&tool, &root).unwrap();
    assert!(out.is_empty());
}

/// Every Tool's project-scope dir, as a table: the registry is the only
/// source, so this pins the mapping against accidental edits.
#[test]
fn project_relative_skills_dir_for_every_tool() {
    let expected: &[(&str, &str)] = &[
        ("agents_skills", ".agents/skills"),
        ("cursor", ".agents/skills"),
        ("claude_code", ".claude/skills"),
        ("codex", ".agents/skills"),
        ("opencode", ".agents/skills"),
        ("antigravity", ".agents/skills"),
        ("amp", ".agents/skills"),
        ("kimi_cli", ".agents/skills"),
        ("augment", ".augment/skills"),
        ("openclaw", "skills"),
        ("copaw", ".copaw/skill_pool"),
        ("cline", ".agents/skills"),
        ("codebuddy", ".codebuddy/skills"),
        ("command_code", ".commandcode/skills"),
        ("continue", ".continue/skills"),
        ("crush", ".crush/skills"),
        ("junie", ".junie/skills"),
        ("iflow_cli", ".iflow/skills"),
        ("kiro_cli", ".kiro/skills"),
        ("kode", ".kode/skills"),
        ("mcpjam", ".mcpjam/skills"),
        ("mistral_vibe", ".vibe/skills"),
        ("mux", ".mux/skills"),
        ("openclaude", ".openclaude/skills"),
        ("openhands", ".openhands/skills"),
        ("pi", ".pi/skills"),
        ("qoder", ".qoder/skills"),
        ("qoderwork", ".qoderwork/skills"),
        ("qwen_code", ".qwen/skills"),
        ("trae", ".trae/skills"),
        ("trae_cn", ".trae/skills"),
        ("zencoder", ".zencoder/skills"),
        ("neovate", ".neovate/skills"),
        ("pochi", ".pochi/skills"),
        ("adal", ".adal/skills"),
        ("kilo_code", ".kilocode/skills"),
        ("roo_code", ".roo/skills"),
        ("goose", ".goose/skills"),
        ("gemini_cli", ".agents/skills"),
        ("github_copilot", ".agents/skills"),
        ("clawdbot", ".clawdbot/skills"),
        ("droid", ".factory/skills"),
        ("windsurf", ".windsurf/skills"),
        ("moltbot", ".moltbot/skills"),
        ("hermes-agent", ".hermes/skills"),
    ];
    assert_eq!(
        expected.len(),
        default_tool_adapters().len(),
        "table must cover every registered tool"
    );
    for (key, dir) in expected {
        let adapter = adapter_by_key(key).unwrap_or_else(|| panic!("adapter {key}"));
        assert_eq!(
            adapter.project_relative_skills_dir, *dir,
            "project dir for {key}"
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
        assert_eq!(a.project_relative_skills_dir, ".agents/skills");
        assert!(!a.is_virtual_group());
    }
    let entries: Vec<&ToolAdapter> = default_tool_adapters()
        .iter()
        .filter(|a| a.is_virtual_group())
        .collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, ToolId::AgentsStandard);
    assert_eq!(
        entries[0].group, None,
        "a group entry is not its own member"
    );
}

#[test]
fn only_cursor_lacks_symlink_support() {
    let no_symlink: Vec<&str> = default_tool_adapters()
        .iter()
        .filter(|a| !a.supports_symlink)
        .map(|a| a.key())
        .collect();
    assert_eq!(no_symlink, vec!["cursor"]);
}

#[test]
fn registry_keys_are_unique() {
    let mut keys: Vec<&str> = default_tool_adapters().iter().map(|a| a.key()).collect();
    let n = keys.len();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), n);
}
