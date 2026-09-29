import { memo } from "react";
import { Plus, RefreshCw, Wrench } from "lucide-react";
import type { TFunction } from "i18next";
import { Button } from "@/components/ui/button";
import type { LibraryExceptions } from "../../lib/skillPresentation";

type LibraryHeaderProps = {
  librarySize: number;
  exceptions: LibraryExceptions;
  /** Apply the Issues chip (the summary's "View N skills with issues"). */
  onViewIssues: () => void;
  /** Why the verbs are disabled while an action runs (D19), or null. */
  blockedReason: string | null;
  onRefreshAll: () => void;
  onConfigureTools: () => void;
  onAddSkill: () => void;
  /**
   * Add skill is the view's one primary action, unless reviewing discovered
   * skills is the first task (the discovery outweighs the library).
   */
  addIsPrimary: boolean;
  t: TFunction;
};

/**
 * The My Skills header row: title + count, the exception summary (from the
 * health fold; healthy is silent), a region for the header progress (ticket
 * 14), Configure Tools and Refresh all (secondary), Add skill (the primary).
 */
const LibraryHeader = ({
  librarySize,
  exceptions,
  onViewIssues,
  blockedReason,
  onRefreshAll,
  onConfigureTools,
  onAddSkill,
  addIsPrimary,
  t,
}: LibraryHeaderProps) => {
  const blocked = blockedReason !== null;
  const { failedTargets, staleTargets, skillsWithIssues } = exceptions;
  return (
    <header className="flex min-h-12 shrink-0 flex-wrap items-center gap-x-4 gap-y-2 border-b border-border px-4 py-2">
      <div className="flex min-w-0 items-baseline gap-2">
        <h1
          data-surface-heading="skills"
          tabIndex={-1}
          className="m-0 rounded-sm text-lg font-semibold text-foreground"
        >
          {t("navMySkills")}
        </h1>
        <span className="text-sm text-faint tabular-nums">{t("library.count", { count: librarySize })}</span>
      </div>

      {skillsWithIssues > 0 ? (
        <p className="m-0 flex min-w-0 items-center gap-2 text-sm">
          {failedTargets > 0 ? (
            <span className="text-failed">{t("library.summary.failedTargets", { count: failedTargets })}</span>
          ) : null}
          {failedTargets > 0 && staleTargets > 0 ? <span aria-hidden="true" className="text-faint">·</span> : null}
          {staleTargets > 0 ? (
            <span className="text-stale">{t("library.summary.staleTargets", { count: staleTargets })}</span>
          ) : null}
          {failedTargets > 0 || staleTargets > 0 ? <span aria-hidden="true" className="text-faint">·</span> : null}
          <Button variant="ghost" tone="accent" size="sm" onClick={onViewIssues}>
            {t("library.summary.viewIssues", { count: skillsWithIssues })}
          </Button>
        </p>
      ) : null}

      {/* Ticket 14 renders the running action's progress here ("Refreshing 12/80 · Cancel"). */}
      <div data-slot="header-progress" aria-label={t("library.progressRegion")} role="status" className="min-w-0" />

      <div className="ml-auto flex items-center gap-2">
        <Button variant="ghost" onClick={onConfigureTools} disabled={blocked} title={blockedReason ?? undefined}>
          <Wrench />
          {t("configureTools")}
        </Button>
        {/* An empty library's one next step is its own body (Add from GitHub /
            a folder); Refresh all and Add skill join once a skill exists. */}
        {librarySize > 0 ? (
          <>
            <Button onClick={onRefreshAll} disabled={blocked} title={blockedReason ?? undefined}>
              <RefreshCw />
              {t("library.refreshAll")}
            </Button>
            <Button
              variant={addIsPrimary ? "solid" : "outline"}
              tone={addIsPrimary ? "accent" : "neutral"}
              onClick={onAddSkill}
              disabled={blocked}
              title={blockedReason ?? undefined}
            >
              <Plus />
              {t("addSkillButton")}
            </Button>
          </>
        ) : null}
      </div>
    </header>
  );
};

export default memo(LibraryHeader);
