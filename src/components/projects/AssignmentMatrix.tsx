import { memo, useCallback, useMemo } from "react";
import { RefreshCw, Settings } from "lucide-react";
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
          >
            <RefreshCw size={14} />
            {t("projects.syncProject")}
          </button>
          <button
            className="btn btn-secondary btn-sm"
            onClick={handleResyncAll}
          >
            <RefreshCw size={14} />
            {t("projects.syncAll")}
          </button>
        </div>
      </div>

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
              {skills.map((skill) => (
                <MatrixRow
                  key={skill.id}
                  skill={skill}
                  tools={tools}
                  assignments={assignments}
                  pendingCells={pendingCells}
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
  onToggleAssignment: (skillId: string, tool: string) => Promise<void>;
  onBulkAssign: (skillId: string) => Promise<void>;
  t: TFunction;
};

const MatrixRow = memo(
  ({
    skill,
    tools,
    assignments,
    pendingCells,
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
                isError
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
                  disabled={isPending}
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
          >
            {t("projects.allTools")}
          </button>
        </td>
      </tr>
    );
  },
);

MatrixRow.displayName = "MatrixRow";

export default memo(AssignmentMatrix);
