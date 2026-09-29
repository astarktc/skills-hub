import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  BatchSyncOverrideDto,
  BatchTargetOutcome,
  BatchSyncSkillDto,
  SyncProgressDto,
  ToolStatusDto,
} from "../components/skills/types";
import { createProgressChannel } from "../lib/platform";
import { invokeTauri, isTauri } from "../lib/tauri";
import type { ToolCatalog } from "./useToolCatalog";
import type { AskAnswer, DialogDoor, SurfaceState } from "./useSurfaceState";
import {
  useSharedDirConfirmation,
  type SharedDirTool,
} from "./useSharedDirConfirmation";
import type {
  StatusReporter,
  TranslateFn,
} from "./useStatusReporter";

/** One occupied sync target the operator may authorise overwriting. */
export type OverwriteRow = {
  skillId: string;
  skillName: string;
  toolKey: string;
  toolLabel: string;
  /** The occupied target path, as the backend reported it. */
  path: string;
};

/** The overwrite ask as its modal renders it (`surface.pendingAsk("overwrite")`). */
export type OverwritePending = {
  rows: OverwriteRow[];
  answer: (answer: AskAnswer) => void;
};

/**
 * The operator's answer to the overwrite ask. `keep` is only ever the
 * explicit "Keep existing" button (the backend then records the pairs
 * `disabled`); `dismissed` — Escape, outside press, a displacing ask, the
 * stack replaced, unmount — is no answer, and records nothing.
 */
export type OverwriteAnswer = "overwrite" | "keep" | "dismissed";

const OVERWRITE_ANSWER: Record<AskAnswer, OverwriteAnswer> = {
  confirmed: "overwrite",
  declined: "keep",
  dismissed: "dismissed",
};

export type SyncPolicy = {
  overwrite?: boolean;
  overwriteIfSameContent?: boolean;
  overrides?: BatchSyncOverrideDto[];
};

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

/** The rows of a batch report that settled as an occupied target. */
function occupiedTargets(
  report: BatchTargetOutcome[],
  toolLabelById: Readonly<Record<string, string>>,
): OverwriteRow[] {
  const rows: OverwriteRow[] = [];
  for (const result of report) {
    if (
      result.status.status === "failed" &&
      result.status.error.code === "TARGET_EXISTS"
    )
      rows.push({
        skillId: result.skill_id,
        skillName: result.skill_name,
        toolKey: result.tool_key,
        toolLabel: toolLabelById[result.tool_key] ?? result.tool_key,
        path: result.status.error.path,
      });
  }
  return rows;
}

/**
 * The first report with each asked (skill, tool) row replaced by the retry's
 * row for that pair. Rows the retry produced for other pairs are dropped:
 * the report stays faithful to the operator's action.
 */
function mergeRetry(
  report: BatchTargetOutcome[],
  asked: OverwriteRow[],
  retry: BatchTargetOutcome[],
): BatchTargetOutcome[] {
  const key = (skillId: string, toolKey: string) => `${skillId}\n${toolKey}`;
  const askedKeys = new Set(asked.map((row) => key(row.skillId, row.toolKey)));
  const retried = new Map<string, BatchTargetOutcome>();
  for (const result of retry) {
    const k = key(result.skill_id, result.tool_key);
    if (askedKeys.has(k)) retried.set(k, result);
  }
  return report.map(
    (result) => retried.get(key(result.skill_id, result.tool_key)) ?? result,
  );
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
  // The overwrite ask is a nested ask on the surface stack: a newer ask
  // declines the one it displaces, and every ask settles exactly once.
  const requestOverwriteConfirmation = useCallback(
    async (rows: OverwriteRow[]): Promise<OverwriteAnswer> =>
      OVERWRITE_ANSWER[await ask({ kind: "overwrite", rows })],
    [ask],
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

  /**
   * The one sync fan-out seam, with the overwrite ask: a batch that settled
   * `TARGET_EXISTS` rows (a target occupied by *different* content — the
   * caller's same-content rule already replaced identical content silently)
   * raises one confirmation listing them. Answered, one more batch runs over
   * the affected skills × tools with a per-pair decision for each asked pair
   * — `overwrite` when confirmed, `keep` for the explicit "Keep existing"
   * (the backend records the pair `disabled`, so no later pass re-asks) — and
   * the same same-content rule, and the asked rows are replaced in the report.
   * The operator's answer is what authorises the second batch. A dismissed
   * ask (Escape, displaced, dropped) is no answer: no retry batch runs, no
   * decision is recorded, and the report is returned as it settled.
   */
  const syncSkillsToTools = useCallback(
    async (
      skills: BatchSyncSkillDto[],
      toolIds: string[],
      policy?: SyncPolicy,
    ): Promise<BatchTargetOutcome[]> => {
      const report = await invokeBatchSync(skills, toolIds, policy);
      const occupied = occupiedTargets(report, toolLabelById);
      if (occupied.length === 0) return report;

      setActionMessage(t("overwrite.waiting"));
      const decision = await requestOverwriteConfirmation(occupied);
      if (decision === "dismissed") return report;

      const askedSkillIds = new Set(occupied.map((row) => row.skillId));
      const askedToolIds = [...new Set(occupied.map((row) => row.toolKey))];
      const overrides: BatchSyncOverrideDto[] = occupied.map((row) => ({
        skill_id: row.skillId,
        tool: row.toolKey,
        decision,
      }));
      const retry = await invokeBatchSync(
        skills.filter((skill) => askedSkillIds.has(skill.skill_id)),
        askedToolIds,
        {
          overwrite: false,
          overwriteIfSameContent: policy?.overwriteIfSameContent ?? false,
          overrides,
        },
      );
      return mergeRetry(report, occupied, retry);
    },
    [
      invokeBatchSync,
      requestOverwriteConfirmation,
      setActionMessage,
      t,
      toolLabelById,
    ],
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
