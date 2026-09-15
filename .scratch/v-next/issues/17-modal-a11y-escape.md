# 17: Modal shell — a11y labelling, initial focus, Escape-to-close

Status: resolved

Type: task
Blocked by: None (can start immediately)

## What to build

The shell (`src/components/shared/Modal.tsx`) renders `role="dialog"`/`aria-modal="true"` unconditionally (ticket 08's fix) but the dialog announces without a name, receives no focus, and can't be dismissed by keyboard. Implement once in the shell, inherited by all 12 modals:

- **Labelling**: wire the rendered title to the dialog via `aria-labelledby` (generate an id, e.g. `useId`); when there's no title (or `plain` mode), accept an optional `aria-label` prop so callers can name the dialog.
- **Initial focus**: move focus into the dialog on open (the dialog container or first focusable — keep it simple and predictable).
- **Escape-to-close**: pressing Escape triggers `onRequestClose`, honoring the same `canClose` gate the backdrop click uses. This clears the nicety queued on the map since ticket 08.

Behavior parity otherwise — no other interaction changes. No JSX tests by design (per AGENTS.md); verify by reading + the gate.

## Acceptance criteria

- [x] Every modal announces with a name (`aria-labelledby` or `aria-label`); `role`/`aria-modal` unchanged.
- [x] Focus enters the dialog on open; Escape closes exactly when the backdrop click would.
- [x] `npm run version:check && npm run check` green (`> /tmp/gate.log 2>&1; echo $?`).

## Answer

Landed green in `946a44c` (Fable 5 low subagent; orchestrator-verified, rebased, combined-tree gate green). Shell owns it all: `useId`-based `aria-labelledby` when a title renders, optional `aria-label` prop otherwise (3 titleless call sites named from existing i18n keys — no new keys); `tabIndex={-1}` container focused on mount (mount === open); document-level Escape gated by the exact `canClose` predicate the backdrop uses (so `closeDisabled` correctly blocks Escape mid-install). No focus trap / focus-restore — kept to ticket scope. This clears the Escape nicety queued since ticket 08.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
