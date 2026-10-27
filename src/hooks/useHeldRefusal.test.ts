// The binder's mapping of the primitives' held-refusal event to the
// reporter (ticket 27): one event, one `refuseHeld(reason)`; mounted once,
// removed on unmount.

import { renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { HELD_REFUSAL_EVENT } from "@/components/ui/held-refusal";
import { useHeldRefusal } from "./useHeldRefusal";

afterEach(() => {
  document.body.innerHTML = "";
});

function refuseFrom(element: HTMLElement, detail?: { reason: string | null }) {
  element.dispatchEvent(new CustomEvent(HELD_REFUSAL_EVENT, { bubbles: true, detail }));
}

describe("useHeldRefusal", () => {
  it("hands each refusal bubbling from anywhere in the document to refuseHeld, with its reason", () => {
    const refuseHeld = vi.fn();
    renderHook(() => useHeldRefusal(refuseHeld));
    const control = document.createElement("button");
    // A held control inside a portal-like subtree still reaches the binder.
    const portal = document.createElement("div");
    portal.append(control);
    document.body.append(portal);

    refuseFrom(control, { reason: "Disable is running — wait for it to finish" });
    refuseFrom(control, { reason: null });
    refuseFrom(control);

    expect(refuseHeld.mock.calls).toEqual([["Disable is running — wait for it to finish"], [null], [null]]);
  });

  it("stops listening when the binder unmounts", () => {
    const refuseHeld = vi.fn();
    const { unmount } = renderHook(() => useHeldRefusal(refuseHeld));
    unmount();
    const control = document.createElement("button");
    document.body.append(control);
    refuseFrom(control, { reason: "x" });
    expect(refuseHeld).not.toHaveBeenCalled();
  });
});
