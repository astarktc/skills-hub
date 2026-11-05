# Ticket 25 — Spec review and adversarial audit

**Verdict: PASS-with-findings.** No P0 or P1 found; three P2 findings below. The deletion and the four migrations can proceed, with the guard limitations and fixture follow-up recorded rather than described as fully enforced/proven.

Reviewed 2026-11-04, read-only on source, in `/Users/alexstark/.t3/worktrees/skills-hub/t3-5966ed62`, branch `t3/delete-app-css`, HEAD `771358bfb123ef1ad9299f06cb2c64a743647ac2`. Review target was **`git diff HEAD`**, including the staged deletions and unstaged edits, not an empty branch/merge-base diff. No source fix, stash, commit, or tracker disposition was made. This report is the only worktree file I wrote.

Sources: ticket 25 Work, Comments and Result; changed AGENTS.md; ADR-0006 (especially §5–§6); DESIGN.md Migration rule, Dialog/ConfirmDialog, Button, Field, Toaster and last-four-dialogs section. Used the current web-design-guidelines, Impeccable audit and React review guidance, subordinated to the repo's documented contracts. As the delegated reviewer, I performed the audit myself rather than spawning reviewers.

## Spec

### (a) Missing / partial

- **Documentation overstates the new guard's coverage.** Ticket Result: “any `src/` source file whose class expressions … use one fails”; ADR §6: “any `src/` source file”. The implementation excludes tests, fixtures and bindings. See F2.
- **The strengthened import guarantee is partial.** Ticket Result: “no source file … may import an `App.css` (comments are not imports).” The ordinary positive/negative cases pass, but the new textual import recognizer has a reproducible false positive and false negative. See F1. This does **not** defeat the unconditional `src/App.css` existence check.
- Ticket Gate: “fixture walk of every surface in both themes shows no unstyled element.” I reached all named surfaces and independently rendered New harnesses without editing source. Its **startup route remains unproven**, due to the fixture/StrictMode issue F3. The same-content overwrite note was source-reviewed, not encountered through the normal collision flow; exact before/after AX-tree identity was not independently recorded on main. These limits are not silently credited as passing tests.
- The requested warning-free build criterion is not met: the build exits 0 but prints two warnings (details under Gates). The ticket itself requires a passing build/check, not a warning-free bundle.

### (b) Scope creep

None found. The four live legacy surfaces had to move before deletion. Operator Comment explicitly authorizes deleting Layout and Dashboard. Removing their translations and sole router dependency is consequent cleanup. The Tailwind source restriction implements the ticket's 2026-10-02 Comment: “narrow irrelevant source scanning … and keep useful utilities.” The accepted frozen-name baseline and Toaster relocation stay within the requested closure. No backend, action-door, collision-answer, or updater-lifecycle changes were smuggled into the diff.

### (c) Implemented but wrong

F1's recognizer and F2's advertised coverage are the only new implementation/contract discrepancies I reproduced. I found no wrong rendering, changed batch answer shape, broken invocation save, or update-button focus regression in the four dialogs. Ticket Work's core requirement — “Every remaining `App.css` rule is either migrated … or deleted with the alias block” — is satisfied by the source audit and generated-class check below.

## Findings

### F1 — P2: import detection parses comments and imports with regex, not TS syntax

**Location:** `scripts/check-styles.mjs:72–84`, especially line 80.

In an external temporary copy (never in the worktree):

```ts
// Harmless source, but guard exits 1: [legacy] imports an App.css
export const example = 'import "./App.css"';

// Real import, but guard exits 0: // inside the URL erases the rest of the line
const url = "https://example.com"; import "./App.css";
```

The claim “comments are not imports” works for the tested comment, but string contents are not comments or import declarations either. A recreated `src/App.css` still fails independently, so this is a guard correctness/maintainability defect, not a current runtime regression or a way to restore that file undetected.

**Exact fix:** use the already-imported TypeScript parser to recognize static import/export module specifiers and literal `import()`/`require()` arguments in TS/TSX; inspect the actual module string for the App.css basename/query instead of stripping comments from text. Add both examples above to `check-styles.test.mjs`, requiring exits 0 and 1 respectively. Keep the CSS/HTML paths separate and retain the existing positive cases.

### F2 — P2: rule 2 inherits raw-colour exclusions while the new docs say every source file

**Location:** `scripts/check-styles.mjs:50,610–612,635–636`; `DESIGN.md:1454–1457`; ADR-0006 §6 at lines 209–212.

The shared `sources` list excludes `*.test.ts[x]`, `src/fixtures/**` and `src/bindings/**`. External-copy reproduction:

```tsx
// src/fixtures/probe.tsx — guard exits 0
export const x = <div className="btn-primary" />;
```

The same code under `src/probe.tsx` correctly exits 1. The docs and success message therefore describe a broader invariant than the checker provides. The exclusions predated this change for the raw-colour rule; reusing them for the newly universal retired-class rule is the discrepancy.

**Exact fix:** form an unfiltered TS/TSX source list for rule 2, and apply `SOURCE_SKIP` only to the raw-colour rule, with a regression case under `src/fixtures/`. If exclusions are deliberately retained instead, name them explicitly in the guard header, DESIGN.md, ADR note and ticket Result; do not say “any src source file.” Scanning all TS/TSX for rule 2 is the stronger fit to the accepted contract.

### F3 — P2: New harnesses has a fixture/StrictMode reachability hole

**Location:** `src/fixtures/backend.ts:1572–1575`, `src/hooks/useToolCatalog.ts` startup effect, `src/main.tsx` StrictMode roots; unchanged by this ticket.

`toolStatus()` derives `newly_installed` and immediately sets `state.seenTools`. StrictMode invokes the startup effect twice; the second result is empty. The fixture test at `backend.test.ts:449–453` deliberately pins the backend's once-only behavior. In `first-run`, the app displayed the discovery review but never the New harnesses announcement. This is a **development coverage hole**, not evidence that the production dialog or the backend's once-only semantics are wrong.

I independently imported the actual `NewToolsModal.tsx` and existing React/i18n modules through the running Vite page, mounted it into a temporary browser DOM root, and supplied controlled props (`Cursor, Pi`, not loading). Both themes and both viewport sizes rendered correctly; Escape returned to the focused Add skill button. The temporary root was unmounted. No fixture/source line was commented out.

**Exact follow-up:** add a BACKLOG item for StrictMode-safe newly-detected-harness coverage. Deduplicate the catalog's mount-time detection request across StrictMode effect replay (retaining the result), with a hook-level StrictMode regression proving the first non-empty result is not lost; alternatively provide an explicit fixture-only announcement scenario/driver. Preserve the backend's “reported once” test. Suggested backlog text: “Make New harnesses reachable in the fixture under StrictMode without source edits; retain once-only detection semantics; prove announcement, Later and Deploy entry points.” Do not change `toolStatus()` to report new tools forever just to obtain screenshots.

## Evidence — guard honesty and deletion

Independent mutations ran in `/var/folders/6p/j8hvzsjx4wjf__r2jbgbff980000gn/T/t25-guard-1WCJAY`, copied from the reviewed source and with its own probe files. No probe was created under the worktree.

| Probe | Exit | Result |
|---|---:|---|
| Recreate `src/App.css` | 1 | `[legacy] src/App.css exists`, also rejected as a fourth stylesheet |
| `import "./App.css"`, file absent | 1 | `[legacy]` import rejection |
| Comment containing `import "./App.css"` | 0 | Correctly ignored |
| JSX `className="btn-primary"` | 1 | `[retired]` |
| i18n-like value `{ en: { button: "btn-primary" } }` in ordinary source | 0 | No non-class false positive |
| `it("btn-primary", () => {})` in ordinary source | 0 | No non-class false positive, even without test-file exclusion |
| Import-looking string / URL before real import | 1 / 0 | F1 |
| Retired JSX class under `src/fixtures/` | 0 | F2 |

The committed self-test suite independently passed **26/26**. It covers recreated file, TS import with absent file, CSS import, HTML link, comment acceptance, class-expression indirection and non-class retired words. It does not cover F1 or F2.

Baseline keys are exactly `$comment`, `retiredClasses`, `rawColourExempt`; no `appCssCeiling` or `migrated`. I extracted the selectors from `HEAD:src/App.css` using the guard's CSS-rule reader: **44 old names, 44 frozen names, no missing or extra name**. `rawColourExempt` is empty. Only the permanent three stylesheets remain. The deleted App.css import is gone from App.tsx.

`src/components/Layout.tsx`, `src/pages/Dashboard.tsx`, and the `src/pages/` directory are absent. Searches of `src`, `package.json` and `package-lock.json` found no `react-router`, `layout.*` or `dashboard.*` use. The diff removes only the scaffold's nine translation leaves and router-related dependency entries. `version:check` reports **1.3.0**, and the lockfile diff does not change either root version entry. The 1.3.1 doc references describe the planned release, not a version bump in this ticket.

`npm ci --dry-run --ignore-scripts` exits 0. `package-lock.json` SHA-1 before and after: `9fb38fbb39ba3148fbbf630112fb8ed75392d717`. This proves dry-run compatibility and no lockfile mutation, not a fresh physical reinstall.

CHANGELOG has the Unreleased “The last legacy styles are gone” entry. ADR §6 has the **2026-11-04** deletion note. AGENTS.md and DESIGN.md correctly describe the permanent stylesheet set and no-resurrection goal, subject to F1/F2's precision issues. ADR's older numbered ratchet rules remain historical context and are explicitly superseded by the dated note.

## Evidence — browser and interaction

Own Vite fixture server: **port 5202**, never a live Tauri run. Ego task space was closed after review. Own captures, AX snapshots and computed-style samples are under **`/tmp/t25-browser/`** (`.png`, `.ax`, `.json`). These are reviewer evidence, not the child's screenshots. Early captures taken before asynchronous UI settlement were superseded where needed; use the explicit filenames below. In particular, use `add-result-*-1280`, not the earlier unsuffixed Add-result files.

Themes were switched through emulated `prefers-color-scheme` and awaited until the settings world's `data-theme` matched. Every authoritative capture records `innerWidth/innerHeight`; viewport overrides were reapplied in each browser call. At least a few controls/headings/paragraphs per surface were sampled with `getComputedStyle`. The aggregate samples use the system font stack, token-derived colours and the expected 12/13/16 px typography, with no initial/Times fallback. Transparent secondary buttons are intentional, not unstyled controls.

| Surface | Both themes, 1280×800 | Both themes, 960×640 | Evidence / interaction |
|---|---|---|---|
| My Skills list + grid | Yes | Not requested for the list | `list-*`, `grid-*`; styled rows/cards and deployment marks |
| Explore | Yes | — | `explore-*`; populated Featured list |
| Projects list + populated matrix | Yes | — | `projects-*`, `matrix-*` (matrix recaptured after the table loaded) |
| ProjectToolsModal | Yes | Yes | `project-tools-*-1280`, `project-tools-*-960`; named Configure harnesses, Escape → opener at stable width; resizing away the opener correctly falls back to main |
| Settings, all five panes | Yes | Yes | `settings-{General,Harnesses,Appearance,Updates,Diagnostics}-*`; Escape → Settings |
| Keyboard shortcuts | Yes | Yes | `shortcuts-*`; Escape → Keyboard shortcuts |
| ⌘K | Yes | Yes | `palette-*`; named combobox, Escape → opener |
| Row menu | Yes | — | `row-menu-*`; Edit invocation entry used, menu-trigger focus return verified |
| Inspector | Yes | Yes | `inspector-*`, `inspector-*-960`; 960 sheet, styled deployments/source |
| Toast with two-line description | Yes | — | `toast-*`; actual Sonner toast through the existing mounted Toaster; computed **`white-space: pre-wrap`**, newline preserved |
| Add Source / Skills / Result | Yes | Yes | `add-source-*`, `add-skills-*`, `add-result-*-1280` / `*-960`; `failures` + `acme/collide-kit` gives settled “Added 3 of 4”, named Retry and target failures; Escape → Add skill |
| Import review (`first-run`) | Yes | Yes | `import-review-*`, `import-review-*-960`; actual review, not the discovery banner; Escape → Review 12 discovered skills |
| Invocation, including conflict | Yes | Yes | `invocation-*`, `invocation-conflict-*-{1280,960}`; details below |
| Overwrite ask | Yes | Yes | `overwrite-*-{1280,960}`, `overwrite-batch-*-960`; actual global collision and actual multi-candidate Add collision |
| Update dialog | Yes | Yes | `update-*`, `update-*-960`; install identity and dismiss/ignore verified |
| New harnesses | Yes, controlled runtime render | Yes, controlled runtime render | `newtools-runtime-*-{1280,960}`; actual component, not startup-route proof (F3) |

No unstyled element was found in these checks. At 960, the conflict dialog's long body and Import/Add results scroll inside the shell; footer controls remain visible. The overwrite batch's paths wrap on their own lines and its controls are not clipped.

**Console evidence limit:** monitored passes (console.error plus window error/unhandledrejection) for Settings' five panes, shortcuts, palette, Add picker, import review and the runtime New harnesses render reported no errors. Error instrumentation was not continuously installed across every earlier navigation, so the early capture JSON's empty fallback arrays are not a complete console log for every surface. No visible app crash or error overlay occurred. This is a logging coverage limit, not a claimed universal “zero console errors” proof.

### Invocation semantics and the action door

- CDP's full AX tree names the native fieldset **“Who can invoke this skill?”** from its legend, and describes it with the Claude-only note. Ego's compact snapshot prints a bare group; I checked the actual AX name rather than treating that rendering as an accessibility failure.
- Four explicit modes are named by their mode label and separately described by their explanatory sentence; Follow the skill's own setting has its self-contained label and no separate sentence.
- With the checked Model only radio focused, ArrowDown moved focus **and checked state** to Not invocable. Tab then landed on Cancel, not another radio: one radio-group Tab stop. The visible keyboard ring is present.
- Cancel and Escape return to the row's More actions trigger. Changing to User only and saving closed the dialog, returned focus to that trigger, and reopening showed **User only checked**.
- No direct `invokeTauri` in InvocationModeModal. App.tsx:876 calls `run("editInvocation", …, { mode })`; `useSkillActions.ts:690–710` owns `setSkillInvocationOverride`, catalog replacement and collision settlement.
- Conflict banner remains `role="status"`, words plus aria-hidden TriangleAlert. Measured actual text/ground: dark `rgb(251,172,22)` on `rgb(14,14,14)` = **10.125:1**; light `rgb(152,87,0)` on white = **5.678:1**. Both pass AA normal text.

### Overwrite and Update

- Single overwrite: AX dialog **“Folder already exists”**, skill → harness, path on a separate line, “Modified 3d ago · differs from the skill”, Keep existing / Overwrite.
- Batch overwrite: actual `failures` → Add `acme/collide-kit` produced two rows. Each Keep/Overwrite pair sits in a group named for its skill → harness. At 960×640 both rows and the footer are readable. Keep existing settled the two pairs as kept in Add's result. `src/lib/overwriteAsk.ts` and its tests are unchanged; focused run **9/9 green**. The `sameContent` fact branch remains in RowFacts and is passed through unchanged; no actual same-content ask was forced (normal identical content is silently replaced).
- Update: AX **“Update available.”**, header/body/footer, Markdown under `.prose`, scroll-capped notes. In `?update=1.4.2&latency=0.5`, saved a reference to the Update Now DOM button: it was still `document.activeElement` while **Installing... / aria-disabled=true** and still the same connected element at **Done**. Done calls dismiss; Escape returns to the sidebar item. Don't remind again removed the announcement and closed the dialog; focus fell back to main because the opener was removed. `useAppUpdate` remains unchanged and owns both callbacks.
- Main comparison: reviewed the original hunks in `git diff HEAD`, and used the child's before images only as historical hints. Independent current captures confirm token radios and standard footer buttons, separated overwrite paths, proper release-note Markdown and standard New harnesses buttons. The four dialog names match the ticket's claimed names. I did not stash or run a second main checkout, so this is **not** a fresh paired-main AX/screenshot regression run.

## Evidence — Tailwind scope

Production CSS is **54,079 bytes** (build prints 54.07 kB, gzip 10.61 kB). Collected **490 unique class candidates from 129 production TS/TSX files** with the guard's AST reader and checked them against selectors parsed from the **built CSS**, not just dev CSS. Only `chip` and `peer` have no standalone class selector: `chip` is a Badge variant value picked up by conservative AST traversal (`badge.tsx:38,92`), and `peer` is Tailwind's relationship marker. Neither is a lost styling rule. Result: `/tmp/t25-css-audit.json`.

`index.html` contains no class attributes; its pre-paint script sets theme state, not class utilities. Restricting Tailwind to `src/` therefore loses no HTML utility candidate. Built `.transition`, `.transition-colors` and `.ease-in-out` are absent; useful explicit opacity/transform motion is retained. The ticket's “`.transition*`” cleanup must not be read as banning all useful transition utilities.

I verified the resulting class coverage and byte count, **not independently the child's exact intermediate 62,307 → 54,079 / 94-removed comparison**. That intermediate pre-source-restriction build is not the main source state. Impeccable's source detector over the four dialogs and Toaster returned `[]`.

## Gates

Fresh commands in the reviewed worktree:

| Command | Result |
|---|---|
| `npm run styles:check` | Exit 0; absent App.css, 44 frozen names unused within scanned scope, self-test passed |
| `node --test scripts/check-styles.test.mjs` | Exit 0; **26/26** |
| `npx vitest run src` | Exit 0; **56 files, 1,250 tests** |
| `npx vitest run src/lib/overwriteAsk.test.ts --reporter=verbose` | Exit 0; **9/9** |
| `npx tsc --noEmit -p tsconfig.app.json` | Exit 0 |
| `npm run lint` | Exit 0 |
| `npm run build` | Exit 0; warnings below |
| `git diff --check HEAD` | Exit 0 |
| `npm run version:check` | Exit 0; **1.3.0** |
| `npm ci --dry-run --ignore-scripts` | Exit 0; lock hash unchanged |

Logs: `/tmp/t25-{styles,guard,tests,overwrite-tests,tsc,lint,build,ci}.log`. Vitest emits Node's experimental localStorage warning. Build emits **INEFFECTIVE_DYNAMIC_IMPORT** for Tauri core's dynamic/static import combination, and the **>500 kB chunk warning** (main JS 1,726.34 kB, gzip 559.27 kB). Do not describe this build as warning-free; these warnings are not a test failure. Their owning source/config was not changed here, but I did not run a fresh main build to establish a before-warning baseline.

Active LSP probe of the four dialogs plus Toaster: zero diagnostics, four confirmed clean and one unconfirmed push-only result; the full fresh TypeScript command is the independent complete type-check evidence. No Rust source changed; I did not rerun Rust gates or independently certify the child's full `npm run check` claim.

**Final verdict: PASS-with-findings.** The user's goal is achieved: App.css and the dormant scaffold are gone, the four remaining dialogs now use the app's primitives/tokens, generated production CSS covers the app's class candidates, invocation saves through the action door, overwrite decisions retain their shape, update installation retains focus, and all requested frontend gates pass. The three nonblocking follow-ups are syntax-honest import detection, truthful retired-class scan coverage, and a StrictMode-safe New harnesses fixture route. Keep the build-warning and browser-evidence limits above attached to the result rather than upgrading them into proof that was not collected.
