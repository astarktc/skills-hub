import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type {
  BatchSyncOverrideDto,
  BatchSyncReportDto,
  BatchSyncSkillDto,
  AppSettings,
  SyncProgressDto,
  ToolOption,
  ToolStatusDto,
} from "../components/skills/types";
import { invokeTauri, isTauri } from "../lib/tauri";
import type {
  ActionErrorEntry,
  StatusReporter,
  TranslateFn,
} from "./useStatusReporter";

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

export type SyncOrchestrationDeps = {
  t: TranslateFn;
  reporter: Pick<
    StatusReporter,
    "loading" | "setActionMessage" | "setError" | "formatError"
  >;
};

/**
 * Tools + sync world: tool detection status, the saved global tool selection,
 * deploy targets, the auto-sync switch, and the one sync fan-out seam. The
 * backend owns the sync choreography (installedness filtering, shared-dir
 * dedupe, overwrite policy, DB record fan-out) and streams per-pair progress
 * back over a channel. Nothing else may loop a per-pair sync command —
 * sync_skill_to_tool no longer exists.
 */
export function useSyncOrchestration({ t, reporter }: SyncOrchestrationDeps) {
  const { loading, setActionMessage, setError, formatError } = reporter;
  const [toolStatus, setToolStatus] = useState<ToolStatusDto | null>(null);
  const [showNewToolsModal, setShowNewToolsModal] = useState(false);
  const [showToolConfigModal, setShowToolConfigModal] = useState(false);
  const [globalSelectedTools, setGlobalSelectedTools] = useState<
    string[] | null
  >(null);
  const [scanSelectedToolsOnly, setScanSelectedToolsOnly] = useState(true);
  const [syncTargets, setSyncTargets] = useState<Record<string, boolean>>({});
  const [autoSyncEnabled, setAutoSyncEnabled] = useState(true);

  const toolInfos = useMemo(() => toolStatus?.tools ?? [], [toolStatus]);

  const tools: ToolOption[] = useMemo(() => {
    return toolInfos.map((info) => ({
      id: info.key,
      // Prefer i18n label if present; fallback to backend label.
      label: t(`tools.${info.key}`, { defaultValue: info.label }),
    }));
  }, [t, toolInfos]);

  const toolLabelById = useMemo(() => {
    const out: Record<string, string> = {};
    for (const tool of tools) out[tool.id] = tool.label;
    return out;
  }, [tools]);

  const sharedToolIdsByToolId = useMemo(() => {
    // toolId -> all toolIds sharing the same skills dir. The backend owns
    // the grouping (ToolInfoDto.shared_with); this map only feeds the
    // shared-dir confirmation UX.
    const out: Record<string, string[]> = {};
    for (const info of toolInfos) {
      if (info.shared_with.length > 1) out[info.key] = info.shared_with;
    }
    return out;
  }, [toolInfos]);

  const installedToolIds = useMemo(
    () => toolStatus?.installed ?? [],
    [toolStatus],
  );
  const isInstalled = useCallback(
    (id: string) => installedToolIds.includes(id),
    [installedToolIds],
  );
  const installedTools = useMemo(
    () => tools.filter((tool) => installedToolIds.includes(tool.id)),
    [tools, installedToolIds],
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
    const load = async () => {
      if (!isTauri) return;
      // Load the settings snapshot first so the new-tools popup and sync
      // target defaults respect the saved selection.
      let selectedTools: string[] | null = null;
      let scanSelectedOnly = true;
      try {
        const settings = await invokeTauri<AppSettings>("get_settings");
        selectedTools = settings.global_selected_tools;
        scanSelectedOnly = settings.scan_selected_tools_only;
        setAutoSyncEnabled(settings.auto_sync_enabled);
        setGlobalSelectedTools(selectedTools);
        setScanSelectedToolsOnly(scanSelectedOnly);
      } catch (err) {
        // Non-fatal; fall back to defaults.
        console.warn(err);
      }
      try {
        const status = await invokeTauri<ToolStatusDto>("get_tool_status");
        setToolStatus(status);

        // Default sync targets: saved global selection if configured,
        // otherwise installed tools (if user hasn't toggled yet).
        setSyncTargets((prev) => {
          if (Object.keys(prev).length > 0) return prev;
          const next: Record<string, boolean> = {};
          for (const t of status.tools) {
            next[t.key] = selectedTools
              ? selectedTools.includes(t.key)
              : status.installed.includes(t.key);
          }
          return next;
        });

        const relevantNew = filterRelevantNewlyInstalled(
          status.newly_installed,
          scanSelectedOnly,
          selectedTools,
        );
        if (relevantNew.length > 0) {
          setShowNewToolsModal(true);
        }
      } catch (err) {
        // Non-fatal; app can still work without detection.
        console.warn(err);
      }
    };
    void load();
  }, []);

  const syncSkillsToTools = useCallback(
    async (
      skills: BatchSyncSkillDto[],
      toolIds: string[],
      policy?: SyncPolicy,
    ): Promise<BatchSyncReportDto> => {
      const { Channel } = await import("@tauri-apps/api/core");
      const onProgress = new Channel<SyncProgressDto>();
      onProgress.onmessage = (progress) => {
        setActionMessage(
          t("actions.syncStep", {
            index: progress.index,
            total: progress.total,
            name: progress.skill_name,
            tool: toolLabelById[progress.tool] ?? progress.tool,
          }),
        );
      };
      return invokeTauri<BatchSyncReportDto>("sync_skills_to_tools", {
        skills,
        tools: toolIds,
        policy: {
          overwrite: policy?.overwrite ?? false,
          overwrite_if_same_content: policy?.overwriteIfSameContent ?? false,
          overrides: policy?.overrides ?? [],
        },
        onProgress,
      });
    },
    [setActionMessage, t, toolLabelById],
  );

  // Failed targets as showActionErrors entries. Skips (tool absent, dir
  // unwritable) stay silent by default — bulk flows ignore them — but flows
  // that target user-selected tools surface not-writable skips.
  const syncFailureEntries = useCallback(
    (
      report: BatchSyncReportDto,
      opts?: { includeNotWritableSkips?: boolean },
    ) => {
      const entries: ActionErrorEntry[] = [];
      for (const result of report.results) {
        const status = result.status;
        if (status.status === "synced") continue;
        const surface =
          status.status === "failed" ||
          ((opts?.includeNotWritableSkips ?? false) &&
            status.error.code === "TOOL_NOT_WRITABLE");
        if (!surface) continue;
        entries.push({
          title: t("errors.syncFailedTitle", {
            name: result.skill_name,
            tool: toolLabelById[result.tool] ?? result.tool,
          }),
          message: formatError(status.error) ?? "",
        });
      }
      return entries;
    },
    [formatError, t, toolLabelById],
  );

  const handleAutoSyncToggle = useCallback(
    async (enabled: boolean) => {
      try {
        await invokeTauri("update_setting", {
          update: { key: "auto_sync_enabled", value: enabled },
        });
        setAutoSyncEnabled(enabled);
      } catch (err) {
        const msg = formatError(err);
        if (msg) toast.error(msg);
      }
    },
    [formatError],
  );

  const handleOpenToolConfig = useCallback(() => {
    setShowToolConfigModal(true);
  }, []);

  const handleCloseToolConfig = useCallback(() => {
    if (!loading) setShowToolConfigModal(false);
  }, [loading]);

  const handleCloseNewTools = useCallback(() => {
    if (!loading) setShowNewToolsModal(false);
  }, [loading]);

  const handleToolConfigConfirm = useCallback(
    async (selected: string[], scanOnly = false) => {
      try {
        await invokeTauri("update_setting", {
          update: {
            key: "global_tool_config",
            value: { selected_tools: selected, scan_selected_only: scanOnly },
          },
        });
        setGlobalSelectedTools(selected);
        setScanSelectedToolsOnly(scanOnly);
        // Deploy targets follow the saved selection exactly.
        setSyncTargets(() => {
          const next: Record<string, boolean> = {};
          for (const info of toolInfos) {
            next[info.key] = selected.includes(info.key);
          }
          return next;
        });
        setShowToolConfigModal(false);
        toast.success(t("status.toolConfigSaved"));
      } catch (err) {
        setError(formatError(err));
      }
    },
    [formatError, setError, t, toolInfos],
  );

  const handleSyncTargetChange = useCallback(
    (toolId: string, checked: boolean) => {
      const shared = sharedToolIdsByToolId[toolId] ?? [toolId];
      if (shared.length > 1) {
        const others = shared.filter((id) => id !== toolId);
        const otherLabels = others
          .map((id) => toolLabelById[id] ?? id)
          .join(", ");
        const ok = window.confirm(
          t("sharedDirConfirm", {
            tool: toolLabelById[toolId] ?? toolId,
            others: otherLabels,
          }),
        );
        if (!ok) return;
      }
      setSyncTargets((prev) => {
        const next = { ...prev };
        for (const id of shared) next[id] = checked;
        return next;
      });
    },
    [sharedToolIdsByToolId, t, toolLabelById],
  );

  /** Enable deploy targets for the given tools, expanded to shared-dir groups. */
  const enableTargetsFor = useCallback(
    (toolIds: string[]) => {
      setSyncTargets((prev) => {
        const next = { ...prev };
        for (const id of toolIds) {
          const shared = sharedToolIdsByToolId[id] ?? [id];
          for (const sid of shared) next[sid] = true;
        }
        return next;
      });
    },
    [sharedToolIdsByToolId],
  );

  /** Reset deploy targets to exactly the currently installed tools. */
  const targetAllInstalled = useCallback(() => {
    if (!toolStatus) return;
    const targets: Record<string, boolean> = {};
    for (const id of toolStatus.installed) {
      targets[id] = true;
    }
    setSyncTargets(targets);
  }, [toolStatus]);

  return {
    toolStatus,
    tools,
    toolLabelById,
    sharedToolIdsByToolId,
    installedToolIds,
    isInstalled,
    installedTools,
    globalSelectedTools,
    scanSelectedToolsOnly,
    syncTargets,
    autoSyncEnabled,
    showNewToolsModal,
    setShowNewToolsModal,
    showToolConfigModal,
    relevantNewlyInstalled,
    newlyInstalledToolsText,
    syncSkillsToTools,
    syncFailureEntries,
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
