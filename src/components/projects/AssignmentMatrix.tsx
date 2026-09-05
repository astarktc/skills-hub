import React, { memo, useCallback, useMemo, useState } from "react";
import {
  TriangleAlert,
  ArrowUpDown,
  GitBranch,
  Globe,
  RefreshCw,
  Settings,
} from "lucide-react";
import type { TFunction } from "i18next";
import type {
  ProjectDto,
  ProjectSkillAssignmentDto,
  ProjectToolDto,
  ResyncSummaryDto,
} from "./types";
import type { ManagedSkill } from "../skills/types";
import type { NotifyFn } from "../../hooks/useStatusReporter";
import { describeCommandError } from "../../commandError";
import { SYNC_STATUS_CLASS } from "../../syncStatus";
import {
  filterAndSortSkills,
  formatRelativeTime,
  groupSkillsByRepo,
} from "../../lib/skillPresentation";
import { projectsGroupByRepoPreference } from "../../lib/preferences";
import { usePersistedPreference } from "../../hooks/usePersistedPreference";

export type AssignmentMatrixProps = {
  project: ProjectDto | null;
  tools: ProjectToolDto[];
  assignments: ProjectSkillAssignmentDto[];
  /**
   * False when the backend skipped the reconcile pass because a Sync-target
   * mutation was in flight: the statuses below are the stored ones, not
   * re-derived from disk. Surfaced as a notice — never rendered as healthy.
   */
  assignmentsReconciled: boolean;
  skills: ManagedSkill[];
  pendingCells: Set<string>;
  matrixLoading: boolean;
  onToggleAssignment: (skillId: string, tool: string) => Promise<void>;
  onBulkAssign: (skillId: string) => Promise<void>;
  onResyncProject: () => Promise<ResyncSummaryDto>;
  onResyncAll: () => Promise<ResyncSummaryDto[]>;
  onConfigureTools: () => void;
  /** The reporter's notification entry point, handed down by the page. */
  notify: NotifyFn;
  t: TFunction;
};

const AssignmentMatrix = ({
  project,
  tools,
  assignments,
  assignmentsReconciled,
  skills,
  pendingCells,
  matrixLoading,
  onToggleAssignment,
  onBulkAssign,
  onResyncProject,
  onResyncAll,
  onConfigureTools,
  notify,
  t,
}: AssignmentMatrixProps) => {
  const lastSyncAt = useMemo(() => {
    let max = 0;
    for (const a of assignments) {
      if (a.synced_at && a.synced_at > max) max = a.synced_at;
    }
    return max > 0 ? max : null;
  }, [assignments]);

  const [sortBy, setSortBy] = useState<"name" | "updated" | "added">("name");
  const [groupByRepo, setGroupByRepo] = usePersistedPreference(
    projectsGroupByRepoPreference,
  );

  const sortedSkills = useMemo(
    () => filterAndSortSkills(skills, { query: "", sort: sortBy }),
    [skills, sortBy],
  );

  const skillGroups = useMemo(() => {
    if (!groupByRepo) return null;
    return groupSkillsByRepo(sortedSkills, {
      local: t("localGroup"),
      ungrouped: t("ungrouped"),
    });
  }, [groupByRepo, sortedSkills, t]);

  const assignmentMap = useMemo(() => {
    const map = new Map<string, ProjectSkillAssignmentDto>();
    for (const a of assignments) {
      map.set(`${a.skill_id}:${a.tool}`, a);
    }
    return map;
  }, [assignments]);

  const pathMissing = project ? !project.path_exists : false;

  const handleResyncProject = useCallback(async () => {
    try {
      const summary = await onResyncProject();
      if (summary.failed > 0) {
        notify(
          "warning",
          t("projects.resyncPartial", {
            synced: summary.synced,
            failed: summary.failed,
          }),
        );
      } else {
        notify(
          "success",
          t("projects.resyncSuccess", { synced: summary.synced }),
        );
      }
    } catch (err) {
      const msg = describeCommandError(err, t);
      if (msg) notify("error", msg);
    }
  }, [notify, onResyncProject, t]);

  const handleResyncAll = useCallback(async () => {
    try {
      const summaries = await onResyncAll();
      const totalSynced = summaries.reduce((sum, s) => sum + s.synced, 0);
      const totalFailed = summaries.reduce((sum, s) => sum + s.failed, 0);
      if (totalFailed > 0) {
        notify(
          "warning",
          t("projects.resyncPartial", {
            synced: totalSynced,
            failed: totalFailed,
          }),
        );
      } else {
        notify(
          "success",
          t("projects.resyncSuccess", { synced: totalSynced }),
        );
      }
    } catch (err) {
      const msg = describeCommandError(err, t);
      if (msg) notify("error", msg);
    }
  }, [notify, onResyncAll, t]);

  if (!project) {
    return (
      <div className="matrix-placeholder">{t("projects.selectProject")}</div>
    );
  }

  const lastSyncDisplay = lastSyncAt
    ? t("projects.lastSyncTime", {
        time: formatRelativeTime(lastSyncAt, t),
      })
    : t("projects.lastSyncNever");

  return (
    <div className="matrix-content">
      <div className="matrix-toolbar">
        <div className="matrix-toolbar-info">
          <span className="matrix-toolbar-name">{project.name}</span>
          <span className="matrix-toolbar-path">{project.path}</span>
          <span className="matrix-toolbar-sync-time">{lastSyncDisplay}</span>
        </div>
        <div className="matrix-toolbar-filters">
          <button className="btn btn-secondary btn-sm sort-btn" type="button">
            <span className="sort-label">{t("filterSort")}:</span>
            {sortBy === "name"
              ? t("sortName")
              : sortBy === "added"
                ? t("sortAdded")
                : t("sortUpdated")}
            <ArrowUpDown size={12} />
            <select
              aria-label={t("filterSort")}
              value={sortBy}
              onChange={(e) =>
                setSortBy(e.target.value as "name" | "updated" | "added")
              }
            >
              <option value="name">{t("sortName")}</option>
              <option value="updated">{t("sortUpdated")}</option>
              <option value="added">{t("sortAdded")}</option>
            </select>
          </button>
          <label className="group-by-repo-toggle">
            <input
              type="checkbox"
              checked={groupByRepo}
              onChange={(e) => setGroupByRepo(e.target.checked)}
            />
            <span className="group-by-repo-label">{t("groupByRepo")}</span>
          </label>
        </div>
        <div className="matrix-toolbar-actions">
          <button
            className="btn btn-secondary btn-sm"
            onClick={onConfigureTools}
          >
            <Settings size={14} />
            {t("projects.addTools")}
          </button>
          <button
            className="btn btn-primary btn-sm"
            onClick={handleResyncProject}
            disabled={pathMissing}
            title={
              pathMissing
                ? t("projects.syncDisabledMissing")
                : t("projects.syncProject")
            }
          >
            <RefreshCw size={14} />
            {t("projects.syncProject")}
          </button>
          <button
            className="btn btn-secondary btn-sm"
            onClick={handleResyncAll}
            disabled={pathMissing}
            title={
              pathMissing
                ? t("projects.syncDisabledMissing")
                : t("projects.syncAll")
            }
          >
            <RefreshCw size={14} />
            {t("projects.syncAll")}
          </button>
        </div>
      </div>

      {pathMissing && (
        <div className="matrix-path-missing-banner">
          <TriangleAlert size={14} />
          <span>{t("projects.syncDisabledMissing")}</span>
        </div>
      )}

      {!assignmentsReconciled && (
        <div className="matrix-path-missing-banner">
          <TriangleAlert size={14} />
          <span>{t("projects.reconcileSkipped")}</span>
        </div>
      )}

      {skills.length === 0 ? (
        <div className="matrix-no-skills">{t("projects.noSkills")}</div>
      ) : tools.length === 0 ? (
        <div className="matrix-no-skills">
          {t("projects.addTools")}
          <button
            className="btn btn-secondary btn-sm"
            style={{ marginLeft: 8 }}
            onClick={onConfigureTools}
          >
            <Settings size={14} />
            {t("projects.addTools")}
          </button>
        </div>
      ) : matrixLoading ? (
        <div className="matrix-skeleton">
          {Array.from({ length: 12 }).map((_, i) => (
            <div key={i} className="skeleton-cell" />
          ))}
        </div>
      ) : (
        <div
          className={`matrix-grid${groupByRepo ? " matrix-grid-grouped" : ""}`}
        >
          <table>
            <thead>
              <tr className="matrix-header-row">
                <th />

                {tools.map((tool) => (
                  <th key={tool.id}>{tool.tool}</th>
                ))}
                <th />
              </tr>
            </thead>
            <tbody>
              {skillGroups
                ? skillGroups.map((group) => (
                    <React.Fragment key={group.key}>
                      <tr className="matrix-group-header-row">
                        <td colSpan={tools.length + 2}>
                          <span className="matrix-group-label">
                            <GitBranch size={14} className="repo-group-icon" />
                            {group.label}
                          </span>
                        </td>
                      </tr>
                      {group.skills.map((skill) => (
                        <MatrixRow
                          key={skill.id}
                          skill={skill}
                          tools={tools}
                          assignmentMap={assignmentMap}
                          pendingCells={pendingCells}
                          disabled={pathMissing}
                          showBulkAssign={tools.length > 1}
                          onToggleAssignment={onToggleAssignment}
                          onBulkAssign={onBulkAssign}
                          t={t}
                        />
                      ))}
                    </React.Fragment>
                  ))
                : sortedSkills.map((skill) => (
                    <MatrixRow
                      key={skill.id}
                      skill={skill}
                      tools={tools}
                      assignmentMap={assignmentMap}
                      pendingCells={pendingCells}
                      disabled={pathMissing}
                      showBulkAssign={tools.length > 1}
                      onToggleAssignment={onToggleAssignment}
                      onBulkAssign={onBulkAssign}
                      t={t}
                    />
                  ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};

type MatrixRowProps = {
  skill: ManagedSkill;
  tools: ProjectToolDto[];
  assignmentMap: Map<string, ProjectSkillAssignmentDto>;
  pendingCells: Set<string>;
  disabled: boolean;
  showBulkAssign: boolean;
  onToggleAssignment: (skillId: string, tool: string) => Promise<void>;
  onBulkAssign: (skillId: string) => Promise<void>;
  t: TFunction;
};

function setsEqual(a: Set<string>, b: Set<string>): boolean {
  if (a.size !== b.size) return false;
  for (const v of a) {
    if (!b.has(v)) return false;
  }
  return true;
}

const MatrixRow = memo(
  ({
    skill,
    tools,
    assignmentMap,
    pendingCells,
    disabled,
    showBulkAssign,
    onToggleAssignment,
    onBulkAssign,
    t,
  }: MatrixRowProps) => {
    return (
      <tr className="matrix-row">
        <td
          className="matrix-skill-cell"
          title={skill.description ?? undefined}
        >
          {skill.name}
        </td>
        {tools.map((tool) => {
          const cellKey = `${skill.id}:${tool.tool}`;
          const isPending = pendingCells.has(cellKey);
          const assignment = assignmentMap.get(`${skill.id}:${tool.tool}`);
          const isGlobal = skill.targets.some((gt) => gt.tool === tool.tool);
          const lockedGlobal = isGlobal && !assignment;
          const statusClass = isPending
            ? SYNC_STATUS_CLASS.pending
            : assignment
              ? SYNC_STATUS_CLASS[assignment.status]
              : "";
          const isError = assignment?.status === "error";
          const errorTitle = isError
            ? t("projects.syncErrorPrefix") + (assignment?.last_error ?? "")
            : undefined;
          const cellTitle =
            errorTitle ?? (isGlobal ? t("projects.globalSynced") : undefined);

          return (
            <td
              key={cellKey}
              className={`matrix-cell ${statusClass}${isGlobal ? " global" : ""}`}
              title={cellTitle}
              onClick={
                isError && !disabled
                  ? () => onToggleAssignment(skill.id, tool.tool)
                  : undefined
              }
            >
              {isPending ? (
                <span className="cell-spinner" />
              ) : (
                <input
                  type="checkbox"
                  checked={!!assignment}
                  disabled={isPending || disabled || lockedGlobal}
                  onChange={() => onToggleAssignment(skill.id, tool.tool)}
                  aria-label={`${skill.name} - ${tool.tool}`}
                />
              )}
              {isGlobal && !isPending && (
                <Globe
                  size={10}
                  className="cell-global-icon"
                  aria-hidden="true"
                />
              )}
              {isError && (
                <span className="sr-only">
                  {t("projects.syncErrorPrefix")}
                  {assignment?.last_error ?? ""}
                </span>
              )}
            </td>
          );
        })}
        <td>
          {showBulkAssign && (
            <button
              className="btn btn-xs matrix-all-tools-btn"
              onClick={() => onBulkAssign(skill.id)}
              disabled={disabled}
            >
              {t("projects.allTools")}
            </button>
          )}
        </td>
      </tr>
    );
  },
  (prev, next) => {
    if (prev.skill !== next.skill) return false;
    if (prev.tools !== next.tools) return false;
    if (prev.assignmentMap !== next.assignmentMap) return false;
    if (prev.disabled !== next.disabled) return false;
    if (prev.showBulkAssign !== next.showBulkAssign) return false;
    if (prev.onToggleAssignment !== next.onToggleAssignment) return false;
    if (prev.onBulkAssign !== next.onBulkAssign) return false;
    if (prev.t !== next.t) return false;
    return setsEqual(prev.pendingCells, next.pendingCells);
  },
);

MatrixRow.displayName = "MatrixRow";

export default memo(AssignmentMatrix, (prev, next) => {
  if (prev.project !== next.project) return false;
  if (prev.tools !== next.tools) return false;
  if (prev.assignments !== next.assignments) return false;
  if (prev.assignmentsReconciled !== next.assignmentsReconciled) return false;
  if (prev.skills !== next.skills) return false;
  if (prev.matrixLoading !== next.matrixLoading) return false;
  if (prev.onToggleAssignment !== next.onToggleAssignment) return false;
  if (prev.onBulkAssign !== next.onBulkAssign) return false;
  if (prev.onResyncProject !== next.onResyncProject) return false;
  if (prev.onResyncAll !== next.onResyncAll) return false;
  if (prev.onConfigureTools !== next.onConfigureTools) return false;
  if (prev.t !== next.t) return false;
  return setsEqual(prev.pendingCells, next.pendingCells);
});
