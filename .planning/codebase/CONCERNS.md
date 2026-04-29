# Codebase Concerns

**Analysis Date:** 2026-04-29

## Tech Debt

**Frontend orchestration concentration:**

- Issue: `src/App.tsx` is 2,532 lines and owns application state, settings, onboarding, update checks, global sync flows, modal state, explore flows, and direct Tauri command wiring. This exceeds the project skill `code-simplification` review threshold for file/function complexity and makes cross-feature edits risky.
- Files: `src/App.tsx`
- Impact: Small feature work requires navigating unrelated concerns, effect dependencies are suppressed in places, and duplicated async install/sync flows are easy to diverge.
- Fix approach: Keep new per-project state in `src/components/projects/` as established by `src/components/projects/useProjectState.ts`; when touching existing global flows, extract cohesive hooks around settings, updater, onboarding import, and install/sync orchestration before adding more state to `src/App.tsx`.

**Duplicated install-then-sync loops:**

- Issue: Local candidate install, Git candidate install, onboarding import, and global auto-sync all repeat the same pattern of computing installed tool targets, looping tools, invoking `sync_skill_to_tool`, collecting errors, and refreshing skills.
- Files: `src/App.tsx:1800`, `src/App.tsx:1877`, `src/App.tsx:1285`, `src/App.tsx:2013`
- Impact: Behavior changes for sync error handling, target de-duplication, overwrite options, or cancellation must be made in multiple branches; one branch can silently miss fixes.
- Fix approach: Extract one helper/hook in `src/App.tsx` or a new `src/components/skills/useSkillSyncActions.ts` that accepts created skill metadata plus selected target IDs and returns collected errors; keep UI-specific progress messages at call sites.

**Direct IPC import split:**

- Issue: Main skill flows use guarded dynamic `invokeTauri()` in `src/App.tsx`, while the project subtree imports `invoke` directly and assumes Tauri context.
- Files: `src/App.tsx:169`, `src/components/projects/useProjectState.ts:2`, `src/components/projects/ProjectsPage.tsx:3`
- Impact: Browser-only development, test harnesses, or future web previews fail harder in the project subtree than in the rest of the app; error normalization and command wrappers are inconsistent.
- Fix approach: Introduce a small shared IPC module such as `src/lib/tauri.ts` with `invokeTauri<T>()` and reuse it from both `src/App.tsx` and `src/components/projects/`.

**Dormant router shell:**

- Issue: `src/components/Layout.tsx` and `src/pages/Dashboard.tsx` define an alternate React Router-style shell that is not wired into `src/main.tsx` or `src/App.tsx` and uses different styling conventions.
- Files: `src/components/Layout.tsx`, `src/pages/Dashboard.tsx`, `src/main.tsx`, `src/App.tsx`
- Impact: Contributors may place new UI in an unused route tree, creating dead features that compile but never render.
- Fix approach: Either connect the router shell deliberately from `src/main.tsx` or delete the dormant files; new visible screens should be added through `src/App.tsx` / `src/components/skills/` / `src/components/projects/` until routing is intentionally adopted.

**Suppression and dead-code allowances:**

- Issue: Several production files carry `#[allow(dead_code)]`, `#[allow(clippy::too_many_arguments)]`, and `eslint-disable-next-line react-hooks/exhaustive-deps` suppressions.
- Files: `src/App.tsx:1270`, `src/App.tsx:1742`, `src-tauri/src/core/sync_engine.rs:5`, `src-tauri/src/core/skill_lock.rs:13`, `src-tauri/src/core/github_download.rs:47`, `src-tauri/src/core/tool_adapters/mod.rs:442`
- Impact: Suppressions hide real drift: unused helper APIs remain in production modules and React effect dependency mistakes can survive linting.
- Fix approach: Treat new suppressions as exceptions requiring a nearby reason; remove obsolete `dead_code` allowances when the API is unused outside tests, and prefer stable callbacks/refs over dependency lint suppression.

**Project status mutation during reads:**

- Issue: Listing assignments also recalculates staleness and writes updated statuses back to SQLite.
- Files: `src-tauri/src/core/project_sync.rs:226`, `src-tauri/src/core/project_sync.rs:273`, `src-tauri/src/core/project_sync.rs:311`, `src-tauri/src/commands/projects.rs:221`
- Impact: Read commands have side effects, making UI refreshes capable of changing persisted state and complicating debugging of sync status transitions.
- Fix approach: Split state inspection from persistence: expose a pure status computation helper and call a separate reconciliation/update command when the UI explicitly refreshes or resyncs.

**Settings table stores sensitive and non-sensitive values identically:**

- Issue: GitHub tokens are persisted through `settings` as plaintext string values using the same API as UI settings and cache data.
- Files: `src-tauri/src/commands/mod.rs:692`, `src-tauri/src/commands/mod.rs:703`, `src-tauri/src/core/skill_store.rs:47`, `src/App.tsx:421`, `src/App.tsx:698`
- Impact: A database copy exposes the GitHub token; returning the token to the frontend keeps it in React state and increases exposure through debugging tools.
- Fix approach: Move credentials to an OS keychain/secure storage plugin; if that is deferred, store only a masked presence flag in frontend state and require overwrite/clear actions rather than retrieving the full token.

## Known Bugs

**Project path update does not migrate existing artifacts:**

- Symptoms: Updating a registered project path changes the database path only; existing symlinks/copies in the old project remain, and assignment records continue to point conceptually at the new path without cleaning the old targets.
- Files: `src-tauri/src/core/project_ops.rs:241`, `src-tauri/src/core/skill_store.rs:607`, `src/components/projects/ProjectsPage.tsx:207`
- Trigger: Register a project, assign one or more skills, use the update path action, then inspect the old project directory.
- Workaround: Manually remove old `.claude/skills` or equivalent tool directories, then resync the updated project.

**Adding a project tool does not sync existing assignments for that tool:**

- Symptoms: Tool configuration persists new tools, but existing assigned skills are not automatically materialized for newly added tools unless the user runs a resync or toggles assignments.
- Files: `src/components/projects/useProjectState.ts:355`, `src-tauri/src/commands/projects.rs:69`, `src-tauri/src/core/project_sync.rs:166`
- Trigger: Create a project with assignments, reopen tool configuration, add a new tool, and inspect that tool's project skill directory before resync.
- Workaround: Use the project resync button after adding tools.

**Project deletion can leave artifacts for non-synced error statuses:**

- Symptoms: Project cleanup removes artifacts only for assignments with `synced` or `stale` status; assignments marked `error`, `missing`, or `pending` are deleted from SQLite without attempting filesystem cleanup.
- Files: `src-tauri/src/core/project_ops.rs:172`, `src-tauri/src/core/project_ops.rs:179`, `src-tauri/src/core/sync_engine.rs:137`
- Trigger: Create an assignment that leaves a target on disk but records an error status, then remove the project.
- Workaround: Manually remove managed skill directories from the project before or after deleting the project record.

**Skill detail read errors are displayed as file content:**

- Symptoms: File read failures set the active file content to the raw error string instead of using the normal toast/error display path.
- Files: `src/components/skills/SkillDetailView.tsx:458`, `src/components/skills/SkillDetailView.tsx:469`
- Trigger: Select a file that becomes unreadable or disappears after the file tree loads.
- Workaround: Reopen the skill detail view to refresh the file list; inspect app logs for the original backend error.

**Gitignore status silently treats unreadable files as unchecked:**

- Symptoms: Status checks use `unwrap_or_default()` for `.gitignore` and `.git/info/exclude`, so permission or encoding read failures look the same as no Skills Hub marker.
- Files: `src-tauri/src/commands/projects.rs:560`, `src-tauri/src/commands/projects.rs:578`, `src-tauri/src/commands/projects.rs:583`
- Trigger: Register a project where `.gitignore` or `.git/info/exclude` exists but cannot be read.
- Workaround: Fix permissions manually, then reopen or refresh project settings.

## Security Considerations

**Arbitrary project path writes through project gitignore command:**

- Risk: The app writes `.gitignore` and `.git/info/exclude` inside any registered directory, and project registration accepts any canonicalized directory path.
- Files: `src-tauri/src/core/project_ops.rs:73`, `src-tauri/src/commands/projects.rs:392`, `src-tauri/src/commands/projects.rs:488`, `src-tauri/src/commands/projects.rs:517`
- Current mitigation: Registration requires the path to exist and be a directory; writes are limited to `.gitignore` and `.git/info/exclude` below the registered path.
- Recommendations: Keep the native directory picker as the primary UI path source, add explicit confirmation for paths outside the user's home or known project roots, and avoid exposing project write commands to untrusted web content.

**Skill file browser trusts frontend-supplied central path:**

- Risk: `list_skill_files` and `read_skill_file` take `central_path` from the frontend instead of looking up a skill ID in the database; `read_file` protects traversal relative to the supplied base, but `list_files` lists any directory the frontend passes.
- Files: `src-tauri/src/commands/mod.rs:1100`, `src-tauri/src/commands/mod.rs:1120`, `src-tauri/src/core/skill_files.rs:19`, `src-tauri/src/core/skill_files.rs:58`
- Current mitigation: Tauri commands are not exposed to arbitrary external web pages under the current desktop capability file; `read_file` canonicalizes the requested relative file under the supplied base and limits files to 1 MB.
- Recommendations: Change commands to accept `skillId` and resolve `central_path` server-side from `src-tauri/src/core/skill_store.rs`; apply canonical central-repo containment checks to both listing and reading.

**Plaintext credential persistence:**

- Risk: The optional GitHub token is stored in the SQLite settings table and returned unmasked to the frontend.
- Files: `src-tauri/src/commands/mod.rs:678`, `src-tauri/src/commands/mod.rs:692`, `src-tauri/src/commands/mod.rs:703`, `src/App.tsx:421`, `src/App.tsx:641`
- Current mitigation: Token use is limited to GitHub HTTP requests and is not printed by the code paths inspected.
- Recommendations: Store credentials in OS secure storage; redact token values in all UI state and logs; avoid including settings database files in support bundles.

**Recursive copy and cleanup act on resolved tool/project paths:**

- Risk: Sync and cleanup functions can create, overwrite, copy, or remove directories beneath resolved tool/project paths; a wrong adapter path or corrupted database path can affect unintended filesystem locations.
- Files: `src-tauri/src/core/sync_engine.rs:60`, `src-tauri/src/core/sync_engine.rs:91`, `src-tauri/src/core/sync_engine.rs:137`, `src-tauri/src/core/project_sync.rs:365`, `src-tauri/src/commands/mod.rs:902`
- Current mitigation: Most paths are derived from registered projects, central repo records, and tool adapters; overwrite is explicit in command flows.
- Recommendations: Before destructive removal, verify targets are within adapter-relative skills directories and, where possible, are symlinks/copies that Skills Hub created; keep `SyncMutex` coverage around all filesystem mutations.

## Performance Bottlenecks

**Project list performs multiple SQLite queries per row:**

- Problem: Building project DTOs calls count/status queries for every project.
- Files: `src-tauri/src/core/project_ops.rs:54`, `src-tauri/src/core/project_ops.rs:232`, `src-tauri/src/core/skill_store.rs:925`, `src-tauri/src/core/skill_store.rs:936`, `src-tauri/src/core/skill_store.rs:947`, `src-tauri/src/core/skill_store.rs:958`
- Cause: `to_project_dto()` computes `tool_count`, `skill_count`, `assignment_count`, and aggregate sync status independently per project.
- Improvement path: Replace per-project counting with one aggregate query or a summary view that joins/group-counts all projects at once.

**Copy-mode sync hashes and copies whole directories:**

- Problem: Cursor project sync forces copy mode and staleness detection hashes full source directories.
- Files: `src-tauri/src/core/sync_engine.rs:117`, `src-tauri/src/core/sync_engine.rs:123`, `src-tauri/src/core/project_sync.rs:141`, `src-tauri/src/core/project_sync.rs:314`, `src-tauri/src/core/content_hash.rs`
- Cause: Copy fallback needs physical files; staleness compares directory hashes rather than using incremental file metadata or stored source revisions.
- Improvement path: Cache content hashes on skill update/install and reuse them consistently; only rehash when source metadata changes, and avoid copying unchanged files when source and target hashes match.

**Bulk/project sync is serialized and blocking:**

- Problem: A global `SyncMutex` serializes assignment, unassignment, resync, and cleanup operations; long copy operations block other sync tasks.
- Files: `src-tauri/src/lib.rs:11`, `src-tauri/src/commands/projects.rs:30`, `src-tauri/src/commands/projects.rs:146`, `src-tauri/src/commands/projects.rs:262`, `src-tauri/src/commands/projects.rs:287`, `src-tauri/src/commands/projects.rs:327`
- Cause: The mutex protects filesystem consistency but has global granularity rather than target-path granularity.
- Improvement path: Keep correctness first, then move to per-target or per-project locks with explicit conflict keys once sync operations expose stable source/target paths.

**Large skill lists render all cards and matrix cells:**

- Problem: Skill lists and project assignment matrix render directly from in-memory arrays without virtualization.
- Files: `src/components/skills/SkillsList.tsx`, `src/components/projects/AssignmentMatrix.tsx:103`, `src/components/projects/AssignmentMatrix.tsx:144`
- Cause: Sorting/grouping is memoized, but all visible skill rows/cards and tool cells are still rendered in one pass.
- Improvement path: For large libraries, add windowing or pagination to `src/components/skills/SkillsList.tsx` and `src/components/projects/AssignmentMatrix.tsx`; keep grouping logic memoized and use stable row components.

**Featured skills failures are fully silent:**

- Problem: Network, parse, cache, and bundled JSON parse failures collapse to cached/bundled/empty results without surfacing diagnostics to the UI.
- Files: `src-tauri/src/core/featured_skills.rs:48`, `src-tauri/src/core/featured_skills.rs:57`, `src-tauri/src/core/featured_skills.rs:66`
- Cause: Fallback logic prioritizes resilience but uses `if let` and `unwrap_or_default()` without logging.
- Improvement path: Keep fallback behavior, but log one warning per failed source so stale or empty Explore content can be diagnosed.

## Fragile Areas

**Project sync lifecycle:**

- Files: `src-tauri/src/core/project_sync.rs`, `src-tauri/src/core/project_ops.rs`, `src-tauri/src/commands/projects.rs`, `src/components/projects/useProjectState.ts`, `src/components/projects/AssignmentMatrix.tsx`
- Why fragile: Sync status is stored in SQLite, recalculated during reads, mutated during assignment/removal/resync, and projected into UI badges; filesystem state and database state can diverge.
- Safe modification: Add or update tests in `src-tauri/src/core/tests/project_sync.rs` and `src-tauri/src/core/tests/project_ops.rs` before changing status transitions; keep `src/components/projects/types.ts` aligned with Rust DTOs.
- Test coverage: Backend project sync tests exist in `src-tauri/src/core/tests/project_sync.rs`, but frontend project state behavior in `src/components/projects/useProjectState.ts` has no detected frontend test runner.

**Global skill deletion cleanup:**

- Files: `src-tauri/src/commands/mod.rs:900`, `src-tauri/src/commands/mod.rs:913`, `src-tauri/src/commands/mod.rs:922`, `src-tauri/src/core/sync_engine.rs:137`
- Why fragile: Deletion removes global targets, project artifacts, central repo directories, and database records in one command; partial cleanup failures can leave a mixed state.
- Safe modification: Preserve the current order of collecting cleanup failures before database deletion unless a transaction/rollback strategy is introduced; add tests in `src-tauri/src/commands/tests/commands.rs` for partial cleanup behavior.
- Test coverage: Backend command tests exist in `src-tauri/src/commands/tests/commands.rs`, but destructive filesystem edge cases around project artifacts need targeted coverage.

**GitHub install/download fallback paths:**

- Files: `src-tauri/src/core/installer.rs:180`, `src-tauri/src/core/installer.rs:1734`, `src-tauri/src/core/github_download.rs:28`, `src-tauri/src/core/git_fetcher.rs`
- Why fragile: Install logic combines sparse clone, GitHub Contents API download, cached clone fallback, rate-limit translation, cancellation, and partial directory cleanup.
- Safe modification: Keep cancellation prefix `CANCELLED|` and rate-limit handling `RATE_LIMITED|` intact; update tests in `src-tauri/src/core/tests/installer.rs`, `src-tauri/src/core/tests/github_search.rs`, and inline tests in `src-tauri/src/core/github_download.rs` for any flow change.
- Test coverage: Backend coverage is present, but end-to-end frontend install/cancel flows are not covered by detected frontend tests.

**Tool adapter registry:**

- Files: `src-tauri/src/core/tool_adapters/mod.rs`, `src-tauri/src/core/tests/tool_adapters.rs`, `src/i18n/resources.ts`, `src/components/skills/types.ts`
- Why fragile: Adding a tool requires adapter metadata, shared-directory grouping behavior, frontend labels, and project relative skills directory behavior to stay consistent.
- Safe modification: Add adapter tests in `src-tauri/src/core/tests/tool_adapters.rs`; verify shared skills directory grouping in both `src/App.tsx` and project assignment flows.
- Test coverage: Backend adapter tests exist, but UI presentation and per-project tool configuration are not covered by frontend tests.

**React effect dependency suppressions:**

- Files: `src/App.tsx:1244`, `src/App.tsx:1270`, `src/App.tsx:1742`
- Why fragile: Suppressed dependency linting can freeze callbacks/state in effects and makes future refactors harder to reason about.
- Safe modification: Replace suppressed effects with stable refs or extracted hooks; run `npm run lint` and manually verify initial load, tool detection, and modal flows after changes.
- Test coverage: No detected frontend test suite covers these hooks, so regressions rely on manual verification and TypeScript/lint checks.

## Scaling Limits

**SQLite access opens a new connection for every store operation:**

- Current capacity: Adequate for local desktop usage with small-to-medium skill/project counts.
- Limit: Repeated `with_conn()` calls create many short-lived SQLite connections during list and sync operations.
- Scaling path: Introduce a connection pool or batch APIs for high-churn operations; group project summary queries instead of calling many store methods per DTO.

**Project assignment matrix grows as skills × tools:**

- Current capacity: Comfortable for dozens of skills and configured tools.
- Limit: Hundreds of skills across many tools produce large DOM tables and many status lookups.
- Scaling path: Virtualize rows, memoize cell components, and add search/filter controls directly in `src/components/projects/AssignmentMatrix.tsx`.

**GitHub unauthenticated API quota:**

- Current capacity: 60 requests/hour without token and 5,000/hour with token, reflected in settings copy.
- Limit: GitHub Contents API directory downloads make one request per directory plus file downloads; large repos or repeated explore/install actions can hit rate limits.
- Scaling path: Prefer cached clone paths for repeated repositories, keep token configuration visible, and batch/cancel requests where possible.

**Copy fallback duplicates skill directories:**

- Current capacity: Acceptable for small text-based skills.
- Limit: Copy mode duplicates data per project/tool and can become expensive for binary-heavy or large skill directories.
- Scaling path: Keep skills text-only where possible, skip generated/heavy directories during copy, and add per-skill size warnings before assignment.

## Dependencies at Risk

**No frontend test framework:**

- Risk: React state orchestration, project matrix interactions, settings persistence, and updater/install UI flows rely on TypeScript and manual checks only.
- Impact: UI regressions in `src/App.tsx`, `src/components/projects/useProjectState.ts`, and modal components can pass `npm run check` if types/lint/build remain valid.
- Migration plan: Add a lightweight Vitest + React Testing Library setup focused on hooks and components; start with `src/components/projects/useProjectState.ts` and pure formatting/error helpers.

**GitHub API and git clone behavior:**

- Risk: External rate limits, repository layout changes, branch naming, private repository auth, and network/proxy failures directly affect install and explore flows.
- Impact: `src-tauri/src/core/installer.rs`, `src-tauri/src/core/github_download.rs`, `src-tauri/src/core/github_search.rs`, and `src-tauri/src/core/git_fetcher.rs` can fail even when local app logic is correct.
- Migration plan: Keep dual-path download fallback, improve cached metadata, and surface explicit retry/token guidance through `src/App.tsx` error translation.

**Tauri updater and desktop plugin permissions:**

- Risk: Updater, dialog, and webview zoom capabilities are tightly tied to Tauri plugin behavior and permissions.
- Impact: Changes in Tauri permissions can break `src/App.tsx` update checks, native directory pickers, or zoom persistence without compile-time failures.
- Migration plan: Keep `src-tauri/capabilities/default.json` minimal and add smoke tests or release checklist steps that exercise updater check, dialog open, and zoom commands.

## Missing Critical Features

**Frontend automated tests:**

- Problem: No dedicated frontend test runner is detected for React components and hooks.
- Blocks: Safe refactoring of `src/App.tsx`, `src/components/projects/useProjectState.ts`, `src/components/projects/AssignmentMatrix.tsx`, and modal flows.

**Secure credential storage:**

- Problem: GitHub token persistence uses the same SQLite settings path as normal app settings.
- Blocks: Treating GitHub token support as production-grade credential handling.

**Explicit sync artifact ownership markers:**

- Problem: Cleanup uses paths/statuses but does not consistently verify that each target was created by Skills Hub before removal.
- Blocks: Strong safety guarantees for destructive cleanup in `src-tauri/src/core/sync_engine.rs`, `src-tauri/src/core/project_sync.rs`, and `src-tauri/src/commands/mod.rs`.

**First-class project path migration:**

- Problem: Updating a project path does not move or clean existing tool directories.
- Blocks: Reliable relocation of registered projects in `src/components/projects/ProjectsPage.tsx` and `src-tauri/src/core/project_ops.rs`.

## Test Coverage Gaps

**Frontend project state and matrix interactions:**

- What's not tested: Project selection race handling, assignment toggles, bulk assign, tool add/remove refresh behavior, and resync UI feedback.
- Files: `src/components/projects/useProjectState.ts`, `src/components/projects/AssignmentMatrix.tsx`, `src/components/projects/ProjectsPage.tsx`
- Risk: UI can show stale assignment state, double-submit cells, or miss required refreshes without failing backend tests.
- Priority: High

**Global App install/sync flows:**

- What's not tested: Add local/Git skill, multi-candidate selection, onboarding import, auto-sync target de-duplication, cancellation UI, update prompt behavior, and settings persistence.
- Files: `src/App.tsx`, `src/components/skills/modals/*.tsx`, `src/components/skills/SettingsPage.tsx`
- Risk: Large duplicated flows can diverge while TypeScript still compiles.
- Priority: High

**Credential handling:**

- What's not tested: Token masking/clearing, persistence failure handling, and accidental token exposure in UI state/logs.
- Files: `src-tauri/src/commands/mod.rs:692`, `src-tauri/src/commands/mod.rs:703`, `src/App.tsx:421`, `src/App.tsx:698`, `src/components/skills/SettingsPage.tsx`
- Risk: Secrets can remain visible or persisted unexpectedly.
- Priority: Medium

**Destructive cleanup edge cases:**

- What's not tested: Project deletion for `error`/`pending` assignments, project path updates with existing artifacts, unreadable targets, and cleanup after missing skill records.
- Files: `src-tauri/src/core/project_ops.rs`, `src-tauri/src/core/project_sync.rs`, `src-tauri/src/commands/mod.rs:900`, `src-tauri/src/core/sync_engine.rs:137`
- Risk: Orphaned symlinks/copies or unintended deletion behavior can go unnoticed.
- Priority: High

**Skill file browser authorization boundary:**

- What's not tested: Supplying arbitrary `centralPath` values to `list_skill_files` and `read_skill_file`, symlink edge cases, and non-UTF8/large-file UI behavior.
- Files: `src-tauri/src/commands/mod.rs:1100`, `src-tauri/src/commands/mod.rs:1120`, `src-tauri/src/core/skill_files.rs`
- Risk: File browsing scope assumptions can regress and expose unexpected local files inside the desktop app boundary.
- Priority: Medium

---

_Concerns audit: 2026-04-29_
