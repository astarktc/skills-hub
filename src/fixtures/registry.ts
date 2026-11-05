// Snapshot of the Tool registry (`TOOL_ADAPTERS`, src-tauri/src/core/tool_adapters/mod.rs)
// for the dev-only fixture backend. The real catalog is backend-owned; this copy
// only has to be plausible, so drift is harmless. Regenerate it from mod.rs when
// adapters change and the fixture app should show the new Tool.
//
// Known drift (ticket 34, backend half): the backend moved Kimi Code CLI to
// `.kimi-code/skills` and made Codex, Goose and OpenHands deploy to
// `.agents/skills`; this copy keeps their former dirs because the fixture's
// shared-dir scenarios and tests are built on Amp + Kimi sharing a dir. The
// fixture does not simulate the "prefer the AGENTS standard folder" layout
// either — both belong to the UI half that consumes it.

export type FixtureTool = {
  key: string;
  /** Global-scope label (`display_name`). */
  label: string;
  /** Project-scope label, when it differs (the virtual group's). */
  groupLabel?: string;
  /** Global skills dir, relative to home. */
  globalDir: string;
  /** Project skills dir, relative to the project root. */
  projectDir: string;
  /** Absorbed into the AGENTS-standard virtual group at project scope. */
  grouped: boolean;
  /** VERIFIED `~/.agents/skills` read (`reads_agents_standard_globally`). */
  reads?: { since?: string; precedence: Precedence };
  /** Other harnesses' global dirs this one also reads (`also_reads`). */
  alsoReads?: readonly string[];
  /** No project dir (global-only): absent from the project catalog. */
  globalOnly?: boolean;
};

/** Mirrors the backend's `Precedence` (bindings). */
export type Precedence = "own_wins" | "agents_wins" | "both_shown" | "unknown";

const OWN: FixtureTool["reads"] = { precedence: "own_wins" };
const ANY: FixtureTool["reads"] = { precedence: "unknown" };
const CLAUDE = [".claude/skills"] as const;

/** Registry key of the one virtual group entry (project scope). */
export const VIRTUAL_GROUP_KEY = "agents_skills";

export const TOOL_REGISTRY: readonly FixtureTool[] = [
  { key: "agents_skills", label: ".agents/skills", groupLabel: ".agents/skills (9 harnesses)", globalDir: ".agents/skills", projectDir: ".agents/skills", grouped: false },
  { key: "cursor", label: "Cursor", globalDir: ".cursor/skills", projectDir: ".agents/skills", grouped: true, reads: ANY, alsoReads: [".claude/skills", ".codex/skills"] },
  { key: "claude_code", label: "Claude Code", globalDir: ".claude/skills", projectDir: ".claude/skills", grouped: false },
  { key: "codex", label: "Codex", globalDir: ".codex/skills", projectDir: ".agents/skills", grouped: true, reads: { since: "rust-v0.95.0", precedence: "both_shown" } },
  { key: "opencode", label: "OpenCode", globalDir: ".config/opencode/skills", projectDir: ".agents/skills", grouped: true, reads: ANY, alsoReads: CLAUDE },
  { key: "antigravity", label: "Antigravity", globalDir: ".gemini/config/skills", projectDir: ".agents/skills", grouped: true, alsoReads: [".gemini/skills"] },
  { key: "amp", label: "Amp", globalDir: ".config/agents/skills", projectDir: ".agents/skills", grouped: true, reads: OWN, alsoReads: CLAUDE },
  { key: "kimi_cli", label: "Kimi Code CLI", globalDir: ".config/agents/skills", projectDir: ".agents/skills", grouped: true, reads: ANY, alsoReads: [".claude/skills", ".codex/skills", ".config/agents/skills"] },
  { key: "augment", label: "Augment", globalDir: ".augment/skills", projectDir: ".augment/skills", grouped: false, reads: OWN, alsoReads: CLAUDE },
  { key: "openclaw", label: "OpenClaw", globalDir: ".openclaw/skills", projectDir: "skills", grouped: false, reads: { precedence: "agents_wins" } },
  { key: "copaw", label: "Copaw", globalDir: ".copaw/skill_pool", projectDir: ".copaw/skill_pool", grouped: false },
  { key: "cline", label: "Cline", globalDir: ".cline/skills", projectDir: ".agents/skills", grouped: true, reads: ANY },
  { key: "codebuddy", label: "CodeBuddy", globalDir: ".codebuddy/skills", projectDir: ".codebuddy/skills", grouped: false },
  { key: "command_code", label: "Command Code", globalDir: ".commandcode/skills", projectDir: ".commandcode/skills", grouped: false, reads: OWN },
  { key: "continue", label: "Continue", globalDir: ".continue/skills", projectDir: ".continue/skills", grouped: false, alsoReads: CLAUDE },
  { key: "crush", label: "Crush", globalDir: ".config/crush/skills", projectDir: ".crush/skills", grouped: false, reads: ANY, alsoReads: [".config/agents/skills", ".claude/skills"] },
  { key: "junie", label: "Junie", globalDir: ".junie/skills", projectDir: ".junie/skills", grouped: false, reads: ANY },
  { key: "iflow_cli", label: "iFlow CLI", globalDir: ".iflow/skills", projectDir: ".iflow/skills", grouped: false },
  { key: "kiro_cli", label: "Kiro CLI", globalDir: ".kiro/skills", projectDir: ".kiro/skills", grouped: false },
  { key: "kode", label: "Kode", globalDir: ".kode/skills", projectDir: ".kode/skills", grouped: false },
  { key: "mcpjam", label: "MCPJam", globalDir: ".mcpjam/skills", projectDir: ".mcpjam/skills", grouped: false },
  { key: "mistral_vibe", label: "Mistral Vibe", globalDir: ".vibe/skills", projectDir: ".vibe/skills", grouped: false, reads: ANY },
  { key: "mux", label: "Xum", globalDir: ".xum/skills", projectDir: ".mux/skills", grouped: false, reads: OWN },
  { key: "openclaude", label: "OpenClaude IDE", globalDir: ".openclaude/skills", projectDir: ".openclaude/skills", grouped: false },
  { key: "openhands", label: "OpenHands", globalDir: ".openhands/skills", projectDir: ".openhands/skills", grouped: false, reads: { precedence: "agents_wins" } },
  { key: "pi", label: "Pi", globalDir: ".pi/agent/skills", projectDir: ".pi/skills", grouped: false, reads: ANY },
  { key: "qoder", label: "Qoder", globalDir: ".qoder/skills", projectDir: ".qoder/skills", grouped: false },
  { key: "qoderwork", label: "QoderWork", globalDir: ".qoderwork/skills", projectDir: ".qoderwork/skills", grouped: false },
  { key: "qwen_code", label: "Qwen Code", globalDir: ".qwen/skills", projectDir: ".qwen/skills", grouped: false, reads: OWN },
  { key: "trae", label: "Trae", globalDir: ".trae/skills", projectDir: ".trae/skills", grouped: false },
  { key: "trae_cn", label: "Trae CN", globalDir: ".trae-cn/skills", projectDir: ".trae/skills", grouped: false },
  { key: "zencoder", label: "Zencoder", globalDir: ".zencoder/skills", projectDir: ".zencoder/skills", grouped: false },
  { key: "neovate", label: "Neovate", globalDir: ".neovate/skills", projectDir: ".neovate/skills", grouped: false },
  { key: "pochi", label: "Pochi", globalDir: ".pochi/skills", projectDir: ".pochi/skills", grouped: false, reads: { since: "2a9ff73 (#1360)", precedence: "own_wins" } },
  { key: "adal", label: "AdaL", globalDir: ".adal/skills", projectDir: ".adal/skills", grouped: false },
  { key: "kilo_code", label: "Kilo Code", globalDir: ".kilo/skills", projectDir: ".kilocode/skills", grouped: false, reads: ANY },
  { key: "roo_code", label: "Roo Code", globalDir: ".roo/skills", projectDir: ".roo/skills", grouped: false, reads: { since: "PR #11181", precedence: "own_wins" } },
  { key: "goose", label: "Goose", globalDir: ".config/goose/skills", projectDir: ".goose/skills", grouped: false, reads: ANY, alsoReads: CLAUDE },
  { key: "gemini_cli", label: "Gemini CLI", globalDir: ".gemini/skills", projectDir: ".agents/skills", grouped: true, reads: { since: "PR #18151", precedence: "agents_wins" } },
  { key: "github_copilot", label: "GitHub Copilot", globalDir: ".copilot/skills", projectDir: ".agents/skills", grouped: true, reads: ANY },
  { key: "droid", label: "Droid", globalDir: ".factory/skills", projectDir: ".factory/skills", grouped: false, reads: ANY },
  { key: "windsurf", label: "Windsurf", globalDir: ".codeium/windsurf/skills", projectDir: ".windsurf/skills", grouped: false, reads: ANY },
  { key: "hermes-agent", label: "Hermes Agent", globalDir: ".hermes/skills", projectDir: ".hermes/skills", grouped: false },
  { key: "deepseek_harness", label: "DeepSeek Harness", globalDir: ".dsh/skills", projectDir: ".dsh/skills", grouped: false, reads: OWN },
  { key: "zcode", label: "ZCode", globalDir: ".zcode/skills", projectDir: ".zcode/skills", grouped: false, reads: OWN },
  { key: "codewhale", label: "Codewhale", globalDir: ".codewhale/skills", projectDir: ".codewhale/skills", grouped: false, reads: OWN, alsoReads: CLAUDE },
  { key: "workbuddy", label: "WorkBuddy", globalDir: ".workbuddy/skills", projectDir: ".workbuddy/skills", grouped: false, globalOnly: true },
];
