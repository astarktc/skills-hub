# Phase 2: Sync Logic - Research

**Researched:** 2026-04-07
**Domain:** Rust backend sync operations -- symlink/copy management for per-project skill assignments
**Confidence:** HIGH

## Summary

Phase 2 adds sync logic to the Phase 1 data foundation. The core job: when a skill is assigned to a project for a tool, create a symlink (or copy fallback) at `project_path / relative_skills_dir / skill_name` pointing to the skill's central repo path. When unassigned, remove it. When re-synced, repair all assignments. When listing, detect staleness for copy-mode targets.

The existing codebase provides everything needed. `sync_engine.rs` already implements the symlink-junction-copy fallback chain, tool-specific overrides (Cursor forces copy), and path removal. `content_hash.rs` provides directory hashing for staleness. `tool_adapters` provides path resolution. Phase 1 delivered the DB schema, CRUD methods, and command scaffolding. Phase 2's job is to wire sync operations into the existing `add_project_skill_assignment` and `remove_project_skill_assignment` commands, add a global mutex for serialization, and add status update and staleness detection methods.

**Primary recommendation:** Create a new `project_sync.rs` core module containing pure sync logic functions (assign, unassign, re-sync-project, re-sync-all, check-staleness). Enhance existing Phase 1 commands to call these functions inline. Add a `content_hash` column to `project_skill_assignments` via V5 migration. Register a global `Mutex<()>` in Tauri state for concurrency serialization.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Two-step assign: create DB record with status='pending', then immediately sync. If sync succeeds -> update to 'synced' with content_hash and synced_at. If sync fails -> record stays with status='error' + last_error. User can retry via re-sync.
- **D-02:** Assign command does both DB insert and sync inline -- one call from the frontend returns the assignment with its final status (synced/error). No separate sync command needed for individual assignments.
- **D-03:** Unassign is strict: remove symlink/copy from filesystem first, then delete DB record. If filesystem removal fails, keep record with status='error' so user sees what went wrong.
- **D-04:** Enhance existing `remove_project_skill_assignment` command with sync cleanup rather than creating a new command. Phase 1's command becomes sync-aware.
- **D-05:** Full re-sync: 'Sync Project' and 'Sync All' re-sync ALL assignments regardless of current status. Uses `sync_dir_hybrid_with_overwrite(overwrite=true)` to guarantee target matches source.
- **D-06:** Continue on error: if one assignment fails during bulk sync, log the error on that assignment (status='error') and continue syncing the rest. Return a summary of successes and failures.
- **D-07:** Staleness checked on assignment list load (when frontend loads assignments for a project). Automatic detection -- no explicit user trigger needed.
- **D-08:** Store source content hash (`content_hash::hash_dir()`) in the `project_skill_assignments` row at sync time. On list load, recompute hash on source and compare. If different -> status='stale'.
- **D-09:** Skip symlink-mode targets for staleness detection -- symlinks propagate changes instantly by pointing to the source directory. Only copy-mode targets can be stale. This avoids unnecessary I/O for the common case.
- **D-10:** Global mutex: one `Mutex<()>` registered in Tauri state, shared across all sync operations. Only one sync runs at a time across all projects. Simple to reason about, matches existing CancelToken pattern.
- **D-11:** Mutex implementation type is at Claude's discretion (std::sync::Mutex vs tokio::sync::Mutex -- both valid since sync work runs in spawn_blocking).

### Claude's Discretion

- Mutex implementation choice (std::sync vs tokio::sync)
- Module organization: whether sync logic goes in a new `project_sync.rs` or extends `project_ops.rs`
- Internal function signatures and helper decomposition
- Test structure and coverage scope for sync operations
- Whether to add a `content_hash` column to the assignment table (new migration) or use the existing `last_error` field to store hash metadata

### Deferred Ideas (OUT OF SCOPE)

None -- discussion stayed within phase scope.

</user_constraints>

<phase_requirements>

## Phase Requirements

| ID      | Description                                                                                | Research Support                                                                                                                                                                                                                                                                                                          |
| ------- | ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ASGN-02 | Assigning a skill immediately creates a symlink/copy in the project's tool skill directory | `sync_engine::sync_dir_for_tool_with_overwrite()` handles the full fallback chain. Target path = `project_path / adapter.relative_skills_dir / skill.name`. Decision D-01/D-02 define the inline assign-then-sync flow.                                                                                                   |
| ASGN-03 | User can unassign a skill from a project (removes symlink/copy from project directory)     | `sync_engine::remove_path_any()` handles symlinks, directories, and files. Decision D-03 defines strict unassign: filesystem first, then DB delete. D-04 says enhance existing command.                                                                                                                                   |
| ASGN-05 | Global sync continues to work alongside project sync without interference                  | Global sync operates on `skill_targets` table + `~/.skillshub/<skill>` -> tool home dirs. Project sync operates on `project_skill_assignments` table + project dirs. Completely separate DB tables, different filesystem targets. No code changes to global sync needed.                                                  |
| SYNC-04 | App detects content staleness for copy-mode targets via hash comparison                    | `content_hash::hash_dir()` returns SHA-256 hex string. D-08 says store hash at sync time, recompute on list load. D-09 says skip symlink targets (instant propagation).                                                                                                                                                   |
| INFR-01 | App detects cross-filesystem scenarios and auto-falls back to copy mode                    | `sync_dir_hybrid()` already does this transparently: tries symlink, catches failure, falls back to junction (Windows), then copy. Verified on WSL2: symlinks actually work across ext4-9p but the engine handles failures gracefully regardless. The `SyncOutcome.mode_used` tells us which mode was used for DB storage. |
| INFR-02 | Sync operations are serialized to prevent race conditions                                  | D-10 defines a global `Mutex<()>` in Tauri state. All sync operations (assign, unassign, re-sync) acquire this mutex before doing filesystem work.                                                                                                                                                                        |

</phase_requirements>

## Standard Stack

### Core (all already in project -- no new dependencies)

| Library    | Version        | Purpose                              | Why Standard                                                    |
| ---------- | -------------- | ------------------------------------ | --------------------------------------------------------------- |
| rusqlite   | 0.31 (bundled) | SQLite for assignment status updates | Already in project, handles V5 migration [VERIFIED: Cargo.toml] |
| walkdir    | 2.5            | Used by content_hash::hash_dir()     | Already in project [VERIFIED: Cargo.toml]                       |
| sha2 + hex | 0.10 + 0.4     | Content hashing for staleness        | Already in project via content_hash.rs [VERIFIED: codebase]     |
| tempfile   | 3.x            | Test fixtures                        | Already in project [VERIFIED: Cargo.toml]                       |

### No New Dependencies Needed

This phase introduces zero new crate dependencies. All required functionality exists in the current codebase:

- Sync operations: `sync_engine.rs` [VERIFIED: codebase]
- Content hashing: `content_hash.rs` [VERIFIED: codebase]
- Tool path resolution: `tool_adapters/mod.rs` [VERIFIED: codebase]
- DB operations: `skill_store.rs` [VERIFIED: codebase]
- Mutex: `std::sync::Mutex` (Rust stdlib) [VERIFIED: Rust stdlib]

## Architecture Patterns

### Recommended Module Structure

```
src-tauri/src/core/
  project_sync.rs         # NEW: Pure sync logic functions
  project_ops.rs          # Existing: DTO conversion, project register/remove
  sync_engine.rs          # Existing: Unchanged -- low-level symlink/copy
  content_hash.rs         # Existing: Unchanged -- hash_dir()
  skill_store.rs          # Modified: V5 migration, update_assignment_status()
  mod.rs                  # Modified: Export project_sync

src-tauri/src/commands/
  projects.rs             # Modified: Enhance assign/unassign, add re-sync commands

src-tauri/src/
  lib.rs                  # Modified: Register SyncMutex, register new commands
```

**Rationale for `project_sync.rs` as a new module** (Claude's discretion choice):

1. `project_ops.rs` handles project registration/removal and DTO conversion -- conceptually "project management"
2. Sync logic (assign, unassign, re-sync, staleness) is a different concern -- "sync orchestration"
3. Separating keeps both modules focused and independently testable
4. The sync module depends on `sync_engine`, `content_hash`, `tool_adapters`, and `skill_store` -- a clear dependency graph

### Pattern 1: Inline Assign-Then-Sync (D-01, D-02)

**What:** The `add_project_skill_assignment` command creates a DB record with status='pending', then immediately calls sync. The returned DTO reflects the final status.

**When to use:** Every individual skill assignment.

**Example:**

```rust
// Source: Decision D-01, D-02 from CONTEXT.md + existing code patterns
pub fn assign_and_sync(
    store: &SkillStore,
    project: &ProjectRecord,
    skill: &SkillRecord,
    tool_key: &str,
    now: i64,
) -> Result<ProjectSkillAssignmentRecord> {
    let adapter = tool_adapters::adapter_by_key(tool_key)
        .ok_or_else(|| anyhow::anyhow!("unknown tool: {}", tool_key))?;

    // Phase 1 record creation (status='pending')
    let record = ProjectSkillAssignmentRecord {
        id: Uuid::new_v4().to_string(),
        project_id: project.id.clone(),
        skill_id: skill.id.clone(),
        tool: tool_key.to_string(),
        mode: "symlink".to_string(), // placeholder -- updated after sync
        status: "pending".to_string(),
        last_error: None,
        content_hash: None,
        synced_at: None,
        created_at: now,
    };
    store.add_project_skill_assignment(&record)?;

    // Immediately sync
    let source = Path::new(&skill.central_path);
    let target = Path::new(&project.path)
        .join(adapter.relative_skills_dir)
        .join(&skill.name);

    match sync_engine::sync_dir_for_tool_with_overwrite(tool_key, source, &target, false) {
        Ok(outcome) => {
            let hash = if matches!(outcome.mode_used, SyncMode::Copy) {
                Some(content_hash::hash_dir(source)?)
            } else {
                None // symlinks don't need hash -- instant propagation
            };
            let mode_str = sync_mode_to_str(&outcome.mode_used);
            store.update_assignment_status(
                &record.id, "synced", None, Some(now), Some(&mode_str), hash.as_deref(),
            )?;
            // Return updated record...
        }
        Err(e) => {
            store.update_assignment_status(
                &record.id, "error", Some(&e.to_string()), None, None, None,
            )?;
            // Return record with error status...
        }
    }
}
```

### Pattern 2: Strict Unassign (D-03)

**What:** Remove filesystem artifact first, then delete DB record. If filesystem removal fails, keep record with error status.

**Example:**

```rust
// Source: Decision D-03 from CONTEXT.md
pub fn unassign_and_cleanup(
    store: &SkillStore,
    project: &ProjectRecord,
    skill: &SkillRecord,
    tool_key: &str,
) -> Result<()> {
    let adapter = tool_adapters::adapter_by_key(tool_key)
        .ok_or_else(|| anyhow::anyhow!("unknown tool: {}", tool_key))?;

    let target = Path::new(&project.path)
        .join(adapter.relative_skills_dir)
        .join(&skill.name);

    match sync_engine::remove_path_any(&target) {
        Ok(()) => {
            store.remove_project_skill_assignment(
                &project.id, &skill.id, tool_key,
            )?;
        }
        Err(e) => {
            // Filesystem removal failed -- keep record with error
            if let Some(assignment) = store.get_assignment(
                &project.id, &skill.id, tool_key,
            )? {
                store.update_assignment_status(
                    &assignment.id, "error", Some(&e.to_string()), None, None, None,
                )?;
            }
            return Err(e);
        }
    }
    Ok(())
}
```

### Pattern 3: Global Mutex for Sync Serialization (D-10)

**What:** A single `Mutex<()>` registered in Tauri state, acquired by all sync operations.

**Example:**

```rust
// In lib.rs setup
use std::sync::Mutex;
pub struct SyncMutex(pub Mutex<()>);

// Registration
app.manage(SyncMutex(Mutex::new(())));

// In commands/projects.rs
#[tauri::command]
pub async fn add_project_skill_assignment(
    store: State<'_, SkillStore>,
    sync_mutex: State<'_, SyncMutex>,
    projectId: String,
    skillId: String,
    tool: String,
) -> Result<ProjectSkillAssignmentDto, String> {
    let store = store.inner().clone();
    let mutex = sync_mutex.inner().clone(); // Clone the Arc behind State
    tauri::async_runtime::spawn_blocking(move || {
        let _lock = mutex.0.lock().map_err(|e| anyhow::anyhow!("sync lock: {}", e))?;
        project_sync::assign_and_sync(&store, ...)
    }).await...
}
```

**Mutex choice: `std::sync::Mutex`** (Claude's discretion recommendation)

Rationale: All sync work runs inside `spawn_blocking`, which means the lock is held on a blocking thread, not across `.await` points. `std::sync::Mutex` is simpler, has no async overhead, and is the correct choice when the critical section is purely synchronous. `tokio::sync::Mutex` would be needed only if we held the lock across await points, which we do not. [VERIFIED: existing codebase pattern uses `spawn_blocking` for all sync operations]

### Pattern 4: Staleness Detection on List Load (D-07, D-08, D-09)

**What:** When listing assignments for a project, check copy-mode targets for staleness by recomputing source hash and comparing to stored hash.

**Example:**

```rust
// Source: Decisions D-07, D-08, D-09 from CONTEXT.md
pub fn list_assignments_with_staleness(
    store: &SkillStore,
    project_id: &str,
) -> Result<Vec<ProjectSkillAssignmentRecord>> {
    let assignments = store.list_project_skill_assignments(project_id)?;
    let mut result = Vec::with_capacity(assignments.len());

    for mut assignment in assignments {
        // Only check staleness for copy-mode synced targets (D-09)
        if assignment.status == "synced" && assignment.mode == "copy" {
            if let Some(stored_hash) = &assignment.content_hash {
                if let Ok(Some(skill)) = store.get_skill_by_id(&assignment.skill_id) {
                    let source = Path::new(&skill.central_path);
                    if source.exists() {
                        if let Ok(current_hash) = content_hash::hash_dir(source) {
                            if &current_hash != stored_hash {
                                assignment.status = "stale".to_string();
                                // Update DB status too
                                let _ = store.update_assignment_status(
                                    &assignment.id, "stale", None, None, None, None,
                                );
                            }
                        }
                    }
                }
            }
        }
        result.push(assignment);
    }
    Ok(result)
}
```

### Pattern 5: Re-sync All (D-05, D-06)

**What:** Iterate all assignments, sync each with `overwrite=true`, continue on error.

**Example:**

```rust
// Source: Decisions D-05, D-06 from CONTEXT.md
pub struct ResyncSummary {
    pub synced: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

pub fn resync_project(
    store: &SkillStore,
    project_id: &str,
    now: i64,
) -> Result<ResyncSummary> {
    let project = store.get_project_by_id(project_id)?
        .ok_or_else(|| anyhow::anyhow!("project not found"))?;
    let assignments = store.list_project_skill_assignments(project_id)?;
    let mut summary = ResyncSummary { synced: 0, failed: 0, errors: vec![] };

    for assignment in &assignments {
        match sync_single_assignment(store, &project, assignment, true, now) {
            Ok(()) => summary.synced += 1,
            Err(e) => {
                summary.failed += 1;
                summary.errors.push(format!("{}: {}", assignment.id, e));
                // D-06: continue on error
            }
        }
    }
    Ok(summary)
}
```

### Anti-Patterns to Avoid

- **Modifying sync_engine.rs:** The constraint says "reuse existing sync_engine.rs primitives -- do not duplicate or modify". Project sync logic wraps these primitives; it never changes them. [VERIFIED: PROJECT.md constraint]
- **Separate sync command per assignment:** D-02 says assign does DB + sync inline. Don't create a separate `sync_project_skill_assignment` command that the frontend calls after `add_project_skill_assignment`.
- **Holding mutex across await points:** The mutex must be acquired inside `spawn_blocking`, not in the async command function. The `State<'_>` lifetime prevents moving the State into the closure, so extract the inner value first.
- **Computing hash for symlink targets:** D-09 says skip symlinks for staleness -- they propagate instantly. Unnecessary I/O.

## Don't Hand-Roll

| Problem                        | Don't Build                                     | Use Instead                                                   | Why                                                                                              |
| ------------------------------ | ----------------------------------------------- | ------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| Symlink/copy/junction creation | Custom symlink logic                            | `sync_engine::sync_dir_for_tool_with_overwrite()`             | Handles platform differences, Cursor exception, overwrite logic                                  |
| Cross-filesystem detection     | Device ID comparison or filesystem type probing | `sync_engine::sync_dir_hybrid()` try-and-fallback             | Engine already tries symlink first, falls back to copy on failure. No explicit detection needed. |
| Directory content hashing      | Custom hash implementation                      | `content_hash::hash_dir()`                                    | Handles file ordering, .git exclusion, SHA-256                                                   |
| Path removal (symlink or dir)  | `std::fs::remove_dir_all` directly              | `sync_engine::remove_path_any()`                              | Handles symlinks (which are files, not dirs), broken symlinks, regular dirs                      |
| Tool path resolution           | Hardcoded `.claude/skills` paths                | `tool_adapters::adapter_by_key(tool_key).relative_skills_dir` | 42 tools with different paths, shared-directory grouping                                         |

**Key insight:** The existing sync engine is designed as a library of composable primitives. Phase 2's job is orchestration (when to call them, what DB state to update), not reimplementation.

## Common Pitfalls

### Pitfall 1: State Access Pattern in Tauri Commands with Mutex

**What goes wrong:** Trying to use `State<'_, SyncMutex>` directly inside `spawn_blocking` closure fails because `State` borrows from the Tauri runtime and cannot be sent across threads.

**Why it happens:** `State<'_>` contains a borrow with a specific lifetime tied to the command invocation. `spawn_blocking` requires `'static` captured values.

**How to avoid:** Extract the inner value using `.inner().clone()` before the closure, then move the cloned value into the closure. For `Mutex<()>`, wrap it in an `Arc` (or use Tauri's built-in `Arc` wrapping for managed state).

**Warning signs:** Compiler error about lifetime bounds or `Send` trait not satisfied.

**Correct pattern:**

```rust
// Tauri State internally wraps in Arc, so .inner() returns &T
// For SyncMutex, we need the whole struct to be Clone
#[derive(Clone)]
pub struct SyncMutex(pub Arc<Mutex<()>>);

// In lib.rs
app.manage(SyncMutex(Arc::new(Mutex::new(()))));

// In command
let sync_mutex = sync_mutex.inner().clone();
tauri::async_runtime::spawn_blocking(move || {
    let _lock = sync_mutex.0.lock()...;
    // do sync work
})
```

[VERIFIED: This is how existing `CancelToken` pattern works -- `Arc<CancelToken>` registered via `app.manage(Arc::new(CancelToken::new()))`, cloned in commands via `Arc::clone(cancel.inner())`]

### Pitfall 2: Symlink Target Path Resolution

**What goes wrong:** Using `adapter.relative_skills_dir` with the home directory (global sync path) instead of the project directory.

**Why it happens:** The existing global sync commands resolve target as `resolve_default_path(adapter)` which is `home_dir / relative_skills_dir`. Project sync needs `project_path / relative_skills_dir / skill_name`.

**How to avoid:** Always construct project-specific target paths explicitly:

```rust
let target = Path::new(&project.path)
    .join(adapter.relative_skills_dir)
    .join(&skill.name);
```

**Warning signs:** Skills appearing in the home directory instead of the project directory.

### Pitfall 3: Missing Parent Directory Creation

**What goes wrong:** Sync fails because the intermediate directories don't exist (e.g., `project/.claude/skills/` doesn't exist yet).

**Why it happens:** `sync_dir_hybrid()` calls `ensure_parent_dir()` which creates the parent of the target. But if the project doesn't have a `.claude/` directory at all, the parent `.claude/skills/` needs to be created too.

**How to avoid:** `ensure_parent_dir` in `sync_engine.rs` already calls `create_dir_all` on the parent, which creates all intermediate directories. This is already handled correctly. However, verify that the target path construction produces the right parent:

- Target: `project/.claude/skills/my-skill`
- Parent: `project/.claude/skills/` -- created by `create_dir_all`

[VERIFIED: `sync_engine::ensure_parent_dir()` calls `std::fs::create_dir_all(parent)` which creates all intermediate directories]

### Pitfall 4: Content Hash Column Missing from Assignment Table

**What goes wrong:** D-08 says store content_hash in the assignment row, but the V4 schema doesn't have a `content_hash` column on `project_skill_assignments`.

**Why it happens:** Phase 1 created the V4 schema with status, mode, last_error, synced_at -- but `content_hash` was deferred to Phase 2 as a discretion item.

**How to avoid:** Add a V5 migration that adds `content_hash TEXT NULL` to `project_skill_assignments`. Also update `ProjectSkillAssignmentRecord` struct to include the field.

### Pitfall 5: Mutex Poisoning

**What goes wrong:** If a sync operation panics while holding the mutex, `std::sync::Mutex` becomes "poisoned" and all subsequent `lock()` calls return `Err`.

**Why it happens:** Rare -- would require a bug causing a panic inside the critical section.

**How to avoid:** Use `.lock().unwrap_or_else(|e| e.into_inner())` to recover from poisoned mutexes. The poison indicates a previous panic, but the mutex data (`()`) is trivial, so recovery is safe.

### Pitfall 6: Global Sync Interference

**What goes wrong:** Concern that project sync might interfere with existing global sync.

**Why it doesn't happen:** The two systems operate on completely separate domains:

- **Global sync:** `skill_targets` table, target = `~/{tool_skills_dir}/{skill_name}`
- **Project sync:** `project_skill_assignments` table, target = `{project_path}/{tool_skills_dir}/{skill_name}`

Different DB tables, different filesystem paths. No shared state except the `SkillStore` (which is thread-safe via per-call connections). No code changes to global sync are needed.

[VERIFIED: Global sync operates through `commands/mod.rs::sync_skill_to_tool` which writes to `skill_targets`. Project sync will operate through `commands/projects.rs` which writes to `project_skill_assignments`.]

## Code Examples

### Target Path Construction

```rust
// Source: tool_adapters/mod.rs + project_ops.rs patterns [VERIFIED: codebase]
fn resolve_project_sync_target(
    project_path: &Path,
    adapter: &ToolAdapter,
    skill_name: &str,
) -> PathBuf {
    project_path
        .join(adapter.relative_skills_dir)
        .join(skill_name)
}
// Example: /home/alex/my-project + .claude/skills + my-skill
//       -> /home/alex/my-project/.claude/skills/my-skill
```

### SyncMode to String Conversion

```rust
// Source: commands/mod.rs::sync_skill_to_tool pattern [VERIFIED: codebase]
fn sync_mode_to_str(mode: &SyncMode) -> &'static str {
    match mode {
        SyncMode::Auto => "auto",
        SyncMode::Symlink => "symlink",
        SyncMode::Junction => "junction",
        SyncMode::Copy => "copy",
    }
}
```

### V5 Migration for content_hash Column

```rust
// Source: skill_store.rs migration pattern [VERIFIED: codebase]
const SCHEMA_VERSION: i32 = 5;

// In ensure_schema incremental migrations:
if user_version < 5 {
    conn.execute_batch(
        "ALTER TABLE project_skill_assignments ADD COLUMN content_hash TEXT NULL;"
    )?;
}
```

### Assignment Status Update Method

```rust
// Source: follows existing upsert_skill_target pattern [VERIFIED: codebase]
pub fn update_assignment_status(
    &self,
    assignment_id: &str,
    status: &str,
    last_error: Option<&str>,
    synced_at: Option<i64>,
    mode: Option<&str>,
    content_hash: Option<&str>,
) -> Result<()> {
    self.with_conn(|conn| {
        let mut sql = String::from("UPDATE project_skill_assignments SET status = ?1, last_error = ?2");
        let mut param_idx = 3;

        if synced_at.is_some() {
            sql.push_str(&format!(", synced_at = ?{}", param_idx));
            param_idx += 1;
        }
        if mode.is_some() {
            sql.push_str(&format!(", mode = ?{}", param_idx));
            param_idx += 1;
        }
        if content_hash.is_some() {
            sql.push_str(&format!(", content_hash = ?{}", param_idx));
            param_idx += 1;
        }
        sql.push_str(&format!(" WHERE id = ?{}", param_idx));

        // Build params dynamically...
        // (Alternatively, use a simpler approach: always SET all fields)
        Ok(())
    })
}
```

**Simpler alternative** (recommended -- matches existing patterns better):

```rust
pub fn update_assignment_status(
    &self,
    assignment_id: &str,
    status: &str,
    last_error: Option<&str>,
    synced_at: Option<i64>,
    mode: Option<&str>,
    content_hash: Option<&str>,
) -> Result<()> {
    self.with_conn(|conn| {
        conn.execute(
            "UPDATE project_skill_assignments
             SET status = ?1, last_error = ?2, synced_at = COALESCE(?3, synced_at),
                 mode = COALESCE(?4, mode), content_hash = COALESCE(?5, content_hash)
             WHERE id = ?6",
            params![status, last_error, synced_at, mode, content_hash, assignment_id],
        )?;
        Ok(())
    })
}
```

### Get Assignment by Composite Key

```rust
// Needed for unassign: look up assignment before removing
pub fn get_project_skill_assignment(
    &self,
    project_id: &str,
    skill_id: &str,
    tool: &str,
) -> Result<Option<ProjectSkillAssignmentRecord>> {
    self.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, skill_id, tool, mode, status, last_error,
                    synced_at, created_at, content_hash
             FROM project_skill_assignments
             WHERE project_id = ?1 AND skill_id = ?2 AND tool = ?3
             LIMIT 1",
        )?;
        // ... map row to record
    })
}
```

## State of the Art

| Old Approach                | Current Approach          | When Changed   | Impact                                                  |
| --------------------------- | ------------------------- | -------------- | ------------------------------------------------------- |
| Global-only sync (home dir) | Global + per-project sync | This milestone | Skills scoped to projects, not just globally available  |
| No concurrency control      | Global sync mutex         | Phase 2        | Prevents race conditions between UI toggle and Sync All |
| No staleness detection      | Content hash comparison   | Phase 2        | Users see when copy-mode targets are outdated           |

## Assumptions Log

| #   | Claim                                                                                                            | Section                           | Risk if Wrong                                                                                                                                                                                                 |
| --- | ---------------------------------------------------------------------------------------------------------------- | --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A1  | `std::sync::Mutex` is preferable over `tokio::sync::Mutex` for this use case                                     | Architecture Patterns - Pattern 3 | Low -- both work; tokio variant just adds unnecessary async overhead for purely synchronous critical sections                                                                                                 |
| A2  | V5 migration adding `content_hash` column is the right approach over storing hash in `last_error`                | Architecture Patterns, Pitfall 4  | Low -- using `last_error` for hash is a hack; a proper column is cleaner and matches the schema-upfront philosophy from D-03                                                                                  |
| A3  | Tauri `State` wraps managed values in `Arc` internally, so `.inner()` returns `&T` where `T` is the managed type | Pitfall 1                         | Medium -- if this is wrong, the clone pattern needs adjustment. Verified by inspecting CancelToken usage: `Arc<CancelToken>` is managed, and `cancel.inner()` returns `&Arc<CancelToken>` which is cloneable. |

## Open Questions (RESOLVED)

1. **Re-sync commands: Phase 2 creates both core functions AND Tauri commands.**
   - Resolved: Core functions (`resync_project`, `resync_all_projects`) live in `project_sync.rs`. Tauri commands (`resync_project` and `resync_all_projects`) are added to `commands/projects.rs` in Phase 2, wired through SyncMutex. This is necessary because INFR-02 requires ALL sync operations -- including re-sync -- to be serialized via the mutex. Deferring command creation to Phase 3 would leave the mutex wiring untested for re-sync paths. Phase 3 can still add additional IPC commands (bulk-assign, etc.) but the re-sync commands must exist in Phase 2 to satisfy INFR-02.

2. **List assignments with staleness: modify existing command in Phase 2.**
   - Resolved: The existing `list_project_skill_assignments` command in `commands/projects.rs` is modified to call `project_sync::list_assignments_with_staleness()` instead of the raw `store.list_project_skill_assignments()`. This is necessary because D-07 says staleness is checked on list load, and SYNC-04 requires the app to detect staleness. If the command is not wired to the staleness function, the app cannot detect stale assignments regardless of whether the core function exists. The `content_hash` field is also added to the DTO.

## Environment Availability

Step 2.6: External dependencies verified.

| Dependency       | Required By         | Available | Version                     | Fallback                      |
| ---------------- | ------------------- | --------- | --------------------------- | ----------------------------- |
| Rust toolchain   | Backend compilation | Yes       | rustc 1.94.1 (2026-03-25)   | --                            |
| Cargo            | Build/test          | Yes       | cargo 1.94.1                | --                            |
| Symlink support  | Sync engine         | Yes       | WSL2 ext4 + 9p both support | Copy mode fallback (built-in) |
| SQLite (bundled) | Data layer          | Yes       | rusqlite 0.31               | --                            |

[VERIFIED: `rustc --version` = 1.94.1, cargo 1.94.1, symlinks work on this WSL2 environment including cross-filesystem]

**Missing dependencies with no fallback:** None.
**Missing dependencies with fallback:** None.

## Validation Architecture

### Test Framework

| Property           | Value                                                         |
| ------------------ | ------------------------------------------------------------- |
| Framework          | Rust built-in test harness (`cargo test`)                     |
| Config file        | `src-tauri/Cargo.toml` (test dependencies: tempfile, mockito) |
| Quick run command  | `source ~/.cargo/env && cd src-tauri && cargo test --lib -q`  |
| Full suite command | `source ~/.cargo/env && cd src-tauri && cargo test`           |

### Phase Requirements -> Test Map

| Req ID       | Behavior                                               | Test Type   | Automated Command                                                            | File Exists? |
| ------------ | ------------------------------------------------------ | ----------- | ---------------------------------------------------------------------------- | ------------ |
| ASGN-02      | Assign creates symlink at project_path/tool_dir/skill  | integration | `cargo test project_sync::tests::assign_creates_symlink -q`                  | Wave 0       |
| ASGN-02      | Assign falls back to copy when symlink fails (Cursor)  | integration | `cargo test project_sync::tests::assign_cursor_forces_copy -q`               | Wave 0       |
| ASGN-02      | Assign records correct mode and status in DB           | unit        | `cargo test project_sync::tests::assign_updates_status -q`                   | Wave 0       |
| ASGN-02      | Assign stores content_hash for copy-mode targets       | unit        | `cargo test project_sync::tests::assign_stores_hash_for_copy -q`             | Wave 0       |
| ASGN-02      | Assign records error status on sync failure            | unit        | `cargo test project_sync::tests::assign_records_error -q`                    | Wave 0       |
| ASGN-03      | Unassign removes symlink from project directory        | integration | `cargo test project_sync::tests::unassign_removes_symlink -q`                | Wave 0       |
| ASGN-03      | Unassign removes copy from project directory           | integration | `cargo test project_sync::tests::unassign_removes_copy -q`                   | Wave 0       |
| ASGN-03      | Unassign keeps record with error on filesystem failure | unit        | `cargo test project_sync::tests::unassign_error_keeps_record -q`             | Wave 0       |
| ASGN-05      | Global sync and project sync don't interfere           | unit        | `cargo test project_sync::tests::global_and_project_sync_independent -q`     | Wave 0       |
| SYNC-04      | Staleness detected for copy-mode targets               | integration | `cargo test project_sync::tests::staleness_detected_for_copy -q`             | Wave 0       |
| SYNC-04      | Staleness skipped for symlink-mode targets             | unit        | `cargo test project_sync::tests::staleness_skipped_for_symlink -q`           | Wave 0       |
| INFR-01      | Cross-filesystem falls back to copy                    | integration | Already covered by `sync_engine::tests::hybrid_sync_creates_link` (existing) | Yes          |
| INFR-02      | Concurrent syncs are serialized via mutex              | unit        | `cargo test project_sync::tests::sync_serialization -q`                      | Wave 0       |
| V5-MIGRATION | V5 migration adds content_hash column                  | unit        | `cargo test skill_store::tests::v5_migration -q`                             | Wave 0       |
| RESYNC       | Re-sync updates all assignments with overwrite         | integration | `cargo test project_sync::tests::resync_updates_all -q`                      | Wave 0       |
| RESYNC       | Re-sync continues on individual failures               | unit        | `cargo test project_sync::tests::resync_continues_on_error -q`               | Wave 0       |

### Sampling Rate

- **Per task commit:** `source ~/.cargo/env && cd src-tauri && cargo test --lib -q`
- **Per wave merge:** `source ~/.cargo/env && cd src-tauri && cargo test`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `src-tauri/src/core/tests/project_sync.rs` -- all sync logic tests (new file)
- [ ] `src-tauri/src/core/tests/skill_store.rs` -- V5 migration test (add to existing)
- [ ] V5 migration in `skill_store.rs` must exist before tests can reference `content_hash`

## Security Domain

### Applicable ASVS Categories

| ASVS Category         | Applies | Standard Control                                              |
| --------------------- | ------- | ------------------------------------------------------------- |
| V2 Authentication     | No      | N/A -- local desktop app                                      |
| V3 Session Management | No      | N/A -- no sessions                                            |
| V4 Access Control     | No      | N/A -- single user                                            |
| V5 Input Validation   | Yes     | Validate project/skill/tool IDs exist in DB before operations |
| V6 Cryptography       | No      | SHA-256 used for integrity (content hash), not security       |

### Known Threat Patterns

| Pattern                                  | STRIDE                 | Standard Mitigation                                                                                                                                                                                                  |
| ---------------------------------------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path traversal in project_path           | Tampering              | Project paths are canonicalized on registration (Phase 1 D-08). Skill names come from DB, not user input. Tool relative_skills_dir is hardcoded.                                                                     |
| Symlink target manipulation              | Tampering              | Source path comes from `skill.central_path` in DB (app-controlled). Target constructed from DB-stored project path + hardcoded adapter path + DB-stored skill name. No user-controlled path components at sync time. |
| Race condition: concurrent sync          | Denial of Service      | Global mutex (D-10) serializes all sync operations.                                                                                                                                                                  |
| Stale symlink pointing to deleted source | Information Disclosure | `remove_path_any` handles broken symlinks. Staleness detection checks `source.exists()`.                                                                                                                             |

## Sources

### Primary (HIGH confidence)

- Codebase inspection of `sync_engine.rs` -- all sync primitives verified
- Codebase inspection of `content_hash.rs` -- hash_dir() API verified
- Codebase inspection of `tool_adapters/mod.rs` -- adapter_by_key(), relative_skills_dir verified
- Codebase inspection of `skill_store.rs` -- V4 schema, CRUD patterns, migration mechanism verified
- Codebase inspection of `commands/mod.rs` -- existing sync command patterns verified
- Codebase inspection of `commands/projects.rs` -- Phase 1 command structure verified
- Codebase inspection of `project_ops.rs` -- Phase 1 core logic, remove_project_with_cleanup pattern verified
- Codebase inspection of `lib.rs` -- Tauri state registration, CancelToken pattern verified
- Codebase inspection of `cancel_token.rs` -- shared state pattern verified
- WSL2 filesystem tests -- symlink behavior across ext4/9p verified empirically
- `cargo test` -- all 96 existing tests pass

### Secondary (MEDIUM confidence)

- Phase 2 CONTEXT.md decisions (D-01 through D-11) -- user-locked decisions
- Phase 1 CONTEXT.md -- prior schema and architecture decisions

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH -- no new dependencies, everything verified in codebase
- Architecture: HIGH -- patterns directly follow existing codebase conventions and locked decisions
- Pitfalls: HIGH -- identified from actual code inspection and compiler behavior patterns

**Research date:** 2026-04-07
**Valid until:** 2026-05-07 (stable -- Rust stdlib and existing crate APIs don't change frequently)
