# Architecture deepening — round 2 (post v1.2.1)

Source: 3-model architecture review panel (Fable 5.1 / Opus 5 / GPT-5.6 Sol, medium effort) run via
the `improve-codebase-architecture` skill at HEAD `af71598`, synthesized and line-verified by the
orchestrating agent, then grilled to a settled design tree. Panel findings and the HTML report live
outside the repo (`$TMPDIR/architecture-review-{fable,opus,sol}.md`,
`$TMPDIR/architecture-review-20260807-panel.html`); everything decision-bearing is captured here.

Vocabulary for this round is in `CONTEXT.md`: **Propagation**, **Refresh (all)**, **Artifact
removal**, **Onboarding import** (added during grilling). Use those terms in code, tests and
commit messages.

## Why

v-next tickets 01–39 deepened the big modules (sync fan-out, tool catalog, finalize, discovery,
sync status, error contract). What remains is the *residue between* them: the same domain step
implemented two or three ways, policy living in the wiring tier or in React hooks, and one live
serialisation gap. Every ticket in this round passes the deletion test ("delete it and the
complexity reappears in N callers") — none is a horizontal move.

## Settled decisions (from grilling — do not re-litigate; reopen via a comment on the ticket)

| # | Decision |
|---|---|
| Q1 | Order: 01 → 02 → 03 → 05 → 06 → 07 … on `main`, one PR each; 04 and 08 in parallel worktrees. |
| Q2 | Propagation failure policy is **continue-and-report**: every Sync target resolves to synced/skipped/failed as report data; acquire/finalize may still fail the command. |
| Q3 | Refresh (all) carries a `reassert_auto_sync` policy; default on when auto-sync is on, so targets that never existed are created — today's behaviour, now named. |
| Q4 | Artifact removal **keeps the row with Sync status `error`** when the artifact could not be removed, for both global target rows and project assignment rows; rows are deleted only on success. → ADR-0002 (written in ticket 03). |
| Q5 | One process-wide mutation guard in core covering **all** Sync-target mutations, global and project. |
| Q6 | The reconcile pass **try-locks**; when a mutation is in flight it skips and the listing reports `reconciled: false`. |
| Q7 | Onboarding import with auto-sync off removes **only byte-identical** originals; divergent siblings stay and are reported. |
| Q8 | Terms as in `CONTEXT.md`. |
| Q9 | Lock only at **operation entry points**; per-target internals are unlocked `pub(crate)` internal seams. No reentrant mutex, no guard tokens. |
| Q10 | The guard is its own small core module with a private static mutex and one `serialized(|| …)` helper. Not in the store, not in `project_sync`. |
| Q11 | Propagation input: `(store, paths, skill_id, content_hash, now)`; it reads its own rows. Own core module spanning both scopes. |
| Q12 | One `refresh_managed_skills(ids | all, { reassert_auto_sync })` batch command streaming `Channel` progress; **delete** the per-skill update command; single Update = batch of one. Two phases: acquire all, then finalize+propagate each under the guard. |
| Q13 | Removal ships as two PRs: 03 (module + command-tier unsyncs + guard + admission rule + catalog-in-core) then 05 (project callers). |
| Q14 | New `SignalError::PathOutsideToolDirs { path }` owned by the tool registry, wire code `PATH_OUTSIDE_TOOL_DIRS`, per ADR-0001 procedure (variant + regenerated binding + `describeCommandError` branch + EN/ZH keys). |
| Q15 | Frontend: two **pure** modules (skill presentation; persisted preference), plain vitest, components import them, behaviour props deleted. |
| Q16 | Relative time collapses to the `relative.*` i18n family; `projects.*` relative keys deleted (EN + ZH). |
| Q17 | Execution: ticket 01 by hand (it sets the pattern); the rest delegated one at a time, parent reviews and runs the gate. |
| Q18 | Import: one `import_onboarding_selection(selections, { auto_sync })` command with progress + report; `import_existing_skill` and `remove_skill_source` deleted. |
| Q19 | Project mutations return a `ProjectViewDto { project, tools, assignments }`; the N×4 count queries become one aggregate query per project. |
| Q20 | The 12 raw modal members of the project world hook collapse to one `dialog` value + open/close. |
| Q21 | Git acquisition ships as 08 (cache twins, TTL as value, per-key lock, tests) then 09 (one acquisition module). |
| Q22 | GitHub API fast-path is enabled for install and update (its absence was drift, verified in history); git clone is the fallback adapter. |
| Q23 | Parallel acquisition in Refresh: bounded pool (4), after 08. |
| Q24 | One shared-dir confirmation building-block hook via the Modal shell; `window.confirm` removed. |
| Q25 | Branch `arch/<NN>-<slug>`, `git rebase main` before merge, post-merge revert check per AGENTS.md worktree-safety, gate `npm run version:check && npm run check`, version bumped once in ticket 12. |

## Facts verified during synthesis (so tickets don't re-derive them)

- The mutation lock is acquired at 7 command sites; `update_project_gitignore`, `delete_managed_skill`
  (which removes project-scope artifacts) and the managed-skill update path run without it;
  `remove_project_skill_assignment` does its two lookups before locking; the only serialisation
  test fabricates its own mutex.
- The update flow re-implements both the global and the project sync rule inline, bypasses the
  capability-aware sync entry point, duplicates the `force_copy` predicate, aborts on global
  failure but continues on project failure.
- The two command-tier unsync loops swallow filesystem errors; there are four distinct
  presence-probe / row-settlement policies across removal paths.
- Repo grouping is duplicated wholesale between the My Skills list and the assignment matrix;
  two relative-time formatters with different rules and i18n families; two behaviour props drilled
  through three components.
- The git cache has a single process-wide lock, so two clones of different repos serialise; the
  two cache entry points are line-for-line twins; the module has no tests.
- The API fast-path already records the real commit SHA.

## Ticket map

```
01 mutation guard ──┬─► 02 propagation + refresh batch ──┬─► 06 onboarding import
                    │                                     ├─► 09 git acquisition (also needs 08)
                    │                                     └─► 10 parallel refresh (also needs 08)
                    └─► 03 artifact removal + catalog ───┬─► 05 project removal callers ─► 07 project view
                                                          └─► 06
04 skill presentation (parallel)          08 git-cache (parallel)          11 shared-dir confirm (any time)
12 docs drift + version bump ◄── everything
```

Work the frontier: lowest-numbered open ticket whose blockers are resolved. 04 and 08 may run in
separate worktrees alongside the backend chain; they touch no merge-risk file.

## Closure — 2026-09-15

Shipped: v1.2.2 / 01e64b8. Tickets: 12 terminal (12 resolved), 0 still open (none).
Residue → BACKLOG: #13, #14, #27. Dropped by name: none.
