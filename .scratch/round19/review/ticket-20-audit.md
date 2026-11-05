# Ticket 20 — Spec review and adversarial audit

**Verdict: FAIL.** One P0 blocks committing this implementation: a failed queued selection save can leave the displayed target set different from the persisted target set. Two P1 D48 defects and one P2 accessibility note are below. The dialog/frame migration itself passes the exercised checks.

Reviewed the uncommitted diff and new files on `t3/settings-dialog`, HEAD `8b42ce5`, on 2026-11-04. Read AGENTS.md, the ticket's Work, every Comment and Result, spec D24/D37/D47–D50, and the relevant DESIGN.md contracts. Source was not edited; this report is the only added file. `cargo test export_bindings` regenerated identical bytes. Browser: my own `npx vite --mode fixture --port 5186 --strictPort`, ego-browser task space 13; no native Tauri/library operations. Applied the freshly fetched Vercel Web Interface Guidelines, with this repo's explicit no-dialog-hash rule taking precedence over their generic URL-state advice.

## Spec

### (a) Missing / partial

- D48: **“Adding a harness with auto-deploy on asks once”**, and Result: **“only … when the library has skills to give it”**. Installedness is absent from the ask's eligibility. Augment is not detected, yet the app offers 60 deployments that all skip (F2).
- AGENTS.md / D19: **“one mutation at a time … a second is refused … never queued.”** The new selection writer queues whole-config mutations outside `runAction`; only the subsequent scan and deploy enter the lifecycle. The requested three-action contract is therefore partial, with observable persistence/UI divergence (F1).

### (b) Scope creep

None requiring removal. The five-pane shape supersedes original D24's six-pane names. `size="settings"`, pane Tabs, RadioGroup, the shortcuts dialog, legacy-link conversion, `placement`, and `whenIdle()` serve the approved decisions. Keeping Projects' separate dialog as `ProjectToolsModal` is necessary, not an unwanted second global editor. Keeping the sidebar Theme menu is explicitly approved. Add's **Edit harnesses** correctly remains a session-local picker, per the 2026-09-30 Comment—not a Settings entry to retarget.

### (c) Implemented but wrong

- Work: **“global Tool selection = the effective target set … the effective summary shown on save”**; Result: **“a failed write … re-reads settings.”** That rollback can erase newer optimistic state, while a later queued success saves a different configuration without adopting it (F1).
- D48: **“declined or auto-deploy off → nothing … Never automatic.”** The hook evaluates the old invocation's `selected`/policy, not current intent. On→off before the save settles still raises an actionable deploy ask after the harness is off (F3). On→off→on generates two successive ask identities rather than one coalesced latest-intent ask.
- DESIGN.md Tabs: **“a pane panel … always holds controls.”** The newly shared shortcuts frame contains static panes with no controls, but forces their tabpanels to `tabindex=-1` (F4).

## Findings

### F1 — P0: failed queued save leaves selection UI and persisted targets disagreeing

**Locations:** `src/hooks/useSyncOrchestration.ts:678–713`, especially `:690` (private write queue), `:696` (success only increments scan revision), `:701–705` (unconditional rollback); `src/components/settings/HarnessesPane.tsx:140` (scan checkbox has no lifecycle hold).

**Evidence:** `ego-browser nodejs`, fixture `?scenario=rich&latency=10`. This is a controlled failure injection into the fixture's in-memory settings setter, not a source edit or real filesystem fault:

1. Open Configure harnesses, wait for startup scan to finish.
2. Wrap `window.__fixture.state.settings.global_selected_tools` in an enumerable getter/setter. Reject its first write with `{code:"OTHER", message:"audit: first settings write failed"}`; subsequent writes store normally. Reads still return the last persisted value, so `getSettings` works.
3. Click Cursor off, then Pi off 20 ms later, while the first save is pending.
4. Wait for both writes and the rescan to settle.

Observed output:

```text
writes[0]: [claude_code,codex,amp,kimi_cli,pi,gemini_cli] — failed
writes[1]: [claude_code,codex,amp,kimi_cli,gemini_cli]    — succeeded
backend selected: [claude_code,codex,amp,kimi_cli,gemini_cli]
UI: Cursor checked=true; Pi checked=true
summary: Deploying to 7 of 45 harnesses · 7 detected on this computer
toast: audit: first settings write failed
```

Capture: `/tmp/ticket20-save-failure-divergence.png`. The operator sees two harnesses selected that future backend operations will not select. The second write also replays the first, failed off intent because it captured the entire optimistic configuration. This is not merely a transient pending display.

**Exact proposed fix:** make selection/scan-scope saving a reporter-owned mutation, with admission checked before optimistic state changes. Remove the private mutation queue; synchronously hold/refuse further mutation controls while that save runs, including the scan checkbox. Adopt the successful backend configuration echo (not just a scan revision), and rollback only the admitted write's own revision on failure. Start/await the import-plan read after the save releases the reporter slot. Add deferred-promise hook tests for a failing save plus a second attempted toggle, a successful save, and save→scan→deploy; pin both the UI selection and persisted snapshot. If queued/coalesced saving is deliberately retained instead, it needs an explicit D19 exception plus revision-aware rebasing/settlement—not the current whole-snapshot queue.

### F2 — P1: D48 offers a deploy that cannot land on an undetected harness

**Locations:** `src/hooks/useHarnessSelection.ts:9–21,51–59`; `src/lib/bulkCounts.ts:157–163`. The hook has no Tool-catalog installedness dependency, and the fold examines only skills/target statuses.

**Evidence:** `ego-browser nodejs`, fresh rich fixture, Auto-deploy on → Harnesses → uncheck “Show detected harnesses only” → turn Augment on:

```text
installedTools.includes("augment") = false
ask = Deploy your 60 skills to Augment now?
Deploy now → toast = 60 skipped · View report
skills with any augment target row after settlement = 0
```

Captures: `/tmp/ticket20-undetected-ask.png`, `/tmp/ticket20-undetected-skipped.png`. The child's own Result records this same all-skipped outcome as its deployment proof; it is evidence of the defect, not evidence that the count is actionable. The selected-undetected row's separate helper correctly says every deploy skips it.

**Exact proposed fix:** pass installedness from the existing Tool catalog into the D48 coordinator; save an undetected harness selection but do not raise a deploy-now ask for it. Recheck installedness before dispatch, and keep the undetected explanation/remove action. Retain the skill-level count fold, but test undetected, zero eligible, disabled/unlocatable, already deployed, and detected-with-arrivals cases. Do not infer installedness from the displayed path.

### F3 — P1: a superseded “on” intent can ask and deploy after the harness was turned off

**Locations:** `src/hooks/useHarnessSelection.ts:51–59`; `src/hooks/useSyncOrchestration.ts:678–713`; `src/hooks/useSurfaceState.ts:538–546` (same-kind asks replace rather than queue).

**Evidence:** `ego-browser nodejs`, rich fixture with `latency=5`. Turn Gemini CLI off and let its scan settle; then click on→off approximately 100 ms apart, before the 300 ms save completes. Both clicks are accepted (`aria-disabled` absent). After both writes settle:

```text
saved selection includes gemini_cli = false
switch aria-checked = false
ask remains: Deploy your 3 skills to Gemini CLI now?
accept ask → 3 deployed
saved selection still excludes gemini_cli
```

Capture: `/tmp/ticket20-stale-ask-after-off.png`. No injected error was needed.

A separate on→off→on construction at `latency=10` observed **three persisted selection writes** (`true,false,true`) and **two distinct `deployHarness` ask IDs** (`1,2`), inspected from the rendered React tree. Only one ask is visible at a time because `askAnyKind` dismisses the older same-kind ask. Accepting the final visible ask produced one “3 deployed” report. Capture: `/tmp/ticket20-rapid-on-off-on.png`. Thus there is no claim of two confirmed batches in this test; the failure is stale intent and duplicate ask generation.

**Exact proposed fix:** in addition to F1's synchronous save admission, invalidate superseded per-harness intentions. Before asking and again after `whenIdle`, require the request's generation to remain current, the saved harness selection to remain on, and current Auto-deploy/installedness to permit the offer. An off action must invalidate a waiting on action/ask. Test on→off, on→off→on, policy changes while a save waits, and accepting while a rescan is pending. Do not close over policy/selection as the sole authority across async boundaries.

### F4 — P2: static shortcuts panes have no keyboard tabpanel stop

**Locations:** `src/components/ui/tabs.tsx:122–124`, `src/components/settings/PaneDialog.tsx:79`, `src/components/settings/ShortcutsDialog.tsx:17–32,75–83`.

**Evidence:** browser keyboard walk: Keyboard shortcuts → ArrowDown to My Skills → Tab gives focus to **Close**. DOM: `tabpanel.tabIndex=-1`, number of focusable pane controls `0`. The blanket assertion “a pane always holds controls” is false for both shortcut panes (and for some Updates lifecycle states).

**Exact proposed fix:** let `PaneSpec` declare a static/body-focus pane and pass `tabIndex={0}` to its `TabsContent`; use it for both shortcut panes and any Updates state with no focusable content. Keep `-1` for Settings panes whose first control is the intended Tab destination. Add a primitive/frame contract test, and correct the unconditional DESIGN.md wording. This is a keyboard discoverability/a11y note, not a failure of the arrow-selection implementation.

## Verification and coverage

### Fresh commands

Run in this worktree:

```text
npx vitest run src
  Test Files 54 passed (54)
  Tests 1168 passed (1168)

npx tsc --noEmit -p tsconfig.app.json
  exit 0, no diagnostics

npm run styles:check
  styles:check ok — App.css 561 counted lines (ceiling), 48 migrated file(s),
  prose layer scoped, no raw colours; self-test passed.

git diff --check
  exit 0, no output

cd src-tauri && cargo test export_bindings
  1 passed
cargo test only_the_global_catalog
  1 passed
cargo test global_catalog
  4 passed
cargo test project_catalog
  3 passed
```

The first exploratory `cargo test tool_catalog` matched **zero** tests; the correctly named filters above were then run. Before/after SHA-256 of `src/bindings/index.ts` was identical: `b589563184a65cdd4aff42ba9df3bb90f792c8d56c97aa98d532bdeee8660758`. “Clean” here means **no export drift relative to the reviewed working file**, not no diff against HEAD—the intended new placement binding is uncommitted. `placement: ToolPlacementDto | null` preserves `Option` fidelity. The catalog test checks every global entry against all three registry placement facts, relative paths, and every project entry's `None`; it is meaningful.

The child's full `npm run check`/840-Rust-test claim was not independently rerun: this audit ran the specifically permitted frontend gates and targeted Rust filters. No fresh claim about full clippy/build/full Rust suite is made.

### Layer stack, focus, hash and keyboard

- At **1280×800**, Settings measured **880×640** at `(200,80)`. Sidebar item → Enter opens on General. Escape returns to Settings. General→Harnesses→Appearance→Escape also returns to Settings.
- Focus Configure harnesses in the main header, then **⌘K → Settings → Enter → Escape**: focus returns to **Configure harnesses**, the palette's actual opener, not a hardcoded sidebar item. This strengthens the child's Settings-opener-only example.
- D48 ask over Settings: Settings popup is inert, ask is not; one Escape leaves Settings open and returns focus to `aria-label="Deploy to Gemini CLI"`. App `#root.inert=true`; the body-level `[data-skillshub-live]` host has `inert=false`.
- Shell persistence was verified through the DOM-associated React root: the same Settings `PaneDialog` fiber/alternate remained present with `memoizedProps.open` **true→false** across Escape; DOM popup count became zero and focus returned to Settings. The popup DOM is intentionally unmounted by the primitive; it is the React shell that must persist. `App.tsx:838` also renders SettingsDialog unconditionally.
- Pane DOM has vertical `tablist`, selected tabs with controls associations, and labelled `tabpanel`. ↓ selects Harnesses then Appearance; Home selects General, End Diagnostics; Tab from Appearance enters the checked radio; Shift+Tab returns to Appearance. Automatic activation is appropriate here: panes display already-loaded world values and switching panes starts no network read. The static-pane exception is F4.
- Theme radios: one checked/Tab-0 item, other items Tab -1, named radiogroup; Right selects immediately and updates `<html data-theme>`. Sidebar Light→Appearance shows Light checked; Appearance Right→Dark makes the sidebar “Theme: Dark” and its menu Dark checked. No stale second state.
- Hash stayed `#/skills` through opens and pane switches. Both assigning `location.hash='#/settings/github'` and a fresh URL load focused `#settings-github-token` on Diagnostics and cleaned to `#/skills`; Escape did not reopen it. `tools` selected Harnesses; `nonsense` selected General. Hook tests reject the removed Settings route, cover legacy load/typed links and pane changes without hash changes. AGENTS.md's changed hash statement is accurate.
- Notifications was exercised, not just grepped: failures fixture → Retry update on `pr-review` → rate-limit report → Notifications → Open Settings → GitHub → token focused, one Settings dialog, clean hash → Escape returns to **Notifications**.
- `openSettingsPane` is the common Settings entry used by App's sidebar/header handlers, palette and Notifications. Explore and UpdateDialog have no independent Settings entry to retarget. Add's “Edit harnesses” remains its intentional local `ToolScope` popover (`AddSkillDialog.tsx:235–280`), as required by the ticket Comment.

### Harnesses, D48 and mutation lifecycle

- Rows map `toolStatus.tools` through `visibleToolChoices`; no handwritten product harness list. Expanded rows show home-relative folder, mode/fallback wording, detect dirs, and `shared_with`-based sharing (not path-derived grouping). `rg -n skills_dir src` found the new rendered path only in HarnessesPane, besides bindings/fixture/test data and explanatory comments.
- Failures fixture Windsurf remains in detected-only mode, labelled Not detected with the explanation and Remove from selection. Remove hides the row and focuses **Gemini CLI**. Captures: `/tmp/ticket20-windsurf-{dark,light}-960.png`.
- Scan scope is a separate labelled checkbox/group. Summary updates optimistically; its failure correctness is F1. Off removes no targets: `core/settings.rs:354–372` writes only the selected-tools JSON and scan boolean; it invokes no artifact removal or deployment. The helper's “Update keeps them current” agrees with Propagation's selection-independent policy.
- Detected Gemini CLI in rich: count **3**, reconciled to `prisma-review`, `effect-ts-basics`, `using-git-worktrees`. Yes produced **3 deployed**, focus on the switch, no refusal toast; No kept all three absent and produced no toast; Auto-deploy off → off/on produced no ask or deployment.
- Failures Cursor: count **13**, checked against the fixture's enabled/locatable/non-deployed rows; Yes produced the **one** overwrite dialog with two rows (`zod-schema-first`, `prisma-review`). Answering Overwrite settled **13 deployed**. The action routes through `actions.run("syncToTargets", …)` (`useHarnessSelection.ts:59`), not direct IPC; the sync seam owns the batch and collision retry. Captures: `/tmp/ticket20-d48-overwrite.png`, `/tmp/ticket20-overwrite-settled.png`.
- Progress is rendered at Settings' foot (`App.tsx:608` includes Settings in `progressInDialog`). Observed “Scanning harnesses” there and “Deploying / Waiting for your confirmation…” beneath the overwrite ask. `/tmp/ticket20-d48-progress.png` is the scan/wait stage, not a claimed completed deployment screenshot.
- At explicit fixture `latency=1`, MutationObserver recorded the switch hold from **310 ms to 727 ms** after observer setup (417 ms held), with title **“Scan is running — wait for it to finish”** and visible scan footer. That short hold is understandable. The preceding save has no reporter hold; F1/F3 exploit that gap. Slower scans hold it longer, correctly explaining the read.
- `whenIdle()` was tested in a browser-mounted temporary hook probe using the real module: a running read rejected with an error → `whenIdle` resolved **true** after `finally`; unmount while a read was pending → resolved **false**, not the 1-second timeout sentinel. Source `useStatusReporter.ts:446–452,748–766` wakes waiters on disposal and every settlement. This is not a timeout guarantee for a backend command that never settles; no such guarantee is claimed.

### Other panes, Projects, migration

- General labels/description wiring were checked via DOM `input.labels`/`select.labels`; theme/Auto-deploy use explicit ARIA. Each applicable pane discloses auto-save; token explicitly discloses Save. D49 appears in General and the Harnesses state line, not a new My Skills indicator.
- Browse calls the existing `pickFolder` seam; `useSettingsState.ts:219–227` returns for an empty selection and catches failures. Updates consumes the one `useAppUpdate` value; no second update state was added.
- Diagnostics uses **`openLogFolder`**, not the target-restricted `revealPath`. `commands/mod.rs:265–301` resolves the app log location itself, then reveals the existing log or opens its directory. Fixture button click succeeded with no error toast. Native OS opener/keychain behavior was not exercised.
- A controlled fixture token-write rejection `{code:"CREDENTIAL_STORE_UNAVAILABLE", detail:"audit: keychain unavailable"}` surfaced the full unlock/allow-access guidance through `notifyError`/`describeCommandError`, not a raw object or success. Capture `/tmp/ticket20-keychain-error.png`. No real credential was used.
- **Inherited lifecycle limitation, not introduced by this diff:** Clean Now still directly invokes `clearGitCacheNow` (`useSettingsState.ts:309–320`), not `runAction`; other settings writes likewise retain their previous independent-save behavior. Thus a blanket claim that *every* Settings mutation is reporter-gated would be false. Track/gate cache cleanup and library relocation if the intended D19 scope includes those operations; F1 is the new selection writer's concrete blocker.
- Projects end-to-end: skills-hub → Configure harnesses → Pi off → Save harnesses; fixture project tools changed `[claude_code,agents_skills,pi]` to `[claude_code,agents_skills]`, and Pi's matrix column disappeared. Existing `configureProjectTools` seam remains (`useProjectState.ts:653`); global scan toggle absent. `/tmp/ticket20-project-configured.png`. The rename-in-place migrated entry passes the styling guard. The old conditional mounting pattern in this Projects dialog is inherited; the Settings shell does not use it.
- `settings-*` CSS and bare `.mono`/`.btn-sm` are removed, with no live legacy class consumers found (the `settings-github-token` **id** and Tailwind `font-mono` are not such classes). Removed i18n key reads were searched; the full resource-key tests pass. Ceiling **812→561** and nine new Settings entries + Projects rename agree with the fresh 48-file styles check.

### Accessibility, actual contrast, minimum size and motion

Injected pinned **axe-core 4.10.3** from cdnjs into the fixture only (not installed/committed). `axe.run(Settings dialog)` on General, Harnesses, Appearance, Updates and Diagnostics returned **zero violations and zero incomplete checks** in the exercised dark rich state. This is automated coverage, not a VoiceOver certification.

Computed actual rendered foreground and composited background colors using `getComputedStyle`, a canvas sRGB conversion for color-mixed backgrounds, and WCAG relative luminance:

| Actual pair | Dark | Light |
| --- | ---: | ---: |
| Unselected pane text / dialog | 7.65:1 | 8.36:1 |
| Selected pane text / raised | 15.29:1 | 14.33:1 |
| Windsurf helper / dialog | 7.65:1 | 8.36:1 |
| Selected-undetected Not detected badge / soft stale ground | 8.94:1 | 4.94:1 |
| Faint note / dialog | 5.52:1 | 6.24:1 |

At **960×640**, both themes, all five panes measured **864×544** at `(48,48)`—exactly 90 vw / 85 vh. Pane width and scrollWidth both **670** (no horizontal overflow). General scrollHeight/clientHeight **567/485**, Harnesses **667/485**, other panes **485/485**; scrolling to the bottom reached final controls. Header top **49** and list top **106** stayed fixed while pane scrollTop changed (82/182). Close remained **32×32** at `(867,61)`. With `prefers-reduced-motion: reduce`, popup, panel, list and tabs all computed animation/transition duration **0s**; pane switches stayed instantaneous. Expanded Windsurf also remained readable and scrollable at the minimum size. First-run Settings and shortcuts both measured the same capped frame.

Captures (all `/tmp`, not tracked):

- `ticket20-{general,harnesses,appearance,updates,diagnostics}-{dark,light}-960.png` — all five panes at minimum size, scrolled to end where applicable.
- `ticket20-general-dark-1280.png`, `ticket20-windsurf-{dark,light}-960.png`.
- `ticket20-first-run-960.png`, `ticket20-shortcuts-dark-960.png`.
- D48/failure captures named in the findings and coverage above.

Screenshots of minimum-window General and Diagnostics were also visually inspected; no clipped controls, horizontal spill or unreadable helper text was found. Native release-window, VoiceOver and real keychain/zoom/opener testing remain operator smoke work, not silently inferred from the browser.

## Summary

**FAIL:** the approved Settings dialog, five panes, legacy-link cleanup, focus/layer behavior, theme-radio synchronization, minimum-window layout, reduced motion, generated placement data and Projects rename are substantially correct and freshly verified. However, the new queued auto-save implementation can persist a different effective harness set from the one shown after an error (P0), D48 offers impossible all-skipped deployments to undetected harnesses (P1), and superseded toggle intent can still prompt and deploy after the harness was turned off (P1). Fix those async/eligibility defects and add the adversarial regression cases before approval; the static shortcuts tabpanel issue is a smaller P2 follow-up. No source fixes or commits were made by this reviewer.

## Re-check (2026-11-04)

**Verdict: FAIL — F5 must be reconciled with the orchestrator's explicit D19 decision.** The original F1–F4 reproductions are closed, including the deliberately changed expectation for an activation of a held switch. One narrower, P2 coordinator-boundary defect remains: a refused call still changes the pending D48 intent. This is **not** a claim that the ordinary held switch still calls `noteHeld`, nor that the old saved-off/deploy defect persists.

Scope: bounded re-check of the uncommitted `t3/settings-dialog` worktree at `/Users/alexstark/.t3/worktrees/skills-hub/t3-d7f3657a`. Read AGENTS.md, the original F1–F4, the ticket's Fix pass and its last orchestrator Comment. That Comment overrides the fixer's earlier claim that a refused gesture should withdraw an ask. No source changes, fixes, commits, native Tauri runs or operator-library operations. This appended report is the only file edit. Browser: my own Vite fixture on **5194**, ego-browser task space **19**, rich/failures with explicit latency; the Updates no-control check used the fixture's canned `update=9.9.9`, never a real update.

### F1 — CLOSED

**Failure injection, freshly repeated:** rich, `latency=10`; enumerable getter/setter on `window.__fixture.state.settings.global_selected_tools`, first write rejected with `{ code: "OTHER", message: "recheck: first settings write failed" }`. Cursor off, then Pi activation **21.2 ms** later:

```text
writes: exactly 1
  [claude_code,codex,amp,kimi_cli,pi,gemini_cli] — rejected
Pi at activation:
  aria-disabled=true
  title="Harness settings change is running — wait for it to finish"
settled persisted selection:
  [cursor,claude_code,codex,amp,kimi_cli,pi,gemini_cli]
settled UI: Cursor=true; Pi=true
toasts: exactly 1 error — "recheck: first settings write failed"
```

No second write was queued. UI and persisted selection agree after settlement. An initial run also passed but its requested 20 ms delay took 121 ms under concurrent tests; the run above was repeated after those commands finished to establish the requested timing.

**Hold coverage:** failures fixture, with Windsurf's selected-undetected row expanded. During saving, every harness switch, Remove from selection and the scan-scope checkbox had `aria-disabled=true` and the Harness settings change reason. During the subsequent scan, fresh observation of Pi, Remove from selection and scan-scope checkbox showed `aria-disabled=true` and **“Scan is running — wait for it to finish”**. Activating the held Remove/scan controls left Windsurf selected and scan-only true. General's Auto-deploy, Browse… and Clean Now carried the corresponding save/scan reason too; refused activations left Auto-deploy true, library path `/Users/alex/.skillshub` and cache-entry count 6 unchanged. Disclosures, pane navigation and the detected-only view filter remain usable, correctly: those are not mutations.

Zoom and both cache-number inputs remained enabled in both phases. **Acceptable split for this ticket, not a new finding:** they write separate preference keys and do not relocate the library, clear the cache or change selection. A blanket statement that *all* Settings writes use the reporter would still be false. The previously recorded inherited fact also remains: Browse/Clean are held by the UI but their own actions do not enter `runAction`; this re-check does not expand into repairing that older lifecycle design.

**Source:** `useSyncOrchestration.ts:720–755` runs saving under `runAction`, changes configuration only inside the admitted body, adopts `updateSetting`'s returned `global_selected_tools` / `scan_selected_tools_only`, rolls back its captured configuration on failure and uses `notifyError`. `readSavedSelection` reads the synchronous config ref. Literal search for `queue|Queue` found only explanatory comments here (the permitted import-plan **read** waits), not a private write queue. The fresh full test run includes the real-reporter failed-first-save/refused-second-toggle, backend-echo and save→scan→deploy regressions.

### F2 — CLOSED

Fresh rich fixture, Auto-deploy on, detected-only filter off, Augment on:

```text
installedTools includes augment: false
persisted selection includes augment: true
Augment switch: true
D48 asks: 0
skills with an Augment target row: 0
```

Detected control: Gemini CLI off and fully settled, then on. The ask was **“Deploy your 3 skills to Gemini CLI now?”**. Independently filtered the fixture's DTOs for enabled, locatable skills without a disabled/synced/stale Gemini target; the exact eligible names were **prisma-review, effect-ts-basics, using-git-worktrees**. No borrowed count or all-skipped proof. `useHarnessSelection.ts:90–102` checks current catalog installedness and Auto-deploy before asking and again after `whenIdle`, and derives N from current skills.

### F3 — CLOSED for the original defect and requested UI sequences; see new F5 for the callback boundary

- **On→off at ~100 ms, off refused:** rich `latency=10`, Gemini initially off and settled. Second activation at **100.9 ms**, `aria-disabled=true`, Harness settings change reason. Exactly **one** persisted selection write (`gemini_cli=true`), saved/UI on, and **one** D48 ask with N=3. This is the expected result under the orchestrator's last Comment, not a regression.
- **Off actually saved:** after that on-save and scan settled, dispatched the switch's DOM click while the ask was open. This deliberately exercises the coordinator behind the inert Settings layer; it is not a claim that an ordinary pointer can reach through a modal. Before: saved on, switch not held, one open ask. After the admitted off-save: saved false, UI false, **zero asks**, and all three eligible Gemini target rows still absent. The open ask was dismissed and nothing deployed.
- **Rapid on→off→on:** fresh rich `latency=5`, initially off and settled, three activations ~100 ms apart. The overlapping activations were held. MutationObserver saw **one ask**, persisted-write interceptor saw **one on-write**. Confirming once yielded **one “3 deployed”** completion; each of the three independently identified eligible skills gained exactly one Gemini target row. No second ask or completion appeared. The hook tests separately cover admitted superseding saves; the app does not queue those overlapping gestures.
- **Auto-deploy becomes off during the save:** exercised the production Settings callbacks with an already-in-flight policy write (not by claiming the held General switch is operable). Auto-deploy=false settled at **301.3 ms**, while the harness selection save was pending; Gemini=true settled at **479.4 ms**. Final policy false, selection true, **zero asks**, all three target rows absent. The fresh hook test also explicitly rerenders policy off while a deferred selection save runs.
- Source checks confirm both pre-ask and post-`whenIdle` saved-selection/generation/policy/installedness checks. `rg` found **no `noteHeld` or `HELD_REFUSAL_EVENT`** in `useHarnessSelection.ts`, `HarnessesPane.tsx`, or `App.tsx`. The UI held-event listener really is gone. However, removal was incomplete at the coordinator's *own* refusal boundary (F5).

### F4 — CLOSED

Fresh real browser key input:

```text
Keyboard shortcuts → ArrowDown → Tab
  selected pane: My Skills
  active element: role=tabpanel, tabIndex=0
Tab
  active element: button, aria-label=Close
Escape
  focus returns to Keyboard shortcuts; dialog count=0
```

Settings General, which has controls: panel `tabIndex=-1` (8 input/select/switch/button elements). Updates with Check likewise had panel `-1`. For a genuine no-control state, selected the fixture release and began its canned install; while the panel read **“Installing...”**, it had **0 buttons**, `tabIndex=0`, and Tab from Updates focused the **tabpanel**. The instant no-release fixture check settles too quickly for a reliable keyboard capture, so that first attempt was not counted as no-control evidence.

Source: `PaneSpec.static` flows to `TabsContent.staticPane`; both shortcuts panes opt in, Updates derives it from `updatesPaneHasControls`; the primitive preserves `-1` for panes with controls. Fresh primitive/fold tests passed.

### F5 — P2: a refused coordinator call still changes intent and suppresses the saved-on ask

**Locations:** `src/hooks/useHarnessSelection.ts:75–86,90–91`; `src/hooks/useHarnessSelection.test.ts:260–294`. Related stale comments: hook `:46–53,67–68`, `HarnessesPane.tsx:24`, `SettingsDialog.tsx:42`.

Removing `noteHeld` fixed the normal held-switch path, but `select()` still writes `lastGesture` **before admission**. For a refused off-call it removes only an on-generation from `ons` (a no-op for off), leaves `lastGesture.selected=false`, and can also call `dismissDialog("deployHarness")`. The pending successful on then fails `stillWanted()` even though its saved selection is still on. The test named **“an off refused while the on's save runs still withdraws the on: the operator's last word was off”** explicitly pins the behavior the orchestrator rejected. Its neighboring refused-off/refused-on test also relies on refused calls counting as intent.

**Fresh evidence, production coordinator boundary:** on rich `latency=5`, first saved Gemini off and waited for scan settlement. Read the mounted SettingsDialog's production `deploy.onSelectedChange` callback through its React fiber; called it with `(gemini_cli, true)` and immediately `(gemini_cli, false)` before the first completed. No source edits or mocked coordinator. Intercepted the fixture selection property to count persisted writes:

```text
persisted writes: [true]             # exactly one, the admitted on
refusal toast: Harness settings change is running — wait for it to finish
settled saved selection: on
settled switch: on
D48 asks: 0                          # refused off suppressed the on's ask
```

This is a **callback-level probe, not an ordinary-pointer reproduction**: the held primitive correctly blocks the 100 ms UI activation, and even two same-turn DOM switch clicks produced the correct single ask in a separate control run. Severity is P2, not the original P1 unintended-deployment bug: this path loses an offer; it does not deploy after a saved off or corrupt persisted selection. Nevertheless it blocks this re-check's acceptance because the explicit decision was **a refused activation changes nothing**, not merely “remove one DOM listener”, and the code plus regression test still intentionally implement the opposite at the coordinator boundary.

**Required correction (not implemented here):** make coordinator refusal leave the previously admitted intent/ask untouched. Do not let a refused generation supersede another request or dismiss its ask; account for newer admitted generations when handling async settlement rather than blindly restoring stale state. Replace the two refusal-as-intent expectations with saved-on → exactly one ask (and a refused off leaves an existing ask intact), and remove the contradictory comments. Preserve the already-passing behavior for a genuinely saved off, current policy/installedness and the held primitive.

### Fresh commands and spot checks

Run in this worktree, after the orchestrator's removal of `noteHeld`:

```text
npx vitest run src
  Test Files 55 passed (55)
  Tests 1196 passed (1196)
  exit 0
  non-failing Node warning: localStorage unavailable without --localstorage-file

npx tsc --noEmit -p tsconfig.app.json
  exit 0; no diagnostics

npm run styles:check
  exit 0
  App.css 561 counted lines (ceiling), 48 migrated files
  prose scoped, no raw colours, self-test passed

git diff --check
  exit 0; no output
```

The current count is **1196**, not the fix pass's earlier 1198; the removed held-event tests explain the different snapshot. Green tests do not resolve F5: one of those green tests asserts the rejected policy.

Focus/hash spot checks, once per requested entry:

- Configure harnesses → Settings → Escape: no dialog, focus **Configure harnesses**, hash `#/skills`.
- Sidebar Settings → open → Escape: focus **Settings**, hash unchanged.
- Keyboard shortcuts → open → Escape: focus **Keyboard shortcuts**, no dialog.
- Set `#/settings/github` once: exactly one Settings dialog, **Diagnostics** selected, `#settings-github-token` focused, hash cleaned to `#/skills`. Escape left it closed and the hash clean; no reopen.

No fresh claim is made about full Rust/clippy/build, native keychain/updater/zoom, or VoiceOver; they are outside this bounded command/browser pass. Browser and owned Vite are cleaned up at completion.

**Final re-check verdict: FAIL.** F1, F2, F4 and the original F3 UI/saved-selection defect are closed with fresh evidence. The remaining blocker is narrowly **F5: reconcile the coordinator's refused-call behavior and its tests with the orchestrator's D19 decision**. No fixes were made.
