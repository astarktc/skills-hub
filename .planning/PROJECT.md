# Skills Hub — Per-Project Skill Distribution

## What This Is

Skills Hub is a cross-platform desktop app (Tauri 2 + React 19) for managing AI Agent Skills and syncing them to 47+ AI coding tools. This milestone adds per-project skill distribution: register project directories, assign specific skills to specific projects, and sync via symlinks from `~/.skillshub/<skill>` to `<project>/.claude/skills/<skill>` (or equivalent tool path).

## Core Value

Any skill assigned to a project is immediately available in that project's tool directory via symlink, so AI tools only load the skills that matter for that project — not the entire library.

## Requirements

### Validated

<!-- Shipped and confirmed valuable. Inferred from existing codebase. -->

- ✓ Install skills from local directories — existing
- ✓ Install skills from git repositories — existing
- ✓ Manage skills in central library (`~/.skillshub/`) — existing
- ✓ Sync skills to 47+ AI tool global directories via symlink/junction/copy — existing
- ✓ Detect installed AI tools and show sync status — existing
- ✓ Onboarding: discover existing skills across tool directories — existing
- ✓ Update skills from source (local or git) — existing
- ✓ Browse and search online skill repositories — existing
- ✓ View skill file contents with markdown/code rendering — existing
- ✓ Bilingual UI (English/Chinese) — existing
- ✓ Auto-update via GitHub Releases — existing
- ✓ Cross-platform: macOS, Windows, Linux — existing
- ✓ SQLite-backed persistence with schema migrations — existing
- ✓ Register project directories via folder picker (with manual path fallback) — Validated in Phase 4
- ✓ Assign skills to projects per tool via checkbox matrix UI — Validated in Phase 4
- ✓ User-configurable tool columns per project (pick which tools appear in matrix) — Validated in Phase 4
- ✓ "All Tools" bulk-assign button (checks all user-configured tools for a skill) — Validated in Phase 4
- ✓ Sync all assignments for a single project ("Sync Project" button) — Validated in Phase 4
- ✓ Sync all assignments across all projects ("Sync All" button) — Validated in Phase 4
- ✓ Per-cell status indicators: synced (green), stale (yellow), missing (red), pending (gray) — Validated in Phase 4
- ✓ Prompt user to add tool skill directories to project's .gitignore on registration — Validated in Phase 4

### Active

<!-- Per-project skill distribution — remaining items for Phase 5 polish. -->

- [ ] Remove registered projects (cleans up all synced symlinks/copies)
- [ ] Unassign skills from projects (removes symlink/copy from project directory)
- [ ] Content hash staleness detection for copy-mode targets
- [ ] Search/filter bar in the assignment matrix for large skill libraries
- [ ] Handle removed/renamed project directories gracefully (detect on list, show warning)
- [ ] Handle skills removed from central library (orphaned assignments marked "missing")
- [ ] Global sync (existing feature) continues to work alongside project sync
- [ ] Cross-platform symlink testing (Windows NTFS via WSL, macOS, native Linux)

### Out of Scope

- Per-project skill versioning — one version per skill, central library is single source of truth
- Fork/customize skills with upstream tracking — deferred, no patch system
- Auto-sync / watch mode — symlink mode already propagates changes instantly
- Mobile/web interface — desktop-only via Tauri
- Chinese i18n for new features — English only this milestone, ZH added later
- CLI companion for headless environments — deferred future enhancement
- Skill grouping / presets ("Frontend Pack") — deferred future enhancement
- Project templates (auto-assign skills for new project types) — deferred
- Portable assignment manifests (cross-machine YAML export/import) — deferred
- Syncing projects table across machines — project paths are machine-specific
- "Clone assignments from another project" bulk action — deferred to polish
- Sort by recently synced — deferred to polish

## Context

### Brownfield State

This is an active codebase with a working app (v0.4.2). The existing sync engine (`sync_engine.rs`) accepts generic `source: &Path` and `target: &Path` — only the path resolution layer is global-hardcoded. Per-project sync requires zero changes to the sync engine itself: just project-aware path resolution, assignment storage, and new UI.

### Key Architectural Insight

The sync primitives are path-generic. Changing WHERE to sync (project-local vs global) requires:

1. Project-aware path resolution (project_path + tool_relative_skills_dir + skill_name)
2. Assignment storage (new SQLite tables)
3. Calling the same sync primitives with project-local target paths

### Frontend State

`App.tsx` is 2087 lines with 50+ state variables. The Projects tab will be a fully separate component tree (`src/components/projects/`) with its own state management to avoid further bloating App.tsx. Minimal changes to App.tsx — just adding the tab to navigation.

### Data Model Additions

New SQLite tables via Schema V4 migration:

- `projects` — registered project directories
- `project_tools` — which tools are configured per project (drives matrix columns)
- `project_skill_assignments` — which skills are assigned to which projects, per tool

These coexist with existing `skill_targets` table (global sync). A skill can be synced globally AND to specific projects.

## Constraints

- **Tech stack**: Tauri 2 + React 19 + Rust + SQLite — no new frameworks
- **Sync engine**: Reuse existing `sync_engine.rs` primitives — do not duplicate or modify
- **App.tsx**: Minimize changes — new feature state stays in Projects component tree
- **i18n**: English strings only — Chinese deferred
- **Platform**: Must work on WSL2 (primary dev environment), macOS, and Linux
- **Backward compat**: Existing global sync must continue working alongside project sync

## Key Decisions

| Decision                                            | Rationale                                                                      | Outcome     |
| --------------------------------------------------- | ------------------------------------------------------------------------------ | ----------- |
| Build in current repo (not new fork)                | Repo IS the fork already, simpler workflow                                     | — Validated |
| User-configurable tool columns per project          | Most users use 2-3 tools, not 47+ — avoids unwieldy matrix                     | — Validated |
| Separate project_tools table for tool column config | Cleaner than JSON array in projects table, better SQL queries                  | — Validated |
| Prompt (not auto-add) for .gitignore entries        | Respects user's git workflow preferences                                       | — Validated |
| Extract Projects tab as separate component tree     | App.tsx already at 2087 lines, must not grow further                           | — Validated |
| English-only i18n this milestone                    | Reduces scope, ZH can be added as a follow-up pass                             | — Validated |
| Symlink-first, copy as fallback                     | Symlinks propagate source changes instantly, copy for incompatible filesystems | — Validated |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):

1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):

1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---

_Last updated: 2026-04-08 after Phase 3 (IPC Commands) completion_
