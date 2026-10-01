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
import {
  invalidReasonText,
  rejectSelection,
  settlementComplete,
  settlementRows,
  type PickSummary,
  type PickerCandidate,
  type SettlementEntry,
  type SettlementRow,
} from "../lib/addFlow";
import { defaultImportVariantPath, repoInfo } from "../lib/skillPresentation";
import { createProgressChannel, pickFolder } from "../lib/platform";
import { invokeTauri, isTauri } from "../lib/tauri";
import { useCandidatePick, type CandidatePick } from "./useCandidatePick";
import type { SkillLibrary } from "./useSkillLibrary";
import type { SyncOrchestration } from "./useSyncOrchestration";
import type { ToolCatalog } from "./useToolCatalog";
import type { DialogDoor } from "./useSurfaceState";
import type { ActionHandle, StatusReporter, TranslateFn } from "./useStatusReporter";

export type AddSkillFlowDeps = {
  t: TranslateFn;
  reporter: StatusReporter;
  sync: Pick<
    SyncOrchestration,
    | "autoSyncEnabled"
    | "handleSyncTargetChange"
    | "resolveGlobalCollisions"
    | "scanScopeRevision"
    | "syncTargets"
  >;
  /** The Tool catalog: which Tools the scope offers, and their labels. */
  catalog: Pick<ToolCatalog, "installedTools" | "toolLabelById" | "tools">;
  library: Pick<SkillLibrary, "isSkillNameTaken" | "loadManagedSkills">;
  /** The surface module: Add and Import are its dialogs. */
  surface: Pick<DialogDoor, "isOpen" | "openDialog" | "closeDialog" | "dismissDialog">;
  /**
   * Whether the import review's invitation is still on screen (My Skills is
   * showing). A review that waited for a running action opens its dialog
   * only if it is.
   */
  isReviewWanted?: () => boolean;
};

/** Where the Add dialog is: choosing a source, picking, or reading the result. */
export type AddStep = "source" | "candidates" | "settlement";
export type AddSourceKind = "git" | "local";

/** What opens Add: a source to list at once (Explore's install), a skill to preselect. */
export type AddOpenRequest = {
  /** The source kind to start on, without a source (the empty state's "Add from folder"). */
  kind?: AddSourceKind;
  source?: { kind: "git"; url: string } | { kind: "local"; path: string };
  /** A skill name to check in the listing (the backend resolves it). */
  preselect?: string;
};

type AddDraft = { url: string; path: string; name: string };
const EMPTY_DRAFT: AddDraft = { url: "", path: "", name: "" };

/** The settlement: the source the candidates came from and one entry per sent candidate. */
type Settlement = { source: InstallSource; kind: AddSourceKind; entries: SettlementEntry[] };

/** The Add dialog's view: what it renders, and the intents it calls. */
export type AddView = {
  open: boolean;
  /** Add's own action, while it runs: the listing or the install batch. */
  running: "listing" | "installing" | null;
  step: AddStep;
  kind: AddSourceKind;
  draft: AddDraft;
  /** The source's refusal in words, shown by its field (the listing failed, nothing found). */
  sourceError: string | null;
  /** The selection's refusal in words (a taken name, an empty selection). */
  pickError: string | null;
  /** Explore asked for a skill the listing does not hold. */
  preselectMissing: string | null;
  /** The active source's picker. */
  candidates: PickerCandidate[];
  visible: PickerCandidate[];
  selected: Readonly<Record<string, boolean>>;
  query: string;
  /** The optional name applies only to a listing of one candidate. */
  nameApplies: boolean;
  settlement: SettlementRow[];
  /** Every sent candidate installed. */
  settled: boolean;
  summary: {
    /** The source, as the operator reads it (`owner/repo`, the folder). */
    source: string | null;
    candidates: PickSummary | null;
    /** The effective deploy set (the operator's selection, in registry order). */
    tools: { id: string; label: string }[];
    /** Auto-sync: whether an added skill is deployed at all. */
    deploy: boolean;
  };
  /**
   * The Tool scope's choices: the catalog's installed Tools, plus any Tool
   * the operator selected that is no longer detected (so it can be unticked).
   */
  toolChoices: { id: string; label: string; detected: boolean }[];
  selectedTools: Readonly<Record<string, boolean>>;
  setKind: (kind: AddSourceKind) => void;
  editDraft: (patch: Partial<AddDraft>) => void;
  browse: () => Promise<void>;
  /** Source → Candidates: list the source. */
  list: () => Promise<void>;
  /** Candidates → Source. */
  back: () => void;
  setQuery: (query: string) => void;
  toggle: (id: string, checked: boolean) => void;
  toggleAll: (checked: boolean) => void;
  setTool: (toolId: string, checked: boolean) => void;
  install: () => Promise<void>;
  retry: (ids: readonly string[]) => Promise<void>;
  close: () => void;
};

/**
 * Add/import world. Add is one stepped surface (ticket 16, D27): Source →
 * Candidates → Settlement in one dialog with a sticky summary (source ·
 * candidates · Tool scope). It exposes intents — `open({ source?,
 * preselect? })`, `install()`, `retry(ids)` — and the values its view
 * renders; the draft, the listing and the step are its own. Import (the
 * onboarding plan and its review) is the other half of this world.
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
    reportProgress,
    setError,
    formatError,
    applyOutcome,
  } = reporter;
  const {
    autoSyncEnabled,
    handleSyncTargetChange,
    resolveGlobalCollisions,
    scanScopeRevision,
    syncTargets,
  } = sync;
  const { installedTools, toolLabelById, tools } = catalog;
  const { isSkillNameTaken, loadManagedSkills } = library;

  const [plan, setPlan] = useState<OnboardingPlan | null>(null);
  const [selected, setSelected] = useState<Record<string, boolean>>({});
  const [variantChoice, setVariantChoice] = useState<Record<string, string>>(
    {},
  );
  const { isOpen, openDialog, closeDialog, dismissDialog } = surface;
  const showAddModal = isOpen("addSkill");
  const showImportModal = isOpen("import");

  // ---------------------------------------------------------------------
  // Add: the draft, the step and the two pickers are this hook's own.
  // ---------------------------------------------------------------------
  const [step, setStep] = useState<AddStep>("source");
  const [kind, setKindState] = useState<AddSourceKind>("git");
  const [draft, setDraft] = useState<AddDraft>(EMPTY_DRAFT);
  const [sourceError, setSourceError] = useState<string | null>(null);
  const [pickError, setPickError] = useState<string | null>(null);
  const [preselectMissing, setPreselectMissing] = useState<string | null>(null);
  const [settlement, setSettlement] = useState<Settlement | null>(null);
  const [running, setRunningState] = useState<"listing" | "installing" | null>(null);
  const runningRef = useRef<"listing" | "installing" | null>(null);
  const setRunning = useCallback((next: "listing" | "installing" | null) => {
    runningRef.current = next;
    setRunningState(next);
  }, []);
  // Read by the intents after awaits (the batch's own entries, the draft).
  const settlementRef = useRef<Settlement | null>(null);
  const draftRef = useRef<AddDraft>(EMPTY_DRAFT);
  const kindRef = useRef<AddSourceKind>("git");
  useEffect(() => {
    settlementRef.current = settlement;
    draftRef.current = draft;
    kindRef.current = kind;
  });
  const commitSettlement = useCallback((update: (current: Settlement | null) => Settlement | null) => {
    settlementRef.current = update(settlementRef.current);
    setSettlement(settlementRef.current);
  }, []);

  const toGitPicker = useCallback(
    (candidate: GitSkillCandidate): PickerCandidate => ({
      id: candidate.subpath,
      name: candidate.name,
      description: candidate.description,
      subpath: candidate.subpath,
      unavailable: null,
    }),
    [],
  );
  const toLocalPicker = useCallback(
    (candidate: LocalSkillCandidate): PickerCandidate => ({
      id: candidate.subpath,
      name: candidate.name,
      description: candidate.description,
      subpath: candidate.subpath,
      unavailable: candidate.valid ? null : invalidReasonText(candidate.reason, t),
    }),
    [t],
  );
  // Two pickers, one per source kind: switching the kind keeps each listing.
  const git = useCandidatePick<GitSkillCandidate, string>({ toPicker: toGitPicker });
  const local = useCandidatePick<LocalSkillCandidate, string>({ toPicker: toLocalPicker });
  const pick: CandidatePick<GitSkillCandidate, string> | CandidatePick<LocalSkillCandidate, string> =
    kind === "git" ? git : local;
  const { open: openGit, reset: resetGit } = git;
  const { open: openLocal, reset: resetLocal } = local;

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
   * boundary — an action cancelled before `installSelection` is sent sends
   * nothing and answers every pick `cancelled` — and withdraws Cancel for
   * the ask's retry, which no token stops.
   */
  const installBatch = useCallback(
    async (
      source: InstallSource,
      picks: readonly { subpath: string; name: string; nameOverride: string | null }[],
      action: Pick<ActionHandle, "uncancellable" | "cancelRequested">,
      onStep: (index: number) => void,
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
        onStep(progress.index);
        reportProgress({
          done: progress.index - 1,
          total: progress.total,
          message: t("installBatch.step", {
            index: progress.index,
            total: progress.total,
            name: progress.name,
          }),
        });
      });
      // The boundary: the command resets the shared token on entry, so a
      // Cancel that arrived before it would be lost there — honour it here.
      if (action.cancelRequested()) {
        return picks.map((pick) => ({ name: pick.name, status: "cancelled" }));
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
        // collided rows read as folders to review).
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
      // A candidate that never installed reads under the name it was asked.
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
      reportProgress,
      resolveGlobalCollisions,
      setError,
      t,
      toolLabelById,
    ],
  );

  /**
   * Post-mutation refresh that can never fail the action that already
   * succeeded: the bytes are installed, so a reload failure is a non-fatal
   * warning on the reporter's one-shot error channel, not a thrown failure.
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

  /** Forget everything Add holds (a fresh opening, a finished Add). */
  const resetAdd = useCallback(
    (next: { kind?: AddSourceKind; draft?: AddDraft } = {}) => {
      setStep("source");
      setKindState(next.kind ?? kindRef.current);
      setDraft(next.draft ?? EMPTY_DRAFT);
      setSourceError(null);
      setPickError(null);
      setPreselectMissing(null);
      commitSettlement(() => null);
      resetGit();
      resetLocal();
    },
    [commitSettlement, resetGit, resetLocal],
  );

  /**
   * Source → Candidates. Its own read action (the listing changes nothing),
   * cancellable at its end: a Cancel pressed during the listing discards it
   * and stays on the source. A refusal is shown by the field, never toasted.
   */
  const listSource = useCallback(
    async (sourceKind: AddSourceKind, value: string, preselect: string | null) => {
      const trimmed = value.trim();
      setSourceError(null);
      setPickError(null);
      setPreselectMissing(null);
      if (!trimmed) {
        setSourceError(t(sourceKind === "git" ? "errors.requireGitUrl" : "errors.requireLocalPath"));
        return;
      }
      await runAction(
        {
          read: true,
          label: t("activity.labels.add"),
          activity: t("add.finding"),
          cancel: "installSelection",
        },
        async (action) => {
          setRunning("listing");
          try {
            if (sourceKind === "git") {
              const { candidates, target_match } = await invokeTauri("listGitSkillsCmd", trimmed, preselect);
              if (action.cancelRequested()) return action.fail(null);
              if (candidates.length === 0) {
                setSourceError(t("errors.noSkillsFoundWithHint"));
                return;
              }
              let wanted: string[] | undefined;
              if (preselect) {
                wanted =
                  target_match?.kind === "resolved"
                    ? [target_match.subpath]
                    : target_match?.kind === "ambiguous"
                      ? target_match.subpaths
                      : [];
                // The listing does not hold the skill Explore named: say so,
                // check nothing, never install a different one silently.
                if (wanted.length === 0) setPreselectMissing(preselect);
              }
              openGit(trimmed, candidates, wanted);
            } else {
              const candidates = await invokeTauri("listLocalSkillsCmd", trimmed);
              if (action.cancelRequested()) return action.fail(null);
              if (candidates.length === 0) {
                setSourceError(t("errors.noSkillsFoundLocal"));
                return;
              }
              openLocal(trimmed, candidates);
            }
          } catch (err) {
            if (action.cancelRequested()) return action.fail(null);
            setSourceError(formatError(err) || null);
            return;
          } finally {
            setRunning(null);
          }
          setStep("candidates");
        },
      );
    },
    [formatError, openGit, openLocal, runAction, setRunning, t],
  );

  /** Open Add. A given source is listed at once (Explore's install), a named skill preselected. */
  const open = useCallback(
    (request: AddOpenRequest = {}) => {
      const { source, preselect } = request;
      const nextKind: AddSourceKind = source?.kind ?? request.kind ?? kindRef.current;
      const nextDraft: AddDraft = {
        ...EMPTY_DRAFT,
        url: source?.kind === "git" ? source.url : "",
        path: source?.kind === "local" ? source.path : "",
      };
      resetAdd({ kind: nextKind, draft: nextDraft });
      openDialog({ kind: "addSkill" });
      if (source) {
        void listSource(nextKind, source.kind === "git" ? source.url : source.path, preselect ?? null);
      }
    },
    [listSource, openDialog, resetAdd],
  );

  const list = useCallback(
    () =>
      listSource(
        kindRef.current,
        kindRef.current === "git" ? draftRef.current.url : draftRef.current.path,
        null,
      ),
    [listSource],
  );

  /** Mark one batch's entries (its ids) with a phase, or settle them. */
  const patchEntries = useCallback(
    (ids: readonly string[], patch: (entry: SettlementEntry, position: number) => SettlementEntry) => {
      const position = new Map(ids.map((id, i) => [id, i]));
      commitSettlement((current) =>
        current
          ? {
              ...current,
              entries: current.entries.map((entry) =>
                position.has(entry.id) ? patch(entry, position.get(entry.id)!) : entry,
              ),
            }
          : current,
      );
    },
    [commitSettlement],
  );

  /**
   * One batch's body: send `entries`, settle each in place, fold the run
   * through `installOutcome` (one toast, one report block). The dialog
   * closes only when every sent candidate of the settlement installed
   * cleanly; otherwise it stays on its settlement.
   */
  const settleBatch = useCallback(
    async (source: InstallSource, entries: readonly SettlementEntry[], action: ActionHandle) => {
      const ids = entries.map((entry) => entry.id);
      patchEntries(ids, (entry) => ({ ...entry, phase: "queued" }));
      const onStep = (index: number) =>
        patchEntries(ids, (entry, position) =>
          position < index - 1
            ? { ...entry, phase: "awaiting" }
            : position === index - 1
              ? { ...entry, phase: "adding" }
              : entry,
        );
      const picks = entries.map((entry) => ({
        subpath: entry.id,
        name: entry.name,
        nameOverride: entry.nameOverride,
      }));
      let settlements: InstallSettlement[];
      try {
        settlements = await installBatch(source, picks, action, onStep);
      } catch (err) {
        // The whole command failed (its roots could not be resolved):
        // nothing of it landed, and every candidate can be retried.
        settlements = picks.map((pick) => ({ name: pick.name, status: "failed", error: err }));
      }
      const bySubpath = new Map(picks.map((pick, i) => [pick.subpath, settlements[i]]));
      patchEntries(ids, (entry) => {
        const result = bySubpath.get(entry.id);
        return result ? { ...entry, phase: "settled", result } : entry;
      });
      const single = picks.length === 1 ? source.kind : "selection";
      await applyOutcome(installOutcome(settlements, { t, toolLabelById, source: single }), {
        reload: () => refreshWithoutFailingAction(loadManagedSkills),
        closeModal: () => {
          const current = settlementRef.current;
          if (!current || !settlementComplete(current.entries)) return;
          dismissDialog("addSkill");
          resetAdd();
        },
      });
    },
    [
      applyOutcome,
      dismissDialog,
      installBatch,
      loadManagedSkills,
      patchEntries,
      refreshWithoutFailingAction,
      resetAdd,
      t,
      toolLabelById,
    ],
  );

  /** Send `entries` as one batch under ONE action, cancellable between candidates (D20). */
  const runBatch = useCallback(
    async (source: InstallSource, entries: readonly SettlementEntry[]) => {
      await runAction(
        { cancel: "installSelection", label: t("activity.labels.add"), activity: t("activity.adding") },
        async (action) => {
          setRunning("installing");
          try {
            await settleBatch(source, entries, action);
          } finally {
            setRunning(null);
          }
        },
      );
    },
    [runAction, setRunning, settleBatch, t],
  );

  /** Candidates → Settlement: validate the visible, checked selection and add it as one batch. */
  const install = useCallback(async () => {
    const ctx = pick.ctx;
    if (!ctx || loading) return;
    const picked = pick.picked;
    const listedOne = pick.listed.length === 1;
    const customName = listedOne ? draftRef.current.name.trim() : "";
    const rejection = rejectSelection(picked, customName, isSkillNameTaken, t);
    setPickError(rejection);
    if (rejection) return;
    const source: InstallSource =
      kindRef.current === "git"
        ? {
            kind: "git",
            url: ctx,
            // Every candidate of one listing carries the same source resolution.
            resolution: git.picked[0]?.resolution ?? null,
          }
        : { kind: "local", root: ctx };
    const entries: SettlementEntry[] = picked.map((candidate) => ({
      id: candidate.subpath,
      name: customName || candidate.name,
      nameOverride: customName || null,
      phase: "queued",
    }));
    commitSettlement(() => ({ source, kind: kindRef.current, entries }));
    setStep("settlement");
    await runBatch(source, entries);
  }, [commitSettlement, git.picked, isSkillNameTaken, loading, pick, runBatch, t]);

  /** Re-send the named candidates of the settlement that did not install (a failure, a cancellation). */
  const retry = useCallback(
    async (ids: readonly string[]) => {
      const current = settlementRef.current;
      if (!current || loading) return;
      const wanted = new Set(ids);
      const entries = current.entries.filter(
        (entry) =>
          wanted.has(entry.id) && entry.phase === "settled" && entry.result.status !== "installed",
      );
      if (entries.length === 0) return;
      await runBatch(current.source, entries);
    },
    [loading, runBatch],
  );

  /** The operator's Close (never while the action runs: its Cancel is in the summary). */
  const close = useCallback(() => {
    if (runningRef.current) return;
    closeDialog("addSkill");
    resetAdd();
  }, [closeDialog, resetAdd]);

  const setKind = useCallback((next: AddSourceKind) => {
    setKindState(next);
    setSourceError(null);
  }, []);
  const editDraft = useCallback((patch: Partial<AddDraft>) => {
    setDraft((prev) => ({ ...prev, ...patch }));
    if (patch.url !== undefined || patch.path !== undefined) setSourceError(null);
    if (patch.name !== undefined) setPickError(null);
  }, []);
  const back = useCallback(() => {
    if (runningRef.current) return;
    setStep("source");
    setPickError(null);
  }, []);

  const browse = useCallback(async () => {
    try {
      const [chosen] = await pickFolder({ title: t("selectLocalFolder") });
      if (chosen) editDraft({ path: chosen });
    } catch (err) {
      setError(formatError(err));
    }
  }, [editDraft, formatError, setError, t]);

  const { setQuery, toggle: togglePick, toggleAll: toggleAllPick } = pick;
  const toggle = useCallback(
    (id: string, checked: boolean) => {
      togglePick(id, checked);
      setPickError(null);
    },
    [togglePick],
  );
  const toggleAll = useCallback(
    (checked: boolean) => {
      toggleAllPick(checked);
      setPickError(null);
    },
    [toggleAllPick],
  );
  const setTool = useCallback(
    (toolId: string, checked: boolean) => void handleSyncTargetChange(toolId, checked),
    [handleSyncTargetChange],
  );

  const settlementEntries = settlement?.entries;
  const rows = useMemo(
    () => (settlementEntries ? settlementRows(settlementEntries, { t, toolLabelById }) : []),
    [settlementEntries, t, toolLabelById],
  );
  const scopeTools = useMemo(
    () =>
      tools
        .filter((tool) => syncTargets[tool.id])
        .map((tool) => ({ id: tool.id, label: toolLabelById[tool.id] ?? tool.label })),
    [syncTargets, toolLabelById, tools],
  );
  const toolChoices = useMemo(() => {
    const detected = new Set(installedTools.map((tool) => tool.id));
    return tools
      .filter((tool) => detected.has(tool.id) || syncTargets[tool.id])
      .map((tool) => ({ id: tool.id, label: toolLabelById[tool.id] ?? tool.label, detected: detected.has(tool.id) }));
  }, [installedTools, syncTargets, toolLabelById, tools]);
  const sourceValue = settlement
    ? settlement.source.kind === "git"
      ? settlement.source.url
      : settlement.source.root
    : pick.ctx;
  const sourceLabel = sourceValue ? (repoInfo(sourceValue)?.label ?? sourceValue) : null;

  const add: AddView = {
    open: showAddModal,
    running,
    step,
    kind,
    draft,
    sourceError,
    pickError,
    preselectMissing,
    candidates: pick.candidates,
    visible: pick.visible,
    selected: pick.selected,
    query: pick.query,
    nameApplies: pick.listed.length === 1,
    settlement: rows,
    settled: settlementEntries ? settlementComplete(settlementEntries) : false,
    summary: {
      source: sourceLabel,
      candidates: settlement
        ? { total: settlement.entries.length, checked: settlement.entries.length, hidden: 0, hiddenChecked: 0 }
        : pick.ctx
          ? pick.summary
          : null,
      tools: scopeTools,
      deploy: autoSyncEnabled,
    },
    toolChoices,
    selectedTools: syncTargets,
    setKind,
    editDraft,
    browse,
    list,
    back,
    setQuery,
    toggle,
    toggleAll,
    setTool,
    install,
    retry,
    close,
  };

  // ---------------------------------------------------------------------
  // Import: the onboarding plan and its review.
  // ---------------------------------------------------------------------

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

  /** Modals stay put while their action runs (its progress and Cancel are at the dialog's foot). */
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

  const toggleAllGroups = useCallback(
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

  return {
    // Add — intents and the dialog's view.
    open,
    install,
    retry,
    add,
    // Import.
    plan,
    selected,
    variantChoice,
    showImportModal,
    handleCloseImport,
    handleReviewImport,
    handleToggleGroup,
    handleSelectVariant,
    toggleAll: toggleAllGroups,
    handleImport,
  };
}
