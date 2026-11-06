# Round 20 tickets 05–07 (new harness rows) — adversarial review

## Round 1 — GPT-6 Astra (high), 2026-11-06, `efaf49a..7ba9783`: **fix-then-ship**

1. **P1, reader-only rows bypassed the skills-only-footprint rule.** A bare `.fx/skills/x` (likewise for Warp, Dexto,
   VT Code, DimCode and Corust) detected the reader and made the AGENTS group installed (`reader_installed_in`).
2. **P2, Kimchi's cross-agent reads were project-first fallbacks, yet were encoded as `Default`.** The global
   `~/.agents` read was overstated the same way.
3. **P2, fx's broken-invocation warning vanished when `.agents/skills` was unselected.** It should still warn when
   Claude Code and Codex are both selected, because fx reads both folders.
4. **P3, the fixture showed Deep Agents Code as symlink-capable.**

**Sound:** more than 20 rows' dirs were spot-checked against vendor sources. Other checks that passed:

- crow-cli's `.agents/crow` is its own root.
- Cortex, Posit, Codewhale and Forge avoid the hazard roots.
- Deep Agents copy mode.
- The Reasonix Windows override.
- AstrBot and WorkBuddy have no project scope.
- Letta, Muse, Poolside and Dirac are in the project AGENTS group.
- oh-my-pi `Own` and MiniMax `Shared` are sourced.

## Disposition

All four were fixed by the orchestrator in PR #18. Each regression test was written first and failed before the fix.

1. `ReaderOnlyHarness.skills_dirs` (never deployed to) feeds the footprint rule. Test
   `a_skills_only_footprint_never_detects_a_reader_only_harness`.
2. Kimchi's other-agent reads and its global `~/.agents` read become `Conditional`; Pi stays `Default`. The brief row
   is updated. Tests: the snapshot, and the extended `kimchi_reads_an_installed_agents_folder_but_not_claudes_by_default`.
3. When its entry is unselected, a detected reader-only harness pairs among the selected folders it reads (`among`
   pairs, new `duplicates.among.*` copy). Test: `without its group entry, pairs the reader among two selected folders it reads`.
   Residual: an *installed but unselected* adapter, such as Cursor with Claude Code and Codex selected, still doesn't
   warn. The spec's rule requires the reader to be selected. That belongs to BACKLOG, for #109's folder-and-reader model.
4. Fixture rows carry `supportsSymlink`. Test: `carries each harness's link-vs-copy capability`.

Gate green (Rust 963, vitest 1,352); CI green.
