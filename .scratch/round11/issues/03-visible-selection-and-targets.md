# D3 + D4 — A selection you cannot see, and target rows you cannot reach

Status: done — c909028

Two frontend defects with one root cause: **detection filters what the UI renders, while the DB holds
rows and selections for tools detection no longer sees.** One lane, two commits.

## D3 — Hidden selection entries (`ToolConfigModal.tsx`)

```tsx
const [selectedTools, setSelectedTools] = useState<Set<string>>(() =>
  buildInitialSelection(toolStatus, savedSelection),   // seeded from the FULL saved set
);
const [detectedOnly, setDetectedOnly] = useState(true); // default ON
const tools = detectedOnly
  ? allTools.filter((tool) => installed.includes(tool.key))   // undetected rows never render
  : allTools;

const handleConfirm = async () => {
  await onConfirm(Array.from(selectedTools), …);              // whole set written back
};
```

An undetected-but-selected tool is invisible, cannot be unticked, and is **re-persisted on every
save**. Operator-confirmed: the modal showed no Cursor while the DB kept
`global_selected_tools_v1 = ["cursor","claude_code","pi"]` across multiple saves.

This matters much more once **D1** lands, because the selection then *drives writes*: a stale invisible
entry stops being cosmetic.

### Decision (do not re-decide)

A tool renders **regardless of the `detectedOnly` filter** when it is in `selectedTools`. The
filter means "hide undetected tools I have no relationship with", not "hide my own state". Undetected
rendered rows get a "not detected" badge next to the existing `(Installed)` badge, so the operator can
see *why* it looks unfamiliar and untick it.

Do **not** silently drop undetected keys on save — that fixes the symptom by discarding operator data
without telling them.

## D4 — Unreachable target rows (`SkillCard.tsx`)

`SkillCard` iterates `installedTools` to render tool chips, so a `skill_targets` row for an undetected
tool renders nothing and has no per-skill unsync affordance. The only path that reached the operator's
24 orphaned `cursor` rows was `unsync_all_skills` (`RemovalScope::EveryGlobalTarget`) — which would
also have deleted all 96 live Claude Code / Codex / Pi targets. The rows had to be removed with SQL.

### Decision (do not re-decide)

Render a chip for **every target row the skill actually has**, not for every installed tool. A row
whose tool is undetected renders in a muted/"not detected" state and keeps its unsync affordance, so
orphans are removable one tool at a time through the supported path.

Union semantics: chips = (rows the skill has) ∪ (installed tools, for the "not yet synced here"
affordance that exists today). Do not regress the existing ability to sync to an installed tool that
has no row yet.

`unsync_skill_from_tool` already accepts any `tool_key` and `artifact_removal` treats an absent
artifact as a successful removal (row deleted), so **no backend change is required** — verify this
before writing frontend code and say so in your report if it turns out otherwise.

## Files

- `src/components/shared/ToolConfigModal.tsx`
- `src/components/skills/SkillCard.tsx` (and `SkillsList.tsx` / `App.tsx` only if a prop must widen
  from `installedTools` to include target-bearing tools)
- `src/i18n/resources.ts` — the "not detected" string in **both** `en` and `zh`
- `src/App.css` — muted chip style if needed (no CSS Modules; do not add the pattern)

**Do not touch**: `src-tauri/` at all, `src/hooks/useSkillLibrary.ts`, `src/hooks/useSyncOrchestration.ts`
(D1's lane), `core/tool_adapters/` (D2's lane).

## Tests

Components get no JSX tests by design (AGENTS.md), so the coverage here is:

- Any pure helper you extract (e.g. "which tool keys does this card render") goes in
  `src/lib/skillPresentation.ts` — the established home for pure presentation logic — with a table
  test: rows-only tool, installed-only tool, both, neither.
- Do **not** add a component test harness.

## Gate

`npm run version:check && npm run check`.

## Comments

- 2026-09-15 — Status reconciliation: NO STATUS → done — c909028. Evidence: git log v1.2.10..v1.2.11 finds c909028 then 86cfd63, release 9de83e9. src/lib/skillPresentation.ts:378 retains selected undetected choices and :351 iterates actual target rows; backend removal is separately completed by round11/04, not claimed unnecessary.
