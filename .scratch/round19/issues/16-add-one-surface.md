# 16 — Add as one stepped surface: sticky summary, picker search, one ask, retryable partial failure

Status: ready-for-agent
Blocked by: 04, 09, 15
Spec: `.scratch/round19/spec.md` — D12, D27 (grill Q10, Q21). Absorbs BACKLOG **#47** and D8's "sticky install
summary". Sources: `flows.md` Flow 1; `critique-astra` F06, F14; `critique-opus` P1-4; `audit.md` Add/picker rows.

## Work

- **One Dialog, three steps** — *Source* (Git URL / Local folder segmented control with real `aria-selected`; the
  optional name only when one candidate) → *Candidates* (the picker: search box filtering the list — only checked
  **and visible** candidates install, and the summary says "3 of 4 checked · 1 hidden by filter"; invalid local
  candidates disabled with the reason in words; each checkbox labelled by the candidate name) → *Settlement*
  (progress "Adding (i/n) name", then the report inline: installed rows ticked, failed rows with the typed reason and
  **Retry**, cancelled rows marked). A **sticky summary** across every step: source · N candidates · Tool scope (the
  effective target set with an "Edit Tools" popover using the catalog — visible, focusable Checkbox primitives; the
  audit's `display:none` inputs are gone).
- **Backend calls**: listing as today; install through ticket 04's **`install_selection` batch** with a progress
  Channel; collisions come back as rows and raise **one** ask (`useOverwriteConfirmation`'s list shape) with per-row
  Keep/Overwrite, showing path, modified time and "differs from the skill"; one retry batch with overrides; Keep
  records disabled (ticket 02). `install_git_selection` / `install_local_selection` are deleted from the frontend,
  then from `collect_commands!` and the fixture in the same change.
- **Cancel** (D20): between candidates; the settlement step shows what landed and what was cancelled; the dialog
  stays open on partial failure or cancel; closes only on full success or the operator's Close.
- **Add flow exposes intents** (arch 10): `useAddSkillFlow` becomes `open({ source?, preselect? })`, `install()`,
  `retry(ids)`; the form draft is internal; Explore's "install this" calls the intent (the render-cycle trigger
  effect goes). `useCandidatePick` keeps the pick logic; two instances remain internal.
- Vocabulary: "Add", "Add selected", "Adding (i/n)"; "Import" nowhere in this flow.
- The deploy-after-add choice ("Sync to selected tools after creation") stays, as part of the Tool scope summary.
- Migration: `AddSkillModal`, `GitPickModal`, `LocalPickModal` (or their replacement) join `migrated`; `pick-*`,
  `form` rules leave `App.css`.

## Gate (D31)

Astra audit: keyboard-only Add of a 4-candidate repo with two collisions — Tool scope changed by keyboard, search
filters, one ask, Keep on one / Overwrite on the other, settlement shows 4 rows, focus returns to the Add button;
Escape closes only the ask; 960×640; contrast; `npm run check`.

## Result

(child fills in)
