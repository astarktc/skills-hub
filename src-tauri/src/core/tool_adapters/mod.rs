use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::core::errors::SignalError;

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
    /// The label every scope uses unless a scope-specific one overrides it.
    pub display_name: &'static str,
    /// Project-scope label, when it differs from `display_name`. A virtual
    /// group absorbs its constituents at project scope only, so only there
    /// may its label advertise them (`.agents/skills (9 tools)`); globally the
    /// same entry is an independent target and reads `.agents/skills`.
    /// `None` for every adapter whose label is scope-independent.
    pub group_label: Option<&'static str>,
    /// Global skill directory under user home (aligned with add-skill docs).
    pub relative_skills_dir: &'static str,
    /// Directories whose presence under home marks the tool as installed —
    /// the tool's own configuration root(s), any one of which counts. More
    /// than one entry when a tool moved its root across versions (Kimi:
    /// `.kimi-code` today, `.kimi` before). Never a shared skills convention
    /// dir (`.config/agents`): those are footprints, not tools.
    pub relative_detect_dirs: &'static [&'static str],
    /// Project-scope skill directory relative to a project root. Differs
    /// from the global dir for many tools (e.g. Pi, Windsurf) and is the
    /// only mapping project sync may use.
    pub project_relative_skills_dir: &'static str,
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

    /// The label for this adapter at project scope — `group_label` when the
    /// registry states one, otherwise `display_name`.
    pub fn project_display_name(&self) -> &'static str {
        self.group_label.unwrap_or(self.display_name)
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
        relative_skills_dir: ".agents/skills",
        relative_detect_dirs: &[".agents"],
        project_relative_skills_dir: ".agents/skills",
        group_label: Some(".agents/skills (9 tools)"),
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Cursor,
        display_name: "Cursor",
        relative_skills_dir: ".cursor/skills",
        relative_detect_dirs: &[".cursor"],
        project_relative_skills_dir: ".agents/skills",
        group_label: None,
        group: Some(VirtualGroup::AgentsStandard),
        // Copy-only until Cursor IDE 2.5 (Feb 2026) fixed symlink discovery
        // under ~/.cursor/skills; flipped in v-next ticket 38.
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::ClaudeCode,
        display_name: "Claude Code",
        relative_skills_dir: ".claude/skills",
        relative_detect_dirs: &[".claude"],
        project_relative_skills_dir: ".claude/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Codex,
        display_name: "Codex",
        relative_skills_dir: ".codex/skills",
        relative_detect_dirs: &[".codex"],
        project_relative_skills_dir: ".agents/skills",
        group_label: None,
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::OpenCode,
        display_name: "OpenCode",
        // add-skill global path: ~/.config/opencode/skills/
        relative_skills_dir: ".config/opencode/skills",
        relative_detect_dirs: &[".config/opencode"],
        project_relative_skills_dir: ".agents/skills",
        group_label: None,
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Antigravity,
        display_name: "Antigravity",
        // add-skill global path: ~/.gemini/antigravity/global_skills/
        relative_skills_dir: ".gemini/antigravity/global_skills",
        relative_detect_dirs: &[".gemini/antigravity"],
        project_relative_skills_dir: ".agents/skills",
        group_label: None,
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Amp,
        display_name: "Amp",
        // add-skill global path: ~/.config/agents/skills/; the tool itself
        // lives in ~/.config/amp (settings.json — ampcode.com/docs/cli/settings).
        relative_skills_dir: ".config/agents/skills",
        relative_detect_dirs: &[".config/amp"],
        project_relative_skills_dir: ".agents/skills",
        group_label: None,
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::KimiCli,
        display_name: "Kimi Code CLI",
        // add-skill global path: ~/.config/agents/skills/ (shared with Amp).
        // The tool's config root is ~/.kimi-code (kimi.com/code docs) or
        // ~/.kimi on earlier releases (kimi-cli.com docs); either counts.
        relative_skills_dir: ".config/agents/skills",
        relative_detect_dirs: &[".kimi-code", ".kimi"],
        project_relative_skills_dir: ".agents/skills",
        group_label: None,
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Augment,
        display_name: "Augment",
        // add-skill global path: ~/.augment/rules/
        relative_skills_dir: ".augment/rules",
        relative_detect_dirs: &[".augment"],
        project_relative_skills_dir: ".augment/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::OpenClaw,
        display_name: "OpenClaw",
        // add-skill global path: ~/.openclaw/skills/
        relative_skills_dir: ".openclaw/skills",
        relative_detect_dirs: &[".openclaw"],
        project_relative_skills_dir: "skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Copaw,
        display_name: "Copaw",
        // add-skill global path: ~/.copaw/skill_pool/
        relative_skills_dir: ".copaw/skill_pool",
        relative_detect_dirs: &[".copaw"],
        project_relative_skills_dir: ".copaw/skill_pool",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Cline,
        display_name: "Cline",
        // add-skill global path: ~/.cline/skills/
        relative_skills_dir: ".cline/skills",
        relative_detect_dirs: &[".cline"],
        project_relative_skills_dir: ".agents/skills",
        group_label: None,
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::CodeBuddy,
        display_name: "CodeBuddy",
        // add-skill global path: ~/.codebuddy/skills/
        relative_skills_dir: ".codebuddy/skills",
        relative_detect_dirs: &[".codebuddy"],
        project_relative_skills_dir: ".codebuddy/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::CommandCode,
        display_name: "Command Code",
        // add-skill global path: ~/.commandcode/skills/
        relative_skills_dir: ".commandcode/skills",
        relative_detect_dirs: &[".commandcode"],
        project_relative_skills_dir: ".commandcode/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Continue,
        display_name: "Continue",
        // add-skill global path: ~/.continue/skills/
        relative_skills_dir: ".continue/skills",
        relative_detect_dirs: &[".continue"],
        project_relative_skills_dir: ".continue/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Crush,
        display_name: "Crush",
        // add-skill global path: ~/.config/crush/skills/
        relative_skills_dir: ".config/crush/skills",
        relative_detect_dirs: &[".config/crush"],
        project_relative_skills_dir: ".crush/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Junie,
        display_name: "Junie",
        // add-skill global path: ~/.junie/skills/
        relative_skills_dir: ".junie/skills",
        relative_detect_dirs: &[".junie"],
        project_relative_skills_dir: ".junie/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::IflowCli,
        display_name: "iFlow CLI",
        // add-skill global path: ~/.iflow/skills/
        relative_skills_dir: ".iflow/skills",
        relative_detect_dirs: &[".iflow"],
        project_relative_skills_dir: ".iflow/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::KiroCli,
        display_name: "Kiro CLI",
        // add-skill global path: ~/.kiro/skills/
        relative_skills_dir: ".kiro/skills",
        relative_detect_dirs: &[".kiro"],
        project_relative_skills_dir: ".kiro/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Kode,
        display_name: "Kode",
        // add-skill global path: ~/.kode/skills/
        relative_skills_dir: ".kode/skills",
        relative_detect_dirs: &[".kode"],
        project_relative_skills_dir: ".kode/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::McpJam,
        display_name: "MCPJam",
        // add-skill global path: ~/.mcpjam/skills/
        relative_skills_dir: ".mcpjam/skills",
        relative_detect_dirs: &[".mcpjam"],
        project_relative_skills_dir: ".mcpjam/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::MistralVibe,
        display_name: "Mistral Vibe",
        // add-skill global path: ~/.vibe/skills/
        relative_skills_dir: ".vibe/skills",
        relative_detect_dirs: &[".vibe"],
        project_relative_skills_dir: ".vibe/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Mux,
        display_name: "Mux",
        // add-skill global path: ~/.mux/skills/
        relative_skills_dir: ".mux/skills",
        relative_detect_dirs: &[".mux"],
        project_relative_skills_dir: ".mux/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::OpenClaude,
        display_name: "OpenClaude IDE",
        // add-skill global path: ~/.openclaude/skills/
        relative_skills_dir: ".openclaude/skills",
        relative_detect_dirs: &[".openclaude"],
        project_relative_skills_dir: ".openclaude/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::OpenHands,
        display_name: "OpenHands",
        // add-skill global path: ~/.openhands/skills/
        relative_skills_dir: ".openhands/skills",
        relative_detect_dirs: &[".openhands"],
        project_relative_skills_dir: ".openhands/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Pi,
        display_name: "Pi",
        // add-skill global path: ~/.pi/agent/skills/
        relative_skills_dir: ".pi/agent/skills",
        relative_detect_dirs: &[".pi"],
        project_relative_skills_dir: ".pi/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Qoder,
        display_name: "Qoder",
        // add-skill global path: ~/.qoder/skills/
        relative_skills_dir: ".qoder/skills",
        relative_detect_dirs: &[".qoder"],
        project_relative_skills_dir: ".qoder/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::QoderWork,
        display_name: "QoderWork",
        // add-skill global path: ~/.qoderwork/skills/
        relative_skills_dir: ".qoderwork/skills",
        relative_detect_dirs: &[".qoderwork"],
        project_relative_skills_dir: ".qoderwork/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::QwenCode,
        display_name: "Qwen Code",
        // add-skill global path: ~/.qwen/skills/
        relative_skills_dir: ".qwen/skills",
        relative_detect_dirs: &[".qwen"],
        project_relative_skills_dir: ".qwen/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Trae,
        display_name: "Trae",
        // add-skill global path: ~/.trae/skills/
        relative_skills_dir: ".trae/skills",
        relative_detect_dirs: &[".trae"],
        project_relative_skills_dir: ".trae/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::TraeCn,
        display_name: "Trae CN",
        // add-skill global path: ~/.trae-cn/skills/
        relative_skills_dir: ".trae-cn/skills",
        relative_detect_dirs: &[".trae-cn"],
        project_relative_skills_dir: ".trae/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Zencoder,
        display_name: "Zencoder",
        // add-skill global path: ~/.zencoder/skills/
        relative_skills_dir: ".zencoder/skills",
        relative_detect_dirs: &[".zencoder"],
        project_relative_skills_dir: ".zencoder/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Neovate,
        display_name: "Neovate",
        // add-skill global path: ~/.neovate/skills/
        relative_skills_dir: ".neovate/skills",
        relative_detect_dirs: &[".neovate"],
        project_relative_skills_dir: ".neovate/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Pochi,
        display_name: "Pochi",
        // add-skill global path: ~/.pochi/skills/
        relative_skills_dir: ".pochi/skills",
        relative_detect_dirs: &[".pochi"],
        project_relative_skills_dir: ".pochi/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::AdaL,
        display_name: "AdaL",
        // add-skill global path: ~/.adal/skills/
        relative_skills_dir: ".adal/skills",
        relative_detect_dirs: &[".adal"],
        project_relative_skills_dir: ".adal/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::KiloCode,
        display_name: "Kilo Code",
        // add-skill global path: ~/.kilocode/skills/
        relative_skills_dir: ".kilocode/skills",
        relative_detect_dirs: &[".kilocode"],
        project_relative_skills_dir: ".kilocode/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::RooCode,
        display_name: "Roo Code",
        // add-skill global path: ~/.roo/skills/
        relative_skills_dir: ".roo/skills",
        relative_detect_dirs: &[".roo"],
        project_relative_skills_dir: ".roo/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Goose,
        display_name: "Goose",
        // add-skill global path: ~/.config/goose/skills/
        relative_skills_dir: ".config/goose/skills",
        relative_detect_dirs: &[".config/goose"],
        project_relative_skills_dir: ".goose/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::GeminiCli,
        display_name: "Gemini CLI",
        // add-skill global path: ~/.gemini/skills/
        relative_skills_dir: ".gemini/skills",
        relative_detect_dirs: &[".gemini"],
        project_relative_skills_dir: ".agents/skills",
        group_label: None,
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::GithubCopilot,
        display_name: "GitHub Copilot",
        // add-skill global path: ~/.copilot/skills/
        relative_skills_dir: ".copilot/skills",
        relative_detect_dirs: &[".copilot"],
        project_relative_skills_dir: ".agents/skills",
        group_label: None,
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Clawdbot,
        display_name: "Clawdbot",
        // add-skill global path: ~/.clawdbot/skills/
        relative_skills_dir: ".clawdbot/skills",
        relative_detect_dirs: &[".clawdbot"],
        project_relative_skills_dir: ".clawdbot/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Droid,
        display_name: "Droid",
        // add-skill global path: ~/.factory/skills/
        relative_skills_dir: ".factory/skills",
        relative_detect_dirs: &[".factory"],
        project_relative_skills_dir: ".factory/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Windsurf,
        display_name: "Windsurf",
        // add-skill global path: ~/.codeium/windsurf/skills/
        relative_skills_dir: ".codeium/windsurf/skills",
        relative_detect_dirs: &[".codeium/windsurf"],
        project_relative_skills_dir: ".windsurf/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Moltbot,
        display_name: "MoltBot",
        // add-skill global path: ~/.moltbot/skills/
        relative_skills_dir: ".moltbot/skills",
        relative_detect_dirs: &[".moltbot"],
        project_relative_skills_dir: ".moltbot/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::HermesAgent,
        display_name: "Hermes Agent",
        relative_skills_dir: ".hermes/skills",
        relative_detect_dirs: &[".hermes"],
        project_relative_skills_dir: ".hermes/skills",
        group_label: None,
        group: None,
        supports_symlink: true,
    },
];

/// Every known tool adapter, in registry (presentation) order.
pub fn default_tool_adapters() -> &'static [ToolAdapter] {
    TOOL_ADAPTERS
}

/// Tools can share the same global skills directory (e.g. Amp and Kimi Code CLI).
/// Use this to coordinate UI warnings and avoid duplicate filesystem operations.
pub fn adapters_sharing_skills_dir(adapter: &ToolAdapter) -> Vec<&'static ToolAdapter> {
    TOOL_ADAPTERS
        .iter()
        .filter(|a| a.relative_skills_dir == adapter.relative_skills_dir)
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

/// Test-only registry shadowing, so core tests can exercise a capability no
/// shipped entry carries (e.g. copy-only sync) without a fake registry entry.
/// Overrides are thread-local — each `cargo test` case runs on its own thread
/// — and leaked to `'static` to match the registry's borrow contract.
#[cfg(test)]
pub mod test_overrides {
    use super::ToolAdapter;
    use std::cell::RefCell;

    thread_local! {
        static OVERRIDES: RefCell<Vec<&'static ToolAdapter>> = const { RefCell::new(Vec::new()) };
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

/// Test-only: install `adapter` for the operator whose home is `home` the
/// way a tool does — its detect dir plus a file of its own beside the skills
/// path — so a fixture that then seeds skills under it is not mistaken for a
/// skills-only footprint. Every core fixture installs tools through this
/// door; a bare `create_dir_all(detect_dir)` is the footprint the rule rejects.
#[cfg(test)]
pub fn mark_installed_in(home: &Path, adapter: &ToolAdapter) {
    let detect_dir = home.join(adapter.relative_detect_dirs[0]);
    std::fs::create_dir_all(&detect_dir).expect("create detect dir");
    std::fs::write(detect_dir.join("installed.marker"), b"").expect("write install marker");
}

/// The tool's global skills directory under `home`.
pub fn skills_dir_in(home: &Path, adapter: &ToolAdapter) -> PathBuf {
    home.join(adapter.relative_skills_dir)
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
pub fn tool_holding_path(home: &Path, path: &Path) -> Option<&'static ToolAdapter> {
    let resolved = path.canonicalize().ok();
    TOOL_ADAPTERS.iter().find(|adapter| {
        let dir = skills_dir_in(home, adapter);
        if path.starts_with(&dir) {
            return true;
        }
        match (&resolved, dir.canonicalize()) {
            (Some(resolved), Ok(dir)) => resolved.starts_with(dir),
            _ => false,
        }
    })
}

/// The registry's deletion safety rule: Skills Hub only removes paths inside
/// a Tool's global skills directory. Which directories those are is a
/// registry fact, so the refusal lives here rather than in a command body,
/// and it is raised as the typed `SignalError::PathOutsideToolDirs` (never
/// as prose the frontend would have to parse).
pub fn ensure_path_within_tool_dirs(home: &Path, path: &Path) -> Result<()> {
    let inside = TOOL_ADAPTERS
        .iter()
        .any(|adapter| path.starts_with(skills_dir_in(home, adapter)));
    if inside {
        return Ok(());
    }
    anyhow::bail!(SignalError::PathOutsideToolDirs {
        path: path.to_string_lossy().into_owned(),
    })
}

/// The directories any one of whose presence under `home` marks the tool as
/// installed, in registry order.
pub fn detect_dirs_in(home: &Path, adapter: &ToolAdapter) -> Vec<PathBuf> {
    adapter
        .relative_detect_dirs
        .iter()
        .map(|dir| home.join(dir))
        .collect()
}

/// Whether the tool is installed for the operator whose home is `home`.
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
/// presence alone counts too.
pub fn is_installed_in(home: &Path, adapter: &ToolAdapter) -> bool {
    let skills_dir = skills_dir_in(home, adapter);
    detect_dirs_in(home, adapter).iter().any(|detect_dir| {
        detect_dir.exists()
            && (adapter.as_virtual_group().is_some()
                || !is_skills_only_footprint(detect_dir, &skills_dir))
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

pub fn scan_tool_dir(tool: &ToolAdapter, dir: &Path) -> Result<Vec<DetectedSkill>> {
    let mut results = Vec::new();
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
        results.push(DetectedSkill {
            tool: tool.id.clone(),
            name,
            path,
            is_link,
            link_target,
        });
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
