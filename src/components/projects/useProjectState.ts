import { useCallback, useEffect, useRef, useState } from "react";
// Backend calls go through the shared invokeTauri seam (src/lib/tauri.ts),
// same as every world hook — one mock point for tests.
import { invokeTauri } from "../../lib/tauri";
import type {
  ProjectDto,
  ProjectToolDto,
  ProjectSkillAssignmentDto,
  ResyncSummaryDto,
  BulkAssignResultDto,
  GitignoreStatusDto,
  IgnoreUpdateOptions,
} from "./types";
import type { ManagedSkill, ToolStatusDto } from "../skills/types";

export type ProjectState = {
  // Data
  projects: ProjectDto[];
  selectedProjectId: string | null;
  tools: ProjectToolDto[];
  assignments: ProjectSkillAssignmentDto[];
  skills: ManagedSkill[];
  toolStatus: ToolStatusDto | null;
  // Loading
  projectsLoading: boolean;
  matrixLoading: boolean;
  pendingCells: Set<string>;
  // Errors
  loadFailed: boolean;
  // Modal state
  showAddModal: boolean;
  showEditModal: boolean;
  editTargetId: string | null;
  showToolConfigModal: boolean;
  showRemoveModal: boolean;
  removeTargetId: string | null;
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
  setShowAddModal: (show: boolean) => void;
  setShowEditModal: (show: boolean) => void;
  setEditTargetId: (id: string | null) => void;
  setShowToolConfigModal: (show: boolean) => void;
  setShowRemoveModal: (show: boolean) => void;
  setRemoveTargetId: (id: string | null) => void;
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
  const [skills, setSkills] = useState<ManagedSkill[]>([]);
  const [toolStatus, setToolStatus] = useState<ToolStatusDto | null>(null);

  // Loading state
  const [projectsLoading, setProjectsLoading] = useState(true);
  const [matrixLoading, setMatrixLoading] = useState(false);
  const [pendingCells, setPendingCells] = useState<Set<string>>(new Set());

  // Error state
  const [loadFailed, setLoadFailed] = useState(false);

  // Modal state
  const [showAddModal, setShowAddModal] = useState(false);
  const [showEditModal, setShowEditModal] = useState(false);
  const [editTargetId, setEditTargetId] = useState<string | null>(null);
  const [showToolConfigModal, setShowToolConfigModal] = useState(false);
  const [showRemoveModal, setShowRemoveModal] = useState(false);
  const [removeTargetId, setRemoveTargetId] = useState<string | null>(null);

  // Version counter for stale result discard on project selection
  const selectVersionRef = useRef(0);

  // Ignore intent captured by registerProject, consumed by the next
  // configureTools for that project (and dropped by any other registration
  // or configuration in between).
  const [pendingIgnore, setPendingIgnore] = useState<{
    projectId: string;
    options: IgnoreUpdateOptions;
  } | null>(null);

  // Track latest assignments for stale-closure protection in toggleAssignment.
  // Write the ref in an effect (after commit) rather than during render to
  // satisfy react-hooks/refs. toggleAssignment reads assignmentsRef.current
  // only inside an async event handler (always after the latest commit), so
  // the ref still reflects the newest assignments when read — behavior-preserving.
  const assignmentsRef = useRef(assignments);
  useEffect(() => {
    assignmentsRef.current = assignments;
  }, [assignments]);

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

  const selectProject = useCallback(async (id: string) => {
    setSelectedProjectId(id);
    setMatrixLoading(true);
    const version = ++selectVersionRef.current;
    try {
      const [fetchedTools, fetchedAssignments] = await Promise.all([
        invokeTauri("listProjectTools", id),
        invokeTauri("listProjectSkillAssignments", id),
      ]);
      // Discard stale results if another selection happened
      if (selectVersionRef.current !== version) return;
      setTools(fetchedTools);
      setAssignments(fetchedAssignments);
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
  }, []);

  // Silently re-fetch assignments for a project, keeping stale state when
  // even the re-fetch fails. Shared by mutation error paths and the resync
  // flows so the matrix converges on the backend's view of the assignments.
  // (Success paths of toggle/bulk re-fetch unguarded on purpose: there a
  // failed list surfaces to the caller.)
  const refreshAssignments = useCallback(async (projectId: string) => {
    try {
      const updated = await invokeTauri(
        "listProjectSkillAssignments",
        projectId,
      );
      setAssignments(updated);
    } catch {
      // Silent fallback — state may be stale
    }
  }, []);

  const registerProject = useCallback(
    async (
      path: string,
      gitignore: IgnoreUpdateOptions,
    ): Promise<ProjectDto> => {
      const result = await invokeTauri("registerProject", path);
      setPendingIgnore(
        gitignore.add_to_gitignore || gitignore.add_to_exclude
          ? { projectId: result.id, options: gitignore }
          : null,
      );
      await loadProjects();
      return result;
    },
    [loadProjects],
  );

  const removeProject = useCallback(
    async (id: string) => {
      await invokeTauri("removeProject", id);
      setSelectedProjectId((prev) => {
        if (prev === id) {
          setTools([]);
          setAssignments([]);
          return null;
        }
        return prev;
      });
      await loadProjects();
    },
    [loadProjects],
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
        const exists = assignmentsRef.current.some(
          (a) => a.skill_id === skillId && a.tool === tool,
        );
        if (exists) {
          await invokeTauri(
            "removeProjectSkillAssignment",
            selectedProjectId,
            skillId,
            tool,
          );
        } else {
          await invokeTauri(
            "addProjectSkillAssignment",
            selectedProjectId,
            skillId,
            tool,
          );
        }
        const updated = await invokeTauri(
          "listProjectSkillAssignments",
          selectedProjectId,
        );
        setAssignments(updated);
        await loadProjects();
      } catch (err) {
        // Re-fetch to get consistent state even on error
        await refreshAssignments(selectedProjectId);
        throw err;
      } finally {
        setPendingCells((prev) => {
          const next = new Set(prev);
          next.delete(key);
          return next;
        });
      }
    },
    [selectedProjectId, loadProjects, pendingCells, refreshAssignments],
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
        const updated = await invokeTauri(
          "listProjectSkillAssignments",
          selectedProjectId,
        );
        setAssignments(updated);
        await loadProjects();
        return result;
      } catch (err) {
        await refreshAssignments(selectedProjectId);
        throw err;
      } finally {
        setPendingCells((prev) => {
          const next = new Set(prev);
          for (const k of pendingKeys) next.delete(k);
          return next;
        });
      }
    },
    [selectedProjectId, tools, loadProjects, refreshAssignments],
  );

  const updateProjectPath = useCallback(
    async (projectId: string, newPath: string): Promise<ProjectDto> => {
      const result = await invokeTauri("updateProjectPath", projectId, newPath);
      await loadProjects();
      return result;
    },
    [loadProjects],
  );

  const resyncProject = useCallback(async (): Promise<ResyncSummaryDto> => {
    if (!selectedProjectId) throw new Error("No project selected");
    const result = await invokeTauri("resyncProject", selectedProjectId);
    // Re-fetch assignments to reflect updated sync status
    await refreshAssignments(selectedProjectId);
    await loadProjects();
    return result;
  }, [selectedProjectId, loadProjects, refreshAssignments]);

  const resyncAll = useCallback(async (): Promise<ResyncSummaryDto[]> => {
    const result = await invokeTauri("resyncAllProjects");
    await loadProjects();
    // Re-fetch assignments for selected project if any
    if (selectedProjectId) {
      await refreshAssignments(selectedProjectId);
    }
    return result;
  }, [selectedProjectId, loadProjects, refreshAssignments]);

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
        const updatedTools = await invokeTauri(
          "configureProjectTools",
          selectedProjectId,
          toolIds,
          gitignore,
        );
        setTools(updatedTools);
        // Consume the intent only once the backend has written it. On
        // rejection the modal stays open, so keeping the intent lets a
        // retry replay it instead of silently persisting tools alone.
        setPendingIgnore(null);
      } catch (err) {
        // The tools may have been persisted before the ignore write failed;
        // converge on the backend's view (silently, like refreshAssignments).
        try {
          setTools(
            await invokeTauri("listProjectTools", selectedProjectId),
          );
        } catch {
          // Silent fallback — state may be stale
        }
        throw err;
      } finally {
        // Removing a tool cascades to its assignments; tool_count changed.
        await refreshAssignments(selectedProjectId);
        await loadProjects();
      }
    },
    [selectedProjectId, pendingIgnore, refreshAssignments, loadProjects],
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

  return {
    projects,
    selectedProjectId,
    tools,
    assignments,
    skills,
    toolStatus,
    projectsLoading,
    matrixLoading,
    pendingCells,
    loadFailed,
    showAddModal,
    showEditModal,
    editTargetId,
    showToolConfigModal,
    showRemoveModal,
    removeTargetId,
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
    setShowAddModal,
    setShowEditModal,
    setEditTargetId,
    setShowToolConfigModal,
    setShowRemoveModal,
    setRemoveTargetId,
  };
}
