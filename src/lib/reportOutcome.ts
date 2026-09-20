import type {
  BatchSyncReportDto,
  ImportReportDto,
  InstallResultDto,
  InvocationEditReportDto,
  PropagationTargetDto,
  RefreshReportDto,
  RemovalReportDto,
  SyncTargetResultDto,
} from "../bindings";
import { describeCommandError } from "../commandError";
import type { ActionErrorEntry, TranslateFn } from "../hooks/useStatusReporter";
import { ACQUISITION_SKIP_KEY, INVOCATION_LABEL_KEY, SKIPPED_REASON_KEY } from "./skillPresentation";

/** Declarative actions survive list replacement; only the hook resolves the id. */
export type OutcomeEntry = Omit<ActionErrorEntry, "action"> & {
  action?: { label: string; skillId: string; skillName: string };
};
export type PlainEntry = Omit<ActionErrorEntry, "action">;
export type Outcome<Entry = OutcomeEntry> = {
  toast: {
    kind: "success" | "warning";
    message: string;
    detail?: string;
  } | null;
  errors: Entry[];
  warnings: Entry[];
  completion: { reload: boolean; closeModal: boolean; conflict: boolean };
};
export type ReportContext = {
  t: TranslateFn;
  toolLabelById?: Readonly<Record<string, string>>;
  canRepoint?: (skillId: string) => boolean;
};
const empty = (): Outcome<PlainEntry> => ({
  toast: null,
  errors: [],
  warnings: [],
  completion: { reload: true, closeModal: true, conflict: false },
});
const label = (ctx: ReportContext, tool: string) =>
  ctx.toolLabelById?.[tool] ?? tool;
const errorMessage = (ctx: ReportContext, error: unknown) =>
  describeCommandError(error, ctx.t) ?? "";

/**
 * Update, Restore and BOTH Re-points use the same batch-of-one policy.
 * Conflict > failure (including targets/reassert) > skip > success, without
 * suppressing any detail entries. Batch reports reload; single-mutation
 * responses already supply the complete catalog, even on failure/skip.
 * Only a fully refreshed batch closes a repair modal; conflicts/target failures
 * do not undo central settlement. Thrown invocation errors are not reports.
 * A batch of one never shows the batch count summary: its failure or skip is
 * already the one entry, so a second toast would repeat it.
 */
export function refreshOutcome(
  report: RefreshReportDto,
  ctx: ReportContext & {
    single?: { name: string; success: string };
  },
): Outcome {
  const out: Outcome = empty();
  out.completion.reload = !ctx.single;
  const { t } = ctx;
  const targetErrors: OutcomeEntry[] = [];
  for (const skill of report.skills) {
    const status = skill.status;
    if (status.status === "failed") {
      out.errors.push({
        title: t("errors.updateFailedTitle", { name: skill.skill_name }),
        message: errorMessage(ctx, status.error),
        ...(status.error.code === "GITHUB_SKILL_NOT_FOUND" &&
        ctx.canRepoint?.(skill.skill_id)
          ? {
              action: {
                label: t("gitRepoint.action"),
                skillId: skill.skill_id,
                skillName: skill.skill_name,
              },
            }
          : {}),
      });
    } else if (status.status === "skipped") {
      out.warnings.push({
        title: t("errors.refreshSkippedTitle", { name: skill.skill_name }),
        message: t(SKIPPED_REASON_KEY[status.state]),
      });
    } else if (status.status === "skipped_acquisition") {
      out.warnings.push({
        title: t("errors.refreshSkippedTitle", { name: skill.skill_name }),
        message: t(ACQUISITION_SKIP_KEY[status.reason]),
      });
    } else {
      targetErrors.push(...propagationErrors(status.targets, skill.skill_name, ctx));
      if (status.reassert_error)
        targetErrors.push({
          title: t("errors.reassertFailedTitle", { name: skill.skill_name }),
          message: errorMessage(ctx, status.reassert_error),
        });
    }
  }
  out.errors.push(...targetErrors);
  for (const skill of report.skills) {
    if (skill.status.status !== "refreshed" || !skill.status.edit_conflict)
      continue;
    const conflict = skill.status.edit_conflict;
    out.completion.conflict = true;
    out.warnings.push({
      title: t("invocationEdit.warningTitle", { name: skill.skill_name }),
      message: t("invocationEdit.refreshWarning", {
        name: skill.skill_name,
        upstream: t(INVOCATION_LABEL_KEY[conflict.upstream_mode]),
        override: t(INVOCATION_LABEL_KEY[conflict.override_mode]),
      }),
    });
  }
  const failed =
    report.failed > 0 || report.target_failures > 0 || out.errors.length > 0;
  const skipped = report.skipped > 0;
  out.completion.closeModal =
    report.refreshed > 0 && report.failed === 0 && !skipped;
  const counts = { refreshed: report.refreshed, failed: report.failed };
  if (ctx.single && !out.completion.conflict && (failed || skipped)) {
    return out;
  }
  // Precedence is explicit: conflict > failure > skipped > success.
  let message: string;
  if (out.completion.conflict) {
    message = ctx.single
      ? t("invocationEdit.updateCompletedWithConflict", { name: ctx.single.name })
      : t("invocationEdit.refreshCompletedWithEdits");
  } else if (failed) {
    message = report.failed > 0 ? t("status.refreshSummary", counts) : t("partialFailure");
  } else if (skipped) {
    message = t("status.refreshSummarySkipped", { ...counts, skipped: report.skipped });
  } else {
    message = ctx.single?.success ?? t("status.refreshCompleted");
  }
  out.toast = {
    kind: out.completion.conflict || failed || skipped ? "warning" : "success",
    message,
  };
  return out;
}

function propagationErrors(
  targets: PropagationTargetDto[],
  name: string,
  ctx: ReportContext,
): PlainEntry[] {
  return targets.flatMap((target) => target.status.status === "failed" ? [{
    title: ctx.t("errors.propagationFailedTitle", { name, tool: label(ctx, target.scope.tool) }),
    message: errorMessage(ctx, target.status.error),
  }] : []);
}

/** Edit has already settled centrally, including a no-op or clear. Expected
 * propagation skips are silent; failures in either scope are notifications,
 * not a saved toast. The accompanying catalog replaces the whole library. */
export function invocationEditOutcome(
  report: InvocationEditReportDto,
  ctx: ReportContext,
): Outcome<PlainEntry> {
  const errors = propagationErrors(report.propagation, report.skill_name, ctx);
  return {
    toast: errors.length ? null : { kind: "success", message: ctx.t("invocationEdit.saved") },
    errors,
    warnings: [],
    completion: { reload: false, closeModal: true, conflict: false },
  };
}

export function removalOutcome(
  report: RemovalReportDto,
  ctx: ReportContext & { action: "all" | "skill" | "toggle" },
): Outcome<PlainEntry> {
  const out = empty();
  for (const target of report.targets) {
    if (target.status.status !== "failed") continue;
    out.errors.push({
      title: ctx.t("errors.unsyncFailedTitle", {
        tool: label(ctx, target.tool),
      }),
      message: errorMessage(ctx, target.status.error),
    });
  }
  const failed = report.failed > 0 || out.errors.length > 0;
  // Zero targets is neither success nor failure: nothing was planned, so
  // nothing was removed and whatever the operator clicked is still there.
  // `failed` counts failures and must keep meaning exactly that.
  const nothingPlanned = !failed && report.targets.length === 0;
  out.completion.closeModal = !failed && !nothingPlanned;
  if (ctx.action === "toggle") out.completion.reload = !failed;
  if (nothingPlanned) {
    out.toast = { kind: "warning", message: ctx.t("unsyncNothingPlanned") };
    return out;
  }
  if (ctx.action === "all")
    out.toast = {
      kind: failed ? "warning" : "success",
      message: failed
        ? ctx.t("unsyncPartial", {
            count: report.removed,
            failed: report.failed,
          })
        : ctx.t("unsyncAllComplete", { count: report.removed }),
    };
  if (ctx.action === "toggle" && !failed)
    out.toast = { kind: "success", message: ctx.t("status.syncDisabled") };
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
  absorb(result: SyncTargetResultDto): boolean {
    if (
      result.status.status !== "skipped" ||
      result.status.error.code !== "TOOL_NOT_INSTALLED"
    )
      return false;
    this.perTool.set(result.tool, (this.perTool.get(result.tool) ?? 0) + 1);
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

/** Bulk and install surface not-detected skips as per-tool warnings and
 * other bulk skips silently; selected install targets surface unwritable
 * skips; an explicit toggle surfaces every non-success, including
 * TARGET_EXISTS. */
export function syncOutcome(
  report: BatchSyncReportDto,
  ctx: ReportContext & { action: "bulk" | "install" | "toggle" },
): Outcome<PlainEntry> {
  const out = empty();
  const skips = new NotInstalledSkips();
  for (const result of report.results) {
    const status = result.status;
    if (status.status === "synced") continue;
    if (ctx.action !== "toggle" && skips.absorb(result)) continue;
    if (
      status.status !== "failed" &&
      ctx.action !== "toggle" &&
      !(ctx.action === "install" && status.error.code === "TOOL_NOT_WRITABLE")
    )
      continue;
    out.errors.push({
      title: ctx.t("errors.syncFailedTitle", {
        name: result.skill_name,
        tool: label(ctx, result.tool),
      }),
      message:
        ctx.action === "toggle" && status.error.code === "TARGET_EXISTS"
          ? ctx.t("errors.targetExistsDetail", { path: status.error.path })
          : errorMessage(ctx, status.error),
    });
  }
  out.warnings.push(...skips.warnings(ctx));
  out.completion.closeModal = out.errors.length === 0;
  out.completion.reload = ctx.action !== "toggle" || out.errors.length === 0;
  // No new copy: failed explicit actions have their error entries, not a success.
  if (ctx.action !== "install" && out.errors.length === 0)
    out.toast = {
      kind: "success",
      message: ctx.t(
        ctx.action === "toggle" ? "status.syncEnabled" : "status.syncCompleted",
      ),
    };
  return out;
}

export function importOutcome(
  report: ImportReportDto,
  ctx: ReportContext,
): Outcome<PlainEntry> {
  const out = empty();
  const { t } = ctx;
  const forcedLines: string[] = [];
  const skips = new NotInstalledSkips();
  for (const group of report.groups) {
    const name = group.group_name;
    if (group.status.status === "failed") {
      out.errors.push({
        title: t("errors.importFailedTitle", { name }),
        message: errorMessage(ctx, group.status.error),
      });
      continue;
    }
    for (const tool of group.status.forced_tools)
      forcedLines.push(
        t("status.importSourceToolForced", { name, tool: label(ctx, tool) }),
      );
    for (const target of group.status.targets) {
      if (target.status.status === "synced" || skips.absorb(target)) continue;
      out.errors.push({
        title: t("errors.syncFailedTitle", {
          name,
          tool: label(ctx, target.tool),
        }),
        message:
          target.status.error.code === "TARGET_EXISTS"
            ? t("errors.syncTargetExistsMessage", {
                path: target.status.error.path,
              })
            : errorMessage(ctx, target.status.error),
      });
    }
    for (const original of group.status.originals) {
      const tool = label(ctx, original.tool);
      if (original.status.status === "kept_divergent")
        out.warnings.push({
          title: t("errors.importKeptDivergentTitle", { name, tool }),
          message: t("errors.importKeptDivergentMessage", {
            path: original.path,
          }),
        });
      else if (original.status.status === "failed")
        out.errors.push({
          title: t("errors.importCleanupFailedTitle", { name, tool }),
          message: errorMessage(ctx, original.status.error),
        });
    }
  }
  // Warnings keep the modal open (kept_divergent needs a look), but a stale
  // selection is not the import's problem: the skip warnings do not.
  out.completion.closeModal =
    out.errors.length === 0 && out.warnings.length === 0;
  out.warnings.push(...skips.warnings(ctx));
  out.toast = {
    kind: out.completion.closeModal ? "success" : "warning",
    message:
      report.failed > 0
        ? t("status.importPartial", {
            imported: report.imported,
            failed: report.failed,
          })
        : out.completion.closeModal
          ? t("status.importCompleted")
          : t("partialFailure"),
    ...(forcedLines.length ? { detail: forcedLines.join("\n") } : {}),
  };
  return out;
}

/** Install commands return InstallResultDto (not a backend batch report).
 * The picker collects these settled results, including acquisition exceptions. */
export type InstallDeployment =
  | { status: "disabled" }
  | { status: "no-targets" }
  | { status: "reported"; report: BatchSyncReportDto }
  | { status: "failed"; error: unknown };
export type InstallSettlement = { name: string } & (
  | {
      status: "installed";
      result: InstallResultDto;
      deployment: InstallDeployment;
    }
  | { status: "failed"; error: unknown }
);
export function installOutcome(
  report: InstallSettlement[],
  ctx: ReportContext & { source: "git" | "local" | "selection" },
): Outcome<PlainEntry> {
  const out = empty();
  for (const item of report) {
    if (item.status === "failed")
      out.errors.push({
        title: ctx.t("errors.importFailedTitle", { name: item.name }),
        message: errorMessage(ctx, item.error),
      });
    else {
      const deployment = item.deployment;
      if (deployment.status === "reported")
        out.errors.push(
          ...syncOutcome(deployment.report, { ...ctx, action: "install" })
            .errors,
        );
      else if (deployment.status !== "disabled")
        out.errors.push({
          title: ctx.t("errors.unsyncedTitle", { name: item.result.name }),
          message:
            deployment.status === "no-targets"
              ? ctx.t("errors.noSyncTargets")
              : errorMessage(ctx, deployment.error),
        });
    }
  }
  out.completion.reload = report.some((item) => item.status === "installed");
  out.completion.closeModal = out.completion.reload;
  if (out.completion.reload)
    out.toast = {
      kind: out.errors.length ? "warning" : "success",
      message: ctx.t(
        out.errors.length ? "partialFailure" : ctx.source === "selection"
          ? "status.selectedSkillsInstalled"
          : ctx.source === "git"
            ? "status.gitSkillCreated"
            : "status.localSkillCreated",
      ),
    };
  return out;
}

/** deleteManagedSkill returns null; cleanup failure is a thrown CommandError. */
export function deleteOutcome(
  _report: null,
  ctx: ReportContext,
): Outcome<PlainEntry> {
  return {
    ...empty(),
    toast: { kind: "success", message: ctx.t("status.skillRemoved") },
  };
}
