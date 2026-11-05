# 05 — Replace the hand-simulated `bulk_assign_*` tests with engine coverage (BACKLOG #12)

Status: done — 449d2ab
Spec: `.scratch/round16/spec.md` — D7. Orchestrator ticket.

## Work

Diff the assertions of `core/tests/project_sync.rs` `bulk_assign_to_multiple_tools` (:702),
`bulk_assign_skips_already_assigned` (:768), `bulk_assign_continues_on_error` (:821) against the engine suite
`fanout_assigns_every_tool_in_caller_order` (:1057), `fanout_reports_already_assigned_as_data_and_does_not_duplicate`
(:1093), `fanout_isolates_an_unknown_tool_and_continues` (:1133), `fanout_keeps_sync_failures_inside_the_assignment_record`
(:1179). Delete what is covered; port any uncovered assertion into the engine suite through the real entry point.
Record the mapping below. Runs after ticket 01 merges (the engine's report type changes there).

## Mapping

| Old test (deleted) | Assertion | Engine test that covers it |
|---|---|---|
| `bulk_assign_to_multiple_tools` | both tools assigned, both target paths exist | `fanout_assigns_every_tool_in_caller_order` (items Synced, rows Synced, `.claude/skills` + `.agents/skills` targets exist) |
| `bulk_assign_skips_already_assigned` | pre-assigned tool is skipped, no duplicate row | `fanout_reports_already_assigned_as_data_and_does_not_duplicate` (AlreadyAssigned names the existing row id; row count unchanged for that tool) |
| `bulk_assign_continues_on_error` | a sync failure keeps a row with status Error; other tools still get rows | `fanout_reports_a_sync_failure_with_the_kept_error_row` (kept Error row, typed `INVALID_PATH`, item names the row) + `fanout_isolates_an_unknown_tool_and_continues` (fan-out proceeds past a failed tool). The old test never exercised continuation — it called `assign_and_sync` twice by hand. |

Nothing ported: no uncovered assertion. 662 → 659 tests.

## Comments

- 2026-09-22 — done in `449d2ab`. Ran after ticket 01 merged (the engine's report type changed there). Clippy clean.
