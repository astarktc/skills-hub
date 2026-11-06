import { memo, useId, useMemo, type CSSProperties } from "react";
import { Inbox } from "lucide-react";
import type { TFunction } from "i18next";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import type { ManagedSkill, OnboardingPlan } from "./types";
import type { SkillActions } from "../../hooks/useSkillActions";
import type { ListLens } from "../../hooks/useListLens";
import type { ImportLanding } from "../../hooks/useImportReview";
import type { PendingAction } from "../../lib/actionLifecycle";
import {
  INDIVIDUAL_GROUP_KEY,
  harnessInitials,
  nameColumnWidth,
  ROW_MARK_LABEL_KEY,
  rowMarks,
  skillHealth,
  skillSourceLine,
  syncedStrip,
  type SyncedStrip,
} from "../../lib/skillPresentation";
import { discoveryInvitation } from "../../lib/importReview";
import { GRID_COLUMNS } from "./listColumns";
import SkillRow, { type SkillRowLayout } from "./SkillRow";
import SourceGroupHeader from "./SourceGroupHeader";

type SkillsListProps = {
  /** My Skills' list lens: the visible skills, their groups (null = Flat), the view mode. */
  lens: ListLens<ManagedSkill>;
  /** How many skills the library holds (the empty state is the library's, not the filter's). */
  librarySize: number;
  plan: OnboardingPlan | null;
  /** The import cascade (ticket 21): the rows a large import just added, and the run's token. */
  landing: ImportLanding;
  /** The strip's Tools: the effective target set, registry order, labelled. */
  stripTools: readonly { id: string; label: string; folder: string }[];
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

/** One group card (D38): bordered, rounded, its rows on the `row` ground, clipped to the radius. */
const GROUP_CARD = "overflow-clip rounded-lg border border-border bg-row";

/**
 * The My Skills list: source groups as cards (the singleton fold on; each
 * collapsible, its header sticky inside the scroll area) or one Flat card,
 * 36 px rows or the grid's cards (`auto-grid` / `dense-grid`, ticket 24 —
 * the same row model, in reading order), plus its empty and first-run
 * states. Both views render one group header (`SourceGroupHeader`). The list pane is
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
  const { displayed, onScreen, groups, viewMode, collapsed, setCollapsed } = lens;
  const layout: SkillRowLayout = viewMode === "list" ? "list" : "card";
  // The invitation stays while anything unmanaged sits in the harnesses —
  // discovered skills, or only folders with an invalid SKILL.md (#98).
  const invitation = discoveryInvitation(plan);
  const discovered = invitation !== null;
  const invitationBody = invitation
    ? invitation.skills > 0
      ? [
          t("library.discovered.body", { count: invitation.skills }),
          invitation.invalid > 0 ? t("library.discovered.invalidToo", { count: invitation.invalid }) : null,
        ]
          .filter(Boolean)
          .join(" ")
      : t("library.discovered.invalidOnly", { count: invitation.invalid })
    : null;
  const invitationReview = invitation
    ? invitation.skills > 0
      ? t("library.discovered.review", { count: invitation.skills })
      : t("library.discovered.reviewInvalid", { count: invitation.invalid })
    : null;
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
  // The grid has no strip column to sit over: its header shows no initials,
  // and each card's marks carry their harness's name (AX name + title).
  const initialsLegend = useMemo(() => {
    if (layout !== "list") return null;
    const initials = harnessInitials(stripTools);
    return (
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
    );
  }, [layout, stripTools, t]);

  const renderRows = (skills: readonly ManagedSkill[], inlineSource: boolean, id?: string) => (
    <ul
      id={id}
      className={cn(
        "m-0 list-none p-0",
        layout === "list" && "py-1",
        // Row-major auto-placement: the cards' reading order is the DOM's,
        // which is the lens's `onScreen` order (j / k, ranges, Delete's focus).
        viewMode !== "list" && ["grid", GRID_COLUMNS[viewMode]],
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
          {discovered ? <p className="m-0 text-sm text-muted-foreground">{invitationBody}</p> : null}
          <div className="mt-2 flex flex-wrap items-center gap-2">
            {discovered ? (
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
                {invitationReview}
              </Button>
            ) : null}
            <Button
              variant={discovered ? "outline" : "solid"}
              tone={discovered ? "neutral" : "accent"}
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

  return (
    <div
      data-skill-list=""
      // `text-sm`: the name column's `ch` resolves at the names' own size.
      // `scroll-pt-15` (60 px): j / k scroll a name into view clear of the
      // sticky 40 px group header with its row or card whole beneath it — a
      // card's name sits 13 px inside its edge — never under the header.
      className="@container min-h-0 flex-1 scroll-pt-15 overflow-y-auto text-sm"
      style={paneStyle}
      aria-label={t("library.list")}
      role="region"
    >
      {discovered ? (
        <div className="flex flex-wrap items-center gap-x-3 gap-y-2 border-b border-border bg-card px-4 py-2">
          <Inbox className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
          <p className="m-0 min-w-0 text-sm text-muted-foreground">
            <span className="font-medium text-foreground">{t("library.discovered.title")}</span>
            {" · "}
            {invitationBody}
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
            {invitationReview}
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
            const headingId = `${listId}-${group.key}`;
            const rowsId = `${listId}-${group.key}-rows`;
            const isCollapsed = collapsed.has(group.key);
            return (
              <section key={group.key} aria-labelledby={headingId} className={GROUP_CARD}>
                <SourceGroupHeader
                  group={group}
                  layout={layout === "list" ? "list" : "grid"}
                  headingId={headingId}
                  rowsId={rowsId}
                  collapsed={isCollapsed}
                  onCollapsedChange={(on) => setCollapsed(group.key, on)}
                  selectedCount={group.skills.filter((skill) => selection.has(skill.id)).length}
                  onSelectGroup={onSelectGroup}
                  legend={initialsLegend}
                  busy={busy}
                  blockedReason={blockedReason}
                  actions={actions}
                  t={t}
                />
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
