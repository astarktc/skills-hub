# Round 20 ticket 02 (shared-folder facts and cross-read warning) — adversarial review

## Round 1 — GPT-6 Astra (high), 2026-11-06, `22fdbbf`: **fix-then-ship**

1. **P2: Remove from selection lost keyboard focus.** `HarnessesPane.tsx:314–326` unmounted the focused button, and
   focus fell to the dialog.
2. **P2: Xum's precedence was recorded relative to the legacy deploy dir.** It was `Shared`, but the brief says the
   harness's own `~/.xum` wins.
3. **P3: a folder shared by two selected targets was named as two folders.** Crush with Amp and Kimi read "the Amp
   and Kimi Code CLI folders".

**Checked and sound:**

- The facts of more than 15 sampled rows match the brief.
- The snapshot test restates independent literals.
- Warning pairs: Cursor with Claude Code and Codex, uninstalled and unselected harnesses excluded, and shared-dir
  siblings.
- The warning reads the effective selection.
- Disclosure ARIA and the Settings reset.
- The IPC types are specta-derived.
- No deployment or sync change.

**Deviations:** accepted, except removing the old focus code, which was challenged (#1). The Antigravity/Gemini pair
stays suppressed until #108 reconciles that dir.

## Disposition

All three were fixed by the orchestrator in `50ebdbc`:

1. Focus goes to the row's disclosure before Remove unmounts. Checked in the fixture with agent-browser: focus
   lands on `goose`'s disclosure.
2. Xum is `Precedence::Own`. The snapshot test failed before the test edit.
3. `DuplicatePair.folder` comes from `shared_with`, and a folder is named once ("Amp / Kimi"). The regression test was
   red first.

The gate is green (vitest 1,344, Rust 946) and so is CI. Merged as `efaf49a` (PR #14).
