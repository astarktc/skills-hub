import ts from "typescript";
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
 * Read from the TypeScript AST, not from text (BACKLOG #99 (3)) — a key named
 * in a comment, a log line, a comparison or a type is not a read:
 *
 * - **A translation call** reads its first argument — `t("a.b")`, `ctx.t(…)`,
 *   through `?:` branches, `??` / `||`, parentheses and `as`.
 * - **A key hand-off** reads a key-shaped literal that is data passed on to a
 *   translation call later: an object property's value (a key map such as
 *   `HEALTH_ROW_LABEL_KEY`, a `titleKey:`), an array element, a variable's
 *   initializer, a returned value, or an argument of a call to a
 *   `KEY_FORWARDERS` function. This is still an approximation: the literal is
 *   not traced to the `t` call that reads it (`const k = "a.b"; t(k)` counts
 *   at the constant).
 * - **A dynamic family**: a translation call whose key is built — a template
 *   (`` t(`tools.${id}`) ``) or a concatenation (`t("a." + leaf)`) — reads
 *   every leaf matching its literal pieces (`tools.<anything>`). A template
 *   that never reaches a translation call exempts nothing.
 *
 * A plural leaf (`_one` / `_other` …) is read through its base key. Anything
 * else is dead copy: delete it.
 */
type SourceFileText = readonly [fileName: string, text: string];

/** Functions that translate a key they are handed, each with where and why. Keep it short. */
const KEY_FORWARDERS: Readonly<Record<string, string>> = {
  collectRemovalFailures: "reportOutcome.ts: its title key is t()'d for every kept row",
  part: "reportOutcome.ts tallyHeadline: `part(key, count)` pushes t(key, { count })",
};

/** Leaves the scan cannot see but the app reads, each with its reason. Keep it empty if you can. */
const UNREAD_ALLOW_LIST: Readonly<Record<string, string>> = {};

const PLURAL_SUFFIX = /_(zero|one|two|few|many|other)$/;
const KEY_SHAPED = /^[A-Za-z][\w-]*(?:\.[\w-]+)*$/;

const isTranslationCall = (node: ts.CallExpression) => {
  const callee = node.expression;
  return ts.isIdentifier(callee)
    ? callee.text === "t"
    : ts.isPropertyAccessExpression(callee) && callee.name.text === "t";
};

const calleeName = (node: ts.CallExpression) => {
  const callee = node.expression;
  if (ts.isIdentifier(callee)) return callee.text;
  return ts.isPropertyAccessExpression(callee) ? callee.name.text : null;
};

/** The expressions a value can take: `?:` branches, `??` / `||` sides, parentheses, `as`. */
function alternatives(node: ts.Expression): ts.Expression[] {
  if (ts.isParenthesizedExpression(node) || ts.isAsExpression(node) || ts.isSatisfiesExpression(node))
    return alternatives(node.expression);
  if (ts.isConditionalExpression(node)) return [...alternatives(node.whenTrue), ...alternatives(node.whenFalse)];
  if (
    ts.isBinaryExpression(node) &&
    (node.operatorToken.kind === ts.SyntaxKind.QuestionQuestionToken ||
      node.operatorToken.kind === ts.SyntaxKind.BarBarToken)
  )
    return [...alternatives(node.left), ...alternatives(node.right)];
  return [node];
}

/** A key expression's pieces: literal text, or null where a value is spliced in. */
function keyPieces(node: ts.Expression): Array<string | null> {
  if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) return [node.text];
  if (ts.isTemplateExpression(node))
    return [node.head.text, ...node.templateSpans.flatMap((span) => [null, span.literal.text])];
  if (ts.isBinaryExpression(node) && node.operatorToken.kind === ts.SyntaxKind.PlusToken)
    return [...keyPieces(node.left), ...keyPieces(node.right)];
  if (ts.isParenthesizedExpression(node)) return keyPieces(node.expression);
  return [null];
}

/** Where a literal's value goes once `?:` / `??` / `||` / `&&` / parentheses / `as` are climbed out of. */
function valueSite(node: ts.Node): [ts.Node, ts.Node | undefined] {
  let child: ts.Node = node;
  let parent = node.parent;
  while (
    parent &&
    (ts.isParenthesizedExpression(parent) ||
      ts.isAsExpression(parent) ||
      ts.isSatisfiesExpression(parent) ||
      (ts.isConditionalExpression(parent) && parent.condition !== child) ||
      (ts.isBinaryExpression(parent) &&
        parent.right === child &&
        [
          ts.SyntaxKind.QuestionQuestionToken,
          ts.SyntaxKind.BarBarToken,
          ts.SyntaxKind.AmpersandAmpersandToken,
        ].includes(parent.operatorToken.kind)))
  ) {
    child = parent;
    parent = parent.parent;
  }
  return [child, parent];
}

function isHandOff(child: ts.Node, parent: ts.Node | undefined, forwarders: Readonly<Record<string, string>>) {
  if (!parent) return false;
  if (ts.isPropertyAssignment(parent)) return parent.initializer === child;
  if (ts.isVariableDeclaration(parent)) return parent.initializer === child;
  if (ts.isArrowFunction(parent)) return parent.body === child;
  if (ts.isArrayLiteralExpression(parent) || ts.isReturnStatement(parent)) return true;
  if (ts.isCallExpression(parent)) {
    const name = calleeName(parent);
    return name !== null && name in forwarders && parent.arguments.some((arg) => arg === child);
  }
  return false;
}

/** Every static key read and every dynamic family the sources translate. */
function translationReads(files: readonly SourceFileText[], forwarders = KEY_FORWARDERS) {
  const keys = new Set<string>();
  const families: RegExp[] = [];
  for (const [fileName, text] of files) {
    const kind = fileName.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS;
    const sf = ts.createSourceFile(fileName, text, ts.ScriptTarget.Latest, true, kind);
    const visit = (node: ts.Node) => {
      if (ts.isCallExpression(node) && isTranslationCall(node) && node.arguments[0]) {
        for (const branch of alternatives(node.arguments[0])) {
          const pieces = keyPieces(branch);
          if (pieces.every((piece) => piece !== null)) keys.add(pieces.join(""));
          else if (pieces.some((piece) => piece))
            families.push(
              new RegExp(
                `^${pieces.map((piece) => (piece === null ? ".+" : piece.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))).join("")}$`,
              ),
            );
        }
      } else if ((ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) && KEY_SHAPED.test(node.text)) {
        const [child, parent] = valueSite(node);
        if (isHandOff(child, parent, forwarders)) keys.add(node.text);
      }
      ts.forEachChild(node, visit);
    };
    visit(sf);
  }
  return { keys, families };
}

function unreadLeaves(
  catalog: unknown,
  files: readonly SourceFileText[],
  forwarders: Readonly<Record<string, string>> = KEY_FORWARDERS,
): string[] {
  const { keys: read, families } = translationReads(files, forwarders);
  const keys = [...new Set(leaves(catalog).map(([key]) => key.replace(PLURAL_SUFFIX, "")))];
  return keys.filter(
    (key) => !(key in UNREAD_ALLOW_LIST) && !read.has(key) && !families.some((family) => family.test(key)),
  );
}

/** The app's own sources, without the catalog itself (its property names are not reads). */
const appSources: SourceFileText[] = Object.entries(sources).filter(
  ([file]) => !file.endsWith("/i18n/resources.ts"),
);

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
    expect(unreadLeaves(catalog, texts.map((text, i) => [`f${i}.ts`, text] as const))).toEqual(["a.dead", "orphan"]);
  });

  // Round-19 ticket-31 audit F3: the lexical scan's four counterexamples.
  describe("the reverse check reads syntax, not text (BACKLOG #99 (3))", () => {
    const catalog = { a: { b: "x" } };
    const unread = (text: string, forwarders: Readonly<Record<string, string>> = {}) =>
      unreadLeaves(catalog, [["probe.ts", text]], forwarders);

    it("a key held in a constant counts at the constant (a key map, by approximation)", () => {
      expect(unread('const k = "a.b"; t(k);')).toEqual([]);
    });

    it("a concatenated key reaching t reads its family", () => {
      expect(unread('t("a." + x);')).toEqual([]);
      expect(unread('t("b." + x);')).toEqual(["a.b"]);
    });

    it("a key-shaped string handed to anything but a translation is not a read", () => {
      expect(unread('console.log("a.b");')).toEqual(["a.b"]);
      expect(unread('if (x === "a.b") y();')).toEqual(["a.b"]);
      expect(unread('// t("a.b")\n/* "a.b" */ type K = "a.b";')).toEqual(["a.b"]);
    });

    it("a template that never reaches t exempts nothing", () => {
      expect(unread("const unused = `a.${x}`;")).toEqual(["a.b"]);
      expect(unread("t(`a.${x}`);")).toEqual([]);
    });

    it("a family keeps its literal tail; a key forwarder reads only when listed", () => {
      const nested = { a: { q: { title: "x", body: "x" } } };
      expect(unreadLeaves(nested, [["probe.ts", "t(`a.${x}.title`);"]], {})).toEqual(["a.q.body"]);
      expect(unread('helper(out, "a.b");')).toEqual(["a.b"]);
      expect(unread('helper(out, "a.b");', { helper: "test" })).toEqual([]);
      expect(unread('t(flag ? "a.b" : other ?? "c.d");')).toEqual([]);
    });

    it("every listed key forwarder still exists in src/", () => {
      const text = appSources.map(([, source]) => source).join("\n");
      const stale = Object.keys(KEY_FORWARDERS).filter(
        (name) => !new RegExp(`\\b(?:function\\s+${name}\\b|(?:const|let)\\s+${name}\\s*=)`).test(text),
      );
      expect(stale).toEqual([]);
    });
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
