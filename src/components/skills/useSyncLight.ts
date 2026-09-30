import { useState } from "react";
import type { StripMarkState } from "../../lib/skillPresentation";

/**
 * The Synced strip lighting on sync completion — the contract's signature
 * interaction (DESIGN.md § Motion): `true` from the render in which a mark
 * *becomes* synced while it is on screen (from not synced, failed, missing,
 * broken, stale or disabled) until its state changes again. A mark that
 * mounts synced — first paint, a filter, a scroll, another skill in the
 * inspector — is simply lit: nothing moves on navigation (frequency first).
 *
 * The glyph keys itself by state, so the one that becomes synced is a new
 * element and enters through `@starting-style` once; while `true` its
 * starting style stays declared, which is inert after that first frame.
 */
export function useSyncLight(state: StripMarkState): boolean {
  const [seen, setSeen] = useState(state);
  const [lit, setLit] = useState(false);
  if (state !== seen) {
    setSeen(state);
    setLit(state === "synced");
  }
  return lit;
}
