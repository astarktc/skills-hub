PASS-with-findings

# Ticket 30 — adversarial audit

Reviewed the **uncommitted** implementation on `t3/colour-identity-tokens`, base `fb8ef9a`, including the untracked `src/components/ui/badge.test.tsx`. No source was changed, no commit made. The operator-approved palette, header-only pick A, light amber, row values and chip interface were treated as fixed decisions. Used the installed `web-design-guidelines` skill (fresh upstream guidelines), the token/primitive guidance, DESIGN.md and WCAG 2.x; this is a bounded technical audit, not a new design decision.

## F1 — P1 (fix before 31): contrast documentation quantizes intermediate colours, exceeding the requested ±0.01 tolerance

**Evidence:** `DESIGN.md:108–123`; `.scratch/round19/evidence/30-contrast.mjs:44–45,67–85`. Both `fromOklab()` and `over()` call `toHex()`, which rounds each mixed channel to an 8-bit integer **before** the WCAG luminance calculation. The script therefore measures rounded approximations, not the floating-point CSS mixtures. This explains its claim to reproduce old rounded measurements exactly. Independent `node --input-type=module` calculations (reproducible in Appendix A) checked **184 table cells**, with the following **14 differences greater than 0.01**:

| Pair | Theme | DESIGN.md | Independent, unrounded |
|---|---|---:|---:|
| primary-foreground / primary/90, worst ground | dark | 5.28 | 5.305952 |
| primary-foreground / primary/90, worst ground | light | 4.76 | 4.788994 |
| ok-foreground / ok/90, worst ground | dark | 8.77 | 8.759311 |
| ok-foreground / ok/90, worst ground | light | 4.75 | 4.761756 |
| stale-foreground / stale/90, worst ground | dark | 8.24 | 8.218862 |
| destructive-foreground / destructive/90, worst ground | dark | 5.70 | 5.721485 |
| accent text / 10% primary into popover | dark | 6.07 | 6.052325 |
| ok text / 10% ok into popover | dark | 8.53 | 8.569750 |
| stale text / 10% stale into popover | dark | 8.08 | 8.062909 |
| ok text / 10% ok into popover | light | 5.07 | 5.054721 |
| stale text / 10% stale into popover | light | 4.94 | 4.929205 |
| stale text / 8% stale into popover | dark | 8.32 | 8.351365 |
| accent text / 8% primary into popover | light | 5.22 | 5.232896 |
| ok text / 8% ok into popover | light | 5.22 | 5.204844 |

Browser corroboration from the real specimen: light soft-stale computes to **`oklab(0.952083 0.00545949 0.0106051)`**, dark to **`oklab(0.26391 0.00353765 0.00830398)`**; they are not hex-valued backgrounds. `/90` is composited over each ground, not compared as an opaque fill. The worst dark ground for a white primary label is `raised`; the worst dark ground for dark state labels is the new `row`. The worst light ground is white (`background`/`popover`/`row`).

**Impact:** documentation/measurement precision, **not a newly failing contrast pair**. All gated pairs still pass without rounding. No colour change is requested. It matters because measured contrast and the reusable proof are this ticket's main deliverables.

**Exact proposed fix:** make `lum`/`ratio` accept normalized floating-point RGB arrays, return RGB arrays from `fromOklab` and `over`, and reserve `toHex` for display only. Recompute both complete fill/soft tables and round only the final ratio to two decimals. Update any Result prose quoting those values. Keep the pinned hex tokens unchanged. Alternatively explicitly label rounded pixel measurements as such, but retain an unrounded calculation for the pass/fail gate; simply retaining the claim “exactly as the browser does” is not accurate.

## F2 — P2 (note): the new amber-on-raised prohibition is not true of existing consumers

**Evidence:** `DESIGN.md:99–101` says amber on `raised` is a non-text mark only. `src/components/projects/ProjectList.tsx:169–196` already places the **“Folder not found”** text and stale-status text (`text-stale`, lines 183 and 196) inside a `hover:bg-raised` / selected `bg-raised` button. `src/components/ui/specimen.tsx:77` also deliberately prints stale text on its raised specimen. The consumer grep below reproduces these locations. Independent WCAG result: `#985700` on `#e4e7ec` = **4.58052**, versus **4.95464** before this change: still passing, not a regression below AA.

**Exact proposed fix:** qualify the DESIGN.md rule as a constraint for **new ticket-31 row anatomy**, and name the existing ProjectList text exception with its measured 4.58 ratio (and the specimen's deliberate all-ground matrix). Do not alter the accepted amber or broaden this token-only ticket into a surface migration. If the restriction is intended to be universal instead, explicitly schedule replacing those textual uses with neutral text plus an amber mark; do not claim that universal rule is already implemented.

## Verification record

### 1. Value fidelity — PASS

Mechanically parsed the actual `:root` and light override declarations and compared them to the NOTES tables: **36 checks, zero mismatches** (28 identity values, six proposed light grounds, dark amber and the separately approved light amber). The sanctioned light amber is `#985700`, rather than NOTES' `#9e5d00`. Row is exactly `#09090b` / `#ffffff`. Light aliases `sidebar=card`, `muted=accent=sidebar-accent=raised`, `sidebar-border=border` agree. No variant-B edge tokens were minted.

### 2. Independent contrast — PASS for classifications, F1 for precision

Appendix A contains my own luminance, OKLab matrix conversion, mixture and alpha-compositing functions; it does not import or execute the child's measuring script. I wrote and ran these before reading that script, then traced the discrepancy to its intermediate rounding.

The **184 mechanically compared cells** cover the text/state/lines tables (including all fifth `row` columns), fills and `/90`, 10%/8% OKLab soft grounds, 14 identity pairs, all ten ground-step values, and prose syntax worst grounds. All direct hex-pair cells agree within 0.01. Identity tint/card ranges also independently agree: dark **1.287325–1.349033**, light **1.068261–1.113350**; identity foreground/card ranges dark **10.451752–11.827666**, light **5.976118–7.049541**. Historical round-18/surface measurements are explicitly date-qualified and superseded by the token tables; they were not misrepresented as current measurements in this audit.

Additional floor checks:

| Pair | Independent result | Classification |
|---|---:|---|
| light failed / raised | 4.548834 | Text pass, nearest current ordinary-text floor |
| light faint / raised | 5.037541 | Text pass |
| light input / raised | 3.535394 | Non-text pass |
| light input / card | 3.880392 | Non-text pass |
| dark primary-foreground / primary | 4.679195 | Text pass |
| light identity teal foreground / tint | 5.594249 | Small-text pass; 10.5 px bold still uses 4.5, not 3 |
| dark border / row, border / card | 1.381259 / 1.245823 | Fails 3 if used as sole identifying cue; explicitly decorative in DESIGN.md |
| light faint / legacy bg-element-hover | 4.406895 | Below 4.5, but alias has no reader |

**Extra adversarial pair:** mix 10% light stale into **raised**, then draw stale text: **4.031374**, a text failure. The actual Button ghost/outline hover does **not** do that: `styles.ts:111–115` uses an opaque mix into **popover**, giving **4.929205**, regardless of its outside raised ground. `styles.ts:65–69` and DESIGN.md's amber restriction explicitly cover the dangerous substitution. Faint on the actual soft-stale badge also passes (**5.421021** light, **4.909513** dark). Identity chips retain their opaque tint on raised; even the hypothetical identity text directly on raised has a light minimum of **5.445**. No new failure was found by these extra constructions.

### 3. Moved-token consumers and legacy aliases — no new below-threshold pair found

Commands run from this worktree:

```sh
rg -n 'bg-raised|bg-muted|bg-accent|border-input|text-stale|bg-stale|softGround\.stale|text-2xs|bg-element-hover' src
rg -n 'var\(--(bg-|text-|accent-|warning-|status-)' src --glob '!App.css'
rg -l 'settings-|discovered-list|btn-danger|btn-primary|invocation-edit-warning|bg-element-hover|warning-soft|status-warning' src --glob '*.tsx'
```

Read the entire remaining App.css alias/rule file and `scripts/styles-baseline.json` (812 counted lines / 38 migrated files), then the live legacy SettingsPage, ImportModal, InvocationModeModal, NewToolsModal and UpdateDialog. `bg-element-hover` appears only in its alias definition and a theme comment; there is no use. Legacy `.btn-danger:hover`'s stronger tint has no live JSX consumer, and the old `.settings-toggle` is not the Settings switch (Settings uses the primitive).

| Consumer group | Files / pairing checked | Result on new light grounds |
|---|---|---|
| Raised/highlight text | SkillRow, ExplorePage, SkillFiles, CandidatePicker, AssignProjectModal, AssignmentMatrix, ProjectList; ui/button, badge, tabs, segmented, dropdown-menu, command-palette | Primary 14.33, secondary 6.75, faint 5.04 on raised; failed 4.55, stale 4.58. No new text failure. ProjectList's policy mismatch is F2. |
| Amber status | SkillsList, LibraryHeader, StripMarkIcon, NotificationsModal, AddSkillDialog, inspector HealthSection/EditsSection/SourceSection, ProjectHeader | Ordinary grounds pass; marks exceed 3; soft Badge preserves its own opaque ground. |
| Input boundaries | ui/field, checkbox, switch; legacy `.settings-input` / `.settings-select` | input/card improves **3.51879 → 3.88039**; input/raised improves **3.33154 → 3.53539**. |
| Neutral solid hover | ui/button `foreground` on `border-strong` | **11.59194 → 9.91697**, still text pass. |
| Legacy copy | SettingsPage helpers / labels; UpdateDialog notes and links; InvocationModeModal helpers / warning; NewToolsModal buttons | text aliases map to foreground / muted-foreground / faint; raised and ordinary-ground bounds above cover the changed grounds. Iris link on raised 4.72. |
| Legacy import | ImportModal `.discovered-list .group-card` on card; `.path`/`.meta` faint; `.found-pill`/`.badge` amber on opaque warning-soft-bg | Faint/card 5.53; amber on 8% popover/background-white mixture about 5.07; unchanged shape and no below-threshold pair. |

`border` / `border-strong` remain below 3 on grounds, by design: decorative hairlines and text-identified edges, not input boundaries. The dark row separator challenge above is already prohibited as a sole cue. The worst direct light text loss was `failed/raised` **4.85123 → 4.54883**; it still clears 4.5 without rounding. The audit does not certify every arbitrary future token combination.

### 4. Badge contract and CSS compilation — PASS

- `npx tsc --noEmit -p tsconfig.app.json` exited **0**. The three `@ts-expect-error` assertions in `badge.test.tsx:58–63` are live compiler assertions, not merely runtime tests.
- Independently removed those directives **in memory only**, using the installed TypeScript JS API (TS6, supplemental to the TS7 command above): three `TS2322` errors, respectively outline incompatible with soft, missing required hue, and hue rejected on stale. Neutral `size="chip"` produced no error. The union at `badge.tsx:68–83` also disallows identity solid and identity sm/md.
- `hue` is destructured before `...props` (`badge.tsx:86`); tests and all seven rendered chips confirm `hasAttribute('hue') === false`.
- Identity colour classes are added from the literal `identityGround` map, not a dynamic unscannable cva string. cva owns the size/base; `cn` combines them. Correct division of responsibility.
- Ran an **in-memory mutation check** against the actual test regex: remove one `--color-identity-…` registration at a time and compile Tailwind. **14/14 mutations rejected**; positive controls all matched. The regex requires the exact class selector, opening rule and corresponding declaration; raw token declarations cannot satisfy it.
- `vite.config.ts:30` includes `/theme\.css/` only. The sole direct test CSS import is `@/styles/theme.css?raw` in badge.test.tsx. Current `App.css`, `index.css` and `prose.css` do not match and remain excluded from CSS processing. The expression is a filename substring rather than a canonical-path allowlist, but there is no second matching stylesheet in the repo and no current test broadening defect.

### 5. Fold — PASS

Independent BigInt FNV multiplication/modulo (not `Math.imul`) agreed with the implementation:

| Owner | Independent / implementation hash | Hue | Initials |
|---|---:|---|---|
| anthropics | 2569612480 | teal | AN |
| vercel-labs | 703213605 | sky | VL |
| obra | 3167613737 | teal | OB |
| mattpocock | 3169730402 | slate | MA |
| `-` | 671913016 | rose | `-` |
| `--a` | 283464746 | rose | A |
| `a` | 3826002220 | rose | A |
| `ünïcode-x` | 1669968912 | sky | ÜX |
| `😀` | 3409036472 | cyan | 😀 |

FNV iterates `charCodeAt(i)`, matching NOTES' **UTF-16 code-unit** rule, including the surrogate-pair case. The returned unsigned integer modulo the fixed nonempty seven-element array is always 0–6. Swept **all 65,536 single UTF-16 code units**: zero undefined hues, zero empty initials. For arbitrary nonempty strings, either filtered parts supply a nonempty first part or the original nonempty owner supplies the fallback. `toUpperCase()` is locale-independent and sufficient for ASCII repository owners; no locale-sensitive Turkish casing is introduced. Empty owner returns empty initials and must be routed to the documented neutral ownerless chip by ticket 31, not treated as a repo identity.

### 6. ADR / guard / tests — PASS

Fresh command output:

```text
npm run styles:check
styles:check ok — App.css 812 counted lines (ceiling), 38 migrated file(s), prose layer scoped, no raw colours; self-test passed.

npx vitest run src/components/ui/badge.test.tsx src/components/ui/focus-ring.test.tsx src/lib/skillPresentation.test.ts
Test Files  3 passed (3)
Tests       140 passed (140)

npx tsc --noEmit -p tsconfig.app.json
(exit 0, no diagnostics)

git diff --check
(exit 0, no output)
```

Impeccable's source detector on badge/specimen/styles/theme returned **`[]`**. This is supplemental, not a substitute for contrast calculation. No raw colour in JSX. The specimen draws chips with Badge props, never token classes at call sites. `text-2xs` is registered in `@theme static` at `theme.css:265–276`; the only production class use is badge's `chip` size (`badge.tsx:38`). DESIGN.md documents identity tone, required hue, all seven hue values, soft-only/chip-only identity, neutral chip and 10.5 px bold. ADR-0006's amendment accurately records the new row, identity pairs and type-scale exception. No new stylesheet, palette tier, or surface wiring.

The full Rust/build/release gate was **not rerun** by this read-only auditor; this record distinguishes fresh focused proof from the implementation child's reported full gate.

### 7. Fixture and specimen — PASS

Started my own fixture server with `npx vite --mode fixture --port 5178 --strictPort`; used ego-browser, never Tauri, port 5175 or port 5176. Inspected My Skills after the 60 skills and discovered-skills banner had settled. Captures were taken and read at an actual `innerWidth=1280`, `innerHeight=800`, `scrollWidth=1280`:

- Light: `/var/folders/6p/j8hvzsjx4wjf__r2jbgbff980000gn/T/ego-browser-shot-151722.png`
- Dark: `/var/folders/6p/j8hvzsjx4wjf__r2jbgbff980000gn/T/ego-browser-shot-787104.png`
- Specimen light: `/var/folders/6p/j8hvzsjx4wjf__r2jbgbff980000gn/T/ego-browser-shot-561508.png`
- Specimen dark: `/var/folders/6p/j8hvzsjx4wjf__r2jbgbff980000gn/T/ego-browser-shot-148197.png`

The final My Skills theme comparison applied the data-theme override in the browser only; consequently the sidebar's stored Theme label remains Light in the dark token capture. This is not an app regression. Earlier captures also exercised the actual Theme menu in both modes. No persisted source or real-library data was changed.

The first three row rectangles were identical in both settled captures: **[200,175,1065,36], [200,211,1065,36], [200,247,1065,36]**. Visual inspection found the existing layout, truncation and columns intact, with no horizontal overflow. Production layout files have no diff; no base-checkout comparison was needed or claimed. Header chips and `bg-row` are appropriately absent from My Skills until 31.

All seven specimen hues in **both themes** computed to the exact NOTES RGB equivalents. Every chip was **22px × 22px, 10.5px, font-weight 700**, with no `hue` DOM attribute. Teal examples: dark **rgb(9,54,46) / rgb(116,229,206)**; light **rgb(203,242,232) / rgb(0,104,85)**. The row specimen computed to white in light; the dark token is near-black as specified in source. The five grounds are real generated utilities. Source unchanged, browser task closed and my 5178 server stopped.

### 8. Focus ring floor — PASS, including the narrow disabled margin

Independent floating-point sRGB compositing at 75% opacity, followed by WCAG luminance:

| Light ground | Full ring | Focused-disabled ring at 75% |
|---|---:|---:|
| background | 5.85515 | 3.54079 |
| card | 5.18418 | 3.27876 |
| popover | 5.85515 | 3.54079 |
| raised | **4.72326** | **3.08829** |
| row | 5.85515 | 3.54079 |

Thus `#5a4fdc` on `#e4e7ec` really is the light floor. The documented 4.72 is correct. The stated 3.08 disabled floor is within 0.01 of the precise result (ordinary two-decimal rounding is **3.09**); it passes 3 **before rounding**, with only 0.088 margin. Dark focused-disabled also passes every ground: minimum **3.99196 on raised**, not the background example 4.42.

Browser proof: after a real keyboard Tab, focused the raised specimen's pending button. `getComputedStyle` returned **opacity 0.75**, **solid 2px rgb(90,79,220)** outline, raised **rgb(228,231,236)**, and `matches(':focus-visible') === true`. A preceding pointer-modality probe correctly remained at 0.5 without a focus-visible ring; it was not mistaken for the keyboard case.

## Appendix A — independent reproducible contrast calculation

Run this block with `node --input-type=module` on stdin from the worktree. It reads the current token and document files, does not modify them, and does not use the child's script. The output at audit time was `checked 184 cells` and the 14 rows in F1. Matrix constants implement the standard linear-sRGB ↔ OKLab conversion; all intermediate channels remain floating point.

```js
import fs from 'node:fs';
const css = fs.readFileSync('src/styles/theme.css', 'utf8');
const doc = fs.readFileSync('DESIGN.md', 'utf8');
const T = [...css.matchAll(/:root(?:\[data-theme='light'\])?\s*\{([\s\S]*?)\n\}/g)]
  .map(m => Object.fromEntries([...m[1].matchAll(/--([\w-]+): (#[\da-f]{6});/g)]
    .map(m => [m[1], m[2]])));
const rgb = h => h.slice(1).match(/../g).map(x => parseInt(x, 16) / 255);
const lin = x => x <= .04045 ? x / 12.92 : ((x + .055) / 1.055) ** 2.4;
const enc = x => x <= .0031308 ? 12.92 * x : 1.055 * x ** (1 / 2.4) - .055;
const Y = c => c.map(lin).reduce((s, v, i) => s + v * [.2126, .7152, .0722][i], 0);
const R = (a, b) => (Math.max(Y(a), Y(b)) + .05) / (Math.min(Y(a), Y(b)) + .05);
const M = (m, v) => m.map(row => row.reduce((s, x, i) => s + x * v[i], 0));
const lab = c => M([
  [.2104542553, .793617785, -.0040720468],
  [1.9779984951, -2.428592205, .4505937099],
  [.0259040371, .7827717662, -.808675766]
], M([
  [.4122214708, .5363325363, .0514459929],
  [.2119034982, .6806995451, .1073969566],
  [.0883024619, .2817188376, .6299787005]
], c.map(lin)).map(Math.cbrt));
const unlab = v => M([
  [4.0767416621, -3.3077115913, .2309699292],
  [-1.2684380046, 2.6097574011, -.3413193965],
  [-.0041960863, -.7034186147, 1.707614701]
], M([
  [1, .3963377774, .2158037573],
  [1, -.1055613458, -.0638541728],
  [1, -.0894841775, -1.291485548]
], v).map(x => x ** 3)).map(enc);
const alpha = (a, b, p) => a.map((v, i) => p * v + (1 - p) * b[i]);
const mix = (a, b, p) => unlab(alpha(lab(a), lab(b), p));
const G = ['background', 'card', 'popover', 'raised', 'row'];
const lines = doc.split('\n');
let count = 0;
const differences = [];
const check = (name, actual, expected) => {
  count++;
  if (Math.abs(actual - expected) > .010000001) differences.push({ name, actual, expected });
};
for (const line of lines) {
  const c = line.split('|').map(s => s.trim());
  const k = c[1]?.match(/^`([\w-]+)`/)?.[1];
  if (c.length === 8 && T[0][k] && /\d\.\d\d \/ /.test(c[5])) {
    for (let t = 0; t < 2; t++) {
      const n = c[5 + t].match(/\d+\.\d+/g).map(Number);
      G.forEach((g, i) => check(`${t} ${k}/${g}`, R(rgb(T[t][k]), rgb(T[t][g])), n[i]));
    }
  }
}
for (const line of lines.filter(l => /^\| `(?:moss|teal|cyan|sky|orchid|rose|slate)`/.test(l))) {
  const c = line.split('|').map(s => s.trim()), h = c[1].match(/`([^`]+)`/)[1];
  for (let t = 0; t < 2; t++) check(`${t} identity ${h}`,
    R(rgb(T[t][`identity-${h}-foreground`]), rgb(T[t][`identity-${h}`])), Number(c[t ? 7 : 4]));
}
const fills = doc.split('Text on the fills:')[1].split('Soft grounds')[0]
  .split('\n').filter(l => l.startsWith('| `') || l.startsWith('| …'));
for (let i = 0; i < fills.length; i++) {
  const c = fills[i].split('|').map(s => s.trim());
  const k = ['primary', 'ok', 'stale', 'destructive'][Math.floor(i / 2)];
  for (let t = 0; t < 2; t++) {
    const v = T[t], fg = rgb(v[k + '-foreground']), fill = rgb(v[k]);
    check(`${t} fill ${k} ${i % 2 ? '90' : '100'}`,
      i % 2 ? Math.min(...G.map(g => R(fg, alpha(fill, rgb(v[g]), .9)))) : R(fg, fill),
      Number(c[t + 2].match(/\d+\.\d+/)[0]));
  }
}
for (const [label, p] of [['Badge', .1], ['Toast', .08]]) {
  const c = lines.find(l => l.startsWith('| ' + label + (label === 'Badge' ? ' `soft`' : ':')))
    .split('|').map(s => s.trim());
  for (let t = 0; t < 2; t++) {
    const v = T[t], n = c[t + 2].match(/\d+\.\d+/g).map(Number);
    ['primary', 'ok', 'stale', 'failed'].forEach((k, i) => check(`${t} ${label} ${k}`,
      R(rgb(v[k === 'primary' ? 'ring' : k]), mix(rgb(v[k]), rgb(v.popover), p)), n[i]));
  }
}
for (const line of lines.filter(l => /^\| `(background|card|row)` →/.test(l))) {
  const c = line.split('|').map(s => s.trim());
  const [a, b] = [...c[1].matchAll(/`([^`]+)`/g)].map(m => m[1]);
  for (let t = 0; t < 2; t++) check(`${t} step ${a}/${b}`,
    R(rgb(T[t][a]), rgb(T[t][b])), parseFloat(c[t + 2]));
}
for (const line of lines.filter(l => l.startsWith('| `syntax-'))) {
  const c = line.split('|').map(s => s.trim()), k = c[1].slice(1, -1);
  const n = c[4].match(/\d+\.\d+/g).map(Number);
  for (let t = 0; t < 2; t++) check(`${t} ${k}`, R(rgb(T[t][k]), rgb(T[t].raised)), n[t]);
}
console.log('checked', count, 'cells');
console.table(differences);
for (let t = 0; t < 2; t++) {
  const c = k => rgb(T[t][k]);
  console.log(t ? 'light' : 'dark', G.map(g => ({ ground: g,
    ring: R(c('ring'), c(g)), disabled75: R(alpha(c('ring'), c(g), .75), c(g)) })));
  console.log('stale on wrongly raised-mixed hover', R(c('stale'), mix(c('stale'), c('raised'), .1)));
  console.log('actual stale hover', R(c('stale'), mix(c('stale'), c('popover'), .1)));
}
```

## Final verdict and hand-back

**PASS-with-findings — no P0, one P1 measurement/documentation correction, one P2 policy note.** The accepted tokens, Badge union, generated classes, identity folds, actual chip rendering and focused-disabled ring achieve ticket 30's goal without a demonstrated AA regression or layout change. Before 31 relies on the contrast tables, correct the intermediate-rounding proof and regenerate the affected numbers (F1); clarify the existing amber-text-on-raised exception rather than silently implying it is absent (F2). No palette or source-surface redesign is warranted. The focused tests/type-check/style gate are freshly green; the orchestrator still owns the full definition-of-done gate and commit.
