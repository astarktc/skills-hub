use std::fs;

use crate::core::environment::{HostOs, HostRoot, HostRoots};
use crate::core::errors::SignalError;
use crate::core::tool_adapters::{
    adapter_by_key, constituents_of, default_tool_adapters, detect_dirs_in,
    ensure_path_within_tool_dirs, former_skills_dirs_in, is_installed_in, mark_installed_in,
    project_adapter_by_key, project_skills_dir_in, reader_only_harnesses, scan_tool_dir,
    skills_dir_in, test_overrides, tool_holding_path, ToolAdapter, ToolId, VirtualGroup,
};
use crate::core::tool_adapters::{OsOverride, RegistryPath};

#[test]
fn path_resolution_joins_adapter_dirs_onto_home() {
    let home = tempfile::tempdir().unwrap();
    let cases: [(&str, &str, &[&str]); 6] = [
        ("claude_code", ".claude/skills", &[".claude"]),
        ("codex", ".agents/skills", &[".codex"]),
        ("cursor", ".cursor/skills", &[".cursor"]),
        // Amp keeps the skills convention dir Kimi left (round 20 #108); each
        // is detected by its own config root (Kimi's moved across releases).
        ("amp", ".config/agents/skills", &[".config/amp"]),
        ("kimi_cli", ".kimi-code/skills", &[".kimi-code", ".kimi"]),
        ("pi", ".pi/agent/skills", &[".pi"]),
    ];
    for (key, skills, detect) in cases {
        let adapter = adapter_by_key(key).unwrap_or_else(|| panic!("adapter {key}"));
        assert_eq!(
            skills_dir_in(&HostRoots::under_home(home.path()), adapter),
            home.path().join(skills),
            "skills dir for {key}"
        );
        assert_eq!(
            detect_dirs_in(&HostRoots::under_home(home.path()), adapter),
            detect
                .iter()
                .map(|d| home.path().join(d))
                .collect::<Vec<_>>(),
            "detect dirs for {key}"
        );
    }
}

/// Every registry row's resolved global paths — skills dir, former dirs,
/// detect dirs — as they stood before typed roots (round 20 ticket 01), with
/// the round 20 #108 corrections (ticket 10, spec Q12) as the one deliberate
/// edit since: each corrected row's old dir is its former dir. Typed roots
/// move no Deployment; a correction moves them only through the redeploy.
#[test]
fn every_existing_row_resolves_to_the_paths_it_always_has() {
    // `HostRoots::under_home` puts roaming app data at `~/AppData/Roaming`.
    const REASONIX_SKILLS: &str = if cfg!(windows) {
        "AppData/Roaming/reasonix/skills"
    } else {
        ".reasonix/skills"
    };
    const REASONIX_DETECT: &str = if cfg!(windows) {
        "AppData/Roaming/reasonix"
    } else {
        ".reasonix"
    };
    const DEVIN_SKILLS: &str = if cfg!(windows) {
        "AppData/Roaming/devin/skills"
    } else {
        ".config/devin/skills"
    };
    const DEVIN_DETECT: &str = if cfg!(windows) {
        "AppData/Roaming/devin"
    } else {
        ".config/devin"
    };
    #[allow(clippy::type_complexity)]
    let expected: &[(&str, &str, &[&str], &[&str])] = &[
        ("agents_skills", ".agents/skills", &[], &[".agents"]),
        ("cursor", ".cursor/skills", &[], &[".cursor"]),
        ("claude_code", ".claude/skills", &[], &[".claude"]),
        ("codex", ".agents/skills", &[".codex/skills"], &[".codex"]),
        (
            "opencode",
            ".config/opencode/skills",
            &[],
            &[".config/opencode"],
        ),
        (
            "antigravity",
            ".gemini/config/skills",
            &[".gemini/antigravity/global_skills"],
            &[".gemini/antigravity", ".gemini/antigravity-cli"],
        ),
        ("amp", ".config/agents/skills", &[], &[".config/amp"]),
        (
            "kimi_cli",
            ".kimi-code/skills",
            &[".config/agents/skills"],
            &[".kimi-code", ".kimi"],
        ),
        (
            "augment",
            ".augment/skills",
            &[".augment/rules"],
            &[".augment"],
        ),
        ("openclaw", ".openclaw/skills", &[], &[".openclaw"]),
        ("copaw", ".copaw/skill_pool", &[], &[".copaw"]),
        ("cline", ".cline/skills", &[], &[".cline"]),
        ("codebuddy", ".codebuddy/skills", &[], &[".codebuddy"]),
        (
            "command_code",
            ".commandcode/skills",
            &[],
            &[".commandcode"],
        ),
        ("continue", ".continue/skills", &[], &[".continue"]),
        ("crush", ".config/crush/skills", &[], &[".config/crush"]),
        ("junie", ".junie/skills", &[], &[".junie"]),
        ("iflow_cli", ".iflow/skills", &[], &[".iflow"]),
        ("kiro_cli", ".kiro/skills", &[], &[".kiro"]),
        ("kode", ".kode/skills", &[], &[".kode"]),
        ("mcpjam", ".mcpjam/skills", &[], &[".mcpjam"]),
        ("mistral_vibe", ".vibe/skills", &[], &[".vibe"]),
        ("mux", ".xum/skills", &[".mux/skills"], &[".mux", ".xum"]),
        ("openclaude", ".openclaude/skills", &[], &[".openclaude"]),
        (
            "openhands",
            ".agents/skills",
            &[".openhands/skills"],
            &[".openhands"],
        ),
        ("pi", ".pi/agent/skills", &[], &[".pi"]),
        ("qoder", ".qoder/skills", &[], &[".qoder"]),
        ("qoderwork", ".qoderwork/skills", &[], &[".qoderwork"]),
        ("qwen_code", ".qwen/skills", &[], &[".qwen"]),
        ("trae", ".trae/skills", &[], &[".trae"]),
        ("trae_cn", ".trae-cn/skills", &[], &[".trae-cn"]),
        (
            "zencoder",
            ".agents/skills",
            &[".zencoder/skills"],
            &[".zencoder"],
        ),
        ("neovate", ".neovate/skills", &[], &[".neovate"]),
        ("pochi", ".pochi/skills", &[], &[".pochi"]),
        ("adal", ".adal/skills", &[], &[".adal"]),
        (
            "kilo_code",
            ".kilo/skills",
            &[".kilocode/skills"],
            &[".kilocode", ".kilo"],
        ),
        ("roo_code", ".roo/skills", &[], &[".roo"]),
        (
            "goose",
            ".agents/skills",
            &[".config/goose/skills"],
            &[".config/goose"],
        ),
        ("gemini_cli", ".gemini/skills", &[], &[".gemini"]),
        ("github_copilot", ".copilot/skills", &[], &[".copilot"]),
        ("clawdbot", ".clawdbot/skills", &[], &[".clawdbot"]),
        ("droid", ".factory/skills", &[], &[".factory"]),
        (
            "windsurf",
            DEVIN_SKILLS,
            &[".codeium/windsurf/skills"],
            &[".codeium/windsurf", DEVIN_DETECT],
        ),
        ("moltbot", ".moltbot/skills", &[], &[".moltbot"]),
        ("hermes-agent", ".hermes/skills", &[], &[".hermes"]),
        // round 20 ticket 05 rows
        ("deepseek_harness", ".dsh/skills", &[], &[".dsh"]),
        ("zcode", ".zcode/skills", &[], &[".zcode"]),
        ("codewhale", ".codewhale/skills", &[], &[".codewhale"]),
        ("workbuddy", ".workbuddy/skills", &[], &[".workbuddy"]),
        // round 20 ticket 07 rows
        ("letta_code", ".letta/skills", &[], &[".letta"]),
        ("muse_code", ".config/muse/skills", &[], &[".config/muse"]),
        (
            "poolside",
            ".config/poolside/skills",
            &[],
            &[".config/poolside"],
        ),
        ("dirac", ".dirac/skills", &[], &[".dirac"]),
        ("code_studio", ".codestudio/skills", &[], &[".codestudio"]),
        (
            "codearts_agent",
            ".codeartsdoer/skills",
            &[],
            &[".codeartsdoer"],
        ),
        ("qoder_cn", ".lingma/skills", &[], &[".lingma"]),
        ("qoder_cn_cli", ".qoder-cn/skills", &[], &[".qoder-cn"]),
        (
            "kimchi",
            ".config/kimchi/harness/skills",
            &[],
            &[".config/kimchi"],
        ),
        ("astrbot", ".astrbot/data/skills", &[], &[".astrbot"]),
        ("oh_my_pi", ".omp/agent/skills", &[], &[".omp"]),
        ("stakpak", ".stakpak/skills", &[], &[".stakpak"]),
        // round 20 ticket 06 rows (new, so "always" is their first form;
        // Reasonix resolves under `%APPDATA%` on Windows)
        ("grok_build", ".grok/skills", &[], &[".grok"]),
        (
            "deep_agents_code",
            ".deepagents/agent/skills",
            &[],
            &[".deepagents"],
        ),
        ("reasonix", REASONIX_SKILLS, &[], &[REASONIX_DETECT]),
        (
            "cortex_code",
            ".snowflake/cortex/skills",
            &[],
            &[".snowflake/cortex"],
        ),
        ("rovo_dev", ".rovodev/skills", &[], &[".rovodev"]),
        ("ibm_bob", ".bob/skills", &[], &[".bob"]),
        ("forgecode", ".forge/skills", &[], &[".forge"]),
        ("firebender", ".firebender/skills", &[], &[".firebender"]),
        ("minimax_code", ".minimax/skills", &[], &[".minimax"]),
        (
            "posit_assistant",
            ".posit/assistant/skills",
            &[],
            &[".posit/assistant"],
        ),
        ("aider_desk", ".aider-desk/skills", &[], &[".aider-desk"]),
        ("autohand_code", ".autohand/skills", &[], &[".autohand"]),
    ];
    assert_eq!(
        expected.len(),
        default_tool_adapters().len(),
        "table must cover every registered tool"
    );
    let home = tempfile::tempdir().unwrap();
    let under = |rel: &str| {
        let mut path = home.path().to_path_buf();
        path.extend(rel.split('/'));
        path
    };
    for (key, skills, former, detect) in expected {
        let adapter = adapter_by_key(key).unwrap_or_else(|| panic!("adapter {key}"));
        assert_eq!(
            skills_dir_in(&HostRoots::under_home(home.path()), adapter),
            under(skills),
            "skills dir for {key}"
        );
        assert_eq!(
            former_skills_dirs_in(&HostRoots::under_home(home.path()), adapter),
            former.iter().map(|d| under(d)).collect::<Vec<_>>(),
            "former dirs for {key}"
        );
        assert_eq!(
            detect_dirs_in(&HostRoots::under_home(home.path()), adapter),
            detect.iter().map(|d| under(d)).collect::<Vec<_>>(),
            "detect dirs for {key}"
        );
    }
}

/// Substituted host roots: three distinct directories, so a path resolved
/// under the wrong root cannot pass.
fn distinct_roots(base: &std::path::Path) -> HostRoots {
    HostRoots {
        home: base.join("home"),
        roaming: base.join("roaming"),
        local: base.join("local"),
    }
}

/// A per-OS override applies on its OS only: Reasonix-style rows keep their
/// skills under `%APPDATA%` on Windows and under home elsewhere.
#[test]
fn a_windows_roaming_override_resolves_under_the_roaming_root() {
    let base = tempfile::tempdir().unwrap();
    let roots = distinct_roots(base.path());
    let path = RegistryPath::home(".reasonix/skills").overridden(&[OsOverride {
        os: HostOs::Windows,
        root: HostRoot::RoamingAppData,
        rel: "reasonix/skills",
    }]);

    assert_eq!(
        path.resolve_for(HostOs::Windows, &roots),
        base.path().join("roaming").join("reasonix").join("skills")
    );
    for elsewhere in [HostOs::MacOs, HostOs::Linux] {
        assert_eq!(
            path.resolve_for(elsewhere, &roots),
            base.path().join("home").join(".reasonix").join("skills"),
            "{elsewhere:?} keeps the default"
        );
    }
}

#[test]
fn a_local_app_data_override_resolves_under_the_local_root() {
    let base = tempfile::tempdir().unwrap();
    let roots = distinct_roots(base.path());
    let path = RegistryPath::home(".tool").overridden(&[
        OsOverride {
            os: HostOs::Windows,
            root: HostRoot::LocalAppData,
            rel: "Tool/Data",
        },
        OsOverride {
            os: HostOs::MacOs,
            root: HostRoot::RoamingAppData,
            rel: "Tool",
        },
    ]);

    assert_eq!(
        path.resolve_for(HostOs::Windows, &roots),
        base.path().join("local").join("Tool").join("Data")
    );
    assert_eq!(
        path.resolve_for(HostOs::MacOs, &roots),
        base.path().join("roaming").join("Tool")
    );
    assert_eq!(
        path.resolve_for(HostOs::Linux, &roots),
        base.path().join("home").join(".tool")
    );
}

/// The Harnesses pane shows a path the way the operator locates it: `~/…`
/// when it lies under home (always with `/`), the absolute path otherwise.
#[test]
fn a_registry_path_displays_under_tilde_only_when_it_lies_under_home() {
    let base = tempfile::tempdir().unwrap();
    let roaming = RegistryPath {
        root: HostRoot::RoamingAppData,
        rel: "Tool/skills",
        overrides: &[],
    };
    assert_eq!(
        RegistryPath::home(".claude/skills").display(&distinct_roots(base.path())),
        "~/.claude/skills"
    );
    assert_eq!(
        roaming.display(&HostRoots::under_home(base.path())),
        "~/AppData/Roaming/Tool/skills"
    );
    let elsewhere = distinct_roots(base.path());
    assert_eq!(
        roaming.display(&elsewhere),
        elsewhere
            .roaming
            .join("Tool")
            .join("skills")
            .to_string_lossy()
    );
}

/// Every default root is home, and the only per-OS overrides are Reasonix's
/// (round 20 ticket 06, the first production consumer) and Devin's (round 20
/// #108, brief I7): their skills and their config roots live under
/// `%APPDATA%` on Windows. A row moving off home, or another override, is a
/// deliberate registry edit that updates this pin.
#[test]
fn every_production_path_is_home_rooted_and_only_reasonix_and_devin_override() {
    let overridden: [(&str, &str, &[OsOverride]); 4] = [
        (
            "reasonix",
            ".reasonix/skills",
            &[OsOverride {
                os: HostOs::Windows,
                root: HostRoot::RoamingAppData,
                rel: "reasonix/skills",
            }],
        ),
        (
            "reasonix",
            ".reasonix",
            &[OsOverride {
                os: HostOs::Windows,
                root: HostRoot::RoamingAppData,
                rel: "reasonix",
            }],
        ),
        (
            "windsurf",
            ".config/devin/skills",
            &[OsOverride {
                os: HostOs::Windows,
                root: HostRoot::RoamingAppData,
                rel: "devin/skills",
            }],
        ),
        (
            "windsurf",
            ".config/devin",
            &[OsOverride {
                os: HostOs::Windows,
                root: HostRoot::RoamingAppData,
                rel: "devin",
            }],
        ),
    ];
    for adapter in default_tool_adapters() {
        let mut paths = vec![adapter.skills_dir];
        paths.extend(adapter.former_skills_dirs.iter().copied());
        paths.extend(adapter.detect_dirs.iter().copied());
        for path in paths {
            assert_eq!(path.root, HostRoot::Home, "{}: {path:?}", adapter.key());
            let expected = overridden
                .iter()
                .find(|(key, rel, _)| *key == adapter.key() && *rel == path.rel)
                .map(|(_, _, overrides)| *overrides)
                .unwrap_or(&[]);
            assert_eq!(path.overrides, expected, "{}: {path:?}", adapter.key());
        }
    }
}

/// Reasonix's documented home (`docs/SKILLS.md`, brief T27) is `~/.reasonix`
/// on macOS/Linux and `%APPDATA%\reasonix` on Windows: both its skills dir
/// and its detect dir follow it, resolved against substituted roots.
#[test]
fn reasonix_lives_under_roaming_app_data_on_windows_only() {
    let base = tempfile::tempdir().unwrap();
    let roots = distinct_roots(base.path());
    let reasonix = adapter_by_key("reasonix").expect("reasonix row");
    let roaming = base.path().join("roaming").join("reasonix");
    let home = base.path().join("home").join(".reasonix");

    assert_eq!(
        reasonix.skills_dir.resolve_for(HostOs::Windows, &roots),
        roaming.join("skills")
    );
    assert_eq!(
        reasonix.detect_dirs[0].resolve_for(HostOs::Windows, &roots),
        roaming
    );
    for elsewhere in [HostOs::MacOs, HostOs::Linux] {
        assert_eq!(
            reasonix.skills_dir.resolve_for(elsewhere, &roots),
            home.join("skills"),
            "{elsewhere:?}"
        );
        assert_eq!(
            reasonix.detect_dirs[0].resolve_for(elsewhere, &roots),
            home,
            "{elsewhere:?}"
        );
    }

    // On this host: installed where its config root lives, deployed beside it.
    let here = if cfg!(windows) { &roaming } else { &home };
    assert!(!is_installed_in(&roots, reasonix));
    mark_installed_in(&roots, reasonix);
    assert!(is_installed_in(&roots, reasonix));
    assert_eq!(skills_dir_in(&roots, reasonix), here.join("skills"));
    assert_eq!(detect_dirs_in(&roots, reasonix), vec![here.clone()]);
}

/// Brief I4 detect-dir hazards for the round 20 ticket 06 rows (spec:
/// binding): a harness is detected only by its own config root, never by a
/// parent another product shares. ForgeCode is also never detected by its
/// legacy un-dotted `~/forge`: Forge keeps reading `~/forge/skills` while
/// that directory exists (`forge_config` `resolve_base_path`), so detecting it
/// would deploy into a folder it does not read; Posit Assistant likewise never
/// by its pre-rename `~/.positai` (an old release reads `~/.positai/skills`).
#[test]
fn new_rows_are_never_detected_by_a_shared_or_unread_root() {
    let forbidden = [
        ("cortex_code", ".snowflake"),
        ("posit_assistant", ".posit"),
        ("posit_assistant", ".positai"),
        ("forgecode", "forge"),
    ];
    for (key, rel) in forbidden {
        let adapter = adapter_by_key(key).unwrap_or_else(|| panic!("{key}"));
        assert!(
            adapter.detect_dirs.iter().all(|dir| dir.rel != rel),
            "{key} must never be detected by `{rel}`"
        );
    }
    // A shared parent alone is not the harness.
    let home = tempfile::tempdir().unwrap();
    let roots = HostRoots::under_home(home.path());
    fs::create_dir_all(home.path().join(".snowflake")).unwrap();
    fs::write(home.path().join(".snowflake/config.toml"), b"").unwrap();
    fs::create_dir_all(home.path().join(".posit/connect")).unwrap();
    fs::create_dir_all(home.path().join("forge/skills")).unwrap();
    fs::create_dir_all(home.path().join(".positai/skills")).unwrap();
    for key in ["cortex_code", "posit_assistant", "forgecode"] {
        assert!(
            !is_installed_in(&roots, adapter_by_key(key).unwrap()),
            "{key} detected by a root that is not its own"
        );
    }
}

/// #103: the registry's relative dirs are written with `/`; joined onto a
/// Windows root they must not keep it (`C:\Users\op\.claude/skills` is what
/// 1.3.1 recorded). Every join spells the whole path with the platform's own
/// separator. Meaningful on Windows, where the foreign separator is `/`.
#[test]
fn registry_joins_spell_paths_with_the_platform_separator() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path();
    let foreign = if cfg!(windows) { '/' } else { '\\' };
    for adapter in default_tool_adapters() {
        let roots = HostRoots::under_home(root);
        let mut joined = vec![skills_dir_in(&roots, adapter)];
        joined.extend(project_skills_dir_in(root, adapter));
        joined.extend(detect_dirs_in(&roots, adapter));
        joined.extend(former_skills_dirs_in(&roots, adapter));
        for path in joined {
            assert!(
                !path.to_string_lossy().contains(foreign),
                "{}: {path:?} mixes separators",
                adapter.id.as_key()
            );
        }
    }
}

/// #103 on Windows: the exact spelling of a join, and a path recorded before
/// the fix (mixed separators) still compares equal to the one derived now —
/// `Path` compares components, and `\` and `/` both separate them there.
#[cfg(windows)]
#[test]
fn a_mixed_separator_path_recorded_before_the_fix_still_matches_the_registry() {
    let home = std::path::Path::new(r"C:\Users\op");
    let roots = HostRoots::under_home(home);
    let claude = adapter_by_key("claude_code").expect("claude adapter");
    assert_eq!(
        skills_dir_in(&roots, claude).to_str(),
        Some(r"C:\Users\op\.claude\skills")
    );
    assert_eq!(
        project_skills_dir_in(std::path::Path::new(r"D:\work\app"), claude)
            .expect("claude has a project dir")
            .to_str(),
        Some(r"D:\work\app\.claude\skills")
    );

    let recorded = std::path::PathBuf::from(r"C:\Users\op\.claude/skills\grill-me");
    assert_eq!(recorded, skills_dir_in(&roots, claude).join("grill-me"));
    assert!(recorded.starts_with(skills_dir_in(&roots, claude)));
    ensure_path_within_tool_dirs(&roots, &recorded).expect("a recorded target stays removable");
    assert_eq!(
        tool_holding_path(&roots, &recorded).map(|adapter| adapter.key()),
        Some("claude_code")
    );

    // A row's old artifact is found by its parent: a former dir recorded
    // with mixed separators is still that former dir.
    let augment = adapter_by_key("augment").expect("augment adapter");
    let former = std::path::PathBuf::from(r"C:\Users\op\.augment/rules\grill-me");
    assert_eq!(
        former.parent(),
        Some(former_skills_dirs_in(&roots, augment)[0].as_path())
    );
}

#[test]
fn every_adapter_resolves_under_home() {
    let home = tempfile::tempdir().unwrap();
    for adapter in default_tool_adapters() {
        assert!(
            skills_dir_in(&HostRoots::under_home(home.path()), adapter).starts_with(home.path())
        );
        for detect_dir in detect_dirs_in(&HostRoots::under_home(home.path()), adapter) {
            assert!(detect_dir.starts_with(home.path()));
        }
        assert!(
            !is_installed_in(&HostRoots::under_home(home.path()), adapter),
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

    assert!(!is_installed_in(&HostRoots::under_home(home.path()), codex));

    // Detect dir present (even without a skills dir) => installed.
    fs::create_dir_all(home.path().join(".codex")).unwrap();
    assert!(is_installed_in(&HostRoots::under_home(home.path()), codex));
    assert!(!is_installed_in(
        &HostRoots::under_home(home.path()),
        claude
    ));

    // Only the adapter's own detect dir counts — a sibling under the same
    // parent does not.
    let amp = adapter_by_key("amp").unwrap();
    fs::create_dir_all(home.path().join(".config/other")).unwrap();
    assert!(!is_installed_in(&HostRoots::under_home(home.path()), amp));
    fs::create_dir_all(home.path().join(".config/amp")).unwrap();
    assert!(is_installed_in(&HostRoots::under_home(home.path()), amp));
    // Shared-skills-dir tools are detected independently, by their own roots.
    let kimi = adapter_by_key("kimi_cli").unwrap();
    assert!(!is_installed_in(&HostRoots::under_home(home.path()), kimi));
}

#[test]
fn a_tool_whose_skills_dir_is_a_shared_convention_is_detected_by_its_own_root() {
    // The vendor-documented Kimi / Amp layout: config root beside the shared
    // `~/.config/agents/skills` convention, which alone is a footprint.
    let home = tempfile::tempdir().unwrap();
    let kimi = adapter_by_key("kimi_cli").unwrap();
    let amp = adapter_by_key("amp").unwrap();

    fs::create_dir_all(home.path().join(".config/agents/skills/foo")).unwrap();
    assert!(!is_installed_in(&HostRoots::under_home(home.path()), kimi));
    assert!(!is_installed_in(&HostRoots::under_home(home.path()), amp));

    fs::create_dir_all(home.path().join(".kimi")).unwrap();
    fs::write(home.path().join(".kimi/config.toml"), b"").unwrap();
    assert!(
        is_installed_in(&HostRoots::under_home(home.path()), kimi),
        "kimi-cli.com layout"
    );
    assert!(!is_installed_in(&HostRoots::under_home(home.path()), amp));

    fs::remove_dir_all(home.path().join(".kimi")).unwrap();
    fs::create_dir_all(home.path().join(".kimi-code")).unwrap();
    assert!(
        is_installed_in(&HostRoots::under_home(home.path()), kimi),
        "kimi.com/code layout"
    );

    fs::create_dir_all(home.path().join(".config/amp")).unwrap();
    fs::write(home.path().join(".config/amp/settings.json"), b"{}").unwrap();
    assert!(is_installed_in(&HostRoots::under_home(home.path()), amp));
}

#[test]
fn a_skills_only_footprint_is_not_an_installed_tool() {
    let home = tempfile::tempdir().unwrap();
    let kiro = adapter_by_key("kiro_cli").unwrap();

    // What a skill deployer leaves behind: the skills path and nothing else.
    fs::create_dir_all(home.path().join(".kiro/skills/ego-browser")).unwrap();
    fs::write(home.path().join(".kiro/skills/ego-browser/SKILL.md"), b"x").unwrap();
    assert!(!is_installed_in(&HostRoots::under_home(home.path()), kiro));

    // An empty skills dir is the same footprint.
    fs::remove_dir_all(home.path().join(".kiro/skills/ego-browser")).unwrap();
    assert!(!is_installed_in(&HostRoots::under_home(home.path()), kiro));

    // Finder noise does not promote a footprint.
    fs::write(home.path().join(".kiro/.DS_Store"), b"").unwrap();
    assert!(!is_installed_in(&HostRoots::under_home(home.path()), kiro));

    // Anything else beside the skills path is the tool.
    fs::write(home.path().join(".kiro/settings.json"), b"{}").unwrap();
    assert!(is_installed_in(&HostRoots::under_home(home.path()), kiro));
}

#[test]
fn footprint_rule_walks_every_level_down_to_the_skills_dir() {
    let home = tempfile::tempdir().unwrap();
    let pi = adapter_by_key("pi").unwrap();

    // `.pi/agent/skills/x` alone: `.pi` holds only `agent`, `agent` only `skills`.
    fs::create_dir_all(home.path().join(".pi/agent/skills/x")).unwrap();
    assert!(!is_installed_in(&HostRoots::under_home(home.path()), pi));

    // A sibling at an intermediate level is the tool.
    fs::write(home.path().join(".pi/agent/settings.json"), b"{}").unwrap();
    assert!(is_installed_in(&HostRoots::under_home(home.path()), pi));
}

#[test]
fn a_virtual_group_is_installed_by_its_directory_alone() {
    let home = tempfile::tempdir().unwrap();
    let agents = adapter_by_key("agents_skills").unwrap();
    assert!(agents.as_virtual_group().is_some());

    // `~/.agents` legitimately holds only `skills/`: the convention is the dir.
    fs::create_dir_all(home.path().join(".agents/skills/x")).unwrap();
    assert!(is_installed_in(&HostRoots::under_home(home.path()), agents));
}

#[test]
fn no_adapter_is_detected_by_a_shared_skills_convention_dir() {
    // The parent of a skills dir several tools share (`~/.config/agents`)
    // is a convention, not a tool: presence there is exactly the footprint
    // the rule rejects. Only a virtual group entry may be detected by one.
    for adapter in default_tool_adapters() {
        assert!(
            !adapter.detect_dirs.is_empty(),
            "{} has no detect dir",
            adapter.id.as_key()
        );
        if adapter.as_virtual_group().is_some() {
            continue;
        }
        let shared = default_tool_adapters()
            .iter()
            .filter(|other| other.skills_dir.same_location(&adapter.skills_dir))
            .count()
            > 1;
        let (skills_root, skills_rel) = adapter.skills_dir.effective(HostOs::CURRENT);
        let convention_root = std::path::Path::new(skills_rel).parent();
        for detect_dir in adapter.detect_dirs {
            let (detect_root, detect_rel) = detect_dir.effective(HostOs::CURRENT);
            assert!(
                !(shared
                    && detect_root == skills_root
                    && convention_root == Some(std::path::Path::new(detect_rel))),
                "{}: detected by the shared convention dir {}",
                adapter.id.as_key(),
                detect_rel
            );
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
    assert!(adapter_by_key("openclaw").is_some());
    assert!(adapter_by_key("command_code").is_some());
    assert!(adapter_by_key("qwen_code").is_some());
}

/// Round 20 #108: Kimi Code left the `~/.config/agents/skills` folder it
/// shared with Amp; each is now its own Deployment folder.
#[test]
fn amp_keeps_the_folder_kimi_left() {
    use crate::core::placement::{own_folder, ScopeKind};
    assert_eq!(own_folder(ScopeKind::Global, "amp"), Some("amp"));
    assert_eq!(own_folder(ScopeKind::Global, "kimi_cli"), Some("kimi_cli"));
}

/// A placeholder detect dir for adapters only `scan_tool_dir` reads.
const IGNORED_DIRS: &[RegistryPath] = &[RegistryPath::home("ignored")];

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
        skills_dir: RegistryPath::home("ignored"),
        former_skills_dirs: &[],
        detect_dirs: IGNORED_DIRS,
        project_skills_dir: Some("ignored"),
        group: None,
        supports_symlink: true,
        reads: crate::core::tool_adapters::reads::ReadFacts::UNKNOWN,
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
        skills_dir: RegistryPath::home("ignored"),
        former_skills_dirs: &[],
        detect_dirs: IGNORED_DIRS,
        project_skills_dir: Some("ignored"),
        group: None,
        supports_symlink: true,
        reads: crate::core::tool_adapters::reads::ReadFacts::UNKNOWN,
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
        skills_dir: RegistryPath::home("ignored"),
        former_skills_dirs: &[],
        detect_dirs: IGNORED_DIRS,
        project_skills_dir: Some("ignored"),
        group: None,
        supports_symlink: false,
        reads: crate::core::tool_adapters::reads::ReadFacts::UNKNOWN,
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

/// Every Tool's project-scope dir, as a table: the registry is the only
/// source, so this pins the mapping against accidental edits. `None` means
/// the harness reads no project skills dir (WorkBuddy, AstrBot; the shape is proven by
/// `a_harness_without_a_project_dir_has_no_project_scope`).
#[test]
fn project_relative_skills_dir_for_every_tool() {
    let expected: &[(&str, Option<&str>)] = &[
        ("agents_skills", Some(".agents/skills")),
        ("cursor", Some(".agents/skills")),
        ("claude_code", Some(".claude/skills")),
        ("codex", Some(".agents/skills")),
        ("opencode", Some(".agents/skills")),
        ("antigravity", Some(".agents/skills")),
        ("amp", Some(".agents/skills")),
        ("kimi_cli", Some(".agents/skills")),
        ("augment", Some(".augment/skills")),
        ("openclaw", Some("skills")),
        ("copaw", Some(".copaw/skill_pool")),
        ("cline", Some(".agents/skills")),
        ("codebuddy", Some(".codebuddy/skills")),
        ("command_code", Some(".commandcode/skills")),
        ("continue", Some(".continue/skills")),
        ("crush", Some(".crush/skills")),
        ("junie", Some(".junie/skills")),
        ("iflow_cli", Some(".iflow/skills")),
        ("kiro_cli", Some(".kiro/skills")),
        ("kode", Some(".kode/skills")),
        ("mcpjam", Some(".mcpjam/skills")),
        ("mistral_vibe", Some(".vibe/skills")),
        ("mux", Some(".mux/skills")),
        ("openclaude", Some(".openclaude/skills")),
        ("openhands", Some(".openhands/skills")),
        ("pi", Some(".pi/skills")),
        ("qoder", Some(".qoder/skills")),
        ("qoderwork", Some(".qoderwork/skills")),
        ("qwen_code", Some(".qwen/skills")),
        ("trae", Some(".trae/skills")),
        ("trae_cn", Some(".trae/skills")),
        ("zencoder", Some(".zencoder/skills")),
        ("neovate", Some(".neovate/skills")),
        ("pochi", Some(".pochi/skills")),
        ("adal", Some(".adal/skills")),
        ("kilo_code", Some(".kilocode/skills")),
        ("roo_code", Some(".roo/skills")),
        ("goose", Some(".goose/skills")),
        ("gemini_cli", Some(".agents/skills")),
        ("github_copilot", Some(".agents/skills")),
        ("clawdbot", Some(".clawdbot/skills")),
        ("droid", Some(".factory/skills")),
        ("windsurf", Some(".windsurf/skills")),
        ("moltbot", Some(".moltbot/skills")),
        ("hermes-agent", Some(".hermes/skills")),
        // round 20 ticket 05 rows
        ("deepseek_harness", Some(".dsh/skills")),
        ("zcode", Some(".zcode/skills")),
        ("codewhale", Some(".codewhale/skills")),
        ("workbuddy", None),
        // round 20 ticket 07 rows
        ("letta_code", Some(".agents/skills")),
        ("muse_code", Some(".agents/skills")),
        ("poolside", Some(".agents/skills")),
        ("dirac", Some(".agents/skills")),
        ("code_studio", Some(".codestudio/skills")),
        ("codearts_agent", Some(".codeartsdoer/skills")),
        ("qoder_cn", Some(".lingma/skills")),
        ("qoder_cn_cli", Some(".qoder/skills")),
        ("kimchi", Some(".kimchi/skills")),
        ("astrbot", None),
        ("oh_my_pi", Some(".omp/skills")),
        ("stakpak", Some(".stakpak/skills")),
        // round 20 ticket 06 rows
        ("grok_build", Some(".grok/skills")),
        ("deep_agents_code", Some(".deepagents/skills")),
        ("reasonix", Some(".reasonix/skills")),
        ("cortex_code", Some(".cortex/skills")),
        ("rovo_dev", Some(".rovodev/skills")),
        ("ibm_bob", Some(".bob/skills")),
        ("forgecode", Some(".forge/skills")),
        ("firebender", Some(".firebender/skills")),
        ("minimax_code", Some(".minimax/skills")),
        ("posit_assistant", Some(".posit/assistant/skills")),
        ("aider_desk", Some(".aider-desk/skills")),
        ("autohand_code", Some(".autohand/skills")),
    ];
    assert_eq!(
        expected.len(),
        default_tool_adapters().len(),
        "table must cover every registered tool"
    );
    for (key, dir) in expected {
        let adapter = adapter_by_key(key).unwrap_or_else(|| panic!("adapter {key}"));
        assert_eq!(adapter.project_skills_dir, *dir, "project dir for {key}");
        assert_eq!(
            project_adapter_by_key(key).is_some(),
            dir.is_some(),
            "project-scope admission for {key}"
        );
    }
}

/// A harness with no project skills dir has no project scope at all: no
/// project path, no project-scope admission, no project catalog entry, and
/// no ignore pattern.
#[test]
fn a_harness_without_a_project_dir_has_no_project_scope() {
    let home = tempfile::tempdir().unwrap();
    let roots = HostRoots::under_home(home.path());
    let hermes = adapter_by_key("hermes-agent").unwrap();
    let globally_only = test_overrides::shadow(ToolAdapter {
        project_skills_dir: None,
        ..hermes.clone()
    });
    mark_installed_in(&roots, globally_only);

    assert_eq!(project_skills_dir_in(home.path(), globally_only), None);
    assert!(project_adapter_by_key("hermes-agent").is_none());
    assert!(
        adapter_by_key("hermes-agent").is_some(),
        "still a global target"
    );
    assert!(
        crate::core::tool_adapters::project_tool_entries(&roots)
            .iter()
            .all(|entry| entry.key != "hermes-agent"),
        "absent from the project catalog"
    );
    assert!(
        crate::core::tool_adapters::global_tool_entries(&roots)
            .iter()
            .any(|entry| entry.key == "hermes-agent" && entry.installed),
        "still listed and detected globally"
    );
    assert!(crate::core::gitignore::patterns_for_tools([globally_only]).is_empty());
}

#[test]
fn agents_standard_group_lists_its_constituents_and_one_entry() {
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
            // round 20 ticket 07 rows
            "letta_code",
            "muse_code",
            "poolside",
            "dirac",
        ]
    );
    for a in constituents_of(VirtualGroup::AgentsStandard) {
        assert_eq!(a.project_skills_dir, Some(".agents/skills"));
        assert!(a.as_virtual_group().is_none());
    }
    let entries: Vec<&ToolAdapter> = default_tool_adapters()
        .iter()
        .filter(|a| a.as_virtual_group().is_some())
        .collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, ToolId::AgentsStandard);
    assert_eq!(entries[0].display_name, ".agents/skills");
    assert_eq!(
        entries[0].group, None,
        "a group entry is not its own member"
    );
}

/// Every registry entry but Deep Agents Code can take a symlinked skills dir
/// (Cursor was copy-only until v-next ticket 38). Deep Agents Code rejects a
/// skill whose resolved path leaves its skill roots (brief I8, T8), and a
/// Skills Hub link resolves into the central repo, so it deploys by copy
/// (spec Q29). Pinned so a change of mode is a deliberate registry edit.
#[test]
fn only_deep_agents_code_deploys_by_copy() {
    let no_symlink: Vec<&str> = default_tool_adapters()
        .iter()
        .filter(|a| !a.supports_symlink)
        .map(|a| a.key())
        .collect();
    assert_eq!(no_symlink, vec!["deep_agents_code"], "copy-only entries");
}

/// Keys are unique across both registry tables: a reader-only harness's key
/// must never shadow (or be shadowed by) an adapter's.
#[test]
fn registry_keys_are_unique() {
    let mut keys: Vec<&str> = default_tool_adapters().iter().map(|a| a.key()).collect();
    keys.extend(reader_only_harnesses().iter().map(|r| r.key));
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
        skills_dir_in(&HostRoots::under_home(home.path()), augment),
        home.path().join(".augment/skills")
    );
    assert_eq!(
        former_skills_dirs_in(&HostRoots::under_home(home.path()), augment),
        vec![home.path().join(".augment/rules")]
    );
    assert_eq!(
        detect_dirs_in(&HostRoots::under_home(home.path()), augment),
        vec![home.path().join(".augment")]
    );
    assert_eq!(augment.project_skills_dir, Some(".augment/skills"));
}

/// A former dir is a registry fact about a *correction*: it is never the
/// row's own current skills dir. It may be another harness's current dir —
/// Kimi Code's former `~/.config/agents/skills` is Amp's — because nothing
/// decides placement from a former dir (the redeploy compares rows with
/// placement, and removes an old artifact only when no other live row records
/// it), so Amp's Deployments there are never moved or removed for Kimi's sake.
#[test]
fn no_former_skills_dir_is_its_own_rows_current_dir() {
    let mut shared_with_another_row = Vec::new();
    for adapter in default_tool_adapters() {
        for former in adapter.former_skills_dirs {
            assert!(
                !adapter.skills_dir.same_location(former),
                "{}'s former dir {former:?} is its own skills dir",
                adapter.key()
            );
            for other in default_tool_adapters() {
                if other.skills_dir.same_location(former) {
                    shared_with_another_row.push((adapter.key(), other.key()));
                }
            }
        }
    }
    assert_eq!(shared_with_another_row, vec![("kimi_cli", "amp")]);
}

// ---------------------------------------------------------------------------
// The deletion safety rule (`ensure_path_within_tool_dirs`)
// ---------------------------------------------------------------------------

#[test]
fn a_path_inside_a_former_tool_skills_dir_is_allowed() {
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join(".augment/rules/some-skill");
    ensure_path_within_tool_dirs(&HostRoots::under_home(home.path()), &path)
        .unwrap_or_else(|err| panic!("a former dir artifact is ours to remove: {err:#}"));
}

#[test]
fn a_path_inside_any_tool_skills_dir_is_allowed() {
    let home = tempfile::tempdir().unwrap();
    for key in ["claude_code", "pi", "amp"] {
        let adapter = adapter_by_key(key).unwrap();
        let path = skills_dir_in(&HostRoots::under_home(home.path()), adapter).join("some-skill");
        ensure_path_within_tool_dirs(&HostRoots::under_home(home.path()), &path)
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
        let err = ensure_path_within_tool_dirs(&HostRoots::under_home(home.path()), &outside)
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
        let path = skills_dir_in(&HostRoots::under_home(home.path()), adapter).join("some-skill");
        let holder = tool_holding_path(&HostRoots::under_home(home.path()), &path)
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
            tool_holding_path(&HostRoots::under_home(home.path()), &outside).map(|a| a.key()),
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
    let real = skills_dir_in(&HostRoots::under_home(home.path()), claude).join("some-skill");
    fs::create_dir_all(&real).unwrap();
    let elsewhere = home.path().join("Documents");
    fs::create_dir_all(&elsewhere).unwrap();
    let link = elsewhere.join("alias");
    std::os::unix::fs::symlink(&real, &link).unwrap();

    assert_eq!(
        tool_holding_path(&HostRoots::under_home(home.path()), &link).map(|a| a.key()),
        Some("claude_code")
    );
}

/// Round 20 relabels move nothing: each product's new name is a label only,
/// and the key every stored row (targets, selections, assignments) carries
/// is unchanged. Folders moved only where #108 corrected them (ticket 10),
/// and then the folder the key always had is its former dir — so the
/// redeploy, not the relabel, moves its Deployments.
#[test]
fn relabelled_harnesses_keep_their_stored_keys_and_folders() {
    let home = tempfile::tempdir().unwrap();
    let roots = HostRoots::under_home(home.path());
    let devin = if cfg!(windows) {
        "AppData/Roaming/devin/skills"
    } else {
        ".config/devin/skills"
    };
    let cases: [(&str, &str, &str, Option<&str>); 4] = [
        ("mux", "Xum", ".xum/skills", Some(".mux/skills")),
        (
            "windsurf",
            "Devin (Windsurf)",
            devin,
            Some(".codeium/windsurf/skills"),
        ),
        ("copaw", "QwenPaw", ".copaw/skill_pool", None),
        (
            "zencoder",
            "Zencoder / Zenflow",
            ".agents/skills",
            Some(".zencoder/skills"),
        ),
    ];
    let under = |rel: &str| {
        let mut path = home.path().to_path_buf();
        path.extend(rel.split('/'));
        path
    };
    for (stored_key, label, skills, former) in cases {
        let adapter = adapter_by_key(stored_key).unwrap_or_else(|| panic!("{stored_key}"));
        assert_eq!(adapter.key(), stored_key, "the stored key round-trips");
        assert_eq!(adapter.display_name, label);
        assert_eq!(
            skills_dir_in(&roots, adapter),
            under(skills),
            "{stored_key}"
        );
        assert_eq!(
            former_skills_dirs_in(&roots, adapter),
            former.map(under).into_iter().collect::<Vec<_>>(),
            "{stored_key}'s old folder is its former dir"
        );
    }
}

/// #108 (round 20 ticket 10): the detect dirs Wave A held back landed with
/// their folder corrections — Antigravity CLI's root and Devin's config root
/// now detect rows that deploy where those harnesses read. QwenPaw's
/// `.qwenpaw` still waits for its own correction (BACKLOG #118): its row
/// deploys under `~/.copaw`, which `.qwenpaw` would mis-detect.
#[test]
fn the_deferred_detect_dirs_landed_with_their_corrections() {
    let antigravity = adapter_by_key("antigravity").unwrap();
    assert!(antigravity
        .detect_dirs
        .iter()
        .any(|dir| dir.rel == ".gemini/antigravity-cli"));
    assert_eq!(antigravity.skills_dir.rel, ".gemini/config/skills");
    let devin = adapter_by_key("windsurf").unwrap();
    assert!(devin
        .detect_dirs
        .iter()
        .any(|dir| dir.rel == ".config/devin"));
    assert_eq!(devin.skills_dir.rel, ".config/devin/skills");
    for adapter in default_tool_adapters() {
        assert!(
            adapter.detect_dirs.iter().all(|dir| dir.rel != ".qwenpaw"),
            "{}: `.qwenpaw` waits for #118",
            adapter.key()
        );
    }
}

/// One parsed row of the README's supported-harnesses table.
#[derive(Debug, PartialEq, Eq)]
struct ReadmeRow {
    key: String,
    label: String,
    skills_dir: Option<String>,
    project_dir: Option<String>,
    detect_dirs: Vec<String>,
}

/// The backticked tokens of a README cell; a cell opening with `—` is none.
fn readme_cell_paths(cell: &str) -> Vec<String> {
    if cell.trim_start().starts_with('—') {
        return vec![];
    }
    cell.split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

fn readme_rows() -> Vec<ReadmeRow> {
    let readme = include_str!("../../../../README.md");
    let section = readme
        .split("## Supported AI Coding Harnesses")
        .nth(1)
        .expect("README has the supported-harnesses section");
    let section = section.split("\n## ").next().unwrap_or(section);
    section
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(|line| {
            let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
            assert_eq!(cells.len(), 5, "README row has five columns: {line}");
            ReadmeRow {
                key: cells[0].trim_matches('`').to_string(),
                label: cells[1].to_string(),
                skills_dir: readme_cell_paths(cells[2]).into_iter().next(),
                project_dir: readme_cell_paths(cells[3]).into_iter().next(),
                detect_dirs: readme_cell_paths(cells[4]),
            }
        })
        .collect()
}

/// The README's supported-harnesses table is the registry, row for row —
/// adapters and reader-only harnesses alike, in registry order, with each
/// row's label and (default-root) dirs. Adding, relabelling or moving a
/// harness without its README row fails here.
#[test]
fn the_readme_table_lists_every_registry_row() {
    let expected: Vec<ReadmeRow> = default_tool_adapters()
        .iter()
        .map(|adapter| ReadmeRow {
            key: adapter.key().to_string(),
            label: adapter.display_name.to_string(),
            skills_dir: Some(adapter.skills_dir.rel.to_string()),
            project_dir: adapter.project_skills_dir.map(str::to_string),
            detect_dirs: adapter
                .detect_dirs
                .iter()
                .map(|dir| dir.rel.to_string())
                .collect(),
        })
        .chain(reader_only_harnesses().iter().map(|reader| {
            ReadmeRow {
                key: reader.key.to_string(),
                label: reader.display_name.to_string(),
                skills_dir: None,
                project_dir: None,
                detect_dirs: reader
                    .detect_dirs
                    .iter()
                    .map(|d| d.rel.to_string())
                    .collect(),
            }
        }))
        .collect();
    let rows = readme_rows();
    assert_eq!(
        rows.len(),
        expected.len(),
        "README table rows vs registry rows (adapters + reader-only)"
    );
    assert_eq!(rows, expected);
}

/// The brief-snapshot table (round 20 ticket 02, spec Q5/Q20): every row's
/// shared-folder read facts restated from
/// `docs/design-inputs/2026-11-05-harness-candidates-and-shared-dir-reads.md`
/// § Part 2 "Existing registry rows", each with the brief row it encodes.
/// LIKELY and UNKNOWN cells are `Unknown`. A registry fact changed without
/// this table (and the brief row) fails here.
#[test]
fn every_row_restates_its_brief_read_facts() {
    use crate::core::tool_adapters::reads::{
        DuplicateBehaviour as D, Precedence as P, ReadMode as M, SharedRead::*,
    };
    type Row = (
        &'static str,    // registry key
        &'static str,    // brief row
        [SharedRead; 2], // global: ~/.agents, ~/.claude
        [SharedRead; 2], // project: .agents, .claude
        P,
        D,
        &'static [(&'static str, M)], // also_reads (home-relative)
    );
    use crate::core::tool_adapters::reads::SharedRead;
    const U: SharedRead = Unknown;
    const ALL_UNKNOWN: [SharedRead; 2] = [U, U];
    const CLI_ONLY: M = M::Conditional("Antigravity CLI only");
    // Kimchi reads each agent's home folder only when the project has none of
    // that agent's own (project-local first, home the fallback): never a
    // by-default read, so never a warning pair.
    const KIMCHI_FALLBACK: &str = "only when the project has no folder of its own";
    const KIMCHI_FALLBACK_READ: M = M::Conditional(KIMCHI_FALLBACK);
    let expected: &[Row] = &[
        (
            "agents_skills",
            "(omitted: the convention folder itself)",
            [Yes, No],
            [Yes, No],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "cursor",
            "Cursor",
            [Yes, Yes],
            [Yes, Yes],
            P::Unknown,
            D::ShowsAll,
            &[(".codex/skills", M::Default)],
        ),
        (
            "claude_code",
            "Claude Code",
            [No, Yes],
            [No, Yes],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "codex",
            "Codex",
            [Yes, No],
            [Yes, No],
            P::Both,
            D::ShowsAll,
            // Its own deprecated folder, still read (round 20 #108 moved the row).
            &[(".codex/skills", M::Default)],
        ),
        (
            "opencode",
            "OpenCode",
            [Yes, Yes],
            [Yes, Yes],
            P::Unknown,
            D::ShowsAll,
            &[],
        ),
        (
            "antigravity",
            "Antigravity",
            [No, No],
            [Yes, No],
            P::Unknown,
            D::Unknown,
            &[
                (".gemini/skills", CLI_ONLY),
                (".gemini/antigravity-cli/skills", CLI_ONLY),
                (".gemini/antigravity/skills", M::Legacy),
            ],
        ),
        (
            "amp",
            "Amp",
            [Yes, Yes],
            [Yes, Yes],
            P::Own,
            D::Shadows,
            &[
                (".config/agents/skills", M::Default),
                (".config/amp/skills", M::Default),
                (".claude/plugins/cache", M::Default),
            ],
        ),
        (
            "kimi_cli",
            "Kimi Code CLI",
            [
                Yes,
                Conditional("legacy kimi-cli only, when ~/.kimi/skills is absent"),
            ],
            [Yes, No],
            P::Unknown,
            D::Unknown,
            &[
                (".config/agents/skills", M::Legacy),
                (".kimi/skills", M::Legacy),
                (".codex/skills", M::Legacy),
            ],
        ),
        (
            "augment",
            "Augment",
            [Yes, Yes],
            [Yes, Yes],
            P::Own,
            D::Shadows,
            &[],
        ),
        (
            "openclaw",
            "OpenClaw",
            [
                Conditional("default state dir only: a non-default OPENCLAW_STATE_DIR drops it"),
                No,
            ],
            [Yes, No],
            P::Shared,
            D::Shadows,
            &[],
        ),
        (
            "copaw",
            "Copaw → QwenPaw",
            [No, No],
            [No, No],
            P::Own,
            D::Shadows,
            &[],
        ),
        (
            "cline",
            "Cline",
            [Yes, No],
            [Yes, Yes],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "codebuddy",
            "CodeBuddy",
            [No, U],
            [No, U],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "command_code",
            "Command Code",
            [Yes, No],
            [Yes, No],
            P::Own,
            D::Shadows,
            &[],
        ),
        (
            "continue",
            "Continue",
            ALL_UNKNOWN,
            ALL_UNKNOWN,
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "crush",
            "Crush",
            [Yes, Yes],
            [Yes, Yes],
            P::Unknown,
            D::Unknown,
            &[(".config/agents/skills", M::Default)],
        ),
        (
            "junie",
            "Junie",
            [Yes, No],
            [Conditional("trusted projects only"), No],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "iflow_cli",
            "iFlow CLI",
            [No, No],
            [No, No],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "kiro_cli",
            "Kiro CLI",
            [No, No],
            [No, No],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "kode",
            "Kode",
            ALL_UNKNOWN,
            ALL_UNKNOWN,
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "mcpjam",
            "MCPJam",
            [Yes, Yes],
            [Yes, Yes],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "mistral_vibe",
            "Mistral Vibe",
            [Yes, No],
            [Conditional("trusted projects only"), No],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "mux",
            "Mux → Xum",
            [Yes, U],
            ALL_UNKNOWN,
            P::Own,
            D::Shadows,
            &[(".xum/skills", M::Default), (".mux/skills", M::Default)],
        ),
        (
            "openclaude",
            "OpenClaude IDE",
            ALL_UNKNOWN,
            ALL_UNKNOWN,
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "openhands",
            "OpenHands",
            [Yes, No],
            [Yes, No],
            P::Shared,
            D::Shadows,
            &[(".openhands/skills", M::Default)],
        ),
        (
            "pi",
            "Pi",
            [Yes, No],
            [Yes, No],
            P::Unknown,
            D::Shadows,
            &[],
        ),
        (
            "qoder",
            "Qoder",
            [
                Conditional(
                    "the Qoder desktop app only; the IDE and CLI read only ~/.qoder/skills",
                ),
                No,
            ],
            [No, No],
            P::Own,
            D::Shadows,
            &[],
        ),
        (
            "qoderwork",
            "QoderWork",
            [No, No],
            [No, No],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "qwen_code",
            "Qwen Code",
            [Yes, OptIn],
            [Yes, No],
            P::Own,
            D::Shadows,
            &[],
        ),
        (
            "trae",
            "Trae",
            ALL_UNKNOWN,
            [OptIn, U],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "trae_cn",
            "Trae CN",
            ALL_UNKNOWN,
            [OptIn, U],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "zencoder",
            "Zencoder (+ Zenflow)",
            [Yes, U],
            [Yes, Yes],
            P::Unknown,
            D::Unknown,
            &[(".zencoder/skills", M::Default)],
        ),
        (
            "neovate",
            "Neovate",
            [No, Yes],
            [No, Yes],
            P::Own,
            D::Shadows,
            &[],
        ),
        (
            "pochi",
            "Pochi",
            [Yes, No],
            [Yes, No],
            P::Own,
            D::Shadows,
            &[],
        ),
        (
            "adal",
            "AdaL",
            ALL_UNKNOWN,
            ALL_UNKNOWN,
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "kilo_code",
            "Kilo Code",
            [Yes, OptIn],
            [Yes, OptIn],
            P::Unknown,
            D::Unknown,
            &[
                (".kilo/skills", M::Default),
                (".kilocode/skills", M::Unknown),
            ],
        ),
        (
            "roo_code",
            "Roo Code",
            [Yes, No],
            [Yes, No],
            P::Own,
            D::Shadows,
            &[],
        ),
        (
            "goose",
            "Goose",
            [Yes, Yes],
            [Yes, Yes],
            P::Unknown,
            D::Unknown,
            &[
                (".agents/plugins", M::Default),
                (".config/goose/skills", M::Default),
            ],
        ),
        (
            "gemini_cli",
            "Gemini CLI",
            [Yes, No],
            [Yes, No],
            P::Shared,
            D::Shadows,
            &[],
        ),
        (
            "github_copilot",
            "GitHub Copilot",
            [Yes, No],
            [Yes, Yes],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "clawdbot",
            "Clawdbot (retire, #108)",
            ALL_UNKNOWN,
            ALL_UNKNOWN,
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "droid",
            "Droid",
            [Yes, No],
            [Yes, No],
            P::Unknown,
            D::Shadows,
            &[(".agent/skills", M::Default)],
        ),
        (
            "windsurf",
            "Windsurf → Devin",
            [Yes, No],
            [Yes, Yes],
            P::Unknown,
            D::Unknown,
            &[
                (".codeium/windsurf/skills", M::Default),
                (".copilot/skills", M::Default),
            ],
        ),
        (
            "moltbot",
            "MoltBot (retire, #108)",
            ALL_UNKNOWN,
            ALL_UNKNOWN,
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "hermes-agent",
            "Hermes Agent",
            [OptIn, OptIn],
            ALL_UNKNOWN,
            P::Own,
            D::Shadows,
            &[],
        ),
        // round 20 ticket 05 rows
        (
            "deepseek_harness",
            "DeepSeek Harness (The four #92 rows)",
            [Yes, No],
            [Yes, No],
            P::Own,
            D::Shadows,
            &[],
        ),
        (
            "zcode",
            "ZCode (The four #92 rows)",
            [Yes, No],
            [Yes, No],
            P::Own,
            D::Shadows,
            &[],
        ),
        (
            "codewhale",
            "Codewhale (The four #92 rows)",
            [Yes, Yes],
            [Yes, Yes],
            P::Own,
            D::Shadows,
            &[(".deepseek/skills", M::Default)],
        ),
        (
            "workbuddy",
            "WorkBuddy (The four #92 rows, LIKELY)",
            ALL_UNKNOWN,
            ALL_UNKNOWN,
            P::Unknown,
            D::Unknown,
            &[],
        ),
        // round 20 ticket 07 rows
        (
            "letta_code",
            "Letta Code (extra)",
            [No, No],
            [Yes, No],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "muse_code",
            "addendum: Muse Code",
            [Yes, Yes],
            [Yes, Yes],
            P::Unknown,
            D::Unknown,
            &[(".codex/skills", M::Default)],
        ),
        (
            "poolside",
            "addendum: Poolside",
            [Yes, U],
            [Yes, U],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "dirac",
            "addendum: Dirac",
            [Yes, Yes],
            [Yes, Yes],
            P::Shared,
            D::Shadows,
            &[(".ai/skills", M::Default)],
        ),
        (
            "code_studio",
            "Code Studio",
            [Yes, Yes],
            [Yes, Yes],
            P::Unknown,
            D::Unknown,
            &[(".copilot/skills", M::Default)],
        ),
        (
            "codearts_agent",
            "CodeArts Agent",
            [No, No],
            [No, No],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "qoder_cn",
            "Qoder CN (IDE, ex-Lingma)",
            [No, No],
            [No, No],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "qoder_cn_cli",
            "Qoder CN CLI",
            [No, No],
            [No, No],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "kimchi",
            "Kimchi (optional)",
            [Conditional(KIMCHI_FALLBACK), OptIn],
            [Yes, OptIn],
            P::Unknown,
            D::Shadows,
            &[
                (".pi/agent/skills", M::Default),
                (".codex/skills", KIMCHI_FALLBACK_READ),
                (".cursor/skills", KIMCHI_FALLBACK_READ),
                (".warp/skills", KIMCHI_FALLBACK_READ),
                (".factory/skills", KIMCHI_FALLBACK_READ),
                (".gemini/skills", KIMCHI_FALLBACK_READ),
                (".copilot/skills", KIMCHI_FALLBACK_READ),
                (".github/skills", KIMCHI_FALLBACK_READ),
            ],
        ),
        (
            "astrbot",
            "AstrBot (optional)",
            [No, No],
            [No, No],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "oh_my_pi",
            "oh-my-pi (extra)",
            [Yes, Yes],
            [Yes, Yes],
            P::Own,
            D::Shadows,
            &[
                (".agent/skills", M::Default),
                (".codex/skills", M::Default),
                (".config/opencode/skills", M::Default),
                (".pi/agent/skills", M::Unknown),
            ],
        ),
        (
            "stakpak",
            "addendum: Stakpak",
            [No, No],
            [No, No],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        // round 20 ticket 06 rows: brief § Part 2 "Recommended Part 1 additions"
        (
            "grok_build",
            "Grok Build",
            [Yes, Yes],
            [Yes, Yes],
            P::Own,
            D::Shadows,
            &[(".cursor/skills", M::Default)],
        ),
        (
            "deep_agents_code",
            "Deep Agents Code",
            [Yes, Yes],
            [Yes, Yes],
            P::Shared,
            D::Shadows,
            &[],
        ),
        (
            "reasonix",
            "Reasonix",
            [Yes, Yes],
            [Yes, Yes],
            P::Unknown,
            D::Unknown,
            &[(".agent/skills", M::Default)],
        ),
        (
            "cortex_code",
            "Cortex Code",
            [No, Yes],
            [No, Yes],
            P::Both,
            D::FlagsConflict,
            &[],
        ),
        (
            "rovo_dev",
            "Rovo Dev",
            [Yes, No],
            [Yes, No],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "ibm_bob",
            "IBM Bob",
            [No, No],
            [No, No],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "forgecode",
            "ForgeCode",
            [Yes, No],
            [No, No],
            P::Shared,
            D::Shadows,
            &[],
        ),
        (
            "firebender",
            "Firebender",
            [Yes, Yes],
            [Yes, Yes],
            P::Unknown,
            D::ShowsAll,
            &[
                (".codex/skills", M::Default),
                (".cursor/skills", M::Default),
                (".goose/skills", M::Default),
            ],
        ),
        (
            "minimax_code",
            "MiniMax Code",
            [Yes, Yes],
            [Yes, Yes],
            P::Shared,
            D::Shadows,
            &[(".codex/skills", M::Default)],
        ),
        (
            "posit_assistant",
            "Posit Assistant",
            [Yes, No],
            [Yes, No],
            P::Own,
            D::Shadows,
            &[],
        ),
        (
            "aider_desk",
            "AiderDesk",
            [No, No],
            [No, No],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "autohand_code",
            "Autohand Code CLI",
            [Yes, Yes],
            [Yes, Yes],
            P::Own,
            D::Shadows,
            &[(".codex/skills", M::Default), (".agent/skills", M::Default)],
        ),
    ];
    let keys: Vec<&str> = default_tool_adapters().iter().map(|a| a.key()).collect();
    assert_eq!(
        expected.iter().map(|row| row.0).collect::<Vec<_>>(),
        keys,
        "the snapshot covers every registry row, in registry order"
    );
    for (key, brief_row, global, project, precedence, duplicates, also_reads) in expected {
        let facts = adapter_by_key(key).unwrap().reads;
        let at = format!("{key} (brief row: {brief_row})");
        assert_eq!(
            [facts.global.agents, facts.global.claude],
            *global,
            "{at}: global reads"
        );
        assert_eq!(
            [facts.project.agents, facts.project.claude],
            *project,
            "{at}: project reads"
        );
        assert_eq!(facts.precedence, *precedence, "{at}: precedence");
        assert_eq!(facts.duplicates, *duplicates, "{at}: duplicate behaviour");
        let actual: Vec<(&str, M)> = facts
            .also_reads
            .iter()
            .map(|a| {
                assert_eq!(
                    a.dir.root,
                    HostRoot::Home,
                    "{at}: also_reads are home-rooted today"
                );
                assert!(a.dir.overrides.is_empty(), "{at}: no per-OS also_reads yet");
                (a.dir.rel, a.mode)
            })
            .collect();
        assert_eq!(actual, also_reads.to_vec(), "{at}: also_reads");
    }
}

/// Facts vs `group` (spec Q17): the `group` literal is the deployment
/// decision, but a harness absorbed into the AGENTS group must not be one
/// the brief says ignores `<project>/.agents/skills`.
#[test]
fn no_agents_group_member_ignores_the_project_agents_folder() {
    use crate::core::tool_adapters::reads::SharedRead;
    let members: Vec<&ToolAdapter> = constituents_of(VirtualGroup::AgentsStandard).collect();
    assert!(!members.is_empty());
    for member in members {
        assert_ne!(
            member.reads.project.agents,
            SharedRead::No,
            "{} is an AGENTS-group member but does not read <project>/.agents/skills",
            member.key()
        );
    }
}

// ---------------------------------------------------------------------------
// round 20 ticket 05 rows: the reader-only harnesses
// ---------------------------------------------------------------------------

fn reader(key: &str) -> &'static crate::core::tool_adapters::ReaderOnlyHarness {
    reader_only_harnesses()
        .into_iter()
        .find(|reader| reader.key == key)
        .unwrap_or_else(|| panic!("reader {key}"))
}

/// The brief-snapshot table for the reader-only rows: each row's group,
/// default detect dirs and read facts, restated from brief § Part 2
/// "Recommended Part 1 additions" or the addendum's § Shared-dir reads.
/// LIKELY and UNKNOWN cells are `Unknown`; precedence the brief calls "n/a"
/// (a harness whose only user dir is the shared one) is `Unknown` too.
#[test]
fn every_reader_only_row_restates_its_brief_read_facts() {
    use crate::core::tool_adapters::reads::{
        DuplicateBehaviour as D, Precedence as P, ReadMode as M, SharedRead, SharedRead::*,
    };
    type Row = (
        &'static str,            // registry key
        &'static str,            // brief row
        &'static [&'static str], // detect dirs (default root, home-relative)
        [SharedRead; 2],         // global: ~/.agents, ~/.claude
        [SharedRead; 2],         // project: .agents, .claude
        P,
        D,
        &'static [(&'static str, M)], // also_reads (home-relative)
    );
    const U: SharedRead = Unknown;
    let expected: &[Row] = &[
        (
            "warp",
            "Warp (AGENTS)",
            &[".warp", ".config/warp-terminal"],
            [Yes, Yes],
            [Yes, Yes],
            P::Both,
            D::ShowsAll,
            &[
                (".warp/skills", M::Default),
                (".codex/skills", M::Default),
                (".cursor/skills", M::Default),
                (".gemini/skills", M::Default),
                (".copilot/skills", M::Default),
                (".factory/skills", M::Default),
                (".github/skills", M::Default),
                (".opencode/skills", M::Default),
            ],
        ),
        (
            "zed",
            "Zed (AGENTS)",
            &[".config/zed"],
            [Yes, No],
            [Yes, No],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "dexto",
            "Dexto (AGENTS)",
            &[".dexto"],
            [Yes, No],
            [Yes, No],
            P::Unknown,
            D::Unknown,
            &[(".dexto/skills", M::Default)],
        ),
        (
            "fx",
            "fx (AGENTS)",
            &[".fx"],
            [Yes, Yes],
            [Yes, Yes],
            P::Both,
            D::LoadsNone,
            &[
                (".fx/skills", M::Default),
                (".config/opencode/skills", M::Default),
                (".codex/skills", M::Default),
                (".claw/skills", M::Default),
            ],
        ),
        (
            "sarvam_code",
            "Sarvam Code (AGENTS, LIKELY)",
            &[".sarvam"],
            [U, U],
            [U, U],
            P::Unknown,
            D::Unknown,
            &[],
        ),
        (
            "vt_code",
            "VT Code (extra); addendum VT Code",
            &[".config/vtcode", ".vtcode"],
            [Yes, No],
            [Yes, No],
            P::Shared,
            D::Shadows,
            &[(".vtcode/skills", M::Default)],
        ),
        (
            "dimcode",
            "addendum DimCode",
            &[".dimcode"],
            [Yes, No],
            [Yes, No],
            P::Own,
            D::Shadows,
            &[(".dimcode/v2/skills", M::Default)],
        ),
        (
            "crow_cli",
            "addendum crow-cli",
            &[".agents/crow"],
            [Yes, No],
            [Yes, Yes],
            P::Unknown,
            D::Shadows,
            &[],
        ),
        (
            "corust_agent",
            "addendum Corust Agent",
            &[".corust-agent"],
            [Yes, U],
            [Yes, U],
            P::Unknown,
            D::Shadows,
            &[(".corust-agent/skills", M::Default)],
        ),
    ];
    let keys: Vec<&str> = reader_only_harnesses().iter().map(|r| r.key).collect();
    assert_eq!(
        expected.iter().map(|row| row.0).collect::<Vec<_>>(),
        keys,
        "the snapshot covers every reader-only row, in registry order"
    );
    for (key, brief_row, detect, global, project, precedence, duplicates, also_reads) in expected {
        let row = reader(key);
        let at = format!("{key} (brief row: {brief_row})");
        assert_eq!(row.group, VirtualGroup::AgentsStandard, "{at}: group");
        assert_eq!(
            row.detect_dirs.iter().map(|d| d.rel).collect::<Vec<_>>(),
            detect.to_vec(),
            "{at}: detect dirs"
        );
        let facts = row.reads;
        assert_eq!(
            [facts.global.agents, facts.global.claude],
            *global,
            "{at}: global reads"
        );
        assert_eq!(
            [facts.project.agents, facts.project.claude],
            *project,
            "{at}: project reads"
        );
        assert_eq!(facts.precedence, *precedence, "{at}: precedence");
        assert_eq!(facts.duplicates, *duplicates, "{at}: duplicate behaviour");
        let actual: Vec<(&str, M)> = facts
            .also_reads
            .iter()
            .map(|a| {
                assert_eq!(
                    a.dir.root,
                    HostRoot::Home,
                    "{at}: also_reads are home-rooted"
                );
                assert!(a.dir.overrides.is_empty(), "{at}: no per-OS also_reads");
                (a.dir.rel, a.mode)
            })
            .collect();
        assert_eq!(actual, also_reads.to_vec(), "{at}: also_reads");
    }
}

/// Facts vs `group` for reader-only members too (spec Q17): a harness absorbed
/// into the AGENTS group's project roster must not ignore
/// `<project>/.agents/skills`.
#[test]
fn no_reader_only_member_ignores_the_project_agents_folder() {
    use crate::core::tool_adapters::reads::SharedRead;
    for reader in reader_only_harnesses() {
        assert_ne!(
            reader.reads.project.agents,
            SharedRead::No,
            "{} is a reader-only AGENTS member but does not read <project>/.agents/skills",
            reader.key
        );
    }
}

/// A reader-only harness is detected by its own config root, never by the
/// shared folder it reads (`~/.agents`, `~/.agents/skills`, `~/.claude`):
/// that folder's presence says nothing about the harness. crow-cli's root
/// `~/.agents/crow` is its own folder inside `~/.agents`.
#[test]
fn a_reader_only_harness_is_detected_by_its_own_root_never_a_shared_folder() {
    let shared = [".agents", ".agents/skills", ".claude", ".claude/skills"];
    for reader in reader_only_harnesses() {
        assert!(
            !reader.detect_dirs.is_empty(),
            "{} has no detect dir",
            reader.key
        );
        for dir in reader.detect_dirs {
            for os in [HostOs::MacOs, HostOs::Linux, HostOs::Windows] {
                let (root, rel) = dir.effective(os);
                assert!(
                    !(root == HostRoot::Home && shared.contains(&rel)),
                    "{}: detected by the shared folder {rel} on {os:?}",
                    reader.key
                );
            }
        }
    }
}

/// Zed's config root on Windows is `%APPDATA%\Zed` (brief I7), the roaming
/// root's `Zed`; elsewhere `~/.config/zed`. Its skills folder stays
/// `~/.agents/skills` everywhere — it has no folder of its own.
#[test]
fn zeds_windows_detect_root_is_under_roaming_app_data() {
    let base = tempfile::tempdir().unwrap();
    let roots = distinct_roots(base.path());
    let zed = reader("zed");
    assert_eq!(zed.detect_dirs.len(), 1);
    assert_eq!(
        zed.detect_dirs[0].resolve_for(HostOs::Windows, &roots),
        base.path().join("roaming").join("Zed")
    );
    for elsewhere in [HostOs::MacOs, HostOs::Linux] {
        assert_eq!(
            zed.detect_dirs[0].resolve_for(elsewhere, &roots),
            base.path().join("home").join(".config").join("zed"),
            "{elsewhere:?}"
        );
    }
}

/// Zed installed on this host — whatever the OS — marks the `.agents/skills`
/// entry installed through its effective detect root.
#[test]
fn a_detected_zed_is_found_under_its_effective_root_on_this_host() {
    let base = tempfile::tempdir().unwrap();
    let roots = distinct_roots(base.path());
    let zed = reader("zed");
    assert!(!crate::core::tool_adapters::reader_installed_in(
        &roots, zed
    ));
    crate::core::tool_adapters::mark_reader_installed_in(&roots, zed);
    assert!(crate::core::tool_adapters::reader_installed_in(&roots, zed));
    let expected = if cfg!(windows) {
        base.path().join("roaming").join("Zed")
    } else {
        base.path().join("home").join(".config").join("zed")
    };
    assert!(expected.is_dir(), "created at {expected:?}");
    assert!(is_installed_in(
        &roots,
        adapter_by_key("agents_skills").unwrap()
    ));
}

/// Warp's documented config root per OS (brief T80): `~/.warp` on macOS,
/// `~/.config/warp-terminal` on Linux, `%LOCALAPPDATA%\warp\Warp` on
/// Windows — with `~/.warp` (MCP config, bundled skills) on every OS.
#[test]
fn warps_detect_roots_follow_its_documented_layout() {
    let base = tempfile::tempdir().unwrap();
    let roots = distinct_roots(base.path());
    let home = base.path().join("home");
    let resolved = |os: HostOs| {
        reader("warp")
            .detect_dirs
            .iter()
            .map(|d| d.resolve_for(os, &roots))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        resolved(HostOs::Windows),
        vec![
            home.join(".warp"),
            base.path().join("local").join("warp").join("Warp")
        ]
    );
    assert_eq!(
        resolved(HostOs::Linux),
        vec![
            home.join(".warp"),
            home.join(".config").join("warp-terminal")
        ]
    );
    assert_eq!(resolved(HostOs::MacOs)[0], home.join(".warp"));
}

/// VT Code's config dir per OS (`VtCodePaths`, addendum P12) and its legacy
/// home `~/.vtcode`.
#[test]
fn vt_codes_detect_roots_follow_its_platform_config_dir() {
    let base = tempfile::tempdir().unwrap();
    let roots = distinct_roots(base.path());
    let home = base.path().join("home");
    let resolved = |os: HostOs| {
        reader("vt_code")
            .detect_dirs
            .iter()
            .map(|d| d.resolve_for(os, &roots))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        resolved(HostOs::MacOs),
        vec![
            base.path().join("local").join("com.vinhnx.vtcode"),
            home.join(".vtcode")
        ]
    );
    assert_eq!(
        resolved(HostOs::Windows),
        vec![
            base.path().join("roaming").join("vinhnx").join("vtcode"),
            home.join(".vtcode")
        ]
    );
    assert_eq!(
        resolved(HostOs::Linux),
        vec![home.join(".config").join("vtcode"), home.join(".vtcode")]
    );
}

/// Codewhale's legacy `~/.deepseek` is a read fallback, too generic to mean
/// Codewhale (B02 I13): it never detects the row.
#[test]
fn codewhale_is_never_detected_by_the_legacy_deepseek_root() {
    let home = tempfile::tempdir().unwrap();
    let roots = HostRoots::under_home(home.path());
    let codewhale = adapter_by_key("codewhale").unwrap();
    fs::create_dir_all(home.path().join(".deepseek/skills/x")).unwrap();
    fs::write(home.path().join(".deepseek/config.toml"), b"").unwrap();
    assert!(!is_installed_in(&roots, codewhale));
    mark_installed_in(&roots, codewhale);
    assert!(is_installed_in(&roots, codewhale));
}

/// A reader-only harness is installedness-checked like an adapter: a detect
/// dir holding nothing but the path down to one of the harness's own (or
/// legacy) skills folders is what a skill deployer leaves behind, not the
/// harness (round 20 rows review, P1). Each such reader is probed alone in its
/// own home, with only `<skills folder>/x` on disk: neither the reader nor the
/// AGENTS group it reads may count as installed.
#[test]
fn a_skills_only_footprint_never_detects_a_reader_only_harness() {
    use crate::core::tool_adapters::reader_installed_in;
    let footprints = [
        ("warp", ".warp/skills"),
        ("dexto", ".dexto/skills"),
        ("fx", ".fx/skills"),
        ("vt_code", ".vtcode/skills"),
        ("dimcode", ".dimcode/v2/skills"),
        ("corust_agent", ".corust-agent/skills"),
    ];
    let agents = adapter_by_key(ToolId::AgentsStandard.as_key()).unwrap();
    for (key, skills) in footprints {
        let home = tempfile::tempdir().unwrap();
        let roots = HostRoots::under_home(home.path());
        fs::create_dir_all(home.path().join(skills).join("x")).unwrap();
        let reader = reader_only_harnesses()
            .into_iter()
            .find(|r| r.key == key)
            .unwrap_or_else(|| panic!("{key}"));
        assert!(
            !reader_installed_in(&roots, reader),
            "{key} detected by a skills-only footprint"
        );
        assert!(
            !is_installed_in(&roots, agents),
            "{key}'s footprint made the AGENTS group installed"
        );
        // Anything beside the skills path is the harness itself.
        fs::write(
            home.path().join(skills).parent().unwrap().join("config"),
            b"",
        )
        .unwrap();
        assert!(
            reader_installed_in(&roots, reader),
            "{key} with a config beside its skills"
        );
    }
}
