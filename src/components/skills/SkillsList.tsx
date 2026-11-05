import { memo, useId, useMemo, type CSSProperties, type KeyboardEvent } from "react";
import { ChevronDown, ChevronRight, Folder, GitBranch, Inbox, Layers } from "lucide-react";
import type { TFunction } from "i18next";
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import type { ManagedSkill, OnboardingPlan } from "./types";
import type { SkillActions } from "../../hooks/useSkillActions";
import type { ListLens } from "../../hooks/useListLens";
import type { ImportLanding } from "../../hooks/useImportReview";
import type { PendingAction } from "../../lib/actionLifecycle";
import {
  INDIVIDUAL_GROUP_KEY,
  LOCAL_GROUP_KEY,
  groupState,
  harnessInitials,
  identityHue,
  identityInitials,
  nameColumnWidth,
  ROW_MARK_LABEL_KEY,
  rowMarks,
  skillHealth,
  skillSourceLine,
  syncedStrip,
  type RepoGroup,
  type SyncedStrip,
} from "../../lib/skillPresentation";
import { LIST_COLUMNS } from "./listColumns";
import SkillRow, { type SkillRowLayout } from "./SkillRow";

type SkillsListProps = {
  /** My Skills' list lens: the visible skills, their groups (null = Flat), the view mode. */
  lens: ListLens<ManagedSkill>;
  /** How many skills the library holds (the empty state is the library's, not the filter's). */
  librarySize: number;
  plan: OnboardingPlan | null;
  /** The import cascade (ticket 21): the rows a large import just added, and the run's token. */
  landing: ImportLanding;
  /** The strip's Tools: the effective target set, registry order, labelled. */
  stripTools: readonly { id: string; label: string }[];
  /** The Tool catalog's label for any key (a row outside the set). */
  labelOf: (tool: string) => string;
  isDetected: (tool: string) => boolean;
  /** The surface's selection (surface state, D4); the bulk rail acts on it. */
  selection: ReadonlySet<string>;
  /** The skill the inspector shows (surface state's `focused`), marked on its row. */
  focusedId: string | null;
  /** A row's checkbox: toggle it, or (shift-click) select the range to it. */
  onToggleSelect: (skillId: string, range: boolean) => void;
  /** A group header's checkbox: every skill of the source in or out. */
  onSelectGroup: (skillIds: readonly string[], on: boolean) => void;
  pending: PendingAction | null;
  blockedReason: string | null;
  actions: Pick<SkillActions, "run" | "repair">;
  onOpen: (skill: ManagedSkill) => void;
  onReviewImport: () => void;
  /** Review is the first task (it carries the primary; Add skill steps back). */
  discoveryLeads: boolean;
  onAddGit: () => void;
  onAddLocal: () => void;
  onExplore: () => void;
  t: TFunction;
};

const GRID_COLUMNS = {
  "auto-grid": "grid-cols-[repeat(auto-fill,minmax(20rem,1fr))]",
  "dense-grid": "grid-cols-[repeat(auto-fill,minmax(14rem,1fr))]",
} as const;

/** One group card (D38): bordered, rounded, its rows on the `row` ground, clipped to the radius. */
const GROUP_CARD = "overflow-clip rounded-lg border border-border bg-row";

/**
 * The My Skills list: source groups as cards (the singleton fold on; each
 * collapsible, its header sticky inside the scroll area) or one Flat card,
 * 36 px rows or tiles, plus its empty and first-run states. The list pane is
 * a size container, so the row drops its description column when the pane
 * is narrow, whatever the window. It publishes the shared column template's
 * two measures: `--strip-marks` and `--name-col` (`listColumns.ts`).
 */
const SkillsList = ({
  lens,
  librarySize,
  plan,
  landing,
  stripTools,
  labelOf,
  isDetected,
  selection,
  focusedId,
  onToggleSelect,
  onSelectGroup,
  pending,
  blockedReason,
  actions,
  onOpen,
  onReviewImport,
  discoveryLeads,
  onAddGit,
  onAddLocal,
  onExplore,
  t,
}: SkillsListProps) => {
  const { displayed, onScreen, groups, viewMode, collapsed, setCollapsed, toggleCollapsed } = lens;
  const layout: SkillRowLayout = viewMode === "list" ? "list" : "card";
  const discovered = plan?.group_count ?? 0;
  const busy = pending !== null;
  const listId = useId();

  const strips = useMemo(() => {
    const out = new Map<string, SyncedStrip>();
    for (const skill of displayed) out.set(skill.id, syncedStrip(skill, stripTools, { labelOf, isDetected }));
    return out;
  }, [displayed, isDetected, labelOf, stripTools]);

  // The strip column fits the widest strip in view (the effective set, plus
  // any Tool a row still has outside it) — collapsed groups included, so a
  // collapse never shifts the columns.
  const stripMarks = Math.max(1, ...[...strips.values()].map((strip) => strip.total));

  // The name column fits the longest name (+ its exception marks) a row
  // shows — a collapsed group's rows are not shown (D39).
  const nameCol = useMemo(
    () =>
      nameColumnWidth(
        onScreen.map((skill) => ({
          name: skill.name,
          marks: rowMarks(skillHealth(skill, { t })).map((row) => t(ROW_MARK_LABEL_KEY[row.kind], { count: row.count })),
          edited: skill.invocation_override !== null,
        })),
      ),
    [onScreen, t],
  );
  const paneStyle = { "--strip-marks": stripMarks, "--name-col": nameCol } as CSSProperties;

  // The header's initials row over the strip (D42): the Harness catalog's
  // labels, unique two-letter initials, in the strip's order.
  const initials = useMemo(() => harnessInitials(stripTools), [stripTools]);

  const renderRows = (skills: readonly ManagedSkill[], inlineSource: boolean, id?: string) => (
    <ul
      id={id}
      className={cn(
        "m-0 list-none p-0",
        layout === "list" && "py-1",
        layout === "card" && ["grid gap-3 p-3", GRID_COLUMNS[viewMode as keyof typeof GRID_COLUMNS]],
      )}
    >
      {skills.map((skill) => (
        <SkillRow
          key={skill.id}
          skill={skill}
          strip={strips.get(skill.id)!}
          layout={layout}
          sourceLabel={inlineSource ? skillSourceLine(skill, t).label : null}
          selected={selection.has(skill.id)}
          focused={focusedId === skill.id}
          onToggleSelect={onToggleSelect}
          pending={pending}
          blockedReason={blockedReason}
          actions={actions}
          onOpen={onOpen}
          labelOf={labelOf}
          landing={landing && landing.ids.has(skill.id) ? landing.token : null}
          t={t}
        />
      ))}
    </ul>
  );

  if (librarySize === 0) {
    return (
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="flex max-w-lg flex-col items-start gap-3 px-8 py-16">
          <h2 className="m-0 text-lg font-semibold text-foreground">{t("library.empty.title")}</h2>
          <p className="m-0 text-sm text-muted-foreground">{t("library.empty.body")}</p>
          {discovered > 0 ? (
            <p className="m-0 text-sm text-muted-foreground">
              {t("library.discovered.body", { count: discovered })}
            </p>
          ) : null}
          <div className="mt-2 flex flex-wrap items-center gap-2">
            {discovered > 0 ? (
              <Button
                data-import-review=""
                variant="solid"
                tone="accent"
                onClick={onReviewImport}
                disabled={busy}
                // Held focus survives the scan Review starts, so the dialog
                // returns it here (audit F2); the reason says why it waits.
                focusableWhenDisabled
                title={blockedReason ?? undefined}
                aria-description={busy ? (blockedReason ?? undefined) : undefined}
              >
                <Inbox />
                {t("library.discovered.review", { count: discovered })}
              </Button>
            ) : null}
            <Button
              variant={discovered > 0 ? "outline" : "solid"}
              tone={discovered > 0 ? "neutral" : "accent"}
              onClick={onAddGit}
              disabled={busy}
            >
              {t("library.empty.addGit")}
            </Button>
            <Button onClick={onAddLocal} disabled={busy}>
              {t("library.empty.addLocal")}
            </Button>
            <Button variant="ghost" onClick={onExplore}>
              {t("library.empty.explore")}
            </Button>
          </div>
        </div>
      </div>
    );
  }

  // The source's chip (ticket 30's pick A, header only): a GitHub owner's
  // identity hue and initials; otherwise the neutral chip with the source's
  // icon. `aria-hidden` — the source name beside it is the label.
  const groupChip = (group: RepoGroup<ManagedSkill>) =>
    group.owner ? (
      <Badge tone="identity" hue={identityHue(group.owner)} aria-hidden="true">
        {identityInitials(group.owner)}
      </Badge>
    ) : (
      <Badge variant="solid" tone="neutral" size="chip" aria-hidden="true">
        {group.key === LOCAL_GROUP_KEY ? <Folder /> : group.key === INDIVIDUAL_GROUP_KEY ? <Layers /> : <GitBranch />}
      </Badge>
    );

  /** ←/→ on a focused header collapse / expand its group (D42). */
  const onHeaderKey = (key: string) => (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.altKey || event.metaKey || event.ctrlKey || event.shiftKey) return;
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    setCollapsed(key, event.key === "ArrowLeft");
  };

  return (
    <div
      data-skill-list=""
      // `text-sm`: the name column's `ch` resolves at the names' own size.
      className="@container min-h-0 flex-1 overflow-y-auto text-sm"
      style={paneStyle}
      aria-label={t("library.list")}
      role="region"
    >
      {discovered > 0 ? (
        <div className="flex flex-wrap items-center gap-x-3 gap-y-2 border-b border-border bg-card px-4 py-2">
          <Inbox className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
          <p className="m-0 min-w-0 text-sm text-muted-foreground">
            <span className="font-medium text-foreground">{t("library.discovered.title")}</span>
            {" · "}
            {t("library.discovered.body", { count: discovered })}
          </p>
          <Button
            variant={discoveryLeads ? "solid" : "outline"}
            tone="accent"
            size="sm"
            className="ml-auto"
            data-import-review=""
            onClick={onReviewImport}
            disabled={busy}
            // Held focus survives the scan Review starts, so the dialog
            // returns it here (audit F2); the reason says why it waits.
            focusableWhenDisabled
            title={blockedReason ?? undefined}
            aria-description={busy ? (blockedReason ?? undefined) : undefined}
          >
            {t("library.discovered.review", { count: discovered })}
          </Button>
        </div>
      ) : null}

      {displayed.length === 0 ? (
        <div className="flex max-w-lg flex-col items-start gap-2 px-8 py-12">
          <h2 className="m-0 text-base font-semibold text-foreground">{t("library.noMatches.title")}</h2>
          <p className="m-0 text-sm text-muted-foreground">{t("library.noMatches.body")}</p>
          <Button
            size="sm"
            className="mt-2"
            onClick={() => {
              lens.setQuery("");
              lens.setFilter("all");
            }}
          >
            {t("library.noMatches.clear")}
          </Button>
        </div>
      ) : groups ? (
        <div className="flex flex-col gap-3 px-4 py-3">
          {groups.map((group) => {
            const state = groupState(group.skills);
            const headingId = `${listId}-${group.key}`;
            const rowsId = `${listId}-${group.key}-rows`;
            const isCollapsed = collapsed.has(group.key);
            const chevronLabel = t(isCollapsed ? "library.group.expand" : "library.group.collapse", { source: group.label });
            const selectedHere = group.skills.filter((skill) => selection.has(skill.id)).length;
            const allSelected = selectedHere === group.skills.length;
            const parts = (
              [
                ["failed", state.failed, "text-failed"],
                ["stale", state.stale, "text-stale"],
                ["notDeployed", state.notDeployed, "text-muted-foreground"],
                ["disabled", state.disabled, "text-faint"],
              ] as const
            ).filter(([, count]) => count > 0);
            return (
              <section key={group.key} aria-labelledby={headingId} className={GROUP_CARD}>
                {/* The header shares the rows' column template, so the harness
                    initials sit over the strip's marks. */}
                <div
                  data-group-header={group.key}
                  onKeyDown={onHeaderKey(group.key)}
                  className={cn(
                    "sticky top-0 z-10 grid h-10 items-center gap-x-3 bg-card px-3",
                    !isCollapsed && "border-b border-border",
                    LIST_COLUMNS,
                  )}
                >
                  <span className="flex justify-center">
                    <Checkbox
                      checked={allSelected}
                      indeterminate={selectedHere > 0 && !allSelected}
                      onCheckedChange={() => onSelectGroup(group.skills.map((skill) => skill.id), !allSelected)}
                      aria-label={t("bulk.selectGroup", { count: group.skills.length, source: group.label })}
                    />
                  </span>
                  <span className="col-span-2 flex min-w-0 items-center gap-2 @max-3xl:col-span-1">
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      aria-expanded={!isCollapsed}
                      aria-controls={isCollapsed ? undefined : rowsId}
                      aria-label={chevronLabel}
                      title={chevronLabel}
                      onClick={() => toggleCollapsed(group.key)}
                    >
                      {isCollapsed ? <ChevronRight /> : <ChevronDown />}
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
                  {layout === "list" ? (
                    <span role="group" aria-label={t("library.group.harnesses")} className="flex items-center gap-2">
                      {stripTools.map((tool) => (
                        <span
                          key={tool.id}
                          role="img"
                          aria-label={tool.label}
                          title={tool.label}
                          className="flex w-3 shrink-0 justify-center overflow-visible text-2xs leading-3 font-bold whitespace-nowrap text-faint"
                        >
                          {initials[tool.id]}
                        </span>
                      ))}
                    </span>
                  ) : (
                    <span />
                  )}
                  <span className="col-span-3 flex justify-end">
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
                {isCollapsed ? null : renderRows(group.skills, group.key === INDIVIDUAL_GROUP_KEY, rowsId)}
              </section>
            );
          })}
        </div>
      ) : (
        <div className="px-4 py-3">
          <div className={GROUP_CARD}>{renderRows(displayed, true)}</div>
        </div>
      )}
    </div>
  );
};

export default memo(SkillsList);
