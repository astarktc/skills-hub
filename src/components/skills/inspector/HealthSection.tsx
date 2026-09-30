import { AlertTriangle, CircleCheck, CircleMinus, CircleX } from "lucide-react";
import type { TFunction } from "i18next";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { describeCommandError } from "../../../commandError";
import type { SkillActions } from "../../../hooks/useSkillActions";
import {
  HEALTH_REPAIR_LABEL_KEY,
  HEALTH_ROW_LABEL_KEY,
  UNLOCATABLE_TOOLTIP_KEY,
  formatRelativeTime,
  type HealthRepair,
  type HealthRow,
  type SkillHealthFold,
} from "../../../lib/skillPresentation";
import type { ManagedSkill } from "../types";
import InspectorSection from "./InspectorSection";

type HealthSectionProps = {
  skill: ManagedSkill;
  health: SkillHealthFold;
  labelOf: (tool: string) => string;
  actions: Pick<SkillActions, "repair">;
  blockedReason: string | null;
  t: TFunction;
};

/** A repair that removes or destroys is worded and toned as such. */
const DANGER_REPAIRS = new Set<HealthRepair>(["delete"]);

/**
 * The inspector's first question — is anything wrong? — as the health
 * fold's rows in its precedence, each with what happened and its repairs
 * inline (every repair is a door verb through `actions.repair`). A healthy
 * skill says so in one quiet line. The section is Restore's focus target
 * (D21: `data-inspector-health`).
 */
export default function HealthSection({ skill, health, labelOf, actions, blockedReason, t }: HealthSectionProps) {
  const busy = blockedReason !== null;

  const detail = (row: HealthRow): string | null => {
    const parts: string[] = [];
    if (row.error) {
      const described = describeCommandError(row.error, t);
      if (described) parts.push(described);
    } else if (row.kind === "central_missing" || row.kind === "source_missing") {
      parts.push(t(UNLOCATABLE_TOOLTIP_KEY[row.kind]));
    }
    if (row.tools.length > 0) parts.push(t("inspector.health.onTools", { tools: row.tools.map(labelOf).join(", ") }));
    if (row.at !== null) parts.push(t("inspector.health.settled", { time: formatRelativeTime(row.at, t) }));
    return parts.length > 0 ? parts.join(" · ") : null;
  };

  return (
    <InspectorSection
      title={t("inspector.sections.health")}
      sectionProps={{ "data-inspector-health": skill.id, tabIndex: -1 }}
    >
      {health.rows.length === 0 ? (
        <p className="m-0 flex items-center gap-2 text-sm text-muted-foreground">
          <CircleCheck className="size-4 shrink-0 text-faint" aria-hidden="true" />
          {t("inspector.health.healthy")}
        </p>
      ) : (
        <ul className="m-0 flex list-none flex-col gap-3 p-0">
          {health.rows.map((row, rowIndex) => {
            const Icon = row.severity === "failed" ? CircleX : row.severity === "stale" ? AlertTriangle : CircleMinus;
            const text = detail(row);
            return (
              <li key={row.kind} className="flex flex-col gap-2">
                <div className="flex items-start gap-2">
                  <Icon
                    aria-hidden="true"
                    className={cn(
                      "mt-0.5 size-4 shrink-0",
                      row.severity === "failed" ? "text-failed" : row.severity === "stale" ? "text-stale" : "text-faint",
                    )}
                  />
                  <div className="flex min-w-0 flex-col gap-0.5">
                    <p className="m-0 text-sm font-medium text-foreground">
                      {t(HEALTH_ROW_LABEL_KEY[row.kind], { count: row.count })}
                    </p>
                    {text ? <p className="m-0 text-xs break-words text-muted-foreground">{text}</p> : null}
                  </div>
                </div>
                {row.repairs.length > 0 ? (
                  <div className="flex flex-wrap gap-2 pl-6">
                    {row.repairs.map((key, index) => (
                      <Button
                        key={key}
                        size="sm"
                        variant="outline"
                        tone={DANGER_REPAIRS.has(key) ? "danger" : rowIndex === 0 && index === 0 ? "accent" : "neutral"}
                        disabled={busy}
                        focusableWhenDisabled
                        aria-description={blockedReason ?? undefined}
                        title={blockedReason ?? undefined}
                        aria-haspopup={key === "delete" || key === "repoint" || key === "detach" || key === "editInvocation" ? "dialog" : undefined}
                        onClick={() => void actions.repair(skill, row, key)}
                      >
                        {key === "delete" && row.kind !== "delete_incomplete"
                          ? t("inspector.remove")
                          : t(HEALTH_REPAIR_LABEL_KEY[key])}
                      </Button>
                    ))}
                  </div>
                ) : null}
              </li>
            );
          })}
        </ul>
      )}
    </InspectorSection>
  );
}
