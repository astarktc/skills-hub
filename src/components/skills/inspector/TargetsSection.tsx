import { useEffect, useState } from "react";
import type { TFunction } from "i18next";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Segmented, SegmentedItem } from "@/components/ui/segmented";
import type { SkillActions } from "../../../hooks/useSkillActions";
import { usePersistedPreference } from "../../../hooks/usePersistedPreference";
import { inspectorTargetsViewPreference } from "../../../lib/preferences";
import { UNDO_WINDOW_MS } from "../../../lib/actionLifecycle";
import {
  stripMarkName,
  targetActions,
  type OfferedTargetAction,
  type StripMark,
  type SyncedStrip,
  type TargetAction,
} from "../../../lib/skillPresentation";
import StripMarkIcon from "../StripMarkIcon";
import type { ManagedSkill } from "../types";
import InspectorSection from "./InspectorSection";


type TargetsSectionProps = {
  skill: ManagedSkill;
  /** The skill's Synced strip — the same fold the row reads. */
  strip: SyncedStrip;
  actions: Pick<SkillActions, "run">;
  blockedReason: string | null;
  t: TFunction;
};

/** A mark's state in words, without its Tool (the row names the Tool). */
const STATE_KEY: Record<StripMark["nameKey"], string> = {
  synced: "inspector.targets.state.synced",
  not_synced: "inspector.targets.state.notSynced",
  pending: "inspector.targets.state.pending",
  failed: "inspector.targets.state.failed",
  failed_first: "inspector.targets.state.failedFirst",
  failed_disable: "inspector.targets.state.failedDisable",
  missing: "inspector.targets.state.missing",
  stale: "inspector.targets.state.stale",
  disabled: "inspector.targets.state.disabled",
  skill_disabled: "inspector.targets.state.skillDisabled",
  broken: "inspector.targets.state.broken",
};

const ACTION_LABEL: Record<TargetAction, { short: string; long: string }> = {
  retry: { short: "inspector.targets.retry", long: "inspector.targets.retryLabel" },
  disable: { short: "inspector.targets.disable", long: "inspector.targets.disableLabel" },
  enable: { short: "inspector.targets.enable", long: "inspector.targets.enableLabel" },
  sync: { short: "inspector.targets.sync", long: "inspector.targets.syncLabel" },
};

/**
 * The inspector's per-Tool section: a vertical list by default (mark ·
 * Tool · sync mode · state in words · inline actions) or, behind a
 * persisted toggle, the click-to-sync grid. Every action is a door verb at
 * pair grain; a Disable shows "Removed from {Tool} · Undo" for five
 * seconds, and Undo is the pair's Enable.
 */
export default function TargetsSection({ skill, strip, actions, blockedReason, t }: TargetsSectionProps) {
  const [view, setView] = usePersistedPreference(inspectorTargetsViewPreference);
  const [removed, setRemoved] = useState<{ skillId: string; tool: string } | null>(null);
  const busy = blockedReason !== null;
  // A verb refused in place states its reason (held: focusable, and its
  // activation says why), never a round trip that fails (BACKLOG #72).
  const refusalReason = (offered: OfferedTargetAction, mark: StripMark) =>
    offered.refusal === "not_detected" ? t("inspector.targets.notDetectedReason", { tool: mark.label }) : null;
  const skillOff = skill.disabled_at !== null;
  const centralMissing = skill.unlocatable === "central_missing";
  // Rows are keyed by Deployment folder; a mark names the folder it reads.
  const byFolder = new Map(skill.targets.map((target) => [target.tool, target]));
  const targetOf = (tool: string) => byFolder.get(strip.marks.find((mark) => mark.tool === tool)?.folder ?? tool);
  // The undo line shows only while the pair is still disabled, for this skill.
  const undoTool =
    removed && removed.skillId === skill.id && targetOf(removed.tool)?.disabled ? removed.tool : null;

  useEffect(() => {
    if (!removed) return;
    const timer = window.setTimeout(() => setRemoved(null), UNDO_WINDOW_MS);
    return () => window.clearTimeout(timer);
  }, [removed]);

  const act = async (action: TargetAction, tool: string) => {
    switch (action) {
      case "retry":
        await actions.run("retry", [skill], { tool });
        return;
      case "sync":
        await actions.run("syncToTargets", [skill], { tool });
        return;
      case "enable":
        await actions.run("enable", [skill], { tool });
        return;
      case "disable":
        await actions.run("disable", [skill], { tool });
        setRemoved({ skillId: skill.id, tool });
    }
  };

  const undo = (tool: string, from: HTMLElement) => {
    // Undo is the pair's Enable, started from the Tool's own control (D21:
    // focus stays on it): in the list that is this button; under the grid,
    // the Tool's tile — the Undo line goes as it runs.
    const own = from.closest("section")?.querySelector<HTMLElement>(`[data-target-action="${CSS.escape(tool)}"]`);
    if (own && own !== from) own.focus();
    setRemoved(null);
    void actions.run("enable", [skill], { tool });
  };

  // The one Undo control, in the list (at the row's first action) and under
  // the grid alike: the pair's Enable, labelled as the undo it is.
  const undoButton = (tool: string, label: string, position?: number, targetAction?: string) => (
    <Button
      key={position}
      data-target-action={targetAction}
      size="sm"
      variant="ghost"
      tone="accent"
      aria-label={t("inspector.targets.undoLabel", { tool: label })}
      disabled={busy}
      focusableWhenDisabled
      aria-description={blockedReason ?? undefined}
      title={blockedReason ?? undefined}
      onClick={(event) => undo(tool, event.currentTarget)}
    >
      {t("inspector.targets.undo")}
    </Button>
  );
  const undoMark = undoTool ? (strip.marks.find((mark) => mark.tool === undoTool) ?? null) : null;

  const modeLabel = (tool: string) => {
    const target = targetOf(tool);
    return target ? t(`inspector.targets.mode.${target.mode}`) : null;
  };

  const viewToggle =
    strip.total > 0 ? (
      <Segmented
        aria-label={t("inspector.targets.viewLabel")}
        value={view}
        onValueChange={(next) => setView(next === "grid" ? "grid" : "list")}
      >
        <SegmentedItem value="list">{t("inspector.targets.list")}</SegmentedItem>
        <SegmentedItem value="grid">{t("inspector.targets.grid")}</SegmentedItem>
      </Segmented>
    ) : null;

  return (
    <InspectorSection title={t("inspector.sections.targets")} actions={viewToggle}>
      {strip.total === 0 ? (
        <p className="m-0 text-sm text-muted-foreground">{t("inspector.targets.none")}</p>
      ) : view === "list" ? (
        <ul className="m-0 flex list-none flex-col p-0">
          {strip.marks.map((mark) => {
            const mode = mark.state === "not_synced" ? null : modeLabel(mark.tool);
            const offered = targetActions(mark, skill);
            const refused = offered.some((entry) => entry.refusal !== null);
            const troubled = mark.state === "failed" || mark.state === "missing" || mark.state === "broken";
            const toolLabel = mark.detected ? mark.label : t("strip.notDetected", { name: mark.label });
            return (
              <li key={mark.tool} className="flex min-h-9 items-center gap-2 border-b border-border py-1 last:border-b-0">
                <StripMarkIcon state={mark.state} />
                <div className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate text-sm text-foreground" title={toolLabel}>
                    {toolLabel}
                    {mode ? <span className="ml-2 text-xs text-faint">{mode}</span> : null}
                  </span>
                  {undoTool === mark.tool ? (
                    <span role="status" className="text-xs text-muted-foreground">
                      {t("inspector.targets.removed", { tool: mark.label })}
                    </span>
                  ) : (
                    <span className={cn("text-xs", troubled ? "text-failed" : "text-muted-foreground")}>
                      {refused
                        ? t("inspector.targets.notDetectedState", { state: t(STATE_KEY[mark.nameKey]) })
                        : t(STATE_KEY[mark.nameKey])}
                    </span>
                  )}
                </div>
                <div className="flex shrink-0 items-center gap-1">
                  {/* Keyed by position: a row's first control stays the same
                      element as it changes role (Disable → Undo → Enable,
                      Retry → Disable), so focus stays on it (D21). */}
                  {(undoTool === mark.tool ? (["undo"] as const) : offered).map((entry, index) => {
                    if (entry === "undo") return undoButton(mark.tool, mark.label, index, mark.tool);
                    const { action } = entry;
                    const reason = blockedReason ?? refusalReason(entry, mark);
                    return (
                      <Button
                        key={index}
                        size="sm"
                        variant={action === "retry" ? "outline" : "ghost"}
                        data-target-action={index === 0 ? mark.tool : undefined}
                        aria-label={t(ACTION_LABEL[action].long, { tool: mark.label })}
                        aria-describedby={action === "retry" ? `retry-hint-${skill.id}` : undefined}
                        disabled={reason !== null}
                        focusableWhenDisabled
                        aria-description={reason ?? undefined}
                        title={reason ?? t(ACTION_LABEL[action].long, { tool: mark.label })}
                        onClick={() => void act(action, mark.tool)}
                      >
                        {t(ACTION_LABEL[action].short)}
                      </Button>
                    );
                  })}
                </div>
              </li>
            );
          })}
        </ul>
      ) : (
        <>
          <p className="m-0 text-xs text-muted-foreground">{t("inspector.targets.gridHint")}</p>
          <ul className="m-0 grid list-none grid-cols-[repeat(auto-fill,minmax(9rem,1fr))] gap-1.5 p-0">
            {strip.marks.map((mark) => {
              const [offered] = targetActions(mark, skill);
              const primary = offered?.action;
              const refusal = offered ? refusalReason(offered, mark) : null;
              const reason = blockedReason ?? refusal;
              const name = stripMarkName(mark, t);
              return (
                <li key={mark.tool} className="flex">
                  <Button
                    size="md"
                    variant="outline"
                    className="w-full justify-start"
                    data-target-action={mark.tool}
                    aria-label={
                      primary ? t("inspector.targets.gridAction", { state: name, action: t(ACTION_LABEL[primary].short) }) : name
                    }
                    title={reason ?? name}
                    disabled={reason !== null || !primary}
                    focusableWhenDisabled={reason !== null}
                    aria-description={reason ?? undefined}
                    onClick={() => primary && void act(primary, mark.tool)}
                  >
                    <StripMarkIcon state={mark.state} />
                    <span className="truncate">{mark.label}</span>
                  </Button>
                </li>
              );
            })}
          </ul>
          {/* The same "Removed from {Tool} · Undo" as the list: the tile keeps
              focus (it is now the pair's Enable); the line lives five seconds. */}
          <div className="flex min-h-7 items-center gap-2">
            <span role="status" className="text-xs text-muted-foreground">
              {undoMark ? t("inspector.targets.removed", { tool: undoMark.label }) : null}
            </span>
            {undoMark ? undoButton(undoMark.tool, undoMark.label) : null}
          </div>
        </>
      )}
      {skillOff ? <p className="m-0 text-xs text-muted-foreground">{t("inspector.targets.skillOff")}</p> : null}
      {strip.marks.some((mark) => mark.state === "failed" || mark.state === "missing" || mark.state === "stale") &&
      !skillOff &&
      !centralMissing ? (
        <p id={`retry-hint-${skill.id}`} className="m-0 text-xs text-faint">
          {t("inspector.targets.retryHint")}
        </p>
      ) : null}
    </InspectorSection>
  );
}
