# Skills Hub（Tauri Desktop）

一个跨平台桌面应用（Tauri + React），用于统一管理 Agent Skills，并把它们同步到多种 AI 编程工具的全局 skills 目录（优先 symlink/junction，失败回退 copy），实现 “Install once, sync everywhere”。

> English documentation: [`README.md`](../README.md)

## 主要功能

- **Explore 探索页**：独立页面浏览精选技能推荐（ClawHub 每日更新）和在线搜索（skills.sh），一键安装并同步到所有已检测工具
- **技能详情页**：点击技能名称查看完整文件内容，支持文件树浏览、Markdown 渲染和代码语法高亮（40+ 语言）
- **统一视图**：查看 Hub 托管的 skills 及其在各工具的生效状态
- **迁移接管**：扫描本机工具目录已有 skills，导入到中心仓库并可一键同步 —— 仅当原目录与导入后的中心副本逐字节一致时才会删除原目录；同名但内容不同的目录会被保留并在报告中列出
- **多来源导入**：本地目录 / Git 仓库 URL（含 multi-skill 候选选择、`.claude/skills/` 目录格式支持）；GitHub 来源优先走 GitHub API 下载（记录真实 commit），失败时回退为 clone
- **更新 / 刷新**：从原来源重新获取并把每个已同步副本更新到位；批量刷新会并行获取，并逐个技能、逐个目标地报告结果
- **新工具检测**：发现新安装工具时提示是否同步所有已托管 skills

### My Skills — 技能管理列表
![My Skills](./assets/my-skills.png)

### Explore — 探索与在线搜索
![Explore](./assets/explore-search.png)

### Manual Add — 手动添加技能
![Manual Add](./assets/manual-add.png)

### Skill Detail — 技能详情与文件浏览
![Skill Detail](./assets/skill-detail.png)

## 支持的 AI 编程工具

| tool key | 工具 | skills 目录（相对 `~`） | detect 目录（相对 `~`） |
| --- | --- | --- | --- |
| `cursor` | Cursor | `.cursor/skills` | `.cursor` |
| `claude_code` | Claude Code | `.claude/skills` | `.claude` |
| `codex` | Codex | `.codex/skills` | `.codex` |
| `opencode` | OpenCode | `.config/opencode/skills` | `.config/opencode` |
| `antigravity` | Antigravity | `.gemini/antigravity/global_skills` | `.gemini/antigravity` |
| `amp` | Amp | `.config/agents/skills` | `.config/agents` |
| `kimi_cli` | Kimi Code CLI | `.config/agents/skills` | `.config/agents` |
| `augment` | Augment | `.augment/rules` | `.augment` |
| `openclaw` | OpenClaw | `.openclaw/skills` | `.openclaw` |
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

完整路径规则与检测逻辑见 [`src-tauri/src/core/tool_adapters/mod.rs`](../src-tauri/src/core/tool_adapters/mod.rs)。

## 开发

### 环境要求

- Node.js 18+（建议 20+）
- Rust（stable）
- Tauri 系统依赖（按官方文档安装）

### 启动（桌面端）

```bash
npm install
npm run tauri:dev
```

### 构建

```bash
npm run lint
npm run build
npm run tauri:build
```

#### 各系统构建命令（来自 `package.json`）

- macOS（dmg）：`npm run tauri:build:mac:dmg`
- macOS（universal dmg）：`npm run tauri:build:mac:universal:dmg`
- Windows（MSI）：`npm run tauri:build:win:msi`
- Windows（NSIS exe）：`npm run tauri:build:win:exe`
- Windows（MSI+NSIS）：`npm run tauri:build:win:all`
- Linux（deb）：`npm run tauri:build:linux:deb`
- Linux（AppImage）：`npm run tauri:build:linux:appimage`
- Linux（deb+AppImage）：`npm run tauri:build:linux:all`

### 测试（Rust）

```bash
cd src-tauri
cargo test
```

## FAQ / 备注

- Skill 存在哪里？中心仓库（Central Repo）默认是 `~/.skillshub`，可在设置里修改。
- Cursor 也会用软链吗？会，自 Skills Hub 1.2.1 起 —— Cursor 2.5+（IDE）和当前的 Cursor CLI 都能识别软链形式的技能目录。旧版本 Skills Hub 以复制方式同步到 Cursor 的技能会保持原样，需要以覆盖方式重新同步才会替换为软链。若使用较旧的 Cursor，请先升级再重新同步。
- 为什么有时会变成 Copy？默认优先 symlink/junction，但在某些系统（尤其 Windows）可能因为权限/策略导致无法创建链接，会自动回退到目录复制。
- 取消同步或删除后，为什么工具里还能看到该技能？如果 Skills Hub 无法删除该目录（权限、文件被占用等），它会保留这条记录并标记为 `error`，而不是直接遗忘，这样路径仍然可见、同一操作可以重试。项目中取消分配技能、移除项目或项目的某个工具时同理。
- 项目分配矩阵为什么有时提示状态未重新核对？同步操作同一时间只允许一个。若加载矩阵时正好有同步在进行，界面会直接展示已存储的状态而不与磁盘核对；操作结束后即可刷新。
- `TARGET_EXISTS|...` 是什么意思？目标目录已存在且默认不覆盖（为了安全）。你需要先清理目标目录，或在“接管/覆盖”的明确流程里重试。
- macOS Gatekeeper 备注（未签名/未公证构建，不同 macOS 版本表现可能不同）：如提示“已损坏/无法验证开发者”，可执行 `xattr -cr "/Applications/Skills Hub.app"`（https://v2.tauri.app/distribute/#macos）。

## 支持的系统

- macOS（已验证）
- Windows（按架构应支持，未做本地验证）
- Linux（按架构应支持，未做本地验证）

## License

MIT License（见 `LICENSE`）。
