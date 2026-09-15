# 28: Let the backend own the gitignore ordering

Status: resolved

Type: task
Blocked by: 25

## What to build

Review #2 (Opus + Sol, verified). `update_project_gitignore` derives its patterns from the project's *already-persisted* tools (`commands/projects.rs:439-444` at `943f85c`; after ticket 25, the core function it wraps), so it must run after tools are written. That ordering constraint is part of the interface, and nothing in the interface says so — it is enforced in the least testable place available: a `useRef` in a page component. `src/components/projects/ProjectsPage.tsx:21-29` declares `pendingGitignoreRef` (its comment spells out the constraint), `handleAddProject` (`:31-59`) stashes the user's intent, `handleToolConfigConfirm` (`:61-96`) replays it after tool config is confirmed — through a **raw `invoke`** (`:4`, calls at `:80`, `:133`) that bypasses the `invokeTauri` seam the hook tests mock; `EditProjectModal.tsx:5` imports raw `invoke` too. Per AGENTS.md components get no JSX tests, so these interactions have no test path at all.

Deepen:

- The backend owns the ordering: either the gitignore command accepts the intended tool set (deriving patterns from the request), or the tool-config command takes an optional gitignore intent and core sequences both writes (persist tools → derive → write). Pick the one that leaves the narrower interface; record why.
- `useProjectState` exposes the intent-level action; `ProjectsPage` and `EditProjectModal` call the hook, not `invoke`. Delete `pendingGitignoreRef`, the replay, and both raw `invoke` imports.
- Hook test for the add-project + configure-tools + gitignore sequence through the mocked `invokeTauri` seam; core test for the sequenced write.
- i18n unaffected unless copy changes; bindings regenerated if a DTO changes.

## Acceptance criteria

- [ ] Zero `@tauri-apps/api/core` `invoke` imports under `src/components/projects/`; the projects world reaches the backend only through `useProjectState` → `invokeTauri`.
- [ ] The ordering rule is implemented in core with a test; no frontend ref/replay.
- [ ] Add-project and edit-project flows behave as before (gitignore written after tools are persisted).
- [ ] `npm run version:check && npm run check` green.

## Answer

Landed green in `c969913` (Fable 5.1 child, medium thinking; clean rebase over ticket 26; 336 cargo + 74 vitest, full gate green on main).

**Option B chosen — the tool-config command carries the gitignore intent**: `configure_project_tools(projectId, tools: string[], gitignore: IgnoreUpdateOptions | null)` → `project_ops::configure_project_tools(store, project_id, tools, ignore)` does persist → derive → write in one function (typed `NotFound`; rejects unknown tools before any write; diffs against persisted tools so kept tools keep their record ids; removals go through `remove_tool_with_cleanup`; then `gitignore::update_for_project` if intent given). Why narrower than option A (gitignore command taking a tool list): A still leaves two calls whose tool sets must agree and lets persisted tools and the ignore block drift — the rule just moves from a `useRef` to a calling convention. B gives the frontend exactly one way to say "these are the tools; here's the ignore intent". The per-tool `add_project_tool` / `remove_project_tool` commands were deleted (dead code carrying the same footgun); `update_project_gitignore` stays for the edit flow (tools unchanged). `IgnoreUpdateOptions` now derives `Deserialize + TS` (binding committed).

**Frontend**: `useProjectState.registerProject(path, gitignore)` stores the intent in hook state; `configureTools(tools)` consumes it once (dropped if another project is registered/configured in between), re-fetches tools on failure so state converges, refreshes assignments + project list; new `getGitignoreStatus` / `updateGitignore` for the edit modal. `pendingGitignoreRef`, the replay, `addTools`/`removeTools`, and both raw `invoke` imports deleted — zero `@tauri-apps/api/core` under `src/components/projects/`. New `useProjectState.test.ts` (7 tests through the mocked `invokeTauri` seam: sequence, single consumption, no-intent, cross-project drop, error convergence, refresh, edit-flow commands); 5 core tests (incl. the ordering proof: gitignore written from the tools *just* persisted on a fresh project).

**Behaviour delta** (deliberate): an ignore-write failure now surfaces as the command error (`toast.error`, modal stays open) instead of a separate `toast.warning`; tools are still persisted and the hook re-syncs.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
