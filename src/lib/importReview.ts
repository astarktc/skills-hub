/**
 * The Import review's pure folds (ticket 21, D13 / D14): what the review
 * shows for an onboarding plan and the operator's choices, what the import
 * sends, and how its report reads on the Result step. The flow hook
 * (`useImportReview`) holds the state; these decide from it.
 *
 * - **Rows**: a group whose name is new is an *import* row (a checkbox; a
 *   Conflict's copies are radios), a group whose name the Library already
 *   holds is a *managed* row (Leave / Replace), an invalid manifest is a
 *   disabled row with its reason in words.
 * - **What each badge compares**: Match / Conflict compare the copies with
 *   *each other* (the plan's `has_conflict`); a managed row compares each copy
 *   with *your managed copy* (`same_as_managed`, the content identity rule).
 * - **Fates**: every copy of a group that goes ahead says what happens to it —
 *   the chosen copy becomes the Library skill; a copy with the same bytes
 *   follows the originals policy (linked under auto-deploy, else removed or
 *   left in place by the per-run checkbox); a copy that differs follows the
 *   group's keep / remove choice, and under auto-deploy a kept one in a
 *   deploy harness is asked about before a link replaces it (D12).
 */

import { describeCommandError } from "../commandError";
import type {
  BatchTargetOutcome,
  ImportGroupOutcome,
  ImportPolicyDto,
  ImportReport,
  OnboardingGroup,
  OnboardingPlan,
  OnboardingSelectionDto,
  OnboardingVariant,
  OriginalOutcome,
} from "../components/skills/types";
import { invalidReasonText } from "./addFlow";
import { importGroupKind, isImportTargetFailure, type ImportGroupKind } from "./reportOutcome";
import { defaultImportVariantPath } from "./skillPresentation";

type Translate = (key: string, opts?: Record<string, unknown>) => string;
type Labels = Readonly<Record<string, string>>;

// ---------------------------------------------------------------------------
// Choices
// ---------------------------------------------------------------------------

/** The operator's choices in the review, per group name. */
export type ImportChoices = {
  /** Import this group (a name the Library does not hold). */
  selected: Readonly<Record<string, boolean>>;
  /** The copy (its path) that becomes the Library skill. */
  variant: Readonly<Record<string, string>>;
  /** A managed name: Replace the copies with the managed skill (absent = Leave). */
  replace: Readonly<Record<string, boolean>>;
  /** A Conflict: remove the copies that differ from the chosen one (absent = keep). */
  removeOthers: Readonly<Record<string, boolean>>;
  /** Auto-deploy off: remove the identical originals (D13's per-run checkbox). */
  removeOriginals: boolean;
};

/**
 * The review's starting choices (D14): a new name whose copies agree is
 * checked; a Conflict and a managed name start unchecked (Leave); a group
 * whose last import failed starts unchecked too. Each group's copy is the
 * default one; the other copies of a Conflict are kept; the originals policy
 * is today's — remove the identical ones.
 */
export function defaultChoices(plan: OnboardingPlan, failed: Readonly<Record<string, string>> = {}): ImportChoices {
  const selected: Record<string, boolean> = {};
  const variant: Record<string, string> = {};
  for (const group of plan.groups) {
    selected[group.name] = !group.name_is_managed && !group.has_conflict && !(group.name in failed);
    const chosen = defaultImportVariantPath(group);
    if (chosen) variant[group.name] = chosen;
  }
  return { selected, variant, replace: {}, removeOthers: {}, removeOriginals: true };
}

const chosenPathOf = (group: OnboardingGroup, choices: ImportChoices) =>
  choices.variant[group.name] ?? defaultImportVariantPath(group) ?? null;

/** Whether a copy carries the chosen copy's bytes (the chosen one included). */
function sameAsChosen(group: OnboardingGroup, variant: OnboardingVariant, chosenPath: string | null): boolean {
  if (variant.path === chosenPath) return true;
  if (!group.has_conflict) return true;
  const chosen = group.variants.find((candidate) => candidate.path === chosenPath);
  return chosen?.fingerprint != null && chosen.fingerprint === variant.fingerprint;
}

/** What the import sends: one selection per checked group and per Replace. */
export function importSelections(plan: OnboardingPlan, choices: ImportChoices): OnboardingSelectionDto[] {
  const selections: OnboardingSelectionDto[] = [];
  for (const group of plan.groups) {
    const chosen = chosenPathOf(group, choices);
    if (!chosen) continue;
    if (group.name_is_managed) {
      if (choices.replace[group.name]) selections.push({ group_name: group.name, chosen_path: chosen, name: null, replace: true });
      continue;
    }
    if (!choices.selected[group.name]) continue;
    selections.push({
      group_name: group.name,
      chosen_path: chosen,
      name: null,
      ...(group.has_conflict && choices.removeOthers[group.name] ? { remove_divergent: true } : {}),
    });
  }
  return selections;
}

/** The import's policy: auto-deploy (with its harnesses) or the per-run originals choice. */
export function importPolicy(choices: ImportChoices, ctx: { autoDeploy: boolean; tools: readonly string[] }): ImportPolicyDto {
  return {
    auto_sync: ctx.autoDeploy,
    tools: ctx.autoDeploy ? [...ctx.tools] : null,
    remove_originals: choices.removeOriginals,
  };
}

/**
 * The groups whose different copies the operator chose to keep: a Conflict
 * the review offered the keep / remove choice for, sent without
 * `remove_divergent`. Their `kept_divergent` originals are the operator's
 * answer, not something to review (audit F7).
 */
export function keptByChoice(plan: OnboardingPlan, selections: readonly OnboardingSelectionDto[]): Set<string> {
  const conflicts = new Set(plan.groups.filter((group) => group.has_conflict && !group.name_is_managed).map((group) => group.name));
  return new Set(
    selections.filter((selection) => conflicts.has(selection.group_name) && !selection.remove_divergent).map((selection) => selection.group_name),
  );
}

// ---------------------------------------------------------------------------
// The review
// ---------------------------------------------------------------------------

/** What happens to one copy if its group goes ahead. */
export type CopyFate =
  /** The chosen copy, then the originals policy. */
  | "chosenLinked"
  | "chosenRemoved"
  | "chosenLeft"
  /** A copy with the chosen bytes (or the managed bytes, under Replace). */
  | "linked"
  | "removed"
  | "leftInPlace"
  /** A copy that differs, in a harness the deploy reaches: the overwrite ask decides. */
  | "askFirst";

export const COPY_FATE_KEY: Readonly<Record<CopyFate, string>> = {
  chosenLinked: "importReview.fate.chosenLinked",
  chosenRemoved: "importReview.fate.chosenRemoved",
  chosenLeft: "importReview.fate.chosenLeft",
  linked: "importReview.fate.linked",
  removed: "importReview.fate.removed",
  leftInPlace: "importReview.fate.leftInPlace",
  askFirst: "importReview.fate.askFirst",
};

/** A Match group's copies all share one fate, said once for the row. */
export type GroupFate = "linked" | "removed" | "leftInPlace";
export const GROUP_FATE_KEY: Readonly<Record<GroupFate, string>> = {
  linked: "importReview.groupFate.linked",
  removed: "importReview.groupFate.removed",
  leftInPlace: "importReview.groupFate.leftInPlace",
};

export type ReviewCopy = {
  path: string;
  tool: string;
  toolLabel: string;
  description: string | null;
  modifiedAt: number | null;
  linkTarget: string | null;
  /** Managed row: whether this copy is your managed copy's bytes (`null`: not compared). */
  sameAsManaged: boolean | null;
  /** Import row: the copy that becomes the Library skill. */
  chosen: boolean;
  /** `null` while the group does not go ahead (unchecked, Leave). */
  fate: CopyFate | null;
};

export type ImportRow = {
  kind: "import";
  name: string;
  conflict: boolean;
  checked: boolean;
  copies: ReviewCopy[];
  chosenPath: string | null;
  /** The copies whose bytes differ from the chosen one (the keep / remove choice's subject). */
  others: number;
  /** A checked Match group: its copies' one fate (they all hold the chosen bytes); `null` otherwise. */
  groupFate: GroupFate | null;
  removeOthers: boolean;
  /**
   * What follows removing the differing copies (#98) — they go *before*
   * any deploy, so the harnesses they sat in rely on what comes next:
   * `deployed` (auto-deploy on: the Library skill is deployed to the
   * selected harnesses, and one the deploy misses or fails is left without
   * it), `nothingDeployed` (auto-deploy off: those harnesses are left
   * without it). `null` while the copies stay.
   */
  removeOthersThen: "deployed" | "nothingDeployed" | null;
  /** The harnesses holding a copy, labelled, in plan order. */
  harnesses: string[];
  /** The last import of this group failed: why, in words. */
  lastFailure: string | null;
};

export type ManagedRow = {
  kind: "managed";
  name: string;
  replace: boolean;
  copies: ReviewCopy[];
  harnesses: string[];
  /** Copies whose bytes differ from your managed copy (or could not be compared). */
  differs: number;
  lastFailure: string | null;
};

export type InvalidRow = { key: string; name: string; toolLabel: string; path: string; reason: string };

export type ImportReviewModel = {
  /** The header's counts: name-groups, copies, harnesses holding them, invalid manifests. */
  counts: { skills: number; copies: number; harnesses: number; invalid: number };
  importRows: ImportRow[];
  managedRows: ManagedRow[];
  invalidRows: InvalidRow[];
  /** Checked import rows, of how many; managed rows set to Replace. */
  selection: { imports: number; importable: number; replaces: number };
  policy: {
    autoDeploy: boolean;
    /** The checked groups' copies with the chosen bytes — the identical originals. */
    identical: number;
    identicalHarnesses: string[];
    /** Auto-deploy's harnesses, labelled. */
    deployHarnesses: string[];
    removeOriginals: boolean;
  };
  /** The button's words, and why it is refused when nothing would happen. */
  action: { label: string; refusal: string | null };
};

export type ReviewContext = {
  t: Translate;
  toolLabelById: Labels;
  autoDeploy: boolean;
  /** Auto-deploy's harnesses (the operator's selection). */
  deployTools: readonly string[];
  /** Groups whose last import failed, with the reason in words. */
  lastFailures?: Readonly<Record<string, string>>;
};

const labelOf = (labels: Labels, tool: string) => labels[tool] ?? tool;
const unique = (values: readonly string[]) => [...new Set(values)];

/** The review of a plan under the operator's choices. */
/**
 * My Skills' invitation to review what sits unmanaged in the harnesses, from
 * the banner's plan: the discovered skills, and the folders whose `SKILL.md`
 * the scan refused. Either alone keeps the invitation up — an invalid copy in
 * a harness folder is still an unmanaged skill the operator should see (#98).
 * `null` when there is nothing, or no plan yet.
 */
export function discoveryInvitation(plan: OnboardingPlan | null): { skills: number; invalid: number } | null {
  if (!plan) return null;
  const invitation = { skills: plan.group_count, invalid: plan.invalid.length };
  return invitation.skills > 0 || invitation.invalid > 0 ? invitation : null;
}

export function importReviewModel(plan: OnboardingPlan, choices: ImportChoices, ctx: ReviewContext): ImportReviewModel {
  const { t, toolLabelById: labels, autoDeploy, deployTools } = ctx;
  const failures = ctx.lastFailures ?? {};
  const importRows: ImportRow[] = [];
  const managedRows: ManagedRow[] = [];
  const identicalHarnesses: string[] = [];
  let identical = 0;

  const originalFate = (chosen: boolean): CopyFate =>
    autoDeploy
      ? chosen ? "chosenLinked" : "linked"
      : choices.removeOriginals
        ? chosen ? "chosenRemoved" : "removed"
        : chosen ? "chosenLeft" : "leftInPlace";

  for (const group of plan.groups) {
    const harnesses = unique(group.variants.map((variant) => labelOf(labels, variant.tool)));
    const base = (variant: OnboardingVariant) => ({
      path: variant.path,
      tool: variant.tool,
      toolLabel: labelOf(labels, variant.tool),
      description: variant.description,
      modifiedAt: variant.modified_at,
      linkTarget: variant.is_link ? variant.link_target : null,
    });
    if (group.name_is_managed) {
      const replace = Boolean(choices.replace[group.name]);
      managedRows.push({
        kind: "managed",
        name: group.name,
        replace,
        harnesses,
        differs: group.variants.filter((variant) => variant.same_as_managed !== true).length,
        lastFailure: failures[group.name] ?? null,
        copies: group.variants.map((variant) => ({
          ...base(variant),
          sameAsManaged: variant.same_as_managed,
          chosen: false,
          // Replace links an identical copy at once; any other copy is the
          // overwrite ask's to decide (D12), never a silent overwrite.
          fate: replace ? (variant.same_as_managed === true ? "linked" : "askFirst") : null,
        })),
      });
      continue;
    }
    const checked = Boolean(choices.selected[group.name]);
    const chosenPath = chosenPathOf(group, choices);
    const removeOthers = group.has_conflict && Boolean(choices.removeOthers[group.name]);
    const copies = group.variants.map((variant): ReviewCopy => {
      const same = sameAsChosen(group, variant, chosenPath);
      const chosen = variant.path === chosenPath;
      if (checked && same) {
        identical += 1;
        identicalHarnesses.push(labelOf(labels, variant.tool));
      }
      const fate: CopyFate | null = !checked
        ? null
        : same
          ? originalFate(chosen)
          : removeOthers
            ? "removed"
            : autoDeploy && deployTools.includes(variant.tool)
              ? "askFirst"
              : "leftInPlace";
      return { ...base(variant), sameAsManaged: null, chosen, fate };
    });
    importRows.push({
      kind: "import",
      name: group.name,
      conflict: group.has_conflict,
      checked,
      copies,
      chosenPath,
      others: group.variants.filter((variant) => !sameAsChosen(group, variant, chosenPath)).length,
      groupFate: checked && !group.has_conflict ? (autoDeploy ? "linked" : choices.removeOriginals ? "removed" : "leftInPlace") : null,
      removeOthers,
      removeOthersThen: removeOthers ? (autoDeploy ? "deployed" : "nothingDeployed") : null,
      harnesses,
      lastFailure: failures[group.name] ?? null,
    });
  }

  const invalidRows: InvalidRow[] = plan.invalid.map((entry) => ({
    key: `${entry.tool}\n${entry.path}`,
    name: entry.name,
    toolLabel: labelOf(labels, entry.tool),
    path: entry.path,
    reason: invalidReasonText(entry.reason, t),
  }));

  const imports = importRows.filter((row) => row.checked).length;
  const replaces = managedRows.filter((row) => row.replace).length;
  const action = (): { label: string; refusal: string | null } => {
    // Only folders whose SKILL.md was refused: nothing to select (#98).
    if (importRows.length === 0 && managedRows.length === 0)
      return { label: t("importReview.action.import"), refusal: t("importReview.action.nothingImportable") };
    if (imports === 0 && replaces === 0)
      return { label: t("importReview.action.import"), refusal: t("importReview.action.nothing") };
    if (imports === 0) return { label: t("importReview.action.replace", { count: replaces }), refusal: null };
    if (autoDeploy) return { label: t("importReview.action.importAndDeploy"), refusal: null };
    if (choices.removeOriginals && identical > 0)
      return { label: t("importReview.action.importAndRemove", { count: identical }), refusal: null };
    return { label: t("importReview.action.import"), refusal: null };
  };

  return {
    counts: {
      skills: plan.group_count,
      copies: plan.copy_count,
      harnesses: unique(plan.groups.flatMap((group) => group.variants.map((variant) => variant.tool))).length,
      invalid: plan.invalid.length,
    },
    importRows,
    managedRows,
    invalidRows,
    selection: { imports, importable: importRows.length, replaces },
    policy: {
      autoDeploy,
      identical,
      identicalHarnesses: unique(identicalHarnesses),
      deployHarnesses: deployTools.map((tool) => labelOf(labels, tool)),
      removeOriginals: choices.removeOriginals,
    },
    action: action(),
  };
}

// ---------------------------------------------------------------------------
// The Result
// ---------------------------------------------------------------------------

/**
 * A sent group's outcome on the Result — `importGroupKind`, the settlement
 * rule the headline shares: `kept` is a Replace whose every collision the
 * operator answered Keep (nothing was replaced); `skipped` a group that did
 * nothing (a managed name left alone, a Replace whose every target was
 * skipped or whose collision nobody answered).
 */
export type ImportResultStatus = ImportGroupKind;

/** One sent group as the Result step shows it. */
export type ImportResultRow = {
  name: string;
  status: ImportResultStatus;
  /** What happened, one line per kind of outcome, in words. */
  lines: string[];
  /** Something is left to look at (a copy kept without a say, a folder to review, a failure under it). */
  attention: boolean;
  /** Retry re-sends this group (it failed; nothing of it was touched). */
  retryable: boolean;
};

const isCollision = (row: BatchTargetOutcome) =>
  row.status.status === "failed" && row.status.error.code === "TARGET_EXISTS";

/**
 * The Result step's rows, one per group in the report's order: imported
 * (where it was deployed, and what happened to each original per harness),
 * replaced / kept / skipped as its settled targets say — a skipped target is
 * a neutral line with its reason (disabled, not installed), never folded into
 * a success — or failed (the typed error in words, Retry). A different copy
 * kept in a group the operator chose to keep (`keptByChoice`) needs no
 * attention.
 */
export function importResultRows(
  report: ImportReport,
  ctx: { t: Translate; toolLabelById: Labels; keptByChoice?: ReadonlySet<string> },
): ImportResultRow[] {
  const { t, toolLabelById: labels } = ctx;
  const list = (tools: readonly string[]) => unique(tools.map((tool) => labelOf(labels, tool))).join(t("common.listSeparator"));
  return report.groups.map((group): ImportResultRow => {
    const { group_name: name, status } = group;
    if (status.status === "failed")
      return { name, status: "failed", lines: [describeCommandError(status.error, t) ?? ""], attention: true, retryable: true };
    if (status.status === "skipped")
      return { name, status: "skipped", lines: [t("importReview.result.nameManaged", { name })], attention: false, retryable: false };
    const kind = importGroupKind(status);
    const lines: string[] = [];
    let attention = false;
    const targets = status.targets;
    const synced = targets.filter((row) => row.status.status === "synced").map((row) => row.tool_key);
    const declined = targets.filter((row) => row.status.status === "declined").map((row) => row.tool_key);
    const collisions = targets.filter(isCollision).map((row) => row.tool_key);
    const failed = targets.filter(isImportTargetFailure);
    const skipped = targets.filter((row) => row.status.status === "skipped" && !isImportTargetFailure(row));
    if (status.status === "replaced") {
      if (synced.length > 0) lines.push(t("importReview.result.linked", { harnesses: list(synced) }));
    } else if (synced.length > 0) {
      lines.push(t("importReview.result.deployed", { harnesses: list(synced) }));
    } else if (targets.length === 0) {
      lines.push(t("importReview.result.libraryOnly"));
    }
    if (declined.length > 0) lines.push(t("importReview.result.declined", { harnesses: list(declined) }));
    if (collisions.length > 0) {
      attention = true;
      lines.push(t("importReview.result.toReview", { harnesses: list(collisions) }));
    }
    for (const row of skipped) {
      // A skip is by design (disabled, not installed): its reason, neutrally.
      const error = row.status.status === "skipped" ? row.status.error : null;
      lines.push(t("importReview.result.skippedTarget", { harness: labelOf(labels, row.tool_key), reason: describeCommandError(error, t) ?? "" }));
    }
    for (const row of failed) {
      attention = true;
      const error = row.status.status === "synced" || row.status.status === "declined" ? null : row.status.error;
      lines.push(t("importReview.result.notDeployed", { harness: labelOf(labels, row.tool_key), reason: describeCommandError(error, t) ?? "" }));
    }
    if (status.status === "imported") {
      const by = (kindOf: OriginalOutcome["status"]["status"]) =>
        status.originals.filter((original) => original.status.status === kindOf).map((original) => original.tool);
      const removed = by("removed");
      const left = by("left_in_place");
      const kept = by("kept_divergent");
      if (removed.length > 0) lines.push(t("importReview.result.removed", { harnesses: list(removed) }));
      if (left.length > 0) lines.push(t("importReview.result.leftInPlace", { harnesses: list(left) }));
      if (kept.length > 0) {
        if (!ctx.keptByChoice?.has(name)) attention = true;
        lines.push(t("importReview.result.keptDifferent", { harnesses: list(kept) }));
      }
      for (const original of status.originals) {
        if (original.status.status !== "failed") continue;
        attention = true;
        const error = original.status.error;
        lines.push(
          error.code === "IDENTITY_UNKNOWN"
            ? t("importReview.result.identityUnknown", { harness: labelOf(labels, original.tool) })
            : t("importReview.result.removeFailed", {
                harness: labelOf(labels, original.tool),
                reason: describeCommandError(error, t) ?? "",
              }),
        );
      }
    }
    return { name, status: kind, lines, attention, retryable: false };
  });
}

/**
 * A Retry's report folded into the run's: each retried group's outcome
 * replaces its earlier one in place; the order of the first run holds.
 */
export function mergeImportReports(previous: ImportReport, next: ImportReport): ImportReport {
  const retried = new Map<string, ImportGroupOutcome>(next.groups.map((group) => [group.group_name, group]));
  const merged = previous.groups.map((group) => retried.get(group.group_name) ?? group);
  const known = new Set(previous.groups.map((group) => group.group_name));
  return { groups: [...merged, ...next.groups.filter((group) => !known.has(group.group_name))] };
}

/** The groups the report says failed, each with the reason in words (the review's annotation). */
export function failedGroups(report: ImportReport, t: Translate): Record<string, string> {
  const out: Record<string, string> = {};
  for (const group of report.groups) {
    if (group.status.status === "failed") out[group.group_name] = describeCommandError(group.status.error, t) ?? "";
  }
  return out;
}

/** The skills an import added to the Library (the strip cascade's rows). */
export function importedSkillIds(report: ImportReport): string[] {
  return report.groups.flatMap((group) => (group.status.status === "imported" ? [group.status.skill_id] : []));
}
