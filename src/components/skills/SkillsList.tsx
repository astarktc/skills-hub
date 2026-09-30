import { memo, useId, useMemo, type CSSProperties } from "react";
import { Folder, GitBranch, Inbox, Layers } from "lucide-react";
import type { TFunction } from "i18next";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import type { ManagedSkill, OnboardingPlan } from "./types";
import type { SkillActions } from "../../hooks/useSkillActions";
import type { ListLens } from "../../hooks/useListLens";
import type { PendingAction } from "../../lib/actionLifecycle";
import {
  INDIVIDUAL_GROUP_KEY,
  LOCAL_GROUP_KEY,
  groupState,
  skillSourceLine,
  syncedStrip,
  type RepoGroup,
  type SyncedStrip,
} from "../../lib/skillPresentation";
import SkillRow, { type SkillRowLayout } from "./SkillRow";

type SkillsListProps = {
  /** My Skills' list lens: the visible skills, their groups (null = Flat), the view mode. */
  lens: ListLens<ManagedSkill>;
  /** How many skills the library holds (the empty state is the library's, not the filter's). */
  librarySize: number;
  plan: OnboardingPlan | null;
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

/**
 * The My Skills list: sticky source groups (the singleton fold on) or a
 * Flat list, 36 px rows or cards, plus its empty and first-run states. The
 * list pane is a size container, so the row drops its description column
 * when the pane is narrow, whatever the window.
 */
const SkillsList = ({
  lens,
  librarySize,
  plan,
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
  const { displayed, groups, viewMode } = lens;
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
  // any Tool a row still has outside it).
  const stripMarks = Math.max(1, ...[...strips.values()].map((strip) => strip.total));
  const paneStyle = { "--strip-marks": stripMarks } as CSSProperties;

  const renderRows = (skills: readonly ManagedSkill[], inlineSource: boolean) => (
    <ul
      className={cn(
        "m-0 list-none p-0",
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
              <Button variant="solid" tone="accent" onClick={onReviewImport} disabled={busy} title={blockedReason ?? undefined}>
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

  const groupIcon = (group: RepoGroup<ManagedSkill>) =>
    group.key === LOCAL_GROUP_KEY ? (
      <Folder className="size-3.5 shrink-0 text-faint" aria-hidden="true" />
    ) : group.key === INDIVIDUAL_GROUP_KEY ? (
      <Layers className="size-3.5 shrink-0 text-faint" aria-hidden="true" />
    ) : (
      <GitBranch className="size-3.5 shrink-0 text-faint" aria-hidden="true" />
    );

  return (
    <div
      data-skill-list=""
      className="@container min-h-0 flex-1 overflow-y-auto"
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
            onClick={onReviewImport}
            disabled={busy}
            title={blockedReason ?? undefined}
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
        groups.map((group) => {
          const state = groupState(group.skills);
          const headingId = `${listId}-${group.key}`;
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
            <section key={group.key} aria-labelledby={headingId}>
              <div className="sticky top-0 z-10 flex h-row items-center gap-3 border-b border-border bg-card px-3">
                <span className="flex w-8 shrink-0 justify-center">
                  <Checkbox
                    checked={allSelected}
                    indeterminate={selectedHere > 0 && !allSelected}
                    onCheckedChange={() => onSelectGroup(group.skills.map((skill) => skill.id), !allSelected)}
                    aria-label={t("bulk.selectGroup", { count: group.skills.length, source: group.label })}
                  />
                </span>
                {groupIcon(group)}
                {/* Truncation lives on the inner text, never on an ancestor of
                    the link: an overflow-clipping heading would cut its focus
                    ring (2 px, offset 2 px) down to corner fragments. */}
                <h2 id={headingId} className="m-0 flex min-w-0 text-sm font-semibold text-foreground">
                  {group.href ? (
                    <a
                      href={group.href}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="flex min-w-0 rounded-sm hover:underline"
                    >
                      <span className="truncate">{group.label}</span>
                    </a>
                  ) : (
                    <span className="truncate">{group.label}</span>
                  )}
                </h2>
                <span className="shrink-0 text-xs text-faint tabular-nums">
                  {t("library.group.count", { count: group.skills.length })}
                </span>
                {parts.length > 0 ? (
                  <span className="flex min-w-0 items-center gap-2 truncate text-xs">
                    {parts.map(([key, count, tone]) => (
                      <span key={key} className={tone}>
                        {t(`library.group.${key}`, { count })}
                      </span>
                    ))}
                  </span>
                ) : null}
                <Button
                  variant="ghost"
                  size="sm"
                  className="ml-auto"
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
              </div>
              {renderRows(group.skills, group.key === INDIVIDUAL_GROUP_KEY)}
            </section>
          );
        })
      ) : (
        renderRows(displayed, true)
      )}
    </div>
  );
};

export default memo(SkillsList);
