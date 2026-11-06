# Skills Hub (Tauri Desktop)

A cross-platform desktop app (Tauri + React) to manage Agent Skills in one place and deploy them to many AI coding harnesses — Claude Code, Cursor, Codex, … — in their global skills folders or in your projects (prefer symlink/junction, fallback to copy) — “Install once, deploy everywhere”.

## Documentation

- English (default): `README.md` (this file)

## Key Features

- **Explore page**: Browse featured skills (ClawHub, updated daily) and search online (skills.sh); preview a skill's files, then add it to your Library and deploy it to your selected harnesses
- **Inspector**: Select a skill to see its health, source and per-harness deployments, repair what failed, and browse its files with Markdown rendering and syntax highlighting (40+ languages)
- **Command palette**: ⌘K / Ctrl+K finds a skill, a place or an action
- **Unified view**: Your Library of skills and where each one is deployed, per harness
- **Onboarding import**: Scan existing skills in installed harnesses, import them into your Library, and deploy — an original is removed only when it is byte-identical to the imported copy; a same-named but different sibling is kept and reported, unless you choose "Remove the other copies" in the review
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
| `codex` | Codex | `.codex/skills` | `.codex` |
| `opencode` | OpenCode | `.config/opencode/skills` | `.config/opencode` |
| `antigravity` | Antigravity | `.gemini/antigravity/global_skills` | `.gemini/antigravity` |
| `amp` | Amp | `.config/agents/skills` | `.config/amp` |
| `kimi_cli` | Kimi Code CLI | `.config/agents/skills` | `.kimi-code` or `.kimi` |
| `augment` | Augment | `.augment/skills` | `.augment` |
| `openclaw` | OpenClaw | `.openclaw/skills` | `.openclaw` |
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
| `mux` | Mux | `.mux/skills` | `.mux` |
| `openclaude` | OpenClaude IDE | `.openclaude/skills` | `.openclaude` |
| `openhands` | OpenHands | `.openhands/skills` | `.openhands` |
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
| `kilo_code` | Kilo Code | `.kilocode/skills` | `.kilocode` |
| `roo_code` | Roo Code | `.roo/skills` | `.roo` |
| `goose` | Goose | `.config/goose/skills` | `.config/goose` |
| `gemini_cli` | Gemini CLI | `.gemini/skills` | `.gemini` |
| `github_copilot` | GitHub Copilot | `.copilot/skills` | `.copilot` |
| `clawdbot` | Clawdbot | `.clawdbot/skills` | `.clawdbot` |
| `droid` | Droid | `.factory/skills` | `.factory` |
| `windsurf` | Windsurf | `.codeium/windsurf/skills` | `.codeium/windsurf` |
| `moltbot` | MoltBot | `.moltbot/skills` | `.moltbot` |
| `hermes-agent` | Hermes Agent | `.hermes/skills` | `.hermes` |

`agents_skills` is scope-dependent. **Globally** it is a plain, independently selectable target: it
manages `~/.agents/skills` and nothing else — Cursor still reads `~/.cursor/skills`, Codex still reads
`~/.codex/skills`, so those harnesses keep their own rows in the global list. **In a project** it is an
aggregate shown as `.agents/skills (9 harnesses)`: one folder, `<project>/.agents/skills`, that nine harnesses
(Cursor, Codex, OpenCode, Antigravity, Amp, Kimi Code CLI, Cline, Gemini CLI, GitHub Copilot) all read,
so those nine have no separate project row.

## Development

### Prerequisites

- Node.js 22+ (the frontend tests need it; CI uses 22)
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
- What happens when a folder with the skill's name is already in a harness? Skills Hub asks before replacing it (identical content is replaced silently); choosing Keep leaves that folder alone and records the skill as off for that harness.
- macOS Gatekeeper note (unsigned/notarized builds, may vary by macOS version): if you see “damaged” or “unverified developer”, run `xattr -cr "/Applications/Skills Hub.app"` (https://v2.tauri.app/distribute/#macos).

## Supported Platforms

- macOS (verified)
- Windows (CI runs the Rust test suite on Windows; release builds smoke-tested on Windows 11)
- Linux (CI runs the Rust test suite on Linux; not smoke-tested on a desktop)

## License

MIT License — see `LICENSE`.
