# 29 — A toast raised while a modal is open is announced: the toaster stays live under the layer stack

Status: ready-for-agent
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

## Comments
