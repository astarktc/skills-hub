import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useRef, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { FolderOpen, Info } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { FocusFallback } from "@/components/ui/dialog";
import { isFocusLost, isPendingSkill, type PendingAction } from "../../lib/actionLifecycle";
import type { ProjectState } from "./useProjectState";
import { useProjectActions } from "./useProjectActions";
import ProjectList from "./ProjectList";
import ProjectHeader from "./ProjectHeader";
import AssignmentMatrix from "./AssignmentMatrix";
import AddProjectModal from "./AddProjectModal";
import EditProjectModal from "./EditProjectModal";
import ToolConfigModal from "../shared/ToolConfigModal";
import RemoveProjectModal from "./RemoveProjectModal";
import type { ManagedSkill } from "../skills/types";
import type { ToolCatalog } from "../../hooks/useToolCatalog";
import type { SkillActions } from "../../hooks/useSkillActions";
import { SHELL_WIDE_QUERY, useMediaQuery } from "../../hooks/useMediaQuery";
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
   * The Tool catalog: labels (column headers, removal toasts) and the
   * project tool entries (which Tools are detected; the tool-config dialog).
   */
  catalog: Pick<
    ToolCatalog,
    "toolLabelById" | "projectToolStatus" | "loadProjectToolStatus"
  >;
  /**
   * The skill action door: a row's bulk verbs are its `assignToProject` /
   * `unassignFromProject` — the same verbs the inspector and the bulk rail
   * run, so one rule assigns a skill to a project everywhere.
   */
  actions: Pick<SkillActions, "run">;
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
  /** The header progress for a dialog's foot (the page behind a modal is inert). */
  dialogProgress: ReactNode;
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

/** A removed project's opener is gone: focus the area's heading. */
const projectsHeadingFallback: FocusFallback = (usable) => {
  const heading = document.querySelector<HTMLElement>("[data-surface-heading='projects']");
  return heading && usable(heading) ? heading : null;
};

const ProjectsPage = ({
  state,
  skills,
  catalog,
  actions,
  notify,
  notifyError,
  runAction,
  pending,
  applyOutcome,
  dialogProgress,
  surface,
}: ProjectsPageProps) => {
  const { t } = useTranslation();
  const { projectToolStatus, loadProjectToolStatus, toolLabelById } = catalog;
  const { focus, openDialog, closeDialog, dismissDialog } = surface;
  // D23: below the shell breakpoint the list becomes the header's select.
  const wide = useMediaQuery(SHELL_WIDE_QUERY);

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

  // Column headers say which configured Tool detection does not see: the
  // project tool entries are read once when the area opens.
  useEffect(() => {
    if (projectToolStatus) return;
    loadProjectToolStatus().catch(() => {
      // Labels fall back to the global catalog's; "not detected" is withheld.
    });
  }, [loadProjectToolStatus, projectToolStatus]);

  // This world's dialogs render only on this page: leaving it withdraws them.
  useEffect(
    () => () => {
      for (const kind of PROJECT_DIALOGS) dismissDialog(kind);
    },
    [dismissDialog],
  );

  // A resize across the breakpoint swaps the list and the select: focus
  // that was inside the one that left moves to the one that came (never
  // `<body>`, D23).
  const pageRef = useRef<HTMLDivElement>(null);
  const previousWide = useRef(wide);
  useLayoutEffect(() => {
    if (previousWide.current === wide) return;
    previousWide.current = wide;
    if (!isFocusLost(document.activeElement)) return;
    const target = wide
      ? pageRef.current?.querySelector<HTMLElement>("[data-project-select][aria-current='true']") ??
        pageRef.current?.querySelector<HTMLElement>("[data-surface-heading='projects']")
      : pageRef.current?.querySelector<HTMLElement>("[data-project-header-select]");
    target?.focus();
  }, [wide]);

  // "Refresh view" goes with its line once the view is reconciled: the
  // focus it held moves to the project's heading (D21 — never <body>).
  const refreshFocused = useRef(false);
  const reconciled = state.assignmentsReconciled;
  useLayoutEffect(() => {
    if (!reconciled || !refreshFocused.current) return;
    refreshFocused.current = false;
    pageRef.current?.querySelector<HTMLElement>("[data-project-heading]")?.focus();
  }, [reconciled]);

  const handleSelectProject = useCallback(
    (id: string) => focus({ kind: "project", id }),
    [focus],
  );

  // Every mutation runs under the reporter's lifecycle (one at a time, a
  // cell pending inline, the rest as a batch) and settles its report
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
    handleSyncCell,
    handleRefreshView,
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
    (id: string) => openDialog({ kind: "projectRemove", projectId: id }),
    [openDialog],
  );
  const handlePromptEdit = useCallback(
    (id: string) => openDialog({ kind: "projectEdit", projectId: id }),
    [openDialog],
  );
  const handleAddPrompt = useCallback(() => openDialog({ kind: "projectAdd" }), [openDialog]);

  // A row's bulk verbs are the door's (one rule with the inspector's Assign).
  const { run } = actions;
  const handleAssignAll = useCallback(
    (skill: ManagedSkill) => {
      if (selectedProjectId) void run("assignToProject", [skill], { projectId: selectedProjectId });
    },
    [run, selectedProjectId],
  );
  const handleUnassignAll = useCallback(
    (skill: ManagedSkill) => {
      if (selectedProjectId) void run("unassignFromProject", [skill], { projectId: selectedProjectId });
    },
    [run, selectedProjectId],
  );
  const isRowPending = useCallback((skillId: string) => isPendingSkill(pending, skillId), [pending]);
  const labelOf = useCallback((tool: string) => toolLabelById[tool] ?? tool, [toolLabelById]);

  const project = state.projects.find((p) => p.id === state.selectedProjectId) ?? null;
  const shownProject = focusedProjectId && project ? project : null;
  const lastSyncAt = useMemo(() => {
    let max = 0;
    for (const a of state.assignments) if (a.synced_at && a.synced_at > max) max = a.synced_at;
    return max > 0 ? max : null;
  }, [state.assignments]);
  const removing = pending !== null && removeDialog !== null && pending.subject.kind === "batch";

  const empty = !state.projectsLoading && !state.loadFailed && state.projects.length === 0;

  return (
    <div ref={pageRef} className="flex min-h-0 flex-1 overflow-hidden">
      {empty ? (
        <section className="flex flex-1 flex-col items-start gap-3 px-8 py-12">
          <FolderOpen aria-hidden="true" className="size-8 text-faint" />
          <h1 data-surface-heading="projects" tabIndex={-1} className="m-0 rounded-sm text-lg font-semibold text-foreground">
            {t("projects.emptyTitle")}
          </h1>
          <p className="m-0 max-w-prose text-sm text-muted-foreground">{t("projects.emptyBody")}</p>
          <Button variant="solid" tone="accent" onClick={handleAddPrompt}>
            {t("projects.emptyAction")}
          </Button>
        </section>
      ) : (
        <>
          {wide ? (
            <ProjectList
              projects={state.projects}
              selectedProjectId={focusedProjectId}
              loading={state.projectsLoading}
              loadFailed={state.loadFailed}
              onSelectProject={handleSelectProject}
              onAddProject={handleAddPrompt}
              onEditProject={handlePromptEdit}
              onRemoveProject={handlePromptRemove}
              onUpdatePath={handleUpdatePath}
              blockedReason={blockedReason}
              t={t}
            />
          ) : null}
          <section className="flex min-w-0 flex-1 flex-col">
            <ProjectHeader
              wide={wide}
              projects={state.projects}
              project={shownProject}
              lastSyncAt={lastSyncAt}
              hasTools={state.tools.length > 0}
              blockedReason={blockedReason}
              onSelectProject={handleSelectProject}
              onAddProject={handleAddPrompt}
              onResyncProject={() => void handleResyncProject()}
              onResyncAll={() => void handleResyncAll()}
              onConfigureTools={() => void handleConfigureToolsFromToolbar()}
              onEditProject={handlePromptEdit}
              onRemoveProject={handlePromptRemove}
              onUpdatePath={handleUpdatePath}
              t={t}
            />
            {!shownProject ? (
              wide ? (
                <p className="m-0 px-4 py-6 text-sm text-muted-foreground">{t("projects.selectProject")}</p>
              ) : null
            ) : (
              <>
                {!state.assignmentsReconciled && !state.matrixLoading ? (
                  <p role="status" className="m-0 flex shrink-0 items-center gap-2 border-b border-border px-4 py-1.5 text-xs text-muted-foreground">
                    <Info aria-hidden="true" className="size-3.5 shrink-0" />
                    {t("projectsSurface.header.reconcileSkipped")}
                    <span aria-hidden="true">{"\u2014"}</span>
                    <Button
                      variant="ghost"
                      tone="accent"
                      size="sm"
                      onFocus={() => {
                        refreshFocused.current = true;
                      }}
                      onBlur={(event) => {
                        if (event.currentTarget.isConnected) refreshFocused.current = false;
                      }}
                      onClick={() => void handleRefreshView()}
                    >
                      {t("projectsSurface.header.refreshView")}
                    </Button>
                  </p>
                ) : null}
                <AssignmentMatrix
                  key={shownProject.id}
                  project={shownProject}
                  tools={state.tools}
                  assignments={state.assignments}
                  skills={skills}
                  labelOf={labelOf}
                  installedTools={projectToolStatus?.installed ?? null}
                  pendingCells={pendingCells}
                  isRowPending={isRowPending}
                  blockedReason={blockedReason}
                  matrixLoading={state.matrixLoading}
                  onToggleAssignment={(skillId, tool) => void handleToggleAssignment(skillId, tool)}
                  onSyncCell={(skillId, tool) => void handleSyncCell(skillId, tool)}
                  onAssignAll={handleAssignAll}
                  onUnassignAll={handleUnassignAll}
                  onConfigureTools={() => void handleConfigureToolsFromToolbar()}
                  t={t}
                />
              </>
            )}
          </section>
        </>
      )}

      <AddProjectModal
        open={surface.layer("projectAdd") !== null}
        blockedReason={blockedReason}
        projects={state.projects}
        onPickFolder={handlePickProjectFolder}
        onRegister={handleAddProject}
        onRequestClose={() => closeDialog("projectAdd")}
        progress={dialogProgress}
        t={t}
      />

      <EditProjectModal
        open={editDialog !== null}
        project={state.projects.find((p) => p.id === editDialog?.projectId) ?? null}
        onSave={handleEditSave}
        blockedReason={blockedReason}
        loadStatus={state.getGitignoreStatus}
        onRequestClose={() => closeDialog("projectEdit")}
        progress={dialogProgress}
        t={t}
      />

      <ToolConfigModal
        open={surface.layer("projectToolConfig") !== null}
        loading={pending !== null}
        blockedReason={blockedReason}
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
        progress={dialogProgress}
        t={t}
      />

      <RemoveProjectModal
        open={removeDialog !== null}
        blockedReason={blockedReason}
        removing={removing}
        project={state.projects.find((p) => p.id === removeDialog?.projectId) ?? null}
        onConfirm={handleRemoveProject}
        onRequestClose={() => closeDialog("projectRemove")}
        fallbackFocus={projectsHeadingFallback}
        progress={dialogProgress}
        t={t}
      />
    </div>
  );
};

export default memo(ProjectsPage);
