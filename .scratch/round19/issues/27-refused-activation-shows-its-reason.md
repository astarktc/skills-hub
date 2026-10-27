# 27 — A refused click says why: held-disabled controls show their reason when activated

Status: ready-for-agent
Blocked by: —
Blocks: 19 (1.3.0 push)
Source: ticket 18, P2·Q8 and its reproduction (`18-operator-think-aloud.md` § Findings; operator hold decision
2026-10-27). Spec: D19 (one mutation at a time, refused with a typed reason, never queued), D21 (focus), ADR-0006.

## Problem

While an action runs, every other mutation control is **held disabled**: it stays focusable (`focusableWhenDisabled`,
`data-focusable-when-disabled`, `aria-disabled="true"`), and its reason ("Disable is running — wait for it to finish")
is its `aria-description` / `title`. A pointer click on a held control does **nothing visible**. A switch doesn't
move, and the reason appears only on hover.

In the think-aloud, the operator flipped three row switches in quick succession and believed all three skills were
off. **One stayed on.** Reproduced on the fixture (`?scenario=failures&latency=3`, row switches clicked 250 ms
apart): only the first turned off, and the second and third were silently refused. In 1.3.0, a fast operator can
believe a skill is off when it is on.

## Work

- **One rule for every held control.** Activating a control held disabled by a reason surfaces that reason
  **visibly**, once per activation. Activation means a pointer click, Space or Enter. This covers every primitive that
  carries `data-focusable-when-disabled`: Switch, Button and Checkbox. The rule lives in one place, not at each call
  site.
- **Keep D19.** The click is still refused, never queued, and nothing changes state.
- **Placement and form are yours to choose and justify in the Result.** Two candidates:
  - a short info toast through the reporter ("Still disabling noisy-linter — try again in a moment");
  - a transient inline note anchored to the control.

  Primitives must not import the reporter or any world hook (AGENTS.md state rules). A bubbling DOM event that App's
  binder turns into `notify(...)` is one clean seam; a primitive-level announcer is another.
- **Screen readers.** The reason is already announced on focus as the description, so the visible notice must not
  double-announce noisily. Use one polite announcement, or the toaster's existing polite region.
- **Copy.** English keys in `src/i18n/resources.ts`. Prefer naming the running action (the reporter's `pending.label`)
  over a generic "busy".
- **Docs.** DESIGN.md documents the held-control refusal contract for each affected primitive (AGENTS.md: a primitive's
  behaviour change is documented in the same change). Add one CHANGELOG line to the 1.3.0 block.

## Acceptance

- **Fixture reproduction:** `?scenario=failures&latency=3`, three row switches clicked 250 ms apart. The 2nd and 3rd
  clicks each show the reason visibly; their state is unchanged; the first completes. Drive it with trusted input
  (CDP), not `element.click()`.
- **Keyboard:** Space on a held switch shows the same notice, and focus stays on the switch (never BODY).
- **Tests:**
  - primitive tests (`src/components/ui/*.test.tsx`) pin the event/announcement contract for Switch, Button and
    Checkbox;
  - a hook or pure test covers the binder's mapping to `notify`.
- **Gate:** `npm run version:check && npm run check` green.

## Comments

- **2026-10-27 (implementation child).** Implemented with the bubbling-DOM-event seam + the reporter's refusal toast
  (see Result). Fixture proof with trusted CDP input under `.scratch/round19/evidence/27/`. Found while proving it: a
  held control was `pointer-events: none` (`aria-disabled:pointer-events-none` in `disabledState`), so a click on it
  never reached it — it fell through to the row beneath (which opened the inspector). Held controls now take the
  pointer; every other disabled control stays pointer-transparent. Open for the orchestrator: CHANGELOG line (not
  written here, per brief); the app-wide toaster sits top-right over the header's Configure Tools / Refresh all /
  Add skill, so a refusal toast briefly covers them (a trusted click at Refresh all landed on the toast in
  `proof-buttons-run1-*`) — a toaster-placement question, out of this ticket's scope.

- **2026-10-27 (independent audit of `c8b727d`).** **FAIL / hold unconditional release sign-off**, details in
  `../review/ticket-27-audit.md`. Independently repeated trusted three-click and Space/Enter proofs; all pass.
  Found and fixed a missed activation path: a held Checkbox/Switch's label forwards to Base UI's hidden input,
  bypassing the root handler and silently refusing. Shared `held-refusal.ts` input-ref adapter, six primitive
  regressions (red before green), DESIGN contract note; matrix-label proof and original row/key proofs repeated
  post-fix under `../evidence/27-audit/post-fix/`. Full gate exits 0: 1037 frontend / 836 Rust tests, version 1.3.0.
  Open P1: the sole polite toast region is inert/absent from the accessibility tree inside an inspector sheet
  (no dialog-progress mitigation there); needs a shared layer-aware announcement fix or explicit release exception.
  Open P2: header-covering toast persists beyond 2 s while hovered, verified at 3 s. No Rust/CHANGELOG/BACKLOG edits,
  no commits; fixes and report left for the orchestrator.

## Result

**Approach.** A held control (Button, Switch or Checkbox `aria-disabled="true"` with the `data-focusable-when-disabled`
mark — `disabled` + `focusableWhenDisabled`, or a `pending` Button) dispatches one bubbling
`skillshub:held-refusal` CustomEvent per activation, `detail.reason` = its stated reason (`aria-description`, else
`title`, else `null`). App's binder (`useHeldRefusal`) hands it to the reporter's new `refuseHeld(reason)`, which shows
the one refusal toast: the stated reason — else the running action named via `pending.label`
(`describeRefusal`) — over "Nothing was changed.".

- **Seam: the bubbling DOM event, not a primitive-level announcer.** The refusal already has one dialect — a refused
  `runAction` toasts `describeRefusal` as an unrecorded `info` toast — and only the reporter knows what is running
  (`pending.label`). The event keeps primitives free of the reporter/world hooks (AGENTS.md state rules) and needs no
  call-site wiring: the rule lives in one module (`ui/held-refusal.ts`), spread by the three primitives.
- **Form: the toast, not an inline anchored note.** The toaster already owns a polite live region (`aria-live="polite"`),
  so the reason is announced once without a second announcer; an anchored note would be a new floating layer on the
  layer stack per primitive, would cover neighbouring cells in the dense matrix/rows, and would need its own
  dismissal/focus contract. The toast shares the runAction refusal's id (`lifecycle-refusal`): a repeat while it
  shows refreshes it (sonner restarts its timer — run 1 shows it leaving 2 s after the 3rd click) instead of stacking
  copies, and identical text is not re-announced. It records nothing (nothing ran, D19) and, like every toast, yields
  to the open Notifications panel.
- **Activation = once:** `click` with `detail ≥ 1` (pointer) or `keydown` Enter/Space (not `repeat`); the click a key
  synthesizes (`detail: 0`, native button or Base UI's dispatch) is not counted again. Handlers are capture-phase on
  the control because Base UI drops a disabled control's bubble handlers; they only observe — Base UI still refuses.
- **Pointer fix (needed for the click to be heard):** `disabledState`'s `aria-disabled:pointer-events-none` became
  `aria-disabled:not-data-focusable-when-disabled:pointer-events-none`; Button's hover grounds became
  `not-aria-disabled:hover:*` so a held button does not light up under the pointer.
- **Copy:** one new key, `heldRefusal.unchanged` ("Nothing was changed."); the title reuses the control's own reason /
  `lifecycle.running` so the notice, the hover title and the `aria-description` never disagree.
- **Known limit:** while a modal is open the toaster (inside `#root`) is inert, so the toast is visible but not
  announced — true of every toast today; the held controls in dialogs also show the running action in the dialog's
  own progress foot.

**Files.** `src/components/ui/held-refusal.ts` (new), `button.tsx`, `switch.tsx`, `checkbox.tsx`, `styles.ts`;
`src/hooks/useHeldRefusal.ts` (new), `src/hooks/useStatusReporter.ts` (`refuseHeld`, refusal toast id),
`src/lib/actionLifecycle.ts` (`heldRefusalNotice`), `src/App.tsx` (mounts `useHeldRefusal`),
`src/i18n/resources.ts` (`heldRefusal.unchanged`, appended); `DESIGN.md` (§ Button "Held", § Checkbox and Switch
"Held-control refusal" contract, § Refusal). Tests: `src/components/ui/held-refusal.test.tsx` (new),
`src/hooks/useHeldRefusal.test.ts` (new), `useStatusReporter.test.ts`, `actionLifecycle.test.ts`,
`useAddSkillFlow.test.ts` (mock gains `refuseHeld`). No `src-tauri/**`, no CHANGELOG.

**Tests.**
- `held-refusal.test.tsx` — for each of Switch, Checkbox, Button ("$name held by a reason (ticket 27)"): a pointer
  click dispatches one refusal with the stated reason, changes nothing and keeps focus · Space and Enter each dispatch
  one refusal; the click a key synthesizes is not counted again · falls back to the title · carries a null reason
  when the control states none · still runs the caller's own capture handler · renders no live region of its own ·
  takes the pointer; other disabled controls stay pointer-transparent · an enabled control dispatches nothing. Plus
  "Button pending … is held" and "a natively disabled button … dispatches nothing".
- `useHeldRefusal.test.ts` — hands each refusal bubbling from anywhere in the document to refuseHeld, with its reason ·
  stops listening when the binder unmounts.
- `useStatusReporter.test.ts` "useStatusReporter refuseHeld (ticket 27)" — shows the reason as the one refusal toast
  and records nothing · names the running action when the control states no reason; shares the refused runAction's
  toast id · says nothing for no reason while idle · yields to an open Notifications panel.
- `actionLifecycle.test.ts` "heldRefusalNotice (ticket 27)" — stated reason first · running action named · null.

**Fixture proof** (`?scenario=failures&latency=3`, 1280×800, headless agent-browser session, trusted CDP
`Input.dispatchMouseEvent` / `Input.dispatchKeyEvent`; driver `.scratch/round19/evidence/27/proof.ts`,
`proof-buttons.ts`; page-side recorder observes only):
- Three row switches (alias-skill, api-style, corp-conventions) clicked 250 ms apart: click 1 starts Disable; clicks
  2 and 3 (`isTrusted`, `detail: 1`) each dispatch the refusal and the toast "Disable is running — wait for it to
  finish / Nothing was changed." is on screen after each; api-style and corp-conventions stay `aria-checked="true"`;
  alias-skill settles off. `proof.json`, `proof-summary.txt`, `clicks-after-2nd.png`, `clicks-after-3rd.png`,
  `clicks-settled.png`. (`run1-startup-scan/` — the first run clicked during the startup Scan: all three were
  refused with "Scan is running…", toast left 2 s after the 3rd click.)
- Space: api-style focused, trusted Space starts its Disable (switch holds focus), a second trusted Space shows the
  same notice; state unchanged until its own Disable settles; focus stayed on the switch throughout, no
  `focus-lost-to-body` event. `space-held.png`, `proof.json` › `space`.
- Held Buttons: trusted clicks on the held "Refresh all" and the row's `pending` overflow each show the notice; no
  menu opens, no Refresh starts, focus never reaches BODY. `proof-buttons.json`, `proof-buttons-summary.txt`,
  `buttons-held.png`.

**Gate.** `npm run version:check && npm run check` → exit 0 (`.scratch/round19/evidence/27/gate.log`): version check
ok · lint clean · styles:check ok (App.css 1108, 36 migrated) · vitest 48 files / 1031 tests passed · build ok ·
cargo fmt/clippy clean · cargo test 831 passed, 0 failed (with the sibling's in-progress `src-tauri` edits present).
