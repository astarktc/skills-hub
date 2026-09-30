import { useCallback, useSyncExternalStore } from "react";

/**
 * The shell breakpoint (DESIGN.md § Layout, D23): at and above it the
 * sidebar is full and the inspector sits beside the list; below it the rail
 * and the sheet. The same value as `--breakpoint-shell` in theme.css.
 */
export const SHELL_WIDE_QUERY = "(min-width: 75rem)";

/** Whether a media query matches now, re-rendering when it changes. */
export function useMediaQuery(query: string): boolean {
  const subscribe = useCallback(
    (onChange: () => void) => {
      if (typeof window === "undefined" || !window.matchMedia) return () => {};
      const list = window.matchMedia(query);
      list.addEventListener("change", onChange);
      return () => list.removeEventListener("change", onChange);
    },
    [query],
  );
  const read = useCallback(
    () => (typeof window !== "undefined" && window.matchMedia ? window.matchMedia(query).matches : true),
    [query],
  );
  return useSyncExternalStore(subscribe, read, () => true);
}
