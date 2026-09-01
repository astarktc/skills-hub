//! Managed-skill deletion: global tool targets, project-scope artifacts, the
//! central copy, and the DB record, in that order. The deletion counterpart
//! of `global_sync.rs` / `project_sync.rs`, with the same plan/execute split:
//! [`plan_skill_removal`] only reads the store, [`execute_skill_removal`]
//! touches the filesystem and reports per-target outcomes as data, and
//! [`remove_skill`] composes both behind the typed `DeleteCleanupFailed`.
//!
//! Ordering matters: `SkillStore` runs with `PRAGMA foreign_keys = ON` and
//! `project_skill_assignments.skill_id` is `ON DELETE CASCADE`, so the DB
//! delete erases the rows that locate project artifacts. Planning therefore
//! resolves every path *before* the record goes, and execution removes the
//! artifacts before `delete_skill`.
//!
//! Legacy-orphan rows (assignments whose skill row is already gone) are not
//! this module's concern: a skill that has no row has no `skill_id` to plan
//! from. Their cleanup stays in `project_ops::remove_tool_with_cleanup` /
//! `remove_project_with_cleanup`, which walk assignments from the project
//! side and can therefore still find them.

use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::core::{
    errors::SignalError,
    project_sync::resolve_project_sync_target,
    skill_store::{SkillRecord, SkillStore},
    sync_engine::remove_path_any,
    tool_adapters::adapter_by_key,
};

/// Where a removal target lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemovalScope {
    /// A global tool skills dir (from `skill_targets`).
    Global,
    /// A project-scope skills dir (from `project_skill_assignments`).
    Project { project_id: String },
}

/// One filesystem path the plan intends to remove.
#[derive(Debug, Clone)]
pub struct RemovalTarget {
    pub scope: RemovalScope,
    pub tool_key: String,
    pub path: PathBuf,
}

/// Everything the store knows about a skill that removal needs, resolved
/// before any row is deleted.
#[derive(Debug)]
pub struct RemovalPlan {
    /// `None` when the skill row is already gone (legacy shape); global
    /// targets are still swept, but there is no central path or record.
    pub skill: Option<SkillRecord>,
    pub targets: Vec<RemovalTarget>,
}

#[derive(Debug)]
pub enum RemovalTargetStatus {
    /// Removed, or already absent.
    Removed,
    /// `error` is the `{:#}` chain of the removal failure (diagnostic text).
    Failed { error: String },
}

#[derive(Debug)]
pub struct RemovalTargetOutcome {
    pub scope: RemovalScope,
    pub tool_key: String,
    pub path: PathBuf,
    pub status: RemovalTargetStatus,
}

#[derive(Debug)]
pub struct RemovalReport {
    pub skill_id: String,
    pub targets: Vec<RemovalTargetOutcome>,
    /// The central copy was deleted (false when there was no skill row or
    /// the directory was already absent).
    pub central_removed: bool,
    /// The `skills` row was deleted (cascading targets and assignments).
    pub record_deleted: bool,
}

impl RemovalReport {
    /// `"<path>: <error>"` per failed target — the payload of
    /// `SignalError::DeleteCleanupFailed`.
    pub fn failures(&self) -> Vec<String> {
        self.targets
            .iter()
            .filter_map(|t| match &t.status {
                RemovalTargetStatus::Failed { error } => {
                    Some(format!("{}: {}", t.path.display(), error))
                }
                RemovalTargetStatus::Removed => None,
            })
            .collect()
    }
}

/// One line per target plus the central/record outcome — diagnostic text
/// for logs, not user copy.
impl fmt::Display for RemovalReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "skill {}: central_removed={} record_deleted={}",
            self.skill_id, self.central_removed, self.record_deleted
        )?;
        for t in &self.targets {
            let scope = match &t.scope {
                RemovalScope::Global => "global".to_string(),
                RemovalScope::Project { project_id } => format!("project {}", project_id),
            };
            let status = match &t.status {
                RemovalTargetStatus::Removed => "removed".to_string(),
                RemovalTargetStatus::Failed { error } => format!("failed: {}", error),
            };
            write!(
                f,
                "\n  [{}] {} {} -> {}",
                scope,
                t.tool_key,
                t.path.display(),
                status
            )?;
        }
        Ok(())
    }
}

/// Assignment statuses that imply an artifact was written to the project.
/// (String literals stay until the status enum lands — see ticket 27.)
fn assignment_has_artifact(status: &str) -> bool {
    matches!(status, "synced" | "stale" | "error")
}

/// Read-only: resolve every path deletion will touch.
pub fn plan_skill_removal(store: &SkillStore, skill_id: &str) -> Result<RemovalPlan> {
    let skill = store.get_skill_by_id(skill_id)?;
    let mut targets: Vec<RemovalTarget> = Vec::new();

    for target in store.list_skill_targets(skill_id)? {
        targets.push(RemovalTarget {
            scope: RemovalScope::Global,
            tool_key: target.tool,
            path: PathBuf::from(target.target_path),
        });
    }

    if let Some(skill) = &skill {
        for assignment in store.list_project_skill_assignments_by_skill(skill_id)? {
            if !assignment_has_artifact(&assignment.status) {
                continue;
            }
            let Some(project) = store.get_project_by_id(&assignment.project_id)? else {
                continue;
            };
            let Some(adapter) = adapter_by_key(&assignment.tool) else {
                continue;
            };
            targets.push(RemovalTarget {
                scope: RemovalScope::Project {
                    project_id: assignment.project_id,
                },
                tool_key: assignment.tool,
                path: resolve_project_sync_target(Path::new(&project.path), &adapter, &skill.name),
            });
        }
    }

    Ok(RemovalPlan { skill, targets })
}

/// Execute a plan: remove each target (failures isolated per target), then
/// the central copy, then the DB record. A central-copy failure is a hard
/// error that leaves the record in place so the operation can be retried.
pub fn execute_skill_removal(
    store: &SkillStore,
    skill_id: &str,
    plan: RemovalPlan,
) -> Result<RemovalReport> {
    let mut targets = Vec::with_capacity(plan.targets.len());
    for target in plan.targets {
        let status = match remove_path_any(&target.path) {
            Ok(()) => RemovalTargetStatus::Removed,
            Err(err) => RemovalTargetStatus::Failed {
                error: format!("{:#}", err),
            },
        };
        targets.push(RemovalTargetOutcome {
            scope: target.scope,
            tool_key: target.tool_key,
            path: target.path,
            status,
        });
    }

    let mut central_removed = false;
    let mut record_deleted = false;
    if let Some(skill) = plan.skill {
        let central = Path::new(&skill.central_path);
        if central.exists() {
            std::fs::remove_dir_all(central)
                .with_context(|| format!("remove central copy {:?}", central))?;
            central_removed = true;
        }
        store.delete_skill(skill_id)?;
        record_deleted = true;
    }

    Ok(RemovalReport {
        skill_id: skill_id.to_string(),
        targets,
        central_removed,
        record_deleted,
    })
}

/// Delete a managed skill everywhere. The record is deleted even when some
/// targets could not be removed; those surface as the typed
/// `SignalError::DeleteCleanupFailed` so the frontend can show what is left.
pub fn remove_skill(store: &SkillStore, skill_id: &str) -> Result<RemovalReport> {
    let plan = plan_skill_removal(store, skill_id)?;
    let report = execute_skill_removal(store, skill_id, plan)?;
    let failures = report.failures();
    if !failures.is_empty() {
        anyhow::bail!(SignalError::DeleteCleanupFailed { failures });
    }
    Ok(report)
}

#[cfg(test)]
#[path = "tests/skill_removal.rs"]
mod tests;
