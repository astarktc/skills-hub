import { memo, type MouseEvent } from "react";
import { AlertTriangle, CircleX, Ellipsis, Globe, Pencil } from "lucide-react";
import type { TFunction } from "i18next";
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Switch } from "@/components/ui/switch";
import type { SkillActions } from "../../hooks/useSkillActions";
import { describeCommandError } from "../../commandError";
import { isPendingSkill, type PendingAction } from "../../lib/actionLifecycle";
import {
  HEALTH_REPAIR_LABEL_KEY,
  HEALTH_ROW_LABEL_KEY,
  INVOCATION_LABEL_KEY,
  UNLOCATABLE_TOOLTIP_KEY,
  deployAction,
  skillHealth,
  type HealthRepair,
  type HealthRow,
  type SyncedStrip as SyncedStripFold,
} from "../../lib/skillPresentation";
import SyncedStrip from "./SyncedStrip";
import type { ManagedSkill } from "./types";

/**
 * The list's column template (36 px rows, DESIGN.md § My Skills row):
 * checkbox · name + exceptions · description · Synced strip + N/M · scope ·
 * switch · overflow. The strip column is sized from `--strip-marks` (set by
 * the list: one 16 px step per Tool). Below a 48rem list the description
 * column goes and the name takes its room.
 */
const LIST_COLUMNS = cn(
  "grid-cols-[2rem_minmax(0,1.5fr)_minmax(0,1fr)_calc(var(--strip-marks)_*_1rem_+_2.5rem)_1.5rem_2.5rem_2rem]",
  "@max-3xl:grid-cols-[2rem_minmax(0,1fr)_calc(var(--strip-marks)_*_1rem_+_2.5rem)_1.5rem_2.5rem_2rem]",
);

/** Rows the strip and the switch already say — never repeated as a mark. */
const SILENT_ROWS = new Set<HealthRow["kind"]>(["disabled", "targets_disabled"]);

/** The switch is the Enable; every other repair is reachable from the overflow. */
const MENU_REPAIRS = new Set<HealthRepair>([
  "update",
  "restore",
  "repoint",
  "detach",
  "editInvocation",
  "dismiss",
  "retryTargets",
  "deploy",
  "disable",
]);

export type SkillRowLayout = "list" | "card";

type SkillRowProps = {
  skill: ManagedSkill;
  /** The skill's Synced strip, folded by the list. */
  strip: SyncedStripFold;
  layout: SkillRowLayout;
  /** A muted inline source label (Flat, and the "Individual sources" group). */
  sourceLabel: string | null;
  selected: boolean;
  onToggleSelect: (skillId: string) => void;
  /** The action running now; this row's own shows inline (`aria-busy`). */
  pending: PendingAction | null;
  /** Why every verb is disabled while something runs, in words. */
  blockedReason: string | null;
  actions: Pick<SkillActions, "run" | "repair">;
  /** Focus the skill (hash) and open it. */
  onOpen: (skill: ManagedSkill) => void;
  t: TFunction;
};

/** Clicks on these belong to their control, not to the row. */
const INTERACTIVE = "button, a, input, label, [role='checkbox'], [role='switch'], [role='menu'], [role='menuitem']";

/**
 * One skill: checkbox · name (+ Edit pencil) · exception marks · description
 * · Synced strip + N/M · scope · Disable switch · overflow. Clicking the row
 * (or Enter on its name) focuses the skill; the strip is read-only (D22).
 */
const SkillRow = ({
  skill,
  strip,
  layout,
  sourceLabel,
  selected,
  onToggleSelect,
  pending,
  blockedReason,
  actions,
  onOpen,
  t,
}: SkillRowProps) => {
  const { run, repair } = actions;
  const busy = pending !== null;
  const busyHere = isPendingSkill(pending, skill.id);
  const blockedTitle = busy ? (blockedReason ?? undefined) : undefined;
  const health = skillHealth(skill, { t });
  const marks = health.rows.filter((row) => !SILENT_ROWS.has(row.kind));
  const live = deployAction(skill).kind === "deployed";
  const enabled = skill.disabled_at === null;

  const handleRowClick = (event: MouseEvent<HTMLElement>) => {
    if ((event.target as HTMLElement).closest(INTERACTIVE)) return;
    onOpen(skill);
  };

  const rowLabel = (row: HealthRow) => t(HEALTH_ROW_LABEL_KEY[row.kind], { count: row.count });
  const markTitle = (row: HealthRow) =>
    row.error
      ? (describeCommandError(row.error, t) ?? undefined)
      : row.kind === "central_missing" || row.kind === "source_missing"
        ? t(UNLOCATABLE_TOOLTIP_KEY[row.kind])
        : undefined;

  // The Fix group: every repair the fold offers but the switch (Enable).
  // Remove stays the menu's last item; a Delete that did not finish is
  // offered as its own repair ("Finish removing").
  const fixes = marks.flatMap((row) =>
    row.repairs
      .filter((key) => MENU_REPAIRS.has(key) || (key === "delete" && row.kind === "delete_incomplete"))
      .map((key) => ({ row, key })),
  );
  const fixed = new Set(fixes.map((fix) => fix.key));

  const name = (
    <button
      type="button"
      data-skill-row={skill.id}
      onClick={() => onOpen(skill)}
      title={skill.name}
      className="min-w-0 truncate rounded-sm text-left text-sm font-medium text-foreground hover:underline"
    >
      {skill.name}
    </button>
  );

  const pencil = skill.invocation_override ? (
    <Button
      variant="ghost"
      size="icon-sm"
      aria-label={t("library.row.edited", {
        mode: t(INVOCATION_LABEL_KEY[skill.invocation_override.mode]),
      })}
      title={t("library.row.edited", {
        mode: t(INVOCATION_LABEL_KEY[skill.invocation_override.mode]),
      })}
      aria-haspopup="dialog"
      disabled={busy || skill.unlocatable === "central_missing"}
      onClick={() => void run("editInvocation", [skill])}
    >
      <Pencil />
    </Button>
  ) : null;

  const exceptionMarks =
    marks.length > 0 ? (
      <span
        data-skill-health={skill.id}
        tabIndex={-1}
        className="flex shrink-0 items-center gap-1 rounded-sm"
      >
        {marks.map((row) => (
          <Badge
            key={row.kind}
            tone={row.severity === "failed" ? "failed" : row.severity === "stale" ? "stale" : "neutral"}
            title={markTitle(row)}
          >
            {row.severity === "failed" ? <CircleX aria-hidden="true" /> : null}
            {row.severity === "stale" ? <AlertTriangle aria-hidden="true" /> : null}
            {rowLabel(row)}
          </Badge>
        ))}
      </span>
    ) : null;

  const description =
    sourceLabel || skill.description ? (
      <span className="min-w-0 truncate text-sm text-muted-foreground" title={skill.description ?? undefined}>
        {sourceLabel ? <span className="mr-2 font-mono text-xs text-faint">{sourceLabel}</span> : null}
        {skill.description}
      </span>
    ) : (
      <span />
    );

  const scope = live ? (
    <span role="img" aria-label={t("library.row.scopeGlobal")} title={t("library.row.scopeGlobal")} className="inline-flex text-faint">
      <Globe className="size-3.5" aria-hidden="true" />
    </span>
  ) : (
    <span />
  );

  const toggle = (
    <Switch
      size="sm"
      checked={enabled}
      onCheckedChange={(on) => void run(on ? "enable" : "disable", [skill])}
      disabled={busy}
      aria-label={t("library.row.enabled", { name: skill.name })}
      title={blockedTitle}
    />
  );

  const menu = (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={t("library.row.more", { name: skill.name })}
            title={blockedTitle ?? t("library.row.more", { name: skill.name })}
            disabled={busy}
          />
        }
      >
        <Ellipsis />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-56">
        {fixes.length > 0 ? (
          <>
            <DropdownMenuGroup>
              <DropdownMenuLabel>{t("library.row.fix")}</DropdownMenuLabel>
              {fixes.map(({ row, key }) => (
                <DropdownMenuItem
                  key={`${row.kind}:${key}`}
                  onClick={() => void repair(skill, row, key)}
                >
                  {t("library.row.repairFor", {
                    repair: t(HEALTH_REPAIR_LABEL_KEY[key]),
                    problem: rowLabel(row),
                  })}
                </DropdownMenuItem>
              ))}
            </DropdownMenuGroup>
            <DropdownMenuSeparator />
          </>
        ) : null}
        {skill.refreshable && !skill.unlocatable && !fixed.has("update") ? (
          <DropdownMenuItem onClick={() => void run("update", [skill])}>{t("update")}</DropdownMenuItem>
        ) : null}
        {!fixed.has("repoint") ? (
          <DropdownMenuItem onClick={() => void run("repoint", [skill])}>
            {t("changeSource.action")}
          </DropdownMenuItem>
        ) : null}
        {skill.detachable && !fixed.has("detach") ? (
          <DropdownMenuItem onClick={() => void run("detach", [skill])}>
            {t("unlocatable.detach")}
          </DropdownMenuItem>
        ) : null}
        {skill.unlocatable !== "central_missing" && !fixed.has("editInvocation") ? (
          <DropdownMenuItem onClick={() => void run("editInvocation", [skill])}>
            {t("library.row.editInvocation")}
          </DropdownMenuItem>
        ) : null}
        <DropdownMenuSeparator />
        {!fixed.has("delete") ? (
          <DropdownMenuItem variant="destructive" onClick={() => void run("delete", [skill])}>
            {t("remove")}
          </DropdownMenuItem>
        ) : null}
      </DropdownMenuContent>
    </DropdownMenu>
  );

  const checkbox = (
    <Checkbox
      checked={selected}
      onCheckedChange={() => onToggleSelect(skill.id)}
      aria-label={t("library.row.select", { name: skill.name })}
    />
  );

  if (layout === "card") {
    return (
      <li
        data-selected={selected || undefined}
        aria-busy={busyHere || undefined}
        onClick={handleRowClick}
        className={cn(
          "flex min-w-0 flex-col gap-2 rounded-lg border border-border bg-card p-3 hover:border-border-strong data-selected:bg-raised",
          busyHere && "opacity-70",
        )}
      >
        <div className="flex min-w-0 items-center gap-2">
          {checkbox}
          {name}
          {pencil}
          <span className="ml-auto flex shrink-0 items-center gap-1">
            {toggle}
            {menu}
          </span>
        </div>
        {exceptionMarks ? <div className="flex flex-wrap items-center gap-1">{exceptionMarks}</div> : null}
        {skill.description ? (
          <p className="m-0 line-clamp-2 text-sm text-muted-foreground">{skill.description}</p>
        ) : null}
        <div className="mt-auto flex min-w-0 items-center gap-2">
          <SyncedStrip strip={strip} t={t} />
          <span className="ml-auto">{scope}</span>
        </div>
        {sourceLabel ? <span className="truncate font-mono text-xs text-faint">{sourceLabel}</span> : null}
      </li>
    );
  }

  return (
    <li
      data-selected={selected || undefined}
      aria-busy={busyHere || undefined}
      onClick={handleRowClick}
      className={cn(
        "grid h-row items-center gap-x-3 border-b border-border px-3 hover:bg-card data-selected:bg-raised",
        LIST_COLUMNS,
        busyHere && "opacity-70",
      )}
    >
      <span className="flex items-center justify-center">{checkbox}</span>
      <span className="flex min-w-0 items-center gap-2">
        {name}
        {pencil}
        {exceptionMarks}
      </span>
      <span className="flex min-w-0 @max-3xl:hidden">{description}</span>
      <SyncedStrip strip={strip} t={t} />
      <span className="flex items-center justify-center">{scope}</span>
      <span className="flex items-center justify-center">{toggle}</span>
      <span className="flex items-center justify-center">{menu}</span>
    </li>
  );
};

export default memo(SkillRow);
