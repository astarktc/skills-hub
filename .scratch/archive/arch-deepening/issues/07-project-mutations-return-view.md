# 07: Project mutations return the affected Project's view

Status: resolved

Type: task
Source: `../spec.md` Q19, Q20. Touches two merge-risk files — sequence alone.

**What to build:** Every project mutation (register, remove, assign, unassign, bulk assign, resync one, resync all, update path, configure Tools) returns the fresh view of what it changed — the project row with its counts and aggregate status, its configured Tools, and its reconciled assignments — so the project world hook applies one result instead of orchestrating follow-up reads. The per-project counts come from one aggregate query instead of four. The project+skill lookup that commands re-derive is one public core function; unassign takes ids like assign does. The hook's twelve raw modal members become one `dialog` value with open/close, and the assignment-existence mirror disappears because the backend decides add-vs-remove from its own state.

**Blocked by:** 01, 05

- [x] The project world hook contains no post-mutation refetch tail; its tests assert applied state, not invoke counts
- [x] A mutation whose cascade changes other rows (configure Tools removing assignments) returns a view that already reflects the cascade, with a core test
- [x] Listing N projects issues a bounded number of queries independent of N×4
- [x] The not-found lookup idiom exists once in core; no command body performs it
- [x] `ProjectsPage` consumes one `dialog` value; no behaviour change in any modal
- [x] Bindings regenerated; `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.

Shipped in `06acd11` on `arch/07-project-view`.

**Wire shape.** `ProjectViewDto { project, tools, assignments, reconciled }` (the
ticket 01 listing flag is nested into the view rather than kept as its own DTO).
Mutations return: `register_project`/`update_project_path`/`configure_project_tools`
→ view; `remove_project` → `Vec<ProjectDto>` (the remaining list — the project it
named is gone); `toggle_project_skill_assignment` → `{ view, assigned }`;
`bulk_assign_skill` → `{ view, failed }`; `resync_project` → `{ view, summary }`;
`resync_all_projects` → `{ summaries, projects }`. New read command
`get_project_view` replaced `list_project_tools` + `list_project_skill_assignments`
(both were only ever called by the hook); `ProjectAssignmentListingDto` is gone.
`add_project_skill_assignment` / `remove_project_skill_assignment` deleted.

**Core.** `project_ops::project_view` runs outside the guard (composition happens
in the command body — one style, no per-command wrapper). `require_project` is
the single not-found idiom; `project_sync::lookup_project_and_skill` is public and
delegates to it. `project_sync::toggle_skill_assignment` is the new mutation entry
point (guard test added); its two branches use the unlocked seams.

**Query cost.** `SkillStore::project_aggregates` (+ `project_aggregate`) reads
tool counts and assignment rows in two grouped queries and folds
`sync_status::aggregate` in Rust; the four `count_project_*` /
`aggregate_project_sync_status` methods are gone. `list_project_dtos` is now
1 project query + 2 aggregate queries regardless of N.

**Deviations.**
1. Touched `src-tauri/src/commands/mod.rs` (3 call sites, `&home` argument
   deleted) — unavoidable: the ticket-05 cleanup moved `home` into
   `RemovalScope::SkillTool`, which drops the parameter from `remove_skill`,
   `unsync_skill_targets` and `unsync_all_skill_targets`. Ticket 06 shares that
   file; the hunks are three single-line deletions.
2. `BulkAssignResultDto.assigned` dropped — the view carries every assignment
   that now exists and the frontend only ever read `failed`.
3. The hook keeps a *failure*-path `refreshView` for toggle/bulk/configure. That
   is convergence after a rejected mutation (an unassign whose artifact stayed
   settles its row as `error`), not a success-path refetch tail; success paths
   issue exactly one command, asserted in the tests.
4. `project_view`'s `reconciled` flag is not asserted in the core test — it
   reports whether the try-lock succeeded, so a concurrent test mutation can
   legitimately make it false. Its contract stays covered by the `project_sync`
   listing tests.
5. No i18n keys needed (no new user-facing strings).

**Follow-ups for ticket 12 (docs drift).**
- `AGENTS.md` "Ambiguity resolution" says "Refresh data by re-invoking the
  relevant command … after a mutation" — no longer true for the project world;
  its mutations return a view and the hook applies it. Worth a sentence next to
  the "Global sync fan-out is backend-owned" bullet.
- `CONTEXT.md`: candidate term **Project view** (the project row + its Tools +
  its reconciled assignments, returned by every project mutation) next to
  *Project sync status*. Also note that Artifact removal scopes carry their own
  roots now.
- README/CHANGELOG mention nothing per-command here, but any doc naming
  `add_project_skill_assignment` / `remove_project_skill_assignment` /
  `list_project_skill_assignments` / `list_project_tools` is stale.
