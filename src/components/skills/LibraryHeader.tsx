import { memo } from "react";
import { Plus, RefreshCw, Wrench } from "lucide-react";
import type { TFunction } from "i18next";
import { Button } from "@/components/ui/button";
import { showsHeaderProgress, type PendingAction } from "../../lib/actionLifecycle";
import type { LibraryExceptions } from "../../lib/skillPresentation";
import HeaderProgress from "../shell/HeaderProgress";

type LibraryHeaderProps = {
  librarySize: number;
  exceptions: LibraryExceptions;
  /** Apply the Issues chip (the summary's "View N skills with issues"). */
  onViewIssues: () => void;
  /** Why the verbs are disabled while an action runs (D19), or null. */
  blockedReason: string | null;
  /** The reporter's running action and step line, for the header progress (D19). */
  pending: PendingAction | null;
  actionMessage: string | null;
  /** The reporter's `cancelPending` (offered only while the action is cancellable). */
  onCancel: () => void;
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
 * health fold; healthy is silent) — or, while a batch runs, the header
 * progress in its place (D19: "Refreshing 12/80 · Cancel"; the summary is
 * stale until the run settles, and the Issues chip still filters) — then
 * Configure Tools and Refresh all (secondary), Add skill (the primary). While anything runs the verbs stay focusable, disabled with
 * the reason as their tooltip and description ("Refresh is running").
 */
const LibraryHeader = ({
  librarySize,
  exceptions,
  onViewIssues,
  blockedReason,
  pending,
  actionMessage,
  onCancel,
  onRefreshAll,
  onConfigureTools,
  onAddSkill,
  addIsPrimary,
  t,
}: LibraryHeaderProps) => {
  const blocked = blockedReason !== null;
  // A disabled verb keeps focus and says why (tooltip + description).
  const blockedProps = {
    disabled: blocked,
    focusableWhenDisabled: true,
    title: blockedReason ?? undefined,
    "aria-description": blockedReason ?? undefined,
  };
  const { failedTargets, staleTargets, skillsWithIssues } = exceptions;
  const running = showsHeaderProgress(pending);
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

      {skillsWithIssues > 0 && !running ? (
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

      <HeaderProgress pending={pending} message={actionMessage} onCancel={onCancel} placement="header" t={t} />

      <div className="ml-auto flex items-center gap-2">
        <Button variant="ghost" onClick={onConfigureTools} {...blockedProps}>
          <Wrench />
          {t("configureTools")}
        </Button>
        {/* An empty library's one next step is its own body (Add from GitHub /
            a folder); Refresh all and Add skill join once a skill exists. */}
        {librarySize > 0 ? (
          <>
            <Button onClick={onRefreshAll} {...blockedProps}>
              <RefreshCw />
              {t("library.refreshAll")}
            </Button>
            <Button
              variant={addIsPrimary ? "solid" : "outline"}
              tone={addIsPrimary ? "accent" : "neutral"}
              onClick={onAddSkill}
              {...blockedProps}
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
