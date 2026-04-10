---
phase: quick
plan: 260409-rnu
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/commands/mod.rs
  - src-tauri/src/lib.rs
  - src/App.tsx
autonomous: true
must_haves:
  truths:
    - "handleImport syncs to tools when autoSyncEnabled is true (existing behavior preserved)"
    - "handleImport removes all variant source paths when autoSyncEnabled is false (clean migration)"
    - "handleImport still imports the skill to central repo regardless of autoSyncEnabled"
    - "remove_skill_source command refuses to delete paths not under a known tool skills directory"
  artifacts:
    - path: "src/App.tsx"
      provides: "autoSyncEnabled branch in handleImport: sync when ON, cleanup when OFF"
      contains: "if (autoSyncEnabled)"
    - path: "src-tauri/src/commands/mod.rs"
      provides: "remove_skill_source Tauri command with path safety validation"
      contains: "pub async fn remove_skill_source"
    - path: "src-tauri/src/lib.rs"
      provides: "remove_skill_source registered in generate_handler!"
      contains: "commands::remove_skill_source"
  key_links:
    - from: "handleImport else branch"
      to: "remove_skill_source command"
      via: "invokeTauri('remove_skill_source', { path: variant.path })"
      pattern: "remove_skill_source"
    - from: "remove_skill_source command"
      to: "remove_path_any"
      via: "sync_engine::remove_path_any after path validation"
      pattern: "remove_path_any"
---

<objective>
Wire handleImport to respect autoSyncEnabled: when ON, sync skills back to tool dirs (current behavior); when OFF, import to central repo and delete originals from ALL tool directory variant locations (clean migration into Skills Hub).

Purpose: handleImport is the only install flow that ignores autoSyncEnabled. When a user has auto-sync OFF, importing during onboarding should move skills into the hub and clean up the originals rather than recreating symlinks.

Output: New backend command `remove_skill_source` with safety validation + frontend branch in handleImport.
</objective>

<execution_context>
@/home/alexwsl/skills-hub/.claude/get-shit-done/workflows/execute-plan.md
@/home/alexwsl/skills-hub/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src/App.tsx
@src-tauri/src/commands/mod.rs
@src-tauri/src/lib.rs
@src-tauri/src/core/sync_engine.rs
@src-tauri/src/core/tool_adapters/mod.rs
@src/components/skills/types.ts

<interfaces>
<!-- Key types and contracts the executor needs -->

From src/components/skills/types.ts:

```typescript
export type OnboardingVariant = {
  tool: string;
  name: string;
  path: string; // absolute path to skill dir in a tool folder
  fingerprint?: string | null;
  is_link: boolean;
  link_target?: string | null;
};

export type OnboardingGroup = {
  name: string;
  variants: OnboardingVariant[]; // ALL locations of this skill across tool dirs
  has_conflict: boolean;
};
```

From src-tauri/src/core/sync_engine.rs (line 137):

```rust
pub(crate) fn remove_path_any(path: &Path) -> Result<()>
// Handles symlinks, dirs, files. Returns Ok(()) if not found.
```

From src-tauri/src/core/tool_adapters/mod.rs:

```rust
pub struct ToolAdapter {
    pub id: ToolId,
    pub display_name: &'static str,
    pub relative_skills_dir: &'static str,  // e.g. ".claude/skills"
    pub relative_detect_dir: &'static str,
}
pub fn default_tool_adapters() -> Vec<ToolAdapter>;
```

From src-tauri/src/commands/mod.rs:

```rust
// Already imported:
use crate::core::sync_engine::{..., remove_path_any, ...};
// Not yet imported but needed:
use crate::core::tool_adapters::default_tool_adapters;  // already have adapter_by_key etc.
```

</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add remove_skill_source Tauri command with path safety validation</name>
  <files>src-tauri/src/commands/mod.rs, src-tauri/src/lib.rs</files>
  <action>
1. In `src-tauri/src/commands/mod.rs`:

a. Add `default_tool_adapters` to the existing import from `crate::core::tool_adapters`:
Change the import line to include it: `use crate::core::tool_adapters::{adapter_by_key, default_tool_adapters, is_tool_installed, resolve_default_path};`

b. Add a new Tauri command near `import_existing_skill` (after line 797):

      ```rust
      #[tauri::command]
      pub async fn remove_skill_source(path: String) -> Result<(), String> {
          tauri::async_runtime::spawn_blocking(move || {
              let target = std::path::PathBuf::from(&path);

              // Safety: only allow deletion of paths under known tool skill directories.
              let home = dirs::home_dir()
                  .ok_or_else(|| anyhow::anyhow!("cannot resolve home directory"))?;
              let adapters = default_tool_adapters();
              let is_safe = adapters.iter().any(|adapter| {
                  let tool_skills_dir = home.join(adapter.relative_skills_dir);
                  target.starts_with(&tool_skills_dir)
              });
              if !is_safe {
                  anyhow::bail!(
                      "UNSAFE_PATH|path is not under a known tool skills directory: {}",
                      path
                  );
              }

              remove_path_any(&target)?;
              Ok::<_, anyhow::Error>(())
          })
          .await
          .map_err(|err| err.to_string())?
          .map_err(format_anyhow_error)
      }
      ```

This validates the path is under `~/.<tool>/skills/` (or equivalent) for one of the 40+ known tool adapters before calling `remove_path_any`. The `remove_path_any` function already handles symlinks, real directories, regular files, and missing paths gracefully.

2. In `src-tauri/src/lib.rs`, add `commands::remove_skill_source` to the `generate_handler![]` macro. Place it near `commands::import_existing_skill` (around line 100):
   ```rust
   commands::import_existing_skill,
   commands::remove_skill_source,
   ```
     </action>
     <verify>
       <automated>cd /home/alexwsl/skills-hub && cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5</automated>
     </verify>
     <done>
       - `remove_skill_source` command exists in commands/mod.rs
       - It validates paths against all known tool skill directories before deletion
       - It is registered in generate_handler! in lib.rs
       - `cargo check` passes
     </done>
   </task>

<task type="auto">
  <name>Task 2: Branch handleImport on autoSyncEnabled -- sync when ON, cleanup variants when OFF</name>
  <files>src/App.tsx</files>
  <action>
In `src/App.tsx`, modify the `handleImport` function (line 986). After the `import_existing_skill` call completes (line 1008), the code currently unconditionally runs the sync loop (lines 1010-1059).

Replace the unconditional sync loop with a conditional branch:

```typescript
if (autoSyncEnabled) {
  // Existing sync loop (lines 1010-1059) -- move entirely inside this block
  const selectedInstalledIds = tools
    .filter((tool) => syncTargets[tool.id] && isInstalled(tool.id))
    .map((t) => t.id);
  const targets = uniqueToolIdsBySkillsDir(selectedInstalledIds)
    .map((id) => tools.find((t) => t.id === id))
    .filter(Boolean) as ToolOption[];
  for (const tool of targets) {
    setActionMessage(
      t("actions.syncing", { name: group.name, tool: tool.label }),
    );
    try {
      const overwrite = Boolean(
        chosenVariantTool &&
        (chosenVariantTool === tool.id ||
          (sharedToolIdsByToolId[chosenVariantTool] ?? []).includes(tool.id)),
      );
      await invokeTauri("sync_skill_to_tool", {
        sourcePath: installResult.central_path,
        skillId: installResult.skill_id,
        tool: tool.id,
        name: group.name,
        overwrite,
      });
    } catch (err) {
      const raw = err instanceof Error ? err.message : String(err);
      if (raw.startsWith("TARGET_EXISTS|")) {
        const targetPath = raw.split("|")[1] ?? "";
        collectedErrors.push({
          title: t("errors.syncFailedTitle", {
            name: group.name,
            tool: tool.label,
          }),
          message: t("errors.syncTargetExistsMessage", {
            path: targetPath,
          }),
        });
      } else {
        collectedErrors.push({
          title: t("errors.syncFailedTitle", {
            name: group.name,
            tool: tool.label,
          }),
          message: raw,
        });
      }
    }
  }
} else {
  // Auto-sync OFF: clean migration -- remove originals from all tool directories
  for (const variant of group.variants) {
    try {
      await invokeTauri("remove_skill_source", { path: variant.path });
    } catch (err) {
      // Non-fatal: skill is already imported, cleanup failure is secondary
      const raw = err instanceof Error ? err.message : String(err);
      collectedErrors.push({
        title: t("errors.syncFailedTitle", {
          name: group.name,
          tool: variant.tool,
        }),
        message: raw,
      });
    }
  }
}
```

Key points:

- The `import_existing_skill` call (lines 1001-1008) stays OUTSIDE the branch -- it always runs.
- The `selectedInstalledIds`, `targets`, and sync variables move INSIDE the `if` block to avoid unused-variable lint errors.
- The `else` branch iterates ALL `group.variants` (not just the chosen one) to remove duplicates across tool directories.
- Errors in the cleanup loop are collected but non-fatal (the skill is already safely in central repo).

Do NOT modify any other install flow -- they already handle autoSyncEnabled correctly.
</action>
<verify>
<automated>cd /home/alexwsl/skills-hub && npm run check 2>&1 | tail -20</automated>
</verify>
<done> - handleImport has `if (autoSyncEnabled) { sync loop } else { cleanup variants }` branch - import_existing_skill always runs regardless of autoSyncEnabled - The else branch calls `remove_skill_source` for every variant in the group - `npm run check` passes (lint + build + rust:fmt:check + rust:clippy + rust:test) - No other install flows modified
</done>
</task>

</tasks>

<threat_model>

## Trust Boundaries

| Boundary                        | Description                                                |
| ------------------------------- | ---------------------------------------------------------- |
| Frontend -> remove_skill_source | Frontend passes arbitrary path string from onboarding plan |

## STRIDE Threat Register

| Threat ID  | Category          | Component           | Disposition | Mitigation Plan                                                                                                                           |
| ---------- | ----------------- | ------------------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| T-quick-01 | Tampering         | remove_skill_source | mitigate    | Validate path is under a known tool skills directory (~/.<tool>/skills/) before deletion. Refuses arbitrary paths with UNSAFE_PATH error. |
| T-quick-02 | Denial of Service | remove_skill_source | accept      | Deletes only the single specified directory. Already-missing paths return Ok. Impact limited to one skill folder under a tool dir.        |

</threat_model>

<verification>
1. `cargo check` passes for Rust backend (new command compiles, imports resolve).
2. `npm run check` passes (lint, tsc, clippy, rust:fmt:check, rust:test).
3. `remove_skill_source` validates paths against known tool skill directories before deletion.
4. `handleImport` branches: autoSyncEnabled ON runs sync loop, OFF removes variant paths.
5. No other install flows are modified.
</verification>

<success_criteria>

- `npm run check` passes clean
- handleImport has `if (autoSyncEnabled) { sync } else { cleanup variants }` branch
- New `remove_skill_source` Tauri command exists with path safety validation against all 40+ tool adapters
- Command is registered in `generate_handler!`
- No other install flows are modified
  </success_criteria>

<output>
After completion, create `.planning/quick/260409-rnu-wire-handleimport-to-respect-autosyncena/260409-rnu-SUMMARY.md`
</output>
