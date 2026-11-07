use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use tauri::Manager;

use super::errors::SignalError;
use super::placement::{fold_status, row_folder, RowState, ScopeKind};
use super::sync_status::{aggregate, ProjectSyncStatus, SyncMode, SyncStatus};

const DB_FILE_NAME: &str = "skills_hub.db";
const LEGACY_APP_IDENTIFIERS: &[&str] = &[
    "com.tauri.dev",
    "com.tauri.dev.skillshub",
    "com.qufei1993.skillshub",
];

// Schema versioning: bump when making changes and add a migration step.
const SCHEMA_VERSION: i32 = 12;

/// V11: Skill health — the last settled outcome of a skill-level operation
/// (`core::skill_health`): its kind, the tagged `CommandError` as JSON, and
/// when it settled. All three are NULL for a healthy skill.
const MIGRATION_V11: &str = "ALTER TABLE skills ADD COLUMN last_outcome_kind TEXT NULL;
ALTER TABLE skills ADD COLUMN last_outcome_error TEXT NULL;
ALTER TABLE skills ADD COLUMN last_outcome_at INTEGER NULL;";

/// V12: a global Deployment row records the Content identity it deployed (a
/// copy), as project assignments do since V5 — the evidence the occupant
/// check (`core::ownership`) verifies a copy against (ADR-0007).
const MIGRATION_V12_COLUMN: &str = "ALTER TABLE skill_targets ADD COLUMN content_hash TEXT NULL;";

/// V12, conservative upgrade (ADR-0007): a successful Disable gives its path
/// up from now on, but rows disabled before kept their claim (`synced_at`),
/// and a Keep answer over an existing row did too. Such a claim must never
/// let a removal reach whatever occupies the path later, so every `disabled`
/// row gives it up. A row is only ever `disabled` once its artifact is gone
/// (or was never there), so nothing of ours is orphaned by this.
const MIGRATION_V12_RELINQUISH: &str =
    "UPDATE skill_targets SET synced_at = NULL, content_hash = NULL
  WHERE status = 'disabled';
UPDATE project_skill_assignments SET synced_at = NULL, content_hash = NULL
  WHERE status = 'disabled';";

/// The redeploy's journal (`core::redeploy`): one entry per old artifact a
/// redeploy owes a removal of — written in the same transaction as the row
/// switch that superseded it, deleted once it is removed or reported left
/// behind. Additive (`CREATE TABLE IF NOT EXISTS`), like `skill_edits`.
const REDEPLOY_JOURNAL_DDL: &str = "CREATE TABLE IF NOT EXISTS redeploy_journal (
  id TEXT PRIMARY KEY,
  row_id TEXT NOT NULL,
  skill_id TEXT NOT NULL,
  tool TEXT NOT NULL,
  old_path TEXT NOT NULL,
  old_entry TEXT NOT NULL,
  old_content_hash TEXT NULL,
  new_path TEXT NOT NULL,
  new_entry TEXT NOT NULL,
  state TEXT NOT NULL,
  created_at INTEGER NOT NULL
);";

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

// V4: project tables for per-project skill distribution. No `BEGIN`/`COMMIT`
// of its own: `ensure_schema` runs every step inside one transaction.
const MIGRATION_V4: &str = r#"
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
"#;

#[derive(Clone, Copy, Debug)]
pub enum SkillEditKind {
    InvocationMode,
}

impl SkillEditKind {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::InvocationMode => "invocation_mode",
        }
    }
}

#[derive(Clone, Debug)]
pub struct SkillEditRecord {
    pub skill_id: String,
    pub kind: SkillEditKind,
    pub value: String,
    pub base_value: String,
    pub conflict: bool,
    pub applied_at: i64,
}

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
    /// The Tool an `imported` skill was found in — display-only history, never
    /// a source (see Provenance in `CONTEXT.md`). `None` for every other
    /// provenance.
    pub imported_from_tool: Option<String>,
}

/// The raw Skill health columns (`skills.last_outcome_*`); typed and parsed
/// by `core::skill_health`, which alone writes them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredOutcome {
    pub kind: String,
    /// The settled `CommandError`, serialised as its tagged wire JSON.
    pub error: String,
    pub at: i64,
}

#[derive(Clone, Debug)]
pub struct SkillTargetRecord {
    pub id: String,
    pub skill_id: String,
    pub tool: String,
    pub target_path: String,
    pub mode: SyncMode,
    pub status: SyncStatus,
    /// Diagnostic chain of the last failed sync/removal (see
    /// [`TargetTransition::SyncFailed`]); read back by tests and kept for
    /// parity with the stored column — no production reader yet.
    #[cfg_attr(not(test), allow(dead_code))]
    pub last_error: Option<String>,
    pub synced_at: Option<i64>,
    /// The Content identity this row deployed, recorded for a copy at every
    /// successful write (schema V12); `None` for a link, and for a row
    /// written before V12. The evidence `core::ownership` checks a copy
    /// against.
    pub content_hash: Option<String>,
}

/// A row's **claim** on its path, for both Sync-target tables and
/// independent of its lifecycle status: the app wrote the path through this
/// row (a sync of ours succeeded there, recorded as `synced_at` by every
/// success writer — global first sync, project assign/resync/Enable,
/// Propagation, relocation, Library move) and has not given it up since. A
/// claim is never ownership by itself (ADR-0007): what occupies the path now
/// is verified against the row's evidence when the app acts
/// (`core::ownership::check`). A row without a claim has no evidence at all, so
/// nothing at its path is ever its to remove or replace.
///
/// A row the app never materialised has no claim, whatever its status: the
/// pair-grain marker (`TargetTransition::DisabledRecorded`), the assignment
/// recorded `disabled` for a disabled skill, the "Keep existing" decline of
/// D9, the `error` row a failed first sync records
/// (`TargetTransition::FailedRecorded`) — and the same rows after a failed
/// first sync (Enable's `TARGET_EXISTS`) turned them `error`, or a reconcile
/// re-derived their status. A row gives its claim up when Disable removed its
/// artifact or found it already gone (`TargetTransition::Disabled`,
/// `AssignmentTransition::Disabled`), and when the operator kept another
/// folder in its way; a failed removal keeps it (ADR-0002). Unknown ownership
/// (a legacy or partially written row) is treated as none: at worst an
/// artifact of ours is left on disk, never someone else's deleted.
fn owns_artifact(synced_at: Option<i64>) -> bool {
    synced_at.is_some()
}

impl SkillTargetRecord {
    /// See [`owns_artifact`]: the row's claim on its path (wrote it, kept it).
    pub fn owns_artifact(&self) -> bool {
        owns_artifact(self.synced_at)
    }
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

/// One project deployment of a Managed skill as the library listing reads it:
/// the assignment row's Tool and lifecycle, with the project it belongs to.
/// Read for every skill at once by [`SkillStore::list_skill_project_deployments`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillProjectDeploymentRecord {
    pub skill_id: String,
    pub project_id: String,
    pub project_path: String,
    pub tool: String,
    pub status: SyncStatus,
    pub synced_at: Option<i64>,
}

impl ProjectSkillAssignmentRecord {
    /// See [`owns_artifact`]: the row's claim on its path (wrote it, kept it).
    pub fn owns_artifact(&self) -> bool {
        owns_artifact(self.synced_at)
    }
}

/// One redeploy journal entry (`core::redeploy` owns its meaning): the old
/// artifact a row switch superseded, by path and physical identity, with the
/// row that records the new place and what the old row recorded as deployed.
/// The identities are stored as `removal_fence::EntryId` JSON.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedeployJournalRecord {
    pub id: String,
    /// The row that records the new place now (it owned, or absorbed the row
    /// that owned, the old artifact).
    pub row_id: String,
    pub skill_id: String,
    /// The key of the row that owned the old artifact.
    pub tool: String,
    pub old_path: String,
    pub old_entry: String,
    /// The Content identity the old row recorded as deployed (a copy).
    pub old_content_hash: Option<String>,
    pub new_path: String,
    pub new_entry: String,
    /// `pending` (its removal is owed) or `edited` (an edited copy awaiting
    /// the operator's answer).
    pub state: String,
    pub created_at: i64,
}

/// What a redeploy writes to the row that records a Deployment's new place
/// (`SkillStore::switch_redeployed_rows`): every lifecycle column, its key and
/// its path.
#[derive(Clone, Copy, Debug)]
pub struct SwitchedRow<'a> {
    pub tool: &'a str,
    pub target_path: &'a str,
    pub mode: SyncMode,
    pub status: SyncStatus,
    pub last_error: Option<&'a str>,
    pub synced_at: Option<i64>,
    pub content_hash: Option<&'a str>,
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
    /// A sync just succeeded: records the mode actually used, where the
    /// artifact landed and the Content identity it deployed (`Some` for a
    /// copy only — a link follows the Library copy and cannot drift).
    SyncCompleted {
        mode: SyncMode,
        target_path: &'a str,
        synced_at: i64,
        content_hash: Option<&'a str>,
    },
    /// The insert-shaped sibling of [`SyncCompleted`](Self::SyncCompleted):
    /// a sync succeeded for a `(skill_id, tool)` pair that may have no row
    /// yet (a first global sync, or a re-sync after unsync). The row is
    /// created under the caller's `target_id` when absent; when one already
    /// exists it keeps its id and is settled exactly as `SyncCompleted`
    /// would settle it. Global sync records new targets through this arm so
    /// no caller hand-builds a `Synced` [`SkillTargetRecord`].
    Recorded {
        skill_id: &'a str,
        tool: &'a str,
        mode: SyncMode,
        target_path: &'a str,
        synced_at: i64,
        content_hash: Option<&'a str>,
    },
    /// A sync or removal failed; `error` is the diagnostic chain, and the row
    /// keeps its recorded mode, path, claim and deployed identity so the
    /// failure stays observable and what it deployed stays checkable
    /// (ADR-0002: a failed removal keeps ownership).
    SyncFailed { error: &'a str },
    /// The insert-shaped sibling of [`SyncFailed`](Self::SyncFailed): a
    /// first sync of a `(skill_id, tool)` pair failed to materialise (any
    /// failure but an unanswered `TARGET_EXISTS`, which is asked first), so
    /// the pair is recorded `error` with its diagnostic — symmetric with
    /// ADR-0002, so the strip shows the failure and Retry has a row. A new
    /// row gets `synced_at` NULL: it never materialised, so it owns nothing
    /// ([`owns_artifact`]). An existing row keeps its id, mode, path and
    /// `synced_at` and is settled exactly as `SyncFailed` would settle it.
    FailedRecorded {
        skill_id: &'a str,
        tool: &'a str,
        mode: SyncMode,
        target_path: &'a str,
        error: &'a str,
    },
    /// The row's artifact was removed at the operator's request, or found
    /// already gone, and the row is kept as the durable "not on this Tool"
    /// record (`Disabled`). The row gives its claim up (`synced_at` and the
    /// deployed identity cleared, ADR-0007): whatever appears at the path
    /// later is not its own. Mode and path are kept: where the target was.
    Disabled,
    /// The insert-shaped sibling of [`Disabled`](Self::Disabled): a pair
    /// with no row yet is recorded `Disabled` under the caller's
    /// `target_id` (an existing row keeps its id and only flips status), so
    /// a Tool the skill was never on can be switched off before any Deploy
    /// reaches it — or the operator kept the folder in a row's way (Keep,
    /// D12). `mode` and `target_path` are where the target *would* be. An
    /// existing row gives its claim up like [`Disabled`](Self::Disabled): the
    /// folder at its path is not its own.
    DisabledRecorded {
        skill_id: &'a str,
        tool: &'a str,
        mode: SyncMode,
        target_path: &'a str,
    },
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
    /// A sync or Artifact removal failed; `error` is the diagnostic chain.
    /// The claim and the deployed identity are kept: the occupant check
    /// (`core::ownership`) re-reads the target before anything acts on it, and
    /// a failed removal keeps ownership (ADR-0002).
    SyncFailed { error: &'a str },
    /// The assignment's artifact was removed by a skill-level Disable (or was
    /// already gone), or the operator kept the folder in its way (Keep, D12),
    /// and the row is kept as its record. The row gives its claim up
    /// (`synced_at` and the deployed identity cleared, ADR-0007).
    Disabled,
    /// Enable admits a `disabled` row to its first sync. A successful Disable
    /// already gave the row's claim up; this clears it again for a row
    /// disabled by an older app before the V12 upgrade ran, so a first sync
    /// that meets a folder in the way leaves a non-owner the overwrite ask
    /// can settle, and only a sync that succeeds makes the row an owner again.
    /// Status, mode and path stay until that sync settles the row. A row that
    /// is not `disabled` is left untouched: a failed removal keeps its
    /// ownership (ADR-0002).
    EnableAdmitted,
    /// A reconcile pass decided the row's true status (see
    /// `sync_status::next_status`). Mode, `synced_at` and the deployed
    /// identity are untouched — nothing was written to disk, and a stale copy
    /// is still the copy the row deployed.
    Reconciled { status: SyncStatus },
}

/// What a lifecycle transition writes to a row's claim on its path
/// (`synced_at`) and its deployed identity (`content_hash`) — the two
/// columns ownership is checked from (`core::ownership`), written together.
#[derive(Clone, Copy, Debug)]
enum ClaimWrite<'a> {
    /// A sync of ours just succeeded: the row claims its path, with what it
    /// deployed (`Some` for a copy only).
    Set {
        synced_at: i64,
        content_hash: Option<&'a str>,
    },
    /// Nothing written to disk, or a failure: the claim and evidence stay.
    Keep,
    /// The row gives its path up (Disable, Keep): neither claim nor evidence.
    GiveUp,
}

impl<'a> ClaimWrite<'a> {
    /// The `SET` fragment (appended after the transition's own columns),
    /// binding `synced_at` as `?{at}` and `content_hash` as `?{at + 1}`, and
    /// the two values to bind there.
    fn sql(self, at: usize) -> (String, Option<i64>, Option<&'a str>) {
        let hash = at + 1;
        match self {
            ClaimWrite::Set {
                synced_at,
                content_hash,
            } => (
                format!(", synced_at = ?{at}, content_hash = ?{hash}"),
                Some(synced_at),
                content_hash,
            ),
            ClaimWrite::Keep => (
                format!(
                    ", synced_at = COALESCE(?{at}, synced_at), \
                     content_hash = COALESCE(?{hash}, content_hash)"
                ),
                None,
                None,
            ),
            ClaimWrite::GiveUp => (
                format!(", synced_at = ?{at}, content_hash = ?{hash}"),
                None,
                None,
            ),
        }
    }
}

fn insert_journal_entry(conn: &Connection, entry: &RedeployJournalRecord) -> Result<()> {
    conn.execute(
        "INSERT INTO redeploy_journal (
           id, row_id, skill_id, tool, old_path, old_entry, old_content_hash,
           new_path, new_entry, state, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            entry.id,
            entry.row_id,
            entry.skill_id,
            entry.tool,
            entry.old_path,
            entry.old_entry,
            entry.old_content_hash,
            entry.new_path,
            entry.new_entry,
            entry.state,
            entry.created_at
        ],
    )?;
    Ok(())
}

/// Whether `table` already has a column named `column` (`PRAGMA table_info`).
fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table});"))?;
    let names = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for name in names {
        if name? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

impl SkillStore {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    #[allow(dead_code)]
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Create or upgrade the schema. Every DDL step and the `user_version`
    /// write run in **one transaction**: a crash or a failing step leaves the
    /// database exactly as it was, never half-upgraded with the old version
    /// recorded (which would replay a non-idempotent `ALTER TABLE` and fail
    /// every later launch).
    pub fn ensure_schema(&self) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute_batch("PRAGMA foreign_keys = ON;")?;
            enable_wal(conn, &self.db_path);

            let tx = conn.unchecked_transaction()?;
            let user_version: i32 = tx.query_row("PRAGMA user_version;", [], |row| row.get(0))?;
            if user_version == 0 {
                tx.execute_batch(SCHEMA_V1)?;
                // V2: add description column
                tx.execute_batch("ALTER TABLE skills ADD COLUMN description TEXT NULL;")?;
                // V3: add source_subpath column
                tx.execute_batch("ALTER TABLE skills ADD COLUMN source_subpath TEXT NULL;")?;
                // V4: project tables for per-project skill distribution
                // (DDL includes V5 content_hash column in project_skill_assignments)
                tx.execute_batch(MIGRATION_V4)?;
                // V7: hidden explore skills table
                tx.execute_batch(
                    "CREATE TABLE IF NOT EXISTS hidden_explore_skills (
                        source_url TEXT PRIMARY KEY,
                        hidden_at INTEGER NOT NULL
                    );",
                )?;
                // V9: the Tool an `imported` skill was found in
                tx.execute_batch(
                    "ALTER TABLE skills ADD COLUMN imported_from_tool TEXT NULL;",
                )?;
                // V10: skill-level Disabled marker
                tx.execute_batch("ALTER TABLE skills ADD COLUMN disabled_at INTEGER NULL;")?;
                // V11: Skill health (the stored last outcome)
                tx.execute_batch(MIGRATION_V11)?;
                // V12: the deployed Content identity of a global row
                tx.execute_batch(MIGRATION_V12_COLUMN)?;
                tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
            } else if user_version < SCHEMA_VERSION {
                // Incremental migrations
                if user_version < 2 {
                    tx.execute_batch("ALTER TABLE skills ADD COLUMN description TEXT NULL;")?;
                }
                if user_version < 3 {
                    tx.execute_batch("ALTER TABLE skills ADD COLUMN source_subpath TEXT NULL;")?;
                }
                if user_version < 4 {
                    tx.execute_batch(MIGRATION_V4)?;
                }
                if user_version < 5 {
                    tx.execute_batch(
                        "ALTER TABLE project_skill_assignments ADD COLUMN content_hash TEXT NULL;",
                    )?;
                }
                if user_version < 6 {
                    tx.execute_batch(
                        "ALTER TABLE project_skill_assignments ADD COLUMN skill_name TEXT NOT NULL DEFAULT '';",
                    )?;
                    // Backfill skill_name from the skills table for existing rows
                    tx.execute_batch(
                        "UPDATE project_skill_assignments SET skill_name = COALESCE(
                            (SELECT name FROM skills WHERE skills.id = project_skill_assignments.skill_id),
                            ''
                        ) WHERE skill_name = '';",
                    )?;
                }
                if user_version < 8 {
                    // Consolidate 9 .agents/skills tools into single agents_skills key.
                    // Must DELETE duplicates BEFORE UPDATE due to UNIQUE(project_id, tool) constraint.
                    tx.execute_batch(
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
                if user_version < 9 && !column_exists(&tx, "skills", "imported_from_tool")? {
                    // `imported` provenance: where the skill was found, as
                    // display-only history (ADR-0003). Guarded by presence so
                    // a database that gained the column before the version
                    // write joined this transaction still upgrades.
                    tx.execute_batch(
                        "ALTER TABLE skills ADD COLUMN imported_from_tool TEXT NULL;",
                    )?;
                }
                if user_version < 10 && !column_exists(&tx, "skills", "disabled_at")? {
                    // Skill-level Disabled (see **Disabled** in `CONTEXT.md`):
                    // when set, the skill is off everywhere until enabled.
                    // Presence-guarded like V9 so the step is idempotent.
                    tx.execute_batch("ALTER TABLE skills ADD COLUMN disabled_at INTEGER NULL;")?;
                }
                if user_version < 11 && !column_exists(&tx, "skills", "last_outcome_kind")? {
                    // Skill health (see **Skill health** in `CONTEXT.md`):
                    // the three columns land together, presence-guarded on
                    // the first like V9/V10.
                    tx.execute_batch(MIGRATION_V11)?;
                }
                if user_version < 12 {
                    // Ownership is evidence (ADR-0007): global rows record
                    // what they deployed, presence-guarded like V9–V11; a
                    // pre-V12 row recorded nothing, so it owns only a link
                    // resolving to its Library copy. Every disabled row gives
                    // its path up.
                    if !column_exists(&tx, "skill_targets", "content_hash")? {
                        tx.execute_batch(MIGRATION_V12_COLUMN)?;
                    }
                    tx.execute_batch(MIGRATION_V12_RELINQUISH)?;
                }
                tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
            } else if user_version > SCHEMA_VERSION {
                anyhow::bail!(
                    "database schema version {} is newer than app supports {}",
                    user_version,
                    SCHEMA_VERSION
                );
            }

            // Ensure V7 table exists (handles DBs that were created at V7 without this table)
            tx.execute_batch(
                "CREATE TABLE IF NOT EXISTS hidden_explore_skills (
                    source_url TEXT PRIMARY KEY,
                    hidden_at INTEGER NOT NULL
                );",
            )?;

            // Additive: the redeploy's journal (`core::redeploy`). No foreign
            // key: an entry outlives a deleted skill or row, and is then
            // reported left behind rather than silently dropped.
            tx.execute_batch(REDEPLOY_JOURNAL_DDL)?;

            // Additive schema repair, also run for pre-existing and migrated DBs.
            tx.execute_batch(
                "CREATE TABLE IF NOT EXISTS skill_edits (
                    skill_id TEXT NOT NULL,
                    kind TEXT NOT NULL,
                    value TEXT NOT NULL,
                    base_value TEXT NOT NULL,
                    conflict INTEGER NOT NULL DEFAULT 0,
                    applied_at INTEGER NOT NULL,
                    PRIMARY KEY (skill_id, kind),
                    FOREIGN KEY(skill_id) REFERENCES skills(id) ON DELETE CASCADE
                );",
            )?;
            tx.commit()?;
            Ok(())
        })
    }

    pub fn get_skill_edit(
        &self,
        skill_id: &str,
        kind: SkillEditKind,
    ) -> Result<Option<SkillEditRecord>> {
        self.with_conn(|conn| {
            use rusqlite::OptionalExtension;
            Ok(conn.query_row(
                "SELECT value, base_value, conflict, applied_at FROM skill_edits WHERE skill_id = ?1 AND kind = ?2",
                params![skill_id, kind.as_key()],
                |row| Ok(SkillEditRecord { skill_id: skill_id.to_string(), kind, value: row.get(0)?, base_value: row.get(1)?, conflict: row.get(2)?, applied_at: row.get(3)? }),
            ).optional()?)
        })
    }

    pub fn upsert_skill_edit(&self, edit: &SkillEditRecord) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO skill_edits (skill_id, kind, value, base_value, conflict, applied_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(skill_id, kind) DO UPDATE SET value=excluded.value, base_value=excluded.base_value, conflict=excluded.conflict, applied_at=excluded.applied_at",
                params![edit.skill_id, edit.kind.as_key(), edit.value, edit.base_value, edit.conflict, edit.applied_at],
            )?;
            Ok(())
        })
    }

    pub fn delete_skill_edit(&self, skill_id: &str, kind: SkillEditKind) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "DELETE FROM skill_edits WHERE skill_id = ?1 AND kind = ?2",
                params![skill_id, kind.as_key()],
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

    /// Raw settings-table adapter; see [`SkillStore::get_setting`]. Delete
    /// the rows `delete` and upsert the rows `set` in one transaction (no
    /// secret: a secret goes through [`Self::delete_setting_securely`]).
    pub(super) fn write_settings(&self, delete: &[&str], set: &[(&str, &str)]) -> Result<()> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            for key in delete {
                tx.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
            }
            for (key, value) in set {
                tx.execute(
                    "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    params![key, value],
                )?;
            }
            tx.commit()?;
            Ok(())
        })
    }

    /// Raw settings-table adapter; see [`SkillStore::get_setting`]. Read a
    /// row and delete it in one transaction: a value taken is taken once.
    pub(super) fn take_setting(&self, key: &str) -> Result<Option<String>> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            let value = tx
                .query_row(
                    "SELECT value FROM settings WHERE key = ?1",
                    params![key],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            if value.is_some() {
                tx.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
            }
            tx.commit()?;
            Ok(value)
        })
    }

    /// Delete a settings row that held a secret so its bytes leave the
    /// database file, not just the table. Deleting an absent key is a no-op
    /// (the compaction still runs).
    ///
    /// A plain `DELETE` only unlinks the row: its content stays in the freed
    /// page until reused, and earlier values of the row (previous upserts)
    /// may sit in freelist pages. So, on this one connection:
    /// `secure_delete = ON` zeroes the deleted content
    /// (<https://www.sqlite.org/pragma.html#pragma_secure_delete>), then
    /// `VACUUM` rebuilds the file so no freed page keeps a historical copy
    /// (<https://www.sqlite.org/lang_vacuum.html>). The store runs in WAL mode
    /// (`enable_wal`), so the WAL is then checkpointed and truncated too; a
    /// database still in the rollback journal has no WAL, and SQLite deletes
    /// its journal at commit.
    ///
    /// Limits: this cannot reach copies outside the live file — backups,
    /// APFS / Time Machine snapshots, other legacy-identifier database copies,
    /// or flash-storage remnants.
    ///
    /// Any failure is an error. When the DELETE succeeded but the compaction
    /// failed, the row is already gone (and its live bytes zeroed) — only
    /// historical copies may remain, until the retry below.
    ///
    /// Retry signal: the DELETE and a non-secret marker row
    /// `<key>.cleanup_pending` = `1` commit in one transaction, and the marker
    /// is removed only after the compaction succeeded. A compaction that fails
    /// (busy database, disk error, a WAL checkpoint that could not finish)
    /// leaves the marker, so [`Self::finish_pending_secure_cleanup`] — run at
    /// startup and by every token Save/Remove — completes it later even
    /// though the row is gone. Plaintext is never restored to obtain a retry.
    pub(super) fn delete_setting_securely(&self, key: &str) -> Result<()> {
        self.with_conn(|conn| {
            // Per connection; set before the transaction so it is in force
            // for the DELETE inside it.
            conn.pragma_update(None, "secure_delete", true)?;
            let tx = conn.unchecked_transaction()?;
            tx.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
            tx.execute(
                "INSERT INTO settings (key, value) VALUES (?1, '1')
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![format!("{key}{CLEANUP_PENDING_SUFFIX}")],
            )?;
            tx.commit()?;
            compact_and_clear_cleanup_markers(conn)
        })
        .with_context(|| format!("secure deletion of settings row {key:?} did not complete"))
    }

    /// Finish a secure deletion whose compaction failed earlier: when any
    /// `*.cleanup_pending` marker row exists (see
    /// [`Self::delete_setting_securely`]), compact the database and clear the
    /// markers. Returns whether a pending cleanup was completed; with no
    /// marker it does nothing. A failed compaction is an error and keeps the
    /// markers for the next attempt.
    pub fn finish_pending_secure_cleanup(&self) -> Result<bool> {
        self.with_conn(|conn| {
            let pending: i64 = conn.query_row(
                "SELECT COUNT(*) FROM settings WHERE key GLOB ?1",
                params![format!("*{CLEANUP_PENDING_SUFFIX}")],
                |row| row.get(0),
            )?;
            if pending == 0 {
                return Ok(false);
            }
            conn.pragma_update(None, "secure_delete", true)?;
            compact_and_clear_cleanup_markers(conn)?;
            Ok(true)
        })
        .context("pending secure cleanup of the settings table did not complete")
    }

    pub fn upsert_skill(&self, record: &SkillRecord) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO skills (
          id, name, description, source_type, source_ref, source_subpath, source_revision, central_path, content_hash,
          created_at, updated_at, last_sync_at, last_seen_at, status, imported_from_tool
        ) VALUES (
          ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
          ?10, ?11, ?12, ?13, ?14, ?15
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
          status = excluded.status,
          imported_from_tool = excluded.imported_from_tool",
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
                    record.status,
                    record.imported_from_tool
                ],
            )?;
            Ok(())
        })
    }

    /// Test fixture door: writes a global target row with any lifecycle
    /// columns. Production code never hand-builds a target row — it records
    /// and settles rows through [`Self::transition_skill_target`].
    #[cfg(test)]
    pub fn upsert_skill_target(&self, record: &SkillTargetRecord) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO skill_targets (
          id, skill_id, tool, target_path, mode, status, last_error, synced_at, content_hash
        ) VALUES (
          ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9
        )
        ON CONFLICT(skill_id, tool) DO UPDATE SET
          target_path = excluded.target_path,
          mode = excluded.mode,
          status = excluded.status,
          last_error = excluded.last_error,
          synced_at = excluded.synced_at,
          content_hash = excluded.content_hash",
                params![
                    record.id,
                    record.skill_id,
                    record.tool,
                    record.target_path,
                    record.mode.as_str(),
                    record.status.as_str(),
                    record.last_error,
                    record.synced_at,
                    record.content_hash
                ],
            )?;
            Ok(())
        })
    }

    pub fn list_skills(&self) -> Result<Vec<SkillRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
        "SELECT id, name, description, source_type, source_ref, source_subpath, source_revision, central_path, content_hash,
                created_at, updated_at, last_sync_at, last_seen_at, status, imported_from_tool
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
                    imported_from_tool: row.get(14)?,
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
                created_at, updated_at, last_sync_at, last_seen_at, status, imported_from_tool
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
                    imported_from_tool: row.get(14)?,
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

    /// Move Library folder's switch: in **one transaction**, point every
    /// moved skill (`(skill_id, new central_path)`) at its new copy and write
    /// the settings rows `settings` (`(key, value)` — each key is
    /// `core::settings`'s to spell). Either everything flips or nothing does.
    /// Raw adapter; `settings::switch_library` is its only caller.
    pub(super) fn switch_library_records(
        &self,
        moves: &[(String, String)],
        settings: &[(&str, &str)],
        updated_at: i64,
    ) -> Result<()> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            for (skill_id, central_path) in moves {
                let changed = tx.execute(
                    "UPDATE skills SET central_path = ?1, updated_at = ?2 WHERE id = ?3",
                    params![central_path, updated_at, skill_id],
                )?;
                if changed != 1 {
                    anyhow::bail!("skill {skill_id} vanished during the library switch");
                }
            }
            for (key, value) in settings {
                tx.execute(
                    "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    params![key, value],
                )?;
            }
            tx.commit()?;
            Ok(())
        })
    }

    pub fn list_skills_missing_description(&self) -> Result<Vec<SkillRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
        "SELECT id, name, description, source_type, source_ref, source_subpath, source_revision, central_path, content_hash,
                created_at, updated_at, last_sync_at, last_seen_at, status, imported_from_tool
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
                    imported_from_tool: row.get(14)?,
                })
            })?;
            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(items)
        })
    }

    /// When the skill was disabled at skill grain, or `None` when it is not
    /// (see **Disabled** in `CONTEXT.md`).
    ///
    /// The marker is deliberately **not** a [`SkillRecord`] field: every
    /// writer of the skills row goes through [`Self::upsert_skill`] with a
    /// whole record, so a finalize or re-point that rebuilt the record would
    /// silently re-enable the skill. Only `skill_disable` writes it, through
    /// [`Self::set_skill_disabled_at`]; readers ask here.
    pub fn skill_disabled_at(&self, skill_id: &str) -> Result<Option<i64>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT disabled_at FROM skills WHERE id = ?1")?;
            let mut rows = stmt.query(params![skill_id])?;
            match rows.next()? {
                Some(row) => Ok(row.get(0)?),
                None => Ok(None),
            }
        })
    }

    /// Set (`Some(now)`) or clear (`None`) the skill-level Disabled marker.
    pub fn set_skill_disabled_at(&self, skill_id: &str, at: Option<i64>) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE skills SET disabled_at = ?1 WHERE id = ?2",
                params![at, skill_id],
            )?;
            Ok(())
        })
    }

    /// Every skill disabled at skill grain, with its marker — one read for
    /// the catalog instead of one per skill.
    pub fn disabled_skills(&self) -> Result<HashMap<String, i64>> {
        self.with_conn(|conn| {
            let mut stmt =
                conn.prepare("SELECT id, disabled_at FROM skills WHERE disabled_at IS NOT NULL")?;
            let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
            let mut items = HashMap::new();
            for row in rows {
                let (id, at): (String, i64) = row?;
                items.insert(id, at);
            }
            Ok(items)
        })
    }

    /// The stored Skill health columns of one skill, raw (`kind`, `error`
    /// JSON, `at`); `None` when the skill is healthy or gone. Like
    /// `disabled_at` these are deliberately **not** [`SkillRecord`] fields,
    /// so a whole-record upsert (finalize, Re-point) never touches them: only
    /// `core::skill_health` writes them and parses what it reads.
    pub fn skill_outcome(&self, skill_id: &str) -> Result<Option<StoredOutcome>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT last_outcome_kind, last_outcome_error, last_outcome_at
                 FROM skills WHERE id = ?1 AND last_outcome_kind IS NOT NULL",
            )?;
            let mut rows = stmt.query(params![skill_id])?;
            match rows.next()? {
                Some(row) => Ok(Some(StoredOutcome {
                    kind: row.get(0)?,
                    error: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    at: row.get::<_, Option<i64>>(2)?.unwrap_or_default(),
                })),
                None => Ok(None),
            }
        })
    }

    /// Every stored Skill health, keyed by skill id — one read for the
    /// catalog instead of one per skill.
    pub fn skill_outcomes(&self) -> Result<HashMap<String, StoredOutcome>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, last_outcome_kind, last_outcome_error, last_outcome_at
                 FROM skills WHERE last_outcome_kind IS NOT NULL",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    StoredOutcome {
                        kind: row.get(1)?,
                        error: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                        at: row.get::<_, Option<i64>>(3)?.unwrap_or_default(),
                    },
                ))
            })?;
            let mut items = HashMap::new();
            for row in rows {
                let (id, outcome) = row?;
                items.insert(id, outcome);
            }
            Ok(items)
        })
    }

    /// Write (`Some`) or clear (`None`) the Skill health columns of one
    /// skill. A skill with no row is left alone (no-op).
    pub fn set_skill_outcome(&self, skill_id: &str, outcome: Option<&StoredOutcome>) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE skills
                 SET last_outcome_kind = ?1, last_outcome_error = ?2, last_outcome_at = ?3
                 WHERE id = ?4",
                params![
                    outcome.map(|o| o.kind.as_str()),
                    outcome.map(|o| o.error.as_str()),
                    outcome.map(|o| o.at),
                    skill_id
                ],
            )?;
            Ok(())
        })
    }

    /// Clear the Skill health columns only when the stored kind is one of
    /// `kinds` (a success clears its own family, never another's).
    pub fn clear_skill_outcome_of(&self, skill_id: &str, kinds: &[&str]) -> Result<()> {
        self.with_conn(|conn| {
            use rusqlite::OptionalExtension;
            let stored: Option<String> = conn
                .query_row(
                    "SELECT last_outcome_kind FROM skills WHERE id = ?1",
                    params![skill_id],
                    |row| row.get(0),
                )
                .optional()?
                .flatten();
            if stored.is_some_and(|kind| kinds.contains(&kind.as_str())) {
                conn.execute(
                    "UPDATE skills
                     SET last_outcome_kind = NULL, last_outcome_error = NULL, last_outcome_at = NULL
                     WHERE id = ?1",
                    params![skill_id],
                )?;
            }
            Ok(())
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
                "SELECT id, skill_id, tool, target_path, mode, status, last_error, synced_at, content_hash
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
                    content_hash: row.get(8)?,
                })
            })?;

            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(items)
        })
    }

    /// Every global target path the app **owns** ([`owns_artifact`]:
    /// `synced_at` set), with its Tool — what Onboarding discovery hides as
    /// already managed. A row that never materialised (a pair-grain marker,
    /// a "Keep existing" decline, a failed first sync's `error` row) owns
    /// nothing at its path, so an unmanaged folder sitting there stays
    /// discoverable.
    pub fn list_owned_skill_target_paths(&self) -> Result<Vec<(String, String)>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT tool, target_path
         FROM skill_targets
         WHERE synced_at IS NOT NULL",
            )?;
            let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;

            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(items)
        })
    }

    /// Read one global target row: the row a Propagation ask was raised for
    /// (`propagation::resolve_target_collisions`), and tests. The removal
    /// module plans from `list_skill_targets`.
    pub fn get_skill_target(
        &self,
        skill_id: &str,
        tool: &str,
    ) -> Result<Option<SkillTargetRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, skill_id, tool, target_path, mode, status, last_error, synced_at, content_hash
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
                    content_hash: row.get(8)?,
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

    /// Every project deployment of every Managed skill — or of `only` that
    /// skill — in **one** query (the assignment rows joined to their project),
    /// ordered by skill, project path, then Tool. The library listing groups
    /// them per skill; it never asks per skill.
    pub fn list_skill_project_deployments(
        &self,
        only: Option<&str>,
    ) -> Result<Vec<SkillProjectDeploymentRecord>> {
        self.with_conn(|conn| {
            let sql = match only {
                Some(_) => {
                    "SELECT a.id, a.skill_id, a.project_id, p.path, a.tool, a.mode, a.status, a.last_error, a.synced_at
                     FROM project_skill_assignments a JOIN projects p ON p.id = a.project_id
                     WHERE a.skill_id = ?1
                     ORDER BY a.skill_id ASC, p.path ASC, a.tool ASC"
                }
                None => {
                    "SELECT a.id, a.skill_id, a.project_id, p.path, a.tool, a.mode, a.status, a.last_error, a.synced_at
                     FROM project_skill_assignments a JOIN projects p ON p.id = a.project_id
                     ORDER BY a.skill_id ASC, p.path ASC, a.tool ASC"
                }
            };
            let mut stmt = conn.prepare(sql)?;
            let read = |row: &rusqlite::Row<'_>| {
                let id: String = row.get(0)?;
                let (_, status, _) = read_lifecycle(&id, row.get(5)?, row.get(6)?, row.get(7)?);
                Ok(SkillProjectDeploymentRecord {
                    skill_id: row.get(1)?,
                    project_id: row.get(2)?,
                    project_path: row.get(3)?,
                    tool: row.get(4)?,
                    status,
                    synced_at: row.get(8)?,
                })
            };
            let rows = match only {
                Some(skill_id) => stmt.query_map(params![skill_id], read)?,
                None => stmt.query_map([], read)?,
            };
            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(items)
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
    ///
    /// `target_id` names the row to settle; for the insert-shaped
    /// [`TargetTransition::Recorded`] it is the id a newly created row gets,
    /// and is ignored when a row for that `(skill_id, tool)` already exists.
    pub fn transition_skill_target(
        &self,
        target_id: &str,
        transition: TargetTransition<'_>,
    ) -> Result<()> {
        let (status, last_error, mode, target_path, claim) = match transition {
            TargetTransition::Recorded {
                skill_id,
                tool,
                mode,
                target_path,
                synced_at,
                content_hash,
            } => {
                return self.with_conn(|conn| {
                    conn.execute(
                        "INSERT INTO skill_targets (
                           id, skill_id, tool, target_path, mode, status, last_error, synced_at,
                           content_hash
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8)
                         ON CONFLICT(skill_id, tool) DO UPDATE SET
                           target_path = excluded.target_path,
                           mode = excluded.mode,
                           status = excluded.status,
                           last_error = excluded.last_error,
                           synced_at = excluded.synced_at,
                           content_hash = excluded.content_hash",
                        params![
                            target_id,
                            skill_id,
                            tool,
                            target_path,
                            mode.as_str(),
                            SyncStatus::Synced.as_str(),
                            synced_at,
                            content_hash
                        ],
                    )?;
                    Ok(())
                });
            }
            TargetTransition::SyncCompleted {
                mode,
                target_path,
                synced_at,
                content_hash,
            } => (
                SyncStatus::Synced,
                None,
                Some(mode),
                Some(target_path),
                ClaimWrite::Set {
                    synced_at,
                    content_hash,
                },
            ),
            TargetTransition::SyncFailed { error } => {
                (SyncStatus::Error, Some(error), None, None, ClaimWrite::Keep)
            }
            TargetTransition::FailedRecorded {
                skill_id,
                tool,
                mode,
                target_path,
                error,
            } => {
                return self.with_conn(|conn| {
                    conn.execute(
                        "INSERT INTO skill_targets (
                           id, skill_id, tool, target_path, mode, status, last_error, synced_at
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)
                         ON CONFLICT(skill_id, tool) DO UPDATE SET
                           status = excluded.status,
                           last_error = excluded.last_error",
                        params![
                            target_id,
                            skill_id,
                            tool,
                            target_path,
                            mode.as_str(),
                            SyncStatus::Error.as_str(),
                            error
                        ],
                    )?;
                    Ok(())
                });
            }
            TargetTransition::Disabled => {
                (SyncStatus::Disabled, None, None, None, ClaimWrite::GiveUp)
            }
            TargetTransition::DisabledRecorded {
                skill_id,
                tool,
                mode,
                target_path,
            } => {
                return self.with_conn(|conn| {
                    conn.execute(
                        "INSERT INTO skill_targets (
                           id, skill_id, tool, target_path, mode, status, last_error, synced_at
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, NULL)
                         ON CONFLICT(skill_id, tool) DO UPDATE SET
                           status = excluded.status,
                           last_error = NULL,
                           synced_at = NULL,
                           content_hash = NULL",
                        params![
                            target_id,
                            skill_id,
                            tool,
                            target_path,
                            mode.as_str(),
                            SyncStatus::Disabled.as_str()
                        ],
                    )?;
                    Ok(())
                });
            }
        };
        let (claim_sql, synced_at, content_hash) = claim.sql(6);
        self.with_conn(|conn| {
            conn.execute(
                &format!(
                    "UPDATE skill_targets
                     SET status = ?1, last_error = ?2,
                         mode = COALESCE(?3, mode),
                         target_path = COALESCE(?4, target_path){claim_sql}
                     WHERE id = ?5"
                ),
                params![
                    status.as_str(),
                    last_error,
                    mode.map(SyncMode::as_str),
                    target_path,
                    target_id,
                    synced_at,
                    content_hash
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
        let (status, last_error, mode, claim) = match transition {
            AssignmentTransition::SyncCompleted {
                mode,
                synced_at,
                content_hash,
            } => (
                SyncStatus::Synced,
                None,
                Some(mode),
                ClaimWrite::Set {
                    synced_at,
                    content_hash,
                },
            ),
            AssignmentTransition::SyncFailed { error } => {
                (SyncStatus::Error, Some(error), None, ClaimWrite::Keep)
            }
            AssignmentTransition::Disabled => {
                (SyncStatus::Disabled, None, None, ClaimWrite::GiveUp)
            }
            AssignmentTransition::EnableAdmitted => {
                return self.with_conn(|conn| {
                    conn.execute(
                        "UPDATE project_skill_assignments
                         SET synced_at = NULL, content_hash = NULL
                         WHERE id = ?1 AND status = ?2",
                        params![assignment_id, SyncStatus::Disabled.as_str()],
                    )?;
                    Ok(())
                });
            }
            AssignmentTransition::Reconciled { status } => (status, None, None, ClaimWrite::Keep),
        };
        let (claim_sql, synced_at, content_hash) = claim.sql(5);
        self.with_conn(|conn| {
            conn.execute(
                &format!(
                    "UPDATE project_skill_assignments
                     SET status = ?1, last_error = ?2,
                         mode = COALESCE(?3, mode){claim_sql}
                     WHERE id = ?4"
                ),
                params![
                    status.as_str(),
                    last_error,
                    mode.map(SyncMode::as_str),
                    assignment_id,
                    synced_at,
                    content_hash
                ],
            )?;
            Ok(())
        })
    }

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
                    "SELECT project_id, skill_id, id, mode, status, last_error, tool, synced_at \
                     FROM project_skill_assignments WHERE project_id = ?1"
                }
                None => {
                    "SELECT project_id, skill_id, id, mode, status, last_error, tool, synced_at \
                     FROM project_skill_assignments"
                }
            };
            let mut stmt = conn.prepare(assignments_sql)?;
            let read_assignment = |row: &rusqlite::Row<'_>| {
                let project_id: String = row.get(0)?;
                let skill_id: String = row.get(1)?;
                let id: String = row.get(2)?;
                let (_, status, _) = read_lifecycle(&id, row.get(3)?, row.get(4)?, row.get(5)?);
                let tool: String = row.get(6)?;
                let synced_at: Option<i64> = row.get(7)?;
                let state = RowState {
                    status,
                    owns_artifact: owns_artifact(synced_at),
                };
                Ok((project_id, skill_id, state, tool))
            };
            let assignment_rows = match only {
                Some(project_id) => stmt.query_map(params![project_id], read_assignment)?,
                None => stmt.query_map([], read_assignment)?,
            };

            let mut skills: HashMap<String, HashSet<String>> = HashMap::new();
            // One Deployment per (project, skill, folder): its rows — several
            // when a pre-1.4.0 app wrote one per shared-dir sibling — count
            // once, with the folder's folded facts (`placement::fold_status`,
            // the fold the project listing shows), so the project total never
            // disagrees with its matrix.
            let mut deployments: HashMap<(String, String, String), Vec<RowState>> = HashMap::new();
            for row in assignment_rows {
                let (project_id, skill_id, state, tool) = row?;
                let folder = row_folder(ScopeKind::Project, &tool);
                skills
                    .entry(project_id.clone())
                    .or_default()
                    .insert(skill_id.clone());
                deployments
                    .entry((project_id, skill_id, folder))
                    .or_default()
                    .push(state);
            }
            for (project_id, skill_ids) in skills {
                aggregates.entry(project_id).or_default().skill_count = skill_ids.len();
            }
            let mut statuses: HashMap<String, Vec<SyncStatus>> = HashMap::new();
            for ((project_id, _, _), rows) in deployments {
                aggregates
                    .entry(project_id.clone())
                    .or_default()
                    .assignment_count += 1;
                statuses
                    .entry(project_id)
                    .or_default()
                    .extend(fold_status(rows).map(|facts| facts.total_status()));
            }
            for (project_id, statuses) in statuses {
                aggregates.entry(project_id).or_default().sync_status = aggregate(statuses);
            }

            Ok(aggregates)
        })
    }

    /// Read one global target row by its id (the redeploy's journal names
    /// the row that records a Deployment's new place).
    pub fn get_skill_target_by_id(&self, id: &str) -> Result<Option<SkillTargetRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, skill_id, tool, target_path, mode, status, last_error, synced_at, content_hash
         FROM skill_targets
         WHERE id = ?1",
            )?;
            let mut rows = stmt.query(params![id])?;
            let Some(row) = rows.next()? else {
                return Ok(None);
            };
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
                content_hash: row.get(8)?,
            }))
        })
    }

    /// The redeploy's row switch (`core::redeploy`), in **one transaction**:
    /// delete the rows `absorbed` (collapsed into the survivor), rewrite the
    /// survivor `row_id` as `row` (its key, path and every lifecycle column),
    /// and write the `journal` entries owed for the old artifacts. A crash
    /// leaves either everything or nothing of it.
    pub fn switch_redeployed_rows(
        &self,
        row_id: &str,
        row: SwitchedRow<'_>,
        absorbed: &[&str],
        journal: &[RedeployJournalRecord],
    ) -> Result<()> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            for id in absorbed {
                tx.execute("DELETE FROM skill_targets WHERE id = ?1", params![id])?;
            }
            let updated = tx.execute(
                "UPDATE skill_targets
                 SET tool = ?1, target_path = ?2, mode = ?3, status = ?4, last_error = ?5,
                     synced_at = ?6, content_hash = ?7
                 WHERE id = ?8",
                params![
                    row.tool,
                    row.target_path,
                    row.mode.as_str(),
                    row.status.as_str(),
                    row.last_error,
                    row.synced_at,
                    row.content_hash,
                    row_id
                ],
            )?;
            if updated != 1 {
                anyhow::bail!("redeploy: target row {row_id} is gone");
            }
            for entry in journal {
                insert_journal_entry(&tx, entry)?;
            }
            tx.commit()?;
            Ok(())
        })
    }

    /// Every redeploy journal entry, oldest first.
    pub fn list_redeploy_journal(&self) -> Result<Vec<RedeployJournalRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, row_id, skill_id, tool, old_path, old_entry, old_content_hash,
                        new_path, new_entry, state, created_at
                 FROM redeploy_journal
                 ORDER BY created_at ASC, rowid ASC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(RedeployJournalRecord {
                    id: row.get(0)?,
                    row_id: row.get(1)?,
                    skill_id: row.get(2)?,
                    tool: row.get(3)?,
                    old_path: row.get(4)?,
                    old_entry: row.get(5)?,
                    old_content_hash: row.get(6)?,
                    new_path: row.get(7)?,
                    new_entry: row.get(8)?,
                    state: row.get(9)?,
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

    /// Drop a settled journal entry.
    pub fn delete_redeploy_journal_entry(&self, id: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM redeploy_journal WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    /// Set a journal entry's state (`pending` / `edited`).
    pub fn set_redeploy_journal_state(&self, id: &str, state: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE redeploy_journal SET state = ?1 WHERE id = ?2",
                params![state, id],
            )?;
            Ok(())
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
        // on rusqlite's own default. 30 s, not 5: every commit syncs the WAL
        // (`synchronous = FULL`, below), and on a slow Windows disk eight
        // concurrent writers' queue outlasted 5 s (1.3.3, hosted CI). A
        // waiter that gets its turn later is correct; one that fails is not.
        conn.busy_timeout(std::time::Duration::from_secs(30))?;
        // FULL, stated rather than inherited: every commit is synced to the
        // WAL before the call returns. The store is authoritative (Edit intent,
        // Disable state, artifact ownership), so an acknowledged write must
        // survive a power loss; NORMAL could roll it back (sqlite.org/pragma.html
        // #pragma_synchronous) and a last-close checkpoint does not cover a
        // commit made while another connection is open (1.3.3 L2 review).
        conn.execute_batch("PRAGMA synchronous = FULL;")?;
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

    let backup = target_db_path.with_extension(format!(
        "bak-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    ));
    // The existing file set (database + its sidecars) moves to the backup as
    // one unit. Normally `db_has_any_skills`' connection checkpointed and
    // removed the sidecars; when that read failed, nothing did. A WAL or hot
    // journal must never be split from its database (its commits would be
    // stranded) nor left beside the adopted copy (SQLite would replay it).
    backup_db_file_set(target_db_path, &backup)?;

    std::fs::copy(&legacy_db_path, target_db_path).with_context(|| {
        format!(
            "failed to migrate legacy db {:?} -> {:?}",
            legacy_db_path, target_db_path
        )
    })?;

    Ok(())
}

/// The files SQLite keeps beside a database (`<db>-wal`, its `-shm` index, a
/// rollback `-journal`). They belong to that one file and move with it.
const DB_SIDECAR_SUFFIXES: [&str; 3] = ["-wal", "-shm", "-journal"];

/// Move a database and its sidecars to `backup` as one unit, failure-safe
/// (1.3.3 L2 review): the whole set is first *copied* to the backup — a
/// failed copy deletes the partial backup and leaves the originals untouched
/// — then the originals are removed, sidecars before the database, and a
/// failed removal restores what was removed from the complete backup. A
/// crash at any point leaves either the originals whole or a complete backup;
/// never a database without its WAL, never an old sidecar without a backup.
fn backup_db_file_set(db: &Path, backup: &Path) -> Result<()> {
    let mut set = Vec::new();
    if std::fs::symlink_metadata(db).is_ok() {
        set.push((db.to_path_buf(), backup.to_path_buf()));
    }
    for suffix in DB_SIDECAR_SUFFIXES {
        let from = with_suffix(db, suffix);
        if std::fs::symlink_metadata(&from).is_ok() {
            set.push((from, with_suffix(backup, suffix)));
        }
    }

    for (index, (from, to)) in set.iter().enumerate() {
        if let Err(err) = std::fs::copy(from, to) {
            for (_, copied) in &set[..=index] {
                let _ = std::fs::remove_file(copied);
            }
            return Err(err)
                .with_context(|| format!("failed to backup existing db file {from:?} -> {to:?}"));
        }
    }

    // Sidecars first, the database last: until the database goes, nothing
    // else may sit at the target path.
    let mut removed: Vec<&(PathBuf, PathBuf)> = Vec::new();
    for pair in set.iter().rev() {
        if let Err(err) = std::fs::remove_file(&pair.0) {
            for (original, copy) in removed {
                let _ = std::fs::copy(copy, original);
            }
            return Err(err)
                .with_context(|| format!("failed to remove backed-up db file {:?}", pair.0));
        }
        removed.push(pair);
    }
    Ok(())
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// The store's journal policy (BACKLOG #104): write-ahead logging.
///
/// Under the rollback journal a reader's lock holds up a writer's commit and
/// a writer's lock holds up every reader, so concurrent connections (the
/// Refresh acquire pool, commands on blocking threads) take turns by
/// busy-polling, and on a slow disk (journal create + delete and several
/// fsyncs per commit) a call can lose every turn until its busy timeout. On
/// GitHub's hosted Windows runner four read + upsert workers came within
/// half a second of the 5 s timeout and eight failed "database is locked"
/// every run — even with a 30 s timeout. In WAL mode readers and the writer
/// never block each other and a commit is an append to the WAL, so only
/// writers queue, briefly: eight workers' slowest call stayed near 1 s there.
/// Every connection runs `synchronous = FULL` (see `with_conn`): one sync of
/// the WAL per commit, so an acknowledged write survives a power loss.
///
/// The mode is persistent, so this converts an existing database once
/// (sqlite.org/wal.html § Persistence); an older build opens it unchanged.
/// Failing to switch (a filesystem without shared-memory support, another
/// process holding the file) is logged and the store keeps working in its
/// current mode — contention-prone, never wrong.
fn enable_wal(conn: &Connection, db_path: &Path) {
    match conn.query_row("PRAGMA journal_mode = WAL;", [], |row| {
        row.get::<_, String>(0)
    }) {
        Ok(mode) if mode.eq_ignore_ascii_case("wal") => {}
        Ok(mode) => log::warn!("database {db_path:?} stays in journal mode {mode}, not WAL"),
        Err(err) => log::warn!("database {db_path:?} could not switch to WAL: {err:#}"),
    }
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

/// Suffix of the non-secret marker row that records an unfinished secure
/// deletion (`SkillStore::delete_setting_securely`).
const CLEANUP_PENDING_SUFFIX: &str = ".cleanup_pending";

/// Rebuild the file so no freed page keeps a deleted value (`VACUUM`), and
/// in WAL mode checkpoint and truncate the WAL — failing when the
/// checkpoint reports it could not finish (its result row's `busy` column is
/// `1`; that is not a query error). Only then delete every cleanup marker:
/// the markers hold no secret, so a plain delete suffices. Must run outside
/// a transaction (`VACUUM` refuses one).
fn compact_and_clear_cleanup_markers(conn: &Connection) -> Result<()> {
    #[cfg(test)]
    if compaction_fault::take() {
        anyhow::bail!("compaction fault injected by test");
    }
    conn.execute_batch("VACUUM;")?;
    let journal_mode: String = conn.query_row("PRAGMA journal_mode;", [], |row| row.get(0))?;
    if journal_mode.eq_ignore_ascii_case("wal") {
        let (busy, log, checkpointed): (i64, i64, i64) =
            conn.query_row("PRAGMA wal_checkpoint(TRUNCATE);", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?;
        if busy != 0 {
            anyhow::bail!(
                "WAL checkpoint could not complete (busy; {checkpointed} of {log} frames checkpointed)"
            );
        }
    }
    conn.execute(
        "DELETE FROM settings WHERE key GLOB ?1",
        params![format!("*{CLEANUP_PENDING_SUFFIX}")],
    )?;
    Ok(())
}

/// Test-only fault injection for the compaction step. A real busy reader
/// cannot fail `VACUUM` alone: in WAL mode it fails the checkpoint after it
/// (pinned on its own by `busy_wal_checkpoint_is_an_error_...`), and in
/// rollback-journal mode a reader holding its lock before the deletion blocks
/// the DELETE's commit instead. Per thread, so parallel tests do not see each
/// other's fault.
#[cfg(test)]
pub(crate) mod compaction_fault {
    use std::cell::Cell;

    thread_local! {
        static FAIL_NEXT: Cell<bool> = const { Cell::new(false) };
    }

    /// Make the next compaction on this thread fail.
    pub(crate) fn fail_next() {
        FAIL_NEXT.with(|flag| flag.set(true));
    }

    pub(super) fn take() -> bool {
        FAIL_NEXT.with(|flag| flag.replace(false))
    }
}

#[cfg(test)]
#[path = "tests/skill_store.rs"]
mod tests;
