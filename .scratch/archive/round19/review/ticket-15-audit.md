# Ticket 15 — independent D31 / Emil audit

**2026-09-30 · commit `34a7624763ead34d442be9624b0941729d87d784` · Acceptance: FIX-THEN-PASS.**

One P1: N4 loses a second arrival when its words equal the preceding arrival. No P0. Motion, contrast, primitive refusal, N2 reparenting and N5's single accessible progress surface pass the measured scope. Three P2 notes below do not independently block acceptance.

## Method and evidence

Read the ticket's Work/Gate/Result/Comments, round-19 D21/D23/D31/D32 and signature interaction, round-18 D6, DESIGN's changed primitive/motion sections, ADR-0006/AGENTS styling and lifecycle rules, commit diff, and audits 11–14's floors. Applied `emil-design-eng`, `animate`, `review-animations` + STANDARDS, Impeccable audit, shadcn/React guidance and freshly fetched Web Interface Guidelines. The pinned desktop/24px/token contract takes precedence over generic mobile/44px advice.

All evidence is **fresh** under `.scratch/round19/evidence/15-audit/` (ignored). Application source was not edited; no live Tauri instance was launched. Existing fixture port 5175 returned HTTP 200. One named headless agent-browser session, `r19-15-audit`; trusted keys through the supplied CDP driver, not agent-browser's broken `press`. Main configurations: **1280×800 dark / 960×640 light**, plus 1280 light to measure the fill hidden at 960. Reduced motion explicitly emulated. Native VoiceOver/WebKit release smoke remains the operator half of D31, not inferred from Chromium.

- Reviewed/copied the implementer's motion, fixes, axe, details, ring and keyboard drivers into the audit evidence directory, changed only output/session plumbing and the renamed Remove… selector, then **executed** them. `motion-*`, `fixes-*`, `a11y-*`, `ring-*`, `details-*`, `gate-*` contain new results, not copied outputs.
- Independent probes: `adversarial.ts` (Refresh, false-light triggers, rapid reversal, rail interruption, full accessibility-tree inspection), `repeat-note.ts` (same-text arrivals), `pairs-details.ts` (resolved token pairs, grouped layout, empty states, primitive attributes), `progress-reduce.ts` (wide light fill and first-frame toast staging).
- `css-inventory.ts` parses the **fresh built** `dist/assets/index-BYSRrxYU.css` with PostCSS; `built-css.txt` and `css-inventory.json` retain every transition/animation declaration and keyframe. `transition-refs.txt` is the source-reference inventory. Detector result: `[]`.
- Inspected fresh screenshots, including grouped/selected rows, panel progress, inspector/sheet and dialogs. Timing conclusions come from rAF + WAAPI/computed styles, **not** the implementer's ~1fps recording. Its missing segment remains missing; this audit does not claim to replace it with a new recording.
- An initial 5s CDP evaluation timeout and a combined recovery-run shell timeout were bounded harness failures; completed reruns are the evidence. The final action/recovery matrix preserves navigation boundaries rather than resetting away preceding transcript stops.

## Per-clause verdict

| Clause | Verdict | Evidence / qualification |
| --- | --- | --- |
| Built CSS: no `transition: all`; duration/reduce coverage | Pass | **0** explicit all declarations; finite transitions resolve to 120ms. Loops and runtime Sonner caveat below. |
| Five intended motions, both themes | Pass | Measured table below. Light fill additionally measured at 1280 because it is `display:none` at 960. |
| Interruptibility | Pass | Controlled real Switch specimen reversed at **48ms dark / 55ms light**; retargets without jumping to an endpoint. Rail cancelled at ~49ms, absent next frame. Product switch correctly refuses a second mutation while pending. |
| Strip light only on status transition, not navigation | Pass for tested triggers | Latency-0.2 Refresh: 20 marks acquire `data-lit`, opacity transitions observed; zero running animations after settlement. Mount, filter, unfilter, scroll down/back and theme change each produce **0 lit-mark transitions**. Ordinary Enable also measured. “On screen” here means the mounted presentation; no viewport-intersection gate exists or is claimed. |
| Inspector, sheet, menu, dialog, hover/focus/selection | Pass | No entrance motion on layers; first rendered layer sample opacity 1. Row/hover/focus state styles are instant. |
| Reduced motion: no interpolation or looping | Pass; literal first-DOM-frame claim needs correction | No running animations; fill jumps, rail/light settled, header spinner hidden. Sonner's initially invisible staging frame is P2 A3 below. |
| Frequency/purpose/cohesion | Pass against pinned world | Rubric verdicts below; generic absolute keyboard-motion rule conflicts with the explicitly named switch/rail contract. |
| 4px rhythm / numerals / hit targets / truncation / copy | Pass for measured content | Positions and dimensions below; no visible clipped text without title in sampled rows/groups/source. Inspector-long-description edge cases beyond fixture content are not certified. |
| Switch held-disabled contract | Pass | SPAN role switch; `aria-disabled=true`, `tabindex=0`, no native disabled attribute or aria-readonly; reason as title/aria-description. Space/Enter leave value/focus unchanged. |
| Primitive-owned changes, no styling overrides | Pass | New switch/sonner motion and disabled state live in ui owners; strip/header own their native moving elements. No new call-site primitive colour/duration/transition override. Source tokens unchanged. |
| N2 resize with child closing | Pass | Both configurations: **0 BODY frames** after child Escape and each side↔sheet transition; heading receives focus. |
| N4 polite arrival once without focus movement | **Fail — A1** | First run and first stale note each mutate the live region once, no focus movement. A second identical note grows history but never changes the region. |
| N5 one visible and accessible progress surface | Pass | Header yielded/inert/aria-hidden; panel foot alone drawn. Full AX tree exposes exactly one nonempty progress status (“Refreshing”); header status excluded. No simultaneous duplicate progress announcement surface. |
| Prior contrast floors, changed pair | Pass | Fresh ratios below; no settled pair lowered. Held-disabled outline improves to 4.42 / 3.49. |
| Keyboard matrix / axe / gates / bindings | Pass for executed scope | Six axe runs zero violations; keyboard and gate details below; bindings unchanged. Not a native accessibility certificate. |

## Findings

### A1 — P1: consecutive identical arrivals are silent in the open report

**Location:** `src/components/shared/NotificationsModal.tsx:285–293`; `src/lib/notificationRuns.ts:216–223`.

**Reproduction:** failures fixture; insert the same supplemental foreign-folder condition used in the existing N4 reproduction for retired-playbook/Claude Code. Enable → dismiss collision → open Notifications → Overwrite… → confirm Overwrite. Activate the historical Overwrite… again: the new note says “Nothing to overwrite: retired-playbook is already deployed to Claude Code.” Leave focus there and activate it **again**.

`repeat-note-n4-dark-1280.json` records **runs 3 → 4**, but the arrival-region MutationObserver remains at **2 → 2** (the earlier successful run and first note). The second note is present in history; the status text node is unchanged. Same behaviour reproduced in light (`repeat-note-n4-light-960.json`, note-node count 6 → 10, observer 2 → 2). Focus stays on `Overwrite…: retired-playbook` throughout, as intended.

**Cause/impact:** the fold returns only title text. Different group keys with equal titles collapse to the same React text child. Neither the ordinary history list nor its newly inserted row is a live region; toasts are suppressed while this panel is open. Thus the intended N4 announcement channel emits no update for the second action. This is DOM/AX evidence of missing announcement delivery, not a claimed VoiceOver listening test. Repeated equal success/failure headlines have the same structural problem.

**Concrete fix:** carry arrival identity alongside the announcement text. Keep the polite region mounted; publish exactly once per genuinely new group, including equal-text groups (e.g. clear then publish on a subsequent task for a new identity), without announcing rerenders or moving focus. Do not merely key/remount the whole live region: newly mounted populated regions are not a reliable announcement mechanism. Add hook/fold coverage for consecutive equal titles and no-op rerenders, plus the browser reproduction above. Preserve the initial-empty/open behaviour and one accessible progress surface.

### A2 — P2: new Button regression assertion is disconnected from its spy

**Location:** `src/components/ui/button.test.tsx:42–55`.

The new test creates `onClick = vi.fn()` but never supplies `onClick={onClick}` to the rendered Button; its “not called” assertion passes regardless of click suppression. The neighbouring existing test already covers refusal, so this is not a production defect or a missing-all-coverage claim. Bind this spy to the Button, or remove the redundant assertion and keep this test specifically about held focus/ring classes. Run the primitive test afterward.

### A3 — P2: tighten the first-frame evidence claim, not the motion budget

**Location:** `src/components/ui/sonner.tsx:14–24` (contract comment), `.scratch/round19/issues/15-emil-pass.md` § Verification, `DESIGN.md` § Reduced motion.

With reduce, Sonner still inserts `data-mounted=false`, opacity **0**, translateY **−92.5px**; the next sample has `data-mounted=true`, opacity **1**, transform identity. Fresh dark samples: **896.8 → 901.1ms** after trigger; light **901.8 → 905.6ms**. Both have `transition:none`, duration **0s**, **0 running animations**. There is no visible glide: the preliminary element is invisible. Therefore “every animated element settled on its first [DOM] frame” is not literally proven/true. Describe **first visible frame / no interpolated motion**, or, if first DOM frame is a hard product requirement, implement a primitive-owned reduced-motion staging override and remeasure. Do not report the light 960 fill as animated either: it is hidden there; the independent light-1280 run verifies `transform:120`.

### A4 — P2: ticket-25 source-scan cleanup; distinguish dormant source from no reference

**Location:** `src/components/Layout.tsx:15`; `src/index.css` source discovery / generated `.transition`, `.transition-colors`, `.ease-in-out`.

Live audited DOM has **0 uses** of all three utilities. However, `transition-colors` **does** appear in `src/`: dormant Layout.tsx. There is no live importer of that dormant surface. The broad generated utilities are harmless today and use 120ms, not `all`; they should not be described as having no source references. Forward to ticket 25: narrow irrelevant source scanning/remove the dormant residue at its authorized cleanup, retain useful utilities. No application edit is requested here. The legacy `.skeleton-row` is also a shimmer loop, not a spinner; the accurate claim is “loops only above 120ms,” not “spinners only everywhere.”

### Required Emil Before / After table

| Before (audited state) | After (recommended disposition) | Why |
| --- | --- | --- |
| Equal-title new groups leave the status node unchanged | Publish by arrival identity, once per new group | A distinct operator action still needs its outcome announced. |
| Unbound `onClick` spy asserts refusal | Bind the spy, or restrict the test to focus/ring | Avoid an assertion that cannot fail for the advertised regression. |
| “Settled on first frame” includes invisible Sonner staging | “No interpolation; settled on first visible frame” | Accurate evidence without adding gratuitous motion machinery. |
| Broad unused utilities and “no source references” | Ticket-25 scan cleanup; acknowledge dormant Layout | Keep bundle/source claims precise. |

## Measured motion versus DESIGN.md

All five declared durations are **120ms**, easing **`cubic-bezier(0.16,1,0.3,1)`**, matching the existing world token rather than inventing a second curve. rAF observations bracket CSS timing; event→settlement includes backend latency and rendering, and must not be labelled transition duration.

| Element / property | DESIGN | Fresh measurements dark / light | Interruption / reduce |
| --- | --- | --- | --- |
| Switch thumb `translate` | 120ms | WAAPI 120 / 120; first changed sample→settled **117.4 / 116.1ms** (12px→none) | Real specimen second toggle **48 / 55ms**; reverses from intermediate ~2.30px toward 16px. Reduce no animations; backend state arrival still asynchronous. |
| Rail opacity + translate 8px→0 | 120ms entry, instant exit | WAAPI 120 / 120; first changed→settled **116.4 / 116.8ms**; initial zero-opacity sample precedes CSS advancement | Cancel mid-rise ~48.5 / 48.7ms; absent at **68.5 / 70.4ms**, next sample. Reduce first rendered sample 1/none. |
| Strip glyph opacity | 120ms | WAAPI 120 / 120; first opacity-0→settled **123.2 / 123.7ms** (frame-quantized); Refresh also samples only 120ms opacity transitions | 20 marks light after Refresh; 0 animations after settlement. No mount/filter/unfilter/scroll/theme light. Reduce first glyph sample opacity 1. |
| Header fill `transform:scaleX` | 120ms per tick | Dark 1280 `transform:120`; **light 1280 `transform:120`**. 960 fill hidden, not moving | Tick retargeting observed; no layout-width transition. Reduce 0s, jumps, spinner display none. |
| Toast transform + opacity | 120ms, no height | Both themes: starts opacity 0, translateY −92.5px, 120ms custom ease; exit returns along same edge. Last observed exit currentTime **116.609 / 116.633ms**, nearly zero opacity | CSS transitions retarget; height fixed during sampled entry/exit. Reduce none/0s; invisible staging caveat A3. Swipe uses Sonner keyframes with inline duration override/base reduce coverage; no physical touch swipe claim. |

Inspector side/sheet and Remove dialog: opacity 1 at first rendered sample, **no entrance animation**. Menu and inline Enable popover: no animation. A Files-fetch spinner observed inside the inspector is not inspector entrance motion. Header summary↔progress swap stays instant.

### CSS inventory

- Explicit `transition: all` / `transition-property: all` in built CSS: **0**. Default computed `transition-property:all` on an element with 0s/no active transition is not an emitted `transition:all` declaration.
- Every finite transition duration resolves to **.12s**. Over-120ms periods: `--animate-spin` **1s**, legacy `.spinner` **.9s**, `.cell-spinner` **.6s**, `.skeleton-row` shimmer **1.5s**. All infinite loops; no long finite transition.
- Built keyframes: **spin** (transform) and **shimmer** (background-position). Both covered by base-layer important `animation-duration:0s`, iteration-count 1 and delay 0 under reduce. New spinners also have `motion-safe`; header additionally hides. Shimmer is legacy/non-compositor motion, not one of this pass's new interactions.
- Sonner injects its CSS at runtime, so the bundled CSS inventory alone cannot certify it. Actual toast transitions are 120ms transform/opacity; the primitive overrides stacked child duration too. The global important reduced-motion layer wins over inline durations and child utility durations. Extra reduced wide runs measured **maximum 0 running animations** throughout Refresh and toast arrival, both themes.
- Source diff has **12 removed `transition:all` declarations**, six other removed transition declarations (one multiline), and one retimed knob transition. The ticket's “17 all + 3” wording is inaccurate; **1862** is the verified styling-guard counted-line ceiling, not a count of removed transitions.

## Emil rubric verdicts

- **Frequency first:** row focus, hover and selection grounds are instant; no repeated row/panel sweep. Switch is short, tiny and interruptible. Rail enters only on crossing zero→nonzero selection and never delays interaction or exit. This is near-invisible enough for the pinned operator UI.
- **Keyboard-rule tension:** the generic skill forbids all keyboard-triggered animation, while this project's named switch/rail motion runs for keyboard and pointer alike. Judge this as an explicit world-contract exception, not literal compliance with the generic prohibition. If Alex intends that prohibition literally, suppress switch/rail motion for keyboard input; do not silently redesign it during this audit.
- **Toast at an edge, 120ms:** approve. A ~92.5px edge entry is brisk but legible with this strong ease-out; toast lifetime, not travel duration, provides reading time. No height animation or delayed controls. No evidence justifies lengthening it beyond the pinned budget.
- **Strip light:** approve as state indication, not celebration. It affects only changed glyphs, adds neither displacement nor repeated pulse, and does not replay when filtering/scrolling/theme-switching. Quiet complete strips retain their contrast floor after settling.
- **Instant summary↔progress:** right choice. No period of two competing statements, no artificial wait on every action.
- **Missing signature motion:** no ticket-15 omission found. Palette belongs to ticket 23; rare import cascade/install landing to 21/16. Do not add inspector/menu/dialog motion to “complete” this pass.

## Details, contrast and keyboard evidence

### Details

`pairs-details-*` records **positions**, not only heights: header y=0..49, filter y=49..98; x=200 dark-wide / 56 light-narrow. Rows 36px, padding 0/12; header/filter padding 8/16; rail 41px with padding 8/16 and gaps 8/12. Rail wraps to **69px** beside the wide inspector rather than clipping. Group linked headings 24px; noninteractive “Individual sources” text line 20px. Hairlines and centering account for .5px/global-offset positions; a 4px spacing rhythm does not require every absolute coordinate to be divisible by four. Panel foot 41px, padding 8/20; changed item padding is 8px, run padding 12px. Optical 2px icon alignment is deliberate, not structural spacing drift.

59 sampled counts per primary configuration use computed `tabular-nums`; additional grouped counts and the panel inherit the same base rule. No sampled row/rail control below **24×24**, counting the primitive's pseudo-element hit extension. Titles cover sampled visible truncations (names, group text, source); light inventory's “Skills Hub” is an **sr-only brand label**, not visibly clipped text (`AppSidebar.tsx:130`). Sentence-case labels remain consistent; Remove… correctly distinguishes the menu's dialog-opening step from final Remove. Empty library offers a clear explanation and Add from GitHub / Add from a folder / Browse Explore; first-run discovery remains an explicit review action.

### Fresh contrast (settled values)

Resolved colours, canvas sRGB conversion/compositing, and rendered text/outline inventories; disabled text excluded, **focused disabled outline included**. All unchanged token pairs match prior audits to rounding. A transitional fade's instantaneous opacity is not reported as its settled contrast.

| Pair / floor | Dark | Light | Result |
| --- | ---: | ---: | --- |
| Minimum rendered enabled text, My Skills | **4.68** | **4.89** | Floors preserved |
| Foreground on background / card / raised | 15.499 / 14.512 / 13.211 | 17.320 / 16.137 / 15.278 | Unchanged |
| Muted / ordinary synced mark on background / card / raised | 8.310 / 7.781 / 7.083 | 8.158 / 7.600 / 7.196 | Unchanged |
| Input / hollow / quiet synced mark, same grounds | 3.900 / 3.651 / **3.324** | 3.777 / 3.519 / **3.332** | ≥3.32 raised floor preserved |
| Faint / disabled mark, same grounds | 6.121 / 5.731 / 5.217 | 6.091 / 5.674 / 5.372 | Unchanged |
| Stale mark, same grounds | 10.414 / 9.751 / 8.876 | 5.617 / 5.233 / 4.955 | Unchanged |
| Failed mark, same grounds | 6.906 / 6.466 / 5.886 | 5.500 / 5.124 / 4.851 | Unchanged |
| Ring on raised | **6.013** | **5.037** | Prior 6.01 / 5.04 rounded floors preserved |
| Ring on background / card / popover | 7.055 / 6.605 / 6.605 | 5.711 / 5.320 / 5.855 | Unchanged |
| Held-disabled focused ring on app ground | **4.42** | **3.49** | Improved from 2.60 / 2.19; ≥3 |
| Panel minimum in sampled state | 7.78 | 7.60 | No regression; not a claim every panel state has this minimum |
| Warning toast text on tinted ground | 8.62 | 5.13 | Unchanged |
| Lit mark after settlement | 8.31 | 8.16 | Same as ordinary synced |

### Keyboard / axe

- Ticket-13 gate: **84 recorded stops each**, 0 BODY, visible solid 2px focus. Select three → Disable → Refresh preserves disabled → Enable → select all/count confirmation; independent state hand-count remains **25 skills / 8 Tools / 80 targets / 12 assignments / 1 already disabled** after that flow.
- Complete supplementary action transcripts (`flows-*-bulkDisable,bulkRemove,dialogClose,restore,delete.json`): **83 dark / 80 light stops**, 0 BODY. Delete lands on expected next skill; Restore on Health; close on opener.
- Recovery transcripts (`flows-*-late,resize,dismissed.json`): **194 stops each**, 0 BODY. Late settlement does not reclaim Settings focus; child stays above inspector through resize; dismissed collision retains unmanaged content and returns focus.
- Dedicated N2 rAF probes: 0 BODY after child close and both reparent directions, both themes. Dedicated Switch own-action/refusal probes: 0 BODY, held focus, refused Space/Enter, restored enabled state. These are sampled-frame claims, not an assertion that DOM activeElement can never be transiently BODY between synchronous statements.
- **Six fresh axe 4.10.3 runs, zero violations**: list+inspector/sheet after Enable, running Notifications foot, refused switch/header; both themes/sizes. Incompletes: Base UI `aria-hidden-focus` guards and disabled-control `color-contrast`, preserved in JSON, not counted as automated passes. N4's duplicate-text hole is not detected by axe.
- N4 initial note: role status implies polite; **no explicit `aria-live` attribute**, but the full AX tree resolves `live=polite`, `atomic=true`. Explicit spelling is not needed for equivalent semantics. First arrival and focus behaviour pass; A1 is the missing distinct subsequent arrival.

## Gates and source integrity

Fresh `npm run version:check && npm run check` exited successfully (`gate.txt`, `version.txt`): version **1.2.18**; lint clean; styles guard **1862 lines / 26 migrated files**; **43 Vitest files / 917 tests passed**; TS7 `tsc -b` and Vite production build passed; rustfmt/clippy passed; **807 Rust tests passed**, no failures. Additional fresh `cargo test --all`: **807 passed**, binaries/doc tests clean (`rust-all.txt`).

`git diff --exit-code 34a7624 -- src/bindings/index.ts` and source/design/script drift checks returned 0. No version/source/bindings change. No stash, revert or commit. The only tracked audit writes are this report and ticket 15's dated Comments entry.

Impeccable scoped indicative score: accessibility 3, performance 4, responsive 4, theming 4, integrity 3 = **18/20**. This is not acceptance: the reproduced N4 defect still requires a fix.

## Acceptance

**FIX-THEN-PASS — A1 must be fixed before ticket 15's D31 gate is closed.** Motion/contrast/focus results are otherwise accepted for the executed scope. A2–A4 are nonblocking notes; send the cleanup note to ticket 25 through the orchestrator. The existing low-frame-rate recording is supporting context, not 120ms proof.

Exact bounded re-check:

1. Repeat N4 in dark 1280 and light 960: first new run, first stale note, **second identical stale note**, consecutive same-headline runs, and no-op rerender/progress tick. Each actual arrival must produce one polite update; no new announcement on open/rerender, no duplicate toast, no focus movement.
2. Retest panel close/reopen and Clear; ensure baseline/seen identities do not leak between openings. Inspect AX tree with the panel open mid-Refresh: only panel progress accessible, header still inert/hidden.
3. Run affected hook/fold tests and `npm run version:check && npm run check`; bindings unchanged. Re-run panel axe in both configurations. Repeat broader keyboard/contrast/motion matrices only if the fix changes shared layering, focus, tokens or animation owners.
4. Operator VoiceOver/native-minimum-window/reduced-motion release smoke remains D31's separate release closure. No need to increase motion duration or add animations to resolve A1.

### Residuals closed (2026-09-30, orchestrator)

- **A1 (P1)**: `arrivalAnnouncement` now answers `{ key, text } | null` (the group's identity beside its text) and the panel publishes by identity: a new key empties the `role="status"` region on this task and refills it on the next (`setTimeout 0`, keyed on the primitives so a rerender cannot cancel the refill), the region stays mounted, an unchanged arrival publishes nothing, focus never moves. Fold test for two equal-text arrivals with distinct keys. Re-run of the audit's own `repeat-note.ts n4` (dark 1280, light 960): runs 3→4 now gives region mutations **3→5** (`""`, then the note again), focus stays on `Overwrite…: retired-playbook`; `n5` still one drawn progress surface.
- **A2 (P2)**: the Button test binds its `onClick` spy.
- **A3 (P2)**: DESIGN.md § Reduced motion now claims "no interpolated motion; settled on the first *visible* frame" and names sonner's invisible one-DOM-frame staging.
- **A4 (P2)**: ticket 25 already owns the source-scan cleanup (dormant `Layout.tsx`, generated `.transition*`); no edit.
- Gate: `npm run version:check && npm run check` exit 0 — vitest 918, cargo 807, App.css 1862 / 26 migrated. **D31 gate passed** for ticket 15.
