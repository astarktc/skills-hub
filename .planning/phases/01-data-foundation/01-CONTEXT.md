# Phase 1: Data Foundation - Context

**Gathered:** 2026-04-07
**Status:** Ready for planning

<domain>
## Phase Boundary

Backend data layer for project management: Schema V4 migration creating 3 new tables, CRUD functions in `skill_store.rs`, and a separate `commands/projects.rs` module. No UI, no sync logic beyond what delete cleanup requires.

</domain>

<decisions>
## Implementation Decisions

### Project Identity

- **D-01:** Projects derive display name from directory basename (`basename(path)`). No separate `name` column. Frontend (Phase 4) renders basename as title and truncated path as subtitle.
- **D-02:** Projects use UUID `TEXT PRIMARY KEY`, matching existing `SkillRecord` and `SkillTargetRecord` pattern (`Uuid::new_v4().to_string()`).

### Schema Design

- **D-03:** Full columns upfront — include status, sync metadata, mode, last_error, synced_at, and timestamps in the initial V4 migration to avoid churn from V5/V6 migrations in later phases.
- **D-04:** `project_skill_assignments` mirrors the existing `skill_targets` pattern: `status TEXT NOT NULL` (pending/synced/stale/error), `mode TEXT NOT NULL` (symlink/copy), `last_error TEXT NULL`, `synced_at INTEGER NULL`, `created_at INTEGER NOT NULL`.
- **D-05:** Standard indexes created upfront: `idx_psa_project` on `project_skill_assignments(project_id)`, `idx_psa_skill` on `project_skill_assignments(skill_id)`, `idx_pt_project` on `project_tools(project_id)`. Path uniqueness on `projects(path)` is covered by UNIQUE constraint.

### Schema Tables (V4)

```sql
-- projects
id TEXT PRIMARY KEY
path TEXT NOT NULL UNIQUE
created_at INTEGER NOT NULL
updated_at INTEGER NOT NULL

-- project_tools
id TEXT PRIMARY KEY
project_id TEXT NOT NULL  (FK -> projects ON DELETE CASCADE)
tool TEXT NOT NULL         (ToolId string)
UNIQUE(project_id, tool)

-- project_skill_assignments
id TEXT PRIMARY KEY
project_id TEXT NOT NULL  (FK -> projects ON DELETE CASCADE)
skill_id TEXT NOT NULL    (FK -> skills ON DELETE CASCADE)
tool TEXT NOT NULL        (ToolId string)
mode TEXT NOT NULL        (symlink|copy)
status TEXT NOT NULL      (pending|synced|stale|error)
last_error TEXT NULL
synced_at INTEGER NULL
created_at INTEGER NOT NULL
UNIQUE(project_id, skill_id, tool)
```

### Delete Behavior

- **D-06:** Hard delete with `ON DELETE CASCADE` on all foreign keys, matching existing `skills` → `skill_targets` pattern.
- **D-07:** Delete function performs full cleanup: reads assignment paths before DB delete, removes symlinks/copies from project directories using existing sync engine primitives, then cascade-deletes DB rows. This pulls in a small sync engine dependency but keeps the delete operation complete.

### Path Storage

- **D-08:** Project paths are fully canonicalized before storage — `~` expanded, symlinks resolved, relative paths made absolute. Reuses existing `expand_home_path()` from `commands/mod.rs` plus `std::fs::canonicalize()`.
- **D-09:** Registration validates directory exists on disk (`path.exists() && path.is_dir()`). Also checks for duplicate registration against canonical path. Phase 5 handles directories that go missing after registration.

### Claude's Discretion

- Exact CRUD function signatures and internal structuring of `commands/projects.rs`
- Whether to add helper methods on `SkillStore` or create a separate `ProjectStore` struct (recommend extending `SkillStore` for consistency)
- Test structure and coverage scope for the new functions

</decisions>

<canonical_refs>

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Data Layer

- `src-tauri/src/core/skill_store.rs` — Existing schema, migration pattern (V1→V3), SkillRecord/SkillTargetRecord structs, with_conn() helper, CRUD patterns
- `src-tauri/src/core/mod.rs` — Module exports, new modules must be registered here

### Command Layer

- `src-tauri/src/commands/mod.rs` — Existing command module structure, DTO patterns, format_anyhow_error(), expand_home_path()
- `src-tauri/src/lib.rs` — Command registration via generate_handler![], SkillStore state injection

### Sync (for delete cleanup)

- `src-tauri/src/core/sync_engine.rs` — sync_dir_hybrid, copy_dir_recursive, SyncMode — delete cleanup needs to remove symlinks/copies

### Testing

- `src-tauri/src/core/tests/skill_store.rs` — Existing store test patterns
- `src-tauri/src/commands/tests/commands.rs` — Existing command test patterns

### Project Docs

- `.planning/PROJECT.md` — Constraints, key decisions, brownfield context
- `.planning/REQUIREMENTS.md` — INFR-04, INFR-05, PROJ-01, PROJ-02, PROJ-03, TOOL-01 definitions

</canonical_refs>

<code_context>

## Existing Code Insights

### Reusable Assets

- `SkillStore::with_conn()` — Connection helper with PRAGMA foreign_keys enforcement, reuse for all new queries
- `expand_home_path()` in `commands/mod.rs` — Home directory expansion, extend with canonicalization for project paths
- `format_anyhow_error()` — Error formatting chain, reuse in new command module
- `content_hash.rs` — `hash_dir()` for staleness detection in later phases
- `sync_engine.rs` — `sync_dir_hybrid()` and removal logic for delete cleanup

### Established Patterns

- Schema migration: `PRAGMA user_version` check → incremental `ALTER TABLE` / `CREATE TABLE` per version
- CRUD naming: `upsert_*/list_*/get_*_by_id/delete_*`
- DTO structs: Rust `#[derive(Serialize)]` structs in command module, mirrored in `src/components/skills/types.ts`
- Command wrapping: `tauri::async_runtime::spawn_blocking` for blocking operations
- State access: `State<'_, SkillStore>` parameter in Tauri commands

### Integration Points

- `lib.rs:71` — `generate_handler![]` macro needs new project commands registered
- `lib.rs:1` — `mod commands;` already exists, new `commands/projects.rs` needs sub-module declaration
- `core/mod.rs` — If new core module created, export it here

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches following existing codebase patterns.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

_Phase: 01-data-foundation_
_Context gathered: 2026-04-07_
