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

/**
 * Retired vocabulary (round 19 D45/D46, ticket 32): the operator-facing words
 * are Harness, Deploy / Deployed / Deployment, Deploy to project, Re-deploy,
 * Update / Update all and the state words (Deployed \u00b7 Not deployed \u00b7
 * Not deployed yet \u00b7 Out of date copy \u00b7 Copy missing \u00b7 Failed \u00b7
 * Disabled). Code keeps `tool` / `sync` / `refresh` / `assignment` — so keys and
 * `{{tool}}`-style interpolation names are exempt; only rendered text is checked.
 */
const RETIRED_WORDS = [
  "tool", "tools",
  "sync", "synced", "syncing", "syncs", "resync", "resynced", "unsync",
  "refresh", "refreshed", "refreshing",
  "assign", "assigned", "assigning", "assignment", "assignments", "unassign", "unassigned",
  "stale",
] as const;

const RETIRED = new RegExp(`\\b(${RETIRED_WORDS.join("|")})\\b`, "i");

/**
 * Leaves allowed to keep a retired word, each with its reason. Keep it short:
 * a new entry needs a reason a reviewer would accept.
 */
const RETIRED_WORD_ALLOW_LIST: Readonly<Record<string, string>> = {};

/** Every `en` leaf as `[dotted.key, value]`. */
function leaves(node: unknown, prefix = ""): Array<[string, string]> {
  if (typeof node === "string") return [[prefix, node]];
  if (node === null || typeof node !== "object") return [];
  return Object.entries(node as Record<string, unknown>).flatMap(([key, value]) =>
    leaves(value, prefix ? `${prefix}.${key}` : key),
  );
}

/** The rendered text of a value: interpolation names (`{{tool}}`) are code, not copy. */
const rendered = (value: string) => value.replace(/\{\{[^}]*\}\}/g, "");

function retiredWordHits(catalog: unknown): string[] {
  return leaves(catalog)
    .filter(([key, value]) => !(key in RETIRED_WORD_ALLOW_LIST) && RETIRED.test(rendered(value)))
    .map(([key, value]) => `${key}: ${value}`);
}

/**
 * The reverse guard (BACKLOG #82): every `en` leaf is read somewhere in `src/`.
 * A key is read when the source names it as a quoted literal (`t("a.b")`, or a
 * key map's value such as `HEALTH_ROW_LABEL_KEY`), or when it sits under a
 * prefix the source builds dynamically — a template literal `` `a.b.${…}` ``,
 * detected from the source itself (`DYNAMIC_PREFIX`), so a new dynamic family
 * needs no hand-kept list. A plural leaf (`_one` / `_other` …) is read through
 * its base key. Anything else is dead copy: delete it.
 */
const DYNAMIC_PREFIX = /`([a-zA-Z][\w.]*)\.\$\{/g;

/** Leaves the scan cannot see but the app reads, each with its reason. Keep it empty if you can. */
const UNREAD_ALLOW_LIST: Readonly<Record<string, string>> = {};

const PLURAL_SUFFIX = /_(zero|one|two|few|many|other)$/;

function unreadLeaves(catalog: unknown, texts: readonly string[]): string[] {
  const text = texts.join("\n");
  const prefixes = new Set([...text.matchAll(DYNAMIC_PREFIX)].map((match) => `${match[1]}.`));
  const quoted = (key: string) => {
    const escaped = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    return new RegExp(`["'\`]${escaped}["'\`]`).test(text);
  };
  const keys = [...new Set(leaves(catalog).map(([key]) => key.replace(PLURAL_SUFFIX, "")))];
  return keys.filter(
    (key) => !(key in UNREAD_ALLOW_LIST) && !quoted(key) && ![...prefixes].some((prefix) => key.startsWith(prefix)),
  );
}

/** The app's own sources, without the catalog itself (its property names are not reads). */
const appSources = Object.entries(sources)
  .filter(([file]) => !file.endsWith("/i18n/resources.ts"))
  .map(([, text]) => text);

describe("i18n catalog", () => {
  it("reads every en leaf somewhere in src/ (BACKLOG #82)", () => {
    expect(unreadLeaves(resources.en.translation, appSources)).toEqual([]);
  });

  it("the reverse check sees literal, mapped, plural and template-built reads, and flags the rest", () => {
    // Guards the check itself: a too-generous matcher would make the test above vacuous.
    const catalog = {
      a: { read: "x", mapped: "x", dead: "x" },
      count_one: "x",
      count_other: "x",
      tools: { cursor: "Cursor" },
      orphan_one: "x",
      orphan_other: "x",
    };
    const texts = ['t("a.read")', 'const KEY = { k: "a.mapped" };', 't("count", { count })', "t(`tools.${key}`)"];
    expect(unreadLeaves(catalog, texts)).toEqual(["a.dead", "orphan"]);
  });

  it("uses no retired vocabulary word in any en value (D45)", () => {
    expect(retiredWordHits(resources.en.translation)).toEqual([]);
  });

  it("the retired-words check catches a reinserted word and ignores keys and interpolation names", () => {
    // Guards the check itself: a broken regex would make the test above vacuous.
    expect(retiredWordHits({ a: { b: "Sync to Tools" } })).toEqual(["a.b: Sync to Tools"]);
    expect(retiredWordHits({ c: "Refresh all", d: "Not assigned", e: "stale" })).toHaveLength(3);
    expect(retiredWordHits({ tools: { sync: "Deploy to {{tool}} ({{tools}})" } })).toEqual([]);
    expect(retiredWordHits({ f: "Toolbar", g: "GitHub token", h: "synchronous" })).toEqual([]);
  });

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
