---
phase: quick-260429-s9x
plan: 01
subsystem: project-tools
tags: [tool-adapters, db-migration, project-sync, ui]
dependency_graph:
  requires: []
  provides:
    [agents-standard-virtual-group, v8-migration, project-tool-status-command]
  affects: [project-tools-modal, project-sync, tool-detection]
tech_stack:
  added: []
  patterns: [virtual-tool-group, OR-detection, constituent-tool-consolidation]
key_files:
  created: []
  modified:
    - src-tauri/src/core/tool_adapters/mod.rs
    - src-tauri/src/core/skill_store.rs
    - src-tauri/src/commands/mod.rs
    - src-tauri/src/lib.rs
    - src/components/projects/useProjectState.ts
    - src/components/projects/ToolConfigModal.tsx
    - src/App.css
decisions:
  - "AgentsStandard placed as first adapter in default_tool_adapters() for position 0 in project modal"
  - "New get_project_tool_status command created rather than modifying get_tool_status signature"
  - "OR-detection: virtual group shows installed if ANY of 9 constituent tools detected"
metrics:
  duration: "4m 46s"
  completed: "2026-04-30"
  tasks_completed: 3
  tasks_total: 3
---

# Quick Task 260429-s9x: Consolidate .agents/skills Harnesses Summary

Virtual ToolId::AgentsStandard group consolidating 9 .agents/skills tools into a single project-level checkbox with V8 DB migration and OR-based install detection.

## Tasks Completed

| #   | Task                                                                | Commit  | Key Changes                                                                                                                   |
| --- | ------------------------------------------------------------------- | ------- | ----------------------------------------------------------------------------------------------------------------------------- |
| 1   | Add ToolId::AgentsStandard variant and V8 DB migration              | 4485fbc | Enum variant, as_key, adapter entry, project_relative_skills_dir, AGENTS_STANDARD_KEYS const, SCHEMA_VERSION=8, migration SQL |
| 2   | Exclude AgentsStandard from global, add project tool status command | e189eae | get_tool_status skips AgentsStandard, new get_project_tool_status command with OR-detection, registered in lib.rs             |
| 3   | Frontend uses project tool status with subtitle                     | 226db1f | useProjectState invokes new command, ToolConfigModal shows subtitle listing 9 tools, CSS for subtitle                         |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Rust fmt formatting on AGENTS_STANDARD_KEYS constant**

- **Found during:** Task 3 verification (`npm run check`)
- **Issue:** cargo fmt requires one item per line in array constants; plan had them on two lines
- **Fix:** Reformatted to one element per line
- **Files modified:** src-tauri/src/core/tool_adapters/mod.rs
- **Commit:** 226db1f

## Verification Results

- `npm run check` passes clean (lint + build + rust:fmt:check + clippy + tests)
- AgentsStandard appears 5 times in tool_adapters/mod.rs (enum, as_key, adapter, project_relative_skills_dir, supports_project_scope implicit)
- `user_version < 8` migration block exists in skill_store.rs
- `agents_skills` appears in commands/mod.rs (2 occurrences)
- `get_project_tool_status` invoked in useProjectState.ts
- `agents_skills` key handled in ToolConfigModal.tsx

## Self-Check: PASSED
