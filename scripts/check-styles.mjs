#!/usr/bin/env node
/**
 * styles:check — the styling guard (ADR-0006 §6). Reads the committed
 * baseline `scripts/styles-baseline.json` and fails when:
 *
 *   1. App.css grows — or shrinks without the ceiling being lowered in the
 *      same change (a ratchet). Its non-blank, non-comment line count must
 *      equal `appCssCeiling`; `appCssCeiling: null` means App.css must not
 *      exist.
 *   2. A migrated file references a legacy class — any class token in a
 *      file listed in `migrated`, or in any file under src/components/ui/
 *      (the primitives are born migrated), that App.css defines as a class
 *      selector.
 *   3. A new stylesheet appears — any *.css / *.scss / *.sass / *.less under
 *      src/ other than the permanent three and App.css.
 *   4. The prose layer leaks — a selector in styles/prose.css outside the
 *      prose root, or a colour literal there.
 *   5. A raw colour enters JSX — an arbitrary colour value (`bg-[#…]`,
 *      `text-[rgb(…)]`) or a Tailwind palette utility (`text-red-500`,
 *      `bg-white`) in src/**\/*.{ts,tsx}, outside `rawColourExempt`.
 *
 * Usage: node scripts/check-styles.mjs            (check)
 *        node scripts/check-styles.mjs --count    (print App.css's counted lines)
 */

import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, sep } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const BASELINE_PATH = join(ROOT, "scripts", "styles-baseline.json");
const APP_CSS = "src/App.css";
const PROSE_CSS = "src/styles/prose.css";
const PERMANENT_STYLESHEETS = ["src/index.css", "src/styles/theme.css", PROSE_CSS];
const PROSE_ROOT = "prose";
const STYLESHEET_EXT = /\.(css|scss|sass|less)$/;
const SOURCE_EXT = /\.(ts|tsx)$/;
const SOURCE_SKIP = [/\.test\.tsx?$/, /^src\/fixtures\//, /^src\/bindings\//];

const toPosix = (path) => path.split(sep).join("/");
const read = (rel) => readFileSync(join(ROOT, rel), "utf8");

function walk(dirRel) {
  const out = [];
  for (const name of readdirSync(join(ROOT, dirRel))) {
    const rel = `${dirRel}/${name}`;
    if (statSync(join(ROOT, rel)).isDirectory()) out.push(...walk(rel));
    else out.push(rel);
  }
  return out;
}

const stripComments = (css) => css.replace(/\/\*[\s\S]*?\*\//g, (m) => m.replace(/[^\n]/g, ""));

/** Non-blank, non-comment lines: the ratchet's unit. */
export function countCssLines(css) {
  return stripComments(css)
    .split("\n")
    .filter((line) => line.trim() !== "").length;
}

/**
 * Flatten a stylesheet into { selector, body } rules, descending into
 * at-rule blocks (@media, @supports, @layer). @keyframes / @font-face /
 * @property bodies are skipped (their "selectors" are not selectors).
 */
export function cssRules(css) {
  const src = stripComments(css);
  const rules = [];
  const parse = (text) => {
    let i = 0;
    while (i < text.length) {
      const open = text.indexOf("{", i);
      if (open < 0) break;
      const prelude = text.slice(i, open).trim();
      // matching close brace
      let depth = 1;
      let j = open + 1;
      while (j < text.length && depth > 0) {
        if (text[j] === "{") depth++;
        else if (text[j] === "}") depth--;
        j++;
      }
      const body = text.slice(open + 1, j - 1);
      const statementEnd = prelude.lastIndexOf(";");
      const head = statementEnd >= 0 ? prelude.slice(statementEnd + 1).trim() : prelude;
      if (head.startsWith("@")) {
        if (/^@(media|supports|layer|container|document)\b/.test(head)) parse(body);
      } else if (head) {
        rules.push({ selector: head, body });
      }
      i = j;
    }
  };
  parse(src);
  return rules;
}

/** Class names a stylesheet defines in its selectors. */
export function definedClasses(css) {
  const classes = new Set();
  for (const { selector } of cssRules(css)) {
    const withoutStrings = selector.replace(/"[^"]*"|'[^']*'/g, "");
    for (const match of withoutStrings.matchAll(/\.(-?[_a-zA-Z][_a-zA-Z0-9-]*)/g)) {
      classes.add(match[1]);
    }
  }
  return classes;
}

/** String-literal contents inside className={…} / className="…" and cn()/cva()/clsx() calls. */
export function classTokens(source) {
  const tokens = new Set();
  // In cva() configs, `variant: "solid"` / `tone: "danger"` name a variant,
  // not a class: skip literals that are values of a variant-dimension key.
  const VARIANT_KEY = /\b(?:variant|tone|size|orientation)\s*:\s*$/;
  const addLiteralTokens = (text) => {
    for (const lit of text.matchAll(/"([^"\\]*)"|'([^'\\]*)'|`([^`\\]*)`/g)) {
      if (VARIANT_KEY.test(text.slice(Math.max(0, lit.index - 24), lit.index))) continue;
      const value = (lit[1] ?? lit[2] ?? lit[3] ?? "").replace(/\$\{[^}]*\}/g, " ");
      for (const token of value.split(/\s+/)) if (token) tokens.add(token);
    }
  };
  const balanced = (text, start, openCh, closeCh) => {
    let depth = 0;
    for (let k = start; k < text.length; k++) {
      if (text[k] === openCh) depth++;
      else if (text[k] === closeCh && --depth === 0) return text.slice(start, k + 1);
    }
    return text.slice(start);
  };
  for (const m of source.matchAll(/className\s*=\s*(?=["'{])/g)) {
    const at = m.index + m[0].length;
    if (source[at] === "{") addLiteralTokens(balanced(source, at, "{", "}"));
    else addLiteralTokens(source.slice(at, source.indexOf(source[at], at + 1) + 1));
  }
  for (const m of source.matchAll(/\b(?:cn|cva|clsx)\s*\(/g)) {
    addLiteralTokens(balanced(source, m.index + m[0].length - 1, "(", ")"));
  }
  return tokens;
}

const COLOUR_FUNCTIONS = /\b(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch|color)\(/i;
const HEX = /#[0-9a-fA-F]{3,8}\b/;
// CSS named colours (CSS Color 4), minus the keywords that are not literals.
const NAMED_COLOURS = new Set(
  "aliceblue antiquewhite aqua aquamarine azure beige bisque black blanchedalmond blue blueviolet brown burlywood cadetblue chartreuse chocolate coral cornflowerblue cornsilk crimson cyan darkblue darkcyan darkgoldenrod darkgray darkgreen darkgrey darkkhaki darkmagenta darkolivegreen darkorange darkorchid darkred darksalmon darkseagreen darkslateblue darkslategray darkslategrey darkturquoise darkviolet deeppink deepskyblue dimgray dimgrey dodgerblue firebrick floralwhite forestgreen fuchsia gainsboro ghostwhite gold goldenrod gray green greenyellow grey honeydew hotpink indianred indigo ivory khaki lavender lavenderblush lawngreen lemonchiffon lightblue lightcoral lightcyan lightgoldenrodyellow lightgray lightgreen lightgrey lightpink lightsalmon lightseagreen lightskyblue lightslategray lightslategrey lightsteelblue lightyellow lime limegreen linen magenta maroon mediumaquamarine mediumblue mediumorchid mediumpurple mediumseagreen mediumslateblue mediumspringgreen mediumturquoise mediumvioletred midnightblue mintcream mistyrose moccasin navajowhite navy oldlace olive olivedrab orange orangered orchid palegoldenrod palegreen paleturquoise palevioletred papayawhip peachpuff peru pink plum powderblue purple rebeccapurple red rosybrown royalblue saddlebrown salmon sandybrown seagreen seashell sienna silver skyblue slateblue slategray slategrey snow springgreen steelblue tan teal thistle tomato turquoise violet wheat white whitesmoke yellow yellowgreen".split(
    " ",
  ),
);

/** Colour literals in a declaration block (values only, not property names). */
export function colourLiterals(body) {
  const found = [];
  for (const declaration of body.split(";")) {
    const colon = declaration.indexOf(":");
    if (colon < 0) continue;
    const value = declaration.slice(colon + 1).replace(/var\([^)]*\)/g, "");
    if (HEX.test(value)) found.push(value.match(HEX)[0]);
    if (COLOUR_FUNCTIONS.test(value)) found.push(value.match(COLOUR_FUNCTIONS)[0]);
    for (const word of value.toLowerCase().match(/[a-z]+/g) ?? []) {
      if (NAMED_COLOURS.has(word)) found.push(word);
    }
  }
  return found;
}

const COLOUR_UTILITY_PREFIX =
  "(?:bg|text|border(?:-[trblxyse])?|ring|ring-offset|outline|fill|stroke|from|via|to|decoration|divide|accent|caret|placeholder|shadow|inset-shadow|drop-shadow)";
const PALETTE =
  "(?:slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-(?:50|[1-9]00|950)";
const PALETTE_UTILITY = new RegExp(
  `(?:^|[\\s"'\`:])(${COLOUR_UTILITY_PREFIX}-(?:${PALETTE}|black|white)(?:/\\d+)?)(?=$|[\\s"'\`])`,
  "g",
);
const ARBITRARY_COLOUR = new RegExp(
  `(${COLOUR_UTILITY_PREFIX}-\\[(?:color:)?(?:#[0-9a-fA-F]{3,8}|(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch|color)\\()[^\\]]*\\])`,
  "g",
);

/** Raw colour utilities in a source file. */
export function rawColourUtilities(source) {
  const found = [];
  for (const m of source.matchAll(PALETTE_UTILITY)) found.push(m[1]);
  for (const m of source.matchAll(ARBITRARY_COLOUR)) found.push(m[1]);
  return found;
}

function loadBaseline() {
  let baseline;
  try {
    baseline = JSON.parse(readFileSync(BASELINE_PATH, "utf8"));
  } catch (error) {
    throw new Error(`styles-baseline.json is unreadable: ${error.message}`);
  }
  for (const key of ["appCssCeiling", "migrated", "rawColourExempt"]) {
    if (!(key in baseline)) throw new Error(`styles-baseline.json: missing "${key}"`);
  }
  return baseline;
}

function check() {
  const baseline = loadBaseline();
  const failures = [];
  const fail = (rule, message) => failures.push(`[${rule}] ${message}`);
  const appCssExists = existsSync(join(ROOT, APP_CSS));

  // 1. App.css ratchet
  if (baseline.appCssCeiling === null) {
    if (appCssExists) fail("growth", `${APP_CSS} must not exist (appCssCeiling is null).`);
  } else if (!appCssExists) {
    fail("growth", `${APP_CSS} is gone: set "appCssCeiling": null in the baseline.`);
  } else {
    const lines = countCssLines(read(APP_CSS));
    if (lines > baseline.appCssCeiling) {
      fail(
        "growth",
        `${APP_CSS} has ${lines} counted lines, above its ceiling ${baseline.appCssCeiling}. ` +
          "App.css only shrinks: style the surface with utilities and primitives instead.",
      );
    } else if (lines < baseline.appCssCeiling) {
      fail(
        "growth",
        `${APP_CSS} shrank to ${lines} counted lines: lower "appCssCeiling" from ` +
          `${baseline.appCssCeiling} to ${lines} in scripts/styles-baseline.json (the ratchet).`,
      );
    }
  }

  // 2. Migrated files reference no legacy class
  const legacy = appCssExists ? definedClasses(read(APP_CSS)) : new Set();
  const primitives = walk("src/components/ui")
    .map(toPosix)
    .filter((file) => SOURCE_EXT.test(file) && !/\.test\.tsx?$/.test(file));
  for (const file of new Set([...baseline.migrated, ...primitives])) {
    if (!existsSync(join(ROOT, file))) {
      fail("migrated", `${file} is listed as migrated but does not exist.`);
      continue;
    }
    const hits = [...classTokens(read(file))].filter((token) => legacy.has(token));
    if (hits.length > 0) {
      fail("migrated", `${file} references legacy App.css classes: ${hits.sort().join(", ")}.`);
    }
  }

  // 3. No new stylesheet
  const allowed = new Set([...PERMANENT_STYLESHEETS, APP_CSS]);
  for (const file of walk("src").map(toPosix)) {
    if (STYLESHEET_EXT.test(file) && !allowed.has(file)) {
      fail(
        "stylesheet",
        `${file} is a new stylesheet. The permanent stylesheets are ${PERMANENT_STYLESHEETS.join(", ")}; ` +
          "style with utilities and primitives (tokens go in styles/theme.css).",
      );
    }
  }

  // 4. Prose layer: scoped selectors, tokens only
  if (existsSync(join(ROOT, PROSE_CSS))) {
    const rootPattern = new RegExp(`^\\.${PROSE_ROOT}(?=$|[\\s.:#\\[>+~])`);
    for (const { selector, body } of cssRules(read(PROSE_CSS))) {
      for (const part of selector.split(",").map((s) => s.trim())) {
        if (!rootPattern.test(part)) {
          fail("prose", `${PROSE_CSS}: selector "${part}" is outside the prose root .${PROSE_ROOT}.`);
        }
      }
      for (const literal of colourLiterals(body)) {
        fail("prose", `${PROSE_CSS}: colour literal "${literal}" in "${selector}" — read a token with var().`);
      }
    }
  }

  // 5. No raw colour in JSX / TS
  const exempt = new Set(baseline.rawColourExempt.map((entry) => entry.file));
  for (const file of walk("src").map(toPosix)) {
    if (!SOURCE_EXT.test(file) || SOURCE_SKIP.some((re) => re.test(file)) || exempt.has(file)) {
      continue;
    }
    const hits = rawColourUtilities(read(file));
    if (hits.length > 0) {
      fail(
        "raw-colour",
        `${file}: raw colour utilities ${[...new Set(hits)].join(", ")} — use a semantic token utility ` +
          "(bg-card, text-muted-foreground, text-failed, …).",
      );
    }
  }
  for (const file of exempt) {
    if (!existsSync(join(ROOT, file))) {
      fail("raw-colour", `rawColourExempt lists ${file}, which no longer exists: drop the entry.`);
    }
  }

  return failures;
}

if (process.argv.includes("--count")) {
  console.log(countCssLines(read(APP_CSS)));
} else {
  const failures = check();
  if (failures.length > 0) {
    console.error(`styles:check failed (${failures.length}):`);
    for (const failure of failures) console.error(`  ${failure}`);
    process.exit(1);
  }
  const baseline = loadBaseline();
  console.log(
    `styles:check ok — App.css ${baseline.appCssCeiling ?? "absent"} counted lines (ceiling), ` +
      `${baseline.migrated.length} migrated file(s), prose layer scoped, no raw colours.`,
  );
}

export { check };
