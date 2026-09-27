# 07 — Task-flow and state review of the six core flows

Status: implemented
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

## Result

`.scratch/round18/review/flows.md` (2026-09-26). Six flows modelled to the operator's outcome with force-driven state
tables, ranked gaps and round-19 recommendations; D8 features mapped onto the flows they serve; a consolidated
screen/state inventory with omission review; four pattern decisions (P1–P4); fixture gaps; 12 grill questions for
ticket 09. Evidence boundary recorded: the operator is the only validated user; findings are heuristic + source-read
hypotheses, observed in the fixture (`failures`, `first-run`, `latency=0|3`) or marked "described from code".
Screenshots (gitignored): `.scratch/round18/evidence/flows/`.

Headline findings: failures without a row (declined/failed first sync, failed acquisition, failed import) leave no
trace once toasts close — outcome memory is session-only; overlay Cancel is honoured by only 5 commands; "Uninstall
from tool directories" removed 68 deployments with no confirm and auto-sync re-deploys unsynced skills on the next
Refresh; the fold's `conflict › failure` headline turned a run with 5 failures into "Refresh complete — invocation Edit
conflicts need review"; auto-sync-off import deletes Tool copies behind an "Import & Sync" button; local skills show
their central path instead of their (missing) source folder; Re-point accepts a different skill without preview.
