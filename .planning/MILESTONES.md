# Milestones

## v1.0 Per-Project Skill Distribution (Shipped: 2026-04-09)

**Phases completed:** 6 phases, 12 plans, 19 tasks
**Timeline:** 2 days (Apr 7-8, 2026)
**Source changes:** 30 files, +9,568/-1,665 lines
**Tests:** 130 Rust tests (43 new)

**Delivered:** Per-project skill distribution -- register project directories, assign skills per tool via checkbox matrix, sync via symlinks to project-local tool directories.

**Key accomplishments:**

1. SQLite V4/V5 schema with project data model (3 tables, 18+ CRUD methods, aggregate sync status, cascade deletes)
2. Project sync engine with assign/unassign, SyncMutex concurrency, hash-based staleness detection, and missing status auto-recovery
3. 19 Tauri IPC commands for full project management, bulk-assign, resync, and structured error prefixes
4. Complete Projects tab UI with split-panel layout, assignment matrix, 4 modal dialogs, per-cell status colors
5. Edge case handling: auto-sync toggle, missing project detection, update-path flow, .gitignore prompt, skill deletion cascade
6. Gap closure: tool removal cascades to assignments/artifacts, missing status detection for absent skill sources

**Git range:** `a7c49c8..c51112a`

---
