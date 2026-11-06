use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::core::environment::{HostOs, HostRoot, HostRoots};
use crate::core::errors::SignalError;
use crate::core::skill_discovery::{manifest_validity, InvalidManifestReason};

pub mod catalog;

pub use catalog::{global_tool_entries, installed_keys, project_tool_entries, ToolCatalogEntry};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolId {
    AgentsStandard,
    Cursor,
    ClaudeCode,
    Codex,
    OpenCode,
    Antigravity,
    Amp,
    KimiCli,
    Augment,
    OpenClaw,
    Copaw,
    Cline,
    CodeBuddy,
    CommandCode,
    Continue,
    Crush,
    Junie,
    IflowCli,
    KiroCli,
    Kode,
    McpJam,
    MistralVibe,
    Mux,
    OpenClaude,
    OpenHands,
    Pi,
    Qoder,
    QoderWork,
    QwenCode,
    Trae,
    TraeCn,
    Zencoder,
    Neovate,
    Pochi,
    AdaL,
    KiloCode,
    RooCode,
    Goose,
    GeminiCli,
    GithubCopilot,
    Clawdbot,
    Droid,
    Windsurf,
    Moltbot,
    HermesAgent,
}

impl ToolId {
    pub fn as_key(&self) -> &'static str {
        match self {
            ToolId::AgentsStandard => "agents_skills",
            ToolId::Cursor => "cursor",
            ToolId::ClaudeCode => "claude_code",
            ToolId::Codex => "codex",
            ToolId::OpenCode => "opencode",
            ToolId::Antigravity => "antigravity",
            ToolId::Amp => "amp",
            ToolId::KimiCli => "kimi_cli",
            ToolId::Augment => "augment",
            ToolId::OpenClaw => "openclaw",
            ToolId::Copaw => "copaw",
            ToolId::Cline => "cline",
            ToolId::CodeBuddy => "codebuddy",
            ToolId::CommandCode => "command_code",
            ToolId::Continue => "continue",
            ToolId::Crush => "crush",
            ToolId::Junie => "junie",
            ToolId::IflowCli => "iflow_cli",
            ToolId::KiroCli => "kiro_cli",
            ToolId::Kode => "kode",
            ToolId::McpJam => "mcpjam",
            ToolId::MistralVibe => "mistral_vibe",
            ToolId::Mux => "mux",
            ToolId::OpenClaude => "openclaude",
            ToolId::OpenHands => "openhands",
            ToolId::Pi => "pi",
            ToolId::Qoder => "qoder",
            ToolId::QoderWork => "qoderwork",
            ToolId::QwenCode => "qwen_code",
            ToolId::Trae => "trae",
            ToolId::TraeCn => "trae_cn",
            ToolId::Zencoder => "zencoder",
            ToolId::Neovate => "neovate",
            ToolId::Pochi => "pochi",
            ToolId::AdaL => "adal",
            ToolId::KiloCode => "kilo_code",
            ToolId::RooCode => "roo_code",
            ToolId::Goose => "goose",
            ToolId::GeminiCli => "gemini_cli",
            ToolId::GithubCopilot => "github_copilot",
            ToolId::Clawdbot => "clawdbot",
            ToolId::Droid => "droid",
            ToolId::Windsurf => "windsurf",
            ToolId::Moltbot => "moltbot",
            ToolId::HermesAgent => "hermes-agent",
        }
    }
}

/// A tool entry that stands in for several tools sharing one project-scope
/// skills convention. The group has its own `ToolAdapter` (its entry id) and
/// absorbs every adapter whose `group` names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtualGroup {
    /// The AGENTS standard: every tool reading `<project>/.agents/skills`.
    AgentsStandard,
}

impl VirtualGroup {
    pub const ALL: &'static [VirtualGroup] = &[VirtualGroup::AgentsStandard];

    /// The adapter that represents this group in tool lists.
    pub fn entry_id(&self) -> ToolId {
        match self {
            VirtualGroup::AgentsStandard => ToolId::AgentsStandard,
        }
    }
}

/// Every fact Skills Hub knows about one Tool. The registry (`TOOL_ADAPTERS`)
/// is the single source of truth: adding a tool is one literal here plus a
/// README row.
#[derive(Clone, Debug)]
pub struct ToolAdapter {
    pub id: ToolId,
    /// The label every scope uses. A virtual group entry's project-scope
    /// label (`.agents/skills (N harnesses)`) is composed by the frontend
    /// from its derived `constituents`, never written here.
    pub display_name: &'static str,
    /// Global skill directory (aligned with add-skill docs), under its typed
    /// root (home unless the vendor documents otherwise).
    pub skills_dir: RegistryPath,
    /// Global skill directories this tool was registered with before
    /// `skills_dir` was corrected (Augment: `.augment/rules`, which
    /// is its rules dir, not its skills dir). Artifacts Skills Hub created
    /// there are moved by `core::target_relocation` at startup, and they
    /// stay inside the deletion rule (`ensure_path_within_tool_dirs`) so a
    /// row still pointing there can always be cleaned up. Never scanned,
    /// never synced into. Empty for almost every tool.
    pub former_skills_dirs: &'static [RegistryPath],
    /// Directories whose presence marks the tool as installed —
    /// the tool's own configuration root(s), any one of which counts. More
    /// than one entry when a tool moved its root across versions (Kimi:
    /// `.kimi-code` today, `.kimi` before). Never a shared skills convention
    /// dir (`.config/agents`): those are footprints, not tools.
    pub detect_dirs: &'static [RegistryPath],
    /// Project-scope skill directory relative to a project root. Differs
    /// from the global dir for many tools (e.g. Pi, Windsurf) and is the
    /// only mapping project sync may use. `None` for a harness that reads no
    /// project skills dir: it is absent from the project catalog and no
    /// project-scope operation admits it ([`project_adapter_by_key`]).
    pub project_skills_dir: Option<&'static str>,
    /// The virtual group this tool is absorbed into at project scope;
    /// `None` for standalone tools and for group entries themselves.
    pub group: Option<VirtualGroup>,
    /// Whether the tool can consume a symlinked/junctioned skills dir.
    /// `false` forces copy mode in every sync path. Every current entry is
    /// `true`; the capability stays as the lever for a tool that regresses
    /// (Cursor was `false` until IDE 2.5 / CLI 2026-06 fixed discovery).
    pub supports_symlink: bool,
}

impl ToolAdapter {
    pub fn key(&self) -> &'static str {
        self.id.as_key()
    }

    /// `Some` when this adapter is the entry of a virtual group. Only the
    /// project catalog branches on this: globally a group entry is just
    /// another target, so no path there asks the question.
    pub fn as_virtual_group(&self) -> Option<VirtualGroup> {
        VirtualGroup::ALL
            .iter()
            .copied()
            .find(|g| g.entry_id() == self.id)
    }
}

/// One registry path: `rel` (written with `/`) under a typed host root, plus
/// the per-OS overrides a vendor documents (Reasonix keeps its skills under
/// `%APPDATA%` on Windows). Resolution happens only here, against explicit
/// [`HostRoots`], so a row can never be joined onto the wrong root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegistryPath {
    pub root: HostRoot,
    pub rel: &'static str,
    /// Replaces `root` + `rel` on the named OS; empty for almost every path.
    pub overrides: &'static [OsOverride],
}

/// A registry path's location on one OS, where it differs from the default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OsOverride {
    pub os: HostOs,
    pub root: HostRoot,
    pub rel: &'static str,
}

impl RegistryPath {
    /// A path under the operator's home on every OS.
    pub const fn home(rel: &'static str) -> RegistryPath {
        RegistryPath {
            root: HostRoot::Home,
            rel,
            overrides: &[],
        }
    }

    /// This path with per-OS `overrides` (at most one per OS; the first wins).
    #[cfg_attr(not(test), allow(dead_code, reason = "first override: ticket 06"))]
    pub const fn overridden(self, overrides: &'static [OsOverride]) -> RegistryPath {
        RegistryPath { overrides, ..self }
    }

    /// The root and root-relative path in effect on `os`.
    pub fn effective(&self, os: HostOs) -> (HostRoot, &'static str) {
        self.overrides
            .iter()
            .find(|o| o.os == os)
            .map_or((self.root, self.rel), |o| (o.root, o.rel))
    }

    /// The absolute path on `os` under `roots`.
    pub fn resolve_for(&self, os: HostOs, roots: &HostRoots) -> PathBuf {
        let (root, rel) = self.effective(os);
        join_relative(roots.root(root), rel)
    }

    /// The absolute path on this host under `roots`.
    pub fn resolve(&self, roots: &HostRoots) -> PathBuf {
        self.resolve_for(HostOs::CURRENT, roots)
    }

    /// Whether `self` and `other` name the same directory on this host.
    pub fn same_location(&self, other: &RegistryPath) -> bool {
        self.effective(HostOs::CURRENT) == other.effective(HostOs::CURRENT)
    }

    /// The path as the Harnesses pane shows it: under home it reads
    /// `~/<rel>` (always with `/`), elsewhere it is the absolute path.
    pub fn display(&self, roots: &HostRoots) -> String {
        let path = self.resolve(roots);
        match path.strip_prefix(&roots.home) {
            Ok(under_home) => {
                let parts: Vec<String> = under_home
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect();
                format!("~/{}", parts.join("/"))
            }
            Err(_) => path.to_string_lossy().into_owned(),
        }
    }
}

/// Registry-literal shorthand: a home-rooted path with no overrides.
const fn home(rel: &'static str) -> RegistryPath {
    RegistryPath::home(rel)
}

#[derive(Clone, Debug)]
pub struct DetectedSkill {
    pub tool: ToolId,
    pub name: String,
    pub path: PathBuf,
    pub is_link: bool,
    pub link_target: Option<PathBuf>,
}

static TOOL_ADAPTERS: &[ToolAdapter] = &[
    ToolAdapter {
        id: ToolId::AgentsStandard,
        display_name: ".agents/skills",
        skills_dir: home(".agents/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".agents")],
        project_skills_dir: Some(".agents/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Cursor,
        display_name: "Cursor",
        skills_dir: home(".cursor/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".cursor")],
        project_skills_dir: Some(".agents/skills"),
        group: Some(VirtualGroup::AgentsStandard),
        // Copy-only until Cursor IDE 2.5 (Feb 2026) fixed symlink discovery
        // under ~/.cursor/skills; flipped in v-next ticket 38.
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::ClaudeCode,
        display_name: "Claude Code",
        skills_dir: home(".claude/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".claude")],
        project_skills_dir: Some(".claude/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Codex,
        display_name: "Codex",
        skills_dir: home(".codex/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".codex")],
        project_skills_dir: Some(".agents/skills"),
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::OpenCode,
        display_name: "OpenCode",
        // add-skill global path: ~/.config/opencode/skills/
        skills_dir: home(".config/opencode/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".config/opencode")],
        project_skills_dir: Some(".agents/skills"),
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Antigravity,
        display_name: "Antigravity",
        // add-skill global path: ~/.gemini/antigravity/global_skills/
        skills_dir: home(".gemini/antigravity/global_skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".gemini/antigravity")],
        project_skills_dir: Some(".agents/skills"),
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Amp,
        display_name: "Amp",
        // add-skill global path: ~/.config/agents/skills/; the tool itself
        // lives in ~/.config/amp (settings.json — ampcode.com/docs/cli/settings).
        skills_dir: home(".config/agents/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".config/amp")],
        project_skills_dir: Some(".agents/skills"),
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::KimiCli,
        display_name: "Kimi Code CLI",
        // add-skill global path: ~/.config/agents/skills/ (shared with Amp).
        // The tool's config root is ~/.kimi-code (kimi.com/code docs) or
        // ~/.kimi on earlier releases (kimi-cli.com docs); either counts.
        // `KIMI_CODE_HOME` relocates the root; env-var homes are not honoured
        // (a Finder-launched app does not see shell env).
        skills_dir: home(".config/agents/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".kimi-code"), home(".kimi")],
        project_skills_dir: Some(".agents/skills"),
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Augment,
        display_name: "Augment",
        // Global path ~/.augment/skills/ — docs.augmentcode.com/cli/skills
        // ("Skill Locations") and add-skill's README agree. ~/.augment/rules
        // is Augment's *rules* dir; Skills Hub synced there until 1.2.18.
        skills_dir: home(".augment/skills"),
        former_skills_dirs: &[home(".augment/rules")],
        detect_dirs: &[home(".augment")],
        project_skills_dir: Some(".augment/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::OpenClaw,
        display_name: "OpenClaw",
        // add-skill global path: ~/.openclaw/skills/
        skills_dir: home(".openclaw/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".openclaw")],
        project_skills_dir: Some("skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Copaw,
        // Copaw was renamed QwenPaw (relabel only; the key stays `copaw`).
        // Its root is now `~/.qwenpaw` unless a legacy `~/.copaw` exists;
        // that detect dir waits for its folder correction in #108.
        display_name: "QwenPaw",
        // add-skill global path: ~/.copaw/skill_pool/
        skills_dir: home(".copaw/skill_pool"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".copaw")],
        project_skills_dir: Some(".copaw/skill_pool"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Cline,
        display_name: "Cline",
        // add-skill global path: ~/.cline/skills/
        skills_dir: home(".cline/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".cline")],
        project_skills_dir: Some(".agents/skills"),
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::CodeBuddy,
        display_name: "CodeBuddy",
        // add-skill global path: ~/.codebuddy/skills/
        skills_dir: home(".codebuddy/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".codebuddy")],
        project_skills_dir: Some(".codebuddy/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::CommandCode,
        display_name: "Command Code",
        // add-skill global path: ~/.commandcode/skills/
        skills_dir: home(".commandcode/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".commandcode")],
        project_skills_dir: Some(".commandcode/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Continue,
        display_name: "Continue",
        // add-skill global path: ~/.continue/skills/
        skills_dir: home(".continue/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".continue")],
        project_skills_dir: Some(".continue/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Crush,
        display_name: "Crush",
        // add-skill global path: ~/.config/crush/skills/
        skills_dir: home(".config/crush/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".config/crush")],
        project_skills_dir: Some(".crush/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Junie,
        display_name: "Junie",
        // add-skill global path: ~/.junie/skills/
        skills_dir: home(".junie/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".junie")],
        project_skills_dir: Some(".junie/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::IflowCli,
        display_name: "iFlow CLI",
        // add-skill global path: ~/.iflow/skills/
        skills_dir: home(".iflow/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".iflow")],
        project_skills_dir: Some(".iflow/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::KiroCli,
        display_name: "Kiro CLI",
        // add-skill global path: ~/.kiro/skills/
        skills_dir: home(".kiro/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".kiro")],
        project_skills_dir: Some(".kiro/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Kode,
        display_name: "Kode",
        // add-skill global path: ~/.kode/skills/
        skills_dir: home(".kode/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".kode")],
        project_skills_dir: Some(".kode/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::McpJam,
        display_name: "MCPJam",
        // add-skill global path: ~/.mcpjam/skills/
        skills_dir: home(".mcpjam/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".mcpjam")],
        project_skills_dir: Some(".mcpjam/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::MistralVibe,
        display_name: "Mistral Vibe",
        // add-skill global path: ~/.vibe/skills/
        skills_dir: home(".vibe/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".vibe")],
        project_skills_dir: Some(".vibe/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Mux,
        // Mux is now Xum (relabel only; the key stays `mux`, round 19).
        display_name: "Xum",
        // add-skill global path: ~/.mux/skills/
        skills_dir: home(".mux/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".mux")],
        project_skills_dir: Some(".mux/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::OpenClaude,
        display_name: "OpenClaude IDE",
        // add-skill global path: ~/.openclaude/skills/
        skills_dir: home(".openclaude/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".openclaude")],
        project_skills_dir: Some(".openclaude/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::OpenHands,
        display_name: "OpenHands",
        // add-skill global path: ~/.openhands/skills/
        skills_dir: home(".openhands/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".openhands")],
        project_skills_dir: Some(".openhands/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Pi,
        display_name: "Pi",
        // add-skill global path: ~/.pi/agent/skills/
        skills_dir: home(".pi/agent/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".pi")],
        project_skills_dir: Some(".pi/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Qoder,
        display_name: "Qoder",
        // add-skill global path: ~/.qoder/skills/
        skills_dir: home(".qoder/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".qoder")],
        project_skills_dir: Some(".qoder/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::QoderWork,
        display_name: "QoderWork",
        // add-skill global path: ~/.qoderwork/skills/
        skills_dir: home(".qoderwork/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".qoderwork")],
        project_skills_dir: Some(".qoderwork/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::QwenCode,
        display_name: "Qwen Code",
        // add-skill global path: ~/.qwen/skills/
        skills_dir: home(".qwen/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".qwen")],
        project_skills_dir: Some(".qwen/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Trae,
        display_name: "Trae",
        // add-skill global path: ~/.trae/skills/
        skills_dir: home(".trae/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".trae")],
        project_skills_dir: Some(".trae/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::TraeCn,
        display_name: "Trae CN",
        // add-skill global path: ~/.trae-cn/skills/
        skills_dir: home(".trae-cn/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".trae-cn")],
        project_skills_dir: Some(".trae/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Zencoder,
        // Zenflow is Zencoder's desktop orchestrator and shares its skills
        // (an alias, not a row).
        display_name: "Zencoder / Zenflow",
        // add-skill global path: ~/.zencoder/skills/
        skills_dir: home(".zencoder/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".zencoder")],
        project_skills_dir: Some(".zencoder/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Neovate,
        display_name: "Neovate",
        // add-skill global path: ~/.neovate/skills/
        skills_dir: home(".neovate/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".neovate")],
        project_skills_dir: Some(".neovate/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Pochi,
        display_name: "Pochi",
        // add-skill global path: ~/.pochi/skills/
        skills_dir: home(".pochi/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".pochi")],
        project_skills_dir: Some(".pochi/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::AdaL,
        display_name: "AdaL",
        // add-skill global path: ~/.adal/skills/
        skills_dir: home(".adal/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".adal")],
        project_skills_dir: Some(".adal/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::KiloCode,
        display_name: "Kilo Code",
        // add-skill global path: ~/.kilocode/skills/
        skills_dir: home(".kilocode/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".kilocode")],
        project_skills_dir: Some(".kilocode/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::RooCode,
        display_name: "Roo Code",
        // add-skill global path: ~/.roo/skills/
        skills_dir: home(".roo/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".roo")],
        project_skills_dir: Some(".roo/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Goose,
        display_name: "Goose",
        // add-skill global path: ~/.config/goose/skills/
        skills_dir: home(".config/goose/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".config/goose")],
        project_skills_dir: Some(".goose/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::GeminiCli,
        display_name: "Gemini CLI",
        // add-skill global path: ~/.gemini/skills/
        skills_dir: home(".gemini/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".gemini")],
        project_skills_dir: Some(".agents/skills"),
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::GithubCopilot,
        display_name: "GitHub Copilot",
        // add-skill global path: ~/.copilot/skills/
        skills_dir: home(".copilot/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".copilot")],
        project_skills_dir: Some(".agents/skills"),
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Clawdbot,
        display_name: "Clawdbot",
        // add-skill global path: ~/.clawdbot/skills/
        skills_dir: home(".clawdbot/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".clawdbot")],
        project_skills_dir: Some(".clawdbot/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Droid,
        display_name: "Droid",
        // add-skill global path: ~/.factory/skills/
        skills_dir: home(".factory/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".factory")],
        project_skills_dir: Some(".factory/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Windsurf,
        // Windsurf is now Devin (relabel only; the key stays `windsurf`).
        // Devin's `.config/devin` detect dir waits for #108.
        display_name: "Devin (Windsurf)",
        // add-skill global path: ~/.codeium/windsurf/skills/
        skills_dir: home(".codeium/windsurf/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".codeium/windsurf")],
        project_skills_dir: Some(".windsurf/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Moltbot,
        display_name: "MoltBot",
        // add-skill global path: ~/.moltbot/skills/
        skills_dir: home(".moltbot/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".moltbot")],
        project_skills_dir: Some(".moltbot/skills"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::HermesAgent,
        display_name: "Hermes Agent",
        skills_dir: home(".hermes/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".hermes")],
        project_skills_dir: Some(".hermes/skills"),
        group: None,
        supports_symlink: true,
    },
];

/// A **reader-only harness** (CONTEXT term): a harness Skills Hub detects
/// and lists but never deploys to. It reads a virtual group's shared folder —
/// `~/.agents/skills` globally, `<project>/.agents/skills` at project scope —
/// and has no skills folder of its own. It lives in its own table, apart from
/// `TOOL_ADAPTERS`, so no target path can reach it: `adapter_by_key` never
/// answers its key, so it is never selectable, never saved in a selection,
/// never in the installed set the new-harness ask diffs, and mints no row.
/// The catalogs list it as a reader of its group's entry
/// ([`ToolCatalogEntry::readers`]) and, at project scope, absorb it into the
/// group's roster.
#[derive(Clone, Debug)]
pub struct ReaderOnlyHarness {
    /// Stable key, unique across both registry tables. Never stored.
    pub key: &'static str,
    pub display_name: &'static str,
    /// Directories any one of whose presence marks the harness installed.
    pub detect_dirs: &'static [RegistryPath],
    /// The group whose shared folder it reads.
    pub reads: VirtualGroup,
}

/// The reader-only harnesses, in presentation order. Empty until the first
/// real row lands (round 20 ticket 05); tests add rows per thread
/// (`test_overrides::sample_reader`).
static READER_ONLY_HARNESSES: &[ReaderOnlyHarness] = &[];

/// Every reader-only harness, in registry order (plus, in tests, the calling
/// thread's test-only rows).
pub fn reader_only_harnesses() -> Vec<&'static ReaderOnlyHarness> {
    #[allow(unused_mut, reason = "only tests append rows")]
    let mut rows: Vec<&'static ReaderOnlyHarness> = READER_ONLY_HARNESSES.iter().collect();
    #[cfg(test)]
    rows.extend(test_overrides::readers());
    rows
}

/// The reader-only harnesses reading `group`'s folder, in registry order.
pub fn readers_of(group: VirtualGroup) -> Vec<&'static ReaderOnlyHarness> {
    reader_only_harnesses()
        .into_iter()
        .filter(|reader| reader.reads == group)
        .collect()
}

/// Whether a reader-only harness is installed: any of its detect dirs exists.
/// It has no skills dir, so there is no skills-only footprint to discount.
pub fn reader_installed_in(roots: &HostRoots, reader: &ReaderOnlyHarness) -> bool {
    reader
        .detect_dirs
        .iter()
        .any(|dir| dir.resolve(roots).exists())
}

/// Test-only: install a reader-only harness the way the harness does (its
/// first detect dir), the reader counterpart of [`mark_installed_in`].
#[cfg(test)]
pub fn mark_reader_installed_in(roots: &HostRoots, reader: &ReaderOnlyHarness) {
    std::fs::create_dir_all(reader.detect_dirs[0].resolve(roots)).expect("create detect dir");
}

/// Every known tool adapter, in registry (presentation) order.
pub fn default_tool_adapters() -> &'static [ToolAdapter] {
    TOOL_ADAPTERS
}

/// The registry rows the catalogs list, in registry order — in tests with
/// each row's thread-local shadow applied (`test_overrides::shadow`), so a
/// catalog test can exercise a shape no shipped row has yet.
#[cfg(not(test))]
fn catalog_rows() -> impl Iterator<Item = &'static ToolAdapter> {
    TOOL_ADAPTERS.iter()
}

#[cfg(test)]
fn catalog_rows() -> impl Iterator<Item = &'static ToolAdapter> {
    TOOL_ADAPTERS
        .iter()
        .map(|adapter| test_overrides::lookup(adapter.key()).unwrap_or(adapter))
}

/// Tools can share the same global skills directory (e.g. Amp and Kimi Code CLI).
/// Use this to coordinate UI warnings and avoid duplicate filesystem operations.
pub fn adapters_sharing_skills_dir(adapter: &ToolAdapter) -> Vec<&'static ToolAdapter> {
    TOOL_ADAPTERS
        .iter()
        .filter(|a| a.skills_dir.same_location(&adapter.skills_dir))
        .collect()
}

/// The adapters absorbed into `group` at project scope, in registry order.
pub fn constituents_of(group: VirtualGroup) -> impl Iterator<Item = &'static ToolAdapter> {
    TOOL_ADAPTERS.iter().filter(move |a| a.group == Some(group))
}

/// The registry record for `key`, borrowed from the `static` registry: every
/// fact about a tool has exactly one instance, so callers cannot mutate a copy.
pub fn adapter_by_key(key: &str) -> Option<&'static ToolAdapter> {
    #[cfg(test)]
    if let Some(adapter) = test_overrides::lookup(key) {
        return Some(adapter);
    }
    TOOL_ADAPTERS
        .iter()
        .find(|adapter| adapter.id.as_key() == key)
}

/// The registry record for `key` at project scope: `None` for an unknown key
/// and for a tool that reads no project skills dir
/// ([`ToolAdapter::project_skills_dir`] is `None`). Every project-scope
/// admission (configure, assign, unassign) asks here, so such a tool can
/// never mint a project row.
pub fn project_adapter_by_key(key: &str) -> Option<&'static ToolAdapter> {
    adapter_by_key(key).filter(|adapter| adapter.project_skills_dir.is_some())
}

/// Test-only registry shadowing, so core tests can exercise a capability no
/// shipped entry carries (e.g. copy-only sync) without a fake registry entry.
/// Overrides are thread-local — each `cargo test` case runs on its own thread
/// — and leaked to `'static` to match the registry's borrow contract.
#[cfg(test)]
pub mod test_overrides {
    use super::{ReaderOnlyHarness, RegistryPath, ToolAdapter, VirtualGroup};
    use std::cell::RefCell;

    thread_local! {
        static OVERRIDES: RefCell<Vec<&'static ToolAdapter>> = const { RefCell::new(Vec::new()) };
        static READERS: RefCell<Vec<&'static ReaderOnlyHarness>> = const { RefCell::new(Vec::new()) };
    }

    /// Add a test-only reader-only harness for the calling thread.
    pub fn add_reader(reader: ReaderOnlyHarness) -> &'static ReaderOnlyHarness {
        let leaked: &'static ReaderOnlyHarness = Box::leak(Box::new(reader));
        READERS.with(|r| r.borrow_mut().push(leaked));
        leaked
    }

    const SAMPLE_READER_DETECT: &[RegistryPath] = &[RegistryPath::home(".test-reader")];

    /// The test-only reader-only row that proves the shape before any real
    /// one ships: it reads the AGENTS folder and is detected by `~/.test-reader`.
    pub fn sample_reader() -> &'static ReaderOnlyHarness {
        add_reader(ReaderOnlyHarness {
            key: "test_reader",
            display_name: "Test Reader",
            detect_dirs: SAMPLE_READER_DETECT,
            reads: VirtualGroup::AgentsStandard,
        })
    }

    pub(super) fn readers() -> Vec<&'static ReaderOnlyHarness> {
        READERS.with(|r| r.borrow().clone())
    }

    /// Shadow the registry entry with `adapter.key()` for the calling thread
    /// and return the `'static` record `adapter_by_key` will now hand out.
    pub fn shadow(adapter: ToolAdapter) -> &'static ToolAdapter {
        let leaked: &'static ToolAdapter = Box::leak(Box::new(adapter));
        OVERRIDES.with(|o| o.borrow_mut().push(leaked));
        leaked
    }

    pub(super) fn lookup(key: &str) -> Option<&'static ToolAdapter> {
        OVERRIDES.with(|o| o.borrow().iter().rev().copied().find(|a| a.key() == key))
    }
}

/// Test-only: install `adapter` on the host whose roots are `roots` the
/// way a tool does — its detect dir plus a file of its own beside the skills
/// path — so a fixture that then seeds skills under it is not mistaken for a
/// skills-only footprint. Every core fixture installs tools through this
/// door; a bare `create_dir_all(detect_dir)` is the footprint the rule rejects.
#[cfg(test)]
pub fn mark_installed_in(roots: &HostRoots, adapter: &ToolAdapter) {
    let detect_dir = adapter.detect_dirs[0].resolve(roots);
    std::fs::create_dir_all(&detect_dir).expect("create detect dir");
    std::fs::write(detect_dir.join("installed.marker"), b"").expect("write install marker");
}

/// The registry join: `relative` (a registry dir, always written with `/`)
/// under `root`, one component at a time, so the result is spelled with the
/// platform's own separator — `C:\Users\op\.claude\skills`, never
/// `C:\Users\op\.claude/skills` (#103). Unix spelling is unchanged. A path
/// recorded with mixed separators still compares equal: `Path` compares
/// components, and Windows accepts both separators.
fn join_relative(root: &Path, relative: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    path.extend(relative.split('/').filter(|part| !part.is_empty()));
    path
}

/// The tool's global skills directory on the host whose roots are `roots`.
pub fn skills_dir_in(roots: &HostRoots, adapter: &ToolAdapter) -> PathBuf {
    adapter.skills_dir.resolve(roots)
}

/// The tool's project-scope skills directory under `project_root` (see
/// [`ToolAdapter::project_skills_dir`]); `None` for a tool with none.
pub fn project_skills_dir_in(project_root: &Path, adapter: &ToolAdapter) -> Option<PathBuf> {
    adapter
        .project_skills_dir
        .map(|dir| join_relative(project_root, dir))
}

/// The tool's former global skills directories (see
/// [`ToolAdapter::former_skills_dirs`]).
pub fn former_skills_dirs_in(roots: &HostRoots, adapter: &ToolAdapter) -> Vec<PathBuf> {
    adapter
        .former_skills_dirs
        .iter()
        .map(|dir| dir.resolve(roots))
        .collect()
}

/// Which Tool's global skills directory holds `path`, if any — the inverse
/// of the deletion rule below (`ensure_path_within_tool_dirs`). Which
/// directories those are is a registry fact, so a caller that must refuse
/// to treat a Tool's copy as an independent source (the Add → local folder
/// flow: such a copy is Import's business) asks here rather than testing
/// paths itself.
///
/// A path is held when it lies under the directory as spelled, or when what
/// it resolves to does (a `/private/var` alias of the directory, a link from
/// elsewhere into it): the bytes live in the Tool's directory either way.
/// Resolution needs the path and the directory to exist; when either does
/// not, only the spelling counts.
pub fn tool_holding_path(roots: &HostRoots, path: &Path) -> Option<&'static ToolAdapter> {
    let resolved = path.canonicalize().ok();
    TOOL_ADAPTERS.iter().find(|adapter| {
        let dir = skills_dir_in(roots, adapter);
        if path.starts_with(&dir) {
            return true;
        }
        match (&resolved, dir.canonicalize()) {
            (Some(resolved), Ok(dir)) => resolved.starts_with(dir),
            _ => false,
        }
    })
}

/// Every directory the deletion rule below admits: each Tool's
/// global skills directory and its former ones (an artifact Skills Hub
/// created in a former dir is still its own to remove). Also the claim a
/// physically admitted removal of such a path is checked against
/// (`core::removal_fence::Claim::Within`).
pub fn deletable_skills_dirs_in(roots: &HostRoots) -> Vec<PathBuf> {
    TOOL_ADAPTERS
        .iter()
        .flat_map(|adapter| {
            std::iter::once(skills_dir_in(roots, adapter))
                .chain(former_skills_dirs_in(roots, adapter))
        })
        .collect()
}

/// The registry's deletion safety rule: Skills Hub only removes paths inside
/// a Tool's global skills directory. Which directories those are is a
/// registry fact, so the refusal lives here rather than in a command body,
/// and it is raised as the typed `SignalError::PathOutsideToolDirs` (never
/// as prose the frontend would have to parse). The rule is lexical; the
/// removal itself is admitted physically (`core::removal_fence`).
pub fn ensure_path_within_tool_dirs(roots: &HostRoots, path: &Path) -> Result<()> {
    let inside = deletable_skills_dirs_in(roots)
        .iter()
        .any(|dir| path.starts_with(dir));
    if inside {
        return Ok(());
    }
    anyhow::bail!(SignalError::PathOutsideToolDirs {
        path: path.to_string_lossy().into_owned(),
    })
}

/// The directories any one of whose presence marks the tool as installed,
/// in registry order.
pub fn detect_dirs_in(roots: &HostRoots, adapter: &ToolAdapter) -> Vec<PathBuf> {
    adapter
        .detect_dirs
        .iter()
        .map(|dir| dir.resolve(roots))
        .collect()
}

/// Whether the tool is installed on the host whose roots are `roots`.
///
/// Some detect dir must exist **and** not be a *skills-only footprint*: a
/// detect dir holding nothing but the path down to the tool's skills dir
/// (`~/.kiro/skills/x` and nothing else) is what a skill deployer leaves
/// behind — `npx skills add`, ego-browser, Skills Hub itself after the tool
/// was uninstalled — not evidence of the tool. An empty detect dir still
/// counts, as does any sibling beside the skills path (`~/.pi/agent/
/// settings.json`), and a detect dir the skills dir does not live under
/// (Amp's `~/.config/amp` beside `~/.config/agents/skills`) counts by
/// presence alone. A virtual group's detect dir *is* the convention
/// (`~/.agents` legitimately holds only `skills/`), so for a group entry
/// presence alone counts too. A group entry is also installed when any of
/// its reader-only harnesses is: the group is that reader's only way in, so
/// its first deployment must not wait for the group's folder to exist.
pub fn is_installed_in(roots: &HostRoots, adapter: &ToolAdapter) -> bool {
    let skills_dir = skills_dir_in(roots, adapter);
    let detected = detect_dirs_in(roots, adapter).iter().any(|detect_dir| {
        detect_dir.exists()
            && (adapter.as_virtual_group().is_some()
                || !is_skills_only_footprint(detect_dir, &skills_dir))
    });
    detected
        || adapter.as_virtual_group().is_some_and(|group| {
            readers_of(group)
                .iter()
                .any(|reader| reader_installed_in(roots, reader))
        })
}

/// True when every directory from `detect_dir` down to (but excluding)
/// `skills_dir` holds exactly one entry — the next path component. A skills
/// dir outside the detect dir, or any read error on the way (opening a
/// directory or enumerating one of its entries), is not a footprint: a
/// permissions hiccup must never hide a tool. Finder's `.DS_Store` is not
/// an entry: browsing a footprint must not promote it.
fn is_skills_only_footprint(detect_dir: &Path, skills_dir: &Path) -> bool {
    let Ok(relative) = skills_dir.strip_prefix(detect_dir) else {
        return false;
    };
    let mut dir = detect_dir.to_path_buf();
    for component in relative.components() {
        let Some(only_entry) = sole_entry(&dir) else {
            return false;
        };
        if only_entry != component.as_os_str() {
            return false;
        }
        dir.push(component);
    }
    true
}

/// The one entry of `dir` (ignoring `.DS_Store`), or `None` when it holds
/// none, several, or cannot be read in full — every error is `None`, so a
/// partially enumerable directory is never mistaken for a single-entry one.
fn sole_entry(dir: &Path) -> Option<std::ffi::OsString> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut sole: Option<std::ffi::OsString> = None;
    for entry in entries {
        let name = entry.ok()?.file_name();
        if name == ".DS_Store" {
            continue;
        }
        if sole.is_some() {
            return None;
        }
        sole = Some(name);
    }
    sole
}

/// One Tool skills dir, scanned: the entries that are skills, and the ones
/// that look like skill folders but are not (`SKILL.md` missing or unusable).
#[derive(Clone, Debug, Default)]
pub struct ToolDirScan {
    pub skills: Vec<DetectedSkill>,
    pub invalid: Vec<InvalidSkillDir>,
}

/// A directory in a Tool's skills dir whose manifest the gate refused.
#[derive(Clone, Debug)]
pub struct InvalidSkillDir {
    pub entry: DetectedSkill,
    pub reason: InvalidManifestReason,
}

/// List a Tool's skills dir, gated on the manifest (#46): a subdirectory is a
/// skill only when its `SKILL.md` parses to a valid manifest (the strict read
/// install admission uses); every other visible subdirectory is reported in
/// `invalid` with the typed reason. A hidden directory that is not a skill is
/// not a candidate at all and is not reported.
pub fn scan_tool_dir(tool: &ToolAdapter, dir: &Path) -> Result<ToolDirScan> {
    let mut results = ToolDirScan::default();
    if !dir.exists() {
        return Ok(results);
    }

    let ignore_hint = "Application Support/com.tauri.dev/skills";

    for entry in std::fs::read_dir(dir).with_context(|| format!("read dir {:?}", dir))? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        let is_dir = file_type.is_dir() || (file_type.is_symlink() && path.is_dir());
        if !is_dir {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if tool.id == ToolId::Codex && name == ".system" {
            continue;
        }
        let (is_link, link_target) = detect_link(&path);
        if path.to_string_lossy().contains(ignore_hint)
            || link_target
                .as_ref()
                .map(|p| p.to_string_lossy().contains(ignore_hint))
                .unwrap_or(false)
        {
            continue;
        }
        let validity = manifest_validity(&path);
        let hidden = name.starts_with('.');
        let entry = DetectedSkill {
            tool: tool.id.clone(),
            name,
            path,
            is_link,
            link_target,
        };
        match validity.invalid_reason() {
            None => results.skills.push(entry),
            Some(_) if hidden => {}
            Some(reason) => results.invalid.push(InvalidSkillDir { entry, reason }),
        }
    }

    Ok(results)
}

fn detect_link(path: &Path) -> (bool, Option<PathBuf>) {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            let target = std::fs::read_link(path).ok();
            (true, target)
        }
        _ => {
            let target = std::fs::read_link(path).ok();
            if target.is_some() {
                (true, target)
            } else {
                (false, None)
            }
        }
    }
}

#[cfg(test)]
#[path = "../tests/tool_adapters.rs"]
mod tests;
