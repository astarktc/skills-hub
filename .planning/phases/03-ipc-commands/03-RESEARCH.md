# Phase 3: IPC Commands - Research

**Researched:** 2026-04-07
**Domain:** Tauri IPC command layer, TypeScript DTO contracts, error prefix conventions
**Confidence:** HIGH

## Summary

Phase 3 is a thin integration layer phase. The heavy lifting -- data foundation (Phase 1) and sync logic (Phase 2) -- is already complete. 11 project commands exist in `commands/projects.rs` and are registered in `lib.rs`. This phase adds one new command (`bulk_assign_skill`), error prefix conventions for 3 scenarios, and frontend TypeScript DTO types mirroring the Rust DTOs.

The codebase already has strong, consistent patterns for every aspect of this phase: command wrapping (`spawn_blocking` + `SyncMutex`), error formatting (`format_anyhow_error` with prefix passthrough), DTO serialization (`#[derive(Serialize)]` on Rust side, `export type` on TS side), and bulk-operation error handling (continue-on-error, collect results). Research confirms no new libraries, no architectural decisions, and no surprises. Follow existing patterns exactly.

**Primary recommendation:** Implement by direct pattern replication from existing code. No new dependencies, no new abstractions, no new patterns.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Single backend command `bulk_assign_skill` -- one IPC call where the backend reads configured tools for the project, assigns the skill to each via `assign_and_sync`, and returns a detailed result.
- **D-02:** Response is `BulkAssignResultDto` with per-tool detail: `assigned: Vec<ProjectSkillAssignmentDto>` for successes and `failed: Vec<BulkAssignErrorDto>` (tool + error string) for failures.
- **D-03:** Follows Phase 2 D-06 pattern: continue on error -- if one tool fails, keep assigning the rest and report all results.
- **D-04:** Project DTOs go in a new `src/components/projects/types.ts` file, separate from existing `src/components/skills/types.ts`.
- **D-05:** Types to define: `ProjectDto`, `ProjectToolDto`, `ProjectSkillAssignmentDto`, `ResyncSummaryDto`, `BulkAssignResultDto`, `BulkAssignErrorDto` -- all mirroring their Rust counterparts.
- **D-06:** Project commands use specific error prefixes for frontend detection, following the existing pattern.
- **D-07:** Three prefixes to implement: `DUPLICATE_PROJECT|`, `ASSIGNMENT_EXISTS|`, `NOT_FOUND|`.

### Claude's Discretion

- Internal function signatures and parameter naming for the bulk-assign command
- Whether `BulkAssignErrorDto` is a new struct or reuses an existing pattern
- Test structure for the new bulk-assign command and error prefix validation
- Whether error prefixes are emitted from the command layer or the core layer

### Deferred Ideas (OUT OF SCOPE)

None -- discussion stayed within phase scope.

</user_constraints>

<phase_requirements>

## Phase Requirements

| ID      | Description                                                                          | Research Support                                                                                                                        |
| ------- | ------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| ASGN-01 | User can assign a skill to a project for a specific tool via checkbox in the matrix  | `add_project_skill_assignment` command already exists. Error prefix `ASSIGNMENT_EXISTS                                                  | `enables frontend to detect duplicates. TS DTO`ProjectSkillAssignmentDto` enables typed frontend consumption. |
| ASGN-04 | User can bulk-assign all configured tools for a skill via "All Tools" button per row | New `bulk_assign_skill` command. Loops `store.list_project_tools()`, calls `assign_and_sync()` per tool, returns `BulkAssignResultDto`. |
| SYNC-02 | User can re-sync all assignments for a single project via "Sync Project" button      | `resync_project` command already exists and is registered. TS DTO `ResyncSummaryDto` enables typed frontend consumption.                |
| SYNC-03 | User can re-sync all assignments across all projects via "Sync All" button           | `resync_all_projects` command already exists and is registered. Returns `Vec<ResyncSummaryDto>`.                                        |

</phase_requirements>

## Standard Stack

### Core

No new libraries needed. This phase uses only what Phases 1 and 2 already installed.

| Library | Version | Purpose           | Why Standard                                     |
| ------- | ------- | ----------------- | ------------------------------------------------ |
| serde   | 1.x     | DTO serialization | Already in Cargo.toml, used by all existing DTOs |
| uuid    | 1.x     | ID generation     | Already in Cargo.toml, used by `assign_and_sync` |
| anyhow  | 1.x     | Error handling    | Already in Cargo.toml, used by all core modules  |

**Installation:** None required. All dependencies already present.

## Architecture Patterns

### Existing Project Structure (no changes)

```
src-tauri/src/
  commands/
    mod.rs             # Existing commands + format_anyhow_error
    projects.rs        # 11 existing commands + 1 new (bulk_assign_skill)
  core/
    project_ops.rs     # ProjectDto, ProjectToolDto, ProjectSkillAssignmentDto
    project_sync.rs    # assign_and_sync, resync_project, resync_all_projects
    skill_store.rs     # DB CRUD methods
src/
  components/
    projects/
      types.ts         # NEW: TypeScript DTO types mirroring Rust DTOs
    skills/
      types.ts         # EXISTING: Pattern to follow for DTO structure
```

### Pattern 1: Tauri Command Wrapping

**What:** Every command follows the same `spawn_blocking` + error mapping pattern.
**When to use:** All new commands that touch the store or filesystem.
**Example:**

```rust
// Source: commands/projects.rs (existing pattern, lines 111-150)
#[tauri::command]
#[allow(non_snake_case)]
pub async fn bulk_assign_skill(
    store: State<'_, SkillStore>,
    sync_mutex: State<'_, SyncMutex>,
    projectId: String,
    skillId: String,
) -> Result<BulkAssignResultDto, String> {
    let store = store.inner().clone();
    let mutex = sync_mutex.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _lock = mutex.0.lock().unwrap_or_else(|e| e.into_inner());
        // ... business logic ...
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(format_anyhow_error)
}
```

[VERIFIED: commands/projects.rs lines 111-150]

### Pattern 2: Bulk Operation with Continue-on-Error

**What:** Loop through items, try each, collect successes and failures separately.
**When to use:** `bulk_assign_skill` -- iterating over configured tools.
**Example:**

```rust
// Source: core/project_sync.rs resync_project (lines 164-196)
// Pattern: iterate assignments, match on Ok/Err, collect into summary
for tool_record in tools {
    match assign_and_sync(&store, &project, &skill, &tool_record.tool, now) {
        Ok(assignment) => assigned.push(assignment_to_dto(assignment)),
        Err(e) => failed.push(BulkAssignErrorDto {
            tool: tool_record.tool.clone(),
            error: format!("{:#}", e),
        }),
    }
}
```

[VERIFIED: core/project_sync.rs lines 164-196 for resync pattern]

### Pattern 3: Error Prefix Convention

**What:** Command layer checks error conditions and prepends a machine-parseable prefix before returning to frontend.
**When to use:** When the frontend needs to distinguish error types for different UI flows.
**Example:**

```rust
// Source: commands/mod.rs format_anyhow_error (lines 36-44)
// Existing prefix passthrough in format_anyhow_error:
if first.starts_with("MULTI_SKILLS|")
    || first.starts_with("TARGET_EXISTS|")
    || first.starts_with("TOOL_NOT_INSTALLED|")
{
    return first;
}
// New prefixes to add: DUPLICATE_PROJECT|, ASSIGNMENT_EXISTS|, NOT_FOUND|
```

[VERIFIED: commands/mod.rs lines 36-44]

### Pattern 4: DTO Mirroring (Rust to TypeScript)

**What:** Rust `#[derive(Serialize)]` DTOs map 1:1 to TypeScript `export type` definitions.
**When to use:** Every IPC boundary type.
**Example:**

```rust
// Rust side (commands/projects.rs)
#[derive(serde::Serialize, Clone)]
pub struct ResyncSummaryDto {
    pub project_id: String,
    pub synced: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}
```

```typescript
// TypeScript side (components/projects/types.ts)
export type ResyncSummaryDto = {
  project_id: string;
  synced: number;
  failed: number;
  errors: string[];
};
```

[VERIFIED: commands/projects.rs lines 211-217, components/skills/types.ts for TS pattern]

### Anti-Patterns to Avoid

- **Business logic in commands layer:** The bulk-assign loop belongs in `commands/projects.rs` since it orchestrates existing core functions, but the individual `assign_and_sync` calls delegate to `core/project_sync.rs`. Do not reimplement sync logic in the command.
- **Omitting SyncMutex for sync-touching commands:** `bulk_assign_skill` MUST acquire the `SyncMutex` lock since it calls `assign_and_sync` which touches the filesystem.
- **Returning early on first error in bulk operations:** Decision D-03 explicitly requires continue-on-error. Return `BulkAssignResultDto` with both `assigned` and `failed` arrays, never bail on first failure.

## Don't Hand-Roll

| Problem                        | Don't Build                  | Use Instead                                                             | Why                                                                                     |
| ------------------------------ | ---------------------------- | ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------- | -------- |
| Duplicate assignment detection | Custom pre-check query       | SQLite `UNIQUE(project_id, skill_id, tool)` constraint                  | DB enforces uniqueness; catch the constraint violation error to emit `ASSIGNMENT_EXISTS | ` prefix |
| Duplicate project detection    | Custom pre-check query       | `store.get_project_by_path()` already exists in `register_project_path` | Already implemented in Phase 1                                                          |
| Error chain formatting         | Custom error string building | `format_anyhow_error()` from `commands/mod.rs`                          | Already handles chain formatting and prefix passthrough                                 |

**Key insight:** The bulk-assign command is a thin loop calling existing primitives. The only net-new code is the loop itself, the DTO structs for its response, and prefix strings in `format_anyhow_error`.

## Common Pitfalls

### Pitfall 1: Duplicate Assignment Handling in Bulk-Assign

**What goes wrong:** `assign_and_sync` calls `store.add_project_skill_assignment()` which does a plain `INSERT`. If the skill is already assigned to a tool for that project, the `UNIQUE(project_id, skill_id, tool)` constraint will cause a SQLite error. In bulk-assign, this could cause the entire operation to appear to fail.
**Why it happens:** The caller skips checking for existing assignments before inserting.
**How to avoid:** In the bulk-assign loop, either (a) pre-check via `get_project_skill_assignment` and skip already-assigned tools, or (b) catch the constraint violation in the `Err` branch and classify it as a skip rather than a failure. Option (a) is simpler and clearer.
**Warning signs:** Bulk-assign returns unexpected failures for tools that already have the skill assigned.
[VERIFIED: skill_store.rs line 90 UNIQUE constraint, line 653-668 INSERT statement]

### Pitfall 2: Missing Prefix Passthrough in format_anyhow_error

**What goes wrong:** New error prefixes (`DUPLICATE_PROJECT|`, `ASSIGNMENT_EXISTS|`, `NOT_FOUND|`) get consumed by `format_anyhow_error` which reformats the error chain, stripping the prefix.
**Why it happens:** `format_anyhow_error` only passes through known prefixes listed in its `starts_with` checks (currently `MULTI_SKILLS|`, `TARGET_EXISTS|`, `TOOL_NOT_INSTALLED|`).
**How to avoid:** Add the 3 new prefixes to the passthrough list in `format_anyhow_error`.
**Warning signs:** Frontend receives reformatted error text instead of `PREFIX|payload` strings.
[VERIFIED: commands/mod.rs lines 36-44]

### Pitfall 3: Forgetting Command Registration in generate_handler!

**What goes wrong:** New command compiles but frontend invoke call returns "unknown command" error at runtime.
**Why it happens:** `generate_handler![]` in `lib.rs` must list every command. Missing the new `bulk_assign_skill` silently fails.
**How to avoid:** Add `commands::projects::bulk_assign_skill` to `generate_handler![]` in `lib.rs`.
**Warning signs:** Compile succeeds, but frontend gets "did not find command" error from Tauri.
[VERIFIED: lib.rs lines 77-119]

### Pitfall 4: camelCase Parameters in Tauri Commands

**What goes wrong:** Frontend `invoke` passes `{ projectId, skillId }` but Rust function uses `project_id, skill_id` (snake_case), causing parameter binding failure at runtime.
**Why it happens:** Tauri 2 matches parameter names literally between JS and Rust.
**How to avoid:** Use camelCase parameter names in Rust (with `#[allow(non_snake_case)]`) to match frontend calling convention. All existing project commands follow this pattern.
**Warning signs:** "missing required parameter" errors from Tauri at runtime despite correct types.
[VERIFIED: commands/projects.rs uses camelCase params throughout, e.g. `projectId`, `skillId`]

### Pitfall 5: Rust usize Serializing to JavaScript number

**What goes wrong:** Rust `usize` serializes to a JavaScript `number` via serde, which is fine. But TypeScript type definitions must use `number`, not a hypothetical `usize` type.
**Why it happens:** Trivial but easy to overlook when mirroring DTOs.
**How to avoid:** Map Rust `usize` to TypeScript `number`, Rust `Option<T>` to TypeScript `T | null` or `T?`, Rust `Vec<T>` to TypeScript `T[]`.
**Warning signs:** TypeScript type errors at the DTO layer.
[VERIFIED: components/skills/types.ts consistently uses `number` for Rust integer types]

## Code Examples

### Bulk Assign Command (Recommended Implementation)

```rust
// Source: Pattern derived from commands/projects.rs add_project_skill_assignment
// + core/project_sync.rs resync_project continue-on-error pattern

#[derive(serde::Serialize, Clone)]
pub struct BulkAssignResultDto {
    pub assigned: Vec<ProjectSkillAssignmentDto>,
    pub failed: Vec<BulkAssignErrorDto>,
}

#[derive(serde::Serialize, Clone)]
pub struct BulkAssignErrorDto {
    pub tool: String,
    pub error: String,
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn bulk_assign_skill(
    store: State<'_, SkillStore>,
    sync_mutex: State<'_, SyncMutex>,
    projectId: String,
    skillId: String,
) -> Result<BulkAssignResultDto, String> {
    let store = store.inner().clone();
    let mutex = sync_mutex.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let project = store
            .get_project_by_id(&projectId)?
            .ok_or_else(|| anyhow::anyhow!("NOT_FOUND|project:{}", projectId))?;
        let skill = store
            .get_skill_by_id(&skillId)?
            .ok_or_else(|| anyhow::anyhow!("NOT_FOUND|skill:{}", skillId))?;
        let tools = store.list_project_tools(&projectId)?;

        let _lock = mutex.0.lock().unwrap_or_else(|e| e.into_inner());
        let now = now_ms();

        let mut assigned = Vec::new();
        let mut failed = Vec::new();

        for tool_record in &tools {
            // Skip if already assigned
            if store
                .get_project_skill_assignment(&projectId, &skillId, &tool_record.tool)?
                .is_some()
            {
                continue;
            }

            match project_sync::assign_and_sync(
                &store, &project, &skill, &tool_record.tool, now,
            ) {
                Ok(record) => {
                    assigned.push(ProjectSkillAssignmentDto {
                        id: record.id,
                        project_id: record.project_id,
                        skill_id: record.skill_id,
                        tool: record.tool,
                        mode: record.mode,
                        status: record.status,
                        last_error: record.last_error,
                        synced_at: record.synced_at,
                        content_hash: record.content_hash,
                        created_at: record.created_at,
                    });
                }
                Err(e) => {
                    failed.push(BulkAssignErrorDto {
                        tool: tool_record.tool.clone(),
                        error: format!("{:#}", e),
                    });
                }
            }
        }

        Ok::<_, anyhow::Error>(BulkAssignResultDto { assigned, failed })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(format_anyhow_error)
}
```

[VERIFIED: Pattern assembled from existing code in projects.rs and project_sync.rs]

### Error Prefix Additions to format_anyhow_error

```rust
// Source: commands/mod.rs format_anyhow_error -- add these to the passthrough check
if first.starts_with("MULTI_SKILLS|")
    || first.starts_with("TARGET_EXISTS|")
    || first.starts_with("TOOL_NOT_INSTALLED|")
    || first.starts_with("DUPLICATE_PROJECT|")
    || first.starts_with("ASSIGNMENT_EXISTS|")
    || first.starts_with("NOT_FOUND|")
{
    return first;
}
```

[VERIFIED: commands/mod.rs lines 36-44 for existing pattern]

### Error Prefix Emission in Command Layer

```rust
// register_project: detect duplicate and emit prefix
// Source: commands/projects.rs register_project -- wrap existing call
pub async fn register_project(/* ... */) -> Result<ProjectDto, String> {
    // ... spawn_blocking ...
    // Inside the closure, catch the "already registered" error:
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("already registered") {
            anyhow::anyhow!("DUPLICATE_PROJECT|{}", path)
        } else {
            e
        }
    })
}
```

[VERIFIED: project_ops.rs line 83 produces "project already registered" error]

### Frontend TypeScript DTOs

```typescript
// Source: Pattern from components/skills/types.ts

export type ProjectDto = {
  id: string;
  path: string;
  name: string;
  created_at: number;
  updated_at: number;
  tool_count: number;
  assignment_count: number;
  sync_status: string;
};

export type ProjectToolDto = {
  id: string;
  project_id: string;
  tool: string;
};

export type ProjectSkillAssignmentDto = {
  id: string;
  project_id: string;
  skill_id: string;
  tool: string;
  mode: string;
  status: string;
  last_error?: string | null;
  synced_at?: number | null;
  content_hash?: string | null;
  created_at: number;
};

export type ResyncSummaryDto = {
  project_id: string;
  synced: number;
  failed: number;
  errors: string[];
};

export type BulkAssignResultDto = {
  assigned: ProjectSkillAssignmentDto[];
  failed: BulkAssignErrorDto[];
};

export type BulkAssignErrorDto = {
  tool: string;
  error: string;
};
```

[VERIFIED: Mirrors Rust DTOs in project_ops.rs and commands/projects.rs; follows TS pattern from skills/types.ts]

## Validation Architecture

### Test Framework

| Property           | Value                                                                     |
| ------------------ | ------------------------------------------------------------------------- |
| Framework          | Rust built-in test harness (`cargo test`)                                 |
| Config file        | `src-tauri/Cargo.toml` (test dependencies: `tempfile 3.x`, `mockito 1.x`) |
| Quick run command  | `cargo test --manifest-path src-tauri/Cargo.toml`                         |
| Full suite command | `npm run check` (lint + build + rust:fmt:check + rust:clippy + rust:test) |

### Phase Requirements to Test Map

| Req ID  | Behavior                                                 | Test Type | Automated Command                                                                          | File Exists?                                            |
| ------- | -------------------------------------------------------- | --------- | ------------------------------------------------------------------------------------------ | ------------------------------------------------------- |
| ASGN-01 | Single assign command returns correct DTO                | unit      | `cargo test --manifest-path src-tauri/Cargo.toml -- commands::projects::tests`             | Partial -- core tests exist, command-layer tests needed |
| ASGN-04 | Bulk-assign iterates all tools, returns per-tool results | unit      | `cargo test --manifest-path src-tauri/Cargo.toml -- core::tests::project_sync::bulk`       | Wave 0                                                  |
| SYNC-02 | Resync project command returns summary DTO               | unit      | `cargo test --manifest-path src-tauri/Cargo.toml -- core::tests::project_sync::resync`     | Exists                                                  |
| SYNC-03 | Resync all projects command returns vec of summaries     | unit      | `cargo test --manifest-path src-tauri/Cargo.toml -- core::tests::project_sync::resync_all` | Exists                                                  |
| D-07    | Error prefixes pass through format_anyhow_error          | unit      | `cargo test --manifest-path src-tauri/Cargo.toml -- commands::tests::format_anyhow`        | Exists for old prefixes, needs new ones                 |

### Sampling Rate

- **Per task commit:** `cargo test --manifest-path src-tauri/Cargo.toml`
- **Per wave merge:** `npm run check`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `src-tauri/src/core/tests/project_sync.rs` -- add bulk-assign tests (covers ASGN-04)
- [ ] `src-tauri/src/commands/tests/commands.rs` -- add error prefix passthrough tests for new prefixes (covers D-07)
- [ ] No new test framework or fixtures needed -- existing `make_store()` + `tempfile` pattern covers all needs

## Security Domain

### Applicable ASVS Categories

| ASVS Category         | Applies | Standard Control                                                                                                                                    |
| --------------------- | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| V2 Authentication     | No      | N/A -- IPC commands are local desktop, no auth boundary                                                                                             |
| V3 Session Management | No      | N/A -- single-user desktop app                                                                                                                      |
| V4 Access Control     | No      | N/A -- all operations are user-initiated via UI                                                                                                     |
| V5 Input Validation   | Yes     | Existing pattern: `get_project_by_id` returns `Option`, commands check `ok_or_else` for NOT_FOUND. Path validation via `canonicalize()` in Phase 1. |
| V6 Cryptography       | No      | N/A -- no crypto in this phase                                                                                                                      |

### Known Threat Patterns

| Pattern                         | STRIDE    | Standard Mitigation                                                                 |
| ------------------------------- | --------- | ----------------------------------------------------------------------------------- |
| Path traversal via project path | Tampering | `std::fs::canonicalize()` in `register_project_path` (Phase 1) -- already mitigated |
| SQLite injection via IPC params | Tampering | rusqlite parameterized queries (`params![]`) -- already mitigated                   |

## Assumptions Log

| #   | Claim | Section | Risk if Wrong |
| --- | ----- | ------- | ------------- |
| --  | --    | --      | --            |

**All claims in this research were verified from existing codebase code.** No user confirmation needed.

## Open Questions (RESOLVED)

None. This phase is fully constrained by existing code patterns and locked decisions from CONTEXT.md.

## Sources

### Primary (HIGH confidence)

- `src-tauri/src/commands/projects.rs` -- 11 existing commands, patterns for new bulk_assign_skill
- `src-tauri/src/commands/mod.rs` -- format_anyhow_error, error prefix passthrough, DTO patterns
- `src-tauri/src/core/project_sync.rs` -- assign_and_sync, resync_project, resync_all_projects
- `src-tauri/src/core/project_ops.rs` -- Rust DTOs (ProjectDto, ProjectToolDto, ProjectSkillAssignmentDto)
- `src-tauri/src/core/skill_store.rs` -- UNIQUE constraints, CRUD methods
- `src-tauri/src/lib.rs` -- generate_handler! registration list
- `src/components/skills/types.ts` -- TypeScript DTO pattern
- `src-tauri/src/core/tests/project_sync.rs` -- Test patterns and fixtures

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH -- no new dependencies, all verified in Cargo.toml
- Architecture: HIGH -- pure pattern replication of existing code
- Pitfalls: HIGH -- all identified from direct code inspection of constraint violations and function behavior

**Research date:** 2026-04-07
**Valid until:** 2026-05-07 (stable -- pattern replication, no external dependencies)
