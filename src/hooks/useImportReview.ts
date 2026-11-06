import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ImportProgressDto, ImportReport, OnboardingPlan, OnboardingSelectionDto } from "../components/skills/types";
import { importOutcome } from "../lib/reportOutcome";
import {
  defaultChoices,
  failedGroups,
  importedSkillIds,
  importPolicy,
  importResultRows,
  importReviewModel,
  importSelections,
  keptByChoice,
  mergeImportReports,
  type ImportChoices,
  type ImportResultRow,
  type ImportReviewModel,
} from "../lib/importReview";
import { createProgressChannel } from "../lib/platform";
import { invokeTauri, isTauri } from "../lib/tauri";
import type { CollisionPart, SyncOrchestration } from "./useSyncOrchestration";
import type { ToolCatalog } from "./useToolCatalog";
import type { DialogDoor } from "./useSurfaceState";
import type { StatusReporter, TranslateFn } from "./useStatusReporter";

export type ImportReviewDeps = {
  t: TranslateFn;
  reporter: Pick<StatusReporter, "runAction" | "setActionMessage" | "notifyError" | "applyOutcome" | "loading">;
  sync: Pick<SyncOrchestration, "autoSyncEnabled" | "scanScopeRevision" | "settleCollisionParts">;
  catalog: Pick<ToolCatalog, "toolLabelById">;
  /** Auto-deploy's harnesses: the operator's recorded selection, at the moment of the import. */
  deployTools: () => string[];
  loadManagedSkills: () => Promise<unknown>;
  surface: Pick<DialogDoor, "isOpen" | "openDialog" | "closeDialog">;
  /** Whether the review's invitation is still on screen (see `useAddSkillFlow`). */
  isReviewWanted?: () => boolean;
};

export type ImportStep = "review" | "result";

/**
 * The import cascade (DESIGN.md § Empty and first-run): after a large import
 * with auto-deploy, the imported rows' strips light mark by mark, once.
 * `token` keys the one play; `null` when nothing plays.
 */
export type ImportLanding = { ids: ReadonlySet<string>; token: number } | null;

/** An import of at least this many skills earns the cascade (D6's rare moment). */
export const LANDING_MIN_SKILLS = 10;
/** How long the cascade's marks keep their entry styles: past the last mark's delay. */
const LANDING_MS = 1200;

/** The Import review dialog's view: what it renders, and the intents it calls. */
export type ImportReviewView = {
  open: boolean;
  /** The import's own action runs (its progress sits at the dialog's foot). */
  running: boolean;
  step: ImportStep;
  /** The plan under review; `null` before the first scan. */
  plan: OnboardingPlan | null;
  model: ImportReviewModel | null;
  /** The Result: the composite count line over every run of this opening, and one row per sent group. */
  result: { headline: string; rows: ImportResultRow[] } | null;
  toggle: (name: string, checked: boolean) => void;
  toggleAll: (checked: boolean) => void;
  /** Choose the copy that becomes the Library skill (checks its group). */
  chooseVariant: (name: string, path: string) => void;
  setReplace: (name: string, replace: boolean) => void;
  setRemoveOthers: (name: string, remove: boolean) => void;
  setRemoveOriginals: (remove: boolean) => void;
  importSelected: () => Promise<void>;
  retry: (names: readonly string[]) => Promise<void>;
  close: () => void;
};

const EMPTY_CHOICES: ImportChoices = { selected: {}, variant: {}, replace: {}, removeOthers: {}, removeOriginals: true };

/**
 * Import (the onboarding plan and its review, ticket 21) — a building block
 * of the Add world. The banner's plan is loaded on mount and whenever the
 * scan's scope changes; Review always reviews a plan fetched now. The review
 * is one stepped dialog, Review → Result: the import is one backend call, its
 * `TARGET_EXISTS` rows (a divergent sibling where a deploy lands, a Replace
 * over a copy that differs) raise **one** overwrite ask (D12) whose answers
 * settle inside the report, and the Result shows every sent group with the
 * composite count line; Retry re-sends the failed groups alone.
 */
export function useImportReview({
  t,
  reporter,
  sync,
  catalog,
  deployTools,
  loadManagedSkills,
  surface,
  isReviewWanted,
}: ImportReviewDeps) {
  const { runAction, setActionMessage, notifyError, applyOutcome } = reporter;
  const { autoSyncEnabled, scanScopeRevision, settleCollisionParts } = sync;
  const { toolLabelById } = catalog;
  const { isOpen, openDialog, closeDialog } = surface;
  const open = isOpen("import");

  const [plan, setPlan] = useState<OnboardingPlan | null>(null);
  const [choices, setChoices] = useState<ImportChoices>(EMPTY_CHOICES);
  const [step, setStep] = useState<ImportStep>("review");
  const [running, setRunning] = useState(false);
  /** This opening's runs, merged (a Retry replaces its groups' outcomes). */
  const [report, setReport] = useState<ImportReport | null>(null);
  const reportRef = useRef<ImportReport | null>(null);
  /** What was sent, per group: a Retry re-sends exactly that. */
  const sentRef = useRef<{
    selections: Map<string, OnboardingSelectionDto>;
    autoDeploy: boolean;
    tools: string[];
    removeOriginals: boolean;
    /** The Conflicts whose other copies the operator chose to keep (F7). */
    keptByChoice: ReadonlySet<string>;
  } | null>(null);
  /** The groups whose last import failed (the review annotates and deselects them). */
  const failuresRef = useRef<Record<string, string>>({});
  const [failures, setFailures] = useState<Record<string, string>>({});
  const [landing, setLanding] = useState<ImportLanding>(null);
  /** The sent Conflicts whose other copies the operator chose to keep (F7). */
  const [keptChoice, setKeptChoice] = useState<ReadonlySet<string>>(() => new Set());

  const fetchPlan = useCallback(async () => {
    const result = await invokeTauri("getOnboardingPlan");
    setPlan(result);
    setChoices(defaultChoices(result, failuresRef.current));
    return result;
  }, []);

  const loadPlan = useCallback(
    () => runAction({ read: true, label: t("activity.labels.scan"), activity: t("activity.scanning") }, fetchPlan),
    [fetchPlan, runAction, t],
  );

  useEffect(() => {
    if (!isTauri) return;
    // On mount, and whenever the saved harness configuration changes: the
    // backend resolves the scan's scope from it, so the banner, its count and
    // the reviewed plan follow the configuration.
    void loadPlan();
  }, [loadPlan, scanScopeRevision]);

  // The cascade plays once: its entry styles are withdrawn after it ran.
  useEffect(() => {
    if (!landing) return;
    const timer = setTimeout(() => setLanding(null), LANDING_MS);
    return () => clearTimeout(timer);
  }, [landing]);

  /**
   * Review always reviews a plan fetched now: the import re-derives the plan
   * under the current scope and acts on the originals it finds, so the
   * operator must have seen exactly those. The banner's plan is only the
   * invitation.
   */
  const reviewGenerationRef = useRef(0);
  const openReview = useCallback(async () => {
    const generation = ++reviewGenerationRef.current;
    const stillWanted = () => reviewGenerationRef.current === generation && (isReviewWanted?.() ?? true);
    const result = await runAction(
      { read: true, stillWanted, label: t("activity.labels.scan"), activity: t("activity.scanning") },
      fetchPlan,
    );
    if (result && stillWanted()) {
      setStep("review");
      reportRef.current = null;
      setReport(null);
      openDialog({ kind: "import" });
    }
  }, [fetchPlan, isReviewWanted, openDialog, runAction, t]);

  /**
   * The one overwrite ask over the import's collisions (D12): each imported
   * or replaced group with a `TARGET_EXISTS` row is one sync part (its skill
   * exists by now; the retry syncs from its central copy under the import's
   * same-content rule). The answers' rows replace the asked ones in the
   * report. A thrown retry is reported and the report stands as it settled.
   */
  const settleCollisions = useCallback(
    async (answer: ImportReport): Promise<ImportReport> => {
      const indexed: { index: number; part: CollisionPart }[] = [];
      answer.groups.forEach((group, index) => {
        const status = group.status;
        if (status.status !== "imported" && status.status !== "replaced") return;
        if (!status.targets.some((row) => row.status.status === "failed" && row.status.error.code === "TARGET_EXISTS")) return;
        indexed.push({
          index,
          part: {
            kind: "sync",
            report: status.targets,
            skills: [{ skill_id: status.skill_id, name: status.skill_name, source_path: status.central_path }],
            policy: { overwriteIfSameContent: true },
          },
        });
      });
      if (indexed.length === 0) return answer;
      try {
        const { parts } = await settleCollisionParts(indexed.map((entry) => entry.part));
        const groups = [...answer.groups];
        indexed.forEach(({ index }, i) => {
          const settled = parts[i];
          const group = groups[index];
          if (settled.kind !== "sync" || (group.status.status !== "imported" && group.status.status !== "replaced")) return;
          groups[index] = { ...group, status: { ...group.status, targets: settled.report } };
        });
        return { groups };
      } catch (err) {
        notifyError(err);
        return answer;
      }
    },
    [notifyError, settleCollisionParts],
  );

  /** Send selections under a policy; the Result folds this run into the opening's runs. */
  const send = useCallback(
    async (selections: OnboardingSelectionDto[], policy: { autoDeploy: boolean; tools: string[]; removeOriginals: boolean }) => {
      await runAction({ label: t("activity.labels.import"), activity: t("activity.importing") }, async () => {
        setRunning(true);
        try {
          const onProgress = await createProgressChannel<ImportProgressDto>((progress) => {
            setActionMessage(
              t(progress.phase === "admitting" ? "actions.importStep" : "actions.importApplyStep", {
                index: progress.index,
                total: progress.total,
                name: progress.group_name,
              }),
            );
          });
          const answer = await invokeTauri(
            "importOnboardingSelection",
            selections,
            importPolicy({ ...EMPTY_CHOICES, removeOriginals: policy.removeOriginals }, { autoDeploy: policy.autoDeploy, tools: policy.tools }),
            onProgress,
          );
          const settled = await settleCollisions(answer);
          const merged = reportRef.current ? mergeImportReports(reportRef.current, settled) : settled;
          reportRef.current = merged;
          setReport(merged);
          // Failed groups are remembered for the next review; a group that
          // imported or was retried cleanly is forgotten.
          const nextFailures = { ...failuresRef.current };
          for (const group of settled.groups) delete nextFailures[group.group_name];
          Object.assign(nextFailures, failedGroups(settled, t));
          failuresRef.current = nextFailures;
          setFailures(nextFailures);
          setStep("result");
          await applyOutcome(importOutcome(settled, { t, toolLabelById, keptByChoice: sentRef.current?.keptByChoice }), {
            reload: async () => {
              // The import already happened: a reload failure is its own
              // notice, never the import's failure.
              try {
                await loadManagedSkills();
                await fetchPlan();
              } catch (err) {
                notifyError(err);
              }
            },
            // The dialog stays on its Result: the operator closes it, and
            // the run's toast waits until then (F5).
            resultShownIn: "import",
          });
        } finally {
          setRunning(false);
        }
      });
    },
    [applyOutcome, fetchPlan, loadManagedSkills, notifyError, runAction, setActionMessage, settleCollisions, t, toolLabelById],
  );

  const importSelected = useCallback(async () => {
    if (!plan) return;
    const selections = importSelections(plan, choices);
    if (selections.length === 0) return;
    const policy = { autoDeploy: autoSyncEnabled, tools: autoSyncEnabled ? deployTools() : [], removeOriginals: choices.removeOriginals };
    sentRef.current = {
      selections: new Map(selections.map((selection) => [selection.group_name, selection])),
      ...policy,
      keptByChoice: keptByChoice(plan, selections),
    };
    setKeptChoice(sentRef.current.keptByChoice);
    reportRef.current = null;
    setReport(null);
    await send(selections, policy);
  }, [autoSyncEnabled, choices, deployTools, plan, send]);

  const retry = useCallback(
    async (names: readonly string[]) => {
      const sent = sentRef.current;
      if (!sent) return;
      const selections = names.flatMap((name) => {
        const selection = sent.selections.get(name);
        return selection ? [selection] : [];
      });
      if (selections.length === 0) return;
      await send(selections, sent);
    },
    [send],
  );

  const close = useCallback(() => {
    if (running) return;
    const done = reportRef.current;
    const sent = sentRef.current;
    if (step === "result" && done && sent?.autoDeploy) {
      const ids = importedSkillIds(done);
      if (ids.length >= LANDING_MIN_SKILLS) setLanding({ ids: new Set(ids), token: Date.now() });
    }
    closeDialog("import");
  }, [closeDialog, running, step]);

  const toggle = useCallback((name: string, checked: boolean) => {
    setChoices((prev) => ({ ...prev, selected: { ...prev.selected, [name]: checked } }));
  }, []);
  const toggleAll = useCallback(
    (checked: boolean) => {
      if (!plan) return;
      setChoices((prev) => {
        const selected = { ...prev.selected };
        for (const group of plan.groups) if (!group.name_is_managed) selected[group.name] = checked;
        return { ...prev, selected };
      });
    },
    [plan],
  );
  const chooseVariant = useCallback((name: string, path: string) => {
    setChoices((prev) => ({
      ...prev,
      variant: { ...prev.variant, [name]: path },
      selected: { ...prev.selected, [name]: true },
    }));
  }, []);
  const setReplace = useCallback((name: string, replace: boolean) => {
    setChoices((prev) => ({ ...prev, replace: { ...prev.replace, [name]: replace } }));
  }, []);
  const setRemoveOthers = useCallback((name: string, remove: boolean) => {
    setChoices((prev) => ({ ...prev, removeOthers: { ...prev.removeOthers, [name]: remove } }));
  }, []);
  const setRemoveOriginals = useCallback((remove: boolean) => {
    setChoices((prev) => ({ ...prev, removeOriginals: remove }));
  }, []);

  const tools = autoSyncEnabled ? deployTools() : [];
  const toolsKey = tools.join("\n");
  const model = useMemo(
    () =>
      plan
        ? importReviewModel(plan, choices, {
            t,
            toolLabelById,
            autoDeploy: autoSyncEnabled,
            deployTools: toolsKey ? toolsKey.split("\n") : [],
            lastFailures: failures,
          })
        : null,
    [autoSyncEnabled, choices, failures, plan, t, toolLabelById, toolsKey],
  );
  const result = useMemo(() => {
    if (!report) return null;
    return {
      headline: importOutcome(report, { t, toolLabelById, keptByChoice: keptChoice }).toast?.message ?? "",
      // Every reported group was sent, so Retry can re-send it.
      rows: importResultRows(report, { t, toolLabelById, keptByChoice: keptChoice }),
    };
  }, [keptChoice, report, t, toolLabelById]);

  const view: ImportReviewView = {
    open,
    running,
    step,
    plan,
    model,
    result,
    toggle,
    toggleAll,
    chooseVariant,
    setReplace,
    setRemoveOthers,
    setRemoveOriginals,
    importSelected,
    retry,
    close,
  };

  return { plan, openReview, view, landing, fetchPlan };
}
