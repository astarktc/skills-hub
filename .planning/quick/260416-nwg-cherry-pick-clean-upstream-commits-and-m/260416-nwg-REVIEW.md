---
phase: 260416-nwg
reviewed: 2026-04-16T18:31:00Z
depth: standard
files_reviewed: 16
files_reviewed_list:
  - src-tauri/src/core/installer.rs
  - src-tauri/src/core/tool_adapters/mod.rs
  - src-tauri/src/commands/mod.rs
  - src-tauri/src/core/project_sync.rs
  - src-tauri/src/core/git_fetcher.rs
  - src-tauri/src/core/github_download.rs
  - src-tauri/src/core/tests/installer.rs
  - src-tauri/src/core/tests/git_fetcher.rs
  - src-tauri/src/core/tests/project_ops.rs
  - src-tauri/src/core/tests/project_sync.rs
  - src/App.tsx
  - src/App.css
  - src/components/skills/SkillDetailView.tsx
  - src-tauri/tauri.conf.json
  - README.md
  - featured-skills.json
findings:
  critical: 1
  warning: 1
  info: 0
  total: 2
status: issues_found
---

# Phase 260416-nwg: Code Review Report

**Reviewed:** 2026-04-16T18:31:00Z
**Depth:** standard
**Files Reviewed:** 16
**Status:** issues_found

## Summary

Reviewed the cherry-picked/manual-port changes around sparse git checkout, recursive skill discovery, project sync path resolution, Hermes adapter support, and the new `overwriteIfSameContent` sync behavior.

The main risks are in filesystem path handling. I found one security issue in marketplace plugin path resolution that allows repo-controlled paths to escape the checked-out repo, and one correctness issue where deleting a managed skill still cleans project artifacts using the global tool path instead of the project-relative path introduced in this task.

## Critical Issues

### CR-01: Marketplace manifest can escape the repo root and scan/copy arbitrary local directories

**File:** `src-tauri/src/core/installer.rs:605-607`
**Issue:** `parse_marketplace_json()` trusts each plugin `source` from `.claude-plugin/marketplace.json` and resolves it with `repo_dir.join(cleaned)`. If a malicious repo sets `source` to values like `../../..` or another upward traversal, the resolved path can point outside the cloned repo. That path is then fed into marketplace scanning and later candidate installation flows, which can read `SKILL.md` metadata and even copy arbitrary local directories into the central repo if the user selects the candidate. This is a path traversal / local file exposure bug driven by untrusted repository content.
**Fix:** Canonicalize the resolved path and reject anything that does not remain under the repo root.

```rust
fn parse_marketplace_json(repo_dir: &Path) -> Vec<PathBuf> {
    let manifest_path = repo_dir.join(".claude-plugin/marketplace.json");
    let content = match std::fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let manifest: MarketplaceManifest = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(_) => return vec![],
    };
    let plugins = match manifest.plugins {
        Some(p) => p,
        None => return vec![],
    };

    let repo_root = match repo_dir.canonicalize() {
        Ok(p) => p,
        Err(_) => return vec![],
    };

    plugins
        .iter()
        .filter_map(|plugin| {
            let source = plugin.source.as_ref()?;
            let cleaned = source.strip_prefix("./").unwrap_or(source);
            let resolved = repo_dir.join(cleaned).canonicalize().ok()?;
            if resolved.starts_with(&repo_root) && resolved.exists() {
                Some(resolved)
            } else {
                None
            }
        })
        .collect()
}
```

## Warnings

### WR-01: Managed-skill deletion still cleans project artifacts using global tool paths

**File:** `src-tauri/src/commands/mod.rs:894-907`
**Issue:** `delete_managed_skill()` was not updated alongside `project_sync.rs`. It still builds project cleanup targets with `adapter.relative_skills_dir`, but project sync now uses `project_relative_skills_dir(&adapter)`. For tools whose project path differs from the global path (for example Cursor, OpenCode, Codex, OpenClaw, etc.), deleting a managed skill will leave stale project-linked artifacts behind even though the DB records are removed.
**Fix:** Use the same project path helper as project sync so deletion and sync target the same location.

```rust
if let Some(adapter) = crate::core::tool_adapters::adapter_by_key(&assignment.tool) {
    let project_path = std::path::Path::new(&project.path);
    let target = project_path
        .join(crate::core::tool_adapters::project_relative_skills_dir(&adapter))
        .join(name);
    if let Err(e) = remove_path_any(&target) {
        remove_failures.push(format!("{}: {}", target.display(), e));
    }
}
```

---

_Reviewed: 2026-04-16T18:31:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
