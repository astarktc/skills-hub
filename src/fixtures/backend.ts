// FixtureBackend — every backend command as a typed handler over the
// in-memory `FixtureState`. Mutations mutate the state and answer the real
// report/view shapes; the Channel-streaming commands tick simulated progress
// on a timer. `FIXTURE_HANDLERS satisfies FixtureHandlers` makes the table
// exhaustive over the generated `commands`: a new backend command without a
// fixture handler fails `npm run build`.
//
// Dev-only. `src/lib/tauri.ts` reaches this module through a dynamic import
// behind `import.meta.env.VITE_MOCK_BACKEND === "1"`, which production builds
// fold to false — none of it ships.

import type { Channel } from "@tauri-apps/api/core";
import type {
  AppSettings,
  BatchSyncPolicyDto,
  BatchSyncSkillDto,
  BatchTargetOutcome,
  BatchTargetStatus,
  CandidateMatch,
  CommandError,
  DisableScope,
  EnableReport,
  GitSkillCandidate,
  ImportGroupOutcome,
  ImportGroupStatus,
  ImportPolicyDto,
  ImportProgressDto,
  AssignmentDecision,
  BatchSyncOverrideDto,
  InstallBatchReport,
  InstallCandidateOutcome,
  InstallProgressDto,
  InstallRequest,
  InstallResultDto,
  InvocationEditConflict,
  InvocationMode,
  ManagedSkillDto,
  OnboardingGroup,
  OnboardingPlan,
  OnboardingSelectionDto,
  OriginalOutcome,
  ProjectDto,
  ProjectSkillAssignmentDto,
  ProjectSyncOutcome,
  ProjectSyncOutcomeStatus,
  ProjectSyncStatus,
  ProjectViewDto,
  PropagationOutcome,
  PropagationSkip,
  RefreshPolicyDto,
  RefreshProgressDto,
  RefreshReport,
  RemovalReport,
  RemovalTargetOutcome,
  RepointTarget,
  RowRef,
  SettingUpdate,
  SkillMutationResultDto,
  SkillOutcomeKind,
  SkillRefreshOutcome,
  SkillRefreshStatus,
  SyncMode,
  SyncProgressDto,
  ToolStatusDto,
} from "../bindings";
import type { CommandName, Commands } from "../lib/tauri";
import {
  HOME,
  basename,
  expandHome,
  fingerprint,
  globalRoot,
  globalSharers,
  hashFiles,
  isProjectToolInstalled,
  projectCatalog,
  projectToolDir,
  skillFiles,
  toolByKey,
  toolLabelOf,
  type FixtureFile,
  type FixtureProject,
  type FixtureSkill,
  type FixtureState,
} from "./model";
import { TOOL_REGISTRY, VIRTUAL_GROUP_KEY } from "./registry";

type Handler<K extends CommandName> = (
  ...args: Parameters<Commands[K]>
) => Promise<Awaited<ReturnType<Commands[K]>>>;

/** One handler per generated command, typed by the command's own signature. */
export type FixtureHandlers = { [K in CommandName]: Handler<K> };

export type FixtureBackendOptions = {
  /** Multiplier on every simulated latency; tests pass 0. */
  latencyScale?: number;
  /** Clock for timestamps (defaults to `Date.now`). */
  now?: () => number;
  /** How to wait `ms` of simulated latency (defaults to `setTimeout`). */
  sleep?: (ms: number) => Promise<void>;
};

/** A progress sink: the real `Channel`, or anything with an `onmessage`. */
type ProgressSink<T> = Pick<Channel<T>, "onmessage">;

const permissionDeniedText = (path: string) => `Permission denied (os error 13): ${path}`;
const PERMISSION_DENIED = (path: string): CommandError => ({
  code: "OTHER",
  message: permissionDeniedText(path),
});

export function createFixtureBackend(
  state: FixtureState,
  options: FixtureBackendOptions = {},
) {
  const scale = options.latencyScale ?? 1;
  const now = options.now ?? (() => Date.now());
  const sleep =
    options.sleep ?? ((ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms)));
  /** Explore previews: cache path → bytes (not part of the library). */
  const previews = new Map<string, FixtureFile[]>();

  const pause = (ms: number) => (ms * scale > 0 ? sleep(ms * scale) : Promise.resolve());
  /** One progress step, sized so a long batch still finishes in ~4s. */
  const step = (total: number) => pause(Math.max(25, Math.min(260, 4000 / Math.max(total, 1))));
  const emit = <T>(sink: ProgressSink<T>, tick: T) => {
    sink.onmessage(tick);
  };
  const fail = (error: CommandError): never => {
    throw error;
  };
  const clone = <T>(value: T): T => structuredClone(value);
  const newId = (prefix: string) => {
    state.nextId += 1;
    return `${prefix}-${state.nextId.toString(36)}`;
  };

  // -------------------------------------------------------------------------
  // Lookups
  // -------------------------------------------------------------------------

  const findSkill = (id: string): FixtureSkill =>
    state.skills.find((s) => s.dto.id === id) ??
    fail({ code: "NOT_FOUND", kind: "skill", id });

  const findProject = (id: string): FixtureProject =>
    state.projects.find((p) => p.id === id) ??
    fail({ code: "NOT_FOUND", kind: "project", id });

  const skillsDto = (): ManagedSkillDto[] => state.skills.map((s) => clone(s.dto));

  const isInstalled = (tool: string) => state.installedTools.includes(tool);

  const nameTaken = (name: string) =>
    state.skills.some((s) => s.dto.name.toLowerCase() === name.toLowerCase());

  const insideToolDir = (path: string) =>
    TOOL_REGISTRY.find((tool) => path.startsWith(`${globalRoot(tool.key)}/`));

  // -------------------------------------------------------------------------
  // Creating skills (finalize)
  // -------------------------------------------------------------------------

  const finalize = (input: {
    name: string;
    description: string;
    sourceType: "git" | "local" | "imported";
    sourceRef: string | null;
    importedFrom?: string | null;
    invocation?: InvocationMode;
    hashOverride?: string;
  }): FixtureSkill => {
    if (nameTaken(input.name)) fail({ code: "SKILL_EXISTS", name: input.name });
    const files = skillFiles(input.name, input.description, input.invocation ?? "user-and-model");
    const at = now();
    const skill: FixtureSkill = {
      dto: {
        id: newId("skill"),
        name: input.name,
        description: input.description,
        source_type: input.sourceType,
        source_ref: input.sourceRef,
        imported_from_tool: input.importedFrom ?? null,
        central_path: `${state.settings.central_repo_path}/${input.name}`,
        created_at: at,
        updated_at: at,
        last_sync_at: null,
        status: "ok",
        invocation_mode: input.invocation ?? "user-and-model",
        invocation_override: null,
        targets: [],
        disabled_at: null,
        refreshable: input.sourceType !== "imported",
        unlocatable: null,
        detachable: input.sourceType === "local",
        health: null,
      },
      files,
      contentHash: input.hashOverride ?? hashFiles(files),
      acquisition: {},
    };
    state.skills.push(skill);
    return skill;
  };

  const installResult = (skill: FixtureSkill): InstallResultDto => ({
    skill_id: skill.dto.id,
    name: skill.dto.name,
    central_path: skill.dto.central_path,
    content_hash: skill.contentHash,
  });

  /**
   * A folder in a target's way, with what the one overwrite ask shows
   * (Rust `sync_engine::Occupant`, D12): every seeded occupant was last
   * modified three days before the fixture's clock.
   */
  const targetExists = (path: string, sameContent = false): CommandError => ({
    code: "TARGET_EXISTS",
    path,
    modifiedAt: now() - 3 * 24 * 60 * 60 * 1000,
    sameContent,
  });

  // -------------------------------------------------------------------------
  // Global sync
  // -------------------------------------------------------------------------

  /** Record (or refresh) the target rows of every detected Tool sharing the root. */
  const recordTargets = (skill: FixtureSkill, tool: string) => {
    const path = `${globalRoot(tool)}/${skill.dto.name}`;
    const at = now();
    artifacts.set(path, true);
    for (const sharer of globalSharers(tool).filter(isInstalled)) {
      const row = skill.dto.targets.find((t) => t.tool === sharer);
      if (row) {
        row.status = "synced";
        row.mode = "symlink";
        row.synced_at = at;
        row.target_path = path;
      } else {
        skill.dto.targets.push({ tool: sharer, mode: "symlink", status: "synced", target_path: path, synced_at: at });
      }
    }
    skill.dto.last_sync_at = at;
  };

  /**
   * A failed sync settles its rows (Rust `global_sync::record_failed`, D9):
   * every detected Tool sharing the root gets an `error` row — a new one owns
   * nothing (`synced_at` null), an existing one keeps its ownership. An
   * unanswered TARGET_EXISTS never reaches here (it is asked first).
   */
  const recordFailedTargets = (skill: FixtureSkill, tool: string) => {
    const path = `${globalRoot(tool)}/${skill.dto.name}`;
    for (const sharer of globalSharers(tool).filter(isInstalled)) {
      const row = skill.dto.targets.find((t) => t.tool === sharer);
      if (row) row.status = "error";
      else skill.dto.targets.push({ tool: sharer, mode: "symlink", status: "error", target_path: path, synced_at: null });
    }
  };

  /**
   * "Keep existing" (Rust `global_sync::record_kept`): the attempted Tool's
   * pair is recorded `disabled` — a marker owning nothing — and reported as
   * any disabled pair is.
   */
  const recordKept = (skill: FixtureSkill, tool: string): BatchTargetStatus => {
    const row = skill.dto.targets.find((t) => t.tool === tool);
    if (row) row.status = "disabled";
    else
      skill.dto.targets.push({
        tool,
        mode: "symlink",
        status: "disabled",
        target_path: `${globalRoot(tool)}/${skill.dto.name}`,
        synced_at: null,
      });
    // The report row says what the operator did (Rust `record_kept`).
    return { status: "declined" };
  };

  /** One (skill, tool root) attempt under the overwrite rule. */
  const syncPair = (
    skill: FixtureSkill,
    tool: string,
    overwrite: boolean,
    overwriteIfSameContent: boolean,
  ): BatchTargetStatus => {
    const root = globalRoot(tool);
    const path = `${root}/${skill.dto.name}`;
    if (state.unwritableTools.includes(tool)) {
      recordFailedTargets(skill, tool);
      return { status: "skipped", error: { code: "TOOL_NOT_WRITABLE", tool, path: root } };
    }
    if (skill.files.length === 0) {
      recordFailedTargets(skill, tool);
      return { status: "failed", error: { code: "CENTRAL_PATH_MISSING", path: skill.dto.central_path } };
    }
    const occupantIndex = state.foreign.findIndex(
      (f) => globalRoot(f.tool) === root && f.name === skill.dto.name,
    );
    let replaced = false;
    if (occupantIndex >= 0) {
      const same = state.foreign[occupantIndex].fingerprint === skill.contentHash;
      if (!(overwrite || (same && overwriteIfSameContent))) {
        return { status: "failed", error: targetExists(path, same) };
      }
      state.foreign.splice(occupantIndex, 1);
      replaced = true;
    }
    recordTargets(skill, tool);
    return { status: "synced", outcome: { mode_used: "symlink", target_path: path, replaced } };
  };

  const syncBatch = async (
    skills: BatchSyncSkillDto[],
    tools: string[],
    policy: BatchSyncPolicyDto,
    onProgress: ProgressSink<SyncProgressDto> | null,
  ): Promise<BatchTargetOutcome[]> => {
    const outcomes: BatchTargetOutcome[] = [];
    const seenRoots = new Set<string>();
    const attempted: string[] = [];
    for (const key of tools) {
      if (!toolByKey(key)) {
        for (const s of skills) {
          outcomes.push({ skill_id: s.skill_id, skill_name: s.name, tool_key: key, status: { status: "failed", error: { code: "UNKNOWN_TOOL", tool: key } } });
        }
        continue;
      }
      if (!isInstalled(key)) {
        for (const s of skills) {
          outcomes.push({ skill_id: s.skill_id, skill_name: s.name, tool_key: key, status: { status: "skipped", error: { code: "TOOL_NOT_INSTALLED", tool: key } } });
        }
        continue;
      }
      const root = globalRoot(key);
      if (seenRoots.has(root)) continue;
      seenRoots.add(root);
      attempted.push(key);
    }
    const total = skills.length * attempted.length;
    let index = 0;
    for (const s of skills) {
      for (const key of attempted) {
        index += 1;
        if (onProgress) {
          emit(onProgress, { index, total, skill_name: s.name, tool: key });
          await step(total);
        }
        const skill = state.skills.find((x) => x.dto.id === s.skill_id);
        if (!skill) {
          outcomes.push({ skill_id: s.skill_id, skill_name: s.name, tool_key: key, status: { status: "failed", error: { code: "NOT_FOUND", kind: "skill", id: s.skill_id } } });
          continue;
        }
        if (isDisabledFor(skill, key)) {
          outcomes.push({ skill_id: s.skill_id, skill_name: s.name, tool_key: key, status: { status: "skipped", error: { code: "TARGET_DISABLED", tool: key } } });
          continue;
        }
        const decided = (decision: "overwrite" | "keep") =>
          (policy.overrides ?? []).some(
            (o) => o.decision === decision && o.skill_id === s.skill_id && globalSharers(key).includes(o.tool),
          );
        if (decided("keep")) {
          outcomes.push({ skill_id: s.skill_id, skill_name: s.name, tool_key: key, status: recordKept(skill, key) });
          continue;
        }
        const overwrite = (policy.overwrite ?? false) || decided("overwrite");
        outcomes.push({
          skill_id: s.skill_id,
          skill_name: s.name,
          tool_key: key,
          status: syncPair(skill, key, overwrite, policy.overwrite_if_same_content ?? false),
        });
      }
    }
    return outcomes;
  };

  /**
   * The batch's Disabled rule (Rust `global_sync::DisabledGate`): a skill
   * disabled at skill grain, or any `disabled` row among the Tools sharing
   * this Tool's skills dir, skips the pair — one shared artifact.
   */
  const isDisabledFor = (skill: FixtureSkill, tool: string) =>
    skill.dto.disabled_at !== null ||
    skill.dto.targets.some(
      (t) => t.status === "disabled" && (t.tool === tool || globalSharers(tool).includes(t.tool)),
    );

  /** Effective global target set: the recorded selection, else every detected Tool. */
  const effectiveTargets = () => state.settings.global_selected_tools ?? state.installedTools;

  // -------------------------------------------------------------------------
  // Artifact removal
  // -------------------------------------------------------------------------

  // Artifact removal (Rust `artifact_removal::execute_unlocked`). One presence
  // rule — an absent artifact is removed trivially — and one settlement rule:
  // a present artifact that cannot be removed (a locked path, or a parent
  // dir that refuses writes: unlinking needs write access to the dir) keeps
  // its rows with Sync status `error` (ADR-0002).
  //
  // Presence is modelled by path, independent of any row's lifecycle status
  // (Rust asks `symlink_metadata`, never the row): every successful write
  // sets it, only a successful removal clears it, and a failed removal leaves
  // it set — so a retry on the same locked path fails again. A path this
  // backend never wrote or removed falls back to what the scenario's rows
  // imply: a global artifact is there unless every row describing it is
  // `disabled`; an assignment's when it was deployed (`synced`/`stale`) in an
  // existing folder.
  //
  // Ownership (Rust `owns_artifact`), independent of the row's status: only a
  // row the app materialised (`synced_at !== null`) owns its path. A row that
  // never did (a `disabled` marker, or one a failed Enable turned `error`) is
  // a marker: removal never plans its path, and re-sync / Propagation never
  // overwrite it. Once the whole scope succeeded, Delete clears the row alone
  // and Disable records it `disabled`; until then it keeps status and
  // `last_error`.

  /** Artifact presence by path (see above). */
  const artifacts = new Map<string, boolean>();
  const presentAt = (path: string, implied: boolean) => artifacts.get(path) ?? implied;

  /**
   * `delete` is every removal's settlement; `disable` keeps the removed rows
   * with status `disabled` (Rust `Settlement::Disable`) and skips rows that
   * are already disabled. A failure keeps the row `error` either way.
   */
  type Settle = "delete" | "disable";

  /** A row owning no artifact, set aside by planning, cleared with its scope. */
  type Marker = { row: RowRef; clear: () => void; disable: () => void };
  /** One removal scope's settled targets and its set-aside markers. */
  type Removal = { targets: RemovalTargetOutcome[]; markers: Marker[] };

  const ownsArtifact = (row: { synced_at: number | null }) => row.synced_at !== null;

  /** Rust `Settlement::Disable` on markers: the row flips, no path is touched. */
  const disableMarkers = (settle: Settle, removal: Removal): Removal => {
    if (settle !== "disable") return removal;
    if (!failedIn(removal.targets)) for (const marker of removal.markers) marker.disable();
    return { targets: removal.targets, markers: [] };
  };

  const removeGlobalRows = (
    skill: FixtureSkill,
    tools: string[],
    settle: Settle = "delete",
  ): Removal => {
    const byPath = new Map<string, string[]>();
    const markers: Marker[] = [];
    const inScope = skill.dto.targets.filter(
      (t) => tools.includes(t.tool) && !(settle === "disable" && t.status === "disabled"),
    );
    for (const row of inScope) {
      if (!ownsArtifact(row)) {
        markers.push({
          row: { scope: "global_target", id: `tgt-${skill.dto.id}-${row.tool}`, skill_id: skill.dto.id, tool: row.tool },
          clear: () => {
            skill.dto.targets = skill.dto.targets.filter((t) => t !== row);
          },
          disable: () => {
            row.status = "disabled";
          },
        });
        continue;
      }
      byPath.set(row.target_path, [...(byPath.get(row.target_path) ?? []), row.tool]);
    }
    const targets: RemovalTargetOutcome[] = [];
    for (const [path, rowTools] of byPath) {
      const rows = rowTools.map((tool) => ({ scope: "global_target" as const, id: `tgt-${skill.dto.id}-${tool}`, skill_id: skill.dto.id, tool }));
      const described = skill.dto.targets.filter((t) => rowTools.includes(t.tool));
      const present = presentAt(path, described.some((t) => t.status !== "disabled"));
      const stuck = state.lockedPaths.includes(path) || rowTools.some((tool) => state.unwritableTools.includes(tool));
      if (present && stuck) {
        artifacts.set(path, true);
        for (const row of described) row.status = "error";
        targets.push({ path, rows, status: { status: "failed", error: PERMISSION_DENIED(path) } });
      } else {
        artifacts.set(path, false);
        if (settle === "disable") {
          for (const row of described) row.status = "disabled";
        } else {
          skill.dto.targets = skill.dto.targets.filter((t) => !rowTools.includes(t.tool));
        }
        targets.push({ path, rows, status: { status: "removed" } });
      }
    }
    return disableMarkers(settle, { targets, markers });
  };

  const assignmentPath = (project: FixtureProject, a: ProjectSkillAssignmentDto) =>
    `${projectToolDir(project, a.tool)}/${a.skill_name}`;

  const removeAssignments = (
    project: FixtureProject,
    keep: (a: ProjectSkillAssignmentDto) => boolean,
    settle: Settle = "delete",
  ): Removal => {
    const targets: RemovalTargetOutcome[] = [];
    const markers: Marker[] = [];
    const remaining: ProjectSkillAssignmentDto[] = [];
    for (const a of project.assignments) {
      if (keep(a) || (settle === "disable" && a.status === "disabled")) {
        remaining.push(a);
        continue;
      }
      const rows = [{ scope: "assignment" as const, id: a.id, project_id: project.id, skill_id: a.skill_id, tool: a.tool }];
      if (!ownsArtifact(a)) {
        remaining.push(a);
        markers.push({
          row: rows[0],
          clear: () => {
            project.assignments = project.assignments.filter((x) => x !== a);
          },
          disable: () => {
            a.status = "disabled";
            a.last_error = null;
            a.content_hash = null;
          },
        });
        continue;
      }
      const path = assignmentPath(project, a);
      const present = project.path_exists && presentAt(path, a.status === "synced" || a.status === "stale");
      const stuck =
        state.lockedPaths.includes(path) || state.unwritableProjectTools.includes(`${project.id}:${a.tool}`);
      if (present && stuck) {
        artifacts.set(path, true);
        a.status = "error";
        a.last_error = permissionDeniedText(path);
        remaining.push(a);
        targets.push({ path, rows, status: { status: "failed", error: PERMISSION_DENIED(path) } });
      } else {
        artifacts.set(path, false);
        // Whatever sat at a path this row owns is removed with it.
        state.projectOccupants = state.projectOccupants.filter((p) => p !== path);
        if (settle === "disable") {
          a.status = "disabled";
          a.last_error = null;
          a.content_hash = null;
          remaining.push(a);
        }
        targets.push({ path, rows, status: { status: "removed" } });
      }
    }
    project.assignments = remaining;
    return disableMarkers(settle, { targets, markers });
  };

  const failedIn = (targets: RemovalTargetOutcome[]) =>
    targets.some((t) => t.status.status === "failed");

  /**
   * The scope's report. Markers are cleared only when nothing in the whole
   * scope failed (a kept skill or project keeps its recorded intent).
   */
  const removal = (
    parts: Removal | Removal[],
    extra: Partial<Pick<RemovalReport, "central_removed" | "record_deleted">> = {},
  ): RemovalReport => {
    const all = Array.isArray(parts) ? parts : [parts];
    const targets = all.flatMap((part) => part.targets);
    const markers = all.flatMap((part) => part.markers);
    const cleared = failedIn(targets) ? [] : markers;
    for (const marker of cleared) marker.clear();
    return {
      targets,
      central_removed: false,
      record_deleted: false,
      cleared_markers: cleared.map((marker) => marker.row),
      ...extra,
    };
  };

  // -------------------------------------------------------------------------
  // Projects
  // -------------------------------------------------------------------------

  const aggregate = (project: FixtureProject): ProjectSyncStatus => {
    // Rust `sync_status::aggregate`: a disabled row takes no part.
    const live = project.assignments.filter((a) => a.status !== "disabled");
    if (live.length === 0) return "none";
    const rank = { synced: 1, pending: 2, stale: 3, error: 4 } as const;
    let worst: keyof typeof rank = "synced";
    for (const a of live) {
      const folded = a.status === "missing" ? "error" : (a.status as keyof typeof rank);
      if (rank[folded] > rank[worst]) worst = folded;
    }
    return worst;
  };

  const projectDto = (project: FixtureProject): ProjectDto => ({
    id: project.id,
    path: project.path,
    name: project.name,
    created_at: project.created_at,
    updated_at: project.updated_at,
    tool_count: project.tools.length,
    skill_count: new Set(project.assignments.map((a) => a.skill_id)).size,
    assignment_count: project.assignments.length,
    sync_status: aggregate(project),
    path_exists: project.path_exists,
  });

  const projectsDto = () => state.projects.map(projectDto);

  const viewOf = (project: FixtureProject): ProjectViewDto => ({
    project: projectDto(project),
    tools: project.tools.map((tool) => ({ id: `${project.id}-tool-${tool}`, project_id: project.id, tool })),
    assignments: clone(project.assignments),
    reconciled: true,
  });

  /**
   * One assignment through Rust `project_sync::sync_assignment_target`.
   * `overwrite` is the caller's policy: off for a first sync (assign, Enable),
   * and for a re-sync / Propagation only a row that owns its path may replace
   * what is there (Rust `owns_artifact`). Someone else's folder in the way
   * of a sync without overwrite is refused as `TARGET_EXISTS`, the global
   * batch's code.
   */
  const syncAssignment = (
    project: FixtureProject,
    a: ProjectSkillAssignmentDto,
    overwrite: boolean,
  ): ProjectSyncOutcomeStatus => {
    const skill = state.skills.find((s) => s.dto.id === a.skill_id);
    const dir = projectToolDir(project, a.tool);
    const path = assignmentPath(project, a);
    let error: CommandError | null = null;
    if (!project.path_exists) error = { code: "INVALID_PATH", path: project.path, reason: "missing" };
    else if (!skill || skill.files.length === 0)
      error = { code: "CENTRAL_PATH_MISSING", path: skill?.dto.central_path ?? a.skill_name };
    else if (state.unwritableProjectTools.includes(`${project.id}:${a.tool}`))
      error = { code: "TOOL_NOT_WRITABLE", tool: a.tool, path: dir };
    if (error) {
      a.status = "error";
      // `last_error` carries the backend's diagnostic chain (never user copy).
      a.last_error =
        error.code === "TOOL_NOT_WRITABLE"
          ? permissionDeniedText(`${dir}/${a.skill_name}`)
          : error.code === "INVALID_PATH"
            ? `project directory not found: ${error.path}`
            : `central copy not found: ${skill?.dto.central_path ?? a.skill_name}`;
      return { status: "failed", error };
    }
    if (state.projectOccupants.includes(path)) {
      if (!overwrite) {
        // Typed like the global batch's collision (Rust `CommandError::from_anyhow`
        // recovers `sync_engine::TargetExistsError`); `last_error` keeps the chain.
        a.status = "error";
        a.last_error = `target already exists: "${path}"`;
        a.content_hash = null;
        return { status: "failed", error: targetExists(path) };
      }
      state.projectOccupants = state.projectOccupants.filter((p) => p !== path);
    }
    a.status = "synced";
    a.mode = "symlink";
    a.last_error = null;
    a.synced_at = now();
    artifacts.set(path, true);
    // Rust `project_sync::sync_assignment_target`: only a copy records a
    // content hash; a link follows the central copy and records none.
    a.content_hash = null;
    project.updated_at = now();
    return { status: "synced" };
  };

  const outcomeOf = (
    project: FixtureProject,
    a: ProjectSkillAssignmentDto,
    status: ProjectSyncOutcomeStatus,
  ): ProjectSyncOutcome => ({
    assignment_id: a.id,
    project_id: project.id,
    skill_id: a.skill_id,
    skill_name: a.skill_name,
    tool: a.tool,
    status,
  });

  /** A disabled row, or any row of a skill disabled at skill grain, is skipped. */
  const skippedDisabled: ProjectSyncOutcomeStatus = { status: "skipped", reason: { reason: "disabled" } };

  const assign = (project: FixtureProject, skill: FixtureSkill, tool: string): ProjectSyncOutcome => {
    // Rust `project_sync::assign_skill_to_tools`: a skill disabled at skill
    // grain gets the assignment recorded `disabled`, not synced.
    const disabled = skill.dto.disabled_at !== null;
    const a: ProjectSkillAssignmentDto = {
      id: newId(`${project.id}-asg`),
      project_id: project.id,
      skill_id: skill.dto.id,
      skill_name: skill.dto.name,
      tool,
      mode: "symlink",
      status: disabled ? "disabled" : "pending",
      last_error: null,
      synced_at: null,
      content_hash: null,
      created_at: now(),
    };
    project.assignments.push(a);
    return outcomeOf(project, a, disabled ? skippedDisabled : syncAssignment(project, a, false));
  };

  /**
   * Rust `resync_project_unlocked` / bulk assign: a disabled row, or any row
   * of a skill disabled at skill grain (a failed skill Disable keeps its rows
   * `error`), is skipped before anything is written.
   */
  const isDisabledAssignment = (a: ProjectSkillAssignmentDto) =>
    a.status === "disabled" ||
    (state.skills.find((s) => s.dto.id === a.skill_id)?.dto.disabled_at ?? null) !== null;

  /**
   * Project re-sync: a disabled assignment is skipped (see above); only a row
   * that owns its path replaces what is there, any other is a first sync.
   */
  const resyncOne = (project: FixtureProject, a: ProjectSkillAssignmentDto): ProjectSyncOutcomeStatus =>
    isDisabledAssignment(a) ? skippedDisabled : syncAssignment(project, a, ownsArtifact(a));

  // -------------------------------------------------------------------------
  // Disable / Enable (Rust `skill_disable`)
  // -------------------------------------------------------------------------

  /**
   * One pair: the Tool's shared-dir group when any member is installed, else
   * the row alone (removal's stored-path fallback); removed rows are kept
   * `disabled`, and a pair with no row gets a marker row — only when the
   * shared artifact settled: a failed removal mints no `disabled` sibling.
   */
  const disablePair = (skill: FixtureSkill, tool: string): Removal => {
    const hasRow = skill.dto.targets.some((t) => t.tool === tool);
    if (!toolByKey(tool) && !hasRow) fail({ code: "UNKNOWN_TOOL", tool });
    const group = globalSharers(tool).some(isInstalled) ? globalSharers(tool) : [tool];
    const out = removeGlobalRows(skill, group, "disable");
    if (!failedIn(out.targets) && !skill.dto.targets.some((t) => t.tool === tool)) {
      skill.dto.targets.push({
        tool,
        mode: "symlink",
        status: "disabled",
        target_path: `${globalRoot(tool)}/${skill.dto.name}`,
        synced_at: null,
      });
    }
    return out;
  };

  /** One skill everywhere: global rows, then every assignment; then the marker. */
  const disableSkill = (skill: FixtureSkill): Removal[] => {
    const out = [removeGlobalRows(skill, skill.dto.targets.map((t) => t.tool), "disable")];
    for (const project of state.projects) {
      out.push(removeAssignments(project, (a) => a.skill_id !== skill.dto.id, "disable"));
    }
    skill.dto.disabled_at ??= now();
    return out;
  };

  /**
   * Enable's first sync (Rust `skill_disable::first_sync_global`): a folder in
   * the way is the one ask's (D12) — until answered the pair stays `disabled`
   * (a marker owning nothing), and the ask's retry is the same scope with the
   * per-pair decisions; any other failure is the batch's own `error` row.
   */
  const firstSync = async (skill: FixtureSkill, tools: string[], overrides: BatchSyncOverrideDto[]) => {
    const outcomes = await syncBatch(
      [{ skill_id: skill.dto.id, name: skill.dto.name, source_path: skill.dto.central_path }],
      tools,
      {
        overwrite: false,
        overwrite_if_same_content: true,
        overrides: overrides.filter((o) => o.skill_id === skill.dto.id),
      },
      null,
    );
    for (const o of outcomes) {
      if (o.status.status === "failed" && o.status.error.code === "TARGET_EXISTS") {
        recordKept(skill, o.tool_key);
      }
    }
    return outcomes;
  };

  /** Only a skill-grain Enable lifts a skill-grain Disable. */
  const enablePair = async (
    skill: FixtureSkill,
    tool: string,
    overrides: BatchSyncOverrideDto[],
  ): Promise<EnableReport> => {
    if (skill.dto.disabled_at !== null) {
      return {
        global: [{ skill_id: skill.dto.id, skill_name: skill.dto.name, tool_key: tool, status: { status: "skipped", error: { code: "TARGET_DISABLED", tool } } }],
        projects: { items: [] },
      };
    }
    const group = toolByKey(tool) ? globalSharers(tool) : [tool];
    skill.dto.targets = skill.dto.targets.filter((t) => !(t.status === "disabled" && group.includes(t.tool)));
    return { global: await firstSync(skill, [tool], overrides), projects: { items: [] } };
  };

  /**
   * Clear the marker; re-sync every disabled row as a first sync. Admission
   * (Rust `AssignmentTransition::EnableAdmitted`) clears a disabled
   * assignment's historical ownership — its Disable removed the artifact — so
   * a folder in the way fails it (`error` with the diagnostic) as a row that
   * owns nothing (`synced_at` null), which the overwrite ask can settle.
   */
  const enableSkill = async (skill: FixtureSkill, overrides: BatchSyncOverrideDto[]): Promise<EnableReport> => {
    skill.dto.disabled_at = null;
    const tools = skill.dto.targets.filter((t) => t.status === "disabled").map((t) => t.tool);
    skill.dto.targets = skill.dto.targets.filter((t) => t.status !== "disabled");
    const items: ProjectSyncOutcome[] = [];
    for (const project of state.projects) {
      for (const a of project.assignments.filter((x) => x.skill_id === skill.dto.id && x.status === "disabled")) {
        a.synced_at = null;
        items.push(outcomeOf(project, a, syncAssignment(project, a, false)));
      }
    }
    return { global: tools.length ? await firstSync(skill, tools, overrides) : [], projects: { items } };
  };

  // -------------------------------------------------------------------------
  // Propagation / Refresh
  // -------------------------------------------------------------------------

  /**
   * Rust `propagation::needs_new_bytes`: a target needs its bytes written again
   * when its Sync mode can drift (a copy) or its Tool cannot consume a link.
   * Every registry Tool supports symlinks, so the mode alone decides — never the
   * row's stored status: an errored *link* is skipped (`link_follows_source`),
   * an errored *copy* is retried.
   */
  const needsNewBytes = (mode: SyncMode) => mode === "copy";

  // -------------------------------------------------------------------------
  // Skill health (Rust `skill_health`): one stored last outcome per skill,
  // written where the outcome settles, cleared by its family's next success.
  // -------------------------------------------------------------------------

  const UPDATE_FAMILY: readonly SkillOutcomeKind[] = ["fetch_failed", "repoint_failed", "restore_failed"];
  const EDIT_FAMILY: readonly SkillOutcomeKind[] = ["edit_conflict"];
  const recordHealth = (skill: FixtureSkill, kind: SkillOutcomeKind, error: CommandError) => {
    skill.dto.health = { kind, error, at: now() };
  };
  const clearHealth = (skill: FixtureSkill, family: readonly SkillOutcomeKind[]) => {
    if (skill.dto.health && family.includes(skill.dto.health.kind)) skill.dto.health = null;
  };
  /**
   * A failed Update / Restore / Re-point (Rust `refresh::settle_failure_health`):
   * a cancellation records nothing; Restore is an Update of a skill whose
   * central copy is gone.
   */
  const recordUpdateFailure = (skill: FixtureSkill, repoint: boolean, error: CommandError) => {
    if (error.code === "CANCELLED") return;
    const kind: SkillOutcomeKind = repoint
      ? "repoint_failed"
      : skill.files.length === 0
        ? "restore_failed"
        : "fetch_failed";
    recordHealth(skill, kind, error);
  };
  /**
   * A settled Update (Rust `skill_update::settle_health`): a replay conflict
   * records `edit_conflict`; otherwise the Update family clears, and the Edit
   * family too once the Edit no longer conflicts.
   */
  const settleUpdateHealth = (skill: FixtureSkill, conflict: InvocationEditConflict | null) => {
    if (conflict) {
      recordHealth(skill, "edit_conflict", {
        code: "EDIT_CONFLICT",
        baseMode: conflict.base_mode,
        upstreamMode: conflict.upstream_mode,
        overrideMode: conflict.override_mode,
      });
      return;
    }
    clearHealth(skill, UPDATE_FAMILY);
    if (!skill.dto.invocation_override?.conflict) clearHealth(skill, EDIT_FAMILY);
  };

  /** A missing central copy, as Propagation reports it per target. */
  const missingSource = (skill: FixtureSkill): CommandError => ({
    code: "INVALID_PATH",
    path: skill.dto.central_path,
    reason: "missing",
  });

  /**
   * Propagation (Rust `propagation::propagate_unlocked`): global rows one
   * shared-skills-dir group at a time (one artifact, every member row settled),
   * then every project assignment of the skill.
   */
  const propagate = (skill: FixtureSkill): PropagationOutcome[] => {
    const out: PropagationOutcome[] = [];
    const skippedGlobal = (tool: string, reason: PropagationSkip) => {
      out.push({ scope: { scope: "global", tool }, status: { status: "skipped", reason } });
    };
    const centralMissing = skill.files.length === 0;
    if (skill.dto.disabled_at !== null) {
      // Off everywhere until enabled: every row reported, none touched.
      for (const row of skill.dto.targets) skippedGlobal(row.tool, { reason: "disabled" });
      for (const project of state.projects)
        for (const a of project.assignments.filter((x) => x.skill_id === skill.dto.id))
          out.push({ scope: { scope: "project", project_id: project.id, tool: a.tool }, status: { status: "skipped", reason: { reason: "disabled" } } });
      return out;
    }
    const handled = new Set<string>();
    for (const row of skill.dto.targets) {
      if (handled.has(row.tool)) continue;
      handled.add(row.tool);
      if (!toolByKey(row.tool)) {
        skippedGlobal(row.tool, row.status === "disabled" ? { reason: "disabled" } : { reason: "unknown_tool", tool: row.tool });
        continue;
      }
      const sharers = globalSharers(row.tool);
      const group = skill.dto.targets.filter(
        (r) => (r.tool === row.tool || sharers.includes(r.tool)) && toolByKey(r.tool),
      );
      for (const member of group) handled.add(member.tool);
      // One disabled member disables the shared artifact.
      if (group.some((member) => member.status === "disabled")) {
        for (const member of group) skippedGlobal(member.tool, { reason: "disabled" });
        continue;
      }
      for (const member of group.filter((r) => !isInstalled(r.tool))) {
        skippedGlobal(member.tool, { reason: "tool_not_installed", tool: member.tool });
      }
      const installed = group.filter((r) => isInstalled(r.tool));
      if (installed.length === 0) continue;
      // Only a link of ours follows the source: a row that never materialised
      // (a failed first sync's `error` row) gets a first sync.
      if (!installed.some((member) => needsNewBytes(member.mode) || !ownsArtifact(member))) {
        for (const member of installed) skippedGlobal(member.tool, { reason: "link_follows_source" });
        continue;
      }
      const driverPath = installed[0].target_path;
      // A first sync (no member owns the path) meets a foreign folder as
      // TARGET_EXISTS; an owner replaces it.
      const occupied =
        !installed.some(ownsArtifact) &&
        state.foreign.some((f) => `${globalRoot(f.tool)}/${f.name}` === driverPath);
      // An unwritable dir surfaces as the sync engine's io error, which the
      // command seam classifies as `OTHER` (not the batch's TOOL_NOT_WRITABLE).
      const error: CommandError | null = centralMissing
        ? missingSource(skill)
        : state.unwritableTools.includes(installed[0].tool)
          ? PERMISSION_DENIED(driverPath)
          : occupied
            ? targetExists(driverPath)
            : null;
      const at = now();
      for (const member of installed) {
        const scope = { scope: "global" as const, tool: member.tool };
        if (error) {
          member.status = "error";
          out.push({ scope, status: { status: "failed", error } });
        } else {
          member.status = "synced";
          member.mode = "symlink";
          member.synced_at = at;
          member.target_path = driverPath;
          out.push({ scope, status: { status: "synced", mode_used: "symlink" } });
        }
      }
    }
    for (const project of state.projects) {
      for (const a of project.assignments.filter((x) => x.skill_id === skill.dto.id)) {
        const scope = { scope: "project" as const, project_id: project.id, tool: a.tool };
        const skip = (reason: PropagationSkip) => out.push({ scope, status: { status: "skipped", reason } });
        if (a.status === "disabled") {
          skip({ reason: "disabled" });
        } else if (!toolByKey(a.tool)) {
          skip({ reason: "unknown_tool", tool: a.tool });
        } else if (!project.path_exists) {
          skip({ reason: "project_unavailable", project_id: project.id });
        } else if (ownsArtifact(a) && !needsNewBytes(a.mode)) {
          skip({ reason: "link_follows_source" });
        } else if (centralMissing) {
          a.status = "error";
          a.last_error = `path not found: ${skill.dto.central_path}`;
          out.push({ scope, status: { status: "failed", error: missingSource(skill) } });
        } else {
          const result = syncAssignment(project, a, ownsArtifact(a));
          out.push({
            scope,
            status: result.status === "failed" ? { status: "failed", error: result.error } : { status: "synced", mode_used: a.mode },
          });
        }
      }
    }
    return out;
  };

  /** Rebuild the central copy from the (fake) source; a new revision when upstream moved. */
  const landBytes = (skill: FixtureSkill, revision: number) => {
    skill.files = skillFiles(skill.dto.name, skill.dto.description ?? skill.dto.name, skill.dto.invocation_mode, revision);
    skill.contentHash = hashFiles(skill.files);
    skill.dto.updated_at = now();
    skill.dto.unlocatable = null;
    skill.dto.detachable = skill.dto.source_type === "local";
  };

  /**
   * The auto-sync invariant, re-asserted (Rust `refresh::reassert_auto_sync_unlocked`):
   * the global sync batch over every effective target Tool the skill has no
   * Propagation outcome for — not intersected with detection — with the
   * same-content policy, its outcomes converted to Propagation outcomes: an
   * uninstalled Tool is a skip, every other skip (unwritable) a failure.
   */
  const reassert = async (skill: FixtureSkill, already: PropagationOutcome[]): Promise<PropagationOutcome[]> => {
    const existing = new Set(already.flatMap((o) => (o.scope.scope === "global" ? [o.scope.tool] : [])));
    const missing = effectiveTargets().filter((key) => !existing.has(key));
    if (missing.length === 0) return [];
    const outcomes = await syncBatch(
      [{ skill_id: skill.dto.id, name: skill.dto.name, source_path: skill.dto.central_path }],
      missing,
      { overwrite: false, overwrite_if_same_content: true },
      null,
    );
    return outcomes.map((o): PropagationOutcome => {
      const scope = { scope: "global" as const, tool: o.tool_key };
      const s = o.status;
      if (s.status === "synced") return { scope, status: { status: "synced", mode_used: s.outcome.mode_used } };
      if (s.status === "skipped" && s.error.code === "TOOL_NOT_INSTALLED") {
        return { scope, status: { status: "skipped", reason: { reason: "tool_not_installed", tool: s.error.tool } } };
      }
      // The re-assert carries no overrides, so it never declines; a declined
      // pair is recorded disabled, which is what it reads as.
      if (s.status === "declined" || (s.status === "skipped" && s.error.code === "TARGET_DISABLED")) {
        return { scope, status: { status: "skipped", reason: { reason: "disabled" } } };
      }
      return { scope, status: { status: "failed", error: s.error } };
    });
  };

  /** Propagation, then (by policy) the re-assert merged into it (Rust `merge_reassert`). */
  const settleTargets = async (skill: FixtureSkill, policy: RefreshPolicyDto) => {
    const targets = propagate(skill);
    if (!policy.reassert_auto_sync) return { targets, reassert_error: null };
    // A store failure inside the re-assert: its targets are unknown, the
    // skill stays refreshed, the error is report data.
    const injected = skill.acquisition.reassertError;
    if (injected) return { targets, reassert_error: injected };
    return { targets: [...targets, ...(await reassert(skill, targets))], reassert_error: null };
  };

  /**
   * The acquisition half of an Update (Rust `skill_update::acquire_update`):
   * typed refusals and fetch failures. A failure here is reported and never
   * reaches the apply phase (no `applying` tick, no Propagation, no re-assert).
   */
  const acquireRefresh = (skill: FixtureSkill): CommandError | null => {
    if (!skill.dto.refreshable) return { code: "NOT_REFRESHABLE", name: skill.dto.name };
    if (skill.dto.unlocatable === "source_missing")
      return { code: "SOURCE_PATH_MISSING", path: skill.dto.source_ref ?? skill.dto.name };
    return skill.acquisition.fail ?? null;
  };

  /** The apply half (Rust `refresh::apply_one_unlocked`): admission, finalize, settlement. */
  const applyRefresh = async (skill: FixtureSkill, policy: RefreshPolicyDto): Promise<SkillRefreshStatus> => {
    const { acquisition } = skill;
    if (acquisition.skip) return { status: "skipped_acquisition", reason: acquisition.skip };
    const editConflict = replayInvocationEdit(skill);
    const restoring = skill.dto.unlocatable === "central_missing";
    if (restoring || acquisition.upstreamChanged) {
      landBytes(skill, acquisition.upstreamChanged ? 2 : 1);
      acquisition.upstreamChanged = false;
    }
    const { targets, reassert_error } = await settleTargets(skill, policy);
    settleUpdateHealth(skill, editConflict);
    return {
      status: "refreshed",
      content_hash: skill.contentHash,
      source_revision: skill.dto.source_type === "git" ? fingerprint(`${skill.dto.name}:${skill.dto.updated_at}`) : null,
      targets,
      reassert_error,
      edit_conflict: editConflict,
    };
  };

  /**
   * Edit replay on Update (Rust `skill_edits::replay_unlocked`). The source's
   * declared mode is `acquisition.upstreamInvocation` (default: unchanged).
   * A conflict is reported when upstream moved to neither the recorded base
   * nor the Edit; the persisted flag survives unchanged Updates until upstream
   * converges on the Edit; the recorded base always becomes the upstream — so
   * the card's `base_mode` and the report's `upstream_mode` agree.
   */
  const replayInvocationEdit = (skill: FixtureSkill): InvocationEditConflict | null => {
    const edit = skill.dto.invocation_override;
    if (!edit) {
      if (skill.acquisition.upstreamInvocation) skill.dto.invocation_mode = skill.acquisition.upstreamInvocation;
      return null;
    }
    const base = edit.base_mode;
    const upstream = skill.acquisition.upstreamInvocation ?? base;
    const disagrees = upstream !== base && upstream !== edit.mode;
    edit.conflict = upstream !== edit.mode && (edit.conflict || disagrees);
    edit.base_mode = upstream;
    return disagrees ? { base_mode: base, upstream_mode: upstream, override_mode: edit.mode } : null;
  };

  const refreshRow = (skill: FixtureSkill, status: SkillRefreshStatus): SkillRefreshOutcome => ({
    skill_id: skill.dto.id,
    skill_name: skill.dto.name,
    status,
  });

  /**
   * Refresh / Update (Rust `refresh::refresh_managed_skills_with`). The cancel
   * token is reset at operation entry; acquisition stops dispatching once a
   * cancel is observed, and a cancelled batch applies nothing: every selected
   * skill is reported `cancelled` (its own kind, never a failure), followed by
   * the pre-settled skips.
   */
  const refreshBatch = async (
    ids: string[] | null,
    policy: RefreshPolicyDto,
    onProgress: ProgressSink<RefreshProgressDto>,
  ): Promise<RefreshReport> => {
    state.cancelRequested = false;
    await pause(120);
    const selected: FixtureSkill[] = [];
    const skipped: SkillRefreshOutcome[] = [];
    if (ids === null) {
      // `All` = every refreshable skill; an Unlocatable one is reported skipped.
      for (const skill of state.skills.filter((s) => s.dto.refreshable)) {
        if (skill.dto.unlocatable) skipped.push(refreshRow(skill, { status: "skipped", state: skill.dto.unlocatable }));
        else selected.push(skill);
      }
    } else {
      // An id with no row is dropped, not a batch failure.
      for (const id of ids) {
        const skill = state.skills.find((s) => s.dto.id === id);
        if (skill) selected.push(skill);
      }
    }
    const total = selected.length;
    const acquired: Array<CommandError | null> = [];
    for (const skill of selected) {
      if (state.cancelRequested) break;
      await step(total * 2);
      acquired.push(acquireRefresh(skill));
      // Ticked on completion: `index` counts finished acquisitions.
      emit(onProgress, { index: acquired.length, total, skill_name: skill.dto.name, phase: "acquiring" });
    }
    if (state.cancelRequested || acquired.length < total) {
      return {
        skills: [...selected.map((skill) => refreshRow(skill, { status: "cancelled" })), ...skipped],
      };
    }
    const outcomes: SkillRefreshOutcome[] = [];
    for (const [i, skill] of selected.entries()) {
      const error = acquired[i];
      if (error) {
        recordUpdateFailure(skill, false, error);
        outcomes.push(refreshRow(skill, { status: "failed", error }));
        continue;
      }
      emit(onProgress, { index: i + 1, total, skill_name: skill.dto.name, phase: "applying" });
      await step(total * 2);
      outcomes.push(refreshRow(skill, await applyRefresh(skill, policy)));
    }
    return { skills: [...outcomes, ...skipped] };
  };

  // -------------------------------------------------------------------------
  // Git / local sources
  // -------------------------------------------------------------------------

  const parseGithub = (input: string) => {
    const match = input
      .trim()
      .replace(/^git\+/, "")
      .match(/^(?:https?:\/\/)?(?:www\.)?github\.com\/([^/\s]+)\/([^/\s#?]+?)(?:\.git)?(?:\/(?:tree|blob)\/([^/]+)\/(.+?))?\/?$/i);
    if (!match) return null;
    return { slug: `${match[1]}/${match[2]}`, branch: match[3] ?? null, subpath: match[4] ?? null };
  };

  const findRepo = (url: string) => {
    const parsed = parseGithub(url) ?? fail({ code: "INVALID_GITHUB_URL", url });
    const repo = state.repos.find((r) => r.slug.toLowerCase() === parsed.slug.toLowerCase());
    if (!repo) {
      return fail({
        code: "GIT_CLONE_FAILED",
        kind: "notFound",
        detail: `remote: Repository not found.\nfatal: repository 'https://github.com/${parsed.slug}/' not found`,
      });
    }
    if (repo.listingError) fail(repo.listingError);
    return { repo, parsed };
  };

  /** Listing's candidate admission (`list_git_skills_with`), shared with the Re-point preview. */
  const listingCandidates = (
    found: readonly { name: string; description: string | null; subpath: string }[],
    branch: string | null,
  ): GitSkillCandidate[] =>
    found.map((c) => ({
      name: c.name,
      description: c.description,
      subpath: c.subpath,
      resolution: { branch: branch ?? "main", subpath: c.subpath },
    }));

  const findLocal = (input: string) => {
    const path = expandHome(input);
    const folder = state.localFolders.find((f) => f.path === path);
    if (folder) return { path, candidates: folder.candidates };
    for (const f of state.localFolders) {
      const candidate = f.candidates.find((c) => `${f.path}/${c.subpath}` === path);
      if (candidate) return { path, candidates: [{ ...candidate, subpath: "." }] };
    }
    return fail({ code: "INVALID_PATH", path, reason: "missing" });
  };

  // -------------------------------------------------------------------------
  // Onboarding
  // -------------------------------------------------------------------------

  const scanScope = () =>
    state.settings.scan_selected_tools_only && state.settings.global_selected_tools
      ? state.settings.global_selected_tools
      : state.installedTools;

  const buildPlan = (): OnboardingPlan => {
    const scope = scanScope();
    const groups = new Map<string, OnboardingGroup>();
    const invalid: OnboardingPlan["invalid"] = [];
    const seenPaths = new Set<string>();
    let found = 0;
    for (const tool of TOOL_REGISTRY.filter((t) => scope.includes(t.key))) {
      const root = globalRoot(tool.key);
      for (const dir of state.foreign.filter((f) => globalRoot(f.tool) === root)) {
        const path = `${root}/${dir.name}`;
        if (seenPaths.has(path)) continue;
        seenPaths.add(path);
        // Rust `scan_tool_dir`'s manifest gate (#46): listed apart, never a group.
        if (dir.invalid) {
          invalid.push({ tool: tool.key, name: dir.name, path, reason: dir.invalid });
          continue;
        }
        found += 1;
        const managed = state.skills.find((s) => s.dto.name === dir.name);
        const group = groups.get(dir.name) ?? {
          name: dir.name,
          variants: [],
          has_conflict: false,
          name_is_managed: managed !== undefined,
          managed_skill_id: managed?.dto.id ?? null,
        };
        group.variants.push({
          tool: tool.key,
          name: dir.name,
          path,
          fingerprint: dir.fingerprint,
          is_link: dir.linkTarget !== null,
          link_target: dir.linkTarget,
        });
        group.has_conflict = new Set(group.variants.map((v) => v.fingerprint)).size > 1;
        groups.set(dir.name, group);
      }
    }
    return {
      total_tools_scanned: scope.length,
      group_count: groups.size,
      copy_count: found,
      groups: [...groups.values()].sort((a, b) => a.name.localeCompare(b.name)),
      invalid,
    };
  };

  /**
   * Admission (Rust `onboarding_import::admit`): resolve the selection against
   * the plan built at operation start. Reads only; a refusal never reaches the
   * apply phase (no `applying` tick).
   */
  const admit = (
    selection: OnboardingSelectionDto,
    plan: OnboardingPlan,
  ):
    | { ok: true; group: OnboardingGroup; replace: string | null }
    | { ok: false; status: ImportGroupStatus } => {
    const refuse = (error: CommandError) => ({ ok: false as const, status: { status: "failed" as const, error } });
    const group = plan.groups.find((g) => g.name === selection.group_name);
    if (!group) return refuse({ code: "NOT_FOUND", kind: "onboarding_group", id: selection.group_name });
    if (!group.variants.some((v) => v.path === selection.chosen_path)) {
      return refuse({ code: "NOT_FOUND", kind: "onboarding_variant", id: selection.chosen_path });
    }
    // A managed name is decided here, never by finalize's collision; a
    // different operator-supplied name is an ordinary import.
    const keepsName = selection.name == null || selection.name === group.name;
    if (group.managed_skill_id !== null && keepsName) {
      if (selection.replace) return { ok: true, group, replace: group.managed_skill_id };
      return {
        ok: false,
        status: { status: "skipped", reason: { reason: "name_managed", skill_id: group.managed_skill_id } },
      };
    }
    return { ok: true, group, replace: null };
  };

  /**
   * Replace (Rust `onboarding_import::replace_with_managed_unlocked`): sync
   * the Managed skill to every Tool of the group, same-content policy, no
   * override — a divergent copy answers an askable TARGET_EXISTS row.
   */
  const replaceGroup = async (group: OnboardingGroup, skillId: string): Promise<ImportGroupStatus> => {
    const skill = state.skills.find((s) => s.dto.id === skillId);
    if (!skill) return { status: "failed", error: { code: "NOT_FOUND", kind: "skill", id: skillId } };
    const tools = [...new Set(group.variants.map((v) => v.tool))];
    const targets = await syncBatch(
      [{ skill_id: skill.dto.id, name: skill.dto.name, source_path: skill.dto.central_path }],
      tools,
      { overwrite: false, overwrite_if_same_content: true },
      null,
    );
    return { status: "replaced", skill_id: skill.dto.id, skill_name: skill.dto.name, targets };
  };

  /**
   * Apply one admitted group (Rust `onboarding_import::apply_one_unlocked`):
   * finalize the chosen variant, then either sync through the global batch
   * (auto-sync on) or settle the originals (auto-sync off).
   *
   * Auto-sync on leaves every original in place: an identical original is
   * taken over by the batch's same-content replacement, so an original whose
   * target fails or is skipped survives. The Tool set is the policy's, else
   * the effective global selection (a saved selection, empty included, is
   * honoured), plus every Tool holding an identical variant — one batch, so
   * the shared-skills-dir dedupe applies.
   */
  const importGroup = async (
    selection: OnboardingSelectionDto,
    group: OnboardingGroup,
    policy: ImportPolicyDto,
  ): Promise<ImportGroupStatus> => {
    const failed = (error: CommandError): ImportGroupStatus => ({ status: "failed", error });
    const chosen = group.variants.find((v) => v.path === selection.chosen_path);
    if (!chosen) return failed({ code: "NOT_FOUND", kind: "onboarding_variant", id: selection.chosen_path });
    const name = selection.name ?? group.name;
    if (nameTaken(name)) return failed({ code: "SKILL_EXISTS", name });
    const skill = finalize({
      name,
      description: `${name.replace(/-/g, " ")}, found in ${toolLabelOf(chosen.tool)}`,
      sourceType: "imported",
      sourceRef: null,
      importedFrom: chosen.tool,
      hashOverride: chosen.fingerprint ?? undefined,
    });
    const identical = (v: (typeof group.variants)[number]) => v.fingerprint === skill.contentHash;
    const originals: OriginalOutcome[] = [];
    let targets: BatchTargetOutcome[] = [];
    let forced: string[] = [];
    if (policy.auto_sync ?? false) {
      const identicalTools: string[] = [];
      for (const v of group.variants) {
        if (identical(v)) {
          if (!identicalTools.includes(v.tool)) identicalTools.push(v.tool);
        } else {
          originals.push({ path: v.path, tool: v.tool, status: { status: "kept_divergent" } });
        }
      }
      const requested = policy.tools ?? effectiveTargets();
      forced = identicalTools.filter((tool) => !requested.includes(tool));
      targets = await syncBatch(
        [{ skill_id: skill.dto.id, name: skill.dto.name, source_path: skill.dto.central_path }],
        [...requested, ...forced],
        { overwrite: false, overwrite_if_same_content: true },
        null,
      );
      // Rust: a divergent sibling sitting where a target lands is that
      // target's `TARGET_EXISTS` row — the askable one (D12). Reported once,
      // there: it is not also a kept original.
      const collided = new Set(
        targets.flatMap((o) =>
          o.status.status === "failed" && o.status.error.code === "TARGET_EXISTS" ? [o.status.error.path] : [],
        ),
      );
      const kept = originals.filter((o) => !collided.has(o.path));
      originals.splice(0, originals.length, ...kept);
    } else if (!policy.remove_originals) {
      // Rust `leave_original`: nothing touched; every original still present
      // is reported per Tool.
      for (const v of group.variants) {
        if (!state.foreign.some((f) => `${globalRoot(f.tool)}/${f.name}` === v.path)) continue;
        originals.push({ path: v.path, tool: v.tool, status: { status: identical(v) ? "left_in_place" : "kept_divergent" } });
      }
    } else {
      // Rust `settle_original`, per variant in plan order.
      for (const v of group.variants) {
        const gone = !state.foreign.some((f) => `${globalRoot(f.tool)}/${f.name}` === v.path);
        if (gone) {
          originals.push({ path: v.path, tool: v.tool, status: { status: "removed" } });
        } else if (!identical(v)) {
          originals.push({ path: v.path, tool: v.tool, status: { status: "kept_divergent" } });
        } else if (state.unwritableTools.includes(v.tool)) {
          originals.push({ path: v.path, tool: v.tool, status: { status: "failed", error: PERMISSION_DENIED(v.path) } });
        } else {
          state.foreign = state.foreign.filter((f) => `${globalRoot(f.tool)}/${f.name}` !== v.path);
          originals.push({ path: v.path, tool: v.tool, status: { status: "removed" } });
        }
      }
    }
    // Finalised, then an original could not be settled: `import_failed`.
    const failedOriginal = originals.find((o) => o.status.status === "failed");
    if (failedOriginal && failedOriginal.status.status === "failed") {
      recordHealth(skill, "import_failed", failedOriginal.status.error);
    }
    return { status: "imported", skill_id: skill.dto.id, skill_name: name, targets, forced_tools: forced, originals };
  };

  // -------------------------------------------------------------------------
  // Settings / tools
  // -------------------------------------------------------------------------

  const clamp = (value: number, range: { min: number; max: number }) =>
    Math.min(range.max, Math.max(range.min, value));

  const applySetting = (update: SettingUpdate): AppSettings => {
    const s = state.settings;
    switch (update.key) {
      case "central_repo_path": {
        const next = expandHome(update.value);
        if (!next.startsWith("/")) fail({ code: "INVALID_PATH", path: update.value, reason: "missing" });
        for (const skill of state.skills) {
          skill.dto.central_path = skill.dto.central_path.replace(s.central_repo_path, next);
        }
        s.central_repo_path = next;
        break;
      }
      case "git_cache_cleanup_days":
        s.git_cache_cleanup_days = clamp(Math.round(update.value), s.bounds.git_cache_cleanup_days);
        break;
      case "git_cache_ttl_secs":
        s.git_cache_ttl_secs = clamp(Math.round(update.value), s.bounds.git_cache_ttl_secs);
        break;
      case "github_token":
        s.github_token_set = update.value.trim() !== "";
        break;
      case "auto_sync_enabled":
        s.auto_sync_enabled = update.value;
        break;
      case "global_tool_config":
        s.global_selected_tools = update.value.selected_tools.filter((k) => toolByKey(k));
        s.scan_selected_tools_only = update.value.scan_selected_only;
        s.global_selected_tools_corrupt = false;
        break;
      case "ui_zoom_level":
        s.ui_zoom_level = Number.isFinite(update.value) ? clamp(update.value, s.bounds.ui_zoom_level) : 1;
        break;
    }
    return clone(s);
  };

  const toolStatus = (): ToolStatusDto => {
    const installed = TOOL_REGISTRY.filter((t) => isInstalled(t.key)).map((t) => t.key);
    const newly = installed.filter((k) => !state.seenTools.includes(k));
    state.seenTools = [...installed];
    return {
      tools: TOOL_REGISTRY.map((t) => ({ key: t.key, label: t.label, installed: isInstalled(t.key), shared_with: globalSharers(t.key), constituents: [] })),
      installed,
      newly_installed: newly,
    };
  };

  const projectToolStatus = (): ToolStatusDto => {
    const catalog = projectCatalog();
    const tools = catalog.map((t) => ({
      key: t.key,
      label: t.groupLabel ?? t.label,
      installed: isProjectToolInstalled(state, t.key),
      shared_with: catalog.filter((o) => o.projectDir === t.projectDir).map((o) => o.key),
      constituents: t.key === VIRTUAL_GROUP_KEY ? TOOL_REGISTRY.filter((o) => o.grouped).map((o) => o.label) : [],
    }));
    return { tools, installed: tools.filter((t) => t.installed).map((t) => t.key), newly_installed: [] };
  };

  // -------------------------------------------------------------------------
  // The handler table
  // -------------------------------------------------------------------------

  const skillResult = (report: RefreshReport): SkillMutationResultDto => ({ report, skills: skillsDto() });

  /**
   * The Add batch (Rust `install_batch::install_selection_with`): per
   * candidate, acquire (a latency step, cancellable between candidates and
   * after an acquisition — a cancelled candidate finalizes nothing), then
   * finalize + first sync through the global batch with the same-content
   * policy. Collisions are collected as `TARGET_EXISTS` rows, never asked.
   */
  const installBatch = async (
    request: InstallRequest,
    onProgress: ProgressSink<InstallProgressDto>,
  ): Promise<InstallBatchReport> => {
    state.cancelRequested = false;
    const total = request.candidates.length;
    const candidates: InstallCandidateOutcome[] = [];
    const labelOf = (subpath: string, name: string | null) =>
      name || (subpath === "." ? null : subpath.split("/").pop()) || subpath;
    for (const [i, candidate] of request.candidates.entries()) {
      const label = labelOf(candidate.subpath, candidate.name_override);
      const settle = (status: InstallCandidateOutcome["status"], name = label) =>
        candidates.push({ subpath: candidate.subpath, name, status });
      if (state.cancelRequested) {
        settle({ status: "cancelled" });
        continue;
      }
      emit(onProgress, { index: i + 1, total, name: label });
      await pause(request.source.kind === "git" ? 650 : 400);
      try {
        const staged =
          request.source.kind === "git"
            ? (() => {
                const { repo } = findRepo(request.source.url);
                const found =
                  repo.candidates.find((c) => c.subpath === candidate.subpath) ??
                  fail({ code: "SUBPATH_MISSING", subpath: candidate.subpath });
                return {
                  name: candidate.name_override ?? found.name,
                  description: found.description,
                  sourceType: "git" as const,
                  sourceRef: request.source.url.trim(),
                  invocation: found.invocation,
                };
              })()
            : (() => {
                const local = findLocal(request.source.root);
                const found =
                  local.candidates.find((c) => c.subpath === candidate.subpath) ??
                  fail({ code: "SUBPATH_MISSING", subpath: candidate.subpath });
                if (!found.valid) fail({ code: "SKILL_INVALID", reason: found.reason ?? "missing_skill_md" });
                const tool = insideToolDir(local.path);
                if (tool) fail({ code: "LOCAL_SOURCE_INSIDE_TOOL_DIR", path: local.path, tool: tool.key });
                return {
                  name: candidate.name_override ?? found.name,
                  description: found.description ?? found.name,
                  sourceType: "local" as const,
                  sourceRef: candidate.subpath === "." ? local.path : `${local.path}/${candidate.subpath}`,
                };
              })();
        // Cancelled while acquiring: the staged bytes are discarded.
        if (state.cancelRequested) {
          settle({ status: "cancelled" });
          continue;
        }
        const skill = finalize(staged);
        const targets =
          request.deploy && request.tools.length > 0
            ? await syncBatch(
                [{ skill_id: skill.dto.id, name: skill.dto.name, source_path: skill.dto.central_path }],
                request.tools,
                {
                  overwrite: false,
                  overwrite_if_same_content: request.overwrite_if_same_content,
                  overrides: request.overrides
                    .filter((o) => o.skill_name === skill.dto.name)
                    .map((o) => ({ skill_id: skill.dto.id, tool: o.tool, decision: o.decision })),
                },
                null,
              )
            : [];
        settle({ status: "installed", skill: installResult(skill), targets }, skill.dto.name);
      } catch (error) {
        settle({ status: "failed", error: error as CommandError });
      }
    }
    return { candidates };
  };

  /**
   * The overwrite ask's project retry (Rust `project_sync::resolve_collisions`):
   * Overwrite re-syncs over the folder; Keep keeps the row `disabled`, owning
   * nothing, reported `declined`.
   */
  const resolveCollision = (decision: AssignmentDecision): ProjectSyncOutcome => {
    const project = findProject(decision.project_id);
    const a = project.assignments.find((x) => x.skill_id === decision.skill_id && x.tool === decision.tool);
    if (!a) {
      return {
        assignment_id: null,
        project_id: decision.project_id,
        skill_id: decision.skill_id,
        skill_name: "",
        tool: decision.tool,
        status: { status: "failed", error: { code: "NOT_FOUND", kind: "assignment", id: `${decision.project_id}/${decision.skill_id}/${decision.tool}` } },
      };
    }
    if (isDisabledAssignment(a)) return outcomeOf(project, a, skippedDisabled);
    if (a.synced_at !== null) return outcomeOf(project, a, { status: "already_assigned" });
    if (decision.decision === "keep") {
      a.status = "disabled";
      a.last_error = null;
      return outcomeOf(project, a, { status: "declined" });
    }
    return outcomeOf(project, a, syncAssignment(project, a, true));
  };

  const handlers = {
    getSettings: async () => {
      await pause(40);
      return clone(state.settings);
    },
    updateSetting: async (update) => {
      await pause(60);
      return applySetting(update);
    },
    getToolStatus: async () => {
      await pause(80);
      return toolStatus();
    },
    getProjectToolStatus: async () => {
      await pause(60);
      return projectToolStatus();
    },
    clearGitCacheNow: async () => {
      await pause(300);
      const removed = state.gitCacheEntries;
      state.gitCacheEntries = 0;
      return removed;
    },
    getOnboardingPlan: async () => {
      await pause(350);
      return buildPlan();
    },
    listLocalSkillsCmd: async (basePath) => {
      await pause(250);
      return clone(findLocal(basePath).candidates);
    },
    installLocalSelection: async (basePath, subpath, name) => {
      await pause(400);
      const local = findLocal(basePath);
      const candidate =
        local.candidates.find((c) => c.subpath === subpath) ??
        fail({ code: "SUBPATH_MISSING", subpath });
      if (!candidate.valid) fail({ code: "SKILL_INVALID", reason: candidate.reason ?? "missing_skill_md" });
      const tool = insideToolDir(local.path);
      if (tool) fail({ code: "LOCAL_SOURCE_INSIDE_TOOL_DIR", path: local.path, tool: tool.key });
      const skill = finalize({
        name: name ?? candidate.name,
        description: candidate.description ?? candidate.name,
        sourceType: "local",
        sourceRef: subpath === "." ? local.path : `${local.path}/${subpath}`,
      });
      return installResult(skill);
    },
    listGitSkillsCmd: async (repoUrl, targetName) => {
      await pause(700);
      const { repo, parsed } = findRepo(repoUrl);
      const inScope = parsed.subpath
        ? repo.candidates.filter((c) => c.subpath === parsed.subpath || c.subpath.startsWith(`${parsed.subpath}/`))
        : repo.candidates;
      if (parsed.subpath && inScope.length === 0) fail({ code: "SUBPATH_MISSING", subpath: parsed.subpath });
      const candidates = listingCandidates(inScope, parsed.branch);
      let target_match: CandidateMatch | null = null;
      if (targetName !== null) {
        const hits = candidates.filter((c) => c.name.toLowerCase() === targetName.toLowerCase());
        target_match =
          hits.length === 1
            ? { kind: "resolved", subpath: hits[0].subpath }
            : hits.length > 1
              ? { kind: "ambiguous", subpaths: hits.map((h) => h.subpath) }
              : { kind: "none" };
      }
      return { candidates, target_match };
    },
    installGitSelection: async (repoUrl, subpath, name) => {
      // The command resets the cancel token at entry; a cancel observed by
      // the acquisition is the typed whole-command refusal.
      state.cancelRequested = false;
      await pause(650);
      if (state.cancelRequested) fail({ code: "CANCELLED" });
      const { repo } = findRepo(repoUrl);
      const candidate =
        repo.candidates.find((c) => c.subpath === subpath) ??
        fail({ code: "SUBPATH_MISSING", subpath });
      // The operator's name wins at finalize (and is what the collision
      // check sees), as in the local install.
      const skill = finalize({
        name: name ?? candidate.name,
        description: candidate.description,
        sourceType: "git",
        sourceRef: repoUrl.trim(),
        invocation: candidate.invocation,
      });
      return installResult(skill);
    },
    syncSkillsToTools: async (skills, tools, policy, onProgress) => {
      await pause(120);
      return syncBatch(skills, tools, policy, onProgress);
    },
    installSelection: async (request, onProgress) => installBatch(request, onProgress),
    refreshManagedSkills: async (skillIds, policy, onProgress) =>
      refreshBatch(skillIds, policy, onProgress),
    updateManagedSkill: async (skillId, policy, onProgress) =>
      skillResult(await refreshBatch([skillId], policy, onProgress)),
    repointSkillSource: async (skillId, target: RepointTarget, policy) => {
      // Rust `repoint::repoint_skill_source_with`: validate (thrown), then a
      // batch-of-one Update through `refresh_managed_skills_with` — the same
      // cancellation contract and report shape as Update.
      state.cancelRequested = false;
      const skill = findSkill(skillId);
      // A failure after validation is the batch's report row, and settles as
      // the skill's `repoint_failed` health (a cancellation records nothing).
      const refused = (error: CommandError): SkillMutationResultDto => {
        recordUpdateFailure(skill, true, error);
        return skillResult({ skills: [refreshRow(skill, { status: "failed", error })] });
      };
      const parsed =
        target.kind === "git" ? (parseGithub(target.url) ?? fail({ code: "INVALID_GITHUB_URL", url: target.url })) : null;
      if (target.kind === "local") {
        const path = expandHome(target.path);
        const tool = insideToolDir(path);
        if (tool) fail({ code: "LOCAL_SOURCE_INSIDE_TOOL_DIR", path, tool: tool.key });
        findLocal(path);
      }
      await pause(600);
      // A cancel is the batch's cancelled row, never a failure, no health.
      if (state.cancelRequested) return skillResult({ skills: [refreshRow(skill, { status: "cancelled" })] });
      if (target.kind === "git" && parsed) {
        const repo = state.repos.find((r) => r.slug.toLowerCase() === parsed.slug.toLowerCase());
        if (!repo) return refused({ code: "GIT_CLONE_FAILED", kind: "notFound", detail: `remote: Repository not found (${parsed.slug})` });
        if (repo.listingError) return refused(repo.listingError);
        // A candidate the operator picked (Rust `RepointSelection`) is that folder.
        const chosen = target.kind === "git" ? (target.selection?.subpath ?? null) : null;
        const pick = chosen
          ? repo.candidates.find((c) => c.subpath === chosen)
          : parsed.subpath
          ? repo.candidates.find((c) => c.subpath === parsed.subpath)
          : repo.candidates.length === 1
            ? repo.candidates[0]
            : repo.candidates.find((c) => c.name === skill.dto.name);
        if (!pick) return refused(parsed.subpath ? { code: "SUBPATH_MISSING", subpath: parsed.subpath } : { code: "MULTI_SKILLS" });
        skill.dto.source_type = "git";
        skill.dto.source_ref = target.url.trim();
      } else if (target.kind === "local") {
        skill.dto.source_type = "local";
        skill.dto.source_ref = expandHome(target.path);
      }
      skill.dto.imported_from_tool = null;
      skill.dto.refreshable = true;
      skill.acquisition = {};
      landBytes(skill, 1);
      const { targets, reassert_error } = await settleTargets(skill, policy);
      settleUpdateHealth(skill, null);
      return skillResult({
        skills: [
          refreshRow(skill, {
            status: "refreshed",
            content_hash: skill.contentHash,
            source_revision: target.kind === "git" ? fingerprint(skill.dto.source_ref ?? "") : null,
            targets,
            reassert_error,
            edit_conflict: null,
          }),
        ],
      });
    },
    previewRepointSource: async (skillId, target: RepointTarget) => {
      // Rust `repoint::preview_repoint_source_with`: the Re-point's validation
      // and acquisition intent (the skill's own name), writing nothing. A
      // refusal throws; a multi-skill repository answers its candidates.
      state.cancelRequested = false;
      const skill = findSkill(skillId);
      const sameName = (name: string) => name.trim().toLowerCase() === skill.dto.name.trim().toLowerCase();
      if (target.kind === "local") {
        const path = expandHome(target.path);
        const folder = state.localFolders.find((f) => f.path === path);
        const own = folder
          ? folder.candidates.find((c) => c.subpath === ".")
          : state.localFolders.flatMap((f) => f.candidates.filter((c) => `${f.path}/${c.subpath}` === path))[0];
        if (!folder && !own) fail({ code: "SOURCE_PATH_MISSING", path });
        if (!own || own.reason === "missing_skill_md") fail({ code: "SKILL_INVALID", reason: "missing_skill_md" });
        const tool = insideToolDir(path);
        if (tool) fail({ code: "LOCAL_SOURCE_INSIDE_TOOL_DIR", path, tool: tool.key });
        const name = own!.name;
        return { resolved_name: name, ref: null, subpath: null, name_matches: sameName(name), candidates: null };
      }
      const parsed = parseGithub(target.url) ?? fail({ code: "INVALID_GITHUB_URL", url: target.url });
      await pause(500);
      if (state.cancelRequested) fail({ code: "CANCELLED" });
      const { repo } = findRepo(target.url);
      if (target.selection) {
        // A picked candidate (Rust `RepointSelection`): acquired as that
        // folder, on the listing's branch — the pick is already the choice.
        const chosen = target.selection.subpath;
        const pick =
          repo.candidates.find((c) => c.subpath === chosen) ?? fail({ code: "SUBPATH_MISSING", subpath: chosen });
        const branch = target.selection.resolution?.branch ?? parsed.branch;
        return { resolved_name: pick.name, ref: branch, subpath: pick.subpath, name_matches: sameName(pick.name), candidates: null };
      }
      if (parsed.subpath) {
        // The fast path: a named folder, on the named (or assumed) branch.
        const branch = parsed.branch ?? "main";
        const pick =
          repo.candidates.find((c) => c.subpath === parsed.subpath) ??
          fail({ code: "GITHUB_SKILL_NOT_FOUND", url: `https://github.com/${repo.slug}/tree/${branch}/${parsed.subpath}` });
        return { resolved_name: pick.name, ref: branch, subpath: pick.subpath, name_matches: sameName(pick.name), candidates: null };
      }
      // A repository URL: the clone follows the default branch unnamed.
      const candidates = listingCandidates(repo.candidates, parsed.branch);
      const pick = repo.candidates.length === 1 ? repo.candidates[0] : repo.candidates.find((c) => sameName(c.name));
      return {
        resolved_name: pick?.name ?? null,
        ref: parsed.branch,
        subpath: pick?.subpath ?? null,
        name_matches: pick ? sameName(pick.name) : false,
        candidates: candidates.length > 1 ? candidates : null,
      };
    },
    detachSkillFromSource: async (skillId) => {
      await pause(150);
      const skill = findSkill(skillId);
      if (!skill.dto.detachable) {
        fail({ code: "OTHER", message: "only a local skill whose central copy exists can be detached" });
      }
      Object.assign(skill.dto, { source_type: "imported", source_ref: null, imported_from_tool: null, refreshable: false, unlocatable: null, detachable: false });
      return null;
    },
    // Rust `skill_health::dismiss_outcome`: only `import_failed` is
    // dismissable; a newer outcome of another kind is left alone.
    clearSkillHealth: async (skillId, kind) => {
      await pause(60);
      if (kind !== "import_failed") fail({ code: "OTHER", message: `skill health ${kind} is not dismissable` });
      clearHealth(findSkill(skillId), ["import_failed"]);
      return null;
    },
    importOnboardingSelection: async (selections, policy, onProgress: ProgressSink<ImportProgressDto>) => {
      await pause(150);
      const plan = buildPlan();
      const groups: ImportGroupOutcome[] = [];
      const total = selections.length;
      for (const [i, selection] of selections.entries()) {
        emit(onProgress, { index: i + 1, total, group_name: selection.group_name, phase: "admitting" });
        await step(total * 2);
        const admitted = admit(selection, plan);
        let status: ImportGroupStatus;
        if (admitted.ok) {
          emit(onProgress, { index: i + 1, total, group_name: selection.group_name, phase: "applying" });
          await step(total * 2);
          status =
            admitted.replace !== null
              ? await replaceGroup(admitted.group, admitted.replace)
              : await importGroup(selection, admitted.group, policy);
        } else {
          status = admitted.status;
        }
        groups.push({ group_name: selection.group_name, status });
      }
      return { groups };
    },
    getManagedSkills: async () => {
      await pause(90);
      return skillsDto();
    },
    setSkillInvocationOverride: async (skillId, mode) => {
      await pause(250);
      const skill = findSkill(skillId);
      if (skill.files.length === 0) fail({ code: "CENTRAL_PATH_MISSING", path: skill.dto.central_path });
      const base = skill.dto.invocation_override?.base_mode ?? skill.dto.invocation_mode;
      skill.dto.invocation_override = mode === null ? null : { mode, base_mode: base, conflict: false };
      skill.dto.invocation_mode = mode ?? base;
      skill.files = skillFiles(skill.dto.name, skill.dto.description ?? skill.dto.name, skill.dto.invocation_mode);
      skill.contentHash = hashFiles(skill.files);
      skill.dto.updated_at = now();
      const targets = propagate(skill);
      // A saved (or cleared) Edit is the Edit family's success.
      clearHealth(skill, EDIT_FAMILY);
      return {
        report: { skill_id: skill.dto.id, skill_name: skill.dto.name, propagation: { targets } },
        skills: skillsDto(),
      };
    },
    deleteManagedSkill: async (skillId) => {
      await pause(300);
      const skill = findSkill(skillId);
      const parts = [removeGlobalRows(skill, skill.dto.targets.map((t) => t.tool))];
      for (const project of state.projects) {
        parts.push(removeAssignments(project, (a) => a.skill_id !== skillId));
      }
      const failure = parts
        .flatMap((part) => part.targets)
        .find((target) => target.status.status === "failed");
      if (failure && failure.status.status === "failed") {
        // A kept Delete settles as the skill's `delete_incomplete` health.
        recordHealth(skill, "delete_incomplete", failure.status.error);
        return removal(parts);
      }
      state.skills = state.skills.filter((s) => s !== skill);
      return removal(parts, { central_removed: true, record_deleted: true });
    },
    disableSkillTargets: async (scope: DisableScope) => {
      await pause(200);
      if (scope.kind === "skill_tool") return removal(disablePair(findSkill(scope.skill_id), scope.tool));
      const skills =
        scope.kind === "skill"
          ? [findSkill(scope.skill_id)]
          : state.skills.filter((s) => scope.skill_ids.includes(s.dto.id));
      return removal(skills.flatMap(disableSkill));
    },
    enableSkillTargets: async (scope: DisableScope, overrides) => {
      await pause(200);
      if (scope.kind === "skill_tool") return enablePair(findSkill(scope.skill_id), scope.tool, overrides);
      const skills =
        scope.kind === "skill"
          ? [findSkill(scope.skill_id)]
          : state.skills.filter((s) => scope.skill_ids.includes(s.dto.id));
      const report: EnableReport = { global: [], projects: { items: [] } };
      for (const skill of skills) {
        const one = await enableSkill(skill, overrides);
        report.global.push(...one.global);
        report.projects.items.push(...one.projects.items);
      }
      return report;
    },
    getFeaturedSkills: async () => {
      await pause(300);
      return clone(state.featured);
    },
    searchSkillsOnline: async (query, limit) => {
      await pause(450);
      const q = query.trim().toLowerCase();
      if (!q) return [];
      return clone(state.online.filter((s) => s.name.includes(q) || s.source.includes(q)).slice(0, limit ?? 20));
    },
    listSkillFiles: async (centralPath) => {
      await pause(80);
      const files =
        state.skills.find((s) => s.dto.central_path === centralPath)?.files ?? previews.get(centralPath) ?? [];
      if (files.length === 0) fail({ code: "CENTRAL_PATH_MISSING", path: centralPath });
      return files.map((f) => ({ path: f.path, size: new TextEncoder().encode(f.content).length }));
    },
    readSkillFile: async (centralPath, filePath) => {
      await pause(50);
      const files = state.skills.find((s) => s.dto.central_path === centralPath)?.files ?? previews.get(centralPath) ?? [];
      const file = files.find((f) => f.path === filePath);
      return file ? file.content : fail({ code: "NOT_FOUND", kind: "file", id: filePath });
    },
    cloneExploreSkill: async (sourceUrl, skillName) => {
      state.cancelRequested = false;
      await pause(800);
      if (state.cancelRequested) fail({ code: "CANCELLED" });
      const { repo, parsed } = findRepo(sourceUrl);
      const candidate =
        repo.candidates.find((c) => (parsed.subpath ? c.subpath === parsed.subpath : c.name === skillName)) ??
        repo.candidates[0] ??
        fail({ code: "SUBPATH_MISSING", subpath: parsed.subpath ?? skillName ?? "." });
      const path = `${HOME}/Library/Caches/com.skillshub.app/explore/${repo.slug.replace("/", "__")}/${candidate.name}`;
      previews.set(path, skillFiles(candidate.name, candidate.description, candidate.invocation ?? "user-and-model"));
      return path;
    },
    hideExploreSkill: async (sourceUrl) => {
      if (!state.hiddenExplore.includes(sourceUrl)) state.hiddenExplore.push(sourceUrl);
      return null;
    },
    unhideExploreSkill: async (sourceUrl) => {
      state.hiddenExplore = state.hiddenExplore.filter((u) => u !== sourceUrl);
      return null;
    },
    getHiddenExploreSkills: async () => [...state.hiddenExplore],
    cancelCurrentOperation: async () => {
      state.cancelRequested = true;
      return null;
    },
    openLogFolder: async () => {
      console.info("[fixture] open_log_folder: would reveal ~/Library/Logs/com.skillshub.app/skills-hub.log");
      return null;
    },
    registerProject: async (path) => {
      await pause(200);
      const resolved = expandHome(path);
      if (!resolved.startsWith("/")) fail({ code: "INVALID_PATH", path, reason: "missing" });
      if (state.projects.some((p) => p.path === resolved)) fail({ code: "DUPLICATE_PROJECT", path: resolved });
      const at = now();
      const project: FixtureProject = {
        id: newId("prj"),
        name: basename(resolved),
        path: resolved,
        created_at: at,
        updated_at: at,
        path_exists: true,
        tools: [],
        assignments: [],
        gitignore: { in_gitignore: false, in_exclude: false },
      };
      state.projects.push(project);
      return viewOf(project);
    },
    removeProject: async (projectId) => {
      await pause(250);
      const project = findProject(projectId);
      const removed = removeAssignments(project, () => false);
      const report = removal(removed);
      if (!failedIn(removed.targets)) {
        state.projects = state.projects.filter((p) => p !== project);
      }
      return { projects: projectsDto(), report };
    },
    listProjects: async () => {
      await pause(80);
      return projectsDto();
    },
    updateProjectPath: async (projectId, path) => {
      await pause(150);
      const project = findProject(projectId);
      const resolved = expandHome(path);
      if (state.projects.some((p) => p !== project && p.path === resolved)) fail({ code: "DUPLICATE_PROJECT", path: resolved });
      Object.assign(project, { path: resolved, name: basename(resolved), path_exists: true, updated_at: now() });
      return viewOf(project);
    },
    configureProjectTools: async (projectId, tools, gitignore) => {
      await pause(250);
      const project = findProject(projectId);
      for (const tool of tools) {
        if (!projectCatalog().some((t) => t.key === tool)) fail({ code: "UNKNOWN_TOOL", tool });
      }
      const dropped = project.tools.filter((t) => !tools.includes(t));
      const report = removal(removeAssignments(project, (a) => !dropped.includes(a.tool)));
      const stuck = new Set(project.assignments.filter((a) => dropped.includes(a.tool)).map((a) => a.tool));
      project.tools = [...tools, ...dropped.filter((t) => stuck.has(t))];
      if (gitignore) project.gitignore = { in_gitignore: gitignore.add_to_gitignore, in_exclude: gitignore.add_to_exclude };
      project.updated_at = now();
      return { view: viewOf(project), report };
    },
    getProjectView: async (projectId) => {
      await pause(90);
      return viewOf(findProject(projectId));
    },
    toggleProjectSkillAssignment: async (projectId, skillId, tool) => {
      await pause(180);
      const project = findProject(projectId);
      const skill = findSkill(skillId);
      if (project.assignments.some((a) => a.skill_id === skillId && a.tool === tool)) {
        const report = removal(removeAssignments(project, (a) => !(a.skill_id === skillId && a.tool === tool)));
        return { kind: "unassigned", view: viewOf(project), report };
      }
      if (!project.tools.includes(tool)) fail({ code: "UNKNOWN_TOOL", tool });
      const item = assign(project, skill, tool);
      return { kind: "assigned", view: viewOf(project), report: { items: [item] } };
    },
    resyncProject: async (projectId) => {
      await pause(400);
      const project = findProject(projectId);
      const items = project.assignments.map((a) => outcomeOf(project, a, resyncOne(project, a)));
      return { view: viewOf(project), report: { items } };
    },
    resolveProjectCollisions: async (decisions) => {
      await pause(200);
      return { items: decisions.map(resolveCollision) };
    },
    resyncAllProjects: async () => {
      await pause(700);
      const items = state.projects.flatMap((project) =>
        project.assignments.map((a) => outcomeOf(project, a, resyncOne(project, a))),
      );
      return { report: { items }, projects: projectsDto() };
    },
    bulkAssignSkill: async (projectId, skillId) => {
      await pause(300);
      const project = findProject(projectId);
      const skill = findSkill(skillId);
      const items = project.tools.map((tool) => {
        const existing = project.assignments.find((a) => a.skill_id === skillId && a.tool === tool);
        if (!existing) return assign(project, skill, tool);
        return outcomeOf(project, existing, isDisabledAssignment(existing) ? skippedDisabled : { status: "already_assigned" });
      });
      return { view: viewOf(project), report: { items } };
    },
    bulkUnassignSkill: async (projectId, skillId) => {
      await pause(250);
      const project = findProject(projectId);
      const report = removal(removeAssignments(project, (a) => a.skill_id !== skillId));
      return { view: viewOf(project), report };
    },
    updateProjectGitignore: async (projectId, gitignore) => {
      await pause(120);
      const project = findProject(projectId);
      project.gitignore = { in_gitignore: gitignore.add_to_gitignore, in_exclude: gitignore.add_to_exclude };
      return null;
    },
    getProjectGitignoreStatus: async (projectId) => {
      await pause(60);
      return clone(findProject(projectId).gitignore);
    },
  } satisfies FixtureHandlers;

  /** The fixture counterpart of `invokeTauri`: same name, same positional args. */
  const invoke = <K extends CommandName>(
    command: K,
    ...args: Parameters<Commands[K]>
  ): Promise<Awaited<ReturnType<Commands[K]>>> => {
    const handler = (handlers as FixtureHandlers)[command] as (
      ...handlerArgs: Parameters<Commands[K]>
    ) => Promise<Awaited<ReturnType<Commands[K]>>>;
    return handler(...args);
  };

  return { state, handlers: handlers as FixtureHandlers, invoke };
}

export type FixtureBackend = ReturnType<typeof createFixtureBackend>;
