import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  ProjectDto,
  ProjectToolDto,
  ProjectSkillAssignmentDto,
  ResyncSummaryDto,
  BulkAssignResultDto,
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
  loadError: string | null;
  // Modal state
  showAddModal: boolean;
  showToolConfigModal: boolean;
  showRemoveModal: boolean;
  removeTargetId: string | null;
  // Actions
  loadProjects: () => Promise<void>;
  selectProject: (id: string) => Promise<void>;
  registerProject: (path: string) => Promise<ProjectDto>;
  removeProject: (id: string) => Promise<void>;
  toggleAssignment: (skillId: string, tool: string) => Promise<void>;
  bulkAssign: (skillId: string) => Promise<void>;
  resyncProject: () => Promise<ResyncSummaryDto>;
  resyncAll: () => Promise<ResyncSummaryDto[]>;
  loadToolStatus: () => Promise<void>;
  addTools: (tools: string[]) => Promise<void>;
  removeTools: (tools: string[]) => Promise<void>;
  setShowAddModal: (show: boolean) => void;
  setShowToolConfigModal: (show: boolean) => void;
  setShowRemoveModal: (show: boolean) => void;
  setRemoveTargetId: (id: string | null) => void;
};

function normalizeError(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

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
  const [loadError, setLoadError] = useState<string | null>(null);

  // Modal state
  const [showAddModal, setShowAddModal] = useState(false);
  const [showToolConfigModal, setShowToolConfigModal] = useState(false);
  const [showRemoveModal, setShowRemoveModal] = useState(false);
  const [removeTargetId, setRemoveTargetId] = useState<string | null>(null);

  // Version counter for stale result discard on project selection
  const selectVersionRef = useRef(0);

  const loadProjects = useCallback(async () => {
    setProjectsLoading(true);
    setLoadError(null);
    try {
      const result = await invoke<ProjectDto[]>("list_projects");
      setProjects(result);
    } catch (err) {
      setLoadError(normalizeError(err));
    } finally {
      setProjectsLoading(false);
    }
  }, []);

  const loadSkills = useCallback(async () => {
    try {
      const result = await invoke<ManagedSkill[]>("get_managed_skills");
      setSkills(result);
    } catch {
      // Skills load failure is non-critical for projects tab
    }
  }, []);

  // Load projects and skills on mount
  useEffect(() => {
    void loadProjects();
    void loadSkills();
  }, [loadProjects, loadSkills]);

  const selectProject = useCallback(async (id: string) => {
    setSelectedProjectId(id);
    setMatrixLoading(true);
    const version = ++selectVersionRef.current;
    try {
      const [fetchedTools, fetchedAssignments] = await Promise.all([
        invoke<ProjectToolDto[]>("list_project_tools", { projectId: id }),
        invoke<ProjectSkillAssignmentDto[]>("list_project_skill_assignments", {
          projectId: id,
        }),
      ]);
      // Discard stale results if another selection happened
      if (selectVersionRef.current !== version) return;
      setTools(fetchedTools);
      setAssignments(fetchedAssignments);
    } catch (err) {
      if (selectVersionRef.current !== version) return;
      setTools([]);
      setAssignments([]);
      throw new Error(normalizeError(err));
    } finally {
      if (selectVersionRef.current === version) {
        setMatrixLoading(false);
      }
    }
  }, []);

  const registerProject = useCallback(
    async (path: string): Promise<ProjectDto> => {
      const result = await invoke<ProjectDto>("register_project", { path });
      await loadProjects();
      return result;
    },
    [loadProjects],
  );

  const removeProject = useCallback(
    async (id: string) => {
      await invoke("remove_project", { projectId: id });
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
      setPendingCells((prev) => {
        const next = new Set(prev);
        next.add(key);
        return next;
      });
      try {
        const exists = assignments.some(
          (a) => a.skill_id === skillId && a.tool === tool,
        );
        if (exists) {
          await invoke("remove_project_skill_assignment", {
            projectId: selectedProjectId,
            skillId,
            tool,
          });
        } else {
          await invoke("add_project_skill_assignment", {
            projectId: selectedProjectId,
            skillId,
            tool,
          });
        }
        const updated = await invoke<ProjectSkillAssignmentDto[]>(
          "list_project_skill_assignments",
          { projectId: selectedProjectId },
        );
        setAssignments(updated);
      } catch (err) {
        // Re-fetch to get consistent state even on error
        try {
          const updated = await invoke<ProjectSkillAssignmentDto[]>(
            "list_project_skill_assignments",
            { projectId: selectedProjectId },
          );
          setAssignments(updated);
        } catch {
          // Silent fallback — state may be stale
        }
        throw new Error(normalizeError(err));
      } finally {
        setPendingCells((prev) => {
          const next = new Set(prev);
          next.delete(key);
          return next;
        });
      }
    },
    [selectedProjectId, assignments],
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
        await invoke<BulkAssignResultDto>("bulk_assign_skill", {
          projectId: selectedProjectId,
          skillId,
        });
        const updated = await invoke<ProjectSkillAssignmentDto[]>(
          "list_project_skill_assignments",
          { projectId: selectedProjectId },
        );
        setAssignments(updated);
      } catch (err) {
        try {
          const updated = await invoke<ProjectSkillAssignmentDto[]>(
            "list_project_skill_assignments",
            { projectId: selectedProjectId },
          );
          setAssignments(updated);
        } catch {
          // Silent fallback
        }
        throw new Error(normalizeError(err));
      } finally {
        setPendingCells((prev) => {
          const next = new Set(prev);
          for (const k of pendingKeys) next.delete(k);
          return next;
        });
      }
    },
    [selectedProjectId, tools],
  );

  const resyncProject = useCallback(async (): Promise<ResyncSummaryDto> => {
    if (!selectedProjectId) throw new Error("No project selected");
    const result = await invoke<ResyncSummaryDto>("resync_project", {
      projectId: selectedProjectId,
    });
    // Re-fetch assignments to reflect updated sync status
    try {
      const updated = await invoke<ProjectSkillAssignmentDto[]>(
        "list_project_skill_assignments",
        { projectId: selectedProjectId },
      );
      setAssignments(updated);
    } catch {
      // Silent fallback
    }
    return result;
  }, [selectedProjectId]);

  const resyncAll = useCallback(async (): Promise<ResyncSummaryDto[]> => {
    const result = await invoke<ResyncSummaryDto[]>("resync_all_projects");
    await loadProjects();
    // Re-fetch assignments for selected project if any
    if (selectedProjectId) {
      try {
        const updated = await invoke<ProjectSkillAssignmentDto[]>(
          "list_project_skill_assignments",
          { projectId: selectedProjectId },
        );
        setAssignments(updated);
      } catch {
        // Silent fallback
      }
    }
    return result;
  }, [selectedProjectId, loadProjects]);

  const loadToolStatus = useCallback(async () => {
    try {
      const result = await invoke<ToolStatusDto>("get_tool_status");
      setToolStatus(result);
    } catch (err) {
      throw new Error(normalizeError(err));
    }
  }, []);

  const addTools = useCallback(
    async (toolIds: string[]) => {
      if (!selectedProjectId) return;
      for (const tool of toolIds) {
        await invoke("add_project_tool", {
          projectId: selectedProjectId,
          tool,
        });
      }
      const updated = await invoke<ProjectToolDto[]>("list_project_tools", {
        projectId: selectedProjectId,
      });
      setTools(updated);
    },
    [selectedProjectId],
  );

  const removeTools = useCallback(
    async (toolIds: string[]) => {
      if (!selectedProjectId) return;
      for (const tool of toolIds) {
        await invoke("remove_project_tool", {
          projectId: selectedProjectId,
          tool,
        });
      }
      // Re-fetch both tools and assignments (cascade may have removed assignments)
      const [updatedTools, updatedAssignments] = await Promise.all([
        invoke<ProjectToolDto[]>("list_project_tools", {
          projectId: selectedProjectId,
        }),
        invoke<ProjectSkillAssignmentDto[]>("list_project_skill_assignments", {
          projectId: selectedProjectId,
        }),
      ]);
      setTools(updatedTools);
      setAssignments(updatedAssignments);
    },
    [selectedProjectId],
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
    loadError,
    showAddModal,
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
    resyncAll,
    loadToolStatus,
    addTools,
    removeTools,
    setShowAddModal,
    setShowToolConfigModal,
    setShowRemoveModal,
    setRemoveTargetId,
  };
}
