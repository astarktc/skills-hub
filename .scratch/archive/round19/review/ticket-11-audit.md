fix-then-pass — **P0 A1: dismissing Enable’s overwrite ask locks the app; P1 A2: group-link focus rings are clipped; P1 A3: the minimum-width row loses its name; P1 A4: strip accessibility text creates an outer scrollbar that can remove the shell/header from view.**

# Ticket 11 — D31 accessibility + design-conformance audit

2026-09-29 · Reviewed implementation **353e0fe**. **Do not accept the first-half gate as a D31 pass yet.** No application source edited. This report and the ticket’s dated Comments entry are the only tracked writes.

## Method, scope and evidence

- Read round-19 D5/D22/D23/D28/D31, round-18 D5 and the bake-off rejected list, the direction contract, DESIGN.md, ticket 11 including Result, round-18’s audit and the requested critique findings. Used Impeccable audit guidance and freshly fetched Web Interface Guidelines. Project-specific desktop minimum, 24px dense target floor, read-only strip, and hash policy take precedence over generic mobile/44px/all-state-in-URL advice.
- Reused round-18’s method: fresh observations, WCAG 2.x relative luminance, ancestor background compositing, explicit Pass/Fail/Partial, no invented screen-reader/native-WebView or unexercised-flow certification.
- Existing fixture server **5175**, never Tauri. `rich`, `failures`, `empty`, `first-run`; dark/light; **1280×800 and 960×640**. T3 preview used for independent layout/state checks and saved captures. Headless **`agent-browser --session r19-t11-audit`** supplied trusted keyboard events, focus checks, axe and the full 16-condition measurement/screenshot matrix. No synthetic KeyboardEvent was used as Tab/Space/arrow proof.
- Most repeatable headless runs use `&latency=0`, preserving the scenario but removing simulated wait time. Initial probes caught startup loading rather than the surface; the final matrix waits for startup settlement. The overwrite blocker was independently reproduced at **default latency, reduced-motion=false**, with a 1.2s post-Escape observation and remained stuck afterward.
- The concurrent ticket-09 child finished as `26f0dbf` during the audit. Its project/combined-outcome changes are excluded. `git diff 353e0fe --` on the audited surface, `OverwriteModal.tsx`, `ui/dialog.tsx`, and `useStatusReporter.ts` is empty. The only changed `useSkillActions.ts` block is Deploy-all outcome combination, not the Disable/Enable path exercised here. A1 is an existing shared-dialog integration defect reached by this surface, not attributed to that child.
- Evidence root **E = `.scratch/round19/evidence/11-audit/`** (gitignored). `measure-{scenario}-{theme}-{1280|960}.json`, matching `.png`, `axe-*.json`, `tabs-*.json`, `flow-settled-*.json`, and the named files below are fresh. `probe.js` records the compositing method. Earlier `flow-*.json` are exploratory immediate samples; use **`flow-settled-*`** for the verdict. Menus schedule focus restoration, so the final flow records after 150ms, not the first transition/frame.
- Default fixtures contain no **global stale** strip mark. To test the fifth shape rather than pretend it was observed, dynamically mounted the **actual `SyncedStrip` component** in a disposable browser-only specimen with explicit mixed/complete inputs on the three actual grounds. No source/fixture file changed. `specimen-{dark,light}.json` and `strip-five-shapes-*-grayscale.png` are supplemental component evidence, not seeded-library screenshots.
- No VoiceOver, WKWebView/WebView2, native minimum-window enforcement, physical touch, 200% zoom, or complete Add/Import/Projects audit. D31’s release-operator half remains required. No performance benchmark or whole-app WCAG certificate is implied.

## Per-clause verdict

| Clause | Result | Fresh evidence / value | Disposition |
|---|---|---|---|
| Keyboard-only traversal and activation | **Fail** | Four uninterrupted `flow-settled-*` runs: `/`, search/clear, linked Issues, chips, Grouped/Flat arrows+Space, sort menu arrows, checkbox Space, Enter → detail hash, Escape/Back → row, overflow arrows, switch Space, overwrite ask. **Escape from that ask ends on BODY with root inert**, every run. Full traversal: **126 stops in each** of four `tabs-*` runs, no unringed computed styles or center-obscured stops. | **P0 A1**. Ordinary controls are reachable; dialog dismissal prevents continued work. |
| Visible focus, both themes | **Fail** | All 126 traversal stops compute `solid 2px`; active menu ring measures **6.01 dark / 5.04 light**. But group link’s parent `h2` has `overflow:hidden`: the visible outline is almost entirely cropped (`group-link-focus{,-light}.png`). | **P1 A2**; computed outline alone does not prove painted focus. A1 also loses focus entirely. |
| AA colour pairs, both themes | **Pass for measured colour pairs; overall accessibility still fails above** | 16 baseline inventories, **0 measured enabled text pairs <4.5**; minimum **4.679** white/Add in dark, **4.887** failed badge in light. Required non-text minima **3.324 dark / 3.332 light**. Hover measurements and all five shapes included. **axe 4.10.3: zero violations in all 16 runs**, with manual contrast checks for incomplete nodes. | Do not turn zero axe violations into a D31/AA certificate: A1/A2 are independently reproduced. |
| Reduced motion | **Pass, sampled surface** | Actual media emulation `reduce=true`; list, open Sort menu, and switch after Space: **0 running animations, 0 elements with nonzero transition duration/animation** (`reduced-motion.json`). Chip/menu appearance is static; state still changes. | Release-native check remains. |
| 960×640 and structural collapse | **Fail** | Sidebar **200px at 1280/1200, 56px at 1199/960**, named icons, **36px rows**, no horizontal overflow. Header count/summary present initially. At 960, scratchpad name gets **12px in T3 / 30px headless**, full text needs **70px**. Document height **1192/1193** for an 800/640 viewport; outer scrolling can move header off-screen. | **P1 A3 + A4**. Collapse exists but does not finish the minimum-window job. |
| Contract/spec and migration | **Partial** | Healthy complete strips use quiet `input` marks and no health badge; exceptions follow name; read-only strip opens detail without changing marks; docx 7 broken+1 hollow, noisy-linter Cursor/Codex disabled, retired-playbook all disabled+switch off. Counts, singleton fold, empty/discovery priorities, Tool labels and style guard pass. | Core anatomy is correct; focus/identity/shell failures above must be fixed. Inspector/rail/palette are explicitly later tickets, not new scope here. |
| Round-18 finding closure | **Partial** | Detailed closure table below. F02/F07/F18 and P0-2’s list-state defects closed; F10’s rail and F12’s other boundaries remain assigned elsewhere; P1-7 minimum-window closure remains incomplete. | Do not mark the entire critique closed. |

## Findings and fixes

### A1 — P0 — Escape from the Enable overwrite ask leaves the application inert

**Location:** `src/components/skills/modals/OverwriteModal.tsx:51` (keyed `OverwriteAsk`, which contains `DialogContent` at :69); `src/components/ui/dialog.tsx:212–235` (layer registration and inert cleanup tied to popup unmount). Reached through `src/components/skills/SkillRow.tsx:208–218`.

**Reproduction:** fresh failures → keyboard to `frontend-design enabled` → Space (off) → Space (Enable) → overwrite ask for Gemini CLI → Escape. Four full keyboard runs reproduce it; an isolated default-latency run corroborates it.

**Observed after settlement:**

- `document.activeElement.tagName === "BODY"`, `outline: none 3px`.
- `#root.inert === true`.
- A visible 520×124 dialog remains with **`data-closed` and `data-ending-style`**, text `Folder already exists / Keep existing / Overwrite`; **both buttons disabled**.
- This is not a transient zero-time sample: still present after 1.2s and subsequent inspection. Tab does not restore an operable app. Reload is the practical escape.

Evidence: `flow-settled-{dark,light}-{1280,960}.json` last two observations; `overwrite-escape-default-latency.json`; `overwrite-escape-ghost.png`. The default-latency record explicitly has `reduce:false`.

**Impact:** cancelling an ordinary row Enable blocks subsequent operations, violates D21’s “never BODY,” keyboard completion, focus order and modal recovery. This is a shared integration defect, but it is on ticket 11’s explicitly required switch/ask path and blocks its D31 gate.

**Fix:** keep the DialogContent/popup identity stable through `pending → null`; do not reset the **whole popup** by changing its key to `none` during closure. Reset/key only the per-ask choice state/body. Verify the actual lifecycle releases all inert layers and restores the originating switch once it is enabled again. Add a regression for the disappearing pending answer/closing popup, then repeat both **Escape and explicit Keep/Overwrite** through the row path. Do not paper over it by focusing BODY or suppressing the ask.

### A2 — P1 — Repository group links have a declared ring, not a usable painted ring

**Location:** `src/components/skills/SkillsList.tsx:229–232`.

`h2.truncate` applies `overflow:hidden` around an inline anchor whose focus outline extends outside it. Trusted Tab focus on `anthropics/skills` reports:

- Anchor: x112–216.0469, y148.5–164.5 (16px high).
- Clipping h2: exactly the same x bounds, y146.5–166.5 (20px high).
- Outline: **2px solid, offset 2px**.

The left/right ring lies outside the h2; the top/bottom ring also lies outside except tiny rounded-corner fragments. `group-link-focus.png` and its light counterpart show those fragments, not the intended perimeter. Other external repository group headings use the same markup.

**Impact:** keyboard users can barely locate the focused link. Fails the visible-focus/DESIGN gate and WCAG 2.4.7 intent; the passing colour ratio is irrelevant where the ring is clipped. This specifically disproves “29/29 ringed” as a sufficient visual proof.

**Fix:** put truncation on an inner text span and leave the focus-bearing link’s outline unobstructed, or use a deliberately inset ring on a suitably padded full control. Preserve the full accessible name and external-link semantics. Recheck actual paint in both themes, not just `getComputedStyle().outlineStyle`.

### A3 — P1 — Exception badges take the minimum-window skill name away

**Location:** `src/components/skills/SkillRow.tsx:42–45`, `:143`, `:172`, `:332–336`; related breakpoint claim in DESIGN.md § Layout.

At **960×640**, the `scratchpad` name cell is 289.195px in T3. Its two nonshrinking exception badges take 269.195px, plus the 8px gap, leaving the actual name button **12px** for text that needs **70px**. It reads effectively `s…`. Headless Chromium without the outer scrollbar gives 30px, still truncating this short name. The description retains its own column while the identifying name loses to exception copy.

Evidence: `failures-dark-960-bottom-preview.png`, `measure-failures-*-960.json`, `layout.json`. The child’s Result acknowledges “only a two-mark row truncates,” but a hover title does **not** satisfy this audit’s requirement that names remain readable at the minimum window.

**Impact:** the row most in need of diagnosis loses the identifier needed to select the right skill. This is a D5/product minimum-window defect, not a blanket claim that every ellipsis violates WCAG.

**Fix:** reserve enough name width before allocating badge/description space; collapse the description earlier based on actual row-content needs or compact the exception treatment with an accessible full explanation. Keep shape/words for exceptions and the full strip; do not make text smaller or dimmer. Verify short two-exception names and representative longer names at 960 with real scrollbars.

### A4 — P1 — The strip’s sr-only count escapes the list’s scroll containment

**Location:** `src/components/skills/SyncedStrip.tsx:110–121`, especially the absolute `.sr-only` child at :119 with no positioned count wrapper.

The list itself scrolls, but the uncontained screen-reader counts create **another document scrollbar**. T3 failures: root/body 640px, document **1192px**; rich: document **2488px**. A bottom-row `scrollIntoView` moved the outer document to `scrollY=265.5`, putting the main/header at **y=−265.5**. The bottom screenshot shows the shell/header lost and a large blank region below the app.

The hidden counts retain page-relative positions (e.g. y651.5, 687.5 …) rather than staying contained by their scrolled rows. A temporary **DOM-only** experiment setting the count wrappers `position:relative` changed failures’ document height **1192 → 640** and viewport content width **945 → 960**; removing it restored the defect. No source fix was applied.

Evidence: `layout.json`, `failures-dark-960-bottom-preview.png`; independent T3 measurements above. Initial screenshots may look fine because the outer scrollbar has not yet moved.

**Impact:** ordinary outer scrolling/scroll-into-view can remove navigation, count and summary from view, while wasting width at the minimum size. This undermines the shell and D23 responsive contract.

**Fix:** contain the sr-only count in its row/count wrapper (e.g. a positioned wrapper), preserving its accessible text. Verify `documentElement.scrollHeight === innerHeight` with the list scrolling internally on rich/failures, all four theme/size combinations. Confirm that fixing the outer scrollbar does **not** count as fixing A3: the name still truncates at 960 without it.

## Keyboard transcript

**Setup only:** navigate fixture, wait for startup, set CSS viewport and theme. Then trusted agent-browser keys throughout; no `.focus()` is used in these four primary flow transcripts. Programmatic focus is used only in separately labelled isolated corroborations (A1, link screenshot, ring measurement).

`flow-settled-{dark,light}-{1280,960}.json` records active element, role/name, outline, `:focus-visible`, checked state, hash, row count and dialogs at **56 stops per run**:

| Keys | Focus / observed result (same four runs) |
|---|---|
| Tab ×7 | My Skills → Explore → Projects → Notifications → Theme → Keyboard shortcuts → Settings. Sidebar current area has `aria-current=page`. |
| `/`; type `docx`; Escape | Labelled search; one row after typing; clear back to 25 rows. |
| Shift+Tab ×4 | Add skill → Refresh all → Configure Tools → View 11 skills with issues. Count/summary text is static, correctly not another Tab stop. |
| Enter | Issues applied, **11 rows**. |
| `/`; Tab; Space | Search → All 25; All selected, **25 rows**. |
| Tab ×4 | Issues → Edited → Disabled → Grouped. |
| Right; Space; Left; Space | Flat focus/selection, then Grouped focus/selection. Roving segmented control: the other segment is reached by arrows rather than adding a redundant Tab stop. |
| Tab; Enter; Down; Down; Escape | Sort and view → Name A–Z → Most recent → Date added → trigger. Items have 2px inset iris rings. |
| Tab onward | Review 9 discovered skills → anthropics/skills → Select docx. This group’s Deploy all is disabled, so correctly skipped; full traversal reaches enabled group Deploy all controls. **The link ring is clipped (A2).** |
| Space; Space; Tab | Checkbox checked then unchecked; docx name. Marks/badges are static read-only content, not separate controls/Tab stops. |
| Enter; Escape | `#/skills/skill-004-docx`, detail Back focused → `#/skills`, docx name focused. |
| Enter; Enter | Open again, activate Back → docx name. |
| Tab; Tab; Enter; Down; Down; Escape | docx enabled → More actions → Restore — Central copy missing → Change source… → Remove → overflow trigger. |
| Tab ×3; Space | Select frontend-design → frontend-design → frontend-design enabled; checked **true → false**; focus stays on switch. |
| Space | Enable raises the one overwrite ask, Overwrite focused (2px ring). |
| Escape; settled observation | **BODY, no ring, closed ghost dialog remains, root inert (A1).** |

`tabs-{dark,light}-{1280,960}.json` independently traverses **all 126 stops from sidebar to final row overflow**. No BODY stop, no center-covered control, all computed `solid 2px`. This proves reachability, **not** that every entire ring is painted: A2 is the counterexample. Repeated per-row anatomy: checkbox → name → optional Edit pencil → switch → overflow → next row. Enabled group Deploy-all controls are included. Full browser-history navigation, every repair’s settlement and later-ticket bulk operations are not claimed as completed flows here.

## Measured pairs

WCAG 2.x sRGB relative luminance; canvas resolves computed colours including `color-mix()`, ancestor backgrounds composited before measurement. No ratios copied from DESIGN’s tables. Disabled controls are excluded; decorative separators/edges are not treated as sole identifying boundaries. Baseline inventories: rich **171/163** text nodes at 1280/960, failures **126/118**, empty **16/8**, first-run **43/35**, in each theme (sr-only rail labels explain the size delta). **0 text failures**.

### Text and controls (ratios, dark / light)

| Rendered pair | Dark | Light | Evidence |
|---|---:|---:|---|
| Skill/name foreground on app | 15.50 | 17.32 | `measure-failures-*-1280.json` |
| Description on app | 8.31 | 8.16 | same |
| Faint count/placeholder on app | 6.12 | 6.09 | same, explicit placeholder sample |
| Faint group count on card | 5.73 | 5.67 | same |
| Chip count on selected raised ground | 5.22 | 5.37 | same |
| Active chip/Grouped text on raised | 13.21 | 15.28 | same |
| Inactive chip text on app | 8.31 | 8.16 | same |
| Group heading on card | 14.51 | 16.14 | same |
| Summary link on app | 7.05 | 5.71 | same |
| Failed summary on app | 6.91 | 5.50 | same |
| Failed badge on its opaque mixed ground | 5.71 | 4.89 | same |
| Warning badge on its opaque mixed ground | 8.32 | 4.99 | same |
| Not deployed badge | 7.08 | 7.20 | same |
| Add / empty Add / first-run Review, white on primary | 4.68 | 5.86 | corresponding baseline inventories |
| Empty/first-run explanatory copy | 8.31 | 8.16 | empty/first-run inventories |
| Search/unchecked checkbox/off-switch identifying edge on app | 3.90 | 3.78 | non-text inventories |
| Selected chip iris edge on app | 7.05 | 5.71 | same |
| Switch on track vs row; thumb vs on track | 8.31 | 8.16 | same |
| Switch off thumb vs raised track | 7.08 | 7.20 | same |
| Keyboard menu ring vs highlighted ground | 6.01 | 5.04 | `menu-focus-contrast.json`, `solid 2px`, inset −2px |

### Every strip shape, including hover/selected grounds

Each cell is **app / card (row hover) / raised (selected)**. Real hover measurements on docx, tdd, noisy-linter and retired-playbook are in `hover-*.json`; the actual-component specimen supplies the absent stale variant and complete quiet marks across all grounds.

| Shape/state | Dark | Light |
|---|---|---|
| Filled synced (mixed strip) | 8.31 / 7.78 / 7.08 | 8.16 / 7.60 / 7.20 |
| Quiet filled at N/N | 3.90 / 3.65 / 3.32 | 3.78 / 3.52 / 3.33 |
| Hollow not synced | 3.90 / 3.65 / 3.32 | 3.78 / 3.52 / 3.33 |
| Half-filled stale | 10.41 / 9.75 / 8.88 | 5.62 / 5.23 / 4.95 |
| × failed/broken | 6.91 / 6.47 / 5.89 | 5.50 / 5.12 / 4.85 |
| Slashed disabled | 6.12 / 5.73 / 5.22 | 6.09 / 5.67 / 5.37 |

Grayscale captures show five distinct shapes (filled / hollow / half / × / slashed) without relying on hue. The baseline excludes missing/pending variants not seeded in these global targets; those reuse the ×/hollow shape and colour, respectively, but no backend end-to-end transition into them is claimed.

**axe:** all 16 runs zero violations, tags WCAG2A/AA, WCAG21AA, WCAG22AA and best-practice. Some failures/first-run runs return **1–2 incomplete contrast nodes** (truncated source/description overlaps); manual computed-composited pairs above pass. These are incomplete automated checks, not silently counted as automated passes.

## Contract and prior-findings closure

| Item | Disposition and evidence |
|---|---|
| F02 — healthy decoration outranks problems | **Closed for ticket 11.** Rich header 60 skills / 2 failed targets / View 1; failures 25 / 7 / View 11. Issue chip displays exactly 11 rows. Quiet complete strips (e.g. rich artifacts-builder) have no health badge and use input-colour marks, not green celebration. Discovery uses neutral card ground. |
| F07 — grid clips Tool identity | **Closed for sampled current grids.** `grid.json`: Auto and Dense × both themes × both sizes, **0 marks outside their cards, no horizontal overflow**. Labels are complete accessible names/title on read-only shapes; no cut-off Claude Code pill remains. Grid visual redesign is still 19b, not declared complete here. |
| F10 — no multi-skill operating model | **Partial by planned scope.** 36px rows, persisted checkbox selection, group Deploy all, overflow and singleton fold exist; full traversal reaches enabled group actions. **Bulk rail / select-all / j-k-x / Esc selection handling remain ticket 13**. This audit does not certify Deploy-all backend settlement (ticket 09). |
| F12 — raw Tool keys at decision boundaries | **Closed on this surface; still open cross-surface.** Baseline visible/accessibility strings scanned for `claude_code`, `kimi_cli`, `gemini_cli`, `agents_skills`: none. Strip labels include Claude Code, Kimi Code CLI, Gemini CLI, Windsurf (not detected). Original Import/Projects/notification recovery boundaries belong to their later migrations; not declared fixed here. |
| F18 — empty/discovery attention | **Closed.** Empty retains shell + Configure Tools, one heading/sentence, primary Add from GitHub, folder secondary, Explore tertiary; no filter/Refresh/global uninstall. First-run: 2 managed skills, primary Review 12 discovered, Add outline, neutral discovery copy. Both themes/sizes captured. |
| Opus P0-2 — broken looks healthy | **Closed for My Skills list presentation.** `states.json`: failures docx **7 broken × + 1 hollow, no filled mark**; failed counts exposed, no +N hiding failures; prisma-review explicitly Not deployed; stored skill failures have exception words/shapes after the name. No fabricated upstream update state. Acquisition trouble that has not yet happened is not invented as a stored failure. |
| Opus P1-7 — density/toolbar/minimum width | **Partial.** 36px rows, grouped singleton fold, compact filter/menu hierarchy and real icon-rail collapse replace the old layout. **A3/A4 prevent closing the minimum-window part.** Projects remains ticket 17. |
| D22 read-only strip | **Pass.** `strip-readonly.json`: activating docx mark opens detail/hash, Back focused, no dialog; Escape returns docx and all marks unchanged. Marks are role=img, not tool mutation buttons. |
| Disabled grains | **Pass.** noisy-linter has Cursor + Codex slashed/disabled; retired-playbook has all 8 slashed/skill-disabled and `aria-checked=false`. |
| D18 imported provenance | **Pass on rows.** Imported origin is not an extra health badge; legacy-lint/mystery-skill remain ordinary rows (description data may mention import). Inspector provenance remains ticket 12. |
| Rejected list | **Conforms.** Left sidebar, no top-rail/rack costume, no patched vocabulary, no colour-only strip. Half-fill distinguishes stale as target drift; it is not an upstream-update jack. |
| Migration/primitive use | **Pass.** `styles-check.txt`: **2316 App.css ceiling, 7 migrated files, no raw colours, self-test passed**. This guard inspects legacy classes through AST/imports, stronger than a literal grep. Manual primitive call-site review: layout-only width/margin/flex classes; no variant styling overrides. Impeccable detector `[]` (`detector.json`). |
| D28 target floor | **Pass for interactive dense primitives sampled.** Checkbox 16px visible + ±4px pseudo hit area =24px; small switch 28×16 + vertical extension =24px high; overflow/Edit 24px; sidebar ≥32px. Strip glyphs are 12px **read-only images** inside the row activation area, not 12px tool-action targets. Group-link focus clipping remains A2 regardless of target geometry. |

## Commands / tool runs

- `git show 353e0fe --stat`; `git status --short`; `git diff 353e0fe -- <audited surface/dialog files>` to distinguish concurrent work.
- `.pi/skills/impeccable/scripts/impeccable context --target src/components/skills/SkillsList.tsx`.
- `.pi/skills/impeccable/scripts/impeccable detect --json src/components/skills/{SkillsList,SkillRow,SyncedStrip,FilterBar,LibraryHeader}.tsx src/components/shell/AppSidebar.tsx` → `[]`.
- Fresh `curl` of `https://raw.githubusercontent.com/vercel-labs/web-interface-guidelines/main/command.md`.
- `agent-browser skills get core`; named-session `open`, `wait`, `set viewport`, `press`, `keyboard type`, `hover`, `eval`, `screenshot`, `set media dark reduced-motion`; Bun driver loops only orchestrated these calls and saved evidence.
- T3 `preview_open/navigate/resize/evaluate/snapshot` on the existing server; forced layout/repaint before saved captures. T3 screenshots are copied into E with `-preview` names.
- axe-core **4.10.3** injected from jsDelivr (not installed in this repo); `axe.run` in each settled headless matrix condition.
- `npm run styles:check` twice → exit 0, 2316/7; focused source/DOM inspection for labels, old toolbar path, className policy and primitive hit areas.
- **Not rerun:** full npm/Rust gate. No app source was changed in this audit; the implementation child’s full-gate claim is not presented as fresh execution here. Parent must run the normal gate after fixes.

## Acceptance / next pass

Fix **A1 first**, then **A2–A4**; keep the passing palette, strip semantics and compact hierarchy. Re-run the affected keyboard sequence through dismissal/settlement, painted group-link focus, bottom-row reading at 960 with scrollbars, and document-scroll containment on rich/failures. Then rerun the D31 matrix and normal project gate. These are narrow repair targets, not permission for unrelated redesign. In Impeccable terms: harden (A1/A2), adapt/layout (A3/A4), audit, then polish. The operator’s native/VoiceOver release smoke remains separate.

## Re-check (2026-09-29)

**Verdict: pass for ticket 11's bounded D31 fixture re-check. A1–A4 are closed.** This supersedes the opening fix-then-pass verdict for these findings; it does not certify the operator's native/VoiceOver half (spec D31, `spec.md:161–165`). Reviewed the uncommitted fixes over `2eb1684`; no application source edited, reverted, stashed or committed. Reused the existing fixture server on **5175** (PID 21504), never Tauri. Evidence root **R = `.scratch/round19/evidence/11-recheck/`**. Read the implementer's before/after A1 transcripts as comparison, but the verdict below uses fresh independent runs.

### Findings

| Finding | Status | Independent evidence |
|---|---|---|
| **A1 — inert application after Escape** | **Closed** | Named headless session `r19-11-recheck`, default latency (no URL override), ordinary motion. Fresh failures; trusted **Tab** to the originating `frontend-design enabled` switch, **Space off → Space Enable → Escape**, or **Shift+Tab → Enter** for Keep existing, **Enter** for Overwrite. No `.focus()` or synthetic keyboard dispatch in the primary driver/transcripts. Dark **1280×800**: Escape **twice on the same page**, Keep, Overwrite; light **960×640**: Escape **twice on the same page**. All **18 samples across six cycles** have `#root.inert=false`, **0** `[inert]`, **0** `[role=dialog]`, **0** `[data-closed]`, active element the originating switch with `:focus-visible`; **no BODY sample** after an answer. Every following Tab reaches **More actions for frontend-design**, Shift+Tab returns to the switch. The second asks open and dismiss normally. `a1-{escape,keep,overwrite}-dark-1280.json`, `a1-escape-light-960.json`; reproducible driver `recheck.ts`. |
| **A2 — clipped repository link ring** | **Closed** | Trusted Tab to `anthropics/skills`, both themes, 1280×800. Link box **(256,146.5)–(360.046875,166.5)**; outline **solid 2px, offset 2px**, ring bounds **(252,142.5)–(364.046875,170.5)**. Parent overflow **visible**; all **three** clipping ancestors contain the full ring. Independently inspected full screenshots and crops: the complete perimeter paints in **both** themes, not merely computed styles. `a2-{dark,light}.json`, `a2-{dark,light}.png`, `a2-{dark,light}-crop.png`. |
| **A3 — scratchpad name squeezed away** | **Closed** | Failures, dark **960×640**, grouped rows: scratchpad button width **69.78125px**, `scrollWidth=70`, `clientWidth=70`. Both exception badges remain and the complete name is visible in the bottom-of-list screenshot. `a34-failures-960.json` / `.png`. |
| **A4 — outer document scroll from sr-only counts** | **Closed** | After last-row `scrollIntoView`, failures **25 rows** and rich **60 rows**, dark **960×640**: document height **640 = innerHeight**, width **960 = clientWidth**, `scrollY=0`. Internal list: failures **1121/542px** scroll/client heights, `scrollTop=571`; rich **2417/542px**, `scrollTop=1867`. Shell/header remain visible. `a34-{failures,rich}-960.json` / `.png`; driver `layout.ts`. |

**Sampling precision:** observations are scheduled from the trusted answer key-up at nominal **0 / 300 / 1500ms**, not an invented exact timestamp. Recorded actual ranges were **0.2–6.3 / 300.5–385.2 / 1500.7–1501.9ms** (Keep/Overwrite's middle samples were timer-delayed). The switch may still be disabled at the immediate sample while the action settles, but remains focused; it is enabled by the settled sample. Escape leaves Gemini CLI **disabled**, Keep leaves it **disabled**, Overwrite makes it **synced**. Escape still emits the error toast discussed below. Setup initially met the normal startup overlay before clicking Grouped; the final layout driver waits for actual hit-test readiness rather than clicking through it.

### Regression tests and sensitivity

Fresh `npx vitest run src/components/ui/dialog.test.tsx`: **1 file, 14 tests passed**, exit 0, **1.30s** (`R/vitest.txt`). The three new tests are present at `src/components/ui/dialog.test.tsx:338–391`: Escape dismissal, explicit answer, and displacement with fresh body state. The first two assert popup removal, no inert element and exact opener focus; the third asserts the same popup DOM identity and reset per-ask state.

**Important distinction:** placing `key={pending?.id ?? "none"}` on **the test stand-in's** `DialogContent` would violate those assertions (the displacement identity assertion necessarily fails; the closure assertions target the stranded layer). This is reasoning over the assertions, **not** a mutation performed in this review. Reintroducing the key only in **production `OverwriteModal.tsx` would NOT fail these three tests**: they render their own `PendingAsk`, not `OverwriteModal`. They pin the primitive composition pattern, not the product's adherence to it. The independent browser flow is the integration regression proof here. Keep that limitation explicit; it does not negate A1's measured closure.

No full npm/Rust gate or repeat of the original axe/contrast matrix was requested/run in this bounded review; the prior audit's passing unaffected checks and the implementer's full-gate report are not misrepresented as fresh executions. Parent retains the normal pre-commit gate.

### Bounded classification recommendation (not an A1 blocker)

**(a) An unanswered collision is not an execution failure.** The observed red `data-type="error"` toast says **“Sync failed: frontend-design -> Gemini CLI”** / “Target folder already exists … It was left in place.” Recommend a **conflict / target “to review”** report item and count, optionally quiet on explicit dismissal, rather than counting it as failed. Do not convert Escape to Keep/`declined`, issue a retry, or undo targets that already succeeded. Gemini remaining disabled is compatible with preserving the prior disabled pair; the *classification/copy* is the problem.

- `AGENTS.md:289–291` distinguishes an unanswered `TARGET_EXISTS` from other failed first syncs; `:317–331`, particularly **:323–324**, says a dismissed ask sends nothing. Spec **D10 `spec.md:78–82`** separates outcome kinds and tone, while **D12 `:88–93`** owns the one per-action ask. `AGENTS.md:392–395` explicitly includes **“to review”** and independent conflict completion. Neither text expressly fixes the exact toast wording for dismissal, so quiet-vs-review is a recommendation, not a newly invented literal acceptance clause.
- Mechanism: `useSyncOrchestration.ts:483–484` returns without retries on dismissal; `reportOutcome.ts:705–767` (`foldSync`, particularly **:752–761**) then increments `failed` and makes a `syncFailedTitle` error for the retained collision. `enableOutcome` **:777–807** carries those errors and sets **`conflict:false`**; `useStatusReporter.ts:461–493` renders the error entries as error toasts. This is not just an unfortunate colour chosen by a notification component.
- The vocabulary exists in `reportOutcome.ts:135–205`: `review` counts and `headlineTone`'s conflict branch. However, **`ReviewKind` is currently only `edits | originals` (:148–152)**; a target-collision review kind/copy would need an intentional extension, not an inaccurate “Edit to review.” Conflict completion is independent of tone, but currently serves other conflict flows: do **not** blindly flip its boolean and re-open the overwrite ask after Escape. Preserve D12's one-ask/no-retry-on-dismiss rule.

**(b) Primary owner: ticket 09's door/fold/lifecycle seam, not ticket 14's rendering.** `issues/09-action-door-lifecycle-health-fold.md:9–15` owns which fold settles the action; **:21–27** owns executing the Outcome. Ticket **14 `issues/14-notifications-by-run-header-progress-cancel-focus.md:10–15`** expressly consumes **“tone from the fold”** and adds headline-only toasts / View report / grouped actionable rows, implementing spec **D11 `spec.md:83–87`**. Record the semantic fix as a 09 follow-up (building on 03's D10 vocabulary); 14 should render the corrected outcome and remove per-error toast spam, not reinterpret a failed outcome locally. This recommendation is separate from, and does not reopen, A1's dialog/focus gate.
