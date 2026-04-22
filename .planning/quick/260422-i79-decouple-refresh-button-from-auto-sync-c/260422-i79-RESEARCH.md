# Quick Task 260422-i79: Decouple Refresh Button from Auto-Sync - Research

**Researched:** 2026-04-22
**Domain:** Frontend/Backend refresh flow in Skills Hub
**Confidence:** HIGH

## Summary

The refresh button (`handleRefresh` at App.tsx:937) currently does nothing except re-read the SQLite database via `loadManagedSkills()`. It never contacts source repos. The backend already has `update_managed_skill` (command) / `update_managed_skill_from_source` (core) that handles the full re-download flow for both git and local-path skills. The task is to replace the trivial `handleRefresh` with a batch loop that calls `update_managed_skill` per skill, then conditionally calls `handleSyncAllManagedToTools` based on `autoSyncEnabled`.

**Primary recommendation:** Replace `handleRefresh` body with a batch update loop modeled on `handleSyncAllManagedToTools`, calling `update_managed_skill` per skill with progress messages, then conditionally sync to tools.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

- Re-download + conditional sync: For each managed skill, pull latest from source into central library. If auto-sync ON, also re-sync to tool directories. If OFF, only library updated.
- Use existing loading overlay with per-skill progress messages
- Continue on failure, collect errors, summary toast at end (same pattern as handleSyncAllManagedToTools)

### Claude's Discretion

- None -- all areas discussed.

### Deferred Ideas (OUT OF SCOPE)

- None
  </user_constraints>

## Current State Analysis

### 1. Current `handleRefresh` (App.tsx:937-939)

```typescript
const handleRefresh = useCallback(() => {
  void loadManagedSkills();
}, [loadManagedSkills]);
```

This only re-fetches from SQLite. No network calls, no source repo contact. [VERIFIED: codebase read]

### 2. `update_managed_skill` Command (commands/mod.rs:632-653)

The backend command wraps `update_managed_skill_from_source` in `spawn_blocking`. Returns `UpdateResultDto { skill_id, name, content_hash, source_revision, updated_targets }`. [VERIFIED: codebase read]

### 3. `update_managed_skill_from_source` Core Function (installer.rs:816-1009+)

This function handles the full update flow:

- **Git skills** (`source_type == "git"`): Calls `clone_to_cache()` which does a fresh git clone/fetch, resolves subpaths (including multi-skill repo matching), copies content to a staging dir, swaps old content out. Updates DB with new `content_hash` and `source_revision`.
- **Local-path skills** (`source_type == "local"`): Copies from the original source path (`source_ref`) to staging, then swaps. If the source path no longer exists, it **bails with an error** (`"source path not found"`).
- **Other source types**: Bails with `"unsupported source_type for update"`.
- **Post-update**: Automatically re-syncs copy-mode global targets (symlinks auto-update) AND copy-mode project assignments. This happens INSIDE the backend function regardless of frontend auto-sync setting.

**Key insight:** The backend `update_managed_skill_from_source` already handles re-syncing copy-mode targets (both global `skill_targets` and project `project_skill_assignments`). The frontend auto-sync toggle only controls whether symlink-mode global targets get (re)created via `handleSyncAllManagedToTools`. [VERIFIED: codebase read]

### 4. autoSyncEnabled Guard Locations (App.tsx)

| Line | Context                                           | What It Guards                             |
| ---- | ------------------------------------------------- | ------------------------------------------ |
| 122  | State definition                                  | Default: `true`                            |
| 1017 | After `import_existing_skill`                     | Whether to sync imported skill to tools    |
| 1137 | After `install_local_selection` (single)          | Whether to sync installed skill to tools   |
| 1263 | After `install_git_selection` (single)            | Whether to sync installed skill to tools   |
| 1331 | After `install_git_selection` (multi-select)      | Whether to sync installed skill to tools   |
| 1502 | After `install_local_selection` (multi-select)    | Whether to sync installed skill to tools   |
| 1613 | After `install_git_selection` (batch multi-skill) | Whether to sync installed skill to tools   |
| 1709 | `handleSyncAllManagedToTools`                     | Early return -- prevents ALL sync-to-tools |

The refresh button does NOT currently reference `autoSyncEnabled` at all. The new implementation should reference it to decide whether to call sync-to-tools after updates. [VERIFIED: codebase read]

### 5. `handleSyncAllManagedToTools` Pattern (App.tsx:1707-1779)

This is the pattern to follow for the refresh handler:

```typescript
const handleSyncAllManagedToTools = useCallback(
  async (toolIds: string[]) => {
    if (!autoSyncEnabled) return;          // <-- Guard
    if (managedSkills.length === 0) return;
    // Filter to installed tools, dedup by skills dir
    const installedIds = uniqueToolIdsBySkillsDir(
      toolIds.filter((id) => isInstalled(id)),
    );
    if (installedIds.length === 0) return;

    setLoading(true);
    setLoadingStartAt(Date.now());
    setError(null);
    try {
      const collectedErrors: { title: string; message: string }[] = [];
      for (let si = 0; si < managedSkills.length; si++) {
        const skill = managedSkills[si];
        for (let ti = 0; ti < installedIds.length; ti++) {
          setActionMessage(t("actions.syncStep", { ... }));
          try {
            await invokeTauri("sync_skill_to_tool", { ... });
          } catch (err) {
            // Skip TOOL_NOT_INSTALLED / TOOL_NOT_WRITABLE, collect others
            collectedErrors.push({ title: ..., message: raw });
          }
        }
      }
      setActionMessage(t("status.syncCompleted"));
      setSuccessToastMessage(t("status.syncCompleted"));
      setActionMessage(null);
      await loadManagedSkills();
      if (collectedErrors.length > 0) showActionErrors(collectedErrors);
    } finally {
      setLoading(false);
      setLoadingStartAt(null);
    }
  },
  [autoSyncEnabled, invokeTauri, isInstalled, loadManagedSkills, managedSkills, showActionErrors, t, tools, uniqueToolIdsBySkillsDir],
);
```

### 6. Per-Skill Update Button (App.tsx:1872-1903)

There is already a per-skill "Update" button on each SkillCard that calls `handleUpdateManaged`:

```typescript
const handleUpdateManaged = useCallback(
  async (skill: ManagedSkill) => {
    setLoading(true);
    setLoadingStartAt(Date.now());
    setError(null);
    try {
      setActionMessage(t("actions.updating", { name: skill.name }));
      await invokeTauri<UpdateResultDto>("update_managed_skill", {
        skillId: skill.id,
      });
      setActionMessage(updatedText);
      setSuccessToastMessage(updatedText);
      setActionMessage(null);
      await loadManagedSkills();
    } catch (err) {
      setError(raw); // Single skill: shows error directly
    } finally {
      setLoading(false);
      setLoadingStartAt(null);
    }
  },
  [invokeTauri, loadManagedSkills, t],
);
```

This does NOT handle errors gracefully for batch -- it sets `setError()` on first failure. The new batch refresh should use `collectedErrors` instead. [VERIFIED: codebase read]

### 7. Loading Overlay Pattern

The loading overlay uses three state variables:

- `setLoading(true)` -- shows the overlay
- `setLoadingStartAt(Date.now())` -- enables elapsed time display
- `setActionMessage("...")` -- shows per-step progress text

Reset pattern: `setLoading(false); setLoadingStartAt(null);` in `finally` block. [VERIFIED: codebase read]

### 8. Skills That Cannot Be Updated

- **Local-path skills** where source was deleted: `update_managed_skill_from_source` bails with `"source path not found"`. The batch handler should catch this and add to `collectedErrors`, then continue.
- **Unknown source_type**: Would bail. Extremely unlikely but handled by error collection.
- **Git skills with network errors**: Would bail (clone failure). Catch and continue.

The per-skill update button already encounters these scenarios for individual skills. The batch approach just needs to collect rather than halt. [VERIFIED: codebase read]

## Architecture Patterns

### New `handleRefresh` Structure

```typescript
const handleRefresh = useCallback(async () => {
  if (managedSkills.length === 0) return;

  setLoading(true);
  setLoadingStartAt(Date.now());
  setError(null);

  try {
    const collectedErrors: { title: string; message: string }[] = [];

    // Phase 1: Update all skills from source
    for (let i = 0; i < managedSkills.length; i++) {
      const skill = managedSkills[i];
      setActionMessage(
        t("actions.updatingStep", {
          index: i + 1,
          total: managedSkills.length,
          name: skill.name,
        }),
      );
      try {
        await invokeTauri<UpdateResultDto>("update_managed_skill", {
          skillId: skill.id,
        });
      } catch (err) {
        const raw = err instanceof Error ? err.message : String(err);
        collectedErrors.push({
          title: t("errors.updateFailedTitle", { name: skill.name }),
          message: raw,
        });
      }
    }

    // Phase 2: Conditional sync to tool directories
    if (autoSyncEnabled) {
      // Re-fetch skills to get updated central_paths/hashes
      // Then run sync-to-tools logic (inline or call handleSyncAllManagedToTools)
    }

    setActionMessage(t("status.refreshCompleted"));
    setSuccessToastMessage(t("status.refreshCompleted"));
    setActionMessage(null);
    await loadManagedSkills();
    if (collectedErrors.length > 0) showActionErrors(collectedErrors);
  } finally {
    setLoading(false);
    setLoadingStartAt(null);
  }
}, [
  autoSyncEnabled,
  invokeTauri,
  loadManagedSkills,
  managedSkills,
  showActionErrors,
  t /* sync deps */,
]);
```

### Implementation Notes

1. **Cannot reuse `handleSyncAllManagedToTools` directly** for Phase 2 because it has its own `setLoading(true)` / `finally { setLoading(false) }` which would conflict with the outer loading state. Either: (a) inline the sync-to-tools loop, or (b) extract a shared helper that doesn't manage loading state. Option (a) is simpler for a quick task.

2. **Phase 2 sync needs refreshed `managedSkills`**: After updating all skills, `managedSkills` state is stale. Either: (a) call `loadManagedSkills()` between phases and use the returned data, or (b) use the `UpdateResultDto` responses to get fresh `central_path` values. However, `loadManagedSkills` returns void and updates state async, so the simplest approach is to re-fetch via `invokeTauri('get_managed_skills')` directly to get the fresh list for sync.

3. **New i18n keys needed** (English only per project constraints):
   - `actions.updatingStep`: `"Updating ({{index}}/{{total}}) {{name}} ..."`
   - `status.refreshCompleted`: `"All skills refreshed."`
   - `errors.updateFailedTitle`: `"Failed to update {{name}}"`

4. **The `handleSyncAllManagedToTools` guard**: The `if (!autoSyncEnabled) return` guard at line 1709 is fine -- it protects the sync-all-to-tools from being called independently. The new refresh handler should NOT call `handleSyncAllManagedToTools` (to avoid the loading state conflict); instead it should inline the sync logic.

## Common Pitfalls

### Pitfall 1: Loading State Double-Set

**What goes wrong:** Calling `handleSyncAllManagedToTools` from within `handleRefresh` causes `setLoading(false)` in the inner function's `finally` to fire before the outer function completes.
**How to avoid:** Inline the sync-to-tools loop inside `handleRefresh` rather than delegating to the existing callback.

### Pitfall 2: Stale managedSkills for Sync Phase

**What goes wrong:** After updating skills, `managedSkills` React state still holds pre-update data (stale `central_path`, old hashes). Syncing with stale data could sync old content.
**How to avoid:** After the update loop, do a fresh `invokeTauri<ManagedSkill[]>('get_managed_skills')` and use that returned array for the sync loop.

### Pitfall 3: Backend Already Syncs Copy-Mode Targets

**What goes wrong:** `update_managed_skill_from_source` already re-syncs copy-mode global targets and project assignments. If the frontend then also syncs those same targets, it's redundant (but harmless -- idempotent).
**How to avoid:** This is acceptable. The frontend sync-to-tools handles symlink-mode targets and ensures all tools are covered. The double-copy for copy-mode targets is idempotent.

## Sources

### Primary (HIGH confidence)

- `src/App.tsx` lines 937, 1707-1779, 1872-1903 -- current refresh, sync-all, and per-skill update handlers
- `src-tauri/src/commands/mod.rs` lines 632-653 -- update_managed_skill command
- `src-tauri/src/core/installer.rs` lines 816-1009+ -- update_managed_skill_from_source core logic
- `src/components/skills/types.ts` lines 81-87 -- UpdateResultDto type
- `src/i18n/resources.ts` -- existing i18n keys for actions

## Metadata

**Confidence breakdown:**

- Current flow analysis: HIGH -- direct codebase reads
- Implementation pattern: HIGH -- follows established patterns in same file
- Edge cases: HIGH -- backend error handling verified in source

**Research date:** 2026-04-22
**Valid until:** 2026-05-22
