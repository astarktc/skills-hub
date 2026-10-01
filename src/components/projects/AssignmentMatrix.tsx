import { Fragment, memo, useCallback, useLayoutEffect, useMemo, useRef } from "react";
import { GitBranch, Globe, LoaderCircle, RefreshCw, Search, Settings } from "lucide-react";
import type { TFunction } from "i18next";
import { Button } from "@/components/ui/button";
import { Chip } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldLabel, Input } from "@/components/ui/field";
import { Segmented, SegmentedItem } from "@/components/ui/segmented";
import type { ProjectDto, ProjectSkillAssignmentDto, ProjectToolDto } from "./types";
import type { ManagedSkill } from "../skills/types";
import StripMarkIcon from "../skills/StripMarkIcon";
import { projectsGroupByRepoPreference } from "../../lib/preferences";
import { useListLens } from "../../hooks/useListLens";
import { useMatrixFocusReconciliation } from "./useMatrixFocusReconciliation";
import {
  matrixCell,
  matrixColumns,
  matrixFilters,
  matrixLegend,
  type MatrixColumn,
  type MatrixFilter,
} from "../../lib/projectPresentation";

export type AssignmentMatrixProps = {
  project: ProjectDto;
  tools: ProjectToolDto[];
  assignments: ProjectSkillAssignmentDto[];
  skills: ManagedSkill[];
  /** The Tool catalog's label for a Tool key. */
  labelOf: (tool: string) => string;
  /** Installed project Tool keys (null while the catalog loads). */
  installedTools: readonly string[] | null;
  /** The cells the running action is about: their localized pending mark. */
  pendingCells: Set<string>;
  /** The skills the running action is about (a row's bulk verb runs through the door). */
  isRowPending: (skillId: string) => boolean;
  /**
   * Why every mutation control is disabled while any action runs (D19),
   * shown as its title; null = they may run.
   */
  blockedReason: string | null;
  matrixLoading: boolean;
  onToggleAssignment: (skillId: string, tool: string) => void;
  onSyncCell: (skillId: string, tool: string) => void;
  onAssignAll: (skill: ManagedSkill) => void;
  onUnassignAll: (skill: ManagedSkill) => void;
  onConfigureTools: () => void;
  t: TFunction;
};

const FILTERS: readonly MatrixFilter[] = ["all", "assigned", "issues"];

/**
 * The per-project surface (D26): every skill × the project's Tools. One
 * list lens (search, the Assigned · Issues chips, Grouped / Flat — sorted by
 * name; the sort menu lives on My Skills only). Each cell is the assignment
 * checkbox — its hit area the whole cell — plus the Synced strip's mark for
 * its state, the state's word on focus and hover, and **Sync this** where
 * the state is repairable (projectPresentation.matrixCell). A cell whose
 * skill is also deployed globally stays enabled and is marked "also global"
 * (D17). The row's bulk verbs are named: Assign to all configured Tools ·
 * Unassign from all. When an action takes the focused row out of the shown
 * rows (a chip filters it away), the matrix moves focus to the next surviving
 * row's cell (`useMatrixFocusReconciliation`, N2).
 */
const AssignmentMatrix = ({
  project,
  tools,
  assignments,
  skills,
  labelOf,
  installedTools,
  pendingCells,
  isRowPending,
  blockedReason,
  matrixLoading,
  onToggleAssignment,
  onSyncCell,
  onAssignAll,
  onUnassignAll,
  onConfigureTools,
  t,
}: AssignmentMatrixProps) => {
  const filters = useMemo(() => matrixFilters(assignments), [assignments]);
  const lens = useListLens({
    items: skills,
    labels: {
      local: t("localGroup"),
      ungrouped: t("ungrouped"),
      individual: t("individualSources"),
    },
    groupByRepoPreference: projectsGroupByRepoPreference,
    filters,
  });
  const { query, setQuery, filter, setFilter, filterCounts, groups, visible } = lens;

  const assignmentMap = useMemo(() => {
    const map = new Map<string, ProjectSkillAssignmentDto>();
    for (const a of assignments) map.set(`${a.skill_id}:${a.tool}`, a);
    return map;
  }, [assignments]);
  const columns = useMemo(() => matrixColumns(tools, labelOf, installedTools), [installedTools, labelOf, tools]);
  const legend = useMemo(() => matrixLegend(t), [t]);
  const pathMissing = !project.path_exists;
  // The shown rows in display order: the post-filter focus rule reads it.
  const displayed = useMemo(
    () => (groups ? groups.flatMap((group) => group.skills) : visible).map((skill) => skill.id),
    [groups, visible],
  );
  const rootRef = useRef<HTMLDivElement>(null);
  const focusReconciliation = useMatrixFocusReconciliation(rootRef, displayed);

  if (skills.length === 0) {
    return <p className="m-0 px-4 py-6 text-sm text-muted-foreground">{t("projects.noSkills")}</p>;
  }
  if (tools.length === 0 && !matrixLoading) {
    return (
      <div className="flex flex-col items-start gap-3 px-4 py-6">
        <p className="m-0 text-sm text-muted-foreground">{t("projectsSurface.matrix.noTools")}</p>
        <Button variant="solid" tone="accent" onClick={onConfigureTools}>
          <Settings />
          {t("projectsSurface.header.configureTools")}
        </Button>
      </div>
    );
  }

  const rowProps = {
    columns,
    assignmentMap,
    pendingCells,
    isRowPending,
    pathMissing,
    blockedReason,
    onToggleAssignment,
    onSyncCell,
    onAssignAll,
    onUnassignAll,
    t,
  };

  return (
    <div
      ref={rootRef}
      className="flex min-h-0 flex-1 flex-col"
      onFocus={focusReconciliation.onFocus}
      onBlur={focusReconciliation.onBlur}
    >
      <div className="flex min-h-10 shrink-0 flex-wrap items-center gap-x-4 gap-y-2 border-b border-border px-4 py-2">
        <Field className="flex-row items-center gap-2">
          <FieldLabel>
            <Search className="size-4 text-muted-foreground" aria-hidden="true" />
            <span className="sr-only">{t("projectsSurface.matrix.searchLabel")}</span>
          </FieldLabel>
          <Input
            type="search"
            size="sm"
            className="w-44"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Escape" && query) {
                event.preventDefault();
                setQuery("");
              }
            }}
            placeholder={t("projectsSurface.matrix.searchPlaceholder")}
          />
        </Field>
        <div role="group" aria-label={t("projectsSurface.matrix.filterGroup")} className="flex items-center gap-1.5">
          {FILTERS.map((key) => (
            <Chip key={key} data-matrix-filter={key} pressed={filter === key} onPressedChange={() => setFilter(key)}>
              {t(`projectsSurface.matrix.filters.${key}`)}{" "}
              <span className="text-faint tabular-nums">{filterCounts[key] ?? 0}</span>
            </Chip>
          ))}
        </div>
        <Segmented
          className="ml-auto"
          aria-label={t("projectsSurface.matrix.grouping")}
          value={lens.groupByRepo ? "grouped" : "flat"}
          onValueChange={(value) => lens.setGroupByRepo(value === "grouped")}
        >
          <SegmentedItem value="grouped">{t("library.grouped")}</SegmentedItem>
          <SegmentedItem value="flat">{t("library.flat")}</SegmentedItem>
        </Segmented>
      </div>

      <ul
        aria-label={t("projectsSurface.legend.label")}
        className="m-0 flex shrink-0 list-none flex-wrap items-center gap-x-4 gap-y-1 border-b border-border px-4 py-1.5 text-xs text-muted-foreground"
      >
        {legend.map((entry) => (
          <li key={entry.state} data-legend={entry.state} title={entry.detail} className="flex items-center gap-1.5">
            <StripMarkIcon state={entry.mark} />
            {entry.word}
          </li>
        ))}
        <li data-legend="also-global" title={t("projectsSurface.legend.alsoGlobalDetail")} className="flex items-center gap-1.5">
          <Globe aria-hidden="true" className="size-3 text-muted-foreground" />
          {t("projectsSurface.legend.alsoGlobal")}
        </li>
      </ul>

      {matrixLoading ? (
        <div aria-hidden="true" className="grid grid-cols-4 gap-2 px-4 py-3">
          {Array.from({ length: 12 }).map((_, i) => (
            <div key={i} className="h-6 rounded-sm bg-raised" />
          ))}
        </div>
      ) : visible.length === 0 ? (
        <p className="m-0 px-4 py-6 text-sm text-muted-foreground">{t("projectsSurface.matrix.noMatch")}</p>
      ) : (
        <div className="min-h-0 flex-1 overflow-auto">
          <table className="w-full border-separate border-spacing-0 text-sm">
            <caption className="sr-only">{t("projectsSurface.matrix.caption", { project: project.name })}</caption>
            <thead className="sticky top-0 z-10 bg-background">
              <tr>
                <th scope="col" className="h-row border-b border-border px-4 text-left text-xs font-medium text-muted-foreground">
                  {t("projectsSurface.matrix.skill")}
                </th>
                {columns.map((column) => (
                  <th key={column.key} scope="col" className="h-row min-w-36 border-b border-border px-2 text-left align-middle">
                    <span className="block text-xs font-medium text-foreground">{column.label}</span>
                    {column.detected ? null : (
                      <span className="block text-xs font-normal text-faint">{t("projectsSurface.matrix.notDetected")}</span>
                    )}
                  </th>
                ))}
                <th scope="col" className="h-row border-b border-border px-2 text-left text-xs font-medium text-muted-foreground">
                  {t("projectsSurface.matrix.bulk")}
                </th>
              </tr>
            </thead>
            <tbody>
              {groups
                ? groups.map((group) => (
                    <Fragment key={group.key}>
                      <tr>
                        <th
                          scope="colgroup"
                          colSpan={columns.length + 2}
                          className="h-row border-b border-border bg-card px-4 text-left text-xs font-medium text-muted-foreground"
                        >
                          <span className="flex items-center gap-1.5">
                            <GitBranch aria-hidden="true" className="size-3.5" />
                            {group.label}
                          </span>
                        </th>
                      </tr>
                      {group.skills.map((skill) => (
                        <MatrixRow key={skill.id} skill={skill} {...rowProps} />
                      ))}
                    </Fragment>
                  ))
                : visible.map((skill) => <MatrixRow key={skill.id} skill={skill} {...rowProps} />)}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};

type MatrixRowProps = {
  skill: ManagedSkill;
  columns: MatrixColumn[];
  assignmentMap: Map<string, ProjectSkillAssignmentDto>;
  pendingCells: Set<string>;
  isRowPending: (skillId: string) => boolean;
  pathMissing: boolean;
  blockedReason: string | null;
  onToggleAssignment: (skillId: string, tool: string) => void;
  onSyncCell: (skillId: string, tool: string) => void;
  onAssignAll: (skill: ManagedSkill) => void;
  onUnassignAll: (skill: ManagedSkill) => void;
  t: TFunction;
};

const MatrixRow = memo(
  ({
    skill,
    columns,
    assignmentMap,
    pendingCells,
    isRowPending,
    pathMissing,
    blockedReason,
    onToggleAssignment,
    onSyncCell,
    onAssignAll,
    onUnassignAll,
    t,
  }: MatrixRowProps) => {
    const rowPending = isRowPending(skill.id);
    const assignedCount = columns.filter((column) => assignmentMap.has(`${skill.id}:${column.key}`)).length;
    // Both bulk verbs stay where they are — a verb that would change nothing
    // is refused with the reason, so the focus that ran it never drops (D21).
    const bulkReason = pathMissing ? t("projectsSurface.cell.folderMissing") : blockedReason;
    const assignReason =
      bulkReason ?? (assignedCount === columns.length ? t("projectsSurface.matrix.allAssigned") : null);
    const unassignReason = bulkReason ?? (assignedCount === 0 ? t("projectsSurface.matrix.noneAssigned") : null);
    return (
      <tr data-matrix-row={skill.id} aria-busy={rowPending || undefined} className="hover:bg-card">
        <th
          scope="row"
          title={skill.description ?? undefined}
          className="h-row border-b border-border px-4 text-left text-sm font-medium break-words text-foreground"
        >
          {skill.name}
        </th>
        {columns.map((column) => (
          <MatrixCellView
            key={column.key}
            skill={skill}
            column={column}
            assignment={assignmentMap.get(`${skill.id}:${column.key}`)}
            inFlight={pendingCells.has(`${skill.id}:${column.key}`)}
            pathMissing={pathMissing}
            blockedReason={blockedReason}
            onToggle={onToggleAssignment}
            onSync={onSyncCell}
            t={t}
          />
        ))}
        <td className="h-row border-b border-border px-2">
          <span className="flex items-center gap-1">
            <Button
              variant="ghost"
              tone="accent"
              size="sm"
              aria-label={t("projectsSurface.matrix.assignAllNamed", { skill: skill.name })}
              title={assignReason ?? t("projectsSurface.matrix.assignAllNamed", { skill: skill.name })}
              aria-description={assignReason ?? undefined}
              disabled={assignReason !== null}
              focusableWhenDisabled={!pathMissing}
              pending={rowPending}
              onClick={() => onAssignAll(skill)}
            >
              {t("projectsSurface.matrix.assignAll")}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              aria-label={t("projectsSurface.matrix.unassignAllNamed", { skill: skill.name })}
              title={unassignReason ?? t("projectsSurface.matrix.unassignAllNamed", { skill: skill.name })}
              aria-description={unassignReason ?? undefined}
              disabled={unassignReason !== null}
              focusableWhenDisabled={!pathMissing}
              onClick={() => onUnassignAll(skill)}
            >
              {t("projectsSurface.matrix.unassignAll")}
            </Button>
          </span>
        </td>
      </tr>
    );
  },
);
MatrixRow.displayName = "MatrixRow";

type MatrixCellViewProps = {
  skill: ManagedSkill;
  column: MatrixColumn;
  assignment: ProjectSkillAssignmentDto | undefined;
  inFlight: boolean;
  pathMissing: boolean;
  blockedReason: string | null;
  onToggle: (skillId: string, tool: string) => void;
  onSync: (skillId: string, tool: string) => void;
  t: TFunction;
};

/**
 * One cell: a `<label>` spanning the cell is the checkbox's hit area (≥ the
 * 24 px floor: the cell is the 36 px row), then the mark, "also global" and
 * the state's word (shown on hover and focus; always in the checkbox's
 * name), then **Sync this** for a stale, missing or failed assignment.
 */
function MatrixCellView({ skill, column, assignment, inFlight, pathMissing, blockedReason, onToggle, onSync, t }: MatrixCellViewProps) {
  const cell = matrixCell({ skill, tool: { key: column.key, label: column.label }, assignment, inFlight }, t);
  const blocked = blockedReason !== null;
  const reason = pathMissing ? t("projectsSurface.cell.folderMissing") : blockedReason;
  const toggle = useCallback(() => onToggle(skill.id, column.key), [column.key, onToggle, skill.id]);
  const hint = [cell.word, cell.detail].filter(Boolean).join(" \u2014 ");
  // Sync this goes once the cell is healthy: if it held focus, the cell's
  // checkbox takes it (D21 — never <body>).
  const checkboxRef = useRef<HTMLButtonElement>(null);
  const repairFocused = useRef(false);
  const repairable = cell.repairs.includes("sync");
  useLayoutEffect(() => {
    if (repairable || !repairFocused.current) return;
    repairFocused.current = false;
    checkboxRef.current?.focus();
  }, [repairable]);
  return (
    <td data-cell-state={cell.state} data-matrix-col={column.key} className="group/cell h-row border-b border-border p-0">
      <span className="flex h-full items-center">
        <label
          title={reason ?? (hint || undefined)}
          className="flex h-full min-h-6 min-w-0 flex-1 cursor-default items-center gap-1.5 px-2 group-hover/cell:bg-raised"
        >
          <Checkbox
            ref={checkboxRef}
            checked={cell.assigned}
            onCheckedChange={toggle}
            disabled={pathMissing || blocked}
            focusableWhenDisabled={!pathMissing}
            aria-label={cell.name}
            aria-description={reason ?? undefined}
            aria-busy={inFlight || undefined}
          />
          {inFlight ? (
            <LoaderCircle aria-hidden="true" className="size-3 shrink-0 text-muted-foreground motion-safe:animate-spin" />
          ) : cell.mark ? (
            <StripMarkIcon state={cell.mark} />
          ) : null}
          {cell.alsoGlobal ? (
            <Globe aria-hidden="true" className="size-3 shrink-0 text-muted-foreground" />
          ) : null}
          {cell.word ? (
            <span
              aria-hidden="true"
              className="invisible truncate text-xs text-muted-foreground group-hover/cell:visible group-focus-within/cell:visible"
            >
              {cell.word}
            </span>
          ) : null}
        </label>
        {repairable ? (
          <Button
            variant="ghost"
            size="icon-sm"
            className="mr-1"
            aria-label={t("projectsSurface.cell.syncThisNamed", { skill: skill.name, tool: column.label })}
            title={reason ?? t("projectsSurface.cell.syncThisNamed", { skill: skill.name, tool: column.label })}
            aria-description={reason ?? undefined}
            disabled={pathMissing || blocked}
            focusableWhenDisabled={!pathMissing}
            pending={inFlight}
            onFocus={() => {
              repairFocused.current = true;
            }}
            onBlur={(event) => {
              // A blur from the button's own removal keeps the flag.
              if (event.currentTarget.isConnected) repairFocused.current = false;
            }}
            onClick={() => onSync(skill.id, column.key)}
          >
            <RefreshCw />
          </Button>
        ) : null}
      </span>
    </td>
  );
}

export default memo(AssignmentMatrix);
