import { memo, type KeyboardEvent, type ReactNode } from "react";
import { Box, ChevronDown, ChevronRight, Folder, GitBranch, Layers } from "lucide-react";
import type { TFunction } from "i18next";
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import type { ManagedSkill } from "./types";
import type { SkillActions } from "../../hooks/useSkillActions";
import {
  INDIVIDUAL_GROUP_KEY,
  LOCAL_GROUP_KEY,
  MANAGED_GROUP_KEY,
  groupState,
  identityHue,
  identityInitials,
  type RepoGroup,
} from "../../lib/skillPresentation";
import { LIST_COLUMNS } from "./listColumns";

/**
 * The header's two layouts. `list` shares the rows' column template
 * (`LIST_COLUMNS`), so its strip column — the `legend` — sits over the
 * strip's marks; `grid` has no columns to share: checkbox · identity ·
 * Deploy all, and no legend (each card's marks carry their names).
 */
export type SourceGroupHeaderLayout = "list" | "grid";

type SourceGroupHeaderProps = {
  group: RepoGroup<ManagedSkill>;
  layout: SourceGroupHeaderLayout;
  /** The `h2`'s id: the group's `<section>` is labelled by it. */
  headingId: string;
  /** The rows' (or cards') container, which the chevron controls while expanded. */
  rowsId: string;
  collapsed: boolean;
  /** The chevron, and ←/→ on any control in the header (D42). */
  onCollapsedChange: (collapsed: boolean) => void;
  /** How many of the group's skills are in the surface's selection. */
  selectedCount: number;
  /** The checkbox: every skill of the source in, or out. */
  onSelectGroup: (skillIds: readonly string[], on: boolean) => void;
  /** The list's strip column (the harness initials, D42); the grid passes none. */
  legend?: ReactNode;
  /** Something runs (D19): Deploy all is refused with `blockedReason`. */
  busy: boolean;
  blockedReason: string | null;
  actions: Pick<SkillActions, "run">;
  t: TFunction;
};

/**
 * The source's chip (ticket 30's pick A, header only): a GitHub owner's
 * identity hue and initials; otherwise the neutral chip with the source's
 * icon. `aria-hidden` — the source name beside it is the label.
 */
const groupChip = (group: RepoGroup<ManagedSkill>) =>
  group.owner ? (
    <Badge tone="identity" hue={identityHue(group.owner)} aria-hidden="true">
      {identityInitials(group.owner)}
    </Badge>
  ) : (
    <Badge variant="solid" tone="neutral" size="chip" aria-hidden="true">
      {group.key === LOCAL_GROUP_KEY ? (
        <Folder />
      ) : group.key === MANAGED_GROUP_KEY ? (
        // The imported source line's icon (`skillSourceLine`'s "imported").
        <Box />
      ) : group.key === INDIVIDUAL_GROUP_KEY ? (
        <Layers />
      ) : (
        <GitBranch />
      )}
    </Badge>
  );

/**
 * One source group's header (D38, D42), sticky inside the scroll area on
 * `card`: the group's checkbox · chevron · identity chip · name (links to
 * the repo) · count · state summary · [legend] · Deploy all. The list and
 * the grid both render it; only the layout differs.
 */
const SourceGroupHeader = ({
  group,
  layout,
  headingId,
  rowsId,
  collapsed,
  onCollapsedChange,
  selectedCount,
  onSelectGroup,
  legend,
  busy,
  blockedReason,
  actions,
  t,
}: SourceGroupHeaderProps) => {
  const state = groupState(group.skills);
  const chevronLabel = t(collapsed ? "library.group.expand" : "library.group.collapse", { source: group.label });
  const allSelected = selectedCount === group.skills.length;
  const parts = (
    [
      ["failed", state.failed, "text-failed"],
      ["stale", state.stale, "text-stale"],
      ["notDeployed", state.notDeployed, "text-muted-foreground"],
      ["disabled", state.disabled, "text-faint"],
    ] as const
  ).filter(([, count]) => count > 0);

  /** ←/→ on a focused header collapse / expand its group (D42). */
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.altKey || event.metaKey || event.ctrlKey || event.shiftKey) return;
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    onCollapsedChange(event.key === "ArrowLeft");
  };

  return (
    <div
      data-group-header={group.key}
      onKeyDown={onKeyDown}
      className={cn(
        "sticky top-0 z-10 grid h-10 items-center gap-x-3 bg-card px-3",
        !collapsed && "border-b border-border",
        layout === "list" ? LIST_COLUMNS : "grid-cols-[2rem_minmax(0,1fr)_auto]",
      )}
    >
      <span className="flex justify-center">
        <Checkbox
          checked={allSelected}
          indeterminate={selectedCount > 0 && !allSelected}
          onCheckedChange={() => onSelectGroup(group.skills.map((skill) => skill.id), !allSelected)}
          aria-label={t("bulk.selectGroup", { count: group.skills.length, source: group.label })}
        />
      </span>
      <span className={cn("flex min-w-0 items-center gap-2", layout === "list" && "col-span-2 @max-3xl:col-span-1")}>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-expanded={!collapsed}
          aria-controls={collapsed ? undefined : rowsId}
          aria-label={chevronLabel}
          title={chevronLabel}
          onClick={() => onCollapsedChange(!collapsed)}
        >
          {collapsed ? <ChevronRight /> : <ChevronDown />}
        </Button>
        {groupChip(group)}
        {/* Truncation lives on the inner text, never on an ancestor of
            the link: an overflow-clipping heading would cut its focus
            ring (2 px, offset 2 px) down to corner fragments. */}
        <h2 id={headingId} className="m-0 flex min-w-0 text-sm font-semibold text-foreground">
          {group.href ? (
            <a
              href={group.href}
              target="_blank"
              rel="noopener noreferrer"
              className="flex h-6 min-w-0 items-center rounded-sm hover:underline"
            >
              <span className="truncate" title={group.label}>{group.label}</span>
            </a>
          ) : (
            <span className="flex h-6 min-w-0 items-center">
              <span className="truncate" title={group.label}>{group.label}</span>
            </span>
          )}
        </h2>
        {/* Below a 48rem list the count yields to the name and the state summary. */}
        <span className="shrink-0 text-xs text-faint tabular-nums @max-3xl:hidden">
          {t("library.group.count", { count: group.skills.length })}
        </span>
        {parts.length > 0 ? (
          // `shrink-8`: the summary gives way long before the source name.
          <span
            className="flex min-w-0 shrink-8 items-center gap-2 truncate text-xs"
            title={parts.map(([key, count]) => t(`library.group.${key}`, { count })).join(" · ")}
          >
            {parts.map(([key, count, tone]) => (
              <span key={key} className={tone}>
                {t(`library.group.${key}`, { count })}
              </span>
            ))}
          </span>
        ) : null}
      </span>
      {layout === "list" ? (legend ?? <span />) : null}
      <span className={cn("flex justify-end", layout === "list" && "col-span-3")}>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => void actions.run("deployGroup", group.skills)}
          disabled={busy || state.deployable === 0}
          aria-label={t("library.group.deployAllLabel", { source: group.label })}
          title={
            busy
              ? (blockedReason ?? undefined)
              : state.deployable === 0
                ? t("library.group.nothingToDeploy")
                : t("library.group.deployAllLabel", { source: group.label })
          }
        >
          {t("library.group.deployAll")}
        </Button>
      </span>
    </div>
  );
};

export default memo(SourceGroupHeader);
