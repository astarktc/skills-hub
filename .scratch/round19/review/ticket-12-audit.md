# Ticket 12 — independent D31 inspector audit

**2026-09-30 · FIX-THEN-PASS. Five P1 findings (I1–I5), four P2 notes (I6–I9); no reproduced P0.** The full build/test gate passes, but the fixture accessibility/behavior gate does not. Audited the **uncommitted working tree over `ef7431bf08f03dd4ffeaae77e6878bfd58efc054`**, including untracked inspector/modals/helpers, not just `git diff`'s tracked files. No application source edited, reverted, stashed or committed.

## Method and evidence

Evidence root **E = `.scratch/round19/evidence/12-audit/`** (gitignored). The only tracked audit writes are this report and ticket 12's dated Comments paragraph.

- Read AGENTS.md (including all Ambiguity resolution and Invariants), ticket 12 and Result/Comments, the requested spec clauses, ticket 11's audit/re-check, DESIGN.md, ADR-0006, and the migration diff. Applied Impeccable audit and freshly fetched Web Interface Guidelines. The project's desktop minimum, 24px dense target floor, hash policy, EN-only copy and primitive conventions override generic mobile/web advice.
- Reused the already-listening **5175** fixture server (PID 21504). Never ran Tauri or touched the operator's library. **Headless `agent-browser --session r19-12-audit`**, Chromium 154, default fixture latency (no `latency=0`), dark/light, 1280×800 and 960×640; also tested the exact 1200px boundary. `rich`, `failures`, `empty`.
- `driver.ts` supplies trusted `press`/typing, saves every sampled active element, name/role, `:focus-visible`, outline geometry, clipping ancestors, hash and layer state. **No `.focus()` or synthetic keyboard events in primary transcripts.** Navigation/setup for measurement-only matrices uses hashes; the primary row→inspector→repair transcripts use keys throughout. Native button role is implicit when the DOM `role` attribute is null.
- Primary runs: `keyboard-{dark,light}-{1280,960}.json`; extended runs: `dialog-flows-*`, `ask-*`, `undo-*`, `detach-*`, `repoint-*`, `assign-*`, `delete-*`. Reproducible drivers are beside their outputs. Early driver failures were selector/setup mistakes, retained in `*-output.txt`; final named JSON records are the evidence, not an assertion that every exploratory script completed.
- `matrix.ts`: 8 axe/contrast/motion conditions (rich/failures × themes × sizes). `additional.ts`: another 20 inspector inventories (docx, api-style, json-canvas, legacy-lint, local-notes × themes × sizes), plus Re-point mismatch inventories. `measure.js` uses WCAG sRGB luminance, canvas colour resolution and ancestor-background compositing, includes rendered prose, excludes disabled controls. `contrast-pairs.json` deduplicates 30 observed text pairs. Ratios below are fresh, not copied from DESIGN.
- **Explicit supplemental instrumentation:** `faults.ts` injects one typed command rejection *after* a successful Re-point preview, using a one-shot Array.find interception restricted to that fixture handler's stack. Its second transparent interceptor records project reads. `failed-first.ts` models repaired Amp/Kimi permissions and a newly appeared foreign json-canvas folder in the fixture's in-memory arrays, then drives the real Retry/ask flow. These are fault/state-injection tests, not claims that the stock fixture or Rust spontaneously produced those states. Wrappers were restored; sessions reloaded/closed. An earlier network-route instrumentation attempt did not work and was removed; it supplies no verdict evidence.
- `marks.ts` mounts the actual `StripMarkIcon` component as a disposable browser-only specimen for all five shapes on card/raised/popover, including the unseeded stale mark; no source or fixture file changed. `mark-specimen-*-gray.png` is supplemental shape evidence, not a seeded-library screenshot.
- No VoiceOver, native WebView, native minimum-window enforcement, or full Add/Import/Projects workflow certification. D31's operator release half remains required.

## Per-clause verdict

| Requirement | Verdict | Evidence / qualification |
|---|---|---|
| D3 placement, landmark and section order | Pass | Named `<aside aria-labelledby=h2>` at 1280; named modal sheet at 960. Health → targets → source → edits → projects → files → Remove. `extra-*`, screenshots. |
| Row → inspector → Restore → row, both themes/sizes | Pass for this flow | 20 stops per desktop run, 17 per sheet run; all sampled stops `:focus-visible`, solid 2px, **zero BODY stops**. Desktop keeps row focus then switch→overflow→heading; sheet starts on heading. Restore lands on Health, Escape returns docx. Health's ring is clipped (I8), so computed style alone is not a complete paint pass. |
| D21 every inspector dialog settlement returns valid focus | **Fail** | Escape/Cancel from Re-point, Detach, Invocation and Assign returns its opener, including inside the sheet. Assign success also returns correctly. **Successful Detach, Re-point and conflict Edit lose focus on BODY in the sheet** (I1). |
| Ticket 11 A1 overwrite regression | Pass | Default-latency row switch Disable→Enable→ask; Escape, Keep, Overwrite each tested at dark 1280 and light 960. All six settle on originating `frontend-design enabled` switch, root not inert, zero dialogs/closed ghosts; next Tab reaches overflow. `ask-*`. Existing unanswered-collision error-toast semantics remain ticket 11's separate fold note. |
| D23 resize / deep link | **Partial** | Deep-link arrivals and bare side↔sheet resizes land on the new heading (`deep-link-*`, `resize-heading-*`). Resizing with Re-point already open puts the new sheet **above** its child modal (I2). Inspector-open width was not included in the old breakpoint budget (I3). |
| local-notes missing source | Pass | Recorded `/Users/alex/Projects/notes-skill` shown; copy button is bound to that exact value (code inspection); Re-point pre-filled Local; Check source renders `SOURCE_PATH_MISSING` in FieldError, zero toast. `local-prefill-*`, `local-error-*`. |
| docx central-missing health/marks | Pass | Restore under Health, seven Broken marks + one not synced, no filled mark or per-Tool mutation. `docx-*-state.json`, `extra-docx-*`, images. |
| D15 preview, mismatch and candidates | Pass for admission; **fail final-error channel** | `total-typescript/skills`→ts-reset produces a highlighted mismatch, focused summary, explicit **Continue with ts-reset**. No mutation on preview. Candidate/presentation steps share **one popup**, not nested `pushDialog` layers. Back/Choose-another are local step changes; Escape returns opener. Final command refusal is a toast over the still-open confirmation (I4). |
| D22 targets list/grid and Undo | Partial | List default and grid preference survives reload (`grid-persist-1280`). List Pi Disable→Removed/Undo→Enable via same control, expiry after 5s becomes Enable; Undo re-enables. Grid Disable omits the Undo/status affordance (I7). |
| Failed first-sync Retry → one overwrite ask | Pass with explicit supplemental state setup | `failed-first-before.json` has Amp/Kimi `status:error`, `synced_at:null`. After simulated permission repair + foreign folder arrival, trusted Retry Amp passes the shared-directory ask and opens **one** overwrite ask naming json-canvas→Amp/path/time/differs. Escape returns Retry without a ghost. `failed-first-collision.json/png`, `failed-first-dismiss.json`. Stock fixture alone only exercises the permission refusal. |
| Per-target failure explanation / Overwrite label | Wire gap, not a missed `last_error` field | `SkillTargetDto` contains only tool/mode/status/path/synced_at. Generic “First sync failed”/“Sync failed” is present; the actual error reason is unavailable. I9 needs an explicit disposition, not guessed copy. |
| D18 upstream/provenance, copy, Update/Change source, Detach confirm | Pass with documented metadata deferrals | Imported Cursor provenance explicitly says no upstream/Refresh skips. Shared reporter uses plain `navigator.clipboard.writeText`, no component Tauri import. Detach confirms consequence. Recorded ref/SHA and historical detached path are absent on wire; see I9. |
| Projects assignment and settlement | **Partial** | Picker shows configured Tools and unavailable/no-Tool projects. Assign local-notes to quartermaster yields **Assigned to 2 tools** and Claude Code/Pi rows, returns to Assign in both placements. Mutation does call `applyView` and the door's fold. But new inspector re-reads **all four project views after success** (I5). |
| Files tree/reader, bullets, expand/collapse | Partial | `.prose` reader renders disc/decimal lists; tree buttons work, full-width in-place expansion preserves focused skill and reader, collapse button works. Desktop Escape collapses first. `files-button-collapse.json` and `file-reference-read.json` additionally verify toggle collapse and opening references/checklist.md. Sheet Escape closes the inspector instead (I6). |
| Delete focus rule | Pass for current dialog | Unfiltered alias-skill deletion→api-style; last zod-schema-first→windsurf-rules; search-filtered sole local-notes→My Skills heading, both placements. No BODY/ghost. D16 count-bearing/partial-report redesign still belongs to ticket 13; not claimed complete here. |
| Styling/i18n/IPC generation | Pass, minor boundary note | Migration ratchet 2316→2076, 14 appended files (21 total), no new CSS/raw colours. Full literal i18n test passes. Binding regeneration byte-identical; optional selection correct. Direct CommandError import in RepointModal bypasses world shim (minor cleanup below). |
| axe / contrast / reduced motion | Pass for measured conditions | Zero axe violations in eight runs; text ≥4.679, non-text marks ≥3.324. Reduced motion zeroes durations including pending legacy overlay. See detail below; not a substitute for focus/resize findings. |
| Full gate | Pass | Version 1.2.18; 38/801 frontend; 794 Rust; lint, styles, TS build, rustfmt, clippy; final exit 0. |

## Findings

### I1 — P1 — A successful dialog action can leave the sheet focused on BODY

**Locations:** `src/App.tsx:393–396` (skillDialogFallback), `src/components/ui/dialog.tsx:194–205` (fallback handling), `src/hooks/useSkillActions.ts:270–275` (dialog-trigger exemption from door restoration).

**Reproduce:** failures, 960×640 → local-notes → inspector Health Detach → confirm Detach. At 2 seconds and again after another 3 seconds: `activeElement=BODY`, no visible ring, sheet still open, root correctly inert *because the sheet remains*. Next Tab jumps to the footer Remove control rather than the original task. Also reproduced deprecated-helper→Re-point→grill-me→Continue and api-style→Review invocation→Keep yours at 960; the tested desktop Detach/Re-point versions return to the row. Edit reproduces the disappearing-opener defect too, so removing only the progress overlay is not an adequate fix.

**Evidence:** `detach-sheet-settled.json`, `detach-sheet-5s.json`, `detach-sheet-next-tab.json`, `detach-body-sheet.png`, `repoint-settled-960.json/png`, `edit-conflict-open.json`, `edit-conflict-confirmed.json`; trusted transcripts in `actions.ts`, `edge-flows.ts`, `settlement.ts`.

**Impact:** violates D21's never-BODY and dialog→sheet contract on ordinary successful repairs. Unlike ticket 11 A1, this is not a permanent inert-app lock: Tab can recover inside the sheet. Still a gate-blocking focus defect.

**Fix:** resolve a live, non-inert sheet target (heading/Health/remaining control) before falling back to a behind-sheet row. Coordinate closing action/progress layers so return focus occurs after the relevant inert teardown; validate fallback targets as well as the opener. Keep policy in the shared lifecycle/Dialog seam, not another modal-specific timeout. Recheck success and partial settlement for Detach/Re-point/Edit, not only Escape.

### I2 — P1 — Narrowing the window promotes the inspector above its already-open child dialog

**Locations:** `src/App.tsx:288,568` (placement/remount key), `src/components/skills/inspector/SkillInspector.tsx:153–173` (new open sheet), `src/App.tsx:331–356` (uncancelled focus scheduling).

**Reproduce:** 1280×800 → local-notes inspector → Re-point (field focused) → resize to 960×640. The new sheet registers last and takes focus. Re-point becomes inert underneath it; screenshot shows the modal obscured by the sheet. Escape dismisses the sheet, changes hash to `#/skills`, and leaves **BODY** while the Re-point modal remains.

**Evidence:** `resize-open-modal-narrow.json/png`, `resize-open-modal-escape.json`.

**Impact:** layer order no longer expresses task nesting; resize hides the form being edited and changes which action Escape cancels. D23 reparenting cannot steal a higher modal's focus.

**Fix:** preserve logical layer ancestry when changing presentation. Do not open/register the sheet above existing surface dialogs; defer the presentation change until the child closes or maintain a stable inspector layer beneath it. Cancel stale placement callbacks when subject, area or placement changes. Verify both resize directions with Re-point, Assign, Edit and an overwrite ask open.

### I3 — P1 — Opening the inspector invalidates the list's width budget; a skill name disappears

**Locations:** `src/components/skills/inspector/SkillInspector.tsx:183` (minimum 22.5rem sidebar inspector), `src/components/skills/SkillRow.tsx:154,183,348–354` (shrinking name versus nonshrinking exception marks), `src/App.tsx:540–568` (remaining list width).

**Reproduce:** failures, list view, open local-notes inspector; inspect scratchpad's row. At **1280**, its name button is **62.797px** for 70px text. At the exact **1200** desktop boundary, the remaining list is 640px and scratchpad's name button is **0px** wide (scrollWidth 70). Only “Restore failed / Source folder missing” remains. No horizontal page overflow reveals the missing identity.

**Evidence:** `list-width-{1280,1200}.json/png`; fresh screenshot visually confirms the empty name.

**Impact:** ticket 11 A3's underlying identity loss returns when the new pane is present. A responsive layout cannot make a row's primary identifying control zero-width.

**Fix:** budget for the inspector-open list, not only the full-width list. Preserve name space before exception copy, use a compact accessible exception treatment at narrow container widths, or revisit the one shared breakpoint with the full layout measured. Do not simply shrink type/remove failure meaning. Recheck 1200, 1280, and 960 with inspector open.

### I4 — P1 — Re-point's inline-error contract stops after preview

**Locations:** `src/components/skills/modals/RepointModal.tsx:109–117,342` (only preview answers set error; final submit returns void), `src/App.tsx:760–763`, `src/hooks/useSkillActions.ts:364–374,536–546` (final command through generic runAction/Outcome).

**Reproduce:** complete a successful mismatch preview; before Continue, reject the final command with a typed `GIT_CLONE_FAILED/auth` (e.g. credentials or repository availability changed after preview). The **supplemental one-shot fault injection** in `faults.ts` does exactly this at the fixture command boundary. The confirmation stays open with no error or changed field; a red toast outside it says “Cannot access this repository…”. `fault:true` confirms interception fired.

**Evidence:** `repoint-final-error.json/png`, including the intact dialog and external error toast. This is not a stock-fixture spontaneous failure claim.

**Impact:** violates D15's error-in-modal rule at the step that actually writes. Preview does not guarantee a later command succeeds; keyboard users remain in a modal that contains no reason for failure.

**Fix:** give the door a typed final-action result/error presentation channel to the modal (still one runAction/one fold). Show command refusals inline and preserve the chosen target; keep target fan-out report failures in the report rather than mislabeling them as field validation. Do not bypass the action door to catch locally.

### I5 — P1 — Inspector success-path assignment discards the returned-view advantage

**Locations:** `src/components/projects/useProjectState.ts:214–231,383–401,456–465`; `src/components/skills/inspector/ProjectsSection.tsx:45–59`.

**Reproduce:** assign local-notes to quartermaster, no collisions. `applyView(result.view)` updates `projects`; that changes `readSkillPlacement`'s identity, retriggering ProjectsSection's effect, whose `Promise.all` fetches **every project's getProjectView**. The transparent stack recorder in `faults.ts` observes **4 reads**, all from ProjectsSection, following the successful two-Tool assignment.

**Evidence:** `project-success-reads.json` contains each call stack; `assign-settled-{1280,960}.json` shows the otherwise correct result.

**Impact:** violates the explicit no-success-refetch-tail project invariant. Adds O(projects) reconciliation/read work per assignment and lets an unrelated project read failure replace a known successful placement with a load error.

**Fix:** let the project world retain/expose the placement views it loads and update them with mutation-returned `ProjectViewDto`s through `applyView`. Inspector and picker consume that value, with initial reads and failure/collision recovery kept explicit. Do not move report interpretation into the section. Add a hook test proving successful assign/unassign updates inspector placements without getProjectView tails.

### I6 — P2 — Escape from expanded Files closes the sheet rather than collapsing Files

**Locations:** `src/App.tsx:402–412` limits collapse-on-Escape to side placement; `SkillInspector.tsx:157–159` always closes sheet.

At 960: expand local-notes Files → Escape goes straight to `#/skills`, row focused. Desktop Escape correctly retains the skill and focuses Expand. Evidence: `files-expanded-*`, `files-escape-{1280,960}.json`. Impact: inconsistent reversible reading flow, lost open inspector (not lost skill data). Fix top-level sheet Escape to collapse expanded Files first, respecting nested layers; second Escape closes. The Collapse button remains a workaround.

### I7 — P2 — Grid Disable has no five-second Removed/Undo affordance

**Location:** `TargetsSection.tsx:156–170` versus `:209–240`.

Grid → Pi synced tile → Enter: tile becomes “Pi: disabled — Enable”; **no Removed from Pi status or Undo** is rendered (`grid-disable-1280.json/png`). `removed` state is maintained but read only in list rendering. Impact: the explicit Undo/status promise disappears in the persisted alternate view, though Enable can reverse it. Fix share the status/Undo presentation with the grid while retaining tile focus and the five-second expiry.

### I8 — P2 — Restore's Health focus outline is clipped on three sides

**Locations:** `HealthSection.tsx:57` focuses the full-width section; `SkillInspector.tsx:110` puts it against an overflow-auto boundary.

After Restore, Health computes a 2px ring but its left/right/top outline is clipped. At 1280 its box and scroll container both start x921, width359, y199; only the bottom iris line paints. Same at 960, both themes. Evidence: `health-paint-*.json/png` and clipping records in every primary transcript. This is **not** reported as “no visible focus”: the bottom line is visible, unlike ticket 11's near-entirely clipped link. It does fail the documented ring shape. Fix an inset focus ring on the section, or focus a padded inner Health heading/row.

### I9 — P2 / operator disposition — Three requested metadata treatments need real wire support

**Locations:** `src/bindings/index.ts:1313–1319` (SkillTargetDto), `SourceSection.tsx:35–45,78–80`, `TargetsSection.tsx:52–63`.

- Targets have **no `last_error` and no typed error** on the wire. The implementer did not miss an existing field. Generic state words are present, but “permission denied/auth/etc.” and a pre-known “Overwrite and retry” label cannot be rendered from this DTO. Retry correctly gets the ask from its new report. Add a typed settled-error field through core/catalog/Specta and fold it once, or explicitly defer the richer D22 wording; never infer errors from paths/status or display DB prose as a typed condition.
- Recorded git ref/SHA are not exposed by ManagedSkillDto; the source URL and Last updated are shown, not a fake revision.
- Detach clears source metadata; “Detached from ~/…” cannot be reconstructed. The ticket explicitly allowed recording this backend gap; “Managed here / No upstream” is honest.

These are scope decisions, not a reason to hand-mirror fields in TS. The orchestrator must accept concrete follow-ups or include the DTO work before calling all ticket Work clauses complete. This audit did not add BACKLOG items (outside authorized writes).

## Verdicts on the implementer's four concerns

1. **Shared Dialog fix:** the own-inert diagnosis is credible; `staysInertAfterClose` is the right distinction, and actual opener-return/overwrite regressions pass. **Not sufficient for acceptance:** I1/I2 remain on nested-sheet settlement/reparenting. “Stay mounted” means the four React Dialog/Content shells persist with `open=false`; **Base UI's portal/popup renders nothing closed**. After closure the DOM has zero dormant dialogs/closed layers, not four hidden accessible modals. Small retained hook/context/draft state is acceptable; no measured a11y-tree noise. Candidate/confirm steps are local body states, not nested pushDialog layers—report/test that actual implementation rather than claiming a nonexistent stack path.
2. **Overlay workaround:** acceptable only as a **bounded temporary compatibility seam after focus defects are fixed**, not as the final lifecycle architecture. Current App and door both arbitrate DOM focus by timed retries; App's recursive RAF work lacks disposal on effect replacement, and door `hold` can continue for 12 frames, with its user-interaction guard installed only once it finds a target. I1/I2 prove the overall seam is not yet safe; no separate hypothetical second-action race is claimed as reproduced. **Exact ticket-14 removal note:** remove App's `sheetOpenFor`/`showsProgressSurface` admission latch and overlay-specific `whenSettled` retries; remove the door's overlay wait/12-frame hold; keep placement focus with inspector/surface handling and D21 post-action resolution in lifecycle/Dialog, with cancellation on navigation/reparenting. Recheck late completion after navigation, repeated actions, and resize while a child modal is open.
3. **No Overwrite-and-retry label:** real wire gap, confirmed against the generated DTO. Current row text is product-state copy, **not describeCommandError output**, since there is no row error to describe. Health's typed skill-level errors do use describeCommandError. Retry→ask works, including supplemental failed-first setup; richer copy remains I9, not permission to claim full D22 completion.
4. **axe/reduced motion:** now independently run. All eight axe runs have **zero violations**. Each sheet run has two **incomplete `aria-hidden-focus`** nodes (Base UI focus guards), not violations; trusted traversal stays in the active layer. Reduced-motion settled matrices show zero moving elements. Pending Restore still has legacy animation *names*, but computed **duration=0s, delay=0s, iteration-count=1**; its one retained Animation object is **finished, activeDuration=0** (`reduced-motion-pending-detail.json`). Do not count animation names or `getAnimations().length` alone as motion. New inspector/dialog transitions are static; ordinary-mode legacy overlay motion belongs to 14's removal, not a new inspector animation.

## Contrast and shape results

**Zero measured enabled-text pairs below 4.5; zero measured state-mark pairs below 3.** 30 observed text pairs are fully enumerated in `contrast-pairs.json`. Each cell below is **side / sheet** where different (light side=card, sheet=white popover); dark side/sheet use the same card colour.

| Actual text pair | Dark | Light |
|---|---:|---:|
| Foreground on inspector ground | 14.512 | 16.137 / 17.759 |
| Muted text on inspector ground | 7.781 | 7.600 / 8.364 |
| Faint mode/time/helper on inspector ground | 5.731 | 5.674 / 6.245 |
| Foreground on raised selected segment/tree | 13.211 | 15.278 |
| Faint on raised selected tree | 5.217 | 5.372 |
| Reader foreground on background | 15.499 | 17.320 |
| Syntax function on card/code ground | 9.248 | 6.718 |
| Failed/removal text on inspector ground | 6.466 | 5.124 / 5.639 |
| Accent repair text on inspector ground | 6.605 | 5.320 / 5.855 |
| Missing-source warning on inspector ground | 9.751 | 5.233 / 5.759 |
| Different-name badge text on its mixed ground | 8.319 | 4.990 |
| Continue white text on primary fill | **4.679** | 5.855 |

Non-text mark specimen, each **card / raised / popover**:

| Shape | Dark | Light |
|---|---|---|
| Filled synced | 7.781 / 7.083 / 7.781 | 7.600 / 7.196 / 8.364 |
| Hollow not synced | 3.651 / **3.324** / 3.651 | 3.519 / **3.332** / 3.872 |
| Half-filled stale | 9.751 / 8.876 / 9.751 | 5.233 / 4.955 / 5.759 |
| Failed × | 6.466 / 5.886 / 6.466 | 5.124 / 4.851 / 5.639 |
| Slashed disabled | 5.731 / 5.217 / 5.731 | 5.674 / 5.372 / 6.245 |

Actual focused-heading/repair outline colours match the rendered accent pairs (≥5.320 light on card, ≥5.855 white sheet); the selected-row iris edge on raised uses the existing ring/raised pair. Grayscale captures preserve all five distinct shapes. Decorative section hairlines are not sole identifying boundaries; disabled controls are contrast-exempt. Ratio success does not excuse I8's clipped paint or I1's absent focus.

## Architecture and generation checks

`architecture-grep.txt` records scoped grep results, corroborated with AST searches:

- **Zero invokeTauri calls / command-table imports / native plugin imports / localStorage calls** in the inspector, three new modals and SkillFiles. All mutation controls call `actions.run` or `actions.repair`; no per-component mutation loop.
- Document reads are exactly `listSkillFiles`/`readSkillFile` in `useSkillDocument.ts:68,92`; stale reads are dropped. Re-point preview is `runAction({read:true,cancel:"previewRepointSource"})` at `useSkillLibrary.ts:115–138`. Its errors are values rendered inline. Final errors are I4.
- Health/source are `skillHealth`/its `source` value; target strip state is `syncedStrip`; imported provenance uses the existing presentation helper. No duplicate status/error classifier in the sections.
- Actual dialog subjects are opened through the door/surface union. Re-point's `Step` is internal form progression, not a second global visibility owner. Files expansion is local presentation state. The App sheet admission latch is explicitly temporary overlay coupling, discussed above.
- New persisted key `skills-inspector-targetsView` lives in preferences.ts and is pinned by preferences.test.ts; reload persistence observed.
- Primitive call-site className values are layout-only. Dialog sheet placement is documented. No new stylesheet; App.css deletes 240 counted lines; guard passes at 2076/21. Impeccable detector output `[]` (`detector.json`).
- **Minor import-boundary cleanup:** `RepointModal.tsx:22` imports `CommandError` directly from bindings instead of a skills shim. It is type-only (not a second wire model or runtime bypass), but move it through the agreed boundary when fixing the modal.
- `cargo test export_bindings`: **1 passed, 793 filtered**. `bindings-before.txt` and first line of `bindings-after.txt` have the identical SHA-256; the diff remains **17 insertions / 3 deletions**, exclusively the already-uncommitted extension. `selection?: RepointSelection | null` matches `#[serde(default)] Option`; nested `resolution: GitSourceResolution | null` remains required and nullable. No hand-maintained mirror.
- Rust selection flows through existing acquisition intent and Update settlement; default-branch candidate preview and acquisition tests pass. No independent live GitHub/network smoke is claimed.
- `git diff --check` reports the pre-existing ticket EOF blank and generator-style trailing whitespace on the changed union line. These are not hidden as a clean diff-check or treated as build failures.

## Fresh gate output

`gate-final.txt` is the complete second independent gate run, including its exit marker:

```text
Version OK (1.2.18)
styles:check ok — App.css 2076 counted lines (ceiling), 21 migrated file(s), prose layer scoped, no raw colours; self-test passed.
Test Files  38 passed (38)
Tests       801 passed (801)
[tsc -b && vite build completed]
[cargo fmt --check and cargo clippy -- -D warnings completed]
test result: ok. 794 passed; 0 failed
AUDIT_GATE_EXIT=0
```

Also ran `cargo test --all` independently: **794 passed**, no failures (`cargo-all.txt`). No change to application source was made to obtain green results. The tests do not prove these browser composition paths; the reproduced D31 defects remain despite green tests.

## Acceptance

**FIX-THEN-PASS — not a D31 pass today.** Coherent product-specific visual system, no new palette/system drift; implementation integrity fails on the five P1 behaviors/contracts above. Bounded Impeccable assessment: accessibility 2/4, performance 3/4 (unnecessary read fan-out, not benchmarked), responsive 2/4, theming 4/4, integrity 2/4 = **13/20** for this scoped review, not a whole-app conformance score.

**Exact re-check scope:**

1. Fix I1–I5. Repeat trusted keyboard Detach/Re-point/Edit success and failure from the sheet, including disappearing openers and overlay settlement; never BODY after settlement. Repeat ticket-11 switch ask Escape/Keep/Overwrite to guard the shared primitive.
2. Resize both directions with Re-point/Assign/Edit/overwrite already open: the existing top modal stays top, retains focus and draft; closing it returns to the current inspector presentation. Include immediate navigation/second-action completion to test callback disposal.
3. Re-measure scratchpad and longer names at 1200/1280 with inspector open, both themes; no zero-width identity/control and no outer scrollbar.
4. Repeat typed final Re-point rejection after preview: inline failure in the still-open modal, no competing error toast. Preserve one action/fold and report semantics for partial propagation failures.
5. Hook/command trace of successful assign/unassign: placement updates from returned views, no all-project getProjectView tail. Keep failure/collision recovery reads.
6. Resolve or explicitly accept P2 I6–I9; if fixed, test sheet Escape twice, grid Disable/Undo/expiry/reload, and actual Health ring paint. Record the metadata scope decisions before claiming every Work clause shipped.
7. Re-run the affected four-condition keyboard matrix, eight axe conditions, contrast if pair/layout changes, reduced-motion including pending, bindings drift and the full version+check gate. Operator native/VoiceOver closure remains separate.

Suggested repair order: `/impeccable harden` (focus, nesting, errors), `/impeccable adapt` (name budget), targeted project-world settlement fix, then bounded `/impeccable audit` and `/impeccable polish`. No unrelated redesign is requested.
