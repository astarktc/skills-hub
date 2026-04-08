import { memo, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
// Direct invoke import: projects subtree always runs inside Tauri context.
import { invoke } from "@tauri-apps/api/core";
import { FolderOpen } from "lucide-react";
import { toast } from "sonner";
import { useProjectState } from "./useProjectState";
import ProjectList from "./ProjectList";
import AssignmentMatrix from "./AssignmentMatrix";
import AddProjectModal from "./AddProjectModal";
import EditProjectModal from "./EditProjectModal";
import ToolConfigModal from "./ToolConfigModal";
import RemoveProjectModal from "./RemoveProjectModal";

const ProjectsPage = () => {
  const { t } = useTranslation();
  const state = useProjectState();

  // D-13: Store gitignore options from AddProjectModal so they survive
  // the modal transition to ToolConfigModal. The backend command
  // update_project_gitignore derives patterns from list_project_tools,
  // so it MUST be called AFTER tools are persisted -- not after registration.
  const pendingGitignoreRef = useRef<{
    projectId: string;
    addToGitignore: boolean;
    addToExclude: boolean;
  } | null>(null);

  const handleAddProject = useCallback(
    async (
      path: string,
      gitignoreOptions: { addToGitignore: boolean; addToExclude: boolean },
    ) => {
      try {
        const project = await state.registerProject(path);
        state.setShowAddModal(false);
        await state.selectProject(project.id);
        state.setShowToolConfigModal(true);
        await state.loadToolStatus();

        // D-13: Store gitignore options for use AFTER tool config confirmation.
        if (gitignoreOptions.addToGitignore || gitignoreOptions.addToExclude) {
          pendingGitignoreRef.current = {
            projectId: project.id,
            addToGitignore: gitignoreOptions.addToGitignore,
            addToExclude: gitignoreOptions.addToExclude,
          };
        } else {
          pendingGitignoreRef.current = null;
        }
      } catch (err) {
        toast.error(err instanceof Error ? err.message : String(err));
      }
    },
    [state],
  );

  const handleToolConfigConfirm = useCallback(
    async (selectedTools: string[]) => {
      try {
        const currentToolKeys = state.tools.map((t) => t.tool);
        const toAdd = selectedTools.filter((t) => !currentToolKeys.includes(t));
        const toRemove = currentToolKeys.filter(
          (t) => !selectedTools.includes(t),
        );
        if (toAdd.length > 0) await state.addTools(toAdd);
        if (toRemove.length > 0) await state.removeTools(toRemove);
        state.setShowToolConfigModal(false);

        // D-13 full delivery: NOW that tools are persisted in the database,
        // call update_project_gitignore. The backend derives gitignore patterns
        // from list_project_tools, which requires tools to already exist.
        const pending = pendingGitignoreRef.current;
        if (pending) {
          pendingGitignoreRef.current = null;
          try {
            await invoke("update_project_gitignore", {
              projectId: pending.projectId,
              addToGitignore: pending.addToGitignore,
              addToExclude: pending.addToExclude,
            });
          } catch (gitErr) {
            toast.warning(
              gitErr instanceof Error ? gitErr.message : String(gitErr),
            );
          }
        }
      } catch (err) {
        toast.error(err instanceof Error ? err.message : String(err));
      }
    },
    [state],
  );

  const handleRemoveProject = useCallback(async () => {
    if (!state.removeTargetId) return;
    try {
      await state.removeProject(state.removeTargetId);
      state.setShowRemoveModal(false);
      state.setRemoveTargetId(null);
      toast.success(t("projects.removeConfirm"));
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }, [state, t]);

  const handlePromptRemove = useCallback(
    (id: string) => {
      state.setRemoveTargetId(id);
      state.setShowRemoveModal(true);
    },
    [state],
  );

  const handlePromptEdit = useCallback(
    (id: string) => {
      state.setEditTargetId(id);
      state.setShowEditModal(true);
    },
    [state],
  );

  const handleEditSave = useCallback(
    async (
      projectId: string,
      gitignoreOptions: { addToGitignore: boolean; addToExclude: boolean },
    ) => {
      try {
        await invoke("update_project_gitignore", {
          projectId,
          addToGitignore: gitignoreOptions.addToGitignore,
          addToExclude: gitignoreOptions.addToExclude,
        });
        state.setShowEditModal(false);
        state.setEditTargetId(null);
        toast.success(t("projects.configureProject"));
      } catch (err) {
        toast.error(err instanceof Error ? err.message : String(err));
      }
    },
    [state, t],
  );

  const handleResyncProject = useCallback(async () => {
    return await state.resyncProject();
  }, [state]);

  const handleResyncAll = useCallback(async () => {
    return await state.resyncAll();
  }, [state]);

  const handleToggleAssignment = useCallback(
    async (skillId: string, tool: string) => {
      try {
        await state.toggleAssignment(skillId, tool);
      } catch (err) {
        toast.error(err instanceof Error ? err.message : String(err));
      }
    },
    [state],
  );

  const handleBulkAssign = useCallback(
    async (skillId: string) => {
      try {
        await state.bulkAssign(skillId);
      } catch (err) {
        toast.error(err instanceof Error ? err.message : String(err));
      }
    },
    [state],
  );

  const handleConfigureToolsFromToolbar = useCallback(async () => {
    await state.loadToolStatus();
    state.setShowToolConfigModal(true);
  }, [state]);

  return (
    <div className="projects-page">
      {!state.projectsLoading &&
      !state.loadError &&
      state.projects.length === 0 ? (
        <div className="projects-empty-fullwidth">
          <FolderOpen size={48} className="projects-empty-icon" />
          <p className="projects-empty-title">{t("projects.emptyTitle")}</p>
          <p className="projects-empty-body">{t("projects.emptyBody")}</p>
          <button
            className="btn btn-primary"
            onClick={() => state.setShowAddModal(true)}
          >
            {t("projects.emptyAction")}
          </button>
        </div>
      ) : (
        <div className="projects-layout">
          <ProjectList
            projects={state.projects}
            selectedProjectId={state.selectedProjectId}
            loading={state.projectsLoading}
            loadError={state.loadError}
            onSelectProject={state.selectProject}
            onAddProject={() => state.setShowAddModal(true)}
            onEditProject={handlePromptEdit}
            onRemoveProject={handlePromptRemove}
            t={t}
          />
          <section className="matrix-panel">
            {!state.selectedProjectId ? (
              <div className="matrix-placeholder">
                {t("projects.selectProject")}
              </div>
            ) : (
              <div className="matrix-content">
                <AssignmentMatrix
                  project={
                    state.projects.find(
                      (p) => p.id === state.selectedProjectId,
                    ) ?? null
                  }
                  tools={state.tools}
                  assignments={state.assignments}
                  skills={state.skills}
                  pendingCells={state.pendingCells}
                  matrixLoading={state.matrixLoading}
                  onToggleAssignment={handleToggleAssignment}
                  onBulkAssign={handleBulkAssign}
                  onResyncProject={handleResyncProject}
                  onResyncAll={handleResyncAll}
                  onConfigureTools={handleConfigureToolsFromToolbar}
                  t={t}
                />
              </div>
            )}
          </section>
        </div>
      )}

      <AddProjectModal
        open={state.showAddModal}
        loading={false}
        projects={state.projects}
        onRegister={handleAddProject}
        onRequestClose={() => state.setShowAddModal(false)}
        t={t}
      />

      <EditProjectModal
        open={state.showEditModal}
        project={
          state.projects.find((p) => p.id === state.editTargetId) ?? null
        }
        onSave={handleEditSave}
        onRequestClose={() => {
          state.setShowEditModal(false);
          state.setEditTargetId(null);
        }}
        t={t}
      />

      <ToolConfigModal
        open={state.showToolConfigModal}
        loading={false}
        toolStatus={state.toolStatus}
        currentTools={state.tools}
        onConfirm={handleToolConfigConfirm}
        onRequestClose={() => state.setShowToolConfigModal(false)}
        t={t}
      />

      <RemoveProjectModal
        open={state.showRemoveModal}
        loading={false}
        projectName={
          state.projects.find((p) => p.id === state.removeTargetId)?.name ??
          null
        }
        onConfirm={handleRemoveProject}
        onRequestClose={() => {
          state.setShowRemoveModal(false);
          state.setRemoveTargetId(null);
        }}
        t={t}
      />
    </div>
  );
};

export default memo(ProjectsPage);
