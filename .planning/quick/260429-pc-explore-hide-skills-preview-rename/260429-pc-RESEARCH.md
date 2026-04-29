# Quick Task: Explore Page - Hide Skills + Preview Rename

**Researched:** 2026-04-29
**Domain:** Frontend (React) + Backend (Rust/SQLite) Explore page modifications
**Confidence:** HIGH

## Summary

Three changes to the Explore page: (1) rename "View" button to "Preview" (trivial i18n key change), (2) add a hide button per skill card that persists to SQLite so hidden skills disappear from future searches, (3) add a "Show hidden skills" checkbox that defaults to off. The existing codebase provides clear patterns for all three.

**Primary recommendation:** Use a new `hidden_explore_skills` table (keyed on `source_url`) with a V7 migration, filter on the frontend side using a Set derived from a new Tauri command, and persist the "show hidden" checkbox in localStorage (matching the `groupByRepo` pattern).

## Findings

### 1. Current "View" Button Location

The button is in `src/components/skills/ExplorePage.tsx` at lines 172-186 (featured cards) and 245-255 (online results). Both render:

```tsx
<button className="explore-btn-view" ...>
  <Eye size={12} />
  {t("exploreView")}
</button>
```

The i18n key `exploreView` is defined in `src/i18n/resources.ts` line 156 (EN: "View") and line 563 (ZH: "View"). [VERIFIED: codebase grep]

**Action:** Change both EN and ZH values from `"View"` to `"Preview"`. No other code changes needed -- the `t("exploreView")` call stays the same.

### 2. Database Schema for Hidden Skills

**Current schema version:** 6 (`SCHEMA_VERSION` constant in `skill_store.rs` line 15). [VERIFIED: codebase read]

**Migration pattern:** The `ensure_schema()` method uses `PRAGMA user_version` with incremental `if user_version < N` blocks (lines 195-228). Next migration is V7.

**Recommended approach: New table `hidden_explore_skills`**

```sql
CREATE TABLE IF NOT EXISTS hidden_explore_skills (
  source_url TEXT PRIMARY KEY,
  hidden_at INTEGER NOT NULL
);
```

**Rationale for new table (not a column on `skills`):**

- Hidden skills are NOT installed skills -- they are online/featured skills the user hasn't installed. There's no row in the `skills` table for them.
- The key is `source_url` because that's the unique identifier shared by both `FeaturedSkillDto` and `OnlineSkillDto`.
- A separate table keeps the concern isolated and doesn't pollute installed-skill queries.

**Migration V7 code:**

```rust
if user_version < 7 {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS hidden_explore_skills (
            source_url TEXT PRIMARY KEY,
            hidden_at INTEGER NOT NULL
        );"
    )?;
}
```

Bump `SCHEMA_VERSION` to 7.

### 3. Backend Commands Needed

Two new Tauri commands:

| Command                     | Parameters          | Returns       | Purpose                           |
| --------------------------- | ------------------- | ------------- | --------------------------------- |
| `hide_explore_skill`        | `sourceUrl: String` | `()`          | INSERT into hidden_explore_skills |
| `unhide_explore_skill`      | `sourceUrl: String` | `()`          | DELETE from hidden_explore_skills |
| `get_hidden_explore_skills` | none                | `Vec<String>` | SELECT all source_url values      |

Follow the existing command pattern: async + `spawn_blocking`, `store.method()` calls, `format_anyhow_error` on error. Register in `generate_handler!` in `lib.rs`.

### 4. Filtering Strategy: Frontend-Side

**Recommendation: Filter on the frontend using a `Set<string>` of hidden source_urls.**

Reasons:

- Featured skills are loaded once from a JSON file and cached in memory (line 848: `if (featuredSkills.length > 0) return`). Backend filtering would require passing the hidden list into `fetch_featured_skills` -- more invasive.
- Online search results come from the skills.sh API. Adding filtering there would couple the hidden list to the network call.
- The hidden list will be small (tens to low hundreds) -- O(1) Set lookup is trivial.
- The "show hidden" toggle needs instant feedback without re-fetching.

**Data flow:**

1. On Explore tab activation, call `get_hidden_explore_skills` once to populate a `Set<string>`.
2. In `ExplorePage`, filter `featuredSkills` and `searchResults` through the set (unless "show hidden" is checked).
3. When user clicks hide, call `hide_explore_skill`, then add to local Set (optimistic update).
4. When user clicks unhide (visible in "show hidden" mode), call `unhide_explore_skill`, remove from Set.

### 5. "Show Hidden Skills" Checkbox Pattern

The existing `groupByRepo` checkbox pattern in `App.tsx`:

- State: `useState(() => localStorage.getItem(key) === "true")`
- Persist: `useEffect` writes to `localStorage` on change
- Pass as prop to page component

**Recommended approach:** Same localStorage pattern. Key: `"explore-showHidden"`, default: `false`.

The checkbox should live in the Explore page hero area (near the search input), similar to how `groupByRepo` sits in the filter bar. Since Explore's hero row already has search + Manual button, add the checkbox below or beside the source label.

### 6. Frontend Component Changes

`ExplorePageProps` needs new props:

```typescript
hiddenSkills: Set<string>
showHidden: boolean
onShowHiddenChange: (value: boolean) => void
onHideSkill: (sourceUrl: string) => void
onUnhideSkill: (sourceUrl: string) => void
```

Each card gets a hide/unhide button (small, secondary style). Use `EyeOff` icon from lucide-react for hide, `Eye` for unhide.

### 7. Pitfalls

| Pitfall                                                               | Prevention                                                                                               |
| --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| `source_url` format inconsistency between featured and online results | Both DTOs have `source_url: string` with the same format -- no normalization needed [VERIFIED: types.ts] |
| Hiding a skill then installing it -- should it stay hidden?           | Yes -- hidden means "don't show in Explore". Installation is separate. User can always unhide.           |
| Migration failure on existing DB                                      | V7 migration is additive (new table) -- no data loss risk                                                |
| Props drilling bloat in App.tsx                                       | Keep hidden state + toggle in App.tsx but pass only the minimal interface to ExplorePage                 |

### 8. i18n Keys Needed

```typescript
exploreView: "Preview",           // rename from "View"
exploreHide: "Hide",              // hide button text
exploreUnhide: "Show",            // unhide button text
exploreShowHidden: "Show hidden", // checkbox label
```

### 9. SkillStore Methods Needed

```rust
pub fn hide_explore_skill(&self, source_url: &str) -> Result<()>
pub fn unhide_explore_skill(&self, source_url: &str) -> Result<()>
pub fn list_hidden_explore_skills(&self) -> Result<Vec<String>>
```

## Implementation Order

1. **Backend:** Add V7 migration + 3 `SkillStore` methods + 3 Tauri commands + register in `generate_handler!`
2. **Frontend state:** Add `hiddenSkills` Set + `showHidden` boolean + handlers in `App.tsx`
3. **ExplorePage UI:** Rename button text, add hide/unhide buttons, add checkbox, apply filter
4. **i18n:** Update `exploreView` value, add new keys (EN only per project constraint)

## Sources

- `src/components/skills/ExplorePage.tsx` -- current Explore page structure [VERIFIED: codebase read]
- `src-tauri/src/core/skill_store.rs` -- schema version 6, migration pattern [VERIFIED: codebase read]
- `src-tauri/src/commands/mod.rs` -- command pattern, `format_anyhow_error` [VERIFIED: codebase read]
- `src/i18n/resources.ts` -- `exploreView` key at line 156/563 [VERIFIED: codebase grep]
- `src/App.tsx` -- `groupByRepo` localStorage pattern at lines 50/117/345 [VERIFIED: codebase grep]
- `src/components/skills/types.ts` -- `FeaturedSkillDto.source_url`, `OnlineSkillDto.source_url` [VERIFIED: codebase read]
