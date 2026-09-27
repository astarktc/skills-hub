// The fixture backend's in-memory world: one mutable `FixtureState` per page
// load, built by a scenario (`scenarios.ts`) and mutated by the command
// handlers (`backend.ts`). Wire values live here *as* the generated DTO types,
// so a scenario can only contain states the real backend emits; everything
// that is not on the wire (file bytes, fault injection) sits beside them.
//
// Dev-only: reached solely through the `VITE_MOCK_BACKEND` gate, never part
// of a production build.

import type {
  AppSettings,
  CommandError,
  FeaturedSkillDto,
  GitignoreStatusDto,
  InvocationMode,
  LocalSkillCandidate,
  ManagedSkillDto,
  OnlineSkillDto,
  ProjectSkillAssignmentDto,
  SkillTargetDto,
  SyncMode,
  SyncStatus,
  UpdateSkip,
} from "../bindings";
import { TOOL_REGISTRY, VIRTUAL_GROUP_KEY, type FixtureTool } from "./registry";

export const HOME = "/Users/alex";
export const CENTRAL_DIR = `${HOME}/.skillshub`;

export type FixtureFile = { path: string; content: string };

/** One Managed skill: its wire record plus the bytes and faults behind it. */
export type FixtureSkill = {
  dto: ManagedSkillDto;
  /** The central copy; `SKILL.md` first. Empty when the copy is gone. */
  files: FixtureFile[];
  contentHash: string;
  /** What the next acquisition from the source answers. */
  acquisition: {
    /** Acquisition/finalize fails with this typed condition. */
    fail?: CommandError;
    /** The acquired bytes are discarded at admission. */
    skip?: UpdateSkip;
    /** Upstream moved since the last finalize (a new revision lands). */
    upstreamChanged?: boolean;
    /** The auto-sync re-assert hits a store failure. */
    reassertError?: CommandError;
    /**
     * The invocation mode the source's manifest declares (default: the
     * skill's current base). The one source of truth for Edit replay: the
     * Refresh conflict row and the card's `base_mode` both derive from it.
     */
    upstreamInvocation?: InvocationMode;
  };
};

/** A directory in a Tool's global skills dir that Skills Hub does not manage. */
export type ForeignDir = {
  tool: string;
  name: string;
  /** Content fingerprint; equal fingerprints mean byte-identical. */
  fingerprint: string;
  linkTarget: string | null;
};

export type FixtureProject = {
  id: string;
  name: string;
  path: string;
  created_at: number;
  updated_at: number;
  path_exists: boolean;
  tools: string[];
  assignments: ProjectSkillAssignmentDto[];
  gitignore: GitignoreStatusDto;
};

export type FixtureRepoCandidate = {
  name: string;
  description: string;
  subpath: string;
  invocation?: InvocationMode;
  installs: number;
};

export type FixtureRepo = {
  /** `owner/repo` on GitHub. */
  slug: string;
  candidates: FixtureRepoCandidate[];
  /** Listing/acquisition fails with this typed condition. */
  listingError?: CommandError;
};

export type FixtureLocalFolder = {
  path: string;
  candidates: LocalSkillCandidate[];
};

export type ScenarioName = "rich" | "empty" | "first-run" | "failures";
export const SCENARIOS: readonly ScenarioName[] = [
  "rich",
  "empty",
  "first-run",
  "failures",
];

export type FixtureState = {
  scenario: ScenarioName;
  settings: AppSettings;
  /** Detected tools (global registry keys). */
  installedTools: string[];
  /** Tools a previous `get_tool_status` already reported (newly-installed rule). */
  seenTools: string[];
  skills: FixtureSkill[];
  projects: FixtureProject[];
  foreign: ForeignDir[];
  repos: FixtureRepo[];
  localFolders: FixtureLocalFolder[];
  featured: FeaturedSkillDto[];
  online: OnlineSkillDto[];
  hiddenExplore: string[];
  /** Global Tools whose skills dir refuses writes (TOOL_NOT_WRITABLE). */
  unwritableTools: string[];
  /** `${projectId}:${tool}` pairs whose project skills dir refuses writes. */
  unwritableProjectTools: string[];
  /** Artifact paths whose removal fails (the row is kept, ADR-0002). */
  lockedPaths: string[];
  /** Folders the fake native picker answers with, in rotation. */
  dialogPicks: string[];
  gitCacheEntries: number;
  /** Monotonic id source. */
  nextId: number;
  cancelRequested: boolean;
};

// ---------------------------------------------------------------------------
// Registry facts
// ---------------------------------------------------------------------------

export function toolByKey(key: string): FixtureTool | undefined {
  return TOOL_REGISTRY.find((tool) => tool.key === key);
}

export function toolLabelOf(key: string): string {
  return toolByKey(key)?.label ?? key;
}

/** Absolute global skills dir of a Tool. */
export function globalRoot(key: string): string {
  return `${HOME}/${toolByKey(key)?.globalDir ?? key}`;
}

/** Every registry key sharing this Tool's global skills dir, itself included. */
export function globalSharers(key: string): string[] {
  const dir = toolByKey(key)?.globalDir;
  return TOOL_REGISTRY.filter((tool) => tool.globalDir === dir).map((t) => t.key);
}

/** Project-scope catalog entries (virtual-group constituents absorbed). */
export function projectCatalog(): FixtureTool[] {
  return TOOL_REGISTRY.filter((tool) => !tool.grouped);
}

export function isProjectToolInstalled(state: FixtureState, key: string): boolean {
  if (key === VIRTUAL_GROUP_KEY) {
    return (
      state.installedTools.includes(key) ||
      TOOL_REGISTRY.some(
        (tool) => tool.grouped && state.installedTools.includes(tool.key),
      )
    );
  }
  return state.installedTools.includes(key);
}

export function projectToolDir(project: { path: string }, key: string): string {
  return `${project.path}/${toolByKey(key)?.projectDir ?? key}`;
}

// ---------------------------------------------------------------------------
// Small pure helpers
// ---------------------------------------------------------------------------

/** Expand a leading `~` against the fixture home. */
export function expandHome(input: string): string {
  const trimmed = input.trim();
  if (trimmed === "~") return HOME;
  if (trimmed.startsWith("~/")) return `${HOME}/${trimmed.slice(2)}`;
  return trimmed.replace(/\/+$/, "");
}

export function basename(path: string): string {
  const parts = path.split("/").filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

/** Deterministic 40-hex fingerprint (FNV-1a, widened) — not cryptographic. */
export function fingerprint(input: string): string {
  let out = "";
  for (let round = 0; round < 5; round += 1) {
    let hash = 0x811c9dc5 ^ round;
    for (let i = 0; i < input.length; i += 1) {
      hash ^= input.charCodeAt(i);
      hash = Math.imul(hash, 0x01000193);
    }
    out += (hash >>> 0).toString(16).padStart(8, "0");
  }
  return out;
}

export const MINUTE = 60_000;
export const HOUR = 60 * MINUTE;
export const DAY = 24 * HOUR;

// ---------------------------------------------------------------------------
// Skill bytes
// ---------------------------------------------------------------------------

function frontmatterFor(mode: InvocationMode): string[] {
  switch (mode) {
    case "user-only":
      return ["disable-model-invocation: true"];
    case "model-only":
      return ["user-invocable: false"];
    case "neither":
      return ["disable-model-invocation: true", "user-invocable: false"];
    case "user-and-model":
      return [];
  }
}

/** A plausible `SKILL.md` plus supporting files for one skill. */
export function skillFiles(
  name: string,
  description: string,
  mode: InvocationMode,
  revision = 1,
): FixtureFile[] {
  const title = name
    .split("-")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
  const skillMd = [
    "---",
    `name: ${name}`,
    `description: ${description}`,
    ...frontmatterFor(mode),
    "---",
    "",
    `# ${title}`,
    "",
    `${description}.`,
    "",
    "## When to use",
    "",
    `- The task clearly matches: ${description.toLowerCase()}.`,
    "- The operator asks for it by name.",
    "",
    "## Steps",
    "",
    "1. Read the relevant files before changing anything.",
    "2. State the plan in two or three lines.",
    "3. Make the smallest change that satisfies the request.",
    "4. Verify with the project's own test or build command.",
    "",
    "## Checklist",
    "",
    "See `references/checklist.md` for the full review list.",
    "",
    "```bash",
    "npm run check",
    "```",
    "",
    revision > 1 ? `<!-- revision ${revision} -->` : "",
  ].join("\n");
  const files: FixtureFile[] = [
    { path: "SKILL.md", content: skillMd },
    {
      path: "references/checklist.md",
      content: `# ${title} checklist\n\n- [ ] Inputs understood\n- [ ] Change is minimal\n- [ ] Verified\n`,
    },
  ];
  if (name.length % 3 === 0) {
    files.push({
      path: "scripts/run.sh",
      content: `#!/usr/bin/env bash\nset -euo pipefail\necho "running ${name}"\n`,
    });
  }
  return files;
}

export function hashFiles(files: FixtureFile[]): string {
  return fingerprint(files.map((f) => `${f.path}\n${f.content}`).join("\0"));
}

// ---------------------------------------------------------------------------
// Builders the scenarios share
// ---------------------------------------------------------------------------

export type SkillSource =
  | { kind: "git"; repo: string; subpath?: string }
  | { kind: "local"; path: string }
  | { kind: "imported"; fromTool: string | null };

export type SkillSpec = {
  name: string;
  description: string;
  source: SkillSource;
  /** Tools with a `synced` global target row. */
  synced?: string[];
  /** Tools whose global target row is `error` (a failed propagation or removal). */
  errored?: string[];
  mode?: SyncMode;
  /**
   * Tools whose row is a `copy` (an earlier symlink → copy fallback), whatever
   * `mode` says. Only copies need new bytes on Propagation (`needs_new_bytes`),
   * so these are the rows a Refresh can repair — or fail on.
   */
  copyTools?: string[];
  invocation?: InvocationMode;
  /** An Edit of the invocation mode (`base` = what upstream says). */
  override?: { mode: InvocationMode; conflict?: boolean };
  unlocatable?: "source_missing" | "central_missing";
  /** With `source_missing`: whether the central copy is also gone. */
  centralGone?: boolean;
  ageDays?: number;
  syncedHoursAgo?: number;
  acquisition?: FixtureSkill["acquisition"];
};

export function makeSkill(spec: SkillSpec, now: number, index: number): FixtureSkill {
  const base: InvocationMode = spec.invocation ?? "user-and-model";
  const effective = spec.override?.mode ?? base;
  const centralPath = `${CENTRAL_DIR}/${spec.name}`;
  const centralGone =
    spec.unlocatable === "central_missing" || spec.centralGone === true;
  const files = centralGone ? [] : skillFiles(spec.name, spec.description, effective);
  const created = now - (spec.ageDays ?? 30 + (index % 40)) * DAY - index * 7 * MINUTE;
  const syncedAt = now - (spec.syncedHoursAgo ?? 2 + (index % 30)) * HOUR;
  const kind = spec.source.kind;
  const sourceRef =
    spec.source.kind === "git"
      ? `https://github.com/${spec.source.repo}`
      : spec.source.kind === "local"
        ? spec.source.path
        : null;
  const targets: SkillTargetDto[] = [];
  const row = (tool: string, status: SyncStatus): SkillTargetDto => ({
    tool,
    mode: spec.copyTools?.includes(tool) ? "copy" : (spec.mode ?? "symlink"),
    status,
    target_path: `${globalRoot(tool)}/${spec.name}`,
    synced_at: status === "synced" ? syncedAt : null,
  });
  for (const tool of spec.synced ?? []) targets.push(row(tool, "synced"));
  for (const tool of spec.errored ?? []) targets.push(row(tool, "error"));
  const refreshable = kind !== "imported";
  return {
    dto: {
      id: `skill-${String(index + 1).padStart(3, "0")}-${spec.name}`,
      name: spec.name,
      description: spec.description,
      source_type: kind,
      source_ref: sourceRef,
      imported_from_tool:
        spec.source.kind === "imported" ? spec.source.fromTool : null,
      central_path: centralPath,
      created_at: created,
      updated_at: created + Math.min(now - created, (index % 9) * DAY),
      last_sync_at: targets.length > 0 ? syncedAt : null,
      status: "ok",
      invocation_mode: effective,
      invocation_override: spec.override
        ? { mode: spec.override.mode, base_mode: base, conflict: spec.override.conflict ?? false }
        : null,
      targets,
      refreshable,
      unlocatable: spec.unlocatable ?? null,
      detachable: kind === "local" && !centralGone,
    },
    files,
    contentHash: centralGone ? "" : hashFiles(files),
    acquisition: spec.acquisition ?? {},
  };
}

export type AssignmentSpec = {
  skill: string;
  tool: string;
  mode?: SyncMode;
  status?: SyncStatus;
  lastError?: string;
};

export function makeProject(
  spec: {
    id: string;
    path: string;
    tools: string[];
    assignments: AssignmentSpec[];
    pathExists?: boolean;
    ageDays: number;
    gitignore?: Partial<GitignoreStatusDto>;
  },
  skills: FixtureSkill[],
  now: number,
): FixtureProject {
  const created = now - spec.ageDays * DAY;
  return {
    id: spec.id,
    name: basename(spec.path),
    path: spec.path,
    created_at: created,
    updated_at: now - 3 * HOUR,
    path_exists: spec.pathExists ?? true,
    tools: spec.tools,
    gitignore: { in_gitignore: false, in_exclude: false, ...spec.gitignore },
    assignments: spec.assignments.map((a, i) => {
      const skill = skills.find((s) => s.dto.name === a.skill);
      if (!skill) throw new Error(`fixture: unknown skill ${a.skill}`);
      const status = a.status ?? "synced";
      const mode = a.mode ?? (status === "stale" ? "copy" : "symlink");
      return {
        id: `${spec.id}-asg-${i + 1}`,
        project_id: spec.id,
        skill_id: skill.dto.id,
        skill_name: skill.dto.name,
        tool: a.tool,
        mode,
        status,
        last_error: status === "error" ? (a.lastError ?? "sync failed") : null,
        synced_at: status === "pending" || status === "error" ? null : now - (5 + i) * HOUR,
        content_hash: recordedHash(mode, status, skill.contentHash),
        created_at: created + i * HOUR,
      };
    }),
  };
}

/**
 * The content identity a project assignment row records (Rust
 * `project_sync::sync_assignment_target` + `sync_status::next_status`): only a
 * copy records a hash — a link follows the central copy and records none; a
 * row that never synced (pending/error) has none; a `stale` copy recorded a
 * hash that differs from the current central one (that difference *is* the
 * drift); every other deployed copy recorded the current one.
 */
export function recordedHash(mode: SyncMode, status: SyncStatus, current: string): string | null {
  if (mode !== "copy" || status === "pending" || status === "error" || !current) return null;
  return status === "stale" ? fingerprint(`${current}:previous revision`) : current;
}

export function defaultSettings(
  selected: string[] | null,
  overrides: Partial<AppSettings> = {},
): AppSettings {
  return {
    central_repo_path: CENTRAL_DIR,
    git_cache_cleanup_days: 30,
    git_cache_ttl_secs: 60,
    github_token_set: false,
    auto_sync_enabled: true,
    global_selected_tools: selected,
    global_selected_tools_corrupt: false,
    scan_selected_tools_only: true,
    ui_zoom_level: 1,
    bounds: {
      git_cache_cleanup_days: { min: 0, max: 3650 },
      git_cache_ttl_secs: { min: 0, max: 3600 },
      ui_zoom_level: { min: 0.5, max: 3 },
    },
    ...overrides,
  };
}
