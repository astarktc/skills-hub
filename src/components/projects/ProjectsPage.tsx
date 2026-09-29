import { memo, useCallback, useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { FolderOpen } from "lucide-react";
import type { PendingAction } from "../../lib/actionLifecycle";
import type { ProjectState } from "./useProjectState";
import { useProjectActions } from "./useProjectActions";
import ProjectList from "./ProjectList";
import AssignmentMatrix from "./AssignmentMatrix";
import AddProjectModal from "./AddProjectModal";
import EditProjectModal from "./EditProjectModal";
import ToolConfigModal from "../shared/ToolConfigModal";
import RemoveProjectModal from "./RemoveProjectModal";
import type { ManagedSkill } from "../skills/types";
import type { ToolCatalog } from "../../hooks/useToolCatalog";
import type {
  ApplyOutcomeFn,
  NotifyErrorFn,
  NotifyFn,
  StatusReporter,
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
  /**
   * The reporter's action lifecycle: matrix mutations run one at a time
   * (D19), a cell or row pends inline (`pending`), a resync is a batch.
   */
  runAction: StatusReporter["runAction"];
  /** The action running now; the matrix shows its cells pending. */
  pending: PendingAction | null;
  /** The reporter's Outcome executor: every project report settles through it. */
  applyOutcome: ApplyOutcomeFn;
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
  runAction,
  pending,
  applyOutcome,
  surface,
}: ProjectsPageProps) => {
  const { t } = useTranslation();
  const { projectToolStatus } = catalog;
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

  // Every mutation runs under the reporter's lifecycle (one at a time, a
  // cell or row pending inline, the rest as a batch) and settles its report
  // through `applyOutcome`; while one runs, every other mutation control is
  // disabled with `blockedReason` as its title.
  const {
    pendingCells,
    blockedReason,
    handleAddProject,
    handleToolConfigConfirm,
    handleRemoveProject,
    handleEditSave,
    handleUpdatePath,
    handleResyncProject,
    handleResyncAll,
    handleToggleAssignment,
    handleBulkAssign,
    handleBulkUnassign,
    handleConfigureToolsFromToolbar,
    handlePickProjectFolder,
  } = useProjectActions({
    t,
    state,
    catalog,
    reporter: { pending, runAction, applyOutcome, notify, notifyError },
    surface,
  });

  const editDialog = surface.layer("projectEdit");
  const removeDialog = surface.layer("projectRemove");

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
            blockedReason={blockedReason}
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
                  pendingCells={pendingCells}
                  blockedReason={blockedReason}
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
        loading={pending !== null}
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
        blockedReason={blockedReason}
        loadStatus={state.getGitignoreStatus}
        onRequestClose={() => closeDialog("projectEdit")}
        t={t}
      />

      <ToolConfigModal
        open={surface.layer("projectToolConfig") !== null}
        loading={pending !== null}
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
        loading={pending !== null}
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
