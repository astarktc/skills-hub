import type {
  BatchTargetOutcome,
  CommandError,
  EnableReport,
  ImportReport,
  InstallBatchReport,
  InstallResultDto,
  InvocationEditReport,
  ProjectSyncReport,
  PropagationOutcome,
  RefreshReport,
  RemovalReport,
} from "../bindings";
import { describeCommandError, toCommandError } from "../commandError";
import type { DeclinedPair } from "./overwriteAsk";
import type { TranslateFn } from "../hooks/useStatusReporter";
import { ACQUISITION_SKIP_KEY, INVOCATION_LABEL_KEY, SKIPPED_REASON_KEY } from "./skillPresentation";

/**
 * What one report entry is about: the skill (by id — resolved when the
 * operator acts, against the catalog then), the Tool and, for a project
 * assignment, the project. The Notifications panel names a project row by it
 * and derives the row's actions from it (`notificationRuns.rowActions`).
 */
export type EntrySubject = {
  skillId?: string;
  skillName: string;
  tool?: string;
  projectId?: string;
  /**
   * The entry's title already names its project (a resync of every project):
   * the panel's project-aware presentation does not prefix it again.
   */
  projectInTitle?: true;
};

/**
 * The repair a report entry offers, decided where the entry is folded (the
 * fold knows what failed): re-sync the pair from the central copy, answer a
 * collision (the overwrite ask again), re-run the skill's Update, or ask to
 * remove it again. The panel maps it onto the door's verb.
 */
export type EntryRepair =
  | { verb: "retry"; tool: string }
  | {
      verb: "overwrite";
      tool: string;
      /**
       * The route that met the collision, kept as data so answering it later
       * takes the same one (ticket 14 audit A3): `sync` — a sync, Refresh's
       * Propagation / re-assert, Edit, install or import (the ordinary re-sync
       * asks again); `enable` — an Enable, whose pair stays a disabled marker
       * until answered, so only the Enable path (its disabled-marker
       * admission) can reach it — an ordinary sync skips a disabled pair.
       */
      via: CollisionRoute;
    }
  | { verb: "update" }
  | { verb: "delete" };

/** Which operator path met a collision: its answer goes the same way. */
export type CollisionRoute = "sync" | "enable";

/**
 * One report entry. Declarative only — no closures — so it survives catalog
 * replacement and lands in the session history as data: `subject` and
 * `error` (the typed failure, when there is one) let the panel offer Show,
 * Retry, Overwrite…, Change source… and Open Settings → GitHub.
 * `review` marks an entry of the "to review" kind: a collision the operator
 * has not answered (D10/D12), never a failure.
 */
export type OutcomeEntry = {
  title: string;
  message: string;
  subject?: EntrySubject;
  error?: CommandError;
  repair?: EntryRepair;
  review?: true;
};
/** The entry every fold produces (one shape; kept as a name for the folds' signatures). */
export type PlainEntry = OutcomeEntry;

/**
 * Which outcome kind decides a report's toast, by precedence (D10):
 * failure › conflict › skipped › declined › success. `neutral` is a run the
 * operator cancelled with nothing else to report. The tone chooses the toast
 * kind; it never chooses the headline's words — the composite count line does.
 */
export type OutcomeTone =
  | "failure"
  | "conflict"
  | "skipped"
  | "declined"
  | "success"
  | "neutral";
const TONE_KIND = {
  failure: "warning",
  conflict: "warning",
  skipped: "warning",
  declined: "info",
  success: "success",
  neutral: "info",
} as const satisfies Record<OutcomeTone, "success" | "warning" | "info">;
export type OutcomeToast = {
  kind: (typeof TONE_KIND)[OutcomeTone];
  tone: OutcomeTone;
  /** The headline: a composite count line for a batch, one sentence otherwise. */
  message: string;
  detail?: string;
};
export type Outcome<Entry = OutcomeEntry> = {
  toast: OutcomeToast | null;
  errors: Entry[];
  warnings: Entry[];
  /**
   * Quiet report lines that are neither errors nor warnings — a declined
   * pair ("alpha: kept your existing Claude Code folder"). A fold with a
   * toast also carries them as its `detail`.
   */
  notes: string[];
  completion: { reload: boolean; closeModal: boolean; conflict: boolean };
  /**
   * The structured counts behind a count-bearing fold's headline, carried so
   * `combineOutcomes` can build one composite line and one tone over several
   * reports without reading a toast's words. Set by the folds whose parts an
   * operator action may combine (the bulk sync, Enable).
   */
  tally?: OutcomeTally;
};
/**
 * A fold's headline counts: its done count(s), each phrased by its own verb
 * ("1 synced · 1 enabled"), and the kinds every verb shares — the units are
 * the same Sync targets, so they sum.
 */
export type OutcomeTally = {
  done: { verb: HeadlineVerb; count: number }[];
  counts: Omit<Partial<HeadlineCounts>, "done">;
  review?: ReviewKind;
};
export type ReportContext = {
  t: TranslateFn;
  toolLabelById?: Readonly<Record<string, string>>;
};
const empty = (): Outcome<PlainEntry> => ({
  toast: null,
  errors: [],
  warnings: [],
  notes: [],
  completion: { reload: true, closeModal: true, conflict: false },
});
const label = (ctx: ReportContext, tool: string) =>
  ctx.toolLabelById?.[tool] ?? tool;
const errorMessage = (ctx: ReportContext, error: unknown) =>
  describeCommandError(error, ctx.t) ?? "";
const toast = (tone: OutcomeTone, message: string, detail?: string): OutcomeToast => ({
  kind: TONE_KIND[tone],
  tone,
  message,
  ...(detail ? { detail } : {}),
});
const detailOf = (lines: string[]) => (lines.length ? lines.join("\n") : undefined);

// ---------------------------------------------------------------------------
// The composite headline (D10)
// ---------------------------------------------------------------------------

/** The action a batch's "done" count is phrased with. */
export type HeadlineVerb =
  | "refreshed"
  | "installed"
  | "synced"
  | "imported"
  | "removed"
  | "assigned"
  | "enabled"
  | "disabled";
export const HEADLINE_DONE_KEY: Record<HeadlineVerb, string> = {
  refreshed: "outcome.done.refreshed",
  installed: "outcome.done.installed",
  synced: "outcome.done.synced",
  imported: "outcome.done.imported",
  removed: "outcome.done.removed",
  assigned: "outcome.done.assigned",
  enabled: "outcome.done.enabled",
  disabled: "outcome.done.disabled",
};
/**
 * One count per outcome kind, in headline order. `failed` counts the batch's
 * own units (skills, groups, pairs); `targetsFailed` counts Sync targets that
 * failed under a unit that itself succeeded (a refreshed skill's Propagation,
 * an installed skill's deployment). `review` is the fold's "to review" kind.
 */
export type HeadlineCounts = {
  done: number;
  failed: number;
  targetsFailed: number;
  skipped: number;
  declined: number;
  cancelled: number;
  review: number;
  /**
   * Sync targets occupied by a different folder that nobody answered — the
   * overwrite ask was dismissed (D12: "a dismissed ask sends nothing"): the
   * pair stays as it was, "to review", never a failure (D10).
   */
  collisions: number;
};
const ZERO_COUNTS: HeadlineCounts = {
  done: 0, failed: 0, targetsFailed: 0, skipped: 0, declined: 0, cancelled: 0, review: 0, collisions: 0,
};
/** What the "to review" count is of. */
export type ReviewKind = "edits" | "originals";
const REVIEW_KEY: Record<ReviewKind, string> = {
  edits: "outcome.review.edits",
  originals: "outcome.review.originals",
};
/** The separator between headline parts; punctuation, not copy. */
export const HEADLINE_SEPARATOR = " · ";

/**
 * The composite count line: "12 refreshed · 5 failed · 6 targets failed ·
 * 4 skipped · 1 Edit to review". Fixed order — done · failed · targets failed
 * · skipped · declined · cancelled · to review — and a zero count is omitted;
 * when every count is zero the line is the done part at zero ("0 synced").
 */
export function compositeHeadline(
  t: TranslateFn,
  verb: HeadlineVerb,
  partial: Partial<HeadlineCounts>,
  review: ReviewKind = "edits",
): string {
  return tallyHeadline(t, {
    done: [{ verb, count: partial.done ?? 0 }],
    counts: partial,
    review,
  });
}

/** The composite line of a tally: every done count first (in the tally's
 * order), then the shared kinds in headline order. */
function tallyHeadline(t: TranslateFn, tally: OutcomeTally): string {
  const counts = { ...ZERO_COUNTS, ...tally.counts };
  const review = tally.review ?? "edits";
  const parts: string[] = [];
  const part = (key: string, count: number) => {
    if (count > 0) parts.push(t(key, { count }));
  };
  for (const { verb, count } of tally.done) part(HEADLINE_DONE_KEY[verb], count);
  part("outcome.failed", counts.failed);
  part("outcome.targetsFailed", counts.targetsFailed);
  part("outcome.skipped", counts.skipped);
  part("outcome.declined", counts.declined);
  part("outcome.cancelled", counts.cancelled);
  part(REVIEW_KEY[review], counts.review);
  part("runReport.collisions", counts.collisions);
  const [first] = tally.done;
  if (parts.length === 0 && first) parts.push(t(HEADLINE_DONE_KEY[first.verb], { count: 0 }));
  return parts.join(HEADLINE_SEPARATOR);
}

/** The toast tone of a set of counts: the D10 precedence, cancelled last. */
export function headlineTone(partial: Partial<HeadlineCounts>): OutcomeTone {
  const counts = { ...ZERO_COUNTS, ...partial };
  if (counts.failed > 0 || counts.targetsFailed > 0) return "failure";
  if (counts.review > 0 || counts.collisions > 0) return "conflict";
  if (counts.skipped > 0) return "skipped";
  if (counts.declined > 0) return "declined";
  if (counts.cancelled > 0) return "neutral";
  return "success";
}

/** A batch's toast: its composite line, toned by precedence. */
function headlineToast(
  t: TranslateFn,
  verb: HeadlineVerb,
  counts: Partial<HeadlineCounts>,
  options: { review?: ReviewKind; detail?: string } = {},
): OutcomeToast {
  return toast(
    headlineTone(counts),
    compositeHeadline(t, verb, counts, options.review),
    options.detail,
  );
}

/** The quiet line of a pair the operator declined ("Keep existing"). */
const declinedNote = (ctx: ReportContext, name: string, tool: string) =>
  ctx.t("outcome.declinedLine", { name, tool: label(ctx, tool) });

type TargetExists = Extract<CommandError, { code: "TARGET_EXISTS" }>;

/**
 * A `TARGET_EXISTS` that reached a fold is an unresolved collision: an
 * answered row is replaced by its retry (or becomes `declined`), so one a
 * report still carries was not answered — the ask was dismissed, or the
 * producer raised none (Import does not ask yet: ticket 21). This is not
 * proof that an ask happened. It is "to review": the pair stays as it was,
 * nothing was retried, nothing failed (D10).
 */
const unansweredCollision = (error: CommandError): TargetExists | null =>
  error.code === "TARGET_EXISTS" ? error : null;

/** The "to review" entry of an unanswered collision (global pair or project assignment). */
function collisionEntry(
  ctx: ReportContext,
  error: TargetExists,
  subject: EntrySubject & { tool: string },
  via: CollisionRoute = "sync",
): OutcomeEntry {
  return {
    title: ctx.t("runReport.collisionTitle", { name: subject.skillName, tool: label(ctx, subject.tool) }),
    message: ctx.t("runReport.collisionMessage", { path: error.path }),
    subject,
    error,
    // A project assignment is answered from its matrix; a global pair here.
    ...(subject.projectId === undefined ? { repair: { verb: "overwrite" as const, tool: subject.tool, via } } : {}),
    review: true,
  };
}

function refreshCounts(report: RefreshReport) {
  const counts = { refreshed: 0, failed: 0, skipped: 0, cancelled: 0, conflicts: 0, target_failures: 0, collisions: 0 };
  for (const { status } of report.skills) {
    if (status.status === "refreshed") {
      counts.refreshed++;
      for (const target of status.targets) {
        if (target.status.status !== "failed") continue;
        if (unansweredCollision(target.status.error)) counts.collisions++;
        else counts.target_failures++;
      }
      counts.target_failures += Number(status.reassert_error !== null);
      counts.conflicts += Number(status.edit_conflict !== null);
    } else if (status.status === "failed") counts.failed++;
    else if (status.status === "cancelled") counts.cancelled++;
    else counts.skipped++;
  }
  return counts;
}

/**
 * `removed` / `failed` count the rows of removed / kept targets; `cleared`
 * counts rows settled without touching any path (`cleared_markers`: rows that
 * owned no artifact), so an unassignment that only cleared such a row still
 * counts as one without claiming an artifact was removed.
 */
function removalCounts(report: RemovalReport) {
  const counts = { removed: 0, failed: 0, cleared: report.cleared_markers.length };
  for (const target of report.targets)
    counts[target.status.status] += target.rows.length;
  return counts;
}

function projectSyncCounts(report: ProjectSyncReport) {
  const counts = { synced: 0, alreadyAssigned: 0, failed: 0, skipped: 0, declined: 0 };
  for (const { status } of report.items) {
    if (status.status === "synced") counts.synced++;
    else if (status.status === "already_assigned") counts.alreadyAssigned++;
    else if (status.status === "skipped") counts.skipped++;
    else if (status.status === "declined") counts.declined++;
    else counts.failed++;
  }
  return counts;
}

function importCounts(report: ImportReport) {
  const counts = { imported: 0, replaced: 0, skipped: 0, failed: 0 };
  for (const group of report.groups) counts[group.status.status]++;
  return counts;
}

/**
 * Update, Restore and BOTH Re-points use the same batch-of-one policy.
 * Batch reports reload; single-mutation responses already supply the
 * complete catalog, even on failure/skip. Only a fully refreshed batch closes
 * a repair modal; conflicts/target failures do not undo central settlement.
 * Thrown invocation errors are not reports.
 *
 * The batch headline is the composite count line (D10): refreshed · failed ·
 * targets failed · skipped · cancelled · Edits to review, toned by precedence
 * (failure › conflict › skipped › success) — a conflict is additive news about
 * a skill that refreshed (ADR-0004), never a headline over failures. A run the
 * operator cancelled before anything was applied reads "Refresh cancelled —
 * nothing was changed", neutral; cancelled rows raise no entry.
 * A batch of one never shows the count line: its failure or skip is already
 * the one entry, so a second toast would repeat it; a conflict keeps its
 * sentence (toned failure when a target failed too), a cancel its own.
 *
 * Re-assert additions are announced ("alpha also deployed to Cursor, Pi"):
 * a global target the report synced that the skill did not have before the
 * run (`priorGlobalTools`, the catalog the operator acted on) was created
 * by the auto-sync re-assert. A collision nobody answered is "to review".
 */
export function refreshOutcome(
  report: RefreshReport,
  ctx: ReportContext & {
    single?: { name: string; success: string };
    /** Pairs the operator kept in the overwrite ask the report's collisions raised. */
    declined?: readonly DeclinedPair[];
    /** Each skill's global Tools before the run (skill id → tools); absent = unknown, nothing announced. */
    priorGlobalTools?: Readonly<Record<string, readonly string[]>>;
  },
): Outcome {
  const out: Outcome = empty();
  out.completion.reload = !ctx.single;
  const { t } = ctx;
  const summary = refreshCounts(report);
  const targetErrors: OutcomeEntry[] = [];
  const additions: string[] = [];
  for (const skill of report.skills) {
    const status = skill.status;
    const subject: EntrySubject = { skillId: skill.skill_id, skillName: skill.skill_name };
    if (status.status === "failed") {
      out.errors.push({
        title: t("errors.updateFailedTitle", { name: skill.skill_name }),
        message: errorMessage(ctx, status.error),
        subject,
        error: status.error,
        repair: { verb: "update" },
      });
    } else if (status.status === "skipped") {
      out.warnings.push({
        title: t("errors.refreshSkippedTitle", { name: skill.skill_name }),
        message: t(SKIPPED_REASON_KEY[status.state]),
        subject,
      });
    } else if (status.status === "skipped_acquisition") {
      out.warnings.push({
        title: t("errors.refreshSkippedTitle", { name: skill.skill_name }),
        message: t(ACQUISITION_SKIP_KEY[status.reason]),
        subject,
      });
    } else if (status.status === "refreshed") {
      const { errors, reviews } = propagationEntries(status.targets, subject, ctx);
      targetErrors.push(...errors);
      out.warnings.push(...reviews);
      if (status.reassert_error)
        targetErrors.push({
          title: t("errors.reassertFailedTitle", { name: skill.skill_name }),
          message: errorMessage(ctx, status.reassert_error),
          subject,
          error: status.reassert_error,
        });
      const prior = ctx.priorGlobalTools?.[skill.skill_id];
      if (prior) {
        const added = status.targets.flatMap((target) =>
          target.scope.scope === "global" &&
          target.status.status === "synced" &&
          !prior.includes(target.scope.tool)
            ? [label(ctx, target.scope.tool)]
            : [],
        );
        if (added.length > 0)
          additions.push(
            t("runReport.alsoDeployed", { name: skill.skill_name, tools: added.join(t("common.listSeparator")) }),
          );
      }
    }
    // A cancelled row is counted in the headline and raises no entry.
  }
  out.errors.push(...targetErrors);
  out.notes.push(...additions);
  for (const pair of ctx.declined ?? []) out.notes.push(declinedNote(ctx, pair.skillName, pair.tool));
  for (const skill of report.skills) {
    if (skill.status.status !== "refreshed" || !skill.status.edit_conflict)
      continue;
    const conflict = skill.status.edit_conflict;
    out.completion.conflict = true;
    out.warnings.push({
      subject: { skillId: skill.skill_id, skillName: skill.skill_name },
      title: t("invocationEdit.warningTitle", { name: skill.skill_name }),
      message: t("invocationEdit.refreshWarning", {
        name: skill.skill_name,
        upstream: t(INVOCATION_LABEL_KEY[conflict.upstream_mode]),
        override: t(INVOCATION_LABEL_KEY[conflict.override_mode]),
      }),
    });
  }
  const failed =
    summary.failed > 0 || summary.target_failures > 0 || out.errors.length > 0;
  const skipped = summary.skipped > 0;
  out.completion.closeModal =
    summary.refreshed > 0 && summary.failed === 0 && !skipped;
  const counts: HeadlineCounts = {
    done: summary.refreshed,
    failed: summary.failed,
    targetsFailed: summary.target_failures,
    skipped: summary.skipped,
    declined: ctx.declined?.length ?? 0,
    cancelled: summary.cancelled,
    review: summary.conflicts,
    collisions: summary.collisions,
  };
  // Cancelled before anything was applied: nothing changed, nothing failed.
  if (summary.cancelled > 0 && summary.refreshed === 0 && !failed) {
    out.toast = toast(
      "neutral",
      ctx.single
        ? t("outcome.updateCancelled", { name: ctx.single.name })
        : t("outcome.refreshCancelled"),
    );
    return out;
  }
  if (ctx.single) {
    if (out.completion.conflict)
      out.toast = toast(
        headlineTone(counts),
        t("invocationEdit.updateCompletedWithConflict", { name: ctx.single.name }),
      );
    else if (!failed && !skipped)
      out.toast = toast(
        headlineTone(counts),
        [ctx.single.success, ...(counts.collisions > 0 ? [t("runReport.collisions", { count: counts.collisions })] : [])].join(
          HEADLINE_SEPARATOR,
        ),
        detailOf(out.notes),
      );
    return out;
  }
  out.toast = headlineToast(t, "refreshed", counts, { detail: detailOf(out.notes) });
  return out;
}

/**
 * One skill's Propagation targets as entries: a failed target is an error
 * (a global one offers Retry); a collision nobody answered is "to review".
 */
function propagationEntries(
  targets: PropagationOutcome[],
  skill: EntrySubject,
  ctx: ReportContext,
): { errors: OutcomeEntry[]; reviews: OutcomeEntry[] } {
  const errors: OutcomeEntry[] = [];
  const reviews: OutcomeEntry[] = [];
  for (const target of targets) {
    if (target.status.status !== "failed") continue;
    const { tool } = target.scope;
    const subject: EntrySubject & { tool: string } = {
      ...skill,
      tool,
      ...(target.scope.scope === "project" ? { projectId: target.scope.project_id } : {}),
    };
    const collision = unansweredCollision(target.status.error);
    if (collision) {
      reviews.push(collisionEntry(ctx, collision, subject));
      continue;
    }
    errors.push({
      title: ctx.t("errors.propagationFailedTitle", { name: skill.skillName, tool: label(ctx, tool) }),
      message: errorMessage(ctx, target.status.error),
      subject,
      error: target.status.error,
      ...(target.scope.scope === "global" ? { repair: { verb: "retry" as const, tool } } : {}),
    });
  }
  return { errors, reviews };
}

/** Edit has already settled centrally, including a no-op or clear. Expected
 * propagation skips are silent; a target failure is its notification plus a
 * headline that still says the Edit was saved — "Invocation setting saved ·
 * 2 targets failed", warning-toned (ADR-0004: the Edit did save). The
 * accompanying catalog replaces the whole library. */
export function invocationEditOutcome(
  report: InvocationEditReport,
  ctx: ReportContext & { declined?: readonly DeclinedPair[] },
): Outcome<PlainEntry> {
  const { errors, reviews } = propagationEntries(
    report.propagation.targets,
    { skillId: report.skill_id, skillName: report.skill_name },
    ctx,
  );
  const saved = ctx.t("invocationEdit.saved");
  // Pairs kept in the overwrite ask its collisions raised: quiet notes.
  const notes = (ctx.declined ?? []).map((pair) => declinedNote(ctx, pair.skillName, pair.tool));
  const parts = [
    saved,
    ...(errors.length ? [ctx.t("outcome.targetsFailed", { count: errors.length })] : []),
    ...(reviews.length ? [ctx.t("runReport.collisions", { count: reviews.length })] : []),
  ];
  return {
    toast: errors.length || reviews.length
      ? toast(errors.length ? "failure" : "conflict", parts.join(HEADLINE_SEPARATOR), detailOf(notes))
      : toast(notes.length ? "declined" : "success", saved, detailOf(notes)),
    errors,
    warnings: reviews,
    notes,
    completion: { reload: false, closeModal: true, conflict: false },
  };
}

/**
 * One error entry per kept row of a removal report (ADR-0002: shared
 * artifacts may describe several tools' rows). Shared by every removal-shaped fold; the title key
 * is the only thing that differs per action. Returns whether anything failed.
 */
function collectRemovalFailures(
  out: Outcome<PlainEntry>,
  report: RemovalReport,
  ctx: ReportContext,
  titleKey: string,
  /** Skill names by id, when the fold knows them (a Delete names its skill). */
  skillNames?: Readonly<Record<string, string>>,
): boolean {
  for (const target of report.targets) {
    if (target.status.status !== "failed") continue;
    for (const row of target.rows) {
      out.errors.push({
        title: ctx.t(titleKey, { tool: label(ctx, row.tool) }),
        message: errorMessage(ctx, target.status.error),
        subject: {
          skillId: row.skill_id,
          skillName: skillNames?.[row.skill_id] ?? "",
          tool: row.tool,
          ...(row.scope === "assignment" ? { projectId: row.project_id } : {}),
        },
        error: target.status.error,
      });
    }
  }
  return removalCounts(report).failed > 0 || out.errors.length > 0;
}

/**
 * Unsync-shaped folds: `all` / `skill` (skills world, reloads the catalog)
 * and `toggle` — used by BOTH the global unsync toggle (skills world, which
 * honours `completion.reload`) and the project assignment toggle-off (projects
 * world, which applies the mutation's returned view and ignores `completion`).
 * `bulkUnassign` is the projects world's inverse of bulk assign: one skill
 * off every Tool of one project; it never reloads (the view came back).
 */
export function removalOutcome(
  report: RemovalReport,
  ctx: ReportContext & { action: "all" | "skill" | "toggle" | "bulkUnassign" },
): Outcome<PlainEntry> {
  const out = empty();
  const counts = removalCounts(report);
  const failed = collectRemovalFailures(out, report, ctx, "errors.unsyncFailedTitle");
  // Zero targets is neither success nor failure: nothing was planned, so
  // nothing was removed and whatever the operator clicked is still there.
  // `failed` counts failures and must keep meaning exactly that.
  // Clearing a marker (a row that owned no artifact: the app never
  // materialised it, whatever its status) is work done, not "nothing planned", even though no path was touched.
  const nothingPlanned =
    !failed && report.targets.length === 0 && report.cleared_markers.length === 0;
  out.completion.closeModal = !failed && !nothingPlanned;
  if (ctx.action === "toggle") out.completion.reload = !failed;
  if (ctx.action === "bulkUnassign") out.completion.reload = false;
  if (nothingPlanned) {
    out.toast = toast("skipped", ctx.t("unsyncNothingPlanned"));
    return out;
  }
  // A Disable over several skills (the bulk rail, D8) reads as the count
  // line over its Sync targets: "412 disabled · 3 failed" (D10).
  if (ctx.action === "all")
    out.toast = headlineToast(ctx.t, "disabled", { done: counts.removed + counts.cleared, failed: counts.failed });
  if (ctx.action === "toggle" && !failed)
    out.toast = toast("success", ctx.t("status.syncDisabled"));
  if (ctx.action === "bulkUnassign")
    out.toast = failed
      ? toast("failure", ctx.t("projects.bulkUnassignPartial", {
          count: counts.removed + counts.cleared,
          failed: counts.failed,
        }))
      : toast("success", ctx.t("projects.bulkUnassignSuccess", {
          count: counts.removed + counts.cleared,
        }));
  return out;
}

/**
 * Every project-assignment sync (toggle-on, bulk assign, resync of one
 * project or of all of them) answers with one `ProjectSyncReport`. A failed
 * item is a row the backend kept with status `error` (its cell is red); the
 * fold names each one by skill and tool label. There is no conflict or skip
 * outcome here, so the precedence collapses to failure › success; an
 * `already_assigned` or `skipped` (Disabled) item is neither — except that a
 * toggle-on of a disabled skill says the assignment was recorded, not synced.
 * Nothing reloads: the project world
 * applies the view the mutation returned.
 *
 * - `toggleOn`: success confirms like the global toggle; a failure is its
 *   error entry alone (no success claim, no summary for a batch of one).
 * - `bulkAssign`: success counts the tools newly assigned; a failure adds
 *   a partial summary; disabled skips are counted as "not synced (disabled)",
 *   never as "already assigned" or as failures.
 * - `resync` / `resyncAll`: synced/failed summary, plus the disabled-skip
 *   count when there is one.
 */
export function projectSyncOutcome(
  report: ProjectSyncReport,
  ctx: ReportContext & {
    action: "toggleOn" | "bulkAssign" | "resync" | "resyncAll";
    /** Project names by id — a resync of every project names the project in each failure. */
    projectLabelById?: Readonly<Record<string, string>>;
  },
): Outcome<PlainEntry> {
  const out = empty();
  out.completion.reload = false;
  const counts = projectSyncCounts(report);
  let collisions = 0;
  for (const item of report.items) {
    // "Keep existing" in the overwrite ask (ticket 04): a quiet note, never
    // an error — the row is kept `disabled` (D9, D10).
    if (item.status.status === "declined") {
      out.notes.push(declinedNote(ctx, item.skill_name, item.tool));
      continue;
    }
    if (item.status.status !== "failed") continue;
    const subject = { skillId: item.skill_id, skillName: item.skill_name, tool: item.tool, projectId: item.project_id };
    const collision = unansweredCollision(item.status.error);
    if (collision) {
      collisions++;
      out.warnings.push(collisionEntry(ctx, collision, subject));
      continue;
    }
    const namesProject = ctx.action === "resyncAll";
    const title =
      namesProject
        ? ctx.t("errors.syncFailedInProjectTitle", {
            name: item.skill_name,
            tool: label(ctx, item.tool),
            project: ctx.projectLabelById?.[item.project_id] ?? item.project_id,
          })
        : ctx.t("errors.syncFailedTitle", {
            name: item.skill_name,
            tool: label(ctx, item.tool),
          });
    out.errors.push({
      title,
      message: errorMessage(ctx, item.status.error),
      subject: namesProject ? { ...subject, projectInTitle: true } : subject,
      error: item.status.error,
    });
  }
  // An unanswered collision is "to review", never counted as failed (D10).
  counts.failed -= collisions;
  const failed = counts.failed > 0;
  out.completion.closeModal = !failed;
  const { t } = ctx;
  switch (ctx.action) {
    case "toggleOn":
      if (!failed && counts.declined > 0)
        out.toast = toast("declined", out.notes.join("\n"));
      else if (!failed)
        out.toast = counts.skipped > 0
          ? toast("skipped", t("projects.assignedWhileDisabled"))
          : toast("success", t("status.syncEnabled"));
      break;
    case "bulkAssign": {
      // A disabled skip is an assignment recorded but not synced — never
      // "already assigned", never a failure.
      const { synced: assigned, failed: failures, skipped } = counts;
      if (failed)
        out.toast = toast("failure", skipped > 0
          ? t("disabledSkips.bulkAssignPartial", { assigned, failed: failures, skipped })
          : t("projects.bulkAssignPartial", { assigned, failed: failures }));
      else if (skipped > 0)
        out.toast = toast("skipped", assigned > 0
          ? t("disabledSkips.bulkAssignMixed", { assigned, skipped })
          : t("disabledSkips.bulkAssignNotSynced", { count: skipped }));
      else
        out.toast = toast("success", assigned > 0
          ? t("projects.bulkAssignSuccess", { count: assigned })
          : t("projects.bulkAssignNothing"));
      break;
    }
    case "resync":
    case "resyncAll": {
      const all = ctx.action === "resyncAll";
      const { synced, failed: failures, skipped } = counts;
      const key = `${skipped > 0 ? "disabledSkips" : "projects"}.${all ? "resyncAll" : "resync"}${failed ? "Partial" : "Success"}`;
      // A disabled skip is counted in the copy but is by design: it keeps
      // the success tone (the pre-D10 contract of this fold).
      out.toast = toast(failed ? "failure" : "success", t(key, {
        synced,
        ...(failed ? { failed: failures } : {}),
        ...(skipped > 0 ? { skipped } : {}),
      }));
      break;
    }
  }
  // Declined pairs ride as the detail of a count-bearing project toast.
  if (out.toast && ctx.action !== "toggleOn" && out.notes.length && !out.toast.detail)
    out.toast = { ...out.toast, detail: detailOf(out.notes) };
  // Unanswered collisions join the headline as "to review" (conflict tone
  // unless something failed).
  if (collisions > 0) {
    const line = t("runReport.collisions", { count: collisions });
    // A toggle-on that synced nothing says only what is to review.
    const sentence = ctx.action === "toggleOn" && counts.synced === 0 ? null : out.toast;
    out.toast = sentence && out.toast
      ? toast(
          out.toast.tone === "failure" ? "failure" : "conflict",
          [out.toast.message, line].join(HEADLINE_SEPARATOR),
          out.toast.detail,
        )
      : toast(failed ? "failure" : "conflict", line);
  }
  return out;
}

/**
 * Project removal and tool-set configuration (which removes the dropped
 * tools' artifacts) settle per target under ADR-0002: a row whose artifact
 * stayed is kept with status `error`, and for `removeProject` the project
 * itself is kept too. The fold names every failed target; the toast says
 * what was kept. An empty report is a normal outcome here (a project or
 * dropped tool with no deployments), never a "nothing planned" warning.
 * Neither action reloads: the project world applies what the mutation
 * returned.
 */
export function projectRemovalOutcome(
  report: RemovalReport,
  ctx: ReportContext & { action: "removeProject" | "configureTools" },
): Outcome<PlainEntry> {
  const out = empty();
  out.completion.reload = false;
  const counts = removalCounts(report);
  const failed = collectRemovalFailures(
    out,
    report,
    ctx,
    "errors.projectRemovalFailedTitle",
  );
  out.completion.closeModal = !failed;
  if (ctx.action === "removeProject")
    out.toast = failed
      ? toast("failure", ctx.t("projects.removeKept", {
          count: counts.removed,
          failed: counts.failed,
        }))
      : toast("success", ctx.t("projects.removeComplete"));
  if (ctx.action === "configureTools" && failed)
    out.toast = toast("failure", ctx.t("projects.toolRemovalKept", {
      count: counts.removed,
      failed: counts.failed,
    }));
  return out;
}

/**
 * A selected tool that is not detected is skipped per skill
 * (`TOOL_NOT_INSTALLED`); the operator sees it once per tool, as a warning:
 * nothing failed, the selection is stale (round 12 D3). `count` is how many
 * skills the skip covered.
 */
class NotInstalledSkips {
  private readonly perTool = new Map<string, number>();
  /** Absorb the result when it is such a skip; false leaves it to the caller. */
  absorb(result: BatchTargetOutcome): boolean {
    if (
      result.status.status !== "skipped" ||
      result.status.error.code !== "TOOL_NOT_INSTALLED"
    )
      return false;
    this.perTool.set(result.tool_key, (this.perTool.get(result.tool_key) ?? 0) + 1);
    return true;
  }
  warnings(ctx: ReportContext): PlainEntry[] {
    return Array.from(this.perTool, ([tool, count]) => ({
      title: ctx.t("errors.syncSkippedNotInstalledTitle", {
        tool: label(ctx, tool),
      }),
      message: ctx.t("errors.syncSkippedNotInstalledMessage", { count }),
    }));
  }
}

/** A pair the batch skipped because it is Disabled (either grain) — by
 * design, so every batch fold treats it as a skip, never a failure. */
function isDisabledSkip(result: BatchTargetOutcome): boolean {
  return result.status.status === "skipped" && result.status.error.code === "TARGET_DISABLED";
}

/** What a batch report's pairs settled as, beside the entries the fold raised. */
type SyncTally = { synced: number; failed: number; skipped: number; declined: number; collisions: number };

/** Bulk and install surface not-detected skips as per-tool warnings and
 * other bulk skips silently; selected install targets surface unwritable
 * skips; an explicit toggle surfaces every non-success, including
 * TARGET_EXISTS. A declined pair ("Keep existing") is never an entry: it is
 * one quiet note. Bulk answers the composite count line ("7 synced · 1 failed
 * · 2 skipped · 1 kept existing"); a toggle keeps its sentence, or the
 * declined note itself; install leaves the toast to `installOutcome`. */
export function syncOutcome(
  report: BatchTargetOutcome[],
  ctx: ReportContext & { action: "bulk" | "install" | "toggle" },
): Outcome<PlainEntry> {
  const { out, tally, disabledSkip } = foldSync(report, ctx);
  if (ctx.action === "bulk") {
    const counts = { failed: tally.failed, skipped: tally.skipped, declined: tally.declined, collisions: tally.collisions };
    out.toast = headlineToast(ctx.t, "synced", { done: tally.synced, ...counts }, { detail: detailOf(out.notes) });
    out.tally = { done: [{ verb: "synced", count: tally.synced }], counts };
  }
  // No new copy: failed explicit actions have their error entries, not a success.
  else if (ctx.action === "toggle" && out.errors.length === 0) {
    // A dismissed ask: the pair stays as it was, "to review" (D10/D12).
    if (tally.collisions > 0) out.toast = toast("conflict", ctx.t("runReport.collisions", { count: tally.collisions }));
    else if (tally.declined > 0) out.toast = toast("declined", out.notes.join("\n"));
    else if (!disabledSkip) out.toast = toast("success", ctx.t("status.syncEnabled"));
  }
  return out;
}

function foldSync(
  report: BatchTargetOutcome[],
  ctx: ReportContext & {
    action: "bulk" | "install" | "toggle";
    /** The route that produced the report (default `sync`): Enable's collisions are answered by Enable. */
    collisionRoute?: CollisionRoute;
  },
): { out: Outcome<PlainEntry>; tally: SyncTally; disabledSkip: boolean } {
  const out = empty();
  const tally: SyncTally = { synced: 0, failed: 0, skipped: 0, declined: 0, collisions: 0 };
  const skips = new NotInstalledSkips();
  let disabledSkip = false;
  for (const result of report) {
    const status = result.status;
    if (status.status === "synced") {
      tally.synced++;
      continue;
    }
    if (status.status === "declined") {
      tally.declined++;
      out.notes.push(declinedNote(ctx, result.skill_name, result.tool_key));
      continue;
    }
    // A Disabled pair is skipped by design, never a failure: silent in bulk
    // and install; an explicit toggle says why nothing happened.
    const subject = { skillId: result.skill_id, skillName: result.skill_name, tool: result.tool_key };
    if (isDisabledSkip(result)) {
      tally.skipped++;
      if (ctx.action === "toggle") {
        disabledSkip = true;
        out.warnings.push({
          title: ctx.t("errors.syncSkippedDisabledTitle", {
            name: result.skill_name,
            tool: label(ctx, result.tool_key),
          }),
          message: errorMessage(ctx, status.error),
          subject,
        });
      }
      continue;
    }
    // An unresolved collision (the ask dismissed, or none raised):
    // "to review", never a failure or a skip.
    const collision = unansweredCollision(status.error);
    if (collision) {
      tally.collisions++;
      out.warnings.push(collisionEntry(ctx, collision, subject, ctx.collisionRoute));
      continue;
    }
    if (ctx.action !== "toggle" && skips.absorb(result)) {
      tally.skipped++;
      continue;
    }
    if (
      status.status !== "failed" &&
      ctx.action !== "toggle" &&
      !(ctx.action === "install" && status.error.code === "TOOL_NOT_WRITABLE")
    ) {
      tally.skipped++;
      continue;
    }
    tally.failed++;
    out.errors.push({
      title: ctx.t("errors.syncFailedTitle", {
        name: result.skill_name,
        tool: label(ctx, result.tool_key),
      }),
      message: errorMessage(ctx, status.error),
      subject,
      error: status.error,
      repair: { verb: "retry", tool: result.tool_key },
    });
  }
  out.warnings.push(...skips.warnings(ctx));
  out.completion.closeModal = out.errors.length === 0;
  out.completion.reload = ctx.action !== "toggle" || out.errors.length === 0;
  return { out, tally, disabledSkip };
}

/**
 * Enable is a first sync again: its global rows fold like an explicit toggle
 * (a directory in the way names its path; a pair still disabled at skill grain
 * is a warning), its project rows like a toggle-on. One success toast only
 * when every target synced. Always reloads: enabling rewrites rows even when
 * the sync then fails.
 */
export function enableOutcome(
  report: EnableReport,
  ctx: ReportContext & {
    /** Several skills (the bulk rail): the headline is the count line over the targets, always. */
    batch?: boolean;
  },
): Outcome<PlainEntry> {
  const { out: global, tally } = foldSync(report.global, { ...ctx, action: "toggle", collisionRoute: "enable" });
  const projects = projectSyncOutcome(report.projects, { ...ctx, action: "toggleOn" });
  const projectCounts = projectSyncCounts(report.projects);
  const projectCollisions = report.projects.items.filter(
    (item) => item.status.status === "failed" && unansweredCollision(item.status.error),
  ).length;
  const errors = [...global.errors, ...projects.errors];
  const warnings = [...global.warnings, ...projects.warnings];
  const notes = [...global.notes, ...projects.notes];
  // Counted in targets, global and project alike: an assignment already in
  // place is enabled too.
  const enabledTally: OutcomeTally = {
    done: [{ verb: "enabled", count: tally.synced + projectCounts.synced + projectCounts.alreadyAssigned }],
    counts: {
      failed: tally.failed + projectCounts.failed - projectCollisions,
      skipped: tally.skipped + projectCounts.skipped,
      declined: tally.declined + projectCounts.declined,
      collisions: tally.collisions + projectCollisions,
    },
  };
  const collisions = enabledTally.counts.collisions ?? 0;
  return {
    toast: ctx.batch
      ? toast(headlineTone(enabledTally.counts), tallyHeadline(ctx.t, enabledTally), detailOf(notes))
      : collisions > 0 && errors.length === 0
        ? toast("conflict", ctx.t("runReport.collisions", { count: collisions }))
        : errors.length || warnings.length || notes.length
          ? null
          : toast("success", ctx.t("status.targetsEnabled")),
    errors,
    warnings,
    notes,
    completion: { reload: true, closeModal: errors.length === 0, conflict: false },
    tally: enabledTally,
  };
}

/**
 * Import's headline counts groups: imported (a Replace included) · failed ·
 * targets failed (a target sync or an original's cleanup that failed under
 * an imported group) · skipped (a managed name left alone) · kept existing ·
 * originals to review (kept because they differ). Forced-tool lines and
 * declined notes ride as the toast's detail.
 */
export function importOutcome(
  report: ImportReport,
  ctx: ReportContext,
): Outcome<PlainEntry> {
  const out = empty();
  const { t } = ctx;
  const counts = importCounts(report);
  const forcedLines: string[] = [];
  const skips = new NotInstalledSkips();
  let targetsFailed = 0;
  let declined = 0;
  let divergent = 0;
  let collisions = 0;
  for (const group of report.groups) {
    const name = group.group_name;
    if (group.status.status === "failed") {
      out.errors.push({
        title: t("errors.importFailedTitle", { name }),
        message: errorMessage(ctx, group.status.error),
        error: group.status.error,
      });
      continue;
    }
    const skill: EntrySubject =
      group.status.status === "skipped"
        ? { skillName: name }
        : { skillId: group.status.skill_id, skillName: group.status.skill_name };
    // A managed name with no Replace decision: nothing was touched.
    if (group.status.status === "skipped") {
      out.warnings.push({
        title: t("importGroups.nameManagedTitle", { name }),
        message: t("importGroups.nameManagedMessage", { name }),
      });
      continue;
    }
    const forcedTools =
      group.status.status === "imported" ? group.status.forced_tools : [];
    const originals =
      group.status.status === "imported" ? group.status.originals : [];
    for (const tool of forcedTools)
      forcedLines.push(
        t("status.importSourceToolForced", { name, tool: label(ctx, tool) }),
      );
    for (const target of group.status.targets) {
      if (target.status.status === "synced" || skips.absorb(target)) continue;
      // The same skip policy as the bulk sync fold: a Disabled pair is
      // skipped by design, silently — never a sync failure.
      if (isDisabledSkip(target)) continue;
      if (target.status.status === "declined") {
        declined++;
        out.notes.push(declinedNote(ctx, name, target.tool_key));
        continue;
      }
      const subject = { ...skill, tool: target.tool_key };
      const collision = unansweredCollision(target.status.error);
      if (collision) {
        collisions++;
        out.warnings.push(collisionEntry(ctx, collision, subject));
        continue;
      }
      targetsFailed++;
      out.errors.push({
        title: t("errors.syncFailedTitle", {
          name,
          tool: label(ctx, target.tool_key),
        }),
        message: errorMessage(ctx, target.status.error),
        subject,
        error: target.status.error,
        repair: { verb: "retry", tool: target.tool_key },
      });
    }
    for (const original of originals) {
      const tool = label(ctx, original.tool);
      if (original.status.status === "kept_divergent") {
        divergent++;
        out.warnings.push({
          title: t("errors.importKeptDivergentTitle", { name, tool }),
          message: t("errors.importKeptDivergentMessage", {
            path: original.path,
          }),
          subject: { ...skill, tool: original.tool },
        });
      } else if (original.status.status === "failed") {
        targetsFailed++;
        out.errors.push({
          title: t("errors.importCleanupFailedTitle", { name, tool }),
          message: errorMessage(ctx, original.status.error),
          subject: { ...skill, tool: original.tool },
          error: original.status.error,
        });
      }
    }
  }
  // Warnings keep the modal open (kept_divergent needs a look), but a stale
  // selection is not the import's problem: the skip warnings do not.
  out.completion.closeModal =
    out.errors.length === 0 && out.warnings.length === collisions;
  out.warnings.push(...skips.warnings(ctx));
  out.toast = headlineToast(t, "imported", {
    done: counts.imported + counts.replaced,
    failed: counts.failed,
    targetsFailed,
    skipped: counts.skipped,
    declined,
    review: divergent,
    collisions,
  }, { review: "originals", detail: detailOf([...forcedLines, ...out.notes]) });
  return out;
}

/** Install commands return InstallResultDto (not a backend batch report).
 * The picker collects these settled results, including acquisition exceptions.
 * `cancelled` is a candidate the batch never reached once the operator
 * cancelled — the shape ticket 04's install batch answers (D10). */
export type InstallDeployment =
  | { status: "disabled" }
  | { status: "no-targets" }
  | { status: "reported"; report: BatchTargetOutcome[] }
  | { status: "failed"; error: unknown };
export type InstallSettlement = { name: string } & (
  | {
      status: "installed";
      result: InstallResultDto;
      deployment: InstallDeployment;
    }
  | { status: "failed"; error: unknown }
  | { status: "cancelled" }
);

/**
 * The install batch's report (`installSelection`, ticket 04) as the fold's
 * settlements, in candidate order. The request says whether the action
 * deployed at all (`deploy`, auto-sync) and which Tools it named — the
 * backend answers an empty target list for both "not deployed" and "no
 * Tools", and the fold tells them apart.
 */
export function installBatchSettlements(
  report: InstallBatchReport,
  request: { deploy: boolean; tools: readonly string[] },
): InstallSettlement[] {
  return report.candidates.map((candidate): InstallSettlement => {
    const status = candidate.status;
    if (status.status === "cancelled") return { name: candidate.name, status: "cancelled" };
    if (status.status === "failed")
      return { name: candidate.name, status: "failed", error: status.error };
    const deployment: InstallDeployment = !request.deploy
      ? { status: "disabled" }
      : request.tools.length === 0
        ? { status: "no-targets" }
        : { status: "reported", report: status.targets };
    return { name: status.skill.name, status: "installed", result: status.skill, deployment };
  });
}
/**
 * The headline counts candidates: installed · failed · targets failed (a
 * deployment problem under an installed skill) · kept existing · cancelled.
 * A clean single git/local install keeps its sentence; nothing installed and
 * nothing cancelled leaves the error entries alone, without a toast.
 */
export function installOutcome(
  report: InstallSettlement[],
  ctx: ReportContext & { source: "git" | "local" | "selection" },
): Outcome<PlainEntry> {
  const out = empty();
  const counts = { done: 0, failed: 0, targetsFailed: 0, declined: 0, cancelled: 0, collisions: 0 };
  for (const item of report) {
    if (item.status === "cancelled") {
      counts.cancelled++;
      continue;
    }
    if (item.status === "failed") {
      counts.failed++;
      const error = toCommandError(item.error);
      out.errors.push({
        title: ctx.t("errors.importFailedTitle", { name: item.name }),
        message: errorMessage(ctx, item.error),
        error,
      });
      continue;
    }
    counts.done++;
    const deployment = item.deployment;
    if (deployment.status === "reported") {
      const { out: deployed, tally } = foldSync(deployment.report, { ...ctx, action: "install" });
      counts.targetsFailed += deployed.errors.length;
      counts.declined += tally.declined;
      counts.collisions += tally.collisions;
      out.errors.push(...deployed.errors);
      out.warnings.push(...deployed.warnings.filter((entry) => entry.review));
      out.notes.push(...deployed.notes);
    } else if (deployment.status !== "disabled") {
      counts.targetsFailed++;
      out.errors.push({
        subject: { skillId: item.result.skill_id, skillName: item.result.name },
        title: ctx.t("errors.unsyncedTitle", { name: item.result.name }),
        message:
          deployment.status === "no-targets"
            ? ctx.t("errors.noSyncTargets")
            : errorMessage(ctx, deployment.error),
      });
    }
  }
  out.completion.reload = counts.done > 0;
  out.completion.closeModal = out.completion.reload;
  const clean = headlineTone(counts) === "success";
  if (clean && counts.done > 0 && ctx.source !== "selection")
    out.toast = toast(
      "success",
      ctx.t(ctx.source === "git" ? "status.gitSkillCreated" : "status.localSkillCreated"),
    );
  else if (counts.done > 0 || counts.cancelled > 0)
    out.toast = headlineToast(ctx.t, "installed", counts, { detail: detailOf(out.notes) });
  return out;
}

/**
 * One skill's Delete as it settled: its `RemovalReport`, or the whole-command
 * error its `delete_managed_skill` threw (a skill of a bulk Remove that could
 * not even be planned). A bulk Remove is one settlement per skill, folded as
 * one run.
 */
export type DeleteSettlement = { skillId: string; name: string } & (
  | { report: RemovalReport }
  | { error: unknown }
);

const deleted = (settlement: DeleteSettlement) =>
  "report" in settlement && settlement.report.record_deleted;

/** A skill whose command answered `CANCELLED`: an outcome kind, never a failure (D10). */
const cancelledDelete = (settlement: DeleteSettlement) =>
  "error" in settlement &&
  typeof settlement.error === "object" &&
  settlement.error !== null &&
  (settlement.error as { code?: unknown }).code === "CANCELLED";

/**
 * Delete keeps failed rows and the dialog available for retry (it becomes the
 * report, D16). One skill reads as a sentence; several as the count line over
 * skills ("10 removed · 2 failed · 1 cancelled", D10). Each kept target is an
 * error entry (named by skill in a batch); a thrown skill is one entry of its
 * own, unless it was cancelled — counted, neutral, no entry. The dialog closes
 * once nothing failed.
 */
export function deleteOutcome(
  settled: readonly DeleteSettlement[],
  ctx: ReportContext,
): Outcome<PlainEntry> {
  const out = empty();
  const batch = settled.length > 1;
  let keptTargets = 0;
  for (const settlement of settled) {
    if (cancelledDelete(settlement)) continue;
    const subject = { skillId: settlement.skillId, skillName: settlement.name };
    if (!("report" in settlement)) {
      out.errors.push({
        title: ctx.t("deleteReport.failedTitle", { name: settlement.name }),
        message: errorMessage(ctx, settlement.error),
        subject,
        error: toCommandError(settlement.error),
        repair: { verb: "delete" },
      });
      continue;
    }
    const part = empty();
    collectRemovalFailures(part, settlement.report, ctx, "errors.deleteKeptTargetTitle", {
      [settlement.skillId]: settlement.name,
    });
    keptTargets += removalCounts(settlement.report).failed;
    // A kept target is finished by removing the skill again (the Delete
    // dialog, D16), never by a sync.
    const kept = part.errors.map((entry): OutcomeEntry => ({ ...entry, repair: { verb: "delete" } }));
    out.errors.push(
      ...(batch ? kept.map((entry) => ({ ...entry, title: `${settlement.name}: ${entry.title}` })) : kept),
    );
  }
  const removed = settled.filter(deleted).length;
  const cancelled = settled.filter(cancelledDelete).length;
  const failed = settled.length - removed - cancelled;
  out.completion.closeModal = failed === 0;
  if (batch) out.toast = headlineToast(ctx.t, "removed", { done: removed, failed, cancelled });
  else if (cancelled) out.toast = toast("neutral", ctx.t("deleteCancel.single", { name: settled[0].name }));
  else if (failed)
    // A kept target names the count; a skill that threw before anything was
    // planned kept no target, so it reads as "not deleted", never "0 targets".
    out.toast =
      keptTargets > 0
        ? toast("failure", ctx.t("status.skillDeleteKept", { failed: keptTargets }))
        : toast("failure", ctx.t("deleteReport.failedTitle", { name: settled[0].name }));
  else out.toast = toast("success", ctx.t("status.skillRemoved"));
  return out;
}

/** One kept target of a Delete, as the report dialog shows it. */
export type DeleteKeptTarget = {
  path: string;
  /** Tool labels whose rows describe the path (a project's are "Tool · project"). */
  tools: string[];
  /** The typed failure, and its words (`describeCommandError`). */
  error: unknown;
  message: string;
};

/** One skill of a Delete, as the report dialog shows it (D16). */
export type DeleteReportRow = {
  skillId: string;
  name: string;
  deleted: boolean;
  /** Where it was removed from: Tool labels (project rows "Tool · project"). */
  removedFrom: string[];
  kept: DeleteKeptTarget[];
  /** A whole-command failure for this skill (nothing was planned), in words. */
  failure: string | null;
};

/**
 * The Delete dialog's report state: one row per skill, kept skills first
 * (they carry Reveal / Retry), in settlement order otherwise. Only the
 * `RemovalReport` decides what went and what stayed; the words come from the
 * Tool catalog's labels and `describeCommandError`.
 */
export function deleteReportRows(
  settled: readonly DeleteSettlement[],
  ctx: ReportContext & { projectLabelById?: Readonly<Record<string, string>> },
): DeleteReportRow[] {
  const rowLabel = (row: RemovalReport["targets"][number]["rows"][number]) =>
    row.scope === "assignment"
      ? `${label(ctx, row.tool)} · ${ctx.projectLabelById?.[row.project_id] ?? row.project_id}`
      : label(ctx, row.tool);
  const unique = (values: string[]) => [...new Set(values)];
  const rows = settled.map((settlement): DeleteReportRow => {
    if (!("report" in settlement))
      return {
        skillId: settlement.skillId,
        name: settlement.name,
        deleted: false,
        removedFrom: [],
        kept: [],
        failure: errorMessage(ctx, settlement.error),
      };
    const { report } = settlement;
    return {
      skillId: settlement.skillId,
      name: settlement.name,
      deleted: report.record_deleted,
      removedFrom: unique(
        report.targets.filter((target) => target.status.status === "removed").flatMap((target) => target.rows.map(rowLabel)),
      ),
      kept: report.targets.flatMap((target) =>
        target.status.status === "failed"
          ? [
              {
                path: target.path,
                tools: unique(target.rows.map(rowLabel)),
                error: target.status.error,
                message: errorMessage(ctx, target.status.error),
              },
            ]
          : [],
      ),
      failure: null,
    };
  });
  return [...rows.filter((row) => !row.deleted), ...rows.filter((row) => row.deleted)];
}

/** D10's precedence as a rank: the strongest tone of several parts wins. */
const TONE_RANK: Record<OutcomeTone, number> = {
  failure: 5,
  conflict: 4,
  skipped: 3,
  declined: 2,
  neutral: 1,
  success: 0,
};
const strongest = (tones: OutcomeTone[]): OutcomeTone | null =>
  tones.reduce<OutcomeTone | null>(
    (best, tone) => (best === null || TONE_RANK[tone] > TONE_RANK[best] ? tone : best),
    null,
  );

/** Several tallies as one: done counts per verb (first-seen order), shared kinds summed. */
function mergeTallies(tallies: OutcomeTally[]): OutcomeTally {
  const done: OutcomeTally["done"] = [];
  const counts: Record<Exclude<keyof HeadlineCounts, "done">, number> = {
    failed: 0, targetsFailed: 0, skipped: 0, declined: 0, cancelled: 0, review: 0, collisions: 0,
  };
  let review: ReviewKind | undefined;
  for (const tally of tallies) {
    for (const { verb, count } of tally.done) {
      const same = done.find((entry) => entry.verb === verb);
      if (same) same.count += count;
      else done.push({ verb, count });
    }
    for (const key of Object.keys(counts) as (keyof typeof counts)[])
      counts[key] += tally.counts[key] ?? 0;
    review ??= tally.review;
  }
  return { done, counts, ...(review ? { review } : {}) };
}

/**
 * One operator action that settled several reports (a group's Deploy all:
 * a first sync and an Enable, asked once) as one Outcome — nothing a part
 * said is lost. The parts that carry a `tally` make ONE composite count line
 * ("1 synced · 1 enabled · 1 kept existing"), every done verb and every
 * shared kind summed; with no tally the first toast heads. The tone is the
 * strongest across every part (D10: failure › conflict › skipped › declined ›
 * success) — any part's counts, any other toast, any error entry. Every
 * part's entries and notes are kept; the notes (and an untallied part's
 * toast sentence) ride as the detail. One completion: reload / conflict if
 * any part asks; close only when every part may. The reporter executes it
 * once: one toast set, one history block.
 */
export function combineOutcomes<Entry>(
  parts: Outcome<Entry>[],
  ctx: Pick<ReportContext, "t">,
): Outcome<Entry> {
  const errors = parts.flatMap((part) => part.errors);
  const notes = parts.flatMap((part) => part.notes);
  const tallies = parts.flatMap((part) => (part.tally ? [part.tally] : []));
  const tally = tallies.length ? mergeTallies(tallies) : undefined;
  const headPart = tally ? null : (parts.find((part) => part.toast) ?? null);
  // A sentence toast not already heading the line survives as a detail line
  // (a declined toast's sentence is its notes, already there).
  const sentences = parts.flatMap((part) =>
    !part.tally && part !== headPart && part.toast && part.toast.tone !== "declined"
      ? [part.toast.message]
      : [],
  );
  const tone = strongest([
    ...(tally ? [headlineTone(tally.counts)] : []),
    ...parts.flatMap((part) => (part.toast ? [part.toast.tone] : [])),
    ...(errors.length ? (["failure"] as const) : []),
  ]);
  const message = tally ? tallyHeadline(ctx.t, tally) : (headPart?.toast?.message ?? null);
  const detail = detailOf([
    ...(headPart?.toast?.detail ? [headPart.toast.detail] : []),
    ...sentences,
    ...parts.filter((part) => part !== headPart).flatMap((part) => part.notes),
  ]);
  return {
    toast: message !== null && tone ? toast(tone, message, detail) : null,
    errors,
    warnings: parts.flatMap((part) => part.warnings),
    notes,
    completion: {
      reload: parts.some((part) => part.completion.reload),
      closeModal: parts.every((part) => part.completion.closeModal),
      conflict: parts.some((part) => part.completion.conflict),
    },
    ...(tally ? { tally } : {}),
  };
}
