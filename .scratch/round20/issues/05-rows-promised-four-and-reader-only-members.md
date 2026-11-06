# 05: Rows I — the promised four and the reader-only members

Status: ready-for-agent

**Blocked by:** 02 (facts and warning). The rows carry facts, and the reader-only rows need 01's kind.

**Spec:** `.scratch/round20/spec.md` § Registry structure; the brief (`docs/design-inputs/2026-11-05-harness-candidates-and-shared-dir-reads.md`)
§ Part 1 and § Part 2 "The four #92 rows" / "Recommended Part 1 additions"; the addendum
(`docs/design-inputs/2026-11-05-harness-candidates-acp-addendum.md`) for DimCode, crow-cli, Corust Agent and VT Code.

**What to build:** the four harnesses CHANGELOG 1.3.1 promised become real rows, and nine harnesses that only read
`~/.agents/skills` appear as reader-only members of the AGENTS group.

## Rows

- **Own rows:**
  - DeepSeek Harness (`deepseek_harness`, `.dsh/skills`);
  - ZCode (`zcode`, `.zcode/skills`);
  - Codewhale (`codewhale`, `.codewhale/skills`; never detected on `.deepseek`);
  - WorkBuddy (`workbuddy`, `.workbuddy/skills`, **no project dir**, facts `Unknown`).
- **Reader-only:** Warp, Zed, Dexto, fx, Sarvam Code, VT Code, DimCode, crow-cli, Corust Agent.

Take each row's keys, detect dirs and project dirs from its brief row. Round 19's draft
(`archive/round19/issues/34-…md`, the table near :116) is reference only.

## Acceptance criteria

- [ ] **Re-verified first.**
  - Before adding a row, re-check its global dir, detect dir, project dir and shared-read facts against the brief's
    cited source; a web fetch is fine.
  - Where the source no longer holds, record the evidence in this ticket's Comments, then adjust the row, or leave it
    out and report back.
  - A detect dir must be the tool's own config root, never a skills-only footprint or a shared convention dir.
- [ ] **Each row** has:
  - its `ToolId` variant and `as_key` arm;
  - its `TOOL_ADAPTERS` literal with read facts, citing the brief row;
  - a README supported-tools table row;
  - rows in `project_relative_skills_dir_for_every_tool` and the brief-snapshot test;
  - an i18n `tools.<key>` label.
- [ ] Zed's Windows detect root (`%APPDATA%\Zed`) uses 01's override.
- [ ] fx's row says `LoadsNone`: its warning copy must say invocation breaks.
- [ ] Sarvam Code ships low-confidence (LIKELY), with a re-verify note in its comment.
- [ ] The AGENTS group count updates by derivation, and the README-count test passes.
- [ ] The fixture `registry` scenario includes at least one of these rows.
- [ ] CHANGELOG is the orchestrator's job: list the rows added in one plain sentence.
- [ ] `npm run version:check && npm run check` passes.

## Comments
