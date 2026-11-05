PASS-with-findings

# Ticket 35 — adversarial audit

Reviewed the **uncommitted `git diff`** in `/Users/alexstark/.t3/worktrees/skills-hub/t3-684f3510`, branch `t3/dark-ground-two-tone`, base `85b79fe`. Source remained read-only; this report is the only repository file the auditor wrote. No commit. The accepted palette, pane step, black state ink, selection edge and orchestrator's row/control hover rules were treated as fixed decisions, not design proposals to reopen.

Used `web-design-guidelines` with a fresh fetch of its upstream rules, the installed token/primitive guidance and the ticket-30 audit's float-precision method. Per the child brief, I performed the review myself rather than spawning reviewers. The product brief and approved decisions supplied direction; no new design interview was needed.

## F1 — P1 (fix before 31): inline code did not follow its documented `raised` role through the split

**Evidence:** `src/styles/prose.css:144–152`, specifically `:148`, still draws `.prose .md-inline-code` with **`background: var(--muted)`**. Before this diff, dark `muted = raised = #1d2025`; after it, the code pill is `#0e0e0e`, not `#1e1e1e`. `DESIGN.md:63` explicitly assigns **inline code** to `raised`; `src/styles/theme.css:40–42` says the same. This is an actual split-sensitive consumer missed by the otherwise good sweep.

Reproduction commands:

```sh
rg -n 'md-inline-code|background: var\(--muted\)' src/styles/prose.css
# 144:.prose .md-inline-code {
# 148:  background: var(--muted);
```

Browser reproduction used the **actual imported Dialog primitive and the existing `.prose .md-inline-code` rule**, composed in memory on the fixture specimen (not an edited source file). A `<p class="prose">` containing `<code class="md-inline-code">example()</code>` inside DialogBody computed to:

```text
inline background: rgb(14, 14, 14)
dialog background: rgb(14, 14, 14)
inline border: 1px solid rgb(23, 23, 23)
inline text: rgb(245, 245, 245)
```

Capture: `/tmp/t35-menu-over-dialog.png` (the code pill is above the nested menu). Its fill-to-panel contrast is **1.000000**, while the intended `raised` fill would be **1.157958**. On `card`, the actual pill steps **1.062040**, versus **1.229798** for `raised`. This is **not a text-contrast failure**: the current text is 17.71 on muted, and the border/monospace still distinguish code. It is a documented-role regression caused by the split, precisely the kind of consumer this ticket asks to catch.

**Exact proposed fix:** change only `.prose .md-inline-code`'s `background` from `var(--muted)` to **`var(--raised)`** in `src/styles/prose.css:148`. Leave blockquotes and table headers on `muted`, and code blocks on `card`. This restores the documented role without adding a token or changing any accepted value; light is unchanged because its muted and raised are equal. Recheck the inline pill on card/popover/raised and run `styles:check`. If the operator instead intends panel-tone code pills, explicitly correct the two role claims rather than saying inline code is raised.

## F2 — P2 (note): neutral outline controls on `raised` have no pointer-hover change

**Evidence:** `src/components/ui/button.tsx:43`, specimen `src/components/ui/specimen.tsx:54,83–106`. On `/?specimen`, hover `[data-surface=bg-raised] [data-specimen=button-outline-neutral]`. Settled browser output:

```text
hover: true
background: rgb(30, 30, 30)
ancestor ground: rgb(30, 30, 30)
text: rgb(245, 245, 245)
border: 1px solid rgb(38, 38, 38)
```

At rest its transparent background exposes exactly the same ground; text and border do not change. Fill step **1.00**; `/tmp/t35-raised-specimen.png`. A neutral ghost on raised also has a 1.00 fill step, **but does brighten its text** (`#a3a3a3` → `#f5f5f5`), so it does not lose all feedback. A selected row's ghost overflow therefore still has a hover cue.

This outline case **predates the change** (old accent and raised were equal) and I found no newly broken production composition relying on it. It does not block ticket 35 and is not a reason to undo the pinned control-hover rule.

**Exact proposed fix:** add a short Button usage note to DESIGN.md: neutral outline on a raised parent has no hover delta; do not rely on it for pointer feedback. If ticket 31 introduces such a composition, choose a suitable existing ghost/semantic control, or design and document a primitive variant for that use. Do not add a call-site colour override or change the global raised-hover rule as a drive-by fix.

## F3 — P2 (note): semantic `ok` hover on raised crosses the requested near-invisibility threshold

**Evidence:** independent Appendix A output for `styles.ts:109` / `button.tsx:45,51`:

```text
SOFT HOVER STEP ok 1.239572 / 1.210186 / 1.139492 / 1.016206 / 1.239572
OLD SOFT ON RAISED ok 1.0682540759474184
```

The 10% OKLab ok/popover fill has only **1.016206** contrast against raised (≤1.02 by the brief's criterion), down from **1.068254** before this diff. These semantic buttons retain the same text/border on hover, unlike neutral ghost's brightened ink. Text on the fill still passes at **9.494315**; the loss is the hover fill's luminance cue, not readability. Hue can still provide a faint cue, so this is not described as literal RGB equality.

Reproduction: run Appendix A; for the old value compute `R(mix(rgb(prev.ok), rgb(prev.popover), .1), rgb(prev.raised))` with the same functions. `rg -n 'tone="ok"|tone: "ok"' src --glob '*.tsx'` finds **no production ok Button caller** (the only product `tone="ok"` match is a Badge in ToolConfigModal); the specimen exercises the variant. Thus no current production task is blocked and this is a P2 design-system edge case, not a demand to change the accepted state or ground values.

**Exact proposed fix:** document this pair next to the raised-parent limitation in F2 and prohibit relying on its fill alone. Before introducing an ok ghost/outline Button on raised in a product surface, give that composition a documented primitive-owned non-fill hover cue (for example an underline) or use a different existing visual weight. Preserve the opaque popover-based mix, whose text contrast is correct; do not replace it with a translucent tint over arbitrary ancestors.

## Verification record

### 1. Value fidelity — PASS

Mechanically compared the raw dark declarations and complete light block against `HEAD:src/styles/theme.css` with Node (`git show` read only). The independent script in Appendix A asserts the exact accepted mapping and that **all other dark hex tokens are unchanged**.

```text
LIGHT BLOCK BYTE IDENTICAL true
25 changed dark hex declarations, all in the accepted mapping
```

Changes are background/row black; card/sidebar #040404; popover/accent/muted/sidebar-accent #0e0e0e; raised #1e1e1e; foreground aliases #f5f5f5; secondary/sidebar foreground #a3a3a3; faint #898989; border/sidebar-border #171717; border-strong #262626; input #6b6b6b; four state foregrounds black. Iris, state fills, identity, syntax and elevation values are unmoved. No token added/renamed, and no light hunk. ADR-0006's four-line amendment (`docs/adr/0006-styling-architecture.md:63–66`) describes the implementation accurately.

**Qualification of “light unchanged”:** light **tokens** and the resting light My Skills appearance are unchanged. The approved last Comment deliberately changes the three row hovers from `card` to `accent` in both themes; in light that is #f0f1f4 → #e4e7ec. This is expressly authorized, not an accidental light-token edit. Neutral control/menu/sidebar changes resolve to the former light values.

The prototype header already contains the dated operator verdict `(d)` + kept edge → `(f)` at `.scratch/round19/prototypes/30-dark-ground/index.html:15–16`; no missing prototype-record finding.

### 2. Independent contrast — PASS; no >0.01 errors in the current dark tables

I wrote and ran my own normalized floating-point RGB, WCAG luminance, linear-sRGB ↔ OKLab matrices, OKLab mixing and sRGB alpha-compositing functions **before reading the rebuilt child script**. I did **not execute or import** `.scratch/round19/evidence/30-contrast.mjs`. Its subsequent source review confirms it now keeps intermediates floating point. Appendix A is a self-contained reproduction of my calculation.

**95 dark table cells mechanically compared, zero differences greater than 0.01:** 60 Text/Accent-and-state/Lines cells, eight ground steps, eight fill/hover cells, eight soft-ground cells, seven identity text/tint cells and four syntax cells. The prose claims, identity tint/card and foreground/card ranges, other non-text pairs, hardest-pair list and all focus floors were independently checked as well. No passing gated pair was misclassified. Decorative borders and disabled controls remain explicitly exempt.

Ground order below is **background / card / popover / raised / row**:

| Pair | Independent values |
|---|---|
| foreground | 19.261973 / 18.805339 / 17.706805 / 15.291399 / 19.261973 |
| muted-foreground | 8.325052 / 8.127694 / 7.652906 / 6.608964 / 8.325052 |
| faint | 6.003166 / 5.860852 / 5.518484 / **4.765701** / 6.003166 |
| ring | 7.730110 / 7.546856 / 7.105998 / **6.136660** / 7.730110 |
| input | 3.940545 / 3.847129 / 3.622395 / **3.128259** / 3.940545 |
| primary fill | 4.487951 / 4.381557 / 4.125604 / **3.562825** / 4.487951 |
| border, decorative | 1.171363 / 1.143594 / 1.076789 / 1.075381 / 1.171363 |
| border-strong, decorative | 1.387647 / 1.354751 / 1.275612 / 1.101604 / 1.387647 |

| Fill text | Solid | Worst `/90` |
|---|---:|---:|
| white / primary | **4.679195** | 5.323086 (over raised) |
| black / ok | 11.768888 | 9.504936 (over black) |
| black / stale | 11.014441 | 8.915712 (over black) |
| black / destructive | 7.567385 | 6.202208 (over black) |

| Opaque OKLab mixture into popover | accent/ring | ok | stale | failed |
|---|---:|---:|---:|---:|
| 10%, Badge/semantic hover | 6.609040 | 9.494315 | 8.925990 | 6.291165 |
| 8%, toast table | 6.719337 | 9.799244 | 9.201395 | 6.442211 |

The accent entry in the toast table is a computed potential pair, not a claim that the actual info toast is iris-tinted: `sonner.tsx` makes info neutral. State toasts really use the 8% tint and a 35% state edge.

| Identity hue | Text/tint | Tint/card | Text/card |
|---|---:|---:|---:|
| moss | 8.773795 | 1.520375 | 13.339455 |
| teal | 8.767517 | 1.541307 | 13.513436 |
| cyan | 8.804441 | 1.522305 | 13.403044 |
| sky | 8.646867 | 1.516753 | 13.115160 |
| orchid | 8.424196 | 1.474569 | 12.422059 |
| rose | 8.118968 | 1.470805 | 11.941416 |
| slate | 8.627317 | 1.501881 | 12.957202 |

The chips remain visibly tinted, discrete shapes on the specimen's card section; tint contrast is decorative, not a 3:1 claim. Initials have ample contrast on both tint and header. Production headers do not yet carry these chips (ticket 31), so the specimen is the correct consumer proof.

Syntax keyword/string/number/function on raised rounds to **8.52 / 8.89 / 8.99 / 8.59**. Comment/line-number faint still clears 4.5 on all five grounds. Other non-text pairs independently reproduce DESIGN.md: switch off thumb/track **6.61**, checked thumb/track **8.33**, check glyph/primary **4.68**, selected iris edge/raised **6.14**, soft-neutral Badge **7.65**, solid-neutral Badge **15.29**, inverted Tooltip **19.26**.

#### Extra adversarial pairs

| Constructed pair | Independent result | Assessment |
|---|---:|---|
| muted-foreground on raised, inside a popover's hovered ghost | **6.608964** | Text passes. Actual neutral ghost brightens to foreground on hover, so actual label is 15.291399; the requested muted-text construction is the stricter case. |
| faint on neutral soft Badge (`muted`), on raised | **5.518484** | Text passes. Badge fill vs raised 1.157958; static label, not state boundary. Actual neutral Badge text is muted-foreground, 7.652906. |
| border between two popover layers | **1.076789** | Thin but nonzero edge, visually present in composed menu-over-dialog capture. Not a 3:1 identifying-control edge; no shadow-only layer found. |
| input checkbox edge on menu's raised highlight | **3.128259** | Non-text passes. Actual DropdownMenuCheckboxItem uses a check glyph rather than a boxed Checkbox; standalone-checkbox construction is still safe. |
| off Switch raised track on raised row | fill **1.00**, edge **3.128259**, thumb **6.608964** | Track fill merges, but input border identifies the control; no loss of state. |
| identity tint on card | **1.470805–1.541307** | Decorative; visibly a chip, not just detached initials. |
| menu/palette focus ring on raised | **6.136660** | Pass. Menu only draws the outline in keyboard modality; palette draws it on data-highlighted. Pointer menu highlight alone is the fill step, not an outline. |
| neutral toast border-strong on black | **1.387647** | Visible decorative edge, confirmed in browser. On popover: 1.275612. |
| 50%-disabled ghost text over popover | **2.734605** | Below ordinary text AA, intentionally disabled/exempt. It does not invalidate the focused-disabled ring requirement. |
| inline code on raised | fill **1.157958**, text **17.706805** | Actual code is muted, thus darker than its raised ancestor; readable but inconsistent with its assigned role (F1). |
| outline-neutral hover on raised | **1.00** | No visual change (F2); ghost still changes ink. |

### 3. Hover/state sweep — all matching source occurrences classified

Commands (run in this worktree):

```sh
rg -n 'hover:bg-|data-highlighted:bg-|aria-\[current|data-selected:bg-|data-focused:bg-|bg-accent|bg-muted|bg-raised|bg-card|bg-popover' src
rg -n '#[[:xdigit:]]{3,8}|rgba?\(|:hover|background:' src/App.css
rg -n 'btn-secondary|btn-primary|card-btn|settings-theme-btn|settings-toggle|modal-backdrop|update-modal-notes|bg-element-hover' src --glob '*.tsx'
```

Read `scripts/styles-baseline.json`: 38 migrated files, App.css ceiling 812. The table groups repeated occurrences of the same rule; all production hits from the requested sweep are represented. “Step” compares state fill with the actual underlying ground, not its text.

| Consumer and source location | Dark base → hover/current | Step and distinguishing cue |
|---|---|---|
| SkillRow list `:432`; ExplorePage row `:263`; AssignmentMatrix row `:304` | black → accent | **1.09** hover; browser confirms rgb(14,14,14). Not the former 1.02 card hover. |
| SkillRow selected/focused | black → raised | **1.26**. Selected: checked box + **3px iris leading edge**. Focused: **1px inset ring**. Both coexist; browser confirms all three. Hover does not override raised. |
| Explore focused preview row | black → raised | **1.26**, inset ring; focused and hovered differ in dark. In light same fill but ring persists. |
| SkillRow card layout `:397` | card; hover changes border → border-strong, selected → raised | Fill remains 1.00 on hover but edge changes **1.18** relative to former border; selected **1.23** + leading edge, focused iris border. Not a missing hover cue. |
| AppSidebar `:48–50` | card → sidebar-accent; current raised | **1.06** hover, **1.23** current; text also brightens and current icon is iris. Current stays raised under hover. Expanded theme/shortcuts use panel tone and their open popup is an additional cue. |
| ProjectList `:169–172` | card → raised for hover and selected | **1.23**. Same fills, but selected carries **3px iris edge**, keyboard focus inset outline. `/tmp/t35-project-list-hover.png` shows selected skills-hub and hovered quartermaster together without ambiguity. |
| Matrix cell `:401` | hovered accent row → raised cell | **1.16**; whole-row step 1.09. Cell state word appears on hover/focus, checkbox and marks remain. |
| NotificationsModal `:417` | popover → raised | **1.16**, expanded state additionally chevron rotation and item list. Title foreground and metadata muted stay legible. Actual rich Update-all success produced no child items, so this disclosure rule was source-checked, not claimed as a rendered disclosure capture. |
| Button neutral ghost/outline `:43,49` | transparent on background/card/popover → raised | **1.26 / 1.23 / 1.16**. Ghost additionally brightens ink. On raised: 1.00 (F2). Dialog close verified raised + foreground. |
| Button solid neutral `:37` | raised → border-strong | **1.10**, text **13.88** on hover. |
| Button solid semantic `:38–41` | state fill → same state at 90% over ancestor | Actual fill changes; all ink pairs pass (table above). No selected/focused state is encoded only by this hover. |
| Button semantic ghost/outline; `styles.ts:108–111` | ancestor → opaque 10% state/popover mix | A coloured soft fill, not 10% into the ancestor; text floors **6.29–9.49**. Raised-parent steps accent/ok/stale/failed: **1.076977 / 1.016206 / 1.020816 / 1.047222**. The ok case is ≤1.02 (F3); stale narrowly exceeds it. Full per-ground steps printed by Appendix A. Fixed opaque mixing prevents text failures. |
| DropdownMenu `:24`; command-palette `:87` | popover → raised | **1.16**. Menu keyboard inset outline; palette data-highlighted outline, both **6.14**. Destructive menu item `:100` intentionally uses destructive/10 plus red text, not raised. |
| CandidatePicker single RadioRow `:203` | popover → raised | **1.16**, radio carries selection separately. Verified by Change source → Check source → Choose another skill from this repository…; `/tmp/t35-radio-picker.png`. |
| CandidatePicker multiple CheckRow `:170–186` | popover → unchanged | **No hover rule exists** (not a collapsed bg-raised rule). Checked box and label identify selection. Add dialog capture `/tmp/t35-candidates.png`; this is pre-existing and the audit does not misattribute the RadioRow rule to Add's multiple picker. |
| AssignProjectModal `:173` | popover → raised when available | **1.16**; radio/choice control says selected. Unavailable rows deliberately omit hover. |
| SkillFiles `:33,67` | card (inspector/preview) → raised | **1.23**; in sheet/popover **1.16**. Open file and hover share fill, but active file is font-medium/foreground and has the displayed reader; no new split regression. |
| Chip (`badge.tsx:110`) | ancestor → raised when pressed | **1.26 / 1.23 / 1.16 / 1.00** by ground. **Iris border** is the pressed cue even on raised. |
| Tabs segmented (`tabs.tsx:69`), Segmented (`:57`) | ancestor → raised selected | Same steps; **iris inset edge** prevents loss on raised. Line Tabs use iris underline rather than a fill. |
| Switch `:50–70` | raised off-track → muted-foreground checked | **6.61** track change; off boundary input ≥3.13; checked thumb black ≥8.33. |
| Badge neutral soft/solid `:42,52` | muted / raised static | Text **7.65 / 15.29**. Neither is a hover/selection control; on equal ancestor the fill can merge without erasing the label. |
| Inspector `SkillInspector:198`, ExplorePreviewView `:89` | card pane, border separators | card/black **1.02 accepted pane cue**; border/card **1.14**, headings/spacing divide sections. Controls hover raised, file highlight raised. |
| SkillsList `:187,239`; Explore section header `:358`; ProjectList `:97`; matrix group/header `:243` | card static panes/headers | **1.02** against black, by approved decision; no state depends on that step. |
| BulkRail `:48`, HeaderProgress dialog `:62`, DialogFooter `:436`, Add summary `:195`, palette footer `:234`, SkillFiles header `:205` | card static support bands | card vs popover **1.06**, card vs black **1.02**; explicit border and content structure. Not a selected/hover state. |
| Notifications expanded items `:439`, DeleteModal target block `:189`, Add result message `:411` | card static blocks | Same steps; labelled content, not interactive-state fills. |
| HeaderProgress track `:90`, matrix skeleton `:208`, ProjectList skeleton `:118–120`, EditProjectModal skeleton `:96` | raised | Noninteractive. Progress primary/raised **3.56**, skeleton has no state cue to distinguish. |
| Floating surface `styles.ts:63`, dialog `:276` | popover | Edge audit below. |
| Specimen `:55` | all five grounds | Intentional test matrix; exposes F2 and permits focused-disabled tests on every ground. |
| `Layout.tsx:18` raw grey hover | dormant | Imported by nothing, explicit AGENTS.md/baseline exemption; not a production black-theme consumer. |

#### Legacy aliases and remaining rules

Every alias in `App.css:10–46` uses a token, another alias or a token-only `color-mix`; **no literal dark alias**. Remaining live consumers:

- Settings `.settings-theme-btn` (`:663–685`): black → muted **1.09** on hover plus stronger border and brightened ink; selected ring fill **7.73** on black and black text **7.73**. Does not collapse with hover. Settings text fields use input edge **3.94** on black, not decorative hairlines.
- `.btn-secondary:hover` (`:153–156`) in Settings, Update, Import, Invocation and NewTools: black/card → muted **1.09/1.06**; in popover, fill step **1.00** but border changes border → border-strong (**1.18**), so not wholly invisible. Most dialog footer instances sit on card. `.btn-primary:hover` uses ring mixed 12% toward foreground; actual opaque fill changes and black ink remains passing (Appendix A).
- Import `.discovered-list .group-card` (`:363–378`): card static fill; hover strengthens border (1.18), adds shadow and the existing transform. Black shadow alone would vanish, but it is not the only cue. `.found-pill` and `.badge` are opaque legacy 8%-state/background mixes; legacy warning text passes.
- Update notes `:905–918` use `--bg-hover = accent`. Inside a popover dialog, their fill is **1.00** against the body; text is still 7.65, and this is static prose with padding, not hover/current state. No semantic control vanishes.
- `--bg-element-hover` (`:14`) has **no reader**; independent faint on its true mix (muted + 6% foreground) rounds to **5.04**. An earlier exploratory calculation using a guessed different mix was discarded; the actual alias above is what this report verifies.
- Literal colours outside aliases: dormant `.modal-backdrop` rgba(17,24,39,.45), dormant `.settings-toggle-knob` black shadow, and Settings fallback literals at `:840` / `:863`. Both fallback aliases are defined (`--success`, `--danger`), so the hex fallbacks do not apply. No live JSX uses `.modal-backdrop`, `.settings-toggle`, or `.card-btn`. No literal secretly keeps a live pane on the former blue-grey dark ground.

### 4. Elevation on black — PASS, with precise exceptions to the brief's all-borders wording

| Primitive | Source evidence | Browser / independent contrast |
|---|---|---|
| Dialog (center/top) | `dialog.tsx:155–159` `border border-border-strong`; body :276 popover | **1px solid rgb(38,38,38)**. Edge/black **1.39**, edge/popover **1.28**. `/tmp/t35-dialog.png`. Sheet correctly has the **left** edge only; other sides abut the viewport. |
| CommandPalette | `command-palette.tsx:143–153` uses DialogContent top | Same 1px strong edge; `/tmp/t35-palette.png`. |
| Popover | `popover.tsx:144–146` uses `floatingSurface`, `styles.ts:63` | **1px solid rgb(23,23,23)**. Edge/black **1.17**, edge/popover **1.08**. Real Add-over-dialog capture `/tmp/t35-popover-over-dialog.png`. |
| DropdownMenu | `dropdown-menu.tsx:54–59` uses same fragment | Same 1px border. `/tmp/t35-row-menu.png` over app ground and `/tmp/t35-menu-over-dialog.png` over an actual Dialog. |
| Toast | `sonner.tsx:44–59` normal/info strong edge; state 35% tint edge | Plain **1px solid rgb(38,38,38)** (settled at top right in `/tmp/t35-tooltip.png`; `t35-toast-plain.png` catches its entry motion). Real Update-all success **1px solid oklab(0.384003 -0.0566874 0.0276651)**, not border-strong (`/tmp/t35-toast.png`). Every tested toast had an edge. |
| Tooltip | `tooltip.tsx:36` inverted foreground fill, **no border utility** | **0px** border, background rgb(245,245,245), text black. Fill/black **19.26**, fill/popover **17.71**; `/tmp/t35-tooltip.png`, `/tmp/t35-tooltip-over-dialog.png`. This is not shadow-only separation and needs no gratuitous border. |

The brief asks to prove a 1px edge for *every* primitive. Literally that is false for Tooltip, and sheet has only its exposed edge. These are safe documented exceptions, not failures. Likewise “Toast border-strong” is shorthand only for normal/info; state toasts carry their own coloured edge.

**Stacked identical panels:** the nested menu and dialog both computed #0e0e0e; the menu's #171717 perimeter was visible but quiet in the capture. Its top/left/right/lower edges are not transparent and do not disappear into the dialog. The actual Add popover similarly has a full perimeter over the dialog. No shadow-only or zero-edge same-ground floating layer was found. I do not claim that a screenshot establishes visibility on every physical display/brightness setting; the weakest decorative edge is **1.076789**, and the operator's hardware/eyes remain the appropriate final subjective check. The token choices are not reopened here.

### 5. Focus ring — PASS

`focus-ring.test.tsx` was included in the green 556-test run. The browser specimen also confirmed on **all five dark grounds**, after trusted keyboard Tab and focus of a pending button:

```text
opacity: 0.75
outline: rgb(150, 144, 255) solid 2px
matches(':focus-visible'): true
```

| Dark ground | Full ring | Ring at 50% | Focused-disabled at 75% |
|---|---:|---:|---:|
| background | 7.730110 | 2.535192 | 4.595073 |
| card | 7.546856 | 2.552744 | 4.549159 |
| popover | 7.105998 | 2.596184 | 4.434546 |
| raised | **6.136660** | 2.533714 | **4.045661** |
| row | 7.730110 | 2.535192 | 4.595073 |

Thus the claimed **4.05** dark floor is correct before rounding and safely above 3. The light floor remains 3.08829 (normally rounded 3.09; documented 3.08 differs by <0.01). No inherited `outline-none` suppression was found; menu's actual keyboard outline and palette's highlighted outline both rendered solid 2px.

### 6. Browser consumer sanity and capture provenance — PASS within the stated scope

Started only my own fixture server:

```sh
npx vite --mode fixture --port 5178 --strictPort
```

Used **ego-browser**, one task space, no Tauri and no other developer's server. Final captures listed here are **1280×800** (initial captures at the native window size were replaced). Device emulation is round-local in this browser, so it was reapplied before each final capture; `innerWidth/innerHeight` was checked. Screenshots were read with the image tool, not merely saved. Colour ratios come from token/computed CSS values, not 8-bit screenshot colour-management guesses.

| Flow/state | Capture in `/tmp` |
|---|---|
| Sidebar hover vs current | `t35-sidebar-hover.png` |
| My Skills row hover | `t35-row-hover.png` |
| Focused + selected My Skills row, inspector | `t35-row-focused-selected.png` |
| Row menu, keyboard highlight | `t35-row-menu.png` |
| Explore hover | `t35-explore-hover.png` |
| Explore preview, focused row, file reader | `t35-explore-preview.png` |
| Projects matrix hovered row | `t35-projects-hover.png` |
| Project list selected and hovered simultaneously | `t35-project-list-hover.png` |
| Add dialog, hovered ghost close | `t35-dialog.png` |
| Add's real popover over dialog | `t35-popover-over-dialog.png` |
| Add multiple candidate picker | `t35-candidates.png` |
| Re-point single candidate picker, raised hover | `t35-radio-picker.png` |
| ⌘K palette | `t35-palette.png` |
| Update-all success toast | `t35-toast.png` |
| Plain toast, strong edge | `t35-tooltip.png` (settled toast at top right); `t35-toast-plain.png` (entry motion) |
| Settings | `t35-settings.png` |
| Specimen + raised section | `t35-specimen.png`, `t35-raised-specimen.png` |
| Tooltip on black / on actual Dialog | `t35-tooltip.png`, `t35-tooltip-over-dialog.png` |
| Actual primitive composition, menu over Dialog | `t35-menu-over-dialog.png` |
| Light My Skills, settled | `t35-light-myskills.png` |

**What the images show:** the sidebar hover is very subtle but distinguishable from its black-ish pane and weaker than current; its text brightening helps. Row hover is visible, selected and inspector-focused cues coexist, and the dialog close now clearly acquires a ground. No colour-induced missing label or failed identifying control was found. Existing matrix horizontal scrolling is unchanged layout, not attributed to this colour-only diff. The light capture's ground is rgb(255,255,255), sidebar rgb(240,241,244), with the same layout; actual Theme menu was restored to Light at the end (the initial fixture preference).

Menu-over-dialog and tooltip-over-dialog were additional **in-memory compositions of the real exported primitives**, mounted by importing their Vite modules and React createRoot into a separate temporary DOM host on the specimen. They are not claimed as an existing product flow. The real Add popover-over-dialog was also tested. No source/specimen file was edited to make the test pass. Browser space was closed; the audit-owned Vite server was stopped. No real skill data was touched.

### 7. Guards and documentation sweep — PASS

Fresh output from the worktree:

```text
npm run styles:check
styles:check ok — App.css 812 counted lines (ceiling), 38 migrated file(s), prose layer scoped, no raw colours; self-test passed.

npx tsc --noEmit -p tsconfig.app.json
(exit 0; no diagnostics)

npx vitest run src/components/ui src/lib
Test Files  27 passed (27)
Tests       556 passed (556)

# Includes focus-ring.test.tsx and badge.test.tsx.
# One Node ExperimentalWarning about --localstorage-file, not a failed test.

git diff --check
(exit 0; no output)
```

The source-only Impeccable detector over the nine changed source targets returned **`[]`**. It does not supersede contrast or browser tests. No API/behaviour change, dependency change, raw JSX colour, stylesheet addition, or App.css growth was introduced. The full Rust/build/release gate was **not rerun** by this bounded read-only auditor; the implementation child's full-gate claim is not presented as my fresh proof.

Stale-number command:

```sh
rg -n '15\.50|14\.51|8\.31|7\.78|6\.12|5\.73|6\.01|6\.61|7\.08|3\.90|3\.65|5\.22' DESIGN.md
```

**No stale dark value remains in an unqualified current token-fed line.** Important distinctions:

- `:218–225` retains 6.12/5.22/5.73 in the historical round-18 regression table, explicitly dated 2026-09-29 and superseded at `:234–236`.
- `:246–247` retains 3.90/3.65/6.61 as explicitly dated old rendered measurements, immediately followed by current input/iris figures.
- `:1101–1105` retains old Explore figures in a **dated 2026-11-02** paragraph, followed at `:1106–1108` by the new dark values. It is historical, not an unnoticed stale current claim.
- 6.61 is now legitimately muted-foreground/raised and the 10% accent soft pair; 6.01 is now legitimately failed/raised. Blind replacement would corrupt correct numbers.
- Light numbers in old strip/Projects tables remain dated/superseded; this audit did not broaden ticket 35 into a light-document cleanup.

F1 is the remaining **role** discrepancy, not a bad contrast cell. DESIGN.md's generic introductory statement that all three floating primitives have borders is contradicted by its explicit Tooltip exception; the precise exception is documented above and is not a functional defect.

## Appendix A — independent float calculation and reproducible token/table checks

Run this block with `node --input-type=module` on stdin from the audited worktree. It reads current files and git HEAD only. No child calculation is imported or executed; no file is written. Intermediate arrays retain full precision, and rounding is only for printing. The extra hover-step and legacy outputs support the sweep as well as the required table comparisons.

```js
import fs from 'node:fs';
import cp from 'node:child_process';
import assert from 'node:assert/strict';
const css = fs.readFileSync('src/styles/theme.css', 'utf8');
const doc = fs.readFileSync('DESIGN.md', 'utf8');
const old = cp.execFileSync('git', ['show', 'HEAD:src/styles/theme.css'], { encoding: 'utf8' });
const blocks = s => [...s.matchAll(/:root(?:\[data-theme='light'\])?\s*\{([\s\S]*?)\n\}/g)].map(m => m[1]);
const tokens = s => Object.fromEntries([...s.matchAll(/--([\w-]+): (#[\da-f]{6});/g)].map(m => [m[1], m[2]]));
const t = tokens(blocks(css)[0]), prev = tokens(blocks(old)[0]);
assert.equal(blocks(css)[1], blocks(old)[1]);
const expected = {};
for (const [value, names] of [
  ['#000000', 'background row ok-foreground stale-foreground failed-foreground destructive-foreground'],
  ['#040404', 'card sidebar'], ['#0e0e0e', 'popover accent muted sidebar-accent'],
  ['#1e1e1e', 'raised'], ['#f5f5f5', 'foreground card-foreground popover-foreground accent-foreground sidebar-accent-foreground'],
  ['#a3a3a3', 'muted-foreground sidebar-foreground'], ['#898989', 'faint'],
  ['#171717', 'border sidebar-border'], ['#262626', 'border-strong'], ['#6b6b6b', 'input'],
]) for (const k of names.split(' ')) expected[k] = value;
assert.deepEqual(Object.keys(t).sort(), Object.keys(prev).sort());
for (const k of Object.keys(t)) assert.equal(t[k], expected[k] ?? prev[k], k);
console.log('LIGHT BLOCK BYTE IDENTICAL true; accepted changed tokens', Object.keys(expected).length);
const rgb = h => h.slice(1).match(/../g).map(x => parseInt(x, 16) / 255), c = k => rgb(t[k]);
const linear = x => x <= .04045 ? x / 12.92 : ((x + .055) / 1.055) ** 2.4;
const encoded = x => x <= .0031308 ? 12.92 * x : 1.055 * x ** (1 / 2.4) - .055;
const Y = a => a.map(linear).reduce((s, v, i) => s + v * [.2126, .7152, .0722][i], 0);
const R = (a, b) => (Math.max(Y(a), Y(b)) + .05) / (Math.min(Y(a), Y(b)) + .05);
const r = (a, b) => R(c(a), c(b));
const mat = (m, v) => m.map(row => row.reduce((s, x, i) => s + x * v[i], 0));
const lab = v => mat([
  [.2104542553, .793617785, -.0040720468], [1.9779984951, -2.428592205, .4505937099],
  [.0259040371, .7827717662, -.808675766],
], mat([
  [.4122214708, .5363325363, .0514459929], [.2119034982, .6806995451, .1073969566],
  [.0883024619, .2817188376, .6299787005],
], v.map(linear)).map(Math.cbrt));
const unlab = v => mat([
  [4.0767416621, -3.3077115913, .2309699292], [-1.2684380046, 2.6097574011, -.3413193965],
  [-.0041960863, -.7034186147, 1.707614701],
], mat([
  [1, .3963377774, .2158037573], [1, -.1055613458, -.0638541728], [1, -.0894841775, -1.291485548],
], v).map(x => x ** 3)).map(encoded);
const over = (a, b, p) => a.map((v, i) => v * p + b[i] * (1 - p));
const mix = (a, b, p) => unlab(over(lab(a), lab(b), p));
const G = ['background', 'card', 'popover', 'raised', 'row'], lines = doc.split('\n');
let n = 0; const wrong = [];
const check = (name, value, expected) => {
  n++; if (Math.abs(value - expected) > .01000001) wrong.push({ name, value, expected });
};
for (const l of lines) {
  const a = l.split('|').map(s => s.trim()), k = a[1]?.match(/^`([\w-]+)`/)?.[1];
  if (a.length === 8 && t[k] && /\d\.\d\d \/ /.test(a[5])) {
    const expected = a[5].match(/\d+\.\d+/g).map(Number);
    G.forEach((g, i) => check(k + '/' + g, r(k, g), expected[i]));
  }
}
for (const l of lines.filter(l => /^\| `(background|card|popover|row)` →/.test(l))) {
  const a = l.split('|').map(s => s.trim());
  const [x, y] = [...a[1].matchAll(/`([^`]+)`/g)].map(m => m[1]);
  check(x + ' to ' + y, r(x, y), parseFloat(a[2]));
}
const fills = doc.split('Text on the fills (')[1].split('Soft grounds')[0].split('\n')
  .filter(l => l.startsWith('| `') || l.startsWith('| …'));
for (let i = 0; i < fills.length; i++) {
  const a = fills[i].split('|'), k = ['primary', 'ok', 'stale', 'destructive'][Math.floor(i / 2)];
  const value = i % 2 ? Math.min(...G.map(g => R(c(k + '-foreground'), over(c(k), c(g), .9)))) : r(k + '-foreground', k);
  check(k + (i % 2 ? '/90' : ' solid'), value, parseFloat(a[2]));
  assert(value >= 4.5); console.log('FILL', k, i % 2 ? 90 : 100, value.toFixed(6));
}
for (const [label, p] of [['Badge', .1], ['Toast', .08]]) {
  const a = lines.find(l => l.startsWith('| ' + label + (label === 'Badge' ? ' `soft`' : ':'))).split('|');
  const ex = a[2].match(/\d+\.\d+/g).map(Number);
  ['primary', 'ok', 'stale', 'failed'].forEach((k, i) => {
    const value = R(c(k === 'primary' ? 'ring' : k), mix(c(k), c('popover'), p));
    check(label + k, value, ex[i]); assert(value >= 4.5);
    console.log('SOFT', label, k, value.toFixed(6));
  });
}
for (const hue of ['moss', 'teal', 'cyan', 'sky', 'orchid', 'rose', 'slate']) {
  const a = lines.find(l => l.startsWith('| `' + hue + '`')).split('|');
  const fg = 'identity-' + hue + '-foreground', tint = 'identity-' + hue;
  check(hue, r(fg, tint), parseFloat(a[4]));
  console.log('IDENTITY', hue, r(fg, tint).toFixed(6), r(tint, 'card').toFixed(6), r(fg, 'card').toFixed(6));
}
for (const l of lines.filter(l => l.startsWith('| `syntax-'))) {
  const a = l.split('|').map(s => s.trim()), k = a[1].slice(1, -1);
  check(k, r(k, 'raised'), parseFloat(a[4]));
}
console.log('CHECKED', n, 'DARK TABLE CELLS; MISMATCHES', JSON.stringify(wrong));
assert.equal(wrong.length, 0);
for (const k of ['foreground', 'muted-foreground', 'faint', 'ring', 'input', 'primary', 'ok', 'stale', 'failed', 'border', 'border-strong']) {
  console.log('GROUNDS', k, G.map(g => r(k, g).toFixed(6)).join(' / '));
  if (!k.startsWith('border')) for (const g of G) assert(r(k, g) >= (['ring', 'input', 'primary'].includes(k) ? 3 : 4.5));
}
for (const p of [.5, .75]) {
  const values = G.map(g => R(over(c('ring'), c(g), p), c(g)));
  console.log('DISABLED RING', p, values.map(v => v.toFixed(6)).join(' / '));
  if (p === .75) assert(values.every(v => v >= 3));
}
for (const [a, b] of [
  ['muted-foreground', 'raised'], ['background', 'muted-foreground'], ['primary-foreground', 'primary'],
  ['ring', 'raised'], ['muted-foreground', 'muted'], ['foreground', 'raised'], ['background', 'foreground'],
  ['faint', 'muted'], ['muted', 'raised'], ['border', 'popover'], ['input', 'raised'],
  ['border-strong', 'background'], ['border-strong', 'popover'], ['raised', 'popover'],
  ['sidebar-accent', 'sidebar'], ['raised', 'card'], ['foreground', 'border-strong'],
]) console.log('EXTRA', a, b, r(a, b).toFixed(6));
console.log('DISABLED ghost/popover', R(over(c('muted-foreground'), c('popover'), .5), c('popover')).toFixed(6));
console.log('LEGACY hover faint', R(c('faint'), mix(c('foreground'), c('muted'), .06)).toFixed(6));
console.log('LEGACY primary hover ink', R(c('background'), mix(c('foreground'), c('ring'), .12)).toFixed(6));
for (const k of ['primary', 'ok', 'stale', 'failed']) {
  const soft = mix(c(k), c('popover'), .1);
  console.log('SOFT HOVER STEP', k, G.map(g => R(soft, c(g)).toFixed(6)).join(' / '));
  console.log('SOLID HOVER CHANGE', k, G.map(g => R(c(k), over(c(k), c(g), .9)).toFixed(6)).join(' / '));
}
console.log('DESTRUCTIVE MENU', R(c('failed'), over(c('failed'), c('popover'), .1)).toFixed(6));
console.log('All gated current dark pairs pass.');
```

## Final verdict and hand-back

**PASS-with-findings — no P0, one P1 role correction, two P2 hover-composition notes.** The implementation achieves the accepted true-black/two-tone theme, preserves every pinned value and the light token block, and keeps text, identifying controls and focused-disabled rings above their gates. The independent numbers agree, and the exercised product row/control/menu/sidebar states retain their distinguishing cues. Before 31, move inline code from the newly split muted role to its documented raised role (F1); do not redesign the palette or reopen the accepted pane step. F2/F3 are bounded raised-parent composition notes (neutral outline has no delta, semantic ok's fill is only 1.016), not reasons to undo the global hover decision. Source was not fixed by the auditor; the orchestrator owns the correction, full gate and commit.
