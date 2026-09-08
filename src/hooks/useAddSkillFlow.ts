import { useCallback, useEffect, useRef, useState } from "react";
import type {
  GitSkillCandidate,
  ImportProgressDto,
  InstallResultDto,
  LocalSkillCandidate,
  OnboardingPlan,
  OnboardingSelectionDto,
} from "../components/skills/types";

import { importOutcome, installOutcome, type InstallDeployment, type Outcome, type PlainEntry } from "../lib/reportOutcome";
import { defaultImportVariantPath } from "../lib/skillPresentation";
import { invokeTauri, isTauri } from "../lib/tauri";
import { useCandidatePick } from "./useCandidatePick";
import type { SkillLibrary } from "./useSkillLibrary";
import type { SyncOrchestration } from "./useSyncOrchestration";
import type {
  StatusReporter,
  TranslateFn,
} from "./useStatusReporter";

/** The `{skill_id, name, source_path}` batch item for a freshly installed skill. */
const toSyncItem = (created: InstallResultDto) => ({
  skill_id: created.skill_id,
  name: created.name,
  source_path: created.central_path,
});

export type AddSkillFlowDeps = {
  t: TranslateFn;
  reporter: StatusReporter;
  sync: Pick<
    SyncOrchestration,
    | "autoSyncEnabled"
    | "isInstalled"
    | "syncSkillsToTools"
    | "syncTargets"
    | "targetAllInstalled"
    | "toolLabelById"
    | "tools"
  >;
  library: Pick<SkillLibrary, "isSkillNameTaken" | "loadManagedSkills">;
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
  library,
}: AddSkillFlowDeps) {
  const {
    loading,
    notify,
    showActionWarnings,
    runAction,
    setActionMessage,
    setError,
    formatError,
    showActionErrors,
  } = reporter;
  const {
    autoSyncEnabled,
    isInstalled,
    syncSkillsToTools,
    syncTargets,
    targetAllInstalled,
    toolLabelById,
    tools,
  } = sync;
  const { isSkillNameTaken, loadManagedSkills } = library;

  const [plan, setPlan] = useState<OnboardingPlan | null>(null);
  const [selected, setSelected] = useState<Record<string, boolean>>({});
  const [variantChoice, setVariantChoice] = useState<Record<string, string>>(
    {},
  );
  const [showAddModal, setShowAddModal] = useState(false);
  const [showImportModal, setShowImportModal] = useState(false);
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

  /** Deploy targets that are both user-selected and actually installed. */
  const getSelectedInstalledIds = useCallback(
    () =>
      tools
        .filter((tool) => syncTargets[tool.id] && isInstalled(tool.id))
        .map((t) => t.id),
    [isInstalled, syncTargets, tools],
  );

  /** Acquisition already succeeded: even a thrown deploy cannot undo it. */
  const deployNewSkill = useCallback(
    async (created: InstallResultDto): Promise<InstallDeployment> => {
      if (!autoSyncEnabled) return { status: "disabled" };
      const selectedInstalledIds = getSelectedInstalledIds();
      if (!selectedInstalledIds.length) return { status: "no-targets" };
      try {
        return { status: "reported", report: await syncSkillsToTools(
          [toSyncItem(created)], selectedInstalledIds, { overwriteIfSameContent: true },
        ) };
      } catch (error) {
        return { status: "failed", error };
      }
    },
    [autoSyncEnabled, getSelectedInstalledIds, syncSkillsToTools],
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

  /** After any install (single or batch): the add modal is done, the library has changed. */
  const finishInstall = useCallback(async (completion: Outcome["completion"]) => {
    if (completion.closeModal) setShowAddModal(false);
    if (completion.reload) await refreshWithoutFailingAction(loadManagedSkills);
  }, [loadManagedSkills, refreshWithoutFailingAction]);

  const pickDeps = {
    t,
    reporter,
    isSkillNameTaken,
    deploy: deployNewSkill,
    toolLabelById,
    afterBatch: finishInstall,
  };

  /** Git picker: candidates of one repo URL (the picker's context). */
  const git = useCandidatePick<GitSkillCandidate, string>(
    {
      customName: gitName,
      installOne: (repoUrl, candidate, name) =>
        invokeTauri("installGitSelection", repoUrl, candidate.subpath, name, candidate.resolution ?? null),
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
      customName: localName,
      selectable: (candidate) => candidate.valid,
      installOne: (basePath, candidate, name) =>
        invokeTauri("installLocalSelection", basePath, candidate.subpath, name),
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
   * fetchPlan as its own action (loading overlay, error toast). Resolves to
   * the plan, or undefined when loading it failed.
   */
  const loadPlan = useCallback(
    () => runAction({}, fetchPlan),
    [fetchPlan, runAction],
  );

  useEffect(() => {
    if (!isTauri) return;
    // Fire-and-forget load on mount (see loadManagedSkills effect). loadPlan's
    // intentional eager loading-overlay setState is preserved exactly.
    void (async () => {
      await loadPlan();
    })();
  }, [loadPlan]);

  const handleOpenAdd = useCallback(() => {
    setShowAddModal(true);
  }, []);

  /** Modals stay put while an action runs (the overlay owns the screen). */
  const closeUnlessLoading = useCallback(
    (setShow: (open: boolean) => void) => {
      if (!loading) setShow(false);
    },
    [loading],
  );
  const handleCloseAdd = useCallback(
    () => closeUnlessLoading(setShowAddModal),
    [closeUnlessLoading],
  );
  const handleCloseImport = useCallback(
    () => closeUnlessLoading(setShowImportModal),
    [closeUnlessLoading],
  );

  const handleReviewImport = useCallback(async () => {
    if (plan) {
      setShowImportModal(true);
      return;
    }
    const result = await loadPlan();
    if (result) {
      setShowImportModal(true);
    }
  }, [loadPlan, plan]);

  const handlePickLocalPath = useCallback(async () => {
    try {
      if (!isTauri) {
        throw new Error(t("errors.notTauri"));
      }
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        directory: true,
        multiple: false,
        title: t("selectLocalFolder"),
      });
      if (!selected || Array.isArray(selected)) return;
      setLocalPath(selected);
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

  const publishOutcome = (outcome: Outcome<PlainEntry>) => {
    showActionErrors(outcome.errors);
    showActionWarnings(outcome.warnings);
    if (outcome.toast) notify(outcome.toast.kind, outcome.toast.message, outcome.toast.detail);
  };

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
    await runAction({}, async () => {
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

      const { Channel } = await import("@tauri-apps/api/core");
      const onProgress = new Channel<ImportProgressDto>();
      onProgress.onmessage = (progress) => {
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
      };
      const report = await invokeTauri(
        "importOnboardingSelection",
        selections,
        {
          auto_sync: autoSyncEnabled,
          tools: autoSyncEnabled ? getSelectedInstalledIds() : null,
        },
        onProgress,
      );
      const outcome = importOutcome(report, { t, toolLabelById });
      if (outcome.completion.reload) await refreshWithoutFailingAction(async () => {
        await loadManagedSkills();
        await fetchPlan();
      });
      if (outcome.completion.closeModal) setShowImportModal(false);
      publishOutcome(outcome);
    });
  };

  const handleCreateLocal = async () => {
    if (!localPath.trim()) {
      setError(t("errors.requireLocalPath"));
      return;
    }
    await runAction(
      {
        message: t("actions.creatingLocalSkill"),
      },
      async (action) => {
        const basePath = localPath.trim();
        const candidates = await invokeTauri("listLocalSkillsCmd", basePath);
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
        const created = await invokeTauri(
          "installLocalSelection",
          basePath,
          candidates[0].subpath,
          localName.trim() || null,
        );
        const deployment = await deployNewSkill(created);
        const outcome = installOutcome([{ name: created.name, status: "installed", result: created, deployment }], { t, toolLabelById, source: "local" });
        publishOutcome(outcome);
        setLocalPath("");
        setLocalName("");
        await finishInstall(outcome.completion);
      },
    );
  };

  const handleCreateGit = async () => {
    if (!gitUrl.trim()) {
      setError(t("errors.requireGitUrl"));
      return;
    }
    await runAction(
      {
        message: t("actions.creatingGitSkill"),
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
        const created = await invokeTauri(
          "installGitSelection",
          url,
          chosen.subpath,
          gitName.trim() || null,
          chosen.resolution ?? null,
        );
        const deployment = await deployNewSkill(created);
        const outcome = installOutcome([{ name: created.name, status: "installed", result: created, deployment }], { t, toolLabelById, source: "git" });
        publishOutcome(outcome);
        setGitUrl("");
        setGitName("");
        await finishInstall(outcome.completion);
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
