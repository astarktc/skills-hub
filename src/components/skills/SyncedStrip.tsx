import { memo } from "react";
import type { TFunction } from "i18next";
import { cn } from "@/lib/utils";
import {
  STRIP_MARK_NAME_KEY,
  type StripMark,
  type StripMarkState,
  type SyncedStrip as SyncedStripFold,
} from "../../lib/skillPresentation";

/**
 * The glyph of each mark state, drawn on a 12 × 12 grid in `currentColor`.
 * Shape carries the state (DESIGN.md § Synced strip): filled = synced,
 * hollow = not synced, half-filled = stale, × = failed / missing / broken,
 * slashed = disabled — distinguishable in grayscale; colour reinforces.
 */
function MarkGlyph({ state }: { state: StripMarkState }) {
  switch (state) {
    case "synced":
      return <circle cx="6" cy="6" r="4.5" fill="currentColor" />;
    case "not_synced":
      return <circle cx="6" cy="6" r="4" fill="none" stroke="currentColor" strokeWidth="1.5" />;
    case "stale":
      return (
        <>
          <circle cx="6" cy="6" r="4" fill="none" stroke="currentColor" strokeWidth="1.5" />
          <path d="M6 2a4 4 0 0 0 0 8z" fill="currentColor" />
        </>
      );
    case "disabled":
      return (
        <>
          <circle cx="6" cy="6" r="4" fill="none" stroke="currentColor" strokeWidth="1.5" />
          <path d="M3.2 8.8 8.8 3.2" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
        </>
      );
    case "failed":
    case "missing":
    case "broken":
      return (
        <path
          d="M2.5 2.5l7 7m0-7-7 7"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
        />
      );
  }
}

/** Colour reinforces the shape; a complete strip is the quiet one. */
function markTone(state: StripMarkState, complete: boolean): string {
  switch (state) {
    case "synced":
      return complete ? "text-input" : "text-muted-foreground";
    case "not_synced":
      return "text-input";
    case "stale":
      return "text-stale";
    case "disabled":
      return "text-faint";
    case "failed":
    case "missing":
    case "broken":
      return "text-failed";
  }
}

/** A mark's accessible name: "Amp: failed — the last sync did not complete". */
function stripMarkName(mark: StripMark, t: TFunction): string {
  const tool = mark.detected ? mark.label : t("strip.notDetected", { name: mark.label });
  return t(STRIP_MARK_NAME_KEY[mark.nameKey], { tool });
}

type SyncedStripProps = {
  strip: SyncedStripFold;
  t: TFunction;
};

/**
 * The Synced strip (D22): one read-only mark per Tool, then N/M. Each mark
 * is an image with its own name (the Tool label and the state in words);
 * clicking the row, not a mark, opens the skill.
 */
const SyncedStrip = ({ strip, t }: SyncedStripProps) => {
  if (strip.total === 0) {
    return <span className="text-xs text-faint">{t("library.row.noTargets")}</span>;
  }
  return (
    <span className="flex min-w-0 items-center gap-2">
      <span role="group" aria-label={t("library.row.strip")} className="flex items-center gap-1">
        {strip.marks.map((mark) => {
          const name = stripMarkName(mark, t);
          return (
            <span
              key={mark.tool}
              role="img"
              aria-label={name}
              title={name}
              data-mark={mark.state}
              className={cn("inline-flex size-3 shrink-0", markTone(mark.state, strip.complete))}
            >
              <svg viewBox="0 0 12 12" className="size-3" aria-hidden="true">
                <MarkGlyph state={mark.state} />
              </svg>
            </span>
          );
        })}
      </span>
      <span
        className={cn(
          "shrink-0 text-xs tabular-nums",
          strip.complete ? "text-faint" : "text-muted-foreground",
        )}
      >
        <span aria-hidden="true">
          {strip.synced}/{strip.total}
        </span>
        <span className="sr-only">
          {t("library.row.syncedCount", { synced: strip.synced, total: strip.total })}
        </span>
      </span>
    </span>
  );
};

export default memo(SyncedStrip);
