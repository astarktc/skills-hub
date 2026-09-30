import { memo } from "react";
import { X } from "lucide-react";
import type { TFunction } from "i18next";
import { Button } from "@/components/ui/button";
import type { SkillActions } from "../../hooks/useSkillActions";
import { enableCounts, disableCounts, updateCounts } from "../../lib/bulkCounts";
import type { ManagedSkill } from "./types";

type BulkRailProps = {
  /** The surface's selected skills (surface state's `selection.skills`, resolved). */
  selected: readonly ManagedSkill[];
  actions: Pick<SkillActions, "run">;
  /** Something is running: every verb waits, with the reason (D19). */
  blockedReason: string | null;
  onClear: () => void;
  t: TFunction;
};

const kbd =
  "inline-flex h-5 min-w-5 items-center justify-center rounded-sm border border-border-strong px-1 font-mono text-xs text-foreground";

/**
 * The bulk rail (ticket 13, the signature interaction's selection half): it
 * rises from the foot of the list while anything is selected — 120 ms of
 * opacity and translate on entry (`@starting-style`), none under reduced
 * motion, never gating a press — and acts on the selection through the door
 * alone: every verb is `run(verb, selected)`, and the door asks each wide
 * verb's count-bearing confirmation (Sync to Tools its Tool pick, Assign its
 * project pick, Remove the Delete dialog). The rows' own controls stay the
 * row's. It is drawn at the foot (`order-last`) but comes first in the DOM,
 * so its verbs are one Tab from the filter bar and read before the rows.
 */
const BulkRail = ({ selected, actions, blockedReason, onClear, t }: BulkRailProps) => {
  const { run } = actions;
  const busy = blockedReason !== null;
  // What each verb would change, from the same folds the confirmation reads:
  // a verb that would change nothing says so instead of opening a question.
  const nothingToDisable = disableCounts(selected, null).alreadyDisabled === selected.length;
  const nothingToEnable = enableCounts(selected, null).alreadyEnabled === selected.length;
  const nothingToUpdate = updateCounts(selected).refreshable === 0;
  const reason = (idle: string | null) => (busy ? (blockedReason ?? undefined) : (idle ?? undefined));

  return (
    <div
      role="group"
      aria-label={t("bulk.rail")}
      data-bulk-rail=""
      className="order-last flex shrink-0 flex-wrap items-center gap-x-3 gap-y-2 border-t border-border-strong bg-card px-4 py-2 transition-[opacity,translate] duration-(--motion-fast) ease-out starting:translate-y-2 starting:opacity-0 motion-reduce:transition-none"
    >
      <p role="status" className="m-0 flex min-w-0 items-center gap-2 text-sm">
        <span className="font-semibold text-foreground tabular-nums">{t("bulk.selected", { count: selected.length })}</span>
        <span aria-hidden="true" className="flex items-center gap-1.5 text-xs text-muted-foreground max-shell:hidden">
          <kbd className={kbd}>x</kbd>
          {t("bulk.hintSelect")}
          <kbd className={kbd}>Esc</kbd>
          {t("bulk.hintClear")}
        </span>
      </p>
      <div className="ml-auto flex flex-wrap items-center gap-1.5">
        <Button size="sm" focusableWhenDisabled onClick={() => void run("syncToTargets", selected)} disabled={busy} title={reason(null)} aria-description={reason(null)}>
          {t("bulk.sync")}
        </Button>
        <Button size="sm" focusableWhenDisabled onClick={() => void run("assignToProject", selected)} disabled={busy} title={reason(null)} aria-description={reason(null)}>
          {t("bulk.assignAction")}
        </Button>
        <Button
          size="sm"
          focusableWhenDisabled
          onClick={() => void run("disable", selected)}
          disabled={busy || nothingToDisable}
          title={reason(nothingToDisable ? t("bulk.nothingToDisable") : null)} aria-description={reason(nothingToDisable ? t("bulk.nothingToDisable") : null)}
        >
          {t("bulk.disable")}
        </Button>
        <Button
          size="sm"
          focusableWhenDisabled
          onClick={() => void run("enable", selected)}
          disabled={busy || nothingToEnable}
          title={reason(nothingToEnable ? t("bulk.nothingToEnable") : null)} aria-description={reason(nothingToEnable ? t("bulk.nothingToEnable") : null)}
        >
          {t("bulk.enable")}
        </Button>
        <Button
          size="sm"
          focusableWhenDisabled
          onClick={() => void run("update", selected)}
          disabled={busy || nothingToUpdate}
          title={reason(nothingToUpdate ? t("bulk.nothingToUpdate") : null)} aria-description={reason(nothingToUpdate ? t("bulk.nothingToUpdate") : null)}
        >
          {t("bulk.update")}
        </Button>
        <Button size="sm" focusableWhenDisabled tone="danger" onClick={() => void run("delete", selected)} disabled={busy} title={reason(null)} aria-description={reason(null)}>
          {t("bulk.remove")}
        </Button>
        <Button variant="ghost" size="icon-sm" onClick={onClear} aria-label={t("bulk.clear")} title={t("bulk.clear")}>
          <X />
        </Button>
      </div>
    </div>
  );
};

export default memo(BulkRail);
