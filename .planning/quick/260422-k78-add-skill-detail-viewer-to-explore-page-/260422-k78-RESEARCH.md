# Quick Task 260422-k78: Add Skill Detail Viewer to Explore Page - Research

**Researched:** 2026-04-22
**Domain:** Tauri IPC + git clone caching + React view routing
**Confidence:** HIGH

## Summary

This task adds a "View" button to Explore page cards that clones the skill repo into a temporary cache dir and opens the existing `SkillDetailView` component. The investigation confirms that all major building blocks exist and can be reused with minimal adaptation. The git clone infrastructure (`git_fetcher.rs::clone_or_pull`) already does shallow `--depth 1` clones when using the system git binary. The `list_skill_files` and `read_skill_file` commands accept any path string -- they are not tied to the central repo, so the explore-cache path works out of the box.

**Primary recommendation:** Create one new Tauri command (`clone_explore_skill`) that calls `clone_or_pull` targeting `~/.skillshub/.explore-cache/<hash>/`, add startup cleanup in `lib.rs`, add `explore-detail` view state in App.tsx, and pass a synthetic `ManagedSkill` object to the existing `SkillDetailView`.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Implementation Decisions

- Shallow git clone into `~/.skillshub/.explore-cache/<owner>/<repo>/`
- Reuses existing `git_fetcher.rs` clone logic
- Produces a real directory on disk that existing `list_skill_files` / `read_skill_file` commands can read from with zero changes
- New backend command: `clone_explore_skill` that returns the cache path
- Staleness check: `git fetch --depth 1` before reuse in same session
- Cached clones persist for the duration of the app session
- On next app launch, startup cleanup wipes `.explore-cache/` directory
- Fits existing `cache_cleanup.rs` startup pattern
- Re-opening same skill in same session = instant (no re-clone)
- No user-facing cache management needed
- Reuse `SkillDetailView` as-is -- pass the explore-cache path as `centralPath`
- Construct a minimal ManagedSkill-like object with: central_path = cache path, name from explore card, source_ref = GitHub URL, source_type = 'github'
- Add optional "Install" button to SkillDetailView header when viewing an explore skill
- Back button returns to Explore page (not My Skills) when in explore-detail mode
- New view state `explore-detail` in App.tsx to distinguish from regular `detail` view

### Claude's Discretion

- Exact directory structure within `.explore-cache/`
- Whether to show a loading spinner during clone operation
- Error handling UX when clone fails (e.g., no network)
  </user_constraints>

## Architecture Patterns

### 1. Git Clone Infrastructure (git_fetcher.rs)

**What exists:** `clone_or_pull(repo_url, dest, branch, cancel)` in `src-tauri/src/core/git_fetcher.rs`. [VERIFIED: codebase]

Key characteristics:

- **Already does shallow clones.** The CLI path uses `--depth 1 --filter=blob:none --no-tags` for fresh clones (line 296-298). This is exactly what we need. [VERIFIED: codebase]
- **Supports CancelToken.** The `cancel: Option<&CancelToken>` parameter threads through `run_cmd_with_timeout` for user cancellation. [VERIFIED: codebase]
- **Pull path does full fetch.** When dest exists, it does `git fetch --prune origin` + `git reset --hard FETCH_HEAD`. This is the staleness refresh mechanism -- calling `clone_or_pull` on an existing clone effectively does a freshness check. [VERIFIED: codebase]
- **dest can be any path.** It creates parent dirs automatically (line 219-222). No coupling to central repo or Tauri app cache dirs. [VERIFIED: codebase]
- **Returns HEAD sha string.** Can be used for cache metadata if needed. [VERIFIED: codebase]

**Reuse plan:** Call `clone_or_pull` directly from the new `clone_explore_skill` command, targeting the explore-cache directory. No wrapper needed -- the function is generic.

### 2. Installer's Cache Pattern (installer.rs)

**What exists:** `clone_to_cache()` in `src-tauri/src/core/installer.rs` (line 1543) implements a TTL-based git cache in `app_cache_dir/skills-hub-git-cache/`. [VERIFIED: codebase]

Key characteristics:

- Uses `sha256(url + branch)` as the cache key directory name (line 1624-1633)
- Writes `.skills-hub-cache.json` with `last_fetched_ms` and `head` fields
- Uses a `Mutex` to prevent concurrent clones to the same cache entry
- TTL is configurable via SQLite settings (`git_cache_ttl_secs`, default 60s)

**For explore-cache:** We can use a simpler scheme since the CONTEXT.md specifies wiping on startup rather than TTL-based staleness. However, we should still use a mutex and the same cache-key pattern for robustness. The cache dir will be `~/.skillshub/.explore-cache/<hash>/` per CONTEXT.md decision.

**Note on CONTEXT.md wording:** CONTEXT.md says `~/.skillshub/.explore-cache/<owner>/<repo>/` but using `<hash>` (like the installer does) is safer -- avoids path-unsafe characters in repo names. This falls under Claude's discretion for "exact directory structure." Recommend using `sha256(source_url)` as the dir name for consistency with installer pattern.

### 3. Startup Cleanup (lib.rs)

**What exists:** Startup cleanup in `src-tauri/src/lib.rs` (lines 52-73) runs two cleanup tasks in a spawned async block: [VERIFIED: codebase]

1. `temp_cleanup::cleanup_old_git_temp_dirs` -- removes temp dirs older than 24h
2. `cache_cleanup::cleanup_git_cache_dirs` -- removes git cache dirs older than N days

**For explore-cache:** The simplest approach is to add a third cleanup call in the same async block that does `std::fs::remove_dir_all(central_repo_path.join(".explore-cache"))`. This is simpler than the existing cleanup functions because it wipes the entire directory unconditionally (no age check needed -- per CONTEXT.md, the cache is session-scoped and wiped on next launch).

No new module needed -- just 5-6 lines in the existing startup block in `lib.rs`.

### 4. SkillDetailView Data Requirements

**What `SkillDetailView` actually uses from `ManagedSkill`:** [VERIFIED: codebase]

| Field          | Usage                                                        | Required for Explore?             |
| -------------- | ------------------------------------------------------------ | --------------------------------- |
| `central_path` | Passed to `list_skill_files` and `read_skill_file` IPC calls | YES -- set to cache path          |
| `name`         | Displayed in header                                          | YES -- from card data             |
| `description`  | Displayed in header (nullable)                               | Optional -- set from card summary |
| `source_type`  | Determines icon (git vs folder)                              | YES -- set to `'github'`          |
| `source_ref`   | Displayed as source label                                    | YES -- set to GitHub URL          |
| `updated_at`   | Displayed via `formatRelative()`                             | Provide `Date.now()` as fallback  |
| `id`           | Not used in rendering                                        | Provide empty string              |
| `created_at`   | Not used in rendering                                        | Provide 0                         |
| `last_sync_at` | Not used in rendering                                        | Provide null                      |
| `status`       | Not used in rendering                                        | Provide empty string              |
| `targets`      | Not used in rendering                                        | Provide empty array               |

**Key finding:** `list_skill_files` and `read_skill_file` commands accept raw `centralPath: String` and are NOT gated behind any skill-ID lookup or auth check. They simply read files from the given path. Any valid directory path works. [VERIFIED: codebase, commands/mod.rs lines 1055-1083]

### 5. ExplorePage Card Data Available

**FeaturedSkillDto fields:** `slug, name, summary, downloads, stars, source_url` [VERIFIED: codebase]
**OnlineSkillDto fields:** `name, installs, source, source_url` [VERIFIED: codebase]

Both types have `source_url` (GitHub URL) and `name`. This is sufficient to:

1. Pass `source_url` to the backend for cloning
2. Construct the synthetic `ManagedSkill` for `SkillDetailView`

### 6. App.tsx View Routing

**Current view states:** `"myskills" | "explore" | "detail" | "settings" | "projects"` [VERIFIED: codebase]

**Detail view wiring:**

- `detailSkill: ManagedSkill | null` state variable (line 118)
- `handleOpenDetail(skill)` sets `detailSkill` and `activeView = "detail"` (line 753-756)
- `handleBackToList()` clears `detailSkill` and sets `activeView = "myskills"` (line 758-761)
- Rendering: `activeView === "detail" && detailSkill ? <SkillDetailView ... />` (line 2009)

**For explore-detail:**

- Add `"explore-detail"` to the view state union type
- Add a new handler like `handleOpenExploreDetail(skill: ManagedSkill)` that sets `detailSkill` and `activeView = "explore-detail"`
- Add `handleBackToExplore()` that clears `detailSkill` and sets `activeView = "explore"`
- In the rendering conditional: add `activeView === "explore-detail"` alongside `"detail"` for the SkillDetailView render, but with `onBack={handleBackToExplore}`

### 7. Multi-Skill Repo Consideration

**Important:** Some repos contain multiple skills in subdirectories. The `source_url` from explore cards may point to a repo that has skills in subdirectories (e.g., `https://github.com/anthropics/claude-code/tree/main/.agent/skills/some-skill`). [VERIFIED: codebase, installer.rs has multi-skill handling]

The `source_url` field in `FeaturedSkillDto` may include `/tree/<branch>/<path>` for skills within a subdirectory. The clone command should clone the repo root, but the `central_path` passed to `SkillDetailView` should point to the specific skill subdirectory within the clone.

This means the backend command needs to:

1. Parse the GitHub URL to extract repo URL, branch, and subpath
2. Clone the full repo (to the cache dir)
3. Return `cache_dir + subpath` as the path for the frontend

The existing `installer.rs` already has URL parsing logic for this (extracting branch and subpath from GitHub URLs). Can reuse or reference that pattern.

## Don't Hand-Roll

| Problem              | Don't Build                    | Use Instead                                           | Why                                                           |
| -------------------- | ------------------------------ | ----------------------------------------------------- | ------------------------------------------------------------- |
| Git cloning          | Custom HTTP/tar download       | `git_fetcher::clone_or_pull`                          | Already handles shallow clone, cancellation, timeout, retries |
| File listing/reading | Custom file walker for explore | Existing `list_skill_files` / `read_skill_file` IPC   | Accept any path, already filter `.git`, handle encoding       |
| Cache dir hashing    | Manual string sanitization     | `sha256(url)` like `repo_cache_key()` in installer.rs | Safe for all URL characters, collision-free                   |

## Common Pitfalls

### Pitfall 1: Multi-Skill Repos Need Subpath Handling

**What goes wrong:** Cloning a repo and pointing SkillDetailView at the repo root shows ALL files, not just the specific skill's files.
**Why it happens:** Many skill repos have multiple skills in subdirectories; the `source_url` includes `/tree/main/path/to/skill`.
**How to avoid:** Parse the URL to extract the subpath, clone the full repo, but return `cache_path + subpath` as the viewer path.
**Warning signs:** SkillDetailView shows unexpected files like other skills, CI configs, etc.

### Pitfall 2: Concurrent Clone Requests

**What goes wrong:** User clicks "View" on the same skill twice rapidly, causing two concurrent clone operations to the same directory.
**Why it happens:** Frontend doesn't disable the button during clone, or user navigates away and back quickly.
**How to avoid:** Use a Mutex in the backend (like `GIT_CACHE_LOCK` in installer.rs) and disable the View button while loading on the frontend.

### Pitfall 3: Loading State UX

**What goes wrong:** User clicks "View" and nothing happens for 5-10 seconds while the repo clones.
**Why it happens:** Network latency for git clone with no visual feedback.
**How to avoid:** Show a loading spinner immediately. Use the existing `LoadingOverlay` component or a local spinner in the explore card. The `CancelToken` is already available for cancellation.

### Pitfall 4: Stale activeView State After Install

**What goes wrong:** User views an explore skill, clicks "Install" from the detail view, and the back button or state gets confused.
**How to avoid:** After install from explore-detail, either navigate to myskills-detail (now it's installed) or back to explore with a success toast.

## Code Examples

### Synthetic ManagedSkill Construction (Frontend)

```typescript
// Construct a ManagedSkill-compatible object for SkillDetailView
const exploreManagedSkill: ManagedSkill = {
  id: "",
  name: skillName, // from FeaturedSkillDto.name or OnlineSkillDto.name
  description: summary, // from FeaturedSkillDto.summary or null
  source_type: "github",
  source_ref: sourceUrl, // from FeaturedSkillDto.source_url
  central_path: cachePath, // returned by clone_explore_skill command
  created_at: 0,
  updated_at: Date.now(),
  last_sync_at: null,
  status: "",
  targets: [],
};
```

### Backend Command Signature (Rust)

```rust
// New command in commands/mod.rs
#[tauri::command]
pub async fn clone_explore_skill(
    source_url: String,
    app: tauri::AppHandle,
    cancel: State<'_, Arc<CancelToken>>,
) -> Result<String, String> {
    // Returns the filesystem path to the cloned skill directory
    // (including subpath if source_url contains /tree/branch/path)
}
```

### Startup Cleanup Addition (lib.rs)

```rust
// Add after existing cleanup tasks in the setup block
// Wipe explore-cache on startup (session-scoped cache)
if let Ok(central) = core::central_repo::resolve_central_repo_path(&handle, &store_for_cleanup) {
    let explore_cache = central.join(".explore-cache");
    if explore_cache.exists() {
        match std::fs::remove_dir_all(&explore_cache) {
            Ok(()) => log::info!("cleaned up explore-cache"),
            Err(e) => log::warn!("failed to clean explore-cache: {}", e),
        }
    }
}
```

## Assumptions Log

| #   | Claim                                                            | Section                  | Risk if Wrong                                                                                                                               |
| --- | ---------------------------------------------------------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------- |
| A1  | Using sha256 hash for cache dir name instead of `<owner>/<repo>` | Architecture Patterns #2 | Low -- CONTEXT.md says `<owner>/<repo>` but also says "exact structure is Claude's discretion"; hash is safer and consistent with installer |

## Open Questions

1. **Install from explore-detail navigation**
   - What we know: User wants an "Install" button in SkillDetailView when viewing an explore skill
   - What's unclear: After clicking Install, should the view switch to myskills-detail (now viewing the installed copy) or return to explore with a toast?
   - Recommendation: Return to explore with a success toast -- simpler, avoids needing to reload the skill from the DB

2. **SkillDetailView prop extension vs separate wrapper**
   - What we know: Need `isExplorePreview` prop to show Install button and adjust back nav
   - What's unclear: Should we add the prop to SkillDetailView directly or wrap it in an ExploreDetailView?
   - Recommendation: Add optional `isExplorePreview?: boolean` and `onInstall?: () => void` props directly to SkillDetailView -- smaller change, the component already handles all file viewing

## Sources

### Primary (HIGH confidence)

- `src-tauri/src/core/git_fetcher.rs` - clone_or_pull function, shallow clone flags, CancelToken support
- `src-tauri/src/core/installer.rs` - clone_to_cache pattern, cache key hashing, mutex usage
- `src-tauri/src/core/cache_cleanup.rs` - startup cleanup pattern
- `src-tauri/src/core/temp_cleanup.rs` - startup cleanup pattern
- `src-tauri/src/lib.rs` - startup cleanup wiring, command registration
- `src-tauri/src/commands/mod.rs` - list_skill_files/read_skill_file accept raw path strings
- `src/components/skills/SkillDetailView.tsx` - ManagedSkill field usage
- `src/components/skills/ExplorePage.tsx` - card structure, available data
- `src/components/skills/types.ts` - FeaturedSkillDto, OnlineSkillDto, ManagedSkill shapes
- `src/App.tsx` - view state management, detail view wiring

## Metadata

**Confidence breakdown:**

- Git clone reuse: HIGH - code verified, function is generic
- Startup cleanup: HIGH - pattern verified, 5-line addition
- SkillDetailView reuse: HIGH - field usage verified, all path-based
- View routing: HIGH - pattern verified, straightforward extension

**Research date:** 2026-04-22
**Valid until:** 2026-05-22
