# Skills Hub

## What This Is

Skills Hub is a cross-platform desktop app (Tauri 2 + React 19) for managing AI Agent Skills and syncing them to 47+ AI coding tools. It supports both global sync (deploy skills to all tool directories) and per-project sync (assign specific skills to specific projects via symlinks from `~/.skillshub/<skill>` to `<project>/.claude/skills/<skill>` or equivalent tool path).

## Core Value

Install once, sync everywhere -- with per-project precision. Any skill assigned to a project is immediately available in that project's tool directory via symlink, so AI tools only load the skills that matter for that project.

## Requirements

### Validated

- ✓ Install skills from local directories -- existing
- ✓ Install skills from git repositories -- existing
- ✓ Manage skills in central library (`~/.skillshub/`) -- existing
- ✓ Sync skills to 47+ AI tool global directories via symlink/junction/copy -- existing
- ✓ Detect installed AI tools and show sync status -- existing
- ✓ Onboarding: discover existing skills across tool directories -- existing
- ✓ Update skills from source (local or git) -- existing
- ✓ Browse and search online skill repositories -- existing
- ✓ View skill file contents with markdown/code rendering -- existing
- ✓ Bilingual UI (English/Chinese) -- existing
- ✓ Auto-update via GitHub Releases -- existing
- ✓ Cross-platform: macOS, Windows, Linux -- existing
- ✓ SQLite-backed persistence with schema migrations -- existing
- ✓ Register project directories via folder picker (with manual path fallback) -- v1.0
- ✓ Remove registered projects with full cleanup (symlinks/copies/DB) -- v1.0
- ✓ See all registered projects with assignment counts and aggregate sync status -- v1.0
- ✓ Configure which tool columns appear per project -- v1.0
- ✓ Auto-detect installed tools on first project setup -- v1.0
- ✓ Add/remove tool columns at any time with cascade cleanup -- v1.0
- ✓ Assign skills to projects per tool via checkbox matrix UI -- v1.0
- ✓ Assigning creates symlink/copy immediately -- v1.0
- ✓ Unassign skills (removes symlink/copy from project directory) -- v1.0
- ✓ "All Tools" bulk-assign button per skill row -- v1.0
- ✓ Global sync continues alongside project sync without interference -- v1.0
- ✓ Per-cell status indicators: synced/stale/missing/error/pending -- v1.0
- ✓ Sync all assignments for a single project ("Sync Project") -- v1.0
- ✓ Sync all assignments across all projects ("Sync All") -- v1.0
- ✓ Hash-based staleness detection for copy-mode targets -- v1.0
- ✓ Cross-filesystem fallback to copy mode (WSL2 ext4-to-NTFS) -- v1.0
- ✓ Sync operations serialized via SyncMutex -- v1.0
- ✓ Skill deletion cascades to project assignments and filesystem artifacts -- v1.0
- ✓ Schema V4/V5 migration with transaction wrapping -- v1.0
- ✓ Separate commands/projects.rs module for project commands -- v1.0
- ✓ Projects tab in main navigation -- v1.0
- ✓ Project list panel with add/edit/remove actions -- v1.0
- ✓ Assignment matrix with checkbox grid -- v1.0
- ✓ Projects tab isolated from App.tsx (own component tree + hook) -- v1.0
- ✓ Detect removed/renamed project directories with warning badge -- v1.0
- ✓ Prompt .gitignore entries on project registration -- v1.0
- ✓ Auto-sync toggle for global tool sync -- v1.0
- ✓ Missing status auto-recovery when source/target reappear -- v1.0

### Active

(None -- fresh for next milestone)

### Out of Scope

- Per-project skill versioning -- one version per skill, central library is single source of truth
- Fork/customize skills with upstream tracking -- deferred, no patch system
- Auto-sync / watch mode -- symlink mode already propagates changes instantly
- Mobile/web interface -- desktop-only via Tauri
- CLI companion for headless environments -- deferred future enhancement
- Skill grouping / presets ("Frontend Pack") -- deferred future enhancement
- Project templates (auto-assign skills for new project types) -- deferred
- Portable assignment manifests (cross-machine YAML export/import) -- deferred
- Syncing projects table across machines -- project paths are machine-specific
- "Clone assignments from another project" bulk action -- deferred to polish
- Sort by recently synced -- deferred to polish
- Chinese i18n for project features -- English only for now, ZH added later

## Context

### Current State

Shipped v1.0 Per-Project Skill Distribution (2026-04-09).

**Codebase:**

- Frontend: ~3,500 LOC TypeScript/React across 20+ components
- Backend: ~5,000 LOC Rust across 15+ modules, 130 Rust tests
- Tech stack: Tauri 2 + React 19 + Vite 7 + SQLite (rusqlite) + Tailwind CSS 4
- Schema version: V5 (projects, project_tools, project_skill_assignments + content_hash)

**Architecture:**

- App.tsx orchestrates global skill management (2087 lines, 50+ state vars)
- Projects tab is fully isolated: `src/components/projects/` with `useProjectState` hook
- Backend layered: `core/` (business logic) -> `commands/` (Tauri IPC wrappers)
- 19 Tauri IPC commands for project management, registered in `commands/projects.rs`
- SyncMutex serializes all filesystem-mutating operations

**Known Tech Debt (10 items from milestone audit):**

- Phase 5 missing VALIDATION.md (Nyquist gap)
- Chinese translations deferred for all project features
- EditProjectModal.tsx:42 silent catch on gitignore load
- Serialization test uses local mutex instead of Tauri-managed path
- No direct test for cross-filesystem symlink failure
- No direct test for unassign_and_cleanup removal-failure branch
- Contract tests validate prefix passthrough without actual Tauri command
- No direct test for duplicate-path in register_project wrapper
- EditProjectModal.tsx is unplanned scope not covered in phase plans
- UI-04 delivered as toolbar instead of bottom bar (functional equivalent)

## Key Decisions

| Decision                                   | Rationale                                  | Outcome |
| ------------------------------------------ | ------------------------------------------ | ------- |
| Build in current repo (not new fork)       | Repo IS the fork already, simpler workflow | ✓ Good  |
| User-configurable tool columns per project | Most users use 2-3 tools, not 47+          | ✓ Good  |
| Separate project_tools table               | Cleaner SQL queries than JSON array        | ✓ Good  |
| Prompt (not auto-add) .gitignore           | Respects user's git workflow               | ✓ Good  |
| Extract Projects tab as separate tree      | App.tsx at 2087 lines, must not grow       | ✓ Good  |
| English-only i18n this milestone           | Reduces scope, ZH can follow               | ✓ Good  |
| Symlink-first, copy as fallback            | Symlinks propagate instantly               | ✓ Good  |
| Callback injection for expand_home_path    | Core/ stays pure, no reverse dependency    | ✓ Good  |
| DTOs in core/project_ops.rs                | Both commands and tests can import         | ✓ Good  |
| V5 migration for content_hash column       | Incremental upgrade vs V4 fresh-install    | ✓ Good  |
| SyncMutex via Arc<Mutex<()>>               | Serializes all sync ops, poison-recovery   | ✓ Good  |
| Bottom-up build order                      | Each phase independently testable          | ✓ Good  |
| 6 phases at standard granularity           | Derived from requirement categories        | ✓ Good  |

## Constraints

- **Tech stack**: Tauri 2 + React 19 + Rust + SQLite -- no new frameworks
- **Sync engine**: Reuse existing `sync_engine.rs` primitives -- do not duplicate or modify
- **App.tsx**: Minimize changes -- new feature state stays in Projects component tree
- **Platform**: Must work on WSL2 (primary dev environment), macOS, and Linux
- **Backward compat**: Existing global sync must continue working alongside project sync

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):

1. Requirements invalidated? -> Move to Out of Scope with reason
2. Requirements validated? -> Move to Validated with phase reference
3. New requirements emerged? -> Add to Active
4. Decisions to log? -> Add to Key Decisions
5. "What This Is" still accurate? -> Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):

1. Full review of all sections
2. Core Value check -- still the right priority?
3. Audit Out of Scope -- reasons still valid?
4. Update Context with current state

---

_Last updated: 2026-04-09 after v1.0 milestone_
