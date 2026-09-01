import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { ManagedSkill } from "../components/skills/types";
import { invokeTauri, isTauri } from "../lib/tauri";
import type { SyncOrchestration } from "./useSyncOrchestration";
import type { StatusReporter, TranslateFn } from "./useStatusReporter";

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
    | "sharedToolIdsByToolId"
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
    sharedToolIdsByToolId,
    syncFailureEntries,
    syncSkillsToTools,
    toolLabelById,
    tools,
  } = sync;

  const [managedSkills, setManagedSkills] = useState<ManagedSkill[]>([]);
  const [pendingDeleteId, setPendingDeleteId] = useState<string | null>(null);
  const [pendingSharedToggle, setPendingSharedToggle] = useState<{
    skill: ManagedSkill;
    toolId: string;
  } | null>(null);

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

  const handleRefresh = useCallback(async () => {
    if (managedSkills.length === 0) return;

    await runAction({ successToast: t("status.refreshCompleted") }, async () => {
      const collectedErrors: { title: string; message: string }[] = [];

      for (let i = 0; i < managedSkills.length; i++) {
        const skill = managedSkills[i];
        setActionMessage(
          t("actions.refreshStep", {
            index: i + 1,
            total: managedSkills.length,
            name: skill.name,
          }),
        );
        try {
          await invokeTauri("updateManagedSkill", skill.id);
        } catch (err) {
          const raw = formatError(err) ?? "";
          collectedErrors.push({
            title: t("errors.updateFailedTitle", { name: skill.name }),
            message: raw,
          });
        }
      }

      if (autoSyncEnabled) {
        const freshSkills =
          await invokeTauri("getManagedSkills");
        if (installedToolIds.length > 0 && freshSkills.length > 0) {
          // Refresh means "push the updated content": without overwrite,
          // every target whose skill actually changed would fail with
          // TARGET_EXISTS — the one outcome refresh exists to avoid.
          const report = await syncSkillsToTools(
            freshSkills.map(toSyncItem),
            installedToolIds,
            { overwrite: true },
          );
          collectedErrors.push(...syncFailureEntries(report));
        }
      }

      await loadManagedSkills();
      if (collectedErrors.length > 0) showActionErrors(collectedErrors);
    });
  }, [
    autoSyncEnabled,
    formatError,
    installedToolIds,
    loadManagedSkills,
    managedSkills,
    runAction,
    setActionMessage,
    showActionErrors,
    syncFailureEntries,
    syncSkillsToTools,
    t,
  ]);

  const handleUnsyncAll = useCallback(async () => {
    await runAction(
      {
        successToast: (count: number) => t("unsyncAllComplete", { count }),
      },
      async () => {
        const count = await invokeTauri("unsyncAllSkills");
        await loadManagedSkills();
        return count;
      },
    );
  }, [loadManagedSkills, runAction, t]);

  const handleUnsyncSkill = useCallback(
    async (skillId: string) => {
      try {
        await invokeTauri("unsyncSkill", skillId);
        await loadManagedSkills();
      } catch (err) {
        const msg = formatError(err);
        if (msg) toast.error(msg);
      }
    },
    [formatError, loadManagedSkills],
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
            await invokeTauri("unsyncSkillFromTool", skill.id, toolId);
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
    (skill: ManagedSkill, toolId: string) => {
      if (loading) return;
      const shared = sharedToolIdsByToolId[toolId] ?? null;
      if (shared && shared.length > 1) {
        setPendingSharedToggle({ skill, toolId });
        return;
      }
      void runToggleToolForSkill(skill, toolId);
    },
    [loading, runToggleToolForSkill, sharedToolIdsByToolId],
  );

  const handleUpdateManaged = useCallback(
    async (skill: ManagedSkill) => {
      await runAction(
        {
          message: t("actions.updating", { name: skill.name }),
          successToast: t("status.updated", { name: skill.name }),
        },
        async () => {
          await invokeTauri("updateManagedSkill", skill.id);
          await loadManagedSkills();
        },
      );
    },
    [loadManagedSkills, runAction, t],
  );

  const handleUpdateSkill = useCallback(
    (skill: ManagedSkill) => {
      void handleUpdateManaged(skill);
    },
    [handleUpdateManaged],
  );

  const handleSharedCancel = useCallback(() => {
    if (loading) return;
    setPendingSharedToggle(null);
  }, [loading]);

  const handleSharedConfirm = useCallback(() => {
    if (!pendingSharedToggle) return;
    const payload = pendingSharedToggle;
    setPendingSharedToggle(null);
    void runToggleToolForSkill(payload.skill, payload.toolId);
  }, [pendingSharedToggle, runToggleToolForSkill]);

  const pendingSharedLabels = useMemo(() => {
    if (!pendingSharedToggle) return null;
    const toolId = pendingSharedToggle.toolId;
    const shared = sharedToolIdsByToolId[toolId] ?? [];
    const others = shared.filter((id) => id !== toolId);
    return {
      toolLabel: toolLabelById[toolId] ?? toolId,
      otherLabels: others.map((id) => toolLabelById[id] ?? id).join(", "),
    };
  }, [pendingSharedToggle, sharedToolIdsByToolId, toolLabelById]);

  return {
    managedSkills,
    pendingDeleteId,
    pendingDeleteSkill,
    pendingSharedToggle,
    pendingSharedLabels,
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
    handleSharedCancel,
    handleSharedConfirm,
  };
}

export type SkillLibrary = ReturnType<typeof useSkillLibrary>;
