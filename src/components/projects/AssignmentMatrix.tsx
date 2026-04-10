import React, { memo, useCallback, useMemo, useState } from "react";
import {
  AlertTriangle,
  ArrowUpDown,
  GitBranch,
  RefreshCw,
  Settings,
} from "lucide-react";
import { toast } from "sonner";
import type { TFunction } from "i18next";
import type {
  ProjectDto,
  ProjectSkillAssignmentDto,
  ProjectToolDto,
  ResyncSummaryDto,
} from "./types";
import type { ManagedSkill } from "../skills/types";

export type AssignmentMatrixProps = {
  project: ProjectDto | null;
  tools: ProjectToolDto[];
  assignments: ProjectSkillAssignmentDto[];
  skills: ManagedSkill[];
  pendingCells: Set<string>;
  matrixLoading: boolean;
  onToggleAssignment: (skillId: string, tool: string) => Promise<void>;
  onBulkAssign: (skillId: string) => Promise<void>;
  onResyncProject: () => Promise<ResyncSummaryDto>;
  onResyncAll: () => Promise<ResyncSummaryDto[]>;
  onConfigureTools: () => void;
  t: TFunction;
};

function formatRelativeTime(timestampMs: number, t: TFunction): string {
  const diffMs = Date.now() - timestampMs;
  const diffSec = Math.floor(diffMs / 1000);
  if (diffSec < 60) return t("projects.justNow");
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return t("projects.minutesAgo", { count: diffMin });
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return t("projects.hoursAgo", { count: diffHr });
  const diffDay = Math.floor(diffHr / 24);
  return t("projects.daysAgo", { count: diffDay });
}

function shortRepoLabel(url: string): string | null {
  try {
    const normalized = url.replace(/^git\+/, "");
    const parsed = new URL(normalized);
    if (!parsed.hostname.includes("github.com")) return null;
    const parts = parsed.pathname.split("/").filter(Boolean);
    const owner = parts[0];
    const repo = parts[1]?.replace(/\.git$/, "");
    if (!owner || !repo) return null;
    return `${owner}/${repo}`;
  } catch {
    const match = url.match(/github\.com\/([^/]+)\/([^/#?]+)/i);
    if (!match) return null;
    return `${match[1]}/${match[2].replace(/\.git$/, "")}`;
  }
}

const AssignmentMatrix = ({
  project,
  tools,
  assignments,
  skills,
  pendingCells,
  matrixLoading,
  onToggleAssignment,
  onBulkAssign,
  onResyncProject,
  onResyncAll,
  onConfigureTools,
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
  const [groupByRepo, setGroupByRepo] = useState(false);

  const sortedSkills = useMemo(() => {
    return [...skills].sort((a, b) => {
      if (sortBy === "name") return a.name.localeCompare(b.name);
      if (sortBy === "added") return (b.created_at ?? 0) - (a.created_at ?? 0);
      return (b.updated_at ?? 0) - (a.updated_at ?? 0);
    });
  }, [skills, sortBy]);

  const skillGroups = useMemo(() => {
    if (!groupByRepo) return null;
    const map = new Map<string, ManagedSkill[]>();
    for (const skill of sortedSkills) {
      const ref = skill.source_ref ?? "";
      const isGitUrl =
        ref.startsWith("git+") ||
        ref.startsWith("https://") ||
        ref.startsWith("http://") ||
        ref.includes("github.com");
      const key = isGitUrl ? ref : "__local__";
      const list = map.get(key);
      if (list) {
        list.push(skill);
      } else {
        map.set(key, [skill]);
      }
    }
    const keys = [...map.keys()].sort((a, b) => {
      if (a === "__local__" && b !== "__local__") return 1;
      if (b === "__local__" && a !== "__local__") return -1;
      return a.localeCompare(b);
    });
    return keys.map((key) => {
      const short = key !== "__local__" && key ? shortRepoLabel(key) : null;
      return {
        key,
        label:
          key === "__local__"
            ? t("localGroup")
            : (short ?? key) || t("ungrouped"),
        skills: map.get(key)!,
      };
    });
  }, [groupByRepo, sortedSkills, t]);

  const pathMissing = project ? !project.path_exists : false;

  const handleResyncProject = useCallback(async () => {
    try {
      const summary = await onResyncProject();
      if (summary.failed > 0) {
        toast.warning(
          t("projects.resyncPartial", {
            synced: summary.synced,
            failed: summary.failed,
          }),
        );
      } else {
        toast.success(t("projects.resyncSuccess", { synced: summary.synced }));
      }
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }, [onResyncProject, t]);

  const handleResyncAll = useCallback(async () => {
    try {
      const summaries = await onResyncAll();
      const totalSynced = summaries.reduce((sum, s) => sum + s.synced, 0);
      const totalFailed = summaries.reduce((sum, s) => sum + s.failed, 0);
      if (totalFailed > 0) {
        toast.warning(
          t("projects.resyncPartial", {
            synced: totalSynced,
            failed: totalFailed,
          }),
        );
      } else {
        toast.success(t("projects.resyncSuccess", { synced: totalSynced }));
      }
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }, [onResyncAll, t]);

  if (!project) {
    return (
      <div className="matrix-placeholder">{t("projects.selectProject")}</div>
    );
  }

  const lastSyncDisplay = lastSyncAt
    ? t("projects.lastSyncTime", { time: formatRelativeTime(lastSyncAt, t) })
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
          <AlertTriangle size={14} />
          <span>{t("projects.syncDisabledMissing")}</span>
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
        <div className="matrix-grid">
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
                          <GitBranch size={14} className="repo-group-icon" />
                          {group.label}
                        </td>
                      </tr>
                      {group.skills.map((skill) => (
                        <MatrixRow
                          key={skill.id}
                          skill={skill}
                          tools={tools}
                          assignments={assignments}
                          pendingCells={pendingCells}
                          disabled={pathMissing}
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
                      assignments={assignments}
                      pendingCells={pendingCells}
                      disabled={pathMissing}
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
  assignments: ProjectSkillAssignmentDto[];
  pendingCells: Set<string>;
  disabled: boolean;
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
    assignments,
    pendingCells,
    disabled,
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
          const assignment = assignments.find(
            (a) => a.skill_id === skill.id && a.tool === tool.tool,
          );
          const statusClass = isPending
            ? "pending"
            : assignment
              ? assignment.status
              : "";
          const isError = assignment?.status === "error";
          const errorTitle = isError
            ? t("projects.syncErrorPrefix") + (assignment?.last_error ?? "")
            : undefined;

          return (
            <td
              key={cellKey}
              className={`matrix-cell ${statusClass}`}
              title={errorTitle}
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
                  disabled={isPending || disabled}
                  onChange={() => onToggleAssignment(skill.id, tool.tool)}
                  aria-label={`${skill.name} - ${tool.tool}`}
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
          <button
            className="btn btn-xs matrix-all-tools-btn"
            onClick={() => onBulkAssign(skill.id)}
            disabled={disabled}
          >
            {t("projects.allTools")}
          </button>
        </td>
      </tr>
    );
  },
  (prev, next) => {
    if (prev.skill !== next.skill) return false;
    if (prev.tools !== next.tools) return false;
    if (prev.assignments !== next.assignments) return false;
    if (prev.disabled !== next.disabled) return false;
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
