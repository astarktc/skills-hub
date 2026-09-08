import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  ManagedSkill,
  InvocationMode,
  RefreshProgressDto,
  RefreshReportDto,
} from "../components/skills/types";
import {
  deleteOutcome,
  refreshOutcome,
  removalOutcome,
  syncOutcome,
  type Outcome,
} from "../lib/reportOutcome";
import { invokeTauri, isTauri } from "../lib/tauri";
import { sourceKind } from "../lib/skillPresentation";
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
    | "effectiveSyncTargetIds"
    | "requestSharedDirConfirmation"
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
    notify,
    runAction,
    setActionMessage,
    setError,
    formatError,
    showActionErrors,
    showActionWarnings,
  } = reporter;
  const {
    autoSyncEnabled,
    effectiveSyncTargetIds,
    requestSharedDirConfirmation,
    syncSkillsToTools,
    toolLabelById,
    tools,
  } = sync;

  const [managedSkills, setManagedSkills] = useState<ManagedSkill[]>([]);
  const [detailSkillId, setDetailSkillId] = useState<string | null>(null);
  const detailSkill = managedSkills.find((skill) => skill.id === detailSkillId) ?? null;
  const openDetail = useCallback((id: string) => setDetailSkillId(id), []);
  const closeDetail = useCallback(() => setDetailSkillId(null), []);
  const [invocationEditSkillId, setInvocationEditSkillId] = useState<string | null>(null);
  const invocationEditSkill = managedSkills.find((skill) => skill.id === invocationEditSkillId) ?? null;
  const openInvocationEdit = useCallback((id: string) => setInvocationEditSkillId(id), []);
  const closeInvocationEdit = useCallback(() => { if (!loading) setInvocationEditSkillId(null); }, [loading]);
  const setInvocationOverride = useCallback(async (skillId: string, mode: InvocationMode | null) => {
    await runAction({ successToast: t("invocationEdit.saved") }, async () => {
      const { entry: updated } = await invokeTauri("setSkillInvocationOverride", skillId, mode);
      setManagedSkills((skills) => skills.map((skill) => skill.id === updated.id ? updated : skill));
      setInvocationEditSkillId(null);
    });
  }, [runAction, t]);
  const [pendingDeleteId, setPendingDeleteId] = useState<string | null>(null);
  const [gitRepointSelection, setGitRepointSelection] =
    useState<{ skillId: string; name: string } | null>(null);
  const pendingGitRepointSkill = managedSkills.find(
    (skill) => skill.id === gitRepointSelection?.skillId,
  ) ?? null;

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

  const handleRepointGitSkill = useCallback((skill: ManagedSkill) => {
    setGitRepointSelection({ skillId: skill.id, name: skill.name });
  }, []);

  // Both the modal and historical notification actions select by id. A click
  // resolves against this render's list, not the list that produced the report.
  const repointSkillGone = gitRepointSelection !== null && pendingGitRepointSkill === null;
  useEffect(() => {
    if (repointSkillGone && gitRepointSelection) {
      notify("warning", t("errors.skillGone", { name: gitRepointSelection.name }));
    }
  }, [gitRepointSelection, notify, repointSkillGone, t]);

  const foldContext = useMemo(() => ({
    t, toolLabelById,
    canRepoint: (id: string) => managedSkills.some((skill) => skill.id === id && sourceKind(skill) === "git"),
  }), [managedSkills, t, toolLabelById]);

  const applyOutcome = useCallback(async (outcome: Outcome) => {
    const entries = (items: Outcome["errors"]): ActionErrorEntry[] => items.map(({ action, ...entry }) => ({
      ...entry,
      ...(action ? { action: { label: action.label, onClick: () => setGitRepointSelection({ skillId: action.skillId, name: action.skillName }) } } : {}),
    }));
    if (outcome.completion.reload) await loadManagedSkills();
    showActionErrors(entries(outcome.errors));
    showActionWarnings(entries(outcome.warnings));
    if (outcome.toast) notify(outcome.toast.kind, outcome.toast.message, outcome.toast.detail);
    return outcome.completion;
  }, [loadManagedSkills, notify, showActionErrors, showActionWarnings]);

  const handleRefresh = useCallback(async () => {
    if (managedSkills.length === 0) return;
    await runAction({}, async () => {
      const report = await refreshSkills(null);
      return applyOutcome(refreshOutcome(report, foldContext));
    });
  }, [applyOutcome, foldContext, managedSkills.length, refreshSkills, runAction]);

  const handleUnsyncAll = useCallback(async () => {
    await runAction({}, async () => {
      const report = await invokeTauri("unsyncAllSkills");
      return applyOutcome(removalOutcome(report, { ...foldContext, action: "all" }));
    });
  }, [applyOutcome, foldContext, runAction]);

  const handleUnsyncSkill = useCallback(async (skillId: string) => {
    try {
      const report = await invokeTauri("unsyncSkill", skillId);
      await applyOutcome(removalOutcome(report, { ...foldContext, action: "skill" }));
    } catch (err) {
      setError(formatError(err));
    }
  }, [applyOutcome, foldContext, formatError, setError]);

  // The link button deploys to the operator's effective target set (their
  // recorded selection, or detection when they never configured one) — never
  // to every detected tool.
  const handleSyncSkillToAllTools = useCallback(async (skill: ManagedSkill) => {
    // Zero work is reported, never silent: an operator who selected no tools
    // clicked a deploy button and must learn why nothing happened.
    if (!effectiveSyncTargetIds.length) {
      notify("warning", t("noSyncTargets"));
      return;
    }
    await runAction({}, async () => {
      const report = await syncSkillsToTools([toSyncItem(skill)], effectiveSyncTargetIds);
      return applyOutcome(syncOutcome(report, { ...foldContext, action: "bulk" }));
    });
  }, [
    applyOutcome,
    effectiveSyncTargetIds,
    foldContext,
    notify,
    runAction,
    syncSkillsToTools,
    t,
  ]);

  const syncAllManagedToTools = useCallback(async (toolIds: string[]) => {
    if (!autoSyncEnabled || !managedSkills.length || !toolIds.length) return;
    await runAction({}, async () => {
      const report = await syncSkillsToTools(
        managedSkills.map(toSyncItem), toolIds, { overwriteIfSameContent: true },
      );
      return applyOutcome(syncOutcome(report, { ...foldContext, action: "bulk" }));
    });
  }, [applyOutcome, autoSyncEnabled, foldContext, managedSkills, runAction, syncSkillsToTools]);

  const handleDeleteManaged = useCallback(async (skill: ManagedSkill) => {
    await runAction({ message: t("actions.removing", { name: skill.name }) }, async () => {
      const result = await invokeTauri("deleteManagedSkill", skill.id);
      const completion = await applyOutcome(deleteOutcome(result, foldContext));
      if (completion.closeModal) setPendingDeleteId(null);
    });
  }, [applyOutcome, foldContext, runAction, t]);

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
        },
        async () => {
          if (synced) {
            const report = await invokeTauri("unsyncSkillFromTool", skill.id, toolId);
            await applyOutcome(removalOutcome(report, { ...foldContext, action: "toggle" }));
          } else {
            const report = await syncSkillsToTools(
              [toSyncItem(skill)], [toolId], { overwriteIfSameContent: true },
            );
            await applyOutcome(syncOutcome(report, { ...foldContext, action: "toggle" }));
          }
        },
      );
    },
    [applyOutcome, foldContext, loading, runAction, syncSkillsToTools, t, tools],
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

  /** Thrown requests also reload: Update, Restore and both repairs agree. */
  const runSingleRefresh = useCallback(async (
    skill: ManagedSkill,
    copy: { message: string; success: string },
    requestRefresh?: () => Promise<RefreshReportDto>,
  ) => runAction({ message: copy.message }, async () => {
    let report: RefreshReportDto;
    try {
      report = await (requestRefresh ? requestRefresh() : refreshSkills([skill.id]));
    } catch (error) {
      await loadManagedSkills();
      throw error;
    }
    return applyOutcome(refreshOutcome(report, { ...foldContext, single: { name: skill.name, success: copy.success } }));
  }), [applyOutcome, foldContext, loadManagedSkills, refreshSkills, runAction]);

  const handleUpdateManaged = useCallback(
    (skill: ManagedSkill) =>
      runSingleRefresh(skill, {
        message: t("actions.updating", { name: skill.name }),
        success: t("status.updated", { name: skill.name }),
      }),
    [runSingleRefresh, t],
  );

  const handleUpdateSkill = useCallback(
    (skill: ManagedSkill) => {
      void handleUpdateManaged(skill);
    },
    [handleUpdateManaged],
  );

  /**
   * Restore an Unlocatable skill whose central copy is gone: the same
   * batch of one — the backend re-acquires it from its source and rebuilds
   * the central copy, and Propagation follows.
   */
  const handleRestoreSkill = useCallback(
    (skill: ManagedSkill) =>
      runSingleRefresh(skill, {
        message: t("actions.restoring", { name: skill.name }),
        success: t("status.restored", { name: skill.name }),
      }),
    [runSingleRefresh, t],
  );

  const handleCloseRepointGitSkill = useCallback(() => {
    setGitRepointSelection(null);
  }, []);

  const handleConfirmRepointGitSkill = useCallback(
    async (url: string) => {
      const skill = pendingGitRepointSkill;
      if (!skill) return;
      const completed = await runSingleRefresh(
        skill,
        {
          message: t("actions.repointing", { name: skill.name }),
          success: t("status.repointed", { name: skill.name }),
        },
        () => invokeTauri("repointGitSkillSource", skill.id, url.trim(), {
          reassert_auto_sync: autoSyncEnabled,
        }),
      );
      if (completed?.closeModal) setGitRepointSelection(null);
    },
    [autoSyncEnabled, pendingGitRepointSkill, runSingleRefresh, t],
  );

  /**
   * Re-point by provenance: git opens the URL Modal; local picks a folder.
   * For a `local` skill whose folder is gone, pick its new location,
   * then the backend rewrites the source and runs the Update from it. A
   * cancelled picker is not an action at all.
   */
  const handleRepointSkill = useCallback(
    async (skill: ManagedSkill) => {
      if (sourceKind(skill) === "git") {
        handleRepointGitSkill(skill);
        return;
      }
      let newPath: string;
      try {
        const { open } = await import("@tauri-apps/plugin-dialog");
        const selected = await open({
          directory: true,
          multiple: false,
          title: t("unlocatable.selectNewSourceFolder", { name: skill.name }),
        });
        if (!selected || Array.isArray(selected)) return;
        newPath = selected;
      } catch (err) {
        setError(formatError(err));
        return;
      }
      await runSingleRefresh(skill, {
        message: t("actions.repointing", { name: skill.name }),
        success: t("status.repointed", { name: skill.name }),
      }, () => invokeTauri("repointLocalSkillSource", skill.id, newPath, { reassert_auto_sync: autoSyncEnabled }));
    },
    [autoSyncEnabled, formatError, handleRepointGitSkill, runSingleRefresh, setError, t],
  );

  /**
   * Detach a `local` skill from its vanished folder: it becomes `imported`
   * (the central copy is its truth). Store-only on the backend.
   */
  const handleDetachSkill = useCallback(
    async (skill: ManagedSkill) => {
      await runAction(
        {
          message: t("actions.detaching", { name: skill.name }),
          successToast: t("status.detached", { name: skill.name }),
        },
        async () => {
          await invokeTauri("detachSkillFromSource", skill.id);
          await loadManagedSkills();
        },
      );
    },
    [loadManagedSkills, runAction, t],
  );

  return {
    managedSkills,
    invocationEditSkillId,
    invocationEditSkill,
    openInvocationEdit,
    closeInvocationEdit,
    setInvocationOverride,
    detailSkill,
    openDetail,
    closeDetail,
    pendingDeleteId,
    pendingDeleteSkill,
    pendingGitRepointSkill,
    handleRepointGitSkill,
    handleCloseRepointGitSkill,
    handleConfirmRepointGitSkill,
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
    handleRestoreSkill,
    handleRepointSkill,
    handleDetachSkill,
  };
}

export type SkillLibrary = ReturnType<typeof useSkillLibrary>;
