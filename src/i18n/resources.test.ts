import { describe, expect, it } from "vitest";

import { resources } from "./resources";

/**
 * Locale parity guard: every user-facing string must exist in both catalogs.
 *
 * i18next silently falls back to `en` for a missing `zh` key, so a divergence
 * ships as English text to a Chinese user instead of failing anywhere — this
 * test is the only enforcement (`errors.skillNotFoundInRepo` and the whole
 * `projects` namespace were EN-only until ticket 33).
 */
function keyPaths(value: unknown, prefix = ""): string[] {
  if (value === null || typeof value !== "object") return [prefix];
  return Object.entries(value as Record<string, unknown>).flatMap(([key, v]) =>
    keyPaths(v, prefix ? `${prefix}.${key}` : key),
  );
}

const enKeys = keyPaths(resources.en.translation).sort();
const zhKeys = keyPaths(resources.zh.translation).sort();

describe("i18n catalogs", () => {
  it("has identical key sets in en and zh", () => {
    const zhSet = new Set(zhKeys);
    const enSet = new Set(enKeys);
    expect({
      missingInZh: enKeys.filter((k) => !zhSet.has(k)),
      missingInEn: zhKeys.filter((k) => !enSet.has(k)),
    }).toEqual({ missingInZh: [], missingInEn: [] });
  });

  it("carries copy for every error key referenced by describeCommandError", () => {
    // Spot-check the variants typed in ticket 33 (the compiler guards the
    // code union; nothing guards that the i18n keys exist).
    for (const key of [
      "errors.unknownTool",
      "errors.invalidPathMissing",
      "errors.invalidPathNotADirectory",
      "errors.invalidPath",
      "projects.notFoundError",
    ]) {
      expect(enKeys, `en is missing ${key}`).toContain(key);
      expect(zhKeys, `zh is missing ${key}`).toContain(key);
    }
  });
});
