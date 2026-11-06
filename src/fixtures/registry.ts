// Snapshot of the Tool registry (`TOOL_ADAPTERS`, src-tauri/src/core/tool_adapters/mod.rs)
// for the dev-only fixture backend. The real catalog is backend-owned; this copy
// only has to be plausible, so drift is harmless. Regenerate it from mod.rs when
// adapters change and the fixture app should show the new Tool.

import type { DuplicateBehaviour } from "../bindings";

export type FixtureTool = {
  key: string;
  /** Global-scope label (`display_name`). */
  label: string;
  /** Global skills dir, relative to home. */
  globalDir: string;
  /** Project skills dir, relative to the project root; `null` for a harness with none (no project scope). */
  projectDir: string | null;
  /** Absorbed into the AGENTS-standard virtual group at project scope. */
  grouped: boolean;
};

/**
 * A reader-only harness (`READER_ONLY_HARNESSES` in the registry): detected
 * and listed under the `.agents/skills` entry, never a target. Fixture-only
 * rows until the real ones land (round 20 ticket 05); `detected` stands in
 * for its detect dirs.
 */
export type FixtureReader = { key: string; label: string; detected: boolean };

export const READER_REGISTRY: readonly FixtureReader[] = [
  { key: "zed", label: "Zed", detected: true },
  { key: "warp", label: "Warp", detected: false },
];

/** Registry key of the one virtual group entry (project scope). */
export const VIRTUAL_GROUP_KEY = "agents_skills";

export const TOOL_REGISTRY: readonly FixtureTool[] = [
  { key: "agents_skills", label: ".agents/skills", globalDir: ".agents/skills", projectDir: ".agents/skills", grouped: false },
  { key: "cursor", label: "Cursor", globalDir: ".cursor/skills", projectDir: ".agents/skills", grouped: true },
  { key: "claude_code", label: "Claude Code", globalDir: ".claude/skills", projectDir: ".claude/skills", grouped: false },
  { key: "codex", label: "Codex", globalDir: ".codex/skills", projectDir: ".agents/skills", grouped: true },
  { key: "opencode", label: "OpenCode", globalDir: ".config/opencode/skills", projectDir: ".agents/skills", grouped: true },
  { key: "antigravity", label: "Antigravity", globalDir: ".gemini/antigravity/global_skills", projectDir: ".agents/skills", grouped: true },
  { key: "amp", label: "Amp", globalDir: ".config/agents/skills", projectDir: ".agents/skills", grouped: true },
  { key: "kimi_cli", label: "Kimi Code CLI", globalDir: ".config/agents/skills", projectDir: ".agents/skills", grouped: true },
  { key: "augment", label: "Augment", globalDir: ".augment/skills", projectDir: ".augment/skills", grouped: false },
  { key: "openclaw", label: "OpenClaw", globalDir: ".openclaw/skills", projectDir: "skills", grouped: false },
  { key: "copaw", label: "QwenPaw", globalDir: ".copaw/skill_pool", projectDir: ".copaw/skill_pool", grouped: false },
  { key: "cline", label: "Cline", globalDir: ".cline/skills", projectDir: ".agents/skills", grouped: true },
  { key: "codebuddy", label: "CodeBuddy", globalDir: ".codebuddy/skills", projectDir: ".codebuddy/skills", grouped: false },
  { key: "command_code", label: "Command Code", globalDir: ".commandcode/skills", projectDir: ".commandcode/skills", grouped: false },
  { key: "continue", label: "Continue", globalDir: ".continue/skills", projectDir: ".continue/skills", grouped: false },
  { key: "crush", label: "Crush", globalDir: ".config/crush/skills", projectDir: ".crush/skills", grouped: false },
  { key: "junie", label: "Junie", globalDir: ".junie/skills", projectDir: ".junie/skills", grouped: false },
  { key: "iflow_cli", label: "iFlow CLI", globalDir: ".iflow/skills", projectDir: ".iflow/skills", grouped: false },
  { key: "kiro_cli", label: "Kiro CLI", globalDir: ".kiro/skills", projectDir: ".kiro/skills", grouped: false },
  { key: "kode", label: "Kode", globalDir: ".kode/skills", projectDir: ".kode/skills", grouped: false },
  { key: "mcpjam", label: "MCPJam", globalDir: ".mcpjam/skills", projectDir: ".mcpjam/skills", grouped: false },
  { key: "mistral_vibe", label: "Mistral Vibe", globalDir: ".vibe/skills", projectDir: ".vibe/skills", grouped: false },
  { key: "mux", label: "Xum", globalDir: ".mux/skills", projectDir: ".mux/skills", grouped: false },
  { key: "openclaude", label: "OpenClaude IDE", globalDir: ".openclaude/skills", projectDir: ".openclaude/skills", grouped: false },
  { key: "openhands", label: "OpenHands", globalDir: ".openhands/skills", projectDir: ".openhands/skills", grouped: false },
  { key: "pi", label: "Pi", globalDir: ".pi/agent/skills", projectDir: ".pi/skills", grouped: false },
  { key: "qoder", label: "Qoder", globalDir: ".qoder/skills", projectDir: ".qoder/skills", grouped: false },
  { key: "qoderwork", label: "QoderWork", globalDir: ".qoderwork/skills", projectDir: ".qoderwork/skills", grouped: false },
  { key: "qwen_code", label: "Qwen Code", globalDir: ".qwen/skills", projectDir: ".qwen/skills", grouped: false },
  { key: "trae", label: "Trae", globalDir: ".trae/skills", projectDir: ".trae/skills", grouped: false },
  { key: "trae_cn", label: "Trae CN", globalDir: ".trae-cn/skills", projectDir: ".trae/skills", grouped: false },
  { key: "zencoder", label: "Zencoder / Zenflow", globalDir: ".zencoder/skills", projectDir: ".zencoder/skills", grouped: false },
  { key: "neovate", label: "Neovate", globalDir: ".neovate/skills", projectDir: ".neovate/skills", grouped: false },
  { key: "pochi", label: "Pochi", globalDir: ".pochi/skills", projectDir: ".pochi/skills", grouped: false },
  { key: "adal", label: "AdaL", globalDir: ".adal/skills", projectDir: ".adal/skills", grouped: false },
  { key: "kilo_code", label: "Kilo Code", globalDir: ".kilocode/skills", projectDir: ".kilocode/skills", grouped: false },
  { key: "roo_code", label: "Roo Code", globalDir: ".roo/skills", projectDir: ".roo/skills", grouped: false },
  { key: "goose", label: "Goose", globalDir: ".config/goose/skills", projectDir: ".goose/skills", grouped: false },
  { key: "gemini_cli", label: "Gemini CLI", globalDir: ".gemini/skills", projectDir: ".agents/skills", grouped: true },
  { key: "github_copilot", label: "GitHub Copilot", globalDir: ".copilot/skills", projectDir: ".agents/skills", grouped: true },
  { key: "clawdbot", label: "Clawdbot", globalDir: ".clawdbot/skills", projectDir: ".clawdbot/skills", grouped: false },
  { key: "droid", label: "Droid", globalDir: ".factory/skills", projectDir: ".factory/skills", grouped: false },
  { key: "windsurf", label: "Devin (Windsurf)", globalDir: ".codeium/windsurf/skills", projectDir: ".windsurf/skills", grouped: false },
  { key: "moltbot", label: "MoltBot", globalDir: ".moltbot/skills", projectDir: ".moltbot/skills", grouped: false },
  { key: "hermes-agent", label: "Hermes Agent", globalDir: ".hermes/skills", projectDir: ".hermes/skills", grouped: false },
  // Fixture-only stand-in for a harness with no project skills dir until the
  // real WorkBuddy row lands (round 20 ticket 05).
  { key: "workbuddy", label: "WorkBuddy", globalDir: ".workbuddy/skills", projectDir: null, grouped: false },
  // round 20 ticket 07 rows
  { key: "letta_code", label: "Letta Code", globalDir: ".letta/skills", projectDir: ".agents/skills", grouped: true },
  { key: "muse_code", label: "Muse Code", globalDir: ".config/muse/skills", projectDir: ".agents/skills", grouped: true },
  { key: "poolside", label: "Poolside", globalDir: ".config/poolside/skills", projectDir: ".agents/skills", grouped: true },
  { key: "dirac", label: "Dirac", globalDir: ".dirac/skills", projectDir: ".agents/skills", grouped: true },
];

/**
 * Snapshot of the registry's cross-read facts as Rust resolves them with
 * every harness installed (`catalog::duplicate_reads`, round 20 ticket 02):
 * reader key → the entries whose deploy dir it reads by default, beside its
 * own, and its duplicate behaviour. The fixture backend keeps the installed
 * ones, as Rust does. Drift is harmless; regenerate from the Rust catalog.
 */
export const DUPLICATE_READS: Readonly<Record<string, { keys: readonly string[]; behaviour: DuplicateBehaviour }>> = {
  cursor: { keys: ["agents_skills", "claude_code", "codex"], behaviour: "shows_all" },
  codex: { keys: ["agents_skills"], behaviour: "shows_all" },
  opencode: { keys: ["agents_skills", "claude_code"], behaviour: "shows_all" },
  amp: { keys: ["agents_skills", "claude_code"], behaviour: "shadows" },
  augment: { keys: ["agents_skills", "claude_code"], behaviour: "shadows" },
  cline: { keys: ["agents_skills"], behaviour: "unknown" },
  command_code: { keys: ["agents_skills"], behaviour: "shadows" },
  crush: { keys: ["agents_skills", "claude_code", "amp", "kimi_cli"], behaviour: "unknown" },
  junie: { keys: ["agents_skills"], behaviour: "unknown" },
  mcpjam: { keys: ["agents_skills", "claude_code"], behaviour: "unknown" },
  mistral_vibe: { keys: ["agents_skills"], behaviour: "unknown" },
  mux: { keys: ["agents_skills"], behaviour: "shadows" },
  openhands: { keys: ["agents_skills"], behaviour: "shadows" },
  pi: { keys: ["agents_skills"], behaviour: "shadows" },
  qwen_code: { keys: ["agents_skills"], behaviour: "shadows" },
  zencoder: { keys: ["agents_skills"], behaviour: "unknown" },
  neovate: { keys: ["claude_code"], behaviour: "shadows" },
  pochi: { keys: ["agents_skills"], behaviour: "shadows" },
  roo_code: { keys: ["agents_skills"], behaviour: "shadows" },
  goose: { keys: ["agents_skills", "claude_code"], behaviour: "unknown" },
  gemini_cli: { keys: ["agents_skills"], behaviour: "shadows" },
  github_copilot: { keys: ["agents_skills"], behaviour: "unknown" },
  droid: { keys: ["agents_skills"], behaviour: "shadows" },
  windsurf: { keys: ["agents_skills", "github_copilot"], behaviour: "unknown" },
  // round 20 ticket 07 rows
  muse_code: { keys: ["agents_skills", "claude_code", "codex"], behaviour: "unknown" },
  poolside: { keys: ["agents_skills"], behaviour: "unknown" },
  dirac: { keys: ["agents_skills", "claude_code"], behaviour: "shadows" },
};
