// Pins the storage-key contract between src/lib/preferences.ts and the one
// sanctioned storage read outside it: the pre-paint theme script in
// index.html (round-19 spec D6). Rename one literal and this fails.

import { describe, expect, it } from "vitest";
import indexHtml from "../../index.html?raw";
import { themePreference } from "./preferences";

describe("pre-paint theme script", () => {
  it("reads the same storage key the settings world writes", () => {
    const keys = [
      ...indexHtml.matchAll(/localStorage\.getItem\(\s*["']([^"']+)["']\s*\)/g),
    ].map((match) => match[1]);

    expect(keys).toEqual([themePreference.key]);
  });

  it("treats exactly the explicit choices as overrides of the system theme", () => {
    expect(indexHtml).toContain('pref === "light" || pref === "dark"');
    expect(themePreference.read()).toBe("system");
  });
});
