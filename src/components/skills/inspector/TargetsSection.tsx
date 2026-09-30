import { useEffect, useState } from "react";
import type { TFunction } from "i18next";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Segmented, SegmentedItem } from "@/components/ui/segmented";
import type { SkillActions } from "../../../hooks/useSkillActions";
import { usePersistedPreference } from "../../../hooks/usePersistedPreference";
import { inspectorTargetsViewPreference } from "../../../lib/preferences";
import { stripMarkName, type StripMark, type SyncedStrip } from "../../../lib/skillPresentation";
import StripMarkIcon from "../StripMarkIcon";
import type { ManagedSkill } from "../types";
import InspectorSection from "./InspectorSection";

/** How long "Removed from {Tool} · Undo" stays after a pair-grain Disable. */
export const UNDO_WINDOW_MS = 5000;

type TargetAction = "retry" | "disable" | "enable" | "sync";

type TargetsSectionProps = {
  skill: ManagedSkill;
  /** The skill's Synced strip — the same fold the row reads. */
  strip: SyncedStrip;
  actions: Pick<SkillActions, "run">;
  blockedReason: string | null;
  t: TFunction;
};

/**
 * What a Tool's row offers, by its mark (D22): a failed / missing / stale
 * copy is retried from the central copy (a folder in the way raises the one
 * overwrite ask) and can be disabled; a live copy disabled; a disabled pair
 * enabled; a Tool without a copy synced. A skill off everywhere, or whose
 * central copy is gone, offers nothing here — Health carries its repair.
 */
function targetActions(mark: StripMark, skillOff: boolean, centralMissing: boolean): TargetAction[] {
  if (skillOff || centralMissing) return [];
  switch (mark.state) {
    case "failed":
    case "missing":
    case "stale":
      return ["retry", "disable"];
    case "synced":
      return ["disable"];
    case "disabled":
      return ["enable"];
    case "not_synced":
      return ["sync"];
    case "broken":
      return [];
  }
}

/** A mark's state in words, without its Tool (the row names the Tool). */
const STATE_KEY: Record<StripMark["nameKey"], string> = {
  synced: "inspector.targets.state.synced",
  not_synced: "inspector.targets.state.notSynced",
  pending: "inspector.targets.state.pending",
  failed: "inspector.targets.state.failed",
  failed_first: "inspector.targets.state.failedFirst",
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
  const skillOff = skill.disabled_at !== null;
  const centralMissing = skill.unlocatable === "central_missing";
  const byTool = new Map(skill.targets.map((target) => [target.tool, target]));
  // The undo line shows only while the pair is still disabled, for this skill.
  const undoTool =
    removed && removed.skillId === skill.id && byTool.get(removed.tool)?.status === "disabled" ? removed.tool : null;

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

  const undo = (tool: string) => {
    setRemoved(null);
    void actions.run("enable", [skill], { tool });
  };

  const modeLabel = (tool: string) => {
    const target = byTool.get(tool);
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
            const offered = targetActions(mark, skillOff, centralMissing);
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
                      {t(STATE_KEY[mark.nameKey])}
                    </span>
                  )}
                </div>
                <div className="flex shrink-0 items-center gap-1">
                  {/* Keyed by position: a row's first control stays the same
                      element as it changes role (Disable → Undo → Enable,
                      Retry → Disable), so focus stays on it (D21). */}
                  {(undoTool === mark.tool ? (["undo"] as const) : offered).map((action, index) =>
                    action === "undo" ? (
                      <Button
                        key={index}
                        size="sm"
                        variant="ghost"
                        tone="accent"
                        data-target-action={mark.tool}
                        aria-label={t("inspector.targets.undoLabel", { tool: mark.label })}
                        disabled={busy}
                        title={blockedReason ?? undefined}
                        onClick={() => undo(mark.tool)}
                      >
                        {t("inspector.targets.undo")}
                      </Button>
                    ) : (
                      <Button
                        key={index}
                        size="sm"
                        variant={action === "retry" ? "outline" : "ghost"}
                        data-target-action={index === 0 ? mark.tool : undefined}
                        aria-label={t(ACTION_LABEL[action].long, { tool: mark.label })}
                        aria-describedby={action === "retry" ? `retry-hint-${skill.id}` : undefined}
                        disabled={busy}
                        title={blockedReason ?? t(ACTION_LABEL[action].long, { tool: mark.label })}
                        onClick={() => void act(action, mark.tool)}
                      >
                        {t(ACTION_LABEL[action].short)}
                      </Button>
                    ),
                  )}
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
              const [primary] = targetActions(mark, skillOff, centralMissing);
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
                    title={blockedReason ?? name}
                    disabled={busy || !primary}
                    onClick={() => primary && void act(primary, mark.tool)}
                  >
                    <StripMarkIcon state={mark.state} />
                    <span className="truncate">{mark.label}</span>
                  </Button>
                </li>
              );
            })}
          </ul>
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
