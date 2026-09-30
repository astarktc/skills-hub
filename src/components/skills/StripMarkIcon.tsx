import { cn } from "@/lib/utils";
import type { StripMarkState } from "../../lib/skillPresentation";
import { useSyncLight } from "./useSyncLight";

/**
 * The glyph of each mark state, drawn on a 12 × 12 grid in `currentColor`.
 * Shape carries the state (DESIGN.md § Synced strip): filled = synced,
 * hollow = not synced, half-filled = stale, × = failed / missing / broken,
 * slashed = disabled — distinguishable in grayscale; colour reinforces.
 */
function MarkGlyph({ state }: { state: StripMarkState }) {
  switch (state) {
    case "synced":
      return <circle cx="6" cy="6" r="4.5" fill="currentColor" />;
    case "not_synced":
      return <circle cx="6" cy="6" r="4" fill="none" stroke="currentColor" strokeWidth="1.5" />;
    case "stale":
      return (
        <>
          <circle cx="6" cy="6" r="4" fill="none" stroke="currentColor" strokeWidth="1.5" />
          <path d="M6 2a4 4 0 0 0 0 8z" fill="currentColor" />
        </>
      );
    case "disabled":
      return (
        <>
          <circle cx="6" cy="6" r="4" fill="none" stroke="currentColor" strokeWidth="1.5" />
          <path d="M3.2 8.8 8.8 3.2" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
        </>
      );
    case "failed":
    case "missing":
    case "broken":
      return <path d="M2.5 2.5l7 7m0-7-7 7" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />;
  }
}

/** Colour reinforces the shape; a complete strip is the quiet one. */
function markTone(state: StripMarkState, quiet: boolean): string {
  switch (state) {
    case "synced":
      return quiet ? "text-input" : "text-muted-foreground";
    case "not_synced":
      return "text-input";
    case "stale":
      return "text-stale";
    case "disabled":
      return "text-faint";
    case "failed":
    case "missing":
    case "broken":
      return "text-failed";
  }
}

/**
 * One Sync-target mark (12 px, decorative — the caller names it): the
 * Synced strip's and the inspector's Sync targets list's shared vocabulary.
 * `quiet` = the strip is complete (N/N), so synced marks step down to `input`.
 *
 * A mark that becomes synced on screen lights (the contract's signature
 * interaction): its glyph is a new element that fades in from opacity 0 over
 * 120 ms (`--motion-fast`, `ease-out`) — opacity only, once, never on mount;
 * reduced motion shows it at once. The ticket-21 import cascade staggers the
 * same light with a per-mark delay.
 */
export default function StripMarkIcon({ state, quiet = false }: { state: StripMarkState; quiet?: boolean }) {
  const lit = useSyncLight(state);
  return (
    <svg
      key={state}
      viewBox="0 0 12 12"
      data-lit={lit || undefined}
      className={cn(
        "size-3 shrink-0 transition-opacity duration-(--motion-fast) ease-out motion-reduce:transition-none",
        lit && "starting:opacity-0",
        markTone(state, quiet),
      )}
      aria-hidden="true"
    >
      <MarkGlyph state={state} />
    </svg>
  );
}
