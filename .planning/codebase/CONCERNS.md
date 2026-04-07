# Codebase Concerns

**Analysis Date:** 2026-04-07

## Tech Debt

**Monolithic frontend state and workflow orchestration:**

- Issue: `src/App.tsx` contains most UI state, async orchestration, tool-sync logic, modal flow, update checks, settings persistence, and error handling in a single 2,086-line component.
- Files: `src/App.tsx`
- Impact: Changes to add/import/sync/update flows are high-risk because unrelated UI state and side effects are tightly coupled. Repeated install-and-sync code paths are easy to drift apart, and React hook dependency suppression increases the chance of stale-state bugs.
- Fix approach: Split `src/App.tsx` into feature hooks and workflow modules, such as install flow, sync flow, settings flow, and updates flow. Move repeated install+sync sequences into shared helpers and remove `eslint-disable-next-line react-hooks/exhaustive-deps` workarounds by giving effects stable dependencies.

**Duplicated install-and-sync logic across local, Git, picker, onboarding, and explore flows:**

- Issue: The same sequence of "install skill, compute selected targets, sync per tool, collect errors, refresh managed skills" is implemented multiple times instead of once.
- Files: `src/App.tsx`
- Impact: Bug fixes and new behaviors must be applied in many branches. Inconsistent edge-case handling is likely, especially around no-target selection, duplicate-name handling, and post-install cleanup.
- Fix approach: Extract a shared frontend action helper or backend command that accepts an install result and target list, then performs sync/error aggregation consistently for all entry points.

**Large command layer with mixed responsibilities:**

- Issue: `src-tauri/src/commands/mod.rs` is nearly 1,000 lines and mixes DTO definitions, path expansion, command handlers, cross-tool syncing, deletion behavior, GitHub token storage, file reading, and error text formatting.
- Files: `src-tauri/src/commands/mod.rs`
- Impact: Backend command changes are harder to reason about, and command-specific bugs are more likely to leak into unrelated areas. The module is also a bottleneck for adding new commands or changing DTOs.
- Fix approach: Split the command layer into focused modules such as `settings`, `skills`, `sync`, `files`, and `github`, while keeping DTOs close to their handlers.

**Installer complexity concentrated in one backend module:**

- Issue: `src-tauri/src/core/installer.rs` handles local install, Git install, GitHub API directory download fallback, multi-skill discovery, SKILL.md parsing, description backfill, update logic, caching, naming, and target refresh.
- Files: `src-tauri/src/core/installer.rs`
- Impact: Installer changes are fragile because Git, filesystem, parsing, and persistence concerns are intertwined. Regressions can affect both initial import and update flows.
- Fix approach: Split installer responsibilities into smaller modules for repo fetching, skill discovery, metadata parsing, central-repo import, and update propagation.

**Mixed-language user-facing error strings in backend:**

- Issue: Backend error formatting and command responses contain Chinese user-facing strings directly, while the frontend i18n system is otherwise centralized in `src/i18n/resources.ts`.
- Files: `src-tauri/src/commands/mod.rs`, `src-tauri/src/core/installer.rs`, `src-tauri/src/core/git_fetcher.rs`, `src-tauri/src/core/github_download.rs`
- Impact: Error UX is inconsistent, localization is incomplete, and frontend message parsing depends on raw backend text rather than stable error codes.
- Fix approach: Return structured error codes plus parameters from Rust, then localize entirely in `src/i18n/resources.ts` on the frontend.

## Known Bugs

**Cancel action does not reset all in-flight operations:**

- Symptoms: Pressing cancel only affects the `install_git` command path that explicitly resets and passes the shared cancel token. Other long-running workflows continue despite the loading overlay being dismissed.
- Files: `src/App.tsx`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/core/cancel_token.rs`, `src-tauri/src/core/github_download.rs`, `src-tauri/src/core/git_fetcher.rs`
- Trigger: Start a long-running operation from flows that do not wire the cancel token, such as `list_git_skills_cmd`, `install_git_selection`, search-related requests, or local copy-heavy operations, then press cancel via `cancel_current_operation`.
- Workaround: Wait for the backend task to finish, or avoid using cancel outside the main Git install path.

**Deleting a skill can leave the app in a partial-success state while reporting failure:**

- Symptoms: The central skill record and directory can be removed even when cleanup of one or more synced tool targets fails, after which the command returns an error message.
- Files: `src-tauri/src/commands/mod.rs`
- Trigger: Run `delete_managed_skill` when one synced target cannot be removed due to permissions, locks, or filesystem errors.
- Workaround: Manually remove leftover tool directories after the failed deletion message.

**Git skill selection path cannot be cancelled once started:**

- Symptoms: Multi-skill discovery and selected-skill Git installs continue running after the user dismisses loading state because those commands do not receive the shared cancel token.
- Files: `src/App.tsx`, `src-tauri/src/commands/mod.rs`
- Trigger: Start `list_git_skills_cmd` or `install_git_selection`, then invoke `cancel_current_operation` from the frontend.
- Workaround: Wait for the request to complete before interacting again.

**Search result “installed” detection can misclassify skills:**

- Symptoms: Explore/search cards determine installed state from a synthesized key of skill name plus normalized source path, which can miss renamed installs or collide for unrelated skills with the same name and repo root.
- Files: `src/components/skills/ExplorePage.tsx`, `src/App.tsx`
- Trigger: Install a skill with a custom display name, or import multiple skills from one repository where source normalization collapses to the same repo root.
- Workaround: Verify installation status from `My Skills` rather than relying on Explore badges.

## Security Considerations

**GitHub token is stored in plaintext application settings:**

- Risk: Personal access tokens are persisted as plain text in the SQLite settings store rather than OS-backed secure storage.
- Files: `src-tauri/src/commands/mod.rs`, `src-tauri/src/core/skill_store.rs`, `src/components/skills/SettingsPage.tsx`
- Current mitigation: The token input uses `type="password"` in `src/components/skills/SettingsPage.tsx`, and the app only reads/writes it through Tauri commands.
- Recommendations: Store tokens in OS credential storage via a Tauri keychain/secret plugin, and keep only a reference or non-sensitive metadata in SQLite.

**Local file viewer can expose any text file inside a managed skill directory:**

- Risk: `read_skill_file` blocks traversal outside the central skill directory, but it still displays arbitrary UTF-8 text files within that directory, including copied config or accidentally imported secrets.
- Files: `src-tauri/src/commands/mod.rs`, `src-tauri/src/core/skill_files.rs`, `src/components/skills/SkillDetailView.tsx`
- Current mitigation: Path canonicalization prevents traversal, symlinks are not followed by file listing, and files over 1 MB are rejected.
- Recommendations: Add filename/content allowlists or explicit warnings for sensitive names like `.env`, `credentials`, `*.pem`, and `*.key`, and consider hiding dotfiles by default in the viewer.

**Remote content trust is broad for featured and searchable skills:**

- Risk: The app surfaces externally hosted skill repositories and downloads remote content from GitHub and `skills.sh` without signature verification or publisher trust controls.
- Files: `src-tauri/src/core/featured_skills.rs`, `src-tauri/src/core/skills_search.rs`, `src-tauri/src/core/github_download.rs`, `src-tauri/src/core/git_fetcher.rs`, `src/components/skills/ExplorePage.tsx`
- Current mitigation: Downloads are limited to declared repo paths, and the app copies content into its own managed directory instead of executing it.
- Recommendations: Add source verification signals, repository trust indicators, and optional allowlists for approved owners.

## Performance Bottlenecks

**Copy-based sync path scales poorly for large skills and repeated updates:**

- Problem: When symlinks are unavailable or disabled for a tool such as Cursor, the app recursively copies full directories for sync and update operations.
- Files: `src-tauri/src/core/sync_engine.rs`, `src-tauri/src/commands/mod.rs`
- Cause: `copy_dir_recursive` walks and copies the entire tree every time, and `sync_dir_for_tool_with_overwrite` forces copy mode for Cursor.
- Improvement path: Add incremental syncing based on content hashes or per-file timestamps, and surface mode-specific UX so users know when a tool is on the slow copy path.

**Frontend re-renders and state churn are concentrated in one root component:**

- Problem: `src/App.tsx` owns nearly all state for the app, so many unrelated interactions can trigger wide rerender scope and expensive recomputation.
- Files: `src/App.tsx`
- Cause: Centralized state plus many `useMemo`, `useCallback`, and effect-driven async flows all live in one component.
- Improvement path: Move state into feature-level components or custom hooks and memoize only where profiling shows value.

**Online search issues can flood the backend with requests during typing:**

- Problem: Search triggers network-backed Tauri calls after a 500 ms timer without request cancellation or stale-response suppression beyond the latest state overwrite.
- Files: `src/App.tsx`, `src-tauri/src/core/skills_search.rs`
- Cause: Debouncing is implemented with `setTimeout`, but in-flight requests are not cancelled and every qualifying input can still hit `skills.sh`.
- Improvement path: Add abortable requests or sequence guards in the frontend and request-level cancellation support in the backend.

## Fragile Areas

**Tool adapters with shared directories are easy to break:**

- Files: `src-tauri/src/core/tool_adapters/mod.rs`, `src-tauri/src/commands/mod.rs`, `src/App.tsx`
- Why fragile: Some tools share the same skills directory, so sync and unsync behavior depends on grouping logic in both backend and frontend. A new adapter with a shared path must be reflected consistently in detection, UI selection, persistence, and sync semantics.
- Safe modification: When adding or changing adapters in `src-tauri/src/core/tool_adapters/mod.rs`, verify shared-directory behavior through `sync_skill_to_tool`, `unsync_skill_from_tool`, and frontend selection confirmation in `src/App.tsx`.
- Test coverage: Rust coverage exists for tool adapters in `src-tauri/src/core/tests/tool_adapters.rs`, but there are no frontend tests for shared-directory confirmation or selection UX.

**Name-based onboarding grouping can merge unrelated skills:**

- Files: `src-tauri/src/core/onboarding.rs`
- Why fragile: Onboarding groups detected skills by `name` first, then uses content hashes to flag conflicts. Different skills with the same declared name are treated as one logical group.
- Safe modification: Preserve the current grouping semantics only if duplicate-name handling is intentional; otherwise introduce stable source identifiers before changing import behavior.
- Test coverage: Rust tests exist in `src-tauri/src/core/tests/onboarding.rs`, but edge cases around same-name/different-origin skills are not obvious from the current suite.

**Error handling depends on string prefixes and message parsing:**

- Files: `src-tauri/src/commands/mod.rs`, `src/App.tsx`
- Why fragile: Frontend behavior branches on raw string prefixes like `MULTI_SKILLS|`, `TARGET_EXISTS|`, `TOOL_NOT_INSTALLED|`, and `TOOL_NOT_WRITABLE|`. Any wording change in Rust can silently break frontend flows.
- Safe modification: Add new stable error codes before changing existing text, and migrate the frontend to parse structured payloads instead of free-form messages.
- Test coverage: Backend command tests exist in `src-tauri/src/commands/tests/commands.rs`, but there are no frontend tests asserting parsing behavior.

**Skill detail rendering mixes markdown, frontmatter parsing, syntax highlighting, and file browsing in one component:**

- Files: `src/components/skills/SkillDetailView.tsx`
- Why fragile: The component owns file tree building, frontmatter parsing, text loading, markdown rendering, syntax highlighting, and async file state. Changes to one concern can affect the others.
- Safe modification: Extract file tree, content loading, markdown rendering, and metadata parsing into separate utilities/components before expanding viewer features.
- Test coverage: No dedicated frontend tests were detected for `src/components/skills/SkillDetailView.tsx`.

## Scaling Limits

**Tool adapter matrix grows linearly in maintenance cost:**

- Current capacity: `src-tauri/src/core/tool_adapters/mod.rs` defines 47 tool adapters, each with detection and skill-directory metadata.
- Limit: Every new tool increases the chance of path drift, duplicate directories, platform-specific edge cases, and translation/UI mismatches.
- Scaling path: Move adapter data into a declarative registry with validation tests for duplicate keys, duplicate detection paths, and duplicate skills directories.

**Single-file translation resource will become harder to maintain as features grow:**

- Current capacity: `src/i18n/resources.ts` is already 599 lines and holds both English and Chinese resources in one file.
- Limit: New UX flows increase merge conflicts and make missing-key review harder.
- Scaling path: Split translations by feature namespace and add automated key parity checks.

**SQLite settings store is taking on more operational state:**

- Current capacity: Settings already store central repo path, Git cache config, installed tools history, featured skills cache, and GitHub token.
- Limit: As more state is added, a single generic key/value table in `src-tauri/src/core/skill_store.rs` becomes less self-describing and harder to migrate safely.
- Scaling path: Introduce typed settings accessors or dedicated tables for security-sensitive and structured settings.

## Dependencies at Risk

**Git integration depends on external system `git` behavior:**

- Risk: The app prefers the system `git` binary and only falls back to libgit2 when `SKILLS_HUB_ALLOW_LIBGIT2_FALLBACK=1` is set.
- Impact: User environment differences in PATH, proxies, certs, or CLI behavior can directly affect install/update reliability.
- Migration plan: Make backend diagnostics more structured, add preflight checks for git availability, and consider a safer fallback policy rather than environment-variable-gated recovery.

**App behavior depends on third-party APIs with rate limits and schema expectations:**

- Risk: `skills.sh`, GitHub search, GitHub contents API, and a raw GitHub JSON URL are all external dependencies.
- Impact: Rate limits, downtime, or response shape changes can break Explore, search, featured skills, and GitHub subpath installs.
- Migration plan: Cache more aggressively, add resilient parsing and telemetry, and define graceful degraded states in the UI.

## Missing Critical Features

**No secure secret storage for GitHub token:**

- Problem: Token handling is functional but not hardened for desktop-app secret management.
- Blocks: Safe enterprise or long-lived personal token use.

**No frontend automated test suite detected:**

- Problem: No `.test.ts`, `.test.tsx`, `.spec.ts`, or `.spec.tsx` files were detected under `src/`.
- Blocks: Safe refactoring of `src/App.tsx`, `src/components/skills/ExplorePage.tsx`, `src/components/skills/SkillDetailView.tsx`, and settings/onboarding flows.

**No structured error contract between frontend and backend:**

- Problem: Errors are mostly plain strings with a few reserved prefixes.
- Blocks: Reliable localization, durable error UX, and safer backend refactors.

## Test Coverage Gaps

**Frontend app workflows are untested:**

- What's not tested: Add/import/sync/update/delete flows, update banner behavior, localStorage-backed theme/language persistence, and Explore search/install behavior.
- Files: `src/App.tsx`, `src/components/skills/ExplorePage.tsx`, `src/components/skills/SettingsPage.tsx`
- Risk: Regressions in core user workflows can ship unnoticed because the most stateful code path has no automated UI coverage.
- Priority: High

**Skill detail viewer has no automated coverage:**

- What's not tested: File listing, frontmatter table parsing, markdown rendering, syntax highlighting selection, large-file error display, and file-tree interactions.
- Files: `src/components/skills/SkillDetailView.tsx`, `src-tauri/src/core/skill_files.rs`
- Risk: Viewer regressions or file-reading edge cases can break inspection of managed skills without detection.
- Priority: Medium

**Cross-layer error mapping is not verified end-to-end:**

- What's not tested: Frontend handling for backend prefixes like `MULTI_SKILLS|`, `TARGET_EXISTS|`, `TOOL_NOT_INSTALLED|`, and `TOOL_NOT_WRITABLE|`.
- Files: `src/App.tsx`, `src-tauri/src/commands/mod.rs`
- Risk: Small wording or formatting changes in Rust can silently break frontend error flows.
- Priority: High

**Deletion and partial-cleanup behavior lacks targeted regression tests:**

- What's not tested: Partial target-removal failure cases during `delete_managed_skill` and the resulting user-visible state.
- Files: `src-tauri/src/commands/mod.rs`
- Risk: Users can lose the managed record while orphaned tool directories remain.
- Priority: High

---

_Concerns audit: 2026-04-07_
