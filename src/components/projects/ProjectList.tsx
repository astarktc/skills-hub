import { memo } from "react";
import { Plus, Trash2 } from "lucide-react";
import type { TFunction } from "i18next";
import type { ProjectDto } from "./types";

type ProjectListProps = {
  projects: ProjectDto[];
  selectedProjectId: string | null;
  loading: boolean;
  loadError: string | null;
  onSelectProject: (id: string) => void;
  onAddProject: () => void;
  onRemoveProject: (id: string) => void;
  t: TFunction;
};

const ProjectList = ({
  projects,
  selectedProjectId,
  loading,
  loadError,
  onSelectProject,
  onAddProject,
  onRemoveProject,
  t,
}: ProjectListProps) => {
  return (
    <aside className="project-list">
      <div className="project-list-header">
        <span className="project-list-title">{t("navProjects")}</span>
        <button
          className="btn-icon"
          type="button"
          onClick={onAddProject}
          aria-label={t("projects.addProject")}
        >
          <Plus size={16} />
        </button>
      </div>

      {loadError ? (
        <div className="project-list-error">{t("projects.loadError")}</div>
      ) : loading ? (
        <div className="project-list-skeleton" aria-hidden="true">
          <div className="skeleton-row" />
          <div className="skeleton-row" />
          <div className="skeleton-row" />
        </div>
      ) : (
        <div role="listbox" aria-label={t("navProjects")}>
          {projects.map((p) => (
            <div
              key={p.id}
              className={`project-item${selectedProjectId === p.id ? " selected" : ""}`}
              onClick={() => onSelectProject(p.id)}
              role="option"
              aria-selected={selectedProjectId === p.id}
            >
              <div className="project-item-row">
                <span className="project-item-name">{p.name}</span>
                <button
                  className="btn-icon remove-btn"
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    onRemoveProject(p.id);
                  }}
                  aria-label={t("projects.removeTitle")}
                >
                  <Trash2 size={14} />
                </button>
              </div>
              <span className="project-item-path">{p.path}</span>
              <div className="project-item-meta">
                <span>
                  {t("projects.assignmentCount", { count: p.assignment_count })}
                </span>
                <span className={`project-status-dot ${p.sync_status}`} />
              </div>
            </div>
          ))}
        </div>
      )}
    </aside>
  );
};

export default memo(ProjectList);
