import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type {
  ManagedSkill,
  UpdateResultDto,
} from "../components/skills/types";
import { invokeTauri, isTauri } from "../lib/tauri";
import type { SyncOrchestration } from "./useSyncOrchestration";
import type { StatusReporter, TranslateFn } from "./useStatusReporter";

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
    setLoading,
    setLoadingStartAt,
    setActionMessage,
    setError,
    setSuccessToastMessage,
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
      const result = await invokeTauri<ManagedSkill[]>("get_managed_skills");
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

    setLoading(true);
    setLoadingStartAt(Date.now());
    setError(null);

    try {
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
          await invokeTauri<UpdateResultDto>("update_managed_skill", {
            skillId: skill.id,
          });
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
          await invokeTauri<ManagedSkill[]>("get_managed_skills");
        if (installedToolIds.length > 0 && freshSkills.length > 0) {
          // Refresh means "push the updated content": without overwrite,
          // every target whose skill actually changed would fail with
          // TARGET_EXISTS — the one outcome refresh exists to avoid.
          const report = await syncSkillsToTools(
            freshSkills.map((skill) => ({
              skill_id: skill.id,
              name: skill.name,
              source_path: skill.central_path,
            })),
            installedToolIds,
            { overwrite: true },
          );
          collectedErrors.push(...syncFailureEntries(report));
        }
      }

      setActionMessage(t("status.refreshCompleted"));
      setSuccessToastMessage(t("status.refreshCompleted"));
      setActionMessage(null);
      await loadManagedSkills();
      if (collectedErrors.length > 0) showActionErrors(collectedErrors);
    } finally {
      setLoading(false);
      setLoadingStartAt(null);
    }
  }, [
    autoSyncEnabled,
    formatError,
    installedToolIds,
    loadManagedSkills,
    managedSkills,
    setActionMessage,
    setError,
    setLoading,
    setLoadingStartAt,
    setSuccessToastMessage,
    showActionErrors,
    syncFailureEntries,
    syncSkillsToTools,
    t,
  ]);

  const handleUnsyncAll = useCallback(async () => {
    setLoading(true);
    setLoadingStartAt(Date.now());
    try {
      const count = await invokeTauri<number>("unsync_all_skills");
      setSuccessToastMessage(t("unsyncAllComplete", { count }));
      await loadManagedSkills();
    } catch (err) {
      setError(formatError(err));
    } finally {
      setLoading(false);
      setLoadingStartAt(null);
    }
  }, [
    formatError,
    loadManagedSkills,
    setError,
    setLoading,
    setLoadingStartAt,
    setSuccessToastMessage,
    t,
  ]);

  const handleUnsyncSkill = useCallback(
    async (skillId: string) => {
      try {
        await invokeTauri("unsync_skill", { skillId });
        await loadManagedSkills();
      } catch (err) {
        {
          const msg = formatError(err);
          if (msg) toast.error(msg);
        }
      }
    },
    [formatError, loadManagedSkills],
  );

  const handleSyncSkillToAllTools = useCallback(
    async (skill: ManagedSkill) => {
      if (installedToolIds.length === 0) return;

      setLoading(true);
      setLoadingStartAt(Date.now());
      setError(null);
      try {
        const report = await syncSkillsToTools(
          [
            {
              skill_id: skill.id,
              name: skill.name,
              source_path: skill.central_path,
            },
          ],
          installedToolIds,
        );
        setActionMessage(null);
        showActionErrors(syncFailureEntries(report));
        toast.success(t("status.syncCompleted"));
        await loadManagedSkills();
      } finally {
        setLoading(false);
        setLoadingStartAt(null);
      }
    },
    [
      installedToolIds,
      loadManagedSkills,
      setActionMessage,
      setError,
      setLoading,
      setLoadingStartAt,
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

      setLoading(true);
      setLoadingStartAt(Date.now());
      setError(null);
      try {
        const report = await syncSkillsToTools(
          managedSkills.map((skill) => ({
            skill_id: skill.id,
            name: skill.name,
            source_path: skill.central_path,
          })),
          toolIds,
          { overwriteIfSameContent: true },
        );
        const collectedErrors = syncFailureEntries(report);
        setActionMessage(t("status.syncCompleted"));
        setSuccessToastMessage(t("status.syncCompleted"));
        setActionMessage(null);
        await loadManagedSkills();
        if (collectedErrors.length > 0) showActionErrors(collectedErrors);
      } finally {
        setLoading(false);
        setLoadingStartAt(null);
      }
    },
    [
      autoSyncEnabled,
      loadManagedSkills,
      managedSkills,
      setActionMessage,
      setError,
      setLoading,
      setLoadingStartAt,
      setSuccessToastMessage,
      showActionErrors,
      syncFailureEntries,
      syncSkillsToTools,
      t,
    ],
  );

  const handleDeleteManaged = useCallback(
    async (skill: ManagedSkill) => {
      setLoading(true);
      setLoadingStartAt(Date.now());
      setActionMessage(t("actions.removing", { name: skill.name }));
      setError(null);
      try {
        await invokeTauri("delete_managed_skill", { skillId: skill.id });
        setActionMessage(t("status.skillRemoved"));
        setSuccessToastMessage(t("status.skillRemoved"));
        setActionMessage(null);
        await loadManagedSkills();
        setPendingDeleteId(null);
      } catch (err) {
        setError(formatError(err));
      } finally {
        setLoading(false);
        setLoadingStartAt(null);
      }
    },
    [
      formatError,
      loadManagedSkills,
      setActionMessage,
      setError,
      setLoading,
      setLoadingStartAt,
      setSuccessToastMessage,
      t,
    ],
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

      setLoading(true);
      setLoadingStartAt(Date.now());
      setError(null);
      try {
        if (synced) {
          setActionMessage(
            t("actions.unsyncing", { name: skill.name, tool: toolLabel }),
          );
          await invokeTauri("unsync_skill_from_tool", {
            skillId: skill.id,
            tool: toolId,
          });
        } else {
          setActionMessage(
            t("actions.syncing", { name: skill.name, tool: toolLabel }),
          );
          const report = await syncSkillsToTools(
            [
              {
                skill_id: skill.id,
                name: skill.name,
                source_path: skill.central_path,
              },
            ],
            [toolId],
            { overwriteIfSameContent: true },
          );
          const status = report.results[0]?.status;
          if (status && status.status !== "synced") {
            // An explicit single toggle surfaces every non-success,
            // including skips a bulk flow would ignore.
            setActionMessage(null);
            if (status.error.code === "TARGET_EXISTS") {
              setError(
                t("errors.targetExistsDetail", { path: status.error.path }),
              );
            } else {
              setError(formatError(status.error));
            }
            return;
          }
        }
        const statusText = synced
          ? t("status.syncDisabled")
          : t("status.syncEnabled");
        setActionMessage(statusText);
        setSuccessToastMessage(statusText);
        setActionMessage(null);
        await loadManagedSkills();
      } catch (err) {
        setError(formatError(err));
      } finally {
        setLoading(false);
        setLoadingStartAt(null);
      }
    },
    [
      formatError,
      loading,
      loadManagedSkills,
      setActionMessage,
      setError,
      setLoading,
      setLoadingStartAt,
      setSuccessToastMessage,
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
      setLoading(true);
      setLoadingStartAt(Date.now());
      setError(null);
      try {
        setActionMessage(t("actions.updating", { name: skill.name }));
        await invokeTauri<UpdateResultDto>("update_managed_skill", {
          skillId: skill.id,
        });
        const updatedText = t("status.updated", { name: skill.name });
        setActionMessage(updatedText);
        setSuccessToastMessage(updatedText);
        setActionMessage(null);
        await loadManagedSkills();
      } catch (err) {
        setError(formatError(err));
      } finally {
        setLoading(false);
        setLoadingStartAt(null);
      }
    },
    [
      formatError,
      loadManagedSkills,
      setActionMessage,
      setError,
      setLoading,
      setLoadingStartAt,
      setSuccessToastMessage,
      t,
    ],
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
