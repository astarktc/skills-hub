# Quick Task 260416-hw6: Fix Multi-Skill Repo Install Bug - Research

**Researched:** 2026-04-16
**Domain:** Rust installer + TypeScript install flow (skill discovery & selection)
**Confidence:** HIGH

## Summary

The bug has two interrelated root causes, both in how `list_git_skills` discovers skill candidates in repos with non-standard directory structures like `better-auth/skills`.

**Root Cause 1 (macOS/Windows -- critical):** On case-insensitive filesystems, the root-level scan in `list_git_skills` (installer.rs:1046) finds `security/SKILL.MD` via case-insensitive match on `p.join("SKILL.md").exists()`. This sets `priority_count = 1`, which prevents the recursive fallback from running. Result: only 1 candidate returned. The frontend's `candidates.length === 1` branch (App.tsx:1274) then installs that single candidate directly, completely ignoring `autoSelectSkillName`. Every skill from this repo installs as "better-auth-security-best-practices".

**Root Cause 2 (Linux -- partial):** On case-sensitive filesystems, `security/SKILL.MD` is never found (case mismatch), so `priority_count = 0` and the recursive fallback runs, finding the 5 skills in `better-auth/*/`. The `autoSelectSkillName` matching then works correctly for those 5. However, the 6th skill ("better-auth-security-best-practices" in `security/SKILL.MD`) can never be installed on Linux.

**Primary recommendation:** Fix `list_git_skills` to always perform recursive scanning when the repo contains nested skill directories, and make SKILL.md detection case-insensitive. Also fix the `candidates.length === 1` branch to respect `autoSelectSkillName`.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

- SKILL.md frontmatter is the canonical source of truth for skill names
- Folder name is only a fallback when SKILL.md is missing or has no valid frontmatter
- Always use the candidate-based flow (clone repo, scan for skills, match by name, install specific candidate)
- When user clicks "Install" on a specific skill from Explore page, the skills.sh name should be used to auto-select the correct candidate from the scan results
- Each candidate install reads its own SKILL.md from its own subdirectory

### Claude's Discretion

- Implementation details of how the candidate matching logic works (exact match vs containment)
- Whether to remove the isFolderUrl shortcut entirely or just ensure it doesn't apply to multi-skill repos

### Deferred Ideas (OUT OF SCOPE)

- None specified
  </user_constraints>

## Bug Trace: Complete Code Path

### Step 1: Explore Page Click

User clicks "Install" on "create-auth-skill" from skills.sh search results.

**ExplorePage.tsx:189:**

```typescript
onClick={() => onInstallSkill(skill.source_url, skill.name)}
// source_url = "https://github.com/better-auth/skills"  (bare repo URL, no /tree/)
// skill.name = "create-auth-skill"
```

[VERIFIED: source code at ExplorePage.tsx:189 and skills.sh API response]

### Step 2: handleExploreInstall

**App.tsx:1446-1461:**

```typescript
const handleExploreInstall = useCallback(
  (sourceUrl: string, skillName?: string) => {
    setGitUrl(sourceUrl); // "https://github.com/better-auth/skills"
    if (skillName) setAutoSelectSkillName(skillName); // "create-auth-skill"
    // ... sets sync targets, triggers handleCreateGit via effect
  },
  [toolStatus],
);
```

### Step 3: handleCreateGit -- isFolderUrl Check

**App.tsx:1217:**

```typescript
const isFolderUrl = url.includes("/tree/") || url.includes("/blob/");
// "https://github.com/better-auth/skills" -> false (no /tree/ or /blob/)
```

Result: `isFolderUrl = false`. Code falls through to the candidate-based path (line 1267). This is correct behavior -- the issue is downstream. [VERIFIED: source code]

### Step 4: list_git_skills_cmd scans the repo

**App.tsx:1267-1270:**

```typescript
const candidates = await invokeTauri<GitSkillCandidate[]>(
  "list_git_skills_cmd",
  { repoUrl: url },
);
```

This calls `list_git_skills` in installer.rs.

**installer.rs:981-1132 -- list_git_skills scan order:**

1. **Folder URL single-candidate** (line 998): `parsed.subpath` is `None` -- skipped.
2. **Root SKILL.md** (line 1012): `repo_root/SKILL.md` doesn't exist -- skipped.
3. **SKILL_SCAN_BASES** (line 1024): Checks `skills/`, `skills/.curated/`, etc. None exist in this repo.
4. **Root-level subdirs** (line 1035-1061): Checks immediate children of repo root:
   - `better-auth/`: `better-auth/SKILL.md` does NOT exist -- skipped
   - `security/`: `security/SKILL.md` -- on **macOS/Windows** (case-insensitive) matches `SKILL.MD` -> **FOUND**. On **Linux** -> NOT found.
5. **SKILL_SCAN_BASES iteration** (line 1063-1083): Scans dirs found in step 3 -- none exist.
6. **priority_count check** (line 1086-1089):
   - **macOS/Windows**: `priority_count = 1` (the security skill). **Recursive fallback is SKIPPED.**
   - **Linux**: `priority_count = 0`. Falls through to recursive.
7. **Marketplace fallback** (line 1090-1104): Parses `.claude-plugin/marketplace.json`. Plugin source is `"./""` (repo root). Scans `repo_root/skills/*/SKILL.md` (doesn't exist) and direct children (same as root-level -- nothing new).
8. **Recursive fallback** (line 1107-1126): `find_skill_dirs_recursive` with max_depth=5.
   - **Linux**: Finds 5 skills in `better-auth/*/SKILL.md` (depth 2). Returns 5 candidates.
   - **macOS**: NEVER REACHED (priority_count was 1).

[VERIFIED: source code analysis + repo structure from GitHub API]

### Step 5: Frontend Candidate Handling

**macOS/Windows (1 candidate):**

**App.tsx:1274-1330:** `candidates.length === 1` branch executes. Installs `security/` skill directly as "better-auth-security-best-practices". **`autoSelectSkillName` is completely ignored.**

```typescript
if (candidates.length === 1) {
    // Installs candidates[0] directly -- NEVER checks autoSelectSkillName
    const created = await invokeTauri<InstallResultDto>(
        "install_git_selection",
        { repoUrl: url, subpath: candidates[0].subpath, ... }
    );
}
```

**Linux (5 candidates):**

**App.tsx:1331-1414:** `autoSelectSkillName` matching runs.

```typescript
} else if (autoSelectSkillName) {
    const target = autoSelectSkillName.toLowerCase(); // "create-auth-skill"
    // Exact match works: finds candidate with name "create-auth-skill"
    const match = candidates.find((c) => c.name.toLowerCase() === target) ?? ...;
}
```

This works correctly for the 5 found candidates. But `security/SKILL.MD` skill is never found.

[VERIFIED: source code analysis]

## Repo Structure: better-auth/skills

Verified via GitHub API:

```
better-auth/skills/
  .claude-plugin/
    marketplace.json          # source: "./"
  better-auth/
    .claude-plugin/plugin.json
    best-practices/SKILL.md   # name: "better-auth-best-practices"
    create-auth/SKILL.md      # name: "create-auth-skill"
    emailAndPassword/SKILL.md # name: "email-and-password-best-practices"
    organization/SKILL.md     # name: "organization-best-practices"
    twoFactor/SKILL.md        # name: "two-factor-authentication-best-practices"
  security/
    SKILL.MD                  # name: "better-auth-security-best-practices" (UPPERCASE .MD)
```

[VERIFIED: GitHub API `git/trees/main?recursive=1` and raw SKILL.md contents]

## skills.sh API Response

For query "better-auth", all 6 skills from this repo return `source: "better-auth/skills"`, which becomes `source_url: "https://github.com/better-auth/skills"` -- a bare repo URL with NO subpath. [VERIFIED: skills.sh API response]

## Identified Bugs (3 total)

### Bug 1: `candidates.length === 1` ignores autoSelectSkillName (CRITICAL)

**Location:** App.tsx:1274
**Impact:** When scan returns exactly 1 candidate (e.g., macOS finding only `security/SKILL.MD`), the code installs it without checking if it matches the user's intended skill.
**Fix:** Check `autoSelectSkillName` before auto-installing. If the single candidate doesn't match, show an error or fall back to the picker modal.

### Bug 2: Root-level scan prevents recursive fallback (CRITICAL)

**Location:** installer.rs:1086-1089
**Impact:** On case-insensitive filesystems, finding even 1 root-level skill causes `priority_count > 0`, which skips the recursive fallback that would find the other 5 skills nested at depth 2.
**Fix:** The scan should always include recursive results for repos with nested skill directories. Either:

- (a) Always run recursive scan regardless of priority_count, OR
- (b) After priority scan, also scan child directories of root-level dirs that look like category folders (dirs without SKILL.md but with subdirs that have SKILL.md).

### Bug 3: Case-sensitive SKILL.md detection misses SKILL.MD (MODERATE)

**Location:** installer.rs:1046, installer.rs:482, and other `SKILL.md` checks
**Impact:** On Linux, `security/SKILL.MD` is never found. The skill cannot be installed from this repo on case-sensitive filesystems.
**Fix:** Make SKILL.md detection case-insensitive. Instead of `p.join("SKILL.md").exists()`, scan directory entries and match case-insensitively, e.g., check for `SKILL.md`, `SKILL.MD`, `skill.md`.

## Architecture Patterns

### Recommended Fix Approach

**Backend (installer.rs):**

1. Make `list_git_skills` always run the recursive scan, not gated behind `priority_count == 0`. The recursive scan already handles deduplication via `dedup_by(|a, b| a.subpath == b.subpath)` (line 1129).

2. Add a case-insensitive SKILL.md finder. Replace bare `p.join("SKILL.md").exists()` with a helper like:

```rust
fn has_skill_md(dir: &Path) -> bool {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.eq_ignore_ascii_case("skill.md") && entry.path().is_file() {
                return true;
            }
        }
    }
    false
}
```

And a companion to get the actual path:

```rust
fn find_skill_md(dir: &Path) -> Option<PathBuf> {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.eq_ignore_ascii_case("skill.md") && entry.path().is_file() {
                return Some(entry.path());
            }
        }
    }
    None
}
```

3. Update `find_skill_dirs_recursive`, `is_skill_dir`, root-level scan loops, and `extract_skill_info` to use the case-insensitive helpers.

**Frontend (App.tsx):**

4. Fix the `candidates.length === 1` branch to check `autoSelectSkillName`:

```typescript
if (candidates.length === 1) {
  // When autoSelectSkillName is set, verify the single candidate matches
  if (autoSelectSkillName) {
    const target = autoSelectSkillName.toLowerCase();
    const name = candidates[0].name.toLowerCase();
    if (name !== target && !name.includes(target) && !target.includes(name)) {
      // Single candidate doesn't match -- show error
      setAutoSelectSkillName(null);
      setError(t("errors.skillNotFoundInRepo", { name: autoSelectSkillName }));
      return;
    }
    setAutoSelectSkillName(null);
  }
  // ... proceed with install
}
```

### Anti-Patterns to Avoid

- **Relying on priority_count to gate recursive scan:** The assumption that finding any root-level skill means all skills are at root level is wrong for repos with mixed nesting.
- **Hard-coding SKILL.md case:** Real repos use both `SKILL.md` and `SKILL.MD`. The detection must be case-insensitive.

## Don't Hand-Roll

| Problem                        | Don't Build                       | Use Instead                           | Why                                     |
| ------------------------------ | --------------------------------- | ------------------------------------- | --------------------------------------- |
| Case-insensitive file matching | Platform-specific `#[cfg]` blocks | `eq_ignore_ascii_case` on dir entries | Works consistently across all platforms |

## Common Pitfalls

### Pitfall 1: Breaking the single-candidate fast path

**What goes wrong:** Over-complicating the `candidates.length === 1` branch could break the normal case where a user manually enters a repo URL with exactly one skill.
**How to avoid:** Only add the `autoSelectSkillName` mismatch check. When `autoSelectSkillName` is null (manual URL entry), the existing behavior is correct.

### Pitfall 2: Dedup issues with mixed scan paths

**What goes wrong:** Running both priority scan and recursive scan could produce duplicate candidates with the same subpath.
**How to avoid:** The existing `dedup_by(|a, b| a.subpath == b.subpath)` on line 1129 already handles this. Verify it still works after changes.

### Pitfall 3: Performance regression from always-recursive scan

**What goes wrong:** Large repos with many files could be slow to scan recursively.
**How to avoid:** The recursive scan already has `max_depth=5` and `SKIP_DIRS` exclusions. The git cache also means the repo is local. Performance impact is negligible.

## Code Examples

### Affected Functions (files to modify)

**installer.rs:**

- `list_git_skills` (line 981) -- main scan function, needs recursive always + case-insensitive
- `find_skill_dirs_recursive` (line 477) -- needs case-insensitive SKILL.md check
- `count_skills_in_repo` (line 710) -- same case-insensitive fix
- `is_skill_dir` (line 590) -- needs case-insensitive check
- `extract_skill_info` (line 621) -- needs case-insensitive SKILL.md path
- `parse_skill_md` (line 1587) -- caller passes path, callers need to find actual filename
- `scan_skill_candidates_in_dir` (line 640) -- same pattern as list_git_skills
- `install_git_skill` (line 107) -- the non-selection path, reads `central_path.join("SKILL.md")` which should be fine since it copies the file as-is

**App.tsx:**

- `handleCreateGit` (line 1206) -- fix `candidates.length === 1` branch

### Count of locations using hardcoded "SKILL.md"

All locations in installer.rs that check for SKILL.md existence or join SKILL.md:

- Line 482: `dir.join("SKILL.md").exists()`
- Line 591: `p.join("SKILL.md").exists()`
- Line 622-623: `skill_dir.join("SKILL.md")` and `skill_md.exists()`
- Line 640-678: Multiple `SKILL.md` references in `scan_skill_candidates_in_dir`
- Line 710-755: Multiple in `count_skills_in_repo`
- Line 997-1000: In `list_git_skills` folder URL path
- Line 1012: Root SKILL.md check
- Line 1046: Root-level subdir check
- Line 1067: via `is_skill_dir`
- Line 1186-1191: In `list_local_skills`

[VERIFIED: grep of source code]

## Assumptions Log

| #   | Claim                                                                    | Section   | Risk if Wrong                                                                            |
| --- | ------------------------------------------------------------------------ | --------- | ---------------------------------------------------------------------------------------- |
| A1  | The primary user experiencing this bug is on macOS (case-insensitive FS) | Bug Trace | If on Linux, only Bug 3 applies and the candidate matching should work for 5 of 6 skills |

All other claims were verified against source code and GitHub API.

## Open Questions

1. **What percentage of skill repos use SKILL.MD vs SKILL.md?**
   - What we know: better-auth/skills uses both (SKILL.md in most, SKILL.MD in security/)
   - What's unclear: How common uppercase is across the ecosystem
   - Recommendation: Fix case-insensitivity regardless -- it's a correctness issue

## Sources

### Primary (HIGH confidence)

- Source code: `src-tauri/src/core/installer.rs` -- full analysis of scan logic
- Source code: `src/App.tsx` -- full analysis of handleCreateGit flow
- Source code: `src-tauri/src/core/skills_search.rs` -- source_url construction
- Source code: `src/components/skills/ExplorePage.tsx` -- onInstallSkill call patterns
- GitHub API: `repos/better-auth/skills/git/trees/main?recursive=1` -- verified repo structure
- GitHub raw content: all 6 SKILL.md files in better-auth/skills -- verified frontmatter names
- skills.sh API: `api/search?q=better-auth` -- verified response format and source_url values

## Metadata

**Confidence breakdown:**

- Bug identification: HIGH -- traced exact code paths against actual repo structure and API responses
- Fix approach: HIGH -- minimal changes to well-understood code
- Case-sensitivity issue: HIGH -- verified behavior difference between case-sensitive and case-insensitive filesystems

**Research date:** 2026-04-16
**Valid until:** Until installer.rs scan logic changes
