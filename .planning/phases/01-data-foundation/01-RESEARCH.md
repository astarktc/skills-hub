# Phase 1: Data Foundation - Research

**Researched:** 2026-04-07
**Domain:** SQLite schema migration, Rust CRUD data layer, Tauri IPC command module structure
**Confidence:** HIGH

## Summary

Phase 1 is a backend-only data layer phase that adds three new SQLite tables (`projects`, `project_tools`, `project_skill_assignments`) via a Schema V4 migration, implements CRUD operations as methods on the existing `SkillStore` struct, and exposes them through a new `commands/projects.rs` Tauri command module. No frontend changes, no sync logic (except delete cleanup which removes deployed symlinks/copies).

The existing codebase provides strong patterns to follow. The schema migration pattern (`PRAGMA user_version` check with incremental `ALTER TABLE` / `CREATE TABLE` branches), the CRUD naming convention (`upsert_*/list_*/get_*_by_id/delete_*`), the command wrapping pattern (`spawn_blocking` + `format_anyhow_error`), and the test infrastructure (`tempfile` temp dirs, `make_store()` helper) are all well-established and should be replicated exactly.

**Primary recommendation:** Follow existing patterns exactly -- extend `SkillStore` with new methods, add `commands/projects.rs` as a sub-module of `commands/`, and transaction-wrap the V4 migration using SQLite `execute_batch` with explicit `BEGIN/COMMIT` statements.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Projects derive display name from directory basename (`basename(path)`). No separate `name` column. Frontend (Phase 4) renders basename as title and truncated path as subtitle.
- **D-02:** Projects use UUID `TEXT PRIMARY KEY`, matching existing `SkillRecord` and `SkillTargetRecord` pattern (`Uuid::new_v4().to_string()`).
- **D-03:** Full columns upfront -- include status, sync metadata, mode, last_error, synced_at, and timestamps in the initial V4 migration to avoid churn from V5/V6 migrations in later phases.
- **D-04:** `project_skill_assignments` mirrors the existing `skill_targets` pattern: `status TEXT NOT NULL` (pending/synced/stale/error), `mode TEXT NOT NULL` (symlink/copy), `last_error TEXT NULL`, `synced_at INTEGER NULL`, `created_at INTEGER NOT NULL`.
- **D-05:** Standard indexes created upfront: `idx_psa_project` on `project_skill_assignments(project_id)`, `idx_psa_skill` on `project_skill_assignments(skill_id)`, `idx_pt_project` on `project_tools(project_id)`. Path uniqueness on `projects(path)` is covered by UNIQUE constraint.
- **D-06:** Hard delete with `ON DELETE CASCADE` on all foreign keys, matching existing `skills` -> `skill_targets` pattern.
- **D-07:** Delete function performs full cleanup: reads assignment paths before DB delete, removes symlinks/copies from project directories using existing sync engine primitives, then cascade-deletes DB rows.
- **D-08:** Project paths are fully canonicalized before storage -- `~` expanded, symlinks resolved, relative paths made absolute. Reuses existing `expand_home_path()` from `commands/mod.rs` plus `std::fs::canonicalize()`.
- **D-09:** Registration validates directory exists on disk (`path.exists() && path.is_dir()`). Also checks for duplicate registration against canonical path. Phase 5 handles directories that go missing after registration.

### Claude's Discretion

- Exact CRUD function signatures and internal structuring of `commands/projects.rs`
- Whether to add helper methods on `SkillStore` or create a separate `ProjectStore` struct (recommend extending `SkillStore` for consistency)
- Test structure and coverage scope for the new functions

### Deferred Ideas (OUT OF SCOPE)

None -- discussion stayed within phase scope.

</user_constraints>

<phase_requirements>

## Phase Requirements

| ID      | Description                                                                                                      | Research Support                                                                                                                                                        |
| ------- | ---------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| INFR-04 | Schema V4 migration adds projects, project_tools, and project_skill_assignments tables with transaction wrapping | Migration pattern documented in Architecture Patterns; transaction wrapping via `execute_batch("BEGIN; ... COMMIT;")` verified against rusqlite API                     |
| INFR-05 | New Tauri IPC commands are in a separate `commands/projects.rs` module (not in existing `commands/mod.rs`)       | Sub-module pattern documented; requires `pub mod projects;` in `commands/mod.rs` and handler registration in `lib.rs`                                                   |
| PROJ-01 | User can register a project directory via folder picker (with manual path entry fallback)                        | Backend: CRUD for project registration with path canonicalization; Frontend picker is Phase 4 scope, but the core `register_project` function is Phase 1                |
| PROJ-02 | User can remove a registered project (cleans up all deployed symlinks/copies in the project directory)           | Delete with cleanup pattern documented; requires reading assignment paths before cascade delete, then removing filesystem artifacts via sync engine's `remove_path_any` |
| PROJ-03 | User can see all registered projects in a list with assignment counts and aggregate sync status                  | Backend: `list_projects` query with JOIN counts; actual UI rendering is Phase 4, but the data retrieval function is Phase 1                                             |
| TOOL-01 | User can configure which tool columns appear in the assignment matrix per project                                | Backend: `project_tools` CRUD (add_project_tool, remove_project_tool, list_project_tools); tool string is `ToolId::as_key()` value                                      |

</phase_requirements>

## Standard Stack

### Core

| Library  | Version        | Purpose                            | Why Standard                                                                               |
| -------- | -------------- | ---------------------------------- | ------------------------------------------------------------------------------------------ |
| rusqlite | 0.31 (bundled) | SQLite database operations         | Already in use; `bundled` feature means no system SQLite dependency [VERIFIED: Cargo.toml] |
| uuid     | 1.x (v4)       | Primary key generation             | Already in use for `SkillRecord` and `SkillTargetRecord` IDs [VERIFIED: Cargo.toml]        |
| anyhow   | 1.0            | Error handling with context chains | Already in use throughout the codebase [VERIFIED: Cargo.toml]                              |
| serde    | 1.0 (derive)   | DTO serialization for Tauri IPC    | Already in use for all command DTOs [VERIFIED: Cargo.toml]                                 |
| dirs     | 5.0            | Home directory resolution          | Already in use for path expansion [VERIFIED: Cargo.toml]                                   |

### Supporting (Dev)

| Library  | Version | Purpose                    | When to Use                                                         |
| -------- | ------- | -------------------------- | ------------------------------------------------------------------- |
| tempfile | 3       | Temp directories for tests | All store and command tests [VERIFIED: Cargo.toml dev-dependencies] |

### Alternatives Considered

No alternatives needed. This phase uses exclusively existing dependencies -- zero new crates required.

## Architecture Patterns

### Recommended Project Structure

Changes are confined to these files:

```
src-tauri/src/
  core/
    skill_store.rs          # MODIFIED: V4 migration + new record structs + CRUD methods
  commands/
    mod.rs                  # MODIFIED: add `pub mod projects;` declaration
    projects.rs             # NEW: project-related Tauri commands and DTOs
    tests/
      commands.rs           # MODIFIED: (optional) any shared test helpers
      projects.rs           # NEW: tests for project commands
  lib.rs                    # MODIFIED: register new commands in generate_handler!
```

No new core modules needed -- all data operations extend `SkillStore`. [VERIFIED: existing pattern in skill_store.rs]

### Pattern 1: Schema V4 Migration (Transaction-Wrapped DDL)

**What:** Add three new tables and three indexes in a single atomic migration step.
**When to use:** When `user_version < 4` in the incremental migration block of `ensure_schema()`.
**Critical detail:** The existing migration does NOT use explicit transactions -- V1-V3 migrations use individual `execute_batch` calls. V4 MUST use `BEGIN/COMMIT` wrapping per CONTEXT.md decision. [VERIFIED: ensure_schema() source code]

**Example:**

```rust
// Source: skill_store.rs ensure_schema() pattern + rusqlite docs
if user_version < 4 {
    conn.execute_batch(
        "BEGIN;
         CREATE TABLE IF NOT EXISTS projects (
           id TEXT PRIMARY KEY,
           path TEXT NOT NULL UNIQUE,
           created_at INTEGER NOT NULL,
           updated_at INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS project_tools (
           id TEXT PRIMARY KEY,
           project_id TEXT NOT NULL,
           tool TEXT NOT NULL,
           UNIQUE(project_id, tool),
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
         );
         CREATE TABLE IF NOT EXISTS project_skill_assignments (
           id TEXT PRIMARY KEY,
           project_id TEXT NOT NULL,
           skill_id TEXT NOT NULL,
           tool TEXT NOT NULL,
           mode TEXT NOT NULL,
           status TEXT NOT NULL,
           last_error TEXT NULL,
           synced_at INTEGER NULL,
           created_at INTEGER NOT NULL,
           UNIQUE(project_id, skill_id, tool),
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(skill_id) REFERENCES skills(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_psa_project ON project_skill_assignments(project_id);
         CREATE INDEX IF NOT EXISTS idx_psa_skill ON project_skill_assignments(skill_id);
         CREATE INDEX IF NOT EXISTS idx_pt_project ON project_tools(project_id);
         COMMIT;"
    )?;
}
```

**Why `execute_batch` instead of `conn.transaction()`:** The `with_conn` helper provides `&Connection` (not `&mut Connection`), and `Connection::transaction()` requires `&mut`. Using `execute_batch` with explicit `BEGIN/COMMIT` achieves the same atomicity without refactoring the connection helper. SQLite DDL statements are transactional. [VERIFIED: with_conn signature in skill_store.rs, rusqlite docs]

### Pattern 2: Record Structs for New Tables

**What:** Rust structs mirroring each table's columns, used for CRUD parameter passing.
**When to use:** For every new table.

**Example:**

```rust
// Source: existing SkillRecord/SkillTargetRecord pattern in skill_store.rs
#[derive(Clone, Debug)]
pub struct ProjectRecord {
    pub id: String,
    pub path: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug)]
pub struct ProjectToolRecord {
    pub id: String,
    pub project_id: String,
    pub tool: String,
}

#[derive(Clone, Debug)]
pub struct ProjectSkillAssignmentRecord {
    pub id: String,
    pub project_id: String,
    pub skill_id: String,
    pub tool: String,
    pub mode: String,
    pub status: String,
    pub last_error: Option<String>,
    pub synced_at: Option<i64>,
    pub created_at: i64,
}
```

### Pattern 3: Tauri Command Module (commands/projects.rs)

**What:** Separate file for project-related Tauri commands, following the same patterns as `commands/mod.rs`.
**When to use:** INFR-05 requires this separation.

**Key structural elements:**

1. Import `SkillStore` from `crate::core::skill_store`
2. Import shared helpers: `format_anyhow_error` and `expand_home_path` must be accessible from the new module (currently private in `commands/mod.rs` -- need to make `pub(crate)` or extract)
3. Define DTO structs with `#[derive(Debug, Serialize)]`
4. Use `#[tauri::command]` + `spawn_blocking` pattern
5. Use `State<'_, SkillStore>` for state access
6. Parameter names use camelCase for Tauri IPC compatibility

**Example:**

```rust
// Source: commands/mod.rs pattern
use serde::Serialize;
use tauri::State;
use crate::core::skill_store::SkillStore;

// Re-use from commands/mod.rs (must be made pub(crate))
use super::{format_anyhow_error, now_ms};

#[derive(Debug, Serialize)]
pub struct ProjectDto {
    pub id: String,
    pub path: String,
    pub name: String,  // derived from basename(path)
    pub created_at: i64,
    pub updated_at: i64,
    pub tool_count: usize,
    pub assignment_count: usize,
}

#[tauri::command]
pub async fn register_project(
    store: State<'_, SkillStore>,
    path: String,
) -> Result<ProjectDto, String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        // canonicalize, validate, insert
        // ...
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(format_anyhow_error)
}
```

### Pattern 4: Command Registration in lib.rs

**What:** New commands must be added to the `generate_handler![]` macro in `lib.rs`.
**When to use:** After creating any new `#[tauri::command]` function.

**Example:**

```rust
// Source: lib.rs lines 71-102
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    commands::projects::register_project,
    commands::projects::remove_project,
    commands::projects::list_projects,
    commands::projects::add_project_tool,
    commands::projects::remove_project_tool,
    commands::projects::list_project_tools,
    commands::projects::add_project_skill_assignment,
    commands::projects::remove_project_skill_assignment,
    commands::projects::list_project_skill_assignments,
])
```

### Pattern 5: Delete with Filesystem Cleanup (D-07)

**What:** Before DB cascade delete, read assignment paths, remove filesystem artifacts, then delete the DB row.
**When to use:** When removing a project (similar to existing `delete_managed_skill` pattern).

**Example:**

```rust
// Source: delete_managed_skill in commands/mod.rs (lines 750-791)
// 1. List all project_skill_assignments for the project
// 2. For each assignment with status "synced", compute target path and remove via remove_path_any
// 3. Delete from projects table (CASCADE handles project_tools and assignments)
```

The `remove_path_any` function in `commands/mod.rs` handles symlinks, directories, and files. It is currently a private function -- needs to be made `pub(crate)` for the projects module to use it. [VERIFIED: remove_path_any source at line 793]

### Anti-Patterns to Avoid

- **Creating a separate ProjectStore struct:** Adds unnecessary complexity. Extend `SkillStore` with project methods -- it already has the `with_conn` helper and db_path. [ASSUMED: recommendation based on codebase analysis]
- **Using `conn.transaction()` instead of `execute_batch("BEGIN...COMMIT")`:** Would require changing `with_conn` signature from `&Connection` to `&mut Connection`, breaking all existing callers. [VERIFIED: with_conn signature]
- **Adding project state to App.tsx:** Phase 1 is backend-only. Frontend is Phase 4 scope. [VERIFIED: CONTEXT.md phase boundary]
- **Modifying sync_engine.rs:** Only use its existing functions (`remove_path_any` pattern). The sync engine is deliberately unchanged per project constraints. [VERIFIED: PROJECT.md constraints]
- **Storing display name in DB:** D-01 says derive from `basename(path)` at query time. No `name` column. [VERIFIED: CONTEXT.md D-01]

## Don't Hand-Roll

| Problem               | Don't Build                 | Use Instead                                      | Why                                                                                        |
| --------------------- | --------------------------- | ------------------------------------------------ | ------------------------------------------------------------------------------------------ |
| UUID generation       | Custom ID scheme            | `Uuid::new_v4().to_string()`                     | Matches existing pattern, collision-resistant [VERIFIED: commands/mod.rs usage]            |
| Path canonicalization | Custom path normalization   | `expand_home_path()` + `std::fs::canonicalize()` | Handles `~`, symlinks, relative paths correctly [VERIFIED: CONTEXT.md D-08]                |
| Symlink/copy removal  | Custom filesystem cleanup   | Existing `remove_path_any` pattern               | Handles symlinks, dirs, files, and missing paths [VERIFIED: commands/mod.rs line 793]      |
| Schema migration      | Manual SQL version tracking | Existing `PRAGMA user_version` pattern           | Proven incremental migration system [VERIFIED: skill_store.rs ensure_schema()]             |
| Timestamp generation  | Custom time functions       | Existing `now_ms()` helper                       | Millisecond Unix timestamps matching existing columns [VERIFIED: commands/mod.rs line 826] |

**Key insight:** This phase should introduce zero new utility functions for common operations. Every helper needed already exists in the codebase.

## Common Pitfalls

### Pitfall 1: Foreign Key Enforcement Per-Connection

**What goes wrong:** `ON DELETE CASCADE` silently does nothing because foreign keys are disabled.
**Why it happens:** SQLite disables foreign key enforcement by default. It must be enabled per-connection with `PRAGMA foreign_keys = ON`.
**How to avoid:** The existing `with_conn` helper already sets this pragma on every connection. All new CRUD methods MUST use `self.with_conn(|conn| ...)` -- never open a raw connection.
**Warning signs:** Delete operations succeed but related rows remain in child tables. [VERIFIED: with_conn at line 452-458]

### Pitfall 2: Transaction Wrapping for Multi-Statement DDL

**What goes wrong:** Migration partially completes (e.g., `projects` table created but `project_skill_assignments` fails), leaving the database in an inconsistent state with `user_version` still at 3.
**Why it happens:** Without `BEGIN/COMMIT`, each `CREATE TABLE` is auto-committed individually. If one fails mid-way, the schema is half-migrated.
**How to avoid:** Wrap all V4 DDL in a single `execute_batch("BEGIN; ... COMMIT;")` block. Do NOT update `user_version` until after the transaction commits.
**Warning signs:** `user_version` is 3 but some V4 tables exist (or don't). [VERIFIED: CONTEXT.md requires transaction wrapping]

### Pitfall 3: Path Canonicalization Race Condition

**What goes wrong:** `std::fs::canonicalize()` fails if the directory doesn't exist yet, or resolves differently on different calls if symlinks change.
**Why it happens:** `canonicalize()` requires the path to exist on disk and resolves all symlinks at call time.
**How to avoid:** Validate existence first (`path.exists() && path.is_dir()` per D-09), then canonicalize. Store the canonicalized path. Check for duplicate registration against stored canonical paths.
**Warning signs:** Registration fails with "No such file or directory" or duplicate projects with different path representations. [VERIFIED: CONTEXT.md D-08, D-09]

### Pitfall 4: `commands/mod.rs` Private Helpers

**What goes wrong:** `commands/projects.rs` cannot call `format_anyhow_error`, `expand_home_path`, `now_ms`, or `remove_path_any` because they are private to `commands/mod.rs`.
**Why it happens:** These functions are defined without `pub` visibility in `mod.rs`.
**How to avoid:** Change visibility to `pub(crate)` for the functions needed by `projects.rs`: `format_anyhow_error`, `expand_home_path`, `now_ms`, `remove_path_any`. This is the minimal change -- `pub(crate)` keeps them crate-internal.
**Warning signs:** Compilation errors referencing private functions. [VERIFIED: all four functions are defined without pub in commands/mod.rs]

### Pitfall 5: SCHEMA_VERSION Constant Must Be Bumped

**What goes wrong:** Existing users who already have `user_version = 3` never trigger the V4 migration because `SCHEMA_VERSION` still equals 3.
**Why it happens:** Forgetting to update the `SCHEMA_VERSION` constant at the top of `skill_store.rs`.
**How to avoid:** Change `const SCHEMA_VERSION: i32 = 3;` to `const SCHEMA_VERSION: i32 = 4;`. Add the `if user_version < 4` block in the migration ladder.
**Warning signs:** App runs fine for new installs but existing users don't get the new tables. [VERIFIED: SCHEMA_VERSION const at line 11]

### Pitfall 6: Tauri Command Parameter CamelCase

**What goes wrong:** Frontend `invoke('command_name', { projectId: '...' })` silently passes `undefined` because the Rust parameter is `project_id` instead of `projectId`.
**Why it happens:** Tauri IPC expects parameter names to match exactly between frontend and backend. Rust convention is snake_case but Tauri commands need camelCase for JS compatibility.
**How to avoid:** Use `#[allow(non_snake_case)]` attribute and camelCase parameter names in command functions, matching the existing pattern (e.g., `skillId`, `sourcePath`).
**Warning signs:** Commands return default/error values when called from frontend. [VERIFIED: existing pattern in commands/mod.rs, e.g., line 339-340]

### Pitfall 7: Forgetting to Register Commands in generate_handler!

**What goes wrong:** Frontend calls to new project commands fail with "command not found" errors.
**Why it happens:** Tauri requires explicit command registration in the `generate_handler!` macro.
**How to avoid:** Every new `#[tauri::command]` function in `commands/projects.rs` must be added to the handler list in `lib.rs` using the `commands::projects::function_name` path.
**Warning signs:** Runtime "unknown command" errors from the Tauri bridge. [VERIFIED: lib.rs generate_handler! at lines 71-102]

## Code Examples

### CRUD Method: Register Project

```rust
// Source: Follows upsert_skill pattern from skill_store.rs lines 171-214
pub fn register_project(&self, record: &ProjectRecord) -> Result<()> {
    self.with_conn(|conn| {
        conn.execute(
            "INSERT INTO projects (id, path, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![record.id, record.path, record.created_at, record.updated_at],
        )?;
        Ok(())
    })
}
```

### CRUD Method: List Projects with Counts

```rust
// Source: Follows list_skills pattern from skill_store.rs lines 245-278
pub fn list_projects(&self) -> Result<Vec<ProjectRecord>> {
    self.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, path, created_at, updated_at
             FROM projects
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ProjectRecord {
                id: row.get(0)?,
                path: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    })
}
```

### CRUD Method: Add Tool to Project

```rust
// Source: Follows upsert_skill_target pattern from skill_store.rs lines 216-243
pub fn add_project_tool(&self, record: &ProjectToolRecord) -> Result<()> {
    self.with_conn(|conn| {
        conn.execute(
            "INSERT OR IGNORE INTO project_tools (id, project_id, tool)
             VALUES (?1, ?2, ?3)",
            params![record.id, record.project_id, record.tool],
        )?;
        Ok(())
    })
}
```

### CRUD Method: Get Project by Path (Duplicate Check)

```rust
// Source: Follows get_skill_by_id pattern from skill_store.rs lines 280-311
pub fn get_project_by_path(&self, path: &str) -> Result<Option<ProjectRecord>> {
    self.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, path, created_at, updated_at
             FROM projects WHERE path = ?1 LIMIT 1",
        )?;
        let mut rows = stmt.query(params![path])?;
        if let Some(row) = rows.next()? {
            Ok(Some(ProjectRecord {
                id: row.get(0)?,
                path: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            }))
        } else {
            Ok(None)
        }
    })
}
```

### Command: Project Registration with Validation

```rust
// Source: Follows set_central_repo_path pattern from commands/mod.rs lines 279-336
#[tauri::command]
pub async fn register_project(
    store: State<'_, SkillStore>,
    path: String,
) -> Result<ProjectDto, String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        // D-08: Canonicalize path
        let expanded = expand_home_path(&path)?;
        let canonical = std::fs::canonicalize(&expanded)
            .with_context(|| format!("failed to canonicalize {:?}", expanded))?;

        // D-09: Validate directory exists
        if !canonical.is_dir() {
            anyhow::bail!("path is not a directory: {:?}", canonical);
        }

        let path_str = canonical.to_string_lossy().to_string();

        // D-09: Check for duplicate
        if store.get_project_by_path(&path_str)?.is_some() {
            anyhow::bail!("project already registered: {:?}", path_str);
        }

        let now = now_ms();
        let record = ProjectRecord {
            id: Uuid::new_v4().to_string(),
            path: path_str.clone(),
            created_at: now,
            updated_at: now,
        };
        store.register_project(&record)?;

        // D-01: Derive name from basename
        let name = canonical
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path_str.clone());

        Ok::<_, anyhow::Error>(ProjectDto {
            id: record.id,
            path: path_str,
            name,
            created_at: record.created_at,
            updated_at: record.updated_at,
            tool_count: 0,
            assignment_count: 0,
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(format_anyhow_error)
}
```

### Test: Migration Creates Tables

```rust
// Source: Follows make_store pattern from tests/skill_store.rs
#[test]
fn v4_migration_creates_project_tables() {
    let (_dir, store) = make_store();

    // Verify tables exist by inserting and querying
    let record = ProjectRecord {
        id: "p1".to_string(),
        path: "/tmp/test-project".to_string(),
        created_at: 1,
        updated_at: 1,
    };
    store.register_project(&record).unwrap();

    let projects = store.list_projects().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].path, "/tmp/test-project");
}
```

## State of the Art

| Old Approach                                                   | Current Approach                                      | When Changed    | Impact                                       |
| -------------------------------------------------------------- | ----------------------------------------------------- | --------------- | -------------------------------------------- |
| V1 schema (skills, skill_targets, settings, discovered_skills) | V3 schema (added description, source_subpath columns) | Pre-existing    | V4 must extend from V3, not V1               |
| Individual `execute_batch` per migration step                  | Transaction-wrapped DDL for multi-table migrations    | V4 (this phase) | Ensures atomicity for complex schema changes |

**No deprecated patterns:** The codebase uses current rusqlite 0.31 APIs. No migration from deprecated APIs needed.

## Assumptions Log

| #   | Claim                                                                                                                   | Section                           | Risk if Wrong                                                                                                                                                                                  |
| --- | ----------------------------------------------------------------------------------------------------------------------- | --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A1  | `execute_batch("BEGIN; ... COMMIT;")` provides atomic DDL in SQLite via rusqlite                                        | Architecture Patterns / Pattern 1 | If SQLite DDL isn't transactional with this approach, migration could partially fail. Risk is LOW -- SQLite DDL has been transactional since version 3.x and rusqlite passes through directly. |
| A2  | Extending `SkillStore` rather than creating `ProjectStore` is the better approach                                       | Anti-Patterns                     | If the store grows too large, it may need splitting later. Risk is LOW -- Phase 1 adds ~10 methods, manageable in one struct.                                                                  |
| A3  | Making `format_anyhow_error`, `expand_home_path`, `now_ms`, `remove_path_any` `pub(crate)` has no negative side effects | Pitfall 4                         | These are utility functions with no mutable state. Widening visibility from private to crate-internal is safe. Risk is NEGLIGIBLE.                                                             |

## Open Questions

1. **Assignment target path computation for delete cleanup**
   - What we know: D-07 says delete reads assignment paths before DB delete and removes filesystem artifacts. The `project_skill_assignments` table does NOT have a `target_path` column (unlike `skill_targets` which does).
   - What's unclear: How will the delete function know which filesystem path to clean up? It must compute `project_path / tool_relative_skills_dir / skill_name` -- but `tool_relative_skills_dir` comes from `ToolAdapter` and `skill_name` comes from the `skills` table.
   - Recommendation: The delete command should JOIN `project_skill_assignments` with `skills` (for `name`) and look up the tool adapter (for `relative_skills_dir`), then compute the target path as `project.path / adapter.relative_skills_dir / skill.name`. This is a command-layer concern, not a store-layer concern. Alternatively, add a `target_path` column to `project_skill_assignments` -- but this would deviate from the CONTEXT.md schema which does not include one. **The planner should decide whether to compute paths at delete time or add a target_path column.**

2. **Shared helper visibility**
   - What we know: `format_anyhow_error`, `expand_home_path`, `now_ms`, `remove_path_any` are private to `commands/mod.rs`.
   - What's unclear: Should these be moved to a separate `commands/helpers.rs` utility module, or just made `pub(crate)` in place?
   - Recommendation: Make them `pub(crate)` in place -- minimal change, matches the "keep changes minimal" development workflow constraint.

## Environment Availability

Step 2.6: SKIPPED -- This phase is purely code/config changes (Rust source files and SQLite schema). No external dependencies beyond the existing build toolchain.

## Validation Architecture

### Test Framework

| Property           | Value                                                             |
| ------------------ | ----------------------------------------------------------------- |
| Framework          | Rust built-in test harness (`cargo test`)                         |
| Config file        | `src-tauri/Cargo.toml` (test dependencies: tempfile 3, mockito 1) |
| Quick run command  | `cd src-tauri && cargo test`                                      |
| Full suite command | `npm run rust:test`                                               |

### Phase Requirements -> Test Map

| Req ID  | Behavior                                                         | Test Type   | Automated Command                                                                                 | File Exists?                          |
| ------- | ---------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------- | ------------------------------------- |
| INFR-04 | V4 migration creates 3 tables + 3 indexes atomically             | unit        | `cd src-tauri && cargo test core::skill_store::tests::v4_migration -- --exact`                    | No -- Wave 0                          |
| INFR-04 | V4 migration is idempotent (running ensure_schema twice is safe) | unit        | `cd src-tauri && cargo test core::skill_store::tests::schema_is_idempotent -- --exact`            | Yes (existing, but needs V4 coverage) |
| INFR-04 | V4 migration from V3 database preserves existing data            | unit        | `cd src-tauri && cargo test core::skill_store::tests::v4_migration_preserves_existing -- --exact` | No -- Wave 0                          |
| INFR-05 | Project commands compile and are registered                      | integration | `cd src-tauri && cargo test commands::projects::tests -- --exact`                                 | No -- Wave 0                          |
| PROJ-01 | Register project stores correct canonical path                   | unit        | `cd src-tauri && cargo test core::skill_store::tests::register_project -- --exact`                | No -- Wave 0                          |
| PROJ-01 | Register project rejects duplicate path                          | unit        | `cd src-tauri && cargo test core::skill_store::tests::register_project_duplicate -- --exact`      | No -- Wave 0                          |
| PROJ-01 | Register project rejects non-directory path                      | unit        | `cd src-tauri && cargo test commands::projects::tests::register_rejects_non_dir -- --exact`       | No -- Wave 0                          |
| PROJ-02 | Delete project cascades to project_tools and assignments         | unit        | `cd src-tauri && cargo test core::skill_store::tests::delete_project_cascades -- --exact`         | No -- Wave 0                          |
| PROJ-03 | List projects returns records with correct data                  | unit        | `cd src-tauri && cargo test core::skill_store::tests::list_projects -- --exact`                   | No -- Wave 0                          |
| TOOL-01 | Add/remove tool for project                                      | unit        | `cd src-tauri && cargo test core::skill_store::tests::project_tools_crud -- --exact`              | No -- Wave 0                          |
| TOOL-01 | Add duplicate tool is ignored (INSERT OR IGNORE)                 | unit        | `cd src-tauri && cargo test core::skill_store::tests::project_tools_duplicate -- --exact`         | No -- Wave 0                          |

### Sampling Rate

- **Per task commit:** `cd src-tauri && cargo test`
- **Per wave merge:** `npm run check` (includes lint + build + rust:fmt:check + rust:clippy + rust:test)
- **Phase gate:** Full `npm run check` green before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] Tests in `src-tauri/src/core/tests/skill_store.rs` -- V4 migration tests, project CRUD tests, cascade tests
- [ ] Tests in `src-tauri/src/commands/tests/projects.rs` -- command-level tests for project registration, delete with cleanup
- [ ] No new framework install needed -- existing `cargo test` infrastructure is sufficient

## Security Domain

### Applicable ASVS Categories

| ASVS Category         | Applies | Standard Control                                                                                          |
| --------------------- | ------- | --------------------------------------------------------------------------------------------------------- |
| V2 Authentication     | No      | N/A -- local desktop app, no auth                                                                         |
| V3 Session Management | No      | N/A -- no sessions                                                                                        |
| V4 Access Control     | No      | N/A -- single user, local filesystem                                                                      |
| V5 Input Validation   | Yes     | Path validation: `canonicalize()` + `is_dir()` check; SQL: parameterized queries via rusqlite `params![]` |
| V6 Cryptography       | No      | N/A -- no encryption in data layer                                                                        |

### Known Threat Patterns for SQLite + Filesystem

| Pattern                                 | STRIDE    | Standard Mitigation                                                                                                                               |
| --------------------------------------- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| SQL injection via path strings          | Tampering | Parameterized queries (`params![]` macro) -- never interpolate strings into SQL [VERIFIED: all existing queries use params!]                      |
| Path traversal via project registration | Tampering | `std::fs::canonicalize()` resolves all symlinks and `..` components; combined with `is_dir()` check [VERIFIED: CONTEXT.md D-08, D-09]             |
| Symlink following during delete cleanup | Elevation | `remove_path_any` uses `symlink_metadata` to detect symlinks and removes the link itself, not the target [VERIFIED: commands/mod.rs line 799-800] |

## Sources

### Primary (HIGH confidence)

- `src-tauri/src/core/skill_store.rs` -- Schema migration pattern, CRUD patterns, record structs, `with_conn` helper
- `src-tauri/src/commands/mod.rs` -- Command wrapping pattern, DTO definitions, error formatting, helper functions
- `src-tauri/src/lib.rs` -- Command registration pattern, state injection
- `src-tauri/src/core/sync_engine.rs` -- Sync primitives (for delete cleanup reference)
- `src-tauri/src/core/tool_adapters/mod.rs` -- ToolId enum, adapter lookup functions
- `src-tauri/src/core/tests/skill_store.rs` -- Test patterns, `make_store()` helper
- `src-tauri/src/commands/tests/commands.rs` -- Command test patterns
- `src-tauri/Cargo.toml` -- Dependency versions confirmed
- `.planning/phases/01-data-foundation/01-CONTEXT.md` -- All locked decisions (D-01 through D-09)
- rusqlite 0.31 official docs (https://docs.rs/rusqlite/0.31.0/) -- Transaction API, `execute_batch` behavior

### Secondary (MEDIUM confidence)

- None needed -- all findings from primary codebase analysis

### Tertiary (LOW confidence)

- None

## Project Constraints (from CLAUDE.md)

The following directives from CLAUDE.md must be honored during planning and implementation:

1. **Run `npm run check` before committing** -- ensures lint + build + rust:fmt:check + rust:clippy + rust:test all pass
2. **Frontend/Backend in one pass** -- Phase 1 is backend-only, but if any frontend types need adding, do both sides together
3. **DTO sync** -- Keep `src/components/skills/types.ts` in sync with Rust DTOs (Phase 1 may add project DTOs that Phase 4 will consume)
4. **i18n** -- English strings only this milestone (CLAUDE.md says provide both EN/ZH, but PROJECT.md constraint overrides: English only, Chinese deferred)
5. **Minimal changes** -- Only modify what is necessary for the requirement. Do not refactor unrelated code.
6. **New core modules must be exported in `core/mod.rs`** -- No new core module needed for Phase 1 (extending `skill_store.rs`), but if one is created, export it there
7. **New commands must be registered in `lib.rs` via `generate_handler!`** -- Applies directly to Phase 1
8. **Test with `tempfile` crate for temp directories** -- Applies directly to Phase 1 tests
9. **Use `anyhow::Context` to add context to errors** -- Applies to all new error paths
10. **Tauri command parameters use camelCase** -- Applies to all new project commands
11. **Verify Before Claiming Done** -- Run `npm run check` and confirm tests pass

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH -- using exclusively existing dependencies, verified in Cargo.toml
- Architecture: HIGH -- all patterns directly extracted from existing codebase with line-level verification
- Pitfalls: HIGH -- identified from actual code analysis (visibility, pragma, migration transaction)

**Research date:** 2026-04-07
**Valid until:** 2026-05-07 (stable -- no external dependency changes expected)
