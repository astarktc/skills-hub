import { memo } from "react";
import type { TFunction } from "i18next";
import { cn } from "@/lib/utils";
import { stripMarkName, type SyncedStrip as SyncedStripFold } from "../../lib/skillPresentation";
import StripMarkIcon from "./StripMarkIcon";

type SyncedStripProps = {
  strip: SyncedStripFold;
  /** The import cascade's token while this row's strip lights (ticket 21); `null` otherwise. */
  landing?: number | null;
  t: TFunction;
};

/**
 * The Synced strip (D22): one read-only mark per Tool, then N/M. Each mark
 * is an image with its own name (the Tool label and the state in words);
 * clicking the row, not a mark, opens the skill.
 */
const SyncedStrip = ({ strip, landing = null, t }: SyncedStripProps) => {
  if (strip.total === 0) {
    return <span className="text-xs text-faint">{t("library.row.noTargets")}</span>;
  }
  return (
    <span className="flex min-w-0 items-center gap-2">
      {/* 12 px marks at a 20 px pitch (8 px gap): the group header's harness
          initials sit over them, two letters each (ticket 31). */}
      <span role="group" aria-label={t("library.row.strip")} className="flex items-center gap-2">
        {strip.marks.map((mark, index) => {
          const name = stripMarkName(mark, t);
          return (
            <span
              key={mark.tool}
              role="img"
              aria-label={name}
              title={name}
              data-mark={mark.state}
              className="inline-flex size-3 shrink-0"
            >
              <StripMarkIcon
                state={mark.state}
                quiet={strip.complete}
                landing={landing !== null ? { token: landing, index } : undefined}
              />
            </span>
          );
        })}
      </span>
      {/* `relative` contains the absolute sr-only count: without it the count's
          containing block is outside the list's scroller and it stretches the
          document (a second scrollbar). */}
      <span
        className={cn(
          "relative shrink-0 text-xs tabular-nums",
          strip.complete ? "text-faint" : "text-muted-foreground",
        )}
      >
        <span aria-hidden="true">
          {strip.synced}/{strip.total}
        </span>
        <span className="sr-only">
          {t("library.row.deployedCount", { deployed: strip.synced, total: strip.total })}
        </span>
      </span>
    </span>
  );
};

export default memo(SyncedStrip);
