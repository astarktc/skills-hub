# 29 — A toast raised while a modal is open is announced: the toaster stays live under the layer stack

Status: done — 27c0862
Blocked by: —
Blocks: 19 (1.3.0 push)
Source: BACKLOG #79 (absorbed here 2026-10-27 — operator decision "fix first", before the 1.3.0 push). Found by ticket
27's audit (`round19/review/ticket-27-audit.md` § F2, P1). Spec: D19 (refusal says why), D31 (a11y gate), the layer
stack contract (AGENTS.md "Every layer is on one stack", DESIGN.md § Dialog / § Toast).

## Problem

The one `Toaster` is mounted inside `#root` (`src/App.tsx` ~:594). When a modal layer opens, `layer-stack.ts::applyInert`
marks every `body` child except `data-base-ui-portal` nodes `inert` — `#root` included — so a toast raised while a
Dialog, ConfirmDialog or the inspector sheet stands open is **visible but absent from the accessibility tree**: its
`aria-live="polite"` region is inert, nothing is announced, and its action (View report) is unreachable.

Pre-existing for every toast (a run's outcome toast while the Add dialog's Result step is open, a refused `runAction`
toast inside a dialog). Ticket 27 sharpened it: a held control's refusal inside the inspector sheet ("Disable on
Codex" while "Disable on Claude Code" runs) is seen, not heard. Reproduction: `?scenario=failures&latency=3` at
960×640 → api-style inspector sheet → Enter on "Disable on Claude Code", then Enter on held "Disable on Codex":
one refusal event, a visible toast, `#root[inert]`, AX tree empty for the toast (`evidence/27-audit/modal.json`).

## Work

- **Fix once, in the shared layer** — never per control, never a second announcer. Two candidate shapes; choose and
  justify in `## Result`:
  1. **Exempt the toaster from inerting.** Mount the Toaster (or its region) as a `body` child outside `#root` through a
     portal carrying an explicit mark the layer stack recognises (e.g. `data-skillshub-live`), and have `applyInert`
     skip it the way it skips `data-base-ui-portal`. The toaster then stays live above every modal: announced, and its
     View report reachable by pointer. Check the interaction with Base UI's dialog focus trap (a toast's action button
     outside the modal: is Tab trapped inside the dialog, and does a pointer click on View report work? sonner's own
     hotkey focus?), with `inert`'s exclusion of the region from Tab order when there is no modal, and with the
     dismissal rule (one Escape dismisses one layer — a toast must not consume the modal's Escape).
  2. **An announcement host inside the active layer**: a non-interactive polite region the top modal layer owns, fed by
     the reporter, mirroring the toast text while a modal is open. More machinery (two surfaces for one notice).
  Option 1 is the expected answer unless its focus/Escape interplay proves wrong.
- **One announcement.** The notice is announced once, by one live region, with no duplicate when no modal is open.
  Keep the shared refusal toast id (`REFUSAL_TOAST_ID`) and the Notifications-panel suppression (`toastUnlessPanel`).
- **Layer contract.** DESIGN.md § Dialog / layer stack and the `layer-stack.ts` module doc state what the toaster is
  with respect to the stack (live above every layer; not a layer; dismisses nothing). AGENTS.md's "Every layer is on
  one stack" bullet gains the toaster's one-line exception if the shape needs one.
- **Fixture mode and tests.** The fixture mounts the same App, so it follows. `layer-stack` has unit tests
  (`src/components/ui/*.test.ts(x)`) — extend them for the exemption; no component-rendering tests for product
  surfaces (AGENTS.md).
- **Docs.** One CHANGELOG line in the 1.3.0 "Fixed" block — written by the orchestrator at commit, not the child.

## Acceptance

- **Fixture, trusted input (CDP, never `element.click()`):**
  - the ticket-27 modal reproduction above: the refusal toast's live region is **present in the accessibility tree**
    with the toast text while the sheet is open (CDP `Accessibility.getFullAXTree` or the audit's `modal.json` probe);
  - a run's outcome toast raised while a Dialog is open (e.g. Add → Result step, or a Delete dialog's bulk Remove)
    is likewise exposed, and its **View report** action works by pointer;
  - Tab from inside the open dialog stays inside the dialog (focus trap intact); Escape closes the dialog, not a
    toast; with no modal open, Tab order is unchanged from today;
  - the Notifications panel open → still no toast.
- **Unit tests:** `layer-stack` tests pin that the toaster's host is never inerted while a modal stands, and that
  everything else still is; the reporter tests still pass unchanged.
- **DESIGN.md** + `layer-stack.ts` doc updated; `AGENTS.md` only if the contract sentence needs the exception.
- **Gate:** `npm run version:check && npm run check` green.

## Result

**Shape: option 1 — the toaster's host is a marked `body` child the layer stack never inerts.** `ui/sonner.tsx`'s
`Toaster` now portals sonner into its own `<div data-skillshub-live>` (created with the Toaster, `prepend`ed to `body`
in a layout effect, removed with it); `layer-stack.ts` exports `LIVE_HOST_ATTRIBUTE` and `applyInert` skips that child
the way it skips `data-base-ui-portal`. One polite region (sonner's `<section aria-live="polite">`), one toast call
site (`showToast`), `REFUSAL_TOAST_ID` and `toastUnlessPanel` untouched; the primitive's props are unchanged and App's
mount is unchanged (the fix lives once, in the primitive + the stack). Why not option 2: a mirrored region inside the
top modal layer is a second surface for one notice (de-dup rules, two announcers to keep in sync, no clickable
View report under a modal) — and option 1's focus/Escape interplay held up in every check below, so there was no
reason to pay for it. Base UI's own `markOthers` (aria-hidden on everything outside the dialog) already keeps any
`[aria-live]` subtree visible, so ours was the only thing hiding the region.

**Focus / Escape / pointer interplay** (source read of sonner 2.0.7 + Base UI 1.8.0, then proven in the fixture):
- *Focus trap holds.* Base UI's modal focus guards sit inside the popup and wrap Tab within it; a tabbable toast
  outside is never reached by Tab/Shift+Tab (B: 24 Tabs + 8 Shift+Tabs with a live failure toast on screen, every stop
  inside the sheet).
- *A press on a toast is not an outside press.* With a backdrop, Base UI's dialog closes only on its own backdrop
  (`useDialogRoot` `outsidePress`); the toaster (z-index above the backdrop) takes the click. Pointer **View report**
  under the sheet opens the Notifications panel above it, focus in the panel (B).
- *Escape is the top layer's.* Sonner's document keydown only collapses its expanded stack when focus is in it;
  it claims nothing. Escape with a toast on screen closes the sheet and leaves the toast (C); with focus moved into the
  toaster by sonner's hotkey (⌥T) under the sheet, Escape still closes the sheet and focus returns to the row (C).
  The panel over the sheet: Escape 1 closes the panel only (focus back in the sheet), Escape 2 the sheet (B).
- *No-modal Tab order unchanged.* The host is the first `body` child; before, the Toaster was the first child of
  App's root `div` — the same document position relative to everything focusable. Measured (D): toast → Close toast →
  View report → My Skills → Explore …, and Shift+Tab from My Skills enters the toast's controls.
- Residual (accepted, documented): ⌥T can move focus out of a modal into the toaster; leaving it, sonner returns focus
  where it was. Not a trap break the operator can stumble into with Tab.

**Files touched:** `src/components/ui/sonner.tsx` (host portal + doc), `src/components/ui/layer-stack.ts`
(`LIVE_HOST_ATTRIBUTE`, `applyInert` skip, module doc: what the toaster is w.r.t. the stack),
`src/components/ui/layer-stack.test.ts`, new `src/components/ui/sonner.test.tsx` (primitive contract test),
`DESIGN.md` (§ Dialog "Background" exception + § Toaster "Live above every layer"), `AGENTS.md` (one-clause exception in
"Every layer is on one stack"). No i18n keys, no `src-tauri/**`, no CHANGELOG.

**Tests:** `layer-stack.test.ts` › "the live host (ticket 29 …)": *is never inerted while a modal layer stands; every
other body child still is* · *stays live under nested modal layers, and the layer beneath still goes inert* · *is not
a layer: it claims no event, so Escape still dismisses the top layer*. `sonner.test.tsx` › "Toaster host": *is the
first body child, outside the app root, and holds the one polite region* · *keeps a toast raised under a modal layer
out of the inert background* · *leaves with the Toaster*. Red check: dropping the `applyInert` skip fails 3 of them
(the two inert tests + the toast-under-modal test). Reporter tests (`useStatusReporter*.test.ts`) pass unchanged.

**Fixture proof** (trusted CDP keys/mouse, `?scenario=failures&latency=3`; evidence `.scratch/round19/evidence/29/`,
driver `proof.ts`, one JSON + output per part):
- **A** (960×640, ticket-27 reproduction, `A.json`, `A-refusal.png`): one refusal event; `#root[inert]`, host not inert;
  AX tree: region "Notifications alt+T" `live: polite` not ignored, StaticText "Disable is running — wait for it to
  finish" and "Nothing was changed." not ignored (27-audit `modal.json` had `ax: []`).
- **B** (960×640, `B.json`, `B-toast-under-sheet.png`, `B-view-report.png`): release-train sheet → Disable on Claude Code
  fails → toast "Could not remove from Claude Code" + View report, exposed in the AX tree (button "View report" not
  ignored); Tab trap; pointer View report → Notifications panel (sheet now inert beneath), Escape ×2 as above.
- **C** (`C.json`): Escape closes the sheet, not the toast; ⌥T then Escape likewise.
- **D** (1280×800, `D.json`): no-modal Tab order and AX exposure.
- **E/F** (1280×800): E was a wrong probe (the bell is held while a run is in flight, so the panel never opened — the
  toast it saw is the bell's own refusal, correct with no modal); **F** (`F.json`) opens the panel by pointer on View
  report, activates Copy all inside it ("Copied to clipboard" inline): **0 toasts added** while the panel is open.

**Gate:** `npm run version:check && npm run check` → exit 0 (`evidence/29/gate.log`, `gate.exit`): versions agree
(1.3.0); lint; styles:check ok (App.css 1108 / 36 migrated); 49 files / 1043 frontend tests; build; rustfmt; clippy;
Rust tests all ok. Bindings untouched.

## Comments

- **2026-10-27 (implementation child).** Option 1 implemented in the Toaster primitive + the layer stack (see Result);
  web-design-guidelines review of the live region/focus: "async updates need `aria-live=polite`" now holds under a
  modal; F3 (toast over header targets) is untouched and still open. Note for the orchestrator: the brief said the
  fixture server was running on :5175, but nothing listened there (connection refused), so this child started one
  (`npm run dev:fixture`, log `evidence/29/fixture-server.log`) and left it running. CHANGELOG line not written (per
  brief).

- **2026-10-27 (independent audit). PASS.** All ten audit checks pass; ticket-27 F2 is closed against its DOM/AX
  acceptance criteria. Independently repeated the sheet refusal and a centred bulk-Remove failure: one exposed polite
  toast region, working pointer View report above the dialog, trapped ordinary Tab/Shift+Tab, and one-layer Escape.
  Deliberate ⌥T access returns focus into the dialog and is acceptable for 1.3.0, not an accidental background escape.
  StrictMode host lifecycle leaves no duplicate/orphan. Removing the inert-exemption line makes three tests fail;
  restored byte-for-byte, the two primitive and two reporter suites pass (77 tests). Fresh full gate passes:
  1043 frontend / 836 Rust tests, version 1.3.0, styles 1108/36. No implementation fixes needed or source changes left.
  Report: `round19/review/ticket-29-audit.md`; own evidence: `round19/evidence/29-audit/`. Native VoiceOver speech was
  not measured; F3 / BACKLOG #80 remains separate. Browser closed before tracked writes; no commit.
- 2026-10-29 — **done — 27c0862** (post-rebase SHAs on `origin/main`; the pre-rebase SHAs cited above exist only in the orchestrator's local history). Shipped in 1.3.0 (`v1.3.0`, published 2026-10-28; operator smoke on the main Mac passed 2026-10-29, ticket 19 § Smoke outcome). 19a closure: `round19/spec.md` § Closure — 2026-10-29.
