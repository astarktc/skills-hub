# Ticket 23 — independent D31 audit

- **Candidate:** `359bb3b8a8f9a1c000f71248ea47f9cb1aac17ec`; reviewed `git diff e856b7b..359bb3b` and `git show --stat 359bb3b`.
- **Checkout:** `/Users/alexstark/.t3/worktrees/skills-hub/t3-196a27b0`, independently verified with `git rev-parse --show-toplevel`. HEAD `9fa54f30ef7fc8dac812b8b871d696e3e3105960` adds the ticket-status record only; candidate source, DESIGN and scripts are unchanged.
- **Date:** 2026-11-02.
- **Auditor:** GPT-6 Astra (`PI_MODEL=gpt-6-astra`), independent of the implementer.
- **Verdict: PASS-WITH-FINDINGS.** No P0 or P1 found. Three P2 findings below: contradictory documentation, accessible-name verbosity, and missing focused regression coverage.

Read the ticket's Work, Gate, Result and all Comments, including the orchestrator's final 11-02 corrections; spec D1/D4/D5/D19–D21/D31; the relevant AGENTS contracts; DESIGN's primitive, palette and Motion sections; ADR-0006; the requested implementation and test files. The final pre-merge Comment, not the older Result bullets, describes the implemented shortcut and initial-highlight behavior.

## Evidence and execution

All evidence paths below are relative to **`.scratch/round19/evidence/23-audit/`**, gitignored in this worktree. The orchestrator should copy that directory as well if it wants the raw evidence to survive removal of this checkout.

- Own Vite fixture server: `npx vite --mode fixture --port 5179 --strictPort`; only `http://localhost:5179/` was driven. Scenarios: `rich`, `failures&latency=3`, and `rich&latency=0` for the completed-action recent and confirmed bulk mutation.
- Ego-browser task space 6, one page; trusted keyboard input through its keyboard API. CDP supplied viewport/media emulation and full accessibility trees. Browser task space closed, server stopped; port 5179 has no listener (`server-stopped.log`). No `tauri:dev`, live library or native credentials used.
- Dependencies were initially absent: the first test/server attempts failed before application execution. `npm ci --ignore-scripts` installed the locked dependencies without tracked changes; all reported results below are subsequent successful runs.
- **Fresh full gate:** `npm run version:check && npm run check` exited **0** (`full-gate.log`, `AUDIT_GATE_EXIT=0`): version 1.3.0, lint, styling guard, **51 Vitest files / 1,074 tests**, TypeScript build + Vite, rustfmt, clippy, **837 Rust tests** plus zero-test binary/doc targets. Bindings unchanged. Vite's large-chunk advisory is non-blocking and not palette-specific.
- Separate `npm run test`: **51 files / 1,074 passed** (`tests.log`). Targeted fold/primitive run: **3 files / 130 passed** (`folds-and-primitive.log`).

## The ten checks

### 1. Keyboard-only uninterrupted transcript — PASS

`keyboard-dark-1280.json`, `keyboard-light-1280.json`, `keyboard-dark-960.json`, `keyboard-light-960.json` record every stop, active element/name, hash, selected rows, dialog names and real `openLayerCount()`. Each starts from a fresh rich fixture. There are **50 stops** per wide run and **56** per narrow run; **zero post-key stops on BODY**. No click, `.focus()` or DOM state mutation is used to advance these flows. Inspection calls do not move focus.

The Tab sequence is identical in all four runs:

1. My Skills → Explore → Projects → Notifications → Theme: System → Keyboard shortcuts → Settings.
2. View 1 skill with issues → Configure harnesses → Update all → Add skill → My Skills search.
3. All 60 → Issues 1 → Edited 3 → Disabled 0 → Grouped → Sort and view: Name A–Z → Review 5 discovered skills → Select api-contract-review → api-contract-review row.

Then, without reloading or pointer input:

| Input | Observed stop |
| --- | --- |
| `j`, `j`, `k`, `j` | artifacts-builder → brainstorming → artifacts-builder → brainstorming row |
| `⌘K`, type `brainstorming` | Palette input; the matching option is the active descendant |
| Enter | `#/skills/skill-038-brainstorming`; focus on the inspector's **H2**, not the palette's old input |
| Escape | `#/skills`; focus on the brainstorming row, no remaining layer |
| `x`, `j`, `x`, `j`, `x` | brainstorming, branded-types and canvas-design selected; focus on canvas-design |
| `⌘K`, type `Disable 3`, Enter | **Disable 3 skills on all harnesses?**; Cancel focused; count copy includes 21 global deployments and 2 project deployments |
| Enter on Cancel | Focus returns to **canvas-design**, exactly where the palette chord was pressed; no layer/inert residue |
| `⌘K`, type `Projects`, Enter | `#/projects`; focus on the **Projects H1**; stack empty |
| `⌘K`, type `My Skills`, Enter, `/` | My Skills heading, then its search input |
| `⌘K`, Escape, `Ctrl+K` | Palette opens from the editable, returns to it, and Ctrl+K does **not** open the palette |
| Narrow runs: palette → brainstorming → `⌘K` → Escape | Inspector sheet → palette above it → only palette closes; sheet and its focused H2 remain |

Additional proof in `mac-editable-chords.json`: on MacIntel, native Ctrl+K at the start of `abc` clears the line to `""` without opening the palette. Explicit cancelable key dispatch reports `ctrlKey: defaultPrevented=false`, `metaKey: defaultPrevented=true`.

`bulk-confirmed.json` also runs an actual three-skill Disable: Enter opens the count confirmation, Tab reaches its affirmative button, Enter disables exactly api-contract-review/artifacts-builder/brainstorming, and focus returns to the row with zero layers. No verb bypassed confirmation.

Focus-policy detail: the preliminary `keyboard.json` chooses brainstorming from an api-contract-review opener. Closing the inspector returns to **the inspected skill's row**, per App's existing focused-row policy, not to the unrelated old row. Cancelling the palette/count confirmation returns to its actual opener. The four final transcripts use the same invoking/inspected row and directly satisfy the requested return walk.

Code: `shell/CommandPalette.tsx:26–34, 85–105, 137–199`; `App.tsx:347–385`; the Dialog return-chain contract.

### 2. No highlight on open; safe recent entries — PASS

`recent-update-idle-and-trap.json` is the decisive case: Update all has completed, is **first** under Recently used, and is **not aria-disabled**. Reopening shows no active descendant. Enter leaves the palette open with no highlight or new run; Down then highlights Update all. This does not rely on a running-action refusal to make Enter harmless.

The initial/recent cases are also in `no-highlight.json`, `no-highlight-and-add.json`, and `recent-update-first-noop.json`. Typing highlights the first match, as the four keyboard transcripts show. Primitive tests pin initial Enter as a no-op, Down navigation and typed highlight. Source: `ui/command-palette.tsx:158–164` (`autoHighlight`, not `"always"`). **F1** records the stale prose saying otherwise.

### 3. Running-action refusal — PASS

`refusal-transcript.json`, `refusal-options.json`, `refusal-toasts.json`, `refusal-ax.json`: failures fixture, latency 3, three selected rows, Update all started through the palette.

All **10 offered mutation/action entries** remain present and `aria-disabled=true`: six selection verbs plus Update all, Add, Configure harnesses and Disable every skill. Each visibly carries **“Update all is running — wait for it to finish”**. Places and skill lookup remain enabled. Enter on the highlighted Disable 3 entry leaves the palette open, opens no confirmation, changes no selection, and raises **“Update all is running — wait for it to finish / Nothing was changed.”** Projects subsequently navigates and focuses its H1 while the action is in flight.

The pure fold test covers all six selection/four action refusals; the primitive test proves refused Enter dispatches one refusal and never calls `onSelect`. Source: `lib/commandPalette.ts` selection/action folds; `ui/command-palette.tsx:120–130`; exported `ui/held-refusal.ts:50` dispatch.

### 4. Accessibility semantics — PASS, with P2 naming hygiene (F2)

Full trees: `ax-{dark,light}-{1280,960}.json`, plus the running/refused trees. Verified:

- Named **dialog “Command palette”**.
- Focused **combobox “Find a skill, a place or an action”**, expanded, list autocomplete, controlling the existing listbox ID.
- Named groups: Recently used / 3 selected / Go to / Actions / Skills as applicable.
- Options and disabled states exposed; `aria-activedescendant` tracks typing and arrow navigation while DOM focus stays in the input.
- Input description **“↑↓ move Enter run Esc close”** resolves to the footer.

**Do not confuse `textContent` with the accessible name.** Chromium actually exposes **“Update all Update all is running — wait for it to finish”**, and **“brainstorming obra/superpowers”**: there is a separating space. It does **not** expose the run-together `Update allDeploy is running` string that the primitive test asserts as textContent. The remaining name-versus-description verbosity is F2; there is no demonstrated unseparated-word screen-reader defect.

### 5. Layer stack and inert cleanup — PASS

Read and exercised `ui/layer-stack.ts`, not just DOM dialog counts. The transcripts import its existing `openLayerCount()` as an observation-only helper:

- Empty page → palette: **0 → 1 → 0**.
- Narrow inspector sheet → palette → Escape: **1 → 2 → 1**; the sheet remains and regains heading focus. Second Escape: **0**, root no longer inert.
- Palette choice → count confirmation: one confirmation layer, not a stranded palette. Cancel/confirmed completion: **0**, root not inert, correct row focused.
- Palette choice → Projects: **0**, root not inert.
- Add dialog → `⌘K`: Add stays the only layer, its source input keeps focus, no palette opens; Escape closes Add and returns focus (`no-highlight-and-add.json`).

“Empty stack” in the ticket is the **surface dialog stack**, not the primitive stack: the inspector sheet is intentionally not a surface dialog. This distinction accounts for the required sheet exception. Primitive tests also pass their zero-layer and inert-release assertions.

### 6. axe, both themes and sizes — PASS

axe-core **4.10.3**, full document with the unfiltered palette open:

| Theme | Viewport | Violations | Best-practice violations |
| --- | --- | ---: | ---: |
| Dark | 1280×800 | 0 | 0 |
| Dark | 960×640 | 0 | 0 |
| Light | 1280×800 | 0 | 0 |
| Light | 960×640 | 0 | 0 |

Raw results: `axe-{dark,light}-{1280,960}.json`. No `region` finding and no menu exception needed.

All four report the same **incomplete/manual-review** cases, not violations: Base UI's two inside focus guards (`aria-hidden-focus`), axe's inability to resolve the combobox's controls reference under `aria-haspopup` (`aria-valid-attr-value`), and the non-text ↑↓ glyph (`color-contrast`). Manually checked: Tab/Shift+Tab cycle to the input rather than stopping on an exposed guard (`recent-update-idle-and-trap.json`); controls ID resolves to the actual listbox; footer key text uses foreground on card. The underlying modal app is inert.

### 7. Actual rendered contrast — PASS

`contrast-{dark,light}-{1280,960}.json` stores `getComputedStyle` colors, normalized RGB, outline geometry and WCAG sRGB contrast calculations. Both viewport measurements agree. `measurement-summary.json` aggregates results. No nominal-token-only estimate was used.

| Rendered pair | Dark | Light | DESIGN claim |
| --- | ---: | ---: | --- |
| Row text / popover | 14.51:1 | 17.76:1 | Passes normal-text 4.5:1 |
| Hint muted / popover | 7.78:1 | 8.36:1 | 7.78 / 8.36 — matches |
| Hint muted / highlight | 7.08:1 | 7.20:1 | 7.08 / 7.20 — matches |
| Highlight outline / highlight | 6.01:1 | 5.04:1 | 6.01 / 5.04 — matches |
| Highlight outline / popover | 6.61:1 | 5.86:1 | 6.61 / 5.86 — matches |

Outline is **2px, −2px inset**. The low-contrast ground change is not the sole highlight cue. Screenshots: `palette-{dark,light}-{1280,960}.png`; additionally `palette-full-{dark,light}-960.png` with actual viewport dimensions recorded in `full-screenshot-sizes.json`.

### 8. Reduced motion and default motion — PASS

`motion-open-close.json` covers both themes × both sizes × **no-preference/reduce** (eight cases). A MutationObserver sampled computed transition/animation durations while the popup/backdrop mounted, changed and unmounted; event listeners captured transitionrun/transitionstart/animationstart. This is not merely a late screenshot after a 120ms animation could finish.

Every case: **905 element-style samples, 0 nonzero durations, 0 transition/animation start events, 0 running palette/backdrop animations**. Reduced-motion media state was confirmed true in the reduced cases. This matches DESIGN's decided **no motion in either mode**. F1 corrects its inaccurate claim that the primitive test proves motion.

### 9. Tests and what they do not pin — PASS, P2 coverage gap (F3)

Fresh counts: **51 files / 1,074 tests**, including **8 primitive tests** and **13 pure palette tests**. Read both requested files. The targeted palette + skillPresentation run passes **130 tests**. Full gate also passes, as above.

The two palette suites do **not** themselves pin:

- Platform-exclusive shortcut routing from an editable (including preserving macOS kill-line).
- The complete palette → inspector heading → row walk or area-choice → Projects heading.
- Palette close/focus return **before** the next confirmation captures its opener.
- Palette-over-sheet Escape isolation, versus refusing the chord over Add.
- The first recent entry being an **enabled Update all** on reopening.
- An exact accessible option name when the option has a hint/refusal; the existing refusal test checks textContent and finds the option by a regex.

These behaviors were independently exercised above; lack of product-surface JSX tests is **not** a standards violation. AGENTS deliberately permits JSX rendering tests only for primitives. See F3 for additions within that boundary.

### 10. Scope hygiene and My Skills query compatibility — PASS

Exactly **15 files** in the candidate diff, all attributable to ticket 23: palette primitive/surface/fold and their tests, Dialog top placement, held-refusal export, surface union, App mount, sidebar shortcut, EN keys, style baseline, DESIGN and the ticket. `layer-stack.ts` itself is unchanged. No backend, dependency/version, unrelated surface, legacy stylesheet or dormant component edits. `git diff --check` passes.

`App.tsx` changes only its palette import and the one bound mount. Styling guard passes at **1108 App.css lines / 37 migrated files**.

`skillPresentation.ts:272–323` extracts the same trimmed, case-insensitive substring/wildcard rule into `queryMatcher`; name/source/path/type fields and sorting are retained. Existing `filterAndSortSkills` tests ran successfully, specifically:

- “sorts by name, added and updated”;
- “matches a plain query as a case-insensitive substring”;
- “supports wildcard queries”;
- “escapes regex metacharacters between wildcards”;
- “does not throw on an unbalanced bracket query”.

## Findings and exact fixes

### F1 — P2 — Documentation contradicts the final safety behavior and callback timing

**Locations:** `DESIGN.md:380, 901–904`; `src/lib/commandPalette.ts:25–28`; `src/lib/commandPalette.test.ts:123`; ticket 23 Result `:42, 75–77`; `src/components/ui/command-palette.tsx:59–60`.

The lower DESIGN palette section and fold comments still say the first entry is highlighted and ⌘K → Enter navigates; the Result says a recent Update all repeats and macOS Ctrl+K is intercepted. The final Comment and running code correctly do the opposite. The primitive prop comment also promises focus has already returned when `onOpenChangeComplete(false)` fires, although the shell explicitly compensates for the callback firing earlier. DESIGN says motion is pinned by a test which contains no motion assertion.

**Fix:** synchronize the lower DESIGN section, fold comments/test title and Result with the final Comment: no initial highlight, Enter no-op, typed first-match/Down opt-in, Apple-only Meta versus non-Apple Control. Annotate the earlier 7-test/1,073-test Result as a pre-correction snapshot or update it to 8/1,074. Describe the completion callback without promising focus-return ordering, and point the no-motion claim at browser evidence rather than the primitive unit test.

### F2 — P2 — Option hints/refusals become part of the name rather than a description

**Locations:** `src/components/ui/command-palette.tsx:198–216`; `src/components/ui/command-palette.test.tsx:153–155`. Evidence: `refusal-ax.json`.

The actual accessible name is readable but verbose: **“Update all Update all is running — wait for it to finish”**. Source hints similarly become part of skill names. No description is exposed for these options. This is **not** the alleged missing-space defect; Chromium supplies the space.

**Fix:** give the label span a stable ID and use it as the option's `aria-labelledby`; give the visible hint/refusal a stable ID and reference it with `aria-describedby` when present. Keep refusal text visible and keep `aria-disabled`. Replace the textContent-only expectation with an exact accessible-name assertion plus a separate accessible-description assertion. Recheck the browser AX tree so the label remains stable while refusal changes.

### F3 — P2 — Important shortcut and layer-handoff regressions remain fixture-only

**Locations:** `src/components/shell/CommandPalette.tsx:26–34, 85–105, 176–199`; `src/components/ui/command-palette.test.tsx:109–199`; `src/lib/commandPalette.test.ts`.

The existing suites pass but do not cover the specific integration boundaries listed in check 9. The pre-merge shortcut/highlight corrections demonstrate why those boundaries deserve durable regression protection.

**Fix:** extract the platform chord predicate as a pure function taking the platform/key modifiers and test Meta-only on Apple, Control-only elsewhere, and rejected extra modifiers/repeats at the appropriate handler seam. Extend the **primitive** harness with (a) an enabled mutation as the first recent entry, asserting initial Enter does not select it, and (b) nested Dialog/palette and close-to-confirm focus-return cases, asserting top-only Escape, correct opener and final zero layers/inert cleanup. Retain the product-level keyboard transcript as fixture regression evidence; do **not** introduce product-surface rendering tests contrary to AGENTS.

## Deliberately not checked / limits of the sign-off

- Native Tauri/WebKit/VoiceOver behavior, release installation and operator reduced-motion/native-minimum-window smoke: D31's second half remains the operator's release check. Chromium AX trees are not a claim of having listened to VoiceOver.
- Windows/Linux Ctrl+K execution: Apple was measured; the other branch was source-reviewed, not run on another OS.
- Every selection verb's backend semantics, cancellation, real filesystem propagation and destructive live-library operations. The full existing Rust suite passed; this audit exercised fixture Update/Disable and refusal/confirmation routing, not a new backend audit.
- No new matrix grid keymap, triggerless-popover R1 repair, future Settings panes or matrix multi-selection certification. The ticket explicitly records these limits. The matrix currently has mutation checkboxes, not a producer of surface `selection.matrix`; the palette correctly did not reuse My Skills' three selected rows after navigating to Projects. This pass does not assert that matrix multi-select has been built.
- No code changes or commits. The only tracked deliverable is this audit; supporting scripts, raw output, trees and images are under the gitignored evidence directory.
