# Ticket 13 — independent D31 audit of `c393037`

**2026-09-30 · FIX-THEN-PASS. Three P1 findings (A1–A3), one P2 defensive-contract note (A4); no reproduced P0.** The main selection/removal flows and full build gate pass. D31 does not pass because reader shortcut isolation and desktop Files Escape fail; the new native Reveal command also needs a safety fix.

## Method and evidence

**E = `.scratch/round19/evidence/13-audit/`** (gitignored). Audited `c393037342694aefe8afd45d06f364c7f7031bdd`, initially clean; no application source edits, revert, stash or commit. Only tracked writes: this report and ticket 13's Comments. Read AGENTS.md's full Invariants/Ambiguity resolution, BACKLOG/newest handoff, ticket Work/Result/Comments, spec and signature contract, ticket 12 audit/re-check, DESIGN, requested diff (`E/requested.diff`), and the additional settlement/fixture/native seams. Applied Impeccable's technical audit, fresh Web Interface Guidelines (`web-guidelines.md`), React guidance, and separate standards/spec checks in this terminal reviewer; no further delegation.

- Reused the listening fixture server **5175**, never Tauri. Headless **agent-browser 0.31.1**, main session `r19-13-audit`, explicitly named isolated suffix sessions for parallel matrices. Default fixture latency, `failures`/`rich`, dark/light, **1280×800 / 960×640**. Each final scenario navigation has a unique query nonce: opening the identical URL in this CLI can retain the previous in-memory fixture. Final evidence is the completed named JSON, not failed exploratory setup attempts.
- Adapted/read the implementer's saved drivers and ran them **anew** into E. `gate.ts` / `delete.ts` provide uninterrupted trusted-key flows; `adversarial.ts` / `edges.ts` add independent negative cases; `reader-light.ts` is the fresh-session light reader confirmation after the longer light driver timed out. Primary transcripts have **no CLI focus, DOM `.focus()`, or synthetic keyboard events**. JS is used for observation and explicitly supplemental fixture state/count reads, unlock, pure-fold input, and rAF measurement. Application focus calls are, of course, the behavior under test.
- `extras.ts` + `cdp.ts` independently prove range selection through Chromium **Input.dispatchMouseEvent**, `modifiers:8`, on the browser launched by agent-browser. The recorded originating click has **`isTrusted:true, shiftKey:true`**. Base UI subsequently emits its own untrusted input click; that is not our test input.
- `a11y.ts`: **eight axe runs** (rich/failures × dark/light × rail/count dialog), with both viewport sizes represented. Rendered RGB/background-composited WCAG luminance measurements for rail/selected rows/count dialogs, plus motion samples. Captures inspected include `rail-dark-960.png`, `selected-focused-dark.png`, `delete-report-light-960.png`.
- Native safety evidence is a **pure Rust probe of the actual module**, not execution of an opener (`reveal-proof.rs/txt`). No app or script was launched. No VoiceOver/native WebView/native-minimum-window certification; D31's operator release half remains required.

## Per-clause verdict

| Clause | Verdict | Fresh evidence / qualification |
| --- | --- | --- |
| Keyboard three-row selection → Disable → Refresh → Enable, four theme/size combinations | **Pass** | `gate-{dark,light}-{1280,960}.json`: **86 stops each / 344 total, zero sampled BODY**. `j x j x j x` selects alias-skill, api-style, corp-conventions. Confirm focuses Cancel. Five targets on three Tools; zero project assignments. Every strip mark disabled after Disable **and Refresh**; Enable re-syncs the stored rows. Rail controls retain focus on settlement. |
| Select-all / filtered select-all / counts | **Pass for observed scopes** | `adversarial-dark.json`, `gate-*`: fresh library **25 skills / 8 Tools / 63 targets / 12 assignments / 1 already off**. After the gate's Refresh/re-assert and Enable, **80 targets**, other counts unchanged. Both agree with independent state totals and displayed counts; 80 is **not** the pristine fixture total. Filter `re` selects exactly its eight shown ids, with **19 targets / 7 Tools / 4 assignments / 1 off**. |
| Checkbox, group checkbox, Shift range | **Pass** | `extras-{dark,light}.json`: trusted Shift-click api-style→design-tokens selects exactly four consecutive rows. mattpocock/skills group selects four; unchecking one makes the group mixed. |
| Delete locked release-train → report → Reveal → Retry | **Pass in fixture** | `delete-*`: **31 stops each / 124 total, zero sampled BODY**. Confirm lists Claude Code/Codex/Pi and monorepo. Remove samples 0/300/2000 ms: **Remove / Retry (pending) / report H2**. Report names removed Codex/Pi and the two denied paths. Reveal records `/Users/alex/.claude/skills/release-train`. Supplemental unlock then trusted Retry: **Retry / Retry / retired-playbook row**; skill absent in fixture catalog, dialog gone/root released. |
| Only-copy / Edits lost / bulk Remove | **Pass** | `edges-{1280,960}.json`: imported legacy-lint gets the only-copy warning; api-style gets invocation Edit lost. `extras-*`: pr-review + prisma-review + locked release-train → **“1 of 3 skills was not deleted”**, removed names and kept paths; heading focused, Close returns to rail Remove. |
| Single row switch | **Pass for seeded case** | `edges-*`: Disable api-style opens no ask, switch keeps focus at all three samples. `extras-*`: retired-playbook Enable asks **2 Tools**, Escape returns to switch, acceptance enables it. This count matches its two stored disabled global rows, not the eight strip slots. |
| Keymap isolation | **Fail A1** | Inputs/search and count dialogs retain focus/selection under j/k/x///⌘A; search Escape clears its own query. In the desktop reader, x changes selection, ⌘A selects the library, / steals focus and j exits to the row. |
| Escape hierarchy | **Fail A2** | Desktop selection clears first, retaining inspector. Once Files expand, **Escape does nothing twice**. Sheet Files correctly collapse then close; a sheet's own Escape closes it without clearing behind-sheet selection (consistent with the modal keyboard boundary, not proof of the desktop hierarchy). |
| Selected + focused row / inspector Tab bridge | **Pass** | `adversarial-dark.json`, `reader-light.json` and `selected-focused-*`: checked box, 3px leading iris edge, `aria-current=true`, 1px inset iris ring coexist. Trusted Tab from local-notes overflow goes to inspector H2 despite the checkbox. |
| Rail order, semantics, motion | **Pass with explicit verdict below** | Named group + live count, DOM before rows and visually at foot. CSS 120ms opacity/translate; reduced first frame settled, non-inert. |
| Door / surface / batch / report architecture | **Pass except documented #66 and A4** | All rail verbs use the door; one dialog union and per-surface selection; one Sync batch and consolidated ask; Skills scopes for multi-skill Disable/Enable; Delete loops under one lifecycle and folds once. CANCELLED is **not** handled neutrally by the Delete fold. |
| Reveal registration / errors / bindings | **Structural pass; safety fail A3** | Both attributes, collect_commands, generated binding, error conversion/description/i18n exist. Regeneration has zero diff. Path admission is not restricted to known resources. |
| Fixture isolation / production exclusion | **Pass** | Production grep has no `__fixture`, `skills-hub-fixture-backend`, `lockedPaths`, or fixture installer. Fixture platform/command adapters operate on memory; unexpected native calls reject. Browser preferences/caches and audit files are not a claim of zero OS disk writes; **operator skill library/app DB are untouched**. |
| Styling / i18n / gate | **Pass** | No raw colours/new stylesheet, inspected primitive className props layout-only; documented primitive additions; ceiling **2076**, 24 migrated files. Literal EN-key test, TS build and full gate pass. |

## Findings

### A1 — P1 — List shortcuts operate on the library while the reader has focus

**Locations:** `src/components/skills/useSkillListKeymap.ts:5–7,119–171`; `src/App.tsx:446–452`.

**Reproduce:** failures, 1280, dark or light → select local-notes → Enter → Tab through inspector to the focusable **SKILL.md region**. Press x: selected local-notes becomes unselected without leaving the reader. Press ⌘A: **25 skills selected**, rather than native text selection. Press /: focus jumps to search. In a separate dark reader arrival, j jumps to local-notes' row. `adversarial-dark.json` and `reader-light.json` record each active element and selection.

**Cause/impact:** `isEditable` recognizes only editable controls, not the inspector reader; the window handler substitutes `focusedSkillId` when focus is outside a row. Reading/copying can silently widen the next bulk operation to the entire library. Violates this audit's explicit reader-isolation contract and the separation of reading from list selection. This is not a BODY-focus issue.

**Concrete fix:** scope list selection/navigation shortcuts to their intended surface/context and explicitly exclude reader/document descendants. Preserve native text selection. Handle Escape separately so reader isolation does not remove the inspector's legitimate collapse/close command. Add hook tests with a focused reader region and its descendants, then trusted desktop-reader regressions for j/k/x///⌘A. Suggested targeted pass: `/impeccable harden`.

### A2 — P1 — Expanding desktop Files unregisters the only Escape handler

**Locations:** `src/App.tsx:309,441–451`; `src/components/skills/useSkillListKeymap.ts:119,130–138`.

**Reproduce:** failures, 1280 → local-notes → Expand files → Escape → Escape. Focus stays **Collapse files**, hash stays on local-notes, Files remain expanded. `edges-1280.json` has both stops. At 960 the same sequence correctly returns to Expand files then the row (`edges-960.json`).

**Cause/impact:** the newly consolidated keymap is enabled only with **`!filesCover`**; desktop expansion sets `filesCover=true`. Its callback contains the collapse logic but is no longer subscribed. This regresses ticket 12's repaired desktop Escape contract. There is a Collapse button workaround, so not P0, but the required keyboard hierarchy fails.

**Concrete fix:** retain an Escape path while Files cover the list, while refusing row shortcuts against that inert list. Preserve top-layer precedence and one owner for each key (do not add competing window handlers). Recheck selected/nonselected, expanded/collapsed, side/sheet, and a child dialog above each. Add a hook/integration-seam test for the disabled-row-navigation/enabled-Escape combination.

### A3 — P1 — “Reveal” accepts arbitrary paths and its ancestor fallback can launch an app/file

**Locations:** `src-tauri/src/commands/mod.rs:270–291`; `src-tauri/src/core/log_reveal.rs:54–68`.

**What it actually validates:** `PathBuf::from(path)` and whether **any** requested item/ancestor has successful `symlink_metadata`. It does **not** check a known Sync target, central path, report identity, allowed root, absolute path, or that the fallback is a directory; it does not expand `~`. The comment explicitly accepts any path.

**Safe reproduction:** `reveal-proof.rs` imports the actual pure module and supplies a controlled presence probe. `/Applications/Calculator.app/not-a-real-child` with the bundle as existing ancestor returns `selected:None`, folder `/Applications/Calculator.app`; `/tmp/audit.command/not-a-real-child` similarly chooses the `.command` file. Both enter the command's **`opener.open_path`** branch. A direct outside-library existing path is also accepted. **No native opener was run.** The same module already documents why macOS `open` of `.app` launches a bundle and avoids that in `log_reveal_target`; the new fallback omits that protection.

**Impact:** the exposed frontend command is broader than “show a reported target in its file manager”; a fabricated/stale child path can turn Reveal into native open/launch. This is a demonstrated unsafe branch, **not** a claim of shell interpolation, arbitrary command-string injection, or a reproduced remote exploit.

**Concrete fix:** accept/resolve a known app resource identity at the command seam (or validate the requested path against known target/central records with the repository's path rules). Keep the fallback filesystem-only and **reveal/select the existing ancestor rather than opening it with its default handler**; never treat an arbitrary existing file as a folder. Preserve dangling-link selection and typed REVEAL_PATH_FAILED. Add tests for outside-root/unknown paths, relative/`~` admission, missing children of `.app`/executable files, dangling links and vanished paths. Keep opener execution mocked in tests.

### A4 — P2 — Delete's defensive CANCELLED classification does not satisfy the requested check

**Locations:** `src/hooks/useSkillActions.ts:662–673`; `src/lib/reportOutcome.ts:1046–1069,1110–1118`.

The loop catches all thrown commands and continues. A supplemental call of the **real** `deleteOutcome` with `{error:{code:"CANCELLED"}}` (`edges-*.json`) produces a failure-tone toast and an error entry with empty message; it does not classify cancellation neutrally. A single failure even reads a kept-target count of zero. **Current Delete exposes no Cancel and its backend is not token-taking (D20), so no ordinary operator cancellation path was reproduced.** Do not claim “CANCELLED honoured”; this is a defensive/future-contract note, not another release blocker. Normalize it distinctly if the seam can return it, and add a fold/door test without introducing a fake Cancel button. The multi-command API compromise remains accepted BACKLOG #66.

## Verdicts on the five implementer concerns

1. **Shift-click: independently proved, limitation resolved for the audit.** Held Shift in the CLI click still records false; direct CDP Input events with explicit modifier yield a trusted shift-click and the correct range in both themes. Evidence includes the true originating event and Base UI's separate synthetic follow-up.
2. **Rail reading order: acceptable; it need not move after the rows.** A selection-action group logically preceding the items it controls does not corrupt meaning (WCAG 1.3.2) or the keyboard task sequence (2.4.3). It gives a bounded Tab route rather than requiring traversal of every row. **Actual implementation is `role="group"`, accessible name “Selected skills”, with a separate live “N selected” status—not the `role="toolbar"` / count-bearing name suggested in the brief.** This is valid grouping; adding toolbar semantics without arrow-key behavior would not improve it. The signature contract requires the visual foot/rise and working selection verbs, not a mandatory DOM-after position. No formal VoiceOver certification is claimed.
3. **Legacy progress overlay: inherited debt, not a new reproduced focus/inert regression.** Bulk Disable still shows **“Installing Skills…”** and blocks browsing, contrary to final D19; ticket 14 owns removal. Across the four main flows it takes focus while running, releases the root, and returns to the initiating rail control, including now-inapplicable Disable. No sampled BODY or stuck inert state. Delete avoids that overlay. Keep the existing ticket-14 compatibility-removal handoff; a passing ticket 13 does not close D19.
4. **Project toast/CSS handoff: confirmed.** Ticket 17 Comments, **2026-09-30**, explicitly names `bulkUnassign`/project-removal count-line work and `RemoveProjectModal` as the last legacy delete-* user. Ceiling remains 2076; no unrelated Projects migration was made here.
5. **Fixture hook: excluded from production and memory-only.** `vite.config.ts` replaces the fixture flag with `0` outside fixture mode. Fresh production output contains no marker/global/backend strings (`production-and-drift.txt`). Reveal appends only to `state.revealed`; unlock changed only fixture arrays. No native library/DB/keychain action was used.

## Counts, contrast, motion and accessibility detail

**Hand count, pristine failures:** materialised, non-disabled target counts in fixture order are
`7+6+5+7+2+1+5+2+2+1+1+5+2+1+3+2+1+2+1+0+2+0+2+1+2 = 63`.
The union is eight Tool ids. Non-disabled assignments are `4(tdd)+1(worktrees)+2(frontend-design)+1(docx)+1(pr-review)+1(design-tokens)+2(release-train)=12`. Only retired-playbook is skill-grain off. This independently matches `bulkCounts.disableCounts` and the initial confirm. The three first rows are `1+2+2=5` targets on Claude Code/Codex/Cursor. `adversarial-dark.json` saves every row's contribution, not just an asserted total. The later **80** is the correctly recomputed **post-Refresh** state in `gate-*`; the assertion is state-dependent.

**Inline Enable count verdict:** retired-playbook has disabled Claude Code and Codex rows and no project assignments. Backend `enable_skill` re-syncs disabled stored rows, not every effective Tool slot. **“2 Tools” is correct and preferable to misleading “8 Tools”.** The eight slashed marks express skill-grain off everywhere, not eight prior deployments. This verdict is for that seeded case, not certification that every possible project-only-disabled selection is representable in the rail.

**Axe 4.10.3:** eight runs, **zero violations**. Rail runs retain color-contrast incompletes (6 in failures, 17 rich); count dialogs have two aria-hidden-focus incompletes from Base UI guards. Trusted Tab stays in the count dialog. Axe does not detect A1/A2.

| Measured enabled pairs | Dark | Light |
| --- | ---: | ---: |
| Lowest text in sampled rail + selected rows | 5.22 | 4.95 failures / 5.12 rich |
| Count dialog primary fill/text floor | 4.68 | ≥4.95 across sampled scopes |
| Selected edge on raised | 6.01 | 5.04 |
| Checked box on raised | 3.49 | 5.04 |

No sampled enabled text below 4.5 or state mark below 3. The selected/focused inset ring uses the same ring/raised pair. Disabled controls and decorative structural hairlines are exempt, not counted as independent identifying boundaries. These are scoped measurements, not a whole-app conformance claim.

**Motion:** fresh `extras-*` rAF traces start opacity 0/translate 8px, computed duration **0.12s**, only opacity/translate. Last partial frames at approximately **111ms dark / 107ms light**, first settled samples at 127/123ms (frame sampling brackets the 120ms transition; not a claim of a 127ms CSS duration). `edges-*` under reduce: **first sample opacity 1, translate none, duration 0s, transition none, inert=false**. No delay, inert or pointer gate is attached to rail entry; removal is immediate.

## Architecture and command-safety review

- `BulkRail` resolves surface selection via App and calls `actions.run(verb, selected)` for all six verbs. Counts are pure tested folds; no command loop in components. `confirmCount` is one union member, opened by the door and confirmed through the same door with `confirmed:true`; Sync adds the chosen Tools. Assignment's existing picker was widened to skill ids, not duplicated.
- Sync goes once through `invokeBatchSync` → typed `syncSkillsToTools`; `settleCollisionParts` supplies the consolidated overwrite ask/retry routing. Multi-skill Disable/Enable use `DisableScope::Skills`. No new frontend target fan-out. Fresh door tests cover a single initial batch, same-content rule, confirmation, and refusal of a second mutation.
- Delete's per-skill loop is under **one runAction**, supplies `{kind:"skills",ids}` pending/progress, collects thrown per-skill errors, invokes **one deleteOutcome** and one applyOutcome/reload/history settlement. Its report is stored on the existing deleteSkill dialog via updateDialog before reload. It is **not one backend batch**; accepted #66, with the partial-command-report limitations already recorded there. A4 qualifies cancellation.
- Selection remains per surface in useSurfaceState, reconciled against removed catalog ids through useFocusReconciliation. `focusAfter` handles removed-id sets; the actual retry lands on the next surviving row. Row focus/inspector and checkbox selection coexist independently.
- `reveal_path` has both Tauri/Specta attributes and is in `collect_commands!` (`lib.rs:59`); generated `commands.revealPath` and REVEAL_PATH_FAILED exist; conversion, `describeCommandError` and EN copy agree. `platform.revealPath` is the only native UI door used here. **Generation correctness does not mitigate A3.**
- `cargo test export_bindings`: **1 passed / 797 filtered**; `git diff --stat -- src/bindings/index.ts` empty. Fixture/backend types and adapters match. No hand-mirrored wire enum.
- ConfirmDialog initialFocus and PopoverContent anchor are documented in DESIGN and primitive tests pass. Inspected call-site primitive classes are layout-only; guard passes, detector returns no findings. No source refactoring requested beyond the fixes above.

## Fresh gate output

Executed `npm run version:check && npm run check` myself; complete logs **`gate.txt` / `gate-stderr.txt`**, runner returned **AUDIT_GATE_EXIT=0**:

```text
Version OK (1.2.18)
styles:check ok — App.css 2076 counted lines (ceiling), 24 migrated file(s), prose layer scoped, no raw colours; self-test passed.
Test Files  40 passed (40)
Tests       853 passed (853)
tsc -b && vite build: passed (built in 1.89s)
cargo fmt --check: passed
cargo clippy -- -D warnings: passed
test result: ok. 798 passed; 0 failed
AUDIT_GATE_EXIT=0
```

Also independently ran **cargo test --all: 798 passed / 0 failed**, exit 0 (`cargo-all.txt`). No application edit was made to obtain green results. Browser findings remain despite the passing hook/unit/build gate.

## Acceptance

**FIX-THEN-PASS — D31 not passed.** Scope-specific Impeccable assessment: accessibility **2/4** (reader scope/Escape defects), performance **3/4** (no benchmark claim), responsive **3/4** (main matrix passes; desktop/sheet keyboard divergence), theming **4/4**, implementation integrity **2/4** (native Reveal admission) = **14/20**. The visual system is coherent and product-specific; the blocking findings are behavior/safety, not a request for redesign.

**Exact bounded re-check:**

1. **A1/A2:** trusted keys from the reader and reader descendants, dark/light desktop; native ⌘A remains text selection, j/k/x///⌘A do not change library focus/selection. Verify Escape clear-selection → collapse Files → close inspector where the list owns selection, and sheet collapse → close without breaking modal isolation. Include a child dialog above expanded Files. Ensure no key can focus an inert row.
2. **A3:** tests for known-resource admission and safe reveal-only fallback, including outside/unknown/relative paths, `~` policy, `.app`, ordinary executable-file ancestor, dangling link and vanished target. Mock opener; no app/script execution in the test. Regenerate bindings if signature/types change; verify typed error copy and fixture Reveal/Retry path.
3. Repeat affected four-condition keyboard/Delete flows, eight axe states if shared keyboard/dialog primitives change, reduced-motion first frame and inspector Tab bridge. Rerun version+check and binding drift. Keep #66, ticket 14 and ticket 17 handoffs; explicitly accept or fix P2 A4 rather than claiming cancellation works.
4. Operator VoiceOver/native release half remains outstanding as designed. Suggested work: targeted `/impeccable harden`, bounded audit re-check, then `/impeccable polish` only if the repairs change presentation.
