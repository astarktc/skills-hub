import { memo } from "react";
import { useTranslation } from "react-i18next";
import { useProjectState } from "./useProjectState";

const ProjectsPage = () => {
  const { t } = useTranslation();
  const state = useProjectState();

  return (
    <div className="projects-page">
      {!state.projectsLoading &&
      !state.loadError &&
      state.projects.length === 0 ? (
        <div className="projects-empty-fullwidth">
          <p className="projects-empty-title">{t("projects.emptyTitle")}</p>
          <p className="projects-empty-body">{t("projects.emptyBody")}</p>
        </div>
      ) : (
        <div className="projects-layout">
          <aside className="project-list">
            <div className="project-list-header">
              <span className="project-list-title">{t("navProjects")}</span>
            </div>
            {state.loadError ? (
              <div className="project-list-error">
                {t("projects.loadError")}
              </div>
            ) : state.projectsLoading ? (
              <div className="project-list-skeleton" aria-hidden="true" />
            ) : (
              <div className="project-items">
                {state.projects.map((p) => (
                  <div
                    key={p.id}
                    className={`project-item${state.selectedProjectId === p.id ? " selected" : ""}`}
                    onClick={() => state.selectProject(p.id)}
                  >
                    <div className="project-item-name">{p.name}</div>
                    <div className="project-item-path">{p.path}</div>
                  </div>
                ))}
              </div>
            )}
          </aside>
          <section className="matrix-panel">
            {!state.selectedProjectId ? (
              <div className="matrix-placeholder">
                {t("projects.selectProject")}
              </div>
            ) : (
              <div className="matrix-content">
                {/* AssignmentMatrix component will be inserted by Plan 03 */}
                <p>{t("projects.selectProject")}</p>
              </div>
            )}
          </section>
        </div>
      )}
    </div>
  );
};

export default memo(ProjectsPage);
