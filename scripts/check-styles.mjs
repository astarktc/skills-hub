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
 *      selector. Class expressions are read from the TypeScript AST:
 *      `className` attributes and cn()/cva()/clsx() arguments, following
 *      constants, objects, conditionals, templates and helper functions —
 *      same-file and through named relative / `@/` imports.
 *   3. A new stylesheet appears — any *.css / *.scss / *.sass / *.less under
 *      src/ other than the permanent three and App.css.
 *   4. The prose layer leaks — a selector in styles/prose.css (nesting
 *      resolved) that does not start at the prose root or leaves it through
 *      a sibling combinator (`.prose + x`, `.prose ~ x`), or a colour literal
 *      anywhere in a value, `var()` fallbacks included.
 *   5. A raw colour enters JSX — an arbitrary colour value (`bg-[#…]`,
 *      `text-[rgb(…)]`, `bg-[red]`, `[color:red]`), a Tailwind palette
 *      utility (`text-red-500`, `bg-white`), or a colour literal in an inline
 *      `style={{…}}` in src/**\/*.{ts,tsx}, outside `rawColourExempt`.
 *
 * Before checking the repository it runs its own regression suite
 * (scripts/check-styles.test.mjs: every rule's violations exit 1 against a
 * temp copy, the clean copy exits 0). `--no-self-test` skips it (the suite's
 * own runs use it).
 *
 * Usage: node scripts/check-styles.mjs            (self-test, then check)
 *        node scripts/check-styles.mjs --count    (print App.css's counted lines)
 */

import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, realpathSync, statSync } from "node:fs";
import { dirname, join, sep } from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";

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
 * Split `text` at top-level occurrences of `separator` (outside strings,
 * parentheses and brackets).
 */
export function splitTopLevel(text, separator) {
  const parts = [];
  let depth = 0;
  let quote = null;
  let start = 0;
  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (quote) {
      if (ch === "\\") i++;
      else if (ch === quote) quote = null;
    } else if (ch === '"' || ch === "'") quote = ch;
    else if (ch === "\\") i++;
    else if (ch === "(" || ch === "[") depth++;
    else if (ch === ")" || ch === "]") depth--;
    else if (ch === separator && depth === 0) {
      parts.push(text.slice(start, i));
      start = i + 1;
    }
  }
  parts.push(text.slice(start));
  return parts;
}

/** Resolve a nested selector list against its parent's (CSS nesting). */
function resolveNesting(selector, parent) {
  const own = splitTopLevel(selector, ",").map((part) => part.trim()).filter(Boolean);
  if (parent === null) return own.join(", ");
  const parents = splitTopLevel(parent, ",").map((part) => part.trim());
  const out = [];
  for (const p of parents) {
    for (const part of own) {
      out.push(part.includes("&") ? part.replace(/&/g, p) : `${p} ${part}`);
    }
  }
  return out.join(", ");
}

/**
 * Flatten a stylesheet into { selector, body } rules — `selector` with CSS
 * nesting resolved, `body` the rule's own declarations (nested blocks cut
 * out) — descending into grouping at-rules (@media, @supports, @layer,
 * @container). @keyframes / @font-face / @property bodies are skipped (their
 * "selectors" are not selectors). String- and comment-aware.
 */
export function cssRules(css) {
  const src = stripComments(css);
  const rules = [];
  const parse = (text, parentSelector) => {
    let declarations = "";
    let segment = 0;
    let i = 0;
    let quote = null;
    while (i < text.length) {
      const ch = text[i];
      if (quote) {
        if (ch === "\\") i++;
        else if (ch === quote) quote = null;
        i++;
        continue;
      }
      if (ch === '"' || ch === "'") {
        quote = ch;
        i++;
        continue;
      }
      if (ch !== "{") {
        i++;
        continue;
      }
      // matching close brace (string-aware)
      let depth = 1;
      let j = i + 1;
      let q = null;
      while (j < text.length && depth > 0) {
        const c = text[j];
        if (q) {
          if (c === "\\") j++;
          else if (c === q) q = null;
        } else if (c === '"' || c === "'") q = c;
        else if (c === "{") depth++;
        else if (c === "}") depth--;
        j++;
      }
      const chunk = text.slice(segment, i);
      const statementEnd = chunk.lastIndexOf(";");
      declarations += chunk.slice(0, statementEnd + 1);
      const head = chunk.slice(statementEnd + 1).trim();
      const body = text.slice(i + 1, j - 1);
      if (head.startsWith("@")) {
        if (/^@(media|supports|layer|container|document|scope|starting-style)\b/.test(head)) {
          parse(body, parentSelector);
        }
      } else if (head) {
        parse(body, resolveNesting(head, parentSelector));
      }
      i = j;
      segment = j;
    }
    declarations += text.slice(segment);
    if (parentSelector !== null) rules.push({ selector: parentSelector, body: declarations });
  };
  parse(src, null);
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

/**
 * Why a prose selector escapes the prose root, or null when it is contained.
 * Contained: the complex selector starts with the root compound (`.prose`,
 * optionally refined: `.prose:not(.x)`, `.prose.dark`) and leaves it only
 * through a descendant or child combinator — never a sibling one.
 */
export function proseEscape(complexSelector, root) {
  const selector = complexSelector.trim();
  const rootStart = new RegExp(`^\\.${root}(?=$|[\\s.:#\\[>+~])`);
  if (!rootStart.test(selector)) return "does not start at the prose root";
  // Walk the root compound, then read the first top-level combinator.
  let depth = 0;
  let quote = null;
  for (let i = root.length + 1; i < selector.length; i++) {
    const ch = selector[i];
    if (quote) {
      if (ch === "\\") i++;
      else if (ch === quote) quote = null;
      continue;
    }
    if (ch === '"' || ch === "'") quote = ch;
    else if (ch === "\\") i++;
    else if (ch === "(" || ch === "[") depth++;
    else if (ch === ")" || ch === "]") depth--;
    else if (depth === 0 && /[\s>+~]/.test(ch)) {
      const rest = selector.slice(i).trimStart();
      if (rest.startsWith("+") || rest.startsWith("~")) {
        return "leaves the prose root through a sibling combinator";
      }
      return null;
    }
  }
  return null;
}

// --- TypeScript-aware class and style reading --------------------------------

const CLASS_HELPERS = new Set(["cn", "cva", "clsx", "cx", "twMerge", "twJoin"]);

function sourceFile(source, fileName) {
  const kind = fileName.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS;
  return ts.createSourceFile(fileName, source, ts.ScriptTarget.Latest, true, kind);
}

/** Declarations by name in one file: const/let initializers, functions, imports. */
function fileScope(sf) {
  const locals = new Map();
  const imports = new Map();
  const add = (name, node) => {
    if (!locals.has(name)) locals.set(name, []);
    locals.get(name).push(node);
  };
  const visit = (node) => {
    if (ts.isVariableDeclaration(node) && ts.isIdentifier(node.name) && node.initializer) {
      add(node.name.text, node.initializer);
    } else if (ts.isFunctionDeclaration(node) && node.name && node.body) {
      add(node.name.text, node);
    } else if (
      ts.isImportDeclaration(node) &&
      ts.isStringLiteral(node.moduleSpecifier) &&
      node.importClause?.namedBindings &&
      ts.isNamedImports(node.importClause.namedBindings)
    ) {
      for (const element of node.importClause.namedBindings.elements) {
        imports.set(element.name.text, {
          module: node.moduleSpecifier.text,
          exported: (element.propertyName ?? element.name).text,
        });
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(sf);
  return { locals, imports };
}

const scopeCache = new WeakMap();
const moduleCache = new Map();

/** A repository module by root-relative path, or null. */
function loadModule(rel) {
  if (moduleCache.has(rel)) return moduleCache.get(rel);
  let sf = null;
  if (existsSync(join(ROOT, rel))) sf = sourceFile(read(rel), rel);
  moduleCache.set(rel, sf);
  return sf;
}

/** Resolve a relative or `@/` import to a root-relative source path. */
function resolveImport(fromFile, specifier) {
  let base;
  if (specifier.startsWith("@/")) base = `src/${specifier.slice(2)}`;
  else if (specifier.startsWith("./") || specifier.startsWith("../")) {
    base = toPosix(join(dirname(fromFile), specifier));
  } else return null;
  for (const candidate of [base, `${base}.ts`, `${base}.tsx`, `${base}/index.ts`, `${base}/index.tsx`]) {
    if (SOURCE_EXT.test(candidate) && existsSync(join(ROOT, candidate))) return candidate;
  }
  return null;
}

/**
 * The declarations `name` refers to from `sf`: same-file first, then a
 * named import from a repository module (followed one module at a time).
 */
function lookup(sf, name, depth = 0) {
  if (!scopeCache.has(sf)) scopeCache.set(sf, fileScope(sf));
  const scope = scopeCache.get(sf);
  const local = scope.locals.get(name);
  if (local) return local;
  const imported = scope.imports.get(name);
  if (!imported || depth > 8) return [];
  const target = resolveImport(sf.fileName, imported.module);
  const module = target && loadModule(target);
  return module ? lookup(module, imported.exported, depth + 1) : [];
}

function propertyName(name) {
  if (ts.isIdentifier(name) || ts.isStringLiteral(name) || ts.isNumericLiteral(name)) return name.text;
  return null;
}

function unwrap(node) {
  while (
    node &&
    (ts.isParenthesizedExpression(node) ||
      ts.isAsExpression(node) ||
      ts.isSatisfiesExpression?.(node) ||
      ts.isNonNullExpression(node) ||
      ts.isTypeAssertionExpression(node))
  ) {
    node = node.expression;
  }
  return node;
}

/** Return-statement expressions (or the expression body) of a function. */
function returnedExpressions(fn) {
  if (!fn.body) return [];
  if (!ts.isBlock(fn.body)) return [fn.body];
  const out = [];
  const visit = (node) => {
    if (ts.isFunctionLike(node) && node !== fn) return;
    if (ts.isReturnStatement(node) && node.expression) out.push(node.expression);
    ts.forEachChild(node, visit);
  };
  ts.forEachChild(fn.body, visit);
  return out;
}

/**
 * The string values a class expression can take, following references
 * (same file, then named repository imports). `onString(text)` is called with
 * every literal piece.
 */
function walkClassExpression(root, onString) {
  const visited = new Set();
  const resolveObject = (expr) => {
    const node = unwrap(expr);
    if (ts.isObjectLiteralExpression(node)) return node;
    if (ts.isIdentifier(node)) {
      for (const decl of lookup(node.getSourceFile(), node.text)) {
        const init = unwrap(decl);
        if (ts.isObjectLiteralExpression(init)) return init;
      }
    }
    return null;
  };
  const visit = (raw, mode = "classes") => {
    const node = unwrap(raw);
    if (!node || visited.has(node)) return;
    visited.add(node);
    if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) {
      onString(node.text);
    } else if (ts.isTemplateExpression(node)) {
      onString(node.head.text);
      for (const span of node.templateSpans) {
        visit(span.expression);
        onString(span.literal.text);
      }
    } else if (ts.isIdentifier(node)) {
      for (const decl of lookup(node.getSourceFile(), node.text)) visit(decl);
    } else if (ts.isFunctionDeclaration(node) || ts.isArrowFunction(node) || ts.isFunctionExpression(node)) {
      for (const expr of returnedExpressions(node)) visit(expr);
    } else if (ts.isPropertyAccessExpression(node)) {
      const object = resolveObject(node.expression);
      const property = object?.properties.find(
        (p) => (ts.isPropertyAssignment(p) || ts.isShorthandPropertyAssignment(p)) && propertyName(p.name) === node.name.text,
      );
      if (property) visit(ts.isPropertyAssignment(property) ? property.initializer : property.name);
      else visit(node.expression);
    } else if (ts.isElementAccessExpression(node)) {
      visit(node.expression);
    } else if (ts.isObjectLiteralExpression(node)) {
      for (const property of node.properties) {
        if (ts.isPropertyAssignment(property)) {
          const name = propertyName(property.name);
          if (mode === "compound") {
            if (name === "class" || name === "className") visit(property.initializer);
          } else if (name === "defaultVariants") {
            // variant *names* (`variant: "solid"`), not classes
          } else if (name === "compoundVariants") {
            const list = unwrap(property.initializer);
            if (ts.isArrayLiteralExpression(list)) {
              for (const element of list.elements) {
                const object = unwrap(element);
                if (ts.isObjectLiteralExpression(object)) {
                  visited.add(object);
                  for (const p of object.properties) {
                    if (ts.isPropertyAssignment(p) && ["class", "className"].includes(propertyName(p.name))) {
                      visit(p.initializer);
                    }
                  }
                }
              }
            }
          } else {
            visit(property.initializer);
          }
        } else if (ts.isShorthandPropertyAssignment(property)) {
          visit(property.name);
        } else if (ts.isSpreadAssignment(property)) {
          visit(property.expression);
        }
      }
    } else if (ts.isArrayLiteralExpression(node)) {
      for (const element of node.elements) visit(ts.isSpreadElement(element) ? element.expression : element);
    } else if (ts.isConditionalExpression(node)) {
      visit(node.whenTrue);
      visit(node.whenFalse);
    } else if (ts.isBinaryExpression(node)) {
      const op = node.operatorToken.kind;
      if (op === ts.SyntaxKind.AmpersandAmpersandToken) visit(node.right);
      else if (
        op === ts.SyntaxKind.BarBarToken ||
        op === ts.SyntaxKind.QuestionQuestionToken ||
        op === ts.SyntaxKind.PlusToken
      ) {
        visit(node.left);
        visit(node.right);
      }
      // comparisons (`variant === "destructive"`) carry variant names, not classes
    } else if (ts.isCallExpression(node)) {
      const callee = unwrap(node.expression);
      const name = ts.isIdentifier(callee) ? callee.text : null;
      if (name && (CLASS_HELPERS.has(name) || lookup(node.getSourceFile(), name).length > 0)) {
        if (!CLASS_HELPERS.has(name)) visit(callee);
        for (const arg of node.arguments) visit(arg);
      }
      // other calls (t("…"), imported helpers) are not followed
    } else if (ts.isJsxExpression(node)) {
      visit(node.expression);
    }
  };
  visit(root);
}

/**
 * Class tokens a source file uses: every `className` / `class` attribute and
 * every cn()/cva()/clsx() call, with references followed.
 */
export function classTokens(source, fileName = "file.tsx") {
  const sf = sourceFile(source, fileName);
  const tokens = new Set();
  const onString = (text) => {
    for (const token of text.split(/\s+/)) if (token) tokens.add(token);
  };
  const visit = (node) => {
    if (ts.isJsxAttribute(node) && ["className", "class"].includes(node.name.getText(sf)) && node.initializer) {
      walkClassExpression(node.initializer, onString);
    } else if (ts.isCallExpression(node)) {
      const callee = unwrap(node.expression);
      if (ts.isIdentifier(callee) && CLASS_HELPERS.has(callee.text)) {
        walkClassExpression(node, onString);
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(sf);
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

/**
 * Colour literals in a CSS value. `var(--name, fallback)` keeps its fallback:
 * only custom-property *names* are set aside, so `var(--x, #f00)` is caught.
 */
export function valueColourLiterals(rawValue) {
  const found = [];
  const value = rawValue.replace(/--[\w-]+/g, " ");
  const hex = value.match(new RegExp(HEX.source, "g"));
  if (hex) found.push(...hex);
  if (COLOUR_FUNCTIONS.test(value)) found.push(value.match(COLOUR_FUNCTIONS)[0]);
  for (const word of value.toLowerCase().match(/[a-z]+/g) ?? []) {
    if (NAMED_COLOURS.has(word)) found.push(word);
  }
  return found;
}

/** Colour literals in a declaration block (values only, not property names). */
export function colourLiterals(body) {
  const found = [];
  for (const declaration of splitTopLevel(body, ";")) {
    const colon = declaration.indexOf(":");
    if (colon < 0) continue;
    found.push(...valueColourLiterals(declaration.slice(colon + 1)));
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
const ARBITRARY_VALUE = new RegExp(
  `(?:^|[\\s"'\`:!])(${COLOUR_UTILITY_PREFIX}-\\[([^\\]\\s"'\`]+)\\](?:/\\d+)?)`,
  "g",
);
const ARBITRARY_PROPERTY = /(?:^|[\s"'`:!])(\[([a-z-]+):([^\]\s"'`]+)\])/g;
const COLOUR_PROPERTY = /color|background|fill|stroke|border|outline|shadow|caret|accent|decoration/i;
// Arbitrary values are written with `_` for spaces.
const arbitraryColours = (value) => valueColourLiterals(value.replace(/_/g, " "));

/** Raw colour utilities in a source file. */
export function rawColourUtilities(source) {
  const found = [];
  for (const m of source.matchAll(PALETTE_UTILITY)) found.push(m[1]);
  for (const m of source.matchAll(ARBITRARY_VALUE)) {
    if (arbitraryColours(m[2]).length > 0) found.push(m[1]);
  }
  for (const m of source.matchAll(ARBITRARY_PROPERTY)) {
    if (COLOUR_PROPERTY.test(m[2]) && arbitraryColours(m[3]).length > 0) found.push(m[1]);
  }
  return found;
}

/**
 * Colour literals in inline `style={…}` objects (referenced objects followed):
 * hex and colour functions in any property, named colours in colour-valued
 * properties (`display: "none"` is not a colour).
 */
export function inlineStyleColours(source, fileName = "file.tsx") {
  const sf = sourceFile(source, fileName);
  const found = [];
  const inspect = (expression, visited = new Set()) => {
    const node = unwrap(expression);
    if (!node || visited.has(node)) return;
    visited.add(node);
    if (ts.isJsxExpression(node)) return inspect(node.expression, visited);
    if (ts.isIdentifier(node)) {
      for (const decl of lookup(sf, node.text)) inspect(decl, visited);
      return;
    }
    if (ts.isConditionalExpression(node)) {
      inspect(node.whenTrue, visited);
      inspect(node.whenFalse, visited);
      return;
    }
    if (!ts.isObjectLiteralExpression(node)) return;
    for (const property of node.properties) {
      if (ts.isSpreadAssignment(property)) {
        inspect(property.expression, visited);
        continue;
      }
      if (!ts.isPropertyAssignment(property)) continue;
      const name = propertyName(property.name) ?? "";
      const pieces = [];
      walkClassExpression(property.initializer, (text) => pieces.push(text));
      for (const piece of pieces) {
        for (const literal of valueColourLiterals(piece)) {
          const named = /^[a-z]+$/i.test(literal);
          if (!named || COLOUR_PROPERTY.test(name)) found.push(`${name}: ${literal}`);
        }
      }
    }
  };
  const visit = (node) => {
    if (ts.isJsxAttribute(node) && node.name.getText(sf) === "style" && node.initializer) {
      inspect(node.initializer);
    }
    ts.forEachChild(node, visit);
  };
  visit(sf);
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
    const hits = [...classTokens(read(file), file)].filter((token) => legacy.has(token));
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
    for (const { selector, body } of cssRules(read(PROSE_CSS))) {
      for (const part of splitTopLevel(selector, ",").map((s) => s.trim())) {
        const escape = proseEscape(part, PROSE_ROOT);
        if (escape) {
          fail("prose", `${PROSE_CSS}: selector "${part}" is outside the prose root .${PROSE_ROOT} (${escape}).`);
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
    const inline = file.endsWith(".tsx") ? inlineStyleColours(read(file), file) : [];
    if (inline.length > 0) {
      fail(
        "raw-colour",
        `${file}: colour literals in inline style ${[...new Set(inline)].join(", ")} — colour through a ` +
          "semantic token utility, never an inline style.",
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

function selfTest() {
  const suite = join(ROOT, "scripts", "check-styles.test.mjs");
  if (!existsSync(suite)) return true;
  const run = spawnSync(process.execPath, ["--test", "--test-reporter=dot", suite], {
    cwd: ROOT,
    encoding: "utf8",
  });
  if (run.status === 0) return true;
  console.error("styles:check self-test failed — the guard no longer rejects what it claims to:");
  console.error(run.stdout, run.stderr);
  return false;
}

function main() {
  if (process.argv.includes("--count")) {
    console.log(countCssLines(read(APP_CSS)));
    return;
  }
  if (!process.argv.includes("--no-self-test") && !selfTest()) process.exit(1);
  const failures = check();
  if (failures.length > 0) {
    console.error(`styles:check failed (${failures.length}):`);
    for (const failure of failures) console.error(`  ${failure}`);
    process.exit(1);
  }
  const baseline = loadBaseline();
  console.log(
    `styles:check ok — App.css ${baseline.appCssCeiling ?? "absent"} counted lines (ceiling), ` +
      `${baseline.migrated.length} migrated file(s), prose layer scoped, no raw colours` +
      `${process.argv.includes("--no-self-test") ? "" : "; self-test passed"}.`,
  );
}

// Run only as a CLI, not when imported (realpaths: macOS temp dirs are symlinked).
const invokedDirectly =
  process.argv[1] && realpathSync(process.argv[1]) === realpathSync(fileURLToPath(import.meta.url));
if (invokedDirectly) main();

export { check };
