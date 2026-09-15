# 01: Make Sync-target mutation serialisation structural

Status: resolved

Type: task
Source: `../spec.md` Q5, Q6, Q9, Q10, Q17 (implemented by the orchestrating agent by hand — it sets the pattern the later tickets copy)

**What to build:** Every operation that materialises or removes a Sync target — global sync batch, assign/unassign, resync, configure project tools, remove project, gitignore update, managed-skill update propagation, managed-skill delete — runs one at a time, by construction. The rule lives in one small core module with a private mutex and a single `serialized(|| …)` helper that operation entry points wrap themselves in; commands carry no lock state and make no per-command decision. Per-target internals become unlocked crate-private internal seams (the git-cache module is the pattern). The reconcile pass run by the project listing try-locks: when a mutation is in flight it skips reconciliation and the listing result says so.

**Blocked by:** None (can start immediately)

- [x] No `#[tauri::command]` takes a mutex state parameter; the app-level mutex type is gone
- [x] Deleting a managed skill, updating a project's gitignore, and the managed-skill update path can no longer interleave with a project resync (covered by tests that drive the real core entry points from two threads — the existing serialisation test no longer constructs its own mutex)
- [x] `remove_project_skill_assignment`'s lookups happen inside the critical section
- [x] Listing assignments while a mutation is in flight returns promptly and reports that reconciliation was skipped; the UI does not treat a skipped reconcile as healthy
- [x] The guard module has no public surface beyond the helper; a grep for the mutex outside that module finds nothing
- [x] Bindings regenerate with no wire-shape change except the reconciliation flag; `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.

### Shipped (branch `arch/01-mutation-guard`)

**New module** `src-tauri/src/core/mutation_guard.rs` — private `static GUARD: Mutex<()>`,
`pub(crate) serialized(|| …)` (poison-recovering) and `pub(crate) try_serialized(|| …)`
(`None` on `WouldBlock`, recovers poison). Nothing else is exported. The module header states
the rule and the discipline it imposes: **an entry point never calls another entry point**;
composites call the unlocked `pub(crate)` seam.

**Entry points now wrap themselves** (their previous bodies became `*_unlocked` `pub(crate)`
seams unless noted):

| module | locked entry point | unlocked seam it composes |
| --- | --- | --- |
| `global_sync` | `sync_skills_to_tools` | `sync_skills_to_tools_unlocked` → `sync_skills_to_planned_tools` → `sync_skill_into_root` |
| `global_sync` | `unsync_skill_from_tool_with_records` | `…_unlocked` → `remove_targets_for_tools` |
| `global_sync` | `unsync_skill_targets` (**new**) | — (moved verbatim from the `unsync_skill` command) |
| `global_sync` | `unsync_all_skill_targets` (**new**) | — (moved verbatim from the `unsync_all_skills` command) |
| `project_sync` | `assign_skill_to_project_tool(s)` | `…_unlocked` → `assign_skill_to_tools` → `assign_and_sync` |
| `project_sync` | `resync_project` / `resync_all_projects` | `resync_project_unlocked` (the all-variant loops the unlocked one) |
| `project_sync` | `unassign_skill_from_project_tool` (**new**) | `unassign_and_cleanup` |
| `project_ops` | `configure_project_tools` | `…_unlocked` → `remove_tool_with_cleanup` + `gitignore::update_for_project_unlocked` |
| `project_ops` | `remove_project_with_cleanup` | `…_unlocked` |
| `gitignore` | `update_for_project` | `update_for_project_unlocked` |
| `skill_removal` | `remove_skill` | `plan_skill_removal` + `execute_skill_removal` |
| `installer` | `update_managed_skill_from_source` | `finalize_and_propagate_unlocked` |

`installer::update_managed_skill_from_source` keeps **acquisition outside the guard** (git clone /
local copy into the Staging dir); only finalize + Propagation run inside it. That is the
"acquire, then finalize+propagate under the guard" split ticket 02 builds on. Propagation
semantics are unchanged.

**Guard removed from the wiring tier**: `SyncMutex` (struct + `app.manage`) is gone from
`lib.rs`; all 7 `State<'_, SyncMutex>` parameters and `_lock` lines are gone from
`commands/projects.rs`. `remove_project_skill_assignment` no longer does its own lookups —
it calls the new `project_sync::unassign_skill_from_project_tool`, which does them **inside**
the critical section. `unsync_skill` / `unsync_all_skills` in `commands/mod.rs` are now one
core call each.

**Reconcile try-lock**: `project_sync::list_assignments_with_staleness` returns
`AssignmentListing { assignments, reconciled }`. The reconcile pass (`reconcile_listing`,
now reconciling in place) runs through `try_serialized`; a busy guard means the stored rows
come back untouched with `reconciled: false`. Wire: new `ProjectAssignmentListingDto`
(`commands/projects.rs`) — the **only** bindings shape change. Frontend: `useProjectState`
funnels every listing result through one `applyAssignmentListing` so `assignments` and the
new `assignmentsReconciled` state can't drift; `AssignmentMatrix` renders a notice
(`projects.reconcileSkipped`, EN + ZH) when it is false.

**Tests** (`src-tauri/src/core/tests/mutation_guard.rs`, 10 tests): guard unit tests plus
six entry-point tests using `assert_serialized(label, op)` — hold the guard on the main
thread, spawn `op`, sleep 300 ms, assert it has NOT completed, release, join, assert it did.
No test-owned mutex anywhere. Covered: `resync_project`, `skill_removal::remove_skill`,
`gitignore::update_for_project`, `project_ops::configure_project_tools`,
`global_sync::sync_skills_to_tools`, `installer::update_managed_skill_from_source`. Plus:
a listing taken during a mutation returns in < 300 ms with `reconciled: false` and leaves
a drifted copy row's stored status at `synced`; an unblocked listing reconciles it to
`stale`. The old `sync_serialization` test (which fabricated its own `Arc<Mutex<()>>`) is
deleted. `useProjectState.test.ts` gained two tests for the flag.

### Deviations from the recommended shape

1. **The entry-point serialisation tests live in `core/tests/mutation_guard.rs`, not spread
   across each module's test file.** Reason: the repo's test convention is one
   `#[path = "tests/<module>.rs"] mod tests;` per module, so a cross-module test file needs a
   host module; the guard module is the honest home for tests of the guard's rule, and keeping
   them together makes the rule auditable in one place.
2. **`reconcile_listing` reconciles in place (`&mut Vec<…>`) instead of consuming/returning the
   rows.** Reason: the consume/return shape forced an `Option::take().expect(…)` dance to get
   the rows back on the skipped path (two panic-shaped call sites flagged by pi-lens). In-place
   removes the fallback entirely.
3. **New test helper `list_reconciled` in `core/tests/project_sync.rs`.** The guard is
   process-global and `cargo test` runs in parallel, so pre-existing tests asserting on
   *reconciled* status became genuinely flaky (an unlucky call legitimately returns
   `reconciled: false`). Those call sites now retry through the helper; the skip path has its
   own dedicated test. Verified with 3 consecutive full `cargo test --all` runs, all green.

### Follow-ups for later tickets

- **Ticket 03** owns `global_sync::unsync_skill_targets` / `unsync_all_skill_targets`. They were
  moved **verbatim** per Q13 and still swallow filesystem errors and drop rows regardless —
  which contradicts the Artifact-removal rule in `CONTEXT.md` ("a row whose artifact could not
  be removed is kept with Sync status `error`"). Both carry a doc comment saying so.
- **Ticket 02** owns Propagation policy; `finalize_and_propagate_unlocked` is the seam it should
  restructure (global target loop + project assignment loop, currently inline in one function).
- The global sync batch holds the guard for its whole duration, so a listing during a large
  batch will consistently report `reconciled: false`. That is the intended Q6 behaviour, but it
  is the most likely place an operator meets the new notice.
- `remove_project_with_cleanup_unlocked` and `assign_skill_to_project_tools_unlocked` currently
  have no non-test callers other than their locked twins; they exist for symmetry and for tests
  that need unlocked access. If a later ticket finds them unused, they can collapse back.
