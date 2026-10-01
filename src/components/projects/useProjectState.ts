import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
// Backend calls go through the shared invokeTauri seam (src/lib/tauri.ts),
// same as every world hook — one mock point for tests.
import { invokeTauri } from "../../lib/tauri";
import type {
  ProjectDto,
  ProjectToolDto,
  ProjectSkillAssignmentDto,
  ProjectViewDto,
  ProjectSyncReport,
  ResyncAllResultDto,
  GitignoreStatusDto,
  IgnoreUpdateOptions,
  RemovalReport,
} from "./types";

/**
 * What a toggle did, as the backend decided it under the mutation guard:
 * toggle-on answers with a batch-of-one sync report, toggle-off with the
 * removal report. The view the command returned is already applied.
 */
export type ToggleResult =
  | { kind: "assigned"; report: ProjectSyncReport }
  | { kind: "unassigned"; report: RemovalReport };

export type ProjectState = {
  // Data
  projects: ProjectDto[];
  selectedProjectId: string | null;
  tools: ProjectToolDto[];
  assignments: ProjectSkillAssignmentDto[];
  /**
   * Whether the last applied view's reconcile pass actually ran. The
   * backend skips it (rather than queue) while a Sync-target mutation is in
   * flight, so `false` means the rows shown are stored, not re-derived from
   * disk — never treat it as "healthy".
   */
  assignmentsReconciled: boolean;
  // The matrix's skill rows are the library's catalog and the Tool list is
  // the Tool catalog's — both passed to the view by the binder, never
  // loaded here (one skill catalog, one Tool catalog).
  // Loading
  projectsLoading: boolean;
  matrixLoading: boolean;
  // Cells in flight are the reporter's pending subject (`{ kind: "cell" }`),
  // run by the view one mutation at a time (D19) — not state of this world.
  // Errors
  loadFailed: boolean;
  // Modal state lives in the surface module (`projectAdd`, `projectEdit`,
  // `projectToolConfig`, `projectRemove` — the union this world introduced).
  // Actions
  loadProjects: () => Promise<void>;
  selectProject: (id: string) => Promise<void>;
  /**
   * Register a project and remember its ignore intent. The intent is not
   * applied here — ignore patterns derive from the project's persisted
   * tools, which don't exist yet — but handed to the backend by the next
   * `configureTools` for this project, which sequences both writes.
   */
  registerProject: (
    path: string,
    gitignore: IgnoreUpdateOptions,
  ) => Promise<ProjectDto>;
  /**
   * Remove a project and its artifacts. Resolves to the per-target removal
   * report (folded by the caller, `projectRemovalOutcome`); a project whose
   * artifact stayed is still listed (ADR-0002), and its matrix converges on
   * the backend's settled rows.
   */
  removeProject: (id: string) => Promise<RemovalReport>;
  /**
   * The mutations below apply the view their command returned and resolve
   * to its report, unread — the caller folds it. A thrown command converges
   * on the backend's view (`refreshView`) before rethrowing. `null` /
   * `undefined`: nothing was sent (no project selected, cell already pending).
   */
  toggleAssignment: (skillId: string, tool: string) => Promise<ToggleResult | null>;
  /**
   * A matrix cell's **Sync this**: re-sync one assignment of the selected
   * project (a stale copy, a missing artifact, a failed sync). Applies the
   * returned view; a folder in the way is the one overwrite ask's. Null:
   * nothing was sent (no project selected).
   */
  syncAssignment: (skillId: string, tool: string) => Promise<ProjectSyncReport | null>;
  /**
   * Assign skills to every Tool of any project — the project-assignment
   * verb other surfaces call (the skill action door, the inspector). The
   * project need not be the selected one: its row updates, and its matrix
   * too when it is shown. The backend command is per skill, so the skills
   * are assigned one after another and their reports merged; their
   * collisions are then **one** overwrite ask. Resolves to the merged
   * report, unread; a thrown command converges on the backend's view
   * before rethrowing.
   */
  assignToProject: (projectId: string, skillIds: readonly string[]) => Promise<ProjectSyncReport>;
  /** Inverse of `assignToProject`: skills off every Tool of a project, one merged report. */
  unassignFromProject: (projectId: string, skillIds: readonly string[]) => Promise<RemovalReport>;
  /**
   * Re-read one project's view (its row, and its matrix when shown) — for a
   * caller whose mutation answered no view (a collision retry another world
   * ran). Resolves to whether the view was applied.
   */
  refreshView: (projectId: string) => Promise<boolean>;
  /**
   * Every project's view, as the world holds it for surfaces outside the
   * matrix (the inspector's Projects section, the Assign picker): read
   * explicitly by `loadPlacementViews`, then kept current by every view a
   * mutation returns (`applyView`) — an assignment settles here with no
   * follow-up read (I5). Only a collision retry or a failure re-reads.
   */
  placementViews: PlacementViews;
  /**
   * The explicit read of every project's view (a surface opening, or a
   * skills-world mutation that answers no project view). A view a mutation
   * applied while the read was in flight is kept, not overwritten.
   */
  loadPlacementViews: () => Promise<void>;
  /** Where one skill is assigned, from `placementViews` (pure; no read). */
  placementOf: (skillId: string) => SkillPlacementValue;
  resyncProject: () => Promise<ProjectSyncReport>;
  updateProjectPath: (
    projectId: string,
    newPath: string,
  ) => Promise<ProjectDto>;
  /**
   * Re-sync every project. Answers the spanning report plus whether the
   * selected project's matrix was re-read afterwards (`viewRefreshed: false`
   * means the matrix shown may be stale).
   */
  resyncAll: () => Promise<{ report: ProjectSyncReport; viewRefreshed: boolean }>;
  /**
   * Make `tools` the selected project's tool set (one backend command).
   * A pending ignore intent rides along and is consumed only on success, so
   * a retry after a failure replays it. Resolves to the merged removal
   * report for the dropped tools (empty when nothing was dropped), or
   * `undefined` when no project is selected.
   */
  configureTools: (tools: string[]) => Promise<RemovalReport | undefined>;
  /** Abandon a pending ignore intent (tool-config modal dismissed). */
  discardPendingIgnore: () => void;
  getGitignoreStatus: (projectId: string) => Promise<GitignoreStatusDto>;
  updateGitignore: (
    projectId: string,
    options: IgnoreUpdateOptions,
  ) => Promise<void>;
};

/** One project as seen from one skill: its configured Tools and the skill's rows there. */
export type SkillPlacement = {
  project: ProjectDto;
  tools: ProjectToolDto[];
  assignments: ProjectSkillAssignmentDto[];
};

/**
 * Every project's view the world holds, by project id, and how the last read
 * went. `loaded` says a full read has landed at least once: a later failed
 * re-read keeps those last-good views (`status: "failed"` beside them), so a
 * surface shows what it knows with a read-error note rather than replacing
 * confirmed assignments with an error.
 */
export type PlacementViews = {
  status: "idle" | "loading" | "ready" | "failed";
  loaded: boolean;
  views: Readonly<Record<string, ProjectViewDto>>;
};

/**
 * One skill across projects: `rows` null until a read landed (a never-loaded
 * failure invents no assignments); `failed` when the last read failed —
 * with `rows` still present, they are the last-good state.
 */
export type SkillPlacementValue = { rows: SkillPlacement[] | null; failed: boolean };

const NO_PLACEMENT_VIEWS: PlacementViews = { status: "idle", loaded: false, views: {} };

/**
 * One skill across the projects listed, from the views held: each project
 * (its row as its view last said) with its configured Tools and that skill's
 * assignment rows. Pure.
 */
export function skillPlacement(
  projects: readonly ProjectDto[],
  placement: PlacementViews,
  skillId: string,
): SkillPlacementValue {
  const failed = placement.status === "failed";
  if (!placement.loaded) return { rows: null, failed };
  return {
    rows: projects.map((project) => {
      const view = placement.views[project.id];
      return {
        project: view?.project ?? project,
        tools: view?.tools ?? [],
        assignments: view?.assignments.filter((row) => row.skill_id === skillId) ?? [],
      };
    }),
    failed,
  };
}

/**
 * The selected project together with the matrix shown for it. One value so
 * that "is this view for the selected project?" is answered inside a pure
 * updater against the selection it will update — never by a side effect in
 * another state's updater (which React invokes twice under StrictMode).
 */
type Selection = {
  projectId: string | null;
  tools: ProjectToolDto[];
  assignments: ProjectSkillAssignmentDto[];
  reconciled: boolean;
};

const NO_SELECTION: Selection = {
  projectId: null,
  tools: [],
  assignments: [],
  reconciled: true,
};

/** What the project world needs from outside it (App passes it in). */
export type ProjectStateDeps = {
  /**
   * The one overwrite ask for a project report (the sync world's
   * `settleProjectCollisions`): its `TARGET_EXISTS` assignments are asked
   * and resolved; `resolved` says a retry ran (and answered no view).
   */
  settleCollisions?: (
    report: ProjectSyncReport,
  ) => Promise<{ report: ProjectSyncReport; resolved: boolean }>;
};

const UNSETTLED = async (report: ProjectSyncReport) => ({ report, resolved: false });

export function useProjectState(deps: ProjectStateDeps = {}): ProjectState {
  const settleCollisions = deps.settleCollisions ?? UNSETTLED;
  // Data state
  const [projects, setProjects] = useState<ProjectDto[]>([]);
  const [selection, setSelection] = useState<Selection>(NO_SELECTION);
  const {
    projectId: selectedProjectId,
    tools,
    assignments,
    reconciled: assignmentsReconciled,
  } = selection;

  // Loading state
  const [projectsLoading, setProjectsLoading] = useState(true);
  const [matrixLoading, setMatrixLoading] = useState(false);

  // Error state
  const [loadFailed, setLoadFailed] = useState(false);

  // Ignore intent captured by registerProject, consumed by the next
  // configureTools for that project (and dropped by any other registration
  // or configuration in between).
  const [pendingIgnore, setPendingIgnore] = useState<{
    projectId: string;
    options: IgnoreUpdateOptions;
  } | null>(null);

  // Version counter for stale result discard on project selection.
  const selectVersionRef = useRef(0);

  // Every project's view for surfaces outside the matrix. A view applied
  // after a read started (stamped) outranks what that read brings back.
  const [placementViews, setPlacementViews] = useState<PlacementViews>(NO_PLACEMENT_VIEWS);
  const viewStampRef = useRef({ next: 0, byProject: new Map<string, number>() });
  const placementReadRef = useRef(0);

  /**
   * Apply one backend view: the project row replaces its entry in the list,
   * and the matrix takes the view's tools and assignments wholesale. Every
   * mutation returns a view, so nothing here needs a follow-up read — and a
   * view for a project the operator has since navigated away from only
   * updates that project's row.
   */
  const applyView = useCallback((view: ProjectViewDto) => {
    const stamps = viewStampRef.current;
    stamps.byProject.set(view.project.id, ++stamps.next);
    setPlacementViews((current) => ({
      ...current,
      views: { ...current.views, [view.project.id]: view },
    }));
    setProjects((prev) => {
      const index = prev.findIndex((p) => p.id === view.project.id);
      if (index === -1) return [view.project, ...prev];
      const next = [...prev];
      next[index] = view.project;
      return next;
    });
    setSelection((current) =>
      current.projectId === view.project.id
        ? {
            projectId: current.projectId,
            tools: view.tools,
            assignments: view.assignments,
            reconciled: view.reconciled,
          }
        : current,
    );
  }, []);

  /**
   * Re-read a project's view so what is shown matches what the backend
   * settled: the error-path convergence after a failed mutation, and the one
   * success path whose mutation answers with no view (`resyncAll`). Returns
   * whether the view was applied — a caller on a success path reports a
   * `false` instead of letting a stale matrix pass for current.
   */
  const refreshView = useCallback(
    async (projectId: string): Promise<boolean> => {
      try {
        applyView(await invokeTauri("getProjectView", projectId));
        return true;
      } catch {
        // The mutation's own outcome is what the caller reports; the read
        // failure is a stale marker, not a second error.
        return false;
      }
    },
    [applyView],
  );

  const loadProjects = useCallback(async () => {
    setProjectsLoading(true);
    setLoadFailed(false);
    try {
      const result = await invokeTauri("listProjects");
      setProjects(result);
    } catch {
      setLoadFailed(true);
    } finally {
      setProjectsLoading(false);
    }
  }, []);

  // Load the project list on mount — the binder composes this world at
  // startup, so it lives as long as the app and survives navigation.
  // Awaited inside an IIFE so the loader's setState runs in an async
  // continuation (satisfies react-hooks/set-state-in-effect).
  useEffect(() => {
    void (async () => {
      await loadProjects();
    })();
  }, [loadProjects]);

  const selectProject = useCallback(
    async (id: string) => {
      // The previous matrix stays up under the loading flag until the view
      // lands; `applyView` matches it by this id.
      setSelection((current) => ({ ...current, projectId: id }));
      setMatrixLoading(true);
      const version = ++selectVersionRef.current;
      try {
        const view = await invokeTauri("getProjectView", id);
        // `applyView` ignores a view for a project no longer selected; the
        // version guard covers the loading flag and the failure path.
        if (selectVersionRef.current !== version) return;
        applyView(view);
      } catch (err) {
        if (selectVersionRef.current !== version) return;
        setSelection((current) => ({ ...current, tools: [], assignments: [] }));
        throw err;
      } finally {
        if (selectVersionRef.current === version) {
          setMatrixLoading(false);
        }
      }
    },
    [applyView],
  );

  const registerProject = useCallback(
    async (
      path: string,
      gitignore: IgnoreUpdateOptions,
    ): Promise<ProjectDto> => {
      const view = await invokeTauri("registerProject", path);
      setPendingIgnore(
        gitignore.add_to_gitignore || gitignore.add_to_exclude
          ? { projectId: view.project.id, options: gitignore }
          : null,
      );
      applyView(view);
      return view.project;
    },
    [applyView],
  );

  const removeProject = useCallback(
    async (id: string): Promise<RemovalReport> => {
      let remaining: ProjectDto[];
      let report: RemovalReport;
      try {
        // The mutation's view is the project list after the removal.
        ({ projects: remaining, report } = await invokeTauri(
          "removeProject",
          id,
        ));
      } catch (err) {
        // A thrown error is a store failure, not a per-target one; the
        // rows may still have settled, so converge on the backend's view
        // before surfacing it.
        await refreshView(id);
        throw err;
      }
      setProjects(remaining);
      if (remaining.some((p) => p.id === id)) {
        // Artifact removal kept a row whose artifact stayed on disk with
        // status `error` (ADR-0002) and the project stays registered. The
        // response carries no matrix, so this is the failure-path read.
        await refreshView(id);
      } else {
        setSelection((current) =>
          current.projectId === id ? NO_SELECTION : current,
        );
        setPlacementViews((current) => {
          if (!(id in current.views)) return current;
          const views = { ...current.views };
          delete views[id];
          return { ...current, views };
        });
      }
      return report;
    },
    [refreshView],
  );

  const toggleAssignment = useCallback(
    async (skillId: string, tool: string): Promise<ToggleResult | null> => {
      if (!selectedProjectId) return null;
      try {
        // Add-vs-remove is the backend's decision, read from its own rows
        // under the mutation guard — the frontend mirrors nothing.
        const result = await invokeTauri(
          "toggleProjectSkillAssignment",
          selectedProjectId,
          skillId,
          tool,
        );
        applyView(result.view);
        if (result.kind === "unassigned") return { kind: "unassigned", report: result.report };
        // A folder in the way is the one overwrite ask's (D12); a retry
        // answers no view, so the matrix is re-read.
        const settled = await settleCollisions(result.report);
        if (settled.resolved) await refreshView(selectedProjectId);
        return { kind: "assigned", report: settled.report };
      } catch (err) {
        // A whole-command failure may follow settled rows, so converge on
        // the backend's view before surfacing the failure.
        await refreshView(selectedProjectId);
        throw err;
      }
    },
    [selectedProjectId, applyView, refreshView, settleCollisions],
  );

  const assignToProject = useCallback(
    async (projectId: string, skillIds: readonly string[]): Promise<ProjectSyncReport> => {
      try {
        // Every skill's report first, then one ask over all of them. A
        // project skipped whole (folder missing) answers per skill; the set
        // reports it once.
        const items: ProjectSyncReport["items"] = [];
        const skipped_projects: ProjectSyncReport["skipped_projects"] = [];
        for (const skillId of skillIds) {
          const result = await invokeTauri("bulkAssignSkill", projectId, skillId);
          applyView(result.view);
          items.push(...result.report.items);
          for (const skip of result.report.skipped_projects)
            if (!skipped_projects.some((s) => s.project_id === skip.project_id)) skipped_projects.push(skip);
        }
        const settled = await settleCollisions({ items, skipped_projects });
        if (settled.resolved) await refreshView(projectId);
        return settled.report;
      } catch (err) {
        await refreshView(projectId);
        throw err;
      }
    },
    [applyView, refreshView, settleCollisions],
  );

  const unassignFromProject = useCallback(
    async (projectId: string, skillIds: readonly string[]): Promise<RemovalReport> => {
      try {
        const reports: RemovalReport[] = [];
        for (const skillId of skillIds) {
          const result = await invokeTauri("bulkUnassignSkill", projectId, skillId);
          // A row whose artifact stayed is kept with status `error`
          // (ADR-0002); the view already shows it.
          applyView(result.view);
          reports.push(result.report);
        }
        return reports.length === 1
          ? reports[0]
          : {
              targets: reports.flatMap((report) => report.targets),
              central_removed: false,
              record_deleted: false,
              cleared_markers: reports.flatMap((report) => report.cleared_markers),
            };
      } catch (err) {
        await refreshView(projectId);
        throw err;
      }
    },
    [applyView, refreshView],
  );

  const syncAssignment = useCallback(
    async (skillId: string, tool: string): Promise<ProjectSyncReport | null> => {
      if (!selectedProjectId) return null;
      try {
        const result = await invokeTauri("resyncProjectAssignment", selectedProjectId, skillId, tool);
        applyView(result.view);
        const settled = await settleCollisions(result.report);
        if (settled.resolved) await refreshView(selectedProjectId);
        return settled.report;
      } catch (err) {
        await refreshView(selectedProjectId);
        throw err;
      }
    },
    [selectedProjectId, applyView, refreshView, settleCollisions],
  );

  const updateProjectPath = useCallback(
    async (projectId: string, newPath: string): Promise<ProjectDto> => {
      const view = await invokeTauri("updateProjectPath", projectId, newPath);
      applyView(view);
      return view.project;
    },
    [applyView],
  );

  // The read takes the project list as it is when called (a stable
  // function: surfaces key their read on what they show, never on it).
  // (A layout effect: a surface's passive effect may read it in the same commit.)
  const projectsRef = useRef(projects);
  useLayoutEffect(() => {
    projectsRef.current = projects;
  });
  const loadPlacementViews = useCallback(async (): Promise<void> => {
    const read = ++placementReadRef.current;
    const started = viewStampRef.current.next;
    // A re-read keeps what is shown until it lands (a failed one too — the
    // views stay, the note goes).
    setPlacementViews((current) => (current.status === "ready" ? current : { ...current, status: "loading" }));
    try {
      const views = await Promise.all(
        projectsRef.current.map((project) => invokeTauri("getProjectView", project.id)),
      );
      if (placementReadRef.current !== read) return;
      const stamps = viewStampRef.current.byProject;
      setPlacementViews((current) => {
        const next = { ...current.views };
        for (const view of views) {
          if ((stamps.get(view.project.id) ?? 0) <= started) next[view.project.id] = view;
        }
        return { status: "ready", loaded: true, views: next };
      });
    } catch {
      if (placementReadRef.current !== read) return;
      // Last-good views (and any a mutation applied meanwhile) stay.
      setPlacementViews((current) => ({ ...current, status: "failed" }));
    }
  }, []);
  const placementOf = useCallback(
    (skillId: string) => skillPlacement(projects, placementViews, skillId),
    [placementViews, projects],
  );

  const resyncProject = useCallback(async (): Promise<ProjectSyncReport> => {
    if (!selectedProjectId) throw new Error("No project selected");
    try {
      const result = await invokeTauri("resyncProject", selectedProjectId);
      applyView(result.view);
      const settled = await settleCollisions(result.report);
      if (settled.resolved) await refreshView(selectedProjectId);
      return settled.report;
    } catch (err) {
      await refreshView(selectedProjectId);
      throw err;
    }
  }, [selectedProjectId, applyView, refreshView, settleCollisions]);

  const resyncAll = useCallback(async (): Promise<{
    report: ProjectSyncReport;
    viewRefreshed: boolean;
  }> => {
    let result: ResyncAllResultDto;
    let report: ProjectSyncReport;
    try {
      result = await invokeTauri("resyncAllProjects");
      // Collisions across every project are one ask, settled before the
      // shown matrix is re-read below.
      ({ report } = await settleCollisions(result.report));
    } catch (err) {
      if (selectedProjectId) await refreshView(selectedProjectId);
      throw err;
    }
    setProjects(result.projects);
    // The batch touches every project and answers with no view; only the
    // shown one needs its matrix, so this read is the success path here.
    const viewRefreshed = selectedProjectId ? await refreshView(selectedProjectId) : true;
    return { report, viewRefreshed };
  }, [selectedProjectId, refreshView, settleCollisions]);

  const configureTools = useCallback(
    async (toolIds: string[]): Promise<RemovalReport | undefined> => {
      if (!selectedProjectId) return undefined;
      const gitignore =
        pendingIgnore?.projectId === selectedProjectId
          ? pendingIgnore.options
          : null;
      try {
        // The view already reflects the cascade of dropping a tool (its
        // assignments are gone, or kept with status `error` when the
        // artifact stayed), so there is nothing to re-read.
        const { view, report } = await invokeTauri(
          "configureProjectTools",
          selectedProjectId,
          toolIds,
          gitignore,
        );
        applyView(view);
        // Consume the intent only once the backend has written it. On
        // rejection the modal stays open, so keeping the intent lets a
        // retry replay it instead of silently persisting tools alone.
        setPendingIgnore(null);
        return report;
      } catch (err) {
        // The tools may have been persisted before the ignore write failed;
        // converge on the backend's view (silently — the caller sees `err`).
        await refreshView(selectedProjectId);
        throw err;
      }
    },
    [selectedProjectId, pendingIgnore, applyView, refreshView],
  );

  // Dismissing the tool-config modal abandons the registration flow that
  // captured the intent, so drop it: otherwise it would leak into a later
  // tool-config for the same project the operator never opted into.
  const discardPendingIgnore = useCallback(() => {
    setPendingIgnore(null);
  }, []);

  const getGitignoreStatus = useCallback(
    (projectId: string) =>
      invokeTauri("getProjectGitignoreStatus", projectId),
    [],
  );

  const updateGitignore = useCallback(
    async (projectId: string, options: IgnoreUpdateOptions) => {
      await invokeTauri("updateProjectGitignore", projectId, options);
    },
    [],
  );

  // One value per state change: the binder hands it to the view whole.
  return useMemo(
    () => ({
      projects,
      selectedProjectId,
      tools,
      assignments,
      assignmentsReconciled,
      projectsLoading,
      matrixLoading,
      loadFailed,
      loadProjects,
      selectProject,
      registerProject,
      removeProject,
      toggleAssignment,
      syncAssignment,
      assignToProject,
      unassignFromProject,
      refreshView,
      placementViews,
      loadPlacementViews,
      placementOf,
      resyncProject,
      updateProjectPath,
      resyncAll,
      configureTools,
      discardPendingIgnore,
      getGitignoreStatus,
      updateGitignore,
    }),
    [
      projects,
      selectedProjectId,
      tools,
      assignments,
      assignmentsReconciled,
      projectsLoading,
      matrixLoading,
      loadFailed,
      loadProjects,
      selectProject,
      registerProject,
      removeProject,
      toggleAssignment,
      syncAssignment,
      assignToProject,
      unassignFromProject,
      refreshView,
      placementViews,
      loadPlacementViews,
      placementOf,
      resyncProject,
      updateProjectPath,
      resyncAll,
      configureTools,
      discardPendingIgnore,
      getGitignoreStatus,
      updateGitignore,
    ],
  );
}
