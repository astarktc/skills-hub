# Design inputs — ACP-registry harness candidates: skills support and shared-dir reads (2026-11-05)

An addendum to `2026-11-02-harness-global-skills-dirs.md`. It covers 15 coding agents not in `TOOL_ADAPTERS`, and it feeds
BACKLOG #92 (new rows and the `reads_agents_standard_globally` / `also_reads` registry facts) and the future Settings
toggle that deploys only to the shared dirs. It is a read-only survey: no code changed. **Provenance**: the candidate list
is the ACP registry (`agentclientprotocol/registry` at HEAD **a44e8fc**, 2026-11-05; each agent's `agent.json` gives its
repo and homepage) plus **Muse Code**, which appears as a provider in T3 Code and is absent from the registry. A delegated
child researched it directly on 2026-11-05. It has no Quartermaster source file, so fix a wrong claim here, with its
source row. **Confidence**: **VERIFIED** means a vendor doc, a release note, or the agent's own source. "Own source"
includes the vendor's shipped bundle or binary when no repo is public; the row says so. **LIKELY** means a secondary or
indirect source. **UNKNOWN** means no source was found. An ACP listing was **not** taken to imply skills support: each
row establishes support first. Paths are home-relative unless absolute. "Shared dirs" means `~/.agents/skills` and
`~/.claude/skills`.

## Summary

1. **13 of 15 support Agent Skills** (`SKILL.md` folders, progressive disclosure). **GLM Agent is partial**: it turns
   `.claude/skills/*/SKILL.md` into ACP slash commands only, and the model cannot invoke them. **Agoragentic is no**: it
   is a marketplace bridge that reads no local files.
2. **New row (4), recommended first:**
   - **Muse Code** (Meta). Own dir `~/.config/muse/skills`. Also reads both shared dirs.
   - **Poolside** (`pool`). Own dir `~/.config/poolside/skills`. Also reads `~/.agents/skills`, since 1.0.11 (2026-07-09).
   - **Dirac**. Own dir `~/.dirac/skills`. Also reads both shared dirs, and they **beat** its own dir.
   - **Stakpak**. Own dir `~/.stakpak/skills`, and reads no shared dir. It is low priority because its last release was
     2026-06-10.

   The first three would carry `group: AgentsStandard`, because each reads project `.agents/skills`.
3. **AGENTS-group member only (4):** VT Code, DimCode, crow-cli and Corust Agent. Each reads `~/.agents/skills` as a user
   dir, and their own global dirs either don't fit a fixed home-relative registry path or are in-app install roots. The
   `agents_skills` row already deploys to them. Only the group label/count and README need a mention.
4. **Skip (7):**
   - fast-agent has no global dir by default: its "home" is `./.fast-agent`, relative to cwd.
   - siGit Code and Minion Code are low-signal, and both already read `~/.claude/skills`.
   - Harn is a language runtime with its own `~/.harn/skills` and 29 stars.
   - Nova is proprietary with 14 stars. Its dir is conditional (`~/.nova` else `~/.compass`), and it tells users to move
     skills *out* of shared dirs.
   - GLM Agent is partial.
   - Agoragentic has no skills support.
5. **Aliases: none.** No candidate is a rename of an existing row. Dirac is a **Cline fork** (it benchmarks itself
   against Cline, and its loader matches Cline's shape), but it is a separate product with its own `~/.dirac` root. Do not
   alias it to `cline`.
6. **Shared-dir-only deploy would suffice for every recommended row except Stakpak:**
   - Muse Code: `~/.agents/skills`, or `~/.claude/skills` unless a user preference turns that off.
   - Poolside: `~/.agents/skills`.
   - Dirac: either shared dir, and the shared copy wins.
   - All four AGENTS-only members: `~/.agents/skills`.
   - Stakpak reads no shared dir. The skipped siGit and Minion are reached through `~/.claude/skills`.
7. **Precedence varies here too:**
   - Own dir wins: siGit, DimCode, Corust (own dir listed first, first match kept), Harn.
   - Shared dir wins: Dirac (`~/.ai` > `~/.claude` > `~/.agents` > `~/.dirac`), Minion (`~/.claude` before `~/.minion`),
     VT Code (`~/.agents` before its data dir), fast-agent (`.claude` > `.agents` > `.fast-agent`, project scope).
   - UNKNOWN: Muse Code, Poolside.

   So with a row plus a shared-dir deploy of the same skill, **Dirac would silently use the shared copy**. Muse Code would
   see it under both a user and a foreign root. Collisions are not documented for either.
8. **Still UNKNOWN:** Muse Code and Poolside collision rules. Muse Code's Windows default for `$XDG_CONFIG_HOME`. Which
   "other tools' directories" Poolside also scans (its docs say only that it scans some). Whether Corust reads
   `.claude/skills` (a `.claude` literal sits beside its skills-discovery constants, and its own docs omit it).

## § Candidates

"Rec." is the recommendation. "Detect" is the tool's own config root (AGENTS.md invariant "New AI tool adapter": never a
skills-only footprint, never a shared convention dir).

| # | Candidate (ACP id) | Vendor · kind | Skills? | Global skills dir(s) (+ overrides) | Detect dir | Project skills dir(s) | Relevance signal | Rec. | Confidence | Src |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | **Muse Code** (not in ACP registry) | Meta · CLI (native binary, macOS/Linux/Windows) + SDK | Yes | `$XDG_CONFIG_HOME/muse/skills` (default `~/.config/muse/skills`), `~/.agents/skills`; also `~/.claude/skills` and `$CODEX_HOME/skills` (fallback `~/.codex/skills`) unless a user preference or rollout gate turns off "foreign personal skill roots" | `~/.config/muse` (holds `settings.json`) | `<repo>/.agents/skills` (documented home), plus repo-local `.codex/skills` and `.claude/skills` | Meta-backed. GA 2026-09-01 (launched 2026-08-05). Mature docs. Built-in `muse skills import --from claude\|codex` | **New row** (`group: AgentsStandard`, global `.config/muse/skills`, detect `.config/muse`) | VERIFIED (docs). Windows default for `XDG_CONFIG_HOME` UNKNOWN | P1, P2, S1 |
| 2 | **Poolside** (`poolside`) | Poolside · CLI `pool` (also ACP server, Poolside Assistant) | Yes | `~/.config/poolside/skills/`, `~/.agents/skills/` (since 1.0.11). "Also scans directories used by other tools" (unnamed) | `~/.config/poolside` (holds `settings.yaml`, the only file `pool` settings are read from) | `.poolside/skills/`, `.agents/skills/` (1.0.x changelog also: "Discover skills from .agent/skills") | Well-funded vendor. 441★ release-only repo. v1.0.16 2026-08-14. Full docs site | **New row** (`group: AgentsStandard`, global `.config/poolside/skills`, detect `.config/poolside`) | VERIFIED (docs + changelog). Binary is closed | P3, P4, P5 |
| 3 | **Dirac** (`dirac`) | Dirac Delta Labs · VS Code extension + CLI (`dirac-cli`, ACP) — Cline fork | Yes | `~/.dirac/skills`, `~/.agents/skills`, `~/.claude/skills`, `~/.ai/skills`. No env override for the skills root (`getDiracHomePath` = `homedir/.dirac`; `DIRAC_DIR` covers only tools) | `~/.dirac` | `.diracrules/skills`, `.dirac/skills`, `.claude/skills`, `.ai/skills`, `.agents/skills` | 1,550★. v0.5.24 2026-10-24, pushed 2026-11-02. ~4.2k npm/month (CLI) | **New row** (`group: AgentsStandard`, global `.dirac/skills`, detect `.dirac`) | VERIFIED (source) | P6, P7, P8 |
| 4 | **Stakpak** (`stakpak`) | Stakpak · CLI DevOps agent (Rust) | Yes | `$HOME/.stakpak/skills` only | `~/.stakpak` | `.stakpak/skills` (cwd-relative) | 1,855★. Last release v0.3.88 2026-06-10, last push 2026-07-06 (stale ~5 months) | **New row, low priority** (only way to reach it) | VERIFIED (source) | P9 |
| 5 | **VT Code** (`vtcode`) | vinhnx (individual) · CLI (Rust) | Yes | USER, first wins: `~/.agents/skills`, then `<data_dir>/skills` (macOS `~/Library/Application Support/com.vinhnx.vtcode/skills`, Linux `~/.local/share/vtcode/skills`, Windows `%APPDATA%\vinhnx\vtcode\data\skills`; `VTCODE_DATA` / `XDG_DATA_HOME` override), then `~/.vtcode/skills` (legacy; `VTCODE_HOME`). ADMIN `/etc/codex/skills` | `~/.vtcode` (legacy home) or the platform config dir. Neither is a clean single home-relative dir | Nearest ancestor `.agents/skills` up to the git root | 898★. Release 0.181.4 2026-11-05 (very active). Docs in repo | **AGENTS-group member only** (docs name `~/.agents/skills` as the user dir) | VERIFIED (source + docs) | P10, P11, P12 |
| 6 | **DimCode** (`dimcode`) | ArcShips · CLI `dim` + desktop (DimAgent), proprietary | Yes | `$DIMCODE_HOME/skills` (default `~/.dimcode/v2/skills`; `$XDG_CONFIG_HOME/.dimcode/v2` if set). This is the in-app market's install root. Then `~/.agents/skills` (`settings.userDir` overrides) | `~/.dimcode` | `.agents/skills` (`settings.workspaceDir` overrides) | ~7.0k npm/month. v0.5.24 2026-11-04. Docs (zh/en), no public repo | **AGENTS-group member only** | VERIFIED (shipped bundle `dimcode-darwin-arm64@0.5.24`) + docs | P13, P14 |
| 7 | **crow-cli** (`crow-cli`) | Thomas Wood · CLI (Python, "minimal ACP-native") | Yes | `~/.agents/skills` only (config key `skills_dir`) | `~/.agents/crow` (config root; note: inside `~/.agents`) | Every hidden dir with a `skills/` child, from cwd up to the git root, so `.agents/skills`, `.claude/skills`, … | 63★. v0.1.49 2026-10-31. ~450 PyPI/month | **AGENTS-group member only** | VERIFIED (source + README) | P15, P16 |
| 8 | **Corust Agent** (`corust-agent`) | Corust AI · ACP agent binary (Zed extension), Rust-focused | Yes | `~/.corust-agent/skills`, `~/.agents/skills` (`CORUST_HOME` overrides the base) | `~/.corust-agent` | `.corust-agent/skills`, `.agents/skills` (cwd and ancestors up to home) | 31★ release-only repo. v0.6.0 2026-05-13 (stale ~6 months) | **AGENTS-group member only** | VERIFIED (strings in shipped v0.6.0 binary). `.claude` UNKNOWN | P17 |
| 9 | **fast-agent** (`fast-agent`) | evalstate · Python framework + CLI/TUI (ACP via `uvx`) | Yes | **None by default.** Defaults `.fast-agent/skills`, `.agents/skills`, `.claude/skills` resolve against **cwd**, and "home" defaults to `./.fast-agent`. `FAST_AGENT_HOME` or `--home` relocates only `.fast-agent/skills` | None at `~` (per-project `./.fast-agent`) | `.fast-agent/skills`, `.agents/skills`, `.claude/skills` | 3,959★. v0.10.53 2026-11-05. ~108k PyPI/month | **Skip**: nothing global to deploy to; project `.agents/skills` and `.claude/skills` are already covered | VERIFIED (source + docs) | P18, P19, P20 |
| 10 | **siGit Code** (`sigit`) | smbCloud · CLI (Rust), local-first | Yes | `$SIGIT_CONFIG_DIR/skills` (default `~/.config/sigit/skills`), then `~/.claude/skills` | `~/.config/sigit` | `.sigit/skills`, `.claude/skills` | 57★. v1.7.9 2026-11-04 | **Skip**: low signal; reached through the Claude Code row's `~/.claude/skills` | VERIFIED (source + CHANGELOG) | P21, P22 |
| 11 | **Minion Code** (`minion-code`) | femto · CLI/ACP (Python) | Yes | `~/.claude/skills`, then `~/.minion/skills` | `~/.minion` | `.claude/skills`, `.minion/skills` | 42★. Tags to v0.1.45. ~630 PyPI/month | **Skip**: low signal; reached through `~/.claude/skills` | VERIFIED (source) | P23, P24 |
| 12 | **Harn** (`harn`) | Burin Labs · agent-pipeline language/runtime | Yes (native `SKILL.md` layers) | `~/.harn/skills`. System: `/etc/harn/skills`, `$XDG_CONFIG_HOME/harn/skills`. Env `$HARN_SKILLS_PATH` | `~/.harn` | `.harn/skills` (walking up). `harn.toml [skills]` | 29★. v0.10.188 2026-11-05 | **Skip**: not a coding-tool harness users install skills for; tiny audience | VERIFIED (source) | P25 |
| 13 | **Nova** (`nova`) | Compass AI · CLI (npm), proprietary | Yes (`SKILL.md`) | `~/<state>/skills`, where `<state>` = `.nova`, or an existing `.compass`. Also `~/<state>/projects/<slug>/skills` | `~/.nova` (or `~/.compass`) | `<state>/skills` in the project | 14★. ~4.3k npm/month. v1.1.64 | **Skip**: low signal; conditional dir; shipped guidance moves skills *out of* `.claude/skills` / `.agents/skills` | VERIFIED (shipped bundle text + CHANGELOG) | P26, P27 |
| 14 | **GLM Agent** (`glm-acp-agent`) | Stefan de Vogelaere · ACP adapter for Zhipu GLM | **Partial**: `SKILL.md` is only a `/name` slash command (body injected on invocation). No model-invoked skills | `~/.claude/skills` (as commands) | `$XDG_CONFIG_HOME/glm-acp-agent` (default `~/.config/glm-acp-agent`) | `.claude/skills`, `.claude/commands` | 74★. ~23k npm/month. v1.17.0 2026-10-30 | **Skip**: partial support; reached through `~/.claude/skills` anyway | VERIFIED (source + README) | P28, P29 |
| 15 | **Agoragentic** (`agoragentic-acp`) | ACRE / Agoragentic · MCP/ACP bridge to a paid agent marketplace | **No**: the ACP mode (`agoragentic-mcp --acp`) relays marketplace calls. Its 787-line server has no `readFile`/`readdir`; "skill.md" is a hosted web contract | — | — | — | 41★. ~3.2k npm/month | **Skip**: not a skills-reading harness | VERIFIED (shipped package, absence) | P30 |

## § Shared-dir reads (skills-supporting candidates)

Precedence is for a same-name collision between the candidate's own global dir and a shared dir, unless stated.
"Shared-only suffices" means deploying only to `~/.agents/skills` and/or `~/.claude/skills` reaches the harness at global
scope without its own row.

| Candidate | Reads `~/.agents/skills` | Reads `~/.claude/skills` | Reads project `.agents/skills` / `.claude/skills` | Precedence | Shared-only suffices? | Confidence | Src |
|---|---|---|---|---|---|---|---|
| Muse Code | Yes (user tier) | Yes, by default ("foreign personal root"; a user preference/rollout gate can disable it) | Yes / Yes | UNKNOWN (undocumented) | **Yes**, via `~/.agents/skills` (robust). `~/.claude/skills` is gateable | VERIFIED | P1 |
| Poolside | Yes (since 1.0.11, 2026-07-09) | UNKNOWN ("also scans directories used by other tools", unnamed) | Yes / UNKNOWN | UNKNOWN | **Yes**, via `~/.agents/skills`. Caveat: user-level skills in `~/.config/poolside/skills` "are not mounted in local sandboxes"; `~/.agents/skills` mounting is undocumented | VERIFIED (absence of a `.claude` claim is not proof) | P3, P4 |
| Dirac | Yes | Yes | Yes / Yes (also `.ai`, `.dirac`, `.diracrules`) | **Shared wins.** Scan order, project dirs first: `~/.dirac`, `~/.agents`, `~/.claude`, `~/.ai`. Dedupe iterates backwards (last wins), so global > project and `~/.ai` > `~/.claude` > `~/.agents` > `~/.dirac` | **Yes** (either shared dir) | VERIFIED | P6, P7 |
| Stakpak | No | No | No / No (`.stakpak/skills` only) | n/a (own dirs: first wins, project before user) | **No** | VERIFIED (complete dir list in source) | P9 |
| VT Code | Yes, the first USER root | No | Yes / No (project `.agents/skills` only) | **Shared wins**: `~/.agents/skills` before data-dir and `~/.vtcode` roots; "only the first occurrence is loaded" | **Yes** | VERIFIED | P10, P11, P12 |
| DimCode | Yes | No (at runtime; `.claude/skills` appears only in its embedded `skills` installer's repo-scan list) | Yes / No | **Own wins**: market root `~/.dimcode/v2/skills` before `~/.agents/skills`; user beats workspace | **Yes** | VERIFIED (shipped bundle) | P13 |
| crow-cli | Yes (its only user dir) | No | Yes / Yes (any hidden `*/skills` up to git root) | Project wins over user (earlier root wins; shadowed copies logged) | **Yes** | VERIFIED | P15, P16 |
| Corust Agent | Yes | UNKNOWN | Yes / UNKNOWN | Own listed first; "duplicate skill name discovered; keeping the first match". **Own wins** LIKELY | **Yes** | VERIFIED (dirs) / LIKELY (order) | P17 |
| fast-agent | Only when cwd is `~` (paths are cwd-relative) | Same | Yes / Yes | Later dirs override earlier: `.claude` > `.agents` > `.fast-agent` (shared wins), with a warning | n/a (no global scope) | VERIFIED | P18, P19 |
| siGit Code | No | Yes (last root) | No / Yes | **Own wins**: earlier root wins; `~/.config/sigit/skills` before `~/.claude/skills` | **Yes**, via `~/.claude/skills` | VERIFIED | P21 |
| Minion Code | No | Yes (first user root) | No / Yes | **Shared wins**: `~/.claude/skills` registered before `~/.minion/skills`; equal priority skips the later one | **Yes**, via `~/.claude/skills` | VERIFIED | P23, P24 |
| Harn | No | No | No / No | n/a (higher layer wins; losers recorded as `Shadowed`) | **No** | VERIFIED (complete layer list) | P25 |
| Nova | No (its guidance deletes copies there) | No (same) | No / No | n/a | **No** | VERIFIED (shipped text) / LIKELY (absence) | P26 |
| GLM Agent (partial) | No | Yes, as slash commands only | No / Yes | Project beats user | Partially (commands only) | VERIFIED | P28 |

**Implication for the toggle:** a "shared dirs only" mode keeps Muse Code, Poolside, Dirac, VT Code, DimCode, crow-cli and
Corust fully served by `~/.agents/skills`, and siGit and Minion by `~/.claude/skills`. **Stakpak, Harn and Nova drop out.**
For Dirac, any overlap between a Dirac row and a shared dir is decided in the shared copy's favour, so a stale shared copy
would mask a fresh `~/.dirac/skills` one.

## § Sources

All fetched 2026-11-05. The repos were shallow-cloned at the SHA shown. P = primary, S = secondary.

| Id | URL | P/S | Note |
|---|---|---|---|
| R0 | https://github.com/agentclientprotocol/registry/tree/a44e8fc | P | Candidate list; each `<id>/agent.json` (repo, website, distribution, version) |
| P1 | https://dev.meta.ai/docs/muse-code/extending#skills | P | Muse Code skill sources (built-in, user, project, plugin); foreign roots gateable |
| P2 | https://dev.meta.ai/docs/muse-code/configuration#settings-file | P | `~/.config/muse/settings.json` |
| S1 | https://mossbysea.com/tools/muse-code-ships-the-door-out/ | S | Launch 2026-08-05, GA 2026-09-01 (dates only) |
| P3 | https://docs.poolside.ai/skills | P | Local skill sources list; sandbox caveat |
| P4 | https://github.com/poolsideai/pool/blob/fcabc59ff67810ecec5429ec9d45fa1dda6c1136/CHANGELOG.md#L73-L78 | P | 1.0.11 (2026-07-09): "Skills are now also discovered from `~/.agents/skills`"; L197 `.agent/skills` |
| P5 | https://docs.poolside.ai/settings-file-reference.md | P | `~/.config/poolside/settings.yaml`; `pool` settings read only there |
| P6 | https://github.com/dirac-run/dirac/blob/a7cf472759250159246af4f0729a4aa8f2ff3dc3/src/core/storage/skillsStorage.ts#L12-L57 | P | Global + project scan list |
| P7 | https://github.com/dirac-run/dirac/blob/a7cf472759250159246af4f0729a4aa8f2ff3dc3/src/core/context/instructions/user-instructions/skills.ts#L263-L299 | P | "Global skills take precedence"; backwards dedupe |
| P8 | https://github.com/dirac-run/dirac/blob/a7cf472759250159246af4f0729a4aa8f2ff3dc3/src/core/storage/paths.ts#L42-L43 · …/fileNames.ts#L23-L27 | P | `~/.dirac` home; project dir names |
| P9 | https://github.com/stakpak/agent/blob/760cd2b5984d29c2d513bb15ca33e995fae45f17/libs/api/src/local/skills/mod.rs#L7-L58 · #L153-L164 | P | First-seen dedupe; `default_skill_directories` |
| P10 | https://github.com/vinhnx/VTCode/blob/86902fce7e1d879797cef49586865cb7136c55c1/crates/codegen/vtcode-core/src/skills/loader/discovery.rs#L17-L110 | P | Skill roots by scope |
| P11 | https://github.com/vinhnx/VTCode/blob/86902fce7e1d879797cef49586865cb7136c55c1/crates/codegen/vtcode-core/src/skills/loader.rs#L120-L143 | P | "only the first occurrence is loaded" |
| P12 | https://github.com/vinhnx/VTCode/blob/86902fce7e1d879797cef49586865cb7136c55c1/crates/common/vtcode-commons/src/vtcode_paths/mod.rs#L21-L151 · docs/skills/SKILLS_GUIDE.md#L5-L14 | P | Platform data dirs, `VTCODE_HOME`/`VTCODE_DATA`; documented discovery order |
| P13 | https://registry.npmjs.org/dimcode-darwin-arm64/-/dimcode-darwin-arm64-0.5.24.tgz (`bin/dimcode`, plugin `skill-loader`: `DEFAULT_USER_DIR`, `discoverHostUserSkills`, `discoverWorkspaceSkills`, `getMarketSkillsRoot`) | P | Shipped bundle, inspected via `strings`. No public repo |
| P14 | https://dimagent.cn/en/docs/config | P | `~/.dimcode/v2`, `DIMCODE_HOME` |
| P15 | https://github.com/crow-cli/crow-cli/blob/a459cc59d249401172dbdceaf65de1a8efc9a497/src/crow_cli/config/config.py#L30-L36 | P | `~/.agents/crow` config, `~/.agents/skills` |
| P16 | https://github.com/crow-cli/crow-cli/blob/a459cc59d249401172dbdceaf65de1a8efc9a497/src/crow_cli/agent/prompt.py#L234-L275 | P | Skill roots; earlier root wins |
| P17 | https://github.com/Corust-ai/corust-agent-release/releases/download/v0.6.0/agent-darwin-arm64.tar.gz (`corust-agent-acp`) | P | Shipped binary strings: User/Project skill-dir table, `CORUST_HOME`, "keeping the first match" |
| P18 | https://github.com/evalstate/fast-agent/blob/1ec7978009a002b531b9d094d5abc8eaa2c2035c/src/fast_agent/constants.py#L162-L168 · src/fast_agent/paths.py#L126-L150 | P | Default skill paths, cwd-relative |
| P19 | https://github.com/evalstate/fast-agent/blob/1ec7978009a002b531b9d094d5abc8eaa2c2035c/docs/docs/guides/skills.md#L13-L23 | P | Dirs; "later directories override earlier ones" |
| P20 | https://github.com/evalstate/fast-agent/blob/1ec7978009a002b531b9d094d5abc8eaa2c2035c/src/fast_agent/home.py#L67-L89 | P | Home: `--home` > `FAST_AGENT_HOME` > `./.fast-agent` |
| P21 | https://github.com/getsigit/sigit/blob/df34acc6053409d1b63a50bd5532cf4ca061416d/crates/cli/src/skills.rs#L18-L222 | P | Roots in priority order; `SIGIT_CONFIG_DIR` |
| P22 | https://github.com/getsigit/sigit/blob/df34acc6053409d1b63a50bd5532cf4ca061416d/CHANGELOG.md#L1324 | P | Release note for skill discovery |
| P23 | https://github.com/femto/minion-code/blob/bd4cf5988ecbee5e776b542f324c874900debeec/minion_code/skills/skill_loader.py#L19-L68 | P | Search paths |
| P24 | https://github.com/femto/minion-code/blob/bd4cf5988ecbee5e776b542f324c874900debeec/minion_code/skills/skill_registry.py#L27-L52 · minion_code/acp_server/main.py#L51 | P | Equal priority keeps the first; `~/.minion` config |
| P25 | https://github.com/burin-labs/harn/blob/1ae83203075d0dd380f5ca2a5a6a42dd5ef30b39/crates/harn-vm/src/skills/discovery.rs#L1-L17 · crates/harn-vm/src/skills/mod.rs#L185-L199 | P | Layer list; `~/.harn/skills` |
| P26 | https://registry.npmjs.org/@compass-ai/nova/-/nova-1.1.64.tgz (`dist/index.js`, built-in skill-install guidance) | P | `<state>` = `.nova` / `.compass`; move-out-of-`.claude/skills` guidance |
| P27 | https://github.com/Compass-Agentic-Platform/nova/blob/4c53e2f/CHANGELOG.md (v1.0.35, 2026-01-27) | P | "Skills are now discovered from `.compass/skills/`" |
| P28 | https://github.com/stefandevo/glm-acp-agent/blob/54080ad473bf543ebb885b479a12bb563607dee7/src/protocol/slash-commands.ts#L11-L18 · #L90 | P | `.claude/skills` as slash commands, cwd + `~/.claude` |
| P29 | https://github.com/stefandevo/glm-acp-agent/blob/54080ad473bf543ebb885b479a12bb563607dee7/src/llm/credentials.ts#L34-L37 | P | Config root |
| P30 | https://registry.npmjs.org/agoragentic-mcp/-/agoragentic-mcp-1.3.0.tgz (`mcp-server.js`) | P | ACP mode; no local file reads |
| M1 | GitHub REST `repos/<owner>/<repo>` and `releases/latest`; npm `api.npmjs.org/downloads/point/last-month`; `pypistats.org` | P | Stars, push/release dates, download counts (relevance only) |
