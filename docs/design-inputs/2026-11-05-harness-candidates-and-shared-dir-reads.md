# Design inputs — harness candidates and shared-dir reads (2026-11-05)

A research brief for BACKLOG #92 (new registry rows, the `reads_agents_standard_globally` / `also_reads` facts, the
cross-read warning) and for the future Settings toggle that deploys only to the reusable shared dirs (#88, formerly #83).
Read-only survey: no code changed. **Provenance**: like the 2026-11-02 brief (cited here as **B02**), this one has no
Quartermaster source file. A delegated child researched it directly on 2026-11-05, and its evidence is the § Sources
table below. Fix a wrong claim here, with its source row. VERIFIED rows of B02 are reused with their B02 source id
(`B02:S3`) and were not re-researched unless a cell changed. Every new source was fetched on 2026-11-05.
**Confidence** (B02's vocabulary): **VERIFIED** = vendor doc, release note or the harness's own source code. A vendor
page read through a search-engine index because direct fetch was blocked counts as VERIFIED when the indexed text is
the vendor's own, and the source row says so. **LIKELY** = secondary source: aggregator, DeepWiki, blog, forum,
third-party table, `vercel-labs/skills` `src/agents.ts` (T1, never upgraded) or clients.dev. **UNKNOWN** = no source
found. "Reads X" always means **by default**. An absence claim cites a page that lists the harness's dirs and omits X.

## Summary

1. **Recommended additions.** Of the 38 candidates from `agents.ts`, I recommend **18 new rows**, **5 AGENTS-group
   members**, **3 aliases** of existing rows and **12 skips** (§ Part 1).
   - **New rows, by evidence and relevance:**
     - Grok Build (`.grok/skills`)
     - Deep Agents Code (`.deepagents/agent/skills`)
     - Reasonix (`.reasonix/skills`)
     - Cortex Code (`.snowflake/cortex/skills`)
     - Rovo Dev (`.rovodev/skills`)
     - IBM Bob (`.bob/skills`)
     - ForgeCode (`.forge/skills`)
     - Firebender (`.firebender/skills`)
     - MiniMax Code (`.minimax/skills`)
     - Posit Assistant (`.posit/assistant/skills`)
     - AiderDesk (`.aider-desk/skills`)
     - Autohand Code CLI (`.autohand/skills`)
     - Code Studio (`.codestudio/skills`)
     - CodeArts Agent (`.codeartsdoer/skills`)
     - Qoder CN IDE, formerly Lingma (`.lingma/skills`)
     - Qoder CN CLI (`.qoder-cn/skills`)
     - Optional, low priority: Kimchi (`.config/kimchi/harness/skills`) and AstrBot (`.astrbot/data/skills`, desktop
       runtime only).
   - **AGENTS-group members only:** Warp, Zed, Dexto, fx, Sarvam Code.
   - **Aliases:**
     - Devin for Terminal → the Windsurf row (it *is* that product now).
     - Antigravity CLI → the Antigravity row (add detect `.gemini/antigravity-cli`).
     - Zenflow → the Zencoder row.
   - **Found outside both lists:** oh-my-pi / `omp` (38k stars, `.omp/agent/skills`) and Letta Code (`.letta/skills`)
     merit new rows. VT Code is an AGENTS member. ZeroClaw and nanobot need their own pass: their skill dirs are
     per-workspace (§ Part 1, "Not in either list").
2. **Shared-dir sufficiency.** Deploying **only to `~/.agents/skills`** is enough for these harnesses:
   - 27 of our 42 kept harness rows: the 44 rows minus Clawdbot and MoltBot, which retire. Qoder counts partially:
     only its new desktop app is documented.
   - 3 of the 4 #92 rows. WorkBuddy is UNKNOWN.
   - 11 of the 18 recommended new rows, all 5 AGENTS members, plus omp and VT Code.

   It is **not** enough for:
   - Claude Code, Antigravity, Kiro, iFlow, CodeBuddy, QoderWork, Trae, Trae CN, Neovate and QwenPaw (ex-Copaw).
   - Hermes. It reads the dir only when it is listed in `skills.external_dirs`.
   - Cortex Code, IBM Bob, AiderDesk, CodeArts, both Qoder CN surfaces, Letta and AstrBot.

   **Only `~/.claude/skills`** reaches about 20 harnesses by default: Claude Code, Cursor, OpenCode, Amp, Augment,
   Crush, Goose, MCPJam, Neovate, Codewhale, Cortex Code, Grok Build, Firebender, Reasonix, Autohand, MiniMax Code, Code
   Studio, fx, Warp and omp. Continue and Kode are LIKELY. A toggle that deploys to **both** shared dirs reaches the
   union, but it creates same-name duplicates in every harness that reads both (§ Part 2 hazards, I3).
3. **Duplicates are not just cosmetic.**
   - **fx** refuses to load an ambiguous `$name`: "If several skills have the same name, typing `$name` loads none of
     them" (T35).
   - **Codex** and **Warp** show every copy.
   - **Cortex Code** flags a "conflict" when the copies differ.
   - **Pi**, **Hermes**, **Droid**, **Command Code** and **MiniMax Code** shadow one copy and warn.

   Deploying one skill to a harness's own dir *and* a shared dir it reads is therefore a correctness problem for fx and
   a noise problem for the rest.
4. **Opt-out settings exist and must be modelled as "default on, user can disable":**
   - Amp: `disableGlobalAgentsSkills`, `disableClaudeCodeSkills`
   - Grok: `[compat.claude].skills`
   - Devin: `read_config_from.windsurf|copilot|claude`
   - MiniMax: `skills.external.sources.*`
   - OpenClaw: a non-default `OPENCLAW_STATE_DIR` drops `~/.agents`
   - Code Studio: Skill File Locations
   - Posit: `skills.paths` replaces the defaults
5. **Rows the 2026-11-02 brief got wrong or that changed since** (§ Implications I6):
   - **Qoder** now reads `~/.agents/skills`. Its desktop-app docs say so, and its own dir wins. B02 said "No".
   - **Windsurf's `~/.codeium/windsurf/skills`** is VERIFIED read by Devin CLI. B02 had it LIKELY.
   - **Zencoder** reads `~/.agents/skills` as its user dir and calls `.zencoder/skills` deprecated. B02 had it UNKNOWN.
   - **Copaw was renamed QwenPaw.** Its default root is `~/.qwenpaw`; `~/.copaw` is used only when a legacy dir
     exists.
   - **QoderWork and Trae** are now VERIFIED. Trae reads project `.agents/skills` only behind an opt-in toggle.
   - **MCPJam** reads `~/.claude/skills`, `~/.mcpjam/skills` and `~/.agents/skills`.
   - **Neovate** reads `~/.claude/skills` and not `~/.agents/skills`.
   - **AdaL**'s own dir is VERIFIED.
6. **Windows paths break the home-relative registry for four candidates:**
   - Devin: `%APPDATA%\devin\skills`
   - Reasonix: `%APPDATA%\reasonix`
   - Zed's config root: `%APPDATA%\Zed`
   - Crush: also `%LOCALAPPDATA%\crush\skills`

   Everyone else documents `%USERPROFILE%\.<tool>` (= `~`) (I7).

## Part 1 — candidate rows

Global dir = what the registry's `relative_skills_dir` would be. The detect dir must be the tool's own config root, never
a skills-only footprint (AGENTS.md "New AI tool adapter").
Stars and push dates come from `gh api repos/<repo>` on 2026-11-05 (T79). "AGENTS member" means the harness becomes a
`VirtualGroup::AgentsStandard` member with no deploy target of its own; it is reached through `~/.agents/skills`.

### A. Candidates with their own global dir (28)

| Candidate | Vendor · kind | Global skills dir(s) per OS · env override | Detect dir (config root) | Project dir | Same product as a row? | Relevance | Recommendation | Conf · src |
|---|---|---|---|---|---|---|---|---|
| AiderDesk | Hotovo · desktop app (Aider GUI) | `~/.aider-desk/skills` · `AIDER_DESK_HOME_DIR` | `.aider-desk` | `.aider-desk/skills` | No | 1.5k★, pushed 2026-11-05 | **New row** | VERIFIED · T3 |
| Antigravity CLI | Google · CLI | `~/.gemini/config/skills` (shared with App/IDE) · also `~/.gemini/antigravity-cli/skills`, `~/.gemini/skills` | `.gemini/antigravity-cli` | `.agents/skills` | **Yes: Antigravity** (one product, three surfaces) | Google | **Alias of Antigravity**: add `.gemini/antigravity-cli` to its detect dirs. #88 moves the global dir to `.gemini/config/skills` | VERIFIED · T71, B02:S9 |
| AstrBot | AstrBotDevs · IM-bot / assistant framework (Python; packaged desktop app) | `<root>/data/skills`. Root = `$ASTRBOT_ROOT`, else `~/.astrbot` **only in the packaged desktop runtime**, else the cwd | `.astrbot` (desktop only) | `data/skills` (relative to root) | No | 46k★ | **New row, optional**. Desktop runtime only; source installs are undetectable | VERIFIED · T26 |
| Autohand Code CLI | Autohand AI · CLI | `~/.autohand/skills` · `AUTOHAND_HOME` | `.autohand` | `.autohand/skills` | No | 216★, docs mature | **New row** | VERIFIED · T25 |
| IBM Bob | IBM · IDE + Bob Shell CLI | `~/.bob/skills` | `.bob` (holds `settings/settings.json`) | `.bob/skills` | No | IBM enterprise | **New row** | VERIFIED · T4 |
| CodeArts Agent | Huawei Cloud · IDE | `%USERPROFILE%\.codeartsdoer\skills`. The docs are Windows-first; macOS/Linux `~/.codeartsdoer/skills` is LIKELY | `.codeartsdoer` (**flag**: unverified whether anything but skills lives there) | `.codeartsdoer/skills` | No | Huawei enterprise (CN) | **New row**, after confirming the detect dir on a real install | VERIFIED dir · T6 |
| Codemaker | NetEase (internal / Y3-editor agent) | `~/.codemaker/skills` per T1 only | `.codemaker` (T1) | `.codemaker/skills` (T1) | No | No public vendor skills doc | **Skip**: no primary source; NetEase-internal | LIKELY · T1, T2, T33 |
| Code Studio | Syncfusion · IDE (VS Code-based) | `~/.codestudio/skills`. Also reads `~/.copilot/skills`, `~/.claude/skills`, `~/.agents/skills` (setting "Skill File Locations") | `.codestudio` (**flag**: unverified as config root) | `.codestudio/skills` | No | Syncfusion enterprise | **New row** | VERIFIED (vendor help via search index) · T7 |
| Cortex Code ("CoCo") | Snowflake · CLI + Desktop | `~/.snowflake/cortex/skills` (CLI and Desktop share it). Also reads `~/.claude/skills` | `.snowflake/cortex`. **Never `.snowflake`**, which Snowflake CLI shares | `.cortex/skills` (also reads `.claude/skills`) | No | Snowflake enterprise | **New row** | VERIFIED · T5 |
| Deep Agents Code (`dcode`) | LangChain · CLI | `~/.deepagents/<agent>/skills` (default agent `agent`) · `DEEPAGENTS_HOME`. Also `~/.agents/skills` (stays under the real home) | `.deepagents` | `.deepagents/skills` (+ `.agents/skills`) | No | 31k★ | **New row** (`.deepagents/agent/skills`). Custom-named agents are unreachable; `~/.agents` *wins* over it, so AGENTS membership is an equally valid choice | VERIFIED · T8 |
| Devin for Terminal | Cognition · CLI (`devin`) | `~/.config/devin/skills` (Windows `%APPDATA%\devin\skills`), `~/.agents/skills`, `~/.codeium/<channel>/skills` | `.config/devin` (Windows `%APPDATA%\devin`) | `.devin/skills` (+ `.agents`, `.windsurf`) | **Yes: Windsurf row.** Windsurf docs now live at docs.devin.ai; Windsurf auth is "legacy" | Cognition | **Alias of Windsurf** (relabel / retarget there; see I5) | VERIFIED · T9, T10, T11 |
| Firebender | Firebender · JetBrains/Android Studio plugin | `~/.firebender/skills`. Also `~/.claude`, `~/.codex`, `~/.cursor`, `~/.goose`, `~/.agents` | `.firebender` | `.firebender/skills` | No | Funded startup; docs mature | **New row** | VERIFIED · T12 |
| ForgeCode | Tailcall/Antinomy · CLI | `~/.forge/skills` (migrated from `~/forge` in PR #2817, 2026-04). Also `~/.agents/skills` | `.forge` (legacy `forge` without the dot) | `.forge/skills` | No | 7.6k★ | **New row**. `~/.agents` wins over it, so AGENTS membership is also valid | VERIFIED · T13 |
| Grok Build | xAI ("SpaceXSI") · CLI/TUI | `~/.grok/skills` (Windows `%USERPROFILE%\.grok`) · `GROK_HOME`. Also `~/.agents/skills`, `~/.claude/skills`, `~/.cursor/skills` | `.grok` | `.grok/skills` (+ `.agents`, `.claude`, `.cursor`) | No | 28k★, xAI | **New row (top priority)** | VERIFIED · T14 |
| inference.sh | inference.sh · cloud platform + `belt` CLI | None of its own. `belt` *writes* into other agents' dirs; `~/.inferencesh/skills` appears only in T1 | n/a | n/a | No | — | **Skip**: a distributor, not a reader | VERIFIED (absence of reader role) · T31 |
| Jazz | lvndry · CLI | `~/.jazz/skills`, `~/.agents/skills` | `.jazz` | `./skills` | No | 54★ | **Skip**: low adoption, and it reads `~/.agents` anyway | VERIFIED · T29 |
| Kimchi | Cast AI · CLI ("governed AI coding") | `~/.config/kimchi/harness/skills`. Also `~/.agents/skills`, `~/.pi/agent/skills`, `~/.codex`, `~/.cursor`, `~/.warp`, `~/.factory`, `~/.gemini`, `~/.copilot`, `~/.github` (each "if installed"). `~/.claude` opt-in | `.config/kimchi` | `.kimchi/skills` | No | Cast AI enterprise, 2.2k★ | **New row, optional** | VERIFIED · T28 |
| Lingma | Alibaba (通义灵码) · IDE plugin, **rebranded Qoder CN IDE** | `~/.lingma/skills` | `.lingma` | `.lingma/skills` | Same product as Qoder CN (IDE) | Alibaba, large CN user base | Merged into **new row "Qoder CN"** (IDE): `.lingma/skills` | VERIFIED · T19 |
| MiniMax Code (`mcode`) | MiniMax · CLI (+ desktop app) | `<dataDir>/skills` (= `~/.minimax/skills`, LIKELY). Also `~/.claude/skills`, `~/.codex/skills`, `~/.agents/skills` | `.minimax` (T1 also accepts `/Applications/MiniMax Code.app`) | `.minimax/skills` (+ `.claude`, `.agents`) | No | 2.2k★, MIT | **New row** | VERIFIED (roots) · T24. `dataDir` = `~/.minimax` LIKELY · T1 |
| Moxby | Moxby · browser extension | None on disk (marketplace and "Chat Toolkit" skills) | n/a | n/a | No | — | **Skip**: no filesystem skills dir documented | VERIFIED (absence) · T30 |
| Ona | Ona (ex-Gitpod) · cloud agent | None. Skills are repo-level (`.ona/skills`, `.claude/skills`, `.agents/skills`) or organization-level in the cloud | n/a | `.ona/skills` | No | Funded | **Skip**: cloud; no user-level dir | VERIFIED · T17 |
| Posit Assistant | Posit · IDE assistant (Positron / RStudio) | `~/.posit/assistant/skills` (legacy `~/.positai`). Also `~/.agents/skills` | `.posit/assistant` (+ legacy `.positai`). **Never `.posit`** | `.posit/assistant/skills` (+ `.agents/skills`) | No | Posit enterprise | **New row** | VERIFIED · T18 |
| Qoder CN | Alibaba · CLI (Qoder CN CLI) + desktop app | CLI: `~/.qoder-cn/skills`. IDE: `~/.lingma/skills` (see Lingma) | `.qoder-cn` | `.qoder/skills` | Sibling of our Qoder row (the Trae / Trae CN pattern) | Alibaba | **New row "Qoder CN CLI"** (`.qoder-cn/skills`). The CN desktop app reads `~/.agents/skills` (T19) | VERIFIED · T19 |
| Reasonix | esengine · CLI / desktop / editor | `<Reasonix home>/skills` = `~/.reasonix/skills` (Windows `%APPDATA%\reasonix\skills`) · `REASONIX_HOME`. Also `~/.agents`, `~/.agent`, `~/.claude` | `.reasonix` (Windows `%APPDATA%\reasonix`) | `.reasonix/skills` | No | 36k★ | **New row**. Windows path hazard (I7) | VERIFIED · T27 |
| Rovo Dev | Atlassian · CLI | `~/.rovodev/skills` or `~/.agents/skills` (one "User" tier) | `.rovodev` | `.rovodev/skills` (+ `.agents/skills`) | No | Atlassian enterprise | **New row** | VERIFIED · T15 |
| Tabnine CLI | Tabnine · CLI (Gemini CLI fork) | `~/.tabnine/agent/skills`; `~/.agents/skills` alias wins over it | `.tabnine` (**hazard**: shared with Tabnine IDE plugins) | `.tabnine/agent/skills` | No | **Maintenance mode since 6.6; deprecated after 2026-12-31** | **Skip**: end-of-life, and the AGENTS row reaches it | VERIFIED · T16 |
| Terramind | Terramind · Mac apps (Nucleus) wrapping Claude Code / Codex / Grok; the old `terramind-cli` repo is gone | `~/.terramind/skills` per T1 only | — | — | No | CLI dead (AUR upstream link 404) | **Skip** | LIKELY · T1, T32 |
| Tinycloud | Cloudglue · video-agent CLI | `~/.tinycloud/skills` per T1/clients.dev only; `~/.tinycloud/versions` is its install dir | `.tinycloud` | — | No | Niche, non-coding | **Skip**: no vendor skills doc | LIKELY · T1, T32 |

### B. Candidates whose global dir is only `~/.agents/skills` per `agents.ts` (6)

| Candidate | Vendor · kind | What the harness actually reads | Detect dir | Relevance | Recommendation | Conf · src |
|---|---|---|---|---|---|---|
| Dexto | Truffle AI · CLI / agent runtime | `~/.agents/skills` (canonical) + legacy `~/.dexto/skills`; project `.agents/skills`, legacy `skills/`, `.dexto/skills` | `.dexto` | 659★ | **AGENTS member** | VERIFIED · T34 |
| fx | Vercel · CLI | **Own** `~/.fx/skills` (where `/skills install` writes), plus `~/.config/opencode/skills`, `~/.codex/skills`, `~/.claude/skills`, `~/.agents/skills`, `~/.claw/skills`. T1 is wrong to call `~/.agents` its global dir | `.fx` | 3.8k★, Vercel | **AGENTS member**. Never also deploy to `~/.fx/skills`: duplicate names make `$name` load nothing | VERIFIED · T35 |
| Loaf | veyzyn · CLI | `~/.loaf/skills`, `~/.agents/skills` | `.loaf` | 1★, last push 2026-02 | **Skip** | VERIFIED (project docs) · T36 |
| Sarvam Code | Sarvam AI · CLI (Codex fork) | `~/.sarvam/skills/.system` (bundled). `~/.agents/skills` per T1 only (likely inherited from Codex) | `.sarvam` (holds `config.toml`) | Invite-gated | **AGENTS member** (low confidence; re-verify when public) | LIKELY · T1, T37 |
| Warp | Warp · terminal + agent (and Oz CLI) | `~/.agents/skills` (recommended), `~/.warp/skills`, **and** `~/.claude`, `~/.codex`, `~/.cursor`, `~/.gemini`, `~/.copilot`, `~/.factory`, `~/.github`, `~/.opencode` `/skills` | `.warp` (macOS. **Flag**: Linux/Windows config roots differ, UNKNOWN) | 66k★ | **AGENTS member** | VERIFIED · T22 |
| Zed | Zed Industries · editor | **Only** `~/.agents/skills` (+ project `.agents/skills`). Deliberately no tool-specific dirs, no extra paths | `.config/zed` (Windows `%APPDATA%\Zed`; Flatpak differs) | 93k★ | **AGENTS member** | VERIFIED · T23 |

### C. Probably not applicable (4)

| Candidate | Finding | Recommendation | Conf · src |
|---|---|---|---|
| Eve | Project-only (`agent/skills`, detected by an `eve` dependency in `package.json`); no global dir | **Skip** | LIKELY · T1 |
| PromptScript | Project-only (`.agents/skills`); no global dir | **Skip** | LIKELY · T1 |
| Replit | Cloud workspace, detected by `.replit` in the cwd | **Skip** | LIKELY · T1 |
| Zenflow | Zencoder's desktop orchestrator. Its docs reuse Zencoder's skills page ("Skills are available in both Zenflow and Zencoder IDE Agents") | **Alias of Zencoder** | VERIFIED · T38 |

### Not in either list (from the agentskills.io client showcase, T39, and the sources above)

| Harness | Global dir(s) | Reads `~/.agents` / `~/.claude` | Detect | Relevance | Recommendation | Conf · src |
|---|---|---|---|---|---|---|
| oh-my-pi (`omp`) | `~/.omp/agent/skills`, managed `~/.omp/agent/managed-skills` | Both yes. Also `~/.agent`, `~/.codex`, `~/.config/opencode`; per-source switches `skills.enable*User` | `.omp` | 38k★ | **New row** (`.omp/agent/skills`, project `.omp/skills`) | VERIFIED · T41 |
| Letta Code | `~/.letta/skills` (computer scope) + agent memory | No / no (project `.agents/skills` yes) | `.letta` | 3.8k★, Letta | **New row** (project dir = AGENTS group) | VERIFIED · T40 |
| VT Code | `~/.agents/skills` only (+ `/etc/codex/skills`) | Yes / no | `.vtcode` (UNKNOWN) | 898★ | **AGENTS member** | VERIFIED · T42 |
| ZeroClaw | `~/.zeroclaw/workspace/skills` (per-agent workspaces, shared bundles) | UNKNOWN | `.zeroclaw` | 33k★ | **Research further**: workspace layout ≠ one global dir | LIKELY · T43 |
| nanobot | `~/.nanobot/workspace/skills` (+ built-ins) | UNKNOWN | `.nanobot` | 50k★ | **Research further** (same shape as ZeroClaw) | LIKELY · T43 |

The other showcase entries are cloud, library or mobile products with no user skills dir, or are already covered:

- Already covered: VS Code = Copilot.
- No user skills dir: Claude.ai app, Databricks Genie Code, Pulumi Neo, Spring AI, Google AI Edge Gallery, Laravel Boost
  (an installer), Qodo (its toolbox installs into other agents).
- Not checked: Piebald, Agentman, Emdash, Workshop, fast-agent, bub, Vita, Superconductor, Deep Code.

## Part 2 — shared-dir read matrix

Legend:

- **Y** = reads by default. **opt** = only after a user opts in. **N** = does not read (absence source cited). **?** =
  UNKNOWN.
- **Prec.** = what wins when one skill name is in the harness's own dir and a shared dir:
  - **own** = own dir wins
  - **shared** = the shared dir wins
  - **both** = both listed
  - **?** = undocumented
- **A-only** / **C-only** = does deploying *only* to `~/.agents/skills` / *only* to `~/.claude/skills` make the skill
  visible in that harness? "n/a" = that shared dir is the harness's own dir.

### Existing registry rows (B02 order; the `agents_skills` convention row omitted)

| Harness | `~/.agents` | `~/.claude` | proj `.agents` | proj `.claude` | Other cross-reads | Prec. | Disable switch | Since | A-only | C-only | Hazard | Conf · src |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Cursor | Y | Y | Y | Y | `~/.codex/skills`, `.codex/skills` | ? | — | ? | **Yes** (local agents only; not copied to Cloud Agents / SSH / self-hosted workers) | **Yes** | Listed up to 4× if also in `~/.cursor`, `~/.codex` | VERIFIED · T75 |
| Claude Code | N | n/a (own) | N | n/a | Managed-settings dir | enterprise > personal > project | — | — | **No** | **Yes** | — | VERIFIED · B02:S2 |
| Codex | Y (canonical user dir) | N | Y | N | `$CODEX_HOME/skills` (deprecated, read), `/etc/codex/skills` | both | — | rust-v0.95.0 | **Yes** | **No** | Shown twice if also in `~/.codex/skills` | VERIFIED · T49, B02:S4–S5 |
| OpenCode | Y | Y | Y | Y | — | ? | — | ? | **Yes** | **Yes** | Up to 3 copies | VERIFIED · T64 |
| Antigravity | N | N | Y | N | CLI also reads `~/.gemini/skills` (= the **Gemini CLI row's dir**) and `~/.gemini/antigravity-cli/skills`; IDE legacy `~/.gemini/antigravity/skills` | ? | — | — | **No** | **No** | Gemini-row deployments appear in Antigravity CLI | VERIFIED · T71 |
| Amp | Y (rank 2) | Y (rank 6) | Y | Y | `~/.config/agents/skills` (rank 1), `~/.config/amp/skills`, `~/.claude/plugins/cache` | first name wins: `~/.config/agents` > **`~/.agents`** > `~/.config/amp` | `amp.skills.disableGlobalAgentsSkills`, `…disableClaudeCodeSkills` | ? | **Yes** unless disabled | **Yes** unless disabled | — | VERIFIED · T77 |
| Kimi Code CLI | Y | N (current). Legacy `kimi-cli`: first existing of `~/.kimi/skills` › `~/.claude` › `~/.codex` | Y | N | Legacy: `~/.config/agents/skills` | ? (user tier) | — | ? | **Yes** (Kimi Code). Legacy `kimi-cli`: **No** if `~/.config/agents/skills` exists | Legacy only, and only if `~/.kimi/skills` is absent | First-existing-dir rule (legacy) | VERIFIED · T50, B02:S14 |
| Augment | Y | Y | Y | Y | — | `~/.augment` > workspace > `~/.claude` > `~/.agents` | — | ? | **Yes** | **Yes** | — | VERIFIED · T69 |
| OpenClaw | Y (default state only) | N | Y | N | — | **shared** (`~/.agents` rank 3 > `~/.openclaw` rank 4) | Non-default `OPENCLAW_STATE_DIR` drops `~/.agents` | ? | **Yes** (default state) | **No** | — | VERIFIED · T51 |
| Copaw → **QwenPaw** | N | N | N | N | Configured extra skill dirs only | primary pool first | — | — | **No** | **No** | Renamed; root `~/.qwenpaw` unless `~/.copaw` exists (I6) | VERIFIED · T46 |
| Cline | Y | N | Y | Y (+ `.clinerules/skills`) | — | ? (scan order `~/.cline`, then `~/.agents`) | — | v3.67.0 (S) | **Yes** | **No** | — | VERIFIED · T70 |
| CodeBuddy | N | N | N | N | — | project > user | — | — | **No** | **No** | A third party confirms `.claude` paths are never read | VERIFIED (docs) · T78; LIKELY (`.claude`) |
| Command Code | Y | N | Y | N | `--skill`, settings `skills` | **own** (`.commandcode` > `.agents` within a level) | — | ? | **Yes** | **No** | Shadowed copy reported as a duplicate | VERIFIED · T52 |
| Continue | ? | Y | ? | ? | — | ? | — | — | ? | LIKELY yes | Third parties say it is unmaintained | LIKELY · B02:S43 |
| Crush | Y | Y | Y | Y (+ `.cursor/skills`) | `~/.config/agents/skills`, `$CRUSH_SKILLS_DIR`; Windows also `%LOCALAPPDATA%\agents\skills` | ? (listed order) | `disable-skill <name>` | v0.52.0 (B02) | **Yes** | **Yes** | Shares `~/.config/agents/skills` with Amp | VERIFIED · T76 |
| Junie | Y | N | Y (trusted) | N (offers import) | `--skill-location` | ? | — | ? | **Yes** | **No** | — | VERIFIED · T53 |
| iFlow CLI | N | N | N | N | — | project > global | — | — | **No** | **No** | — | VERIFIED · T67 |
| Kiro CLI | N | N | N | N | — | workspace > global | Custom agents load skills only via `resources` | — | **No** | **No** | — | VERIFIED · T68 |
| Kode | ? | Y (compat) | ? | Y (compat) | — | ? | — | — | ? | LIKELY yes | — | LIKELY · B02:S47 |
| MCPJam | Y | Y | Y | Y | — | ? | — | ? | **Yes** | **Yes** | An MCP inspector, not a coding harness | VERIFIED · T44 |
| Mistral Vibe | Y | N | Y (trusted) | N | `skill_paths` | ? | `enabled_skills` / `disabled_skills` | ? | **Yes** | **No** | — | VERIFIED · T54 |
| Mux → **Xum** | Y | ? | ? | ? | `~/.mux/skills` (legacy, lowest) | `~/.xum` > **`~/.agents`** > `~/.mux` | — | ? | **Yes** | ? | — | VERIFIED · B02:S42 |
| OpenClaude IDE | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | No vendor doc found | UNKNOWN |
| OpenHands | Y (recommended user dir) | N | Y | N | Legacy `.openhands/skills`, `.openhands/microagents` | `.agents` beats the legacy dirs | Agent Canvas skill catalog is an **allow-list of recommended skills by default** | ? | **Yes** (CLI / SDK). In Agent Canvas the skill may still need enabling | **No** | — | VERIFIED · T60 |
| Pi | Y | N | Y (ancestors) | N | Settings extra paths | first discovered wins, with warning (docs list `~/.pi/agent` first) | — | ? | **Yes** | **No** | Warning on collision | VERIFIED · T59; order LIKELY |
| Qoder | Y (**desktop app**). IDE/CLI pages list only `~/.qoder/skills` | N | N | N | — | **own** (user-managed dir wins; duplicate shown once) | — | ? | **Yes** for the Qoder app; unverified for the IDE/CLI | **No** | **Contradicts B02** (I6) | VERIFIED · T20 |
| QoderWork | N ("All Skills live in `~/.qoderwork/skills`") | N | N | N | — | — | — | — | **No** | **No** | CN build `~/.qoderworkcn` (B02, LIKELY) | VERIFIED · T21 |
| Qwen Code | Y | opt (`skills.directories: ["~/.claude/skills"]`) | Y | N | `skills.directories` | **own** (`~/.qwen` wins) | — | ? | **Yes** | Only if configured | — | VERIFIED · T65 |
| Trae | N / ? (only project `.agents`, behind a toggle) | ? | opt (Settings → Skills & Commands toggle) | ? | — | ? | the `.agents` toggle | ? | **No** | ? | Windows `%userprofile%\.trae\skills` | VERIFIED (vendor via search) · T48 |
| Trae CN | as Trae | ? | opt | ? | — | ? | same toggle | ? | **No** | ? | Global `~/.trae-cn/skills`; Windows `%userprofile%\.trae-cn\skills` | VERIFIED · T48 |
| Zencoder (+ Zenflow) | Y (its user dir) | ? (project `.claude/skills` yes) | Y | Y | Legacy `.zencoder/skills` (deprecated, still read) | ? | — | ? | **Yes** | ? | Our row targets the deprecated dir (I6) | VERIFIED (vendor via search; direct fetch 403) · T38 |
| Neovate | N | Y | N | Y | — | **own** (`~/.neovate` loaded after `~/.claude`; `Map.set` → later wins) | — | — | **No** | **Yes** | — | VERIFIED · T47 |
| Pochi | Y | N | Y | N | — | **own** | — | commit 2a9ff73 | **Yes** | **No** | — | VERIFIED · T66 |
| AdaL | ? | ? | ? | ? | — | ? | — | — | ? | ? | Own `~/.adal/skills` VERIFIED | VERIFIED (own) · T45 |
| Kilo Code | Y | opt (Claude compat) | Y | opt | `~/.kilo/skills` is current; `~/.kilocode` legacy read is ? | ? | Claude-compat toggle | ? | **Yes** | Only with compat on | Row targets legacy `.kilocode` (#88) | VERIFIED · T61, B02:S24 |
| Roo Code | Y | N | Y | N | `skills-{mode}` variants | **own** (`~/.roo` > `~/.agents`; any project skill beats any global one) | — | PR #11181 | **Yes** | **No** | — | VERIFIED · T55 |
| Goose | Y (recommended) | Y (compat) | Y | Y (compat) | `.goose/skills`, platform config dir (compat), `~/.agents/plugins` | ? | — | ? | **Yes** | **Yes** | — | VERIFIED · T62 |
| Gemini CLI | Y ("alias") | N | Y | N | — | **shared** (within a tier `.agents` beats `.gemini`) | — | PR #18151 | **Yes** | **No** | Its dir `~/.gemini/skills` is also read by Antigravity CLI | VERIFIED · T56 |
| GitHub Copilot | Y | N (project `.claude` only) | Y | Y (+ `.github/skills`) | `/skills add <dir>` | ? | — | ? | **Yes** on local surfaces (CLI, IDE agent mode, app); **not** cloud agent / code review | **No** | `~/.copilot/skills` is read by Devin CLI and Code Studio | VERIFIED · T57 |
| Clawdbot | — | — | — | — | Former OpenClaw name | — | — | — | — | — | **Retire** (operator 2026-11-05) | LIKELY · B02:S7 |
| Droid | Y | N ("Import from Claude" tab = import, not read) | Y | N | `~/.agent/skills`, `.agent/skills` | ? (one personal tier; manager shows "Overridden") | `disabledSkills` | ? | **Yes** | **No** | — | VERIFIED · T58 |
| Windsurf → **Devin** | Y | N (project `.claude/skills` via `read_config_from.claude`) | Y | Y | `~/.codeium/<channel>/skills` (CLI, `read_config_from.windsurf`), `~/.copilot/skills` (`read_config_from.copilot`) | ? | `read_config_from.{windsurf,copilot,claude}` in `~/.config/devin/config.json` | ? | **Yes** (Devin CLI + Desktop) | **No** | Windows `%APPDATA%\devin\skills` (I7) | VERIFIED · T9–T11 |
| MoltBot | — | — | — | — | Former OpenClaw name | — | — | — | — | — | **Retire** | LIKELY · B02:S7 |
| Hermes Agent | opt (`skills.external_dirs`) | opt (any path in `external_dirs`) | ? (trusted project discovery) | ? | `external_dirs` | **own** (`~/.hermes` shadows external dirs) | — | PR #3678 | **No** unless configured | **No** unless configured | Agents can *write* into external dirs | VERIFIED · T63 |

### The four #92 rows

| Harness | `~/.agents` | `~/.claude` | proj `.agents` | proj `.claude` | Other | Prec. | Switch | A-only | C-only | Conf · src |
|---|---|---|---|---|---|---|---|---|---|---|
| DeepSeek Harness | Y (`$DSH_AGENTS_HOME`) | N | Y | N | `customSkillDirs` | **own** (rank 400 < 500; lower rank wins) | — | **Yes** | **No** | VERIFIED · T72 |
| ZCode | Y (product guide) | N (only "Import" from Claude Code / Codex / OpenClaw / Augment / Windsurf dirs) | Y | N | — | **own** | per-skill enable switch | **Yes** | **No** | VERIFIED · T73, B02:S53 |
| Codewhale | Y (read-only) | Y (read-only) | Y | Y | `~/.deepseek/skills` legacy; project `.opencode`, `.cursor` | **own** (`.codewhale` > `.agents` > `.claude` > `.deepseek`) | — | **Yes** | **Yes** | VERIFIED · T74 |
| WorkBuddy | ? | ? | ? | ? | — | ? | — | ? | ? | LIKELY · B02:S57 |

### Recommended Part 1 additions

| Harness | `~/.agents` | `~/.claude` | proj `.agents` | proj `.claude` | Other cross-reads | Prec. | Switch | Since | A-only | C-only | Hazard | Conf · src |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Grok Build | Y | Y | Y | Y | `~/.cursor/skills`, `.cursor/skills`; `[skills] paths` | **own** (`~/.grok` first-seen wins over `~/.agents`) | `[compat.claude].skills=false`, `GROK_CLAUDE_SKILLS_ENABLED` | ? | **Yes** | **Yes** | — | VERIFIED · T14 |
| Deep Agents Code | Y | Y ("experimental") | Y | Y (experimental) | — | **shared** (`~/.agents` overrides `~/.deepagents/<agent>`) | — | ? | **Yes** (see I8) | **Yes** (experimental) | `~/.agents` resolves from the launch home, not `DEEPAGENTS_HOME`. **Symlinks resolving outside the skill roots are rejected** until approved (I8) | VERIFIED · T8 |
| Reasonix | Y | Y | Y | Y (+ `.agent`) | `~/.agent/skills`, `[skills] paths` | **own** LIKELY (global list order puts the Reasonix home first) | — | ? | **Yes** | **Yes** | Windows `%APPDATA%\reasonix` | VERIFIED · T27 |
| Cortex Code | N | Y | N | Y | — | **both, flagged**: differing copies show a "conflict" indicator; `/skill sync` resolves | — | ? | **No** | **Yes** | — | VERIFIED · T5 |
| Rovo Dev | Y | N | Y | N | — | ? (one user tier; built-in > user > project) | — | ? | **Yes** | **No** | — | VERIFIED · T15 |
| IBM Bob | N | N | N | N | — | project > global | — | — | **No** | **No** | — | VERIFIED · T4 |
| ForgeCode | Y | N | N | N | — | **shared** (`~/.agents` > `~/.forge`) | — | PR #2890 (2026-04-08) | **Yes** | **No** | — | VERIFIED · T13 |
| Firebender | Y | Y | Y | Y | `~/.codex`, `~/.cursor`, `~/.goose` (+ project equivalents) | ? | — | ? | **Yes** | **Yes** | Up to 6 copies | VERIFIED · T12 |
| MiniMax Code | Y (priority 30) | Y (40) | Y (55) | Y (60) | `~/.codex/skills` (35) | ? for own vs external (own root has no explicit priority); among externals `~/.claude` > `~/.codex` > `~/.agents` | `skills.external.enabled` / `.sources.<kind>.enabled` | ? | **Yes** unless disabled | **Yes** unless disabled | `duplicateWarn` logs collisions | VERIFIED · T24 |
| Posit Assistant | Y | N | Y | N | — | **own** (`.posit/assistant` wins at the same level) | `skills.paths` *replaces* the defaults | ? | **Yes** unless `skills.paths` omits it | **No** | — | VERIFIED · T18 |
| AiderDesk | N | N | N | N | — | extension > project > global > built-in | "Use Skills Tools" must be on per agent profile | — | **No** | **No** | Skills are off unless Skills Tools are enabled | VERIFIED · T3 |
| Autohand Code CLI | Y | Y | Y | Y (+ ~40 third-party project dirs) | `~/.codex/skills`, `~/.agent/skills` | **own** (later wins; `~/.autohand` loads last) | — | ? | **Yes** | **Yes** | — | VERIFIED · T25 |
| Code Studio | Y | Y | Y | Y (+ `.github/skills`) | `~/.copilot/skills` | ? | "Skill File Locations" setting | ? | **Yes** | **Yes** | Reads the Copilot row's dir | VERIFIED (via search) · T7 |
| CodeArts Agent | N | N | N | N | — | cloud enterprise > team > project > user > cloud user > built-in | — | — | **No** | **No** | — | VERIFIED · T6 |
| Qoder CN (IDE, ex-Lingma) | N (IDE page) | N | N | N | — | project > user | — | — | **No** | **No** | The CN *desktop app* reads `~/.agents` | VERIFIED · T19 |
| Qoder CN CLI | N (CLI page) | N | N | N | — | project > user | — | — | **No** | **No** | — | VERIFIED · T19 |
| Kimchi (optional) | Y | opt (`kimchi resources enable extensions.claude-code-skills`) | Y | opt | `~/.pi/agent`, `~/.codex`, `~/.cursor`, `~/.warp`, `~/.factory`, `~/.gemini`, `~/.copilot`, `~/.github` (each "if installed") | earlier `skillPaths` entry wins | the extension toggle; `skillPaths` | ? | **Yes** | Only if enabled | Reads many Tool dirs: heavy duplicates | VERIFIED · T28 |
| AstrBot (optional) | N | N | N | N | — | workspace > local > plugin > sandbox | — | — | **No** | **No** | Root is the cwd for non-desktop installs | VERIFIED · T26 |
| Warp (AGENTS) | Y | Y | Y | Y | `~/.warp`, `~/.codex`, `~/.cursor`, `~/.gemini`, `~/.copilot`, `~/.factory`, `~/.github`, `~/.opencode` | **both** shown to the agent; background resolution prefers home skills | — | ? | **Yes** | **Yes** | Every copy in any Tool dir is listed | VERIFIED · T22 |
| Zed (AGENTS) | Y (only global dir) | N | Y | N | none, by design | n/a | — | ? | **Yes** | **No** | Symlinked `SKILL.md` not discovered (zed#64992, S); a symlinked skill *dir* is fine | VERIFIED · T23 |
| Dexto (AGENTS) | Y | N | Y | N | legacy `~/.dexto/skills`, Claude plugin skills | ? ("no source precedence rules") | — | ? | **Yes** | **No** | — | VERIFIED · T34 |
| fx (AGENTS) | Y | Y | Y | Y | `~/.fx`, `~/.config/opencode`, `~/.codex`, `~/.claw` | **both**: an ambiguous `$name` loads **none** | `skill_symlink_authorities` (symlinks outside home) | ? | **Yes** | **Yes** | Duplicates break explicit invocation; symlink targets must resolve inside home or be listed | VERIFIED · T35 |
| Sarvam Code (AGENTS) | Y (LIKELY) | ? | ? | ? | — | ? | — | — | LIKELY yes | ? | Invite-gated | LIKELY · T1, T37 |
| oh-my-pi (extra) | Y | Y | Y | Y | `~/.agent`, `~/.codex`, `~/.config/opencode`; Pi user dir (`enablePiUser`) ? (not among the listed locations) | **own** ("highest-priority provider: native `.omp`, …, Claude, Agent/Codex-compatible, OpenCode, GitHub"; `/extensions` shows the shadowed copy) | `skills.enable{Pi,Agents,Claude,Codex}{User,Project}` | ? | **Yes** | **Yes** | — | VERIFIED · T41 (precedence re-read 2026-11-06; was LIKELY from the blog) |
| Letta Code (extra) | N | N | Y | N | agent-memory skills | project > agent > global > bundled | — | — | **No** | **No** | — | VERIFIED · T40 |
| VT Code (extra) | Y (only) | N | Y | N | `/etc/codex/skills`; disabled list in `~/.codex/config.toml` | n/a | — | ? | **Yes** | **No** | Reads Codex's config file | VERIFIED · T42 |

## Implications for #92 and the shared-dir toggle

- **I1 — Registry facts.** `reads_agents_standard_globally` needs four states, not a bool: `yes`, `opt_in`, `no`,
  `unknown`. A fifth, `yes_with_condition`, would cover:
  - OpenClaw (default state only)
  - Copilot and Cursor (local surfaces only)
  - Posit (unless `skills.paths` overrides)
  - Kimi (legacy `kimi-cli`'s first-existing rule)

  The toggle should treat anything but `yes` as "keep the harness-specific deployment".
  - **`also_reads`** should list concrete home-relative dirs: `.claude/skills`, `.codex/skills`, `.cursor/skills`,
    `.copilot/skills`, `.gemini/skills`, `.config/opencode/skills`, `.config/agents/skills`,
    `.codeium/windsurf/skills`, `.factory/skills`, `.pi/agent/skills`, `.goose/skills`, `.warp/skills`. Then the
    cross-read warning can match them against *any* Tool row's `relative_skills_dir`, not just the two shared dirs.
  - **Cross-reads that hit another row's own dir:**
    - Antigravity CLI → Gemini CLI's dir
    - Cursor, MiniMax, Firebender, fx, Warp, omp, Autohand → the Codex row's `~/.codex/skills`
    - Devin, Code Studio, Warp, Kimchi → Copilot's dir
    - Warp, Kimchi → Droid's dir
    - Kimchi → Pi's dir
    - Firebender → Cursor's dir
    - fx, omp → OpenCode's dir
    - Amp, Crush → each other's `~/.config/agents/skills`
  - A precedence enum `own | shared | both | unknown` belongs beside it.
- **I2 — Sufficiency for the toggle.** "Deploy only to `~/.agents/skills`" is safe by default for every `A-only = Yes`
  row in § Part 2. It must **not** silently stop deploying to:
  - Claude Code, Antigravity, Kiro, iFlow, CodeBuddy, QoderWork, Trae, Trae CN, Neovate, QwenPaw, Hermes
  - IBM Bob, AiderDesk, CodeArts, Cortex Code, Letta, AstrBot, both Qoder CN surfaces
  - the UNKNOWN rows (Continue, Kode, OpenClaude, AdaL, WorkBuddy, Sarvam)

  For **Cortex Code, Neovate** (and Claude Code), `~/.claude/skills` is the shared dir that suffices, so a "prefer
  shared dirs" toggle needs both shared dirs as targets. Then each harness gets its skill from exactly one dir it reads.
- **I3 — The toggle must pick one shared dir per harness, not both.**
  - **Both shared dirs** duplicate in Cursor, OpenCode, Amp, Augment, Crush, Goose, MCPJam, Codewhale, Grok,
    Firebender, Reasonix, MiniMax, Autohand, Code Studio, fx, Warp, omp and Deep Agents. fx then cannot invoke the
    skill by `$name` at all.
  - **Suggested rule:** deploy to `~/.agents/skills` when any selected harness has `A-only = Yes`. Deploy to
    `~/.claude/skills` only for selected harnesses that are `A-only = No` and `C-only = Yes` (today: Claude Code, Cortex
    Code, Neovate). Keep the harness-specific dir for the rest.
  - **A Claude Code user selected alongside Cursor/OpenCode/etc.** gets a duplicate in those harnesses. That
    combination is exactly the cross-read warning's job (no auto-deselect, per #92).
- **I4 — Detect-dir hazards for new rows:**
  - **Cortex:** detect `.snowflake/cortex`, never `.snowflake` (Snowflake CLI).
  - **Posit:** detect `.posit/assistant` and `.positai`, never `.posit`.
  - **Tabnine** (if ever added): `.tabnine` is shared with the IDE plugins.
  - **AstrBot:** `~/.astrbot` exists only for the packaged desktop runtime.
  - **ForgeCode:** also `forge` (the pre-2026-04 un-dotted dir).
  - **MiniMax:** `.minimax` may be shared with other MiniMax products (UNKNOWN).
  - **Code Studio and CodeArts:** verify on a real install that `.codestudio` / `.codeartsdoer` hold more than skills,
    or they fail `is_installed_in`'s skills-only-footprint rule.
  - **Warp:** `.warp` is the macOS root; Linux and Windows roots are UNKNOWN.
  - **Deep Agents, Grok, Autohand, Reasonix, AiderDesk:** env-var homes (`DEEPAGENTS_HOME`, `GROK_HOME`,
    `AUTOHAND_HOME`, `REASONIX_HOME`, `AIDER_DESK_HOME_DIR`) are outside the registry's model. Note them, as B02 did
    for DSH.
- **I5 — Windsurf → Devin.**
  - Devin for Terminal (CLI) and Devin Desktop ("Devin Local", the old Cascade) are the Windsurf row's product.
    Windsurf auth is now "legacy".
  - Devin CLI still reads `~/.codeium/windsurf/skills` (stable channel; `-next` / `-insiders` read their own channel
    dirs), switchable by `read_config_from.windsurf`. So the row's current dir works for the CLI. Devin Desktop
    documents only `~/.config/devin/skills` + `~/.agents/skills`.
  - **Recommendation:** relabel the row "Devin (Windsurf)", add detect `.config/devin`, and keep `.codeium/windsurf` as
    detect. Prefer AGENTS-group membership as the global target over moving to `.config/devin/skills`. Moving is
    #88's job, and the Windows path differs (I7). The project dir `.windsurf/skills` is "legacy" in the Desktop docs:
    `.devin/skills` is current and `.agents/skills` is read too.
- **I6 — Rows B02 got wrong or that changed** (#92 should carry these):
  - **Qoder** reads `~/.agents/skills` (desktop app, T20). Move it from B02's "never group" to `yes_with_condition`
    until the IDE/CLI pages say so.
  - **Zencoder:** `~/.agents/skills` is its user dir and `.zencoder/skills` is deprecated. The row targets a deprecated
    dir. Make it an AGENTS member (a dir move is #88's), and alias Zenflow onto it.
  - **Copaw → QwenPaw:**
    - Display name "QwenPaw".
    - Detect `.qwenpaw` + `.copaw`.
    - Global dir is `.qwenpaw/skill_pool` for new installs and `.copaw/skill_pool` for legacy ones. Only the legacy dir
      is right today.
    - The relocation is #88's. Also note that pool skills may need a `skill.json` manifest and a per-workspace
      "broadcast" to reach an agent. That is UNKNOWN; verify before promising deployment works.
  - **Windsurf** `~/.codeium/windsurf/skills`: LIKELY → VERIFIED read (Devin CLI).
  - **QoderWork** (`~/.qoderwork/skills`, no shared reads) and **Trae / Trae CN** (own dirs; project `.agents` behind a
    toggle) move from LIKELY to VERIFIED.
  - **MCPJam:** UNKNOWN → VERIFIED (reads `~/.agents` and `~/.claude`). It is an MCP inspector, so keeping it as a
    row is an operator call.
  - **Neovate:** reads `~/.claude/skills`, not `~/.agents`.
  - **AdaL:** own dir VERIFIED; shared reads UNKNOWN.
  - **Copilot:** B02 said "and the other surfaces". The current docs say supported locations "depend on the surface",
    and the cloud agent does not see personal dirs.
  - **Cursor:** does not copy `~/.agents/skills` to Cloud Agents.
- **I7 — Windows.** The registry is home-relative only. These candidates break it on Windows:
  - **Devin:** `%APPDATA%\devin\skills`, config `%APPDATA%\devin`.
  - **Reasonix:** `%APPDATA%\reasonix`.
  - **Zed's detect root:** `%APPDATA%\Zed`. Its skills dir `~/.agents/skills` is fine.
  - **Crush** also reads `%LOCALAPPDATA%\agents\skills` and `%LOCALAPPDATA%\crush\skills` on Windows. Its `~/.config`
    dirs are documented too.

  The others that document Windows use `%USERPROFILE%\.<tool>` (Roo, Junie, Grok, Trae, CodeArts).
  - **Options:** a per-OS override field on `ToolAdapter`, or ship those rows as AGENTS members, which sidesteps the
    problem for Devin and Zed. Reasonix needs its own dir only for Windows users without `~/.agents`. It reads
    `~/.agents/skills` everywhere.
- **I8 — Symlink containment** (Skills Hub deploys by symlink into `~/.skillshub`):
  - **Deep Agents Code** rejects a skill whose resolved path leaves its skill roots. It asks in interactive mode and
    denies the turn headless, unless the target is in `[skills].extra_allowed_dirs` or `DEEPAGENTS_CODE_EXTRA_SKILLS_DIRS`
    (T8). A symlink deployment there needs the central repo allow-listed, or a copy.
  - **fx** follows a symlinked skill folder only when it resolves inside the folder holding the root (home or
    workspace), or inside `skill_symlink_authorities`. `~/.skillshub` is under home, so it passes; a relocated central
    repo outside home would not (T35). It never follows symlinks in `~/.fx/skills`.
  - **OpenClaw** accepts symlinked skill folders in `~/.agents/skills` as long as `SKILL.md` stays inside its resolved
    folder (T51).
  - **Zed** reads resource files of global skills through a canonicalized fast path under `~/.agents/skills`. Whether a
    symlinked skill dir (canonicalizing into `~/.skillshub`) can read its supporting files is UNKNOWN (T23).
  - This is a deployment-mode fact the registry does not model today (`supports_symlink` is per entry). It applies to
    `~/.agents/skills` as a whole once these harnesses read it.
- **I9 — Version gates.** "Since" is still mostly UNKNOWN. Newly VERIFIED: ForgeCode `~/.agents` (PR #2890,
  2026-04-08). B02's Codex / Gemini / Roo / Pochi dates stand. The toggle should not gate on versions it cannot
  observe. Prefer the I1 `yes` fact plus the cross-read warning.

## Sources

All fetched 2026-11-05 unless marked B02 (fetched 2026-11-02, reused). P = primary (vendor doc, release, source, vendor
staff); S = secondary. "(via search)" = the vendor page's text as returned by a search index because direct fetch was
blocked or JS-rendered.

| Id | URL | Kind | Note |
|---|---|---|---|
| T1 | https://raw.githubusercontent.com/vercel-labs/skills/main/src/agents.ts (clone `/tmp/vskills` at `cabe072`, 2026-11-05) | S | Candidate source; never upgraded |
| T2 | https://github.com/vercel-labs/skills/pull/1023 ; …/pull/849 ; …/pull/868 ; …/pull/928 ; …/issues/1319 | S | Agent-add PRs (Codemaker, Rovo "also reads `.agents`", Devin, Qoder CN) |
| T3 | https://aiderdesk.hotovo.com/docs/agent-mode/skills ; https://github.com/hotovo/aider-desk/blob/main/packages/extensions/extensions/skill-gallery/index.ts | P | Dirs, precedence, `AIDER_DESK_HOME_DIR` |
| T4 | https://bob.ibm.com/docs/shell/features/skills | P | `~/.bob/skills`; project > global |
| T5 | https://docs.snowflake.com/en/user-guide/cortex-code/extensibility | P | Skill locations table, conflict indicator |
| T6 | https://support.huaweicloud.com/intl/en-us/usermanual-codeartsagent/codeartsagent_ug_0024.html | P | `%USERPROFILE%/.codeartsdoer/skills`, priority list |
| T7 | https://help.syncfusion.com/code-studio/customization/skills ; …/usersettings ; https://codestudio.syncfusion.com/kb/article/23692/… (via search) | P | Personal and project paths; "Skill File Locations" |
| T8 | https://docs.langchain.com/oss/deepagents/code/memory-and-skills ; https://docs.langchain.com/oss/deepagents/code/configuration | P | Discovery list, precedence, `DEEPAGENTS_HOME` |
| T9 | https://cli.devin.ai/docs/extensibility/skills/overview (= docs.devin.ai/cli/…) | P | Global dirs incl. `~/.codeium/<channel>/skills`; Windows `%APPDATA%\devin` |
| T10 | https://docs.devin.ai/desktop/cascade/skills.md | P | Devin Desktop scopes; `.windsurf` legacy |
| T11 | https://docs.devin.ai/cli/reference/configuration/read-config-from.md ; https://docs.devin.ai/llms.txt | P | `read_config_from` switches; "Legacy Windsurf" |
| T12 | https://docs.firebender.com/api-reference/skills | P | Cross-compatibility dirs; TEAM > USER > PLUGIN |
| T13 | https://github.com/antinomyhq/forgecode/pull/2890 ; `crates/forge_repo/src/skill.rs` ; `crates/forge_domain/src/env.rs` ; …/pull/2817 | P | `~/.agents` added 2026-04-08, precedence; `~/.forge` base path |
| T14 | https://docs.x.ai/build/features/skills-plugins-marketplaces ; https://github.com/xai-org/grok-build `crates/codegen/xai-grok-pager/docs/user-guide/08-skills.md` + `crates/codegen/xai-grok-agent/src/prompt/skills.rs` ; https://docs.x.ai/build/settings (via search) | P | Locations, compat switches, "first-seen wins" (`~/.grok` vs `~/.agents`), `GROK_HOME`, Windows path |
| T15 | https://support.atlassian.com/rovo/docs/extend-rovo-dev-cli-with-agent-skills/ | P | User tier = `~/.rovodev` or `~/.agents` |
| T16 | https://docs.tabnine.com/main/getting-started/tabnine-cli/features/agent-skills | P | Maintenance mode / EOL; priority table |
| T17 | https://ona.com/docs/ona/agents/skills | P | Repo / org / built-in sources only |
| T18 | http://assistant.posit.co/docs/features/skills/ | P | Dirs, order, `skills.paths`, `~/.positai` legacy |
| T19 | https://docs.qoder.cn/user-guide/skills ; https://help.aliyun.com/en/lingma/qoder-cn/user-guide/skills ; https://help.aliyun.com/zh/lingma/qoder-cn/user-guide/skills (via search) ; https://docs.qoder.cn/qoder/skills | P | Qoder CN IDE `~/.lingma/skills`; CLI `~/.qoder-cn/skills`; CN app reads `~/.agents` |
| T20 | https://docs.qoder.com/qoder/skills ; https://docs.qoder.com/extensions/skills ; https://docs.qoder.com/cli/Skills ; https://docs.qoder.com/llms.txt | P | Qoder app reads `~/.agents`; IDE/CLI list only `~/.qoder/skills` |
| T21 | https://docs.qoder.com/qoderwork/skills | P | "All Skills live in `~/.qoderwork/skills/`" |
| T22 | https://docs.warp.dev/agents/capabilities/skills/ | P | Supported dir names, home scope, conflicts |
| T23 | https://github.com/zed-industries/zed/blob/main/crates/agent_skills/README.md ; zed issue #64992 (S) | P | "Only `.agents/skills`" |
| T24 | https://github.com/MiniMax-AI/minimax-code `docs/tui-capabilities.md`, `packages/config/src/skills-config.ts`, `packages/local-runtime/src/skills/roots.ts` | P | External source kinds and priorities |
| T25 | https://docs.autohand.ai/working-with-autohand-code/agent-skills ; https://github.com/autohandai/code-cli/blob/056d8fb6/src/constants.ts | P | Load order (later wins), `AUTOHAND_HOME` |
| T26 | https://docs.astrbot.app/en/use/skills.html ; https://github.com/AstrBotDevs/AstrBot `astrbot/core/utils/astrbot_path.py` | P | `data/skills`; root = env / desktop `~/.astrbot` / cwd |
| T27 | https://raw.githubusercontent.com/esengine/DeepSeek-Reasonix/master/docs/SKILLS.md | P | Scopes incl. Windows `%APPDATA%\reasonix` |
| T28 | https://docs.kimchi.dev/docs/coding-skills ; https://cast.ai/kimchi/ (S) | P | Discovery table, `skillPaths` precedence |
| T29 | https://jazz-cli.vercel.app/docs/concepts/skills.md | P | Locations |
| T30 | https://moxby.com/support/use-and-create-skills | P | Marketplace-only skills |
| T31 | https://inference.sh/docs/skills/other-agents | P | `belt` installs into other agents |
| T32 | https://www.tinycloud.sh/ ; https://terramind.com/ ; https://aur.archlinux.org/packages/terramind | P/S | Product shape; dead CLI upstream |
| T33 | https://163.com/y3/docs/guides/Editor/AI_Development/AssistantDevelopment (via search) | S | NetEase CodeMaker context; no skills path |
| T34 | https://cli.dexto.ai/docs/architecture/skills | P | Canonical and legacy roots |
| T35 | https://fx.sh/docs/capabilities/skills | P | Discovery roots, ambiguous-name rule, symlink rule |
| T36 | https://www.mintlify.com/veyzyn/loaf/customization/creating-skills (project docs) | P | Locations |
| T37 | https://ojasviyadav.com/reverse-engineering-sarvam-code.html ; https://phaser.io/cli/sarvam-code | S | Codex fork, `~/.sarvam`; third party lists it as an `~/.agents` reader |
| T38 | https://docs.zencoder.ai/features/skills (via search; direct fetch 403) ; https://docs.zencoder.ai/zenflow/skills (via search) | P | User-level `~/.agents/skills`; `.zencoder/skills` deprecated |
| T39 | https://github.com/agentskills/agentskills `docs/snippets/clients.jsx` | S | Client showcase list |
| T40 | https://docs.letta.com/configuration/skills/index.md | P | Skill scopes |
| T41 | https://omp.sh/docs/skills ; https://maxtokens.ai/posts/oh-my-pi-setup/ (S) | P | Locations, per-source switches; precedence (re-fetched 2026-11-06: "keeps the highest-priority provider: native `.omp` …"; the blog (S) agrees) |
| T42 | https://github.com/vinhnx/VTCode/blob/main/docs/skills/SKILLS_GUIDE.md | P | Discovery order |
| T43 | https://github.com/zeroclaw-labs/zeroclaw/discussions/2149 ; https://github.com/HKUDS/nanobot/blob/main/nanobot/agent/skills.py | P/S | Workspace skill dirs |
| T44 | https://www.mcpjam.com/blog/skills ; https://docs.mcpjam.com/inspector/skills | P | Global and project dirs |
| T45 | https://github.com/SylphAI-Inc/skills/blob/main/skills/create-skill/SKILL.md | P | `~/.adal/skills` |
| T46 | https://github.com/agentscope-ai/QwenPaw `src/qwenpaw/constant.py` (L82–94), `src/qwenpaw/agents/skill_system/store.py` | P | Rename, working-dir rule, pool roots |
| T47 | https://github.com/neovateai/neovate-code `src/skill.ts` | P | `GlobalClaude` then `Global` load; `Map.set` |
| T48 | https://docs.trae.ai/ide/skills (via search) ; https://docs.trae.cn/ide_skills (via search) ; https://forum.trae.cn/t/topic/19344 (staff, S) | P/S | Global dirs per OS; `.agents` toggle |
| T49 | https://developers.openai.com/codex/skills | P | Re-check: locations, no `.claude` |
| T50 | https://www.kimi.com/code/docs/en/kimi-code-cli/customization/skills.html | P | Re-check: tiers, no `.claude` |
| T51 | https://docs.openclaw.ai/tools/skills | P | Re-check: loading order, state-dir rule |
| T52 | https://commandcode.ai/docs/skills | P | Re-check |
| T53 | https://junie.jetbrains.com/docs/agent-skills.html | P | `~/.agents`; `.claude` import only; Windows path |
| T54 | https://docs.mistral.ai/vibe/code/cli/skills | P | Re-check |
| T55 | https://docs.roocode.com/features/skills | P | Priority list; Windows paths |
| T56 | https://geminicli.com/docs/cli/skills/ | P | Re-check: precedence |
| T57 | https://docs.github.com/en/copilot/concepts/agents/about-agent-skills ; https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-skills | P | Personal dirs; surface caveat |
| T58 | https://docs.factory.com/harness/skills | P | Personal compatibility dirs; Overridden status |
| T59 | https://raw.githubusercontent.com/badlogic/pi-mono/main/packages/coding-agent/docs/skills.md | P | First-discovered wins |
| T60 | https://docs.openhands.dev/overview/skills | P | Precedence; recommended allow-list |
| T61 | https://kilo.ai/docs/customize/skills | P | `~/.agents` default; trusted global dirs |
| T62 | https://goose-docs.ai/docs/guides/context-engineering/using-skills/ | P | Re-check: backward-compat list |
| T63 | https://hermes-agent.nousresearch.com/docs/user-guide/features/skills | P | `external_dirs`, precedence, write-through |
| T64 | https://opencode.ai/docs/skills/ | P | Re-check |
| T65 | https://qwenlm.github.io/qwen-code-docs/en/users/features/skills/ | P | `skills.directories` opt-in for `~/.claude` |
| T66 | https://docs.getpochi.com/skills/ | P | Priority list |
| T67 | https://platform.iflow.cn/en/cli/examples/skill | P | Re-check (absence) |
| T68 | https://kiro.dev/docs/skills/ | P | Re-check (absence) |
| T69 | https://docs.augmentcode.com/cli/skills | P | Re-check: precedence |
| T70 | https://raw.githubusercontent.com/cline/cline/main/apps/vscode/src/core/storage/skill-directories.ts | P | Scan dirs (no global `.claude`) |
| T71 | https://antigravity.google/docs/skills (raw HTML, path grep) | P | No `~/.agents`, no `.claude` |
| T72 | https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/subsystems/skills.md | P | Rank table; lower rank wins |
| T73 | https://zcode.z.ai/en/docs/skill | P | `~/.zcode/skills`; Import from other agents |
| T74 | https://raw.githubusercontent.com/codewhale-hq/Codewhale/main/docs/SKILLS.md | P | Owned vs read-only compatible roots |
| T75 | https://cursor.com/docs/context/skills | P | Dirs; compat dirs; Cloud Agents caveat |
| T76 | https://raw.githubusercontent.com/charmbracelet/crush/main/README.md | P | Global paths incl. Windows |
| T77 | https://ampcode.com/docs/customize/skills | P | Precedence list; disable switches |
| T78 | https://www.codebuddy.ai/docs/cli/skills (via search) ; https://github.com/multica-ai/multica/pull/5224 (S) | P/S | Two dirs; `.claude` not read (S) |
| T79 | `gh api repos/<owner>/<repo>` for each repo named above | P (metadata) | Stars / last push, 2026-11-05 |
| B02:Sn | `docs/design-inputs/2026-11-02-harness-global-skills-dirs.md` § Sources | — | Reused VERIFIED rows (S2, S4–S7, S9, S12, S14, S24, S42–S43, S47, S53, S57) |
