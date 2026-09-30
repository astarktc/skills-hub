# 13 — Selection, bulk rail and verbs: per-surface selection, `x`/`Esc`, bulk Disable/Enable/Sync/Assign/Delete with count confirmations, Delete dialog → report, `j/k`

Status: implemented
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

Implemented 2026-09-30 (child, Opus 5.5). Nothing committed. Evidence: `.scratch/round19/evidence/13/` (gitignored;
drivers `driver.ts`, `gate.ts`, `delete.ts`, `extras.ts`, `a11y.ts` beside their JSON/PNG; headless
`agent-browser` 0.31.1, trusted keys in every primary transcript; fixture reads/writes marked `supplemental`).

### Files

- **New** — `components/skills/BulkRail.tsx` (the rail), `components/skills/useSkillListKeymap.ts` (+ test: My
  Skills' one keymap and the pointer-selection anchor), `modals/CountConfirmModal.tsx` (the `confirmCount` dialog),
  `lib/bulkCounts.ts` (+ test: the pure count folds — `disableCounts`, `enableCounts`, `syncCounts`, `updateCounts`,
  `deleteScope`, `deleteTotals`).
- **Rewritten** — `modals/DeleteModal.tsx` (D16: one Dialog, confirm → report; migrated, off the legacy classes).
- **Changed** — `hooks/useSkillActions.ts` (the confirm rules, bulk Delete, `DisableScope::Skills` Enable batch fold),
  `hooks/useSurfaceState.ts` (`confirmCount` kind, `deleteSkill`/`assignProject` over `skillIds`, `report` on the
  Delete layer, `updateDialog`, multi-id reconciliation), `hooks/useSkillLibrary.ts` (`pendingDelete`,
  `countConfirm`, `assignProjectSkills`), `lib/reportOutcome.ts` (`deleteOutcome` over settlements,
  `deleteReportRows`, count lines for bulk Disable / Enable / Delete), `lib/actionLifecycle.ts` (`skills` pending
  subject; `focusAfter` steps past every removed row), `lib/platform.ts` (`revealPath`), `commandError.ts`
  (`REVEAL_PATH_FAILED`), `App.tsx` (keymap, rail, the three dialogs, placements → `assignmentsOf`),
  `SkillRow.tsx` (iris selection edge, shift-click, `data-skill-item`, the switch's inline Enable popover),
  `SkillsList.tsx` (group-header checkbox), `FilterBar.tsx` (its `/` handler moved into the keymap; `data-skill-search`),
  `AssignProjectModal.tsx` (one skill or a selection), `AppSidebar.tsx` (shortcuts list), `ui/confirm-dialog.tsx`
  (`initialFocus`), `ui/popover.tsx` (`anchor`), fixture (`revealPath` handler, `state.revealed`, dev-only
  `window.__fixture`), `i18n/resources.ts` (`revealPath`, `bulk`, `deleteDialog`, `deleteReport` appended at the end
  of `en`), `DESIGN.md` (§ Selection, the bulk rail and the Delete dialog; ConfirmDialog `initialFocus`; Popover
  without a trigger; rail motion; selected-row edge), `scripts/styles-baseline.json` (3 files appended; ceiling
  unchanged at 2076 — the legacy `delete-*` rules are still used by `RemoveProjectModal`, ticket 17).
- **Rust** — `commands::reveal_path` (`#[tauri::command] #[specta::specta]`, in `collect_commands!`),
  `core::log_reveal::path_reveal_target` (select the item in its folder while it exists — `symlink_metadata`, so a
  dangling link too — else open its nearest existing ancestor; `None` on a vanished volume), `SignalError` /
  `CommandError::RevealPathFailed { path, detail }`; 4 tests in `core/tests/log_reveal.rs`; bindings regenerated.

### Decisions

- **The confirm-count ask** is a dialog kind, not an ask promise: `{ kind: "confirmCount", verb, skillIds }`
  (camelCase like every other kind). The door opens it for **Disable / Enable / Update over several skills (or
  "all") without `confirmed`**, and for **Sync to Tools without `tool` or `confirmed`, even for one skill** (its
  confirmation is the Tool pick). Its Confirm re-runs the verb with `confirmed: true` (+ `tools`); the door closes it
  once the verb settled. One skill's Disable / Enable / Update never asks (D8). Callers that already confirmed pass
  `confirmed`: the header's Refresh all, the new-Tools notice, the health repair "Deploy". Counts are derived at
  render from the selected skills + the project world's held placements (read explicitly as a Disable / Enable /
  Delete confirmation opens; "Reading project assignments…" until they land).
- **Assign to project…** widened the existing picker (`assignProject` over `skillIds`): it shows per project what
  assigning adds ("Adds 2 skills to Claude Code, Pi") — the picker is the count-bearing confirmation; its answer is
  the door's `projectId`.
- **Delete batch shape**: `delete_managed_skill` is per skill, so the door loops **inside one `runAction`** (subject
  `{ kind: "skills", ids }` — rows pend inline, the Delete dialog shows the count line, no progress overlay), catches
  each skill's thrown command as a settlement, and folds **one** report (`deleteOutcome(settlements)`: one sentence
  for one skill, the count line "10 removed · 2 failed" for several). **BACKLOG follow-up: a multi-skill delete
  command** (one backend batch per operator action) — not invented here.
- **The Delete dialog is one Dialog, two states**; the report is the `deleteSkill` layer's `report` (settlements),
  set with `updateDialog` *before* the reload so the confirmation never re-renders in between; rows come from
  `deleteReportRows` (kept skills first; "Removed from …"; per kept path: Tools · path · `describeCommandError` ·
  Reveal in Finder). Retry re-runs Delete for the kept skills still in the catalog. Focus: Cancel on open; the
  pressed Remove / Retry keeps focus while pending (`pending` = focusable-when-disabled); the report's heading
  (`tabIndex=-1`) takes focus when the dialog turns into the report; a completed Delete closes to D21's next row.
- **Reveal in Finder** goes through `platform.revealPath` → the new `reveal_path` command (`open_log_folder` reveals
  only the log folder, with no path argument). The fixture's adapter records the path (`state.revealed`).
- **The rail** is drawn at the foot (`order-last`) but comes first in the list column's DOM, so `/` then Tab reaches
  it (~7 stops) and Shift+Tab from the top rows does; it rises on entry only (`@starting-style`, 120 ms opacity +
  translate, `motion-reduce:transition-none`), leaves instantly, never covers a row. Rail buttons are
  `focusableWhenDisabled`, so after a verb that made itself inapplicable (Disable → nothing left to disable) focus
  stays on it (D21 "same control"). Clearing the selection from inside the rail moves focus to the row last toggled.
- **One keymap** (`useSkillListKeymap`, called by App): `/` · `j`/`k` · `x` · ⌘A/Ctrl+A (the lens's visible subset)
  · Esc (clear the selection → collapse Files → close the inspector; App's former Escape effect moved in here). Off
  while typing and while any dialog / sheet / popover / menu is in the DOM; it still owns a key a disabled-but-
  focusable control "consumed" to block activation (Base UI prevents default there).
- **Row switch**: Disable never asks; Enable asks in an anchored Popover only when it will sync the skill to Tools
  again, counting the *disabled rows it re-syncs* (`enableCounts`): `retired-playbook` reads "2 Tools" although its
  strip shows 8 slashed marks — 6 of those are "off everywhere" marks with no row, which Enable does not create.
- **Selection mark**: checked checkbox + `raised` + a 3 px `ring` edge (`::before`) — coexists with the focused row's
  1 px inset ring.
- **Toasts → count line** (09's handoff): bulk Disable ("80 disabled · 3 failed"), bulk Enable ("5 enabled · …"),
  bulk Delete ("2 removed · 1 failed"). Left on their sentences: the project folds (`bulkUnassign`, project removal)
  → ticket 17; the single-skill Delete / Enable sentences stay (a batch of one reads as a sentence, like Refresh).

### Verification (fresh, fixture 5175 `failures` / `rich`)

- **D31 gate transcript** (`gate-{dark,light}-{1280,960}.json`, 83 stops each, **0 BODY, every stop `:focus-visible`**;
  the only non-`solid 2px` stops are the legacy progress overlay's popup, ticket 14): `j x j x j x` → 3 selected →
  Shift+Tab ×~17 → rail **Disable on all Tools** → Enter → confirmation, focus **Cancel** ("Disable 3 skills on 3
  Tools — 5 Sync targets are removed and kept disabled; no project assignment changes. …") → Tab → Enter → samples
  0/300/2000 ms: overlay popup → the rail's Disable (kept) → strips: every mark disabled, switches off →
  **Refresh all** (the one overwrite ask from the re-assert answered Keep existing by keyboard) → the 3 rows **still
  disabled** → rail **Enable** → confirmation → Enter → strips re-synced, switches on.
- **Select-all counts vs a hand count** (all four conditions): Esc, `j`, ⌘A → "Disable 25 skills on 8 Tools — 80
  Sync targets are removed and kept disabled; 12 project assignments for these skills are disabled too. … 1 of them
  is already off everywhere." = fixture state hand count `{skills 25, tools 8, targets 80, assignments 12,
  alreadyDisabled 1}` (supplemental read).
- **Delete of the locked `release-train`** (`delete-{dark,light}-{1280,960}.json`, 29 stops, **0 BODY**): row menu
  → Finish removing → dialog "It is removed from Claude Code, Codex, Pi. Projects that carry it: monorepo. Skills
  Hub's copy is deleted.", focus **Cancel** → Tab → Remove → **0 ms: Remove (pending, focused) · 300 ms: Retry
  (pending) · 2000 ms: the report heading** "release-train was not deleted" — "Removed from Codex, Pi. Still in
  Claude Code: /Users/alex/.claude/skills/release-train — Permission denied …" + the monorepo assignment path, each
  with Reveal in Finder; Close · Retry. Reveal → `state.revealed = ["/Users/alex/.claude/skills/release-train"]`.
  Supplemental unlock (`lockedPaths`, `unwritableProjectTools` cleared) → Retry → 0/300 ms Retry (pending) → 2000
  ms dialog closed, focus on the **next row, `retired-playbook`**.
- **Extras** (`extras-{dark,light}.json`, 73 stops, 0 BODY): ⌘A under the search "re" selects exactly the 8 visible
  rows; `j j k` → alias-skill; `x`/Esc; **shift-click range** api-style → design-tokens = the 4 rows (the shift-click
  is a dispatched `MouseEvent{shiftKey}` — agent-browser 0.31.1 drops a held Shift from click/mouse events, measured
  `shiftKey:false`; also pinned by the keymap hook test); **group header** "Select the 4 skills from
  mattpocock/skills" selects the 4, then `mixed` after unchecking one; switch **inline Enable**: Space → popover
  "Enable retired-playbook and sync it to 2 Tools again?", focus Enable → Escape → switch; Space → Enter → enabled,
  focus on the switch; **Sync to Tools…** → pick dialog, focus Cancel, "14 Sync targets for 2 skills on 7 Tools. …"
  → uncheck one → "12 … on 6 Tools. 10 new · 2 already synced (re-synced)" → Escape → rail button; **Assign to
  project…** over 2 → "Assign 2 skills to a project" → Escape → rail button; **bulk Remove** of pr-review,
  prisma-review, release-train → "Remove 3 skills? 8 Sync targets are removed from 5 Tools. 2 projects lose their
  assignments." → report "1 of 3 skills was not deleted" (heading focused) → Close → focus on the rail's Remove….
- **Rail rise**: rAF samples 0 → 1 opacity, `translate 0 8px → none`, `transition-duration 0.12s`, last partial
  frame at 109 / 112 ms (dark / light). **Reduced motion** (`reduced-motion-rise.json`): first frame opacity 1,
  `transition-property: none`, `0s`.
- **axe** (`axe-*.json`): 8 conditions (failures/rich × dark/light × 1280/960, 3 rows selected, rail showing) **0
  violations** each, plus the count confirmation in failures dark/light 1280: 0 violations (the usual
  `color-contrast` incompletes).
- **Contrast** (rendered, composited, every text node in the rail, the selected rows and open dialogs): **0 pairs
  < 4.5**; lowest 5.22 dark / 4.85 light in the rail + rows; 4.68 (white on `primary`, dark, the known token floor) in
  the confirmation. Non-text: selected edge `ring` on `raised` 6.01 / 5.04; checked box 3.49 / 5.04.
- **Gate** (`gate.txt`): `npm run version:check` → Version OK (1.2.18); `npm run check` → exit 0: lint clean;
  styles:check "App.css 2076 counted lines (ceiling), 24 migrated file(s)"; vitest 40 files / 853 tests; build ok;
  rustfmt + clippy clean; cargo test 798 passed.

### Deferrals / follow-ups

- **BACKLOG**: a multi-skill delete command (the door loops `delete_managed_skill` under one action today).
- **Ticket 14**: the progress overlay still shows for the rail's batch verbs (its popup takes focus at 0 ms — never
  BODY) and its legacy title reads "Installing Skills…" for a Disable; the rail and the count line interplay (below).
- **Ticket 23 (⌘K)**: the palette is the fastest keyboard route to the rail's verbs; today it is `/` + Tab.
- The rail offers no "Select all N" affordance beyond ⌘A (and the group checkboxes); rows a filter hides stay
  selected and are acted on (the rail's count says how many).

## Comments

- 2026-09-29 — from ticket 09: the door (`useSkillActions.run(verb, skills | ids | "all", opts)`) exists; a verb called without its confirming option opens its dialog. Count-bearing confirmations need a new ask kind in `useSurfaceState` (04 owned the ask kinds this round; add `{kind:'confirm-count', …}` here). The toolbar's library-wide Disable still has no confirmation until this ticket lands. Ticket 03 left project/removal/disable-all/delete/enable toasts on their old sentences — move them to the count line here with the rail.

- 2026-09-30 — from ticket 12: the inspector is the focused skill (surface `focused`), the rail acts on
  `selection.skills` — they are independent. A checked row shows `raised`; the focused row adds `aria-current` and a
  1 px inset `ring` edge, so both can coexist. `useInspectorTabOrder` moves Tab from the focused row's last control
  into the inspector (and back): if the rail inserts controls into a row, keep the row's last tabbable last, or adjust
  the hook. `run("assignToProject", ids)` without `projectId` opens the picker for **one** skill only (the dialog
  holds a `skillId`); the rail's multi-skill assign should pass `projectId` from its own picker or widen the dialog
  kind. Detach now confirms (`detach` without `confirmed` opens `detachSkill`).

- 2026-09-30 — implementation child: see § Result. For the D31 audit: (1) the legacy Delete dialog's ~300 ms BODY
  interval is gone — the pressed Remove/Retry keeps focus (`pending`), the report heading takes it after; sampled at
  0/300/2000 ms in four conditions; (2) shift-click needs a dispatched `shiftKey` click in agent-browser 0.31.1
  (held Shift is not carried into clicks) — the T3 preview's click has no modifiers either; (3) the rail sits first in
  the DOM (drawn last): check that reading order is acceptable to you, or propose a skip route; (4)
  `window.__fixture.state` (dev-only) lets you unlock `lockedPaths` / `unwritableProjectTools` for Retry and read
  `revealed`.

- 2026-09-30 — independent adversarial D31 audit of `c393037`: **FIX-THEN-PASS**, not accepted yet; see `.scratch/round19/review/ticket-13-audit.md` (evidence `evidence/13-audit/`). Three P1s: **A1** the desktop reader's x/⌘A/j///keys operate on the library; **A2** desktop expanded Files disables its only Escape handler; **A3** unrestricted `reveal_path` can send an existing `.app`/file ancestor to native `open_path` rather than safely reveal it (pure probe only, no native launch). P2 A4: synthetic CANCELLED is folded as failure; current Delete has no operator cancellation path. Four-condition primary flow: 344 stops, Delete: 124 stops, zero sampled BODY; eight axe runs clean; contrast/motion pass; trusted CDP Shift-click independently proved. Rail DOM-before-rows is acceptable; actual role is group + live count. Initial fixture count is 63 targets, post-Refresh 80; both displayed totals agree. The 2-Tool inline Enable count is correct. Fixture tree-shaking and ticket-17 handoff confirmed; ticket-14 overlay debt and BACKLOG #66 remain. Fresh version+check exit 0 (853 frontend / 798 Rust), cargo --all and bindings drift clean. Re-check A1/A2 reader + Escape contexts and A3 resource admission/reveal-only fallback, then affected browser regressions and gate; report has exact scope. No application source edited, no commit.

- 2026-09-30 — fix child for the D31 audit's A1–A4 (nothing committed). Evidence `evidence/13-fix/` (gitignored).
  **A1** (reader shortcuts): `useSkillListKeymap.ts` gains `inListScope` — the row keys (`/ j k x ⌘A`) fire only on
  the My Skills surface (`[data-skill-surface]` on App's wrapper: header, filters, rail, rows) or with nothing
  focused, never inside `[data-inspector]`, `[data-skill-reader]` (new marker on `SkillFiles`' reader region) or a
  dialog; Escape is unscoped and keeps its hierarchy from the reader. Side effect: `j`/`x` from the sidebar or the
  inspector's own controls no longer reach the list (they used to). **A2** (desktop Files Escape): App no longer
  disables the keymap while `filesCover`; it passes `listCovered` and the hook refuses every row key while the list is
  covered/inert but still runs Escape (clear → collapse → close). One owner per key: still the one window handler,
  still off under any dialog/sheet (`layerOpen`), so the sheet's `interceptEscape` and a child dialog keep Escape.
  Hook tests: reader region, reader descendant, inspector heading, sidebar × j/k/x///⌘A (⌘A left un-prevented), Escape
  from the reader, `inListScope`, and the covered-list combination (row keys refused, Escape runs). **A3** (Reveal):
  `core/log_reveal.rs` — `admit_reveal_path` (pure: `~` expanded against home, plain absolute only — relative,
  `~user`, `.`/`..` refused; admitted iff equal to a recorded artifact path or inside the central repo),
  `path_reveal_item` (the path, else its nearest existing ancestor — always **selected**, never opened; replaces
  `path_reveal_target`) and `resolve_reveal_item` over `artifact_removal::recorded_target_paths` (every owned global
  row + project assignment path, resolved exactly as Delete plans them). `commands::reveal_path` resolves home + central
  repo once, calls core, and only ever `reveal_item_in_dir`s (no `open_path`). `REVEAL_PATH_FAILED` gains
  `reason: RevealPathFailure` (`notManaged` / `gone` / `opener`) → binding regenerated (+25/−6, the only diff there),
  `describeCommandError` branch, `revealRefusal.*` EN keys at the end of `en`. 10 new Rust tests (temp dirs + store;
  no opener runs): recorded global + assignment paths admitted/selected, vanished → ancestor selected, `.app` bundle
  and an executable `.command` ancestor selected not opened, dangling link, vanished volume `Gone`, unrecorded /
  `.app` child / bundle / sibling / child-of-target / relative refused `NotManaged`, `~` expansion, `..` refused.
  **A4**: `deleteOutcome` classifies a `CANCELLED` settlement as the `cancelled` kind (counted in the count line,
  neutral tone alone, no error entry, closes the dialog; `deleteCancel.single` for one skill); a single skill that
  threw before planning reads "`name` was not deleted", never "0 targets could not be removed". Fold tests + a door
  test (Delete answered CANCELLED → neutral toast, no error row, dialog closed). No Cancel control added. DESIGN.md
  Keys + Reveal lines updated. **Browser** (fixture 5175, headless, one session at a time): `keys-{dark,light}-1280`,
  `keys-{dark,light}-960` — reader (selected and not): x/j/k/`/` change nothing, ⌘A leaves the library selection as it
  was and native text selection grows (0 → 2714 chars), Escape clears the selection (focus stays in the reader) then
  closes → row; 1280 expanded Files: row keys refused, Escape → Expand files → Escape → row; with a selection: clear →
  collapse → close; Keyboard shortcuts dialog above expanded Files: Escape closes it only; Change source above the
  inspector: Escape closes it only. 960: sheet collapse → close → (then) clear; Change source above the sheet closes
  alone. Gate transcripts `gate-dark-1280` / `gate-light-960`: 95 stops each, 0 BODY, 3 rows disabled (8 marks each)
  → Refresh leaves them → Enable re-syncs; select-all "25 skills on 8 Tools — 80 … 12 …" = hand count. Delete
  `delete-dark-1280` / `delete-light-960`: 37 stops each, 0 BODY, Remove → Retry (pending) → report heading, Reveal
  records `/Users/alex/.claude/skills/release-train`, Retry → next row. axe ×4 (failures dark 1280 / light 960, rich
  light 1280 / dark 960): 0 violations, text ≥ 4.95 light / 5.22 dark, confirmation ≥ 4.68. **Harness finding**:
  agent-browser 0.31.1's `press` (and any CDP key with `nativeVirtualKeyCode`) against Chrome 154 floods the page with
  thousands of phantom trusted keydowns (`x|Numpad6`, `Unidentified|Minus`) that toggle selection, steal focus and reset
  the tab to about:blank — every transcript here uses `keys-cdp.ts` (CDP keyDown/keyUp without a native code;
  ⌘A carries `selectAll`). Re-check drivers must do the same. **Gate** (`gate.txt`): Version OK (1.2.18);
  styles:check 2076 (ceiling), 24 migrated; vitest 40 files / 864 tests; build ok; fmt + clippy clean; cargo test 804
  passed (`--all` too); `GATE_EXIT=0`. **Re-check focus**: A3's admission set (is "recorded artifact path or inside
  central repo" the right fence, and is selecting an ancestor in its parent acceptable UX); A1's scope change for
  sidebar/inspector-control focus; the 960 order (sheet Escape closes before the behind-sheet selection clears).

- 2026-09-30 — independent bounded re-check of committed `98212ef`: **FIX-THEN-PASS**; see `.scratch/round19/review/ticket-13-audit.md` § Re-check and `evidence/13-recheck/`. **A1/A2/A4 closed**: trusted CDP reader/descendant and covered-list keys pass in both themes, desktop clear → collapse → close and modal-sheet collapse → close → clear are correct; one list-key handler, child layers close alone, native ⌘A preserved. **A3 not fully closed**: the unsafe `open_path` branch is gone, but **R1 P1** reproduces central-prefix admission of an unrecorded outside file through an intermediate directory symlink (temp sentinel only, no native opener). **R2 P2**: NUL is admitted then treated as missing, interior `.` contradicts the refusal claim, and trailing slash changes dangling-leaf selection into ancestor fallback; normalize/refuse explicitly and test. Four questions: owned global/project target identity is the right fence (successfully removed rows have no Reveal button; failed owning rows survive); central descendants need R1's physical-traversal check or narrower recorded identities; ancestor selection is acceptable; sidebar/inspector key isolation is acceptable; sheet-first Escape is intentional modal ownership. Fresh gate/Delete flows at 1280 dark and 960 light: **254 sampled stops, zero BODY**; four axe states zero violations; reduced-motion and inspector Tab bridge pass. Version+check exit 0 (**864 frontend / 804 Rust**), targeted frontend 245, log_reveal 14, cargo --all 804, export_bindings 1 with empty drift. No application source changed or commit made. Recommend retaining the Reveal fix gate; an explicit decision to accept merely lexical central admission would require correcting its documented guarantee. Existing #66, ticket-14/17 and operator native/VoiceOver follow-ups remain.

- 2026-09-30 — **D31 gate passed** (orchestrator). Astra's re-check closed A1/A2/A4 and left R1 (a directory link inside the central repo could reveal an outside file) and R2 (NUL / interior `.` / trailing `/` normalisation); both fixed directly by the orchestrator in `core/log_reveal.rs`: central admission now requires physical containment (`central_contains` — the selected item's parent canonicalises under the canonical root; the leaf is selected, never followed), malformed input refused as written, trailing separators normalised; 17 `log_reveal` tests; fresh gate exit 0 (vitest 864, cargo 807, App.css 2076 / 24 migrated, bindings unchanged). Carried forward: 14 owns the progress overlay's "Installing Skills…" title for a Disable and the overlay removal; 17 the project toasts + `RemoveProjectModal`'s `delete-*` CSS; BACKLOG #66 the multi-skill delete command. Harness note for later children: agent-browser 0.31.1 `press` → phantom keys on Chrome 154; use `.scratch/round19/evidence/13-fix/keys-cdp.ts`.
