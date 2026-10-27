import type * as React from "react";

/**
 * The held-control refusal (round-19 ticket 27, D19). A control held disabled
 * by a reason — Button, Switch or Checkbox with `focusableWhenDisabled` (or a
 * `pending` Button) — stays focusable and refuses every change. Activating it
 * must not do nothing visible: the primitive dispatches one bubbling
 * `HELD_REFUSAL_EVENT` per activation, carrying the reason the control states
 * (its `aria-description`, else its `title`). The binder turns that event
 * into the reporter's refusal notice (`useHeldRefusal` → `refuseHeld`), so no
 * primitive imports the reporter or a world hook, and no call site wires it.
 *
 * One activation is one event:
 * - a pointer click (`click` with `detail >= 1`);
 * - a key press, Enter or Space (`keydown`, never an auto-repeat).
 * A click a key press synthesizes (native `<button>` or Base UI's own
 * dispatch, both `detail: 0`) is the same activation as its key and is not
 * counted again; a programmatic `element.click()` is not an activation.
 * The handlers run in the capture phase on the control itself, because Base
 * UI drops a disabled control's bubble-phase handlers. They observe only:
 * the refusal itself stays Base UI's (nothing toggles, nothing is queued).
 */
export const HELD_REFUSAL_EVENT = "skillshub:held-refusal";

export type HeldRefusalDetail = {
  /** The reason the control states (`aria-description`, else `title`), or null. */
  reason: string | null;
};

type CaptureHandlers = {
  onClickCapture?: React.MouseEventHandler<HTMLElement>;
  onKeyDownCapture?: React.KeyboardEventHandler<HTMLElement>;
};

function statedReason(element: HTMLElement): string | null {
  const reason = element.getAttribute("aria-description") ?? element.getAttribute("title");
  return reason && reason.trim() !== "" ? reason : null;
}

function dispatchRefusal(element: HTMLElement): void {
  element.dispatchEvent(
    new CustomEvent<HeldRefusalDetail>(HELD_REFUSAL_EVENT, {
      bubbles: true,
      composed: true,
      detail: { reason: statedReason(element) },
    }),
  );
}

/**
 * The capture handlers a primitive spreads on its control: while `held`, an
 * activation dispatches the refusal event, then the caller's own capture
 * handler (if any) runs as before. Not held: the caller's handlers, untouched.
 */
export function heldRefusalHandlers<T extends CaptureHandlers>(held: boolean, props: T): CaptureHandlers {
  const { onClickCapture, onKeyDownCapture } = props;
  if (!held) return { onClickCapture, onKeyDownCapture };
  return {
    onClickCapture: (event) => {
      if (event.detail >= 1) dispatchRefusal(event.currentTarget);
      onClickCapture?.(event);
    },
    onKeyDownCapture: (event) => {
      if (!event.repeat && (event.key === "Enter" || event.key === " ") && event.target === event.currentTarget) {
        dispatchRefusal(event.currentTarget);
      }
      onKeyDownCapture?.(event);
    },
  };
}
