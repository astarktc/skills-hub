import { memo, useCallback, useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { FolderOpen } from "lucide-react";
import { pickFolder } from "../../lib/platform";
import {
  projectRemovalOutcome,
  projectSyncOutcome,
  removalOutcome,
  type Outcome,
  type PlainEntry,
} from "../../lib/reportOutcome";
import type { ProjectState } from "./useProjectState";
import ProjectList from "./ProjectList";
import AssignmentMatrix from "./AssignmentMatrix";
import AddProjectModal from "./AddProjectModal";
import EditProjectModal from "./EditProjectModal";
import ToolConfigModal from "../shared/ToolConfigModal";
import RemoveProjectModal from "./RemoveProjectModal";
import type { IgnoreUpdateOptions } from "./types";
import type { ManagedSkill } from "../skills/types";
import type { ToolCatalog } from "../../hooks/useToolCatalog";
import type {
  ActionErrorEntry,
  NotifyErrorFn,
  NotifyFn,
} from "../../hooks/useStatusReporter";
import {
  useFocusReconciliation,
  type SurfaceState,
} from "../../hooks/useSurfaceState";

const PROJECT_DIALOGS = ["projectAdd", "projectEdit", "projectToolConfig", "projectRemove"] as const;

type ProjectsPageProps = {
  /**
   * The project world (`useProjectState`), composed by the binder so the
   * selected project and its matrix outlive this page.
   */
  state: ProjectState;
  /** The matrix's skill rows: the library's catalog, passed by the binder. */
  skills: ManagedSkill[];
  /**
   * The Tool catalog: labels (removal toasts name tools even before the
   * project tool status has loaded, round 14 D4) and the project tool
   * entries the tool-config dialog offers.
   */
  catalog: Pick<
    ToolCatalog,
    "toolLabelById" | "projectToolStatus" | "loadProjectToolStatus"
  >;
  /** The reporter's notification entry point, handed down by the binder. */
  notify: NotifyFn;
  /** The reporter's command-failure entry point: `notifyError(err)`. */
  notifyError: NotifyErrorFn;
  /** The reporter's per-target failure batch (one toast per kept artifact). */
  showActionErrors: (errors: ActionErrorEntry[]) => void;
  /**
   * The surface module: the focused project (`#/projects/<id>`) and this
   * world's dialogs.
   */
  surface: Pick<
    SurfaceState,
    | "route"
    | "selection"
    | "focused"
    | "focus"
    | "reconcile"
    | "layer"
    | "openDialog"
    | "closeDialog"
    | "dismissDialog"
  >;
};

const ProjectsPage = ({
  state,
  skills,
  catalog,
  notify,
  notifyError,
  showActionErrors,
  surface,
}: ProjectsPageProps) => {
  const { t } = useTranslation();
  const { toolLabelById, projectToolStatus, loadProjectToolStatus } = catalog;
  const { focus, openDialog, closeDialog, dismissDialog } = surface;

  // The focused project is the surface's (hash-addressable); this world
  // loads the matrix for it. An id that is not (or no longer) a project
  // clears itself once the list has loaded.
  const focusedProjectId =
    surface.focused?.kind === "project" ? surface.focused.id : null;
  const projectIds = useMemo(
    () =>
      state.projectsLoading || state.loadFailed
        ? null
        : state.projects.map((project) => project.id),
    [state.loadFailed, state.projects, state.projectsLoading],
  );
  useFocusReconciliation(surface, "project", projectIds);
  const { selectProject, selectedProjectId } = state;
  useEffect(() => {
    if (!focusedProjectId || focusedProjectId === selectedProjectId) return;
    if (!projectIds?.includes(focusedProjectId)) return;
    selectProject(focusedProjectId).catch(notifyError);
  }, [focusedProjectId, notifyError, projectIds, selectProject, selectedProjectId]);

  // This world's dialogs render only on this page: leaving it withdraws them.
  useEffect(
    () => () => {
      for (const kind of PROJECT_DIALOGS) dismissDialog(kind);
    },
    [dismissDialog],
  );
  const handleSelectProject = useCallback(
    (id: string) => focus({ kind: "project", id }),
    [focus],
  );

  // Per-target outcomes (removal under ADR-0002, project sync) are report
  // data: the fold decides the entries and the toast; this only shows them
  // and reports whether the modal may close.
  const applyOutcome = useCallback(
    (outcome: Outcome<PlainEntry>) => {
      showActionErrors(outcome.errors);
      if (outcome.toast)
        notify(outcome.toast.kind, outcome.toast.message, outcome.toast.detail);
      return outcome.completion;
    },
    [notify, showActionErrors],
  );

  const handleAddProject = useCallback(
    async (path: string, gitignore: IgnoreUpdateOptions) => {
      try {
        // The ignore intent rides along with registration; the hook hands
        // it to the backend once the tool set is confirmed.
        const project = await state.registerProject(path, gitignore);
        focus({ kind: "project", id: project.id });
        await state.selectProject(project.id);
        openDialog({ kind: "projectToolConfig" });
        await loadProjectToolStatus();
      } catch (err) {
        notifyError(err);
      }
    },
    [focus, loadProjectToolStatus, notifyError, openDialog, state],
  );

  const handleToolConfigConfirm = useCallback(
    async (selectedTools: string[]) => {
      try {
        const report = await state.configureTools(selectedTools);
        if (!report) return;
        const completion = applyOutcome(
          projectRemovalOutcome(report, { t, toolLabelById, action: "configureTools" }),
        );
        if (completion.closeModal) dismissDialog("projectToolConfig");
      } catch (err) {
        notifyError(err);
      }
    },
    [applyOutcome, dismissDialog, notifyError, state, t, toolLabelById],
  );

  const editDialog = surface.layer("projectEdit");
  const removeDialog = surface.layer("projectRemove");

  const handleRemoveProject = useCallback(async () => {
    if (!removeDialog) return;
    try {
      const report = await state.removeProject(removeDialog.projectId);
      const completion = applyOutcome(
        projectRemovalOutcome(report, { t, toolLabelById, action: "removeProject" }),
      );
      if (completion.closeModal) dismissDialog("projectRemove");
    } catch (err) {
      notifyError(err);
    }
  }, [applyOutcome, dismissDialog, notifyError, removeDialog, state, t, toolLabelById]);

  const handlePromptRemove = useCallback(
    (id: string) => {
      openDialog({ kind: "projectRemove", projectId: id });
    },
    [openDialog],
  );

  const handlePromptEdit = useCallback(
    (id: string) => {
      openDialog({ kind: "projectEdit", projectId: id });
    },
    [openDialog],
  );

  const handleEditSave = useCallback(
    async (projectId: string, gitignore: IgnoreUpdateOptions) => {
      try {
        await state.updateGitignore(projectId, gitignore);
        dismissDialog("projectEdit");
        notify("success", t("projects.configureProject"));
      } catch (err) {
        notifyError(err);
      }
    },
    [dismissDialog, notify, notifyError, state, t],
  );

  const handleResyncProject = useCallback(async () => {
    try {
      const report = await state.resyncProject();
      applyOutcome(projectSyncOutcome(report, { t, toolLabelById, action: "resync" }));
    } catch (err) {
      notifyError(err);
    }
  }, [applyOutcome, notifyError, state, t, toolLabelById]);

  const handleResyncAll = useCallback(async () => {
    try {
      const { report, viewRefreshed } = await state.resyncAll();
      const projectLabelById = Object.fromEntries(
        state.projects.map((project) => [project.id, project.name]),
      );
      applyOutcome(
        projectSyncOutcome(report, { t, toolLabelById, projectLabelById, action: "resyncAll" }),
      );
      if (!viewRefreshed) notify("warning", t("projects.viewRefreshFailed"));
    } catch (err) {
      notifyError(err);
    }
  }, [applyOutcome, notify, notifyError, state, t, toolLabelById]);

  const handleToggleAssignment = useCallback(
    async (skillId: string, tool: string) => {
      try {
        const result = await state.toggleAssignment(skillId, tool);
        if (!result) return;
        applyOutcome(
          result.kind === "assigned"
            ? projectSyncOutcome(result.report, { t, toolLabelById, action: "toggleOn" })
            : removalOutcome(result.report, { t, toolLabelById, action: "toggle" }),
        );
      } catch (err) {
        notifyError(err);
      }
    },
    [applyOutcome, notifyError, state, t, toolLabelById],
  );

  const handleBulkAssign = useCallback(
    async (skillId: string) => {
      try {
        const report = await state.bulkAssign(skillId);
        if (report)
          applyOutcome(projectSyncOutcome(report, { t, toolLabelById, action: "bulkAssign" }));
      } catch (err) {
        notifyError(err);
      }
    },
    [applyOutcome, notifyError, state, t, toolLabelById],
  );

  // No confirmation, symmetric with bulk assign: a removed link is
  // re-assignable, not data loss (round 16 D2).
  const handleBulkUnassign = useCallback(
    async (skillId: string) => {
      try {
        const report = await state.bulkUnassign(skillId);
        if (report)
          applyOutcome(removalOutcome(report, { t, toolLabelById, action: "bulkUnassign" }));
      } catch (err) {
        notifyError(err);
      }
    },
    [applyOutcome, notifyError, state, t, toolLabelById],
  );

  const handleConfigureToolsFromToolbar = useCallback(async () => {
    try {
      await loadProjectToolStatus();
      openDialog({ kind: "projectToolConfig" });
    } catch (err) {
      notifyError(err);
    }
  }, [loadProjectToolStatus, notifyError, openDialog]);

  /** Add project's Browse: the picked folder, or null (cancelled / failed). */
  const handlePickProjectFolder = useCallback(async () => {
    try {
      const [picked] = await pickFolder({ title: t("projects.pathLabel") });
      return picked ?? null;
    } catch (err) {
      notifyError(err);
      return null;
    }
  }, [notifyError, t]);

  const handleUpdatePath = useCallback(
    async (projectId: string) => {
      try {
        const [newPath] = await pickFolder({ title: t("projects.updatePath") });
        if (!newPath) return;
        await state.updateProjectPath(projectId, newPath);
        notify("success", t("projects.updatePathSuccess"));
      } catch (err) {
        notifyError(err);
      }
    },
    [notify, notifyError, state, t],
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
            onClick={() => openDialog({ kind: "projectAdd" })}
          >
            {t("projects.emptyAction")}
          </button>
        </div>
      ) : (
        <div className="projects-layout">
          <ProjectList
            projects={state.projects}
            selectedProjectId={focusedProjectId}
            loading={state.projectsLoading}
            loadFailed={state.loadFailed}
            onSelectProject={handleSelectProject}
            onAddProject={() => openDialog({ kind: "projectAdd" })}
            onEditProject={handlePromptEdit}
            onRemoveProject={handlePromptRemove}
            onUpdatePath={handleUpdatePath}
            t={t}
          />
          <section className="matrix-panel">
            {!focusedProjectId || !state.selectedProjectId ? (
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
                  assignmentsReconciled={state.assignmentsReconciled}
                  skills={skills}
                  pendingCells={state.pendingCells}
                  matrixLoading={state.matrixLoading}
                  onToggleAssignment={handleToggleAssignment}
                  onBulkAssign={handleBulkAssign}
                  onBulkUnassign={handleBulkUnassign}
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
        open={surface.layer("projectAdd") !== null}
        loading={false}
        projects={state.projects}
        onPickFolder={handlePickProjectFolder}
        onRegister={handleAddProject}
        onRequestClose={() => closeDialog("projectAdd")}
        t={t}
      />

      <EditProjectModal
        open={editDialog !== null}
        project={state.projects.find((p) => p.id === editDialog?.projectId) ?? null}
        onSave={handleEditSave}
        loadStatus={state.getGitignoreStatus}
        onRequestClose={() => closeDialog("projectEdit")}
        t={t}
      />

      <ToolConfigModal
        open={surface.layer("projectToolConfig") !== null}
        loading={false}
        toolStatus={projectToolStatus}
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
          closeDialog("projectToolConfig");
        }}
        t={t}
      />

      <RemoveProjectModal
        open={removeDialog !== null}
        loading={false}
        projectName={
          state.projects.find((p) => p.id === removeDialog?.projectId)?.name ?? null
        }
        onConfirm={handleRemoveProject}
        onRequestClose={() => closeDialog("projectRemove")}
        t={t}
      />
    </div>
  );
};

export default memo(ProjectsPage);
