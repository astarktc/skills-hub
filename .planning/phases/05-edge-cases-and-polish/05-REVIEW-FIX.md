---
phase: 05-edge-cases-and-polish
fixed_at: 2026-04-09T01:43:23Z
review_path: .planning/phases/05-edge-cases-and-polish/05-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 5: Code Review Fix Report

**Fixed at:** 2026-04-09T01:43:23Z
**Source review:** .planning/phases/05-edge-cases-and-polish/05-REVIEW.md
**Iteration:** 1

**Summary:**

- Findings in scope: 5
- Fixed: 5
- Skipped: 0

## Fixed Issues

### WR-01: Header `onViewChange` type excludes "projects"

**Files modified:** `src/components/skills/Header.tsx`
**Commit:** dfa7418
**Applied fix:** Updated `onViewChange` prop type from `(view: "myskills" | "explore") => void` to `(view: "myskills" | "explore" | "projects") => void`. Added `FolderKanban` icon import and a Projects tab button in the nav-tabs section, consistent with the existing My Skills and Explore tab pattern. Users can now navigate to the Projects view directly from the header.

### WR-02: Stale closure over `assignments` in `toggleAssignment`

**Files modified:** `src/components/projects/useProjectState.ts`
**Commit:** 1fa49dc
**Applied fix:** Added `assignmentsRef` (a `useRef`) that tracks the latest `assignments` state value on every render. Updated `toggleAssignment` to read `assignmentsRef.current` instead of the closure-captured `assignments` state. Removed `assignments` from the `useCallback` dependency array since the ref always provides the current value, eliminating the stale closure risk when rapid toggles fire.

### WR-03: `remove_block` closure in gitignore update does not handle multiple Skills Hub blocks

**Files modified:** `src-tauri/src/commands/projects.rs`
**Commit:** 6a84452
**Applied fix:** Rewrote the `remove_block` closure to use a line-by-line state machine instead of single-pass index tracking. The new implementation iterates all lines, entering an `in_block` state when it encounters the marker comment, and skipping marker lines plus all following pattern lines (starting with `/`). It also removes the preceding blank line and handles trailing blanks within blocks. This naturally handles multiple Skills Hub blocks if present (e.g., from a double-write bug), removing all of them in a single pass.

### WR-04: `println!` used for debug output instead of `log` crate

**Files modified:** `src-tauri/src/commands/mod.rs`
**Commit:** 3be3d7e
**Applied fix:** Replaced `println!("[delete_managed_skill] skillId={}", skillId)` with `log::debug!(...)` at line 836, and replaced `eprintln!("rename failed, fallback used: {}", err)` with `log::warn!(...)` at line 328. Both now route through the `log` crate which is captured by `tauri-plugin-log` and visible in production builds, following the project convention.

### WR-05: Missing Chinese translations for Projects section

**Files modified:** `src/i18n/resources.ts`
**Commit:** dcc85da
**Applied fix:** Added `navProjects: "项目"` to the zh translation block (needed for the Projects tab added in WR-01). Added a comment `// projects: deferred to future phase (English fallback active via i18next)` at the end of the zh block to explicitly track that the full projects translation block is intentionally deferred per the project constraint ("i18n: English strings only -- Chinese deferred"). This prevents silent regression and documents the decision.

---

_Fixed: 2026-04-09T01:43:23Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
