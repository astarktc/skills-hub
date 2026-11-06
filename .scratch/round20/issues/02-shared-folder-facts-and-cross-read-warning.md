# 02: Shared-folder facts and the cross-read warning

Status: done — efaf49a

**Blocked by:** 01 (registry structure).

**Spec:** `.scratch/round20/spec.md` § Registry structure (shared-folder read facts, facts vs `group`, the brief is the
record), § Cross-read warning, § Harnesses pane.

**What to build:** every existing harness row records which shared folders it reads, and the app warns when two selected
harnesses would make one of them see a skill twice. The Harnesses pane collapses undetected harnesses by default.

## Acceptance criteria

- [x] **Facts on every existing row.**
  - Per scope (global, project), `~/.agents/skills` and `~/.claude/skills` are each
    `Yes | Conditional(reason) | OptIn | No | Unknown`.
  - Precedence is `Own | Shared | Both | Unknown`.
  - Duplicate behaviour is `ShowsAll | Shadows | LoadsNone | FlagsConflict | Unknown`.
  - `also_reads` holds root-relative dirs read by default; legacy, opt-in and conditional entries are marked.
  - Values come from `docs/design-inputs/2026-11-05-harness-candidates-and-shared-dir-reads.md` § Part 2, "Existing
    registry rows". LIKELY and UNKNOWN cells are encoded as `Unknown`, and a comment on each row cites its brief row.
- [x] A **brief-snapshot table test** in `core/tests/tool_adapters.rs` restates every row's facts. A registry fact
      change without a test change fails.
- [x] A test forbids an AGENTS-group member whose project `.agents` fact is `No`.
- [x] **`duplicate_readers` on the wire.**
  - `ToolInfoDto` gains it: the installed harness keys whose deploy dir this harness reads by default (`Yes`, not
    legacy, opt-in or conditional) *and* that would also deploy, plus the duplicate behaviour.
  - Shared skills dir siblings (`shared_with`) are excluded.
  - It's resolved in the backend from the registry alone. The frontend re-derives nothing from dirs.
  - Tests cover:
    - Cursor + Claude Code + Codex → Cursor sees both;
    - Kimi's legacy-only reads → no pair;
    - an uninstalled harness → no pair;
    - shared-dir siblings → no pair.
- [x] The **Harnesses pane** row shows one warning line when a pair exists with both harnesses selected.
  `DeployHarnessAsk` shows one line when turning a harness on completes a pair. The copy follows the duplicate
  behaviour: `LoadsNone` says invocation breaks, `FlagsConflict` says the harness flags a conflict, otherwise "shows
  twice". No auto-deselect. i18n `en` keys only.
- [x] **Undetected harnesses** (reader-only included) collapse into one closed "Not detected (N)" group that resets when
      Settings opens and is not persisted. Detected harnesses keep today's order. A new primitive variant, if needed,
      is documented in DESIGN.md.
- [x] A fixture **`registry`** scenario (`?scenario=registry`) shows detected and undetected harnesses, at least one
      warning pair, a reader-only member and a row without a project dir. The fixture-backend parity tests pass.
- [x] `npm run version:check && npm run check` passes.

## Comments

- 2026-11-06 (implementer, branch `t3/shared-folder-facts`, draft PR #14): all criteria built; gate passes.
  Shape notes for 05–07:
  - Facts are `ToolAdapter::reads: ReadFacts` (`core/tool_adapters/reads.rs`). A row's own deploy dir is read by
    default unless an `also_reads` entry at the same location says otherwise. That is how Kimi (own dir `Legacy`) and
    Kilo (`.kilocode` `Unknown`) are kept out of pairs.
  - `ToolInfoDto.duplicate_readers` sits on the reader R: `{ keys, behaviour }`, where keys are the other installed
    entries whose dir R reads by default. The frontend (`src/lib/harnessPresentation.ts`) inverts it for the read side.
  - For reader-only rows (fx `LoadsNone`, ticket 05), give `ReaderOnlyHarness` a `reads` too. Resolve its pairs with
    its own deploy entry being the group entry (`agents_skills`), and carry them on `ReaderHarnessDto`. The pane fold
    then has to treat a reader-only pair as "group entry + other both selected".
  - Deviations:
    - Cursor and Copilot have global `.agents` = `Yes`. The brief's I1 suggests a condition, but the Part 2 cell is
      `Y` and the A-only caveat is about Cloud surfaces, not this read.
    - Stray (selected but undetected) harnesses sit in the Not detected group, which shows an "N selected" badge, so
      rows never jump when switched.
    - The fixture's WorkBuddy row is a stand-in until 05.
- 2026-11-06 (orchestrator): review round 1 (GPT-6 Astra, high) fix-then-ship, no P1. The orchestrator fixed all three
  findings in `50ebdbc`:
  - P2: Remove from selection now hands focus to the row's disclosure before it unmounts. Verified in the fixture with
    agent-browser: focus lands on Goose's disclosure.
  - P2: Xum's precedence is now `Own`, the harness fact.
  - P3: the warning names a folder shared by two selected harnesses once, via `shared_with`. Regression
    `names one folder once when two selected harnesses share it`.

  CI is green. Review record: `.scratch/reviews/round20-t02-shared-folder-facts-review.md`.

