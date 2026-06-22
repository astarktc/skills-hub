# Phase 3: IPC Commands - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-07
**Phase:** 03-ipc-commands
**Areas discussed:** Bulk-assign design, Frontend DTO placement, Error response contract

---

## Bulk-assign design

| Option                        | Description                                                                                                                                            | Selected |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | -------- |
| Backend command (Recommended) | One IPC call: backend reads configured tools, assigns each, returns summary with per-tool success/failure. Efficient, one roundtrip, atomic reporting. | ✓        |
| Frontend loop                 | Frontend calls add_project_skill_assignment N times (once per configured tool). Simpler backend, but N roundtrips and no atomic summary.               |          |
| You decide                    | Let Claude pick the best approach during planning.                                                                                                     |          |

**User's choice:** Backend command
**Notes:** None — straightforward selection of recommended approach.

### Follow-up: Bulk-assign response shape

| Option                        | Description                                                                                                                                                            | Selected |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| Per-tool detail (Recommended) | BulkAssignResultDto with assigned (Vec<ProjectSkillAssignmentDto>) and failed (Vec<BulkAssignErrorDto>) lists. Frontend can show exactly which tools succeeded/failed. | ✓        |
| Summary counts only           | Just counts (assigned: N, failed: N) like ResyncSummaryDto. Simpler but frontend can't show which tools failed.                                                        |          |

**User's choice:** Per-tool detail
**Notes:** None.

---

## Frontend DTO placement

| Option                              | Description                                                                                                                | Selected |
| ----------------------------------- | -------------------------------------------------------------------------------------------------------------------------- | -------- |
| New projects/types.ts (Recommended) | Create src/components/projects/types.ts now. Phase 4's component tree imports from its own directory. Clean separation.    | ✓        |
| Existing skills/types.ts            | Add project DTOs to existing types.ts. Simpler now but mixes concerns. Phase 4 would import from skills/ which is awkward. |          |
| You decide                          | Let Claude decide during planning.                                                                                         |          |

**User's choice:** New projects/types.ts
**Notes:** None — anticipates Phase 4's separate component tree.

---

## Error response contract

| Option                             | Description                                                                                         | Selected |
| ---------------------------------- | --------------------------------------------------------------------------------------------------- | -------- |
| Add project prefixes (Recommended) | Add project-specific prefixes following existing pattern. Frontend can detect and show targeted UI. | ✓        |
| Generic errors only                | Rely on format_anyhow_error() strings. Frontend just shows toast.                                   |          |
| You decide                         | Let Claude decide which errors need prefixes.                                                       |          |

**User's choice:** Add project prefixes

### Follow-up: Which prefixes

| Option            | Description        | Selected                                                                            |
| ----------------- | ------------------ | ----------------------------------------------------------------------------------- | --- |
| DUPLICATE_PROJECT |                    | Path already registered. Frontend highlights duplicate, offers navigation.          | ✓   |
| ASSIGNMENT_EXISTS |                    | Skill already assigned to project+tool. Frontend shows "already assigned".          | ✓   |
| NOT_FOUND         | (project or skill) | ID doesn't exist (deleted between list load and action). Frontend triggers refresh. | ✓   |

**User's choice:** All three prefixes
**Notes:** None.

---

## Claude's Discretion

- Internal function signatures for bulk-assign command
- Whether BulkAssignErrorDto is a new struct or reuses existing pattern
- Test structure for new command and error prefix validation
- Whether error prefixes are emitted from command layer or core layer

## Deferred Ideas

None — discussion stayed within phase scope.
