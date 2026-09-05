import { useCallback, useEffect, useRef, useState } from "react";
// Backend calls go through the shared invokeTauri seam (src/lib/tauri.ts),
// same as every world hook — one mock point for tests.
import { invokeTauri } from "../../lib/tauri";
import type {
  ProjectDto,
  ProjectToolDto,
  ProjectSkillAssignmentDto,
  ProjectViewDto,
  ResyncSummaryDto,
  BulkAssignResultDto,
  GitignoreStatusDto,
  IgnoreUpdateOptions,
} from "./types";
import type { ManagedSkill, ToolStatusDto } from "../skills/types";

/**
 * Which modal the project world is showing, and what it is about. One value
 * instead of a flag-plus-target pair per modal: the two can never disagree,
 * and closing is one call.
 */
export type ProjectDialog =
  | { kind: "add" }
  | { kind: "edit"; projectId: string }
  | { kind: "toolConfig" }
  | { kind: "remove"; projectId: string };

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
  skills: ManagedSkill[];
  toolStatus: ToolStatusDto | null;
  // Loading
  projectsLoading: boolean;
  matrixLoading: boolean;
  pendingCells: Set<string>;
  // Errors
  loadFailed: boolean;
  // Modal state
  dialog: ProjectDialog | null;
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
  removeProject: (id: string) => Promise<void>;
  toggleAssignment: (skillId: string, tool: string) => Promise<void>;
  bulkAssign: (skillId: string) => Promise<BulkAssignResultDto | undefined>;
  resyncProject: () => Promise<ResyncSummaryDto>;
  updateProjectPath: (
    projectId: string,
    newPath: string,
  ) => Promise<ProjectDto>;
  resyncAll: () => Promise<ResyncSummaryDto[]>;
  loadToolStatus: () => Promise<void>;
  /**
   * Make `tools` the selected project's tool set (one backend command).
   * A pending ignore intent rides along and is consumed only on success, so
   * a retry after a failure replays it.
   */
  configureTools: (tools: string[]) => Promise<void>;
  /** Abandon a pending ignore intent (tool-config modal dismissed). */
  discardPendingIgnore: () => void;
  getGitignoreStatus: (projectId: string) => Promise<GitignoreStatusDto>;
  updateGitignore: (
    projectId: string,
    options: IgnoreUpdateOptions,
  ) => Promise<void>;
  openDialog: (dialog: ProjectDialog) => void;
  closeDialog: () => void;
};

export function useProjectState(): ProjectState {
  // Data state
  const [projects, setProjects] = useState<ProjectDto[]>([]);
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(
    null,
  );
  const [tools, setTools] = useState<ProjectToolDto[]>([]);
  const [assignments, setAssignments] = useState<ProjectSkillAssignmentDto[]>(
    [],
  );
  const [assignmentsReconciled, setAssignmentsReconciled] = useState(true);
  const [skills, setSkills] = useState<ManagedSkill[]>([]);
  const [toolStatus, setToolStatus] = useState<ToolStatusDto | null>(null);

  // Loading state
  const [projectsLoading, setProjectsLoading] = useState(true);
  const [matrixLoading, setMatrixLoading] = useState(false);
  const [pendingCells, setPendingCells] = useState<Set<string>>(new Set());

  // Error state
  const [loadFailed, setLoadFailed] = useState(false);

  // Modal state
  const [dialog, setDialog] = useState<ProjectDialog | null>(null);

  // Ignore intent captured by registerProject, consumed by the next
  // configureTools for that project (and dropped by any other registration
  // or configuration in between).
  const [pendingIgnore, setPendingIgnore] = useState<{
    projectId: string;
    options: IgnoreUpdateOptions;
  } | null>(null);

  // Version counter for stale result discard on project selection.
  const selectVersionRef = useRef(0);

  /**
   * Apply one backend view: the project row replaces its entry in the list,
   * and the matrix takes the view's tools and assignments wholesale. Every
   * mutation returns a view, so nothing here needs a follow-up read — and a
   * view for a project the operator has since navigated away from only
   * updates that project's row.
   */
  const applyView = useCallback((view: ProjectViewDto) => {
    setProjects((prev) => {
      const index = prev.findIndex((p) => p.id === view.project.id);
      if (index === -1) return [view.project, ...prev];
      const next = [...prev];
      next[index] = view.project;
      return next;
    });
    setSelectedProjectId((current) => {
      if (current === view.project.id) {
        setTools(view.tools);
        setAssignments(view.assignments);
        setAssignmentsReconciled(view.reconciled);
      }
      return current;
    });
  }, []);

  /**
   * Error-path convergence only: re-read a project's view after a mutation
   * failed, so what is shown matches what the backend settled. Success
   * paths never need it — the mutation returned its own view.
   */
  const refreshView = useCallback(
    async (projectId: string) => {
      try {
        applyView(await invokeTauri("getProjectView", projectId));
      } catch {
        // Silent fallback — state may be stale
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

  const loadSkills = useCallback(async () => {
    try {
      const result = await invokeTauri("getManagedSkills");
      setSkills(result);
    } catch {
      // Skills load failure is non-critical for projects tab
    }
  }, []);

  // Load projects and skills on mount. Concurrent fire-and-forget, awaited
  // inside an IIFE so the loaders' setState runs in an async continuation
  // (satisfies react-hooks/set-state-in-effect). Promise.all preserves the
  // original concurrent start; behavior is unchanged.
  useEffect(() => {
    void (async () => {
      await Promise.all([loadProjects(), loadSkills()]);
    })();
  }, [loadProjects, loadSkills]);

  const selectProject = useCallback(
    async (id: string) => {
      setSelectedProjectId(id);
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
        setTools([]);
        setAssignments([]);
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
    async (id: string) => {
      let remaining: ProjectDto[];
      try {
        // The project is gone, so the mutation's view is the remaining list.
        remaining = await invokeTauri("removeProject", id);
      } catch (err) {
        // Artifact removal keeps a row whose artifact stayed on disk with
        // status `error` (ADR-0002) and the project stays registered, so
        // converge on the backend's view before surfacing the failure.
        await refreshView(id);
        throw err;
      }
      setProjects(remaining);
      setSelectedProjectId((prev) => {
        if (prev === id) {
          setTools([]);
          setAssignments([]);
          return null;
        }
        return prev;
      });
    },
    [refreshView],
  );

  const toggleAssignment = useCallback(
    async (skillId: string, tool: string) => {
      if (!selectedProjectId) return;
      const key = `${skillId}:${tool}`;
      // Prevent double-toggle while a pending operation is in flight
      if (pendingCells.has(key)) return;
      setPendingCells((prev) => {
        const next = new Set(prev);
        next.add(key);
        return next;
      });
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
      } catch (err) {
        // A failed mutation may still have settled rows (an unassign whose
        // artifact stayed keeps the row with status `error`), so converge on
        // the backend's view before surfacing the failure.
        await refreshView(selectedProjectId);
        throw err;
      } finally {
        setPendingCells((prev) => {
          const next = new Set(prev);
          next.delete(key);
          return next;
        });
      }
    },
    [selectedProjectId, pendingCells, applyView, refreshView],
  );

  const bulkAssign = useCallback(
    async (skillId: string) => {
      if (!selectedProjectId) return;
      const toolKeys = tools.map((t) => t.tool);
      const pendingKeys = toolKeys.map((tk) => `${skillId}:${tk}`);
      setPendingCells((prev) => {
        const next = new Set(prev);
        for (const k of pendingKeys) next.add(k);
        return next;
      });
      try {
        const result = await invokeTauri(
          "bulkAssignSkill",
          selectedProjectId,
          skillId,
        );
        applyView(result.view);
        return result;
      } catch (err) {
        await refreshView(selectedProjectId);
        throw err;
      } finally {
        setPendingCells((prev) => {
          const next = new Set(prev);
          for (const k of pendingKeys) next.delete(k);
          return next;
        });
      }
    },
    [selectedProjectId, tools, applyView, refreshView],
  );

  const updateProjectPath = useCallback(
    async (projectId: string, newPath: string): Promise<ProjectDto> => {
      const view = await invokeTauri("updateProjectPath", projectId, newPath);
      applyView(view);
      return view.project;
    },
    [applyView],
  );

  const resyncProject = useCallback(async (): Promise<ResyncSummaryDto> => {
    if (!selectedProjectId) throw new Error("No project selected");
    const result = await invokeTauri("resyncProject", selectedProjectId);
    applyView(result.view);
    return result.summary;
  }, [selectedProjectId, applyView]);

  const resyncAll = useCallback(async (): Promise<ResyncSummaryDto[]> => {
    const result = await invokeTauri("resyncAllProjects");
    setProjects(result.projects);
    // The batch touches every project; only the shown one needs its matrix.
    if (selectedProjectId) {
      applyView(await invokeTauri("getProjectView", selectedProjectId));
    }
    return result.summaries;
  }, [selectedProjectId, applyView]);

  const loadToolStatus = useCallback(async () => {
    const result = await invokeTauri("getProjectToolStatus");
    setToolStatus(result);
  }, []);

  const configureTools = useCallback(
    async (toolIds: string[]) => {
      if (!selectedProjectId) return;
      const gitignore =
        pendingIgnore?.projectId === selectedProjectId
          ? pendingIgnore.options
          : null;
      try {
        // The view already reflects the cascade of dropping a tool (its
        // assignments are gone), so there is nothing to re-read.
        const view = await invokeTauri(
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

  const openDialog = useCallback((next: ProjectDialog) => {
    setDialog(next);
  }, []);

  const closeDialog = useCallback(() => {
    setDialog(null);
  }, []);

  return {
    projects,
    selectedProjectId,
    tools,
    assignments,
    assignmentsReconciled,
    skills,
    toolStatus,
    projectsLoading,
    matrixLoading,
    pendingCells,
    loadFailed,
    dialog,
    loadProjects,
    selectProject,
    registerProject,
    removeProject,
    toggleAssignment,
    bulkAssign,
    resyncProject,
    updateProjectPath,
    resyncAll,
    loadToolStatus,
    configureTools,
    discardPendingIgnore,
    getGitignoreStatus,
    updateGitignore,
    openDialog,
    closeDialog,
  };
}
