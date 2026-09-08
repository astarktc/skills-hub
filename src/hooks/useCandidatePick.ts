import { useCallback, useState } from "react";
import { installOutcome, type InstallDeployment, type InstallSettlement, type Outcome } from "../lib/reportOutcome";
import type { InstallResultDto } from "../components/skills/types";
import type {
  StatusReporter,
  TranslateFn,
} from "./useStatusReporter";

/** The shape every pickable candidate shares (git and local). */
export type PickCandidate = { name: string; subpath: string };

const EMPTY: never[] = [];

/**
 * Per-instance adapter: what differs between the git and the local picker.
 * `Ctx` is what `installOne` needs besides the candidate (repo URL, base
 * path), captured when the picker opens so it survives form edits.
 */
export type CandidatePickSource<C extends PickCandidate, Ctx> = {
  /** Custom name typed in the add form; applies to a single selection only. */
  customName: string;
  /** Which candidates can be selected at all; every candidate when omitted. */
  selectable?: (candidate: C) => boolean;
  installOne: (
    ctx: Ctx,
    candidate: C,
    /** Custom name for a single selection; `null` = keep the skill's own. */
    name: string | null,
  ) => Promise<InstallResultDto>;
  /** Clear the add-form fields this flow owns, after a successful batch. */
  resetForm: () => void;
};

/** What the module needs from its world (shared by both instances). */
export type CandidatePickDeps = {
  t: TranslateFn;
  reporter: Pick<
    StatusReporter,
    | "loading"
    | "runAction"
    | "setActionMessage"
    | "setError"
    | "notify"
    | "showActionWarnings"
    | "showActionErrors"
  >;
  isSkillNameTaken: (name: string) => boolean;
  toolLabelById?: Readonly<Record<string, string>>;
  /** The install→deploy tail returns data, including whole-command failure. */
  deploy: (created: InstallResultDto) => Promise<InstallDeployment>;
  /** Runs once after every batch (close the add modal, reload the library). */
  afterBatch: (completion: Outcome["completion"]) => Promise<void>;
};

export type CandidatePick<C extends PickCandidate, Ctx> = {
  candidates: C[];
  /** Selection keyed by subpath (the candidate identity). */
  selected: Record<string, boolean>;
  visible: boolean;
  /** Hand the flow to the picker: keep the candidates and their context, preselect the selectable ones, show. */
  open: (ctx: Ctx, candidates: C[]) => void;
  /** Hide the picker, keeping its state. No-op while an action runs. */
  close: () => void;
  /** Hide the picker and discard its state. No-op while an action runs. */
  cancel: () => void;
  toggle: (subpath: string, checked: boolean) => void;
  toggleAll: (checked: boolean) => void;
  /** Validate the selection, then install it as one batch. */
  install: () => Promise<void>;
};

/**
 * The names a selection would install under: the custom name (if any) for a
 * single pick, each candidate's own name otherwise.
 */
const desiredNames = <C extends PickCandidate>(
  selected: C[],
  customName: string,
) =>
  selected.length === 1 && customName
    ? [customName]
    : selected.map((c) => c.name);

/**
 * The one selection validation, applied to git and local alike. Returns the
 * localized rejection, or null when the selection may be installed.
 */
const rejectSelection = <C extends PickCandidate>(
  selected: C[],
  customName: string,
  isSkillNameTaken: (name: string) => boolean,
  t: TranslateFn,
): string | null => {
  if (selected.length === 0) return t("errors.selectAtLeastOneSkill");
  if (selected.length > 1 && customName) {
    return t("errors.multiSelectNoCustomName");
  }
  const seen = new Set<string>();
  for (const c of selected) {
    if (seen.has(c.name)) {
      return t("errors.duplicateSelectedSkills", { name: c.name });
    }
    seen.add(c.name);
  }
  const taken = desiredNames(selected, customName).find(isSkillNameTaken);
  return taken ? t("errors.skillAlreadyExists", { name: taken }) : null;
};

/**
 * One candidate picker: the list, the selection, the modal visibility, the
 * shared selection validation and the batch install. Instantiated once per
 * source (git, local) by the add flow; it is a building block of that world,
 * not a world hook of its own.
 */
export function useCandidatePick<C extends PickCandidate, Ctx>(
  source: CandidatePickSource<C, Ctx>,
  deps: CandidatePickDeps,
): CandidatePick<C, Ctx> {
  const { t, reporter, isSkillNameTaken, deploy, afterBatch, toolLabelById } = deps;
  const {
    loading,
    runAction,
    setActionMessage,
    setError,
    notify,
    showActionWarnings,
    showActionErrors,
  } = reporter;
  const { selectable = () => true } = source;

  // The candidates and the context they were discovered in travel together.
  const [listing, setListing] = useState<{ ctx: Ctx; candidates: C[] } | null>(
    null,
  );
  const [selected, setSelected] = useState<Record<string, boolean>>({});
  const [visible, setVisible] = useState(false);
  const candidates = listing?.candidates ?? EMPTY;

  const reset = useCallback(() => {
    setVisible(false);
    setListing(null);
    setSelected({});
  }, []);

  const open = useCallback(
    (ctx: Ctx, next: C[]) => {
      setListing({ ctx, candidates: next });
      setSelected(
        Object.fromEntries(next.map((c) => [c.subpath, selectable(c)])),
      );
      setVisible(true);
    },
    // The selectable rule is an adapter constant (a plain predicate on the
    // candidate), not render state.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  const close = useCallback(() => {
    if (!loading) setVisible(false);
  }, [loading]);

  const cancel = useCallback(() => {
    if (!loading) reset();
  }, [loading, reset]);

  const toggle = useCallback((subpath: string, checked: boolean) => {
    setSelected((prev) => ({ ...prev, [subpath]: checked }));
  }, []);

  const toggleAll = useCallback(
    (checked: boolean) => {
      setSelected(
        Object.fromEntries(
          candidates.map((c) => [c.subpath, selectable(c) && checked]),
        ),
      );
    },
    // See `open`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [candidates],
  );

  const install = useCallback(async () => {
    if (!listing) return;
    const picked = candidates.filter(
      (c) => selectable(c) && selected[c.subpath],
    );
    const customName = source.customName.trim();
    const rejection = rejectSelection(picked, customName, isSkillNameTaken, t);
    if (rejection) {
      setError(rejection);
      return;
    }
    const name = picked.length === 1 && customName ? customName : null;
    await runAction(
      {},
      async () => {
        const settlements: InstallSettlement[] = [];
        for (const [i, candidate] of picked.entries()) {
          setActionMessage(
            t("actions.importStep", {
              index: i + 1,
              total: picked.length,
              name: candidate.name,
            }),
          );
          try {
            const created = await source.installOne(
              listing.ctx,
              candidate,
              name,
            );
            settlements.push({ status: "installed", name: candidate.name, result: created, deployment: await deploy(created) });
          } catch (err) {
            settlements.push({ status: "failed", name: candidate.name, error: err });
          }
        }
        const outcome = installOutcome(settlements, { t, toolLabelById, source: "selection" });
        if (outcome.completion.closeModal) {
          reset();
          source.resetForm();
        }
        await afterBatch(outcome.completion);
        showActionErrors(outcome.errors);
        showActionWarnings(outcome.warnings);
        if (outcome.toast) notify(outcome.toast.kind, outcome.toast.message, outcome.toast.detail);
      },
    );
    // `selectable` is an adapter constant, not render state (see `open`).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    afterBatch,
    candidates,
    deploy,
    notify,
    showActionWarnings,
    isSkillNameTaken,
    listing,
    reset,
    runAction,
    selected,
    setActionMessage,
    setError,
    showActionErrors,
    source,
    t,
    toolLabelById,
  ]);

  return {
    candidates,
    selected,
    visible,
    open,
    close,
    cancel,
    toggle,
    toggleAll,
    install,
  };
}
