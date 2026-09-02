use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

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
    pub display_name: &'static str,
    /// Global skill directory under user home (aligned with add-skill docs).
    pub relative_skills_dir: &'static str,
    /// Directory used to detect whether the tool is installed (aligned with add-skill docs).
    pub relative_detect_dir: &'static str,
    /// Project-scope skill directory relative to a project root. Differs
    /// from the global dir for many tools (e.g. Pi, Windsurf) and is the
    /// only mapping project sync may use.
    pub project_relative_skills_dir: &'static str,
    /// The virtual group this tool is absorbed into at project scope;
    /// `None` for standalone tools and for group entries themselves.
    pub group: Option<VirtualGroup>,
    /// Whether the tool can consume a symlinked/junctioned skills dir.
    /// `false` forces copy mode in every sync path (Cursor).
    pub supports_symlink: bool,
}

impl ToolAdapter {
    pub fn key(&self) -> &'static str {
        self.id.as_key()
    }

    /// `Some` when this adapter is the entry of a virtual group.
    pub fn as_virtual_group(&self) -> Option<VirtualGroup> {
        VirtualGroup::ALL
            .iter()
            .copied()
            .find(|g| g.entry_id() == self.id)
    }

    pub fn is_virtual_group(&self) -> bool {
        self.as_virtual_group().is_some()
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
        display_name: ".agents/skills (9 tools)",
        relative_skills_dir: ".agents/skills",
        relative_detect_dir: ".agents",
        project_relative_skills_dir: ".agents/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Cursor,
        display_name: "Cursor",
        relative_skills_dir: ".cursor/skills",
        relative_detect_dir: ".cursor",
        project_relative_skills_dir: ".agents/skills",
        group: Some(VirtualGroup::AgentsStandard),
        // Cursor cannot read a symlinked/junctioned skills dir: every sync path copies.
        supports_symlink: false,
    },
    ToolAdapter {
        id: ToolId::ClaudeCode,
        display_name: "Claude Code",
        relative_skills_dir: ".claude/skills",
        relative_detect_dir: ".claude",
        project_relative_skills_dir: ".claude/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Codex,
        display_name: "Codex",
        relative_skills_dir: ".codex/skills",
        relative_detect_dir: ".codex",
        project_relative_skills_dir: ".agents/skills",
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::OpenCode,
        display_name: "OpenCode",
        // add-skill global path: ~/.config/opencode/skills/
        relative_skills_dir: ".config/opencode/skills",
        relative_detect_dir: ".config/opencode",
        project_relative_skills_dir: ".agents/skills",
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Antigravity,
        display_name: "Antigravity",
        // add-skill global path: ~/.gemini/antigravity/global_skills/
        relative_skills_dir: ".gemini/antigravity/global_skills",
        relative_detect_dir: ".gemini/antigravity",
        project_relative_skills_dir: ".agents/skills",
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Amp,
        display_name: "Amp",
        // add-skill global path: ~/.config/agents/skills/
        relative_skills_dir: ".config/agents/skills",
        relative_detect_dir: ".config/agents",
        project_relative_skills_dir: ".agents/skills",
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::KimiCli,
        display_name: "Kimi Code CLI",
        // add-skill global path: ~/.config/agents/skills/
        // NOTE: Shares the same skills directory with Amp.
        relative_skills_dir: ".config/agents/skills",
        relative_detect_dir: ".config/agents",
        project_relative_skills_dir: ".agents/skills",
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Augment,
        display_name: "Augment",
        // add-skill global path: ~/.augment/rules/
        relative_skills_dir: ".augment/rules",
        relative_detect_dir: ".augment",
        project_relative_skills_dir: ".augment/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::OpenClaw,
        display_name: "OpenClaw",
        // add-skill global path: ~/.openclaw/skills/
        relative_skills_dir: ".openclaw/skills",
        relative_detect_dir: ".openclaw",
        project_relative_skills_dir: "skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Copaw,
        display_name: "Copaw",
        // add-skill global path: ~/.copaw/skill_pool/
        relative_skills_dir: ".copaw/skill_pool",
        relative_detect_dir: ".copaw",
        project_relative_skills_dir: ".copaw/skill_pool",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Cline,
        display_name: "Cline",
        // add-skill global path: ~/.cline/skills/
        relative_skills_dir: ".cline/skills",
        relative_detect_dir: ".cline",
        project_relative_skills_dir: ".agents/skills",
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::CodeBuddy,
        display_name: "CodeBuddy",
        // add-skill global path: ~/.codebuddy/skills/
        relative_skills_dir: ".codebuddy/skills",
        relative_detect_dir: ".codebuddy",
        project_relative_skills_dir: ".codebuddy/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::CommandCode,
        display_name: "Command Code",
        // add-skill global path: ~/.commandcode/skills/
        relative_skills_dir: ".commandcode/skills",
        relative_detect_dir: ".commandcode",
        project_relative_skills_dir: ".commandcode/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Continue,
        display_name: "Continue",
        // add-skill global path: ~/.continue/skills/
        relative_skills_dir: ".continue/skills",
        relative_detect_dir: ".continue",
        project_relative_skills_dir: ".continue/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Crush,
        display_name: "Crush",
        // add-skill global path: ~/.config/crush/skills/
        relative_skills_dir: ".config/crush/skills",
        relative_detect_dir: ".config/crush",
        project_relative_skills_dir: ".crush/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Junie,
        display_name: "Junie",
        // add-skill global path: ~/.junie/skills/
        relative_skills_dir: ".junie/skills",
        relative_detect_dir: ".junie",
        project_relative_skills_dir: ".junie/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::IflowCli,
        display_name: "iFlow CLI",
        // add-skill global path: ~/.iflow/skills/
        relative_skills_dir: ".iflow/skills",
        relative_detect_dir: ".iflow",
        project_relative_skills_dir: ".iflow/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::KiroCli,
        display_name: "Kiro CLI",
        // add-skill global path: ~/.kiro/skills/
        relative_skills_dir: ".kiro/skills",
        relative_detect_dir: ".kiro",
        project_relative_skills_dir: ".kiro/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Kode,
        display_name: "Kode",
        // add-skill global path: ~/.kode/skills/
        relative_skills_dir: ".kode/skills",
        relative_detect_dir: ".kode",
        project_relative_skills_dir: ".kode/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::McpJam,
        display_name: "MCPJam",
        // add-skill global path: ~/.mcpjam/skills/
        relative_skills_dir: ".mcpjam/skills",
        relative_detect_dir: ".mcpjam",
        project_relative_skills_dir: ".mcpjam/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::MistralVibe,
        display_name: "Mistral Vibe",
        // add-skill global path: ~/.vibe/skills/
        relative_skills_dir: ".vibe/skills",
        relative_detect_dir: ".vibe",
        project_relative_skills_dir: ".vibe/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Mux,
        display_name: "Mux",
        // add-skill global path: ~/.mux/skills/
        relative_skills_dir: ".mux/skills",
        relative_detect_dir: ".mux",
        project_relative_skills_dir: ".mux/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::OpenClaude,
        display_name: "OpenClaude IDE",
        // add-skill global path: ~/.openclaude/skills/
        relative_skills_dir: ".openclaude/skills",
        relative_detect_dir: ".openclaude",
        project_relative_skills_dir: ".openclaude/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::OpenHands,
        display_name: "OpenHands",
        // add-skill global path: ~/.openhands/skills/
        relative_skills_dir: ".openhands/skills",
        relative_detect_dir: ".openhands",
        project_relative_skills_dir: ".openhands/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Pi,
        display_name: "Pi",
        // add-skill global path: ~/.pi/agent/skills/
        relative_skills_dir: ".pi/agent/skills",
        relative_detect_dir: ".pi",
        project_relative_skills_dir: ".pi/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Qoder,
        display_name: "Qoder",
        // add-skill global path: ~/.qoder/skills/
        relative_skills_dir: ".qoder/skills",
        relative_detect_dir: ".qoder",
        project_relative_skills_dir: ".qoder/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::QoderWork,
        display_name: "QoderWork",
        // add-skill global path: ~/.qoderwork/skills/
        relative_skills_dir: ".qoderwork/skills",
        relative_detect_dir: ".qoderwork",
        project_relative_skills_dir: ".qoderwork/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::QwenCode,
        display_name: "Qwen Code",
        // add-skill global path: ~/.qwen/skills/
        relative_skills_dir: ".qwen/skills",
        relative_detect_dir: ".qwen",
        project_relative_skills_dir: ".qwen/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Trae,
        display_name: "Trae",
        // add-skill global path: ~/.trae/skills/
        relative_skills_dir: ".trae/skills",
        relative_detect_dir: ".trae",
        project_relative_skills_dir: ".trae/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::TraeCn,
        display_name: "Trae CN",
        // add-skill global path: ~/.trae-cn/skills/
        relative_skills_dir: ".trae-cn/skills",
        relative_detect_dir: ".trae-cn",
        project_relative_skills_dir: ".trae/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Zencoder,
        display_name: "Zencoder",
        // add-skill global path: ~/.zencoder/skills/
        relative_skills_dir: ".zencoder/skills",
        relative_detect_dir: ".zencoder",
        project_relative_skills_dir: ".zencoder/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Neovate,
        display_name: "Neovate",
        // add-skill global path: ~/.neovate/skills/
        relative_skills_dir: ".neovate/skills",
        relative_detect_dir: ".neovate",
        project_relative_skills_dir: ".neovate/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Pochi,
        display_name: "Pochi",
        // add-skill global path: ~/.pochi/skills/
        relative_skills_dir: ".pochi/skills",
        relative_detect_dir: ".pochi",
        project_relative_skills_dir: ".pochi/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::AdaL,
        display_name: "AdaL",
        // add-skill global path: ~/.adal/skills/
        relative_skills_dir: ".adal/skills",
        relative_detect_dir: ".adal",
        project_relative_skills_dir: ".adal/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::KiloCode,
        display_name: "Kilo Code",
        // add-skill global path: ~/.kilocode/skills/
        relative_skills_dir: ".kilocode/skills",
        relative_detect_dir: ".kilocode",
        project_relative_skills_dir: ".kilocode/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::RooCode,
        display_name: "Roo Code",
        // add-skill global path: ~/.roo/skills/
        relative_skills_dir: ".roo/skills",
        relative_detect_dir: ".roo",
        project_relative_skills_dir: ".roo/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Goose,
        display_name: "Goose",
        // add-skill global path: ~/.config/goose/skills/
        relative_skills_dir: ".config/goose/skills",
        relative_detect_dir: ".config/goose",
        project_relative_skills_dir: ".goose/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::GeminiCli,
        display_name: "Gemini CLI",
        // add-skill global path: ~/.gemini/skills/
        relative_skills_dir: ".gemini/skills",
        relative_detect_dir: ".gemini",
        project_relative_skills_dir: ".agents/skills",
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::GithubCopilot,
        display_name: "GitHub Copilot",
        // add-skill global path: ~/.copilot/skills/
        relative_skills_dir: ".copilot/skills",
        relative_detect_dir: ".copilot",
        project_relative_skills_dir: ".agents/skills",
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Clawdbot,
        display_name: "Clawdbot",
        // add-skill global path: ~/.clawdbot/skills/
        relative_skills_dir: ".clawdbot/skills",
        relative_detect_dir: ".clawdbot",
        project_relative_skills_dir: ".clawdbot/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Droid,
        display_name: "Droid",
        // add-skill global path: ~/.factory/skills/
        relative_skills_dir: ".factory/skills",
        relative_detect_dir: ".factory",
        project_relative_skills_dir: ".factory/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Windsurf,
        display_name: "Windsurf",
        // add-skill global path: ~/.codeium/windsurf/skills/
        relative_skills_dir: ".codeium/windsurf/skills",
        relative_detect_dir: ".codeium/windsurf",
        project_relative_skills_dir: ".windsurf/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::Moltbot,
        display_name: "MoltBot",
        // add-skill global path: ~/.moltbot/skills/
        relative_skills_dir: ".moltbot/skills",
        relative_detect_dir: ".moltbot",
        project_relative_skills_dir: ".moltbot/skills",
        group: None,
        supports_symlink: true,
    },
    ToolAdapter {
        id: ToolId::HermesAgent,
        display_name: "Hermes Agent",
        relative_skills_dir: ".hermes/skills",
        relative_detect_dir: ".hermes",
        project_relative_skills_dir: ".hermes/skills",
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
    TOOL_ADAPTERS
        .iter()
        .find(|adapter| adapter.id.as_key() == key)
}

/// The tool's global skills directory under `home`.
pub fn skills_dir_in(home: &Path, adapter: &ToolAdapter) -> PathBuf {
    home.join(adapter.relative_skills_dir)
}

/// The directory whose presence under `home` marks the tool as installed.
pub fn detect_dir_in(home: &Path, adapter: &ToolAdapter) -> PathBuf {
    home.join(adapter.relative_detect_dir)
}

/// Whether the tool is installed for the operator whose home is `home`.
pub fn is_installed_in(home: &Path, adapter: &ToolAdapter) -> bool {
    detect_dir_in(home, adapter).exists()
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
