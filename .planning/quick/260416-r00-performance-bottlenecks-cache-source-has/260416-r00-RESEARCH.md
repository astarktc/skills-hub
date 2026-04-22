# Performance Bottlenecks: Cache Source Hashes & Precompute Assignment Lookup - Research

**Researched:** 2026-04-16
**Domain:** Backend hash caching (Rust/SQLite), Frontend rendering optimization (React)
**Confidence:** HIGH

## Summary

Two independent performance bottlenecks exist in the project assignment matrix flow:

**Backend:** `list_assignments_with_staleness()` in `project_sync.rs` calls `content_hash::hash_dir(source)` for every copy-mode assignment on every load. `hash_dir` walks the entire skill directory tree, reads every file into memory, and computes SHA-256. This is O(N\*F) where N = copy-mode assignments and F = files per skill. The same source skill directory gets hashed repeatedly if it appears in multiple assignments. The `skills` table already has a `content_hash` column that gets populated at install/update time -- this cached value should be used instead of recomputing.

**Frontend:** `AssignmentMatrix` passes the full `assignments[]` array to every `MatrixRow`. The row's custom `memo` comparator checks referential equality (`prev.assignments !== next.assignments`), which means ALL rows re-render whenever ANY assignment changes. Each row also does a linear `.find()` scan inside its render to locate its relevant assignments, making the render O(R\*A) where R = rows and A = total assignments.

**Primary recommendation:** Use the existing `skills.content_hash` field as the source-of-truth hash (already computed at install/update), and build a `Map<string, ProjectSkillAssignmentDto>` lookup in the parent so each row receives only its own assignment data.

## Bottleneck 1: Backend Hash Caching

### Current Cost Analysis

`hash_dir` (content_hash.rs:14-42) does:

1. `WalkDir::new(path)` -- traverses the entire directory recursively [VERIFIED: content_hash.rs]
2. For each file: `std::fs::read(entry.path())` -- reads full file contents into memory [VERIFIED: content_hash.rs]
3. Feeds path bytes + file bytes into SHA-256 hasher [VERIFIED: content_hash.rs]
4. Returns hex-encoded digest [VERIFIED: content_hash.rs]

For a typical skill with 10-20 files totaling ~100KB, this is fast individually. But `list_assignments_with_staleness` (project_sync.rs:225-345) calls it in a loop for every copy-mode assignment. With 20 skills assigned to 3 copy-mode tools = 60 hash computations per list load. The same skill's source directory gets hashed multiple times. [VERIFIED: project_sync.rs line 308]

### Where Hashes Are Already Computed

The `skills` table already stores `content_hash TEXT NULL` (skill_store.rs schema). This hash is computed: [VERIFIED: installer.rs]

- At **install time** via `compute_content_hash()` (installer.rs:820-826) -- but only in debug mode or when `SKILLS_HUB_COMPUTE_HASH=1` env var is set (installer.rs:828-836) [VERIFIED: installer.rs]
- At **update time** via the same mechanism [VERIFIED: installer.rs]
- At **sync time** for copy-mode in `assign_and_sync` and `sync_single_assignment` (project_sync.rs:64-73, 141-150) -- this writes to the assignment's `content_hash`, not the skill's [VERIFIED: project_sync.rs]

**Key finding:** The `should_compute_content_hash()` guard means `skills.content_hash` is often NULL in release builds. The staleness check in `list_assignments_with_staleness` recomputes from disk every time because it cannot rely on the skill record having a hash. [VERIFIED: installer.rs:828-836]

### Recommended Solution: Always Compute and Cache on Skill Record

1. **Remove the `should_compute_content_hash()` guard** -- always compute `content_hash` at install and update time. The cost is incurred once (at install/update) rather than on every assignment listing. [ASSUMED -- env var guard may have been added for a reason, but the cost is negligible at install time]

2. **In `list_assignments_with_staleness`**, use `skill.content_hash` from the already-fetched `skill_opt` instead of calling `content_hash::hash_dir(source)`. Compare `assignment.content_hash` against `skill.content_hash`. [VERIFIED: both fields exist in schema]

3. **Invalidation:** The skill record's hash naturally updates on install/update (the only times source content changes). No additional invalidation logic needed. [VERIFIED: installer.rs computes hash at install/update]

4. **Fallback for NULL:** If `skill.content_hash` is None (legacy records), fall back to on-disk computation once and update the skill record. This handles the migration from existing data. [ASSUMED]

### Code Change Pattern

In `project_sync.rs` line 307-309, replace:

```rust
// BEFORE: recomputes on every list call
if let Ok(current_hash) = content_hash::hash_dir(source) {
    let is_stale = assignment.content_hash.as_deref() != Some(&current_hash);
```

with:

```rust
// AFTER: use cached hash from skill record, fallback to disk computation
let current_hash = skill.content_hash.clone().or_else(|| {
    content_hash::hash_dir(source).ok()
});
if let Some(ref hash) = current_hash {
    let is_stale = assignment.content_hash.as_deref() != Some(hash);
```

In `installer.rs` line 828-836, simplify:

```rust
// BEFORE: gated behind env var / debug mode
fn should_compute_content_hash() -> bool { ... }

// AFTER: always compute
fn compute_content_hash(path: &Path) -> Option<String> {
    hash_dir(path).ok()
}
```

### Additional Optimization: Deduplicate Per-Skill Hashing

Even with the skill-record cache, the loop in `list_assignments_with_staleness` calls `store.get_skill_by_id()` for every assignment. Multiple assignments for the same skill_id (different tools) fetch the same skill record repeatedly.

**Optimization:** Pre-fetch all unique skills before the loop using a single query or a HashMap cache:

```rust
let mut skill_cache: HashMap<String, Option<SkillRecord>> = HashMap::new();
for assignment in &assignments {
    skill_cache.entry(assignment.skill_id.clone()).or_insert_with(|| {
        store.get_skill_by_id(&assignment.skill_id).ok().flatten()
    });
}
```

This also deduplicates the `get_project_by_id` call (same project for all assignments in a single list call). [VERIFIED: project_sync.rs line 243 calls get_project_by_id inside the loop]

### Schema Migration

**Not needed.** The `skills.content_hash` column already exists in V1 schema. The `project_skill_assignments.content_hash` column already exists (added in V5 migration). No new columns required. [VERIFIED: skill_store.rs SCHEMA_V1 line 28, migration V5 line 207]

## Bottleneck 2: Frontend Assignment Lookup

### Current Rendering Behavior

`AssignmentMatrix` (AssignmentMatrix.tsx) receives: [VERIFIED: AssignmentMatrix.tsx line 19-32]

- `assignments: ProjectSkillAssignmentDto[]` -- the full flat list
- `skills: ManagedSkill[]` -- all skills
- `tools: ProjectToolDto[]` -- tool columns
- `pendingCells: Set<string>` -- pending operation keys

Every `MatrixRow` receives the entire `assignments` array. The memo comparator on MatrixRow (line 447-458) does referential equality check: `prev.assignments !== next.assignments`. [VERIFIED: AssignmentMatrix.tsx line 449]

**Problem:** When any assignment changes (toggle, sync), the entire `assignments` array reference changes (new array from IPC), which forces ALL MatrixRow components to re-render even though only one row's data changed. [VERIFIED: useProjectState.ts line 224 sets entire new array]

Inside each MatrixRow render, the inner loop does: [VERIFIED: AssignmentMatrix.tsx line 389-391]

```typescript
const assignment = assignments.find(
  (a) => a.skill_id === skill.id && a.tool === tool.tool,
);
```

This is O(A) per cell, O(A*T) per row, O(S*A\*T) total where S=skills, A=assignments, T=tools.

### Recommended Solution: Precomputed Lookup Map

Build a `Map<string, ProjectSkillAssignmentDto>` keyed by `${skill_id}:${tool}` in the parent, pass only the row's slice to each row.

**Step 1: Build lookup map in AssignmentMatrix** (parent component):

```typescript
const assignmentMap = useMemo(() => {
  const map = new Map<string, ProjectSkillAssignmentDto>();
  for (const a of assignments) {
    map.set(`${a.skill_id}:${a.tool}`, a);
  }
  return map;
}, [assignments]);
```

**Step 2: Derive per-row assignment data:**

Option A (simpler, good enough) -- pass the Map to each row, row does O(T) lookups instead of O(A) scans:

```typescript
<MatrixRow
  key={skill.id}
  skill={skill}
  tools={tools}
  assignmentMap={assignmentMap}  // Map instead of array
  ...
/>
```

Row lookup becomes `assignmentMap.get(\`${skill.id}:${tool.tool}\`)` -- O(1) per cell. [ASSUMED: standard Map lookup pattern]

Option B (more complex, best perf) -- extract per-row assignments into a sub-map in the parent and memoize per skill.id, so row memo skips re-render entirely when its data hasn't changed. This requires more plumbing but prevents rows from re-rendering when unrelated assignments change.

**Recommended: Option A.** It is simpler, eliminates the O(A) scan bottleneck, and the memo comparator change from `assignments` array equality to `assignmentMap` referential equality still means all rows re-render on any change -- but each row's render is now O(T) instead of O(A\*T). This is a significant improvement (T is typically 2-5, A could be hundreds) with minimal code change.

Option B can be revisited if profiling shows row re-rendering (not lookup cost) is the dominant factor.

### MatrixRow Memo Comparator Update

Update the MatrixRow props type to accept `assignmentMap: Map<string, ProjectSkillAssignmentDto>` instead of `assignments: ProjectSkillAssignmentDto[]`. The memo comparator at line 447 changes:

```typescript
// BEFORE
if (prev.assignments !== next.assignments) return false;

// AFTER
if (prev.assignmentMap !== next.assignmentMap) return false;
```

This maintains the same referential equality behavior but the inner render benefits from O(1) lookup. [VERIFIED: AssignmentMatrix.tsx line 449]

## Existing Patterns to Follow

### Schema Migration Pattern

The codebase uses incremental version-gated migrations in `ensure_schema()` (skill_store.rs:180-233). Pattern: `if user_version < N { ... }`. No new migration is needed for this task since both columns exist. [VERIFIED: skill_store.rs]

### Caching Pattern

No in-memory caching pattern exists in the Rust backend -- all data is fetched from SQLite per request via `with_conn()` which opens a new connection each call. The `SkillStore` is stateless (just holds `db_path`). Adding a HashMap cache for skill records within a single function call (not across calls) is the appropriate scope. [VERIFIED: skill_store.rs line 992-998]

### React Memo Pattern

Components use `memo()` with custom comparators (AssignmentMatrix.tsx:462-475, MatrixRow at 366-458). The project follows the pattern of comparing each prop individually rather than shallow-equal. `useMemo` is used for derived data (sortedSkills, skillGroups, lastSyncAt). Adding another `useMemo` for the assignment map fits this pattern. [VERIFIED: AssignmentMatrix.tsx]

## Common Pitfalls

### Pitfall 1: Stale Hash After Source Edit

**What goes wrong:** If a user manually edits files in `~/.skillshub/<skill>/` without going through install/update, `skills.content_hash` becomes stale.
**Why it happens:** Hash is only recomputed on install/update, not on arbitrary filesystem changes.
**How to avoid:** This is acceptable behavior -- the app's data model treats install/update as the canonical mutation points. Resync recomputes hashes anyway.
**Warning signs:** Copy-mode assignments showing "synced" when they should be "stale".

### Pitfall 2: NULL content_hash on Legacy Records

**What goes wrong:** Skills installed before the hash-always-on change will have `content_hash = NULL`, making the staleness comparison short-circuit.
**How to avoid:** Add a backfill: when `skill.content_hash` is None, compute it, update the skill record, and proceed. This is a one-time cost per legacy skill.

### Pitfall 3: Map Key Format Mismatch

**What goes wrong:** Frontend lookup map uses `${skill_id}:${tool}` but the pending cells also use `${skillId}:${tool}`. If formats diverge, lookups silently fail.
**How to avoid:** Use the same `cellKey` format already established in AssignmentMatrix.tsx line 387: `${skill.id}:${tool.tool}`. Reuse this exact pattern for the map key.

## Assumptions Log

| #   | Claim                                                                                            | Section      | Risk if Wrong                                                             |
| --- | ------------------------------------------------------------------------------------------------ | ------------ | ------------------------------------------------------------------------- |
| A1  | The `should_compute_content_hash()` env var guard was a performance optimization, safe to remove | Bottleneck 1 | Low -- worst case adds ~50ms to install/update operations                 |
| A2  | Backfilling NULL hashes on legacy skill records is a one-time acceptable cost                    | Bottleneck 1 | Low -- happens once per skill, same cost as current per-load computation  |
| A3  | Option A (Map in parent, O(1) lookup per cell) is sufficient without Option B (per-row slice)    | Bottleneck 2 | Low -- can iterate to Option B if profiling shows row re-render dominance |

## Sources

### Primary (HIGH confidence)

- `src-tauri/src/core/content_hash.rs` -- full hash_dir implementation read
- `src-tauri/src/core/project_sync.rs` -- full staleness logic read
- `src-tauri/src/core/installer.rs` -- hash computation at install/update
- `src-tauri/src/core/skill_store.rs` -- full schema and migration pattern
- `src/components/projects/AssignmentMatrix.tsx` -- full component and memo logic
- `src/components/projects/useProjectState.ts` -- full state management hook
- `src-tauri/src/commands/projects.rs` -- full IPC command layer

### Secondary (MEDIUM confidence)

- React useMemo / memo patterns -- standard React 19 APIs [ASSUMED: stable since React 16.6+]

## Metadata

**Confidence breakdown:**

- Backend hash caching: HIGH -- all code paths traced, schema verified, solution uses existing infrastructure
- Frontend lookup map: HIGH -- rendering pattern fully analyzed, solution is standard React optimization
- Integration: HIGH -- no schema migration needed, both changes are additive

**Research date:** 2026-04-16
**Valid until:** 2026-05-16 (stable domain, no external dependencies)
