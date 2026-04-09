# Phase 01 -- Data Foundation: Security Audit

**Audited:** 2026-04-07
**ASVS Level:** 1
**Threats Closed:** 9/9
**Threats Open:** 0/9
**Block Policy:** block_on=open (no open threats -- not blocking)

## Threat Verification

| Threat ID | Category               | Component                               | Disposition | Status | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                        |
| --------- | ---------------------- | --------------------------------------- | ----------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| T-01-01   | Tampering              | skill_store.rs SQL queries              | mitigate    | CLOSED | skill_store.rs: 24 occurrences of `params![]` macro across all query methods. Zero occurrences of `format!` with SQL keywords (SELECT, INSERT, DELETE, UPDATE, WHERE). No string interpolation into SQL.                                                                                                                                                                                                                        |
| T-01-02   | Tampering              | Project path storage                    | mitigate    | CLOSED | project_ops.rs:72 -- `std::fs::canonicalize(&expanded)` resolves all symlinks and `..` before storing. project_ops.rs:79 -- path stored as canonical string via `canonical.to_string_lossy()`. Store methods (skill_store.rs:523-531) receive already-validated path strings through `params![]`.                                                                                                                               |
| T-01-03   | Information Disclosure | SQLite database file                    | accept      | CLOSED | Accepted risk. Database file is local to the user's app data directory (skill_store.rs:829-837 `default_db_path`). Single-user desktop app with no multi-tenant access. No network exposure of database contents.                                                                                                                                                                                                               |
| T-01-04   | Denial of Service      | CASCADE delete on large datasets        | accept      | CLOSED | Accepted risk. Desktop app with bounded data volume. CASCADE delete via `ON DELETE CASCADE` on foreign keys (skill_store.rs:77,90-91) operates on user-scale project/assignment counts.                                                                                                                                                                                                                                         |
| T-02-01   | Tampering              | register_project_path input             | mitigate    | CLOSED | project_ops.rs:69,71 -- `expand_home(path)` callback invokes commands/mod.rs:250 `expand_home_path()` which handles `~` expansion and empty-string rejection. project_ops.rs:72 -- `std::fs::canonicalize()` resolves all symlinks and `..` components. project_ops.rs:75 -- `canonical.is_dir()` validates target exists and is a directory.                                                                                   |
| T-02-02   | Tampering              | remove_project filesystem cleanup       | mitigate    | CLOSED | project_ops.rs:111 -- calls `sync_engine::remove_path_any(&target)`. sync_engine.rs:137 -- `pub(crate) fn remove_path_any` uses `std::fs::symlink_metadata(path)` at line 138 to detect symlinks without following them. sync_engine.rs:146-148 -- symlinks detected via `ft.is_symlink()` are removed with `std::fs::remove_file(path)`, which removes the link itself, not the target. Line 140 returns `Ok(())` on NotFound. |
| T-02-03   | Tampering              | SQL via IPC parameters                  | mitigate    | CLOSED | commands/projects.rs delegates all IPC parameters through SkillStore methods (e.g., lines 16, 28, 38, 63, 78, 92, 139, 167, 182). All SkillStore methods in skill_store.rs use `params![]` macro exclusively (24 usages, zero string interpolation into SQL). No raw SQL construction in commands/projects.rs.                                                                                                                  |
| T-02-04   | Elevation              | Path traversal via project registration | mitigate    | CLOSED | project_ops.rs:72 -- `std::fs::canonicalize(&expanded)` resolves all `..` and symlink components to an absolute real path. project_ops.rs:75 -- `canonical.is_dir()` rejects non-directory paths. Combined, these prevent registering arbitrary file paths or traversal targets. Validation in core layer (project_ops.rs), not in the command layer (commands/projects.rs contains no canonicalize or is_dir calls).           |
| T-02-05   | Denial of Service      | Large project list queries              | accept      | CLOSED | Accepted risk. Desktop app with bounded project count. `list_projects` (skill_store.rs:533) and aggregate queries (skill_store.rs:776) operate on user-scale data volumes.                                                                                                                                                                                                                                                      |

## Accepted Risks Log

| Threat ID | Category               | Risk Description                                  | Justification                                                                                                                                           |
| --------- | ---------------------- | ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| T-01-03   | Information Disclosure | SQLite database file readable on local filesystem | Single-user desktop app. Database is in user's app data directory. No multi-tenant isolation needed. OS file permissions provide adequate protection.   |
| T-01-04   | Denial of Service      | CASCADE delete may be slow on very large datasets | Desktop app with bounded data volume. Users manage tens to low hundreds of projects/assignments, not millions. Performance is acceptable at this scale. |
| T-02-05   | Denial of Service      | Large project list queries with JOINed counts     | Desktop app with bounded project count. Aggregate status queries use GROUP BY on small datasets. No adversarial workload expected.                      |

## Unregistered Flags

None. Neither 01-01-SUMMARY.md nor 01-02-SUMMARY.md contain a `## Threat Flags` section. No unregistered attack surface was flagged during implementation.

## Verification Method

- **mitigate** threats: Verified by grep for declared mitigation patterns in cited implementation files. Each pattern was confirmed present at specific file:line locations.
- **accept** threats: Verified by presence in the Accepted Risks Log above.
- All implementation files were read-only during this audit (no modifications made).

---

_Phase: 01-data-foundation_
_Audited: 2026-04-07_
_Auditor: gsd-security-auditor_
