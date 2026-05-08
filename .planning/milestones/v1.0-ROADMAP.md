# Roadmap: Per-Project Skill Distribution

## Overview

This milestone adds per-project skill distribution to Skills Hub. The work follows a bottom-up build order: data layer first, then sync logic, then IPC commands, then frontend, then edge cases. Each phase is independently testable before the next begins. The existing sync engine is reused unchanged -- the entire feature reduces to "compute different target paths, call the same functions."

## Phases

**Phase Numbering:**

- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Data Foundation** - Schema V4 migration, project/tool CRUD, command module structure
- [ ] **Phase 2: Sync Logic** - Project-aware path resolution, sync operations, cross-platform fallback, serialization
- [ ] **Phase 3: IPC Commands** - Tauri command layer wiring backend to frontend with DTOs
- [ ] **Phase 4: Frontend Component Tree** - Full Projects tab UI with assignment matrix, status indicators, tool configuration
- [ ] **Phase 5: Edge Cases and Polish** - Graceful error handling for stale paths, orphaned assignments, .gitignore prompt
- [ ] **Phase 6: Gap Closure** - Tool removal cascades to assignments/artifacts, missing status production

## Phase Details

### Phase 1: Data Foundation

**Goal**: Projects, tool configurations, and skill assignments can be stored and retrieved reliably
**Depends on**: Nothing (first phase)
**Requirements**: INFR-04, INFR-05, PROJ-01, PROJ-02, PROJ-03, TOOL-01
**Success Criteria** (what must be TRUE):

1. Schema V4 migration creates projects, project_tools, and project_skill_assignments tables in a single transaction without corrupting existing data
2. A project directory can be registered (stored) and removed (deleted with CASCADE cleanup) via Rust core functions
3. Tool columns can be configured per project (add/remove tool associations) via Rust core functions
4. Skill assignments can be created and deleted per project/tool combination via Rust core functions
5. All CRUD operations are in skill_store.rs and project commands are in a separate commands/projects.rs module

**Plans:** 2 plans

Plans:

- [x] 01-01-PLAN.md -- Schema V4 migration + SkillStore CRUD methods (record structs, 13 methods, tests)
- [x] 01-02-PLAN.md -- Tauri command module (commands/projects.rs with 9 commands, DTOs, registration)

### Phase 2: Sync Logic

**Goal**: Assigning or unassigning a skill to a project creates or removes the correct symlink/copy in the project's tool directory
**Depends on**: Phase 1
**Requirements**: ASGN-02, ASGN-03, ASGN-05, SYNC-04, INFR-01, INFR-02
**Success Criteria** (what must be TRUE):

1. Syncing a skill to a project creates a symlink (or copy on cross-filesystem) at project_path/relative_skills_dir/skill_name
2. Unsyncing a skill from a project removes the symlink/copy from the project directory
3. Cross-filesystem scenarios (WSL2 ext4-to-NTFS) are auto-detected and fall back to copy mode
4. Concurrent sync operations (Sync All vs individual toggle) are serialized and do not corrupt state
5. Existing global sync continues to work unchanged alongside project sync

**Plans:** 2 plans

Plans:

- [x] 02-01-PLAN.md -- V5 migration, SyncMutex, project_sync.rs core module (assign_and_sync, unassign_and_cleanup), enhanced commands
- [x] 02-02-PLAN.md -- Re-sync operations, staleness detection with list command wiring, mutex-protected re-sync commands, serialization test

### Phase 3: IPC Commands

**Goal**: All project management and sync operations are accessible from the frontend via Tauri IPC
**Depends on**: Phase 2
**Requirements**: ASGN-01, ASGN-04, SYNC-02, SYNC-03
**Success Criteria** (what must be TRUE):

1. Frontend can invoke assign/unassign skill commands and receive success/error responses
2. Frontend can invoke bulk-assign ("All Tools") for a skill and all configured tools are assigned in one call
3. Frontend can invoke "Sync Project" to re-sync all assignments for one project
4. Frontend can invoke "Sync All" to re-sync all assignments across all projects

**Plans:** 1 plan

Plans:

- [x] 03-01-PLAN.md -- bulk_assign_skill command, error prefix infrastructure (DUPLICATE_PROJECT|, ASSIGNMENT_EXISTS|, NOT_FOUND|), TypeScript DTO types, tests

### Phase 4: Frontend Component Tree

**Goal**: Users can register projects, configure tools, assign skills, and see sync status through a complete Projects tab
**Depends on**: Phase 3
**Requirements**: UI-01, UI-02, UI-03, UI-05, SYNC-01, TOOL-02, TOOL-03
**Success Criteria** (what must be TRUE):

1. A "Projects" tab appears in main navigation and clicking it shows the projects interface
2. User can register a project via folder picker (with manual path fallback) and see it in the project list with assignment counts
3. User can select a project and see a checkbox matrix of skills (rows) vs configured tools (columns) with per-cell status indicators (green/yellow/red/gray)
4. User can add or remove tool columns for a project, with auto-detection of installed tools on first setup
5. Projects tab uses its own component tree and state hook, isolated from App.tsx (App.tsx changes limited to tab navigation)

**Plans:** 3 plans

Plans:

- [x] 04-01-PLAN.md -- useProjectState hook, ProjectsPage shell, i18n strings, App.tsx/Header.tsx navigation integration
- [x] 04-02-PLAN.md -- ProjectList panel, AddProjectModal, ToolConfigModal, RemoveProjectModal, CSS styles
- [x] 04-03-PLAN.md -- AssignmentMatrix with checkbox grid, status colors, toolbar, bulk-assign, visual verification

### Phase 5: Edge Cases and Polish

**Goal**: The app handles missing projects, orphaned assignments, and .gitignore concerns gracefully
**Depends on**: Phase 4
**Requirements**: PROJ-04, PROJ-05, INFR-03
**Success Criteria** (what must be TRUE):

1. When a registered project directory has been moved or deleted, the project list shows a warning badge and sync operations skip it gracefully
2. When a skill is removed from the central library, its project assignments are marked as "missing" with a visual indicator
3. On project registration, user is prompted to add tool skill directories to the project's .gitignore (prompt only, no automatic modification)

**Plans:** 3 plans

Plans:

- [x] 05-01-PLAN.md -- Backend commands: auto-sync setting, bulk/per-skill unsync, INFR-03 skill deletion cleanup, PROJ-04 path_exists, INFR-02 SyncMutex fix, SYNC-01 missing status
- [x] 05-02-PLAN.md -- My Skills frontend: auto-sync toggle, bulk unsync button, per-skill Unlink icon, conditional install sync
- [x] 05-03-PLAN.md -- Projects frontend: missing project warning badges, error prefix translation, bulk assign failure surfacing, update-path flow, gitignore edge cases

### Phase 6: Gap Closure — Tool Removal Cleanup & Missing Status

**Goal**: Tool column removal cascades to assignments/artifacts, and `missing` assignment status is produced when skill source is absent
**Depends on**: Phase 4, Phase 5
**Requirements**: TOOL-03, SYNC-01
**Gap Closure:** Closes gaps from v1.0 milestone audit
**Success Criteria** (what must be TRUE):

1. Removing a tool column from a project deletes all assignments for that tool and cleans up filesystem artifacts (symlinks/copies) in the project directory
2. When a skill's central repo directory no longer exists, assignments referencing it report `missing` status instead of silently failing
3. The "Remove tool column" E2E flow completes without orphaned data
4. The assignment matrix displays `missing` status cells when appropriate

**Plans:** 1 plan

Plans:

- [x] 06-01-PLAN.md -- Tool removal cascade + missing status detection (TOOL-03, SYNC-01)

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5

| Phase                      | Plans Complete | Status      | Completed |
| -------------------------- | -------------- | ----------- | --------- |
| 1. Data Foundation         | 0/2            | Not started | -         |
| 2. Sync Logic              | 0/2            | Not started | -         |
| 3. IPC Commands            | 0/1            | Not started | -         |
| 4. Frontend Component Tree | 0/3            | Not started | -         |
| 5. Edge Cases and Polish   | 0/3            | Not started | -         |
| 6. Gap Closure             | 0/1            | Not started | -         |
