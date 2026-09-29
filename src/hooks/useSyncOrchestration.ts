import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  BatchSyncOverrideDto,
  BatchTargetOutcome,
  BatchSyncSkillDto,
  DisableScope,
  EnableReport,
  SyncProgressDto,
  ToolStatusDto,
} from "../components/skills/types";
import type { AssignmentDecision, ProjectSyncReport } from "../components/projects/types";
import type { PropagationReport, RefreshReport } from "../bindings";
import { createProgressChannel } from "../lib/platform";
import { invokeTauri, isTauri } from "../lib/tauri";
import type { ToolCatalog } from "./useToolCatalog";
import type { AskAnswer, DialogDoor, RowChoices, SurfaceState } from "./useSurfaceState";
import {
  globalCollisionRows,
  mergeGlobalRetry,
  mergeProjectRetry,
  mergePropagationRetry,
  mergeRefreshRetry,
  overwriteRowKey,
  projectCollisionRows,
  propagationCollisionRows,
  refreshCollisionRows,
  type CollisionRetry,
  type DeclinedPair,
  type OverwriteChoice,
  type OverwriteRow,
} from "../lib/overwriteAsk";
import {
  useSharedDirConfirmation,
  type SharedDirTool,
} from "./useSharedDirConfirmation";
import type {
  StatusReporter,
  TranslateFn,
} from "./useStatusReporter";

export type { OverwriteChoice, OverwriteRow } from "../lib/overwriteAsk";

/** The overwrite ask as its modal renders it (`surface.pendingAsk("overwrite")`). */
export type OverwritePending = {
  rows: OverwriteRow[];
  answer: (answer: AskAnswer | RowChoices) => void;
};

/**
 * The operator's answer to the overwrite ask: one choice per row, in row
 * order — `keep` only ever from an explicit Keep (the backend then records
 * the pair `disabled`) — or `dismissed` (Escape, outside press, a displacing
 * ask, the stack replaced, unmount): no answer, records nothing.
 */
export type OverwriteAnswer = readonly OverwriteChoice[] | "dismissed";

/** The per-row choices an ask settled with. */
function choicesOf(
  answer: AskAnswer | RowChoices,
  rowCount: number,
): OverwriteAnswer {
  if (answer === "dismissed") return "dismissed";
  if (answer === "confirmed") return Array(rowCount).fill("overwrite");
  if (answer === "declined") return Array(rowCount).fill("keep");
  return answer.choices;
}

export type SyncPolicy = {
  overwrite?: boolean;
  overwriteIfSameContent?: boolean;
  overrides?: BatchSyncOverrideDto[];
};

/**
 * How an asked global row is retried: through a sync batch for its skill
 * (a first sync, a re-assert, Propagation over a never-synced row), or
 * through the Enable scope that produced it. Project rows need no route.
 */
export type GlobalRetryRoute =
  | { via: "sync"; skill: BatchSyncSkillDto; overwriteIfSameContent: boolean }
  | { via: "enable"; scope: DisableScope };

/** One askable row and how its answer is carried out. */
export type CollisionClaim = { row: OverwriteRow; route?: GlobalRetryRoute };

/**
 * A report one operator action produced, as the one ask settles it. `sync`:
 * a global sync batch (`skills` are its items, for the retry); `enable`: an
 * Enable over `scope`; `project`: a project sync report; `refresh`: a
 * Refresh / Update / Restore / Re-point report (Propagation and the
 * re-assert, retried through a sync batch for the skill — `skills` are the
 * catalog's items); `propagation`: one skill's Edit Propagation.
 */
export type CollisionPart =
  | { kind: "sync"; report: BatchTargetOutcome[]; skills: readonly BatchSyncSkillDto[]; policy?: SyncPolicy }
  | { kind: "enable"; report: EnableReport; scope: DisableScope }
  | { kind: "project"; report: ProjectSyncReport }
  | { kind: "refresh"; report: RefreshReport; skills: readonly BatchSyncSkillDto[] }
  | { kind: "propagation"; report: PropagationReport; skill: BatchSyncSkillDto };

/** A part with its answered rows settled; `declined` for the shapes that carry no declined row. */
export type SettledPart<P extends CollisionPart> = P & { declined: DeclinedPair[] };

export type SettledParts<P extends readonly CollisionPart[]> = {
  parts: { [K in keyof P]: SettledPart<P[K]> };
  retried: boolean;
  projectIds: string[];
};

/** The askable rows of one part, each with its retry route. */
function claimsOf(part: CollisionPart, labels: Readonly<Record<string, string>>): CollisionClaim[] {
  const project = (row: OverwriteRow): CollisionClaim => ({ row });
  const viaSync = (skills: readonly BatchSyncSkillDto[], sameContent: boolean) => {
    const bySkill = new Map(skills.map((skill) => [skill.skill_id, skill]));
    return (row: OverwriteRow): CollisionClaim[] => {
      if (row.scope.kind === "project") return [project(row)];
      const skill = bySkill.get(row.skillId);
      return skill ? [{ row, route: { via: "sync", skill, overwriteIfSameContent: sameContent } }] : [];
    };
  };
  switch (part.kind) {
    case "sync":
      return globalCollisionRows(part.report, labels).flatMap(
        viaSync(part.skills, part.policy?.overwriteIfSameContent ?? false),
      );
    case "enable":
      return [
        ...globalCollisionRows(part.report.global, labels).map(
          (row): CollisionClaim => ({ row, route: { via: "enable", scope: part.scope } }),
        ),
        ...projectCollisionRows(part.report.projects, labels).map(project),
      ];
    case "project":
      return projectCollisionRows(part.report, labels).map(project);
    // Propagation and the re-assert write with the same-content rule on.
    case "refresh":
      return refreshCollisionRows(part.report, labels).flatMap(viaSync(part.skills, true));
    case "propagation":
      return propagationCollisionRows(
        { skillId: part.skill.skill_id, skillName: part.skill.name },
        part.report.targets,
        labels,
      ).flatMap(viaSync([part.skill], true));
  }
}

/** One part with the retry's rows in place of the asked ones. */
function settlePart(part: CollisionPart, retry: CollisionRetry | null): SettledPart<CollisionPart> {
  if (!retry) return { ...part, declined: [] };
  const projects = (report: ProjectSyncReport) =>
    retry.projects ? mergeProjectRetry(report, retry.projects) : report;
  switch (part.kind) {
    case "sync":
      return { ...part, report: mergeGlobalRetry(part.report, retry.rows, retry.global), declined: [] };
    case "enable":
      return {
        ...part,
        report: {
          global: mergeGlobalRetry(part.report.global, retry.rows, retry.global),
          projects: projects(part.report.projects),
        },
        declined: [],
      };
    case "project":
      return { ...part, report: projects(part.report), declined: [] };
    case "refresh": {
      const merged = mergeRefreshRetry(part.report, retry);
      return { ...part, report: merged.report, declined: merged.declined };
    }
    case "propagation": {
      const merged = mergePropagationRetry(
        { skillId: part.skill.skill_id, skillName: part.skill.name },
        part.report.targets,
        retry,
      );
      return { ...part, report: { targets: merged.targets }, declined: merged.declined };
    }
  }
}

/**
 * "Only scan selected tools": ignore newly detected tools that are not
 * part of the saved global tool selection.
 */
function filterRelevantNewlyInstalled(
  newlyInstalled: string[],
  scanSelectedOnly: boolean,
  selectedTools: string[] | null,
): string[] {
  if (scanSelectedOnly && selectedTools) {
    return newlyInstalled.filter((id) => selectedTools.includes(id));
  }
  return newlyInstalled;
}

export type SyncOrchestrationDeps = {
  t: TranslateFn;
  reporter: Pick<
    StatusReporter,
    | "loading"
    | "notify"
    | "setActionMessage"
    | "setError"
    | "setSuccessToastMessage"
    | "formatError"
  >;
  /** The surface module: this world's dialogs and its two asks. */
  surface: DialogDoor & Pick<SurfaceState, "ask">;
  /** The Tool catalog: what exists, what is installed, labels, shared dirs. */
  catalog: Pick<
    ToolCatalog,
    | "toolStatus"
    | "tools"
    | "toolLabelById"
    | "installedToolIds"
    | "sharedToolIdsByToolId"
  >;
};

/**
 * The Add modal's per-install deploy targets before the operator touches
 * them: the saved global selection when configured, otherwise the installed
 * tools. Empty until the tool status has loaded.
 */
function defaultSyncTargets(
  status: ToolStatusDto | null,
  selectedTools: string[] | null,
): Record<string, boolean> {
  const next: Record<string, boolean> = {};
  if (!status) return next;
  for (const info of status.tools) {
    next[info.key] = selectedTools
      ? selectedTools.includes(info.key)
      : status.installed.includes(info.key);
  }
  return next;
}

/**
 * Sync world: the saved global tool selection, the two target sets (the
 * Effective global target set and the Add modal's per-install deploy
 * targets), the auto-sync switch, and the one sync fan-out seam with its
 * overwrite and shared-dir asks. What tools exist is the Tool catalog's
 * (`useToolCatalog`), received as a dependency. The
 * backend owns the sync choreography (installedness filtering, shared-dir
 * dedupe, overwrite policy, DB record fan-out) and streams per-pair progress
 * back over a channel. Nothing else may loop a per-pair sync command —
 * sync_skill_to_tool no longer exists.
 */
export function useSyncOrchestration({
  t,
  reporter,
  surface,
  catalog,
}: SyncOrchestrationDeps) {
  const {
    loading,
    notify,
    setActionMessage,
    setError,
    setSuccessToastMessage,
    formatError,
  } = reporter;
  const { ask, offerDialog, openDialog, closeDialog, dismissDialog } = surface;
  const {
    toolStatus,
    tools,
    toolLabelById,
    installedToolIds,
    sharedToolIdsByToolId,
  } = catalog;
  // The settings snapshot has been read (or failed to): the target defaults
  // and the new-tools announcement wait for it, so they respect the saved
  // selection.
  const [settingsSettled, setSettingsSettled] = useState(false);
  const [globalSelectedTools, setGlobalSelectedTools] = useState<
    string[] | null
  >(null);
  const [scanSelectedToolsOnly, setScanSelectedToolsOnly] = useState(true);
  /**
   * Bumped on every saved tool configuration. The onboarding scan's scope is
   * resolved by the backend from that configuration, so a plan fetched
   * before a save no longer describes what an import would act on; the
   * add/import world reloads its plan when this changes.
   */
  const [scanScopeRevision, setScanScopeRevision] = useState(0);
  // The Add modal's deploy targets: derived from the saved selection and
  // detection until the operator (or a saved configuration) sets them.
  const [chosenSyncTargets, setChosenSyncTargets] = useState<Record<
    string,
    boolean
  > | null>(null);
  const [autoSyncEnabled, setAutoSyncEnabled] = useState(true);

  const toolInfos = useMemo(() => toolStatus?.tools ?? [], [toolStatus]);

  const defaultTargets = useMemo(
    () =>
      settingsSettled
        ? defaultSyncTargets(toolStatus, globalSelectedTools)
        : {},
    [globalSelectedTools, settingsSettled, toolStatus],
  );
  const syncTargets = chosenSyncTargets ?? defaultTargets;

  // The shared-dir confirmation building-block hook, fed the localized tool
  // labels. Both flows that can affect a whole group (this hook's sync
  // target change and the skill library's per-tool toggle) await it.
  const sharedDirTools: SharedDirTool[] = useMemo(
    () =>
      toolInfos.map((info) => ({
        id: info.key,
        label: toolLabelById[info.key] ?? info.label,
        sharedWith: info.shared_with,
      })),
    [toolInfos, toolLabelById],
  );
  const { request: requestSharedDirConfirmation } = useSharedDirConfirmation(
    sharedDirTools,
    surface,
  );
  // The one overwrite ask (D12) is a nested ask on the surface stack: a
  // newer ask dismisses the one it displaces, and every ask settles exactly
  // once. Every producer's collided rows — global pairs and project
  // assignments alike — are asked in one go and answered per row.
  const requestOverwriteConfirmation = useCallback(
    async (rows: OverwriteRow[]): Promise<OverwriteAnswer> => {
      setActionMessage(t("overwrite.waiting"));
      return choicesOf(await ask({ kind: "overwrite", rows }), rows.length);
    },
    [ask, setActionMessage, t],
  );

  /**
   * The tools a global sync writes to. Mirrors the backend rule
   * (`settings::effective_global_tool_targets`) exactly: the operator's
   * recorded selection when they configured one — including an empty one,
   * which means "sync nowhere" — otherwise every detected tool. Detection is
   * a fallback for a never-configured install, never an override of intent,
   * and the set is not intersected with detection: a selected-but-uninstalled
   * tool stays in it so the backend reports it as a skip.
   */
  const effectiveSyncTargetIds = useMemo(
    () => globalSelectedTools ?? installedToolIds,
    [globalSelectedTools, installedToolIds],
  );

  const relevantNewlyInstalled = useMemo(() => {
    if (!toolStatus) return [] as string[];
    return filterRelevantNewlyInstalled(
      toolStatus.newly_installed,
      scanSelectedToolsOnly,
      globalSelectedTools,
    );
  }, [toolStatus, scanSelectedToolsOnly, globalSelectedTools]);

  const newlyInstalledToolsText = useMemo(() => {
    if (relevantNewlyInstalled.length === 0) return "";
    return relevantNewlyInstalled
      .map((id) => tools.find((t) => t.id === id)?.label ?? id)
      .join(t("common.listSeparator"));
  }, [relevantNewlyInstalled, t, tools]);

  useEffect(() => {
    if (!isTauri) return;
    void (async () => {
      try {
        const settings = await invokeTauri("getSettings");
        setAutoSyncEnabled(settings.auto_sync_enabled);
        setGlobalSelectedTools(settings.global_selected_tools);
        setScanSelectedToolsOnly(settings.scan_selected_tools_only);
        // A corrupt saved selection reads as "unconfigured" for display only:
        // every global sync refuses (SETTING_CORRUPT) until the operator
        // saves the selection again, so say so once, up front.
        if (settings.global_selected_tools_corrupt)
          notify("warning", t("errors.settingCorruptStartup"));
      } catch (err) {
        // Non-fatal; fall back to defaults.
        console.warn(err);
      }
      setSettingsSettled(true);
    })();
    // Mount-once load: the corrupt-selection warning is raised with the
    // reporter and copy of that moment, not again on every language switch.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Newly detected tools are announced once, when both the tool status and
  // the saved selection are known, and only when no other dialog is up (it
  // may be the update's turn).
  const newToolsChecked = useRef(false);
  useEffect(() => {
    if (newToolsChecked.current || !settingsSettled || !toolStatus) return;
    newToolsChecked.current = true;
    if (relevantNewlyInstalled.length > 0) offerDialog({ kind: "newTools" });
  }, [offerDialog, relevantNewlyInstalled, settingsSettled, toolStatus]);

  const invokeBatchSync = useCallback(
    async (
      skills: BatchSyncSkillDto[],
      toolIds: string[],
      policy?: SyncPolicy,
    ): Promise<BatchTargetOutcome[]> => {
      const onProgress = await createProgressChannel<SyncProgressDto>(
        (progress) => {
          setActionMessage(
            t("actions.syncStep", {
              index: progress.index,
              total: progress.total,
              name: progress.skill_name,
              tool: toolLabelById[progress.tool] ?? progress.tool,
            }),
          );
        },
      );
      return invokeTauri(
        "syncSkillsToTools",
        skills,
        toolIds,
        {
          overwrite: policy?.overwrite ?? false,
          overwrite_if_same_content: policy?.overwriteIfSameContent ?? false,
          overrides: policy?.overrides ?? [],
        },
        onProgress,
      );
    },
    [setActionMessage, t, toolLabelById],
  );

  /** The Enable command without its ask: the report as it settled. */
  const collectEnable = useCallback(
    (scope: DisableScope): Promise<EnableReport> =>
      invokeTauri("enableSkillTargets", scope, []),
    [],
  );

  /** Project assignments: the asked rows' decisions, resolved in one call. */
  const resolveAssignments = useCallback(
    async (
      rows: readonly OverwriteRow[],
      choices: readonly OverwriteChoice[],
    ): Promise<ProjectSyncReport | null> => {
      const decisions: AssignmentDecision[] = rows.flatMap((row, index) =>
        row.scope.kind === "project"
          ? [{
              project_id: row.scope.projectId,
              skill_id: row.skillId,
              tool: row.toolKey,
              decision: choices[index],
            }]
          : [],
      );
      return decisions.length
        ? invokeTauri("resolveProjectCollisions", decisions)
        : null;
    },
    [],
  );

  /**
   * The one overwrite ask over one operator action's collisions (D12): every
   * claimed row is asked in one go and answered per row; the answer's
   * retries run for the asked rows only. A global pair is retried the way
   * it was produced — a sync batch per group of skills asking about the
   * same Tools (exact pair identity: no skill × Tool product reaches a pair
   * nobody asked about), or its Enable scope with per-pair decisions — and
   * project assignments through `resolveProjectCollisions`. Null when
   * nothing was askable or the ask was dismissed: no retry ran, nothing was
   * recorded.
   */
  const settleClaims = useCallback(
    async (claims: readonly CollisionClaim[]): Promise<CollisionRetry | null> => {
      const seen = new Set<string>();
      const unique = claims.filter((claim) => {
        const key = overwriteRowKey(claim.row);
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
      });
      if (unique.length === 0) return null;
      const rows = unique.map((claim) => claim.row);
      const answer = await requestOverwriteConfirmation(rows);
      if (answer === "dismissed") return null;

      const global: BatchTargetOutcome[] = [];
      // Sync-route pairs: one batch per (Tool set, same-content rule).
      const bySkill = new Map<string, { skill: BatchSyncSkillDto; sameContent: boolean; tools: string[]; overrides: BatchSyncOverrideDto[] }>();
      // Enable-route pairs: one retry per scope.
      const byScope = new Map<string, { scope: DisableScope; overrides: BatchSyncOverrideDto[] }>();
      unique.forEach((claim, index) => {
        const { row, route } = claim;
        if (row.scope.kind !== "global" || !route) return;
        const override = { skill_id: row.skillId, tool: row.toolKey, decision: answer[index] };
        if (route.via === "sync") {
          const entry = bySkill.get(row.skillId) ?? { skill: route.skill, sameContent: route.overwriteIfSameContent, tools: [], overrides: [] };
          entry.tools.push(row.toolKey);
          entry.overrides.push(override);
          bySkill.set(row.skillId, entry);
        } else {
          const key = JSON.stringify(route.scope);
          const entry = byScope.get(key) ?? { scope: route.scope, overrides: [] };
          entry.overrides.push(override);
          byScope.set(key, entry);
        }
      });
      const batches = new Map<string, { skills: BatchSyncSkillDto[]; tools: string[]; sameContent: boolean; overrides: BatchSyncOverrideDto[] }>();
      for (const entry of bySkill.values()) {
        const key = `${entry.sameContent}\n${entry.tools.join("\n")}`;
        const batch = batches.get(key) ?? { skills: [], tools: entry.tools, sameContent: entry.sameContent, overrides: [] };
        batch.skills.push(entry.skill);
        batch.overrides.push(...entry.overrides);
        batches.set(key, batch);
      }
      for (const batch of batches.values()) {
        global.push(
          ...(await invokeBatchSync(batch.skills, batch.tools, {
            overwrite: false,
            overwriteIfSameContent: batch.sameContent,
            overrides: batch.overrides,
          })),
        );
      }
      for (const entry of byScope.values()) {
        global.push(...(await invokeTauri("enableSkillTargets", entry.scope, entry.overrides)).global);
      }
      const projects = await resolveAssignments(rows, answer);
      // Only the asked pairs' rows: a retry reporting other pairs does not
      // speak for the operator's action.
      const askedPairs = new Set(
        rows.filter((row) => row.scope.kind === "global").map((row) => `${row.skillId}\n${row.toolKey}`),
      );
      return {
        rows,
        global: global.filter((row) => askedPairs.has(`${row.skill_id}\n${row.tool_key}`)),
        projects,
      };
    },
    [invokeBatchSync, requestOverwriteConfirmation, resolveAssignments],
  );

  /**
   * One operator action's reports, settled through one ask: each part's
   * `TARGET_EXISTS` rows are claimed (a sync batch's pairs retried through
   * the batch, Enable's through its scope, project assignments and
   * Propagation / re-assert rows by where they live), asked once, and each
   * part comes back with its answered rows replaced — Refresh / Propagation
   * parts with the kept pairs as `declined` (their report shape has no
   * declined row). `retried` says an answer ran retries (a catalog or view
   * read before it is stale); `projectIds` are the projects a retry touched.
   */
  const settleCollisionParts = useCallback(
    async <P extends readonly CollisionPart[]>(parts: P): Promise<SettledParts<P>> => {
      const claims = parts.flatMap((part) => claimsOf(part, toolLabelById));
      const retry = await settleClaims(claims);
      // SAFETY: `settlePart` returns each part spread with only `report`
      // (same kind's shape) and `declined` replaced, in input order, so the
      // array is the mapped tuple `SettledParts<P>["parts"]`; TypeScript
      // cannot map a tuple through a union-typed function.
      const settled = parts.map((part) => settlePart(part, retry)) as unknown as SettledParts<P>["parts"];
      return {
        parts: settled,
        retried: retry !== null,
        projectIds: retry
          ? [...new Set(retry.rows.flatMap((row) => (row.scope.kind === "project" ? [row.scope.projectId] : [])))]
          : [],
      };
    },
    [settleClaims, toolLabelById],
  );

  /**
   * The global half of the ask for a finished batch: `report`'s
   * `TARGET_EXISTS` rows (a target occupied by *different* content — the
   * caller's same-content rule already replaced identical content silently)
   * raise one ask; each answered pair is retried with its decision —
   * `overwrite`, or `keep` (the backend records the pair `disabled`, so no
   * later pass re-asks) — under the same same-content rule, and replaces its
   * row. A dismissed ask is no answer: no retry, the report as it settled.
   * The skills must exist (a sync batch or a finished install).
   */
  const resolveGlobalCollisions = useCallback(
    async (
      report: BatchTargetOutcome[],
      skills: BatchSyncSkillDto[],
      policy?: SyncPolicy,
    ): Promise<BatchTargetOutcome[]> => {
      const { parts: [settled] } = await settleCollisionParts([{ kind: "sync", report, skills, policy }] as const);
      return settled.report;
    },
    [settleCollisionParts],
  );

  /** The one sync fan-out seam, with the overwrite ask (see above). */
  const syncSkillsToTools = useCallback(
    async (
      skills: BatchSyncSkillDto[],
      toolIds: string[],
      policy?: SyncPolicy,
    ): Promise<BatchTargetOutcome[]> =>
      resolveGlobalCollisions(
        await invokeBatchSync(skills, toolIds, policy),
        skills,
        policy,
      ),
    [invokeBatchSync, resolveGlobalCollisions],
  );

  /**
   * The project half: a project report's collided assignments (assign,
   * resync) raise the one ask and are settled by `resolveProjectCollisions`;
   * their rows are replaced in the report. `resolved: false` when nothing
   * was asked or the ask was dismissed (the report is as it settled).
   */
  const settleProjectCollisions = useCallback(
    async (
      report: ProjectSyncReport,
    ): Promise<{ report: ProjectSyncReport; resolved: boolean }> => {
      const { parts: [settled], retried } = await settleCollisionParts([{ kind: "project", report }] as const);
      return { report: settled.report, resolved: retried };
    },
    [settleCollisionParts],
  );

  /**
   * Enable, with its collisions in the one ask: global pairs whose first
   * sync met a folder stay `disabled` until answered — the retry is the same
   * scope with per-pair decisions — and project assignments are settled by
   * `resolveProjectCollisions`. The answered rows replace the asked ones.
   */
  const enableSkillTargets = useCallback(
    async (scope: DisableScope): Promise<EnableReport> => {
      const report = await collectEnable(scope);
      const { parts: [settled] } = await settleCollisionParts([{ kind: "enable", report, scope }] as const);
      return settled.report;
    },
    [collectEnable, settleCollisionParts],
  );

  const handleAutoSyncToggle = useCallback(
    async (enabled: boolean) => {
      try {
        await invokeTauri("updateSetting", {
          key: "auto_sync_enabled",
          value: enabled,
        });
        setAutoSyncEnabled(enabled);
      } catch (err) {
        setError(formatError(err));
      }
    },
    [formatError, setError],
  );

  const handleOpenToolConfig = useCallback(() => {
    openDialog({ kind: "toolConfig" });
  }, [openDialog]);

  const handleCloseToolConfig = useCallback(() => {
    if (!loading) closeDialog("toolConfig");
  }, [closeDialog, loading]);

  const handleCloseNewTools = useCallback(() => {
    if (!loading) closeDialog("newTools");
  }, [closeDialog, loading]);

  const handleToolConfigConfirm = useCallback(
    async (selected: string[], scanOnly = false) => {
      try {
        await invokeTauri("updateSetting", {
          key: "global_tool_config",
          value: { selected_tools: selected, scan_selected_only: scanOnly },
        });
        setGlobalSelectedTools(selected);
        setScanSelectedToolsOnly(scanOnly);
        setScanScopeRevision((revision) => revision + 1);
        // Deploy targets follow the saved selection exactly.
        const next: Record<string, boolean> = {};
        for (const info of toolInfos) {
          next[info.key] = selected.includes(info.key);
        }
        setChosenSyncTargets(next);
        dismissDialog("toolConfig");
        setSuccessToastMessage(t("status.toolConfigSaved"));
      } catch (err) {
        setError(formatError(err));
      }
    },
    [dismissDialog, formatError, setError, setSuccessToastMessage, t, toolInfos],
  );

  const handleSyncTargetChange = useCallback(
    async (toolId: string, checked: boolean) => {
      const shared = sharedToolIdsByToolId[toolId] ?? [toolId];
      const confirmed = await requestSharedDirConfirmation(toolId);
      if (!confirmed) return;
      setChosenSyncTargets((prev) => {
        const next = { ...(prev ?? defaultTargets) };
        for (const id of shared) next[id] = checked;
        return next;
      });
    },
    [defaultTargets, requestSharedDirConfirmation, sharedToolIdsByToolId],
  );

  /** Enable deploy targets for the given tools, expanded to shared-dir groups. */
  const enableTargetsFor = useCallback(
    (toolIds: string[]) => {
      setChosenSyncTargets((prev) => {
        const next = { ...(prev ?? defaultTargets) };
        for (const id of toolIds) {
          const shared = sharedToolIdsByToolId[id] ?? [id];
          for (const sid of shared) next[sid] = true;
        }
        return next;
      });
    },
    [defaultTargets, sharedToolIdsByToolId],
  );

  /** Reset deploy targets to exactly the currently installed tools. */
  const targetAllInstalled = useCallback(() => {
    if (!toolStatus) return;
    const targets: Record<string, boolean> = {};
    for (const id of toolStatus.installed) {
      targets[id] = true;
    }
    setChosenSyncTargets(targets);
  }, [toolStatus]);

  return {
    requestSharedDirConfirmation,
    effectiveSyncTargetIds,
    globalSelectedTools,
    scanSelectedToolsOnly,
    scanScopeRevision,
    syncTargets,
    autoSyncEnabled,
    relevantNewlyInstalled,
    newlyInstalledToolsText,
    syncSkillsToTools,
    collectSync: invokeBatchSync,
    collectEnable,
    settleCollisionParts,
    resolveGlobalCollisions,
    settleProjectCollisions,
    enableSkillTargets,
    requestOverwriteConfirmation,
    handleAutoSyncToggle,
    handleOpenToolConfig,
    handleCloseToolConfig,
    handleCloseNewTools,
    handleToolConfigConfirm,
    handleSyncTargetChange,
    enableTargetsFor,
    targetAllInstalled,
  };
}

export type SyncOrchestration = ReturnType<typeof useSyncOrchestration>;
