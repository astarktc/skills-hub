# 11 — My Skills in the new world: shell, header + exception summary, filter bar + Issues chip, rows + strip, group headers + Deploy all, singleton fold, empty state

Status: implemented
Blocked by: 02, 07, 08, 10
Spec: `.scratch/round19/spec.md` — D22, D23; direction contract § FIRST VIEWPORT / STORY; absorbs BACKLOG **#49**
(banner + chip) and **#39** (group-level Deploy all). Sources: `critique-astra` F02, F07, F10, F12, F18;
`critique-opus` P0-2, P1-7; `audit.md` My Skills rows. D9: first surface migrated; runs alone on `src/components/skills/`.

## Work

Read the direction contract and `DESIGN.md` (ticket 07) first; read the `impeccable` skill's build guidance; judge the
round-18 prototypes (`round18/prototypes/tracker-list/`, `patch-bay/`) as references, not sources to copy.

- **Shell**: left sidebar 200 px (My Skills · Explore · Projects; theme, shortcuts, settings at the foot; every control
  named; `aria-current` on the active area); header row: title + count, **exception summary** from the health fold
  ("3 failed targets · 3 stale · View 5 skills with issues" → applies the Issues chip), Refresh all (secondary), **Add
  skill** (the one primary). Progress placeholder region for ticket 14. The `Header.tsx` top tabs go.
- **Filter bar**: search (`/` focuses; associated label; placeholder passes contrast), chips **All · Issues · Edited ·
  Disabled**, Grouped/Flat toggle, sort, view mode — through the list lens (ticket 08). The "Uninstall from tool
  directories" button and the "Auto-sync" toggle leave the toolbar (auto-sync lives in Settings; disable is ticket
  13's verb).
- **Rows** (36 px, list view; grid is 19b): checkbox · name (+ Edit pencil when overridden) · description · **exception
  marks after the name only** (skill-level from the fold: source missing, central missing, update failed, Edit
  conflict, delete incomplete; target-level: N failed, N stale) · **Synced strip** — one mark per Tool of the effective
  target set: hollow = not synced, filled = synced, × = failed, a distinct *disabled* mark; near-invisible at N/N; a
  central-missing skill never renders a filled mark; each mark carries an accessible name ("Amp: failed — permission
  denied") · N/M · scope (globe / folder+N projects) · **row switch = Disable** (ticket 13 wires the verb; render it
  disabled-looking until then is *not* acceptable — coordinate: this ticket renders the switch through the door with
  `disable`/`enable`; ticket 13 adds bulk) · overflow menu (Update, Change source…, Restore, Detach, Remove). The
  strip is **read-only** (D22); clicking the row focuses it and opens the inspector (ticket 12 renders the inspector;
  until it lands, focus + hash update and the existing detail view opens in its place).
- **Group headers** (sticky): source name · count · state summary · **Deploy all** (#39: sync every skill of the source
  to the effective target set through the door's `deployGroup`, one batch, one ask). **Singleton fold**: sources with
  one skill fold into "Individual sources" with a muted inline source label. Repo pill repetition inside groups goes.
- **Empty and first-run states** (F18): the shell stays; one clear next step (Add from GitHub / local folder;
  Explore secondary); with discovered groups, "Review N discovered skills" is the first task, no amber branding. The
  D6 "rare delight" moment for a completed large import is designed here and implemented when Import migrates (21).
- **Migration rule**: `SkillsList.tsx`, `SkillCard.tsx`, `FilterBar.tsx`, `Header.tsx` (or their replacements)
  reference no `App.css` class; they join `migrated` in `styles-baseline.json`; the ceiling is lowered by the rules
  deleted from `App.css`.
- Tool labels everywhere (X4/F12): never a raw key in a visible or accessible string.
- **Collapse** (D23): measure the row with the strip at 36 px and set the breakpoint (~1200) at which the sidebar
  becomes an icon rail; record the number in `DESIGN.md`. (The inspector sheet is ticket 12; the Projects select is 17.)
- i18n keys for every string; `resources.test.ts` stays green.

## Gate (D31)

Child audit (Astra) in `preview_*` on `rich` and `failures`, both themes, 1280×800 and 960×640: keyboard transcript
(`/`, Tab order, `j/k` if ticket 13 landed, row focus → hash), axe clean, measured contrast of every rendered pair,
strip marks distinguishable in grayscale, `docx` shows no filled mark, exception summary counts match the fixture,
reduced-motion CSS coverage for anything that moves. `npm run check` green.

## Result

Implemented 2026-09-29 (child, Opus 5.5; one provider 429 mid-run, resumed on the intact tree). Nothing committed.
No primitive, `ui/**`, `src-tauri/**` or fixture file changed; no token *value* changed.

### Files

- **New**: `src/components/shell/AppSidebar.tsx` (sidebar / icon rail), `src/components/skills/LibraryHeader.tsx`
  (header row + exception summary + ticket-14 progress slot), `SkillRow.tsx` (row + card anatomy), `SyncedStrip.tsx`.
- **Rewritten**: `SkillsList.tsx` (groups, singleton fold, empty / first-run / no-match states), `FilterBar.tsx`
  (search, chips, Grouped/Flat, Sort-and-view menu).
- **Deleted**: `Header.tsx` (top tabs), `SkillCard.tsx`, `InvocationModeBadge.tsx` (its only user; the Edit pencil
  replaces it).
- `src/App.tsx`: shell markup (sidebar + `<main>`), lens with `foldSingletons: true` + `filters: SKILL_FILTERS`, the
  exception fold, strip Tools (effective target set, catalog labels), per-surface selection wiring, detail
  focus-in / focus-return (row → detail's first control; Back / Escape / history back → the row), Add-from-GitHub /
  folder intents, `discoveryLeads`.
- `src/lib/skillPresentation.ts`: pure folds `syncedStrip`, `STRIP_MARK_NAME_KEY`, `skillHasIssues`,
  `libraryExceptions`, `SKILL_FILTERS`, `groupState` (all read `skillHealth`; nothing composes health by hand).
- `src/hooks/useListLens.ts`: named filters (`filters`, `filter`/`setFilter`, `filterCounts`).
- `SettingsPage.tsx`: Auto-sync switch (moved from the toolbar; its pane is ticket 20's).
- `src/styles/theme.css`: `--breakpoint-shell: 75rem` (non-colour scale; `shell:`/`max-shell:` variants).
- `src/i18n/resources.ts`: new `shell.*`, `library.*`, `strip.*`, `settingsAutoSync*` appended at the end of `en`;
  15 keys the deleted surfaces alone used removed (`moreTools`, `viewMode`, `searchPlaceholder`, `discovered*`,
  `reviewImport`, `skillsEmpty`, `toolNotDetectedTooltip`, `autoSyncToggle`, `unsyncAll`, `(un)syncSkillTooltip`,
  `syncTarget.*`, `skillHealth.line`).
- `DESIGN.md`: Non-colour scale row, Layout (breakpoint + measurement), new § My Skills (shell, header, filter
  bar, row, group header, card view, **The Synced strip** table, Empty and first-run, the delight moment), Motion
  count. `scripts/styles-baseline.json`: 7 files in `migrated`, ceiling lowered.
- Tests: `skillPresentation.test.ts` "My Skills folds (ticket 11)" (strip order / every state / out-of-set Tool /
  central-missing never filled / skill-grain disabled / pair disabled / N/N; exceptions; chips = summary; group
  state), `useListLens.test.ts` named filter + counts.

### Row anatomy (36 px, `h-row`)

checkbox (surface `selection.skills`, `toggle`) · name button (`data-skill-row`, the Tab stop) + Edit pencil (only
when overridden → `editInvocation`) + exception marks (fold rows, skill-level then target-level; `disabled` /
`targets_disabled` silent — strip + switch say them; `data-skill-health` on the group) · description (Flat and
"Individual sources": muted mono source label first) · Synced strip + N/M · scope (globe when a target is live) ·
Switch `sm` = skill-grain `disable` / `enable` through the door · overflow DropdownMenu: "Fix" group (the fold's
repairs via `actions.repair`), Update, Change source…, Detach (when `detachable`), Edit invocation…, Remove. Row
click (outside controls) = `focus({kind:'skill'})` → hash `#/skills/<id>` → the existing detail in place. Columns
`32 · 1.5fr · 1fr · (marks×16+40) · 24 · 40 · 32`; the description drops below a 768 px list pane (container query).

### Strip vocabulary (read-only, D22; each mark `role="img"`, named)

| state | shape | colour | name |
|---|---|---|---|
| synced | filled | muted-foreground (`input` when N/N) | "Cursor: synced" |
| not synced / pending | hollow | input | "Amp: not synced" / "…not synced yet" |
| stale | half-filled | stale | "Codex: stale — the copy differs from the skill" |
| failed / missing | × | failed | "Amp: failed — the last sync did not complete" / "…the first sync…" / "…the deployed copy is gone" |
| broken (central missing) | × | failed | "Pi: broken — the central copy is missing" |
| disabled | slashed ring | faint | "Codex: disabled" / "…disabled — the skill is off everywhere" |

Undetected Tools read "Windsurf (not detected): …". Fixture proof (`failures`): `docx` = 7 × broken + Windsurf hollow,
0/8, **no filled mark**; `noisy-linter` = Cursor + Codex disabled; `retired-playbook` = every mark disabled, switch
`aria-checked=false`.

### Breakpoint, ceiling

- **1200 px** (`--breakpoint-shell: 75rem`): at 1200 the name column is 347 px = the widest name+marks cell in the
  fixture (`scratchpad`, two marks); narrower would truncate, so the sidebar becomes a 56 px rail. 1199 → name 433;
  960 → pane 874, name 289 (only a two-mark row truncates; title carries it); no horizontal scroll at 1280/1200/1199/960.
- `App.css` ceiling **3038 → 2316** (116 rules only the deleted surfaces used; `transition: all` 22 → 14).

### D31 first half (fixture 5175)

- **Keyboard** (headless `agent-browser --session r19-t11`, trusted keys; `failures`, both themes × 1280×800 and
  960×640, identical sequences): `/` → search (focus-visible, 2 px solid ring) · type `docx` → 1 row · Escape clears ·
  Shift+Tab ×4: Add skill → Refresh all → Configure Tools → "View 11 skills with issues" · Enter → Issues chip
  pressed, 11 rows · `/`, Tab → All, Space → 25 rows · Tab: Issues → Edited → Disabled → Grouped → Sort and view ·
  Enter opens the menu (radio items), ArrowDown, Escape → trigger · Tab: Review 9 discovered → group link → docx
  checkbox · Space / Space (checked true/false) · Tab → name · Enter → `#/skills/skill-004-docx`, focus on the
  detail's Back · Escape → `#/skills`, focus back on the docx name · Tab → switch → overflow · Enter → "Restore —
  Central copy missing | Change source… | Remove" · Escape → trigger · Tab ×3 → frontend-design switch, Space →
  off (focus stays), Space → on → the one overwrite ask opens (fixture's occupied folder). Every stop
  `focus-visible` with a `solid 2px` ring (29/29 per run). Separately (`rich`): Enter → detail, Enter on Back → row;
  history Back → row; history Forward → detail's Back; the sidebar shortcuts popover and theme menu open/close by
  keyboard with focus returning to their triggers. Transcripts: `kt-{dark,light}-{1280x800,960x640}.txt`.
- **Contrast** (rendered, composited, `contrast.jsonl`; rich + failures + empty + first-run, both themes, both
  sizes): **0 text pairs < 4.5**; lowest 4.68 (white on primary, dark) and 4.89 (failed badge, light). Non-text: marks
  ≥ 3.32 (hollow/quiet on raised) up to 8.31; checkbox / search / off-switch edge 3.90 / 3.78; chip pressed edge
  7.05 / 5.71; current-nav icon 6.01 / 5.04; ring ≥ 6.01 / ≥ 5.04. Table in DESIGN.md § The Synced strip.
- **Grayscale**: `failures-dark-1280x800-grayscale.png`, `…-bottom-grayscale.png` — filled / hollow / × / slashed
  distinct.
- **Summary counts**: `failures` "7 failed targets · View 11 skills with issues" = tdd 2 + using-git-worktrees 3 +
  json-canvas 2; Issues = deprecated-helper, pr-review, tdd, using-git-worktrees, api-style, docx, json-canvas,
  release-train, local-notes, scratchpad, shell-aliases (no stale global targets in the fixture, so no stale part).
  `rich` "2 failed targets · View 1 skill with issues".
- **Reduced motion**: the only moving element in `#root` is the Switch thumb (transform, 120 ms); under
  `prefers-reduced-motion: reduce` it computes `transition: none 0s`. Nothing new animates.
- Screenshots (`.scratch/round19/evidence/11/`): `{rich,failures}-{dark,light}-{1280x800,960x640}.png`,
  `failures-dark-1280x800-bottom.png`, grayscale ×2, `empty-{dark,light}-1280x800.png`,
  `first-run-{dark,light}-1280x800.png`, `first-run-light-960x640.png`, `rich-dark-1280x800-shortcuts.png`.
- Not run here: axe (the Astra audit half).
- **Gate**: `npm run version:check` → Version OK (1.2.18); `npm run check` → exit 0 (lint, styles:check "2316
  (ceiling), 7 migrated", vitest 34 files / 762, build, rustfmt, clippy, cargo test 792).

### Decisions / deferrals

- Library-wide Disable left the toolbar with **no** replacement path (no unconfirmed library-wide verb reachable);
  ticket 13's select-all → rail → count confirmation owns it. `run("disable","all")` stays in the door.
- Row switch = skill grain; the door's Enable raises its one overwrite ask when a folder is in the way (seen).
- Opening a row moves focus to the detail's first control and every exit returns to the row — an interim until
  **12**'s inspector (heading focus, sheet below 1200 — the `max-shell:` variant exists).
- **13**: the rail, `x`/`Esc`, `j/k`, select-all (checkboxes already write `selection.skills`).
- **14**: render progress into `[data-slot="header-progress"]` in `LibraryHeader`; `LoadingOverlay` still renders.
- **15/19b**: the card grid (today the same anatomy on a card tile, no clipping).
- **20**: Auto-sync gets its Settings pane (the switch sits in the legacy page for now).
- **21**: the import delight moment (designed in DESIGN.md § Empty and first-run).
- Imported skills still group under "Local" (`groupSkillsByRepo`'s rule, not this ticket's) — worth a "Managed here"
  group key when grouping is next touched.
- `invocationEdit.{overridden,overrideTooltip,conflictTooltip}` and `disabledSkips.deployDisabledPairs` are now
  unused; kept for 12's inspector.

## Comments

- 2026-09-29 — D31 independent audit of `353e0fe`: **fix-then-pass; gate not accepted**. Report: `.scratch/round19/review/ticket-11-audit.md`; fresh fixture evidence: `.scratch/round19/evidence/11-audit/`. **P0 A1:** Disable → Enable → overwrite ask → Escape leaves a visible closed/disabled dialog, `#root` inert and focus on BODY (four keyboard runs; corroborated at default latency, reduced motion off). Keep the overwrite popup identity stable when pending clears; the keyed `OverwriteAsk` currently remounts its DialogContent on close. **P1 A2:** repository links’ 2px focus outlines are clipped by the parent `h2.truncate`; computed ring presence is not painted-focus proof. **P1 A3:** at 960, scratchpad’s two nonshrinking badges leave its 70px name only 12px in T3 / 30px headless. **P1 A4:** uncontained sr-only strip counts create an outer document scrollbar (failures 1192px document in 640px viewport), allowing the header/sidebar to scroll away; positioning the count wrapper in a DOM-only experiment removes it. Colour measurements and all 16 axe runs pass; collapse, strip state/shape semantics, empty/discovery priorities and styles guard pass. F10’s bulk rail and F12’s other boundaries remain explicitly assigned to later tickets. No source edits; concurrent ticket-09 fixes excluded. Ticket 11 needs these gate repairs and a targeted re-audit before D31 closure; see the report for exact file:line fixes and verification limits.

- 2026-09-29 — from ticket 05: the `first-run` fixture scenario now seeds two managed skills (for the managed-name import case), so it is no longer an empty library. Design the empty state on `?scenario=empty`; the "discovered groups present" first-run state on `?scenario=first-run`.
