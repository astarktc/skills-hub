import { useEffect } from "react";
import { HELD_REFUSAL_EVENT, type HeldRefusalDetail } from "@/components/ui/held-refusal";
import type { StatusReporter } from "./useStatusReporter";

/**
 * The binder's half of the held-control refusal (ticket 27, D19): every
 * primitive held disabled by a reason (Button, Switch, Checkbox) dispatches
 * one bubbling `HELD_REFUSAL_EVENT` per activation; this hook, mounted once
 * by App, hands each to the reporter's `refuseHeld`, which shows the reason.
 * Primitives never import the reporter, and no call site wires the notice.
 * One listener on `document`, so a held control anywhere — a row, the matrix,
 * a dialog's portal — is heard.
 */
export function useHeldRefusal(refuseHeld: StatusReporter["refuseHeld"]): void {
  useEffect(() => {
    const onRefusal = (event: Event) => {
      const detail = (event as CustomEvent<HeldRefusalDetail | undefined>).detail;
      refuseHeld(detail?.reason ?? null);
    };
    document.addEventListener(HELD_REFUSAL_EVENT, onRefusal);
    return () => document.removeEventListener(HELD_REFUSAL_EVENT, onRefusal);
  }, [refuseHeld]);
}
