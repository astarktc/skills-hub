# 02: Shared-folder facts and the cross-read warning

Status: ready-for-agent

**Blocked by:** 01 (registry structure).

**Spec:** `.scratch/round20/spec.md` § Registry structure (shared-folder read facts, facts vs `group`, the brief is the
record), § Cross-read warning, § Harnesses pane.

**What to build:** every existing harness row records which shared folders it reads, and the app warns when two selected
harnesses would make one of them see a skill twice. The Harnesses pane collapses undetected harnesses by default.

## Acceptance criteria

- [ ] **Facts on every existing row.**
  - Per scope (global, project), `~/.agents/skills` and `~/.claude/skills` are each
    `Yes | Conditional(reason) | OptIn | No | Unknown`.
  - Precedence is `Own | Shared | Both | Unknown`.
  - Duplicate behaviour is `ShowsAll | Shadows | LoadsNone | FlagsConflict | Unknown`.
  - `also_reads` holds root-relative dirs read by default; legacy, opt-in and conditional entries are marked.
  - Values come from `docs/design-inputs/2026-11-05-harness-candidates-and-shared-dir-reads.md` § Part 2, "Existing
    registry rows". LIKELY and UNKNOWN cells are encoded as `Unknown`, and a comment on each row cites its brief row.
- [ ] A **brief-snapshot table test** in `core/tests/tool_adapters.rs` restates every row's facts. A registry fact
      change without a test change fails.
- [ ] A test forbids an AGENTS-group member whose project `.agents` fact is `No`.
- [ ] **`duplicate_readers` on the wire.**
  - `ToolInfoDto` gains it: the installed harness keys whose deploy dir this harness reads by default (`Yes`, not
    legacy, opt-in or conditional) *and* that would also deploy, plus the duplicate behaviour.
  - Shared skills dir siblings (`shared_with`) are excluded.
  - It's resolved in the backend from the registry alone. The frontend re-derives nothing from dirs.
  - Tests cover:
    - Cursor + Claude Code + Codex → Cursor sees both;
    - Kimi's legacy-only reads → no pair;
    - an uninstalled harness → no pair;
    - shared-dir siblings → no pair.
- [ ] The **Harnesses pane** row shows one warning line when a pair exists with both harnesses selected.
  `DeployHarnessAsk` shows one line when turning a harness on completes a pair. The copy follows the duplicate
  behaviour: `LoadsNone` says invocation breaks, `FlagsConflict` says the harness flags a conflict, otherwise "shows
  twice". No auto-deselect. i18n `en` keys only.
- [ ] **Undetected harnesses** (reader-only included) collapse into one closed "Not detected (N)" group that resets when
      Settings opens and is not persisted. Detected harnesses keep today's order. A new primitive variant, if needed,
      is documented in DESIGN.md.
- [ ] A fixture **`registry`** scenario (`?scenario=registry`) shows detected and undetected harnesses, at least one
      warning pair, a reader-only member and a row without a project dir. The fixture-backend parity tests pass.
- [ ] `npm run version:check && npm run check` passes.

## Comments
