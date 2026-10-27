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
