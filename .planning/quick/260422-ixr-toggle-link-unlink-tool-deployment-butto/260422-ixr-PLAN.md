---
phase: quick-260422-ixr
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/App.tsx
  - src/components/skills/SkillCard.tsx
  - src/components/skills/SkillsList.tsx
  - src/i18n/resources.ts
autonomous: true
must_haves:
  truths:
    - "When a skill has targets (is synced to tools), the button shows a Link icon and clicking it unsyncs from all tools"
    - "When a skill has NO targets (is not synced), the button shows an Unlink icon and clicking it syncs to all installed tools"
    - "The toggle button is never disabled except during loading"
    - "Tooltip text changes dynamically to describe what the click will do"
  artifacts:
    - path: "src/components/skills/SkillCard.tsx"
      provides: "Toggle button with dynamic Link/Unlink icon and conditional callback"
    - path: "src/App.tsx"
      provides: "handleSyncSkillToAllTools handler"
    - path: "src/components/skills/SkillsList.tsx"
      provides: "Prop threading for onSyncSkillToAllTools"
    - path: "src/i18n/resources.ts"
      provides: "i18n keys for both synced/unsynced tooltip states"
  key_links:
    - from: "src/components/skills/SkillCard.tsx"
      to: "src/App.tsx"
      via: "onSyncSkillToAllTools and onUnsync callbacks"
    - from: "src/components/skills/SkillsList.tsx"
      to: "src/components/skills/SkillCard.tsx"
      via: "prop threading of onSyncSkillToAllTools"
---

<objective>
Change the Unlink button on SkillCard to a toggle that syncs/unsyncs a skill to/from all installed tools, with dynamic icon state (Link when synced, Unlink when not synced).

Purpose: Currently the unlink button becomes permanently disabled once a skill is unsynced. Users need a way to re-deploy a skill to all tools without manually clicking each tool badge.
Output: A toggle button that always works -- unsyncs when linked, syncs to all installed tools when unlinked.
</objective>

<execution_context>
@/home/alexwsl/skills-hub/.claude/get-shit-done/workflows/execute-plan.md
@/home/alexwsl/skills-hub/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src/components/skills/SkillCard.tsx
@src/components/skills/SkillsList.tsx
@src/App.tsx
@src/i18n/resources.ts

<interfaces>
<!-- Key types and contracts the executor needs -->

From src/components/skills/types.ts:

```typescript
export type ManagedSkill = {
  id: string;
  name: string;
  central_path: string;
  source_type: string;
  source_ref: string | null;
  targets: { tool: string; mode: string | null }[];
  // ... other fields
};

export type ToolOption = {
  id: string;
  label: string;
};
```

From src/App.tsx (existing handlers to reference):

```typescript
// Line 665 - unsync all targets for a skill
const handleUnsyncSkill = useCallback(async (skillId: string) => {
  await invokeTauri("unsync_skill", { skillId });
  await loadManagedSkills();
}, [...]);

// Line 1896 - toggle individual tool for a skill (sync/unsync pattern)
const runToggleToolForSkill = useCallback(async (skill: ManagedSkill, toolId: string) => {
  const target = skill.targets.find((t) => t.tool === toolId);
  if (synced) {
    await invokeTauri("unsync_skill_from_tool", { skillId: skill.id, tool: toolId });
  } else {
    await invokeTauri("sync_skill_to_tool", { sourcePath: skill.central_path, skillId: skill.id, tool: toolId, name: skill.name });
  }
}, [...]);

// Line 502 - deduplicates tool IDs by skills_dir
const uniqueToolIdsBySkillsDir = useCallback((toolIds: string[]) => { ... }, [toolInfos]);

// Line 523 - checks if a tool is installed
const isInstalled = useCallback((id: string) => installedToolIds.includes(id), [installedToolIds]);

// Line 527 - filtered tools list
const installedTools = useMemo(() => tools.filter((tool) => installedToolIds.includes(tool.id)), [...]);
```

</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add i18n keys and create handleSyncSkillToAllTools in App.tsx</name>
  <files>src/i18n/resources.ts, src/App.tsx</files>
  <action>
1. In `src/i18n/resources.ts`, add two new English translation keys near the existing `unsyncSkillTooltip` key (around line 327):
   - `syncSkillTooltip: "Deploy this skill to all installed tools"` (for when skill has NO targets)
   - `unsyncSkillTooltip` already exists as `"Remove this skill from all tool directories"` -- keep as-is

2. In `src/App.tsx`, create a new `handleSyncSkillToAllTools` handler right after the `handleUnsyncSkill` definition (after line 675). This handler:
   - Signature: `async (skill: ManagedSkill) => void`
   - Gets the deduplicated installed tool IDs: `const targetIds = uniqueToolIdsBySkillsDir(installedToolIds.filter((id) => isInstalled(id)))`
   - If `targetIds.length === 0`, return early (no tools installed)
   - Sets `loading` true, `loadingStartAt` to `Date.now()`, `error` to null
   - Loops over `targetIds`, for each tool:
     - Sets `actionMessage` using `t("actions.syncing", { name: skill.name, tool: toolLabel })` (reuse existing key)
     - Calls `await invokeTauri("sync_skill_to_tool", { sourcePath: skill.central_path, skillId: skill.id, tool: toolId, name: skill.name })`
     - Catches per-tool errors: if `TOOL_NOT_INSTALLED|` or `TOOL_NOT_WRITABLE|` prefix, continue silently; otherwise toast.error
   - After loop: set success toast, clear actionMessage, call `loadManagedSkills()`
   - Finally block: set loading false, loadingStartAt null
   - Wrap in `useCallback` with deps: `[invokeTauri, installedToolIds, isInstalled, loadManagedSkills, t, tools, uniqueToolIdsBySkillsDir]`

3. In `src/App.tsx`, pass the new handler to `SkillsList` at the usage site (~line 2090). Add prop:
   `onSyncSkillToAllTools={handleSyncSkillToAllTools}`
   </action>
   <verify>
   <automated>cd /home/alexwsl/skills-hub && npx tsc --noEmit 2>&1 | head -30</automated>
   </verify>
   <done>New handler exists in App.tsx and is passed to SkillsList. i18n key `syncSkillTooltip` is defined. TypeScript compiles without errors (after Task 2 completes the prop threading).</done>
   </task>

<task type="auto">
  <name>Task 2: Thread prop through SkillsList and convert SkillCard button to toggle</name>
  <files>src/components/skills/SkillsList.tsx, src/components/skills/SkillCard.tsx</files>
  <action>
1. In `src/components/skills/SkillsList.tsx`:
   - Add `onSyncSkillToAllTools: (skill: ManagedSkill) => void` to `SkillsListProps` type
   - Add `onSyncSkillToAllTools` to the destructured props
   - Pass `onSyncSkillToAllTools={onSyncSkillToAllTools}` to `SkillCard` in the `renderSkill` function

2. In `src/components/skills/SkillCard.tsx`:
   - Add `Link` to the lucide-react import (alongside existing `Unlink`)
   - Add `onSyncToAllTools: (skill: ManagedSkill) => void` to `SkillCardProps` type
   - Add `onSyncToAllTools` to the destructured props

3. In `src/components/skills/SkillCard.tsx`, replace the existing Unlink button (lines 195-204) with a toggle button:
   ```tsx
   <button
     className="card-btn secondary-action"
     type="button"
     onClick={() =>
       skill.targets.length > 0 ? onUnsync(skill.id) : onSyncToAllTools(skill)
     }
     disabled={loading}
     aria-label={
       skill.targets.length > 0
         ? t("unsyncSkillTooltip")
         : t("syncSkillTooltip")
     }
     title={
       skill.targets.length > 0
         ? t("unsyncSkillTooltip")
         : t("syncSkillTooltip")
     }
   >
     {skill.targets.length > 0 ? <Link size={16} /> : <Unlink size={16} />}
   </button>
   ```
   Key changes from original:
   - Removed `skill.targets.length === 0` from disabled condition -- only `loading` disables
   - onClick conditionally calls `onUnsync` (has targets) or `onSyncToAllTools` (no targets)
   - Icon is `Link` when synced (targets exist), `Unlink` when not synced (no targets)
   - Tooltip/aria-label switches between the two i18n keys based on state
     </action>
     <verify>
     <automated>cd /home/alexwsl/skills-hub && npm run check 2>&1 | tail -20</automated>
     </verify>
     <done>
   - SkillCard toggle button renders Link icon when `skill.targets.length > 0`, Unlink icon when `skill.targets.length === 0`
   - Clicking when synced calls `onUnsync(skill.id)` to remove all tool targets
   - Clicking when unsynced calls `onSyncToAllTools(skill)` to sync to all installed tools
   - Button is only disabled during loading (never dead/stuck)
   - Tooltip dynamically describes the action
   - `npm run check` passes (lint + build + rust checks)
     </done>
     </task>

</tasks>

<verification>
- `npm run check` passes (lint, TypeScript build, Rust fmt/clippy/test)
- SkillCard shows Link icon for skills with targets, Unlink icon for skills without targets
- Clicking the button when synced calls unsync_skill to remove all tool links
- Clicking the button when unsynced calls sync_skill_to_tool for each installed tool
- Button tooltip changes based on sync state
- Button is never disabled except during loading
</verification>

<success_criteria>
The link/unlink toggle button on SkillCard dynamically reflects sync state and allows users to both deploy and undeploy a skill from all installed tools with a single click.
</success_criteria>

<output>
After completion, create `.planning/quick/260422-ixr-toggle-link-unlink-tool-deployment-butto/260422-ixr-SUMMARY.md`
</output>
