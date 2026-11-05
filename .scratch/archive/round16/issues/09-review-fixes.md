# 09 — Review fixes (Astra, fix-then-ship)

Status: done — b8ebb35, 634cef8
Review: `.scratch/round16/review/astra-review.md` (GPT-6 Astra, thinking high, over `2145472..4a721b9`).

## Disposition

| Finding | Disposition | Commit |
|---|---|---|
| **M1** — a sync failure whose settlement write the store refuses was reported as a settled `error` row (assign: `assign_skill_to_tools` caught every `Err` and re-read whatever row existed; resync: `log::warn!` and carry on) | **Fixed, fuller form**: `AssignAttempt::{Refused(anyhow), Settled{record, sync_error}}` — a refusal (unknown tool) is settlement data classified when the item is built; a store failure is `Err` and propagates. `assign_skill_to_tools` → `Result<ProjectSyncReport>`, redundant post-failure row read gone. Resync's settle failure propagates `with_context` carrying the sync chain. Two regression tests inject a SQLite `BEFORE UPDATE … WHEN NEW.status='error' … RAISE(ABORT)` trigger and assert both paths fail at the boundary with the inserted row still `pending` / the previous row still `synced`. Wire unchanged. | `b8ebb35` |
| **S1** — resync-all's success-path `refreshView` swallowed a failed matrix re-read | **Fixed**: `refreshView` returns whether it applied; `resyncAll` answers `{ report, viewRefreshed }`; `ProjectsPage` warns `projects.viewRefreshFailed` (EN/ZH) when false. Hook test added. | `b8ebb35` |
| Nit — stale compat comments above `changeSource` (EN/ZH) | removed | `b8ebb35` |
| Nit — `refreshView` doc said success paths never use it | rewritten | `b8ebb35` |
| (self-found) `handleResyncAll` missing `notify` in deps — a react-hooks warning `npm run lint` does not fail on | fixed | `634cef8` |

Gate after fixes: cargo 661, vitest 393, eslint 0 warnings, build ok, bindings clean.

## Comments

- 2026-09-22 — orchestrator applied all findings. The M1 fix follows the round-15 lesson: the reviewer's "smallest" fix (propagate the resync settle error only) would have left the assign engine's catch-all in place; the fuller fix removes the producer of the wrong claim.
