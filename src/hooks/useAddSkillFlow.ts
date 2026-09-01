import { useCallback, useEffect, useRef, useState } from "react";
import type {
  GitSkillCandidate,
  InstallResultDto,
  LocalSkillCandidate,
  OnboardingPlan,
} from "../components/skills/types";
import { invokeTauri, isTauri } from "../lib/tauri";
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
  const [gitCandidates, setGitCandidates] = useState<GitSkillCandidate[]>([]);
  const [gitCandidatesRepoUrl, setGitCandidatesRepoUrl] = useState<string>("");
  const [showGitPickModal, setShowGitPickModal] = useState(false);
  const [gitCandidateSelected, setGitCandidateSelected] = useState<
    Record<string, boolean>
  >({});
  const [localCandidates, setLocalCandidates] = useState<LocalSkillCandidate[]>(
    [],
  );
  const [localCandidatesBasePath, setLocalCandidatesBasePath] = useState("");
  const [showLocalPickModal, setShowLocalPickModal] = useState(false);
  const [localCandidateSelected, setLocalCandidateSelected] = useState<
    Record<string, boolean>
  >({});
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

  /** Fetch the onboarding plan and reset the selection to its defaults. */
  const fetchPlan = useCallback(async () => {
    const result = await invokeTauri<OnboardingPlan>("get_onboarding_plan");
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

  const handleCloseAdd = useCallback(() => {
    if (!loading) setShowAddModal(false);
  }, [loading]);

  const handleCloseImport = useCallback(() => {
    if (!loading) setShowImportModal(false);
  }, [loading]);

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

  const handleCloseGitPick = useCallback(() => {
    if (!loading) setShowGitPickModal(false);
  }, [loading]);

  const handleCancelGitPick = useCallback(() => {
    if (loading) return;
    setShowGitPickModal(false);
    setGitCandidates([]);
    setGitCandidateSelected({});
    setGitCandidatesRepoUrl("");
  }, [loading]);

  const handleCloseLocalPick = useCallback(() => {
    if (!loading) setShowLocalPickModal(false);
  }, [loading]);

  const handleCancelLocalPick = useCallback(() => {
    if (loading) return;
    setShowLocalPickModal(false);
    setLocalCandidates([]);
    setLocalCandidateSelected({});
    setLocalCandidatesBasePath("");
  }, [loading]);

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

  const handleToggleAllGitCandidates = useCallback(
    (checked: boolean) => {
      setGitCandidateSelected(
        Object.fromEntries(gitCandidates.map((c) => [c.subpath, checked])),
      );
    },
    [gitCandidates],
  );

  const handleToggleAllLocalCandidates = useCallback(
    (checked: boolean) => {
      setLocalCandidateSelected(
        Object.fromEntries(
          localCandidates.map((c) => [c.subpath, c.valid && checked]),
        ),
      );
    },
    [localCandidates],
  );

  const handleToggleGitCandidate = useCallback(
    (subpath: string, checked: boolean) => {
      setGitCandidateSelected((prev) => ({
        ...prev,
        [subpath]: checked,
      }));
    },
    [],
  );

  const handleToggleLocalCandidate = useCallback(
    (subpath: string, checked: boolean) => {
      setLocalCandidateSelected((prev) => ({
        ...prev,
        [subpath]: checked,
      }));
    },
    [],
  );

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
        const installResult = await invokeTauri<InstallResultDto>(
          "import_existing_skill",
          {
            sourcePath: chosenPath,
            name: group.name,
          },
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
              await invokeTauri("remove_skill_source", { path: variant.path });
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

  /** Hand the local flow to the picker modal with the discovered candidates. */
  const openLocalPicker = (
    basePath: string,
    candidates: LocalSkillCandidate[],
  ) => {
    setLocalCandidatesBasePath(basePath);
    setLocalCandidates(candidates);
    setLocalCandidateSelected(
      Object.fromEntries(candidates.map((c) => [c.subpath, c.valid])),
    );
    setShowLocalPickModal(true);
  };

  /** Hand the git flow to the picker modal with the discovered candidates. */
  const openGitPicker = (url: string, candidates: GitSkillCandidate[]) => {
    setGitCandidatesRepoUrl(url);
    setGitCandidates(candidates);
    setGitCandidateSelected(
      Object.fromEntries(candidates.map((c) => [c.subpath, true])),
    );
    setShowGitPickModal(true);
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
        const candidates = await invokeTauri<LocalSkillCandidate[]>(
          "list_local_skills_cmd",
          { basePath },
        );
        if (candidates.length === 0) {
          return action.fail(t("errors.noSkillsFoundLocal"));
        }
        if (candidates.length !== 1 || !candidates[0].valid) {
          openLocalPicker(basePath, candidates);
          return action.handOff();
        }
        const desiredName = localName.trim() || candidates[0].name;
        if (isSkillNameTaken(desiredName)) {
          return action.fail(
            t("errors.skillAlreadyExists", { name: desiredName }),
          );
        }
        const created = await invokeTauri<InstallResultDto>(
          "install_local_selection",
          {
            basePath,
            subpath: candidates[0].subpath,
            name: localName.trim() || undefined,
          },
        );
        const deployErrors = await deployNewSkill(created, {
          noTargets: "set-error",
        });
        if (deployErrors.length > 0) showActionErrors(deployErrors);
        setLocalPath("");
        setLocalName("");
        setShowAddModal(false);
        await loadManagedSkills();
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
        // folder URL subpath extraction. This ensures every install goes
        // through proper candidate discovery and name matching.
        const candidates = await invokeTauri<GitSkillCandidate[]>(
          "list_git_skills_cmd",
          { repoUrl: url },
        );
        if (candidates.length === 0) {
          return action.fail(t("errors.noSkillsFoundWithHint"));
        }

        /** Which candidate to install, or hand off to the picker. */
        let chosen: GitSkillCandidate;
        if (candidates.length === 1) {
          // When autoSelectSkillName is set (Explore page install), verify
          // the single candidate actually matches the intended skill. If
          // not, the backend scan missed the target skill -- show error
          // instead of silently installing the wrong one.
          if (autoSelectSkillName) {
            const target = autoSelectSkillName.toLowerCase();
            const candidateName = candidates[0].name.toLowerCase();
            setAutoSelectSkillName(null);
            if (
              candidateName !== target &&
              !candidateName.includes(target) &&
              !target.includes(candidateName)
            ) {
              return action.fail(
                t("errors.skillNotFoundInRepo", { name: autoSelectSkillName }),
              );
            }
          }
          chosen = candidates[0];
        } else if (autoSelectSkillName) {
          // Auto-select the matching skill from online search results.
          // skills.sh name may differ from SKILL.md name (e.g.
          // "json-render-react" vs "react"), so try exact match first, then
          // containment match.
          const target = autoSelectSkillName.toLowerCase();
          const containMatches = candidates.filter((c) => {
            const n = c.name.toLowerCase();
            return target.includes(n) || n.includes(target);
          });
          const match =
            candidates.find((c) => c.name.toLowerCase() === target) ??
            (containMatches.length === 1 ? containMatches[0] : undefined);
          setAutoSelectSkillName(null);
          if (!match) {
            // No match found, fall back to picker
            openGitPicker(url, candidates);
            return action.handOff();
          }
          chosen = match;
        } else {
          openGitPicker(url, candidates);
          return action.handOff();
        }

        if (isSkillNameTaken(chosen.name)) {
          return action.fail(
            t("errors.skillAlreadyExists", { name: chosen.name }),
          );
        }
        const created = await invokeTauri<InstallResultDto>(
          "install_git_selection",
          {
            repoUrl: url,
            subpath: chosen.subpath,
            name: gitName.trim() || undefined,
          },
        );
        const deployErrors = await deployNewSkill(created, {
          noTargets: "set-error",
        });
        if (deployErrors.length > 0) showActionErrors(deployErrors);
        setGitUrl("");
        setGitName("");
        setShowAddModal(false);
        await loadManagedSkills();
      },
    );
  };

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

  /**
   * Shared core of the two "install the picked candidates" flows: install
   * each selected candidate via `installOne`, deploy it through the canonical
   * install→deploy tail, then reset that flow's pick state and finish up.
   * Validation stays in the callers.
   */
  const runBatchInstall = async <C extends { name: string; subpath: string }>(
    selected: C[],
    installOne: (candidate: C) => Promise<InstallResultDto>,
    resetPickState: () => void,
  ) => {
    await runAction(
      { successToast: t("status.selectedSkillsInstalled") },
      async () => {
        const collectedErrors: { title: string; message: string }[] = [];
        for (let i = 0; i < selected.length; i++) {
          const candidate = selected[i];
          setActionMessage(
            t("actions.importStep", {
              index: i + 1,
              total: selected.length,
              name: candidate.name,
            }),
          );
          try {
            const created = await installOne(candidate);
            collectedErrors.push(
              ...(await deployNewSkill(created, { noTargets: "collect" })),
            );
          } catch (err) {
            const raw = formatError(err) ?? "";
            collectedErrors.push({
              title: t("errors.importFailedTitle", { name: candidate.name }),
              message: raw,
            });
          }
        }

        resetPickState();
        setShowAddModal(false);
        await loadManagedSkills();
        if (collectedErrors.length > 0) showActionErrors(collectedErrors);
      },
    );
  };

  const handleInstallSelectedLocalCandidates = async () => {
    const selected = localCandidates.filter(
      (c) => c.valid && localCandidateSelected[c.subpath],
    );
    if (selected.length === 0) {
      setError(t("errors.selectAtLeastOneSkill"));
      return;
    }
    if (selected.length > 1 && localName.trim()) {
      setError(t("errors.multiSelectNoCustomName"));
      return;
    }
    if (selected.length > 1) {
      const seen = new Set<string>();
      const dup = selected.find((c) => {
        if (seen.has(c.name)) return true;
        seen.add(c.name);
        return false;
      });
      if (dup) {
        setError(t("errors.duplicateSelectedSkills", { name: dup.name }));
        return;
      }
    }
    const desiredName =
      selected.length === 1 && localName.trim()
        ? localName.trim()
        : selected[0].name;
    if (selected.length === 1 && isSkillNameTaken(desiredName)) {
      setError(t("errors.skillAlreadyExists", { name: desiredName }));
      return;
    }
    const duplicated = selected.find((c) => isSkillNameTaken(c.name));
    if (selected.length > 1 && duplicated) {
      setError(t("errors.skillAlreadyExists", { name: duplicated.name }));
      return;
    }

    await runBatchInstall(
      selected,
      (candidate) =>
        invokeTauri<InstallResultDto>("install_local_selection", {
          basePath: localCandidatesBasePath,
          subpath: candidate.subpath,
          name: localName.trim() || undefined,
        }),
      () => {
        setShowLocalPickModal(false);
        setLocalCandidates([]);
        setLocalCandidateSelected({});
        setLocalCandidatesBasePath("");
        setLocalPath("");
        setLocalName("");
      },
    );
  };

  const handleInstallSelectedCandidates = async () => {
    const selected = gitCandidates.filter(
      (c) => gitCandidateSelected[c.subpath],
    );
    if (selected.length === 0) {
      setError(t("errors.selectAtLeastOneSkill"));
      return;
    }
    const duplicated = selected.find((c) => isSkillNameTaken(c.name));
    if (duplicated) {
      setError(t("errors.skillAlreadyExists", { name: duplicated.name }));
      return;
    }
    if (selected.length > 1 && gitName.trim()) {
      setError(t("errors.multiSelectNoCustomName"));
      return;
    }

    await runBatchInstall(
      selected,
      (candidate) =>
        invokeTauri<InstallResultDto>("install_git_selection", {
          repoUrl: gitCandidatesRepoUrl,
          subpath: candidate.subpath,
          name: gitName.trim() || undefined,
        }),
      () => {
        setShowGitPickModal(false);
        setGitCandidates([]);
        setGitCandidateSelected({});
        setGitCandidatesRepoUrl("");
        setGitUrl("");
        setGitName("");
      },
    );
  };

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
    gitCandidates,
    gitCandidateSelected,
    localCandidates,
    localCandidateSelected,
    showGitPickModal,
    showLocalPickModal,
    handleOpenAdd,
    handleCloseAdd,
    handleCloseImport,
    handleReviewImport,
    handleCloseGitPick,
    handleCancelGitPick,
    handleCloseLocalPick,
    handleCancelLocalPick,
    handlePickLocalPath,
    handleToggleAllGitCandidates,
    handleToggleAllLocalCandidates,
    handleToggleGitCandidate,
    handleToggleLocalCandidate,
    handleToggleGroup,
    handleSelectVariant,
    toggleAll,
    handleImport,
    handleCreateLocal,
    handleCreateGit,
    handleExploreInstall,
    handleInstallSelectedLocalCandidates,
    handleInstallSelectedCandidates,
  };
}
