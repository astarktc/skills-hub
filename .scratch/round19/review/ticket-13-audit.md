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

## Re-check (2026-09-30)

**FIX-THEN-PASS on `98212ef451d96b99eb04e2ded31beb6d3bcc88c9`. A1, A2 and A4 are closed. A3's launch hazard is fixed, but its managed-resource admission is not fully closed: R1 (P1) below. R2 is a P2 input-normalization defect.** No new reproduced keyboard, focus, or modal-layer failure.

### Scope and method

Read the originating Acceptance, ticket Comments, complete application diff of `98212ef`, the selection/inspector/Dialog seams, Delete settlement/planning, platform adapter, generated error binding and EN descriptions. Standards and spec were checked in this terminal reviewer, without further delegation. The tree and HEAD were unchanged before the two permitted documentation appends. No application source edits, revert, stash, commit, Tauri launch, native opener invocation, or operator-library/DB operations.

**E = `.scratch/round19/evidence/13-recheck/`**, gitignored. Fixture HTTP 5175 was already listening (200). One isolated headless session, `r19-13-recheck`, closed afterward; the pre-existing fixture server was left alone. Read and copied the fix child's drivers, changing only their evidence/session destinations; every keyboard input used **`keys-cdp.ts`**, never agent-browser `press`, native key codes, DOM focus, or synthetic keyboard events. All four `keys-*` runs completed without retries. Added independently asserted `extra.ts` for an expanded reader, a focusable reader descendant, dialog isolation, listener inventory and reduced motion. Its one inserted reader link is explicitly supplemental DOM setup, reached with trusted Tab; the rest of the primary flows use the seeded fixture unchanged. `verify-evidence.ts` asserts the completed outputs, document continuity, selection, switches, focus destinations and axe results (`verified-summary.txt`: **ALL RECHECK EVIDENCE ASSERTIONS PASSED**).

### A1–A4 dispositions

| Finding | Re-check | Evidence and reasoning |
| --- | --- | --- |
| **A1 — reader shortcut isolation** | **Closed** | `keys-{dark,light}-1280.json`: selected and unselected reader retains focus and library selection under x/j/k///⌘A. Native text selection grows **0 → 2714** characters with local-notes selected (**0 → 2683** unselected). `extra-{dark,light}.json` repeats inside expanded Files and on a reader descendant; all **14 recorded keydowns per run are trusted**. Reader Escape clears selection while retaining the reader, then collapses to **Expand files**, then closes to **local-notes**. Keys inside the Keyboard shortcuts dialog above expanded Files leave the library alone. Hook tests cover the reader, its descendant, inspector heading and sidebar, including unprevented native ⌘A. |
| **A2 — covered-list Escape** | **Closed** | `keys-*` in dark/light at **1280×800 and 960×640**: desktop without selection collapses → closes; with selection clears → collapses → closes. Row keys cannot reach the covered list. At 960, expanded sheet collapses → closes to row → clears background selection on the next Escape. Child Keyboard shortcuts above expanded desktop Files closes alone; Change source above the side inspector and above the sheet closes alone. No ordinary child-dialog trigger is reachable while the 960 sheet's Files are expanded (other sections are removed and the sidebar is inert), so no fabricated primary flow is claimed for that combination. |
| **A3 — safe, admitted Reveal** | **Not fully closed** | `reveal_path` now delegates admission/resolution to core and calls **only `reveal_item_in_dir`**. Existing `.app`/executable ancestors are selected, never sent to `open_path`; arbitrary outside paths are refused. Real-store temp-dir tests cover an owned global target and project assignment; failed removal keeps both records, so their Reveal paths remain admitted. But the central-prefix branch admits an outside file through an intermediate symlink (**R1**), and malformed/trailing-separator handling has **R2**. |
| **A4 — Delete CANCELLED** | **Closed** | `deleteOutcome` excludes cancellation from failures and error entries; a cancellation alone is neutral and closes the dialog. Fold tests assert single, mixed-success and mixed-failure counts/tones/closure, and distinguish a pre-plan failure from zero failed targets. The door test exercises an actual mocked command rejection through the real fold/reporter and checks neutral tone, no error history and closed dialog. These assert the changed behavior, not merely invocation. No action Cancel affordance or token capability was added; existing confirmation Cancel is unchanged. This remains a defensive contract, not a claim that current Delete is operator-cancellable. |

**One semantic keyboard owner, not literally one listener of any kind.** CDP `getEventListeners(window)` in both extra runs finds the list handler once, the unrelated zoom handler, App's capture-only interaction marker, and this probe's recording observer. The list handler's `layerOpen()`/consumed guard steps aside for dialogs and menus; Dialog's top-layer check and `interceptEscape` retain ownership of the sheet. No shared primitive changed. The inspector Tab bridge is also re-proved: **More actions for local-notes → H2 local-notes**, in both themes.

### Remaining findings

#### R1 — P1 — Central-repo prefix admission can reveal an unrecorded outside file

**Location:** `src-tauri/src/core/log_reveal.rs:78–82` (`path.starts_with(&central)`), then `path_reveal_item:90–94`; command `src-tauri/src/commands/mod.rs:289–292`.

**Fresh safe reproduction:** `E/reveal-probe.rs` includes the actual production `log_reveal.rs` verbatim, stubbing only its integration dependencies (the expansion helper matches `environment.rs`). In a disposable temp tree, create `home/.skillshub/escape` as a directory symlink to `outside`, and create the audit sentinel `outside/private.txt`. With **no known targets**, request `home/.skillshub/escape/private.txt`. Admission succeeds, `symlink_metadata` follows the intermediate link, and the resolver returns that pathname to be revealed. `reveal-probe.txt` proves its canonical destination is the outside sentinel (`outside=true`). Nothing was opened or executed. This is a demonstrated **resource-admission bypass**, not the old launch exploit, shell injection, or a claim of a remote exploit.

A lexical prefix does not establish that the selected descendant is a managed central resource. The existing dangling-link test covers a **leaf** link only; it does not cover traversal through one. Keeping the safe reveal-only operation is necessary but does not make the new admission claim true.

**Concrete fix:** for D16, the narrowest door is an exact app-recorded artifact/central-copy identity; it does not need arbitrary central descendants. If central-descendant Reveal remains a deliberate feature, validate the selected item's **ancestor traversal** against the resolved central root before passing it to the opener. Preserve selection of the final symlink itself (including a dangling link), and handle a configured central root that is itself a symlink; simply canonicalizing/rejecting every leaf link is not the fix. Add real temp-dir tests for an outside-pointing intermediate link, an in-root link, a symlinked root, missing descendants and a dangling leaf. Retain the exact recorded-target branch and reveal-only native operation.

#### R2 — P2 — “Plain absolute” does not reject malformed input or normalize leaf separators

**Location:** `src-tauri/src/core/log_reveal.rs:73–82,131–139`.

The same pure probe demonstrates:

- `~/.skillshub/bad\0name` is **admitted**. Its stat fails, and the fallback selects the central folder rather than returning `notManaged`. Invalid input is misreported as a legitimate vanished item.
- `~/.skillshub/./file` is admitted despite the stated “no `.`” policy: Rust `Path::components()` normalizes interior `.` before the check. This alone does not escape the root.
- A trailing slash is accepted, but the original string is returned. On Unix, `symlink_metadata(link/)` follows a directory link (`is_symlink=false` versus true without the slash). A **dangling leaf** with `/` falls back to the central folder instead of selecting the link; the bare spelling correctly selects it.

**Concrete fix:** reject NUL before any filesystem probe; make dot handling explicitly reject raw dot segments or explicitly normalize/document them; normalize harmless trailing separators before admission/probing so equivalent accepted paths keep leaf-link semantics. Preserve `..` refusal after `~` expansion (both tested forms are already refused). Add these cases to the real tests. This is lower severity than R1 and does not recreate executable launch.

### Verdicts on the implementer's four questions

1. **Recorded owned targets + central resources is a reasonable intended boundary, but not the current lexical implementation.** Project assignments are included via Delete's own `resolve_assignment_artifact` planner, not a second naming rule. A random path under a Tool's skills directory is correctly refused, as are never-owned markers. After a partial Delete, **successful** rows are no longer recorded and are not admitted; that is correct because `DeleteModal` offers Reveal only for **failed/kept** targets, whose owning rows survive under ADR-0002. If a later action removes that record, refusing the stale report path is preferable to giving arbitrary old strings permanent authority. Do not widen admission to whole Tool directories to fix this nonexistent report mismatch. Fix R1, or narrow central admission to recorded identities.
2. **Selecting the nearest existing ancestor is acceptable; do not require `gone` for every vanished leaf.** It gives the operator a useful filesystem location and, crucially, never invokes a default app handler. A recorded path below an `.app` or executable ancestor now selects that ancestor safely. `gone` means no selectable ancestor, not merely a missing leaf. On a real mounted filesystem `/Volumes` or `/` can remain, so the all-probes-false `gone` test is a failure-policy test, not proof that an ordinary missing volume necessarily produces `gone`. Malformed input must be refused before fallback (R2).
3. **Accept the narrower list-key scope, including inspector buttons and sidebar.** The signature contract says **row navigation**, not global next/previous while operating on an inspector control. The operator can return through the established Tab bridge/Escape route. Preserving a local control's context and native reader selection is safer than silently retargeting the library; no requirement demands inspector-local j/k.
4. **Accept sheet-first Escape.** A modal sheet owns the keyboard; mutating selection in the inert background first would violate that boundary. Expanded sheet → collapse → close → clear selection once back in the list is intentional responsive behavior, not a defect to equalize with the nonmodal desktop inspector. It matches the original Acceptance's explicit sheet qualification.

### Fresh flow, accessibility and gate evidence

- `gate-dark-1280.json` and `gate-light-960.json`: **92 sampled stops each**, no BODY or document reset; three rows → Disable (Cancel initially focused) → all eight marks disabled per row → Refresh leaves them disabled → Enable re-syncs. Subsequent select-all confirmation and independent state total agree: **25 skills / 8 Tools / 80 targets / 12 assignments / 1 already off** (post-Refresh, not the pristine 63-target state).
- `delete-dark-1280.json` and `delete-light-960.json`: **35 sampled stops each**, no BODY or document reset; locked release-train → report H2, named kept global/project paths, fixture Reveal records the global path; explicitly supplemental fixture unlock → trusted Retry → dialog gone, skill absent, **retired-playbook row** focused. This checks the UI/platform wiring, not native admission (which is checked separately above). Total primary sampled stops: **254, zero BODY**; no claim about unsampled micro-intervals.
- **Four axe 4.10.3 runs**: failures/dark/1280 and failures/light/960, each rail + count dialog, **zero violations**. Incompletes: rail color-contrast (6 dark / 13 light), dialog aria-hidden-focus (2 each). Rendered enabled text minima: rail **5.22 dark / 4.95 light**; confirmation **4.68 dark / 4.95 light**. `extra-*` reduced-motion first frame: opacity 1, translate none, transition none/0s, non-inert. Inspected fresh `rail-dark-1280.png` and `delete-report-light-960.png`; detector over changed UI returns `[]`. No whole-app or native accessibility certification is inferred.
- `cargo test log_reveal`: **14 passed** (10 Reveal admission/selection tests, four log-folder cases), opener not invoked. `cargo test export_bindings`: **1 passed / 803 filtered**. **`git diff --stat -- src/bindings/index.ts` empty** (`binding-drift.txt`, zero bytes). The committed binding's only semantic wire change is `REVEAL_PATH_FAILED.reason` and its exported three-value type; generated documentation changes accompany it. Command arguments remain `revealPath(path: string)`. All three reason branches and EN keys are present (`opener` retains existing `revealPath.failed`); description tests pass.
- Focused frontend suite: **4 files / 245 tests passed** (`frontend-targeted.txt`). Additional `cargo test --all`: **804 passed** (`cargo-all.txt`).

Fresh command `npm run version:check && npm run check`, `E/gate.txt`:

```text
Version OK (1.2.18)
styles:check ok — App.css 2076 counted lines (ceiling), 24 migrated file(s)
Test Files  40 passed (40)
Tests       864 passed (864)
tsc -b && vite build: passed (built in 2.04s)
cargo fmt --check: passed
cargo clippy -- -D warnings: passed
cargo test: 804 passed; 0 failed
RECHECK_GATE_EXIT=0
```

### Acceptance

**FIX-THEN-PASS — ticket 13's full D31 acceptance remains ungranted solely on the Reveal admission boundary (R1); R2 is a documented P2 companion fix.** The affected keyboard/selection/Delete fixture scope passes, A1/A2/A4 are closed, and the original executable-opening branch is gone. This is not a fresh whole-product audit or a reopening of accepted BACKLOG #66, ticket 14's progress-overlay debt, or ticket 17's fold/CSS handoff.

The next bounded check needs only the corrected Reveal admission/normalization rule, real temp-dir adversarial cases above, safe `.app`/executable/dangling/vanished handling, typed error/binding drift, and the normal gate. Do not repeat the UI matrix unless the fix changes UI or keyboard/dialog ownership. The orchestrator must retain this fix gate (recommended) or explicitly decide that central admission is only a lexical namespace hint, not managed-resource confinement; the present docs must not claim the latter without enforcing it. Operator VoiceOver/native release verification remains outstanding as before.

### Residuals closed (2026-09-30, orchestrator)

- **R1**: a central admission is now a lexical claim only — `admit_reveal_path` answers `(path, RevealAdmission::{RecordedTarget, CentralRepo})`, and `resolve_reveal_item` requires `central_contains` for a central item: the item is the root itself, or its **parent** canonicalises under the canonical root (the command passes `fs::canonicalize`). The leaf is never canonicalised, so a leaf link — dangling included — is selected as the link; an intermediate link that leaves the repo is `NotManaged`; a symlinked root contains its children by either spelling; a root that is not there is `Gone`. A recorded target stays an exact identity with no containment step. Tests: the pure `central_contains` matrix (escape link, leaf link, dangling leaf, symlinked root, missing root) and a store-backed run (escape → `NotManaged`; `escape/not-there` → the link itself selected; dangling selected; root gone → `Gone`).
- **R2**: NUL refused before any probe; a `.` / `..` segment refused **as written** (`has_dot_segment` over the raw string, since `Path::components` drops interior `.`); the admitted path is normalised through `components()` so a trailing separator is dropped and a leaf link keeps its link semantics under the presence probe. Tests added.
- Gate: `npm run version:check && npm run check` exit 0 — vitest 864, cargo 807 (17 `log_reveal`), App.css 2076 / 24 migrated, bindings unchanged. **D31 gate passed** for ticket 13. The four verdicts stand: recorded targets + physically-contained central descendants are the boundary; a vanished path selects its nearest existing ancestor (revealed, never opened); sidebar/inspector controls do not reach the row keys; the sheet closes before the background selection clears.
