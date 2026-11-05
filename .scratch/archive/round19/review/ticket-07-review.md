fix-then-ship

# Ticket 07 adversarial review — 2026-09-29

Reviewed **`553db2a → 9d70918`**, not the concurrent working-tree changes. Two blocking findings, four should-fix findings, one documentation nit. No product source changed.

## Findings, ranked

### B1 — blocking: the primitives do not draw their advertised focus outline

**Locations:** `src/components/ui/styles.ts:11–12`, `src/components/ui/field.tsx:173`; separately, `src/components/ui/dropdown-menu.tsx:18–19`.

`outline-none focus-visible:outline-2 …` does not restore a solid outline in the installed Tailwind 4.3.1. The production CSS contains:

```css
.outline-none { --tw-outline-style:none; outline-style:none }
.focus-visible\:outline-2:focus-visible {
  outline-style:var(--tw-outline-style); outline-width:2px
}
```

Consequently Button, Chip, Checkbox, Switch, Tabs, Segmented and the fields compute **`outline-style:none` even when `:focus-visible` is true**. Dialog Close inherits the same failure. The ring token's excellent contrast does not help an undrawn ring.

**Reproduction:** open `/?specimen`, open its dialog, keyboard-focus Cancel/Save/Close. Trusted Chromium key events with focus emulation enabled produced `:focus-visible=true`, `outline: rgb(150,144,255) none 2px` in dark and `rgb(90,79,220) none 2px` in light. A separate preview CSS-state probe reproduced this across all seven primitive families. Setting only `--tw-outline-style:solid` in that probe changed the computed style to `solid 2px`.

DropdownMenu has a second defect: `itemBase` does not use the shared focus treatment at all. Its highlighted neutral item has unchanged foreground text, no outline, no shadow, and only `raised` against `popover` to indicate focus. Independently rendered with the required Menu Group, that distinction measured **1.10:1 dark / 1.16:1 light**, below the requested 3:1 non-text floor. Fixing `focusRing` alone will not fix the menu.

**Fix:** explicitly restore solid outline style in the focus-visible state (or use a ring implementation that is not neutralized by `outline-none`), including the separately spelled-out field classes. Give keyboard-highlighted menu items a ≥3:1 cue, preferably an inset iris outline to avoid popup clipping. Verify compiled CSS and a trusted keyboard run in both themes; a test that merely searches for the class string is insufficient.

### B2 — blocking: legacy input boundaries still use the decorative line token

**Locations:** `src/App.css:19`, `:225–227`, `:1426–1429`, `:1870–1875`, `:1944–1949`, `:2325–2329`.

The alias block correctly maps `--border-subtle` to `--border`, but the unmigrated search/text/select fields still use that decorative token as their identifying edge. These are not decorative separators or text-bearing outline buttons. Their almost-identical fill does not provide an alternative boundary.

**Reproduction:** at `?scenario=rich`, inspect the unfocused `.search-input` in both themes. Fresh rendered measurements:

| Pair | Dark | Light |
|---|---:|---:|
| Border / outside ground | **1.33** | **1.25** |
| Border / input fill | **1.25** | **1.17** |
| Input fill / outside ground | **1.07** | **1.07** |

The new primitive's `input` token itself passes (≥3.32); the legacy controls are not using it. These selectors predate the ticket: this is an unclosed world-ticket contrast gate, not a claim that the selectors were newly introduced.

**Fix:** change the existing control-boundary declarations to `var(--input)` in place, retaining `border` for structural hairlines. This needs neither a content redesign nor additional App.css lines. Check default, focused and disabled states in Search, Add/Re-point and Settings.

### S1 — should-fix: the guard's five happy-path probes work, but ordinary equivalents bypass three rules

**Locations:** `scripts/check-styles.mjs:112–139`, `:158`, `:176–185`, `:260–267`.

Every explicitly requested mutation failed as intended. Additional independent mutations in the archived copy passed despite violating the stated rules:

| Mutation, applied alone | Exit | Hole |
|---|---:|---|
| `.prose + button { color:var(--foreground) }` | **0** | Selector starts with `.prose` but styles a sibling outside it. |
| `.prose { color:var(--missing, #ff0000) }` | **0** | Removing the whole `var(...)` expression hides its literal fallback. |
| `const legacy="btn"; … <div className={legacy}/>` in a listed migrated file | **0** | The scanner does not follow even a local constant. |
| `<div className="bg-[red]"/>` | **0** | Arbitrary named colours are not checked. |
| `<div style={{color:"#ff0000"}}/>` | **0** | The broader “no raw colour in JSX” claim is not implemented for inline styles. |

**Fix:** add regression tests for the guard itself; use a CSS selector/value parser to establish containment and inspect fallbacks, and a TS/JSX-aware scan for class expressions/local constants and colour-valued style properties. At minimum, reject the concrete cases above rather than advertising coverage they do not have. The first four violate the ADR's explicit scoped-selector/token/class/arbitrary-colour rules; inline styles also violate AGENTS.md's broader no-raw-colour contract.

Related observation, not a discovered production call-site violation: **layout-only `className` is a convention, not enforced**. `<Button className="h-2 rounded-full bg-failed text-xs"/>` passes the guard, type surface and `cn()` merge (`button.tsx:81`). I found no such visual restyling in the committed mechanical Modal replacements. Consider an import-aware lint rule before the surface migrations multiply callers; do not add runtime class sanitization.

### S2 — should-fix: Field can silently sever its label association

**Locations:** `src/components/ui/field.tsx:94–96`, `:163`; `src/components/ui/checkbox.tsx:15–17`.

Two small compositions contradict the wrapping's “label always associated” promise:

```tsx
<Field><FieldLabel>URL</FieldLabel><Input id="repo-url" /></Field>
<Field><FieldLabel>Cursor</FieldLabel><Checkbox /></Field>
```

The first leaves `htmlFor` pointing to the generated id while the input uses `repo-url`. The second is explicitly suggested by Checkbox's comment, but Checkbox never consumes the Field wiring; it has an unrelated Base UI id. Independent jsdom probes returned **`input.labels.length === 0`** and **checkbox accessible name `""`**, respectively. Existing tests exercise only the generated-id input and wrapping-label checkbox cases.

**Fix:** own the control id at the Field seam and wire labels/descriptions to that same resolved control; integrate Checkbox through the appropriate Base UI/ARIA label mechanism, or explicitly remove that unsupported composition from the contract. Add tests for an explicit id and each advertised control composition. Merely assigning a `<label for>` to a non-labelable span is not a complete Checkbox fix.

### S3 — should-fix: “focus never lands on body” is conditional, and deletion callers do not supply the condition

**Locations:** `src/components/ui/dialog.tsx:165–167`, `src/components/skills/modals/DeleteModal.tsx:36`, `src/components/projects/RemoveProjectModal.tsx:36`, `DESIGN.md:248–250`.

The provided `fallbackFocus` works. The default is not a universal fallback: with the only opener removed, no supplied fallback and no lower layer, the wrapper returns `true` to Base UI, which has no connected target. An independent probe with a focusable Library heading, a Delete-row opener, and a confirm that removes the opener and closes the dialog settled on **BODY**. It fails a `not.toBe(document.body)` assertion. Neither committed removal dialog provides `fallbackFocus`.

**Fix:** connect the planned lifecycle/surface fallback (focused/next row → heading) to removal dialogs; require a fallback where the opener can disappear, or provide a stable app-owned fallback through the dialog seam. Add a removed-opener integration contract test. Until ticket 09/D21 provides the application target, qualify DESIGN.md's unconditional assertion. This is **not** a failure of the supplied-fallback test or of the lower-layer fallback, both of which passed; it is an integration gap that should not be hidden by their success.

### S4 — should-fix: the specimen's Menu crashes instead of exercising the menu

**Location:** `src/components/ui/specimen.tsx:189–197`.

**Reproduction:** open `/?specimen` and open Menu. `<DropdownMenuLabel>` wraps Base UI GroupLabel but has no DropdownMenuGroup ancestor. React reports **“An error occurred in the <MenuGroupLabel> component”** and unmounts the specimen. The existing menu test omits the label, so it stays green. This is dev-only, but invalidates this branch of the purported whole-primitives verification surface.

**Fix:** wrap the label and its items in `DropdownMenuGroup`, as required by the Base UI/shadcn composition contract; exercise the labelled group in the primitive test. The valid-group runtime probe used for B1 did not edit source.

### N1 — nit: qualify documentation that currently overstates implementation

**Locations:** `DESIGN.md:299`, `AGENTS.md:159`, `src/components/ui/styles.ts:8`.

- DESIGN says there is no `transition:all` and hover changes instantly; **22** `transition:all` declarations remain in App.css, generally 200 ms. Reduced motion does zero them, but the normal-motion claim is false for legacy surfaces. Scope the statement to migrated primitives and point at ticket 15/the migration rather than asserting it globally.
- AGENTS still bans all component rendering tests while D2 deliberately permits `ui/*.test.tsx`. This is already acknowledged in the ticket's Deferred section; update the exception rather than making the next implementer rediscover it.
- The shared focus comment says the ring is ≥5.7 against every ground; light raised is **5.04**. DESIGN's table is correct; update the comment when fixing B1.

## Coverage of the nine requested areas

| # | Area | Result and evidence |
|---|---|---|
| 1 | Contrast, both themes/states | **Fail non-text; sampled text passes.** Independently remeasured the token matrix, all specimen Button/Badge combinations across four grounds, hover, disabled/pending, Switch and the legacy regression pairs. B1/B2. Detailed values below. |
| 2 | Dialog / stack / tests | **Wrapper behaviour passes the focused keyboard probe; integration caveat S3.** Accessible title, inert root/lower popup, both Tab directions, top-only Escape and trigger return passed. Supplied fallback and removed-child-trigger lower-layer fallback passed jsdom. Normal Close is 32 px by the primitive; no native Tauri/WebView or screen-reader certification claimed. |
| 3 | Primitive depth | **Partial.** Complete 3×5 Button and Badge tone maps; Base UI state semantics and pending nonactivation tested. The only primitive transition is Switch's transform, with reduction; spinner uses motion-safe. Pseudo-element hit extensions give Checkbox/compact Switch their 24 px hit area; icon sizes own 24/32. B1 and S2 remain. `className` is comments/convention, not enforced; no committed replacement caller abuses it. |
| 4 | Tokens / aliases / fonts | **Pass.** One token-definition file, dark-first values, light overrides, inline colour registration, static non-colour scale, palette removal, custom dark variant. Enumerated every old index.css custom-property definition against aliases plus same-name new tokens: missing set **empty**. Google Fonts import removed. Faint deviation recorded explicitly. Dormant Layout/Dashboard raw-utility exemptions are declared, not concealed. |
| 5 | Guard / CI | **Partial.** All five requested negative probes exit 1; additional counterexamples in S1 exit 0. `styles:check` is in package `check` and CI web job (`ci.yml:30`). Baseline 3038; migrated array empty by this ticket's explicit requirement. |
| 6 | Pre-paint theme | **Pass.** Fresh production build places inline script before module/CSS. Key test pins `skills-theme`. Executed built script for null/system/invalid/throwing storage and both OS schemes; explicit light/dark win. Actual specimen reload with light stored + dark system produced light data-theme/color-scheme and `rgb(252,252,253)` body before any settings-world mount. |
| 7 | Modal replacement / asks / loading / theme ownership | **Pass code review, with B1/S3 caveats.** Reviewed every committed replacement diff: content, actions and request resolution retained; wrappers/width/body/footer composition changed, not the flows. Both ask hooks have no diff in this commit. LoadingOverlay now has labelled/described Dialog and status text; global reduction covers its legacy loops. Settings returns resolvedTheme; SkillDetailView no longer reads DOM theme and needs no JS theme prop after class-output conversion. |
| 8 | Minimum / prose / highlighter | **Pass inspected contract.** Config has 960×640; preview was measured at 1280×800 and 960×640, with width 960 and no document horizontal overflow in the library sample. Structural collapse is explicitly ticket 11, not this ticket. Current prose selectors are scoped and token-only. A fresh render of the exact highlighter props emits `prismjs code-file`, `token keyword/string/comment`, and `linenumber` classes, with no inline colours (layout/white-space styles remain intentionally). Native min-window enforcement was not run. |
| 9 | Claims vs implementation | **Partial.** B1, S2/S3 and N1 identify the mismatches. The specimen is also not a usable complete menu check until S4 is fixed. |

## Independent contrast measurements

Method: WCAG 2.x sRGB relative luminance; computed browser colours resolved through a canvas, alpha composited over ancestor backgrounds. Hover CSS was applied in an isolated state probe with transitions disabled, not guessed from token names. Focus was measured separately with real trusted keyboard events. Token/style files and primitives were re-compared with `9d70918` during the run and had **no working-tree diff**.

Order in each cell: **background / card / popover / raised**. These independently reproduce DESIGN's opaque-pair table to rounding. `border`/`border-strong` are acceptable *only when decorative*; using them as identifying input boundaries is B2.

| Token | Dark | Light |
|---|---|---|
| foreground | 15.50 / 14.51 / 14.51 / 13.21 | 17.32 / 16.14 / 17.76 / 15.28 |
| muted-foreground | 8.31 / 7.78 / 7.78 / 7.08 | 8.16 / 7.60 / 8.36 / 7.20 |
| faint | 6.12 / 5.73 / 5.73 / 5.22 | 6.09 / 5.67 / 6.24 / 5.37 |
| ring | 7.05 / 6.61 / 6.61 / 6.01 | 5.71 / 5.32 / 5.86 / 5.04 |
| primary (fill/non-text in dark) | 4.10 / 3.84 / 3.84 / 3.49 | 5.71 / 5.32 / 5.86 / 5.04 |
| ok | 10.74 / 10.06 / 10.06 / 9.15 | 5.69 / 5.30 / 5.84 / 5.02 |
| stale | 10.41 / 9.75 / 9.75 / 8.88 | 5.62 / 5.23 / 5.76 / 4.95 |
| failed/destructive | 6.91 / 6.47 / 6.47 / 5.89 | 5.50 / 5.12 / 5.64 / 4.85 |
| border | 1.33 / 1.25 / 1.25 / 1.13 | 1.25 / 1.17 / 1.28 / 1.10 |
| border-strong | 1.65 / 1.55 / 1.55 / 1.41 | 1.49 / 1.39 / 1.53 / 1.32 |
| input | 3.90 / 3.65 / 3.65 / 3.32 | 3.78 / 3.52 / 3.87 / 3.33 |
| syntax-keyword | 9.79 / 9.17 / 9.17 / 8.34 | 6.47 / 6.03 / 6.64 / 5.71 |
| syntax-string | 10.22 / 9.57 / 9.57 / 8.71 | 6.26 / 5.84 / 6.42 / 5.52 |
| syntax-number | 10.34 / 9.68 / 9.68 / 8.81 | 7.03 / 6.55 / 7.21 / 6.20 |
| syntax-function | 9.88 / 9.25 / 9.25 / 8.42 | 7.21 / 6.72 / 7.39 / 6.36 |

Additional pairs, **dark / light**:

- Foreground on solid accent: **4.68 / 5.86**; solid ok **10.74 / 5.84**; solid stale **10.41 / 5.76**; solid danger **6.91 / 5.64**.
- Worst sampled solid hover across the four grounds: accent **5.33 / 4.81**, ok **8.85 / 4.78**, stale **8.59 / 4.70**, danger **5.82 / 4.88**. Small differences from DESIGN's translucent values arise in browser/canvas colour conversion/compositing; no threshold decision changes. Opaque values reproduce the table.
- Soft Badge / soft hover: accent **6.07 / 5.08**, ok **8.53 / 5.07**, stale **8.32 / 4.99**, failed **5.71 / 4.89**.
- Faint on the actual legacy hover-ground alias: **4.60 / 4.68**, independently reproduced with a rendered token probe. The recorded faint deviation is justified.
- Eight-percent state tints: ok **8.89 / 5.22**, stale **8.62 / 5.13**, failed **5.88 / 5.04**. Actual success toast title **and description** both measured **8.89 / 5.22**. The info toast uses the normal popover/foreground pair, not the hypothetical accent tint.
- Switch checked thumb/track **8.31 / 8.16**; unchecked thumb/track **7.08 / 7.20**. Check glyph **4.68 / 5.86**; selected segment iris edge **6.01 / 5.04**.
- Sampled disabled/pending solid-neutral text: **4.33 / 3.27** after dimming. These are genuinely disabled/nonactivatable controls and contrast-exempt; not falsely reported as ordinary text failures.
- Required focus cue: **absent**, notwithstanding the ring-colour ratios. Menu highlighted ground: **1.10 / 1.16**. Legacy search boundary: **1.33 / 1.25** against its outside ground.

Legacy regression checks on the fixture (same aliases as the commit): tertiary **6.12 / 6.09**, synced pill **10.06 / 5.30**, failed pill **6.39 / 4.94**, +N **5.22 / 5.37**, Install **7.05 / 5.71**, Installed light **5.09**, placeholder **5.73 / 5.67**, Match light **5.03**, Conflict light **4.94**, success toast **8.89 / 5.22**. Library, Import and Explore normal/forced-hover text scans produced no enabled-text failure. The corresponding dark Installed/Match pairs agree with the alias/token calculation; unlike the other listed pairs, they were not retained as separate element-specific transcript rows. Do not turn that into an all-surface conformance claim.

## Dialog keyboard evidence and test sensitivity

T3 preview measurements were performed first. Its hidden tabs reported no window focus and also reloaded during concurrent edits; I did **not** count the failed/interrupted `preview_press` attempts as proof of broken Tab containment.

Supplementary keyboard proof used a **named headless Chromium session `r19-t07-review` against the existing port 5175**, not a second dev server or the user's browser. A single CDP session enabled `Emulation.setFocusEmulationEnabled`; trusted `Input.dispatchKeyEvent` events were checked with `isTrusted=true`, `document.hasFocus()=true`. Each assertion waited two animation frames, allowing Base UI's focus-guard callback to settle. Without that wait, observing a transient guard span is not a trap failure.

In **both themes**, on the unchanged specimen primitives:

1. Programmatic setup focuses/clicks Open dialog → Cancel focused, named dialog present, root inert.
2. Trusted **Shift-Tab** → Close (backward wrap).
3. Trusted **Tab** → Cancel (forward wrap).
4. Trusted **Tab → Tab → Tab** → Save → Close → Cancel.
5. Programmatic setup opens the sibling ConfirmDialog → its Cancel focused; first popup inert, second not inert.
6. Trusted **Escape** → only confirm closes; parent Cancel restored; root remains inert, parent no longer inert.
7. Trusted **Escape** → parent closes; Open dialog restored; root no longer inert.

Every focused primitive in that run had `outline-style:none` (B1). This is a wrapper-focused keyboard transcript with explicit setup, **not** a six-flow keyboard-only app completion claim.

Test assessment:

- Existing title/disabled/pending/inert/top-Escape/explicit-fallback tests make meaningful assertions; deleting those behaviours would fail the matching tests.
- The test comment says “trapped”, but there is **no Tab-wrap assertion**. Base UI's modal prop and focus guards support the contract; the focused browser probe above now supplies separate evidence.
- The existing suite has no compiled-CSS assertion to catch B1. The Close test only checks `size-8`, not hit testing.
- An independent archived-copy suite added six probes: lower-layer return after removal of its child trigger, explicit-id Field, Field+Checkbox, removed lone opener without fallback, initially disabled confirmation, and StrictMode stack cleanup. **3 passed / 3 failed**: the two Field cases and lone-opener return failed. The initially disabled-confirmation result is jsdom-only and is not a native-disabled-focus certification.

## Guard mutation transcript

All mutations were made **one at a time in `/tmp/skills-hub-07-review.6txkkS`**, restored after each invocation; no tracked product file was used as a scratchpad. Each ran `node scripts/check-styles.mjs`:

| Requested mutation | Outcome |
|---|---|
| Add `src/review.css` containing `.x{}` | Exit **1**, `[stylesheet]` |
| List `src/ReviewProbe.tsx` as migrated; render `<div className="btn"/>` | Exit **1**, `[migrated] … btn` |
| Render `<div className="bg-[#123456]"/>` | Exit **1**, `[raw-colour]` |
| Append `button {color:var(--foreground)}` to prose.css | Exit **1**, `[prose] … outside the prose root` |
| Append one nonblank rule line to App.css | Exit **1**, `[growth] … 3039 … ceiling 3038` |

The five passing counterexample probes plus the primitive-restyling probe are recorded in S1. CI wiring is real, but wiring cannot compensate for blind spots in the checker.

## Commands and outcomes

Pinned-copy setup (no git checkout/worktree mutation):

```sh
REVIEW=$(mktemp -d /tmp/skills-hub-07-review.XXXXXX)
git archive 9d70918 | tar -x -C "$REVIEW"
ln -s "$PWD/node_modules" "$REVIEW/node_modules"
cd "$REVIEW"
npm run styles:check
npx vitest run src/components/ui src/lib/preferences.test.ts
npm run build
```

- Archive path: `/tmp/skills-hub-07-review.6txkkS`. Dependency versions are the existing lockfile-installed dependencies; no install or lockfile changes.
- `npm run styles:check`: **exit 0**, 3038 lines / 0 migrated files.
- Original targeted Vitest command: **exit 0, 7 files / 33 tests passed** (before adding independent review probes). Only the Node experimental-localStorage warning.
- `npm run build`: **exit 0**, tsc build and Vite success. 858 modules; CSS 95.30 kB; main JS 1,317.37 kB. Warnings about >500 kB chunk and ineffective dynamic imports; no compilation failure.
- Built `dist/index.html`: inline pre-paint script at lines 8–29; module line 30; CSS line 31.
- `npx vitest run src/components/ui/review-probe.test.tsx --reporter=verbose` in the archive: **exit 1, 3 failed / 3 passed**, expected adversarial failures described above. That test file is not in the repository.
- Guard mutation commands: **five expected exit-1 probes; counterexamples exit 0**, table above.
- Executed the built inline theme script with `node:vm` through Bun: **12/12 cases correct** (null/system/light/dark/invalid/throwing storage × dark/light system).
- Compared old `index.css` property definitions (`git show 553db2a:src/index.css`) to alias/new-token definitions: **missing aliases `[]`**.
- `npx shadcn@latest docs dialog popover`; fetched Base UI's Dialog API and checked installed `DialogPopup` / `FloatingFocusManager`: modal focus trapping, initial/final-focus function semantics and frame-scheduled guards match the wrapper's use.
- `bun /tmp/skills-hub-07-review.6txkkS/keyboard-probe.mjs`: trusted settled Tab/stack/Escape sequence above; repeated light-theme sequence likewise passed behaviour and reproduced B1.
- `agent-browser --session r19-t07-review set media dark reduced-motion`, then **reload without the contrast probe's transition overrides**: media query true, pending spinner animation `none` / `0s`, Switch transition `0s`. Light stored preference + dark system reload also verified as described in coverage.
- Highlighter smoke: `bun run -` with `renderToStaticMarkup(<Prism style={{}} useInlineStyles={false} language="javascript" className="code-file" showLineNumbers …>)`: expected Prism/token/linenumber classes; no inline colour/theme leaked.

## Limits and scope discipline

The Rust implementation, ticket-01 semantics and the specified excluded files were not reviewed. The Tauri configuration was read solely for the explicitly requested window minimum. No Tauri launch, real-library operation, product-source edit, issue archival/move or commit.

The fixture server is a **live shared checkout**, not an immutable deployment of `9d70918`. During this review tickets 02/05/08 changed the app and caused HMR/navigation resets (the later fixture has `#/skills`). Findings are grounded in the archived commit and unchanged styling/primitive modules; later transient application-flow failures were not attributed to ticket 07. No claim is made that every later live dialog flow, Projects state or failure scenario was end-to-end re-audited. Native macOS/Windows WebViews, screen readers and physical minimum-window enforcement remain the operator lane.

**Goal-backward check:** the tokens and most behaviour are usable foundations, and the guard is actually wired into CI. However, the stated accessible world is not achieved while every primitive's intended focus outline is suppressed and legacy identifying input boundaries remain below 3:1. Fix B1/B2 before calling this world ticket shippable; resolve or explicitly disposition S1–S4 before relying on these interfaces for the remaining migrations.
