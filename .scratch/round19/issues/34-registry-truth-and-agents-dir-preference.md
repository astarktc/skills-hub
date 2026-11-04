# 34 — Registry truth pass: harness rows corrected to vendor docs, global AGENTS-standard membership, cross-read dedupe, and the "prefer the AGENTS standard dir" preference

Status: ready-for-agent
Blocked by: 20 (the Harnesses pane is this ticket's UI home); 32 (vocabulary — done)
Blocks: 25
Source: `docs/design-inputs/2026-11-02-harness-global-skills-dirs.md` (the research brief — every claim below cites
its §; a wrong claim is fixed there first). Absorbs BACKLOG **#67** (Codex), **#45** (three path claims + four rows),
**#83** (the preference), and the detection half of **#40**. Operator approval 2026-11-02 ("registry truth approved").
Spec: D47 (per-harness rows in the Harnesses pane), D45 (vocabulary), AGENTS.md "New AI tool adapter" invariant
(registry literal + README row + `project_relative_skills_dir_for_every_tool` table test, all in one change).

## Problem

The registry (`core/tool_adapters/mod.rs`) was written from upstream and a 2026-09 survey. Checked against vendor
docs and harness source on 2026-11-02 (brief § 1, § 5): **Codex** reads `~/.agents/skills` since 0.95.0 and marks
`~/.codex/skills` deprecated — we deploy to the deprecated one, and a same-name skill in both shows twice in Codex
(§ 3); **Antigravity** documents `~/.gemini/config/skills` (ours: `.gemini/antigravity/global_skills`); **Kimi Code
CLI** reads `~/.kimi-code/skills` (ours: Amp's shared `.config/agents/skills`); **Kilo** moved to `~/.kilo/skills`;
**Mux** is now **Xum** (`~/.xum/skills`); **Goose** and **OpenHands** recommend `~/.agents/skills`; **Windsurf**'s
docs moved to `~/.config/devin/skills` (unverified build); **Clawdbot / MoltBot** are former names of OpenClaw.
**24 of 44** harnesses read `~/.agents/skills` globally (§ 1, I11). And harnesses **cross-read** each other's dirs:
Cursor reads `~/.claude/skills` and `~/.codex/skills`; OpenCode, Amp, Augment, Crush, Goose, Continue, Codewhale and
legacy Kimi read `~/.claude/skills` (§ 1, I1) — so deploying one skill to Claude Code *and* Cursor already shows it
twice inside Cursor. The four upstream rows (DeepSeek Harness, ZCode, Codewhale, WorkBuddy) are real products (§ 4).

## Work

### Registry facts (backend, `core/tool_adapters/`)

- **New per-entry facts**, registry literals only, each compiler- or test-enforced: `reads_agents_standard_globally:
  Option<AgentsStandardRead { since: Option<&'static str>, precedence: OwnWins | AgentsWins | BothShown | Unknown }>`
  (brief § 1 column "Reads ~/.agents/skills? / Since / Precedence" — VERIFIED rows only; LIKELY/UNKNOWN stay `None`
  with a comment naming the brief row), and `also_reads: &[&'static str]` (relative dirs of *other* harnesses this one
  reads — Cursor: `.claude/skills`, `.codex/skills`; the `~/.claude/skills` readers listed in I1). Add a table test
  that every entry's facts match a checked-in snapshot derived from the brief, so a registry edit without a brief
  edit fails.
- **Rows corrected** (brief § 5, I2–I10), each with its former dir in `former_relative_skills_dirs` so **Target
  relocation** moves existing deployments on next launch: Antigravity → `.gemini/config/skills` (former
  `.gemini/antigravity/global_skills`, `.gemini/antigravity/skills`; detect stays `.gemini/antigravity`, never
  `.gemini/config` — a skills-only footprint); Kilo → `.kilo/skills` (former `.kilocode/skills`; detect both); Mux →
  Xum (display name, key stays `mux` unless a key migration is cheap — decide and record; `.xum/skills`, former
  `.mux/skills`, detect both); Clawdbot and MoltBot: retire as rows **only after** checking the operator's machine
  has no targets there (brief I10) — else keep detect-only; Windsurf: hold, `.config/devin/skills` noted as a
  candidate (I9).
- **Kimi Code CLI → `.kimi-code/skills`** with the I4 hazard handled: `.config/agents/skills` is Amp's (and Crush's)
  live shared dir, so it must **not** go into Kimi's `former_relative_skills_dirs` blindly — relocation would move
  Amp's targets. Model it so relocation only moves a row whose Tool is Kimi and whose path is inside the former dir,
  and never removes the artifact when another installed sharer's row points at it (extend
  `core/target_relocation.rs`'s rule; test it with Amp installed beside Kimi).
- **Codex joins the AGENTS-standard group globally** (I2 option (a)): `.agents/skills` becomes Codex's global
  deployment dir through group membership (shared-dir dedupe, `shared_with`, the README group sentence), with
  `.codex/skills` as a former dir so existing targets relocate. Goose and OpenHands likewise (I8). The virtual group's
  own capability rule (AGENTS.md "Do not") is unchanged.
- **Four new rows** (I13): DeepSeek Harness (`.dsh/skills`), ZCode (`.zcode/skills`), Codewhale (`.codewhale/skills`,
  never detect on `.deepseek`), WorkBuddy (`.workbuddy/skills`, project dir unresolved — global-only, say so in the
  row comment and README). README table rows + the table test, per the invariant.

### Cross-read dedupe (backend + Harnesses pane)

- The Tool catalog (`global_tool_entries`) gains, per entry, the harnesses whose dirs it **also reads**
  (`also_reads` resolved against installedness), so the frontend can warn: "Cursor also reads Claude Code's and
  Codex's skills — a skill deployed to all three shows three times in Cursor." Shown in the Harnesses pane row and
  as a one-line note on the deploy confirmation when the chosen set contains such a pair. **No automatic
  de-selection** — a warning, since precedence is per-harness and often unknown.

### The preference (#83; D47 — UI in ticket 20's Harnesses pane)

- Setting **"Prefer the AGENTS standard folder (`~/.agents/skills`) for harnesses that read it"** (off by default
  for existing installs; decide the default for a fresh install and record it). When on: for every selected harness
  with `reads_agents_standard_globally` VERIFIED, the effective global target becomes the AGENTS-standard group
  entry instead of the harness's own dir — **and the harness-specific deployment stops** (brief I1: otherwise the
  skill appears twice), with existing harness-dir targets relocated into `~/.agents/skills` by the same relocation
  rule (byte-identical replace, else leave + log) on save, under one confirmation that counts the moves. Sub-setting
  (visible only when on): "Hide harness-specific folders in this list" — collapses those rows to a single
  "AGENTS standard — Cursor, Codex, OpenCode +5" row. A harness with precedence `OwnWins` whose own dir still holds a
  foreign copy is flagged (the own copy shadows ours). Settings keys + `AppSettings` fields + `specta` export.
- Tests: effective-target-set derivation with the preference on/off across `OwnWins` / `AgentsWins` / `BothShown` /
  `None`; the relocation on save; the warning rows.

## Proof

- `cargo test --all` (registry snapshot test, relocation tests incl. the Kimi/Amp hazard, effective-set tests);
  README table matches the `ToolId` count; `npm run test`.
- Fixture: a `registry` scenario (or `rich`) showing the Harnesses pane with the cross-read warning, the preference
  on/off, and the collapsed group row; screenshots both themes under `evidence/34/`.
- Operator check on the real machine before release: `ls -la ~/.codex/skills ~/.agents/skills` before/after the
  relocation (release smoke item on ticket 26).
- D31 audit (Astra) on the Harnesses pane changes; `npm run version:check && npm run check`.

## Result

## Comments

- 2026-11-02 — opened at the operator's approval, after the research child's brief landed (`1cb3d36`). Blocked on
  ticket 20 for its UI home; the backend registry half could start earlier in a worktree if 20 is slow.
- 2026-11-04 — operator grill (autonomous run through 31 → 20 ∥ 21 → 34 → 24 → 25 → 26; results reviewed at the end): Codex (and Goose, OpenHands) join the AGENTS-standard group globally as the brief decided — the operator's own `~/.codex/skills` targets relocate on first launch, accepted. **#83 preference "prefer the AGENTS standard dir" defaults ON.** Clawdbot / MoltBot / OpenClaw: the operator never installed them — retire the rows (no machine check needed).
