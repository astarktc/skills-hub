# 20 — Settings panes; Configure Tools moves into Settings → Tools; theme radio group; named header controls

Status: ready-for-agent
Blocked by: 19
Spec: `.scratch/round19/spec.md` — D24 (grill Q18). Sources: `critique-astra` F16, Settings read; `critique-opus` Q8,
P2-6; `audit.md` Settings rows. Phase 19b.

## Work

- Panes inside one Settings surface (a vertical pane list, not a second app navigation): **General** (appearance as a
  radio group with `aria-checked`, zoom, auto-sync policy — moved here from the My Skills toolbar), **Tools** (global
  Tool selection = the effective target set, with the "selected but not detected" rule explained inline; **scan
  scope** visibly distinct; the effective summary shown on save; `ToolConfigModal` retired — the My Skills header and
  ⌘K deep-link to `#/settings/tools`), **Storage & cache** (central repo path, git cache TTL, clear now), **GitHub**
  (token presence, set/clear — the keychain assurance kept), **Updates** (from `useAppUpdate`), **Diagnostics** (log
  folder, version).
- Every control labelled; the header gear named; light helper text on the measured token; auto-save vs explicit save
  disclosed consistently.
- Migration: `SettingsPage`, `ToolConfigModal` (deleted) join `migrated`; `settings-*`, `tool-*` rules leave `App.css`.

## Gate (D31)

Astra audit: keyboard through every pane; theme changed by arrow keys; Tools pane explains Windsurf's presence;
contrast; 960×640; `npm run check`.

## Result

Implemented 2026-11-04 by a delegated child (Opus) on `t3/settings-dialog` (base `8b42ce5`); nothing committed.
Gate: `npm run version:check && npm run check` green (lint · styles:check · vitest 54 files / 1168 tests · build ·
cargo fmt · clippy · cargo test 840 passed).

**Shape (D37, D50).** Settings is the surface module's `settings` dialog `{ pane, field? }` (the `settings` area, its
`Route` arm and `#/settings/<pane>` are gone from `useSurfaceState`; AGENTS.md's hash line updated). One route opens
it: `openSettingsPane(surface, pane, field?)` (`lib/commandPalette.ts`) — sidebar item (General), My Skills'
Configure harnesses + the palette's "Configure harnesses…" (Harnesses), the palette's "Settings → <pane>" entries,
Notifications' "Open Settings → GitHub" (Diagnostics + the token field focused). A pane change is `updateDialog`,
never the hash. An old `#/settings/<pane>` link (load or typed) opens the dialog once on its pane (`github` →
Diagnostics + token, `tools` → Harnesses, else General; `lib/settingsPanes.ts::legacySettingsLink`) and the hash is
replaced with the route showing (tests in `useSurfaceState.test.ts`).
- **Dialog variant:** new `DialogContent size="settings"` — fixed `h-[min(640px,85vh)] w-[min(880px,90vw)]`, not
  resizable (DESIGN.md § Dialog).
- **Pane list:** `Tabs variant="pane"` (new; forces `orientation="vertical"`): `tablist` / `tab` / `tabpanel`,
  activation follows focus (↑ / ↓ show each pane), the panel is `tabindex="-1"` so Tab goes straight into the pane,
  inset rings; selected = `raised` + 2 px iris leading bar (6.14 / 4.72). Frame: `components/settings/PaneDialog.tsx`.
- **Keyboard shortcuts:** its own `shortcuts` dialog kind in the same frame (`ShortcutsDialog.tsx`, panes General ·
  My Skills), opened from the sidebar (the popover is gone) and the palette ("Keyboard shortcuts",
  `openKeyboardShortcuts`); same focus / Escape rules.
- **Theme:** new `RadioGroup` primitive (`ui/radio-group.tsx`, Base UI Radio: `radiogroup` / `radio` +
  `aria-checked`, arrows move and select). Segmented stays `aria-pressed` for views. Sidebar Theme menu kept.

**Panes.** General (Auto-deploy + D49 line, zoom, Library folder + Browse…, Git cache days + Clean Now, freshness) ·
Harnesses (below) · Appearance (theme radios) · Updates (`useAppUpdate`: version, status line, Check, notes in
`.prose`, Download & Install, Don't remind again) · Diagnostics (version, App log → existing `open_log_folder`, Library
folder read-only, GitHub token presence + Save / Remove + keychain text). **Save behaviour:** everything auto-saves
(as today) except the GitHub token (explicit Save token / Remove token); each pane states it.

**Harnesses (D47).** One expandable row per Tool-catalog entry: disclosure (name) · Detected / Not detected (stale
tone when selected) · Switch (saved global selection). Expanded: skills folder `~/…` + "Shared with …", deploy mode
(registry `supports_symlink`), detect dirs; a selected-undetected harness explains why it is listed + **Remove from
selection** (focus moves to the neighbour row first). Detected-only filter and the Import-scan checkbox carried from
Configure harnesses (scan scope its own section). Helper text states turning off removes nothing (verified: a saved
selection writes no target; Propagation ignores it). Selection writes moved into
`useSyncOrchestration.saveToolConfig` / `setHarnessSelected` (auto-save, serialized writes, optimistic, a failed write
re-reads settings); `handleOpenToolConfig` / `handleCloseToolConfig` / `handleToolConfigConfirm` and the `toolConfig`
dialog kind are gone.
- **Backend enabler:** the rows need the dir / detect dirs / deploy mode, which were not on the wire. Added
  `ToolCatalogEntry.placement: Option<GlobalPlacement>` (global catalog only; `None` at project scope, where one field
  could not be honest) → `ToolInfoDto.placement: ToolPlacementDto | null` (`skills_dir`, `detect_dirs` home-relative,
  `supports_symlink`); bindings regenerated; catalog test `only_the_global_catalog_carries_placement_…`; fixture parity
  (`fixtureDetectDir`). Not touched: `tool_adapters/mod.rs`.
- **D48 flow:** `hooks/useHarnessSelection.ts` (composed in App): save → if on ∧ auto-deploy on ∧
  `bulkCounts.harnessArrivals(skills, tool)` (enabled, locatable, no deployment there yet) is non-empty → the new
  `deployHarness` ask (`DeployHarnessAsk.tsx`, ConfirmDialog "Deploy your N skills to {Harness} now?" — Deploy now /
  Not now) → confirmed: `reporter.whenIdle()` (new: the save rescans the import plan, a read that would otherwise
  refuse the deploy — observed on the fixture) → `actions.run("syncToTargets", arrivals, { tools: [tool], confirmed:
  true })` (the door → `syncSkillsToTools` with the one overwrite ask; header progress at the frame's foot via
  `progressInDialog`). Declined / dismissed / auto-deploy off / turning off → nothing. Hook tests cover each branch.

**`ToolConfigModal` deviation:** it was not only global — Projects' "Configure harnesses…" uses it. Its global use is
retired; the file moved (git mv semantics) to `components/projects/ProjectToolsModal.tsx`, trimmed of the global-only
scan toggle, and its `migrated` entry renamed in place (a deleted path fails `styles:check`).

**Deleted i18n keys (11):** `themeMode`, `appUpdates`, `browse`, `diagnostics`, `globalToolConfigTitle`,
`globalToolConfigDesc`, `globalToolConfigConfirm`, `status.toolConfigSaved`, `detail.back`, `settingsAutoDeployHint`,
`palette.panes.github`. New families appended at the catalog's end: `settingsDialog.*`, `shortcutsDialog.*`;
`palette.keywords.{shortcuts,panes.*}`.

**Migration:** every `settings-*` rule plus `.mono` / `.btn-sm` (only Settings used them) left `App.css`;
`appCssCeiling` **812 → 561**; `migrated` += the nine `components/settings/*.tsx` files. No `tool-*` / `pick-*` /
`tool-pick-list` / `tool-filter-toggle` rule was left to remove (ticket 16 took them). DESIGN.md: § Tabs, Segmented
and RadioGroup, § Dialog (size), § My Skills shell bullet, § ⌘K palette (reach / focus), new § Settings (ticket 20);
specimen gains the pane tabs and RadioGroup.

**Fixture proof** (`npx vite --mode fixture --port 5180`, ego-browser), `.scratch/round19/evidence/20/`:
`settings-{general,harnesses,appearance,updates,diagnostics}-{dark,light}-{1280,960}.png` (rich),
`settings-harnesses-windsurf-{dark,light}-{1280,960}.png` (failures: Claude Code + Windsurf expanded),
`settings-d48-ask-dark-1280.png`, `settings-d48-ask-light-960.png`, `settings-d48-deploying-dark-1280.png`,
`settings-d48-settled-dark-1280.png` ("60 skipped" — Augment is undetected in the fixture),
`shortcuts-{general,skills}-{dark,light}-{1280,960}.png`. Keyboard walk (rich, dark, 1280):

```
start: button "Settings"  theme=dark  hash=#/skills
Enter      → tab[aria-selected=true] "General"
ArrowDown  → tab "Harnesses"  (pane shown; hash=#/skills)
ArrowDown  → tab "Appearance" (pane shown; hash=#/skills)
Tab        → radio[aria-checked=true] "Dark"
ArrowRight → radio "System" checked  data-theme=dark (system is dark)
ArrowRight → radio "Light" checked   data-theme=light
Escape     → button "Settings" (dialog gone)
⌘K "Harnesses" → [Settings → Harnesses, Configure harnesses…, Disable every skill…]; Enter → Harnesses pane, its tab focused
Escape     → button "Settings"
#/settings/github (typed)  → Diagnostics, focus #settings-github-token, hash=#/skills; Escape → hash stays #/skills
#/settings/github (fresh load) → same; Escape → main landmark (opened with focus nowhere)
Notifications → Open Settings → GitHub → Settings/Diagnostics, token focused; Escape → the bell
D48: switch on Augment → ask (focus Deploy now); Not now → focus back on the switch, nothing ran;
     Deploy now → waits for the rescan, batch runs (switch held, focused), settles "60 skipped · View report"
Remove from selection (Windsurf, detected-only) → row gone, focus on Gemini CLI's row
```

Contrast (computed from `theme.css`, dark / light): pane tab `muted-foreground` on `popover` 7.65 / 8.36, selected
`foreground` on `raised` 15.29 / 14.33, iris bar and radio edge `ring` on `raised` 6.14 / 4.72, helper `muted` 7.65 /
8.36, notes / group titles `faint` on `popover` 5.52 / 6.24.

**Unproven / left for the audit:** axe and reduced-motion were not run; contrast is computed, not measured on the
page; native Tauri (zoom hotkeys, keychain, real `open_log_folder`) not exercised; the "Scanning harnesses" read a
selection save starts shows at the frame's foot and briefly holds the switches (D19's rule, unchanged).

### Fix pass (audit)

2026-11-04, against `.scratch/round19/review/ticket-20-audit.md` (verdict FAIL: F1 P0, F2/F3 P1, F4 P2), the
dispositions as the orchestrator set them; nothing committed. Gate: `npm run version:check && npm run check` green
(lint · styles:check 561 / 48 · vitest 55 files / 1198 tests · build · fmt · clippy · cargo test 840; `cargo test
--all` 840). `src/bindings/index.ts` is byte-identical to the audited file (SHA-256 `b5895631…0758`): no backend
change was needed — `update_setting` already answers the saved `AppSettings`.

- **F1 — the save is a lifecycle mutation.** `useSyncOrchestration.saveToolConfig` lost its private write queue and
  runs under `runAction` (label "Harness settings change", activity "Saving harness settings"; batch subject). The
  optimistic change happens only once admitted; success adopts the backend's echo (`global_selected_tools`,
  `scan_selected_tools_only`); failure restores exactly the configuration that write was admitted over and calls
  `notifyError` (the old `setError` + `getSettings` re-read is gone). The result is `HarnessSaveResult` =
  `saved | failed | refused`. A synchronous `configRef` (written by the startup read, an admitted save and its
  settlement — never a render) backs the next save and `readSavedSelection()` for D48. Held with the reason while
  anything runs: every harness switch and Remove from selection (as before), plus **the scan checkbox** and General's
  **Browse…** and **Clean Now** (Auto-deploy already was). Zoom and the two cache numbers stay live: they are
  preferences written to their own keys, not mutations an action touches, and `Input` / `Select` have no held state.
- **F2 — the ask needs an installed harness and N ≥ 1.** `useHarnessSelection` takes `installed` (the Tool catalog's
  `installedToolIds`, passed by App) and re-checks it and Auto-deploy after `whenIdle`. An undetected harness is saved
  with no ask; its row keeps the explanation and Remove from selection.
- **F3 — latest intent only.** An "on" asks (and, confirmed, deploys) only while: the harness's last gesture is "on",
  it is the newest "on" the lifecycle did not refuse, the saved selection still includes the harness, and
  Auto-deploy + installedness still permit it — checked before the ask and after `whenIdle`, else dropped silently.
  A saved "off" dismisses that harness's open ask (`surface.dismissDialog("deployHarness")`). **Deviation from the
  brief's wording, deliberate:** with F1, a click on a switch that is already held never reaches the hook (the
  primitive refuses it). Under a rule where "a refused gesture supersedes nothing", on→off at ~100 ms would refuse
  the off and still ask. So a held switch's activation is reported too: `HarnessRow` listens for the primitive's
  `HELD_REFUSAL_EVENT` and calls `noteHeld(tool, !selected)`. It saves nothing and the toast still says "Nothing was
  changed", but it counts as the operator's last word. A refused hook-level call (two activations in one frame)
  counts the same way.
- **F4 — static panes.** `TabsContent` gained `staticPane` (pane variant: `tabindex="0"`, else `-1`); the frame sets
  it from `PaneSpec.static`: both Keyboard-shortcuts panes, and Updates when `updatesPaneHasControls` is false
  (checking / installing / done, or outside the app; `lib/settingsPanes.ts`). DESIGN.md § Tabs no longer says a pane
  always holds controls; § Settings documents the save lifecycle, D48's eligibility and latest-intent rule, and the
  static-pane Tab stop.

**Tests.** `useSyncOrchestration.test.ts` (harness configuration, real `useStatusReporter`): "the save is a lifecycle
action: it holds the slot, and every other mutation is refused while it runs"; "two rapid toggles, the first write
failing: the second is refused (not queued) and the UI equals the persisted selection"; "a successful save adopts the
backend's echoed configuration"; "a failed write rolls back only itself and says why through notifyError"; "save →
scan → deploy: the import-plan read waits for the save to release the slot, and a deploy after it is admitted".
`useHarnessSelection.test.ts` — F2: "an undetected harness is saved to the selection with no ask", "zero eligible
skills: no ask", "disabled and unlocatable skills are not counted", "skills already deployed there are not counted;
only none left → no ask", "a detected harness with arrivals asks with the right N", "a harness that stops being
detected before dispatch deploys nothing"; F3: "on → off before the first save settles: no ask, no deploy", "on → off
→ on: two writes superseded, one ask, one deploy" (three writes reach the sync world when nothing refuses them; in
the app the lifecycle refuses the overlapping two), "an off refused while the on's save runs still withdraws the on",
"on, then a refused off and a refused on: the saved on asks once", "an off on the held switch … withdraws the on;
nothing is saved", "held off then held on: the on stands again and asks once", "auto-deploy turned off while the
save runs: no ask", "a saved selection that no longer includes the harness: no ask", "an off dismisses the open ask
of a waiting on", "an off for another harness leaves this harness's ask alone", "an intent superseded while the
confirmed deploy waits for the rescan deploys nothing". F4: `tabs.test.tsx` "a static pane (no focusable control)
is itself the Tab stop after the pane list"; `lib/settingsPanes.test.ts` `updatesPaneHasControls`.

**Browser** (`npx vite --mode fixture --port 5190`, ego-browser, captures in `.scratch/round19/evidence/20-fix/`,
gitignored):
- F1, the audit's injection (`rich&latency=10`, first `global_selected_tools` write throws; Cursor off, Pi 20 ms
  later, both dispatched in-page): Pi held with "Harness settings change is running — wait for it to finish";
  **one** write (failed), none queued; after settlement the backend holds `[cursor, claude_code, codex, amp,
  kimi_cli, pi, gemini_cli]`, UI Cursor ✓ Pi ✓, "Deploying to 7 of 45", one error toast (`f1-failed-save-ui-equals-persisted.png`).
- F2 (`rich`, Auto-deploy on, detected-only off, Augment on): saved, switch on, **no ask**, 0 Augment target rows
  (`f2-undetected-no-ask.png`).
- F3 (`rich&latency=5`, Gemini CLI off and settled, then on and a real pointer activation ~100 ms later): the off is
  held ("…Nothing was changed."), the selection stays on (switch on), **no ask** (`f3-on-then-held-off-100ms-no-ask.png`).
  Control: a plain on still asks "Deploy your 3 skills to Gemini CLI now?".
- F4: Keyboard shortcuts → Tab → the General tabpanel (`tabindex=0`); Shift+Tab → tab; ↓ My Skills → Tab → its
  panel; Tab → Close (`f4-shortcuts-panel-focused.png`). Settings General/Updates-with-controls panels `-1`; Updates
  while installing (`&update=9.9.9&latency=3`, 0 buttons) `0`, Tab from its tab → the panel
  (`f4-updates-installing-panel-focused.png`).

**Unproven:** native Tauri (a real `update_setting` failure, the keychain, zoom hotkeys). A VoiceOver pass over a
focused static panel. Clean Now and library relocation are held on screen only: they still do not run under
`runAction` (the audit's inherited note; not in this pass).

## Comments

- 2026-09-30 — from ticket 15: the legacy `.settings-toggle-knob` now slides on the tokens (`--motion-fast`, `--ease-out`)
  instead of 200 ms `ease`; the pane migration should replace it with the `Switch` primitive (which also offers
  `focusableWhenDisabled`). `.settings-select` / `.settings-theme-btn` lost their `transition: all` (instant hover).

- 2026-09-30 — from ticket 16: `shared/ToolConfigModal.tsx` (Configure Tools) migrated onto the primitives (Checkbox /
  Field / Badge / Button; joins `migrated`), because the `pick-*` and `tool-pick-list` / `tool-filter-toggle` rules it
  used had to leave `App.css` with the Add pickers. Behaviour unchanged (detected-only filter, a selected undetected
  Tool stays listed "not detected", scan-only toggle). When Configure Tools moves into Settings → Tools, carry it as is.
  Add's Tool scope ("Edit Tools" in Add's summary) is a session choice over the same catalog, not the saved selection.

- 2026-10-27 — from ticket 18's think-aloud (`18-operator-think-aloud.md` § Findings, P3·Q1, P5·Q3):
  - **Auto-sync's new home.** Asked to turn it off, the operator looked in the header, then Configure Tools, then found
    it in Settings. He agrees with the move. Consider a one-time "Auto-sync moved to Settings" hint for upgraders, or a
    link from Configure Tools.
  - Auto-sync drives the re-assert behind BACKLOG #71, and nothing on the main screen says it is on.
  - **A selected Tool that is no longer detected** (Windsurf in the fixture) puts a permanent hollow mark in every strip.
    He understood why, but it's standing noise. Tools could flag "not detected — remove from the selection?".

- 2026-11-02 — **grill decisions that reshape this ticket (spec D37, D47, D48, D49, D50; vocabulary D45 — read
  "Tool" here as Harness, "Sync" as Deploy, "Auto-sync" as Auto-deploy; ticket 32 renames the strings first):**
  - **Settings is a dialog, not an area (D37).** A large modal on the layer stack (max ≈ 880 × 640, capped at 90 vw /
    85 vh, not resizable — D50), pane tabs on the left: General · Harnesses · Appearance · Updates · Diagnostics. Opened
    from the sidebar's Settings item; Escape closes; focus returns there. `#/settings/<pane>` leaves the hash and the
    settings area/page is removed (`SettingsPage.tsx` becomes the dialog's body). Keyboard shortcuts adopts the same
    frame. Configure Tools becomes the **Harnesses** pane.
  - **Harnesses pane = per-harness expandable rows (D47)**: detected or not, the directory in use, the deploy mode in
    use, "remove from selection" for an undetected one (the Windsurf noise above). No speculative options — the
    AGENTS-standard-dir preference is BACKLOG #83, gated on research.
  - **Adding a harness with auto-deploy on asks once (D48)**: "Deploy your N skills to Cursor now?" → one global batch,
    one overwrite ask; declined or auto-deploy off → nothing (hollow marks; Deploy all per group). Never automatic.
  - **Auto-deploy's state shows where it acts (D49)**: Add's summary, the Import review, this pane — not My Skills.
    The 2026-10-27 "nothing on the main screen says it is on" concern is answered by ticket 33 (auto-deploy no longer
    changes anything on Update), not by a header indicator. The upgrader hint ("Auto-sync moved to Settings") is
    dropped: 1.3.0 already moved it.
- 2026-11-02 — Vocabulary: read Tool/Sync/Refresh as Harness/Deploy/Update — ticket 32.
- 2026-11-04 — operator grill (autonomous run through 31 → 20 ∥ 21 → 34 → 24 → 25 → 26; results reviewed at the end): the sidebar's Theme menu stays beside the Appearance pane (quick toggle; D37 does not remove it).
- 2026-11-04 — implemented by delegated child; awaiting Astra audit.
- 2026-11-04 — fix pass after audit: F1–F4 as the orchestrator disposed; re-check pending.
- 2026-11-04 — orchestrator, before the re-check: the fix pass's `noteHeld` mechanism (a click on a *held* switch counted
  as the operator's "last word" and withdrew a pending D48 ask) is **removed**. A refused activation changes nothing
  (D19 — the toast says "Nothing was changed"); treating it as hidden intent contradicts that. With F1 the rapid off is
  refused, the saved selection stays on, and the ask that follows is therefore correct — the operator answers it. F3's
  real defect (an ask or deploy after the *saved* selection turned off) stays fixed by the saved-selection re-check.
- 2026-11-04 — orchestrator, after the re-check (FAIL on F5 only): `useHarnessSelection` recorded a gesture before
  admission, so a *refused* coordinator off-call still suppressed the admitted on's ask. Now a refusal restores the
  previous word (an admitted off still supersedes a pending on); the test that asserted the rejected behaviour is
  inverted. F1–F4 CLOSED by the re-check.
