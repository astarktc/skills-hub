# Requirements: Skills Hub -- Per-Project Skill Distribution

**Defined:** 2026-04-07
**Core Value:** Any skill assigned to a project is immediately available in that project's tool directory via symlink, so AI tools only load the skills that matter for that project.

## v1 Requirements

Requirements for this milestone. Each maps to roadmap phases.

### Project Management

- [ ] **PROJ-01**: User can register a project directory via folder picker (with manual path entry fallback)
- [ ] **PROJ-02**: User can remove a registered project (cleans up all deployed symlinks/copies in the project directory)
- [ ] **PROJ-03**: User can see all registered projects in a list with assignment counts and aggregate sync status
- [ ] **PROJ-04**: App detects removed/renamed project directories on list load and shows a warning badge
- [ ] **PROJ-05**: App prompts user to add tool-specific skill directories to the project's .gitignore on registration

### Tool Configuration

- [ ] **TOOL-01**: User can configure which tool columns appear in the assignment matrix per project
- [ ] **TOOL-02**: Tool column picker auto-detects installed tools and pre-selects them on first setup
- [ ] **TOOL-03**: User can add or remove tool columns from a project at any time

### Skill Assignment

- [ ] **ASGN-01**: User can assign a skill to a project for a specific tool via checkbox in the matrix
- [ ] **ASGN-02**: Assigning a skill immediately creates a symlink/copy in the project's tool skill directory
- [ ] **ASGN-03**: User can unassign a skill from a project (removes symlink/copy from project directory)
- [ ] **ASGN-04**: User can bulk-assign all configured tools for a skill via "All Tools" button per row
- [ ] **ASGN-05**: Global sync (existing feature) continues to work alongside project sync without interference

### Sync & Status

- [ ] **SYNC-01**: Each assignment cell shows status: synced (green), stale (yellow), missing (red), pending (gray)
- [ ] **SYNC-02**: User can re-sync all assignments for a single project via "Sync Project" button
- [ ] **SYNC-03**: User can re-sync all assignments across all projects via "Sync All" button
- [ ] **SYNC-04**: App detects content staleness for copy-mode targets via hash comparison

### Infrastructure

- [ ] **INFR-01**: App detects cross-filesystem scenarios (WSL2 ext4-to-NTFS) and auto-falls back to copy mode
- [ ] **INFR-02**: Sync operations are serialized to prevent race conditions between Sync All and individual toggles
- [ ] **INFR-03**: App detects orphaned assignments when skills are deleted from central library and marks them as "missing"
- [ ] **INFR-04**: Schema V4 migration adds projects, project_tools, and project_skill_assignments tables with transaction wrapping
- [ ] **INFR-05**: New Tauri IPC commands are in a separate `commands/projects.rs` module (not in existing `commands/mod.rs`)

### Frontend

- [ ] **UI-01**: Projects tab appears in main navigation alongside existing tabs
- [ ] **UI-02**: Project list panel (left) with add/remove project actions
- [ ] **UI-03**: Assignment matrix panel (right) with checkbox grid for selected project
- [ ] **UI-04**: Sync status bar (bottom) with last sync time and Sync Project / Sync All buttons
- [ ] **UI-05**: Projects tab uses its own component tree and state (isolated from App.tsx)

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

| Requirement | Phase   | Status  |
| ----------- | ------- | ------- |
| PROJ-01     | Phase 1 | Pending |
| PROJ-02     | Phase 1 | Pending |
| PROJ-03     | Phase 1 | Pending |
| PROJ-04     | Phase 5 | Pending |
| PROJ-05     | Phase 5 | Pending |
| TOOL-01     | Phase 1 | Pending |
| TOOL-02     | Phase 4 | Pending |
| TOOL-03     | Phase 4 | Pending |
| ASGN-01     | Phase 3 | Pending |
| ASGN-02     | Phase 2 | Pending |
| ASGN-03     | Phase 2 | Pending |
| ASGN-04     | Phase 3 | Pending |
| ASGN-05     | Phase 2 | Pending |
| SYNC-01     | Phase 4 | Pending |
| SYNC-02     | Phase 3 | Pending |
| SYNC-03     | Phase 3 | Pending |
| SYNC-04     | Phase 2 | Pending |
| INFR-01     | Phase 2 | Pending |
| INFR-02     | Phase 2 | Pending |
| INFR-03     | Phase 5 | Pending |
| INFR-04     | Phase 1 | Pending |
| INFR-05     | Phase 1 | Pending |
| UI-01       | Phase 4 | Pending |
| UI-02       | Phase 4 | Pending |
| UI-03       | Phase 4 | Pending |
| UI-04       | Phase 4 | Pending |
| UI-05       | Phase 4 | Pending |

**Coverage:**

- v1 requirements: 27 total
- Mapped to phases: 27
- Unmapped: 0

---

_Requirements defined: 2026-04-07_
_Last updated: 2026-04-07 after roadmap creation_
