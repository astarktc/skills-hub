# 23 — ⌘K palette on the surface module and the action door; `cmdk` vs Base UI Combobox decided here

Status: done — 359bb3b (+ audit fixes in the closing commit)
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

**Worktree** `/Users/alexstark/.t3/worktrees/skills-hub/t3-f516c084` · **branch** `t3/f516c084` (from `e856b7b`;
everything uncommitted, for the orchestrator to commit / rebase / merge).

**Base: Base UI `Autocomplete` (`inline`) inside our `Dialog`** — no `cmdk`, no Radix dialog (Comment 2026-11-02,
DESIGN.md § CommandPalette). No D5 / a11y gap appeared on the Base UI route: named dialog → combobox → listbox →
named groups → options, active descendant, one layer of the one stack, axe 0 violations both themes × both sizes.
**Motion: none** (decided, DESIGN.md § Motion). **N1:** the palette's keymap does not apply to the matrix; the matrix
grid model needs its own home (Comment). **R1:** not needed (no anchored popover).

Gate (worktree, 2026-11-02): `npm run version:check && npm run check` → exit 0 — lint, styles:check (App.css 1108,
37 migrated), **vitest 51 files / 1073 tests**, `tsc -b` + vite build, rustfmt, clippy `-D warnings`, **cargo 837
passed** (+ 0-test targets). Bindings unchanged. Fixture proof: `evidence/23/` (gitignored; its README lists every
file) — all three Gate flows keyboard-only in dark/light × 1280×800/960×640, AX tree, axe, contrast, reduced motion,
palette over the sheet, refusal while Update all runs, the shortcuts list. Vite :5178 stopped.

**Files**
- new `src/components/ui/command-palette.tsx` — the primitive (Autocomplete inline in Dialog; refused option =
  `aria-disabled` + reason + held-refusal dispatch); `src/components/ui/command-palette.test.tsx` (7 tests: roles /
  names / active descendant / Enter / refusal / Escape + focus return + zero layers / inert / empty status).
- new `src/lib/commandPalette.ts` — the pure item fold `paletteGroups` (reach, order, count labels, refusal, recents
  via `rememberRecent`, `RECENT_MAX = 5`, `SETTINGS_PANES`) + `openSettingsPane`; `src/lib/commandPalette.test.ts`
  (13 tests).
- new `src/components/shell/CommandPalette.tsx` — the surface: ⌘K/Ctrl+K toggle (opens over an empty dialog stack,
  works from a text field), showing surface's selection (D4), close-first-then-run (waits for focus to leave the
  palette, so a verb's confirmation records the returned element as its opener), place focus → heading / landmark.
- `src/App.tsx` — the one mount (+ import); nothing else.
- `src/hooks/useSurfaceState.ts` — `DialogState` gains `{ kind: "commandPalette" }`.
- `src/lib/skillPresentation.ts` — the lens's query rule extracted as `queryMatcher` (behaviour unchanged;
  `filterAndSortSkills` uses it) so commands and skills match one way.
- `src/components/ui/dialog.tsx` — `placement="top"` (pinned 12 vh; sizes apply). `src/components/ui/held-refusal.ts`
  — `dispatchRefusal` exported. `src/components/shell/AppSidebar.tsx` — ⌘K first in Keyboard shortcuts (`⌘K`, `/`,
  `j / k`, `Enter`, `x`, `⇧ click`, `⌘A`, `Esc`).
- `src/i18n/resources.ts`, `scripts/styles-baseline.json` (+ `CommandPalette.tsx` migrated), `DESIGN.md` (§ Dialog
  placement, § CommandPalette, § DropdownMenu `region` exception, § Motion row + "does not move", § The ⌘K palette).

**Settings route:** every "Settings → …" entry goes through **`openSettingsPane(surface, pane)`**
(`src/lib/commandPalette.ts`) — ticket 20 retargets that one function to the Settings dialog and extends
`SETTINGS_PANES` (today: `github`).

**New `en` keys** (one block appended at the end): `palette.title`, `.inputLabel`, `.placeholder`, `.empty`,
`.groups.{recent, selection_one/_other, actions, places, skills}`, `.selection.{syncToTargets, assignToProject,
disable, enable, update, delete}_{one,other}`, `.disableAll`, `.nothingToUpdateAll`, `.nothingToDisableAll`,
`.settingsPane`, `.panes.github`, `.keywords.{delete, add, disableAll, place}`, `.footer.{move, run, close}`,
`.shortcutKey`, `.shortcutOpen`. Reused: `bulk.nothingTo*`, `library.updateAll`, `addSkillButton`, `configureTools`,
`nav*`, `settings`, `shell.shortcutEnter/Escape`.

**Deliberately left**
- Selection verbs exist on My Skills only in practice: the matrix does not use the surface module's `matrix`
  selection yet (the palette reads it when it does); Explore candidates take no door verb.
- "Remove all skills from tools…" ships as "Disable every skill on all harnesses…" (Comment: Remove = Delete here).
- Recently used comes first, so a recent "Update all" is the highlighted entry (⌘K → Enter repeats it — the same verb
  the header runs without a confirmation). With no recents/selection the first entry is a place.
- Ctrl+K is taken in text fields too (macOS's emacs kill-line binding in an input yields to the palette).
- No component test for the surface `CommandPalette.tsx` (AGENTS: primitives only); its behaviour is the fixture
  transcript. The matrix grid keymap (N1) and the anchored-popover focus guard (R1) are not built here.

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
- 2026-11-02 — Vocabulary: read Tool/Sync/Refresh as Harness/Deploy/Update — ticket 32.
- 2026-11-02 — **Base decided: Base UI `Autocomplete` (`inline`) inside our `Dialog`, not shadcn Command / `cmdk`**
  (implementation child). Prototyped first on Base UI per the Work list; no concrete D5 / a11y gap appeared, so the
  `cmdk` fallback (and the one `@radix-ui/react-dialog` it drags in, ADR-0006 §3) stays out — `cmdk` was never added.
  Evidence: Base UI 1.8 documents this exact composition as its "Command palette" pattern; the palette is one more
  layer of `layer-stack.ts` (Escape over the inspector sheet closes the palette alone; the sheet stays, focus returns
  to its heading — `evidence/23/sheet-transcript.json`); semantics measured in the browser are dialog (named) →
  combobox (`aria-autocomplete=list`, `aria-expanded=true`, `aria-controls` = the listbox) → `group`s named by their
  labels → `option`s, highlighted option = `aria-activedescendant` with DOM focus kept in the input; disabled items
  stay navigable (Base UI passes no `disabledIndices`), which is what "refused, listed, focusable, says why" needs;
  axe 4.10.3 with the palette open: **0 violations** in both themes × 960×640 / 1280×800 (incompletes: Base UI's two
  focus guards, an `aria-controls` axe could not resolve though the id exists, the ↑↓ glyph `kbd`). Recorded in
  DESIGN.md § CommandPalette. Two Base UI facts the implementation depends on: `onOpenChangeComplete(false)` fires a
  frame **before** focus is handed back (the palette waits for focus to leave it before running a choice), and an
  `inline` Autocomplete lets Escape bubble to the Dialog.
- 2026-11-02 — **Motion: none** (decided; DESIGN.md § Motion table row + "What does not move"). Frequency first:
  ⌘K opens tens of times a day from the keyboard, and the reserved 120 ms would only delay the input being typed
  into. Measured under `prefers-reduced-motion: reduce`: 0 running animations, 0 elements with a non-zero transition
  (`evidence/23/extra-transcript.json`) — the same holds without `reduce`.
- 2026-11-02 — **N1 (matrix keyboard model): the palette's keymap does NOT apply to the matrix.** The palette defines
  no grid keymap: its keys (↑/↓ move the active descendant, Enter, Esc) live inside its own popup and never reach a
  surface. A spatial/roving model for the Projects matrix is a design of its own (named cell actions, a clear Tab exit,
  tested as a complete interaction model, per the 17 audit) and was not built here. It stays open as a separate item
  (CHANGELOG 1.3.0's "grid arrow keys come in 1.3.x") — the orchestrator should give it a BACKLOG/ticket home; this
  child does not edit BACKLOG.md.
- 2026-11-02 — **10-01 R1 (triggerless `PopoverContent anchor` focus guard): not needed, not fixed.** The palette
  anchors no popover (it is a Dialog with an inline list), so the primitive defect stays where ticket 17 left it.
  **Menu axe `region`:** the palette's portal is a `role=dialog` and passes `region`; the open-`DropdownMenu` case is
  recorded in DESIGN.md § DropdownMenu as the accepted narrow exception the 17 audit proposed (no landmark injected).
- 2026-11-02 — **Wording decision:** the Work list's "Remove all skills from tools…" ships as **"Disable every skill on
  all harnesses…"** (keywords "remove all undeploy off" find it). It runs `run("disable", "all")` — D8's library-wide
  undeploy and its count confirmation — and "Remove" already means Delete in this app (the rail's Remove…), so the
  literal wording would have promised a Delete.

- 2026-11-02 — orchestrator, before merge: (1) the chord is ⌘K on Apple platforms and Ctrl+K elsewhere — never both (Ctrl+K in a macOS text field is kill-line; `isPaletteChord` in `shell/CommandPalette.tsx`); (2) `autoHighlight` instead of `"always"`: nothing is highlighted on open, so ⌘K → Enter with no query is a no-op and cannot repeat the last Update all; the first match highlights once a query is typed; ↓ reaches a recent. Primitive tests updated (a typing helper carrying `inputType`, which `autoHighlight` keys off; 8 passing); DESIGN.md § CommandPalette says so.

- 2026-11-02 — **done.** Astra D31 audit (`round19/review/ticket-23-audit.md`): PASS-WITH-FINDINGS, all ten checks pass (keyboard transcripts both themes/sizes, axe 0, contrast as DESIGN.md, no motion in either mode). Orchestrator fixed the three P2s: F1 docs synced to the final behaviour (DESIGN.md § CommandPalette, the fold's comment and test title, the completion-callback comment); F2 an option's label is its name (`aria-labelledby`) and its hint/refusal its description (`aria-describedby`), test pins it; F3 the chord predicate is the pure `isPaletteChord(event, apple)` + `isApplePlatform` in `lib/commandPalette.ts` with tests (Apple ⌘-only, elsewhere Ctrl-only, extra modifiers rejected). Carried, not done: the auditor's deeper primitive-harness regressions (an enabled mutation as first recent; nested-layer/confirmation focus handoffs) — fixture transcripts cover them today.
