---
phase: 260416-hw6-fix-multi-skill-repo-install-bug-where-a
reviewed: 2026-04-16T18:37:15Z
depth: quick
files_reviewed: 3
files_reviewed_list:
  - src-tauri/src/core/installer.rs
  - src/App.tsx
  - src/i18n/resources.ts
findings:
  critical: 0
  warning: 2
  info: 0
  total: 2
status: issues_found
---

# Phase 260416-hw6: Code Review Report

**Reviewed:** 2026-04-16T18:37:15Z
**Depth:** quick
**Files Reviewed:** 3
**Status:** issues_found

## Summary

Reviewed the changed code in the Git/local skill candidate flow, with extra attention on the new case-insensitive `SKILL.md` helpers, the candidate auto-selection logic, and the regression risk from routing all Git URLs through candidate discovery.

The `SKILL.md` casing change looks sound, and I did not find any security issues in scope. I did find two logic regressions in the new Git candidate flow: one affects folder URLs that point at a container directory rather than a skill directory, and the other can still auto-install the wrong skill when the requested name only loosely overlaps a single discovered candidate.

## Warnings

### WR-01: Folder URLs to skill containers now return no candidates

**File:** `/home/alexwsl/skills-hub/src-tauri/src/core/installer.rs:1042-1053`
**Issue:** `list_git_skills()` now returns immediately whenever the parsed GitHub URL includes a `subpath`. That works if the folder URL points directly at one skill directory, but it fails when the URL points at a container such as `skills/`, `.claude/skills/`, or another subfolder that contains multiple skills. In those cases `dir.is_dir()` is true but `has_skill_md(&dir)` is false, so the function returns `Ok([])` instead of scanning inside the selected subdirectory. Because `App.tsx` removed the old direct-install shortcut and sends all URLs through this flow, this is now a user-visible regression for valid folder URLs.
**Fix:** If `parsed.subpath` is present and the target directory is not itself a skill, scan within that directory instead of returning immediately. For example:

```rust
if let Some(subpath) = &parsed.subpath {
    let dir = repo_dir.join(subpath);
    if dir.is_dir() && (has_skill_md(&dir) || is_claude_skill_dir(&dir)) {
        let (name, desc) = extract_skill_info(&dir, &repo_dir);
        out.push(GitSkillCandidate {
            name,
            description: desc,
            subpath: subpath.to_string(),
        });
        return Ok(out);
    }

    if dir.is_dir() {
        return Ok(scan_skill_candidates_under_dir(&repo_dir, &dir, subpath));
    }

    return Ok(vec![]);
}
```

### WR-02: Single-candidate auto-match can still install the wrong skill

**File:** `/home/alexwsl/skills-hub/src/App.tsx:1234-1245`
**Issue:** The new guard for `autoSelectSkillName` accepts a single candidate when either name merely contains the other. That prevents some false negatives, but it still allows false positives for generic names. Example: requested online skill `react-form` and a repo scan that only finds `react` will pass the `includes()` check and install `react` silently. This is exactly the failure mode the new guard is trying to prevent, just with a narrower set of mismatches.
**Fix:** Tighten matching for the single-candidate path. Prefer exact match on a normalized slug, and only fall back to containment when the match is unambiguous by token boundary rather than raw substring. For example:

```ts
const normalize = (value: string) =>
  value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");

const target = normalize(autoSelectSkillName);
const candidateName = normalize(candidates[0].name);

if (candidateName !== target) {
  setAutoSelectSkillName(null);
  throw new Error(
    t("errors.skillNotFoundInRepo", { name: autoSelectSkillName }),
  );
}
```

---

_Reviewed: 2026-04-16T18:37:15Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: quick_
