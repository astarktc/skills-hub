# Ticket 27 — independent audit of `c8b727d`

Date: 2026-10-27. Auditor did not implement the commit. Reviewed its stat and full diff against ticket 27 Acceptance/Result, spec D19/D21, AGENTS.md, DESIGN.md and ADR-0006. Applied the freshly fetched Web Interface Guidelines, particularly polite feedback, keyboard operation and unobscured focus.

**Verdict: FAIL / hold an unconditional release sign-off.** The original row-switch reproduction passes, and a newly discovered label-activation defect is **fixed and re-proved**. The final full gate is green. One accessibility gap remains: a refusal in a modal/sheet has no accessible live announcement. This needs a shared-toaster fix or an explicit operator release exception; the implementer's disclosure alone is not a pass.

Evidence root: `.scratch/round19/evidence/27-audit/` (gitignored). Browser: isolated headless Chromium, 1280×800 for rows/header, 960×640 for the inspector sheet; existing fixture server only, `?scenario=failures&latency=3`. Trusted input through the existing `13-fix/cdp.ts` and `keys-cdp.ts`; no `element.click()` activations. No live Tauri/library operations. Browser sessions closed.

## Checklist

| # | Result | Evidence and judgment |
|---|---|---|
| 1 — Three rapid row switches | **PASS** | Independently clicked alias-skill, api-style, corp-conventions. Baseline trusted click intervals: 250.3 / 249.8 ms; post-fix: 249.8 / 251.4 ms. After clicks 2 and 3: visible title **“Disable is running — wait for it to finish”**, message **“Nothing was changed.”**, exactly two refusal events, all three switches still on while pending. Settlement and later observation: `[false, true, true]`. No inspector opened. `proof.json`, `second.png`, `third.png`, `settled.png`; repeated after the fix in `post-fix/proof.json` and corresponding images. Agrees with implementer's `27/proof-summary.txt`, independently generated rather than reused. |
| 2 — Keyboard | **PASS** | Trusted Space starts api-style Disable; second Space while held emits **one** refusal despite the generated detail-0 click. Focus stays on the same switch during/after settlement; zero BODY frames in the recorder, solid focus-visible outline. Trusted Enter on held Refresh all emits one refusal, no action, focus remains BUTTON. `post-fix/proof.json` (`space`, `spaceSettled`, `enter`) and `space.png` / `enter.png`. An additional delayed-Space-keyup probe did not start Refresh after the held action settled (`release-key.json`). |
| 3 — D19 | **PASS** | No queue/retry added. `heldRefusalHandlers` and the added hidden-input observer only emit notice events; Base UI still refuses state changes. Reporter `refuseHeld` only folds/toasts; `runAction` retains its immediate refusal return. The three-click and label settlement proofs show the refused targets unchanged, including after the admitted operation finishes. |
| 4 — One rule and binder | **PASS after fix** | `ui/held-refusal.ts` remains the only CustomEvent producer. Button, Switch and Checkbox use it; Switch/Checkbox now also use its hidden-input adapter for labels. No product call-site notice wiring. No reporter/world runtime import in primitives. `useHeldRefusal` has the sole document listener; `App.tsx:90` mounts it once. |
| 5 — Screen readers | **FAIL in modal/sheet; PASS outside** | Outside modals, one Sonner region is `aria-live="polite"`, `aria-relevant="additions text"`; the refusal text is exposed in the accessibility tree. No primitive live region, shared `lifecycle-refusal` id for `refuseHeld` and refused `runAction`. Description-on-focus plus one notice is the ticket's accepted pattern; identical repeats refresh one toast rather than force new announcements. **Inside the inspector sheet**, the same region is under inert `#root`, and neither its live region nor “Nothing was changed.” is exposed in the AX tree. `modal.json` records exactly one refusal and `rootInert: true`, but `ax: []`. See F2. This was a DOM/AX audit, not a claim of measured VoiceOver speech. |
| 6 — Pointer events and hover | **PASS** | Installed Tailwind compilation saved as `disabled-compiled.css`: the variant nests `[aria-disabled="true"]` with `:not(*[data-focusable-when-disabled])`, declaring `pointer-events: none`; native `:disabled` retains its separate rule. Runtime held switch/Button: `auto`; ordinary disabled buttons (Review discovered skills, Edit, other row overflow): `none`. `hover.json` shows actual `:hover=true` on held Refresh all with unchanged transparent background and text color. The row event can bubble, but `SkillRow`'s `INTERACTIVE` guard ignores it; it does **not** open the inspector (`post-fix/proof.json`). |
| 7 — Tests | **PASS after fix** | Existing primitive tests pin event counts, reason, no change, focus, pending/native-disabled Button, and absence of extra live regions. Binder tests pin event→`refuseHeld` and cleanup; reporter/pure tests pin copy precedence, shared id and no history. These are public contracts, not private-function mocks. **Missing label path found:** added six primitive tests for wrapping/external labels, updated reasons, enabled behavior, public input refs and cleanup. Both wrapping-label tests failed before the fix (`label-red.log`); all seven relevant suites then passed, 153 tests (`label-green.log`). The original pointer CSS test checks class spelling only, so compiled/runtime evidence above—not that test alone—establishes selector correctness. |
| 8 — Copy/i18n | **PASS** | Commit adds only `heldRefusal.unchanged` at the end of `en`. Production notice title reuses the stated reason or translated lifecycle reason; no hardcoded UI copy added by the audit fix. `resources.test.ts` passes in the targeted and full runs. |
| 9 — DESIGN contract | **FAIL only on the unresolved announcement guarantee** | Button, Checkbox and Switch all link to the contract and match the direct activation behavior; audit adds the label/hidden-input contract. The unqualified assertion that the toaster's polite region announces the notice is still false in a modal. Do not silently weaken it into an exception without the release decision on F2. |
| 10 — Gate | **PASS (fresh post-fix run)** | `npm run version:check && npm run check` exits **0**: version 1.3.0; lint; styles guard (App.css 1108 / 36 migrated); **48 files / 1037 frontend tests**; TS build/Vite; Rust fmt/clippy; **836 Rust tests**, 0 failed. `post-fix/gate.log`, `post-fix/gate.exit`. First run failed rustfmt in sibling-owned `src-tauri/src/core/github_download.rs` while that audit was editing; later baseline and post-fix gates passed. No Rust file was edited here. Existing Vite bundle/dynamic-import warnings are nonblocking. |
| 11 — Residual release risks | **FAIL / decision needed for F2** | F2 is P1 accessibility, recommend resolving before 1.3.0 or explicitly waiving. F3 is P2 usability: confirmed header overlap, and hovering the toast extends it beyond the reported ~2 s. Shared toaster/layer changes are proposed below, not implemented in this bounded audit. |

## Defects found and fixes

### F1 — P2, fixed: the label part of a held Checkbox/Switch silently refuses

- **Location:** `src/components/ui/held-refusal.ts`; `src/components/ui/{checkbox,switch}.tsx`. Real call site: `src/components/projects/AssignmentMatrix.tsx:404` (label spans the matrix cell).
- **Reproduction:** failures fixture → Projects → monorepo. Activate the first matrix checkbox (Space) to start Project assignment. While it runs, click the right side of the next cell's label, outside its visible checkbox. Before the fix, the trusted LABEL click forwards a trusted detail-1 INPUT click; the root SPAN never receives it. Zero refusal events/toasts; target remains unchecked (`label.json`). The same gap reproduces with a wrapped Switch in the primitive test.
- **Fix:** observe the public Base UI `inputRef` seam in the shared refusal module, retaining caller object/callback refs and React 19 cleanup. The input path uses the visible control's reason; detail-0 root/key-forwarded clicks are excluded from duplication. No DOM sibling assumptions, no surface wiring, no mutation or new live region.
- **Proof:** wrapping-label regressions red before green; external labels and cleanup covered too. Trusted post-fix fixture label click produces **one** “Project assignment is running — wait for it to finish / Nothing was changed.” notice. Settlement: first checkbox checked, refused second unchecked, focus SPAN (`post-fix/label.json`). Original row/key proofs repeated after all source edits.

### F2 — P1, open: modal refusals have no accessible announcement

- **Locations:** `src/App.tsx:594` mounts the only toaster inside `#root`; `src/components/ui/layer-stack.ts:177` (`applyInert`) correctly makes that root inert while a modal stands open. The new refusal path relies on this inaccessible region.
- **Reproduction:** 960×640 failures fixture → api-style inspector sheet. Enter on “Disable on Claude Code”, then Enter on held “Disable on Codex”. One refusal event and visible toast, but its only polite region is inert and absent from the accessibility tree (`modal.json`, `modal.png`). The focus description remains available; it is **not** the activation announcement. The inspector sheet also has no running-action progress foot in this capture, so the implementer's general dialog-progress mitigation does not cover it.
- **Proposal:** make the existing single toast/announcement delivery layer-aware: deliver within the active modal boundary, or provide a centrally owned, non-interactive announcement host with an explicit exemption from background inerting/ARIA hiding. Preserve one announcement and the shared toast id; do not remove modal inertness or add per-control announcers. Test top/nested dialogs and Notifications suppression; run the operator's VoiceOver smoke.
- **Release assessment:** pre-existing toast infrastructure limitation, but this new contract relies on it. Cannot certify “every held control” with accessible refusal feedback as written. Larger shared-layer work, not a safe incidental audit patch.

### F3 — P2, open: the refusal toast obscures header controls and pauses under the pointer

- **Location:** `src/App.tsx:594` (`position="top-right"`), existing Sonner hover-pause behavior.
- **Reproduction:** start a row Disable, activate held Refresh all. The toast covers center hit points of Configure Tools, Refresh all and Add skill (`proof.json`, `enter.png`). Move onto the toast where the header button was and wait 3 s: it is **still present**. Move away: it then expires (`hover.json`). Thus “covers them for ~2 s” understates the pointer case.
- **Proposal:** put the shared toast stack in reserved space away from primary/header and modal actions, verifying both 960 and 1280 widths. Avoid a blanket pointer-events-none workaround: existing outcome toasts have View report actions. Do not disable hover pause globally, which helps people read notices.
- **Release assessment:** no wrong mutation or lost data, keyboard still works and moving away recovers; follow-up rather than an independent release blocker. It compounds F2's shared-toaster work.

| Before | Proposed after (not implemented) | Why |
|---|---|---|
| Polite region becomes inert with the app root | One announcement host inside the active accessible layer | Refusals must reach assistive technology without duplicate channels. |
| Toast covers header targets and pauses under their pointer | Reserved non-obstructing toast placement | Feedback should not block the next action or hide focused controls. |

## Changed files and boundaries

- `src/components/ui/held-refusal.ts`
- `src/components/ui/checkbox.tsx`
- `src/components/ui/switch.tsx`
- `src/components/ui/held-refusal.test.tsx`
- `DESIGN.md`
- This report and the dated ticket-27 Comment.

No changes to `src-tauri/**`, CHANGELOG, BACKLOG or sibling ticket/report. No commit/revert/stash/reset/checkout. All browser runs finished before tracked report/ticket writes; source edits were followed by fresh browser proofs. Test evidence and driver scripts are under the gitignored evidence root. Impeccable's executable was permission-denied; read PRODUCT/DESIGN directly instead. No claim of a full visual, all-theme, or native-screen-reader audit beyond this ticket's targeted checks.
