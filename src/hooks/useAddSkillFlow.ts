import { useCallback, useEffect, useRef, useState } from "react";
import type {
  GitSkillCandidate,
  InstallResultDto,
  LocalSkillCandidate,
  OnboardingPlan,
} from "../components/skills/types";
import { invokeTauri, isTauri } from "../lib/tauri";
import { useCandidatePick } from "./useCandidatePick";
import type { SkillLibrary } from "./useSkillLibrary";
import type { SyncOrchestration } from "./useSyncOrchestration";
import type { StatusReporter, TranslateFn } from "./useStatusReporter";

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
    | "syncFailureEntries"
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
    runAction,
    setActionMessage,
    setError,
    formatError,
    showActionErrors,
  } = reporter;
  const {
    autoSyncEnabled,
    isInstalled,
    syncFailureEntries,
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

  /**
   * Canonical install→deploy tail: after a skill is installed, auto-sync it
   * to the user-selected installed targets (when auto-sync is on) and return
   * the failure entries to surface. When no targets are selected+installed,
   * `noTargets: "set-error"` reports it immediately via setError (single
   * installs) while `noTargets: "collect"` returns it as an error entry
   * (batch installs).
   */
  const deployNewSkill = useCallback(
    async (
      created: InstallResultDto,
      opts: { noTargets: "set-error" | "collect" },
    ): Promise<{ title: string; message: string }[]> => {
      if (!autoSyncEnabled) return [];
      const selectedInstalledIds = getSelectedInstalledIds();
      if (selectedInstalledIds.length === 0) {
        const message = t("errors.noSyncTargets");
        if (opts.noTargets === "set-error") {
          setError(message);
          return [];
        }
        return [
          { title: t("errors.unsyncedTitle", { name: created.name }), message },
        ];
      }
      const report = await syncSkillsToTools(
        [toSyncItem(created)],
        selectedInstalledIds,
        { overwriteIfSameContent: true },
      );
      return syncFailureEntries(report, { includeNotWritableSkips: true });
    },
    [
      autoSyncEnabled,
      getSelectedInstalledIds,
      setError,
      syncFailureEntries,
      syncSkillsToTools,
      t,
    ],
  );

  /** After any install (single or batch): the add modal is done, the library has changed. */
  const finishInstall = useCallback(async () => {
    setShowAddModal(false);
    await loadManagedSkills();
  }, [loadManagedSkills]);

  const pickDeps = {
    t,
    reporter,
    isSkillNameTaken,
    deploy: (created: InstallResultDto) =>
      deployNewSkill(created, { noTargets: "collect" }),
    afterBatch: finishInstall,
  };

  /** Git picker: candidates of one repo URL (the picker's context). */
  const git = useCandidatePick<GitSkillCandidate, string>(
    {
      customName: gitName,
      installOne: (repoUrl, candidate, name) =>
        invokeTauri("installGitSelection", repoUrl, candidate.subpath, name),
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
      const first = group.variants[0];
      if (first) {
        defaultChoice[group.name] = first.path;
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

  const handleImport = async () => {
    if (!plan) return;
    await runAction({ successToast: t("status.importCompleted") }, async () => {
      const collectedErrors: { title: string; message: string }[] = [];
      for (const group of plan.groups) {
        if (!selected[group.name]) continue;
        const chosenPath = variantChoice[group.name] ?? group.variants[0]?.path;
        if (!chosenPath) continue;
        const chosenVariantTool =
          group.variants.find((v) => v.path === chosenPath)?.tool ?? null;

        setActionMessage(t("actions.importExisting", { name: group.name }));
        const installResult = await invokeTauri(
          "importExistingSkill",
          chosenPath,
          group.name,
        );

        if (autoSyncEnabled) {
          const selectedInstalledIds = getSelectedInstalledIds();
          // The chosen variant's own tool (and its shared-dir group, expanded
          // backend-side) may be overwritten — that copy is the import source.
          const report = await syncSkillsToTools(
            [
              {
                skill_id: installResult.skill_id,
                name: group.name,
                source_path: installResult.central_path,
              },
            ],
            selectedInstalledIds,
            {
              overwriteIfSameContent: true,
              overrides: chosenVariantTool
                ? [
                    {
                      skill_id: installResult.skill_id,
                      tool: chosenVariantTool,
                      overwrite: true,
                    },
                  ]
                : [],
            },
          );
          for (const result of report.results) {
            const status = result.status;
            if (status.status === "synced") continue;
            const toolLabel = toolLabelById[result.tool] ?? result.tool;
            if (status.error.code === "TARGET_EXISTS") {
              collectedErrors.push({
                title: t("errors.syncFailedTitle", {
                  name: group.name,
                  tool: toolLabel,
                }),
                message: t("errors.syncTargetExistsMessage", {
                  path: status.error.path,
                }),
              });
            } else {
              collectedErrors.push({
                title: t("errors.syncFailedTitle", {
                  name: group.name,
                  tool: toolLabel,
                }),
                message: formatError(status.error) ?? "",
              });
            }
          }
        } else {
          // Auto-sync OFF: clean migration -- remove originals from all tool directories
          for (const variant of group.variants) {
            try {
              await invokeTauri("removeSkillSource", variant.path);
            } catch (err) {
              // Non-fatal: skill is already imported, cleanup failure is secondary
              const raw = formatError(err) ?? "";
              collectedErrors.push({
                title: t("errors.syncFailedTitle", {
                  name: group.name,
                  tool: variant.tool,
                }),
                message: raw,
              });
            }
          }
        }
      }

      await loadManagedSkills();
      await fetchPlan();
      if (collectedErrors.length > 0) {
        showActionErrors(collectedErrors);
      } else {
        setShowImportModal(false);
      }
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
        successToast: t("status.localSkillCreated"),
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
        const deployErrors = await deployNewSkill(created, {
          noTargets: "set-error",
        });
        if (deployErrors.length > 0) showActionErrors(deployErrors);
        setLocalPath("");
        setLocalName("");
        await finishInstall();
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
        successToast: t("status.gitSkillCreated"),
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
        const { candidates, target_match } =
          await invokeTauri("listGitSkillsCmd", url, target);
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
        );
        const deployErrors = await deployNewSkill(created, {
          noTargets: "set-error",
        });
        if (deployErrors.length > 0) showActionErrors(deployErrors);
        setGitUrl("");
        setGitName("");
        await finishInstall();
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
