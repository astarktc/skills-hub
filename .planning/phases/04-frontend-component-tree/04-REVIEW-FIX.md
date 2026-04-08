---
phase: 04-frontend-component-tree
fixed_at: 2026-04-08T21:45:00Z
review_path: .planning/phases/04-frontend-component-tree/04-REVIEW.md
iteration: 1
findings_in_scope: 6
fixed: 6
skipped: 0
status: all_fixed
---

# Phase 04: Code Review Fix Report

**Fixed at:** 2026-04-08T21:45:00Z
**Source review:** .planning/phases/04-frontend-component-tree/04-REVIEW.md
**Iteration:** 1

**Summary:**

- Findings in scope: 6
- Fixed: 6
- Skipped: 0

## Fixed Issues

### CR-01: Gitignore remove_block closure can corrupt file content

**Files modified:** `src-tauri/src/commands/projects.rs`
**Commit:** 61ce464
**Applied fix:** Replaced blank-line heuristic end detection with pattern-based detection. The block now continues only while lines start with `/` (our generated gitignore patterns) and stops at the first non-pattern line. This prevents draining unrelated content that follows the Skills Hub block without a blank separator. Also handles the case where the block runs to EOF correctly.

### WR-01: toggleAssignment reads stale assignments from closure

**Files modified:** `src/components/projects/useProjectState.ts`
**Commit:** 7f03b08
**Applied fix:** Added an early-return guard at the top of `toggleAssignment` that checks `pendingCells.has(key)` before proceeding. This prevents double-toggle when two rapid checkbox clicks occur before the first operation completes. Added `pendingCells` to the `useCallback` dependency array.

### WR-02: ToolConfigModal loses selection state when re-opened for existing project

**Files modified:** `src/components/projects/ToolConfigModal.tsx`
**Commit:** b237c40
**Applied fix:** Changed `buildInitialSelection` to check `currentTools.length > 0` first. When the project already has tools configured, only those tools are used as the baseline selection. The "pre-select all installed tools" behavior is now only applied for new projects with no tools yet.

### WR-03: AddProjectModal resets state after handleSubmit even if onRegister throws

**Files modified:** `src/components/projects/AddProjectModal.tsx`
**Commit:** 5751977
**Applied fix:** Wrapped the `onRegister` call in try/catch. State reset (`setPath("")`, `setAddToGitignore(false)`, `setAddToExclude(false)`) now only executes on success. On error, the parent's error toast still fires but the modal preserves the user's input.

### WR-04: AssignmentMatrix memo() ineffective due to pendingCells Set reference

**Files modified:** `src/components/projects/AssignmentMatrix.tsx`
**Commit:** 85f9b50
**Applied fix:** Added a `setsEqual` utility function that compares two `Set<string>` instances by content. Applied custom `areEqual` comparators to both `memo(MatrixRow)` and `memo(AssignmentMatrix)` that use reference equality for all props except `pendingCells`, which uses content-based comparison. This prevents unnecessary re-renders of the entire matrix when only specific pending cells change.

### WR-05: EditProjectModal directly imports invoke instead of going through invokeTauri

**Files modified:** `src/components/projects/EditProjectModal.tsx`, `src/components/projects/ProjectsPage.tsx`, `src/components/projects/useProjectState.ts`
**Commit:** 015ee06
**Applied fix:** Added documenting comments to all three files in the projects subtree that use direct `invoke` import, explaining the convention: the projects subtree always runs inside Tauri context, and the `invokeTauri()` pattern from App.tsx is a hook-local callback not importable elsewhere. This follows the review's alternative suggestion of accepting the direct import with documentation.

---

_Fixed: 2026-04-08T21:45:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
