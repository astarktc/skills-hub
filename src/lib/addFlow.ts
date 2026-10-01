/**
 * The Add surface's pure folds (ticket 16, D27): what the candidate picker
 * shows and installs, and how a settlement row reads. The flow hook
 * (`useAddSkillFlow`) holds the state; these decide from it.
 *
 * - **Visible and checked** (#47): the search box filters the list, and only
 *   a candidate that is checked, visible and selectable installs. The
 *   summary says so ("3 of 4 checked · 1 hidden by filter").
 * - **Settlement rows**: one row per candidate the operator sent, in pick
 *   order — queued / adding / awaiting its result while the batch runs, then
 *   installed (with what its first sync did), failed (the typed reason) or
 *   cancelled.
 * - **Settlement notices**: a first-sync failure shared by several rows (an
 *   unwritable Tool folder) is said once, under the rows, with the skills it
 *   affects; each installed row keeps only which Tools it missed (A4). A
 *   candidate's own failure keeps its reason on its row, beside Retry.
 */

import { describeCommandError } from "../commandError";
import type { BatchTargetOutcome, CommandError } from "../components/skills/types";
import { installComplete, type InstallSettlement } from "./reportOutcome";

type Translate = (key: string, opts?: Record<string, unknown>) => string;

/** One candidate as the picker renders it (git and local alike). */
export type PickerCandidate = {
  /** The candidate identity: its listing subpath. */
  id: string;
  name: string;
  description: string | null;
  subpath: string;
  /** Why it cannot be picked, in words; `null` when it can. */
  unavailable: string | null;
};

/** Whether a candidate matches the search (name, subpath or description). */
export function matchesQuery(candidate: PickerCandidate, query: string): boolean {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  return [candidate.name, candidate.subpath, candidate.description ?? ""].some((field) =>
    field.toLowerCase().includes(needle),
  );
}

/** The candidates the search leaves on screen, in listing order. */
export function visibleCandidates(
  candidates: readonly PickerCandidate[],
  query: string,
): PickerCandidate[] {
  return candidates.filter((candidate) => matchesQuery(candidate, query));
}

/** What installs: checked, visible and selectable — nothing else (#47). */
export function pickedCandidates(
  candidates: readonly PickerCandidate[],
  selected: Readonly<Record<string, boolean>>,
  query: string,
): PickerCandidate[] {
  return visibleCandidates(candidates, query).filter(
    (candidate) => candidate.unavailable === null && selected[candidate.id] === true,
  );
}

export type PickSummary = {
  total: number;
  /** Checked and visible (what "Add selected" installs). */
  checked: number;
  /** Candidates the search hides. */
  hidden: number;
  /** Of those, the ones still checked: they do not install while hidden. */
  hiddenChecked: number;
};

export function pickSummary(
  candidates: readonly PickerCandidate[],
  selected: Readonly<Record<string, boolean>>,
  query: string,
): PickSummary {
  const visible = new Set(visibleCandidates(candidates, query).map((candidate) => candidate.id));
  const hidden = candidates.filter((candidate) => !visible.has(candidate.id));
  return {
    total: candidates.length,
    checked: pickedCandidates(candidates, selected, query).length,
    hidden: hidden.length,
    hiddenChecked: hidden.filter((candidate) => candidate.unavailable === null && selected[candidate.id]).length,
  };
}

/** "3 of 4 checked · 1 hidden by filter" — the picker's and the summary's line. */
export function pickSummaryText(summary: PickSummary, t: Translate): string {
  const head = t("add.summary.checked", { checked: summary.checked, total: summary.total });
  return summary.hidden > 0 ? `${head} · ${t("add.summary.hidden", { count: summary.hidden })}` : head;
}

/** A selection's rejection in words, or null when it may be installed. */
export function rejectSelection(
  picked: readonly { name: string }[],
  customName: string,
  isSkillNameTaken: (name: string) => boolean,
  t: Translate,
): string | null {
  if (picked.length === 0) return t("errors.selectAtLeastOneSkill");
  if (picked.length > 1 && customName) return t("errors.multiSelectNoCustomName");
  const seen = new Set<string>();
  for (const candidate of picked) {
    if (seen.has(candidate.name)) return t("errors.duplicateSelectedSkills", { name: candidate.name });
    seen.add(candidate.name);
  }
  const names = picked.length === 1 && customName ? [customName] : picked.map((candidate) => candidate.name);
  const taken = names.find(isSkillNameTaken);
  return taken ? t("errors.skillAlreadyExists", { name: taken }) : null;
}

/** A local candidate's invalid-manifest code in words. */
export function invalidReasonText(code: string | null | undefined, t: Translate): string {
  switch (code) {
    case "missing_skill_md":
      return t("localSkillInvalid.missingSkillMd");
    case "invalid_frontmatter":
      return t("localSkillInvalid.invalidFrontmatter");
    case "missing_name":
      return t("localSkillInvalid.missingName");
    case "read_failed":
      return t("localSkillInvalid.readFailed");
    case "inside_tool_dir":
      return t("localSkillInvalid.insideToolDir");
    default:
      return t("localSkillInvalid.unknown");
  }
}

// ---------------------------------------------------------------------------
// Settlement
// ---------------------------------------------------------------------------

/** Where one sent candidate stands. */
export type SettlementPhase =
  /** Sent, not reached yet. */
  | { phase: "queued" }
  /** The batch is on it ("Adding (i/n) name"). */
  | { phase: "adding" }
  /** The batch moved past it; its result comes with the report. */
  | { phase: "awaiting" }
  | { phase: "settled"; result: InstallSettlement };

export type SettlementEntry = {
  id: string;
  /** The name it was asked under (the custom name for a single pick). */
  name: string;
  /** The operator's name for a single pick; `null` keeps the skill's own. */
  nameOverride: string | null;
} & SettlementPhase;

export type SettlementRowStatus = "queued" | "adding" | "awaiting" | "installed" | "failed" | "cancelled";

export type SettlementRow = {
  id: string;
  name: string;
  status: SettlementRowStatus;
  /** What happened, in words (a first sync's result, a typed reason). */
  detail: string | null;
  /** An installed row whose first sync left something to look at. */
  attention: boolean;
  /** Retry (a failure) or Add (a cancellation) re-sends this candidate alone. */
  retryable: boolean;
};

const labelList = (tools: readonly string[], toolLabelById: Readonly<Record<string, string>>, t: Translate) =>
  tools.map((tool) => toolLabelById[tool] ?? tool).join(t("common.listSeparator"));

/** An explicit install's unwritable Tool is a failure, every other skip a
 * skip (the same rule as `reportOutcome`'s install fold). */
const unwritable = (row: BatchTargetOutcome) =>
  row.status.status === "skipped" && row.status.error.code === "TOOL_NOT_WRITABLE";

/** A first sync's failed targets (not its collisions: those are folders to review). */
function targetFailures(report: readonly BatchTargetOutcome[]): { tool: string; error: CommandError }[] {
  const failed: { tool: string; error: CommandError }[] = [];
  for (const row of report) {
    if (row.status.status === "skipped" && unwritable(row)) failed.push({ tool: row.tool_key, error: row.status.error });
    if (row.status.status === "failed" && row.status.error.code !== "TARGET_EXISTS")
      failed.push({ tool: row.tool_key, error: row.status.error });
  }
  return failed;
}

/** An installed candidate's first sync, in one line. */
function deploymentDetail(
  report: readonly BatchTargetOutcome[],
  ctx: { t: Translate; toolLabelById: Readonly<Record<string, string>> },
): { detail: string; attention: boolean } {
  const { t, toolLabelById } = ctx;
  const synced = report.filter((row) => row.status.status === "synced");
  const declined = report.filter((row) => row.status.status === "declined").map((row) => row.tool_key);
  const skipped = report
    .filter((row) => row.status.status === "skipped" && !unwritable(row))
    .map((row) => row.tool_key);
  const collisions = report
    .filter((row) => row.status.status === "failed" && row.status.error.code === "TARGET_EXISTS")
    .map((row) => row.tool_key);
  const failed = targetFailures(report);
  const parts: string[] = [];
  if (synced.length > 0) parts.push(t("add.settlement.deployed", { count: synced.length }));
  if (declined.length > 0) parts.push(t("add.settlement.kept", { tools: labelList(declined, toolLabelById, t) }));
  if (collisions.length > 0)
    parts.push(t("add.settlement.toReview", { count: collisions.length, tools: labelList(collisions, toolLabelById, t) }));
  // Which Tools it missed; why is said once, by `settlementNotices`.
  if (failed.length > 0)
    parts.push(t("addResult.notDeployedTo", { tools: labelList(failed.map((item) => item.tool), toolLabelById, t) }));
  if (skipped.length > 0) parts.push(t("add.settlement.skipped", { tools: labelList(skipped, toolLabelById, t) }));
  if (parts.length === 0) parts.push(t("add.settlement.deployedNowhere"));
  return { detail: parts.join(" · "), attention: collisions.length > 0 || failed.length > 0 };
}

/** The settlement step's rows, in the order the candidates were sent. */
export function settlementRows(
  entries: readonly SettlementEntry[],
  ctx: { t: Translate; toolLabelById: Readonly<Record<string, string>> },
): SettlementRow[] {
  const { t } = ctx;
  return entries.map((entry): SettlementRow => {
    const base = { id: entry.id, name: entry.name, detail: null, attention: false, retryable: false };
    if (entry.phase !== "settled") return { ...base, status: entry.phase };
    const result = entry.result;
    if (result.status === "cancelled")
      return { ...base, status: "cancelled", detail: t("add.settlement.cancelledDetail"), retryable: true };
    if (result.status === "failed")
      return { ...base, status: "failed", detail: describeCommandError(result.error, t), retryable: true };
    const deployment = result.deployment;
    const { detail, attention } =
      deployment.status === "reported"
        ? deploymentDetail(deployment.report, ctx)
        : deployment.status === "disabled"
          ? { detail: t("add.settlement.notDeployedAutoSyncOff"), attention: false }
          : deployment.status === "no-targets"
            ? { detail: t("add.settlement.noTools"), attention: true }
            : { detail: describeCommandError(deployment.error, t), attention: true };
    return { ...base, name: result.name, status: "installed", detail, attention };
  });
}

/** One first-sync failure reason, said once for every row it affects. */
export type SettlementNotice = {
  /** The reason in words (the typed error described). */
  reason: string;
  /** The Tools it kept the skills from, labelled, in first-seen order. */
  tools: string[];
  /** The added skills it affects, in row order. */
  skills: string[];
};

/**
 * The first-sync failures of the installed rows, grouped by reason: one
 * notice per distinct reason with the Tools and skills it affects (four rows
 * missing an unwritable Amp read one reason, not four copies of it).
 */
export function settlementNotices(
  entries: readonly SettlementEntry[],
  ctx: { t: Translate; toolLabelById: Readonly<Record<string, string>> },
): SettlementNotice[] {
  const { t, toolLabelById } = ctx;
  const byReason = new Map<string, SettlementNotice>();
  for (const entry of entries) {
    if (entry.phase !== "settled" || entry.result.status !== "installed") continue;
    const deployment = entry.result.deployment;
    if (deployment.status !== "reported") continue;
    for (const failure of targetFailures(deployment.report)) {
      // Only a cancellation describes as null, and a target is never cancelled.
      const reason = describeCommandError(failure.error, t) ?? t("add.settlement.status.cancelled");
      const notice: SettlementNotice = byReason.get(reason) ?? { reason, tools: [], skills: [] };
      const tool = toolLabelById[failure.tool] ?? failure.tool;
      if (!notice.tools.includes(tool)) notice.tools.push(tool);
      if (!notice.skills.includes(entry.result.name)) notice.skills.push(entry.result.name);
      byReason.set(reason, notice);
    }
  }
  return Array.from(byReason.values());
}

/**
 * The Result's work is done (the dialog may close): every entry it holds —
 * this run's and every earlier run's — settled completely, by
 * `installComplete`, the close rule `installOutcome` applies to one run. A
 * clean Retry of one candidate does not resolve another's folder to review.
 */
export function settlementComplete(entries: readonly SettlementEntry[]): boolean {
  const results: InstallSettlement[] = [];
  for (const entry of entries) {
    if (entry.phase !== "settled") return false;
    results.push(entry.result);
  }
  return installComplete(results);
}
