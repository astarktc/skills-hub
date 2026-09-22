# 05 — Replace the hand-simulated `bulk_assign_*` tests with engine coverage (BACKLOG #12)

Status: ready-for-agent
Spec: `.scratch/round16/spec.md` — D7. Orchestrator ticket.

## Work

Diff the assertions of `core/tests/project_sync.rs` `bulk_assign_to_multiple_tools` (:702),
`bulk_assign_skips_already_assigned` (:768), `bulk_assign_continues_on_error` (:821) against the engine suite
`fanout_assigns_every_tool_in_caller_order` (:1057), `fanout_reports_already_assigned_as_data_and_does_not_duplicate`
(:1093), `fanout_isolates_an_unknown_tool_and_continues` (:1133), `fanout_keeps_sync_failures_inside_the_assignment_record`
(:1179). Delete what is covered; port any uncovered assertion into the engine suite through the real entry point.
Record the mapping below. Runs after ticket 01 merges (the engine's report type changes there).

## Mapping

_(filled at close)_
