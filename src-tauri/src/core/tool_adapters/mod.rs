use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::core::environment::{HostOs, HostRoot, HostRoots};
use crate::core::errors::SignalError;
use crate::core::placement::InstalledSet;
use crate::core::skill_discovery::{manifest_validity, InvalidManifestReason};

pub mod catalog;
pub mod reads;

pub use catalog::{global_tool_entries, installed_keys, project_tool_entries, ToolCatalogEntry};
use reads::{
    AlsoRead, DuplicateBehaviour, Precedence, ReadFacts, ReadMode, ScopeReads,
    SharedRead::{Conditional, No, OptIn, Unknown, Yes},
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
    Clawdbot,
    Droid,
    Windsurf,
    Moltbot,
    HermesAgent,
    // round 20 ticket 05 rows
    DeepseekHarness,
    Zcode,
    Codewhale,
    WorkBuddy,
    // round 20 ticket 07 rows
    LettaCode,
    MuseCode,
    Poolside,
    Dirac,
    CodeStudio,
    CodeArtsAgent,
    QoderCn,
    QoderCnCli,
    Kimchi,
    AstrBot,
    OhMyPi,
    Stakpak,
    // round 20 ticket 06 rows
    GrokBuild,
    DeepAgentsCode,
    Reasonix,
    CortexCode,
    RovoDev,
    IbmBob,
    ForgeCode,
    Firebender,
    MiniMaxCode,
    PositAssistant,
    AiderDesk,
    AutohandCode,
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
            // round 20 ticket 05 rows
            ToolId::DeepseekHarness => "deepseek_harness",
            ToolId::Zcode => "zcode",
            ToolId::Codewhale => "codewhale",
            ToolId::WorkBuddy => "workbuddy",
            // round 20 ticket 07 rows
            ToolId::LettaCode => "letta_code",
            ToolId::MuseCode => "muse_code",
            ToolId::Poolside => "poolside",
            ToolId::Dirac => "dirac",
            ToolId::CodeStudio => "code_studio",
            ToolId::CodeArtsAgent => "codearts_agent",
            ToolId::QoderCn => "qoder_cn",
            ToolId::QoderCnCli => "qoder_cn_cli",
            ToolId::Kimchi => "kimchi",
            ToolId::AstrBot => "astrbot",
            ToolId::OhMyPi => "oh_my_pi",
            ToolId::Stakpak => "stakpak",
            // round 20 ticket 06 rows
            ToolId::GrokBuild => "grok_build",
            ToolId::DeepAgentsCode => "deep_agents_code",
            ToolId::Reasonix => "reasonix",
            ToolId::CortexCode => "cortex_code",
            ToolId::RovoDev => "rovo_dev",
            ToolId::IbmBob => "ibm_bob",
            ToolId::ForgeCode => "forgecode",
            ToolId::Firebender => "firebender",
            ToolId::MiniMaxCode => "minimax_code",
            ToolId::PositAssistant => "posit_assistant",
            ToolId::AiderDesk => "aider_desk",
            ToolId::AutohandCode => "autohand_code",
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
    /// `skills_dir` was corrected (Augment: `.augment/rules`, its rules dir;
    /// Kilo: `.kilocode/skills`, round 20 #108). Only two rules read it: the
    /// deletion rule (`ensure_path_within_tool_dirs`, and the managed folders
    /// a row's old artifact is removed in, `placement::managed_global_dirs`),
    /// so a Deployment the redeploy (`core::redeploy`) moves out of it can be
    /// removed there; and onboarding's scan of former dirs. Nothing is ever
    /// synced into one, and nothing decides placement from it.
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
    /// `false` forces copy mode in every sync path. Every current entry but
    /// Deep Agents Code (which rejects links resolving outside its skill
    /// roots) is `true`; the capability is also the lever for a tool that
    /// regresses (Cursor was `false` until IDE 2.5 / CLI 2026-06 fixed discovery).
    pub supports_symlink: bool,
    /// Which shared skills folders the harness reads, what wins, what it
    /// does with a duplicate, and which other folders it also reads — each
    /// value citing its brief row ([`reads`]). Not the deployment decision:
    /// that is `group` (a test keeps the two consistent).
    pub reads: ReadFacts,
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
        // Read facts: brief § Part 2, row "(omitted)".
        // Not a harness: the AGENTS convention folder itself, which the brief's
        // matrix omits. It reads only itself, so it is never a duplicate reader.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "Cursor".
        // VERIFIED T75. The brief's A-only caveat (local agents only, not Cloud
        // Agents / SSH / self-hosted workers) is about surfaces, not this read.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::ShowsAll,
            also_reads: &[AlsoRead {
                dir: home(".codex/skills"),
                mode: ReadMode::Default,
            }],
        },
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
        // Read facts: brief § Part 2, row "Claude Code".
        // VERIFIED B02:S2. `~/.claude/skills` is its own folder. The managed-settings
        // dir is outside home and not modelled.
        reads: ReadFacts {
            global: ScopeReads {
                agents: No,
                claude: Yes,
            },
            project: ScopeReads {
                agents: No,
                claude: Yes,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "Codex".
        // VERIFIED T49, B02:S4–S5. Its own `~/.codex/skills` is deprecated but still
        // read; `/etc/codex/skills` is outside home and not modelled.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Both,
            duplicates: DuplicateBehaviour::ShowsAll,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "OpenCode".
        // VERIFIED T64 ("up to 3 copies").
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::ShowsAll,
            also_reads: &[],
        },
    },
    ToolAdapter {
        id: ToolId::Antigravity,
        display_name: "Antigravity",
        // Brief § Part 1, row "Antigravity CLI" / Q12 (round 20 #108):
        // `~/.gemini/config/skills`, shared by the App, IDE and CLI. The
        // add-skill path `~/.gemini/antigravity/global_skills` is not
        // documented by current releases. The CLI's root
        // `.gemini/antigravity-cli` detects the row (the CLI is its alias).
        skills_dir: home(".gemini/config/skills"),
        former_skills_dirs: &[home(".gemini/antigravity/global_skills")],
        detect_dirs: &[home(".gemini/antigravity"), home(".gemini/antigravity-cli")],
        project_skills_dir: Some(".agents/skills"),
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
        // Read facts: brief § Part 2, row "Antigravity".
        // VERIFIED T71. The CLI (an alias of this row) also reads Gemini CLI's
        // folder; the IDE does not.
        reads: ReadFacts {
            global: ScopeReads {
                agents: No,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[
                AlsoRead {
                    dir: home(".gemini/skills"),
                    mode: ReadMode::Conditional("Antigravity CLI only"),
                },
                AlsoRead {
                    dir: home(".gemini/antigravity-cli/skills"),
                    mode: ReadMode::Conditional("Antigravity CLI only"),
                },
                AlsoRead {
                    dir: home(".gemini/antigravity/skills"),
                    mode: ReadMode::Legacy,
                },
            ],
        },
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
        // Read facts: brief § Part 2, row "Amp".
        // VERIFIED T77. Rank 1 is its own `~/.config/agents/skills`, which wins over
        // `~/.agents` (first name wins). The disable settings keep these default reads.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Own,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[
                AlsoRead {
                    dir: home(".config/agents/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".config/amp/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".claude/plugins/cache"),
                    mode: ReadMode::Default,
                },
            ],
        },
    },
    ToolAdapter {
        id: ToolId::KimiCli,
        display_name: "Kimi Code CLI",
        // Q12 (round 20 #108): `~/.kimi-code/skills`, under the tool's config
        // root. The add-skill path `~/.config/agents/skills` (shared with Amp)
        // is read only by legacy `kimi-cli`: a former dir.
        // The tool's config root is ~/.kimi-code (kimi.com/code docs) or
        // ~/.kimi on earlier releases (kimi-cli.com docs); either counts.
        // `KIMI_CODE_HOME` relocates the root; env-var homes are not honoured
        // (a Finder-launched app does not see shell env).
        skills_dir: home(".kimi-code/skills"),
        former_skills_dirs: &[home(".config/agents/skills")],
        detect_dirs: &[home(".kimi-code"), home(".kimi")],
        project_skills_dir: Some(".agents/skills"),
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
        // Read facts: brief § Part 2, row "Kimi Code CLI".
        // VERIFIED T50, B02:S14. Current Kimi Code reads `~/.agents/skills`; every
        // other read, this row's own folder included, is legacy `kimi-cli`'s.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Conditional("legacy kimi-cli only, when ~/.kimi/skills is absent"),
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[
                AlsoRead {
                    dir: home(".config/agents/skills"),
                    mode: ReadMode::Legacy,
                },
                AlsoRead {
                    dir: home(".kimi/skills"),
                    mode: ReadMode::Legacy,
                },
                AlsoRead {
                    dir: home(".codex/skills"),
                    mode: ReadMode::Legacy,
                },
            ],
        },
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
        // Read facts: brief § Part 2, row "Augment".
        // VERIFIED T69 (`~/.augment` > workspace > `~/.claude` > `~/.agents`).
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Own,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "OpenClaw".
        // VERIFIED T51 (`~/.agents` rank 3 beats `~/.openclaw` rank 4).
        reads: ReadFacts {
            global: ScopeReads {
                agents: Conditional(
                    "default state dir only: a non-default OPENCLAW_STATE_DIR drops it",
                ),
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Shared,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "Copaw → QwenPaw".
        // VERIFIED T46: only configured extra skill dirs; the primary pool wins.
        reads: ReadFacts {
            global: ScopeReads {
                agents: No,
                claude: No,
            },
            project: ScopeReads {
                agents: No,
                claude: No,
            },
            precedence: Precedence::Own,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "Cline".
        // VERIFIED T70; precedence undocumented (scan order only).
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "CodeBuddy".
        // VERIFIED T78 for `.agents`; the `.claude` absence is LIKELY (third party).
        reads: ReadFacts {
            global: ScopeReads {
                agents: No,
                claude: Unknown,
            },
            project: ScopeReads {
                agents: No,
                claude: Unknown,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "Command Code".
        // VERIFIED T52 (`.commandcode` > `.agents`; the shadowed copy is reported).
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Own,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "Continue".
        // LIKELY B02:S43 only.
        reads: ReadFacts::UNKNOWN,
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
        // Read facts: brief § Part 2, row "Crush".
        // VERIFIED T76. Also reads Amp's `~/.config/agents/skills`; Windows adds
        // `%LOCALAPPDATA%\agents\skills` and project `.cursor/skills` (not modelled).
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[AlsoRead {
                dir: home(".config/agents/skills"),
                mode: ReadMode::Default,
            }],
        },
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
        // Read facts: brief § Part 2, row "Junie".
        // VERIFIED T53. Project `.claude` is an import offer, not a read.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Conditional("trusted projects only"),
                claude: No,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "iFlow CLI".
        // VERIFIED T67.
        reads: ReadFacts {
            global: ScopeReads {
                agents: No,
                claude: No,
            },
            project: ScopeReads {
                agents: No,
                claude: No,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "Kiro CLI".
        // VERIFIED T68.
        reads: ReadFacts {
            global: ScopeReads {
                agents: No,
                claude: No,
            },
            project: ScopeReads {
                agents: No,
                claude: No,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "Kode".
        // LIKELY B02:S47 only.
        reads: ReadFacts::UNKNOWN,
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
        // Read facts: brief § Part 2, row "MCPJam".
        // VERIFIED T44.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "Mistral Vibe".
        // VERIFIED T54.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Conditional("trusted projects only"),
                claude: No,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
    },
    ToolAdapter {
        id: ToolId::Mux,
        // Mux is now Xum (relabel only; the key stays `mux`, round 19).
        display_name: "Xum",
        // Q12 (round 20 #108): `~/.xum/skills`, Xum's current dir; the
        // add-skill path `~/.mux/skills` is its legacy, lowest-precedence one.
        skills_dir: home(".xum/skills"),
        former_skills_dirs: &[home(".mux/skills")],
        detect_dirs: &[home(".mux"), home(".xum")],
        project_skills_dir: Some(".mux/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2, row "Mux → Xum".
        // VERIFIED B02:S42 (`~/.xum` > `~/.agents` > this row's legacy `~/.mux`).
        // `precedence` is the harness's own fact (its own current dir, `~/.xum`,
        // wins); the legacy `~/.mux/skills` is still read, lowest.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Unknown,
            },
            project: ScopeReads {
                agents: Unknown,
                claude: Unknown,
            },
            precedence: Precedence::Own,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[
                AlsoRead {
                    dir: home(".xum/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".mux/skills"),
                    mode: ReadMode::Default,
                },
            ],
        },
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
        // Read facts: brief § Part 2, row "OpenClaude IDE".
        // UNKNOWN: no vendor doc found.
        reads: ReadFacts::UNKNOWN,
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
        // Read facts: brief § Part 2, row "OpenHands".
        // VERIFIED T60 (`.agents` beats the legacy `.openhands` dirs, still read).
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Shared,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "Pi".
        // VERIFIED T59 (first discovered wins, with a warning); the order is LIKELY.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "Qoder".
        // VERIFIED T20 (own folder wins; a duplicate is shown once).
        reads: ReadFacts {
            global: ScopeReads {
                agents: Conditional(
                    "the Qoder desktop app only; the IDE and CLI read only ~/.qoder/skills",
                ),
                claude: No,
            },
            project: ScopeReads {
                agents: No,
                claude: No,
            },
            precedence: Precedence::Own,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "QoderWork".
        // VERIFIED T21.
        reads: ReadFacts {
            global: ScopeReads {
                agents: No,
                claude: No,
            },
            project: ScopeReads {
                agents: No,
                claude: No,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "Qwen Code".
        // VERIFIED T65 (`~/.qwen` wins).
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: OptIn,
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Own,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "Trae".
        // VERIFIED T48 for the project toggle; the global cells read "N / ?" and "?".
        reads: ReadFacts {
            global: ScopeReads {
                agents: Unknown,
                claude: Unknown,
            },
            project: ScopeReads {
                agents: OptIn,
                claude: Unknown,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "Trae CN".
        // VERIFIED T48 ("as Trae").
        reads: ReadFacts {
            global: ScopeReads {
                agents: Unknown,
                claude: Unknown,
            },
            project: ScopeReads {
                agents: OptIn,
                claude: Unknown,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "Zencoder (+ Zenflow)".
        // VERIFIED T38. This row's `.zencoder/skills` is deprecated but still read.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Unknown,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "Neovate".
        // VERIFIED T47 (`~/.neovate` loads after `~/.claude`; later wins).
        reads: ReadFacts {
            global: ScopeReads {
                agents: No,
                claude: Yes,
            },
            project: ScopeReads {
                agents: No,
                claude: Yes,
            },
            precedence: Precedence::Own,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "Pochi".
        // VERIFIED T66.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Own,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "AdaL".
        // VERIFIED T45 for its own folder only; every shared read is unknown.
        reads: ReadFacts::UNKNOWN,
    },
    ToolAdapter {
        id: ToolId::KiloCode,
        display_name: "Kilo Code",
        // Q12 (round 20 #108): `~/.kilo/skills` is current; the add-skill
        // path `~/.kilocode/skills` is legacy (whether it is still read is
        // unknown). Its root `.kilo` detects it too.
        skills_dir: home(".kilo/skills"),
        former_skills_dirs: &[home(".kilocode/skills")],
        detect_dirs: &[home(".kilocode"), home(".kilo")],
        project_skills_dir: Some(".kilocode/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2, row "Kilo Code".
        // VERIFIED T61, B02:S24. `~/.kilo/skills` is current (this row's own
        // dir); whether the legacy `.kilocode` is still read is unknown.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: OptIn,
            },
            project: ScopeReads {
                agents: Yes,
                claude: OptIn,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[
                AlsoRead {
                    dir: home(".kilo/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".kilocode/skills"),
                    mode: ReadMode::Unknown,
                },
            ],
        },
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
        // Read facts: brief § Part 2, row "Roo Code".
        // VERIFIED T55 (`~/.roo` > `~/.agents`).
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Own,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "Goose".
        // VERIFIED T62. Project `.goose/skills` is not modelled (project scope).
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[AlsoRead {
                dir: home(".agents/plugins"),
                mode: ReadMode::Default,
            }],
        },
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
        // Read facts: brief § Part 2, row "Gemini CLI".
        // VERIFIED T56 (within a tier `.agents` beats `.gemini`).
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Shared,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "GitHub Copilot".
        // VERIFIED T57. The brief's A-only caveat (local surfaces only) is about
        // surfaces, not this read.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
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
        // Read facts: brief § Part 2, row "Clawdbot".
        // Retires in #108 (former OpenClaw name).
        reads: ReadFacts::UNKNOWN,
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
        // Read facts: brief § Part 2, row "Droid".
        // VERIFIED T58 (one personal tier; the manager shows "Overridden").
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[AlsoRead {
                dir: home(".agent/skills"),
                mode: ReadMode::Default,
            }],
        },
    },
    ToolAdapter {
        id: ToolId::Windsurf,
        // Windsurf is now Devin (relabel only; the key stays `windsurf`).
        display_name: "Devin (Windsurf)",
        // Q12 (round 20 #108): the docs (docs.devin.ai, S40) name
        // `~/.config/devin/skills`; on Windows `%APPDATA%\devin\skills` (brief
        // I7), where its config root `%APPDATA%\devin` detects it too. Whether
        // the Devin CLI reads this folder is UNKNOWN; it reads `~/.agents/skills`
        // either way. The add-skill path `~/.codeium/windsurf/skills` is a
        // former dir (still read, `read_config_from.windsurf`).
        skills_dir: home(".config/devin/skills").overridden(&[OsOverride {
            os: HostOs::Windows,
            root: HostRoot::RoamingAppData,
            rel: "devin/skills",
        }]),
        former_skills_dirs: &[home(".codeium/windsurf/skills")],
        detect_dirs: &[
            home(".codeium/windsurf"),
            home(".config/devin").overridden(&[OsOverride {
                os: HostOs::Windows,
                root: HostRoot::RoamingAppData,
                rel: "devin",
            }]),
        ],
        project_skills_dir: Some(".windsurf/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2, row "Windsurf → Devin".
        // VERIFIED T9–T11. `read_config_from.{windsurf,copilot,claude}` default on.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[
                AlsoRead {
                    dir: home(".codeium/windsurf/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".copilot/skills"),
                    mode: ReadMode::Default,
                },
            ],
        },
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
        // Read facts: brief § Part 2, row "MoltBot".
        // Retires in #108 (former OpenClaw name).
        reads: ReadFacts::UNKNOWN,
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
        // Read facts: brief § Part 2, row "Hermes Agent".
        // VERIFIED T63: shared folders only via `skills.external_dirs`.
        reads: ReadFacts {
            global: ScopeReads {
                agents: OptIn,
                claude: OptIn,
            },
            project: ScopeReads {
                agents: Unknown,
                claude: Unknown,
            },
            precedence: Precedence::Own,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[],
        },
    },
    // round 20 ticket 05 rows
    ToolAdapter {
        id: ToolId::DeepseekHarness,
        display_name: "DeepSeek Harness",
        // `dsh`: `<dshHome>/skills`, default `~/.dsh/skills` (B02 § 4, S51).
        // `DSH_HOME` / `DSH_AGENTS_HOME` are not honoured (env-var homes).
        skills_dir: home(".dsh/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".dsh")],
        project_skills_dir: Some(".dsh/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2 "The four #92 rows", row "DeepSeek Harness".
        // VERIFIED T72 (re-checked 2026-11-06): rank 400 `user-dsh` beats 500
        // `user-agents`, lower rank wins a duplicate name (one copy loads).
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Own,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[],
        },
    },
    ToolAdapter {
        id: ToolId::Zcode,
        display_name: "ZCode",
        // `~/.zcode/skills` (zcode.z.ai/en/docs/skill, B02 S52).
        skills_dir: home(".zcode/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".zcode")],
        project_skills_dir: Some(".zcode/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2 "The four #92 rows", row "ZCode".
        // VERIFIED T73, B02:S53: `~/.zcode` before `~/.agents`, first same-name
        // skill wins. Claude Code's folder is only an Import source, never read.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Own,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[],
        },
    },
    ToolAdapter {
        id: ToolId::Codewhale,
        display_name: "Codewhale",
        // Owned `~/.codewhale/skills` (`$CODEWHALE_HOME/skills`, not
        // honoured). Never detected on `.deepseek`: that legacy root is a read
        // fallback, too generic to mean Codewhale (B02 I13).
        skills_dir: home(".codewhale/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".codewhale")],
        project_skills_dir: Some(".codewhale/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2 "The four #92 rows", row "Codewhale".
        // VERIFIED T74 (re-checked 2026-11-06): global order `.codewhale` >
        // `.agents` > `.claude` > legacy `.deepseek`, all read by the current
        // release; shadowing warnings name both copies.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Own,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[AlsoRead {
                dir: home(".deepseek/skills"),
                mode: ReadMode::Default,
            }],
        },
    },
    ToolAdapter {
        id: ToolId::WorkBuddy,
        display_name: "WorkBuddy",
        // `~/.workbuddy/skills` per secondary sources only (B02 S56–S57; the
        // vendor doc says "the user config directory"). No project dir: the
        // secondary sources conflict, and upstream treats it as global-only.
        skills_dir: home(".workbuddy/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".workbuddy")],
        project_skills_dir: None,
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2 "The four #92 rows", row "WorkBuddy".
        // LIKELY B02:S57: no fact is verified.
        reads: ReadFacts::UNKNOWN,
    },
    // round 20 ticket 07 rows
    ToolAdapter {
        id: ToolId::LettaCode,
        display_name: "Letta Code",
        // Computer-scoped skills: docs.letta.com/configuration/skills (brief T40,
        // re-fetched 2026-11-06). `~/.letta` holds `settings.json` (the config root).
        // Its only project skills dir is `.agents/skills`, so it joins the group.
        skills_dir: home(".letta/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".letta")],
        project_skills_dir: Some(".agents/skills"),
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
        // Read facts: brief § Part 2, row "Letta Code (extra)".
        // VERIFIED T40: no global shared folder; project `.agents/skills` only.
        // Agent-memory skills live in the agent's own MemFS, not modelled.
        reads: ReadFacts {
            global: ScopeReads {
                agents: No,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
    },
    ToolAdapter {
        id: ToolId::MuseCode,
        display_name: "Muse Code",
        // `$XDG_CONFIG_HOME/muse/skills`, default `~/.config/muse/skills` (addendum
        // P1, re-fetched 2026-11-06); `XDG_CONFIG_HOME` is not honoured, and the
        // Windows default is UNKNOWN. `~/.config/muse` holds `settings.json` (P2).
        skills_dir: home(".config/muse/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".config/muse")],
        project_skills_dir: Some(".agents/skills"),
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
        // Read facts: addendum § Shared-dir reads, row "Muse Code".
        // VERIFIED P1. The "foreign personal roots" (`~/.claude/skills`,
        // `$CODEX_HOME/skills` falling back to `~/.codex/skills`) are on by default; a
        // user preference or rollout gate can turn them off. Collisions undocumented.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[AlsoRead {
                dir: home(".codex/skills"),
                mode: ReadMode::Default,
            }],
        },
    },
    ToolAdapter {
        id: ToolId::Poolside,
        display_name: "Poolside",
        // The `pool` CLI: docs.poolside.ai/skills (addendum P3, re-fetched
        // 2026-11-06). `~/.config/poolside` holds `settings.yaml` (P5).
        skills_dir: home(".config/poolside/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".config/poolside")],
        project_skills_dir: Some(".agents/skills"),
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
        // Read facts: addendum § Shared-dir reads, row "Poolside".
        // VERIFIED P3–P4 (`~/.agents/skills` since 1.0.11). It "also scans
        // directories used by other tools", unnamed: `.claude` stays Unknown.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Unknown,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Unknown,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
    },
    ToolAdapter {
        id: ToolId::Dirac,
        // A Cline fork, but its own product with its own `~/.dirac` root: never an
        // alias of the `cline` row (addendum § Summary 5).
        display_name: "Dirac",
        // `getDiracHomePath()` = `~/.dirac`, no env override for skills (addendum
        // P6–P8, re-read at master 2026-11-06).
        skills_dir: home(".dirac/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".dirac")],
        project_skills_dir: Some(".agents/skills"),
        group: Some(VirtualGroup::AgentsStandard),
        supports_symlink: true,
        // Read facts: addendum § Shared-dir reads, row "Dirac".
        // VERIFIED P6–P7: the dedupe iterates backwards, so `~/.ai` > `~/.claude` >
        // `~/.agents` > `~/.dirac` — the shared copy wins, silently.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Shared,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[AlsoRead {
                dir: home(".ai/skills"),
                mode: ReadMode::Default,
            }],
        },
    },
    ToolAdapter {
        id: ToolId::CodeStudio,
        display_name: "Code Studio",
        // Syncfusion Code Studio: personal skills `~/.codestudio/skills` (brief T7,
        // re-read via search index 2026-11-06; the help URL now redirects to
        // docs.boldcreate.ai). FLAG (brief I4, spec Q29): `.codestudio` is not
        // verified to hold more than skills on a real install; the skills-only
        // footprint rule makes the worst case "not detected".
        skills_dir: home(".codestudio/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".codestudio")],
        project_skills_dir: Some(".codestudio/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2, row "Code Studio".
        // VERIFIED T7: setting "Skill File Locations" defaults to `.codestudio`,
        // `.agents`, `.claude` (project and home) plus `~/.copilot/skills`.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[AlsoRead {
                dir: home(".copilot/skills"),
                mode: ReadMode::Default,
            }],
        },
    },
    ToolAdapter {
        id: ToolId::CodeArtsAgent,
        display_name: "CodeArts Agent",
        // Huawei Cloud: user skills `%USERPROFILE%/.codeartsdoer/skills` (brief T6,
        // re-fetched 2026-11-06; macOS/Linux `~/.codeartsdoer/skills` is LIKELY).
        // FLAG (brief I4, spec Q29): `.codeartsdoer` is not verified to hold more
        // than skills; the skills-only footprint rule makes the worst case "not
        // detected".
        skills_dir: home(".codeartsdoer/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".codeartsdoer")],
        project_skills_dir: Some(".codeartsdoer/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2, row "CodeArts Agent".
        // VERIFIED T6: its priority list names no shared folder.
        reads: ReadFacts {
            global: ScopeReads {
                agents: No,
                claude: No,
            },
            project: ScopeReads {
                agents: No,
                claude: No,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
    },
    ToolAdapter {
        id: ToolId::QoderCn,
        // The Qoder CN IDE, formerly Lingma (通义灵码). Its CLI is its own row
        // (`qoder_cn_cli`), the Trae / Trae CN pattern.
        display_name: "Qoder CN",
        // docs.qoder.cn/user-guide/skills (brief T19, re-fetched 2026-11-06).
        skills_dir: home(".lingma/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".lingma")],
        project_skills_dir: Some(".lingma/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2, row "Qoder CN (IDE, ex-Lingma)".
        // VERIFIED T19: the IDE page lists only `.lingma`. The CN *desktop app*
        // reads `~/.agents/skills`, but that is not this surface.
        reads: ReadFacts {
            global: ScopeReads {
                agents: No,
                claude: No,
            },
            project: ScopeReads {
                agents: No,
                claude: No,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
    },
    ToolAdapter {
        id: ToolId::QoderCnCli,
        display_name: "Qoder CN CLI",
        // help.aliyun.com/zh/lingma/qodercli-cn/user-guide/skills (brief T19,
        // re-read 2026-11-06): user `~/.qoder-cn/skills`, project `.qoder/skills`
        // (shared with the Qoder row's project dir). `~/.qoder-cn` holds
        // `settings.json` (docs.qoder.cn/cli/settings); `QODERCN_CONFIG_DIR` is not
        // honoured.
        skills_dir: home(".qoder-cn/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".qoder-cn")],
        project_skills_dir: Some(".qoder/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2, row "Qoder CN CLI".
        // VERIFIED T19: the CLI page lists only its own two dirs.
        reads: ReadFacts {
            global: ScopeReads {
                agents: No,
                claude: No,
            },
            project: ScopeReads {
                agents: No,
                claude: No,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
    },
    ToolAdapter {
        id: ToolId::Kimchi,
        display_name: "Kimchi",
        // Cast AI: docs.kimchi.dev/docs/coding-skills (brief T28, re-fetched
        // 2026-11-06). `~/.config/kimchi` holds `config.json` (`skillPaths`).
        skills_dir: home(".config/kimchi/harness/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".config/kimchi")],
        project_skills_dir: Some(".kimchi/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2, row "Kimchi (optional)".
        // VERIFIED T28. Each other agent's dir is read "if installed", but per agent
        // the project-local dir is checked first and home is only the fallback, so a
        // home read is conditional on the project (round 20 rows review, P2) — never
        // a warning pair. Pi's dir is explicitly global, so it stays `Default`.
        // Earlier `skillPaths` entries win. `.claude` needs `extensions.claude-code-skills`.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Conditional(KIMCHI_FALLBACK),
                claude: OptIn,
            },
            project: ScopeReads {
                agents: Yes,
                claude: OptIn,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[
                AlsoRead {
                    dir: home(".pi/agent/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".codex/skills"),
                    mode: ReadMode::Conditional(KIMCHI_FALLBACK),
                },
                AlsoRead {
                    dir: home(".cursor/skills"),
                    mode: ReadMode::Conditional(KIMCHI_FALLBACK),
                },
                AlsoRead {
                    dir: home(".warp/skills"),
                    mode: ReadMode::Conditional(KIMCHI_FALLBACK),
                },
                AlsoRead {
                    dir: home(".factory/skills"),
                    mode: ReadMode::Conditional(KIMCHI_FALLBACK),
                },
                AlsoRead {
                    dir: home(".gemini/skills"),
                    mode: ReadMode::Conditional(KIMCHI_FALLBACK),
                },
                AlsoRead {
                    dir: home(".copilot/skills"),
                    mode: ReadMode::Conditional(KIMCHI_FALLBACK),
                },
                AlsoRead {
                    dir: home(".github/skills"),
                    mode: ReadMode::Conditional(KIMCHI_FALLBACK),
                },
            ],
        },
    },
    ToolAdapter {
        id: ToolId::AstrBot,
        display_name: "AstrBot",
        // `<root>/data/skills`, root = `$ASTRBOT_ROOT`, else `~/.astrbot` **only in
        // the packaged desktop runtime**, else the cwd (brief T26, `astrbot_path.py`
        // re-read 2026-11-06). Only the desktop runtime is reachable: a source
        // install is undetectable, and `ASTRBOT_ROOT` is not honoured. Detected
        // only through `.astrbot` (brief I4).
        skills_dir: home(".astrbot/data/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".astrbot")],
        // No project dir: its `data/skills` is relative to the AstrBot root, which
        // the detected (desktop) runtime fixes at `~/.astrbot`, and its workspace
        // skills are per chat session (`data/workspaces/<umo>/skills`), never a
        // repository's (brief Part 1 says `data/skills`; deviation, ticket 07).
        project_skills_dir: None,
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2, row "AstrBot (optional)".
        // VERIFIED T26: workspace > local > plugin > sandbox; no shared folder.
        reads: ReadFacts {
            global: ScopeReads {
                agents: No,
                claude: No,
            },
            project: ScopeReads {
                agents: No,
                claude: No,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
    },
    ToolAdapter {
        id: ToolId::OhMyPi,
        display_name: "oh-my-pi",
        // `omp`: omp.sh/docs/skills (brief T41, re-fetched 2026-11-06). User
        // `~/.omp/agent/skills`, project `.omp/skills`; `~/.omp/agent` holds
        // `config.yml`. Its managed `~/.omp/agent/managed-skills` is never a target.
        skills_dir: home(".omp/agent/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".omp")],
        project_skills_dir: Some(".omp/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2, row "oh-my-pi (extra)".
        // VERIFIED T41: every `skills.enable*` switch defaults on; the highest-priority
        // provider wins (native `.omp` first) and `/extensions` shows the shadowed
        // copy. The Pi user dir behind `enablePiUser` is not listed among the
        // compatible locations: `Unknown`.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Own,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[
                AlsoRead {
                    dir: home(".agent/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".codex/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".config/opencode/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".pi/agent/skills"),
                    mode: ReadMode::Unknown,
                },
            ],
        },
    },
    ToolAdapter {
        id: ToolId::Stakpak,
        display_name: "Stakpak",
        // `default_skill_directories()`: project `.stakpak/skills`, then
        // `$HOME/.stakpak/skills` (addendum P9, re-read at main 2026-11-06).
        // `~/.stakpak` holds `config.toml`. Low priority: last release 2026-06.
        skills_dir: home(".stakpak/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".stakpak")],
        project_skills_dir: Some(".stakpak/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: addendum § Shared-dir reads, row "Stakpak".
        // VERIFIED P9 (complete dir list): no shared folder; first seen wins.
        reads: ReadFacts {
            global: ScopeReads {
                agents: No,
                claude: No,
            },
            project: ScopeReads {
                agents: No,
                claude: No,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
    },
    // round 20 ticket 06 rows
    ToolAdapter {
        id: ToolId::GrokBuild,
        display_name: "Grok Build",
        // Brief § Part 1 A, row "Grok Build" (T14): `~/.grok/skills`, Windows
        // `%USERPROFILE%\.grok` (= home). `GROK_HOME` is not honoured.
        skills_dir: home(".grok/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".grok")],
        project_skills_dir: Some(".grok/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2 "Recommended Part 1 additions", row "Grok Build".
        // VERIFIED T14: `~/.grok` first-seen wins over `~/.agents`; `.claude` and
        // `.cursor` are default-on compat cells (`[compat.*].skills`).
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Own,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[AlsoRead {
                dir: home(".cursor/skills"),
                mode: ReadMode::Default,
            }],
        },
    },
    ToolAdapter {
        id: ToolId::DeepAgentsCode,
        display_name: "Deep Agents Code",
        // Brief § Part 1 A, row "Deep Agents Code" (T8): the default agent's
        // `~/.deepagents/agent/skills`; custom-named agents are unreachable.
        // `DEEPAGENTS_HOME` is not honoured.
        skills_dir: home(".deepagents/agent/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".deepagents")],
        project_skills_dir: Some(".deepagents/skills"),
        group: None,
        // Copy (spec Q29, brief I8): it rejects a skill whose resolved path
        // leaves its skill roots unless allow-listed, and a link resolves into
        // the central repo. #84 later makes the mode the operator's choice.
        supports_symlink: false,
        // Read facts: brief § Part 2 "Recommended Part 1 additions", row "Deep Agents Code".
        // VERIFIED T8: `~/.agents` overrides `~/.deepagents/<agent>`; the
        // `.claude` reads are labelled experimental but on by default.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Shared,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[],
        },
    },
    ToolAdapter {
        id: ToolId::Reasonix,
        display_name: "Reasonix",
        // Brief § Part 1 A, row "Reasonix" (T27, I7): `<Reasonix home>/skills`,
        // home `~/.reasonix` on macOS/Linux and `%APPDATA%\reasonix` on Windows
        // (the registry's first per-OS override). `REASONIX_HOME` is not honoured.
        skills_dir: home(".reasonix/skills").overridden(&[OsOverride {
            os: HostOs::Windows,
            root: HostRoot::RoamingAppData,
            rel: "reasonix/skills",
        }]),
        former_skills_dirs: &[],
        detect_dirs: &[home(".reasonix").overridden(&[OsOverride {
            os: HostOs::Windows,
            root: HostRoot::RoamingAppData,
            rel: "reasonix",
        }])],
        project_skills_dir: Some(".reasonix/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2 "Recommended Part 1 additions", row "Reasonix".
        // VERIFIED T27 reads. Precedence is LIKELY own (the global list puts the
        // Reasonix home first; the doc orders scopes, not dirs within one), so
        // `Unknown`, as is what a same-scope duplicate does. It also scans
        // `~/.reasonix/skills` on Windows (not modelled: no other row's dir).
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[AlsoRead {
                dir: home(".agent/skills"),
                mode: ReadMode::Default,
            }],
        },
    },
    ToolAdapter {
        id: ToolId::CortexCode,
        display_name: "Cortex Code",
        // Brief § Part 1 A, row "Cortex Code" (T5): CLI and Desktop share
        // `~/.snowflake/cortex`. Detected there, **never** on `.snowflake`, which
        // the Snowflake CLI shares (brief I4, spec: binding).
        skills_dir: home(".snowflake/cortex/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".snowflake/cortex")],
        project_skills_dir: Some(".cortex/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2 "Recommended Part 1 additions", row "Cortex Code".
        // VERIFIED T5: both copies listed; differing copies show a conflict
        // indicator (`/skill sync` resolves).
        reads: ReadFacts {
            global: ScopeReads {
                agents: No,
                claude: Yes,
            },
            project: ScopeReads {
                agents: No,
                claude: Yes,
            },
            precedence: Precedence::Both,
            duplicates: DuplicateBehaviour::FlagsConflict,
            also_reads: &[],
        },
    },
    ToolAdapter {
        id: ToolId::RovoDev,
        display_name: "Rovo Dev",
        // Brief § Part 1 A, row "Rovo Dev" (T15); `~/.rovodev` holds `config.yml`.
        skills_dir: home(".rovodev/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".rovodev")],
        project_skills_dir: Some(".rovodev/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2 "Recommended Part 1 additions", row "Rovo Dev".
        // VERIFIED T15: `~/.rovodev` and `~/.agents` are one "User" tier; which
        // copy wins inside it is undocumented.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
    },
    ToolAdapter {
        id: ToolId::IbmBob,
        display_name: "IBM Bob",
        // Brief § Part 1 A, row "IBM Bob" (T4); `~/.bob` holds `settings/settings.json`.
        skills_dir: home(".bob/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".bob")],
        project_skills_dir: Some(".bob/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2 "Recommended Part 1 additions", row "IBM Bob".
        // VERIFIED T4: its own folders only (project > global).
        reads: ReadFacts {
            global: ScopeReads {
                agents: No,
                claude: No,
            },
            project: ScopeReads {
                agents: No,
                claude: No,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
    },
    ToolAdapter {
        id: ToolId::ForgeCode,
        display_name: "ForgeCode",
        // Brief § Part 1 A, row "ForgeCode" (T13): `~/.forge/skills`. Detected on
        // `.forge` only: Forge keeps its base at the legacy `~/forge` while that
        // directory exists (`forge_config` `resolve_base_path`, re-verified
        // 2026-11-06), so a legacy install reads `~/forge/skills`, not this dir —
        // detecting `forge` (brief I4) would deploy into a folder it does not
        // read. `FORGE_CONFIG` is not honoured.
        skills_dir: home(".forge/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".forge")],
        project_skills_dir: Some(".forge/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2 "Recommended Part 1 additions", row "ForgeCode".
        // VERIFIED T13: `~/.agents` > `~/.forge` (last occurrence kept).
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: No,
                claude: No,
            },
            precedence: Precedence::Shared,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[],
        },
    },
    ToolAdapter {
        id: ToolId::Firebender,
        display_name: "Firebender",
        // Brief § Part 1 A, row "Firebender" (T12); `~/.firebender` holds
        // `firebender.json`, `rules/` and `commands/`.
        skills_dir: home(".firebender/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".firebender")],
        project_skills_dir: Some(".firebender/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2 "Recommended Part 1 additions", row "Firebender".
        // VERIFIED T12: up to six copies are listed (one USER tier). Its
        // `~/.goose/skills` is not the Goose row's `~/.config/goose/skills`.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::ShowsAll,
            also_reads: &[
                AlsoRead {
                    dir: home(".codex/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".cursor/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".goose/skills"),
                    mode: ReadMode::Default,
                },
            ],
        },
    },
    ToolAdapter {
        id: ToolId::MiniMaxCode,
        display_name: "MiniMax Code",
        // Brief § Part 1 A, row "MiniMax Code" (T24): `<dataDir>/skills`, dataDir
        // `~/.minimax` (`packages/config/src/data-dir.ts`, re-verified 2026-11-06;
        // legacy `~/.mavis` migrates). Detect-dir flag (brief I4, spec Q29):
        // `.minimax` may be shared with other MiniMax products (UNKNOWN).
        // `MINIMAX_DATA_DIR` is not honoured.
        skills_dir: home(".minimax/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".minimax")],
        project_skills_dir: Some(".minimax/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2 "Recommended Part 1 additions", row "MiniMax Code".
        // VERIFIED T24: the external roots (priority 30–40) outrank its own
        // `<dataDir>/skills` root (no priority, 0) in one rank, so the shared
        // copy wins; `duplicateWarn` logs the shadowed one.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Shared,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[AlsoRead {
                dir: home(".codex/skills"),
                mode: ReadMode::Default,
            }],
        },
    },
    ToolAdapter {
        id: ToolId::PositAssistant,
        display_name: "Posit Assistant",
        // Brief § Part 1 A, row "Posit Assistant" (T18): `~/.posit/assistant`
        // holds `settings.json`. Detected there only: **never** on `.posit` (brief
        // I4, spec: binding), and never on the pre-rename `.positai`: an old release
        // reads `~/.positai/skills`, so detecting it would deploy into a folder that
        // install does not read (spec Q7; orchestrator 2026-11-06, as ForgeCode's `forge`).
        skills_dir: home(".posit/assistant/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".posit/assistant")],
        project_skills_dir: Some(".posit/assistant/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2 "Recommended Part 1 additions", row "Posit Assistant".
        // VERIFIED T18: the later dir wins, and `.posit/assistant` loads after
        // `.agents`. `skills.paths` replaces the defaults (a setting: still `Yes`).
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Own,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[],
        },
    },
    ToolAdapter {
        id: ToolId::AiderDesk,
        display_name: "AiderDesk",
        // Brief § Part 1 A, row "AiderDesk" (T3): `~/.aider-desk` holds its rules,
        // prompts, extensions and projects. `AIDER_DESK_HOME_DIR` is not honoured.
        skills_dir: home(".aider-desk/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".aider-desk")],
        project_skills_dir: Some(".aider-desk/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2 "Recommended Part 1 additions", row "AiderDesk".
        // VERIFIED T3: its own folders only; skills load only with "Use Skills
        // Tools" on in the agent profile.
        reads: ReadFacts {
            global: ScopeReads {
                agents: No,
                claude: No,
            },
            project: ScopeReads {
                agents: No,
                claude: No,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
    },
    ToolAdapter {
        id: ToolId::AutohandCode,
        display_name: "Autohand Code CLI",
        // Brief § Part 1 A, row "Autohand Code CLI" (T25): `~/.autohand` holds
        // its config and sessions. `AUTOHAND_HOME` is not honoured.
        skills_dir: home(".autohand/skills"),
        former_skills_dirs: &[],
        detect_dirs: &[home(".autohand")],
        project_skills_dir: Some(".autohand/skills"),
        group: None,
        supports_symlink: true,
        // Read facts: brief § Part 2 "Recommended Part 1 additions", row "Autohand Code CLI".
        // VERIFIED T25: later location wins and `~/.autohand` loads last.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Own,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[
                AlsoRead {
                    dir: home(".codex/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".agent/skills"),
                    mode: ReadMode::Default,
                },
            ],
        },
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
    /// Directories any one of whose presence marks the harness installed —
    /// its own config root(s), like an adapter's (never a shared skills
    /// convention dir).
    pub detect_dirs: &'static [RegistryPath],
    /// The harness's own (or legacy) skills folders, which Skills Hub never
    /// deploys to (it reaches the harness through its group's folder). Read only
    /// by the installedness rule: a detect dir holding nothing but the path down
    /// to one of them is a skill deployer's footprint, not the harness — the same
    /// rule as an adapter's (`is_installed_in`). Empty when it has none.
    pub skills_dirs: &'static [RegistryPath],
    /// The group whose shared folder it reads: its only way in, so that
    /// group entry's folder stands as its own deploy dir.
    pub group: VirtualGroup,
    /// Its shared-folder read facts, each citing its brief row, exactly as an
    /// adapter's ([`ToolAdapter::reads`]). They resolve the cross-read pairs
    /// it would see (`catalog::ReaderEntry::duplicate_readers`).
    pub reads: ReadFacts,
}

/// Kimchi's cross-agent fallback: an agent's home folder is read only when the
/// project has none of that agent's own (docs.kimchi.dev, brief T28).
const KIMCHI_FALLBACK: &str = "only when the project has no folder of its own";

/// The reader-only harnesses, in presentation order; tests add rows per
/// thread (`test_overrides::sample_reader`).
static READER_ONLY_HARNESSES: &[ReaderOnlyHarness] = &[
    // round 20 ticket 05 rows
    ReaderOnlyHarness {
        key: "warp",
        display_name: "Warp",
        // Warp's config root (`settings.toml`) per OS, from its file-locations
        // reference (brief T80): `~/.warp` on macOS, `~/.config/warp-terminal`
        // on Linux, `%LOCALAPPDATA%\warp\Warp` on Windows. `~/.warp` also holds
        // the MCP config and bundled skills on every OS. Preview channels
        // (`.warp-preview`, `warp-terminal-preview`) are not detected.
        detect_dirs: &[
            home(".warp"),
            home(".config/warp-terminal").overridden(&[OsOverride {
                os: HostOs::Windows,
                root: HostRoot::LocalAppData,
                rel: "warp/Warp",
            }]),
        ],
        skills_dirs: &[home(".warp/skills")],
        group: VirtualGroup::AgentsStandard,
        // Read facts: brief § Part 2 "Recommended Part 1 additions", row
        // "Warp (AGENTS)". VERIFIED T22 (re-checked 2026-11-06): every
        // supported dir name under home is read and every copy is listed.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Both,
            duplicates: DuplicateBehaviour::ShowsAll,
            also_reads: &[
                AlsoRead {
                    dir: home(".warp/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".codex/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".cursor/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".gemini/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".copilot/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".factory/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".github/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".opencode/skills"),
                    mode: ReadMode::Default,
                },
            ],
        },
    },
    ReaderOnlyHarness {
        key: "zed",
        display_name: "Zed",
        // Zed's config dir (`paths::config_dir`): `~/.config/zed` on macOS and
        // Linux, `%APPDATA%\Zed` on Windows (brief I7). A Flatpak install keeps
        // it under `FLATPAK_XDG_CONFIG_HOME` and is not detected.
        detect_dirs: &[home(".config/zed").overridden(&[OsOverride {
            os: HostOs::Windows,
            root: HostRoot::RoamingAppData,
            rel: "Zed",
        }])],
        skills_dirs: &[],
        group: VirtualGroup::AgentsStandard,
        // Read facts: brief § Part 2 "Recommended Part 1 additions", row
        // "Zed (AGENTS)". VERIFIED T23 (re-checked 2026-11-06): only
        // `.agents/skills`, by design; precedence n/a.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[],
        },
    },
    ReaderOnlyHarness {
        key: "dexto",
        display_name: "Dexto",
        detect_dirs: &[home(".dexto")],
        skills_dirs: &[home(".dexto/skills")],
        group: VirtualGroup::AgentsStandard,
        // Read facts: brief § Part 2 "Recommended Part 1 additions", row
        // "Dexto (AGENTS)". VERIFIED T34 (re-checked 2026-11-06): canonical
        // `~/.agents/skills`, legacy `~/.dexto/skills` still read; "no source
        // precedence rules". Claude plugin skills are not a skills folder.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Unknown,
            also_reads: &[AlsoRead {
                dir: home(".dexto/skills"),
                mode: ReadMode::Default,
            }],
        },
    },
    ReaderOnlyHarness {
        key: "fx",
        display_name: "fx",
        // `~/.fx` holds `settings.json` and its own `skills/` (T35). Never
        // deployed to: a second copy there makes `$name` load nothing.
        detect_dirs: &[home(".fx")],
        skills_dirs: &[home(".fx/skills")],
        group: VirtualGroup::AgentsStandard,
        // Read facts: brief § Part 2 "Recommended Part 1 additions", row
        // "fx (AGENTS)". VERIFIED T35 (re-checked 2026-11-06): "If several
        // skills have the same name, typing `$name` loads none of them."
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Both,
            duplicates: DuplicateBehaviour::LoadsNone,
            also_reads: &[
                AlsoRead {
                    dir: home(".fx/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".config/opencode/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".codex/skills"),
                    mode: ReadMode::Default,
                },
                AlsoRead {
                    dir: home(".claw/skills"),
                    mode: ReadMode::Default,
                },
            ],
        },
    },
    ReaderOnlyHarness {
        key: "sarvam_code",
        display_name: "Sarvam Code",
        // `~/.sarvam` holds `config.toml` (T37, secondary).
        detect_dirs: &[home(".sarvam")],
        skills_dirs: &[],
        group: VirtualGroup::AgentsStandard,
        // Read facts: brief § Part 2 "Recommended Part 1 additions", row
        // "Sarvam Code (AGENTS)". LOW CONFIDENCE — LIKELY T1, T37 only: the
        // `~/.agents/skills` read is inferred from its Codex fork, so every fact
        // is `Unknown`. Re-verify when the CLI is public (invite-gated as of
        // 2026-11-06; the teardown names no `.agents` path).
        reads: ReadFacts::UNKNOWN,
    },
    ReaderOnlyHarness {
        key: "vt_code",
        display_name: "VT Code",
        // VT Code's config dir (`VtCodePaths`, addendum P12): macOS
        // `~/Library/Application Support/com.vinhnx.vtcode`, Windows
        // `%APPDATA%\vinhnx\vtcode`, else `~/.config/vtcode`; plus the legacy
        // home `~/.vtcode`. `VTCODE_HOME` / `VTCODE_CONFIG` are not honoured.
        detect_dirs: &[
            home(".config/vtcode").overridden(&[
                OsOverride {
                    os: HostOs::MacOs,
                    root: HostRoot::LocalAppData,
                    rel: "com.vinhnx.vtcode",
                },
                OsOverride {
                    os: HostOs::Windows,
                    root: HostRoot::RoamingAppData,
                    rel: "vinhnx/vtcode",
                },
            ]),
            home(".vtcode"),
        ],
        skills_dirs: &[home(".vtcode/skills")],
        group: VirtualGroup::AgentsStandard,
        // Read facts: brief § Part 2 "Recommended Part 1 additions", row
        // "VT Code (extra)"; addendum § Shared-dir reads, row "VT Code".
        // VERIFIED T42, P10–P12 (re-checked at 86902fc): `~/.agents/skills` is
        // the first user root, then its data dir and the legacy `~/.vtcode`;
        // "only the first occurrence is loaded". The per-OS data dir and
        // `/etc/codex/skills` hold no registry row's folder and are not listed.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Shared,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[AlsoRead {
                dir: home(".vtcode/skills"),
                mode: ReadMode::Default,
            }],
        },
    },
    ReaderOnlyHarness {
        key: "dimcode",
        display_name: "DimCode",
        // `~/.dimcode` (v2 data home `~/.dimcode/v2`, P14). `DIMCODE_HOME` is
        // not honoured.
        detect_dirs: &[home(".dimcode")],
        skills_dirs: &[home(".dimcode/v2/skills"), home(".dimcode/skills")],
        group: VirtualGroup::AgentsStandard,
        // Read facts: addendum § Shared-dir reads, row "DimCode". VERIFIED P13
        // (shipped bundle 0.5.24, re-checked 2026-11-06): the market root
        // `~/.dimcode/v2/skills` before `~/.agents/skills` (own wins).
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: No,
            },
            precedence: Precedence::Own,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[AlsoRead {
                dir: home(".dimcode/v2/skills"),
                mode: ReadMode::Default,
            }],
        },
    },
    ReaderOnlyHarness {
        key: "crow_cli",
        display_name: "crow-cli",
        // Its config root is `~/.agents/crow` (P15): its own folder inside
        // `~/.agents`, not the shared convention dir itself.
        detect_dirs: &[home(".agents/crow")],
        skills_dirs: &[],
        group: VirtualGroup::AgentsStandard,
        // Read facts: addendum § Shared-dir reads, row "crow-cli". VERIFIED
        // P15–P16 (re-checked 2026-11-06): `~/.agents/skills` is its only user
        // dir (precedence n/a); every hidden `*/skills` up to the git root at
        // project scope; an earlier root wins, shadowed copies logged.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: No,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Yes,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[],
        },
    },
    ReaderOnlyHarness {
        key: "corust_agent",
        display_name: "Corust Agent",
        // `~/.corust-agent` (P17). `CORUST_HOME` is not honoured.
        detect_dirs: &[home(".corust-agent")],
        skills_dirs: &[home(".corust-agent/skills")],
        group: VirtualGroup::AgentsStandard,
        // Read facts: addendum § Shared-dir reads, row "Corust Agent".
        // VERIFIED dirs P17 (v0.6.0 binary strings, re-checked 2026-11-06):
        // `~/.corust-agent/skills` and `~/.agents/skills`, "keeping the first
        // match"; own-first order LIKELY, `.claude` UNKNOWN.
        reads: ReadFacts {
            global: ScopeReads {
                agents: Yes,
                claude: Unknown,
            },
            project: ScopeReads {
                agents: Yes,
                claude: Unknown,
            },
            precedence: Precedence::Unknown,
            duplicates: DuplicateBehaviour::Shadows,
            also_reads: &[AlsoRead {
                dir: home(".corust-agent/skills"),
                mode: ReadMode::Default,
            }],
        },
    },
];

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
        .filter(|reader| reader.group == group)
        .collect()
}

/// Whether a reader-only harness is installed: some detect dir exists and is
/// not a skills-only footprint of one of the harness's own skills folders
/// (`ReaderOnlyHarness::skills_dirs`) — the adapter rule (`is_installed_in`),
/// since a skill deployer writes into those folders too.
pub fn reader_installed_in(roots: &HostRoots, reader: &ReaderOnlyHarness) -> bool {
    reader.detect_dirs.iter().any(|dir| {
        let detect_dir = dir.resolve(roots);
        detect_dir.exists()
            && !reader
                .skills_dirs
                .iter()
                .any(|skills| is_skills_only_footprint(&detect_dir, &skills.resolve(roots)))
    })
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
pub(crate) fn catalog_rows() -> impl Iterator<Item = &'static ToolAdapter> {
    TOOL_ADAPTERS.iter()
}

#[cfg(test)]
pub(crate) fn catalog_rows() -> impl Iterator<Item = &'static ToolAdapter> {
    TOOL_ADAPTERS
        .iter()
        .map(|adapter| test_overrides::lookup(adapter.key()).unwrap_or(adapter))
}

/// The detected registry entries and reader-only harnesses on the host whose
/// roots are `roots` — the installed set placement is computed over
/// (`core::placement::place`), by the one installedness rule
/// ([`is_installed_in`], [`reader_installed_in`]).
pub fn installed_set_in(roots: &HostRoots) -> InstalledSet {
    InstalledSet::from_keys(
        catalog_rows()
            .filter(|adapter| is_installed_in(roots, adapter))
            .map(ToolAdapter::key)
            .chain(
                reader_only_harnesses()
                    .into_iter()
                    .filter(|reader| reader_installed_in(roots, reader))
                    .map(|reader| reader.key),
            ),
    )
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
            skills_dirs: &[],
            group: VirtualGroup::AgentsStandard,
            reads: super::ReadFacts::UNKNOWN,
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

    /// The pre-#108 registry shape of Kimi Code (round 20 ticket 10 moved
    /// it to `~/.kimi-code/skills`): deployed to Amp's
    /// `~/.config/agents/skills`, a plain shared folder (no virtual group)
    /// whose folder key is Amp's, the first entry in registry order. No
    /// shipped row shares a plain folder since #108; the tests of the plain
    /// shared-folder rules (one folder, one row; a legacy sibling row keeps its
    /// own meaning until the redeploy converts it) run on this shape.
    pub fn kimi_on_amps_folder() -> &'static ToolAdapter {
        let kimi = super::TOOL_ADAPTERS
            .iter()
            .find(|adapter| adapter.key() == "kimi_cli")
            .expect("kimi_cli row");
        shadow(ToolAdapter {
            skills_dir: RegistryPath::home(".config/agents/skills"),
            former_skills_dirs: &[],
            ..kimi.clone()
        })
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
        // A former dir still holds the Tool's copies: a Deployment the
        // redeploy has not moved yet, or one it left behind.
        std::iter::once(skills_dir_in(roots, adapter))
            .chain(former_skills_dirs_in(roots, adapter))
            .any(|dir| {
                if path.starts_with(&dir) {
                    return true;
                }
                match (&resolved, dir.canonicalize()) {
                    (Some(resolved), Ok(dir)) => resolved.starts_with(dir),
                    _ => false,
                }
            })
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
/// detect dir holding nothing but the path down to the tool's skills dir (or
/// one of its former skills dirs)
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
    // A former skills dir leaves the same footprint: `~/.codex/skills/x` and
    // nothing else under `~/.codex` is a deployer's (or an old Skills Hub's)
    // leftover, not Codex.
    let skills_dirs: Vec<PathBuf> = std::iter::once(skills_dir_in(roots, adapter))
        .chain(former_skills_dirs_in(roots, adapter))
        .collect();
    let detected = detect_dirs_in(roots, adapter).iter().any(|detect_dir| {
        detect_dir.exists()
            && (adapter.as_virtual_group().is_some()
                || !skills_dirs
                    .iter()
                    .any(|skills_dir| is_skills_only_footprint(detect_dir, skills_dir)))
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
