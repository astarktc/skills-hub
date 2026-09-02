import { beforeEach, describe, expect, it } from "vitest";
import {
  booleanPreference,
  createPersistedPreference,
  unionPreference,
} from "./persistedPreference";
import {
  groupByRepoPreference,
  ignoredUpdateVersionPreference,
  languagePreference,
  projectsGroupByRepoPreference,
  showHiddenPreference,
  themePreference,
  viewModePreference,
} from "./preferences";

// jsdom here provides no localStorage; the module's contract is with the
// Web Storage interface, so stub it in memory.
function installMemoryStorage() {
  const memory = new Map<string, string>();
  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: {
      getItem: (key: string) => memory.get(key) ?? null,
      setItem: (key: string, value: string) => void memory.set(key, value),
      removeItem: (key: string) => void memory.delete(key),
      clear: () => memory.clear(),
    },
  });
}

beforeEach(() => {
  installMemoryStorage();
});

describe("createPersistedPreference", () => {
  it("round-trips a value through storage", () => {
    const pref = createPersistedPreference<number>({
      key: "test-number",
      encode: String,
      decode: (raw) => {
        const n = Number(raw);
        return Number.isFinite(n) ? n : null;
      },
      fallback: 0,
    });
    expect(pref.read()).toBe(0);
    pref.write(42);
    expect(window.localStorage.getItem("test-number")).toBe("42");
    expect(pref.read()).toBe(42);
  });

  it("returns the fallback for a corrupt stored value", () => {
    window.localStorage.setItem("test-number", "not-a-number");
    const pref = createPersistedPreference<number>({
      key: "test-number",
      encode: String,
      decode: (raw) => {
        const n = Number(raw);
        return Number.isFinite(n) ? n : null;
      },
      fallback: 7,
    });
    expect(pref.read()).toBe(7);
  });

  it("swallows storage failures on both read and write", () => {
    const original = window.localStorage;
    const throwing = {
      getItem() {
        throw new Error("denied");
      },
      setItem() {
        throw new Error("denied");
      },
    };
    Object.defineProperty(window, "localStorage", {
      value: throwing,
      configurable: true,
    });
    try {
      const pref = booleanPreference("test-flag");
      expect(pref.read()).toBe(false);
      expect(() => pref.write(true)).not.toThrow();
    } finally {
      Object.defineProperty(window, "localStorage", {
        value: original,
        configurable: true,
      });
    }
  });
});

describe("booleanPreference", () => {
  it("stores the legacy 'true'/'false' strings", () => {
    const pref = booleanPreference("test-flag");
    expect(pref.read()).toBe(false);
    pref.write(true);
    expect(window.localStorage.getItem("test-flag")).toBe("true");
    expect(pref.read()).toBe(true);
    pref.write(false);
    expect(window.localStorage.getItem("test-flag")).toBe("false");
    expect(pref.read()).toBe(false);
  });
});

describe("unionPreference", () => {
  it("accepts only allowed members and falls back otherwise", () => {
    const pref = unionPreference("test-union", ["a", "b"] as const, "a");
    window.localStorage.setItem("test-union", "zzz");
    expect(pref.read()).toBe("a");
    pref.write("b");
    expect(pref.read()).toBe("b");
  });
});

describe("app preference keys", () => {
  it("keeps every migrated storage key byte-identical", () => {
    expect(languagePreference.key).toBe("skills-language");
    expect(groupByRepoPreference.key).toBe("skills-groupByRepo");
    expect(viewModePreference.key).toBe("skills-viewMode");
    expect(projectsGroupByRepoPreference.key).toBe(
      "skills-projects-groupByRepo",
    );
    expect(showHiddenPreference.key).toBe("explore-showHidden");
    expect(themePreference.key).toBe("skills-theme");
    expect(ignoredUpdateVersionPreference.key).toBe(
      "skills-ignored-update-version",
    );
  });

  it("round-trips the view mode and rejects an unknown one", () => {
    expect(viewModePreference.read()).toBe("list");
    viewModePreference.write("dense-grid");
    expect(viewModePreference.read()).toBe("dense-grid");
    window.localStorage.setItem(viewModePreference.key, "hex-grid");
    expect(viewModePreference.read()).toBe("list");
  });

  it("round-trips the ignored update version", () => {
    expect(ignoredUpdateVersionPreference.read()).toBeNull();
    ignoredUpdateVersionPreference.write("1.2.3");
    expect(ignoredUpdateVersionPreference.read()).toBe("1.2.3");
  });
});
