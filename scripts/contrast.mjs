// Contrast of every token pair DESIGN.md § Tokens records.
// Ticket 30 wrote the first version (lost with its worktree); ticket 35
// (2026-11-04) rebuilt it from the surviving fragments and extended it for the
// four-level dark ground (background = row #000000 · card #040404 ·
// popover = accent = muted #0e0e0e · raised #1e1e1e).
//
// WCAG 2.x relative luminance. `color-mix(in oklab, …)` is mixed in OKLab and
// `/NN` fills / element opacity are alpha-composited over the ground in sRGB —
// in floating point: a mixed or composited intermediate is never quantised to
// 8 bits before the luminance step (ticket-30 audit F1).
//
// Exits 1 on any gated pair below threshold (text 4.5, non-text 3.0).
//
//   node scripts/contrast.mjs [--old]   (--old: pre-35 dark)

const DARK_OLD = {
  background: "#0e0f12", card: "#15171b", popover: "#15171b", raised: "#1d2025", row: "#09090b",
  accent: "#1d2025", muted: "#1d2025",
  foreground: "#e6e7ea", "muted-foreground": "#a6abb4", faint: "#8c929c",
  primary: "#6a61e8", "primary-foreground": "#ffffff", ring: "#9690ff",
  border: "#262a31", "border-strong": "#343942", input: "#6b717b",
  ok: "#3ddc84", stale: "#fbac16", failed: "#ff6b6b", ink: "#0e0f12",
  "syntax-keyword": "#f29fd8", "syntax-string": "#7fc8dc", "syntax-number": "#e8b48a", "syntax-function": "#a9b6ff",
};

// Ticket 35 — sampled from a T3 Code window capture (2026-11-04), prototype 30-dark-ground (f).
const DARK = {
  ...DARK_OLD,
  background: "#000000", row: "#000000", card: "#040404",
  popover: "#0e0e0e", accent: "#0e0e0e", muted: "#0e0e0e", raised: "#1e1e1e",
  foreground: "#f5f5f5", "muted-foreground": "#a3a3a3", faint: "#898989",
  border: "#171717", "border-strong": "#262626", input: "#6b6b6b",
  ink: "#000000", // *-foreground on the state fills — decided below (INK)
};

const LIGHT = {
  background: "#ffffff", card: "#f0f1f4", popover: "#ffffff", raised: "#e4e7ec", row: "#ffffff",
  accent: "#e4e7ec", muted: "#e4e7ec",
  foreground: "#16181d", "muted-foreground": "#494e57", faint: "#5b616a",
  primary: "#5a4fdc", "primary-foreground": "#ffffff", ring: "#5a4fdc",
  border: "#d5d9e0", "border-strong": "#bcc2cc", input: "#737983",
  ok: "#127440", stale: "#985700", failed: "#c22e2e", ink: "#ffffff",
  "syntax-keyword": "#a1306e", "syntax-string": "#16677d", "syntax-number": "#87460f", "syntax-function": "#3a4cb0",
};

const IDENTITY = {
  // hue: [dark tint, dark fg, light tint, light fg]
  moss: ["#283215", "#bddb8a", "#e1edcf", "#475f06"],
  teal: ["#09362e", "#74e5ce", "#cbf2e8", "#006855"],
  cyan: ["#0a3437", "#76e1ea", "#ccf0f3", "#00656d"],
  sky: ["#0e3242", "#7cdbff", "#cdeeff", "#005f86"],
  orchid: ["#39253f", "#efb5ff", "#f5dffc", "#703c80"],
  rose: ["#422232", "#ffadd9", "#ffdcec", "#84345f"],
  slate: ["#292e36", "#bfcfe5", "#e2e8f1", "#485669"],
};

// ── colour math ────────────────────────────────────────────────────────────
// A colour is a hex string (a token) or a float RGB triple (a mixed or
// composited intermediate) — intermediates are never quantised (audit F1).
const hex = (h) => (Array.isArray(h) ? h : [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16) / 255));
const lin = (c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
const gam = (c) => (c <= 0.0031308 ? 12.92 * c : 1.055 * Math.sign(c) * Math.abs(c) ** (1 / 2.4) - 0.055);
const lum = (h) => {
  const [r, g, b] = hex(h).map((c) => lin(Math.min(1, Math.max(0, c))));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
};
const ratio = (a, b) => {
  const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
};
function toOklab(h) {
  const [r, g, b] = hex(h).map(lin);
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  return [
    0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  ];
}
function fromOklab([L, a, b]) {
  const l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;
  return [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ].map(gam);
}
/** color-mix(in oklab, a p, b) */
const mix = (a, p, b) => {
  const [x, y] = [toOklab(a), toOklab(b)];
  return fromOklab(x.map((v, i) => v * p + y[i] * (1 - p)));
};
/** a at alpha p over ground g (sRGB compositing). */
const over = (a, p, g) => hex(a).map((c, i) => c * p + hex(g)[i] * (1 - p));
const f = (n) => n.toFixed(2);

// ── tables ─────────────────────────────────────────────────────────────────
const GROUNDS = ["background", "card", "popover", "raised", "row"];
const failures = [];
const check = (label, value, min) => {
  if (min !== null && value < min) failures.push(`${label}: ${f(value)} < ${min}`);
  return value;
};
const onGrounds = (T, token, min, theme) =>
  GROUNDS.map((g) => f(check(`${theme} ${token} on ${g}`, ratio(T[token], T[g]), min))).join(" / ");
function report(T, theme) {
  const out = [];
  const p = (s) => out.push(s);
  p(`\n══ ${theme} ══`);
  p("Ground steps:");
  for (const [a, b] of [["background", "card"], ["card", "popover"], ["popover", "raised"], ["card", "raised"],
    ["row", "card"], ["row", "popover"], ["row", "raised"], ["row", "background"], ["popover", "accent"]]) {
    p(`  ${a} → ${b}: ${f(ratio(T[a], T[b]))}`);
  }
  p("Text, on background / card / popover / raised / row:");
  for (const t of ["foreground", "muted-foreground", "faint", "ring", "ok", "stale", "failed"]) {
    p(`  ${t}: ${onGrounds(T, t, 4.5, theme)}`);
  }
  p(`  faint → muted-foreground step: ${f(ratio(T.faint, T["muted-foreground"]))}`);
  p("Non-text, on grounds:");
  p(`  primary (fill): ${onGrounds(T, "primary", 3, theme)}`);
  for (const t of ["border", "border-strong"]) p(`  ${t} (decorative): ${onGrounds(T, t, null, theme)}`);
  p(`  input: ${onGrounds(T, "input", 3, theme)}`);
  p(`  ring: ${onGrounds(T, "ring", 3, theme)}`);

  p("Text on the fills:");
  const fill = (fg, bg) => {
    const solid = check(`${theme} ${fg} on ${bg}`, ratio(T[fg] ?? fg, T[bg]), 4.5);
    let lo = Infinity, at = "";
    for (const g of GROUNDS) {
      const r = ratio(T[fg] ?? fg, over(T[bg], 0.9, T[g]));
      if (r < lo) [lo, at] = [r, g];
    }
    check(`${theme} ${fg} on ${bg}/90`, lo, 4.5);
    return `${f(solid)} · on ${bg}/90 worst ${f(lo)} (${at})`;
  };
  p(`  primary-foreground on primary: ${fill("primary-foreground", "primary")}`);
  for (const s of ["ok", "stale", "failed"]) p(`  ink ${T.ink} on ${s}: ${fill(T.ink, s)}`);
  if (theme.startsWith("dark")) {
    p("  INK alternatives (dark):");
    for (const ink of ["#000000", "#0e0f12"]) {
      const row = ["ok", "stale", "failed"].map((s) => {
        let lo = Infinity;
        for (const g of GROUNDS) lo = Math.min(lo, ratio(ink, over(T[s], 0.9, T[g])));
        return `${s} ${f(ratio(ink, T[s]))}/${f(lo)}`;
      });
      p(`    ${ink}: ${row.join(" · ")}  (solid/worst-90)`);
    }
  }

  p("Soft grounds (state text on state mixed into popover; legacy alias into background):");
  for (const [s, text] of [["primary", "ring"], ["ok", "ok"], ["stale", "stale"], ["failed", "failed"]]) {
    const b10 = check(`${theme} ${s} badge 10%`, ratio(T[text], mix(T[s], 0.1, T.popover)), 4.5);
    const t8 = check(`${theme} ${s} toast 8%`, ratio(T[text], mix(T[s], 0.08, T.popover)), 4.5);
    const leg = ratio(T[s === "primary" ? "ring" : s], mix(T[s === "primary" ? "ring" : s], 0.08, T.background));
    p(`  ${s}: badge 10 % ${f(b10)} · toast 8 % ${f(t8)} · legacy soft-bg 8 % into background ${f(leg)}`);
  }
  p(`  toast edge 35 % (ok/stale/failed) vs popover: ${["ok", "stale", "failed"].map((s) => f(ratio(mix(T[s], 0.35, T.popover), T.popover))).join(" / ")} (decorative)`);

  p("Identity (fg on tint · tint vs card · fg vs card):");
  const idx = theme.startsWith("dark") ? 0 : 2;
  let tintLo = Infinity, tintHi = 0, fgLo = Infinity, fgHi = 0;
  for (const [hue, v] of Object.entries(IDENTITY)) {
    const [tint, fg] = [v[idx], v[idx + 1]];
    const a = check(`${theme} identity ${hue}`, ratio(fg, tint), 4.5);
    const b = ratio(tint, T.card), c = ratio(fg, T.card);
    [tintLo, tintHi, fgLo, fgHi] = [Math.min(tintLo, b), Math.max(tintHi, b), Math.min(fgLo, c), Math.max(fgHi, c)];
    p(`  ${hue}: ${f(a)} · ${f(b)} · ${f(c)}`);
  }
  p(`  range: tint vs card ${f(tintLo)}–${f(tintHi)} · fg vs card ${f(fgLo)}–${f(fgHi)}`);

  p("Prose (syntax, worst ground):");
  for (const s of ["syntax-keyword", "syntax-string", "syntax-number", "syntax-function"]) {
    let lo = Infinity, at = "";
    for (const g of GROUNDS) { const r = ratio(T[s], T[g]); if (r < lo) [lo, at] = [r, g]; }
    check(`${theme} ${s}`, lo, 4.5);
    p(`  ${s}: ${f(lo)} on ${at}`);
  }

  p("Other non-text pairs:");
  p(`  switch thumb (muted-foreground) on raised track: ${f(check(`${theme} thumb`, ratio(T["muted-foreground"], T.raised), 3))}`);
  p(`  checked thumb (background) on muted-foreground track: ${f(check(`${theme} checked thumb`, ratio(T.background, T["muted-foreground"]), 3))}`);
  p(`  check glyph (primary-foreground) on primary: ${f(ratio(T["primary-foreground"], T.primary))}`);
  p(`  selected iris edge (ring) on raised: ${f(ratio(T.ring, T.raised))}`);
  p(`  badge soft neutral (muted-foreground on muted): ${f(check(`${theme} badge neutral`, ratio(T["muted-foreground"], T.muted), 4.5))}`);
  p(`  badge solid neutral (foreground on raised): ${f(ratio(T.foreground, T.raised))}`);
  p(`  tooltip (background on foreground): ${f(ratio(T.background, T.foreground))}`);
  p(`  solid-neutral Button hover (border-strong) vs raised: ${f(ratio(T["border-strong"], T.raised))} (decorative step)`);
  p(`  legacy --bg-element-hover (muted + 6 % foreground): faint on it ${f(ratio(T.faint, mix(T.foreground, 0.06, T.muted)))}`);
  p(`  legacy --accent-primary-fg (background) on ring: ${f(ratio(T.background, T.ring))}`);

  p("Menus / palette:");
  p(`  highlight (raised) vs popover: ${f(ratio(T.raised, T.popover))} · accent vs popover: ${f(ratio(T.accent, T.popover))}`);
  p(`  inset ring on highlight (raised): ${f(ratio(T.ring, T.raised))} · on popover: ${f(ratio(T.ring, T.popover))}`);
  p(`  group label (faint) on popover: ${f(ratio(T.faint, T.popover))} · hint (muted-foreground) on popover ${f(ratio(T["muted-foreground"], T.popover))} · on highlight ${f(ratio(T["muted-foreground"], T.raised))}`);
  p(`  item text (foreground) on highlight: ${f(ratio(T.foreground, T.raised))}`);

  p("Focus ring through element opacity (ring at α over the ground, vs the ground):");
  for (const a of [0.5, 0.75]) {
    const row = GROUNDS.map((g) => {
      const r = ratio(over(T.ring, a, T[g]), T[g]);
      if (a === 0.75) check(`${theme} ring@75% on ${g}`, r, 3);
      return f(r);
    });
    p(`  ${a * 100} %: ${row.join(" / ")}`);
  }
  return out.join("\n");
}

const old = process.argv.includes("--old");
console.log(report(old ? DARK_OLD : DARK, old ? "dark (pre-35)" : "dark"));
console.log(report(LIGHT, "light"));
if (failures.length) {
  console.log("\nFAILURES:\n  " + failures.join("\n  "));
  process.exit(1);
}
console.log("\nAll gated pairs pass.");
