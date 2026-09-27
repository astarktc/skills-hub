# 07 — Task-flow and state review of the six core flows

Status: ready-for-agent
Blocked by: 04
Spec: `.scratch/round18/spec.md` — ticket 07.

## Work

Run the `product-design-and-ux` skill (read its SKILL.md; load `references/task-flows-and-state-models.md`,
`content-and-cognitive-demand.md`, `interaction-pattern-selection.md`; templates `task-flow-state-model.md` and
`screen-state-inventory.md`) on the fixture app for: (1) Add → pick → sync → overwrite ask; (2) Refresh (all) and
its report; (3) Onboarding import; (4) Project assignment / bulk / resync; (5) Re-point + Edit + Restore; (6)
Delete / unsync. For each: the task model to the *user's outcome* (not "report shown"), every state the flow forces,
interruption / re-entry / cancellation / recovery, partial-failure presentation, irreversible steps, and the
questions a usability test would ask. Evidence boundary: the only validated user is the operator; record that,
invent no personas. Output `.scratch/round18/review/flows.md`: per flow, the state model, the gaps ranked, and the
concrete recommendation each gap implies for round 19 (with the D8 reserved features mapped onto the flows they
serve). Read `src/lib/reportOutcome.ts` — the fold's precedence (conflict › failure › skipped › success) is the
current presentation contract; judge it, don't ignore it.
