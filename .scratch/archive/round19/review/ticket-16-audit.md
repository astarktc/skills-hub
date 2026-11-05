# Ticket 16 — independent D31 audit

**2026-09-30 · `f5615fd6ae540dec2c045ee91f1db6faf3bded80` · FIX-THEN-PASS.**

**Three P1 findings, no P0; one P2 note.** The normal Add path works, but nested-layer dismissal, aggregate retry completion and an in-flight source change violate the gate. Green tests and zero axe violations do not close these findings.

## Method and evidence

Read AGENTS.md, BACKLOG/newest handoff, ticket 16 Work/Gate/Result/Comments and handoffs, the requested spec decisions, DESIGN Add/Motion, audits 11–15's methods/floors, the commit diff and the settlement, reporter, platform, shared-confirmation and project seams. Standards and spec were reviewed in this terminal auditor, without further delegation. Applied Impeccable audit, Web Interface Guidelines fetched afresh, React/shadcn guidance, animate and Emil's review framework. The pinned desktop minimum, dense 24px target floor, limited hash policy, EN-only catalog and existing motion tokens override generic website guidance.

**E = `.scratch/round19/evidence/16-audit/`**, gitignored. Existing fixture port **5175 returned HTTP 200**. No live Tauri launch, native library/database mutation, source edit, stash, revert or commit. Started at the audited HEAD with a clean tree. The only tracked writes are this report and ticket 16's Comments entry.

- One named isolated headless **agent-browser 0.31.1** session, `r19-16-audit`, using the supplied trusted CDP key/text driver, never the broken CLI `press`. Main configurations: **failures, dark 1280×800 / light 960×640**. Navigation uses a fresh nonce to reset the in-memory fixture.
- Read/copied the implementer's drivers, changed their evidence/session plumbing, and **executed them anew**. `gate-*`: uninterrupted keyboard Add/Tool scope/ask/retry/close. `flows-*`: keyboard Cancel and local source, plus explicitly supplemental pointer/programmatic opening of Explore and Re-point; the latter now continues through confirmation to an actual `repointSkillSource` call. Primary gate/adversarial transcripts contain **no injected `.focus()`**.
- Independent `adversarial.ts`: shared-folder layering; installation while a checked candidate remains filtered out; partial-report Retry; source-tab change during a delayed listing. `closing.ts` isolates clean and Keep-only completion. Its writable-directory setup, and the Retry probe's simulated permission repair, mutate **only `window.__fixture.state.unwritableTools`**; they are explicitly supplemental filesystem-condition models, not native tests.
- `trace.ts` uses a non-pausing CDP conditional breakpoint at the fixture command seam to record **command arguments without replacing handlers**. Gate traces show one install batch, one collision retry batch, then one failed-candidate install retry. Initial function-breakpoint instrumentation recorded nothing and was replaced; repeated-navigation tracing was corrected to disable the preceding debugger before attaching the next. Completed final JSON is authoritative, not an empty/failed exploratory trace.
- `a11y-*`: **12 fresh axe 4.10.3 runs**—Source, Skills, Tools popover, overwrite ask, Result and Re-point picker in each configuration. Actual computed colours are resolved through canvas and composited over ancestor backgrounds for WCAG relative-luminance measurements. Disabled text is excluded; selected marks and unchecked checkbox edges are measured.
- `motion-*`: explicit normal/reduced media, rAF/WAAPI samples, both configurations. Screenshots inspected include shared-confirmation layering, minimum-size Result before/after Retry, Source, candidates, local invalid candidates and Re-point. Shell calls were bounded; browser matrices ran sequentially in the same session.

This is Chromium fixture evidence, **not** a VoiceOver, WKWebView/WebView2, physical-device or native minimum-window certificate. D31's operator release smoke remains separate.

## Per-clause verdict

| Clause | Verdict | Evidence / qualification |
| --- | --- | --- |
| One Add dialog, Source → Skills → Result, sticky summary | Pass, ordinary flow | `gate-*`; one `addSkill` kind, step internal to the hook. The source-change race is A3. |
| Source tabs / URL field / local source | Pass, ordinary flow | Real `role=tab` and `aria-selected`; keyboard Local selection in `flows-*`. In-flight tabs are incorrectly still changeable: A3. |
| Search, checkboxes, “3 of 4 checked · 1 hidden by filter” | Pass | Gate filters, clears and toggles a row. Independent `filtered-*` actually submits while filtered; hidden checked `skills/changelog-writer` is **absent** from `installSelection.candidates`. |
| Keyboard Edit Tools, visible Checkbox primitives | Pass for nonshared Tools; **fail for shared Tools** | Gemini CLI toggles by Space, Escape returns to Edit Tools. No `display:none` input found in sampled Add/picker/popover DOM. Shared Amp/Kimi confirmation is A1. Base UI's visually hidden form input is not mistaken for the visible role-checkbox. |
| Exactly one overwrite ask, two rows, per-row choices | Pass | Full ask text records both skill → Claude Code paths, “Modified 3d ago · differs from the skill”. One sync retry call contains both exact pairs, one `overwrite`, one `keep`, no unrelated targets. |
| Added / kept / failed / cancelled results | Pass | Ordinary run: three Added, one typed RATE_LIMITED failure + Retry. Keep is a quiet declined deployment, not a failed candidate. Separate Cancel run: one Added + three Cancelled, with Add actions. These are separate cases, not a claim all four kinds occur in one four-row run. |
| Retry sends failed id only; “Added 4 of 4”; Close returns to Add | Pass for normal run | Both `gate-*`: first install has all four subpaths, second only `skills/release-notes`; four Added rows remain because Amp deployment still fails. Close → Add skill. **67 recorded stops each, zero BODY, all sampled stops focus-visible.** |
| Full clean success closes; Keep counts as success | Pass in isolation | `closing-*`, real fixture handlers with writable dirs: ordinary success and Keep-only success close to Add skill. Kept Claude Code target is `disabled`, `synced_at:null`. Aggregate retry policy fails A2. |
| Failure / cancel / unanswered collision keeps Result | Pass initially; **fail after a later clean Retry** | Initial Result stays; A2 proves a subsequent successful candidate hides earlier unresolved rows. |
| Escape on overwrite ask preserves Add/draft | Pass | `flows-*` and filtered probes: only overwrite dialog closes, Result/source/rows remain, no overwrite command sent for dismissal. Focus returns to Result heading. |
| Cancel policy D20 | Pass for tested boundary | Keyboard Cancel shows cancelling state; batch returns one Added + three Cancelled and stays open. Source shows guard checks before acquire/finalize, never interruption inside finalize/sync; fresh Rust suite includes cancellation while acquiring and waiting for the guard. No native timing claim. |
| Local invalid candidates / labels / optional name | Pass | `flows-*`: draft-notes/scratch disabled, tabindex −1, reasons in words; Tab skips them. Controls named by candidate. Name is on Skills for a lone listing only, not a filtered single row from a multi-listing. |
| Explore intent / preselection / no implicit install | Pass | `flows-*`: prefilled `some-org/ui-ux-pro-max`, Skills step, one checked candidate; no install call before explicit action. Code removes the render-cycle trigger and target reset. |
| Re-point single-choice picker and wire target | Pass | `flows-*`: radiogroup, four radios, search “notes” → one shown, select release-notes, Check this skill → named mismatch confirmation → `repointSkillSource(id, {kind:"git",url,selection:{subpath:"skills/release-notes",resolution}}, policy)`. |
| One action per install/retry, one fold, refusal, progress | Pass for reviewed/tested scope | `runBatch` is one `runAction`, `installBatch` one command; collision settlement stays inside it. Reporter second-mutation-refusal tests pass freshly. Progress is in Add summary, `progressInDialog` includes Add, header yields. No second browser mutation was forced through an inert background. |
| Motion / reduce / contrast / axe | Pass for measured scope | Tables below. These checks do not excuse A1/A3 focus defects. |
| Command removal / generated bindings / styling / i18n | Pass | Architecture inventory and gates below. |
| Minimum-size usability | Pass with P2 density note | 600×592 dialog at 960×640; fixed summary/footer, body scrolls 40px for four tall Added rows, no horizontal overflow. A4 recommends removing repetition, not truncating reasons. |

## Findings

### A1 — P1: shared-folder confirmation consumes the popover and loses the checkbox return target

**Locations:** `src/components/skills/add/AddSkillDialog.tsx:220–247`; `src/components/ui/popover.tsx:14–16,35–58`; `src/components/ui/layer-stack.ts:106–118,126–149`.

**Reproduction:** Add → keyboard Shift+Tab to Edit Tools → Enter → Tab to Amp → Space. Shared skills directory opens over the popover and Add. Press **Escape once**. `shared-dark-1280.json` / `shared-light-960.json` show both the confirmation **and** popover gone; focus is **Close** in Add, not Amp. Reopen Edit Tools, toggle Amp again, press Continue: the group updates, but the popover again disappears and focus returns to **Edit Tools**, not its checkbox.

The modal-only layer stack does not coordinate the portalled Popover's dismissal/return scope. `staysInertAfterClose` considers only containment in the remaining modal popup; the checkbox is in another portal. The popover also handles dismissal/focus-out independently. The screenshot shows the correct visual stack, but that does not prove the required unwind.

**Impact:** violates the explicitly requested Escape order and D21 focus continuity, interrupts a multi-Tool edit and makes a following Space activate Close rather than the intended checkbox. The page is recoverable and not stranded inert, so P1 rather than P0.

**Concrete fix:** coordinate floating-layer ownership, topmost dismissal and return-focus eligibility in the primitives/layer seam; keep Edit Tools mounted while its shared confirmation is open. After Escape/Cancel/Continue on the ask, return to the originating checkbox, with the popover still open. The next Escape closes only the popover → Edit Tools; the next closes Add → Add skill. Preserve `useSharedDirConfirmation` as the sole policy owner, not a second Add-specific ask. Add a primitive-level nested-portal regression and browser recheck. Update the “only layer above it” doc wording to acknowledge this existing shared-folder confirmation path.

### A2 — P1: a clean candidate Retry auto-closes an aggregate Result that still needs attention

**Locations:** `src/hooks/useAddSkillFlow.ts:533–540`; `src/lib/addFlow.ts:236–242`; related correct per-run rule `src/lib/reportOutcome.ts:1245–1257`.

**Reproduction:** failures → add all four collide-kit candidates → Escape the two-row overwrite ask. Result contains three Added rows (including two folders to review and Amp target failures) plus failed release-notes. Simulate repairing directory permissions **without resyncing existing rows** (`__fixture.state.unwritableTools=[]`), then keyboard Retry release-notes. Both `retry-dark-1280.json` and `retry-light-960.json` record a one-candidate retry and then **zero dialogs**, focus Add skill. No collision-resolution or resync call ran for the old rows.

`installOutcome(settlements, …)` correctly judges the **current retry only** clean. The extra close guard, `settlementComplete(current.entries)`, checks only that each candidate has `status === "installed"`; it ignores previously stored deployment failures and unanswered collisions. Thus the complete surface is not clean even though the latest run is.

**Impact:** loses the visible unresolved Result contrary to the explicit close rule. Data is not overwritten and historical reports still exist, but a successful unrelated retry falsely dismisses the outstanding work.

**Concrete fix:** centralize a complete-install settlement predicate with the outcome policy and apply it to **all accumulated entries** before closing. Continue emitting one report for the current retry only—do not duplicate earlier outcomes/toasts just to obtain aggregate completion. Preserve closure when all entries are clean or deliberately kept. Add hook tests for prior TARGET_EXISTS, prior target failure, prior no-targets, cancelled remainder, Keep-only success and final clean Retry.

### A3 — P1: changing the source tab while listing produces an empty Skills step and BODY focus

**Locations:** `src/components/skills/add/AddSkillDialog.tsx:294–297,64–69`; `src/hooks/useAddSkillFlow.ts:387–446,628–631`.

**Reproduction:** `?scenario=failures&latency=4`, dark 1280×800 and light 960×640. Open Add, enter collide-kit, Enter to list. Before it finishes, Shift+Tab to Git repository, ArrowRight to Local folder, **Enter to select it**. Both tabs are enabled; DOM records Git `aria-selected=false`, Local `true`. When the Git listing completes, it unconditionally changes the step while `pick` now reads the unlisted Local picker. `race-dark-1280.json` and `race-light-960.json` record Skills with source/count “Not chosen yet”, legend “Skills in”, an empty-query “No skill matches” message, and focus **BODY**, root still inert. Matching screenshots preserve the empty step.

The source input alone is read-only during listing; the tab and `setKind` intent are not guarded. Neither first-focus path has a target when this picker has no candidates/search. Add selected also returns silently because `pick.ctx` is null. Back/close offers recovery; no wrong source was installed in this reproduction.

**Concrete fix:** freeze source-kind changes while the listing owns the action (both accessible control state and intent guard), or explicitly invalidate/supersede the old listing before accepting the new kind. Admit a listing only for its current source/intent. Give an empty/unselectable step a valid focus target rather than BODY. Cover Git→Local and Local→Git while deferred listing resolves/rejects, plus Cancel and subsequent successful listing.

### A4 — P2: Result repeats the same full target failure on every Added row

**Locations:** `src/lib/addFlow.ts:168–207`; `src/components/skills/add/AddSkillDialog.tsx:559–565`.

**Reproduction:** ordinary failures gate, then Retry release-notes. Four Added rows each repeat the full Amp permission message and path; three similar explanations precede the actionable acquisition failure before Retry.

**Impact / ruling on concern 1:** **usable at 960×640, but unnecessarily dense**. Fresh geometry (`gate-light-960.json`): dialog y=24..616, width 600; body y=169..550, client height 381 / scroll height 421; width 598 / scroll width 598. Summary and footer stay fixed, Close stays reachable, and the result is not horizontally clipped. This is not a P1 layout failure.

**Concrete improvement:** retain a short row-level deployment count/Tool association, and present a shared typed reason once with affected names/count, or an accessible detail disclosure. Do not clip away the reason or replace it with an unexplained number. Candidate acquisition failures must keep their local reason and Retry. Defer if desired; it does not independently block acceptance.

### Required Before / After summary

| Before | After | Why |
| --- | --- | --- |
| Shared ask Escape also dismisses Tools | Ask → checkbox; next Escape → trigger | One key dismisses one layer. |
| Latest successful Retry plus “all installed” closes everything | Aggregate completion includes outstanding deployment state | A new success does not resolve an older collision. |
| Tab switches while old listing commits | Freeze/supersede source intent and admit only current listing | Step and data must describe the same source. |
| Four copies of a permission paragraph | Row counts/association plus one typed explanation | Preserve recovery information without burying the failed candidate. |

## Verdicts on the implementer's three concerns

1. **Tall Result rows:** approve functional minimum-size behavior; **P2 A4** recommends deduplicating repeated deployment prose. Per-row association is useful; repeating the whole permission remediation is not.
2. **Shared-folder Tools:** **not approved, P1 A1**. The policy owner remains correct (`setTool` → sync `handleSyncTargetChange` → `useSharedDirConfirmation` → surface ask); the UI layer integration is wrong.
3. **Extra edits:** **acceptable and bounded**. ToolConfigModal migration removes the remaining `pick-*` users without changing its selection/filter/confirm contract. Project configure still runs through `useProjectActions.handleToolConfigConfirm` under `runAction`; `useProjectState.configureTools` consumes `{ view, report }`, applies the view, and the action folds `projectRemovalOutcome`. This was source/test verification, not an independent full Projects browser audit. `syncAfterCreate` has only Add's new popover caller; the new copy accurately describes it. AGENTS' single sentence truthfully documents the removed install commands. None warrants reverting the migration.

## Motion, contrast and accessibility

The landing is a small state-indication event at the explicitly permitted rare-install frequency. It uses the existing token, not a new timing system. Words and controls are immediately available; glyphs alone fade. Native dialog/popover/tab changes remain instant. Existing progress-fill/spinner motion still exists while running—“nothing else animates” should mean **no additional decorative surface motion**, not literally no activity indicator.

| Measurement | Dark 1280 | Light 960 |
| --- | --- | --- |
| Landing property / duration | opacity / **120ms** | opacity / **120ms** |
| Easing | `cubic-bezier(0.16,1,0.3,1)` | same |
| Measured two-row stagger | **0 / 20ms** | **0 / 20ms** |
| Reduce: first settled-glyph sample | opacity **1**, no transition | opacity **1**, no transition |
| Running row animations after settlement | **0** | **0** |

Source caps delay at `min(index,8)*20ms`; DESIGN's install-landing row matches the implemented 120ms sequence. This audit sampled two settled glyphs and verified the cap in code; it does not claim an eight-row timing recording. The generic skill's absolute keyboard-motion ban is subordinate to the project's explicitly approved install-landing contract. No scale, layout animation, `transition:all`, new library or call-site UI-primitive transition override was introduced.

**Fresh minimum rendered enabled-text ratios:**

| Surface | Dark | Light |
| --- | ---: | ---: |
| Source (Continue initially disabled) | **6.61** | **5.32** |
| Skills / sticky summary | **4.68** | **5.32** |
| Tools popover | **7.78** | **8.36** |
| Overwrite ask | **4.68** | **5.86** |
| Result | **4.68** | **5.32** |
| Re-point picker | **5.73** | **6.24** |

Selected-tab ring/ground: **6.01 / 5.04**. Checked checkbox boundary/ground: **3.83 / 5.86**, check ink **4.68 / 5.86**. Unchecked popover checkbox edge: **3.65 / 3.87**. Sampled settled Result glyph minima: **6.47 / 5.64**. Thus no measured enabled text below **4.5**, or identifying mark below **3**; prior floors are preserved. Transient fade opacity is not reported as settled contrast.

**Axe: 0 violations in all 12 runs.** Incompletes remain explicit: Base UI `aria-hidden-focus` guards (two on ordinary dialogs, eight with popover), one popover `aria-valid-attr-value` check and two overlapped contrast nodes. Manual computed-pair measurements cover the sampled visible text; zero axe does not certify the focus sequence or native accessibility. Primary normal gates have zero BODY samples; **A3 deliberately does not**. No blanket “focus never reaches BODY” claim.

## Architecture and command inventory

`architecture.txt`, `selected.diff`, `flow.diff`, `rest.diff` and current-source reads underpin these checks:

- **One Add kind:** no `gitPick`, `localPick` or `exploreInstallTrigger` in `src`. Two `useCandidatePick` instances remain internal pick-logic building blocks, not dialog owners.
- **Public Add interface:** `open`, `install`, `retry`, `add`; Import's existing interface is also still returned. The view's editing intents are **`setKind`, `editDraft`, `browse`, `list`, `back`, `setQuery`, `toggle`, `toggleAll`, `setTool`, `close`**, plus `install`/`retry`. App passes the view intact and binds entry intents; it does not independently manipulate draft cells. The view necessarily exposes draft values for rendering. A3 concerns admission/guarding, not a leaked App setter.
- **One overwrite owner:** Add collects install targets and delegates to `useSyncOrchestration.resolveGlobalCollisions`; `mergeInstallRetry` merges exact asked pairs. No new ask policy exists in Add. Gate's two same-Tool collisions become **one** `syncSkillsToTools` call with two overrides. Dismissal sends none.
- **One install per action:** recorded batch candidates 4 → retry 1; filtered batch 3, excluding the hidden checked row. `runBatch` wraps each invocation; `applyOutcome` is the execution door. A2 shows why aggregate completion needs its own correctly shared policy.
- **D19/D20:** `CANCELLABLE_COMMANDS` retains `installSelection`, removes `installGitSelection`; local legacy install was not a token-taking entry. Reporter refusal is tested in the fresh gate. Add withdraws cancellation for collision settlement and checks `cancelRequested()` before starting the resetting batch command.
- **Deleted end-to-end:** zero matches for `install_git_selection`, `install_local_selection`, `installGitSelection`, `installLocalSelection` in frontend, `lib.rs` or commands. Rust retains deliberately test-only staging/finalization helpers under `#[cfg(test)]`, not registered IPC doors. `InstallResultDto` is removed. Generated bindings commit delta is **0 additions / 12 deletions**; fresh export test leaves **no bindings diff**.
- **Styles:** zero `.pick-*` / `.tool-pill-toggle` definitions; **1676 counted lines / 29 migrated files**. Form rules still have Project/Invocation users, with ticket 17 handoff; removing them now would break another surface. New UI primitive call-site classes are layout/a11y only; native result glyphs/radio elements use tokens. Detector output `[]`; styling guard passes.
- **Copy/platform:** literal translation-key coverage passes; new `add.*` uses Add rather than Import. The existing inside-Tool-folder refusal intentionally directs the user to Onboarding **Import**, not a vocabulary regression. `pickFolder` and progress channel come through `lib/platform.ts`; no native plugin import introduced in the hook/components.

## Gates and source integrity

Fresh **`npm run version:check && npm run check` exited 0**, `E/check.log`:

- Version **1.2.18**, five locations consistent.
- ESLint clean; styling guard and its self-tests passed, **1676 / 29**.
- **44 Vitest files / 924 tests passed**.
- TS7 `tsc -b` type-check and Vite production build passed.
- rustfmt and clippy passed; **807 Rust tests passed**; binary/doc targets passed.

Additional fresh **`cargo test export_bindings`** and **`cargo test --all`** passed (`rust-extra.log`), with **807** library tests in the latter. `git diff --stat -- src/bindings/index.ts` is empty; `git diff --exit-code -- src src-tauri AGENTS.md DESIGN.md scripts` succeeds. A passing gate establishes structural integrity, not the D31 behavioral clauses above.

Indicative Impeccable score: accessibility **2**, performance **4**, responsive **3**, theming **4**, implementation integrity **2** = **15/20**. No performance benchmark or complete WCAG conformance is implied. Positive foundations: one backend batch, one collision owner, visible named controls, correct typed per-row outcomes, real retry identity and coherent tokens/motion.

## Acceptance

**FIX-THEN-PASS: A1–A3 must be fixed before ticket 16's D31 acceptance.** A4 is nonblocking. No product decision is needed to fix the three defects; the orchestrator may defer A4. Do not revert the accepted extra edits. Ticket status is left for the orchestrator; this reviewer only appends its finding to Comments.

### Exact bounded re-check scope

1. **A1, both main configurations:** Source and Skills → Edit Tools → Amp and Kimi → shared ask. Escape, explicit Cancel and Continue each return to the originating checkbox with Tools still open; Continue changes both shared members, Cancel/Escape neither. Then Escape closes Tools alone → Edit Tools; next Escape closes Add → its opener. Check Tab containment, one visible top-layer focus ring, no BODY/inert residue. Repeat ordinary overwrite-ask dismissal. If a shared primitive changes, rerun its layer/focus tests and the prior Enable-popover→ask recovery seam.
2. **A2, hook tests + both browser configurations:** partial install with old unanswered collision + clean Retry; old target failure + clean Retry; no-targets; cancelled remainder. Result must remain while any accumulated entry still requires attention. Clean-all and Keep-only outcomes must still auto-close; final clean retry sends only failed/cancelled ids and closes only when the complete result qualifies. One fold/history block for each new action, no replayed old toasts.
3. **A3, deferred hook tests + browser latency:** both source-switch directions while listing, resolution and rejection, Cancel followed by a fresh listing, and empty/all-invalid picker focus. Either prevent the tab change accessibly or discard the obsolete response; never advance to a different unlisted source, never BODY, no silent Add no-op. Keep the Source draft and normal first-control focus rules.
4. Rerun the uninterrupted Add gate in dark 1280/light 960 (including filtered actual submit, shared scope, mixed Keep/Overwrite, Retry and close). Repeat axe on affected Source/Tools/shared ask/Result states in both themes; recompute contrast only where styles change. Repeat landing/reduce only if motion owners change.
5. **`npm run version:check && npm run check`**, plus bindings drift if Rust changes. Operator VoiceOver/native-minimum-window/reduced-motion release smoke remains required separately.

## Re-check (2026-10-01)

**`01fd3d9ff29549807944d1770f801f67c5ce6930` — PASS for A1–A4's bounded acceptance. Two nonblocking P2 findings in the wider floating-layer review, below.** No remaining P0/P1 found in this re-check. This is not an unqualified certification of every Popover interaction.

### Method / fresh evidence

**E = `.scratch/round19/evidence/16-recheck/`** (gitignored). Started with HEAD at the requested commit and a clean tree. Existing fixture on 5175 returned 200. One isolated headless agent-browser session, `r19-16-recheck`, closed afterward; trusted CDP keys/text, never CLI `press`. Re-read the diff, layer/dialog/popover implementations and tests, current Base UI portal/tabbable implementation, both other consumers and the settlement/listing seams. No delegated children, source edits, Tauri launch, live-library writes, stash, revert or commit.

Copied and reviewed the implementer's drivers, redirected their evidence/session, and **executed them afresh**. `fix.ts` ran shared, retry, clean, race-git and race-local in dark 1280×800 and light 960×640. `gate.ts` ran once in dark 1280; `a11y.ts` produced six fresh runs. Independent `floating.ts`, `extra-browser.ts`, `probe-driver.ts` and `tabindex-driver.ts` exercised the wider seam. Pointer opening and the isolated primitive harness are explicitly supplemental to the keyboard Add gate. Fixture-only permission repair / foreign-directory injection model filesystem conditions; they do not bypass the command handlers.

Layer counts come from **the exact timestamped module URL imported by the running Popover**, exposed read-only as `__layers`; an initial unversioned import produced a different module instance and was discarded. `verify-evidence.ts` independently asserts the recorded A1–A4 outcomes, gate and six axe results; `evidence-verdict.txt` passes. Screenshots at 960 were inspected. Chromium evidence does not replace VoiceOver or native WebView release smoke.

### Findings re-checked

| Finding | Status | Fresh proof |
| --- | --- | --- |
| **A1 · P1** | **Closed** | `shared-{dark-1280,light-960}.json`: Source/Amp and Skills/Kimi × Escape, Cancel, Continue. Ask makes Tools inert; all six returns per theme land on the originating checkbox with Tools open/live. Tab reaches Kimi after Amp, Pi after Kimi. Escape/Cancel change neither member; Continue flips both. Next Escape → Edit Tools only; final Escape → Add skill, no inert residue. **105 recorded stops/theme, zero BODY.** Ordinary overwrite dismissal is also exercised by retry/cancel. |
| **A2 · P1** | **Closed** | `retry-*`: earlier two unanswered collisions plus Amp failures survive a clean release-notes-only Retry; Result remains “Added 4 of 4”, focus on its heading. `clean-*`: answered Keep plus final clean Retry closes to Add skill. `closing-dark.json`: clean filtered candidate and Keep-only run each close. `cancel-*`, both sizes: cancelled remainder stays visible. Main hook/pure tests cover prior failed target, no-targets, cancelled remainder and completing that remainder. One predicate serves current-run fold and accumulated entries; current action still folds once, without replaying old reports. |
| **A3 · P1** | **Closed** | `race-{git,local}-*`, `latency=4`: other tab receives focus with `aria-disabled=true`; Enter/Space do not select it. Correct source lands, search receives focus, Back restores switching. `extra-browser.json`: a single invalid local candidate lands on **Back**, not BODY. Deferred hook tests cover both attempted switches and resolve/reject. Supplemental `add-flow-extra.test.ts` adds **obsolete rejection after a fresh opening changes source**, proving the refusal is discarded and a fresh listing succeeds; all **52** copied/extended hook tests pass. A literal tab change during the running listing is now intentionally unrepresentable. |
| **A4 · P2** | **Closed** | `retry-light-960-{settled,after-retry}.png` and `a11y-result-light-960.png`: short row association (“not deployed to Amp”), one full permission reason and “Affects 3 skills” with names; failed release-notes retains its own RATE_LIMITED reason and Retry. No horizontal clipping; summary/footer remain available. Distinct-reason grouping is additionally covered by pure tests. |

### Floating-layer adversarial review

**Working guarantees:**

- Sidebar shortcuts: Escape returns to trigger; outside pointer press closes and leaves focus on the clicked control; Tab exits to Settings. Resizing 1280 → 960 → 1280 retains one floating record and a correctly placed panel; closing returns count to zero. Clicking Add while shortcuts is open closes shortcuts, opens one modal, then Escape → Add skill. There is no action *inside* the shortcuts help panel that opens a dialog.
- SkillRow Enable: Escape → original switch; outside press closes; resize keeps one record then closes cleanly. Actual Enable → overwrite ask → Escape returns to **retired-playbook's switch**, releases inert and leaves **zero** records (`floating-{dark,light}.json`). **Scope correction:** `SkillRow.tsx:292–295` deliberately calls `setAskingEnable(false)` before starting Enable. Thus its real overwrite ask does **not** sit above an open popover; expecting a surviving popover/control there would require a product behavior change. The live-popover-under-modal case is proven by Add's shared ask and the isolated current-primitive harness (`primitive-comparison.json`: floating/modal counts **1 → 2 → 1 → 0**, inert while covered, Escape → Ask control, next Escape → anchor).
- **One event, one owner:** `claimDismissal` checks top identity before claiming the native event in a WeakSet; removal cannot let the lower dialog claim the same event. A rejected lower handler does not consume it. Fresh stack/primitive tests and actual Escape sequences cover both modal-over-floating and floating-over-modal; no double-close or swallowed Escape reproduced.
- **Outside the whole Add dialog:** trusted pointer press at page/backdrop `(20,350)` closes Tools only, count **2 → 1**, focus Edit Tools; next Escape closes Add, count **0**. No background page click-through.
- **Unmount cleanup:** while the triggerless popover remains open after Tab-out (R1), a supplemental route change unmounts SkillRow; count **1 → 0**, no inert residue (`extra-browser.json`). Registration cleanup calls `closeLayer` synchronously. No stale floating record after unmount found.

#### R1 — P2, inherited: triggerless Enable has a hidden Tab stop and does not close on Tab-out

**Location:** `src/components/skills/SkillRow.tsx:272–278`; `src/components/ui/popover.tsx:119` (the anchor is supplied to Positioner, without a registered Trigger).

From focused Enable, Tab stops on an unnamed, `aria-hidden` inside focus-guard span. Subsequent Tab leaves the panel and starts at the page's first controls while the popover remains open and registered. One light run also sampled BODY during that exit; the reproducible core is the hidden stop and retained panel, **not** a claim BODY occurs every time. Escape and pointer dismissal still recover. `floating-light.json`, `extra-browser.json` and the minimal browser `primitive-comparison.json` preserve the sequence. The **pre-01fd3d9 Popover**, extracted into evidence without altering application source, reproduces the hidden guard and retained popup too: this is not introduced by floating registration.

**Concrete fix:** give the triggerless/anchor API a real focus-manager reference/return position (rather than positioning alone), preserving the Switch's enable/disable policy. A primitive/browser regression should require forward Tab from the last control to the next page control, reverse Tab out to the prior control, popup dismissal and zero records, with no focusable hidden guard. Carry as an Enable/primitive follow-up; it does not reopen A1.

#### R2 — P2, new shared-seam edge: restoring stale tabindex can re-tab a newly disabled checkbox

**Location:** `src/components/ui/popover.tsx:73–81` (`restoreTabOrder`).

The restore is not an unconditional `tabindex=0`: it selects only `[data-tabindex]` within the nearest portal, restores the saved string (including `-1`), or removes the temporary attribute. Base UI's `disableFocusInside` marks previously tabbable nodes only. Accordingly, an **already-disabled** Checkbox stays `-1`, and plain text stays without tabindex.

However, a previously enabled Checkbox can become disabled **while the modal is above it**. A supplemental harness using the real primitives opens the modal, then its Return button disables that checkbox and closes the modal. Restoration writes the old **0** over the new **−1**: `aria-disabled=true`, `tabindex=0`, and the next trusted Tab focuses it (`tabindex-safety.json`). This is a real shared-primitive boundary defect, not an observed failure of the current Add scope (its Tool controls do not change disabledness during the shared ask). No accidental activation or data mutation was observed.

**Concrete fix:** restore only temporarily removed tab order without overriding the control's **current** disabled/roving-focus contract; don't indiscriminately exclude every `aria-disabled` control, since explicitly focusable-disabled controls are supported. Add a primitive regression for enabled → disabled while covered, alongside already-disabled, intentionally focusable-disabled, explicit `-1`, and noninteractive controls. Retain the browser null-`relatedTarget` proof. Recommend fixing before extending this seam to more dynamic popovers; no current Add blocker demonstrated.

### Implementer's concerns

- **(a) Other consumers:** checked above, including real Enable → overwrite recovery. No new inert/unmount/double-dismissal failure. R1 is inherited debt, now independently evidenced.
- **(b) Base UI Tab workaround:** necessary for the reproduced A1 browser sequence, and it matches Base UI's attribute protocol. Accept its purpose, **not an unconditional safety claim**: R2 needs a current-state-aware restore. Pin the browser regression when upgrading Base UI; this is an internal attribute dependency.
- **(c) `innerHTML` diagnostics:** nonblocking test-only false positives for this change. The additions assign a fixed local markup constant; cleanup assigns an empty string. No untrusted value, app execution path or HTML injection sink is introduced. Do not change product code to satisfy this warning.
- **(d) jsdom synthetic attributes:** acceptable as a focused unit test of restoration, **not** evidence that Base UI creates/removes those attributes correctly in a browser. The fresh Amp/Kimi return-and-Tab transcripts supply that evidence. Extend the test for R2; the current positive case is insufficient for dynamic disabledness.

### Gate / integrity

`gate-dark-1280.json`: **67 keyboard stops, zero BODY, every stop focus-visible**; collide-kit → Tool scope/search → exactly one two-row ask → mixed Keep/Overwrite → four Result rows → failed-id-only Retry → Close → Add skill. Trace records two `installSelection` calls and one exact-pair `syncSkillsToTools` retry batch. `cancel-*` additionally proves overwrite Escape preserves Add and cancelled rows.

**Six fresh axe 4.10.3 runs, zero violations:** Source while frozen, shared ask over Tools, Result × both themes. Incompletes remain explicit: Base UI `aria-hidden-focus` guards and one overlapped contrast node. Sampled minimum enabled-text contrast **4.68 dark / 5.32 light**, no measured pair below 4.5. No motion owner changed, so prior landing/reduce evidence is retained rather than relabelled as fresh.

**Fresh `npm run version:check && npm run check`: exit 0.** Version **1.2.18**; lint, styles (**1676 / 29**), **44 Vitest files / 946 tests**, TS7 type-check/build, rustfmt, clippy and **807 Rust tests** pass (`check.log`, `check-final.log`, latter records `GATE_EXIT=0`). Separate `bunx vitest run src/components/ui/`: **8 files / 72 tests**, including every `ui/*.test.tsx`, pass (`ui-tests.log`). Supplemental obsolete-listing tests: **52 pass** (`extra-tests.log`). Existing build chunk/dynamic-import warnings are nonblocking. Fresh source/bindings diff checks are empty; Rust export left bindings unchanged. Only this report and ticket Comments are tracked writes.

### Acceptance

**PASS — closes A1–A4 for ticket 16's bounded D31 re-check at `01fd3d9`.** Covers both-theme shared confirmation return/Tab/unwind, aggregate Retry and clean/Keep/cancel completion, delayed source admission/focus, minimum-size deduplicated Result, one fresh dark keyboard gate, six axe checks and the full automated gate. The floating seam's top-layer dismissal, inerting, real Enable overwrite recovery and unmount cleanup pass within the exercised scope.

**Nonblocking residue: R1 and R2 (P2).** The orchestrator should decide whether to fix R2 now or explicitly carry it with R1 as a primitive/Enable follow-up; neither is silently waived or represented as passing. No product decision is needed for A1–A4. Ticket status/closure remains the orchestrator's. Operator VoiceOver, native-minimum-window and reduced-motion release smoke remain separate requirements.

### Residuals (2026-10-01, orchestrator)

- **R2 fixed**: `restoreTabOrder` now undoes only the temporary removal — a control that is natively `disabled`, or `aria-disabled` without the primitives' new `data-focusable-when-disabled` mark (Button `pending` / `focusableWhenDisabled`, the held Switch), is never put back in the tab order; a marked focusable-disabled control is restored; an explicit `-1` stays; non-interactive elements get nothing. Primitive regression `restores only the temporary removal, never a control's current disabled contract (R2)` covers enabled→disabled while covered, always-disabled, held, roving `-1`, plain text. DESIGN.md § Focus notes the mark. Gate: vitest 947 / cargo 807 / 1676, exit 0.
- **R1 carried** to ticket 17's Comments (the triggerless Enable popover's hidden focus-guard Tab stop + staying open on Tab-out — the `anchor` API needs a real focus-manager return position). **D31 gate passed** for ticket 16.
