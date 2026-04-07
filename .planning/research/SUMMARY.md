# Project Research Summary

**Project:** Skills Hub - Per-Project Skill Distribution
**Domain:** Per-project file distribution system in a Tauri 2 desktop app
**Researched:** 2026-04-07
**Confidence:** HIGH

## Executive Summary

Per-project skill distribution is a well-understood pattern across developer tooling (VS Code workspace extensions, JetBrains required plugins, asdf `.tool-versions`, devcontainer features). The core model is always the same: a central registry of plugins, project-level configuration declaring which subset applies, a resolution mechanism that makes them available, and status feedback. Skills Hub's variant is unique in that it is a _meta-distribution tool_ -- it populates per-project skill directories for 47+ external AI coding tools rather than managing its own plugin format. The good news is that the existing sync engine is already path-generic, so the entire feature reduces to "compute different target paths, call the same functions."

The recommended approach requires zero new dependencies. The existing Tauri 2 + React 19 + Rust + rusqlite stack covers everything. The work is a Schema V4 migration (three new tables), a new `core/project_sync.rs` module for path resolution, a new `commands/projects.rs` for IPC commands, and an isolated `components/projects/` frontend component tree with its own custom hook for state management. The sync engine (`sync_engine.rs`) and tool adapter registry (`tool_adapters/mod.rs`) remain untouched -- they are already generic enough. The architecture research confirms this with HIGH confidence after direct code inspection.

The primary risks are cross-platform symlink failures (WSL2 cross-filesystem symlinks are invisible to Windows-native AI tools), schema migration without transaction wrapping (partial migration corrupts the database), and race conditions between concurrent sync operations. All three have clear prevention strategies documented in the pitfalls research. The WSL2 issue requires auto-detecting cross-filesystem scenarios and falling back to copy mode. The migration must be wrapped in a transaction. Sync operations must be serialized through a single queue. These mitigations should be built into the foundation phase, not bolted on later.

## Key Findings

### Recommended Stack

No new frameworks or dependencies. Every technology needed is already in `package.json` and `Cargo.toml`. This is a pure feature build on top of the existing stack.

**Core technologies (all unchanged):**

- **Tauri 2 (2.9.5):** Desktop runtime + IPC -- stay on current version, 2.10.3 offers nothing required for this milestone
- **React 19 (19.2.x):** Frontend -- use `useState` in a custom hook for the Projects tab, no state management library needed
- **rusqlite (0.31):** SQLite persistence -- upgrading to 0.39 introduces breaking changes across 3 major versions, not worth the risk
- **Existing sync engine:** `sync_dir_for_tool_with_overwrite()` already accepts generic `source: &Path, target: &Path` -- reuse as-is
- **Existing tool adapters:** `relative_skills_dir` field drives project-local path computation with zero changes

**Explicitly rejected:** Zustand/Redux (overkill for isolated tab), @tanstack/react-table (matrix too specialized), refinery/diesel migrations (one migration step), rusqlite upgrade (breaking changes).

### Expected Features

**Must have (table stakes):**

- Register/remove project directories (entry point for everything)
- User-configurable tool columns per project (47-column matrix is unusable without this)
- Assign/unassign skills via checkbox matrix with immediate sync on toggle
- Sync status indicators (synced/missing/stale/pending) per assignment cell
- Sync Project and Sync All buttons for recovery
- Coexistence with existing global sync (must not break current behavior)
- Search/filter in assignment matrix (57+ skills need filtering)
- Graceful handling of removed/renamed project directories
- Orphaned assignment detection when skills are deleted from central library
- .gitignore prompt on project registration (prompt-based, never automatic)

**Should have (differentiators):**

- Bulk-assign "All Tools" button per skill row (low complexity, high convenience)
- Auto-detect installed tools for initial tool column setup (reuses existing `get_tool_status`)
- Staleness detection for copy-mode targets via content hash comparison
- Clone assignments from another project

**Defer to v2+:**

- Skill grouping/presets ("Frontend Pack")
- Project templates
- Portable assignment manifests (YAML export/import)
- CLI companion for headless environments
- Context-aware skill suggestions

**Anti-features (explicitly do NOT build):**

- Auto-sync / file watcher daemon (symlinks already propagate instantly; copy mode uses manual re-sync)
- Per-project skill versioning (central library is single source of truth)
- Automatic .gitignore modification (always prompt, never auto-write)
- Nested project support (register monorepo root or sub-packages as separate projects)

### Architecture Approach

The architecture extends the existing layered pattern (commands -> core -> filesystem) without modifying the sync engine or global sync flows. Three new concerns map to distinct components: project CRUD in `skill_store.rs`, project-aware path resolution in a new `core/project_sync.rs`, and IPC commands in a new `commands/projects.rs`. The frontend is an isolated component tree under `components/projects/` with a `useProjectState()` custom hook that owns all state and makes its own IPC calls, keeping App.tsx changes to approximately 5 lines (tab navigation only).

**Major components:**

1. **Schema V4 migration** -- three new tables (projects, project_tools, project_skill_assignments) following the existing PRAGMA user_version pattern
2. **`core/project_sync.rs`** -- project-aware path resolution (`project_path + relative_skills_dir + skill_name`) and sync orchestration, calling existing sync_engine primitives unchanged
3. **`commands/projects.rs`** -- thin IPC command layer (10-12 commands) split from the existing 985-line commands/mod.rs
4. **`components/projects/` tree** -- ProjectsTab (container), ProjectList (left panel), AssignmentMatrix (right panel with hand-rolled table), AssignmentCell (checkbox + status), useProjectState (custom hook)
5. **Sync serialization layer** -- new coordination mechanism to prevent race conditions between Sync All and individual toggles

### Critical Pitfalls

1. **WSL2 cross-filesystem symlink breakage** -- symlinks from Linux ext4 to NTFS `/mnt/c/` paths are invisible to Windows-native AI tools. Detect cross-filesystem scenarios at project registration and auto-fall-back to copy mode. Must be in the foundation phase.

2. **Schema V4 migration without transaction wrapping** -- existing migrations are not wrapped in explicit transactions. A partial migration (one table created, next fails) leaves the database in an inconsistent state. Wrap all V4 DDL in a single `BEGIN`/`COMMIT` transaction. Test V3-to-V4 upgrade path specifically.

3. **Race conditions in concurrent sync operations** -- "Sync All" iterates all projects while individual checkbox toggles create/remove symlinks concurrently. The sync engine's `remove_dir_all` + `sync_dir_hybrid` sequence is non-atomic. Serialize all sync operations through a single queue (`tokio::sync::Mutex` or `mpsc` channel).

4. **SQLite foreign keys silently disabled** -- `PRAGMA foreign_keys = ON` is per-connection. All V4 tables use `ON DELETE CASCADE`. Any code path that bypasses `with_conn` (which sets the pragma) leaves orphaned rows. Add a test that verifies CASCADE behavior; centralize connection creation.

5. **`relative_skills_dir` conflates global and project-local paths** -- some tools' directory patterns (`.config/agents/skills`, `.gemini/antigravity/global_skills`) do not make sense as project-local paths. Start with a curated subset of tools known to support project-local skills (Claude Code, Cursor, Codex), add others incrementally.

## Implications for Roadmap

Based on combined research, the natural phase structure follows a bottom-up build order where each phase is independently testable before the next begins.

### Phase 1: Backend Data Foundation

**Rationale:** Every other phase depends on the database tables and CRUD operations existing. The migration is the first thing that runs and must be correct -- getting it wrong means data loss (Pitfall 5).
**Delivers:** Schema V4 migration (3 new tables), project CRUD methods, project_tools CRUD, assignment CRUD in `skill_store.rs`. Also: refactor `commands/mod.rs` into `commands/skills.rs` + `commands/mod.rs` to establish clean module boundaries before adding project commands.
**Addresses features:** Register/remove project, user-configurable tool columns (data layer only)
**Avoids pitfalls:** Schema migration corruption (wrap in transaction), foreign key enforcement (test CASCADE behavior), path canonicalization (establish canonical storage from day one)

### Phase 2: Backend Sync Logic

**Rationale:** Path resolution and sync orchestration depend on Phase 1's CRUD methods but are independent of the frontend. Building sync logic before IPC commands enables thorough Rust unit testing without a running app.
**Delivers:** `core/project_sync.rs` module -- `resolve_project_target()`, `sync_skill_to_project()`, `unsync_skill_from_project()`, `sync_project()`, `sync_all_projects()`, staleness check for copy-mode. Also: sync operation serialization (Mutex/channel queue) and cross-filesystem detection for WSL2.
**Addresses features:** Assign/unassign with immediate sync, Sync Project, Sync All, staleness detection, coexistence with global sync
**Avoids pitfalls:** WSL2 cross-filesystem breakage (auto-detect and fall back to copy), race conditions (serialize through queue), `is_same_link` path comparison failure (canonicalize paths), shared skills directory duplication (extract shared-dir utility)

### Phase 3: Backend IPC Commands

**Rationale:** Thin command layer that wires Phase 1 + Phase 2 to the frontend. Quick to build because it is pure delegation with no business logic. Can be tested via Tauri devtools console before any frontend code exists.
**Delivers:** `commands/projects.rs` with 10-12 commands registered in `lib.rs`. DTOs for Project, Assignment, SyncStatus, SyncReport.
**Addresses features:** All backend capabilities exposed to frontend via IPC
**Avoids pitfalls:** Monolithic commands file (dedicated projects.rs module)

### Phase 4: Frontend Component Tree

**Rationale:** Largest single phase but has clear internal component boundaries. Depends on all backend phases being complete so there are no integration surprises. The isolated component tree pattern means App.tsx changes are minimal.
**Delivers:** Full Projects tab UI -- ProjectsTab container, ProjectList (left panel with add/remove), tool column picker, AssignmentMatrix (checkbox grid), AssignmentCell (checkbox + status icon), SyncStatusBar, search/filter bar. App.tsx gets approximately 5 new lines for tab navigation.
**Addresses features:** All table-stakes UI: project registration via folder picker, tool column configuration, checkbox matrix interaction, status indicators, Sync Project/All buttons, search/filter, bulk-assign "All Tools"
**Avoids pitfalls:** App.tsx state leakage (custom hook fetches own data via IPC), optimistic UI revert complexity (Map-based pending state tracking)

### Phase 5: Edge Cases and Polish

**Rationale:** Edge cases require a working end-to-end system to test. They do not require architectural changes -- just defensive checks and UX refinements.
**Delivers:** .gitignore prompt on registration, missing/renamed project directory handling (warning badges, update-path command), orphaned assignment detection, auto-detect installed tools for column setup, cross-platform symlink verification tests.
**Addresses features:** .gitignore prompt, handle removed directories, handle orphaned assignments, auto-detect tools, clone assignments from another project
**Avoids pitfalls:** .gitignore timing/content issues (tool-specific entries, one-time flag), stale project paths (validate on list and sync)

### Phase Ordering Rationale

- **Bottom-up build (data -> logic -> IPC -> UI -> polish):** Each layer can be unit-tested independently before the next depends on it. The sync engine reuse assumption (the highest-risk architectural bet) is validated in Phase 2, long before any frontend work.
- **Commands module refactor in Phase 1:** Moving existing commands to `commands/skills.rs` before adding project commands prevents a 1200+ line monolith. This is a one-time cost that pays off permanently.
- **Sync serialization in Phase 2:** The IPC contract (Phase 3) depends on whether sync is synchronous, queued, or cancellable. This decision must be made before exposing commands to the frontend.
- **Frontend as a single phase:** All UI components share the `useProjectState` hook. Building them together ensures the state interface is cohesive. Internal component boundaries (ProjectList, AssignmentMatrix, etc.) allow parallel work within the phase.
- **Polish last:** Edge cases (.gitignore, stale paths, orphans) are defensive checks layered on top of working functionality. They should not block the core interaction loop.

### Research Flags

Phases likely needing deeper research during planning:

- **Phase 2 (Backend Sync Logic):** WSL2 cross-filesystem detection needs early empirical testing. The Tauri folder picker behavior on WSL2 is unverified (Pitfall 11, LOW confidence). Also: which of the 42+ tools actually support project-local skill loading needs per-tool verification (Pitfall 6, MEDIUM confidence).
- **Phase 4 (Frontend Component Tree):** Optimistic UI revert for batch operations (bulk-assign "All Tools") has non-trivial state management complexity (Pitfall 12). The `useProjectState` hook design should be validated before building all components.

Phases with standard patterns (skip deep research):

- **Phase 1 (Backend Data Foundation):** Schema migration follows the proven V1-V3 pattern. Transaction wrapping and CASCADE testing are standard SQLite practices.
- **Phase 3 (Backend IPC Commands):** Pure delegation layer following the existing command pattern. No novel decisions.
- **Phase 5 (Edge Cases and Polish):** All edge cases are well-characterized in the pitfalls research with clear prevention strategies.

## Confidence Assessment

| Area         | Confidence                         | Notes                                                                                                                                                                                 |
| ------------ | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Stack        | HIGH                               | All technologies already in use. No new dependencies. Version decisions verified against crates.io and npm registry APIs.                                                             |
| Features     | HIGH                               | Feature landscape informed by 6+ real-world ecosystems (VS Code, JetBrains, asdf, devcontainers, Cargo, Claude Code). Existing codebase analysis confirms feasibility.                |
| Architecture | HIGH                               | All component boundaries validated by reading source code. Sync engine path-generic interface confirmed. Build order respects actual code dependencies.                               |
| Pitfalls     | HIGH (critical), MEDIUM (moderate) | Critical pitfalls verified by direct code inspection and official documentation. Two items need empirical validation: WSL2 folder picker behavior and per-tool project-local support. |

**Overall confidence:** HIGH

### Gaps to Address

- **Which tools support project-local skill loading:** The tool adapter registry has 42+ tools, but not all have a meaningful project-local skill directory convention. During Phase 2 planning, audit each tool and flag those where `relative_skills_dir` does not apply project-locally. Start with the curated subset (Claude Code, Cursor, Codex, Windsurf) and expand.
- **WSL2 Tauri folder picker behavior:** The dialog plugin may return Windows-format paths. Needs early empirical testing in Phase 1 or 2. If it misbehaves, add a manual path entry fallback.
- **Optimistic UI revert for batch operations:** The custom hook design should handle both individual toggles and batch "All Tools" toggles cleanly. Validate the state management approach before building all matrix components.
- **`is_same_link` path comparison fix:** Pitfall 2 identifies an existing bug in `sync_engine.rs` that will be amplified by project-local paths (more path variation). Decide whether to fix it as part of this milestone or flag it as a known limitation.

## Sources

### Primary (HIGH confidence)

- Codebase analysis: `sync_engine.rs`, `skill_store.rs`, `commands/mod.rs`, `tool_adapters/mod.rs`, `lib.rs`, `App.tsx` -- direct source code inspection
- Design document: `docs/plans/2026-04-02-skills-hub-fork-design.md`
- Project requirements: `.planning/PROJECT.md`
- rusqlite versions: crates.io API (0.39.0 latest, breaking changes documented in GitHub releases)
- Tauri versions: crates.io API (2.10.3 latest), official docs for command organization and state management
- SQLite foreign key documentation: https://www.sqlite.org/foreignkeys.html
- Microsoft WSL filesystem documentation: https://learn.microsoft.com/en-us/windows/wsl/filesystems

### Secondary (MEDIUM confidence)

- VS Code workspace extensions: https://code.visualstudio.com/docs/editor/extension-marketplace
- JetBrains required plugins: https://www.jetbrains.com/help/idea/managing-plugins.html
- Claude Code skills system: https://code.claude.com/docs/en/skills
- asdf configuration: https://asdf-vm.com/manage/configuration.html
- Devcontainer spec: https://containers.dev/implementors/json_reference/
- Cargo workspaces: https://doc.rust-lang.org/cargo/reference/workspaces.html

### Tertiary (LOW confidence)

- WSL2 Tauri folder picker behavior -- flagged as risk in design doc, unverified empirically
- Per-tool project-local skill directory support for all 42+ tools -- inferred from adapter code, not verified per-tool

---

_Research completed: 2026-04-07_
_Ready for roadmap: yes_
