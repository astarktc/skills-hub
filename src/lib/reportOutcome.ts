import type {
  BatchTargetOutcome,
  CommandError,
  EnableReport,
  GitCacheCleanup,
  ImportGroupStatus,
  ImportReport,
  InstallBatchReport,
  InstallResult,
  InvocationEditReport,
  LeftBehindReason,
  LibraryMoveReport,
  ProjectSyncReport,
  PropagationOutcome,
  RefreshReport,
  RemovalReport,
} from "../bindings";
import { describeCommandError, toCommandError } from "../commandError";
import type { DeclinedPair } from "./overwriteAsk";
import type { TranslateFn } from "../hooks/useStatusReporter";
import { ACQUISITION_SKIP_KEY, INVOCATION_LABEL_KEY, SKIPPED_REASON_KEY, toolLabel } from "./skillPresentation";

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
       * Propagation, Edit, install or import (the ordinary re-sync
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
 * The reporter keeps a `failure`-toned toast until it is dismissed.
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
  /**
   * How a declined count is phrased: "kept existing" (a pair's unit), or —
   * where the done counts are groups that may themselves read "kept"
   * (Import) — "existing folders kept", so a target-level decline is never
   * mistaken for a group.
   */
  declined?: "pairs" | "folders";
};
export type ReportContext = {
  t: TranslateFn;
  toolLabelById?: Readonly<Record<string, string>>;
};

/** An entry as a run records it: its kind beside it. */
export type RunItem<Entry extends OutcomeEntry = OutcomeEntry> = Entry & { kind: "error" | "warning" };

/**
 * The order a run's items are recorded and listed in (the reporter's
 * `applyOutcome`). Under a headline the items are free to be read in order
 * of reach (BACKLOG #72): the items about a whole skill — a failed or skipped
 * Update, an Edit to review — first, then the items about one deployment (an
 * entry whose subject names a harness); errors before warnings within each,
 * report order kept. Without a headline the first item heads the run and its
 * toast, so the kinds keep their order: every error, then every warning.
 */
export function runItems<Entry extends OutcomeEntry>(
  errors: readonly Entry[],
  warnings: readonly Entry[],
  options: { headline: boolean },
): RunItem<Entry>[] {
  const items: RunItem<Entry>[] = [
    ...errors.map((entry) => ({ ...entry, kind: "error" as const })),
    ...warnings.map((entry) => ({ ...entry, kind: "warning" as const })),
  ];
  if (!options.headline) return items;
  const aboutDeployment = (item: RunItem<Entry>) => item.subject?.tool !== undefined;
  return [...items.filter((item) => !aboutDeployment(item)), ...items.filter(aboutDeployment)];
}
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
  /** Import: a managed name whose copies were replaced by the managed skill (≥ 1 target deployed). */
  | "replaced"
  /** Import: a managed name the operator chose Replace for and then kept every copy in the ask. */
  | "kept"
  | "removed"
  | "assigned"
  | "enabled"
  | "disabled"
  /** Move Library folder: skills whose central copy moved. */
  | "moved"
  /** Move Library folder: Deployments re-pointed into the new folder. */
  | "relinked"
  | "unassigned";
export const HEADLINE_DONE_KEY: Record<HeadlineVerb, string> = {
  refreshed: "outcome.done.refreshed",
  installed: "outcome.done.installed",
  synced: "outcome.done.synced",
  imported: "outcome.done.imported",
  replaced: "outcome.done.replaced",
  kept: "outcome.done.kept",
  removed: "outcome.done.removed",
  assigned: "outcome.done.assigned",
  enabled: "outcome.done.enabled",
  disabled: "outcome.done.disabled",
  moved: "outcome.done.moved",
  relinked: "outcome.done.relinked",
  unassigned: "projectsSurface.done.unassigned",
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
  part(tally.declined === "folders" ? "outcome.declinedFolders" : "outcome.declined", counts.declined);
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
 * producer raised none. This is not proof that an ask happened. It is "to review": the pair stays as it was,
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
 * counts as one without claiming an artifact was removed. `flipped` counts the
 * markers a Disable recorded `disabled` (`disabled_markers`, #95): intent
 * recorded, nothing removed.
 */
function removalCounts(report: RemovalReport) {
  const counts = {
    removed: 0,
    failed: 0,
    cleared: report.cleared_markers.length,
    flipped: report.disabled_markers.length,
  };
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

/**
 * What one sent Import group settled as — the one settlement rule the Result
 * rows and the headline share (audit F3 / F4). An import, a skipped managed
 * name and a failed admission are the group's own status. A Replace is what
 * its settled target rows say: `replaced` when at least one target was
 * deployed; `failed` when none was and a target failed; `kept` when the
 * operator answered Keep for every collision (and nothing was deployed);
 * otherwise `skipped` — every target skipped (disabled, not installed), no
 * target at all, or a collision nobody answered.
 */
export type ImportGroupKind = "imported" | "replaced" | "kept" | "skipped" | "failed";

/** A target row that failed outright (a collision is not one; an unwritable harness is). */
export const isImportTargetFailure = (row: BatchTargetOutcome) =>
  (row.status.status === "failed" && row.status.error.code !== "TARGET_EXISTS") ||
  (row.status.status === "skipped" && row.status.error.code === "TOOL_NOT_WRITABLE");

export function importGroupKind(status: ImportGroupStatus): ImportGroupKind {
  if (status.status !== "replaced") return status.status;
  const rows = status.targets;
  if (rows.some((row) => row.status.status === "synced")) return "replaced";
  if (rows.some(isImportTargetFailure)) return "failed";
  const declined = rows.some((row) => row.status.status === "declined");
  const unanswered = rows.some(
    (row) => row.status.status === "failed" && row.status.error.code === "TARGET_EXISTS",
  );
  return declined && !unanswered ? "kept" : "skipped";
}

function importCounts(report: ImportReport) {
  const counts: Record<ImportGroupKind, number> = { imported: 0, replaced: 0, kept: 0, skipped: 0, failed: 0 };
  for (const group of report.groups) counts[importGroupKind(group.status)]++;
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
 * An Update only reaches deployments that already exist (D34), so the report
 * announces no new deployment. A skill whose Delete was incomplete is a
 * skipped item offering its health row's repair, Retry the Delete (D35). A
 * collision nobody answered is "to review".
 */
export function refreshOutcome(
  report: RefreshReport,
  ctx: ReportContext & {
    single?: { name: string; success: string };
    /** Pairs the operator kept in the overwrite ask the report's collisions raised. */
    declined?: readonly DeclinedPair[];
  },
): Outcome {
  const out: Outcome = empty();
  out.completion.reload = !ctx.single;
  const { t } = ctx;
  const summary = refreshCounts(report);
  const targetErrors: OutcomeEntry[] = [];
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
      const { reason } = status;
      out.warnings.push(
        reason.kind === "delete_incomplete"
          ? {
              title: t("updateSkip.deleteIncompleteTitle", { name: skill.skill_name }),
              message: t("updateSkip.deleteIncompleteMessage"),
              subject,
              // The health row's repair: finish the Delete (its dialog reveals what stayed).
              repair: { verb: "delete" },
            }
          : {
              title: t("errors.refreshSkippedTitle", { name: skill.skill_name }),
              message: t(SKIPPED_REASON_KEY[reason.state]),
              subject,
            },
      );
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
    }
    // A cancelled row is counted in the headline and raises no entry.
  }
  out.errors.push(...targetErrors);
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
      // A row that never held a copy failed its first deploy; otherwise the
      // app's own copy could not be updated.
      title: ctx.t(target.status.first_deploy ? "propagationFirstDeploy.failedTitle" : "errors.propagationFailedTitle", {
        name: skill.skillName,
        tool: label(ctx, tool),
      }),
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
  ctx: ReportContext & {
    action: "all" | "skill" | "toggle" | "bulkUnassign";
    /** Skill names by id: a pair Disable that only recorded intent names its skill. */
    skillNames?: Readonly<Record<string, string>>;
  },
): Outcome<PlainEntry> {
  const out = empty();
  const counts = removalCounts(report);
  const failed = collectRemovalFailures(out, report, ctx, "errors.unsyncFailedTitle");
  // A skill-grain Disable (`skill` / `all`) always records the skill
  // disabled, so it is never "nothing planned" — even with nothing deployed.
  const skillGrain = ctx.action === "skill" || ctx.action === "all";
  // Zero work is neither success nor failure: nothing was planned, so
  // nothing was removed and whatever the operator clicked is still there.
  // `failed` counts failures and must keep meaning exactly that.
  // Clearing a marker (a row that owned no artifact: the app never
  // materialised it, whatever its status) is work done, not "nothing planned", even though no path was touched;
  // so is recording one `disabled` (#95).
  const markersOnly = report.targets.length === 0;
  const noWork = !failed && markersOnly && counts.cleared === 0 && counts.flipped === 0;
  const nothingPlanned = noWork && !skillGrain;
  out.completion.closeModal = !failed && !nothingPlanned;
  if (ctx.action === "toggle") out.completion.reload = !failed;
  if (ctx.action === "bulkUnassign") out.completion.reload = false;
  if (nothingPlanned) {
    out.toast = toast("skipped", ctx.t("unsyncNothingPlanned"));
    return out;
  }
  // The skill is recorded disabled; there was nothing anywhere to remove.
  if (noWork) {
    out.toast = toast("success", ctx.t("status.disabledNothingDeployed"));
    return out;
  }
  // A Disable over several skills (the bulk rail, D8) reads as the count
  // line over its Sync targets: "412 disabled · 3 failed" (D10).
  if (ctx.action === "all")
    out.toast = headlineToast(ctx.t, "disabled", {
      done: counts.removed + counts.cleared + counts.flipped,
      failed: counts.failed,
    });
  // A pair Disable that only recorded its marker removed nothing: it says
  // the pair stays off (#95) rather than that deploying was disabled.
  const [flipped] = report.disabled_markers;
  if (ctx.action === "toggle" && !failed)
    out.toast =
      markersOnly && counts.cleared === 0 && flipped
        ? toast(
            "success",
            ctx.t("status.pairStaysOff", {
              name: ctx.skillNames?.[flipped.skill_id] ?? flipped.skill_id,
              tool: label(ctx, flipped.tool),
            }),
          )
        : toast("success", ctx.t("status.syncDisabled"));
  // One skill off a project's Tools: the count line over its assignments
  // ("2 unassigned · 1 failed"); a kept row stays in the matrix, failed.
  if (ctx.action === "bulkUnassign")
    out.toast = headlineToast(ctx.t, "unassigned", { done: counts.removed + counts.cleared, failed: counts.failed });
  return out;
}

/**
 * Every project-assignment sync (toggle-on, bulk assign, resync of one
 * project or of all of them, a cell's Sync this) answers with one
 * `ProjectSyncReport`. A failed item is a row the backend kept with status
 * `error` (its cell is failed); the fold names each one by skill and tool
 * label. Nothing reloads: the project world applies the view the mutation
 * returned.
 *
 * - `toggleOn` / `syncCell`: a batch of one — success confirms in a
 *   sentence; a failure is its error entry alone.
 * - `bulkAssign`: the count line over the Tools ("2 assigned · 1 failed ·
 *   1 skipped" — a disabled skip is recorded, not synced); nothing new to
 *   assign says so.
 * - `resync` / `resyncAll`: the count line ("12 synced · 1 failed ·
 *   1 skipped"). A project skipped whole because its folder is missing is
 *   **one** warning line ("archived-app: folder not found — Update path or
 *   Remove"), counted once as skipped — never one failure per assignment.
 * - An unanswered collision is "to review" (D10), never a failure.
 */
export function projectSyncOutcome(
  report: ProjectSyncReport,
  ctx: ReportContext & {
    action: "toggleOn" | "syncCell" | "bulkAssign" | "resync" | "resyncAll";
    /** Project names by id — a resync of every project names the project in each failure. */
    projectLabelById?: Readonly<Record<string, string>>;
  },
): Outcome<PlainEntry> {
  const out = empty();
  out.completion.reload = false;
  const counts = projectSyncCounts(report);
  const { t } = ctx;
  const projectLabel = (id: string) => ctx.projectLabelById?.[id] ?? id;
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
        ? t("errors.syncFailedInProjectTitle", {
            name: item.skill_name,
            tool: label(ctx, item.tool),
            project: projectLabel(item.project_id),
          })
        : t("errors.syncFailedTitle", {
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
  // A project left alone whole: one line, its next step in words.
  for (const skip of report.skipped_projects) {
    out.warnings.push({
      title: t("projectsSurface.folderMissingSkip", { project: projectLabel(skip.project_id) }),
      message: t("projectsSurface.folderMissingDetail"),
      subject: { skillName: "", projectId: skip.project_id, projectInTitle: true },
    });
  }
  // An unanswered collision is "to review", never counted as failed (D10).
  counts.failed -= collisions;
  const failed = counts.failed > 0;
  out.completion.closeModal = !failed;
  const skipped = counts.skipped + report.skipped_projects.length;
  const detail = detailOf(out.notes);
  switch (ctx.action) {
    case "toggleOn":
    case "syncCell":
      if (failed) break;
      if (collisions > 0 && counts.synced === 0) {
        out.toast = toast("conflict", t("runReport.collisions", { count: collisions }));
        return out;
      }
      if (counts.declined > 0) out.toast = toast("declined", out.notes.join("\n"));
      else if (report.skipped_projects.length > 0) out.toast = toast("skipped", out.warnings[out.warnings.length - 1].title);
      else if (counts.skipped > 0)
        out.toast = toast("skipped", t(ctx.action === "toggleOn" ? "projects.assignedWhileDisabled" : "projectsSurface.syncedWhileDisabled"));
      else out.toast = toast("success", t(ctx.action === "toggleOn" ? "status.syncEnabled" : "projectsSurface.cellSynced"));
      break;
    case "bulkAssign":
      out.toast =
        counts.synced === 0 && !failed && skipped === 0 && collisions === 0 && counts.declined === 0
          ? toast("success", t("projects.bulkAssignNothing"))
          : headlineToast(t, "assigned", {
              done: counts.synced,
              failed: counts.failed,
              skipped,
              declined: counts.declined,
              collisions,
            }, {
              // A disabled skill's assignment is recorded, not synced: say why.
              detail: detailOf([...(counts.skipped > 0 ? [t("projectsSurface.syncedWhileDisabled")] : []), ...out.notes]),
            });
      return out;
    case "resync":
    case "resyncAll":
      out.toast = headlineToast(t, "synced", {
        done: counts.synced,
        failed: counts.failed,
        skipped,
        declined: counts.declined,
        collisions,
      }, { detail });
      return out;
  }
  // A batch of one: unanswered collisions join its sentence as "to review".
  if (collisions > 0) {
    const line = t("runReport.collisions", { count: collisions });
    out.toast = out.toast
      ? toast(failed ? "failure" : "conflict", [out.toast.message, line].join(HEADLINE_SEPARATOR))
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
  // Removing a project closes its dialog either way: a row whose artifact
  // stayed keeps the project listed with that cell failed (ADR-0002), which
  // the dialog said before it ran. A tool configuration that kept a row
  // stays open so it can be retried.
  out.completion.closeModal = ctx.action === "removeProject" || !failed;
  if (ctx.action === "removeProject")
    out.toast = failed
      ? toast("failure", [
          ctx.t("projectsSurface.removeKept"),
          compositeHeadline(ctx.t, "removed", { done: counts.removed + counts.cleared, failed: counts.failed }),
        ].join(HEADLINE_SEPARATOR))
      : toast("success", ctx.t("projects.removeComplete"));
  if (ctx.action === "configureTools" && failed)
    out.toast = headlineToast(ctx.t, "removed", { done: counts.removed + counts.cleared, failed: counts.failed });
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
    /** Project names by id — a project skipped whole (folder missing) is named. */
    projectLabelById?: Readonly<Record<string, string>>;
  },
): Outcome<PlainEntry> {
  const { out: global, tally } = foldSync(report.global, { ...ctx, action: "toggle", collisionRoute: "enable" });
  // A project skipped whole is one warning line, counted once as skipped.
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
      skipped: tally.skipped + projectCounts.skipped + report.projects.skipped_projects.length,
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
 * Import's headline counts groups, each by what it settled as
 * (`importGroupKind`): imported · replaced · kept · failed · skipped — the
 * group counts reconcile to the sent selections. Target-level kinds are
 * additional dimensions, never summed as groups: deployments failed (a target
 * sync or an original's cleanup under a group that imported or replaced) ·
 * existing folders kept (a declined target under such a group — a `kept`
 * group's declines are that group) · originals to review (a different copy
 * kept without the operator's say) · folders to review (a collision nobody
 * answered). A different copy the operator chose to keep (`keptByChoice`: the
 * review offered keep / remove and the operator kept) is a quiet note, not a
 * review item. Forced-tool lines and notes ride as the toast's detail.
 */
export function importOutcome(
  report: ImportReport,
  ctx: ReportContext & {
    /** Groups whose different copies the operator chose to keep (sent without `remove_divergent`). */
    keptByChoice?: ReadonlySet<string>;
  },
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
    // A managed name with no Replace decision: nothing was touched.
    if (group.status.status === "skipped") {
      out.warnings.push({
        title: t("importGroups.nameManagedTitle", { name }),
        message: t("importGroups.nameManagedMessage", { name }),
      });
      continue;
    }
    const kind = importGroupKind(group.status);
    // Target-level counts belong to a group that went ahead; a Replace that
    // replaced nothing is counted once, as its own kind.
    const wentAhead = kind === "imported" || kind === "replaced";
    const skill: EntrySubject = { skillId: group.status.skill_id, skillName: group.status.skill_name };
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
        if (wentAhead) declined++;
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
      if (wentAhead) targetsFailed++;
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
    const keptByChoice = ctx.keptByChoice?.has(name) ?? false;
    const keptTools: string[] = [];
    for (const original of originals) {
      const tool = label(ctx, original.tool);
      if (original.status.status === "kept_divergent") {
        if (keptByChoice) {
          keptTools.push(tool);
          continue;
        }
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
    if (keptTools.length > 0) {
      const harnesses = [...new Set(keptTools)];
      out.notes.push(
        t("outcome.keptDifferentLine", {
          name,
          count: keptTools.length,
          harnesses: harnesses.join(t("common.listSeparator")),
        }),
      );
    }
  }
  // Warnings keep the modal open (kept_divergent needs a look), but a stale
  // selection is not the import's problem: the skip warnings do not.
  out.completion.closeModal =
    out.errors.length === 0 && out.warnings.length === collisions;
  out.warnings.push(...skips.warnings(ctx));
  const tally: OutcomeTally = {
    done: [
      { verb: "imported", count: counts.imported },
      { verb: "replaced", count: counts.replaced },
      { verb: "kept", count: counts.kept },
    ],
    counts: {
      failed: counts.failed,
      targetsFailed,
      skipped: counts.skipped,
      declined,
      review: divergent,
      collisions,
    },
    review: "originals",
    declined: "folders",
  };
  out.toast = toast(
    headlineTone(tally.counts),
    tallyHeadline(t, tally),
    detailOf([...forcedLines, ...out.notes]),
  );
  return out;
}

/** One candidate of the Add batch as the fold reads it (`installBatchSettlements`
 * adapts the batch report; a whole-command failure settles every candidate
 * `failed`). `cancelled` is a candidate the batch never reached once the
 * operator cancelled — the shape ticket 04's install batch answers (D10). */
export type InstallDeployment =
  | { status: "disabled" }
  | { status: "no-targets" }
  | { status: "reported"; report: BatchTargetOutcome[] }
  | { status: "failed"; error: unknown };
export type InstallSettlement = { name: string } & (
  | {
      status: "installed";
      result: InstallResult;
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
 * Whether one Add candidate settled **completely** — installed, and deployed
 * as asked: every target synced, kept by the operator's own answer
 * (`declined`) or skipped by design (Disabled, not detected, any skip but an
 * unwritable selected Tool); auto-sync off deploys nothing and is complete.
 * A failure, a cancellation, a target that failed, a folder left to review
 * (an unanswered `TARGET_EXISTS`) or no Tools to deploy to is not. The same
 * classification `installOutcome` folds with `foldSync`'s install rules.
 */
export function installSettlementComplete(item: InstallSettlement): boolean {
  if (item.status !== "installed") return false;
  const deployment = item.deployment;
  if (deployment.status === "disabled") return true;
  if (deployment.status !== "reported") return false;
  return deployment.report.every((row) => {
    const status = row.status;
    if (status.status === "synced" || status.status === "declined") return true;
    if (unansweredCollision(status.error)) return false;
    if (status.status === "failed") return false;
    return status.error.code !== "TOOL_NOT_WRITABLE";
  });
}

/**
 * The Add dialog's one close rule (ticket 16, D10): it closes only when
 * **every** candidate it holds settled completely. `installOutcome` applies
 * it to the run it folds; the flow applies it to every entry its Result has
 * accumulated (an earlier run's folder to review is still unresolved after a
 * clean Retry of another candidate).
 */
export function installComplete(report: readonly InstallSettlement[]): boolean {
  return report.length > 0 && report.every(installSettlementComplete);
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
        title: ctx.t("add.failedTitle", { name: item.name }),
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
  // The Add dialog closes only on full success (ticket 16): every candidate
  // installed and deployed as asked — a kept folder is the operator's own
  // answer and counts. A failure, a cancellation, a target that failed or a
  // folder left to review keeps it open on its settlement.
  out.completion.closeModal = installComplete(report);
  const clean = headlineTone(counts) === "success";
  const lone = report.length === 1 && report[0].status === "installed" ? report[0].result.name : null;
  if (clean && lone !== null && ctx.source !== "selection")
    out.toast = toast("success", ctx.t("add.added", { name: lone }));
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

// ---------------------------------------------------------------------------
// Move Library folder
// ---------------------------------------------------------------------------

/**
 * The counts a Move Library folder report states, derived once here: skills
 * `moved`; Deployments `relinked`, left `unchanged` (copies), `failed`;
 * `skipped` broken down — `missing` (Unlocatable skills, not moved),
 * `notOurs` (an owned link that points elsewhere), `folderMissing` (a project
 * whose folder is gone), `disabled`; `kept` — skills not moved because their
 * copy cannot be relocated faithfully or is shared with another record; and
 * `leftBehind` — what stays in the old place beyond the old folder itself
 * (unrecorded entries, an old copy still reached, still recorded or not
 * removable).
 */
export function libraryMoveCounts(report: LibraryMoveReport) {
  const counts = {
    moved: 0,
    missing: 0,
    relinked: 0,
    unchanged: 0,
    failed: 0,
    notOurs: 0,
    folderMissing: 0,
    disabled: 0,
    kept: 0,
    leftBehind: 0,
  };
  for (const skill of report.skills) {
    if (skill.status.status === "moved") counts.moved++;
    else if (skill.status.status === "kept") counts.kept++;
    else counts.missing++;
  }
  for (const deployment of report.deployments) {
    const { status } = deployment;
    if (status.status === "relinked") counts.relinked++;
    else if (status.status === "unchanged") counts.unchanged++;
    else if (status.status === "failed") counts.failed++;
    else if (status.reason === "not_ours") counts.notOurs++;
    else if (status.reason === "folder_missing") counts.folderMissing++;
    else counts.disabled++;
  }
  counts.leftBehind = report.left_behind.filter((item) => item.reason !== "old_library").length;
  return counts;
}

/**
 * Every reason an old copy stays that is a skip, never a failure: the clean
 * kept it on purpose (something still needs it, it is outside the old
 * Library folder, or it is not provably a duplicate any more). Each is one
 * warning, title and message.
 */
type KeptOldCopyReason = Exclude<LeftBehindReason, "old_library" | "unrecorded" | "removal_failed">;
const KEPT_OLD_COPY_KEYS: Record<KeptOldCopyReason, { title: string; message: string }> = {
  still_linked: {
    title: "libraryMove.report.stillLinkedTitle",
    message: "libraryMove.report.stillLinkedMessage",
  },
  still_recorded: {
    title: "libraryMove.report.stillRecordedTitle",
    message: "libraryMove.report.stillRecordedMessage",
  },
  outside_library_folder: {
    title: "libraryMove.report.outsideLibraryFolderTitle",
    message: "libraryMove.report.outsideLibraryFolderMessage",
  },
  changed_since_copy: {
    title: "libraryMove.report.changedSinceCopyTitle",
    message: "libraryMove.report.changedSinceCopyMessage",
  },
  destination_missing: {
    title: "libraryMove.report.destinationMissingTitle",
    message: "libraryMove.report.destinationMissingMessage",
  },
  replaced: {
    title: "libraryMove.report.replacedTitle",
    message: "libraryMove.report.replacedMessage",
  },
  stale_journal: {
    title: "libraryMove.report.staleJournalTitle",
    message: "libraryMove.report.staleJournalMessage",
  },
  set_aside: {
    title: "libraryMove.report.setAsideTitle",
    message: "libraryMove.report.setAsideMessage",
  },
};

function keptOldCopyKeys(reason: LeftBehindReason): { title: string; message: string } | undefined {
  return Object.hasOwn(KEPT_OLD_COPY_KEYS, reason) ? KEPT_OLD_COPY_KEYS[reason as KeptOldCopyReason] : undefined;
}

/**
 * Clean Now's counts → its one toast: "Cleared N cached repos", plus "; M in
 * use, skipped" when an acquisition was using M entries (omitted at zero). A
 * skipped entry is not a failure — the tone is `skipped`. Nothing on screen
 * reads the git cache, so nothing reloads or closes.
 */
export function gitCacheCleanOutcome(
  cleanup: GitCacheCleanup,
  ctx: ReportContext,
): Outcome<PlainEntry> {
  const out = empty();
  out.completion = { reload: false, closeModal: false, conflict: false };
  const cleared = ctx.t("gitCacheClean.cleared", { count: cleanup.cleared });
  out.toast =
    cleanup.in_use > 0
      ? toast("skipped", ctx.t("gitCacheClean.withInUse", { cleared, count: cleanup.in_use }))
      : toast("success", cleared);
  return out;
}

/**
 * Move Library folder's report as one Outcome. The headline is the composite
 * line "12 moved · 30 re-linked · 1 deployment failed · 2 skipped · 3 left
 * behind" (zeros omitted), toned by D10's precedence: a failed re-point or an
 * old copy that could not be removed is a failure; a skipped skill or
 * Deployment, or an old copy the clean kept on purpose (`KEPT_OLD_COPY_KEYS`:
 * still reached or recorded, outside the old folder, or no longer provably a
 * duplicate), is a skip. Entries: each failed re-point (an error, Retry re-syncs the pair from
 * the new copy), each Unlocatable skill, skill kept in place (with why),
 * link pointing elsewhere, missing project folder and kept old copy
 * (warnings). Quiet notes name the old
 * folder and every unrecorded entry left in it. The catalog is replaced from
 * the command's own response, so nothing reloads, and Settings stays open.
 *
 * `resumed`: the report of a move a crash interrupted, finished by the
 * startup resume (or first thing by the next move) — its headline says so.
 */
export function libraryMoveOutcome(
  report: LibraryMoveReport,
  ctx: ReportContext,
  opts: { resumed?: boolean } = {},
): Outcome<PlainEntry> {
  const out = empty();
  out.completion = { reload: false, closeModal: false, conflict: false };
  const counts = libraryMoveCounts(report);
  const tool = (key: string) => ctx.toolLabelById?.[key] ?? toolLabel(ctx.t, key);
  const names = new Map(report.skills.map((skill) => [skill.skill_id, skill.name]));

  for (const skill of report.skills) {
    const subject = { skillId: skill.skill_id, skillName: skill.name };
    if (skill.status.status === "skipped") {
      out.warnings.push({
        title: ctx.t("libraryMove.report.missingTitle", { name: skill.name }),
        message: ctx.t("libraryMove.report.missingMessage"),
        subject,
      });
    } else if (skill.status.status === "kept") {
      const kept = skill.status.reason;
      out.warnings.push({
        title: ctx.t("libraryMove.report.keptTitle", { name: skill.name }),
        message:
          kept.kind === "unrelocatable"
            ? ctx.t(`libraryMove.report.kept.${kept.cause}`, { entry: kept.entry })
            : ctx.t("libraryMove.report.kept.shared_copy", {
                other: names.get(kept.with_skill_id) ?? kept.with_skill_id,
              }),
        subject,
      });
    }
  }
  for (const deployment of report.deployments) {
    const { status } = deployment;
    const row = deployment.rows[0];
    const subject: EntrySubject = {
      skillId: deployment.skill_id,
      skillName: deployment.name,
      ...(row ? { tool: row.tool } : {}),
      ...(row?.scope === "assignment" ? { projectId: row.project_id } : {}),
    };
    const where = row ? tool(row.tool) : deployment.path;
    if (status.status === "failed") {
      out.errors.push({
        title: ctx.t("libraryMove.report.relinkFailedTitle", { name: deployment.name, tool: where }),
        message: errorMessage(ctx, status.error),
        subject,
        error: status.error,
        ...(row && row.scope === "global_target" ? { repair: { verb: "retry" as const, tool: row.tool } } : {}),
      });
    } else if (status.status === "skipped" && status.reason === "not_ours") {
      out.warnings.push({
        title: ctx.t("libraryMove.report.notOursTitle", { name: deployment.name, tool: where }),
        message: ctx.t("libraryMove.report.notOursMessage", { path: deployment.path }),
        subject,
      });
    } else if (status.status === "skipped" && status.reason === "folder_missing") {
      out.warnings.push({
        title: ctx.t("libraryMove.report.folderMissingTitle", { name: deployment.name, tool: where }),
        message: ctx.t("libraryMove.report.folderMissingMessage", { path: deployment.path }),
        subject,
      });
    }
  }
  for (const item of report.left_behind) {
    const name = item.skill_id ? (names.get(item.skill_id) ?? item.skill_id) : "";
    const subject = item.skill_id ? { skillId: item.skill_id, skillName: name } : undefined;
    const kept = keptOldCopyKeys(item.reason);
    if (kept) {
      out.warnings.push({
        title: ctx.t(kept.title, { name }),
        message: ctx.t(kept.message, { path: item.path }),
        ...(subject ? { subject } : {}),
      });
    } else if (item.reason === "removal_failed") {
      out.errors.push({
        title: ctx.t("libraryMove.report.removalFailedTitle", { name }),
        message: item.error ? errorMessage(ctx, item.error) : item.path,
        ...(subject ? { subject } : {}),
        ...(item.error ? { error: item.error } : {}),
      });
    } else if (item.reason === "old_library") {
      out.notes.push(ctx.t("libraryMove.report.oldLibraryLine", { path: item.path }));
    } else {
      out.notes.push(ctx.t("libraryMove.report.unrecordedLine", { path: item.path }));
    }
  }
  if (counts.unchanged > 0) out.notes.push(ctx.t("libraryMove.report.copiesLine", { count: counts.unchanged }));
  if (counts.disabled > 0) out.notes.push(ctx.t("libraryMove.report.disabledLine", { count: counts.disabled }));

  const removalFailed = report.left_behind.filter((item) => item.reason === "removal_failed").length;
  const stillKept = report.left_behind.filter((item) => keptOldCopyKeys(item.reason)).length;
  const skipped = counts.missing + counts.kept + counts.notOurs + counts.folderMissing;
  const parts: string[] = [];
  const part = (key: string, count: number) => {
    if (count > 0) parts.push(ctx.t(key, { count }));
  };
  part(HEADLINE_DONE_KEY.moved, counts.moved);
  part(HEADLINE_DONE_KEY.relinked, counts.relinked);
  part("outcome.targetsFailed", counts.failed);
  part("outcome.skipped", skipped);
  part("libraryMove.report.leftBehind", counts.leftBehind);
  if (parts.length === 0) parts.push(ctx.t(HEADLINE_DONE_KEY.moved, { count: 0 }));
  const tone: OutcomeTone =
    counts.failed > 0 || removalFailed > 0
      ? "failure"
      : skipped > 0 || stillKept > 0
        ? "skipped"
        : "success";
  const summary = parts.join(HEADLINE_SEPARATOR);
  out.toast = toast(
    tone,
    opts.resumed ? ctx.t("libraryMove.report.resumed", { summary }) : summary,
    detailOf(out.notes),
  );
  return out;
}
