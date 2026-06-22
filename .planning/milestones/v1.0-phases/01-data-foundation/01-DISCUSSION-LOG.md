# Phase 1: Data Foundation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-07
**Phase:** 01-data-foundation
**Areas discussed:** Project identity, Schema columns, Delete behavior, Path storage format

---

## Project Identity

| Option             | Description                                                                   | Selected |
| ------------------ | ----------------------------------------------------------------------------- | -------- |
| Path-derived name  | Derive display name from directory basename. Simpler schema, one fewer field. | ✓        |
| User-assigned name | Custom display name at registration. More flexible but adds a field.          |          |
| You decide         | Let Claude decide.                                                            |          |

**User's choice:** Path-derived name
**Notes:** Display as folder basename (title) + truncated path (subtitle), both derived at render time.

---

| Option                | Description                                                                | Selected |
| --------------------- | -------------------------------------------------------------------------- | -------- |
| UUID                  | UUIDs like existing skills. Consistent pattern, globally unique.           | ✓        |
| Path as primary key   | Use path itself as PK. Simpler but renaming requires cascading PK changes. |          |
| Integer autoincrement | Simple but breaks UUID convention.                                         |          |

**User's choice:** UUID
**Notes:** Matches existing SkillRecord/SkillTargetRecord pattern.

---

| Option               | Description                                             | Selected |
| -------------------- | ------------------------------------------------------- | -------- |
| Free-text discussion | User asked about display intent for path-derived names. | ✓        |

**User's choice:** Basename as title, truncated path as subtitle — both derived from stored path at render time, no extra columns needed.

---

## Schema Columns

| Option       | Description                                                                 | Selected |
| ------------ | --------------------------------------------------------------------------- | -------- |
| Minimal      | Just essentials: id, path, timestamps. Extend via migrations later.         |          |
| Full upfront | Include status, sync metadata, notes columns. Avoids V5/V6 migration churn. | ✓        |

**User's choice:** Full upfront
**Notes:** Avoids schema migration churn in later phases.

---

| Option               | Description                                                               | Selected |
| -------------------- | ------------------------------------------------------------------------- | -------- |
| Mirror skill_targets | status, mode, last_error, synced_at columns. Consistent with global sync. | ✓        |
| Assignment-only      | Just the link, no sync metadata. Status computed from filesystem.         |          |
| You decide           | Let Claude decide.                                                        |          |

**User's choice:** Mirror skill_targets
**Notes:** Consistent with existing pattern.

---

| Option           | Description                                     | Selected |
| ---------------- | ----------------------------------------------- | -------- |
| Standard indexes | idx_psa_project, idx_psa_skill, idx_pt_project. | ✓        |
| No extra indexes | Add later when performance data shows need.     |          |
| You decide       | Let Claude decide.                              |          |

**User's choice:** Standard indexes

---

## Delete Behavior

| Option                | Description                                                      | Selected |
| --------------------- | ---------------------------------------------------------------- | -------- |
| Hard delete + CASCADE | Remove rows, CASCADE to child tables. Matches existing pattern.  | ✓        |
| Soft delete           | Set status='deleted'. Allows recovery but adds query complexity. |          |
| You decide            | Let Claude decide.                                               |          |

**User's choice:** Hard delete + CASCADE
**Notes:** Consistent with existing skills → skill_targets pattern.

---

| Option                  | Description                                                   | Selected |
| ----------------------- | ------------------------------------------------------------- | -------- |
| DB-only in Phase 1      | Just delete DB rows. Filesystem cleanup deferred to Phase 2.  |          |
| Full cleanup in Phase 1 | Delete also removes symlinks/copies from project directories. | ✓        |

**User's choice:** Full cleanup in Phase 1
**Notes:** Pulls in sync engine dependency but keeps delete operation complete.

---

## Path Storage Format

| Option             | Description                                                                     | Selected |
| ------------------ | ------------------------------------------------------------------------------- | -------- |
| Canonicalized      | Expand ~ and resolve symlinks. Consistent matching, no duplicate registrations. | ✓        |
| As-entered         | Store exactly what user provides. ~/foo and /home/user/foo are different.       |          |
| Home-expanded only | Expand ~ but preserve symlinks. Middle ground.                                  |          |

**User's choice:** Canonicalized
**Notes:** Reuses existing expand_home_path() plus std::fs::canonicalize().

---

| Option          | Description                                                 | Selected |
| --------------- | ----------------------------------------------------------- | -------- |
| Validate exists | Require directory to exist at registration. Prevents typos. | ✓        |
| Accept any path | Accept any path even if nonexistent.                        |          |

**User's choice:** Validate exists
**Notes:** Phase 5 handles directories that go missing after registration.

---

## Claude's Discretion

- Exact CRUD function signatures and internal structuring of commands/projects.rs
- Whether to extend SkillStore or create a separate ProjectStore struct
- Test structure and coverage scope

## Deferred Ideas

None — discussion stayed within phase scope.
