import { memo } from "react";
import { LoaderCircle } from "lucide-react";
import type { TFunction } from "i18next";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { progressActivity, showsHeaderProgress, type PendingAction } from "../../lib/actionLifecycle";

type HeaderProgressProps = {
  /** The reporter's one running action (`pending`) — the only progress state there is. */
  pending: PendingAction | null;
  /** The reporter's step line (a streamed action's current skill), under the count. */
  message: string | null;
  /** The reporter's `cancelPending`; offered only while the action is cancellable (D20). */
  onCancel: () => void;
  /**
   * Where it sits: `header` — inline in a surface header (My Skills' slot);
   * `bar` — its own strip above an area that has no such slot; `dialog` —
   * the foot of a dialog whose action is running (the page behind a modal
   * is inert, so its Cancel must be the dialog's).
   */
  placement: "header" | "bar" | "dialog";
  /**
   * A dialog's foot shows this same progress (its action runs, or it was
   * opened over one): the page copy behind the modal yields — it keeps its
   * live region but draws nothing — so there is one visible progress at a
   * time (ticket 14 audit N5).
   */
  yielded?: boolean;
  t: TFunction;
};

/**
 * The header progress (D19): what is running, in its own words and counts
 * ("Refreshing 12/80 · Cancel"), a determinate bar when the count is known,
 * and Cancel only when the action can be cancelled (D20; pressing it hides
 * nothing — the action runs until its command returns, reading "Cancelling…").
 * Not a layer: it never takes focus, never makes the page inert, and the
 * library stays browsable around it. It reads the reporter's `pending`
 * alone. The live region announces the activity (start, Cancelling…), not
 * each tick; the count is plain text beside it. Under reduced motion there is
 * no spinner and the bar does not glide — text and counts only.
 */
const HeaderProgress = ({ pending, message, onCancel, placement, yielded = false, t }: HeaderProgressProps) => {
  const running = pending !== null && showsHeaderProgress(pending);
  const activity = running ? progressActivity(pending, t) : "";
  const shown = running && !yielded;
  const progress = shown ? pending.progress : null;
  const fraction = progress && progress.total > 0 ? Math.min(1, progress.done / progress.total) : null;
  const step = shown && message && message !== activity && !pending.cancelling ? message : null;

  return (
    <div
      data-slot="header-progress"
      data-placement={placement}
      className={cn(
        "flex items-center gap-2 text-sm",
        // In a header row it takes the free room and gives the step line's
        // back first (it truncates); below its own words, count and Cancel it
        // never shrinks — the header wraps its verbs instead of overlapping.
        placement === "header" ? "min-w-min flex-1 basis-0" : "min-w-0",
        shown && placement === "bar" && "shrink-0 border-b border-border px-4 py-2",
        shown && placement === "dialog" && "shrink-0 border-t border-border bg-card px-5 py-2",
      )}
    >
      {/* Always mounted, so the activity is announced when it starts. */}
      <span role="status" className="sr-only">
        {activity}
      </span>
      {shown ? (
        <>
          <LoaderCircle
            aria-hidden="true"
            className="size-3.5 shrink-0 text-muted-foreground motion-safe:animate-spin motion-reduce:hidden"
          />
          <span aria-hidden="true" className="shrink-0 font-medium text-foreground">
            {activity}
          </span>
          {progress ? (
            <span className="shrink-0 text-muted-foreground tabular-nums">
              {t("activity.count", { done: progress.done, total: progress.total })}
            </span>
          ) : null}
          {fraction !== null ? (
            <span
              role="progressbar"
              aria-label={activity}
              aria-valuemin={0}
              aria-valuemax={progress?.total}
              aria-valuenow={progress?.done}
              className="relative h-1 w-16 shrink-0 overflow-hidden rounded-full bg-raised max-shell:hidden"
            >
              <span
                className="absolute inset-0 origin-left bg-primary transition-transform duration-(--motion-fast) ease-out motion-reduce:transition-none"
                style={{ transform: `scaleX(${fraction})` }}
              />
            </span>
          ) : null}
          {step ? (
            // `w-0 flex-1`: the step line takes the room that is left and
            // adds nothing to the region's minimum (it truncates).
            <span className="w-0 min-w-0 flex-1 truncate text-xs text-muted-foreground" title={step}>
              {step}
            </span>
          ) : null}
          {pending.cancellable ? (
            <Button
              size="sm"
              variant="ghost"
              className="shrink-0"
              onClick={onCancel}
              disabled={pending.cancelling}
              focusableWhenDisabled
              aria-label={pending.label ? t("activity.cancelAction", { action: pending.label }) : undefined}
            >
              {pending.cancelling ? t("lifecycle.cancelling") : t("cancel")}
            </Button>
          ) : null}
        </>
      ) : null}
    </div>
  );
};

export default memo(HeaderProgress);
