/**
 * Persisted view preference: one storage key + codec + ignore-on-failure
 * rule per preference. Pure (no React) so it can be read from module scope,
 * a state initializer, or a test. Storage failures and corrupt values never
 * throw — the fallback is used instead.
 */

export type PersistedPreference<T> = {
  /** The literal localStorage key; part of the contract with existing users. */
  readonly key: string;
  read(): T;
  write(value: T): void;
};

export type PreferenceCodec<T> = {
  key: string;
  encode: (value: T) => string;
  /** Returns null when the stored value is unusable, so the fallback wins. */
  decode: (raw: string) => T | null;
  fallback: T;
};

export function createPersistedPreference<T>({
  key,
  encode,
  decode,
  fallback,
}: PreferenceCodec<T>): PersistedPreference<T> {
  return {
    key,
    read() {
      if (typeof window === "undefined") return fallback;
      try {
        const raw = window.localStorage.getItem(key);
        if (raw === null) return fallback;
        return decode(raw) ?? fallback;
      } catch {
        return fallback;
      }
    },
    write(value: T) {
      if (typeof window === "undefined") return;
      try {
        window.localStorage.setItem(key, encode(value));
      } catch {
        // ignore storage failures
      }
    },
  };
}

/** A boolean stored as the legacy `"true"` / `"false"` strings. */
export function booleanPreference(
  key: string,
  fallback = false,
): PersistedPreference<boolean> {
  return createPersistedPreference<boolean>({
    key,
    encode: String,
    decode: (raw) => (raw === "true" ? true : raw === "false" ? false : null),
    fallback,
  });
}

/** A string from a closed set; anything else reads as the fallback. */
export function unionPreference<T extends string>(
  key: string,
  allowed: readonly T[],
  fallback: T,
): PersistedPreference<T> {
  return createPersistedPreference<T>({
    key,
    encode: (value) => value,
    decode: (raw) => (allowed.includes(raw as T) ? (raw as T) : null),
    fallback,
  });
}

/** A free-form string; absent storage reads as null. */
export function stringPreference(
  key: string,
): PersistedPreference<string | null> {
  return createPersistedPreference<string | null>({
    key,
    encode: (value) => value ?? "",
    decode: (raw) => raw,
    fallback: null,
  });
}
