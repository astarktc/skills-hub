# 19 — Docs, CHANGELOG, AGENTS.md invariants, `version:set 1.3.0`, release; operator VoiceOver smoke

Status: ready-for-agent
Blocked by: 15, 16, 17, 27, 28
Spec: `.scratch/round19/spec.md` — D31, D32; § Closure checklist (19a).

## Work

- **CHANGELOG 1.3.0** block: the new world (tokens, primitives on Base UI, `App.css` ratchet), My Skills (rows, strip,
  exception summary, Issues chip, group Deploy all, singleton fold), inspector, selection + bulk rail, Disable as the
  durable un-deploy (and the removal of "Uninstall from tool directories"), persisted skill health, composite
  headlines and `declined`/`cancelled`, notifications by run, header progress, Add as one surface with one ask and
  picker search, Projects keyboard model and cell states, coexistence, min window, pre-paint theme, EN-only copy
  pass on touched strings. Migration notes: schema additions (`disabled`, health columns), the three unsync commands
  replaced, `install_*_selection` replaced.
- **AGENTS.md**: every invariant the tickets changed is re-read and made true (styling bullet, Dialog primitive,
  state-management bullet with the new modules, the fan-out bullet with the install batch, the sync-target mutation
  bullet with disable/enable, Commands with `styles:check`, the removed "(round-19 world ticket lands …)" clause);
  README supported-tools table untouched unless adapters changed (they did not).
- **CONTEXT.md**: *Disabled*, *Skill health*, *Inspector*, *Selection*, *Run report* as terms; the removal of
  "unsync" as an operator verb.
- `docs/adr/0006`: § 7 marked accepted (D2); `DESIGN.md` linked from README's contributor section.
- `npm run version:set 1.3.0`; `npm run version:check && npm run check`; commit; `git fetch && git rebase
  origin/main` (take origin's `featured-skills.json` if it conflicts); push `main` — the push is the release.
  Watch `auto-tag.yml` → `release.yml` (five targets + `updater.json`); record run ids here.
- **Operator smoke list** (D31 human half), written here for Alex: install the build; both themes; the six core flows
  keyboard-only with VoiceOver; `prefers-reduced-motion: reduce` on; window refuses <960×640; Disable survives
  Refresh on the real library (one skill, one Tool, reversible); select-all → Disable on all Tools shows the count and
  is **cancelled**; Add of a repo with a collision raises one ask.

## Result

Docs half done 2026-10-01 (child, Opus 5.5), uncommitted over `9272e23`. The commit, rebase, push and release watch
are the orchestrator's.

### Files

- `CHANGELOG.md`: the `[Unreleased]` entries of tickets 01–05 are folded into **`## [1.3.0] - 2026-10-01`** (an
  empty `[Unreleased]` stays above it). Three stale claims were corrected:
  - "Uninstall … now disables every skill": the action is gone;
  - `install_*_selection` "kept until the Add surface is rebuilt": they are deleted;
  - Import's Replace / remove-originals as a per-run choice: the UI for both is ticket 21 in 1.3.x, so it is listed under
    Known limitations.

  Sections: New look / My Skills / Changed / Fixed / Removed / Known limitations / Migration notes / Internal, written
  from tickets 01–17 Results and Comments, the audits' accepted deferrals and the `v1.2.18..HEAD` bindings diff (8
  commands added, 5 removed).
- `AGENTS.md`:
  - Sync-target mutation: Sync this, `resolve_project_collisions`, and `reveal_path`'s admission
    (`admit_reveal_path`).
  - The reporter bullet: the `skills` subject, `activity`, `HeaderProgress` (no overlay), `focusableWhenDisabled`,
    `returnLostFocus`, and `setError`'s real scope (the sync and Add worlds).
  - Project mutations: `resync_project_assignment` answers `{ view, report }`; `useMatrixFocusReconciliation`.
  - A new bullet: Add intents (`open` / `install` / `retry` + the `add` view) and the file browser
    (`useSkillDocument` + `skillFiles`).
  - The test exception cites ADR-0006 §7.
  - The styling bullet gives the live baseline count (1108 / 36).
  - A new layer-stack bullet: modal and floating layers, `claimDismissal`, inert, return chains (`handOverReturn` /
    `returnChains`), the menu-opener return, `interceptEscape`, `placement="sheet"`, and keep-the-shell-mounted.
  - The fan-out project line: `declined`, `skipped_projects`, `project_folder_exists`, `create_dirs_within` /
    `TargetParents::WithinRoot`.
  - The presentation bullet: "Row" replaces "Card", plus `bulkCounts`, `notificationRuns`, `projectPresentation`,
    `addFlow` and `overwriteAsk`.
  - The door-handle consumers list.
  - Workflow step 2: "i18n (EN)" (ZH left in 1.2.18).
- `CONTEXT.md`:
  - New terms: *Overwrite ask*, *Run* (run report), *Header progress*, *Project folder missing*, *Inspector*,
    *Selection* and *Bulk rail*.
  - *Disabled* records that unsync is no longer an operator verb.
  - *Sync target* points at the Overwrite ask (its definition moved there, not duplicated).
  - *Notification* belongs to a Run.
  - *Project sync report* gains `skipped_projects` / `declined` / skipped.
  - *Former skills dir* no longer says "unsynced".
  - *Disabled* and *Skill health* already existed (tickets 01 / 02); they were re-read and still hold.
- `docs/adr/0006-styling-architecture.md`: §7 is marked **accepted** (round-19 spec D2, shipped in 1.3.0). The
  heading, "Proposed" and the "until ticket 09 accepts" sentence are rewritten.
- `README.md`: the Contributing section links `DESIGN.md` (+ ADR-0006). The supported-tools table is untouched:
  `git diff 2eb1684..HEAD -- src-tauri/src/core/tool_adapters/mod.rs` and the same diff over `v1.2.18..HEAD` are both
  empty.
- `DESIGN.md`: the stale `useOverwriteConfirmation` sentence (the hook was deleted in ticket 08) now describes the
  surface-state asks.
- `npm run version:set 1.3.0` updated the five locations.

### Gate (fresh)

- `npm run version:check` → `Version OK (1.3.0)`.
- `npm run check` → exit 0:
  - eslint clean;
  - `styles:check ok — App.css 1108 counted lines (ceiling), 36 migrated file(s), prose layer scoped, no raw colours;
    self-test passed`;
  - vitest **46 files / 996 tests passed**;
  - `tsc -b` + `vite build` ok;
  - rustfmt ok, clippy `-D warnings` ok;
  - `cargo test` **827 passed**;
  - bindings unchanged.
- `grep -n "round-19\|until ticket\|v-next ticket" AGENTS.md` → three lines, all decision citations: round-19 spec D2
  for the primitive-test exception, and v-next tickets 36 / 38 for the Cursor capability. Kept on purpose.

### Release run ids

- `auto-tag.yml`: TODO (orchestrator)
- `release.yml`: TODO (orchestrator), covering the five targets and `updater.json`.

### Operator smoke list (D31 human half, for Alex)

Install the 1.3.0 build from the release (not `tauri:dev`), then:

1. Check both themes (Settings → theme; also relaunch in light: no dark flash on launch). The library opens in the new
   world. No skill wears a healthy mark. A `docx`-class skill (central copy gone) shows its health.
2. Run the six core flows **keyboard-only with VoiceOver on**:
   - My Skills: browse, search with `/`, open the inspector;
   - Disable / Enable one skill;
   - select several (`x`), then use a bulk-rail verb;
   - Add from a repo;
   - Projects: select a project with the arrows, toggle a cell, Sync this;
   - the Notifications panel: View report, then Show / Retry.
3. Turn `prefers-reduced-motion: reduce` on (System Settings → Accessibility → Display → Reduce motion). The switch,
   the bulk rail, the strip mark lighting, the progress bar and toasts should appear without moving.
4. The window **refuses to shrink below 960 × 640**.
5. **Disable survives Refresh on the real library.** Use one skill and one Tool (reversible): Disable it on that Tool
   from the inspector, run Refresh all, confirm it stays disabled, then Enable it again.
6. **Select-all → Disable on all Tools.** The confirmation shows the count (skills · Tools · Sync targets · project
   assignments). **Cancel it**; nothing should change.
7. **Adding a repo with a collision raises one ask.** With two folders in the way, the ask lists both, each with its own
   Keep / Overwrite.
8. **Executable bits survive Refresh (ticket 28), on both Macs.** Refresh `impeccable`, then `ls -l
   ~/.skillshub/impeccable/scripts/impeccable` shows `-rwxr-xr-x`; run a second Refresh and it still does. Never fix it
   by hand — a hand `chmod` would mask the bug.
9. **A refused click says why (ticket 27).** Start a Disable on one row, then immediately click another row's switch:
   the toast reads the reason ("Disable is running — wait for it to finish" / "Nothing was changed.") and the second
   switch does not move. Known limit (BACKLOG #79): the same refusal inside the inspector sheet is seen, not read by
   VoiceOver.

What the D31 fixture audits could not verify in a browser (only the operator can):

- **VoiceOver announcements.** axe and the DOM checks proved the roles, names and live regions, but no screen reader was
  ever run. Listen for:
  - the arrival status line in the Notifications panel;
  - the header progress's live region;
  - refusal reasons read as descriptions;
  - the matrix cell names (skill · Tool · state · "also global");
  - the strip marks' names.
- **The native minimum-window refusal.** `tauri.conf.json` sets `minWidth 960` / `minHeight 640`. The audits emulated
  viewports, but no real macOS window frame was ever resized.
- **Reduced motion in the real app.** The audits emulated the media query in Chromium. The release runs in WebKit,
  under the OS setting.

(the orchestrator adds the smoke outcome here)

## Comments

- 2026-10-01 — from ticket 17 (CHANGELOG / README inputs): **Sync All skips a project whose folder is missing** —
  one line "archived-app: folder not found — Update path or Remove" instead of a failure per assignment, and it no
  longer recreates the missing folder's Tool dirs (it used to: `sync_engine` creates parents). New per-cell **Sync
  this** (`resync_project_assignment`) for stale / missing / failed assignments. The Projects surface: keyboard project
  selection, Tool labels + "not detected" in the matrix, cell states as mark + word (legend), "also global" cells
  enabled (D17), Assigned · Issues chips, a header select below 1200 px, a "statuses may be out of date — Refresh view"
  line, Remove project closes with a kept copy shown as a failed cell (ADR-0002). Project batch toasts read as the count
  line; a disabled skip now tones a project re-sync "skipped" (was success).
- 2026-10-01 — docs child (Opus 5.5): the docs half is done and uncommitted over `9272e23`; see § Result. That covers
  the CHANGELOG 1.3.0 block, the AGENTS.md invariants, the CONTEXT.md terms, ADR-0006 §7 accepted, the README → DESIGN.md
  link, and `version:set 1.3.0`, with `version:check` and `check` green. Notes for the orchestrator:
  - The CHANGELOG date is **2026-10-01**. Re-date it if the push lands later.
  - The CHANGELOG states the Import limits honestly (Replace / remove-originals UI and the Import ask are ticket 21).
  - Leftovers I did not touch: README "Key Features" still describes the old "Skill detail view" and its screenshots
    show the old UI; `src/i18n/resources.ts` still holds dead `unsyncSkill` / `unsyncAllComplete` keys (source, out of
    scope here); `ui/confirm-dialog.tsx`'s comment still names `useOverwriteConfirmation`.
  - Status stays `ready-for-agent` until the release half runs.

- 2026-10-27 — **Blocked by 27 and 28 as well** (operator decision after ticket 18's think-aloud). 27 makes a refused
  click show its reason; 28 keeps executable bits on the GitHub Contents path (UNI-163). Each adds a CHANGELOG line to
  the 1.3.0 block.
  - Before pushing, change the block's date to the push day; it still reads 2026-10-01.
  - Smoke list addition, on both Macs: Refresh `impeccable`, then check that its `scripts/impeccable` in the central
    repo shows `-rwxr-xr-x` and still does after a second Refresh.
- 2026-10-27 — **27 and 28 landed and were audited** (orchestrator): 27 `c8b727d` + audit fix `d7eab03` (a held
  Checkbox/Switch label click also refuses visibly); 28 `7ad4c17` + audit fix `b00e17c` (encoded trees ref for slash
  branches, umask-proof assertions). Both CHANGELOG lines are in the 1.3.0 Fixed block. Smoke items 8 and 9 added
  above. 27's audit left two open findings as BACKLOG #79 (modal toasts inert to screen readers, P1 — operator decides
  whether it holds the push) and #80 (toast stack covers the header). Local SHAs only; final SHAs after the push.
