# Codebase Concerns

**Analysis Date:** 2026-04-16

## Tech Debt

**Frontend orchestration concentrated in a single component:**

- Issue: `src/App.tsx` is a 2200+ line stateful orchestrator that owns onboarding, install flows, sync flows, update UX, settings, search, modal state, per-tool sync toggles, and top-level navigation. Future feature work must thread new state and callbacks through this file, which raises merge conflict frequency and makes regression scope hard to reason about.
- Files: `src/App.tsx`, `src/components/skills/SkillsList.tsx`, `src/components/skills/modals/*.tsx`, `src/components/projects/ProjectsPage.tsx`
- Impact: Small UI changes can accidentally affect unrelated flows such as onboarding import, manual add, project sync, and updater behavior. Review and testing cost stay high because most app workflows converge in one component.
- Fix approach: Split `src/App.tsx` by workflow boundaries. Keep shell/navigation in `src/App.tsx`, move install/import flows into a dedicated hook or controller module, move settings/updater state into their own hooks, and keep view-specific state inside feature subtrees such as `src/components/projects/useProjectState.ts`.

**Command layer is oversized and mixes transport concerns with product logic:**

- Issue: `src-tauri/src/commands/mod.rs` is over 1100 lines and contains DTO definitions, command registration-facing wrappers, error rewriting, sync orchestration, install orchestration, settings persistence, and file browsing. This weakens the intended separation between Tauri boundary code and core logic.
- Files: `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/projects.rs`
- Impact: Adding or changing one command increases the chance of collateral edits in unrelated code paths. Error contracts are harder to audit because they are spread across long mixed-purpose modules.
- Fix approach: Keep `src-tauri/src/commands/mod.rs` as a thin index and split commands by domain, mirroring core modules: install, sync, settings, skills, onboarding, and projects. Keep shared DTOs and error helpers in small focused support modules.

**Error and UI strings are partially hard-coded in Rust and bypass frontend localization:**

- Issue: Several backend error messages and user-facing strings are hard-coded in Chinese or raw English inside Rust command/helpers instead of being emitted as stable error codes and localized in the frontend.
- Files: `src-tauri/src/commands/mod.rs`, `src-tauri/src/core/installer.rs`, `src-tauri/src/core/sync_engine.rs`
- Impact: English UI can surface untranslated backend text, and future i18n changes require Rust edits instead of resource updates. Error parsing also becomes brittle because the frontend must match literal text in some cases.
- Fix approach: Emit structured error prefixes only, such as `RATE_LIMITED|`, `MULTI_SKILLS|`, `SKILL_INVALID|`, `PROJECT_PATH_MISSING|`, and keep user-facing text in `src/i18n/resources.ts` plus frontend formatters.

**Dormant alternate frontend shell remains in the repo:**

- Issue: Router-style components exist but are not wired into the active app. They use a different UI direction from the current `src/App.tsx` shell and contain placeholder content.
- Files: `src/components/Layout.tsx`, `src/pages/Dashboard.tsx`, `src/main.tsx`
- Impact: New contributors can misread these files as active entry points and place code in dead paths. Styling and architectural drift continues because unused code is not maintained by real user flows.
- Fix approach: Either remove the dormant shell or explicitly wire it into the app. If it remains intentionally dormant, isolate it behind a feature branch or documented experiment directory rather than `src/components/` and `src/pages/`.

## Known Bugs

**Project path updates do not move existing synced project skill directories:**

- Symptoms: Updating a project's registered path only changes the database record. Existing assignments continue to point at artifacts under the old project directory until a manual resync runs, and stale skill directories may remain in the old location.
- Files: `src-tauri/src/core/project_ops.rs`, `src-tauri/src/commands/projects.rs`, `src/components/projects/ProjectsPage.tsx`
- Trigger: Change a project path through `update_project_path`, then inspect old and new project directories before running `resync_project` or re-toggling assignments.
- Workaround: Run the project resync action immediately after path changes and manually clean the old project skill directories if they remain.

**Bulk assign can report success while some tool assignments are stored with error state:**

- Symptoms: `bulk_assign_skill` returns successfully even when individual tool syncs fail, because per-tool failures are accumulated into `failed` instead of failing the command. The UI warns, but the operation still looks broadly successful and leaves mixed assignment states.
- Files: `src-tauri/src/commands/projects.rs`, `src/components/projects/useProjectState.ts`, `src/components/projects/ProjectsPage.tsx`
- Trigger: Bulk assign a skill to a project where one configured tool path is missing or unwritable.
- Workaround: Inspect the warning details and per-cell assignment states after bulk assign, then resync or remove failing tools manually.

**Assignment grid header shows raw tool keys instead of display labels:**

- Symptoms: The project assignment matrix renders `tool.tool` values like `claude_code` and `cursor` directly in the header instead of localized display labels.
- Files: `src/components/projects/AssignmentMatrix.tsx`
- Trigger: Open the Projects page with any configured tools.
- Workaround: None in-app; users must infer tool names from internal keys.

## Security Considerations

**GitHub token is stored as plain application setting instead of OS-backed secret storage:**

- Risk: The personal access token retrieved by `get_github_token` / `set_github_token` is stored through the generic SQLite settings table. Local compromise of the app database exposes the token directly.
- Files: `src-tauri/src/commands/mod.rs`, `src-tauri/src/core/skill_store.rs`
- Current mitigation: The token is optional, trimmed before persistence, and not printed in app UI.
- Recommendations: Move token storage to platform credential storage or a Tauri secure-storage plugin. Keep only a boolean presence indicator in SQLite.

**Skill rendering trusts repository Markdown and code content inside the desktop app:**

- Risk: Untrusted skill repositories can place very large Markdown/code trees or hostile content into the central repo, which is then rendered in the desktop UI. `read_skill_file` blocks traversal and size over 1 MB, but there is still exposure to untrusted rendered content and potential UI degradation.
- Files: `src-tauri/src/core/skill_files.rs`, `src/components/skills/SkillDetailView.tsx`, `src/App.tsx`
- Current mitigation: File reads are restricted to files under the skill directory and rejected when larger than 1 MB or non-UTF-8.
- Recommendations: Add explicit file type allowlists for preview, cap total listed files, and document that imported repositories are untrusted content.

**Project gitignore management writes into repositories without validating repo ownership or cleanliness:**

- Risk: `update_project_gitignore` modifies `.gitignore` and `.git/info/exclude` inside any registered directory if it exists. A mistakenly registered path can silently alter unrelated repositories.
- Files: `src-tauri/src/commands/projects.rs`, `src/components/projects/ProjectsPage.tsx`
- Current mitigation: The path must exist as a directory, and written patterns are derived from known tool adapters.
- Recommendations: Surface a stronger confirmation that includes the exact target files, require an explicit repo detection step before editing `.git/info/exclude`, and log/preview the diff before writing.

## Performance Bottlenecks

**Repeated full-directory hashing for copy-mode sync staleness:**

- Problem: Copy-mode project assignments recompute content hashes of full source trees to determine staleness and to update post-sync metadata.
- Files: `src-tauri/src/core/project_sync.rs`, `src-tauri/src/core/content_hash.rs`, `src-tauri/src/core/sync_engine.rs`
- Cause: `list_assignments_with_staleness` and copy-mode sync flows call `content_hash::hash_dir` across whole skill directories instead of using incremental metadata or cached digests.
- Improvement path: Cache source hashes at the skill record level, recompute only when the central repo changes, and avoid hash recalculation during read-only assignment listing.

**Project matrix does O(skills × tools × assignments) lookups on every render:**

- Problem: Each assignment cell runs `assignments.find(...)` inside nested loops, and memo equality still treats the full assignments array as an input. Large skill inventories and multi-tool projects will degrade UI responsiveness.
- Files: `src/components/projects/AssignmentMatrix.tsx`
- Cause: Assignment lookup uses repeated linear scans instead of a precomputed map keyed by `skill_id:tool`.
- Improvement path: Precompute an assignment lookup map with `useMemo`, pass it to rows, and keep row props stable by key rather than passing the whole assignments array.

**Project add/remove tool actions perform sequential IPC calls:**

- Problem: Adding or removing many tools issues one backend invocation per tool and then refetches full state.
- Files: `src/components/projects/useProjectState.ts`, `src-tauri/src/commands/projects.rs`
- Cause: `addTools` and `removeTools` loop serially over `invoke` calls instead of using a batch backend command.
- Improvement path: Add batch add/remove commands in `src-tauri/src/commands/projects.rs` and update the frontend to submit tool lists in one round trip.

**App startup loads several independent settings and network checks serially from one component tree:**

- Problem: `src/App.tsx` triggers multiple initial effects for storage path, cache settings, GitHub token, auto-sync state, onboarding plan, tool status, and updater checks. Each failure path also mutates shared error state.
- Files: `src/App.tsx`
- Cause: Initialization is split into many loosely coordinated `useEffect` calls instead of a structured startup routine.
- Improvement path: Consolidate startup reads into dedicated hooks or a single boot coordinator, parallelize truly independent requests, and isolate non-fatal failures from shared app error state.

## Fragile Areas

**Project sync state machine mixes persisted state with derived filesystem truth:**

- Files: `src-tauri/src/core/project_sync.rs`, `src-tauri/src/core/project_ops.rs`, `src-tauri/src/core/skill_store.rs`, `src-tauri/src/commands/projects.rs`
- Why fragile: Assignment records can move between `pending`, `synced`, `stale`, `missing`, and `error` based on both database values and live filesystem checks. Several cleanup flows treat `synced`, `stale`, `missing`, and `error` differently. Mixed partial failure cases are easy to create when files are removed externally.
- Safe modification: Change status transitions together with integration tests in `src-tauri/src/core/tests/project_sync.rs` and `src-tauri/src/core/tests/project_ops.rs`. Avoid changing only one cleanup or resync path.
- Test coverage: Backend coverage exists for core project sync cases, but there is no frontend test coverage for Projects page rendering or recovery flows.

**Delete managed skill path cleanup spans global sync targets and project targets:**

- Files: `src-tauri/src/commands/mod.rs`, `src-tauri/src/core/project_ops.rs`, `src-tauri/src/core/project_sync.rs`
- Why fragile: Deletion must remove global tool targets, project assignment artifacts, and the central repo copy while preserving consistent DB state. Partial filesystem failures currently produce a mixed outcome: records may already be deleted when cleanup errors are raised.
- Safe modification: Keep delete order explicit, document transactional intent, and add tests for partial cleanup failure scenarios before changing this flow.
- Test coverage: Command-layer tests in `src-tauri/src/commands/tests/commands.rs` cover error formatting but not full deletion failure recovery.

**Installer flow contains many branch-specific GitHub and subpath behaviors:**

- Files: `src-tauri/src/core/installer.rs`, `src-tauri/src/core/tests/installer.rs`, `src/App.tsx`
- Why fragile: The installer handles shorthand GitHub URLs, `/tree/` and `/blob/` parsing, sparse checkout, API fallback, multi-skill repo detection, SKILL.md-derived renaming, and auto-sync after install. A change in one branch can affect naming, dedupe, or update behavior elsewhere.
- Safe modification: Preserve branch-specific tests, add new tests before changing parse or fallback behavior, and avoid duplicating install logic in the frontend.
- Test coverage: Rust tests exist for URL parsing and install/update behavior, but frontend flows that auto-select candidates or open picker modals are untested.

## Scaling Limits

**Desktop UI and IPC are optimized for small-to-medium skill libraries:**

- Current capacity: The app loads full managed skill lists into memory and renders large arrays directly from `src/App.tsx` and `src/components/projects/AssignmentMatrix.tsx`.
- Limit: Hundreds of skills multiplied by dozens of tools will increase render cost, assignment lookup cost, and startup state hydration time.
- Scaling path: Introduce indexed lookup maps, list virtualization for large tables and skill lists, and narrower IPC payloads for view-specific data.

**SQLite access pattern opens a new connection per store operation:**

- Current capacity: `SkillStore::with_conn` opens a fresh `rusqlite::Connection` for each call, which is acceptable for desktop-local workloads.
- Limit: Bursty workflows such as project bulk assign, repeated resyncs, or future background tasks will amplify connection churn and serialized IO.
- Files: `src-tauri/src/core/skill_store.rs`
- Scaling path: Introduce a managed connection pool or a long-lived connection with well-defined access serialization.

## Dependencies at Risk

**No frontend test runner is configured despite growing React feature surface:**

- Risk: The React app now includes install flows, updater UX, settings, Projects page state, and localization, but `package.json` contains no Vitest/Jest/RTL setup.
- Impact: UI regressions ship unless caught manually or by type/lint checks. Project sync UX and modal interactions are especially exposed.
- Migration plan: Add a lightweight React Testing Library + Vitest setup for unit/integration coverage of `src/App.tsx`-level workflows and `src/components/projects/*.tsx` behavior.

**Tauri/plugin behavior is a critical external dependency for desktop-only flows:**

- Risk: Core features depend on `@tauri-apps/plugin-updater`, `@tauri-apps/plugin-dialog`, and Tauri IPC contracts. There is no browser fallback for most app behavior.
- Impact: Plugin API changes or packaging regressions can break installation, update, or settings flows across all platforms.
- Files: `package.json`, `src/App.tsx`, `src-tauri/src/lib.rs`
- Migration plan: Keep plugin usage isolated behind helpers, and add command/UI integration smoke tests around critical flows.

## Missing Critical Features

**No automated recovery flow after project path changes:**

- Problem: The app exposes project path editing but does not automatically migrate or resync project skill directories to the new location.
- Blocks: Safe project relocation and confidence that registered projects remain correct after directory moves.

**No dedicated UI for inspecting per-assignment error details and remediation:**

- Problem: Assignment cells surface error state through checkbox styling and tooltip text only.
- Blocks: Fast diagnosis of missing directories, permission problems, or stale project sync state for larger projects.

## Test Coverage Gaps

**Frontend installation, updater, and project workflows are untested:**

- What's not tested: Manual add flows, onboarding import, explore-page install, project registration/edit/remove, assignment toggling, bulk assign UX, update modal behavior, and error/toast mapping.
- Files: `src/App.tsx`, `src/components/projects/ProjectsPage.tsx`, `src/components/projects/AssignmentMatrix.tsx`, `src/components/projects/useProjectState.ts`, `src/components/skills/modals/*.tsx`
- Risk: Regressions in state sequencing, stale selection handling, or UI recovery paths can pass lint/build and only fail during manual use.
- Priority: High

**Command-level deletion and project cleanup scenarios are only partially covered:**

- What's not tested: End-to-end command behavior for deleting managed skills with partial filesystem failures, changing project paths with existing assignments, and gitignore file mutation edge cases.
- Files: `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/projects.rs`, `src-tauri/src/core/project_ops.rs`
- Risk: Cleanup regressions can leave orphaned files or inconsistent DB state without immediate detection.
- Priority: High

**Cross-platform path and permission edge cases remain mostly manual:**

- What's not tested: WSL2/macOS/Linux differences for symlink permissions, writable tool directories, project-local target cleanup, and updater-related filesystem behavior.
- Files: `src-tauri/src/core/sync_engine.rs`, `src-tauri/src/core/project_sync.rs`, `src-tauri/src/commands/mod.rs`, `README.md`
- Risk: The app claims cross-platform support, but platform-specific failures can persist until user reports surface them.
- Priority: Medium

---

_Concerns audit: 2026-04-16_
