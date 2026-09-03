import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  ManagedSkill,
  RefreshProgressDto,
  RefreshReportDto,
  RemovalReportDto,
} from "../components/skills/types";
import { invokeTauri, isTauri } from "../lib/tauri";
import type { SyncOrchestration } from "./useSyncOrchestration";
import type {
  ActionErrorEntry,
  StatusReporter,
  TranslateFn,
} from "./useStatusReporter";

/** The `{skill_id, name, source_path}` batch item for a managed skill. */
const toSyncItem = (skill: ManagedSkill) => ({
  skill_id: skill.id,
  name: skill.name,
  source_path: skill.central_path,
});

export type SkillLibraryDeps = {
  t: TranslateFn;
  reporter: StatusReporter;
  sync: Pick<
    SyncOrchestration,
    | "autoSyncEnabled"
    | "installedToolIds"
    | "requestSharedDirConfirmation"
    | "syncFailureEntries"
    | "syncSkillsToTools"
    | "toolLabelById"
    | "tools"
  >;
};

/**
 * Skill library world: the managed-skill list plus every per-skill and bulk
 * action on it (refresh/update, delete, per-tool sync toggles including the
 * shared-dir confirmation, unsync). Sync fan-out goes through the sync
 * world's seam, received as a dependency.
 *
 * Update and Refresh (all) are one backend batch each
 * (`refreshManagedSkills`): the backend acquires, finalizes, propagates and
 * — with `reassert_auto_sync` — re-asserts the auto-sync invariant, then
 * hands back one report. This hook renders that report; it never loops a
 * per-skill command and never fans out a sync of its own for a refresh.
 */
export function useSkillLibrary({ t, reporter, sync }: SkillLibraryDeps) {
  const {
    loading,
    runAction,
    setActionMessage,
    setError,
    formatError,
    showActionErrors,
  } = reporter;
  const {
    autoSyncEnabled,
    installedToolIds,
    requestSharedDirConfirmation,
    syncFailureEntries,
    syncSkillsToTools,
    toolLabelById,
    tools,
  } = sync;

  const [managedSkills, setManagedSkills] = useState<ManagedSkill[]>([]);
  const [pendingDeleteId, setPendingDeleteId] = useState<string | null>(null);

  const loadManagedSkills = useCallback(async () => {
    try {
      const result = await invokeTauri("getManagedSkills");
      setManagedSkills(result);
    } catch (err) {
      setError(formatError(err));
    }
  }, [formatError, setError]);

  useEffect(() => {
    if (!isTauri) return;
    // Fire-and-forget load on mount. Awaited inside an IIFE so the loader's
    // setState runs in an async continuation rather than synchronously in the
    // effect body (satisfies react-hooks/set-state-in-effect). Behavior is
    // unchanged: loadManagedSkills only setStates after its await.
    void (async () => {
      await loadManagedSkills();
    })();
  }, [loadManagedSkills]);

  const isSkillNameTaken = useCallback(
    (name: string) =>
      managedSkills.some(
        (skill) => skill.name.toLowerCase() === name.toLowerCase(),
      ),
    [managedSkills],
  );

  const pendingDeleteSkill = useMemo(
    () => managedSkills.find((skill) => skill.id === pendingDeleteId) ?? null,
    [managedSkills, pendingDeleteId],
  );

  /**
   * The one Refresh invoke: `skillIds === null` refreshes every Managed
   * skill. Progress ticks come from the backend, one per phase step.
   */
  const refreshSkills = useCallback(
    async (skillIds: string[] | null): Promise<RefreshReportDto> => {
      const { Channel } = await import("@tauri-apps/api/core");
      const onProgress = new Channel<RefreshProgressDto>();
      onProgress.onmessage = (progress) => {
        setActionMessage(
          t(
            progress.phase === "acquiring"
              ? "actions.refreshFetchStep"
              : "actions.refreshApplyStep",
            {
              index: progress.index,
              total: progress.total,
              name: progress.skill_name,
            },
          ),
        );
      };
      return invokeTauri(
        "refreshManagedSkills",
        skillIds,
        { reassert_auto_sync: autoSyncEnabled },
        onProgress,
      );
    },
    [autoSyncEnabled, setActionMessage, t],
  );

  /** Skills whose bytes could not be acquired (their targets were untouched). */
  const skillFailureEntries = useCallback(
    (report: RefreshReportDto) => {
      const entries: ActionErrorEntry[] = [];
      for (const skill of report.skills) {
        if (skill.status.status !== "failed") continue;
        entries.push({
          title: t("errors.updateFailedTitle", { name: skill.skill_name }),
          message: formatError(skill.status.error) ?? "",
        });
      }
      return entries;
    },
    [formatError, t],
  );

  /**
   * Sync targets Propagation could not bring into line, plus a failed
   * auto-sync re-assert (its targets never got planned at all). Skips stay
   * silent.
   */
  const targetFailureEntries = useCallback(
    (report: RefreshReportDto) => {
      const entries: ActionErrorEntry[] = [];
      for (const skill of report.skills) {
        if (skill.status.status !== "refreshed") continue;
        for (const target of skill.status.targets) {
          if (target.status.status !== "failed") continue;
          const tool = target.scope.tool;
          entries.push({
            title: t("errors.propagationFailedTitle", {
              name: skill.skill_name,
              tool: toolLabelById[tool] ?? tool,
            }),
            message: formatError(target.status.error) ?? "",
          });
        }
        if (skill.status.reassert_error) {
          entries.push({
            title: t("errors.reassertFailedTitle", { name: skill.skill_name }),
            message: formatError(skill.status.reassert_error) ?? "",
          });
        }
      }
      return entries;
    },
    [formatError, t, toolLabelById],
  );

  const handleRefresh = useCallback(async () => {
    if (managedSkills.length === 0) return;

    await runAction<RefreshReportDto>(
      {
        successToast: (report) =>
          report.failed === 0
            ? t("status.refreshCompleted")
            : t("status.refreshSummary", {
                refreshed: report.refreshed,
                failed: report.failed,
              }),
      },
      async () => {
        const report = await refreshSkills(null);
        await loadManagedSkills();
        showActionErrors([
          ...skillFailureEntries(report),
          ...targetFailureEntries(report),
        ]);
        return report;
      },
    );
  }, [
    loadManagedSkills,
    managedSkills,
    refreshSkills,
    runAction,
    showActionErrors,
    skillFailureEntries,
    t,
    targetFailureEntries,
  ]);

  /**
   * Artifacts the backend could not remove. Their rows were kept with sync
   * status `error` (ADR-0002), so the failure stays visible in the list too.
   */
  const removalFailureEntries = useCallback(
    (report: RemovalReportDto) => {
      const entries: ActionErrorEntry[] = [];
      for (const target of report.targets) {
        if (target.status.status !== "failed") continue;
        entries.push({
          title: t("errors.unsyncFailedTitle", {
            tool: toolLabelById[target.tool] ?? target.tool,
          }),
          message: formatError(target.status.error) ?? "",
        });
      }
      return entries;
    },
    [formatError, t, toolLabelById],
  );

  const removalToast = useCallback(
    (report: RemovalReportDto) =>
      report.failed === 0
        ? t("unsyncAllComplete", { count: report.removed })
        : t("unsyncPartial", {
            count: report.removed,
            failed: report.failed,
          }),
    [t],
  );

  const handleUnsyncAll = useCallback(async () => {
    await runAction({ successToast: removalToast }, async () => {
      const report = await invokeTauri("unsyncAllSkills");
      await loadManagedSkills();
      showActionErrors(removalFailureEntries(report));
      return report;
    });
  }, [
    loadManagedSkills,
    removalFailureEntries,
    removalToast,
    runAction,
    showActionErrors,
  ]);

  const handleUnsyncSkill = useCallback(
    async (skillId: string) => {
      try {
        const report = await invokeTauri("unsyncSkill", skillId);
        await loadManagedSkills();
        showActionErrors(removalFailureEntries(report));
      } catch (err) {
        setError(formatError(err));
      }
    },
    [
      formatError,
      loadManagedSkills,
      removalFailureEntries,
      setError,
      showActionErrors,
    ],
  );

  const handleSyncSkillToAllTools = useCallback(
    async (skill: ManagedSkill) => {
      if (installedToolIds.length === 0) return;

      await runAction({ successToast: t("status.syncCompleted") }, async () => {
        const report = await syncSkillsToTools(
          [toSyncItem(skill)],
          installedToolIds,
        );
        showActionErrors(syncFailureEntries(report));
        await loadManagedSkills();
      });
    },
    [
      installedToolIds,
      loadManagedSkills,
      runAction,
      showActionErrors,
      syncFailureEntries,
      syncSkillsToTools,
      t,
    ],
  );

  const syncAllManagedToTools = useCallback(
    async (toolIds: string[]) => {
      if (!autoSyncEnabled) return;
      if (managedSkills.length === 0) return;
      if (toolIds.length === 0) return;

      await runAction({ successToast: t("status.syncCompleted") }, async () => {
        const report = await syncSkillsToTools(
          managedSkills.map(toSyncItem),
          toolIds,
          { overwriteIfSameContent: true },
        );
        const collectedErrors = syncFailureEntries(report);
        await loadManagedSkills();
        if (collectedErrors.length > 0) showActionErrors(collectedErrors);
      });
    },
    [
      autoSyncEnabled,
      loadManagedSkills,
      managedSkills,
      runAction,
      showActionErrors,
      syncFailureEntries,
      syncSkillsToTools,
      t,
    ],
  );

  const handleDeleteManaged = useCallback(
    async (skill: ManagedSkill) => {
      await runAction(
        {
          message: t("actions.removing", { name: skill.name }),
          successToast: t("status.skillRemoved"),
        },
        async () => {
          await invokeTauri("deleteManagedSkill", skill.id);
          await loadManagedSkills();
          setPendingDeleteId(null);
        },
      );
    },
    [loadManagedSkills, runAction, t],
  );

  const handleDeletePrompt = useCallback((skillId: string) => {
    setPendingDeleteId(skillId);
  }, []);

  const handleCloseDelete = useCallback(() => {
    if (!loading) setPendingDeleteId(null);
  }, [loading]);

  const runToggleToolForSkill = useCallback(
    async (skill: ManagedSkill, toolId: string) => {
      if (loading) return;
      const toolLabel = tools.find((t) => t.id === toolId)?.label ?? toolId;
      const target = skill.targets.find((t) => t.tool === toolId);
      const synced = Boolean(target);

      await runAction(
        {
          message: synced
            ? t("actions.unsyncing", { name: skill.name, tool: toolLabel })
            : t("actions.syncing", { name: skill.name, tool: toolLabel }),
          successToast: synced
            ? t("status.syncDisabled")
            : t("status.syncEnabled"),
        },
        async (action) => {
          if (synced) {
            const report = await invokeTauri(
              "unsyncSkillFromTool",
              skill.id,
              toolId,
            );
            const failed = report.targets.find(
              (target) => target.status.status === "failed",
            );
            if (failed && failed.status.status === "failed") {
              // An explicit single toggle surfaces the failure instead of
              // reporting success over an artifact that is still on disk.
              return action.fail(formatError(failed.status.error));
            }
          } else {
            const report = await syncSkillsToTools(
              [toSyncItem(skill)],
              [toolId],
              { overwriteIfSameContent: true },
            );
            const status = report.results[0]?.status;
            if (status && status.status !== "synced") {
              // An explicit single toggle surfaces every non-success,
              // including skips a bulk flow would ignore.
              return action.fail(
                status.error.code === "TARGET_EXISTS"
                  ? t("errors.targetExistsDetail", { path: status.error.path })
                  : formatError(status.error),
              );
            }
          }
          await loadManagedSkills();
        },
      );
    },
    [
      formatError,
      loading,
      loadManagedSkills,
      runAction,
      syncSkillsToTools,
      t,
      tools,
    ],
  );

  const handleToggleToolForSkill = useCallback(
    async (skill: ManagedSkill, toolId: string) => {
      if (loading) return;
      // A tool sharing its skills dir affects the whole group: the same
      // confirmation both flows use decides before anything is written.
      const confirmed = await requestSharedDirConfirmation(toolId);
      if (!confirmed) return;
      await runToggleToolForSkill(skill, toolId);
    },
    [loading, requestSharedDirConfirmation, runToggleToolForSkill],
  );

  const handleUpdateManaged = useCallback(
    async (skill: ManagedSkill) => {
      await runAction(
        {
          message: t("actions.updating", { name: skill.name }),
          successToast: t("status.updated", { name: skill.name }),
        },
        async (action) => {
          // A single Update is the same batch, of one.
          const report = await refreshSkills([skill.id]);
          await loadManagedSkills();
          const failed = report.skills.find(
            (entry) => entry.status.status === "failed",
          );
          if (failed && failed.status.status === "failed") {
            return action.fail(formatError(failed.status.error));
          }
          showActionErrors(targetFailureEntries(report));
        },
      );
    },
    [
      formatError,
      loadManagedSkills,
      refreshSkills,
      runAction,
      showActionErrors,
      t,
      targetFailureEntries,
    ],
  );

  const handleUpdateSkill = useCallback(
    (skill: ManagedSkill) => {
      void handleUpdateManaged(skill);
    },
    [handleUpdateManaged],
  );

  return {
    managedSkills,
    pendingDeleteId,
    pendingDeleteSkill,
    loadManagedSkills,
    isSkillNameTaken,
    handleRefresh,
    handleUnsyncAll,
    handleUnsyncSkill,
    handleSyncSkillToAllTools,
    syncAllManagedToTools,
    handleDeleteManaged,
    handleDeletePrompt,
    handleCloseDelete,
    handleToggleToolForSkill,
    handleUpdateSkill,
  };
}

export type SkillLibrary = ReturnType<typeof useSkillLibrary>;
