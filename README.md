# Skills Hub (Tauri Desktop)

A cross-platform desktop app (Tauri + React) to manage Agent Skills in one place and deploy them to many AI coding harnesses — Claude Code, Cursor, Codex, … — in their global skills folders or in your projects (prefer symlink/junction, fallback to copy) — “Install once, deploy everywhere”.

## Documentation

- English (default): `README.md` (this file)

## Key Features

- **Explore page**: Browse featured skills (ClawHub, updated daily) and search online (skills.sh) — one-click add & deploy to your selected harnesses
- **Skill detail view**: Click a skill name to browse its files with Markdown rendering and syntax highlighting (40+ languages)
- **Unified view**: Your Library of skills and where each one is deployed, per harness
- **Onboarding import**: Scan existing skills in installed harnesses, import them into your Library, and deploy — an original is removed only when it is byte-identical to the imported copy; a same-named but different sibling is kept and reported
- **Import sources**: Local folder / Git URL (including multi-skill repo selection, `.claude/skills/` directory support); GitHub sources download through the GitHub API where possible (recording the real commit) and fall back to cloning
- **Update / Update all**: Re-fetch from source and bring every deployed copy back into line; updating many skills fetches them in parallel and reports the outcome of each skill and each harness it touched
- **New harness detection**: Detect newly installed harnesses and offer to deploy your skills to them

### My Skills
![My Skills](docs/assets/my-skills.png)

### Explore & Search
![Explore](docs/assets/explore-search.png)

### Manual Add
![Manual Add](docs/assets/manual-add.png)

### Skill Detail
![Skill Detail](docs/assets/skill-detail.png)

## Supported AI Coding Harnesses

| key | Display name | skills dir (relative to `~`) | detect dir (relative to `~`) |
| --- | --- | --- | --- |
| `agents_skills` | .agents/skills | `.agents/skills` | `.agents` |
| `cursor` | Cursor | `.cursor/skills` | `.cursor` |
| `claude_code` | Claude Code | `.claude/skills` | `.claude` |
| `codex` | Codex | `.agents/skills` (formerly `.codex/skills`) | `.codex` |
| `opencode` | OpenCode | `.config/opencode/skills` | `.config/opencode` |
| `antigravity` | Antigravity | `.gemini/config/skills` (formerly `.gemini/antigravity/global_skills`) | `.gemini/antigravity` |
| `amp` | Amp | `.config/agents/skills` | `.config/amp` |
| `kimi_cli` | Kimi Code CLI | `.kimi-code/skills` (formerly `.config/agents/skills`) | `.kimi-code` or `.kimi` |
| `augment` | Augment | `.augment/skills` | `.augment` |
| `openclaw` | OpenClaw | `.openclaw/skills` (formerly `.clawdbot/skills`, `.moltbot/skills`) | `.openclaw` |
| `copaw` | Copaw | `.copaw/skill_pool` | `.copaw` |
| `cline` | Cline | `.cline/skills` | `.cline` |
| `codebuddy` | CodeBuddy | `.codebuddy/skills` | `.codebuddy` |
| `command_code` | Command Code | `.commandcode/skills` | `.commandcode` |
| `continue` | Continue | `.continue/skills` | `.continue` |
| `crush` | Crush | `.config/crush/skills` | `.config/crush` |
| `junie` | Junie | `.junie/skills` | `.junie` |
| `iflow_cli` | iFlow CLI | `.iflow/skills` | `.iflow` |
| `kiro_cli` | Kiro CLI | `.kiro/skills` | `.kiro` |
| `kode` | Kode | `.kode/skills` | `.kode` |
| `mcpjam` | MCPJam | `.mcpjam/skills` | `.mcpjam` |
| `mistral_vibe` | Mistral Vibe | `.vibe/skills` | `.vibe` |
| `mux` | Xum (formerly Mux) | `.xum/skills` (formerly `.mux/skills`) | `.xum` or `.mux` |
| `openclaude` | OpenClaude IDE | `.openclaude/skills` | `.openclaude` |
| `openhands` | OpenHands | `.agents/skills` (formerly `.openhands/skills`) | `.openhands` |
| `pi` | Pi | `.pi/agent/skills` | `.pi` |
| `qoder` | Qoder | `.qoder/skills` | `.qoder` |
| `qoderwork` | QoderWork | `.qoderwork/skills` | `.qoderwork` |
| `qwen_code` | Qwen Code | `.qwen/skills` | `.qwen` |
| `trae` | Trae | `.trae/skills` | `.trae` |
| `trae_cn` | Trae CN | `.trae-cn/skills` | `.trae-cn` |
| `zencoder` | Zencoder | `.zencoder/skills` | `.zencoder` |
| `neovate` | Neovate | `.neovate/skills` | `.neovate` |
| `pochi` | Pochi | `.pochi/skills` | `.pochi` |
| `adal` | AdaL | `.adal/skills` | `.adal` |
| `kilo_code` | Kilo Code | `.kilo/skills` (formerly `.kilocode/skills`) | `.kilo` or `.kilocode` |
| `roo_code` | Roo Code | `.roo/skills` | `.roo` |
| `goose` | Goose | `.agents/skills` (formerly `.config/goose/skills`) | `.config/goose` |
| `gemini_cli` | Gemini CLI | `.gemini/skills` | `.gemini` |
| `github_copilot` | GitHub Copilot | `.copilot/skills` | `.copilot` |
| `droid` | Droid | `.factory/skills` | `.factory` |
| `windsurf` | Windsurf | `.codeium/windsurf/skills` | `.codeium/windsurf` |
| `hermes-agent` | Hermes Agent | `.hermes/skills` | `.hermes` |
| `deepseek_harness` | DeepSeek Harness | `.dsh/skills` | `.dsh` |
| `zcode` | ZCode | `.zcode/skills` | `.zcode` |
| `codewhale` | Codewhale | `.codewhale/skills` | `.codewhale` |
| `workbuddy` | WorkBuddy (global only — no project dir) | `.workbuddy/skills` | `.workbuddy` |

Every path was reviewed in
[`docs/design-inputs/2026-11-02-harness-global-skills-dirs.md`](docs/design-inputs/2026-11-02-harness-global-skills-dirs.md),
which marks each row's evidence: most are VERIFIED against the harness's own docs, release notes or source; others
rely on secondary sources only (LIKELY — WorkBuddy's folder is one: its vendor doc names no path), and some facts remain
UNKNOWN (Trae's official global path; MCPJam, OpenClaude IDE and AdaL have no vendor docs; whether Kilo Code and
Windsurf still read their old folders).
A "formerly" dir is where Skills Hub deployed before; existing deployments there move to the current dir on the next
launch (only Kimi's own deployments leave Amp's `.config/agents/skills`). Clawdbot and MoltBot, former names of
OpenClaw, are no longer listed separately; a deployment recorded for one of them moves to OpenClaw on the next launch.

`agents_skills` is scope-dependent. **Globally** it is the `~/.agents/skills` folder: Codex, Goose and OpenHands
deploy there too (it is their documented user dir), so they share one deployment with it. With **Prefer the AGENTS
standard folder** on (the default for a new install; an existing install is asked once, and the setting moves nothing
until it answers — the registry corrections above still move on the next launch), every harness that is verified to
read `~/.agents/skills` — Cursor, OpenCode, Amp, Kimi Code CLI, Augment, Cline, Gemini CLI, GitHub Copilot, Pi and
others — deploys there as well instead of to its own folder, so it does not see one skill in both its own folder and
`~/.agents/skills` (a deployment that could not move — a different folder already in its way — stays, and is logged).
It does not remove other duplicates: a harness that also reads another harness's folder (Cursor reads Claude Code's
`~/.claude/skills`) still sees a skill deployed to both, which is what the harness list's "also reads" warning is for.
**In a project** it is an aggregate shown as `.agents/skills (9 harnesses)`: one folder, `<project>/.agents/skills`, that nine harnesses (Cursor, Codex, OpenCode,
Antigravity, Amp, Kimi Code CLI, Cline, Gemini CLI, GitHub Copilot) all read, so those nine have no separate project
row.

## Development

### Prerequisites

- Node.js 18+ (recommended: 20+)
- Rust (stable)
- Tauri system dependencies (follow Tauri official docs for your OS)

```bash
npm install
npm run tauri:dev
```

### Build

```bash
npm run lint
npm run build
npm run tauri:build
```

#### Platform build commands (from `package.json`)

- macOS (dmg): `npm run tauri:build:mac:dmg`
- macOS (universal dmg): `npm run tauri:build:mac:universal:dmg`
- Windows (MSI): `npm run tauri:build:win:msi`
- Windows (NSIS exe): `npm run tauri:build:win:exe`
- Windows (MSI+NSIS): `npm run tauri:build:win:all`
- Linux (deb): `npm run tauri:build:linux:deb`
- Linux (AppImage): `npm run tauri:build:linux:appimage`
- Linux (deb+AppImage): `npm run tauri:build:linux:all`

### Tests (Rust)

```bash
cd src-tauri
cargo test
```

## Contributing & Security

- Contributing: [`CONTRIBUTING.md`](CONTRIBUTING.md)
- Design system (tokens, primitives, motion, contrast): [`DESIGN.md`](DESIGN.md) — read it before changing UI; the styling rules are [ADR-0006](docs/adr/0006-styling-architecture.md)
- Code of Conduct: [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)
- Security: [`SECURITY.md`](SECURITY.md)

## FAQ / Notes

- Where are skills stored? In the Library folder, `~/.skillshub` by default (configurable in Settings).
- Does Cursor get symlinks too? Yes, since Skills Hub 1.2.1 — Cursor 2.5+ (IDE) and the current Cursor CLI discover symlinked skill directories. Skills deployed to Cursor by an earlier Skills Hub version stay as copies until you deploy them again with overwrite. If you run an older Cursor, keep it updated or re-deploy after upgrading.
- Why does deploying sometimes fall back to copy? Skills Hub prefers symlink/junction, but on some systems (especially Windows) symlinks may be restricted; in that case it falls back to directory copy.
- Why does a skill still show up in a harness after I disable or remove it? If Skills Hub could not remove the folder (permissions, a locked file), it keeps the deployment and marks it Failed instead of forgetting it, so the path stays visible and the same action can be retried. The same holds for removing a skill from a project and for removing a project or one of its harnesses.
- Why does the project matrix sometimes say statuses may be out of date? Deploy operations run one at a time. If one is running while the matrix loads, the stored statuses are shown without re-checking them against disk; they are re-checked once the operation finishes.
- What does `TARGET_EXISTS|...` mean? The target folder already exists and the operation did not overwrite it (default is non-destructive). Remove the existing folder or retry with the appropriate overwrite flow.
- macOS Gatekeeper note (unsigned/notarized builds, may vary by macOS version): if you see “damaged” or “unverified developer”, run `xattr -cr "/Applications/Skills Hub.app"` (https://v2.tauri.app/distribute/#macos).

## Supported Platforms

- macOS (verified)
- Windows (expected by design; not validated locally)
- Linux (expected by design; not validated locally)

## License

MIT License — see `LICENSE`.
