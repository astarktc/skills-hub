use std::fs;

use crate::core::errors::SignalError;
use crate::core::tool_adapters::{
    adapter_by_key, adapters_sharing_skills_dir, constituents_of, default_tool_adapters,
    detect_dir_in, ensure_path_within_tool_dirs, is_installed_in, scan_tool_dir, skills_dir_in,
    tool_holding_path, ToolAdapter, ToolId, VirtualGroup,
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
            skills_dir_in(home.path(), adapter),
            home.path().join(skills),
            "skills dir for {key}"
        );
        assert_eq!(
            detect_dir_in(home.path(), adapter),
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
    fs::create_dir_all(home.path().join(".config/agents")).unwrap();
    assert!(is_installed_in(home.path(), amp));
    // Shared-dir tools are detected independently by the same dir.
    let kimi = adapter_by_key("kimi_cli").unwrap();
    assert!(is_installed_in(home.path(), kimi));
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
    let group = adapters_sharing_skills_dir(amp);
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
        group_label: None,
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
        group_label: None,
        relative_skills_dir: "ignored",
        relative_detect_dir: "ignored",
        project_relative_skills_dir: "ignored",
        group: None,
        supports_symlink: false,
    };

    let out = scan_tool_dir(&tool, &root).unwrap();
    assert!(out.is_empty());
}

/// Every Tool's project-scope dir and project-scope label override, as a
/// table: the registry is the only source, so this pins the mapping against
/// accidental edits. Only the virtual-group entry may carry a `group_label`
/// (a per-scope fact: globally it absorbs nothing).
#[test]
fn project_relative_skills_dir_for_every_tool() {
    let expected: &[(&str, &str, Option<&str>)] = &[
        (
            "agents_skills",
            ".agents/skills",
            Some(".agents/skills (9 tools)"),
        ),
        ("cursor", ".agents/skills", None),
        ("claude_code", ".claude/skills", None),
        ("codex", ".agents/skills", None),
        ("opencode", ".agents/skills", None),
        ("antigravity", ".agents/skills", None),
        ("amp", ".agents/skills", None),
        ("kimi_cli", ".agents/skills", None),
        ("augment", ".augment/skills", None),
        ("openclaw", "skills", None),
        ("copaw", ".copaw/skill_pool", None),
        ("cline", ".agents/skills", None),
        ("codebuddy", ".codebuddy/skills", None),
        ("command_code", ".commandcode/skills", None),
        ("continue", ".continue/skills", None),
        ("crush", ".crush/skills", None),
        ("junie", ".junie/skills", None),
        ("iflow_cli", ".iflow/skills", None),
        ("kiro_cli", ".kiro/skills", None),
        ("kode", ".kode/skills", None),
        ("mcpjam", ".mcpjam/skills", None),
        ("mistral_vibe", ".vibe/skills", None),
        ("mux", ".mux/skills", None),
        ("openclaude", ".openclaude/skills", None),
        ("openhands", ".openhands/skills", None),
        ("pi", ".pi/skills", None),
        ("qoder", ".qoder/skills", None),
        ("qoderwork", ".qoderwork/skills", None),
        ("qwen_code", ".qwen/skills", None),
        ("trae", ".trae/skills", None),
        ("trae_cn", ".trae/skills", None),
        ("zencoder", ".zencoder/skills", None),
        ("neovate", ".neovate/skills", None),
        ("pochi", ".pochi/skills", None),
        ("adal", ".adal/skills", None),
        ("kilo_code", ".kilocode/skills", None),
        ("roo_code", ".roo/skills", None),
        ("goose", ".goose/skills", None),
        ("gemini_cli", ".agents/skills", None),
        ("github_copilot", ".agents/skills", None),
        ("clawdbot", ".clawdbot/skills", None),
        ("droid", ".factory/skills", None),
        ("windsurf", ".windsurf/skills", None),
        ("moltbot", ".moltbot/skills", None),
        ("hermes-agent", ".hermes/skills", None),
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
        assert_eq!(a.project_relative_skills_dir, ".agents/skills");
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
        format!(".agents/skills ({} tools)", members.len())
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

// ---------------------------------------------------------------------------
// The deletion safety rule (`ensure_path_within_tool_dirs`)
// ---------------------------------------------------------------------------

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
