# Project Retrospective

_A living document updated after each milestone. Lessons feed forward into future planning._

## Milestone: v1.0 -- Per-Project Skill Distribution

**Shipped:** 2026-04-09
**Phases:** 6 | **Plans:** 12

### What Was Built

- SQLite V4/V5 schema with full project data model (projects, tools, assignments) and 18+ CRUD methods
- Project sync engine reusing existing sync_engine.rs primitives -- zero changes to core sync logic
- 19 Tauri IPC commands in separate commands/projects.rs module with structured error prefixes
- Complete Projects tab UI (split-panel, assignment matrix, 4 modals) isolated from App.tsx
- Edge case handling: auto-sync toggle, missing project detection, .gitignore prompt, skill deletion cascade
- Gap closure: tool removal cascade and missing status detection closing all audit gaps

### What Worked

- **Bottom-up build order** (data -> sync -> IPC -> UI -> polish) made each phase independently testable and kept integration smooth
- **Reusing sync_engine.rs unchanged** -- the entire per-project feature reduced to "compute different target paths, call the same functions"
- **Isolating Projects tab** in its own component tree with useProjectState hook prevented App.tsx bloat
- **TDD in Phase 6** caught edge cases (orphaned skills, missing source recovery) that would have been hard to find otherwise
- **Milestone audit before completion** identified the TOOL-03/SYNC-01 gaps that Phase 6 closed -- without it, those would have shipped as silent bugs
- **Callback injection pattern** (expand_home_path as callback to core/) kept business logic testable without Tauri dependencies

### What Was Inefficient

- **SUMMARY.md one-liner extraction** by gsd-tools CLI picked up noise (commit messages, deviation bullet prefixes) instead of clean accomplishments -- had to manually rewrite MILESTONES.md
- **Phase 5 missing VALIDATION.md** -- Nyquist gap detected by audit but not backfilled
- **Worktree agent branching** sometimes based on main instead of feature HEAD, requiring types.ts recreation (Phase 4 Plan 1 deviation)

### Patterns Established

- `commands/projects.rs` as separate command module (vs adding to 986-line mod.rs) -- do this for future feature modules
- `useProjectState` hook pattern for feature-isolated state management -- avoids App.tsx growth
- Inner component mount pattern for modal state initialization (satisfies react-hooks lint without useRef hacks)
- SyncMutex for serializing all filesystem-mutating operations across concurrent IPC calls
- Error prefix infrastructure (DUPLICATE_PROJECT|, NOT_FOUND|, etc.) for structured backend-to-frontend error communication

### Key Lessons

1. **Milestone audit is non-negotiable** -- it found 2 partially-met requirements (TOOL-03, SYNC-01) that looked complete in phase summaries but had missing backend code paths
2. **Sync engine abstraction paid off** -- path-generic sync primitives meant per-project sync was purely a path-resolution problem, not a sync-engine rewrite
3. **Schema migrations need both fresh-install and upgrade paths** -- V5 migration caught a duplicate-column bug because fresh DDL already included the new column while ALTER TABLE also ran
4. **Frontend isolation via hooks works well** -- useProjectState kept the Projects tab self-contained without Redux/Zustand overhead

### Cost Observations

- Sessions: ~6 (across 2 days)
- Notable: Phase 1 (data foundation) took 9 minutes for 2 plans -- schema+CRUD is well-patterned work; Phase 4 (frontend, 3 plans) required the most iteration due to lint fixes and modal wiring

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Phases | Plans | Key Change                                         |
| --------- | ------ | ----- | -------------------------------------------------- |
| v1.0      | 6      | 12    | Bottom-up build order, milestone audit before ship |

### Cumulative Quality

| Milestone | Rust Tests | Frontend Tests | Zero-Dep Additions |
| --------- | ---------- | -------------- | ------------------ |
| v1.0      | 130        | 0 (no runner)  | 0 new deps         |

### Top Lessons (Verified Across Milestones)

1. Milestone audit catches gaps that phase-level summaries miss -- always run before shipping
2. Path-generic sync abstractions enable feature growth without engine rewrites
