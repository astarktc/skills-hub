import { describe, expect, it } from "vitest";

import { resources } from "./resources";

/**
 * Catalog completeness guard: the UI is EN only (BACKLOG #61), so the one
 * thing that can drift is a `t("…")` call naming a key the `en` catalog does
 * not carry — i18next would render the raw key path instead of failing.
 *
 * The scan covers every literal first argument of a `t(` call in `src/`
 * (`t(…)`, `ctx.t(…)`, multi-line calls included). Template-literal keys
 * (`t(\`tools.${key}\`)`) cannot be checked statically and are skipped.
 */
const sources = import.meta.glob<string>(
  ["../**/*.{ts,tsx}", "!../**/*.test.{ts,tsx}", "!../bindings/**"],
  { query: "?raw", import: "default", eager: true },
);

const T_CALL = /\bt\(\s*(["'])([^"'`$\s]+)\1/g;

/** Resolve a dotted key; a plural key resolves through its `_one`/`_other` forms. */
function hasKey(key: string): boolean {
  let node: unknown = resources.en.translation;
  const parts = key.split(".");
  for (const [i, part] of parts.entries()) {
    if (node === null || typeof node !== "object") return false;
    const record = node as Record<string, unknown>;
    if (part in record) {
      node = record[part];
      continue;
    }
    const isLast = i === parts.length - 1;
    return isLast && (`${part}_one` in record || `${part}_other` in record);
  }
  return typeof node === "string";
}

function usedKeys(): Map<string, string> {
  const keys = new Map<string, string>();
  for (const [file, text] of Object.entries(sources)) {
    for (const match of text.matchAll(T_CALL)) {
      if (!keys.has(match[2])) keys.set(match[2], file);
    }
  }
  return keys;
}

describe("i18n catalog", () => {
  it("scans the source tree", () => {
    // Guards the glob itself: an empty scan would make the next test vacuous.
    expect(usedKeys().size).toBeGreaterThan(100);
  });

  it("carries every literal t() key used in src/", () => {
    const missing = [...usedKeys()]
      .filter(([key]) => !hasKey(key))
      .map(([key, file]) => `${key} (${file})`);
    expect(missing).toEqual([]);
  });

  it("ships only the en catalog", () => {
    expect(Object.keys(resources)).toEqual(["en"]);
  });
});
