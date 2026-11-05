# 24 — Card-grid view mode designed inside the world

Status: done — (sha in the tracker commit)
Blocked by: 19
Spec: `.scratch/round19/spec.md` — D32; contract § Unresolved (grid); round-18 D2 ("both list and card-grid view
modes exist in the new world"). Sources: `critique-astra` F07. Phase 19b.

## Work

- One grid card = the same row model: name · marks after the name · description (2 lines, then ellipsis with a title)
  · strip + N/M · scope · switch · overflow; **no clipped Tool labels ever** (the strip is marks, labels are accessible
  names + the inspector); same selection, same keyboard model (`j/k` moves across the grid by reading order), same
  group headers with the singleton fold; auto and dense densities as the two grid sizes through the lens's view
  mode; persisted preference keys kept (`preferences.ts` compat contract).
- Grid uses the primitives and tokens only; no new tokens unless `DESIGN.md` gains them in the same change.

## Gate (D31)

Astra audit on `rich` in both densities: nothing clipped at 960×640; keyboard reading order; contrast; `npm run check`.

## Result

Implemented 2026-11-04 by a delegated child (uncommitted; the orchestrator reviews and commits). No new token, variant or
primitive; no `App.css` rule.

**Header component** — `src/components/skills/SourceGroupHeader.tsx` (private to My Skills). The list and the grid both
render it, and `SkillsList` keeps no copy of the block. Props: `group` · `layout: "list" | "grid"` · `headingId` ·
`rowsId` · `collapsed` + `onCollapsedChange(next)` (the chevron and ←/→ on the header) · `selectedCount` +
`onSelectGroup` · `legend?: ReactNode` (the strip column's content) · `busy` · `blockedReason` · `actions` (`run`) · `t`.

- `list`: the row's `LIST_COLUMNS` template; `legend` sits over the strip.
- `grid`: `2rem · 1fr · auto` (checkbox · identity cluster · Deploy all); no legend.
- `SkillsList` keeps the ordering and measurement (`--strip-marks`, `--name-col`) and builds the initials legend
  (list only). The grid does not read `--name-col`.

**View mode** — `ViewMode` = `"list" | "auto-grid" | "dense-grid"`, under the existing key `skills-viewMode`
(`viewModePreference`). Both grid values already existed in the union, so no key and no value changed. The switch
stays where it was: the "Sort and view" menu's View radio group (Base UI radio items, operable by arrows + Enter).

- Labels are now List · Grid · Dense grid (were "Auto Grid" / "Dense Grid").
- The trigger's accessible name carries the view: "Sort and view: Name A–Z · Grid".

**Card** (`SkillRow` `layout="card"`, rebuilt on the row model), in reading order:

- Line 1: checkbox · name (+ pencil) · switch · overflow.
- Then the `rowMarks` badges, with words, wrapping.
- The description, two lines reserved, clamped, with its whole text as the title (the source in mono first in Flat and
  "Individual sources").
- At the foot: strip + N/M · scope (wraps under the strip before anything clips).

Grounds are the row's:

- base: `rounded-md`, `border` on `row`; hover `accent`;
- focused: `raised` + its edge turned `ring`;
- selected: `raised` + the 3 px iris edge (`rounded-l-md`).

The card keeps the same `data-skill-item` / `data-skill-row` / `data-skill-health` hooks and the same verbs through
the door's handle.

**Widths** (`listColumns.ts` `GRID_COLUMNS`): `repeat(auto-fill, minmax(min(100%, max(floor,
--strip-marks × 20 px + 40 + 26 px)), 1fr))`. The strip term means a card can never clip its marks.

| Density | Floor | Gaps / inset |
|---|---|---|
| Grid (`auto-grid`) | 19rem (304 px) | 12 px |
| Dense (`dense-grid`) | 15rem (240 px) | 8 px |

Measured on the fixture (`rich`, inspector closed):

| Window | Grid: columns (card width) | Dense: columns (card width) |
|---|---|---|
| 960 × 640 | 2 (417 px) | 3 (279 px) |
| 1280 × 800 | 3 (333 px) | 4 (252 px) |
| 1800 × 1000 | 4 (377 px) | 6 (252 px) |

With the inspector beside at 1280 the list pane is 720 px, which gives 2 columns in both densities.

**Reading-order rule**: the grid auto-places row-major in DOM order, and DOM order is the lens's `onScreen` order. So
the one keymap (`j` / `k`, `x`, ⇧-click, ⌘A) and the one focus fold (`focusAfter`) need no grid code. There are no
Home/End keys (the list has none).

**Initials in grid**: dropped. A wrapped grid has no strip column to sit over, and repeating them per card is the
clutter D42 removed. The marks' names (title + accessible name) and the inspector name each harness. DESIGN.md says
so.

**Two small additions beyond the brief, both shared with the list:**

1. **Shift-press on a checkbox.** It no longer extends the page's text selection; before, a ⇧-click range painted a
   text selection across every card or row between. Proof: `getSelection()` is empty after the range.
2. **`scroll-pt-15` on the list pane.** `j` / `k` no longer stop a card, or a row, under the sticky group header.
   Proof: walking 14 cards down and back in both views, no focused item's top was under its header.

**Tests**: `useListLens.test.ts` gains "view modes (ticket 24)":

- all three values persist under `skills-viewMode`, read back, and an unknown value falls back to `list`;
- `onScreen` is identical in every view mode, with a collapsed group excluded;
- Delete's focus rule lands on the next card in reading order across the collapsed group.

**Fixture proofs** (`npx vite --mode fixture --port 5196`, `?scenario=rich`, ego-browser):

- **Nothing clipped.** Both themes × both densities × 960 / 1280 / 1800 (12 runs): 0 cards with `scrollWidth >
  clientWidth`, 0 truncating or clamped elements without a title, no horizontal scroll. Column counts are as in the
  table above.
- **`j` reading order at 1280, Grid.** artifacts-builder (229,204) → canvas-design (574,204) → docx (918,204) →
  frontend-design (229,338) → mcp-builder (574,338): the 4th `j` wraps to the second row. `k` → frontend-design.
- **Selection.**
  - `x` selects frontend-design.
  - ⇧-click from card 1 to card 6 selects 6 cards in reading order.
  - Enter opens mcp-builder: `aria-current`, the inspector beside.
- **Delete's focus rule.**
  - canvas-design → focus on docx (the next card).
  - The last card on screen (pi-lens) → ui-ux-pro-max (the previous one).
  - The only card shown (search "pptx") → the `My Skills` heading (`data-surface-heading`).
- **Collapse.**
  - ←, with focus on anthropics' chevron: `aria-expanded=false`, 0 cards, `skills-collapsedSources` =
    `["anthropics/skills"]`.
  - `j` then goes to api-contract-review; anthropics stays collapsed after a reload.
- **View switch by keyboard**: menu trigger, Enter, arrows to Grid, Enter, Esc → `auto-grid`; again to Dense grid →
  `dense-grid`. After a reload: dense-grid, 4 columns.
- **List unchanged.** The list view at 1280 dark is pixel-identical to `main` 936f464 (served from a throwaway
  worktree on 5197, since removed): the PIL difference bbox is `None`, and every header/row rect and the header's HTML
  length are the same.

**Screenshots** in `.scratch/round19/evidence/24/` (gitignored):

- `grid-{auto,dense}-{dark,light}-{960,1280,1800}.png`
- `grid-keys-dark-1280.png`: 6 selected, the focused mcp-builder, the inspector and the rail
- `grid-collapsed-dark-1280.png`
- `list-unchanged-dark-1280.png`

**Gate**: `npm run version:check && npm run check` exits 0.

- vitest: 1250 passed (56 files).
- cargo: 854 passed.
- lint, build (TS 7), clippy and fmt are green.
- styles:check: App.css 355, the ceiling, unchanged; 50 migrated files (`SourceGroupHeader.tsx` appended).
- `npx tsc --noEmit -p tsconfig.app.json` and `git diff --check` are clean.

**Files:**

- New: `src/components/skills/SourceGroupHeader.tsx`.
- Changed: `src/components/skills/{SkillsList,SkillRow,FilterBar}.tsx`, `src/components/skills/listColumns.ts`,
  `src/hooks/useListLens.test.ts`, `src/i18n/resources.ts` (two values), `scripts/styles-baseline.json`, and
  `DESIGN.md` (§ My Skills: files, the header bullet, the new "The grid (ticket 24)" subsection replacing the
  stop-gap bullet, and two notes in § Selection keys).

**Not proven here:**

- The Astra audit (D31).
- Contrast is not re-measured, because every pair is a row pair DESIGN.md already lists.
- VoiceOver speech (the AX names were read from the DOM only).
- Reduced motion: the grid adds no motion.
- In Dense, names over ~130 px truncate, with their title. That is inherent to 4 columns at 1280; operator's eyes on
  `grid-dense-*-1800.png`.

## Comments

- 2026-11-02 — Vocabulary: read Tool/Sync/Refresh as Harness/Deploy/Update — ticket 32.
- 2026-11-04 — operator grill (autonomous run through 31 → 20 ∥ 21 → 34 → 24 → 25 → 26; results reviewed at the end): grid still wanted in the T3-style dark world (round-18 D2 stands).
- 2026-11-04 — from ticket 31's standards review (S3): the group header is inline in `SkillsList.tsx` (selection, collapse, identity, summary, initials legend, Deploy all); extract it as a private header component when the grid reuses it — the grid must not copy the block.
- 2026-11-04 — implemented by delegated child; awaiting Astra audit
- 2026-11-04 — orchestrator, after the audit (`round19/review/ticket-24-audit.md`, FAIL on F1 only): the ⇧-click
  text-selection fix's `preventDefault` also suppressed the native focus move, so `x` after a shift-click did nothing
  and `j` from the search typed into it — fixed by focusing the checkbox by hand on the shifted press; proven in the
  browser (range of 3, no text selection, focus on the clicked box, `x` toggles it). F2 (Dense's edited-name allowance)
  noted; Dense stays as measured.
