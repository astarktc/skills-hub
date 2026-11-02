# Design inputs — which global skills directories each harness reads (2026-11-02)

A research brief for BACKLOG #83 (the "prefer the AGENTS standard dir" preference), #67 (Codex), #45 (three path claims + four
new rows) and #40. Read-only survey: no code changed. **Provenance**: unlike the other briefs here, this one has no
Quartermaster source file — it was researched directly by a delegated child on 2026-11-02 and its evidence is the § Sources
table below; a wrong claim is fixed here, with its source row. Every claim carries a source pointer (§ Sources, all fetched 2026-11-02).
**Confidence**: **VERIFIED** = vendor doc, release note or the harness's own source. **LIKELY** = secondary source (aggregator,
DeepWiki, blog, forum, third-party install table) or an older/indirect vendor page. **UNKNOWN** = no source found. The
`vercel-labs/skills` `src/agents.ts` table (the "add-skill" table our registry comments cite) is **secondary** throughout.

## Summary

1. **#67 is true.** Codex reads `~/.agents/skills` as its user skills dir since **rust-v0.95.0 (2026-02-04)**. `~/.codex/skills` is still read but the source labels it *"Deprecated user skills location, kept for backward compatibility"*. Codex removes duplicate roots by path only. Same-name skills are **not** merged: both show up (§3).
2. **#45 Antigravity: upstream is right.** All three surfaces (App, CLI, IDE) document `~/.gemini/config/skills`. The IDE still reads the legacy `~/.gemini/antigravity/skills`. Our `~/.gemini/antigravity/global_skills` appears in no current vendor doc (§2).
3. **#45 Kimi: our row targets a dir the current product does not read.** Kimi Code CLI (`~/.kimi-code`) reads `~/.kimi-code/skills` (or `$KIMI_CODE_HOME/skills`) plus `~/.agents/skills`. Only the legacy open-source `kimi-cli` (`~/.kimi`) reads `~/.config/agents/skills`, and it reads only the *first existing* of `~/.config/agents/skills` / `~/.agents/skills` (§2).
4. **#45 Cline: both are right.** Cline's source scans `~/.cline/skills` **and** `~/.agents/skills` (plus `.agents/skills` at project scope). Cline's own docs are stale and list only `~/.cline/skills` (§2).
5. **24 of the 44 harness rows** (45 registry entries minus the `agents_skills` convention row) VERIFIED-read `~/.agents/skills` globally, either beside their own dir or as their main dir. Rows that do **not** read it (VERIFIED): Claude Code, Antigravity, Kiro, Qoder, iFlow, CodeBuddy (per docs), Hermes (only when listed in `skills.external_dirs`).
6. **Precedence is not uniform.** Vendor dir wins: Amp-style `~/.config/agents` first, Augment, Roo, Qwen, Command Code, Pochi, Xum, DeepSeek Harness, ZCode, Codewhale, Kimi-legacy brand. `~/.agents` wins: Gemini CLI (same tier), OpenClaw, OpenHands. Both shown, no dedupe: Codex. Many say nothing (Cursor, OpenCode, Copilot, Goose, Crush, Junie, Vibe, Kilo, Droid within tier, Windsurf/Devin).
7. **New: cross-reading of other harnesses' dirs.** Cursor also reads `~/.claude/skills` **and** `~/.codex/skills`. OpenCode, Amp, Augment, Crush, Goose, Continue, Codewhale, Kilo (opt-in) and Kimi-legacy read `~/.claude/skills`. Deploying one skill to several Tool dirs can therefore show it twice in one harness (§ Implications, I1).
8. **Rows whose documented dir moved** (§5): Codex, Antigravity, Kimi, Kilo (`~/.kilo/skills`), Mux (renamed **Xum**, `~/.xum/skills`), Goose and OpenHands (own dir now "backward compat", `~/.agents/skills` recommended), Windsurf (docs now at docs.devin.ai: `~/.config/devin/skills`). Clawdbot and MoltBot are former names of OpenClaw.
9. **The four upstream rows exist as products.** DeepSeek Harness (`~/.dsh/skills`, plus `~/.agents/skills`) and ZCode (`~/.zcode/skills`, plus `~/.agents/skills`) are VERIFIED. Codewhale is `~/.codewhale/skills`, plus `~/.agents/skills`, legacy `~/.deepseek/skills`, VERIFIED. WorkBuddy is `~/.workbuddy/skills` per secondary sources only: the vendor doc says only "the user config directory" (§4).
10. **Still UNKNOWN:** precedence for the "silent" harnesses above; whether Kilo still reads `~/.kilocode/skills` and Windsurf still reads `~/.codeium/windsurf/skills`; Trae's official global path (the docs are JS-rendered); MCPJam, OpenClaude IDE, AdaL (no vendor docs found); Zencoder and Copaw (secondary only); "since" versions for most harnesses.

## §1 Global skills dirs per registry row

Registry order (`core/tool_adapters/mod.rs` `TOOL_ADAPTERS`). "Reads `~/.agents/skills`?" means at **global** scope, by default.
Precedence describes a same-name collision between the harness's own global dir and `~/.agents/skills` unless stated.

| # | Harness (key) | Registry global dir | Global dir(s) the current version reads | Reads `~/.agents/skills`? | Since | Precedence / dedupe | Confidence | Source |
|---|---|---|---|---|---|---|---|---|
| 1 | AGENTS standard (`agents_skills`) | `.agents/skills` | n/a (a convention dir, not a harness) | — | — | — | — | — |
| 2 | Cursor | `.cursor/skills` | `~/.cursor/skills`, `~/.agents/skills`; compat `~/.claude/skills`, `~/.codex/skills` | Yes | UNKNOWN | Undocumented | VERIFIED | S3 |
| 3 | Claude Code | `.claude/skills` | `~/.claude/skills` (+ `~/.claude/skills/synced/`, managed-settings dir) | **No** | — | Enterprise > personal > project | VERIFIED (absence: no `.agents` anywhere in the skills doc) | S2 |
| 4 | Codex | `.codex/skills` | `~/.agents/skills` (USER), `$CODEX_HOME/skills` (deprecated, still read), `/etc/codex/skills` (ADMIN) | Yes, the canonical user dir | rust-v0.95.0, 2026-02-04 (PR #10437) | Roots deduped by path only; same-name skills not merged, both listed | VERIFIED | S1, S4, S5, S6 |
| 5 | OpenCode | `.config/opencode/skills` | `~/.config/opencode/skills`, `~/.claude/skills`, `~/.agents/skills` | Yes | UNKNOWN | Undocumented | VERIFIED | S8 |
| 6 | Antigravity | `.gemini/antigravity/global_skills` | `~/.gemini/config/skills` (App, CLI, IDE); IDE legacy `~/.gemini/antigravity/skills`; CLI also `~/.gemini/antigravity-cli/skills`, `~/.gemini/skills` | **No** | — | Undocumented | VERIFIED | S9 |
| 7 | Amp | `.config/agents/skills` | 1 `~/.config/agents/skills`, 2 `~/.agents/skills`, 3 `~/.config/amp/skills`, 6 `~/.claude/skills` | Yes (`amp.skills.disableGlobalAgentsSkills` turns it off) | UNKNOWN | First `name` wins in that order: `~/.config/agents` > `~/.agents` > `~/.config/amp` | VERIFIED | S11, S12 |
| 8 | Kimi Code CLI (`kimi_cli`) | `.config/agents/skills` | **Kimi Code CLI:** `$KIMI_CODE_HOME/skills` (default `~/.kimi-code/skills`), `~/.agents/skills`. **Legacy kimi-cli (`~/.kimi`):** first existing of `~/.kimi/skills` › `~/.claude/skills` › `~/.codex/skills`, plus first existing of `~/.config/agents/skills` › `~/.agents/skills` | Yes (Kimi Code); legacy only if `~/.config/agents/skills` is absent | UNKNOWN | Kimi Code: undocumented within user tier. Legacy: brand dir before generic, first match per name | VERIFIED | S13, S14 |
| 9 | Augment | `.augment/skills` | `~/.augment/skills`, `~/.claude/skills`, `~/.agents/skills` | Yes | UNKNOWN | `~/.augment/skills` (highest) > workspace > `~/.claude` > `~/.agents` | VERIFIED | S20 |
| 10 | OpenClaw | `.openclaw/skills` | `~/.agents/skills` (default state only), `<state-dir>/skills` = `~/.openclaw/skills` | Yes | UNKNOWN | **`~/.agents` (rank 3) beats `~/.openclaw` (rank 4)** | VERIFIED | S33 |
| 11 | Copaw | `.copaw/skill_pool` | `~/.copaw/skill_pool/` (shared pool); per-agent `~/.copaw/workspaces/{agent}/skills/` | UNKNOWN | — | UNKNOWN | LIKELY (DeepWiki only) | S48 |
| 12 | Cline | `.cline/skills` | `~/.cline/skills`, `~/.agents/skills` (source); docs list only `~/.cline/skills` | Yes (source) | v3.67.0 per a vercel-labs PR (secondary) | Scan order: `~/.cline` then `~/.agents`; collision rule undocumented (docs: global beats project) | VERIFIED (source), docs stale | S15, S16, S17 |
| 13 | CodeBuddy | `.codebuddy/skills` | `~/.codebuddy/skills` | No (per docs v2.137.0) | — | Project > user | VERIFIED (docs; absence) | S38 |
| 14 | Command Code | `.commandcode/skills` | `~/.commandcode/skills`, `~/.agents/skills` | Yes | UNKNOWN | `~/.commandcode` > `~/.agents`; shadowed copy reported as a duplicate | VERIFIED | S41 |
| 15 | Continue | `.continue/skills` | `$CONTINUE_HOME/skills` (`~/.continue/skills`), `~/.claude/skills` | No source says yes | — | UNKNOWN | LIKELY (merged PRs; a third party says Continue is no longer maintained) | S43 |
| 16 | Crush | `.config/crush/skills` | `$CRUSH_SKILLS_DIR`, `~/.config/agents/skills`, `~/.config/crush/skills`, `~/.agents/skills`, `~/.claude/skills` | Yes | UNKNOWN | Undocumented (listed order only) | VERIFIED | S21 |
| 17 | Junie | `.junie/skills` | `~/.junie/skills`, `~/.agents/skills` | Yes | UNKNOWN | Undocumented | VERIFIED | S28 |
| 18 | iFlow CLI | `.iflow/skills` | `~/.iflow/skills` | **No** (not mentioned) | — | Project > global | VERIFIED (absence) | S35 |
| 19 | Kiro CLI | `.kiro/skills` | `~/.kiro/skills` | **No** (not mentioned) | — | Workspace > global | VERIFIED (absence) | S26 |
| 20 | Kode | `.kode/skills` | `~/.kode/**` (Kode-native), `~/.claude/**` compat | UNKNOWN | — | UNKNOWN | LIKELY (vendor docs mirrored on jsdelivr; no skills-path table) | S47 |
| 21 | MCPJam | `.mcpjam/skills` | UNKNOWN (no vendor doc found) | UNKNOWN | — | UNKNOWN | UNKNOWN | — |
| 22 | Mistral Vibe | `.vibe/skills` | `~/.vibe/skills`, `~/.agents/skills` (+ `skill_paths`) | Yes | UNKNOWN | Undocumented within user level | VERIFIED | S29 |
| 23 | Mux | `.mux/skills` | Renamed **Xum**: `~/.xum/skills` › `~/.agents/skills` › `~/.mux/skills` (read-only, Skills-CLI legacy) | Yes | UNKNOWN | `~/.xum` > `~/.agents` > `~/.mux` | VERIFIED | S42 |
| 24 | OpenClaude IDE | `.openclaude/skills` | UNKNOWN | UNKNOWN | — | UNKNOWN | UNKNOWN | — |
| 25 | OpenHands | `.openhands/skills` | `~/.agents/skills` (recommended); legacy `.openhands/skills` "remain supported" | Yes, the recommended dir | UNKNOWN | `.agents/skills` beats the legacy dirs within the user scope | VERIFIED | S27 |
| 26 | Pi | `.pi/agent/skills` | `~/.pi/agent/skills`, `~/.agents/skills` | Yes | UNKNOWN | First discovered wins, with a warning; docs list `~/.pi/agent/skills` first | VERIFIED (dirs); LIKELY (order) | S18, S19 |
| 27 | Qoder | `.qoder/skills` | `~/.qoder/skills` | **No** (not mentioned) | — | Project > user | VERIFIED (absence) | S37 |
| 28 | QoderWork | `.qoderwork/skills` | `~/.qoderwork/skills` (CN build: `~/.qoderworkcn/skills`) | UNKNOWN | — | UNKNOWN | LIKELY (vercel PR cites docs.qoder.com/qoderwork/skills; CN path from a third-party note) | S49 |
| 29 | Qwen Code | `.qwen/skills` | `~/.qwen/skills`, `~/.agents/skills` | Yes | UNKNOWN | `~/.qwen/skills` wins | VERIFIED | S25 |
| 30 | Trae | `.trae/skills` | `~/.trae/skills` | UNKNOWN | — | UNKNOWN | LIKELY (official docs JS-rendered; community repo + gaal) | S36 |
| 31 | Trae CN | `.trae-cn/skills` | `~/.trae-cn/skills` | UNKNOWN | — | UNKNOWN | LIKELY (official Trae CN forum staff answer) | S36 |
| 32 | Zencoder | `.zencoder/skills` | `~/.zencoder/skills` | UNKNOWN (a Zencoder automation recipe probes `~/.agents/skills` first, which is not loader behaviour) | — | UNKNOWN | LIKELY (docs.zencoder.ai returned 403) | S46 |
| 33 | Neovate | `.neovate/skills` | `~/.neovate/skills` (`neovate skill add -g`) | UNKNOWN | — | UNKNOWN | VERIFIED (dir); UNKNOWN (`.agents`) | S44 |
| 34 | Pochi | `.pochi/skills` | `~/.pochi/skills`, `~/.agents/skills` (fallback) | Yes | commit 2a9ff73, 2026-03-16 (#1360) | `.agents` paths are fallbacks; own dir wins | VERIFIED | S45 |
| 35 | AdaL | `.adal/skills` | UNKNOWN (no vendor doc found) | UNKNOWN | — | UNKNOWN | UNKNOWN | — |
| 36 | Kilo Code | `.kilocode/skills` | `~/.kilo/skills`, `~/.agents/skills` (default); `~/.claude/skills` when Claude compat is on | Yes | UNKNOWN | Project `.kilo` > global `.kilo`; compat dirs "loaded alongside", rule undocumented | VERIFIED | S24 |
| 37 | Roo Code | `.roo/skills` | `~/.roo/skills`, `~/.agents/skills` (+ `skills-{mode}` variants) | Yes | PR #11181, merged 2026-02-06 | `~/.roo` > `~/.agents`; any project skill beats any global one | VERIFIED | S22, S23 |
| 38 | Goose | `.config/goose/skills` | `~/.agents/skills` (recommended); backward compat `~/.claude/skills` and "platform-specific config directories" (= `~/.config/goose/skills`, not named) | Yes, the recommended dir | UNKNOWN | Undocumented | VERIFIED (`~/.agents`); LIKELY (`~/.config/goose` still read) | S10 |
| 39 | Gemini CLI | `.gemini/skills` | `~/.gemini/skills`, `~/.agents/skills` ("alias") | Yes | PR #18151, merged 2026-02-03 (release version not checked) | **Within the user tier `~/.agents/skills` beats `~/.gemini/skills`** | VERIFIED | S30, S31 |
| 40 | GitHub Copilot | `.copilot/skills` | `~/.copilot/skills`, `~/.agents/skills` (CLI and the other surfaces) | Yes | UNKNOWN (CLI issue #2230 shows it was once missing) | Undocumented | VERIFIED | S32, S34 |
| 41 | Clawdbot | `.clawdbot/skills` | Former name of OpenClaw; the vercel table falls back to `~/.clawdbot/skills` only when `~/.openclaw` is absent | — | — | — | LIKELY | S7 |
| 42 | Droid | `.factory/skills` | `~/.factory/skills`, `~/.agents/skills`, `~/.agent/skills` | Yes | UNKNOWN | All three are one "personal" tier; within-tier rule undocumented | VERIFIED | S39 |
| 43 | Windsurf | `.codeium/windsurf/skills` | Docs (docs.devin.ai, "Devin Local"/Cascade): `~/.config/devin/skills`, `~/.agents/skills`. A search-indexed copy of the same page also names `~/.codeium/windsurf/skills` | Yes | UNKNOWN | Undocumented | VERIFIED (`~/.config/devin`, `~/.agents`); LIKELY (`~/.codeium/windsurf` still read) | S40 |
| 44 | MoltBot | `.moltbot/skills` | Former name of OpenClaw (as Clawdbot) | — | — | — | LIKELY | S7 |
| 45 | Hermes Agent | `.hermes/skills` | `~/.hermes/skills`; `~/.agents/skills` only if listed in `skills.external_dirs` | **No** (opt-in only) | external_dirs: PR #3678, 2026-03-29 | Local `~/.hermes/skills` shadows external dirs | VERIFIED | S50 |

## §2 The three path claims from #45

| Claim (upstream) | Verdict | Evidence |
|---|---|---|
| Antigravity global `~/.gemini/config/skills` (we have `.gemini/antigravity/global_skills`) | **Upstream is right.** All three surfaces document `~/.gemini/config/skills` ("shared with the Antigravity App"). The IDE keeps `~/.gemini/antigravity/skills` as legacy. `global_skills` appears nowhere. The CLI also reads `~/.gemini/antigravity-cli/skills` and `~/.gemini/skills`. Since: UNKNOWN. | VERIFIED, S9 |
| Kimi global `~/.kimi-code/skills` or `$KIMI_CODE_HOME/skills` | **Upstream is right for Kimi Code CLI.** User tier = `$KIMI_CODE_HOME/skills` (default `~/.kimi-code/skills`) + `~/.agents/skills`; the generic dir stays under the real home even when `KIMI_CODE_HOME` moves. Our `~/.config/agents/skills` is read only by the legacy `kimi-cli` (`~/.kimi`), and only when it exists (it then hides `~/.agents/skills`). | VERIFIED, S13, S14 |
| Cline global `~/.agents/skills` | **Both dirs are read.** `getSkillsDirectoriesForScan` returns `~/.cline/skills` then `~/.agents/skills`. The SDK's `resolveSkillsConfigSearchPaths` does the same. Project scope reads `.clinerules/skills`, `.cline/skills`, `.claude/skills`, `.agents/skills`. A contributor reports that global skill *creation* writes to `~/.agents/skills` (secondary). Cline's docs still list only `~/.cline/skills`. | VERIFIED (source), S15–S17 |

## §3 #67 — Codex and `~/.agents/skills`

- **True.** Docs: user scope is `$HOME/.agents/skills`; `~/.codex/skills` is not listed (S1). Maintainer, 2026-03-11: *"`~/.codex/skills` is the old (legacy) name … We recommend moving to this directory"* (S6, issue comment, VERIFIED as an OpenAI staff statement).
- **Version:** rust-v0.95.0 (2026-02-04): *"Added personal skill loading from `~/.agents/skills` (with `~/.codex/skills` compatibility)"* (S5; PR #10437 merged 2026-02-03). Repo-scope `.agents/skills` landed just before it (PR #10317, 2026-02-01).
- **Both read, today:** `codex-rs/ext/skills/src/host_roots.rs` pushes `$CODEX_HOME/skills` ("Deprecated user skills location … kept for backward compatibility"), then `~/.agents/skills`, both `SkillScope::User`. `dedupe_skill_roots_by_path` removes only identical paths (S4).
- **Precedence:** none. Same-name skills are not merged and *"both can appear in skill selectors"* (S1). A skill deployed to both dirs shows twice in Codex.
- Caveat: the bundled `skill-installer` still defaults to `$CODEX_HOME/skills` (openai/skills#420), and `gh skill install --agent codex --scope user` writes `~/.codex/skills` (cli/cli#14153), so legacy dirs keep getting populated by other tools (LIKELY, issue reports).

## §4 The four upstream adapter rows

| Key (upstream) | Exists? | Global dir(s) | Project dir(s) | Detect dir | Confidence | Source |
|---|---|---|---|---|---|---|
| `deepseek_harness` | Yes: `deepseek-ai/deepseek-harness` (dsh) | `<dshHome>/skills` (default `~/.dsh/skills`, `$DSH_HOME`), `<agentsHome>/skills` (default `~/.agents/skills`, `$DSH_AGENTS_HOME`) | `.dsh/skills`, `.agents/skills` | `~/.dsh` (the dsh home) | VERIFIED (dirs, ranks); env-var names LIKELY (Reddit) | S51 |
| `zcode` | Yes: ZCode by Z.ai (`zai-org/ZCode`) | `~/.zcode/skills`; `~/.agents/skills` per the product's bundled diagnosing-skills guide | `.zcode/skills`, `.agents/skills` | `~/.zcode`; the vercel table also accepts `/Applications/ZCode.app` | VERIFIED (`~/.zcode/skills`: zcode.z.ai docs + FAQ; `~/.agents`: guide shipped inside `zcode-app-cli`) | S52, S53 |
| `codewhale` | Yes: Codewhale (rebrand of a DeepSeek-based TUI) | Owned `~/.codewhale/skills` (`$CODEWHALE_HOME/skills`); read-only `~/.agents/skills`, `~/.claude/skills`, legacy `~/.deepseek/skills` | `.codewhale/skills` (owned); read-only `.agents`, `.claude`, `.opencode`, `.cursor` | `~/.codewhale` (legacy `~/.deepseek` is a read fallback, too generic to detect on) | VERIFIED | S54, S55 |
| `workbuddy` | Yes: Tencent WorkBuddy (desktop AI assistant; CodeBuddy's sibling) | `~/.workbuddy/skills` (vendor doc says only "the user config directory") | Conflicting secondary claims: `{workspace}/.workbuddy/skills` vs `.agents/skills` + `skills/` | `~/.workbuddy` | LIKELY (Tencent Cloud developer-community articles); vendor doc does not name the path | S56, S57 |

Precedence for the new rows: DSH scans in rank order, `~/.dsh` (400) before `~/.agents` (500); a Reddit answer says first match
wins (LIKELY). ZCode: `~/.zcode` before `~/.agents`, first same-name skill wins. Codewhale: `.codewhale` > `.agents` > `.claude`
> `.deepseek`, project before global.

## §5 Registry rows whose documented dirs differ

| Harness | Registry global dir | Documented global dir (current) | Source | Confidence |
|---|---|---|---|---|
| Codex | `.codex/skills` | `~/.agents/skills`; `~/.codex/skills` deprecated but read | S1, S4, S5 | VERIFIED |
| Antigravity | `.gemini/antigravity/global_skills` | `~/.gemini/config/skills` (legacy IDE `~/.gemini/antigravity/skills`) | S9 | VERIFIED |
| Kimi Code CLI | `.config/agents/skills` | `~/.kimi-code/skills` + `~/.agents/skills` | S13 | VERIFIED |
| Kilo Code | `.kilocode/skills` (detect `.kilocode`) | `~/.kilo/skills` + `~/.agents/skills`; whether `~/.kilocode/skills` is still read: UNKNOWN | S24 | VERIFIED / UNKNOWN |
| Mux | `.mux/skills` (detect `.mux`) | Product renamed **Xum**: `~/.xum/skills`; `~/.mux/skills` read-only legacy, lowest | S42 | VERIFIED |
| Goose | `.config/goose/skills` | `~/.agents/skills` recommended; own config dir = backward compat | S10 | VERIFIED / LIKELY |
| OpenHands | `.openhands/skills` | `~/.agents/skills` recommended; `.openhands/skills` legacy, supported | S27 | VERIFIED |
| Windsurf | `.codeium/windsurf/skills` | `~/.config/devin/skills` + `~/.agents/skills` (docs moved to docs.devin.ai) | S40 | VERIFIED / LIKELY |
| Clawdbot, MoltBot | `.clawdbot/skills`, `.moltbot/skills` | Former names of OpenClaw (`~/.openclaw/skills`) | S7, S33 | LIKELY |
| Cline (docs vs source) | `.cline/skills` | Unchanged: source reads `~/.cline/skills` and `~/.agents/skills` | S15 | VERIFIED — no change needed |

Project-scope differences found on the way (not this brief's focus): Goose and OpenHands now recommend `.agents/skills` (ours:
`.goose/skills`, `.openhands/skills`). Windsurf/Devin's workspace dir is `.devin/skills`, with `.windsurf/skills` legacy and
`.agents/skills` also read. Kilo's is `.kilo/skills` (ours `.kilocode/skills`). Xum's is `.xum/skills`. Many non-grouped harnesses
also read project `.agents/skills`: Pi, Roo, Kilo, Junie, Vibe, Command Code, Droid, Crush, Augment, Hermes (trusted), Pochi,
OpenClaw, Qwen.

## Implications for the registry (recommendations only)

- **I1 — Duplicates are a #83 design input, not just a path fix.** A harness that reads two of the dirs we deploy to shows the skill twice (Codex: documented) or resolves by an undocumented rule. Today selecting both `agents_skills` and Codex/Cursor/OpenCode/Gemini/… puts one skill in two dirs one harness reads. Cross-reads widen this: Cursor reads `~/.claude/skills` and `~/.codex/skills`; OpenCode, Amp, Augment, Crush, Goose, Continue, Codewhale and Kimi-legacy read `~/.claude/skills`. #83's "deploy to `~/.agents/skills` for harnesses that read it" should also *stop* the harness-specific deployment by default, or say why not. A per-harness registry fact "also reads: [dirs]" (beyond `~/.agents`) would let `shared_with`-style dedupe warn about cross-reads.
- **I2 — Codex (#67): join the AGENTS-standard group globally.** Codex's real user dir *is* `~/.agents/skills`. Either (a) Codex becomes a global member of the `agents_skills` group (`shared_with` dedupe, README group sentence, `group_label` count), or (b) its row moves to `.agents/skills` with `.codex/skills` in `former_relative_skills_dirs` so Target relocation moves existing targets. (a) avoids two rows owning one dir. Either way, stop deploying to `~/.codex/skills` (deprecated, and duplicates in Codex itself and in Cursor).
- **I3 — Antigravity:** move to `.gemini/config/skills`; add `.gemini/antigravity/global_skills` **and** `.gemini/antigravity/skills` to `former_relative_skills_dirs`. Detect: keep `.gemini/antigravity`, consider `.gemini/antigravity-cli`. Do **not** detect on `.gemini/config`: its only documented content here is skills/plugins, i.e. the skills-only footprint `is_installed_in` rejects. Needs verifying on disk.
- **I4 — Kimi Code CLI:** move to `.kimi-code/skills`, or join the group globally (it reads `~/.agents/skills`). **Hazard:** do not put `.config/agents/skills` in `former_relative_skills_dirs` blindly. Amp (and Crush) read and share that dir, so relocation would move Amp-shared targets. Model "former" per pair, or leave the old dir to the Amp row. Keep detect `.kimi-code`, `.kimi` (the legacy `~/.kimi` kimi-cli still reads `~/.config/agents/skills`).
- **I5 — Cline:** no move needed (`~/.cline/skills` VERIFIED read). Optional: global group membership, since `~/.agents/skills` is read too.
- **I6 — Kilo Code:** move to `.kilo/skills`, `former_relative_skills_dirs: [".kilocode/skills"]` (legacy read is UNKNOWN, so relocation is the safe path); detect `.kilo` + `.kilocode`; project dir `.kilo/skills` (or join the project group: `.agents/skills` is "loaded by default").
- **I7 — Mux → Xum:** operator decision. Either a new display name with global `.xum/skills`, former `.mux/skills`, detect `.xum` + `.mux`, project `.xum/skills`; or global group membership (Xum reads `~/.agents/skills` above `~/.mux/skills`).
- **I8 — Goose, OpenHands:** strongest candidates to join the group globally (vendor-recommended dir is `~/.agents/skills`; own dirs are compat/legacy). If kept as own rows, leave the dirs: they are still read.
- **I9 — Windsurf:** hold. Verify `~/.codeium/windsurf/skills` against a current Windsurf/Devin build before moving; candidate new dir `.config/devin/skills`. Reads `~/.agents/skills`, so group membership is the no-risk option.
- **I10 — Clawdbot, MoltBot:** retire as separate rows (OpenClaw renames), or keep them detect-only. Operator decision; check whether any operator machine has targets there first. OpenClaw reads `~/.agents/skills` *above* `~/.openclaw/skills`, so group membership is the cleaner global target.
- **I11 — Global AGENTS-group candidates for #83's per-harness fact** (VERIFIED read of `~/.agents/skills`): Cursor, Codex, OpenCode, Amp, Kimi Code CLI, Augment, OpenClaw, Cline, Command Code, Crush, Junie, Mistral Vibe, Xum/Mux, OpenHands, Pi, Qwen Code, Pochi, Kilo, Roo, Goose, Gemini CLI, Copilot, Droid, Windsurf/Devin, plus new DeepSeek Harness, ZCode, Codewhale. The "since" version matters for #83's version gate: VERIFIED only for Codex (0.95.0), Gemini CLI (PR #18151), Roo (PR #11181) and Pochi (#1360). The registry fact should carry precedence as well: `own-wins` / `agents-wins` / `both-shown` / `unknown`, from §1.
- **I12 — Never group:** Claude Code, Antigravity, Kiro, Qoder, iFlow, CodeBuddy, Hermes (opt-in only). UNKNOWN stays UNKNOWN until verified: Copaw, Continue, Kode, MCPJam, OpenClaude, QoderWork, Trae/Trae CN, Zencoder, Neovate, AdaL.
- **I13 — New rows (#45):** DeepSeek Harness (`.dsh/skills` / detect `.dsh` / project `.dsh/skills`), ZCode (`.zcode/skills` / `.zcode` / `.zcode/skills`), Codewhale (`.codewhale/skills` / `.codewhale` / `.codewhale/skills`; do **not** detect on `.deepseek`), WorkBuddy (`.workbuddy/skills` / `.workbuddy`, project dir unresolved, upstream treats it as global-only). All but WorkBuddy also read `~/.agents/skills` and are group candidates. README table + `project_relative_skills_dir_for_every_tool` per AGENTS.md's adapter invariant.

## Sources

All fetched 2026-11-02. P = primary (vendor doc / release / source / vendor staff), S = secondary.

| Id | URL | Kind | Note |
|---|---|---|---|
| S1 | https://developers.openai.com/codex/skills (served as learn.chatgpt.com "Build skills") | P | Codex skill locations table |
| S2 | https://code.claude.com/docs/en/skills | P | Claude Code locations, precedence |
| S3 | https://cursor.com/docs/context/skills | P | Cursor dirs incl. compat dirs |
| S4 | https://raw.githubusercontent.com/openai/codex/main/codex-rs/ext/skills/src/host_roots.rs | P | Root resolution, deprecated comment, path dedupe |
| S5 | https://github.com/openai/codex/releases/tag/rust-v0.95.0 | P | Release note (2026-02-04); PRs #10437 / #10317 |
| S6 | https://github.com/openai/codex/issues/14337 | P (staff comment) | "legacy name" statement, 2026-03-11; also cli/cli#14153, openai/skills#420 (S, issue reports) |
| S7 | https://raw.githubusercontent.com/vercel-labs/skills/main/src/agents.ts | S | add-skill agent table |
| S8 | https://opencode.ai/docs/skills/ | P | |
| S9 | https://antigravity.google/docs/skills (raw HTML, all three surface tabs) | P | |
| S10 | https://goose-docs.ai/docs/guides/context-engineering/using-skills/ | P | (block.github.io URL 404s) |
| S11 | https://ampcode.com/docs/customize/skills | P | Precedence list |
| S12 | https://ampcode.com/docs/cli/settings | P | `disableGlobalAgentsSkills` |
| S13 | https://www.kimi.com/code/docs/en/kimi-code-cli/customization/skills.html | P | Kimi Code CLI tiers |
| S14 | https://raw.githubusercontent.com/MoonshotAI/kimi-cli/main/src/kimi_cli/skill/__init__.py | P | Legacy kimi-cli candidates |
| S15 | https://raw.githubusercontent.com/cline/cline/main/apps/vscode/src/core/storage/skill-directories.ts | P | Cline scan dirs |
| S16 | https://raw.githubusercontent.com/cline/cline/main/sdk/packages/shared/src/storage/paths.ts | P | Cline SDK search paths |
| S17 | https://docs.cline.bot/features/skills ; https://github.com/cline/cline/issues/12523 ; https://github.com/vercel-labs/skills/pull/428 ; https://github.com/vercel-labs/skills/issues/1061 | P (docs) / S | Stale docs; contributor comment; v3.67.0 claim |
| S18 | https://raw.githubusercontent.com/badlogic/pi-mono/main/packages/coding-agent/docs/skills.md | P | Collision rule |
| S19 | https://github.com/badlogic/pi-mono/blob/351efc82/packages/coding-agent/docs/skills.md (via search) | P | Names `~/.pi/agent/skills` + `~/.agents/skills` |
| S20 | https://docs.augmentcode.com/cli/skills | P | |
| S21 | https://raw.githubusercontent.com/charmbracelet/crush/main/README.md | P | Also v0.52.0 release note (2026-03-25) |
| S22 | https://docs.roocode.com/features/skills | P | |
| S23 | https://github.com/RooCodeInc/Roo-Code/pull/11181 | P | Merged 2026-02-06 |
| S24 | https://kilo.ai/docs/customize/skills | P | |
| S25 | https://qwenlm.github.io/qwen-code-docs/en/users/features/skills/ (via search) | P | |
| S26 | https://kiro.dev/docs/skills/ | P | |
| S27 | https://docs.openhands.dev/overview/skills | P | |
| S28 | https://junie.jetbrains.com/docs/agent-skills.html | P | |
| S29 | https://docs.mistral.ai/vibe/code/cli/skills | P | |
| S30 | https://geminicli.com/docs/cli/skills/ | P | |
| S31 | https://github.com/google-gemini/gemini-cli/pull/18151 ; …/pull/19166 (via search) | P | Merged 2026-02-03 / 2026-02-16 |
| S32 | https://docs.github.com/en/copilot/concepts/agents/about-agent-skills | P | |
| S33 | https://docs.openclaw.ai/tools/skills | P | Loading-order table |
| S34 | https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-skills ; https://github.com/github/copilot-cli/issues/2230 | P / S | |
| S35 | https://platform.iflow.cn/en/cli/examples/skill | P | |
| S36 | https://docs.trae.ai/ide/skills (JS-rendered, not readable) ; https://github.com/trae-community/trae-skills ; https://forum.trae.cn/t/topic/67755 | P (unreadable) / S | |
| S37 | https://docs.qoder.com/extensions/skills | P | |
| S38 | https://cdn.jsdelivr.net/npm/@tencent-ai/codebuddy-code@2.137.0/dist/web-ui/docs/en/cli/skills.md | P (docs shipped in the npm package; latest npm is 2.179.3) | codebuddy.ai blocked from fetch |
| S39 | https://docs.factory.com/harness/skills | P | |
| S40 | https://docs.devin.ai/desktop/cascade/skills.md (docs.windsurf.com redirects here) | P | Search snippet of the same page also names `~/.codeium/windsurf/skills` (S) |
| S41 | https://commandcode.ai/docs/skills | P | |
| S42 | https://mux.coder.com/agent-skills (served as Xum docs) | P | |
| S43 | https://github.com/continuedev/continue/pull/9353 ; …/pull/11113 (via search) ; https://agnostic-ai.org/docs/targets/continue/ | P (PRs) / S | |
| S44 | https://neovateai.dev/docs/cli (via search) | P | |
| S45 | https://github.com/TabbyML/pochi/commit/2a9ff730a24cb318bd4e93b8564aa5f20cba4c2d ; https://docs.getpochi.com/skills/ (via search) | P | |
| S46 | https://docs.zencoder.ai/zenflow/scheduled-automation/product-managers (via search; features/skills returned 403) | P (indirect) | |
| S47 | https://cdn.jsdelivr.net/gh/shareai-lab/kode-cli@main/docs/compatibility.md (via search) | P | |
| S48 | https://deepwiki.com/agentscope-ai/CoPaw/5.5-skills-and-toolkit-system (via search) | S | |
| S49 | https://github.com/vercel-labs/skills/pull/1394 (via search) ; https://docs.qoder.com/qoderwork/skills (not fetched) | S | |
| S50 | https://hermes-agent.nousresearch.com/docs/user-guide/features/skills ; https://github.com/NousResearch/hermes-agent/pull/3678 | P | |
| S51 | https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/subsystems/skills.md ; reddit r/DeepSeek 1vqk5w5 (S) | P / S | |
| S52 | https://zcode.z.ai/en/docs/skill ; https://zcode.z.ai/en/docs/qa (via search) | P | |
| S53 | https://cdn.jsdelivr.net/npm/zcode-app-cli@3.8.1-15/vendor/packages/zcode-guide-plugin/skills/diagnosing-skills/SKILL.md (via search) | P (shipped in the product package) | |
| S54 | https://raw.githubusercontent.com/codewhale-hq/Codewhale/main/docs/SKILLS.md | P | |
| S55 | https://github.com/Hmbown/CodeWhale/blob/main/docs/REBRAND.md (via search) | P | `~/.deepseek` fallback |
| S56 | https://www.workbuddy.ai/docs/zh/workbuddy/From-Beginner-to-Expert-Guide/Function-Description/Skills (via search) | P | No path named |
| S57 | https://cloud.tencent.com/developer/article/2693324 ; https://cloud.tencent.com/developer/techpedia/2610/20530 (via search) | S | `~/.workbuddy/skills`; conflicting project dirs |
