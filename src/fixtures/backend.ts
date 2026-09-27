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
  GitSkillCandidate,
  ImportGroupOutcome,
  ImportPolicyDto,
  ImportProgressDto,
  InstallResultDto,
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
  RefreshPolicyDto,
  RefreshProgressDto,
  RefreshReport,
  RemovalReport,
  RemovalTargetOutcome,
  RepointTarget,
  SettingUpdate,
  SkillMutationResultDto,
  SkillRefreshOutcome,
  SkillRefreshStatus,
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
import { BROKEN_IMPORT_NAMES } from "./scenarios";

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
        refreshable: input.sourceType !== "imported",
        unlocatable: null,
        detachable: input.sourceType === "local",
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

  // -------------------------------------------------------------------------
  // Global sync
  // -------------------------------------------------------------------------

  /** Record (or refresh) the target rows of every detected Tool sharing the root. */
  const recordTargets = (skill: FixtureSkill, tool: string) => {
    const path = `${globalRoot(tool)}/${skill.dto.name}`;
    const at = now();
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
      return { status: "skipped", error: { code: "TOOL_NOT_WRITABLE", tool, path: root } };
    }
    if (skill.files.length === 0) {
      return { status: "failed", error: { code: "CENTRAL_PATH_MISSING", path: skill.dto.central_path } };
    }
    const occupantIndex = state.foreign.findIndex(
      (f) => globalRoot(f.tool) === root && f.name === skill.dto.name,
    );
    let replaced = false;
    if (occupantIndex >= 0) {
      const same = state.foreign[occupantIndex].fingerprint === skill.contentHash;
      if (!(overwrite || (same && overwriteIfSameContent))) {
        return { status: "failed", error: { code: "TARGET_EXISTS", path } };
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
        const overwrite =
          (policy.overwrite ?? false) ||
          (policy.overrides ?? []).some(
            (o) => o.overwrite && o.skill_id === s.skill_id && globalSharers(key).includes(o.tool),
          );
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

  /** Effective global target set: the recorded selection, else every detected Tool. */
  const effectiveTargets = () => state.settings.global_selected_tools ?? state.installedTools;

  // -------------------------------------------------------------------------
  // Artifact removal
  // -------------------------------------------------------------------------

  const removeGlobalRows = (skill: FixtureSkill, tools: string[]): RemovalTargetOutcome[] => {
    const byPath = new Map<string, string[]>();
    for (const row of skill.dto.targets.filter((t) => tools.includes(t.tool))) {
      byPath.set(row.target_path, [...(byPath.get(row.target_path) ?? []), row.tool]);
    }
    const out: RemovalTargetOutcome[] = [];
    for (const [path, rowTools] of byPath) {
      const rows = rowTools.map((tool) => ({ scope: "global_target" as const, id: `tgt-${skill.dto.id}-${tool}`, skill_id: skill.dto.id, tool }));
      if (state.lockedPaths.includes(path)) {
        for (const row of skill.dto.targets) if (rowTools.includes(row.tool)) row.status = "error";
        out.push({ path, rows, status: { status: "failed", error: PERMISSION_DENIED(path) } });
      } else {
        skill.dto.targets = skill.dto.targets.filter((t) => !rowTools.includes(t.tool));
        out.push({ path, rows, status: { status: "removed" } });
      }
    }
    return out;
  };

  const assignmentPath = (project: FixtureProject, a: ProjectSkillAssignmentDto) =>
    `${projectToolDir(project, a.tool)}/${a.skill_name}`;

  const removeAssignments = (
    project: FixtureProject,
    keep: (a: ProjectSkillAssignmentDto) => boolean,
  ): RemovalTargetOutcome[] => {
    const out: RemovalTargetOutcome[] = [];
    const remaining: ProjectSkillAssignmentDto[] = [];
    for (const a of project.assignments) {
      if (keep(a)) {
        remaining.push(a);
        continue;
      }
      const path = assignmentPath(project, a);
      const rows = [{ scope: "assignment" as const, id: a.id, project_id: project.id, skill_id: a.skill_id, tool: a.tool }];
      if (project.path_exists && state.lockedPaths.includes(path)) {
        a.status = "error";
        a.last_error = permissionDeniedText(path);
        remaining.push(a);
        out.push({ path, rows, status: { status: "failed", error: PERMISSION_DENIED(path) } });
      } else {
        out.push({ path, rows, status: { status: "removed" } });
      }
    }
    project.assignments = remaining;
    return out;
  };

  const removal = (
    targets: RemovalTargetOutcome[],
    extra: Partial<RemovalReport> = {},
  ): RemovalReport => ({ targets, central_removed: false, record_deleted: false, ...extra });

  // -------------------------------------------------------------------------
  // Projects
  // -------------------------------------------------------------------------

  const aggregate = (project: FixtureProject): ProjectSyncStatus => {
    if (project.assignments.length === 0) return "none";
    const rank = { synced: 1, pending: 2, stale: 3, error: 4 } as const;
    let worst: keyof typeof rank = "synced";
    for (const a of project.assignments) {
      const folded = a.status === "missing" ? "error" : a.status;
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

  const syncAssignment = (
    project: FixtureProject,
    a: ProjectSkillAssignmentDto,
  ): ProjectSyncOutcomeStatus => {
    const skill = state.skills.find((s) => s.dto.id === a.skill_id);
    const dir = projectToolDir(project, a.tool);
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
    a.status = "synced";
    a.mode = "symlink";
    a.last_error = null;
    a.synced_at = now();
    a.content_hash = skill?.contentHash ?? null;
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

  const assign = (project: FixtureProject, skill: FixtureSkill, tool: string): ProjectSyncOutcome => {
    const a: ProjectSkillAssignmentDto = {
      id: newId(`${project.id}-asg`),
      project_id: project.id,
      skill_id: skill.dto.id,
      skill_name: skill.dto.name,
      tool,
      mode: "symlink",
      status: "pending",
      last_error: null,
      synced_at: null,
      content_hash: null,
      created_at: now(),
    };
    project.assignments.push(a);
    return outcomeOf(project, a, syncAssignment(project, a));
  };

  // -------------------------------------------------------------------------
  // Propagation / Refresh
  // -------------------------------------------------------------------------

  const propagate = (skill: FixtureSkill): PropagationOutcome[] => {
    const out: PropagationOutcome[] = [];
    for (const row of skill.dto.targets) {
      const scope = { scope: "global" as const, tool: row.tool };
      if (!toolByKey(row.tool)) {
        out.push({ scope, status: { status: "skipped", reason: { reason: "unknown_tool", tool: row.tool } } });
      } else if (!isInstalled(row.tool)) {
        out.push({ scope, status: { status: "skipped", reason: { reason: "tool_not_installed", tool: row.tool } } });
      } else if (row.mode !== "copy" && row.status === "synced") {
        out.push({ scope, status: { status: "skipped", reason: { reason: "link_follows_source" } } });
      } else if (state.unwritableTools.includes(row.tool)) {
        row.status = "error";
        out.push({ scope, status: { status: "failed", error: { code: "TOOL_NOT_WRITABLE", tool: row.tool, path: globalRoot(row.tool) } } });
      } else {
        row.status = "synced";
        row.mode = "symlink";
        row.synced_at = now();
        out.push({ scope, status: { status: "synced", mode_used: "symlink" } });
      }
    }
    for (const project of state.projects) {
      for (const a of project.assignments.filter((x) => x.skill_id === skill.dto.id)) {
        const scope = { scope: "project" as const, project_id: project.id, tool: a.tool };
        if (!project.path_exists) {
          out.push({ scope, status: { status: "skipped", reason: { reason: "project_unavailable", project_id: project.id } } });
        } else if (a.mode !== "copy" && a.status === "synced") {
          out.push({ scope, status: { status: "skipped", reason: { reason: "link_follows_source" } } });
        } else {
          const result = syncAssignment(project, a);
          out.push({
            scope,
            status: result.status === "failed" ? { status: "failed", error: result.error } : { status: "synced", mode_used: "symlink" },
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

  const reassert = (skill: FixtureSkill) => {
    for (const tool of effectiveTargets()) {
      if (!isInstalled(tool) || skill.dto.targets.some((t) => t.tool === tool)) continue;
      const status = syncPair(skill, tool, false, true);
      if (status.status !== "synced") continue;
    }
  };

  const applyRefresh = (skill: FixtureSkill, policy: RefreshPolicyDto): SkillRefreshStatus => {
    const { acquisition } = skill;
    if (!skill.dto.refreshable) return { status: "failed", error: { code: "NOT_REFRESHABLE", name: skill.dto.name } };
    if (skill.dto.unlocatable === "source_missing")
      return { status: "failed", error: { code: "SOURCE_PATH_MISSING", path: skill.dto.source_ref ?? skill.dto.name } };
    if (acquisition.fail) return { status: "failed", error: acquisition.fail };
    if (acquisition.skip) return { status: "skipped_acquisition", reason: acquisition.skip };
    const restoring = skill.dto.unlocatable === "central_missing";
    if (restoring || acquisition.upstreamChanged) {
      landBytes(skill, acquisition.upstreamChanged ? 2 : 1);
      acquisition.upstreamChanged = false;
    }
    let editConflict = null;
    if (acquisition.editConflict && skill.dto.invocation_override) {
      skill.dto.invocation_override.conflict = true;
      editConflict = acquisition.editConflict;
    }
    const targets = propagate(skill);
    let reassertError: CommandError | null = null;
    if (policy.reassert_auto_sync) {
      if (acquisition.reassertError) reassertError = acquisition.reassertError;
      else reassert(skill);
    }
    return {
      status: "refreshed",
      content_hash: skill.contentHash,
      source_revision: skill.dto.source_type === "git" ? fingerprint(`${skill.dto.name}:${skill.dto.updated_at}`) : null,
      targets,
      reassert_error: reassertError,
      edit_conflict: editConflict,
    };
  };

  const refreshBatch = async (
    ids: string[] | null,
    policy: RefreshPolicyDto,
    onProgress: ProgressSink<RefreshProgressDto>,
  ): Promise<RefreshReport> => {
    await pause(120);
    const members = ids ? ids.map(findSkill) : state.skills.filter((s) => s.dto.refreshable);
    const outcomes = new Map<string, SkillRefreshOutcome>();
    const dispatched: FixtureSkill[] = [];
    for (const skill of members) {
      if (ids === null && skill.dto.unlocatable) {
        outcomes.set(skill.dto.id, { skill_id: skill.dto.id, skill_name: skill.dto.name, status: { status: "skipped", state: skill.dto.unlocatable } });
      } else {
        dispatched.push(skill);
      }
    }
    state.cancelRequested = false;
    const total = dispatched.length;
    for (const [i, skill] of dispatched.entries()) {
      emit(onProgress, { index: i + 1, total, skill_name: skill.dto.name, phase: "acquiring" });
      await step(total * 2);
      if (state.cancelRequested) {
        state.cancelRequested = false;
        fail({ code: "CANCELLED" });
      }
    }
    for (const [i, skill] of dispatched.entries()) {
      emit(onProgress, { index: i + 1, total, skill_name: skill.dto.name, phase: "applying" });
      await step(total * 2);
      outcomes.set(skill.dto.id, { skill_id: skill.dto.id, skill_name: skill.dto.name, status: applyRefresh(skill, policy) });
    }
    return { skills: members.map((s) => outcomes.get(s.dto.id)).filter((o): o is SkillRefreshOutcome => o !== undefined) };
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
    const seenPaths = new Set<string>();
    let found = 0;
    for (const tool of TOOL_REGISTRY.filter((t) => scope.includes(t.key))) {
      const root = globalRoot(tool.key);
      for (const dir of state.foreign.filter((f) => globalRoot(f.tool) === root)) {
        const path = `${root}/${dir.name}`;
        if (seenPaths.has(path)) continue;
        seenPaths.add(path);
        found += 1;
        const group = groups.get(dir.name) ?? { name: dir.name, variants: [], has_conflict: false };
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
      total_skills_found: found,
      groups: [...groups.values()].sort((a, b) => a.name.localeCompare(b.name)),
    };
  };

  const importGroup = (
    selection: OnboardingSelectionDto,
    plan: OnboardingPlan,
    policy: ImportPolicyDto,
  ): ImportGroupOutcome => {
    const failed = (error: CommandError): ImportGroupOutcome => ({ group_name: selection.group_name, status: { status: "failed", error } });
    const group = plan.groups.find((g) => g.name === selection.group_name);
    if (!group) return failed({ code: "NOT_FOUND", kind: "onboarding_group", id: selection.group_name });
    const chosen = group.variants.find((v) => v.path === selection.chosen_path);
    if (!chosen) return failed({ code: "INVALID_PATH", path: selection.chosen_path, reason: "missing" });
    if (BROKEN_IMPORT_NAMES.has(group.name)) return failed({ code: "SKILL_INVALID", reason: "missing_name" });
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
    const identical = group.variants.filter((v) => v.fingerprint === chosen.fingerprint);
    const divergent = group.variants.filter((v) => v.fingerprint !== chosen.fingerprint);
    const dropForeign = (path: string) => {
      state.foreign = state.foreign.filter((f) => `${globalRoot(f.tool)}/${f.name}` !== path);
    };
    const originals: OriginalOutcome[] = divergent.map((v) => ({ path: v.path, tool: v.tool, status: { status: "kept_divergent" } }));
    let targets: BatchTargetOutcome[] = [];
    const forced: string[] = [];
    if (policy.auto_sync ?? false) {
      const requested = policy.tools ?? state.installedTools;
      for (const v of identical) {
        if (!requested.includes(v.tool) && !forced.includes(v.tool)) forced.push(v.tool);
      }
      const takenOver = new Set(identical.map((v) => v.path));
      for (const path of takenOver) dropForeign(path);
      targets = [...requested, ...forced].flatMap((tool) => {
        if (!isInstalled(tool)) return [{ skill_id: skill.dto.id, skill_name: name, tool_key: tool, status: { status: "skipped", error: { code: "TOOL_NOT_INSTALLED", tool } } } satisfies BatchTargetOutcome];
        return [{ skill_id: skill.dto.id, skill_name: name, tool_key: tool, status: syncPair(skill, tool, false, true) } satisfies BatchTargetOutcome];
      });
    } else {
      for (const v of identical) {
        if (state.unwritableTools.includes(v.tool)) {
          originals.push({ path: v.path, tool: v.tool, status: { status: "failed", error: PERMISSION_DENIED(v.path) } });
        } else {
          dropForeign(v.path);
          originals.push({ path: v.path, tool: v.tool, status: { status: "removed" } });
        }
      }
    }
    return {
      group_name: group.name,
      status: { status: "imported", skill_id: skill.dto.id, skill_name: name, targets, forced_tools: forced, originals },
    };
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
      const candidates: GitSkillCandidate[] = inScope.map((c) => ({
        name: c.name,
        description: c.description,
        subpath: c.subpath,
        resolution: { branch: parsed.branch ?? "main", subpath: c.subpath },
      }));
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
    installGitSelection: async (repoUrl, subpath) => {
      await pause(650);
      const { repo } = findRepo(repoUrl);
      const candidate =
        repo.candidates.find((c) => c.subpath === subpath) ??
        fail({ code: "SUBPATH_MISSING", subpath });
      // An operator-provided name wins in the real finalize; the fixture
      // keeps the manifest name so the catalog stays coherent.
      const skill = finalize({
        name: candidate.name,
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
    unsyncSkillFromTool: async (skillId, tool) => {
      await pause(200);
      const skill = findSkill(skillId);
      return removal(removeGlobalRows(skill, globalSharers(tool)));
    },
    refreshManagedSkills: async (skillIds, policy, onProgress) =>
      refreshBatch(skillIds, policy, onProgress),
    updateManagedSkill: async (skillId, policy, onProgress) =>
      skillResult(await refreshBatch([skillId], policy, onProgress)),
    repointSkillSource: async (skillId, target: RepointTarget, policy) => {
      await pause(600);
      const skill = findSkill(skillId);
      const refused = (error: CommandError): SkillMutationResultDto =>
        skillResult({ skills: [{ skill_id: skill.dto.id, skill_name: skill.dto.name, status: { status: "failed", error } }] });
      if (target.kind === "git") {
        const parsed = parseGithub(target.url) ?? fail({ code: "INVALID_GITHUB_URL", url: target.url });
        const repo = state.repos.find((r) => r.slug.toLowerCase() === parsed.slug.toLowerCase());
        if (!repo) return refused({ code: "GIT_CLONE_FAILED", kind: "notFound", detail: `remote: Repository not found (${parsed.slug})` });
        if (repo.listingError) return refused(repo.listingError);
        const pick = parsed.subpath
          ? repo.candidates.find((c) => c.subpath === parsed.subpath)
          : repo.candidates.length === 1
            ? repo.candidates[0]
            : repo.candidates.find((c) => c.name === skill.dto.name);
        if (!pick) return refused(parsed.subpath ? { code: "SUBPATH_MISSING", subpath: parsed.subpath } : { code: "MULTI_SKILLS" });
        skill.dto.source_type = "git";
        skill.dto.source_ref = target.url.trim();
      } else {
        const path = expandHome(target.path);
        const tool = insideToolDir(path);
        if (tool) fail({ code: "LOCAL_SOURCE_INSIDE_TOOL_DIR", path, tool: tool.key });
        findLocal(path);
        skill.dto.source_type = "local";
        skill.dto.source_ref = path;
      }
      skill.dto.imported_from_tool = null;
      skill.dto.refreshable = true;
      skill.acquisition = {};
      landBytes(skill, 1);
      const targets = propagate(skill);
      if (policy.reassert_auto_sync) reassert(skill);
      return skillResult({
        skills: [{
          skill_id: skill.dto.id,
          skill_name: skill.dto.name,
          status: { status: "refreshed", content_hash: skill.contentHash, source_revision: target.kind === "git" ? fingerprint(skill.dto.source_ref ?? "") : null, targets, reassert_error: null, edit_conflict: null },
        }],
      });
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
    importOnboardingSelection: async (selections, policy, onProgress: ProgressSink<ImportProgressDto>) => {
      await pause(150);
      const plan = buildPlan();
      const groups: ImportGroupOutcome[] = [];
      for (const [i, selection] of selections.entries()) {
        emit(onProgress, { index: i + 1, total: selections.length, group_name: selection.group_name, phase: "admitting" });
        await step(selections.length * 2);
        emit(onProgress, { index: i + 1, total: selections.length, group_name: selection.group_name, phase: "applying" });
        await step(selections.length * 2);
        groups.push(importGroup(selection, plan, policy));
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
      return {
        report: { skill_id: skill.dto.id, skill_name: skill.dto.name, propagation: { targets: propagate(skill) } },
        skills: skillsDto(),
      };
    },
    deleteManagedSkill: async (skillId) => {
      await pause(300);
      const skill = findSkill(skillId);
      const targets = removeGlobalRows(skill, skill.dto.targets.map((t) => t.tool));
      for (const project of state.projects) {
        targets.push(...removeAssignments(project, (a) => a.skill_id !== skillId));
      }
      if (targets.some((t) => t.status.status === "failed")) return removal(targets);
      state.skills = state.skills.filter((s) => s !== skill);
      return removal(targets, { central_removed: true, record_deleted: true });
    },
    unsyncAllSkills: async () => {
      await pause(400);
      return removal(state.skills.flatMap((s) => removeGlobalRows(s, s.dto.targets.map((t) => t.tool))));
    },
    unsyncSkill: async (skillId) => {
      await pause(200);
      const skill = findSkill(skillId);
      return removal(removeGlobalRows(skill, skill.dto.targets.map((t) => t.tool)));
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
      await pause(800);
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
      const targets = removeAssignments(project, () => false);
      if (!targets.some((t) => t.status.status === "failed")) {
        state.projects = state.projects.filter((p) => p !== project);
      }
      return { projects: projectsDto(), report: removal(targets) };
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
      const targets = removeAssignments(project, (a) => !dropped.includes(a.tool));
      const stuck = new Set(project.assignments.filter((a) => dropped.includes(a.tool)).map((a) => a.tool));
      project.tools = [...tools, ...dropped.filter((t) => stuck.has(t))];
      if (gitignore) project.gitignore = { in_gitignore: gitignore.add_to_gitignore, in_exclude: gitignore.add_to_exclude };
      project.updated_at = now();
      return { view: viewOf(project), report: removal(targets) };
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
      const items = project.assignments.map((a) => outcomeOf(project, a, syncAssignment(project, a)));
      return { view: viewOf(project), report: { items } };
    },
    resyncAllProjects: async () => {
      await pause(700);
      const items = state.projects.flatMap((project) =>
        project.assignments.map((a) => outcomeOf(project, a, syncAssignment(project, a))),
      );
      return { report: { items }, projects: projectsDto() };
    },
    bulkAssignSkill: async (projectId, skillId) => {
      await pause(300);
      const project = findProject(projectId);
      const skill = findSkill(skillId);
      const items = project.tools.map((tool) => {
        const existing = project.assignments.find((a) => a.skill_id === skillId && a.tool === tool);
        return existing ? outcomeOf(project, existing, { status: "already_assigned" }) : assign(project, skill, tool);
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
