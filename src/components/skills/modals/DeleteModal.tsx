import { useEffect, useLayoutEffect, useRef } from "react";
import { FolderOpen, TriangleAlert } from "lucide-react";
import type { TFunction } from "i18next";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { FocusFallback } from "@/components/ui/dialog";
import { mainLandmarkFocus } from "@/components/ui/focus-fallback";
import { deleteScope, deleteTotals, type AssignmentsOf } from "../../../lib/bulkCounts";
import { deleteReportRows, type DeleteSettlement } from "../../../lib/reportOutcome";
import type { ManagedSkill } from "../types";

/** How many skill names a bulk confirmation lists before "and N more". */
const LISTED = 6;

type DeleteModalProps = {
  /**
   * The Delete the dialog is about (null = closed; the dialog stays mounted,
   * one identity for its life): the skills still in the catalog and — once a
   * Delete left something behind — its settlements (the report state).
   */
  request: { skills: readonly ManagedSkill[]; report: readonly DeleteSettlement[] | null } | null;
  /** The Delete (or its Retry) is running. */
  pending: boolean;
  /** The running Delete's line ("Removing beta"), shown while it runs. */
  progress: string | null;
  /** Where each skill is assigned (the project world's held views); null while unread. */
  assignmentsOf: AssignmentsOf | null;
  /** The explicit read of every project's view, made as the dialog opens. */
  loadPlacementViews: () => Promise<void>;
  labelOf: (tool: string) => string;
  toolLabelById: Readonly<Record<string, string>>;
  projectLabelById: Readonly<Record<string, string>>;
  onConfirm: (skillIds: string[]) => void;
  /** Retry the skills the report kept. */
  onRetry: (skillIds: string[]) => void;
  /** Show a kept path in the file manager. */
  onReveal: (path: string) => void;
  onRequestClose: () => void;
  /**
   * Where focus goes when the dialog closes and its opener (the row) is gone:
   * the lifecycle's post-Delete target (D21). Falls back to the main landmark.
   */
  fallbackFocus?: FocusFallback;
  t: TFunction;
};

/**
 * Delete (D16) — one dialog, two states. **Confirm** states the scope before
 * anything goes: the Tools and projects that carry the skill, the Edits lost
 * and, for a skill with no external source, that Skills Hub's copy is the
 * only one; several skills read as counts. **Report**: when a Delete left
 * anything behind, the same dialog says what went and what stayed, each kept
 * path with its reason and Reveal in Finder, and offers Retry for the kept
 * skills. Focus never leaves the dialog while the Delete runs (the pressed
 * Remove keeps it, `pending`), and moves to the report's heading when the
 * dialog turns into the report.
 */
export default function DeleteModal({
  request,
  pending,
  progress,
  assignmentsOf,
  loadPlacementViews,
  labelOf,
  toolLabelById,
  projectLabelById,
  onConfirm,
  onRetry,
  onReveal,
  onRequestClose,
  fallbackFocus,
  t,
}: DeleteModalProps) {
  const open = request !== null;
  const skills = request?.skills ?? [];
  const report = request?.report ?? null;
  const cancelRef = useRef<HTMLButtonElement>(null);
  const headingRef = useRef<HTMLHeadingElement>(null);

  // The scope names the projects that carry each skill: read them as it opens.
  useEffect(() => {
    if (open) void loadPlacementViews();
  }, [loadPlacementViews, open]);

  // Turning into the report moves focus to its heading (the words that say
  // what happened), from the Remove / Retry the operator pressed.
  const reporting = report !== null;
  useLayoutEffect(() => {
    if (reporting && !pending) headingRef.current?.focus();
  }, [reporting, pending, report]);

  const scopes = skills.map((skill) => deleteScope(skill, assignmentsOf, labelOf));
  const rows = report ? deleteReportRows(report, { t, toolLabelById, projectLabelById }) : [];
  const kept = rows.filter((row) => !row.deleted);
  const removed = rows.filter((row) => row.deleted);
  // Retry only what is still there to retry.
  const retryIds = kept.map((row) => row.skillId).filter((id) => skills.some((skill) => skill.id === id));
  const many = report ? rows.length > 1 : skills.length > 1;

  const title = report
    ? many
      ? t("deleteDialog.report.titleMany", { count: kept.length, total: rows.length })
      : t("deleteDialog.report.titleOne", { name: kept[0]?.name ?? rows[0]?.name ?? "" })
    : many
      ? t("deleteDialog.titleMany", { count: skills.length })
      : t("deleteDialog.title", { name: skills[0]?.name ?? "" });

  const confirmBody = () => {
    if (scopes.length === 1) {
      const [scope] = scopes;
      return (
        <ul className="m-0 flex list-none flex-col gap-1.5 p-0 text-sm text-foreground">
          <li>{scope.tools.length > 0 ? t("deleteDialog.tools", { tools: scope.tools.join(", ") }) : t("deleteDialog.noTools")}</li>
          <li>
            {scope.projects === null
              ? t("bulk.confirm.readingProjects")
              : scope.projects.length > 0
                ? t("deleteDialog.projects", { projects: scope.projects.join(", ") })
                : t("deleteDialog.noProjects")}
          </li>
          {scope.edits > 0 ? <li>{t("deleteDialog.edits")}</li> : null}
          <li className="text-muted-foreground">{t("deleteDialog.central")}</li>
          {scope.onlyCopy ? (
            <li className="flex items-start gap-2 font-medium text-failed">
              <TriangleAlert className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
              {t("deleteDialog.onlyCopy")}
            </li>
          ) : null}
        </ul>
      );
    }
    const totals = deleteTotals(scopes);
    return (
      <div className="flex flex-col gap-3 text-sm text-foreground">
        <ul className="m-0 flex list-none flex-col gap-1.5 p-0">
          <li>
            {t("deleteDialog.many.targets", {
              targets: t("bulk.count.targets", { count: totals.targets }),
              tools: t("bulk.count.tools", { count: totals.tools }),
            })}
          </li>
          <li>
            {totals.projects === null
              ? t("bulk.confirm.readingProjects")
              : t("deleteDialog.many.projects", { count: totals.projects })}
          </li>
          {totals.edits > 0 ? <li>{t("deleteDialog.many.edits", { count: totals.edits })}</li> : null}
          {totals.onlyCopy > 0 ? (
            <li className="flex items-start gap-2 font-medium text-failed">
              <TriangleAlert className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
              {t("deleteDialog.many.onlyCopy", { count: totals.onlyCopy })}
            </li>
          ) : null}
        </ul>
        <p className="m-0 text-muted-foreground">
          <span className="sr-only">{t("deleteDialog.many.list")}: </span>
          {scopes
            .slice(0, LISTED)
            .map((scope) => scope.name)
            .join(", ")}
          {scopes.length > LISTED ? ` ${t("deleteDialog.many.more", { count: scopes.length - LISTED })}` : null}
        </p>
      </div>
    );
  };

  const reportBody = () => (
    <div className="flex flex-col gap-4 text-sm">
      {kept.map((row) => (
        <section key={row.skillId} className="flex flex-col gap-1.5" aria-label={many ? row.name : undefined}>
          {many ? <h3 className="m-0 text-sm font-semibold text-foreground">{row.name}</h3> : null}
          {row.failure ? (
            <p className="m-0 text-failed">{row.failure}</p>
          ) : (
            <p className="m-0 text-foreground">
              {row.removedFrom.length > 0
                ? t("deleteDialog.report.removedFrom", { tools: row.removedFrom.join(", ") })
                : t("deleteDialog.report.removedNothing")}
            </p>
          )}
          {row.kept.map((target) => (
            <div key={target.path} className="flex flex-col gap-1 rounded-md border border-border bg-card p-2">
              <p className="m-0 text-foreground">{t("deleteDialog.report.stillIn", { tools: target.tools.join(", ") })}</p>
              <code className="font-mono text-xs break-all text-muted-foreground">{target.path}</code>
              <p className="m-0 text-xs text-failed">{target.message}</p>
              <Button
                size="sm"
                variant="outline"
                className="self-start"
                onClick={() => onReveal(target.path)}
                aria-label={t("deleteDialog.report.revealLabel", { path: target.path })}
              >
                <FolderOpen aria-hidden="true" />
                {t("deleteDialog.report.reveal")}
              </Button>
            </div>
          ))}
        </section>
      ))}
      {removed.length > 0 ? (
        <p className="m-0 text-muted-foreground">
          {t("deleteDialog.report.deleted", { names: removed.map((row) => row.name).join(", ") })}
        </p>
      ) : null}
    </div>
  );

  const confirmIds = skills.map((skill) => skill.id);

  return (
    <Dialog
      open={open}
      dismissible={!pending}
      onOpenChange={(next) => {
        if (!next) onRequestClose();
      }}
    >
      <DialogContent
        size={many ? "lg" : "md"}
        showCloseButton={false}
        initialFocus={cancelRef}
        fallbackFocus={(usable) => fallbackFocus?.(usable) ?? mainLandmarkFocus()}
      >
        <DialogHeader>
          {/* The heading is focusable (never a Tab stop): the report state
              focuses it, and it is the dialog's own place for focus should a
              control it held go away mid-action. */}
          <DialogTitle ref={headingRef} tabIndex={-1} data-delete-heading="">
            {title}
          </DialogTitle>
        </DialogHeader>
        <DialogBody>{report ? reportBody() : confirmBody()}</DialogBody>
        <DialogFooter>
          <p role="status" className="m-0 mr-auto min-w-0 truncate text-xs text-muted-foreground">
            {pending ? progress : null}
          </p>
          {report ? (
            <>
              <Button ref={cancelRef} onClick={onRequestClose} disabled={pending}>
                {t("deleteDialog.report.close")}
              </Button>
              {retryIds.length > 0 ? (
                <Button variant="solid" tone="danger" pending={pending} onClick={() => onRetry(retryIds)}>
                  {retryIds.length > 1
                    ? t("deleteDialog.report.retryMany", { count: retryIds.length })
                    : t("deleteDialog.report.retry")}
                </Button>
              ) : null}
            </>
          ) : (
            <>
              <Button ref={cancelRef} onClick={onRequestClose} disabled={pending}>
                {t("cancel")}
              </Button>
              <Button
                variant="solid"
                tone="danger"
                pending={pending}
                disabled={confirmIds.length === 0}
                onClick={() => onConfirm(confirmIds)}
              >
                {many ? t("deleteDialog.confirmMany", { count: confirmIds.length }) : t("deleteDialog.confirm")}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
