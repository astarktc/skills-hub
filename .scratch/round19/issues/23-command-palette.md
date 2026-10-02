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

- 2026-10-01 — from ticket 17: 16-recheck **R1 is still open** — the Projects matrix did not adopt the triggerless
  `PopoverContent anchor`, so its hidden focus-guard Tab stop / stays-open-on-Tab-out fix was not made there. If the
  palette anchors a popover, fix it in the primitive first (a real focus-manager return position; forward/reverse Tab
  leave cleanly; zero layer records; primitive + browser regression). Also seen in 17's axe runs: an open
  `DropdownMenu`'s portal fails `region` (best-practice) — every menu, primitive-level; the palette's own portal will
  likely hit the same rule.

- 2026-10-02 — (orchestrator, recorded late) from ticket 17's audit **N1 (P2)**: the Projects matrix is a plain Tab
  walk. It takes **127 Tab presses** from the first to the last cell of the 25-skill fixture matrix in both themes: 75
  cell checkboxes plus row verbs and repair controls (`round19/review/ticket-17-audit.md` § N1;
  `AssignmentMatrix.tsx` ~:209–256, ~:304–344, ~:388–435). Ticket 17's gate Comment says it was carried here ("N1 the
  matrix keyboard model"), but that entry never landed. The auditor's scope: design spatial/roving navigation with
  named cell actions and a clear Tab exit, test it as a complete interaction model, and don't just add `role=grid` to
  the current table. D23 doesn't require it. If the palette work defines a grid keymap, apply it to the matrix; if
  not, decide explicitly and record the decision here. CHANGELOG 1.3.0 lists it as a known limitation ("grid arrow
  keys come in 1.3.x").
