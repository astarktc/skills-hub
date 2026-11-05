use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::core::errors::SignalError;
use crate::core::skill_discovery::{manifest_validity, InvalidManifestReason};

pub mod catalog;

pub use catalog::{
    global_tool_entries, installed_global_keys, installed_keys, project_tool_entries,
    ToolCatalogEntry,
};

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
    Droid,
    Windsurf,
    HermesAgent,
    DeepseekHarness,
    Zcode,
    Codewhale,
    WorkBuddy,
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
            ToolId::Droid => "droid",
            ToolId::Windsurf => "windsurf",
            ToolId::HermesAgent => "hermes-agent",
            ToolId::DeepseekHarness => "deepseek_harness",
            ToolId::Zcode => "zcode",
            ToolId::Codewhale => "codewhale",
            ToolId::WorkBuddy => "workbuddy",
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

/// Test-only: keys of rows ticket 34 retired from the registry — Clawdbot
/// and MoltBot, former names of OpenClaw (brief § 5 / I10), which keeps its
/// row and lists them as `former_keys`. A user's stored global row under one
/// is re-keyed to OpenClaw when relocation moves it out of a former dir;
/// any other stored row, selection or project Tool naming one is tolerated
/// as every unknown key is (the selection prunes it, Propagation skips it as
/// `UnknownTool`, Delete removes its artifact by the row's stored path); the
/// tests that pin this iterate the list. No production code branches on a
/// retired name.
#[cfg(test)]
pub const RETIRED_TOOL_KEYS: &[&str] = &["clawdbot", "moltbot"];

/// The AGENTS-standard global skills dir, relative to home: the dir of the
/// `agents_skills` entry and of every harness deploying there.
pub const AGENTS_STANDARD_DIR: &str = ".agents/skills";

/// How a harness resolves a same-name skill present both in its own global
/// dir and in `~/.agents/skills` (brief § 1, column "Precedence / dedupe").
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum Precedence {
    /// The harness's own dir wins (Amp, Augment, Qwen, Roo, Pochi, …).
    OwnWins,
    /// `~/.agents/skills` wins (Gemini CLI, OpenHands).
    AgentsWins,
    /// No dedupe: both copies are listed (Codex).
    BothShown,
    /// The vendor does not say.
    Unknown,
}

/// A **VERIFIED** fact (brief § 1): the harness reads `~/.agents/skills` at
/// global scope by default. LIKELY / UNKNOWN rows carry no such fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AgentsStandardRead {
    /// The first version / change known to read it, when the brief verified one.
    pub since: Option<&'static str>,
    pub precedence: Precedence,
}

/// Which global skills dir each harness deploys to — the #83 preference
/// "prefer the AGENTS standard folder", read from the settings
/// (`settings::global_layout`) and handed to every derivation of a global
/// target dir, so the registry stays static and core reads no ambient state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlobalLayout {
    /// When on, every harness with a VERIFIED `reads_agents_standard_globally`
    /// deploys to `~/.agents/skills` instead of its own dir (brief I1: a
    /// harness reading both would otherwise show the skill twice).
    pub prefer_agents_standard: bool,
}

impl GlobalLayout {
    /// Every harness deploys to its own registry dir.
    pub const OWN_DIRS: GlobalLayout = GlobalLayout {
        prefer_agents_standard: false,
    };
    /// AGENTS-standard readers deploy there: a fresh install's default, and
    /// an existing install's only once the operator saves it
    /// (`settings::global_layout`).
    pub const PREFER_AGENTS_STANDARD: GlobalLayout = GlobalLayout {
        prefer_agents_standard: true,
    };
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
    /// may its label advertise them (`.agents/skills (9 harnesses)`); globally the
    /// same entry is an independent target and reads `.agents/skills`.
    /// `None` for every adapter whose label is scope-independent.
    pub group_label: Option<&'static str>,
    /// The harness's own global skill directory under user home (brief § 1).
    /// The dir a sync writes to is [`ToolAdapter::global_relative_skills_dir`]
    /// — this one, unless the `GlobalLayout` moves the harness to the
    /// AGENTS-standard dir.
    pub relative_skills_dir: &'static str,
    /// Global skill directories this tool was registered with before
    /// `relative_skills_dir` was corrected (Augment: `.augment/rules`, which
    /// is its rules dir, not its skills dir). Artifacts Skills Hub created
    /// there are moved by `core::target_relocation` at startup, and they
    /// stay inside the deletion rule (`ensure_path_within_tool_dirs`) so a
    /// row still pointing there can always be cleaned up. Never scanned,
    /// never synced into. Empty for almost every tool.
    pub former_relative_skills_dirs: &'static [&'static str],
    /// Registry keys this harness had under a **former product name**, whose
    /// rows were retired into this one (OpenClaw: `clawdbot`, `moltbot` —
    /// brief rows 41 / 44, I10). A stored global row under such a key whose
    /// artifact sits in one of this Tool's former dirs is re-keyed to this
    /// Tool when `core::target_relocation` moves it. No key here is
    /// registered; empty for almost every tool.
    pub former_keys: &'static [&'static str],
    /// Directories whose presence under home marks the tool as installed —
    /// the tool's own configuration root(s), any one of which counts. More
    /// than one entry when a tool moved its root across versions (Kimi:
    /// `.kimi-code` today, `.kimi` before). Never a shared skills convention
    /// dir (`.config/agents`): those are footprints, not tools.
    pub relative_detect_dirs: &'static [&'static str],
    /// Project-scope skill directory relative to a project root. Differs
    /// from the global dir for many tools (e.g. Pi, Windsurf) and is the
    /// only mapping project sync may use. `None` for a **global-only** Tool
    /// whose project dir is unresolved (WorkBuddy): absent from every project
    /// catalog and refused by project configuration.
    pub project_relative_skills_dir: Option<&'static str>,
    /// The virtual group this tool is absorbed into at project scope;
    /// `None` for standalone tools and for group entries themselves.
    pub group: Option<VirtualGroup>,
    /// Whether the tool can consume a symlinked/junctioned skills dir.
    /// `false` forces copy mode in every sync path. Every current entry is
    /// `true`; the capability stays as the lever for a tool that regresses
    /// (Cursor was `false` until IDE 2.5 / CLI 2026-06 fixed discovery).
    pub supports_symlink: bool,
    /// **AGENTS-standard read** (CONTEXT.md): `Some` only for a VERIFIED read
    /// of `~/.agents/skills` at global scope (brief § 1, I11). Decides where
    /// the harness deploys under `GlobalLayout::PREFER_AGENTS_STANDARD`.
    pub reads_agents_standard_globally: Option<AgentsStandardRead>,
    /// **Cross-read** (CONTEXT.md): global dirs (relative to home) of *other*
    /// harnesses this one also reads by default (brief I1) — Cursor reads
    /// Claude Code's and Codex's. A skill deployed to both shows twice in
    /// this harness; the catalog resolves these to installed harness keys.
    pub also_reads: &'static [&'static str],
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

    /// The global dir (relative to home) this harness deploys to under
    /// `layout`: the AGENTS-standard dir when the preference is on and the
    /// harness VERIFIED-reads it, otherwise its own `relative_skills_dir`.
    pub fn global_relative_skills_dir(&self, layout: GlobalLayout) -> &'static str {
        if layout.prefer_agents_standard && self.reads_agents_standard_globally.is_some() {
            AGENTS_STANDARD_DIR
        } else {
            self.relative_skills_dir
        }
    }

    /// The global dirs (relative to home) of *other* harnesses this one also
    /// reads by default: its cross-reads (`also_reads`) and, for a VERIFIED
    /// reader, the AGENTS-standard dir. The one "what else does it read" rule.
    pub fn other_global_reads(&self) -> impl Iterator<Item = &'static str> {
        self.also_reads.iter().copied().chain(
            self.reads_agents_standard_globally
                .is_some()
                .then_some(AGENTS_STANDARD_DIR),
        )
    }

    /// Whether this harness loads skills from the global dir `dir` (relative
    /// to home): its own dir, or one of [`Self::other_global_reads`].
    pub fn reads_global_dir(&self, dir: &str) -> bool {
        self.relative_skills_dir == dir || self.other_global_reads().any(|read| read == dir)
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

/// Shorthand for a VERIFIED `~/.agents/skills` read with no verified "since".
const fn reads_agents(precedence: Precedence) -> Option<AgentsStandardRead> {
    Some(AgentsStandardRead {
        since: None,
        precedence,
    })
}

/// The registry. Every row's global facts (`relative_skills_dir`,
/// `former_relative_skills_dirs`, `reads_agents_standard_globally`,
/// `also_reads`) cite `docs/design-inputs/2026-11-02-harness-global-skills-dirs.md`
/// ("the brief") by § and row / I-number; `registry_matches_the_brief_snapshot`
/// pins them, so a registry edit without a brief edit fails.
static TOOL_ADAPTERS: &[ToolAdapter] = &[
    ToolAdapter {
        id: ToolId::AgentsStandard,
        display_name: ".agents/skills",
        // Brief § 1 row 1: a convention dir, not a harness.
        relative_skills_dir: AGENTS_STANDARD_DIR,
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".agents"],
        project_relative_skills_dir: Some(".agents/skills"),
        group_label: Some(".agents/skills (9 harnesses)"),
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::Cursor,
        display_name: "Cursor",
        // Brief § 1 row 2 (S3): `~/.cursor/skills` + `~/.agents/skills`,
        // compat `~/.claude/skills` and `~/.codex/skills`; precedence undocumented.
        relative_skills_dir: ".cursor/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".cursor"],
        project_relative_skills_dir: Some(".agents/skills"),
        group_label: None,
        group: Some(VirtualGroup::AgentsStandard),
        // Copy-only until Cursor IDE 2.5 (Feb 2026) fixed symlink discovery
        // under ~/.cursor/skills; flipped in v-next ticket 38.
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::Unknown),
        also_reads: &[".claude/skills", ".codex/skills"],
    },
    ToolAdapter {
        id: ToolId::ClaudeCode,
        display_name: "Claude Code",
        // Brief § 1 row 3 (S2): does not read `~/.agents/skills` (I12: never group).
        relative_skills_dir: ".claude/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".claude"],
        project_relative_skills_dir: Some(".claude/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::Codex,
        display_name: "Codex",
        // Brief § 3 / § 1 row 4 (S1, S4, S5), I2 option (a): `~/.agents/skills`
        // is Codex's user dir since rust-v0.95.0, so Codex deploys there as a
        // global sharer of the AGENTS-standard entry. `~/.codex/skills` is
        // "deprecated, kept for backward compatibility" and a same-name skill
        // in both shows twice — existing targets relocate out of it.
        relative_skills_dir: AGENTS_STANDARD_DIR,
        former_relative_skills_dirs: &[".codex/skills"],
        former_keys: &[],
        relative_detect_dirs: &[".codex"],
        project_relative_skills_dir: Some(".agents/skills"),
        group_label: None,
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
        reads_agents_standard_globally: Some(AgentsStandardRead {
            since: Some("rust-v0.95.0"),
            precedence: Precedence::BothShown,
        }),
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::OpenCode,
        display_name: "OpenCode",
        // Brief § 1 row 5 (S8): own dir, `~/.claude/skills` (I1), `~/.agents/skills`.
        relative_skills_dir: ".config/opencode/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".config/opencode"],
        project_relative_skills_dir: Some(".agents/skills"),
        group_label: None,
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::Unknown),
        also_reads: &[".claude/skills"],
    },
    ToolAdapter {
        id: ToolId::Antigravity,
        display_name: "Antigravity",
        // Brief § 2 / I3 (S9): every surface documents `~/.gemini/config/skills`;
        // our former `global_skills` appears in no vendor doc and the IDE's
        // legacy `~/.gemini/antigravity/skills` is superseded. Detect stays
        // `.gemini/antigravity` — `.gemini/config` would be a skills-only
        // footprint. Does not read `~/.agents/skills` (I12). The CLI also reads
        // `~/.gemini/skills` (Gemini CLI's dir).
        relative_skills_dir: ".gemini/config/skills",
        former_relative_skills_dirs: &[
            ".gemini/antigravity/global_skills",
            ".gemini/antigravity/skills",
        ],
        former_keys: &[],
        relative_detect_dirs: &[".gemini/antigravity"],
        project_relative_skills_dir: Some(".agents/skills"),
        group_label: None,
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[".gemini/skills"],
    },
    ToolAdapter {
        id: ToolId::Amp,
        display_name: "Amp",
        // Brief § 1 row 7 (S11, S12): `~/.config/agents/skills` first, then
        // `~/.agents/skills`, `~/.config/amp/skills`, `~/.claude/skills` (I1);
        // first name wins in that order. The tool itself lives in ~/.config/amp.
        relative_skills_dir: ".config/agents/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".config/amp"],
        project_relative_skills_dir: Some(".agents/skills"),
        group_label: None,
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::OwnWins),
        also_reads: &[".claude/skills"],
    },
    ToolAdapter {
        id: ToolId::KimiCli,
        display_name: "Kimi Code CLI",
        // Brief § 2 / I4 (S13, S14): Kimi Code CLI reads `~/.kimi-code/skills`
        // + `~/.agents/skills`. The former `~/.config/agents/skills` is Amp's
        // (and Crush's) live dir — relocation moves only Kimi's rows out of it
        // and never removes an artifact another row still records there
        // (`core::target_relocation`). Detect keeps `.kimi` (legacy kimi-cli,
        // which reads the first existing of `~/.claude/skills` /
        // `~/.codex/skills` and `~/.config/agents/skills`).
        relative_skills_dir: ".kimi-code/skills",
        former_relative_skills_dirs: &[".config/agents/skills"],
        former_keys: &[],
        relative_detect_dirs: &[".kimi-code", ".kimi"],
        project_relative_skills_dir: Some(".agents/skills"),
        group_label: None,
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::Unknown),
        also_reads: &[".claude/skills", ".codex/skills", ".config/agents/skills"],
    },
    ToolAdapter {
        id: ToolId::Augment,
        display_name: "Augment",
        // Brief § 1 row 9 (S20): `~/.augment/skills` (highest) > workspace >
        // `~/.claude/skills` (I1) > `~/.agents/skills`. `~/.augment/rules` is
        // Augment's *rules* dir; Skills Hub synced there until 1.2.18.
        relative_skills_dir: ".augment/skills",
        former_relative_skills_dirs: &[".augment/rules"],
        former_keys: &[],
        relative_detect_dirs: &[".augment"],
        project_relative_skills_dir: Some(".augment/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::OwnWins),
        also_reads: &[".claude/skills"],
    },
    ToolAdapter {
        id: ToolId::OpenClaw,
        display_name: "OpenClaw",
        // Brief § 1 row 10 (S33): `~/.agents/skills` (rank 3) beats
        // `<state-dir>/skills` = `~/.openclaw/skills` (rank 4). Clawdbot and
        // MoltBot are its former names (rows 41 / 44, I10): their dirs are
        // former dirs, their keys former keys.
        relative_skills_dir: ".openclaw/skills",
        former_relative_skills_dirs: &[".clawdbot/skills", ".moltbot/skills"],
        former_keys: &["clawdbot", "moltbot"],
        relative_detect_dirs: &[".openclaw"],
        project_relative_skills_dir: Some("skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::AgentsWins),
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::Copaw,
        display_name: "Copaw",
        // Brief § 1 row 11: LIKELY only (DeepWiki) — `~/.agents/skills` UNKNOWN.
        relative_skills_dir: ".copaw/skill_pool",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".copaw"],
        project_relative_skills_dir: Some(".copaw/skill_pool"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::Cline,
        display_name: "Cline",
        // Brief § 2 / I5 (S15, S16): source scans `~/.cline/skills` then
        // `~/.agents/skills`; collision rule undocumented; "since v3.67.0" is
        // secondary, so not recorded.
        relative_skills_dir: ".cline/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".cline"],
        project_relative_skills_dir: Some(".agents/skills"),
        group_label: None,
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::Unknown),
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::CodeBuddy,
        display_name: "CodeBuddy",
        // Brief § 1 row 13 (S38): no `~/.agents/skills` per docs (I12).
        relative_skills_dir: ".codebuddy/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".codebuddy"],
        project_relative_skills_dir: Some(".codebuddy/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::CommandCode,
        display_name: "Command Code",
        // Brief § 1 row 14 (S41): `~/.commandcode` > `~/.agents`.
        relative_skills_dir: ".commandcode/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".commandcode"],
        project_relative_skills_dir: Some(".commandcode/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::OwnWins),
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::Continue,
        display_name: "Continue",
        // Brief § 1 row 15 (S43): LIKELY — no source says it reads
        // `~/.agents/skills`; reads `~/.claude/skills` (I1).
        relative_skills_dir: ".continue/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".continue"],
        project_relative_skills_dir: Some(".continue/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[".claude/skills"],
    },
    ToolAdapter {
        id: ToolId::Crush,
        display_name: "Crush",
        // Brief § 1 row 16 (S21): `~/.config/agents/skills` (Amp's dir),
        // own dir, `~/.agents/skills`, `~/.claude/skills` (I1); order only.
        relative_skills_dir: ".config/crush/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".config/crush"],
        project_relative_skills_dir: Some(".crush/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::Unknown),
        also_reads: &[".config/agents/skills", ".claude/skills"],
    },
    ToolAdapter {
        id: ToolId::Junie,
        display_name: "Junie",
        // Brief § 1 row 17 (S28).
        relative_skills_dir: ".junie/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".junie"],
        project_relative_skills_dir: Some(".junie/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::Unknown),
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::IflowCli,
        display_name: "iFlow CLI",
        // Brief § 1 row 18 (S35): no `~/.agents/skills` (I12).
        relative_skills_dir: ".iflow/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".iflow"],
        project_relative_skills_dir: Some(".iflow/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::KiroCli,
        display_name: "Kiro CLI",
        // Brief § 1 row 19 (S26): no `~/.agents/skills` (I12).
        relative_skills_dir: ".kiro/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".kiro"],
        project_relative_skills_dir: Some(".kiro/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::Kode,
        display_name: "Kode",
        // Brief § 1 row 20 (S47): LIKELY; `~/.agents/skills` UNKNOWN.
        relative_skills_dir: ".kode/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".kode"],
        project_relative_skills_dir: Some(".kode/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::McpJam,
        display_name: "MCPJam",
        // Brief § 1 row 21: UNKNOWN (no vendor doc found).
        relative_skills_dir: ".mcpjam/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".mcpjam"],
        project_relative_skills_dir: Some(".mcpjam/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::MistralVibe,
        display_name: "Mistral Vibe",
        // Brief § 1 row 22 (S29).
        relative_skills_dir: ".vibe/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".vibe"],
        project_relative_skills_dir: Some(".vibe/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::Unknown),
        also_reads: &[],
    },
    ToolAdapter {
        // Key stays `mux`: it is stored in target rows, project tool rows,
        // assignments and two settings rows; renaming it buys nothing the
        // display name does not (ticket 34 decision).
        id: ToolId::Mux,
        display_name: "Xum",
        // Brief § 5 / I7 (S42): renamed Xum — `~/.xum/skills` > `~/.agents/skills`
        // > `~/.mux/skills` (read-only legacy, lowest). Detect both roots.
        relative_skills_dir: ".xum/skills",
        former_relative_skills_dirs: &[".mux/skills"],
        former_keys: &[],
        relative_detect_dirs: &[".xum", ".mux"],
        // Xum's project dir is `.xum/skills` (brief § 5 note); project dir
        // held at `.mux/skills` by recorded deferral (ticket 34 Result) — no
        // project-assignment relocation exists; BACKLOG #87, opened by the
        // orchestrator.
        project_relative_skills_dir: Some(".mux/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::OwnWins),
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::OpenClaude,
        display_name: "OpenClaude IDE",
        // Brief § 1 row 24: UNKNOWN.
        relative_skills_dir: ".openclaude/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".openclaude"],
        project_relative_skills_dir: Some(".openclaude/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::OpenHands,
        display_name: "OpenHands",
        // Brief § 1 row 25 / I8 (S27): `~/.agents/skills` is the recommended
        // dir and beats the legacy `.openhands/skills` — OpenHands deploys
        // there as a global sharer of the AGENTS-standard entry; existing
        // targets relocate out of the legacy dir. Project dir unchanged
        // (brief § 5 note: `.agents/skills` recommended there too).
        relative_skills_dir: AGENTS_STANDARD_DIR,
        former_relative_skills_dirs: &[".openhands/skills"],
        former_keys: &[],
        relative_detect_dirs: &[".openhands"],
        project_relative_skills_dir: Some(".openhands/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::AgentsWins),
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::Pi,
        display_name: "Pi",
        // Brief § 1 row 26 (S18, S19): dirs VERIFIED, order LIKELY — precedence
        // recorded as unknown.
        relative_skills_dir: ".pi/agent/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".pi"],
        project_relative_skills_dir: Some(".pi/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::Unknown),
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::Qoder,
        display_name: "Qoder",
        // Brief § 1 row 27 (S37): no `~/.agents/skills` (I12).
        relative_skills_dir: ".qoder/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".qoder"],
        project_relative_skills_dir: Some(".qoder/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::QoderWork,
        display_name: "QoderWork",
        // Brief § 1 row 28 (S49): LIKELY; `~/.agents/skills` UNKNOWN.
        relative_skills_dir: ".qoderwork/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".qoderwork"],
        project_relative_skills_dir: Some(".qoderwork/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::QwenCode,
        display_name: "Qwen Code",
        // Brief § 1 row 29 (S25): `~/.qwen/skills` wins.
        relative_skills_dir: ".qwen/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".qwen"],
        project_relative_skills_dir: Some(".qwen/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::OwnWins),
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::Trae,
        display_name: "Trae",
        // Brief § 1 row 30 (S36): LIKELY; `~/.agents/skills` UNKNOWN.
        relative_skills_dir: ".trae/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".trae"],
        project_relative_skills_dir: Some(".trae/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::TraeCn,
        display_name: "Trae CN",
        // Brief § 1 row 31 (S36): LIKELY; `~/.agents/skills` UNKNOWN.
        relative_skills_dir: ".trae-cn/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".trae-cn"],
        project_relative_skills_dir: Some(".trae/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::Zencoder,
        display_name: "Zencoder",
        // Brief § 1 row 32 (S46): LIKELY; `~/.agents/skills` UNKNOWN.
        relative_skills_dir: ".zencoder/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".zencoder"],
        project_relative_skills_dir: Some(".zencoder/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::Neovate,
        display_name: "Neovate",
        // Brief § 1 row 33 (S44): dir VERIFIED; `~/.agents/skills` UNKNOWN.
        relative_skills_dir: ".neovate/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".neovate"],
        project_relative_skills_dir: Some(".neovate/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::Pochi,
        display_name: "Pochi",
        // Brief § 1 row 34 (S45): `.agents` paths are fallbacks; own dir wins.
        relative_skills_dir: ".pochi/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".pochi"],
        project_relative_skills_dir: Some(".pochi/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: Some(AgentsStandardRead {
            since: Some("2a9ff73 (#1360)"),
            precedence: Precedence::OwnWins,
        }),
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::AdaL,
        display_name: "AdaL",
        // Brief § 1 row 35: UNKNOWN (no vendor doc found).
        relative_skills_dir: ".adal/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".adal"],
        project_relative_skills_dir: Some(".adal/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::KiloCode,
        display_name: "Kilo Code",
        // Brief § 5 / I6 (S24): `~/.kilo/skills` + `~/.agents/skills`; whether
        // `~/.kilocode/skills` is still read is UNKNOWN, so existing targets
        // relocate. Detect both roots. `~/.claude/skills` only when Claude
        // compat is switched on — not a default cross-read. Project dir
        // `.kilo/skills` per the brief; project dir held at `.kilocode/skills`
        // by recorded deferral (ticket 34 Result) — no project-assignment
        // relocation exists; BACKLOG #87, opened by the orchestrator.
        relative_skills_dir: ".kilo/skills",
        former_relative_skills_dirs: &[".kilocode/skills"],
        former_keys: &[],
        relative_detect_dirs: &[".kilo", ".kilocode"],
        project_relative_skills_dir: Some(".kilocode/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::Unknown),
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::RooCode,
        display_name: "Roo Code",
        // Brief § 1 row 37 (S22, S23): `~/.roo` > `~/.agents`, since PR #11181.
        relative_skills_dir: ".roo/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".roo"],
        project_relative_skills_dir: Some(".roo/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: Some(AgentsStandardRead {
            since: Some("PR #11181"),
            precedence: Precedence::OwnWins,
        }),
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::Goose,
        display_name: "Goose",
        // Brief § 1 row 38 / I8 (S10): `~/.agents/skills` is the recommended
        // dir; `~/.config/goose/skills` is backward compat — Goose deploys to
        // the AGENTS-standard dir as a global sharer; existing targets
        // relocate. Also reads `~/.claude/skills` (I1). Project dir unchanged
        // (brief § 5 note).
        relative_skills_dir: AGENTS_STANDARD_DIR,
        former_relative_skills_dirs: &[".config/goose/skills"],
        former_keys: &[],
        relative_detect_dirs: &[".config/goose"],
        project_relative_skills_dir: Some(".goose/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::Unknown),
        also_reads: &[".claude/skills"],
    },
    ToolAdapter {
        id: ToolId::GeminiCli,
        display_name: "Gemini CLI",
        // Brief § 1 row 39 (S30, S31): within the user tier `~/.agents/skills`
        // beats `~/.gemini/skills`; since PR #18151.
        relative_skills_dir: ".gemini/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".gemini"],
        project_relative_skills_dir: Some(".agents/skills"),
        group_label: None,
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
        reads_agents_standard_globally: Some(AgentsStandardRead {
            since: Some("PR #18151"),
            precedence: Precedence::AgentsWins,
        }),
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::GithubCopilot,
        display_name: "GitHub Copilot",
        // Brief § 1 row 40 (S32, S34).
        relative_skills_dir: ".copilot/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".copilot"],
        project_relative_skills_dir: Some(".agents/skills"),
        group_label: None,
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::Unknown),
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::Droid,
        display_name: "Droid",
        // Brief § 1 row 42 (S39): one "personal" tier, rule undocumented.
        relative_skills_dir: ".factory/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".factory"],
        project_relative_skills_dir: Some(".factory/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::Unknown),
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::Windsurf,
        display_name: "Windsurf",
        // Brief § 1 row 43 / I9 (S40): held. The docs moved to docs.devin.ai
        // and name `~/.config/devin/skills` (candidate new dir) + `~/.agents/skills`;
        // `~/.codeium/windsurf/skills` still being read is LIKELY only, so the
        // dir stays until verified against a current build.
        relative_skills_dir: ".codeium/windsurf/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".codeium/windsurf"],
        project_relative_skills_dir: Some(".windsurf/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::Unknown),
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::HermesAgent,
        display_name: "Hermes Agent",
        // Brief § 1 row 45 (S50): `~/.agents/skills` only when listed in
        // `skills.external_dirs` — opt-in, not a default read (I12).
        relative_skills_dir: ".hermes/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".hermes"],
        project_relative_skills_dir: Some(".hermes/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::DeepseekHarness,
        display_name: "DeepSeek Harness",
        // Brief § 4 / I13 (S51): `~/.dsh/skills` (rank 400) before
        // `~/.agents/skills` (rank 500).
        relative_skills_dir: ".dsh/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".dsh"],
        project_relative_skills_dir: Some(".dsh/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::OwnWins),
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::Zcode,
        display_name: "ZCode",
        // Brief § 4 / I13 (S52, S53): `~/.zcode` before `~/.agents`, first
        // same-name skill wins.
        relative_skills_dir: ".zcode/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".zcode"],
        project_relative_skills_dir: Some(".zcode/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::OwnWins),
        also_reads: &[],
    },
    ToolAdapter {
        id: ToolId::Codewhale,
        display_name: "Codewhale",
        // Brief § 4 / I13 (S54, S55): `.codewhale` > `.agents` > `.claude` (I1)
        // > legacy `.deepseek`. Never detected on `~/.deepseek` — a read
        // fallback too generic to mean Codewhale is installed.
        relative_skills_dir: ".codewhale/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".codewhale"],
        project_relative_skills_dir: Some(".codewhale/skills"),
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: reads_agents(Precedence::OwnWins),
        also_reads: &[".claude/skills"],
    },
    ToolAdapter {
        id: ToolId::WorkBuddy,
        display_name: "WorkBuddy",
        // Brief § 4 / I13 (S56, S57): `~/.workbuddy/skills` per secondary
        // sources only; no `~/.agents/skills` read known. **Global-only**:
        // the project dir is unresolved (conflicting secondary claims), so
        // the Tool is absent from every project catalog.
        relative_skills_dir: ".workbuddy/skills",
        former_relative_skills_dirs: &[],
        former_keys: &[],
        relative_detect_dirs: &[".workbuddy"],
        project_relative_skills_dir: None,
        group_label: None,
        group: None,
        supports_symlink: true,
        reads_agents_standard_globally: None,
        also_reads: &[],
    },
];

/// Every known tool adapter, in registry (presentation) order.
pub fn default_tool_adapters() -> &'static [ToolAdapter] {
    TOOL_ADAPTERS
}

/// Tools can share the same global skills directory (the AGENTS-standard
/// entry with Codex, Goose and OpenHands; under the preference, every
/// AGENTS-standard reader). Which tools do depends on `layout`. Use this to
/// coordinate UI warnings and avoid duplicate filesystem operations.
pub fn adapters_sharing_skills_dir(
    adapter: &ToolAdapter,
    layout: GlobalLayout,
) -> Vec<&'static ToolAdapter> {
    let dir = adapter.global_relative_skills_dir(layout);
    TOOL_ADAPTERS
        .iter()
        .filter(|a| a.global_relative_skills_dir(layout) == dir)
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

/// The registered Tool a retired `key` was a former name of
/// ([`ToolAdapter::former_keys`]); `None` for a registered or unknown key.
pub fn adapter_succeeding_key(key: &str) -> Option<&'static ToolAdapter> {
    if adapter_by_key(key).is_some() {
        return None;
    }
    TOOL_ADAPTERS
        .iter()
        .find(|adapter| adapter.former_keys.contains(&key))
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

/// The tool's own global skills directory under `home` (the registry fact;
/// where it *deploys* is [`global_skills_dir_in`]).
pub fn skills_dir_in(home: &Path, adapter: &ToolAdapter) -> PathBuf {
    home.join(adapter.relative_skills_dir)
}

/// The global skills directory under `home` the tool deploys to under
/// `layout` ([`ToolAdapter::global_relative_skills_dir`]).
pub fn global_skills_dir_in(home: &Path, adapter: &ToolAdapter, layout: GlobalLayout) -> PathBuf {
    home.join(adapter.global_relative_skills_dir(layout))
}

/// The global dirs a target of this tool may sit in that are **not** where
/// it deploys under `layout`: its former dirs, its own dir when the layout
/// moved it to the AGENTS-standard dir, and the AGENTS-standard dir when an
/// AGENTS-standard reader is back on its own dir. Target relocation moves
/// this tool's rows out of exactly these; deduplicated, registry order.
pub fn superseded_skills_dirs_in(
    home: &Path,
    adapter: &ToolAdapter,
    layout: GlobalLayout,
) -> Vec<PathBuf> {
    let effective = adapter.global_relative_skills_dir(layout);
    let mut dirs: Vec<&'static str> = adapter.former_relative_skills_dirs.to_vec();
    dirs.push(adapter.relative_skills_dir);
    if adapter.reads_agents_standard_globally.is_some() {
        dirs.push(AGENTS_STANDARD_DIR);
    }
    let mut out: Vec<PathBuf> = Vec::new();
    for dir in dirs {
        let path = home.join(dir);
        if dir != effective && !out.contains(&path) {
            out.push(path);
        }
    }
    out
}

/// The tool's former global skills directories under `home` (see
/// [`ToolAdapter::former_relative_skills_dirs`]).
pub fn former_skills_dirs_in(home: &Path, adapter: &ToolAdapter) -> Vec<PathBuf> {
    adapter
        .former_relative_skills_dirs
        .iter()
        .map(|dir| home.join(dir))
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
    // A former skills dir counts: an artifact Skills Hub created there is
    // still its own to remove.
    let inside = TOOL_ADAPTERS.iter().any(|adapter| {
        path.starts_with(skills_dir_in(home, adapter))
            || former_skills_dirs_in(home, adapter)
                .iter()
                .any(|dir| path.starts_with(dir))
    });
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
///
/// A former skills dir counts as a skills path too: `~/.codex` holding only
/// `skills/` is what `gh skill install --agent codex` leaves behind (brief
/// § 3), not Codex, even though Codex now deploys to `~/.agents/skills`.
pub fn is_installed_in(home: &Path, adapter: &ToolAdapter) -> bool {
    let mut skills_paths = vec![skills_dir_in(home, adapter)];
    skills_paths.extend(former_skills_dirs_in(home, adapter));
    detect_dirs_in(home, adapter).iter().any(|detect_dir| {
        detect_dir.exists()
            && (adapter.as_virtual_group().is_some()
                || !skills_paths
                    .iter()
                    .any(|skills_dir| is_skills_only_footprint(detect_dir, skills_dir)))
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
