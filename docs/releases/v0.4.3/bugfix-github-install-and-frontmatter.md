# Bugfix：优化 GitHub Skill 安装速度并修复多行 Frontmatter 渲染

## 问题 1：从 GitHub 仓库安装 Skill 很慢并最终超时

### 问题描述

从 GitHub 仓库安装某些 Skill 时，安装弹窗会长时间停留在加载状态，最后超时失败。典型表现：

- 弹窗持续显示「正在安装技能...」
- 日志提示正在执行文件/网络操作
- GitHub 网络较慢或仓库文件较多时更容易复现

### 根因分析

对于形如 `https://github.com/owner/repo/tree/branch/path` 的 GitHub 子目录 URL，原逻辑会优先使用 GitHub Contents API 递归下载目录。

该方式的问题是：

- 需要按目录递归请求 GitHub API
- 文件下载是串行 HTTP 请求
- 文件较多时请求数量快速增加
- API 下载失败后还会 fallback 到 `git clone`，导致慢路径被走两遍

因此在网络不稳定或仓库较大时，用户会看到长时间转圈后超时。

### 修复方案

新增 `clone_or_pull_sparse`，对 GitHub 子目录安装优先使用系统 `git` 执行浅克隆 + 稀疏检出：

```bash
git clone --depth 1 --filter=blob:none --sparse --no-tags ...
git sparse-checkout set --no-cone <subpath>
```

这样只检出目标 Skill 子目录，避免下载整个仓库或逐文件调用 GitHub API。

修复后流程：

- 子目录 GitHub URL：优先走 sparse checkout
- sparse checkout 失败：再 fallback 到 GitHub Contents API
- 更新已安装 Skill：如果记录了 `source_subpath`，同样优先走 sparse checkout
- 缓存 key 加入 `subpath`，避免不同子目录复用同一个稀疏工作区造成冲突

### 影响范围

该优化主要改善精确到子目录的安装链接，例如：

```text
https://github.com/anthropics/skills/tree/main/skills/frontend-design
```

如果用户输入的是仓库根 URL，应用仍需要先扫描仓库中的 Skill 候选项，该场景仍可能触发完整浅克隆。

## 问题 2：`description: |` 只渲染出一个 `|`

### 问题描述

部分 `SKILL.md` 使用 YAML block scalar 写多行描述：

```yaml
---
name: technical-writer
description: |
  Creates clear documentation, API references, guides, and
  technical content for developers and users.
author: awesome-llm-apps
---
```

详情页 Frontmatter 表格中只显示 `|`，后面的描述内容没有显示。

### 根因分析

前端详情页的 `parseFrontmatter` 和后端 `parse_skill_md` 都只支持简单的 `key: value` 单行解析。

当遇到 `description: |` 时：

- `description` 被解析成字面量 `|`
- 后续缩进的多行文本没有关联到 `description`
- 后端存入数据库的描述也可能变成 `|`

### 修复方案

前端和后端同时支持 YAML block scalar：

- `description: |`：保留换行，适合多行描述
- `description: >`：折叠为单段文本，适合普通段落

同时调整 Markdown 表格样式：

- `td` 使用 `white-space: pre-wrap` 保留多行文本
- 单元格顶部对齐
- 长文本允许换行，避免撑破布局

## 验证

已完成以下验证：

- `npm run build` 通过
- `cargo test -q` 通过，`71 passed`
- `cargo fmt --all -- --check` 通过
- 新增 Rust 测试覆盖 `description: |` 解析
- 使用真实 GitHub 仓库验证 sparse checkout 可在约 2 秒内检出目标子目录

## 修改文件

- `src-tauri/src/core/git_fetcher.rs`：新增 `clone_or_pull_sparse`
- `src-tauri/src/core/installer.rs`：GitHub 子目录安装和更新优先使用 sparse checkout；补充 block scalar 解析
- `src-tauri/src/core/tests/git_fetcher.rs`：新增 sparse checkout 测试
- `src-tauri/src/core/tests/installer.rs`：新增 `description: |` 解析测试
- `src/components/skills/SkillDetailView.tsx`：前端 Frontmatter 解析支持 `|` 和 `>`
- `src/App.css`：修复 Frontmatter 表格中多行描述的展示样式
