# 23 — ⌘K palette on the surface module and the action door; `cmdk` vs Base UI Combobox decided here

Status: ready-for-agent
Blocked by: 19
Spec: `.scratch/round19/spec.md` — D1 caveat, D4; contract § SIGNATURE INTERACTION; round-18 D8. Phase 19b.

## Work

- Decide the base with evidence (ADR-0006 §3 caveat): prototype the palette on Base UI `Combobox`/`Autocomplete` +
  our Dialog first; fall back to shadcn Command (`cmdk`, drags one Radix dialog) only if a concrete D5 gap appears.
  Record the decision as a dated `## Comments` entry and in `DESIGN.md`.
- Reach: every skill by name (opens the inspector), every area (`#/…`), every door verb on the **current surface's
  selection** ("Disable 3 selected on all Tools…"), Refresh all, Add, Settings panes, "Remove all skills from
  tools…" (the same count confirmation). Recently used first; typed-ahead matching through the lens's query fold.
- ≤120 ms open/close; removed under reduce; focus returns to the invoking element or the focused row.
- The shortcuts entry in the sidebar lists `⌘K`, `/`, `j/k`, `x`, `Esc`, `Enter`.

## Gate (D31)

Astra audit: keyboard-only — open, find a skill, open the inspector, return; run a bulk verb from the palette with
three rows selected; screen-reader semantics of the listbox; `npm run check`.

## Result

(child fills in)

## Comments

- 2026-09-30 — from ticket 15: DESIGN.md § Motion's table reserves the palette's row: opacity + transform, ≤ 120 ms
  (`--motion-fast`, `ease-out`), `motion-reduce:` counterpart, owned by its primitive. Frequency first: ⌘K opens many
  times a day and from the keyboard — the rubric (Emil) says no open/close animation at all is the better default;
  decide there, and record it in the table.
