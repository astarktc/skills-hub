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
  InvalidManifestReason,
  InvocationMode,
  LocalSkillCandidate,
  ManagedSkillDto,
  OnlineSkillDto,
  ProjectSkillAssignmentDto,
  SkillOutcomeKind,
  SkillTargetDto,
  SyncMode,
  SyncStatus,
  UpdateSkip,
} from "../bindings";
import { READER_REGISTRY, TOOL_REGISTRY, VIRTUAL_GROUP_KEY, type FixtureTool } from "./registry";

export const HOME = "/Users/alex";
export const CENTRAL_DIR = `${HOME}/.skillshub`;

export type FixtureFile = { path: string; content: string };

/**
 * The facts the folder fold derives over a folder's rows (Rust
 * `placement::FolderFacts`): stored rows never carry them, the listing adds
 * them.
 */
export type FolderFacts = { disabled: boolean; deployed: boolean };
/**
 * A stored global target row: the wire row less the folder's facts, plus what
 * the row recorded as deployed (Rust `skill_targets.content_hash`, ADR-0007) —
 * the Content identity of the copy it wrote, `null` for a link or a row that
 * recorded none (pre-1.4.0). Internal: the listing strips it, it never
 * reaches the wire.
 */
export type StoredTarget = Omit<SkillTargetDto, keyof FolderFacts> & { deployed_hash?: string | null };
/** A stored project assignment row (the wire row less the folder's facts). */
export type StoredAssignment = Omit<ProjectSkillAssignmentDto, keyof FolderFacts>;
/** A Managed skill's stored record: its global rows unfolded. */
export type StoredSkill = Omit<ManagedSkillDto, "targets"> & { targets: StoredTarget[] };

/** One Managed skill: its wire record plus the bytes and faults behind it. */
export type FixtureSkill = {
  dto: StoredSkill;
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
  /** Its `SKILL.md` description (the review tells a Conflict's copies apart by it). */
  description?: string | null;
  /** Its newest file's modification time (ms); absent = unknown. */
  modifiedAt?: number | null;
  /**
   * Its `SKILL.md` fails the scan's manifest gate (Rust `scan_tool_dir`):
   * listed in the plan's `invalid`, never in a group. Absent = valid.
   */
  invalid?: InvalidManifestReason;
};

export type FixtureProject = {
  id: string;
  name: string;
  path: string;
  created_at: number;
  updated_at: number;
  path_exists: boolean;
  tools: string[];
  assignments: StoredAssignment[];
  gitignore: GitignoreStatusDto;
};

export type FixtureRepoCandidate = {
  name: string;
  description: string;
  subpath: string;
  invocation?: InvocationMode;
  installs: number;
  /** The first install acquisition fails with this; a retry succeeds (consumed). */
  failOnce?: CommandError;
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

export type ScenarioName = "rich" | "empty" | "first-run" | "failures" | "registry";
export const SCENARIOS: readonly ScenarioName[] = [
  "rich",
  "empty",
  "first-run",
  "failures",
  "registry",
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
  /**
   * Projects whose next view answers `reconciled: false` (once): the
   * reconcile pass met a busy mutation guard (Rust `try_serialized`).
   */
  unreconciledViews: string[];
  /** Paths "Reveal in Finder" was asked to show (`reveal_path`), in order. */
  revealed: string[];
  /**
   * Project artifact paths holding bytes the app never wrote: a folder in the
   * way of a project first sync (the project counterpart of `foreign`).
   */
  projectOccupants: string[];
  /**
   * The Content identity of the real copy at a global path *now*, where it
   * differs from what its row deployed — a copy edited in place since (Rust
   * `ownership::check` reads the folder). A copy absent here still holds
   * what its row recorded. Internal, never on the wire.
   */
  copyIdentities: Record<string, string>;
  /** Folders the fake native picker answers with, in rotation. */
  dialogPicks: string[];
  gitCacheEntries: number;
  /** Of `gitCacheEntries`, how many an acquisition is using: Clean Now skips them. */
  gitCacheEntriesInUse: number;
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

/**
 * The global Deployment folder a row keyed `key` describes and the folder
 * serving the harness `key` (Rust `placement::own_folder` /
 * `serving_folder`): `agents_skills` for `.agents`, `claude_code` for
 * `.claude`, otherwise the first registry entry sharing the dir. An unknown
 * key stands for itself.
 */
export function globalFolderOf(key: string): string {
  const dir = toolByKey(key)?.globalDir;
  if (dir === undefined) return key;
  return TOOL_REGISTRY.find((tool) => tool.globalDir === dir)?.key ?? key;
}

/**
 * Rust `placement::is_folder_row` (global scope): a row keyed by its own
 * folder key is a folder row; any other key — a pre-1.4.0 sibling's, an
 * unknown one — is a legacy row, reached through its own harness alone.
 */
export function isFolderRow(key: string): boolean {
  return toolByKey(key) !== undefined && globalFolderOf(key) === key;
}

/** Every registry key served by the global folder `folder`, in registry order. */
export function folderHarnesses(folder: string): string[] {
  return TOOL_REGISTRY.filter((tool) => globalFolderOf(tool.key) === folder).map((tool) => tool.key);
}

/**
 * The project Deployment folder of `key` (Rust `placement::own_folder` at
 * project scope): the virtual group owns `.agents/skills`, otherwise the
 * first registry entry sharing the project dir. Unknown keys stand for
 * themselves.
 */
export function projectFolderOf(key: string): string {
  const dir = toolByKey(key)?.projectDir;
  if (dir === undefined || dir === null) return key;
  if (dir === toolByKey(VIRTUAL_GROUP_KEY)?.projectDir) return VIRTUAL_GROUP_KEY;
  return TOOL_REGISTRY.find((tool) => tool.projectDir === dir)?.key ?? key;
}

/**
 * Rust `placement::fold_status`: one Deployment folder's facts over all its
 * rows — the displayed status, failure ahead of muted (`error` › `missing` ›
 * `stale` › `disabled` › `pending` › `synced`), the disabled intent (any row
 * `disabled`) and whether a live row owns its artifact (`synced_at` set, not
 * `disabled`). `undefined` for no rows.
 */
const DISPLAY_RANK: Record<string, number> = { error: 6, missing: 5, stale: 4, disabled: 3, pending: 2, synced: 1 };
export function foldStatus<S extends string>(
  rows: readonly { status: S; synced_at: number | null }[],
): ({ status: S } & FolderFacts) | undefined {
  let facts: ({ status: S } & FolderFacts) | undefined;
  for (const row of rows) {
    const disabled = row.status === "disabled";
    const deployed = !disabled && row.synced_at !== null;
    facts =
      facts === undefined
        ? { status: row.status, disabled, deployed }
        : {
            status: (DISPLAY_RANK[row.status] ?? 0) > (DISPLAY_RANK[facts.status] ?? 0) ? row.status : facts.status,
            disabled: facts.disabled || disabled,
            deployed: facts.deployed || deployed,
          };
  }
  return facts;
}

/**
 * Rust `FolderFacts::total_status`: what a folder adds to a project total —
 * its displayed status, except a `disabled` folder with a live artifact on
 * disk counts as `synced`.
 */
export function totalStatus<S extends string>(facts: { status: S } & FolderFacts): S | "synced" {
  return facts.status === "disabled" && facts.deployed ? "synced" : facts.status;
}

/**
 * Rust `placement::fold_folder_rows`, for display and counting only: per
 * owner, one row per folder keyed by the folder — its first row whose status
 * is the folder's displayed status, every other field its own — with the
 * folder's facts, in the order each folder first appears. Mutations never
 * read it: they act on every row by itself.
 */
export function foldFolderRows<R extends { tool: string; status: string; synced_at: number | null }>(
  rows: readonly R[],
  folderOf: (tool: string) => string,
  ownerOf: (row: R) => string = () => "",
): (R & FolderFacts)[] {
  const groups = new Map<string, { folder: string; rows: R[] }>();
  for (const row of rows) {
    const folder = folderOf(row.tool);
    const key = `${ownerOf(row)}\n${folder}`;
    const group = groups.get(key) ?? { folder, rows: [] };
    group.rows.push(row);
    groups.set(key, group);
  }
  return [...groups.values()].map(({ folder, rows: members }) => {
    const facts = foldStatus(members)!;
    const pick = members.find((row) => row.status === facts.status) ?? members[0];
    return { ...pick, tool: folder, disabled: facts.disabled, deployed: facts.deployed };
  });
}

/** Project-scope catalog entries (virtual-group constituents absorbed). */
export function projectCatalog(): FixtureTool[] {
  return TOOL_REGISTRY.filter((tool) => !tool.grouped && tool.projectDir !== null);
}

export function isProjectToolInstalled(state: FixtureState, key: string): boolean {
  if (key === VIRTUAL_GROUP_KEY) {
    return (
      state.installedTools.includes(key) ||
      TOOL_REGISTRY.some(
        (tool) => tool.grouped && state.installedTools.includes(tool.key),
      ) ||
      READER_REGISTRY.some((reader) => reader.detected)
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
  /** Tools whose global target row is `disabled` (the per-Tool grain of Disable). */
  disabled?: string[];
  /**
   * Tools whose first sync failed (D9): an `error` row that never
   * materialised, so it owns nothing (`synced_at` null).
   */
  failedFirstSync?: string[];
  /** The skill's stored last outcome (Skill health), settled `hoursAgo`. */
  health?: { kind: SkillOutcomeKind; error: CommandError; hoursAgo?: number };
  /**
   * Disabled at skill grain: `disabled_at` is set and every global target and
   * project assignment of the skill is `disabled` (off everywhere).
   */
  skillDisabled?: boolean;
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
  const contentHash = centralGone ? "" : hashFiles(files);
  const created = now - (spec.ageDays ?? 30 + (index % 40)) * DAY - index * 7 * MINUTE;
  const syncedAt = now - (spec.syncedHoursAgo ?? 2 + (index % 30)) * HOUR;
  const kind = spec.source.kind;
  const sourceRef =
    spec.source.kind === "git"
      ? `https://github.com/${spec.source.repo}`
      : spec.source.kind === "local"
        ? spec.source.path
        : null;
  const targets: StoredTarget[] = [];
  const modeOf = (tool: string): SyncMode => (spec.copyTools?.includes(tool) ? "copy" : (spec.mode ?? "symlink"));
  const row = (tool: string, status: SyncStatus): StoredTarget => ({
    tool,
    mode: modeOf(tool),
    status,
    target_path: `${globalRoot(tool)}/${spec.name}`,
    // Rust ownership (`owns_artifact`): a global `error` row is a deployed
    // target whose later sync failed (`SyncFailed` keeps `synced_at`; global
    // first sync records successes only), so it owns its path. A `disabled`
    // row from `spec.disabled` is a pair marker that never synced.
    synced_at: status === "synced" || status === "error" ? syncedAt : null,
    // A copy the app wrote records what it deployed (ADR-0007); a link
    // records nothing.
    deployed_hash: modeOf(tool) === "copy" && (status === "synced" || status === "error") ? contentHash : null,
  });
  // A skill disabled at skill grain has every row disabled (Rust
  // `skill_disable`): a synced row flips status, keeps where it was, and gives
  // its claim on the path up (`TargetTransition::Disabled`, ADR-0007).
  for (const tool of spec.synced ?? [])
    targets.push(
      spec.skillDisabled
        ? { ...row(tool, "synced"), status: "disabled", synced_at: null, deployed_hash: null }
        : row(tool, "synced"),
    );
  for (const tool of spec.errored ?? []) targets.push(row(tool, "error"));
  for (const tool of spec.disabled ?? []) targets.push(row(tool, "disabled"));
  for (const tool of spec.failedFirstSync ?? [])
    targets.push({ ...row(tool, "error"), synced_at: null, deployed_hash: null });
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
      last_sync_at: targets.some((t) => t.status === "synced") ? syncedAt : null,
      status: "ok",
      invocation_mode: effective,
      invocation_override: spec.override
        ? { mode: spec.override.mode, base_mode: base, conflict: spec.override.conflict ?? false }
        : null,
      targets,
      disabled_at: spec.skillDisabled ? now - 3 * DAY : null,
      refreshable,
      unlocatable: spec.unlocatable ?? null,
      detachable: kind === "local" && !centralGone,
      health: spec.health
        ? { kind: spec.health.kind, error: spec.health.error, at: now - (spec.health.hoursAgo ?? 2) * HOUR }
        : null,
      // Derived from the projects' assignment rows by the listing (`skillsDto`).
      projects: [],
    },
    files,
    contentHash,
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
        // A disabled assignment gave its claim up when its Disable succeeded (ADR-0007).
        synced_at: status === "pending" || status === "error" || status === "disabled" ? null : now - (5 + i) * HOUR,
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
  if (mode !== "copy" || status === "pending" || status === "error" || status === "disabled" || !current)
    return null;
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
