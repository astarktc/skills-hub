# 18 — Operator think-aloud on the fixture (45 minutes)

Status: ready-for-human
Blocked by: 11
Spec: `.scratch/round19/spec.md` — D30 (grill Q23). Script: `round18/review/flows.md` § "Usability questions" per flow.

## Work

Orchestrator + operator, on `npm run dev:fixture` (`?scenario=failures`, then `?scenario=first-run`), the app as it
stands after ticket 11 (new My Skills; other surfaces still legacy under the alias palette). Never `tauri:dev`.

1. Flow 2 (Refresh) and Flow 6 (disable/delete) on the new list: the per-flow questions verbatim; note where the
   operator hesitates, misreads a mark, or looks for a control elsewhere.
2. Flow 1 (Add) and Flow 3 (Import) on the legacy surfaces: the questions verbatim — they calibrate tickets 16 and 21.
3. Flow 4 (Projects) on legacy: the colour-blind grayscale question.
4. Five minutes of free use: "what would you do first each morning?"

Record verbatim quotes and the moment of each under `## Findings` here; each finding becomes a line in the owning
ticket (`## Comments`, dated) or a BACKLOG item. Nothing here blocks release; it sharpens 16, 17, 21, 22.

## Findings

(orchestrator fills in after the session)
