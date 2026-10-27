import { useCallback, type Ref, type RefCallback } from "react";
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

type ReasonProps = { "aria-description"?: string | null; title?: string | null };

function statedReason(props: ReasonProps): string | null {
  const reason = props["aria-description"] ?? props.title;
  return reason && reason.trim() !== "" ? reason : null;
}

function dispatchRefusal(
  element: HTMLElement,
  reason = statedReason({
    "aria-description": element.getAttribute("aria-description"),
    title: element.getAttribute("title"),
  }),
): void {
  element.dispatchEvent(
    new CustomEvent<HeldRefusalDetail>(HELD_REFUSAL_EVENT, {
      bubbles: true,
      composed: true,
      detail: { reason },
    }),
  );
}

/** Attach a public ref, preserving its React 19 cleanup or legacy null call. */
function attachInputRef(ref: Ref<HTMLInputElement> | undefined, input: HTMLInputElement | null): () => void {
  if (typeof ref === "function") {
    const cleanup = ref(input);
    return () => {
      if (typeof cleanup === "function") cleanup();
      else ref(null);
    };
  }
  if (ref) ref.current = input;
  return () => { if (ref) ref.current = null; };
}

/**
 * A label activates Base UI's hidden input, a sibling of the visible control:
 * the root's capture handlers cannot hear it. Observe that public inputRef
 * seam too, using the visible control's reason. Root/key-forwarded clicks have
 * detail 0 and are already handled by the root, so they never count twice.
 * Preserve both object refs and React 19 callback-ref cleanup for callers.
 */
export function useHeldRefusalInputRef(
  held: boolean,
  props: ReasonProps & { inputRef?: Ref<HTMLInputElement> },
): RefCallback<HTMLInputElement> {
  const { inputRef } = props;
  const reason = statedReason(props);
  return useCallback((input) => {
    const cleanup = attachInputRef(inputRef, input);
    if (!input) return cleanup;
    const onClick = (event: MouseEvent) => {
      if (held && event.detail >= 1) dispatchRefusal(input, reason);
    };
    input.addEventListener("click", onClick, true);
    return () => {
      input.removeEventListener("click", onClick, true);
      cleanup();
    };
  }, [held, inputRef, reason]);
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
