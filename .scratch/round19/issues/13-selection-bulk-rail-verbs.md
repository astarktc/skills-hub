# 13 — Selection, bulk rail and verbs: per-surface selection, `x`/`Esc`, bulk Disable/Enable/Sync/Assign/Delete with count confirmations, Delete dialog → report, `j/k`

Status: ready-for-agent
Blocked by: 01, 09, 11
Spec: `.scratch/round19/spec.md` — D8, D16, D21; contract § SIGNATURE INTERACTION. Absorbs BACKLOG **#52**'s
multi-select half. Sources: `critique-opus` P0-1, P1-1; `critique-astra` F01, F10, F17; `flows.md` Flow 6, P2.

## Work

- **Selection model** on My Skills (surface state, per surface): row checkbox, `x` toggles the focused row, `Esc`
  clears, shift-click range, group-header checkbox selects the source, ⌘A selects the visible subset (lens). Selected
  rows are marked with the iris accent (selection is one of its three uses).
- **Bulk rail** rises from the foot on ≥1 selection (≤120 ms opacity/transform; removed under reduce; never gates
  interaction): "N selected · `x` select · `Esc` clear" + verbs through the door: **Sync to Tools…** (Tool multi-pick
  from the catalog; one batch; one ask) · **Assign to project…** · **Disable on all Tools** / **Enable** · **Update** ·
  **Remove…**. Every wide verb shows a **count-bearing confirmation** (ConfirmDialog, focus on Cancel, destructive
  tone for Remove): "Disable 60 skills on 7 Tools — 412 Sync targets are removed and kept disabled; project
  assignments for these skills are disabled too. Refresh will not re-add them." Library-wide undeploy is exactly
  select-all → Disable on all Tools (the old toolbar button is already gone in ticket 11).
- **Row switch** = per-skill Disable/Enable (D8) with a quiet inline confirmation for *Enable* only when it will
  re-sync to N Tools (count shown) — no confirm for Disable of one skill (reversible, and the switch shows the state).
- **Delete** (D16): the confirm lists the Tools and the projects that carry the skill and the number of Edits lost;
  imported/detached skills add "Skills Hub's copy is the only copy — this cannot be undone". After a partial delete
  the same dialog **becomes the report**: "*release-train* was not deleted. Removed from Codex, Pi. Still in Claude
  Code: `/Users/…/release-train` — permission denied. [Reveal in Finder] [Retry] [Close]" (`open_log_folder`'s reveal
  seam or a new `reveal_path` command — check the platform seam first). Bulk Remove uses the same dialog with counts.
- **Keyboard rows**: `j`/`k` move focus, Enter opens the inspector, `x` selects, `/` search, `Esc` clears selection
  then closes the inspector — documented in the sidebar's shortcuts entry.
- **Focus after mutation** per D21 through the lifecycle's `focusAfter`.
- Migration: `FilterBar`/`SkillsList` already migrated; the rail is a new component under `components/skills/`.

## Gate (D31)

Astra audit: keyboard-only select three rows → Disable → confirm → strip shows disabled → Refresh (fixture) leaves
them → Enable re-syncs; select-all → Disable on all Tools confirmation shows the right counts; Delete of the locked
`release-train` turns the dialog into the report; focus never lands on `<body>`; rail motion absent under reduce.
`npm run check` green.

## Result

(child fills in)

## Comments

- 2026-09-29 — from ticket 09: the door (`useSkillActions.run(verb, skills | ids | "all", opts)`) exists; a verb called without its confirming option opens its dialog. Count-bearing confirmations need a new ask kind in `useSurfaceState` (04 owned the ask kinds this round; add `{kind:'confirm-count', …}` here). The toolbar's library-wide Disable still has no confirmation until this ticket lands. Ticket 03 left project/removal/disable-all/delete/enable toasts on their old sentences — move them to the count line here with the rail.
