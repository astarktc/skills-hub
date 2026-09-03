use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use tauri::Manager;

use super::errors::SignalError;
use super::sync_status::{aggregate, ProjectSyncStatus, SyncMode, SyncStatus};

const DB_FILE_NAME: &str = "skills_hub.db";
const LEGACY_APP_IDENTIFIERS: &[&str] = &[
    "com.tauri.dev",
    "com.tauri.dev.skillshub",
    "com.qufei1993.skillshub",
];

// Schema versioning: bump when making changes and add a migration step.
const SCHEMA_VERSION: i32 = 8;

// Minimal schema for MVP: skills, skill_targets, settings, discovered_skills(optional).
const SCHEMA_V1: &str = r#"
CREATE TABLE IF NOT EXISTS skills (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  source_type TEXT NOT NULL,
  source_ref TEXT NULL,
  source_revision TEXT NULL,
  central_path TEXT NOT NULL UNIQUE,
  content_hash TEXT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  last_sync_at INTEGER NULL,
  last_seen_at INTEGER NOT NULL,
  status TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS skill_targets (
  id TEXT PRIMARY KEY,
  skill_id TEXT NOT NULL,
  tool TEXT NOT NULL,
  target_path TEXT NOT NULL,
  mode TEXT NOT NULL,
  status TEXT NOT NULL,
  last_error TEXT NULL,
  synced_at INTEGER NULL,
  UNIQUE(skill_id, tool),
  FOREIGN KEY(skill_id) REFERENCES skills(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS discovered_skills (
  id TEXT PRIMARY KEY,
  tool TEXT NOT NULL,
  found_path TEXT NOT NULL,
  name_guess TEXT NULL,
  fingerprint TEXT NULL,
  found_at INTEGER NOT NULL,
  imported_skill_id TEXT NULL,
  FOREIGN KEY(imported_skill_id) REFERENCES skills(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_skills_name ON skills(name);
CREATE INDEX IF NOT EXISTS idx_skills_updated_at ON skills(updated_at);
"#;

// V4: project tables for per-project skill distribution.
const MIGRATION_V4: &str = r#"
BEGIN;
CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS project_tools (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  tool TEXT NOT NULL,
  UNIQUE(project_id, tool),
  FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS project_skill_assignments (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  skill_id TEXT NOT NULL,
  skill_name TEXT NOT NULL DEFAULT '',
  tool TEXT NOT NULL,
  mode TEXT NOT NULL,
  status TEXT NOT NULL,
  last_error TEXT NULL,
  synced_at INTEGER NULL,
  content_hash TEXT NULL,
  created_at INTEGER NOT NULL,
  UNIQUE(project_id, skill_id, tool),
  FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
  FOREIGN KEY(skill_id) REFERENCES skills(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_psa_project ON project_skill_assignments(project_id);
CREATE INDEX IF NOT EXISTS idx_psa_skill ON project_skill_assignments(skill_id);
CREATE INDEX IF NOT EXISTS idx_pt_project ON project_tools(project_id);
COMMIT;
"#;

#[derive(Clone, Debug)]
pub struct SkillStore {
    db_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct SkillRecord {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub source_type: String,
    pub source_ref: Option<String>,
    pub source_subpath: Option<String>,
    pub source_revision: Option<String>,
    pub central_path: String,
    pub content_hash: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_sync_at: Option<i64>,
    pub last_seen_at: i64,
    pub status: String,
}

#[derive(Clone, Debug)]
pub struct SkillTargetRecord {
    pub id: String,
    pub skill_id: String,
    pub tool: String,
    pub target_path: String,
    pub mode: SyncMode,
    pub status: SyncStatus,
    pub last_error: Option<String>,
    pub synced_at: Option<i64>,
}

#[derive(Clone, Debug)]
pub struct ProjectRecord {
    pub id: String,
    pub path: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug)]
pub struct ProjectToolRecord {
    pub id: String,
    pub project_id: String,
    pub tool: String,
}

/// One project's derived numbers: how many Tools it has configured, how many
/// distinct Managed skills and assignment rows it carries, and the
/// precedence fold of those rows' Sync statuses (Project sync status).
/// Read by [`SkillStore::project_aggregates`] in one grouped pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectAggregate {
    pub tool_count: usize,
    pub skill_count: usize,
    pub assignment_count: usize,
    pub sync_status: ProjectSyncStatus,
}

impl Default for ProjectAggregate {
    fn default() -> Self {
        Self {
            tool_count: 0,
            skill_count: 0,
            assignment_count: 0,
            sync_status: ProjectSyncStatus::Empty,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProjectSkillAssignmentRecord {
    pub id: String,
    pub project_id: String,
    pub skill_id: String,
    pub skill_name: String,
    pub tool: String,
    pub mode: SyncMode,
    pub status: SyncStatus,
    pub last_error: Option<String>,
    pub synced_at: Option<i64>,
    pub content_hash: Option<String>,
    pub created_at: i64,
}

/// The store seam for the lifecycle columns: parse the stored `mode` and
/// `status` strings into the typed vocabulary.
///
/// Legacy policy (no schema change, so any string may be on disk): a value
/// `sync_status` does not recognise cannot be treated as healthy and must not
/// abort the whole listing. The row surfaces as `Error` with the raw value in
/// `last_error` (a diagnostic, not user copy) and a warning is logged; an
/// unknown mode is read as `Copy` so the next update re-syncs it rather than
/// assuming a link that follows the source. A re-sync then rewrites the row
/// with canonical strings. Never a panic, never a silent coercion.
fn read_lifecycle(
    row_id: &str,
    raw_mode: String,
    raw_status: String,
    last_error: Option<String>,
) -> (SyncMode, SyncStatus, Option<String>) {
    let mode = SyncMode::from_stored(&raw_mode);
    let status = SyncStatus::from_stored(&raw_status);
    match (mode, status) {
        (Some(mode), Some(status)) => (mode, status, last_error),
        _ => {
            let diagnostic = format!(
                "unrecognised stored sync lifecycle (mode: {:?}, status: {:?})",
                raw_mode, raw_status
            );
            log::warn!("row {}: {}", row_id, diagnostic);
            (
                mode.unwrap_or(SyncMode::Copy),
                SyncStatus::Error,
                Some(diagnostic),
            )
        }
    }
}

/// A typed write to a global target row's lifecycle columns — the global
/// counterpart of [`AssignmentTransition`], so Propagation settles a row by
/// naming what happened instead of rebuilding a whole record literal.
#[derive(Clone, Copy, Debug)]
pub enum TargetTransition<'a> {
    /// A sync just succeeded: records the mode actually used and where the
    /// artifact landed (a shared skills dir group settles every member row
    /// with the one path that was written).
    SyncCompleted {
        mode: SyncMode,
        target_path: &'a str,
        synced_at: i64,
    },
    /// A sync failed; `error` is the diagnostic chain, and the row keeps its
    /// recorded mode and path so the failure stays observable.
    SyncFailed { error: &'a str },
}

/// A typed write to an assignment's lifecycle columns — the only way the
/// `status`/`mode`/`last_error`/`synced_at`/`content_hash` group changes
/// after insertion, so the legal combinations are spelled out here rather
/// than left to positional `None`s.
#[derive(Clone, Copy, Debug)]
pub enum AssignmentTransition<'a> {
    /// A sync just succeeded: records the mode used, the timestamp, and the
    /// source content hash (copies only) so drift can be detected later.
    SyncCompleted {
        mode: SyncMode,
        synced_at: i64,
        content_hash: Option<&'a str>,
    },
    /// A sync or cleanup failed; `error` is the diagnostic chain. The
    /// recorded hash is dropped (the target's content is unknown).
    SyncFailed { error: &'a str },
    /// A reconcile pass decided the row's true status (see
    /// `sync_status::next_status`). `content_hash` is the confirmed source
    /// hash for a `Synced` copy, `None` otherwise. Mode and `synced_at` are
    /// untouched — nothing was written to disk.
    Reconciled {
        status: SyncStatus,
        content_hash: Option<&'a str>,
    },
}

impl SkillStore {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    #[allow(dead_code)]
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn ensure_schema(&self) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute_batch("PRAGMA foreign_keys = ON;")?;

            let user_version: i32 = conn.query_row("PRAGMA user_version;", [], |row| row.get(0))?;
            if user_version == 0 {
                conn.execute_batch(SCHEMA_V1)?;
                // V2: add description column
                conn.execute_batch("ALTER TABLE skills ADD COLUMN description TEXT NULL;")?;
                // V3: add source_subpath column
                conn.execute_batch("ALTER TABLE skills ADD COLUMN source_subpath TEXT NULL;")?;
                // V4: project tables for per-project skill distribution
                // (DDL includes V5 content_hash column in project_skill_assignments)
                conn.execute_batch(MIGRATION_V4)?;
                // V7: hidden explore skills table
                conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS hidden_explore_skills (
                        source_url TEXT PRIMARY KEY,
                        hidden_at INTEGER NOT NULL
                    );",
                )?;
                conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
            } else if user_version < SCHEMA_VERSION {
                // Incremental migrations
                if user_version < 2 {
                    conn.execute_batch("ALTER TABLE skills ADD COLUMN description TEXT NULL;")?;
                }
                if user_version < 3 {
                    conn.execute_batch("ALTER TABLE skills ADD COLUMN source_subpath TEXT NULL;")?;
                }
                if user_version < 4 {
                    conn.execute_batch(MIGRATION_V4)?;
                }
                if user_version < 5 {
                    conn.execute_batch(
                        "ALTER TABLE project_skill_assignments ADD COLUMN content_hash TEXT NULL;",
                    )?;
                }
                if user_version < 6 {
                    conn.execute_batch(
                        "ALTER TABLE project_skill_assignments ADD COLUMN skill_name TEXT NOT NULL DEFAULT '';",
                    )?;
                    // Backfill skill_name from the skills table for existing rows
                    conn.execute_batch(
                        "UPDATE project_skill_assignments SET skill_name = COALESCE(
                            (SELECT name FROM skills WHERE skills.id = project_skill_assignments.skill_id),
                            ''
                        ) WHERE skill_name = '';",
                    )?;
                }
                if user_version < 8 {
                    // Consolidate 9 .agents/skills tools into single agents_skills key.
                    // Must DELETE duplicates BEFORE UPDATE due to UNIQUE(project_id, tool) constraint.
                    conn.execute_batch(
                        "DELETE FROM project_tools WHERE rowid NOT IN (
                            SELECT MIN(rowid) FROM project_tools
                            WHERE tool IN ('cursor','codex','amp','kimi_cli','antigravity','cline','gemini_cli','github_copilot','opencode')
                            GROUP BY project_id
                        ) AND tool IN ('cursor','codex','amp','kimi_cli','antigravity','cline','gemini_cli','github_copilot','opencode');
                        UPDATE project_tools SET tool = 'agents_skills'
                            WHERE tool IN ('cursor','codex','amp','kimi_cli','antigravity','cline','gemini_cli','github_copilot','opencode');

                        DELETE FROM project_skill_assignments WHERE rowid NOT IN (
                            SELECT MIN(rowid) FROM project_skill_assignments
                            WHERE tool IN ('cursor','codex','amp','kimi_cli','antigravity','cline','gemini_cli','github_copilot','opencode')
                            GROUP BY project_id, skill_id
                        ) AND tool IN ('cursor','codex','amp','kimi_cli','antigravity','cline','gemini_cli','github_copilot','opencode');
                        UPDATE project_skill_assignments SET tool = 'agents_skills'
                            WHERE tool IN ('cursor','codex','amp','kimi_cli','antigravity','cline','gemini_cli','github_copilot','opencode');",
                    )?;
                }
            } else if user_version > SCHEMA_VERSION {
                anyhow::bail!(
                    "database schema version {} is newer than app supports {}",
                    user_version,
                    SCHEMA_VERSION
                );
            }

            // Ensure V7 table exists (handles DBs that were created at V7 without this table)
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS hidden_explore_skills (
                    source_url TEXT PRIMARY KEY,
                    hidden_at INTEGER NOT NULL
                );",
            )?;

            Ok(())
        })
    }

    /// Raw settings-table adapter. `core::settings` is the only intended
    /// caller — it owns key names, defaults and validation — so visibility is
    /// limited to `core` and the invariant is compiler-enforced rather than
    /// documented.
    pub(super) fn get_setting(&self, key: &str) -> Result<Option<String>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
            let mut rows = stmt.query(params![key])?;
            Ok(rows
                .next()?
                .map(|row| row.get::<_, String>(0))
                .transpose()?)
        })
    }

    /// Raw settings-table adapter; see [`SkillStore::get_setting`].
    pub(super) fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )?;
            Ok(())
        })
    }

    pub fn upsert_skill(&self, record: &SkillRecord) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO skills (
          id, name, description, source_type, source_ref, source_subpath, source_revision, central_path, content_hash,
          created_at, updated_at, last_sync_at, last_seen_at, status
        ) VALUES (
          ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
          ?10, ?11, ?12, ?13, ?14
        )
        ON CONFLICT(id) DO UPDATE SET
          name = excluded.name,
          description = excluded.description,
          source_type = excluded.source_type,
          source_ref = excluded.source_ref,
          source_subpath = excluded.source_subpath,
          source_revision = excluded.source_revision,
          central_path = excluded.central_path,
          content_hash = excluded.content_hash,
          created_at = excluded.created_at,
          updated_at = excluded.updated_at,
          last_sync_at = excluded.last_sync_at,
          last_seen_at = excluded.last_seen_at,
          status = excluded.status",
                params![
                    record.id,
                    record.name,
                    record.description,
                    record.source_type,
                    record.source_ref,
                    record.source_subpath,
                    record.source_revision,
                    record.central_path,
                    record.content_hash,
                    record.created_at,
                    record.updated_at,
                    record.last_sync_at,
                    record.last_seen_at,
                    record.status
                ],
            )?;
            Ok(())
        })
    }

    pub fn upsert_skill_target(&self, record: &SkillTargetRecord) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO skill_targets (
          id, skill_id, tool, target_path, mode, status, last_error, synced_at
        ) VALUES (
          ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8
        )
        ON CONFLICT(skill_id, tool) DO UPDATE SET
          target_path = excluded.target_path,
          mode = excluded.mode,
          status = excluded.status,
          last_error = excluded.last_error,
          synced_at = excluded.synced_at",
                params![
                    record.id,
                    record.skill_id,
                    record.tool,
                    record.target_path,
                    record.mode.as_str(),
                    record.status.as_str(),
                    record.last_error,
                    record.synced_at
                ],
            )?;
            Ok(())
        })
    }

    pub fn list_skills(&self) -> Result<Vec<SkillRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
        "SELECT id, name, description, source_type, source_ref, source_subpath, source_revision, central_path, content_hash,
                created_at, updated_at, last_sync_at, last_seen_at, status
         FROM skills
         ORDER BY updated_at DESC",
      )?;
            let rows = stmt.query_map([], |row| {
                Ok(SkillRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    source_type: row.get(3)?,
                    source_ref: row.get(4)?,
                    source_subpath: row.get(5)?,
                    source_revision: row.get(6)?,
                    central_path: row.get(7)?,
                    content_hash: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    last_sync_at: row.get(11)?,
                    last_seen_at: row.get(12)?,
                    status: row.get(13)?,
                })
            })?;

            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(items)
        })
    }

    pub fn get_skill_by_id(&self, skill_id: &str) -> Result<Option<SkillRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
        "SELECT id, name, description, source_type, source_ref, source_subpath, source_revision, central_path, content_hash,
                created_at, updated_at, last_sync_at, last_seen_at, status
         FROM skills
         WHERE id = ?1
         LIMIT 1",
      )?;
            let mut rows = stmt.query(params![skill_id])?;
            if let Some(row) = rows.next()? {
                Ok(Some(SkillRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    source_type: row.get(3)?,
                    source_ref: row.get(4)?,
                    source_subpath: row.get(5)?,
                    source_revision: row.get(6)?,
                    central_path: row.get(7)?,
                    content_hash: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    last_sync_at: row.get(11)?,
                    last_seen_at: row.get(12)?,
                    status: row.get(13)?,
                }))
            } else {
                Ok(None)
            }
        })
    }

    pub fn update_skill_description(
        &self,
        skill_id: &str,
        description: Option<&str>,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE skills SET description = ?1 WHERE id = ?2",
                params![description, skill_id],
            )?;
            Ok(())
        })
    }

    pub fn update_skill_content_hash(&self, skill_id: &str, hash: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE skills SET content_hash = ?1 WHERE id = ?2",
                params![hash, skill_id],
            )?;
            Ok(())
        })
    }

    pub fn list_skills_missing_description(&self) -> Result<Vec<SkillRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
        "SELECT id, name, description, source_type, source_ref, source_subpath, source_revision, central_path, content_hash,
                created_at, updated_at, last_sync_at, last_seen_at, status
         FROM skills
         WHERE description IS NULL",
      )?;
            let rows = stmt.query_map([], |row| {
                Ok(SkillRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    source_type: row.get(3)?,
                    source_ref: row.get(4)?,
                    source_subpath: row.get(5)?,
                    source_revision: row.get(6)?,
                    central_path: row.get(7)?,
                    content_hash: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    last_sync_at: row.get(11)?,
                    last_seen_at: row.get(12)?,
                    status: row.get(13)?,
                })
            })?;
            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(items)
        })
    }

    pub fn delete_skill(&self, skill_id: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM skills WHERE id = ?1", params![skill_id])?;
            Ok(())
        })
    }

    pub fn list_skill_targets(&self, skill_id: &str) -> Result<Vec<SkillTargetRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, skill_id, tool, target_path, mode, status, last_error, synced_at
         FROM skill_targets
         WHERE skill_id = ?1
         ORDER BY tool ASC",
            )?;
            let rows = stmt.query_map(params![skill_id], |row| {
                let id: String = row.get(0)?;
                let (mode, status, last_error) =
                    read_lifecycle(&id, row.get(4)?, row.get(5)?, row.get(6)?);
                Ok(SkillTargetRecord {
                    id,
                    skill_id: row.get(1)?,
                    tool: row.get(2)?,
                    target_path: row.get(3)?,
                    mode,
                    status,
                    last_error,
                    synced_at: row.get(7)?,
                })
            })?;

            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(items)
        })
    }

    pub fn list_all_skill_target_paths(&self) -> Result<Vec<(String, String)>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT tool, target_path
         FROM skill_targets",
            )?;
            let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;

            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(items)
        })
    }

    /// Read one global target row. Store-level read used by tests and by
    /// future callers; the removal module plans from `list_skill_targets`.
    #[allow(dead_code)]
    pub fn get_skill_target(
        &self,
        skill_id: &str,
        tool: &str,
    ) -> Result<Option<SkillTargetRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, skill_id, tool, target_path, mode, status, last_error, synced_at
         FROM skill_targets
         WHERE skill_id = ?1 AND tool = ?2",
            )?;
            let mut rows = stmt.query(params![skill_id, tool])?;
            if let Some(row) = rows.next()? {
                let id: String = row.get(0)?;
                let (mode, status, last_error) =
                    read_lifecycle(&id, row.get(4)?, row.get(5)?, row.get(6)?);
                Ok(Some(SkillTargetRecord {
                    id,
                    skill_id: row.get(1)?,
                    tool: row.get(2)?,
                    target_path: row.get(3)?,
                    mode,
                    status,
                    last_error,
                    synced_at: row.get(7)?,
                }))
            } else {
                Ok(None)
            }
        })
    }

    pub fn delete_skill_target(&self, skill_id: &str, tool: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "DELETE FROM skill_targets WHERE skill_id = ?1 AND tool = ?2",
                params![skill_id, tool],
            )?;
            Ok(())
        })
    }

    /// Bulk delete, kept as a store capability; Artifact removal settles
    /// rows one at a time so a failed artifact keeps its row.
    #[allow(dead_code)]
    pub fn delete_all_skill_targets(&self) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM skill_targets", [])?;
            Ok(())
        })
    }

    /// Bulk delete of one skill's rows; see [`Self::delete_all_skill_targets`].
    #[allow(dead_code)]
    pub fn delete_skill_targets(&self, skill_id: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "DELETE FROM skill_targets WHERE skill_id = ?1",
                params![skill_id],
            )?;
            Ok(())
        })
    }

    pub fn list_project_skill_assignments_by_skill(
        &self,
        skill_id: &str,
    ) -> Result<Vec<ProjectSkillAssignmentRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, project_id, skill_id, skill_name, tool, mode, status, last_error, synced_at, content_hash, created_at
                 FROM project_skill_assignments
                 WHERE skill_id = ?1",
            )?;
            let rows = stmt.query_map(params![skill_id], |row| {
                let id: String = row.get(0)?;
                let (mode, status, last_error) =
                    read_lifecycle(&id, row.get(5)?, row.get(6)?, row.get(7)?);
                Ok(ProjectSkillAssignmentRecord {
                    id,
                    project_id: row.get(1)?,
                    skill_id: row.get(2)?,
                    skill_name: row.get(3)?,
                    tool: row.get(4)?,
                    mode,
                    status,
                    last_error,
                    synced_at: row.get(8)?,
                    content_hash: row.get(9)?,
                    created_at: row.get(10)?,
                })
            })?;
            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(items)
        })
    }

    pub fn update_project_path(&self, project_id: &str, new_path: &str, now_ms: i64) -> Result<()> {
        self.with_conn(|conn| {
            let rows = conn.execute(
                "UPDATE projects SET path = ?1, updated_at = ?2 WHERE id = ?3",
                params![new_path, now_ms, project_id],
            )?;
            if rows == 0 {
                anyhow::bail!(SignalError::NotFound {
                    kind: "project".to_string(),
                    id: project_id.to_string(),
                });
            }
            Ok(())
        })
    }

    // --- Project methods ---

    pub fn register_project(&self, record: &ProjectRecord) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO projects (id, path, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                params![record.id, record.path, record.created_at, record.updated_at],
            )?;
            Ok(())
        })
    }

    pub fn list_projects(&self) -> Result<Vec<ProjectRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, path, created_at, updated_at FROM projects ORDER BY created_at DESC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(ProjectRecord {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            })?;

            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(items)
        })
    }

    pub fn get_project_by_path(&self, path: &str) -> Result<Option<ProjectRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, path, created_at, updated_at FROM projects WHERE path = ?1 LIMIT 1",
            )?;
            let mut rows = stmt.query(params![path])?;
            if let Some(row) = rows.next()? {
                Ok(Some(ProjectRecord {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                }))
            } else {
                Ok(None)
            }
        })
    }

    pub fn get_project_by_id(&self, project_id: &str) -> Result<Option<ProjectRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, path, created_at, updated_at FROM projects WHERE id = ?1 LIMIT 1",
            )?;
            let mut rows = stmt.query(params![project_id])?;
            if let Some(row) = rows.next()? {
                Ok(Some(ProjectRecord {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                }))
            } else {
                Ok(None)
            }
        })
    }

    pub fn delete_project(&self, project_id: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM projects WHERE id = ?1", params![project_id])?;
            Ok(())
        })
    }

    // --- Project tool methods ---

    pub fn add_project_tool(&self, record: &ProjectToolRecord) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT OR IGNORE INTO project_tools (id, project_id, tool) VALUES (?1, ?2, ?3)",
                params![record.id, record.project_id, record.tool],
            )?;
            Ok(())
        })
    }

    pub fn list_project_tools(&self, project_id: &str) -> Result<Vec<ProjectToolRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, project_id, tool FROM project_tools WHERE project_id = ?1 ORDER BY tool ASC",
            )?;
            let rows = stmt.query_map(params![project_id], |row| {
                Ok(ProjectToolRecord {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    tool: row.get(2)?,
                })
            })?;

            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(items)
        })
    }

    pub fn remove_project_tool(&self, project_id: &str, tool: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "DELETE FROM project_tools WHERE project_id = ?1 AND tool = ?2",
                params![project_id, tool],
            )?;
            Ok(())
        })
    }

    // --- Project skill assignment methods ---

    pub fn add_project_skill_assignment(
        &self,
        record: &ProjectSkillAssignmentRecord,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO project_skill_assignments
                 (id, project_id, skill_id, skill_name, tool, mode, status, last_error, synced_at, content_hash, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    record.id,
                    record.project_id,
                    record.skill_id,
                    record.skill_name,
                    record.tool,
                    record.mode.as_str(),
                    record.status.as_str(),
                    record.last_error,
                    record.synced_at,
                    record.content_hash,
                    record.created_at
                ],
            )?;
            Ok(())
        })
    }

    pub fn list_project_skill_assignments(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectSkillAssignmentRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, project_id, skill_id, skill_name, tool, mode, status, last_error, synced_at, content_hash, created_at
                 FROM project_skill_assignments
                 WHERE project_id = ?1
                 ORDER BY tool ASC, created_at ASC",
            )?;
            let rows = stmt.query_map(params![project_id], |row| {
                let id: String = row.get(0)?;
                let (mode, status, last_error) =
                    read_lifecycle(&id, row.get(5)?, row.get(6)?, row.get(7)?);
                Ok(ProjectSkillAssignmentRecord {
                    id,
                    project_id: row.get(1)?,
                    skill_id: row.get(2)?,
                    skill_name: row.get(3)?,
                    tool: row.get(4)?,
                    mode,
                    status,
                    last_error,
                    synced_at: row.get(8)?,
                    content_hash: row.get(9)?,
                    created_at: row.get(10)?,
                })
            })?;

            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(items)
        })
    }

    pub fn remove_project_skill_assignment(
        &self,
        project_id: &str,
        skill_id: &str,
        tool: &str,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "DELETE FROM project_skill_assignments
                 WHERE project_id = ?1 AND skill_id = ?2 AND tool = ?3",
                params![project_id, skill_id, tool],
            )?;
            Ok(())
        })
    }

    /// Apply a typed lifecycle transition to one global target row.
    pub fn transition_skill_target(
        &self,
        target_id: &str,
        transition: TargetTransition<'_>,
    ) -> Result<()> {
        let (status, last_error, mode, target_path, synced_at) = match transition {
            TargetTransition::SyncCompleted {
                mode,
                target_path,
                synced_at,
            } => (
                SyncStatus::Synced,
                None,
                Some(mode),
                Some(target_path),
                Some(synced_at),
            ),
            TargetTransition::SyncFailed { error } => {
                (SyncStatus::Error, Some(error), None, None, None)
            }
        };
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE skill_targets
                 SET status = ?1, last_error = ?2,
                     mode = COALESCE(?3, mode),
                     target_path = COALESCE(?4, target_path),
                     synced_at = COALESCE(?5, synced_at)
                 WHERE id = ?6",
                params![
                    status.as_str(),
                    last_error,
                    mode.map(SyncMode::as_str),
                    target_path,
                    synced_at,
                    target_id
                ],
            )?;
            Ok(())
        })
    }

    /// Apply a typed lifecycle transition to one assignment row.
    pub fn transition_assignment(
        &self,
        assignment_id: &str,
        transition: AssignmentTransition<'_>,
    ) -> Result<()> {
        let (status, last_error, synced_at, mode, content_hash) = match transition {
            AssignmentTransition::SyncCompleted {
                mode,
                synced_at,
                content_hash,
            } => (
                SyncStatus::Synced,
                None,
                Some(synced_at),
                Some(mode),
                content_hash,
            ),
            AssignmentTransition::SyncFailed { error } => {
                (SyncStatus::Error, Some(error), None, None, None)
            }
            AssignmentTransition::Reconciled {
                status,
                content_hash,
            } => (status, None, None, None, content_hash),
        };
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE project_skill_assignments
                 SET status = ?1, last_error = ?2,
                     synced_at = COALESCE(?3, synced_at),
                     mode = COALESCE(?4, mode),
                     content_hash = ?5
                 WHERE id = ?6",
                params![
                    status.as_str(),
                    last_error,
                    synced_at,
                    mode.map(SyncMode::as_str),
                    content_hash,
                    assignment_id
                ],
            )?;
            Ok(())
        })
    }

    #[allow(dead_code)] // Used in Phase 2 (project_sync module)
    pub fn get_project_skill_assignment(
        &self,
        project_id: &str,
        skill_id: &str,
        tool: &str,
    ) -> Result<Option<ProjectSkillAssignmentRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, project_id, skill_id, skill_name, tool, mode, status, last_error,
                        synced_at, content_hash, created_at
                 FROM project_skill_assignments
                 WHERE project_id = ?1 AND skill_id = ?2 AND tool = ?3
                 LIMIT 1",
            )?;
            let mut rows = stmt.query(params![project_id, skill_id, tool])?;
            match rows.next()? {
                Some(row) => {
                    let id: String = row.get(0)?;
                    let (mode, status, last_error) =
                        read_lifecycle(&id, row.get(5)?, row.get(6)?, row.get(7)?);
                    Ok(Some(ProjectSkillAssignmentRecord {
                        id,
                        project_id: row.get(1)?,
                        skill_id: row.get(2)?,
                        skill_name: row.get(3)?,
                        tool: row.get(4)?,
                        mode,
                        status,
                        last_error,
                        synced_at: row.get(8)?,
                        content_hash: row.get(9)?,
                        created_at: row.get(10)?,
                    }))
                }
                None => Ok(None),
            }
        })
    }

    #[allow(dead_code)] // Used in Phase 2 (sync logic)
    pub fn list_project_skill_assignments_for_project_tool(
        &self,
        project_id: &str,
        tool: &str,
    ) -> Result<Vec<ProjectSkillAssignmentRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, project_id, skill_id, skill_name, tool, mode, status, last_error, synced_at, content_hash, created_at
                 FROM project_skill_assignments
                 WHERE project_id = ?1 AND tool = ?2
                 ORDER BY created_at ASC",
            )?;
            let rows = stmt.query_map(params![project_id, tool], |row| {
                let id: String = row.get(0)?;
                let (mode, status, last_error) =
                    read_lifecycle(&id, row.get(5)?, row.get(6)?, row.get(7)?);
                Ok(ProjectSkillAssignmentRecord {
                    id,
                    project_id: row.get(1)?,
                    skill_id: row.get(2)?,
                    skill_name: row.get(3)?,
                    tool: row.get(4)?,
                    mode,
                    status,
                    last_error,
                    synced_at: row.get(8)?,
                    content_hash: row.get(9)?,
                    created_at: row.get(10)?,
                })
            })?;

            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(items)
        })
    }

    /// Every project's counts and status fold, in two grouped queries — the
    /// cost is independent of the project count, so a listing of N projects
    /// never fans out to N×4 reads. Projects with no tools and no
    /// assignments are absent from the map; read them as
    /// [`ProjectAggregate::default`].
    pub fn project_aggregates(&self) -> Result<HashMap<String, ProjectAggregate>> {
        self.aggregates_for(None)
    }

    /// One project's counts and status fold — the same two queries, filtered.
    pub fn project_aggregate(&self, project_id: &str) -> Result<ProjectAggregate> {
        Ok(self
            .aggregates_for(Some(project_id))?
            .remove(project_id)
            .unwrap_or_default())
    }

    /// The one aggregation pass: tool counts, then assignment rows folded per
    /// project. Statuses pass through the same legacy seam as row reads (an
    /// unrecognised value counts as `Error`) and the fold is
    /// `sync_status::aggregate`, so the precedence rule stays in one place.
    fn aggregates_for(&self, only: Option<&str>) -> Result<HashMap<String, ProjectAggregate>> {
        self.with_conn(|conn| {
            let mut aggregates: HashMap<String, ProjectAggregate> = HashMap::new();

            let tools_sql = match only {
                Some(_) => {
                    "SELECT project_id, COUNT(*) FROM project_tools \
                     WHERE project_id = ?1 GROUP BY project_id"
                }
                None => "SELECT project_id, COUNT(*) FROM project_tools GROUP BY project_id",
            };
            let mut stmt = conn.prepare(tools_sql)?;
            let read_tools =
                |row: &rusqlite::Row<'_>| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?));
            let tool_rows = match only {
                Some(project_id) => stmt.query_map(params![project_id], read_tools)?,
                None => stmt.query_map([], read_tools)?,
            };
            for row in tool_rows {
                let (project_id, count) = row?;
                aggregates.entry(project_id).or_default().tool_count = count as usize;
            }

            let assignments_sql = match only {
                Some(_) => {
                    "SELECT project_id, skill_id, id, mode, status, last_error \
                     FROM project_skill_assignments WHERE project_id = ?1"
                }
                None => {
                    "SELECT project_id, skill_id, id, mode, status, last_error \
                     FROM project_skill_assignments"
                }
            };
            let mut stmt = conn.prepare(assignments_sql)?;
            let read_assignment = |row: &rusqlite::Row<'_>| {
                let project_id: String = row.get(0)?;
                let skill_id: String = row.get(1)?;
                let id: String = row.get(2)?;
                let (_, status, _) = read_lifecycle(&id, row.get(3)?, row.get(4)?, row.get(5)?);
                Ok((project_id, skill_id, status))
            };
            let assignment_rows = match only {
                Some(project_id) => stmt.query_map(params![project_id], read_assignment)?,
                None => stmt.query_map([], read_assignment)?,
            };

            let mut statuses: HashMap<String, Vec<SyncStatus>> = HashMap::new();
            let mut skills: HashMap<String, HashSet<String>> = HashMap::new();
            for row in assignment_rows {
                let (project_id, skill_id, status) = row?;
                aggregates
                    .entry(project_id.clone())
                    .or_default()
                    .assignment_count += 1;
                skills
                    .entry(project_id.clone())
                    .or_default()
                    .insert(skill_id);
                statuses.entry(project_id).or_default().push(status);
            }
            for (project_id, skill_ids) in skills {
                aggregates.entry(project_id).or_default().skill_count = skill_ids.len();
            }
            for (project_id, statuses) in statuses {
                aggregates.entry(project_id).or_default().sync_status = aggregate(statuses);
            }

            Ok(aggregates)
        })
    }

    pub fn hide_explore_skill(&self, source_url: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT OR IGNORE INTO hidden_explore_skills (source_url, hidden_at) VALUES (?1, strftime('%s', 'now'))",
                params![source_url],
            )?;
            Ok(())
        })
    }

    pub fn unhide_explore_skill(&self, source_url: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "DELETE FROM hidden_explore_skills WHERE source_url = ?1",
                params![source_url],
            )?;
            Ok(())
        })
    }

    pub fn list_hidden_explore_skills(&self) -> Result<Vec<String>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT source_url FROM hidden_explore_skills")?;
            let urls = stmt
                .query_map([], |row| row.get(0))?
                .collect::<std::result::Result<Vec<String>, _>>()?;
            Ok(urls)
        })
    }

    fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("failed to open db at {:?}", self.db_path))?;
        // Every call opens its own connection and several may run at once (the
        // Refresh acquire pool). Wait out a sibling's write window instead of
        // failing the caller with SQLITE_BUSY. Stated here rather than relying
        // on rusqlite's own default.
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        // Enforce foreign key constraints on every connection (rusqlite PRAGMA is per-connection).
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        f(&conn)
    }
}

pub fn default_db_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf> {
    let app_dir = app
        .path()
        .app_data_dir()
        .context("failed to resolve app data dir")?;
    std::fs::create_dir_all(&app_dir)
        .with_context(|| format!("failed to create app data dir {:?}", app_dir))?;
    Ok(app_dir.join(DB_FILE_NAME))
}

/// Adopt a pre-rename installation's database when this one has no skills yet.
///
/// `data_root` is the platform data directory that holds one subdirectory per
/// app identifier (the parent of this app's own data dir); the legacy
/// identifiers are probed under it. It is passed in rather than resolved here
/// so core reads no environment and a test can substitute a temp dir.
pub fn migrate_legacy_db_if_needed(data_root: &Path, target_db_path: &Path) -> Result<()> {
    if let Ok(true) = db_has_any_skills(target_db_path) {
        return Ok(());
    }

    let legacy_db_path = LEGACY_APP_IDENTIFIERS
        .iter()
        .map(|id| data_root.join(id).join(DB_FILE_NAME))
        .find(|path| path.exists());

    let Some(legacy_db_path) = legacy_db_path else {
        return Ok(());
    };

    if legacy_db_path == target_db_path {
        return Ok(());
    }

    if let Some(parent) = target_db_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create app data dir {:?}", parent))?;
    }

    if target_db_path.exists() {
        let backup = target_db_path.with_extension(format!(
            "bak-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        ));
        std::fs::rename(target_db_path, &backup).with_context(|| {
            format!(
                "failed to backup existing db {:?} -> {:?}",
                target_db_path, backup
            )
        })?;
    }

    std::fs::copy(&legacy_db_path, target_db_path).with_context(|| {
        format!(
            "failed to migrate legacy db {:?} -> {:?}",
            legacy_db_path, target_db_path
        )
    })?;

    Ok(())
}

fn db_has_any_skills(db_path: &Path) -> Result<bool> {
    if !db_path.exists() {
        return Ok(false);
    }

    let conn =
        Connection::open(db_path).with_context(|| format!("failed to open db at {:?}", db_path))?;
    let has_table: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='skills';",
        [],
        |row| row.get(0),
    )?;
    if has_table == 0 {
        return Ok(false);
    }

    let count: i64 = conn.query_row("SELECT COUNT(*) FROM skills;", [], |row| row.get(0))?;
    Ok(count > 0)
}

#[cfg(test)]
#[path = "tests/skill_store.rs"]
mod tests;
