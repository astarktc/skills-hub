# 11 — My Skills in the new world: shell, header + exception summary, filter bar + Issues chip, rows + strip, group headers + Deploy all, singleton fold, empty state

Status: ready-for-agent
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

(child fills in)
