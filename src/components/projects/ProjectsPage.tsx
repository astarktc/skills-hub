import { memo, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen } from "lucide-react";
import { toast } from "sonner";
import { useProjectState } from "./useProjectState";
import { describeCommandError } from "../../commandError";
import ProjectList from "./ProjectList";
import AssignmentMatrix from "./AssignmentMatrix";
import AddProjectModal from "./AddProjectModal";
import EditProjectModal from "./EditProjectModal";
import ToolConfigModal from "../shared/ToolConfigModal";
import RemoveProjectModal from "./RemoveProjectModal";

const ProjectsPage = () => {
  const { t } = useTranslation();
  const state = useProjectState();

  const handleAddProject = useCallback(
    async (
      path: string,
      gitignoreOptions: { addToGitignore: boolean; addToExclude: boolean },
    ) => {
      try {
        // The ignore intent rides along with registration; the hook hands
        // it to the backend once the tool set is confirmed.
        const project = await state.registerProject(path, {
          add_to_gitignore: gitignoreOptions.addToGitignore,
          add_to_exclude: gitignoreOptions.addToExclude,
        });
        state.setShowAddModal(false);
        await state.selectProject(project.id);
        state.setShowToolConfigModal(true);
        await state.loadToolStatus();
      } catch (err) {
        const msg = describeCommandError(err, t);
        if (msg) toast.error(msg);
      }
    },
    [state, t],
  );

  const handleToolConfigConfirm = useCallback(
    async (selectedTools: string[]) => {
      try {
        await state.configureTools(selectedTools);
        state.setShowToolConfigModal(false);
      } catch (err) {
        const msg = describeCommandError(err, t);
        if (msg) toast.error(msg);
      }
    },
    [state, t],
  );

  const handleRemoveProject = useCallback(async () => {
    if (!state.removeTargetId) return;
    try {
      await state.removeProject(state.removeTargetId);
      state.setShowRemoveModal(false);
      state.setRemoveTargetId(null);
      toast.success(t("projects.removeConfirm"));
    } catch (err) {
      const msg = describeCommandError(err, t);
      if (msg) toast.error(msg);
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
        await state.updateGitignore(projectId, {
          add_to_gitignore: gitignoreOptions.addToGitignore,
          add_to_exclude: gitignoreOptions.addToExclude,
        });
        state.setShowEditModal(false);
        state.setEditTargetId(null);
        toast.success(t("projects.configureProject"));
      } catch (err) {
        const msg = describeCommandError(err, t);
        if (msg) toast.error(msg);
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
        const msg = describeCommandError(err, t);
        if (msg) toast.error(msg);
      }
    },
    [state, t],
  );

  const handleBulkAssign = useCallback(
    async (skillId: string) => {
      try {
        const result = await state.bulkAssign(skillId);
        if (result && result.failed.length > 0) {
          const details = result.failed
            .map(
              (f) =>
                `${f.tool}: ${describeCommandError(f.error, t) ?? f.error.code}`,
            )
            .join(", ");
          toast.warning(
            t("projects.bulkAssignFailed", {
              details,
            }),
          );
        }
      } catch (err) {
        const msg = describeCommandError(err, t);
        if (msg) toast.error(msg);
      }
    },
    [state, t],
  );

  const handleConfigureToolsFromToolbar = useCallback(async () => {
    await state.loadToolStatus();
    state.setShowToolConfigModal(true);
  }, [state]);

  const handleUpdatePath = useCallback(
    async (projectId: string) => {
      try {
        const selected = await open({ directory: true, multiple: false });
        if (!selected) return;
        const newPath = typeof selected === "string" ? selected : selected[0];
        if (!newPath) return;
        await state.updateProjectPath(projectId, newPath);
        toast.success(t("projects.updatePathSuccess"));
      } catch (err) {
        const msg = describeCommandError(err, t);
        if (msg) toast.error(msg);
      }
    },
    [state, t],
  );

  return (
    <div className="projects-page">
      {!state.projectsLoading &&
      !state.loadFailed &&
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
            loadFailed={state.loadFailed}
            onSelectProject={state.selectProject}
            onAddProject={() => state.setShowAddModal(true)}
            onEditProject={handlePromptEdit}
            onRemoveProject={handlePromptRemove}
            onUpdatePath={handleUpdatePath}
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
        loadStatus={state.getGitignoreStatus}
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
        savedSelection={
          state.tools.length > 0 ? state.tools.map((ct) => ct.tool) : null
        }
        labels={{
          title: t("projects.toolConfigTitle"),
          description: t("projects.toolConfigDesc"),
          confirmLabel: t("projects.toolConfigConfirm"),
        }}
        onConfirm={handleToolConfigConfirm}
        onRequestClose={() => {
          state.discardPendingIgnore();
          state.setShowToolConfigModal(false);
        }}
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
