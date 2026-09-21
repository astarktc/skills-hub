# Handoff — 2026-09-21 · round 15 (wave C, BACKLOG #02) closed, released as 1.2.15

The queue is `.scratch/BACKLOG.md` — this note does not repeat it.

## Where things stand

- **1.2.14 verified** at session start: release `v1.2.14` published with all five targets + `updater.json`
  (`release.yml` 35619110341 green on the bumped Actions majors); Windows never carried `.sig`/updater entries, so
  that asset shape is not a regression. Operator smoke passed.
- **Round 15 is done and archived** (`archive/round15/`): BACKLOG #02 shipped — every fan-out report crosses the wire
  as its core type; `CommandError` lives in `core/errors.rs`; per-row failures are classified where the row settles
  (ADR-0001 round-15 amendment: two timings); counters derived in `src/lib/reportOutcome.ts`; delete and the project
  assignment toggle-off return a `RemovalReport`; `DELETE_CLEANUP_FAILED` retired. Two sequential Astra lanes + a
  parent docs ticket + an Astra fix ticket; Opus 5 review fix-then-ship → re-check ship. Released as **1.2.15**
  (`a3cff96`).
- **Release:** the push carrying `a3cff96` triggers `auto-tag.yml` → `release.yml`. Watch it like last time
  (five assets, `.sig`s, `updater.json`). Operator smoke list is in `archive/round15/spec.md` Closure.
- Gate at close: `npm run version:check && npm run check` green; `cargo test --all` 647; vitest 362.
- No live effort. Next free BACKLOG number **#37**.

## Exact next step

1. Confirm the v1.2.15 GitHub release built (all five targets); operator smokes the delete-kept path and the project
   toggle-off toast.
2. BACKLOG "Now" is empty. "Later" holds #12 #15 #16 #17 #18 #35 #36 — pick by value; #35 + #36 are small and both
   close the last report-shaped asymmetries left by round 15 (a natural "round 16 small bundle" with #12).

## Gotchas learned this session

- A reviewer's *minimal* fix can bend the rule it is defending: "return `Result<(), CommandError>`" would have made
  core classify store failures it then propagates. Take the fuller fix when it removes the producer instead of
  guarding it — here that also let the compiler enforce the invariant (`impl Error for CommandError` deleted).
- A lane's disclosed deviation ("preserved toggle failure behaviour") is a prompt to re-check operator-visible copy,
  not just control flow — the failure was preserved, the localized framing was not.
- Ticket grep gates should name the legitimate survivors up front (`ToolStatusDto`, `SkillTargetDto`, …); both lanes
  spent report space explaining a gate that was too broad.
- Two sequential lanes worked cleanly for a bindings-dependent change: the backend lane's old→new rename map pasted
  into the frontend lane's brief was enough — no shared-file collisions, no rework.
