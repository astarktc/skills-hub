import { useCallback, useState } from "react";
import { installOutcome, type InstallSettlement } from "../lib/reportOutcome";
import type { InstallSource } from "../components/skills/types";
import type { DialogDoor } from "./useSurfaceState";
import type {
  ActionHandle,
  OutcomeHandlers,
  StatusReporter,
  TranslateFn,
} from "./useStatusReporter";

/** The part of an action's handle the install batch scopes Cancel with (D20). */
export type InstallBatchAction = Pick<ActionHandle, "uncancellable" | "cancelRequested">;

/** The shape every pickable candidate shares (git and local). */
export type PickCandidate = { name: string; subpath: string };

const EMPTY: never[] = [];

/** One picked candidate as the install batch takes it. */
export type InstallPick = {
  subpath: string;
  /** The candidate's own name (the report's label when it is not installed). */
  name: string;
  /** Custom name for a single selection; `null` = keep the skill's own. */
  nameOverride: string | null;
};

/**
 * Per-instance adapter: what differs between the git and the local picker.
 * `Ctx` is what the install source is built from (repo URL + listing
 * resolution, base path), captured when the picker opens so it survives
 * form edits.
 */
export type CandidatePickSource<C extends PickCandidate, Ctx> = {
  /** This picker's dialog on the surface stack. */
  dialogKind: "gitPick" | "localPick";
  /** Custom name typed in the add form; applies to a single selection only. */
  customName: string;
  /** Which candidates can be selected at all; every candidate when omitted. */
  selectable?: (candidate: C) => boolean;
  /** The install batch's source for the candidates of `ctx`. */
  installSource: (ctx: Ctx, candidates: C[]) => InstallSource;
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
    | "applyOutcome"
  >;
  /** The surface module: the picker is a dialog layered over whatever is open. */
  surface: Pick<DialogDoor, "isOpen" | "pushDialog" | "closeDialog" | "dismissDialog">;
  isSkillNameTaken: (name: string) => boolean;
  toolLabelById?: Readonly<Record<string, string>>;
  /**
   * The Add action's one batch (`installSelection`): install + deploy every
   * pick, raise the one overwrite ask over the batch's collisions, and
   * answer the settlements in pick order. `action` is the running action's
   * handle: the batch sends nothing once the action is cancelled, and
   * withdraws Cancel for the overwrite retry (no token stops it).
   */
  installBatch: (
    source: InstallSource,
    picks: InstallPick[],
    action: InstallBatchAction,
  ) => Promise<InstallSettlement[]>;
  /**
   * The parent's half of a batch's completion, carried out by the reporter's
   * one `applyOutcome` (reload first, then the toasts and the run block,
   * then close): `reload` re-reads the library (its failure is the parent's
   * nonfatal policy), `closeModal` dismisses the parent's dialog after the
   * picker has reset itself.
   */
  outcomeHandlers: Pick<OutcomeHandlers, "reload" | "closeModal">;
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
  /** Validate the selection, then install it as one backend batch. */
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
  const { t, reporter, surface, isSkillNameTaken, installBatch, outcomeHandlers, toolLabelById } = deps;
  const { isOpen, pushDialog, closeDialog, dismissDialog } = surface;
  const { dialogKind } = source;
  const {
    loading,
    runAction,
    setError,
    applyOutcome,
  } = reporter;
  const { selectable = () => true } = source;

  // The candidates and the context they were discovered in travel together.
  const [listing, setListing] = useState<{ ctx: Ctx; candidates: C[] } | null>(
    null,
  );
  const [selected, setSelected] = useState<Record<string, boolean>>({});
  // Visibility is the surface module's: the picker opens as a layer above
  // the Add dialog (or alone, from Explore's one-click install).
  const visible = isOpen(dialogKind);
  const candidates = listing?.candidates ?? EMPTY;

  const reset = useCallback(() => {
    dismissDialog(dialogKind);
    setListing(null);
    setSelected({});
  }, [dialogKind, dismissDialog]);

  const open = useCallback(
    (ctx: Ctx, next: C[]) => {
      setListing({ ctx, candidates: next });
      setSelected(
        Object.fromEntries(next.map((c) => [c.subpath, selectable(c)])),
      );
      if (!isOpen(dialogKind)) pushDialog({ kind: dialogKind });
    },
    // The selectable rule is an adapter constant (a plain predicate on the
    // candidate), not render state.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [dialogKind, isOpen, pushDialog],
  );

  const close = useCallback(() => {
    if (!loading) closeDialog(dialogKind);
  }, [closeDialog, dialogKind, loading]);

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
      { cancel: "installSelection" },
      async (action) => {
        // One backend batch for the whole selection (D12): per-candidate
        // failures and a cancelled tail are report data, and every
        // collision of the batch is one ask.
        const settlements = await installBatch(
          source.installSource(listing.ctx, picked),
          picked.map((candidate) => ({
            subpath: candidate.subpath,
            name: candidate.name,
            nameOverride: name,
          })),
          action,
        );
        // The reporter executes the completion; the picker only says how
        // its own part closes (reset, clear the form, then the parent's).
        await applyOutcome(installOutcome(settlements, { t, toolLabelById, source: "selection" }), {
          reload: outcomeHandlers.reload,
          closeModal: () => {
            reset();
            source.resetForm();
            outcomeHandlers.closeModal?.();
          },
        });
      },
    );
    // `selectable` is an adapter constant, not render state (see `open`).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    applyOutcome,
    candidates,
    installBatch,
    isSkillNameTaken,
    listing,
    outcomeHandlers,
    reset,
    runAction,
    selected,
    setError,
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
