export type ProjectDto = {
  id: string;
  path: string;
  name: string;
  created_at: number;
  updated_at: number;
  tool_count: number;
  skill_count: number;
  assignment_count: number;
  sync_status: string;
};

export type ProjectToolDto = {
  id: string;
  project_id: string;
  tool: string;
};

export type ProjectSkillAssignmentDto = {
  id: string;
  project_id: string;
  skill_id: string;
  tool: string;
  mode: string;
  status: string;
  last_error?: string | null;
  synced_at?: number | null;
  content_hash?: string | null;
  created_at: number;
};

export type ResyncSummaryDto = {
  project_id: string;
  synced: number;
  failed: number;
  errors: string[];
};

export type BulkAssignResultDto = {
  assigned: ProjectSkillAssignmentDto[];
  failed: BulkAssignErrorDto[];
};

export type BulkAssignErrorDto = {
  tool: string;
  error: string;
};
