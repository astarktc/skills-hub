import { useCallback, useState } from "react";
import type { PersistedPreference } from "../lib/persistedPreference";

/**
 * View state backed by a persisted preference: seeded from storage on first
 * render, written back on every change. The preference object owns the key,
 * the codec and the ignore-on-failure rule, so it must be module-stable.
 */
export function usePersistedPreference<T>(
  preference: PersistedPreference<T>,
): [T, (value: T) => void] {
  const [value, setValue] = useState<T>(() => preference.read());
  const set = useCallback(
    (next: T) => {
      setValue(next);
      preference.write(next);
    },
    [preference],
  );
  return [value, set];
}
