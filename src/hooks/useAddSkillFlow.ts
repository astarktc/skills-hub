import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  GitSkillCandidate,
  ImportProgressDto,
  InstallProgressDto,
  InstallRequest,
  InstallSource,
  LocalSkillCandidate,
  OnboardingPlan,
  OnboardingSelectionDto,
} from "../components/skills/types";

import {
  importOutcome,
  installBatchSettlements,
  installOutcome,
  type InstallSettlement,
} from "../lib/reportOutcome";
import { globalCollisionRows, installTargets, mergeInstallRetry } from "../lib/overwriteAsk";
import { defaultImportVariantPath } from "../lib/skillPresentation";
import { createProgressChannel, pickFolder } from "../lib/platform";
import { invokeTauri, isTauri } from "../lib/tauri";
import {
  useCandidatePick,
  type InstallBatchAction,
  type InstallPick,
} from "./useCandidatePick";
import type { SkillLibrary } from "./useSkillLibrary";
import type { SyncOrchestration } from "./useSyncOrchestration";
import type { ToolCatalog } from "./useToolCatalog";
import type { DialogDoor } from "./useSurfaceState";
import type {
  StatusReporter,
  TranslateFn,
} from "./useStatusReporter";

export type AddSkillFlowDeps = {
  t: TranslateFn;
  reporter: StatusReporter;
  sync: Pick<
    SyncOrchestration,
    | "autoSyncEnabled"
    | "resolveGlobalCollisions"
    | "scanScopeRevision"
    | "syncTargets"
    | "targetAllInstalled"
  >;
  /** The Tool catalog: which tools a deploy target can name, and their labels. */
  catalog: Pick<ToolCatalog, "toolLabelById" | "tools">;
  library: Pick<SkillLibrary, "isSkillNameTaken" | "loadManagedSkills">;
  /** The surface module: Add, Import and the two pickers are its dialogs. */
  surface: Pick<
    DialogDoor,
    "isOpen" | "openDialog" | "pushDialog" | "closeDialog" | "dismissDialog"
  >;
  /**
   * Whether the import review's invitation is still on screen (My Skills is
   * showing). A review that waited for a running action opens its dialog
   * only if it is.
   */
  isReviewWanted?: () => boolean;
};

/**
 * Add/import world: the add-skill modal (local + git tabs), candidate
 * discovery and the pick modals, the onboarding import plan, and the
 * Explore-page one-click install path that funnels into the git flow.
 */
export function useAddSkillFlow({
  t,
  reporter,
  sync,
  catalog,
  library,
  surface,
  isReviewWanted,
}: AddSkillFlowDeps) {
  const {
    loading,
    runAction,
    setActionMessage,
    setError,
    formatError,
    applyOutcome,
  } = reporter;
  const {
    autoSyncEnabled,
    resolveGlobalCollisions,
    scanScopeRevision,
    syncTargets,
    targetAllInstalled,
  } = sync;
  const { toolLabelById, tools } = catalog;
  const { isSkillNameTaken, loadManagedSkills } = library;

  const [plan, setPlan] = useState<OnboardingPlan | null>(null);
  const [selected, setSelected] = useState<Record<string, boolean>>({});
  const [variantChoice, setVariantChoice] = useState<Record<string, string>>(
    {},
  );
  const { isOpen, openDialog, closeDialog, dismissDialog } = surface;
  const showAddModal = isOpen("addSkill");
  const showImportModal = isOpen("import");
  const [addModalTab, setAddModalTab] = useState<"local" | "git">("git");
  const [localPath, setLocalPath] = useState("");
  const [localName, setLocalName] = useState("");
  const [gitUrl, setGitUrl] = useState("");
  const [gitName, setGitName] = useState("");
  const [autoSelectSkillName, setAutoSelectSkillName] = useState<string | null>(
    null,
  );
  const [exploreInstallTrigger, setExploreInstallTrigger] = useState(0);
  const exploreInstallUrlRef = useRef<string | null>(null);

  /**
   * The operator's deploy targets, not intersected with detection: the
   * batch reports a selected-but-undetected tool as a skip, which is the
   * operator's signal that the selection has gone stale (round 12 D3).
   */
  const getSelectedIds = useCallback(
    () => tools.filter((tool) => syncTargets[tool.id]).map((t) => t.id),
    [syncTargets, tools],
  );

  /**
   * The Add action as one backend batch (`installSelection`, D12): every
   * pick is acquired, finalized and deployed to the operator's deploy
   * targets in one call that answers one report. Its collisions — across
   * every installed candidate — raise **one** overwrite ask; the answer's
   * retry is an ordinary sync batch (the skills exist by then) whose rows
   * replace the asked ones. A dismissed ask records nothing.
   *
   * Cancel (D20): the batch honours the action's cancellation at its
   * boundary — an action cancelled before `installSelection` is sent (a
   * Cancel pressed during the caller's listing, which takes no token) sends
   * nothing and answers every pick `cancelled` — and withdraws Cancel for
   * the ask's retry, which no token stops.
   */
  const installBatch = useCallback(
    async (
      source: InstallSource,
      picks: InstallPick[],
      action: InstallBatchAction,
    ): Promise<InstallSettlement[]> => {
      const tools = autoSyncEnabled ? getSelectedIds() : [];
      const request: InstallRequest = {
        source,
        candidates: picks.map((pick) => ({
          subpath: pick.subpath,
          name_override: pick.nameOverride,
        })),
        tools,
        overwrite_if_same_content: true,
        overrides: [],
        deploy: autoSyncEnabled,
      };
      const onProgress = await createProgressChannel<InstallProgressDto>((progress) => {
        setActionMessage(
          t("installBatch.step", {
            index: progress.index,
            total: progress.total,
            name: progress.name,
          }),
        );
      });
      // The boundary: the command resets the shared token on entry, so a
      // Cancel that arrived before it would be lost there — honour it here.
      if (action.cancelRequested()) {
        return installBatchSettlements(
          {
            candidates: picks.map((pick) => ({
              subpath: pick.subpath,
              name: pick.name,
              status: { status: "cancelled" },
            })),
          },
          { deploy: autoSyncEnabled, tools },
        );
      }
      let report = await invokeTauri("installSelection", request, onProgress);
      const targets = installTargets(report);
      const asked = globalCollisionRows(targets, toolLabelById);
      if (asked.length > 0) {
        const skills = report.candidates.flatMap((candidate) =>
          candidate.status.status === "installed"
            ? [{
                skill_id: candidate.status.skill.skill_id,
                name: candidate.status.skill.name,
                source_path: candidate.status.skill.central_path,
              }]
            : [],
        );
        // The skills are installed whatever the retry does: a thrown retry
        // is reported and the batch settles as it first answered (the
        // collided rows read as target failures).
        try {
          const settled = await action.uncancellable(() =>
            resolveGlobalCollisions(targets, skills, {
              overwriteIfSameContent: true,
            }),
          );
          report = mergeInstallRetry(report, asked, settled);
        } catch (err) {
          setError(formatError(err));
        }
      }
      // A candidate that never installed reads under the listing's name.
      const names = new Map(picks.map((pick) => [pick.subpath, pick.name]));
      report = {
        candidates: report.candidates.map((candidate) =>
          candidate.status.status === "installed"
            ? candidate
            : { ...candidate, name: names.get(candidate.subpath) ?? candidate.name },
        ),
      };
      return installBatchSettlements(report, { deploy: autoSyncEnabled, tools });
    },
    [
      autoSyncEnabled,
      formatError,
      getSelectedIds,
      resolveGlobalCollisions,
      setActionMessage,
      setError,
      t,
      toolLabelById,
    ],
  );

  /**
   * Post-mutation refresh that can never fail the action that already
   * succeeded: the bytes are installed, so a reload failure is a non-fatal
   * warning on the reporter's one-shot error channel, not a thrown failure
   * that would suppress the success toast and leave the modal open.
   */
  const refreshWithoutFailingAction = useCallback(
    async (refresh: () => Promise<unknown>) => {
      try {
        await refresh();
      } catch (err) {
        setError(formatError(err));
      }
    },
    [formatError, setError],
  );

  /**
   * Any install's completion (single or batch), executed by the reporter's
   * one `applyOutcome`: the library has changed (a failed re-read stays
   * nonfatal), and the add modal is done.
   */
  const installHandlers = useMemo(
    () => ({
      reload: () => refreshWithoutFailingAction(loadManagedSkills),
      closeModal: () => dismissDialog("addSkill"),
    }),
    [dismissDialog, loadManagedSkills, refreshWithoutFailingAction],
  );

  const pickDeps = {
    t,
    reporter,
    surface,
    isSkillNameTaken,
    installBatch,
    toolLabelById,
    outcomeHandlers: installHandlers,
  };

  /** Git picker: candidates of one repo URL (the picker's context). */
  const git = useCandidatePick<GitSkillCandidate, string>(
    {
      dialogKind: "gitPick",
      customName: gitName,
      // Every candidate of one listing carries the same source resolution.
      installSource: (repoUrl, picked) => ({
        kind: "git",
        url: repoUrl,
        resolution: picked[0]?.resolution ?? null,
      }),
      resetForm: () => {
        setGitUrl("");
        setGitName("");
      },
    },
    pickDeps,
  );

  /** Local picker: candidates under one base path (the picker's context). */
  const local = useCandidatePick<LocalSkillCandidate, string>(
    {
      dialogKind: "localPick",
      customName: localName,
      selectable: (candidate) => candidate.valid,
      installSource: (basePath) => ({ kind: "local", root: basePath }),
      resetForm: () => {
        setLocalPath("");
        setLocalName("");
      },
    },
    pickDeps,
  );

  /** Fetch the onboarding plan and reset the selection to its defaults. */
  const fetchPlan = useCallback(async () => {
    const result = await invokeTauri("getOnboardingPlan");
    setPlan(result);
    const defaultSelected: Record<string, boolean> = {};
    const defaultChoice: Record<string, string> = {};
    result.groups.forEach((group) => {
      defaultSelected[group.name] = true;
      const chosen = defaultImportVariantPath(group);
      if (chosen) {
        defaultChoice[group.name] = chosen;
      }
    });
    setSelected(defaultSelected);
    setVariantChoice(defaultChoice);
    return result;
  }, []);

  /**
   * fetchPlan as its own action (header progress, error toast). Resolves to
   * the plan, or undefined when loading it failed.
   */
  const loadPlan = useCallback(
    () => runAction({ read: true, label: t("activity.labels.scan"), activity: t("activity.scanning") }, fetchPlan),
    [fetchPlan, runAction, t],
  );

  useEffect(() => {
    if (!isTauri) return;
    // Fire-and-forget load on mount (see loadManagedSkills effect) and again
    // whenever the saved tool configuration changes: the backend resolves
    // the scan's scope from it, so the banner, its count and the reviewed
    // plan must follow the configuration, never a plan from before it.
    // loadPlan's intentional eager progress setState is preserved.
    void (async () => {
      await loadPlan();
    })();
  }, [loadPlan, scanScopeRevision]);

  const handleOpenAdd = useCallback(() => {
    openDialog({ kind: "addSkill" });
  }, [openDialog]);

  /** Modals stay put while their action runs (its progress and Cancel are at the dialog's foot). */
  const handleCloseAdd = useCallback(() => {
    if (!loading) closeDialog("addSkill");
  }, [closeDialog, loading]);
  const handleCloseImport = useCallback(() => {
    if (!loading) closeDialog("import");
  }, [closeDialog, loading]);

  /**
   * Review always reviews a plan fetched now: the import re-derives the
   * plan under the current scope and acts on the originals it finds, so the
   * operator must have seen exactly those. The banner's plan is only the
   * invitation.
   */
  const reviewGenerationRef = useRef(0);
  const handleReviewImport = useCallback(async () => {
    // Still wanted while it is the latest request and its invitation is on
    // screen — checked when its turn comes and before the dialog opens.
    const generation = ++reviewGenerationRef.current;
    const stillWanted = () =>
      reviewGenerationRef.current === generation && (isReviewWanted?.() ?? true);
    const result = await runAction(
      { read: true, stillWanted, label: t("activity.labels.scan"), activity: t("activity.scanning") },
      fetchPlan,
    );
    if (result && stillWanted()) {
      openDialog({ kind: "import" });
    }
  }, [fetchPlan, isReviewWanted, openDialog, runAction, t]);

  const handlePickLocalPath = useCallback(async () => {
    try {
      const [selected] = await pickFolder({ title: t("selectLocalFolder") });
      if (selected) setLocalPath(selected);
    } catch (err) {
      setError(formatError(err));
    }
  }, [formatError, setError, t]);

  const handleToggleGroup = useCallback(
    (groupName: string, checked: boolean) => {
      setSelected((prev) => ({
        ...prev,
        [groupName]: checked,
      }));
    },
    [],
  );

  const handleSelectVariant = useCallback((groupName: string, path: string) => {
    setVariantChoice((prev) => ({
      ...prev,
      [groupName]: path,
    }));
  }, []);

  const toggleAll = useCallback(
    (checked: boolean) => {
      if (!plan) return;
      const next: Record<string, boolean> = {};
      plan.groups.forEach((group) => {
        next[group.name] = checked;
      });
      setSelected(next);
    },
    [plan],
  );

  /**
   * Import is one backend call: the selections plus the auto-sync policy.
   * The backend admits each chosen variant, finalizes it, and either syncs
   * it (auto-sync on — every Tool holding a byte-identical variant is
   * overwritten in place, whether or not the policy names it) or removes
   * the byte-identical originals; this side only states the selection and
   * renders the report.
   */
  const handleImport = async () => {
    if (!plan) return;
    await runAction({ label: t("activity.labels.import"), activity: t("activity.importing") }, async () => {
      const selections: OnboardingSelectionDto[] = [];
      for (const group of plan.groups) {
        if (!selected[group.name]) continue;
        const chosenPath =
          variantChoice[group.name] ?? defaultImportVariantPath(group);
        if (!chosenPath) continue;
        selections.push({
          group_name: group.name,
          chosen_path: chosenPath,
          name: null,
        });
      }

      const onProgress = await createProgressChannel<ImportProgressDto>((progress) => {
        setActionMessage(
          t(
            progress.phase === "admitting"
              ? "actions.importStep"
              : "actions.importApplyStep",
            {
              index: progress.index,
              total: progress.total,
              name: progress.group_name,
            },
          ),
        );
      });
      const report = await invokeTauri(
        "importOnboardingSelection",
        selections,
        {
          auto_sync: autoSyncEnabled,
          tools: autoSyncEnabled ? getSelectedIds() : null,
          // Today's policy until the review offers the per-run choice
          // (ticket 21): auto-sync off removes the byte-identical
          // originals; auto-sync on ignores it (the sync takes them over).
          remove_originals: true,
        },
        onProgress,
      );
      await applyOutcome(importOutcome(report, { t, toolLabelById }), {
        reload: () =>
          refreshWithoutFailingAction(async () => {
            await loadManagedSkills();
            await fetchPlan();
          }),
        closeModal: () => dismissDialog("import"),
      });
    });
  };

  const handleCreateLocal = async () => {
    if (!localPath.trim()) {
      setError(t("errors.requireLocalPath"));
      return;
    }
    await runAction(
      // Cancellable from the start: the listing takes no token, so a Cancel
      // pressed during it is honoured at the batch boundary (installBatch)
      // and here, before a picker would open.
      {
        message: t("actions.creatingLocalSkill"),
        label: t("activity.labels.add"),
        activity: t("activity.adding"),
        cancel: "installSelection",
      },
      async (action) => {
        const basePath = localPath.trim();
        const candidates = await invokeTauri("listLocalSkillsCmd", basePath);
        if (action.cancelRequested()) return action.fail(null);
        if (candidates.length === 0) {
          return action.fail(t("errors.noSkillsFoundLocal"));
        }
        if (candidates.length !== 1 || !candidates[0].valid) {
          local.open(basePath, candidates);
          return action.handOff();
        }
        const desiredName = localName.trim() || candidates[0].name;
        if (isSkillNameTaken(desiredName)) {
          return action.fail(
            t("errors.skillAlreadyExists", { name: desiredName }),
          );
        }
        const settlements = await installBatch(
          { kind: "local", root: basePath },
          [{
            subpath: candidates[0].subpath,
            name: desiredName,
            nameOverride: localName.trim() || null,
          }],
          action,
        );
        setLocalPath("");
        setLocalName("");
        await applyOutcome(installOutcome(settlements, { t, toolLabelById, source: "local" }), installHandlers);
      },
    );
  };

  const handleCreateGit = async () => {
    if (!gitUrl.trim()) {
      setError(t("errors.requireGitUrl"));
      return;
    }
    await runAction(
      // Cancellable from the start: the listing takes no token, so a Cancel
      // pressed during it is honoured at the batch boundary (installBatch)
      // and here, before a picker would open.
      {
        message: t("actions.creatingGitSkill"),
        label: t("activity.labels.add"),
        activity: t("activity.adding"),
        cancel: "installSelection",
      },
      async (action) => {
        const url = gitUrl.trim();

        // All URLs (including /tree/ and /blob/ folder URLs) route through
        // the candidate-based flow. The backend's list_git_skills handles
        // folder URL subpath extraction and, for an Explore install, resolves
        // the intended skill name against the candidates (the one core
        // matching rule) -- this side only decides between install / pick.
        const target = autoSelectSkillName;
        setAutoSelectSkillName(null);
        const { candidates, target_match } = await invokeTauri(
          "listGitSkillsCmd",
          url,
          target,
        );
        if (action.cancelRequested()) return action.fail(null);
        if (candidates.length === 0) {
          return action.fail(t("errors.noSkillsFoundWithHint"));
        }

        /** Which candidate to install, or hand off to the picker. */
        let chosen: GitSkillCandidate | undefined;
        if (target) {
          chosen =
            target_match?.kind === "resolved"
              ? candidates.find((c) => c.subpath === target_match.subpath)
              : undefined;
          // A lone candidate that is not the intended skill means the scan
          // missed it: report, never silently install the wrong one.
          if (!chosen && candidates.length === 1) {
            return action.fail(
              t("errors.skillNotFoundInRepo", { name: target }),
            );
          }
        } else if (candidates.length === 1) {
          chosen = candidates[0];
        }
        if (!chosen) {
          git.open(url, candidates);
          return action.handOff();
        }

        if (isSkillNameTaken(chosen.name)) {
          return action.fail(
            t("errors.skillAlreadyExists", { name: chosen.name }),
          );
        }
        const settlements = await installBatch(
          { kind: "git", url, resolution: chosen.resolution },
          [{
            subpath: chosen.subpath,
            name: chosen.name,
            nameOverride: gitName.trim() || null,
          }],
          action,
        );
        setGitUrl("");
        setGitName("");
        await applyOutcome(installOutcome(settlements, { t, toolLabelById, source: "git" }), installHandlers);
      },
    );
  };

  /** The add modal's primary action: whichever tab is showing. */
  const handleCreate = () =>
    addModalTab === "local" ? handleCreateLocal() : handleCreateGit();

  const handleExploreInstall = useCallback(
    (sourceUrl: string, skillName?: string) => {
      setGitUrl(sourceUrl);
      if (skillName) setAutoSelectSkillName(skillName);
      targetAllInstalled();
      exploreInstallUrlRef.current = sourceUrl;
      setExploreInstallTrigger((n) => n + 1);
    },
    [targetAllInstalled],
  );

  useEffect(() => {
    if (exploreInstallTrigger > 0 && exploreInstallUrlRef.current && !loading) {
      exploreInstallUrlRef.current = null;
      void handleCreateGit();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [exploreInstallTrigger]);

  return {
    plan,
    selected,
    variantChoice,
    showAddModal,
    showImportModal,
    addModalTab,
    setAddModalTab,
    localPath,
    setLocalPath,
    localName,
    setLocalName,
    gitUrl,
    setGitUrl,
    gitName,
    setGitName,
    git,
    local,
    handleOpenAdd,
    handleCloseAdd,
    handleCloseImport,
    handleReviewImport,
    handlePickLocalPath,
    handleToggleGroup,
    handleSelectVariant,
    toggleAll,
    handleImport,
    handleCreate,
    handleExploreInstall,
  };
}
