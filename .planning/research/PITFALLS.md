# Domain Pitfalls

**Domain:** Per-project symlink-based skill distribution in a Tauri 2 desktop app
**Researched:** 2026-04-07
**Confidence:** HIGH (derived from codebase analysis + official documentation)

## Critical Pitfalls

Mistakes that cause rewrites, data loss, or cross-platform failures.

---

### Pitfall 1: WSL2 Cross-Filesystem Symlink Breakage

**What goes wrong:** Symlinks created from the WSL2 Linux filesystem (`/home/...`) pointing into NTFS-mounted Windows directories (`/mnt/c/...`) or vice versa appear valid inside WSL but are invisible or broken when accessed from Windows tools. Many AI coding tools (Cursor, VS Code extensions, Windsurf) run as Windows processes that read the project directory via the Windows filesystem. A symlink created by `std::os::unix::fs::symlink` inside WSL points to a Linux path like `/home/alex/.skillshub/brainstorming` -- Windows processes cannot resolve this path.

**Why it happens:** WSL2 uses a Linux ext4 virtual disk for its filesystem. When a WSL symlink is created inside a Windows-mounted directory (`/mnt/c/Users/alex/Projects/...`), the symlink target is stored as a Linux path. Windows has no mechanism to resolve `/home/...` paths. The reverse direction (Windows symlinks visible in WSL) has different but related issues. Microsoft's own documentation recommends against working across filesystem boundaries for performance and compatibility.

**Consequences:**

- Skills appear "synced" in the UI (green indicator) but AI tools running as Windows processes cannot read them
- Silent failure -- no error raised, symlink creation succeeds, but the target is unreachable from the consuming tool
- Affects the primary dev environment (WSL2 is listed as the primary development platform)

**Warning signs:**

- Project paths start with `/mnt/c/` or `/mnt/d/` (NTFS mount)
- Central repo (`~/.skillshub/`) is on the Linux filesystem while projects are on NTFS
- `ls -la` in WSL shows valid symlink but Windows `dir` shows nothing or a broken link

**Prevention:**

1. **Detect cross-filesystem scenarios at registration time.** When `add_project` is called, check if the project path is on a different filesystem than `~/.skillshub/`. If the project is on NTFS (`/mnt/...`) and the central repo is on ext4, warn the user and auto-select `copy` mode instead of `symlink`.
2. **Add a `resolve_sync_mode` function** that inspects source and target mount points (parse `/proc/mounts` on Linux, or compare path prefixes `/mnt/` vs `/home/`) before choosing symlink vs copy.
3. **Test the actual resolution:** After creating a symlink, verify it resolves by checking `target.join("SKILL.md").exists()` rather than just checking the symlink creation succeeded.

**Detection:** Integration test that creates a symlink across `/mnt/c/` boundary and verifies file access works. If running on WSL2 (check for `/proc/sys/fs/binfmt_misc/WSLInterop`), run cross-filesystem validation.

**Phase:** Backend Foundation -- must be addressed in the `sync_skill_to_project` path resolution logic before any frontend work. The existing `sync_dir_hybrid` function does not check for cross-filesystem boundaries.

**Confidence:** HIGH -- verified against Microsoft's official WSL filesystem documentation and direct code inspection of `sync_engine.rs` which has no cross-filesystem awareness.

---

### Pitfall 2: `is_same_link` Path Comparison Fails with Non-Canonical Paths

**What goes wrong:** The existing `is_same_link` function at `sync_engine.rs:158-161` compares `read_link()` output directly against the target path using `==`. This raw path comparison fails when paths differ in representation but refer to the same location -- trailing slashes, `.` or `..` segments, different mount representations (`/mnt/c` vs `/mnt/c/`), or non-canonicalized home paths (`~/.skillshub` vs `/home/alex/.skillshub`).

**Why it happens:** `std::fs::read_link()` returns the exact bytes stored in the symlink, not a canonicalized path. If the symlink was created with `/home/alex/.skillshub/brainstorming` but compared against `~/.skillshub/brainstorming` (expanded differently), or if paths go through `expand_home_path()` which can produce subtly different results, the comparison fails. The idempotency check then treats an existing correct symlink as "different" and either errors or unnecessarily replaces it.

**Consequences:**

- "Sync All" replaces every symlink on every run (re-creates identical links)
- Brief broken state during unnecessary re-creation (race window for AI tools)
- False "stale" status indicators in the UI

**Warning signs:**

- "Sync Project" always reports changes even when nothing changed
- `replaced: true` in SyncOutcome when the link was already correct
- Status flickers between "synced" and "pending" on repeated syncs

**Prevention:**

1. **Canonicalize both paths before comparison.** Replace `existing == target` with `canonicalize(existing) == canonicalize(target)`, falling back to the raw comparison if canonicalization fails (broken symlink target).
2. **Store canonical paths in the database.** When recording `central_path` and `target_path`, always canonicalize first via `std::fs::canonicalize()` or a platform-aware normalization function.

**Phase:** Backend Foundation -- fix before implementing project sync, since project paths add more path variation than global paths (which are all under `~/`).

**Confidence:** HIGH -- directly observed in `sync_engine.rs` lines 158-161. The `expand_home_path` function in `commands/mod.rs:247-261` already shows path normalization is a known concern.

---

### Pitfall 3: SQLite Foreign Keys Silently Disabled on New Connections

**What goes wrong:** `PRAGMA foreign_keys = ON` is a per-connection setting in SQLite. The current codebase opens a new `Connection` on every `with_conn` call (line 452-458 in `skill_store.rs`) and correctly sets the pragma each time. However, the V4 migration adds tables with `ON DELETE CASCADE` foreign keys. If any code path -- a migration utility, a direct `Connection::open()` for debugging, a test setup, or a future refactoring that pools connections -- forgets to set the pragma, CASCADE deletes silently fail, leaving orphaned `project_skill_assignments` rows when a project or skill is deleted.

**Why it happens:** SQLite disables foreign key enforcement by default. The SQLite documentation explicitly states: "future releases of SQLite might change so that foreign key constraints are enabled by default. Careful developers will not make any assumptions." The current `with_conn` pattern handles this, but the risk increases as more tables and more code paths interact with the database.

**Consequences:**

- Deleting a project leaves behind orphaned assignment rows
- Deleting a skill leaves behind orphaned project assignments
- Orphaned rows accumulate silently; no visible error
- Assignment matrix shows skills for deleted projects (ghost data)
- The `db_has_any_skills` helper at line 524 opens its own connection WITHOUT setting `PRAGMA foreign_keys` -- this is fine for a read query, but establishes a pattern where non-`with_conn` access paths exist

**Warning signs:**

- After deleting a project, re-registering the same path shows stale assignments
- Assignment counts in project list don't decrease after skill deletion
- `SELECT count(*) FROM project_skill_assignments WHERE project_id NOT IN (SELECT id FROM projects)` returns > 0

**Prevention:**

1. **Centralize connection creation.** Ensure `with_conn` is the ONLY way to get a connection. Make the pattern enforceable (e.g., no public `Connection::open` calls outside `with_conn`).
2. **Add a test that verifies CASCADE behavior.** Insert a project + assignments, delete the project, assert assignments count is 0. This test will fail immediately if someone breaks the pragma.
3. **Consider `PRAGMA foreign_keys` in the migration itself.** Before running V4 migration DDL, explicitly enable foreign keys so that even the migration path enforces them.
4. **Add an index on child foreign keys.** Without `CREATE INDEX idx_psa_project_id ON project_skill_assignments(project_id)`, every DELETE on `projects` triggers a full table scan of `project_skill_assignments`. Same for `skill_id`.

**Phase:** Backend Foundation (schema migration) -- must be correct from the first migration. Fixing orphaned data after the fact is painful.

**Confidence:** HIGH -- verified against official SQLite foreign key documentation and direct code inspection. The `PRAGMA foreign_keys = ON` pattern exists in `with_conn` but the risk is in code paths that bypass it.

---

### Pitfall 4: Race Conditions in Concurrent Sync Operations

**What goes wrong:** The UI allows "Sync Project", "Sync All", and individual checkbox toggles simultaneously. Each operation calls `spawn_blocking` independently. If a user clicks "Sync All" then immediately toggles a checkbox, two concurrent filesystem operations may target the same path: one creating a symlink while the other removes/recreates it. The sync engine has no locking mechanism -- `sync_dir_hybrid_with_overwrite` does `remove_dir_all` then `sync_dir_hybrid` as separate filesystem operations with no atomicity.

**Why it happens:** The existing codebase serializes through `spawn_blocking` but doesn't coordinate between spawned tasks. The current architecture works because global syncs are user-initiated one-at-a-time operations. Per-project sync introduces batch operations ("Sync All Projects" iterates all projects, each with many assignments) that run concurrently with individual toggle operations.

**Consequences:**

- TOCTOU: `symlink_metadata(target).is_ok()` check passes, then another task creates/removes the target before the next operation
- Half-completed sync states: assignment marked "synced" in DB but symlink was removed by concurrent operation
- Error messages about "target already exists" when it shouldn't
- On Windows/NTFS, file locking means concurrent `remove_dir_all` + `sync_dir_hybrid` can produce `Access is denied` errors

**Warning signs:**

- Sporadic "target already exists" errors during "Sync All"
- UI status indicators inconsistent after bulk operations
- "Access is denied" errors on Windows that disappear on retry

**Prevention:**

1. **Serialize all sync operations through a single task queue.** Use a `tokio::sync::Mutex` or an `mpsc` channel to ensure only one sync operation runs at a time. The UI can still be responsive (optimistic updates) but the backend processes syncs sequentially.
2. **Disable UI interaction during batch syncs.** When "Sync All" is running, disable individual toggles. Show a progress indicator that prevents conflicting operations.
3. **Make toggle operations optimistic-then-queue.** The checkbox updates the UI immediately but queues the actual sync operation. If "Sync All" is already running, the toggle waits for it to complete.
4. **Use the existing CancelToken pattern** to allow "Sync All" to be cancelled before starting a conflicting operation.

**Phase:** Backend Foundation (sync coordinator) -- design the sync queueing before building the frontend, since the IPC contract depends on whether sync is synchronous, queued, or cancellable.

**Confidence:** HIGH -- the `spawn_blocking` pattern is observed throughout `commands/mod.rs`. The sync engine's `remove_dir_all` -> `sync_dir_hybrid` sequence at lines 75-88 of `sync_engine.rs` is explicitly non-atomic.

---

### Pitfall 5: Schema V4 Migration Breaks Rollback Path

**What goes wrong:** The existing migration pattern (lines 108-138 in `skill_store.rs`) uses `PRAGMA user_version` as a one-way gate. Once the version is bumped to 4, there is no rollback mechanism. If V4 migration partially completes (creates `projects` table but fails on `project_skill_assignments`), the database is in an inconsistent state: `user_version` may still be 3 (failure before pragma update) or may be 4 with missing tables (if pragma was updated before all DDL).

**Why it happens:** The existing migration does not use a transaction. Lines 112-119 run multiple `execute_batch` calls sequentially -- if the second succeeds but the third fails, partial state is committed. SQLite DDL (CREATE TABLE) is transactional, but only if explicitly wrapped in `BEGIN`/`COMMIT`.

**Consequences:**

- App fails to start on next launch because `ensure_schema()` sees version 4 but tables are missing
- User must manually delete the database file, losing all skill data
- Bug reports from users who updated the app and lost their configuration

**Warning signs:**

- Migration test only tests clean install (version 0 -> latest), not upgrade from version 3
- No explicit transaction wrapping around DDL statements
- `user_version` update happens at the end (good) but partial table creation still commits

**Prevention:**

1. **Wrap the entire V4 migration in a transaction.** `conn.execute_batch("BEGIN; ... CREATE TABLE projects ...; CREATE TABLE project_skill_assignments ...; COMMIT;")` or use rusqlite's `Transaction` type.
2. **Test upgrade from V3 to V4 specifically.** Create a V3 database fixture, run `ensure_schema()`, verify all V4 tables exist with correct columns.
3. **Add a migration smoke test in CI.** Take a real V3 database snapshot, run the migration, verify schema integrity.
4. **Backup before migration.** The existing `migrate_legacy_db_if_needed` pattern already creates backups. Apply the same pattern to schema upgrades: copy the DB file before attempting migration.

**Phase:** Backend Foundation -- the migration is literally the first thing that runs. Getting it wrong means data loss.

**Confidence:** HIGH -- directly observed that existing migrations at lines 112-127 are not wrapped in explicit transactions.

---

## Moderate Pitfalls

Mistakes that cause significant debugging time or poor UX but are recoverable.

---

### Pitfall 6: `relative_skills_dir` Semantics Change Between Global and Project-Local

**What goes wrong:** The `ToolAdapter.relative_skills_dir` field (e.g., `.claude/skills`) is currently used only for global paths: `home.join(relative_skills_dir)`. For project-local sync, the same field would produce `project_path.join(".claude/skills/skill_name")`. But some tools define skills directories that don't make sense as project-local paths. For example, `.config/agents/skills` (Amp/Kimi CLI) is a global config pattern -- no project would have a `.config/agents/skills/` subdirectory. Similarly, `.gemini/antigravity/global_skills` has "global" in the name.

**Why it happens:** The tool adapter registry was designed for global-only sync. The `relative_skills_dir` field conflates "where the tool stores skills globally" with "where the tool expects skills in a project." These may not be the same for all tools.

**Prevention:**

1. **Add an optional `relative_project_skills_dir` field to ToolAdapter.** Default to `relative_skills_dir` but allow overrides for tools where the project-local path differs from the global path.
2. **Research and document which of the 42+ tools actually support project-local skill loading.** Only expose tools with verified project-local support in the assignment matrix. If a tool only reads from its global directory, project-local symlinks are wasted effort.
3. **Start with a curated subset.** For the MVP, only support Claude Code (`.claude/skills/`), Cursor (`.cursor/skills/`), Codex (`.codex/skills/`), and similar tools where the project-local convention is well-established. Add others incrementally.

**Phase:** Backend Foundation -- the path resolution function must handle this distinction. Frontend matrix columns depend on which tools support project-local sync.

**Confidence:** MEDIUM -- inferred from code inspection of tool_adapters/mod.rs. Not all 42 tools have documented project-local skill directory conventions. Needs verification per-tool.

---

### Pitfall 7: App.tsx State Leaks Into Projects Tab

**What goes wrong:** The design calls for a separate `ProjectsTab` component tree with its own state. But the existing pattern in App.tsx is that ALL state lives in the top-level component and gets passed as props. If the Projects tab needs to know about `managedSkills` (to populate the assignment matrix), `toolStatus` (to show tool columns), or `centralRepoPath` (to resolve source paths), the temptation is to thread these from App.tsx via props. This creates a dependency where App.tsx must re-render whenever the Projects tab's parent data changes, negating the benefit of extraction.

**Why it happens:** There is no state management layer -- App.tsx IS the state layer. The design doc (Section 4.4) says "separate component tree with its own state management" but doesn't specify how the Projects tab will access shared data (skill list, tool status) without going through App.tsx.

**Prevention:**

1. **Projects tab fetches its own data via IPC.** Instead of receiving `managedSkills` as a prop from App.tsx, the ProjectsTab calls `invoke('get_managed_skills')` directly. This decouples it from App.tsx's render cycle.
2. **Pass only stable references.** If the tab must receive data from App.tsx, limit it to: `activeView` (string), `store` (stable reference), and callback functions. No large arrays or objects.
3. **Use `React.memo` aggressively on the tab boundary.** Wrap ProjectsTab in `React.memo` with a custom comparator that only re-renders when `activeView === 'projects'`.
4. **Consider `useReducer` for local tab state.** The assignment matrix state (checked cells, sync statuses, filter/search) is complex enough to benefit from a reducer pattern instead of 10+ individual `useState` calls.

**Phase:** Frontend (Phase 2) -- but the architecture decision must be made before coding starts. Retrofitting state isolation is expensive.

**Confidence:** HIGH -- directly observed App.tsx at 2086 lines with 50+ useState calls. The pattern of threading state as props is well-established in the codebase.

---

### Pitfall 8: Stale Project Paths After Directory Rename/Move/Delete

**What goes wrong:** The `projects` table stores the filesystem path as a string. If the user renames, moves, or deletes the project directory, the stored path becomes stale. Unlike symlinks (which break visibly), a stale project path in the database silently causes all syncs to fail with "directory not found" errors. Worse, if the user creates a new directory at the old path, syncs resume but target the wrong project.

**Why it happens:** There is no filesystem watcher and no inode-based tracking. The path is the identity.

**Prevention:**

1. **Validate project paths on list load.** When `list_projects` is called, check `Path::exists()` for each project and include an `exists: bool` field in the response DTO. Frontend can show a warning badge.
2. **Do NOT auto-remove stale projects.** The user may have temporarily unmounted a drive or renamed then renamed back. Show a warning and let the user manually remove or update the path.
3. **Add an `update_project_path` command** for the case where a project was moved. Transfer all assignments to the new path.
4. **Validate on sync, not just on list.** Before `sync_project`, verify the path exists. Return a structured error (`PROJECT_PATH_MISSING|...`) that the frontend can handle with a specific modal.

**Phase:** Backend Foundation + Frontend polish. The validation logic belongs in the backend, but the UX for handling stale paths is a frontend concern.

**Confidence:** HIGH -- the design doc explicitly mentions this as a requirement ("Handle removed/renamed project directories gracefully") and it is in the Active requirements list.

---

### Pitfall 9: Shared Skills Directory Creates Duplicate Assignments

**What goes wrong:** Some tools share the same skills directory (e.g., Amp and Kimi CLI both use `.config/agents/skills`). The existing global sync handles this with `adapters_sharing_skills_dir()` -- creating one filesystem operation but multiple DB records. For project-local sync, the same logic must apply: if a user assigns a skill to Amp and Kimi CLI for the same project, only one symlink should be created in the project, but the assignment matrix should show both as checked. If this sharing isn't handled, duplicate symlink creation will fail with "target already exists."

**Why it happens:** The `adapters_sharing_skills_dir` pattern exists in `commands/mod.rs:523` for global sync but would need to be replicated (or extracted) for project sync. It's easy to miss because the shared-directory case only affects 2 out of 42 tools currently.

**Prevention:**

1. **Extract the shared-dir logic into a reusable utility.** Move the pattern from `sync_skill_to_tool` into a function that both global and project sync can call.
2. **In the assignment matrix UI, link shared-dir tools visually.** When toggling Amp, auto-toggle Kimi CLI (with a tooltip explaining why). This prevents user confusion about why unchecking one unchecks another.
3. **Test the shared-dir case explicitly** in project sync unit tests.

**Phase:** Backend Foundation -- extract the shared-dir utility before implementing project sync.

**Confidence:** HIGH -- directly observed in `commands/mod.rs:523` and `tool_adapters/mod.rs:418-423`.

---

### Pitfall 10: .gitignore Prompt Timing and Content

**What goes wrong:** The design calls for prompting users to add tool skill directories to `.gitignore` on project registration. If this is done wrong: (a) the user registers a project but declines the gitignore prompt, later commits symlinks to git, which break on clone because the symlink targets are machine-specific; (b) the gitignore entry is too broad and excludes files the user wants tracked; (c) the prompt fires every time the project is selected, not just on registration.

**Why it happens:** Each tool has a different skill directory pattern. Claude Code uses `.claude/skills/`, Cursor uses `.cursor/skills/`, etc. A blanket `.*skills*` pattern could catch unrelated files. But listing each tool's pattern individually produces an unwieldy gitignore block.

**Prevention:**

1. **Generate tool-specific gitignore entries based on which tools the user configures for the project.** Only add entries for tools the user actually enables in the matrix.
2. **Track whether the gitignore prompt was shown** via a flag in the `projects` table (`gitignore_prompted: boolean`). Only show once per project.
3. **Offer to APPEND, not overwrite.** Read the existing `.gitignore`, check if the entry already exists, and only suggest additions.
4. **Default to the symlink approach** (which is the primary mode). Symlinks in git repos are valid on Linux/macOS but break on Windows clone. The gitignore entry prevents this footgun.

**Phase:** Frontend (Phase 2) + Polish (Phase 3). The prompt UX is frontend, but the gitignore content generation needs backend tool knowledge.

**Confidence:** MEDIUM -- the requirement exists in the Active list but the design doc doesn't specify the detailed behavior.

---

## Minor Pitfalls

Mistakes that cause small annoyances but are easy to fix.

---

### Pitfall 11: Tauri Folder Picker Returns Windows Paths in WSL

**What goes wrong:** Tauri's `dialog.open({ directory: true })` on WSL2 may return a Windows-format path (`C:\Users\alex\Projects\BDA`) instead of a WSL-mounted path (`/mnt/c/Users/alex/Projects/BDA`). The backend Rust code expects Unix paths for `Path::join` and symlink operations.

**Prevention:**

1. Add a path normalization layer in the `add_project` command that detects Windows-style paths and converts them to WSL mount paths.
2. Offer a manual path entry fallback (already in the design) for cases where the folder picker misbehaves.
3. Test the folder picker on WSL2 early (Phase 1) to determine actual behavior.

**Phase:** Backend Foundation -- path handling in `add_project` command.

**Confidence:** LOW -- the design doc flags this as a risk ("Tauri folder picker may not work well on WSL") but the actual behavior is unverified. Needs early testing.

---

### Pitfall 12: Optimistic UI Revert Complexity

**What goes wrong:** The design calls for optimistic UI updates when toggling checkboxes (update UI immediately, revert on error). With a matrix of N skills x M tools, reverting a failed batch operation (e.g., "Enable all for Claude Code") requires tracking which individual cells were changed and reverting only those, while preserving any concurrent changes from other operations.

**Prevention:**

1. **Track pending operations with a Map.** Key: `${projectId}:${skillId}:${tool}`, Value: previous state. On error, revert only the keys in the pending map.
2. **For batch operations, use a loading state** instead of optimistic updates. "Enable all" is rare enough that waiting 1-2 seconds for confirmation is acceptable.
3. **Debounce rapid individual toggles** (e.g., 300ms) to batch them into a single IPC call.

**Phase:** Frontend (Phase 2).

**Confidence:** MEDIUM -- standard UI pattern but tricky to get right with concurrent operations.

---

### Pitfall 13: Content Hash Staleness Detection for Symlinks is Unnecessary

**What goes wrong:** The design includes content hash staleness detection for all assignments. But for symlink-mode targets, staleness detection is meaningless -- symlinks always point to the current source content. Computing and storing content hashes for symlink assignments wastes storage and CPU, and showing "stale" indicators for symlinks confuses users.

**Prevention:**

1. **Only compute and store content_hash for copy-mode assignments.** When `mode = "symlink"`, skip the hash computation entirely.
2. **Only show stale (yellow) indicators for copy-mode cells.** Symlink cells can only be: synced (green), missing/broken (red), or pending (gray).
3. **Document this distinction** in the status indicator legend.

**Phase:** Backend Foundation (staleness check function) + Frontend (status indicator rendering).

**Confidence:** HIGH -- symlinks are transparent filesystem pointers. Hashing through a symlink measures the source, which is always "current" by definition.

---

## Phase-Specific Warnings

| Phase Topic             | Likely Pitfall                                                                    | Mitigation                                                                              |
| ----------------------- | --------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| Schema V4 Migration     | Partial migration without transaction wrapping leaves broken database (Pitfall 5) | Wrap all DDL in a single transaction; test V3->V4 upgrade path; backup before migration |
| Project Path Resolution | Cross-filesystem symlinks invisible to Windows tools (Pitfall 1)                  | Detect cross-fs scenarios; auto-fall-back to copy mode on WSL2 NTFS mounts              |
| Sync Engine Integration | Non-canonical path comparison causes unnecessary re-syncs (Pitfall 2)             | Canonicalize paths before `is_same_link` comparison                                     |
| Project Sync Commands   | Race conditions between "Sync All" and individual toggles (Pitfall 4)             | Serialize sync operations; disable conflicting UI during batch ops                      |
| Tool Adapter Extension  | `relative_skills_dir` conflates global and project-local paths (Pitfall 6)        | Add optional `relative_project_skills_dir` field; curate supported tools                |
| Frontend Extraction     | App.tsx state leaks into Projects tab via props drilling (Pitfall 7)              | Projects tab fetches own data via IPC; minimize prop interface                          |
| Assignment Matrix UI    | Shared-dir tools create duplicate assignments (Pitfall 9)                         | Extract shared-dir utility; link related tools in UI                                    |
| Project Registration    | Stale paths after rename/move (Pitfall 8)                                         | Validate on list and sync; structured error for missing paths                           |
| .gitignore Handling     | Wrong timing, too broad patterns, repeated prompts (Pitfall 10)                   | Tool-specific entries; one-time prompt flag; append-only                                |

## Sources

- Microsoft WSL filesystem documentation: https://learn.microsoft.com/en-us/windows/wsl/filesystems
- Microsoft WSL configuration: https://learn.microsoft.com/en-us/windows/wsl/wsl-config
- SQLite foreign key documentation: https://www.sqlite.org/foreignkeys.html
- rusqlite API documentation: https://docs.rs/rusqlite/latest/rusqlite/
- Direct code inspection: `src-tauri/src/core/sync_engine.rs`, `src-tauri/src/core/skill_store.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/core/tool_adapters/mod.rs`, `src-tauri/src/lib.rs`, `src/App.tsx`

---

_Pitfalls analysis: 2026-04-07_
