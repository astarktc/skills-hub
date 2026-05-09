# Requirements: Skills Hub -- Per-Project Skill Distribution

**Defined:** 2026-04-07
**Core Value:** Any skill assigned to a project is immediately available in that project's tool directory via symlink, so AI tools only load the skills that matter for that project.

## v1 Requirements

Requirements for this milestone. Each maps to roadmap phases.

### Project Management

- [x] **PROJ-01**: User can register a project directory via folder picker (with manual path entry fallback)
- [x] **PROJ-02**: User can remove a registered project (cleans up all deployed symlinks/copies in the project directory)
- [x] **PROJ-03**: User can see all registered projects in a list with assignment counts and aggregate sync status
- [x] **PROJ-04**: App detects removed/renamed project directories on list load and shows a warning badge
- [x] **PROJ-05**: App prompts user to add tool-specific skill directories to the project's .gitignore on registration

### Tool Configuration

- [x] **TOOL-01**: User can configure which tool columns appear in the assignment matrix per project
- [x] **TOOL-02**: Tool column picker auto-detects installed tools and pre-selects them on first setup
- [x] **TOOL-03**: User can add or remove tool columns from a project at any time

### Skill Assignment

- [x] **ASGN-01**: User can assign a skill to a project for a specific tool via checkbox in the matrix
- [x] **ASGN-02**: Assigning a skill immediately creates a symlink/copy in the project's tool skill directory
- [x] **ASGN-03**: User can unassign a skill from a project (removes symlink/copy from project directory)
- [x] **ASGN-04**: User can bulk-assign all configured tools for a skill via "All Tools" button per row
- [x] **ASGN-05**: Global sync (existing feature) continues to work alongside project sync without interference

### Sync & Status

- [x] **SYNC-01**: Each assignment cell shows status: synced (green), stale (yellow), missing (red), pending (gray)
- [x] **SYNC-02**: User can re-sync all assignments for a single project via "Sync Project" button
- [x] **SYNC-03**: User can re-sync all assignments across all projects via "Sync All" button
- [x] **SYNC-04**: App detects content staleness for copy-mode targets via hash comparison

### Infrastructure

- [x] **INFR-01**: App detects cross-filesystem scenarios (WSL2 ext4-to-NTFS) and auto-falls back to copy mode
- [x] **INFR-02**: Sync operations are serialized to prevent race conditions between Sync All and individual toggles
- [x] **INFR-03**: When a skill is deleted from the central library, all its project assignments and filesystem artifacts are cleaned up (cascade delete)
- [x] **INFR-04**: Schema V4 migration adds projects, project_tools, and project_skill_assignments tables with transaction wrapping
- [x] **INFR-05**: New Tauri IPC commands are in a separate `commands/projects.rs` module (not in existing `commands/mod.rs`)

### Frontend

- [x] **UI-01**: Projects tab appears in main navigation alongside existing tabs
- [x] **UI-02**: Project list panel (left) with add/remove project actions
- [x] **UI-03**: Assignment matrix panel (right) with checkbox grid for selected project
- [x] **UI-05**: Projects tab uses its own component tree and state (isolated from App.tsx)

## v2 Requirements

Deferred to future milestones. Tracked but not in current roadmap.

### UI Enhancements

- **UINX-01**: Search/filter bar in the assignment matrix for large skill libraries
- **UINX-02**: Clone assignments from another project
- **UINX-03**: Sort assignments by name, status, recently synced
- **UINX-04**: Chinese i18n for all new project management strings

### Advanced Features

- **ADV-01**: Skill grouping / presets ("Frontend Pack" bundles)
- **ADV-02**: Project templates (auto-assign skills for new project types)
- **ADV-03**: Portable assignment manifests (YAML export/import for cross-machine setup)
- **ADV-04**: CLI companion for headless environments (`skillshub sync --project BDA`)
- **ADV-05**: Context-aware skill suggestions based on project contents

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature                                        | Reason                                                                                  |
| ---------------------------------------------- | --------------------------------------------------------------------------------------- |
| Per-project skill versioning                   | Central library is single source of truth -- one version per skill                      |
| Fork/customize skills with upstream tracking   | Patch systems are complex and fragile -- create separate skills instead                 |
| Auto-sync / file watcher daemon                | Symlink mode propagates instantly; copy mode uses manual re-sync + staleness indicators |
| Automatic .gitignore modification              | Always prompt, never auto-write -- respect user's git workflow                          |
| Cross-machine project sync                     | Project paths are machine-specific -- portable manifests (deferred) solve this better   |
| Nested project support (monorepo sub-projects) | Register monorepo root or sub-packages as separate projects                             |
| Mobile/web interface                           | Desktop-only via Tauri                                                                  |
| Undo/redo for assignment changes               | Toggle cost is trivially low -- just uncheck the box                                    |
| Drag-and-drop skill ordering                   | Skills are an unordered set -- AI tools don't care about order                          |
| Real-time team collaboration                   | Skills Hub is local-only -- team sharing via git is the natural mechanism               |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase   | Status |
| ----------- | ------- | ------ |
| PROJ-01     | Phase 1 | Done   |
| PROJ-02     | Phase 1 | Done   |
| PROJ-03     | Phase 1 | Done   |
| PROJ-04     | Phase 5 | Done   |
| PROJ-05     | Phase 5 | Done   |
| TOOL-01     | Phase 1 | Done   |
| TOOL-02     | Phase 4 | Done   |
| TOOL-03     | Phase 6 | Done   |
| ASGN-01     | Phase 3 | Done   |
| ASGN-02     | Phase 2 | Done   |
| ASGN-03     | Phase 2 | Done   |
| ASGN-04     | Phase 3 | Done   |
| ASGN-05     | Phase 2 | Done   |
| SYNC-01     | Phase 6 | Done   |
| SYNC-02     | Phase 3 | Done   |
| SYNC-03     | Phase 3 | Done   |
| SYNC-04     | Phase 2 | Done   |
| INFR-01     | Phase 2 | Done   |
| INFR-02     | Phase 2 | Done   |
| INFR-03     | Phase 5 | Done   |
| INFR-04     | Phase 1 | Done   |
| INFR-05     | Phase 1 | Done   |
| UI-01       | Phase 4 | Done   |
| UI-02       | Phase 4 | Done   |
| UI-03       | Phase 4 | Done   |
| UI-05       | Phase 4 | Done   |

**Coverage:**

- v1 requirements: 26 total
- Mapped to phases: 26
- Unmapped: 0

---

_Requirements defined: 2026-04-07_
_Last updated: 2026-04-09 after v1.0 milestone completion_
