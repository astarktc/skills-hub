//! Artifact removal: taking Sync targets off disk and settling their rows.
//!
//! One module for every scope an operator can ask to remove — one Managed
//! skill, one skill × Tool pair, one Project, one Project × Tool pair, or
//! every global target — because "take the artifact off disk, then settle
//! the row that describes it" is one rule, not five. The deletion
//! counterpart of `global_sync.rs` / `project_sync.rs`, with the same
//! plan/execute/report split:
//!
//! * [`plan`] only reads the store: it resolves every path a scope touches
//!   and attaches the rows that describe each path. Tools sharing a skills
//!   directory share one artifact, so one [`RemovalTarget`] carries several
//!   [`RowRef`]s — the path is removed once and every member row is settled.
//! * [`execute_unlocked`] touches the filesystem with **one presence rule**
//!   and **one settlement rule**:
//!   - presence: `symlink_metadata` decides, so a broken symlink counts as
//!     present and is removed; an absent path is a successful removal.
//!   - settlement: on success every attached row is deleted; on failure
//!     every attached row is **kept** with Sync status `error` carrying the
//!     diagnostic, so the failure stays observable and the operator can
//!     retry (ADR-0002). Rows are never deleted blind.
//! * Per-target outcomes are report data ([`RemovalReport`]); only a store
//!   failure fails the whole operation.
//!
//! Ordering matters for the `Skill` scope: `SkillStore` runs with
//! `PRAGMA foreign_keys = ON` and both `skill_targets.skill_id` and
//! `project_skill_assignments.skill_id` are `ON DELETE CASCADE`, so deleting
//! the `skills` row erases the rows that locate the artifacts. Planning
//! therefore resolves every path *before* the record goes, and the central
//! copy plus the record are removed last — and only when every target
//! succeeded, so a failed target never loses the row that describes it.
//!
//! Legacy-orphan rows (assignments whose skill row is already gone) are
//! reachable through the `Project` scopes, which walk assignments from the
//! project side.
//!
//! Locking: the operator-facing entry points ([`remove_skill`],
//! [`unsync_skill_targets`], [`unsync_all_skill_targets`],
//! [`unsync_skill_from_tool`]) take the mutation guard; [`plan`] and
//! [`execute_unlocked`] are unlocked `pub(crate)` seams (see
//! `mutation_guard`).

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::core::{
    errors::SignalError,
    mutation_guard,
    project_sync::resolve_project_sync_target,
    skill_store::{AssignmentTransition, ProjectRecord, SkillRecord, SkillStore, TargetTransition},
    sync_engine::remove_path_any,
    tool_adapters::{adapter_by_key, adapters_sharing_skills_dir, is_installed_in},
};

/// What an operator asked to remove. Planning input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemovalScope {
    /// Every Sync target of one Managed skill — global target rows *and*
    /// project assignment rows — plus its central copy and its record.
    /// Deleting a Managed skill.
    Skill { skill_id: String },
    /// Every **global** Sync target of one Managed skill; project artifacts
    /// are untouched. "Uninstall this skill from tool directories".
    SkillGlobal { skill_id: String },
    /// One skill × Tool pair at global scope, expanded across the tool's
    /// shared skills dir group. Carries the operator's home because it is
    /// the only scope whose planning depends on Tool installedness; every
    /// other scope resolves its paths from stored rows.
    SkillTool {
        skill_id: String,
        tool_key: String,
        home: PathBuf,
    },
    /// Every Sync target of one Project. Planned by
    /// `project_ops::remove_project_with_cleanup`.
    Project { project_id: String },
    /// One Project × Tool pair. Planned by
    /// `project_ops::remove_tool_with_cleanup`.
    ProjectTool {
        project_id: String,
        tool_key: String,
    },
    /// One Project × skill × Tool triple — a single assignment row. Planned
    /// by `project_sync::unassign_and_cleanup`.
    ProjectSkillTool {
        project_id: String,
        skill_id: String,
        tool_key: String,
    },
    /// Every global Sync target of every Managed skill. "Uninstall
    /// everything from tool directories".
    EveryGlobalTarget,
}

/// Where a row lives — the two tables that record Sync targets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowRef {
    /// A `skill_targets` row (global scope).
    GlobalTarget {
        id: String,
        skill_id: String,
        tool: String,
    },
    /// A `project_skill_assignments` row (project scope).
    Assignment {
        id: String,
        project_id: String,
        skill_id: String,
        tool: String,
    },
}

impl RowRef {
    pub fn tool(&self) -> &str {
        match self {
            RowRef::GlobalTarget { tool, .. } | RowRef::Assignment { tool, .. } => tool,
        }
    }

    pub fn project_id(&self) -> Option<&str> {
        match self {
            RowRef::GlobalTarget { .. } => None,
            RowRef::Assignment { project_id, .. } => Some(project_id),
        }
    }
}

/// One filesystem path to remove, with every row that describes it. Tools
/// sharing a skills directory produce one target with several rows.
#[derive(Debug, Clone)]
pub struct RemovalTarget {
    pub path: PathBuf,
    pub rows: Vec<RowRef>,
}

/// Everything a scope resolves to, read before any row is deleted.
#[derive(Debug)]
pub struct RemovalPlan {
    pub scope: RemovalScope,
    pub targets: Vec<RemovalTarget>,
    /// Set only for [`RemovalScope::Skill`]: the record whose central copy
    /// and row are removed after every target succeeded. `None` when the
    /// skill row is already gone (legacy shape) or for any other scope.
    pub skill: Option<SkillRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemovalTargetStatus {
    /// Removed, or already absent.
    Removed,
    /// `error` is the `{:#}` chain of the removal failure (diagnostic text,
    /// not user copy). Every attached row was kept with status `error`.
    Failed { error: String },
}

#[derive(Debug, Clone)]
pub struct RemovalTargetOutcome {
    pub path: PathBuf,
    pub rows: Vec<RowRef>,
    pub status: RemovalTargetStatus,
}

#[derive(Debug)]
pub struct RemovalReport {
    pub scope: RemovalScope,
    pub targets: Vec<RemovalTargetOutcome>,
    /// The central copy was deleted (only ever true for the `Skill` scope).
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

    /// Rows whose artifact was removed (and whose row was therefore deleted).
    pub fn removed_rows(&self) -> usize {
        self.count_rows(true)
    }

    /// Rows kept with Sync status `error` because their artifact stayed.
    pub fn failed_rows(&self) -> usize {
        self.count_rows(false)
    }

    fn count_rows(&self, removed: bool) -> usize {
        self.targets
            .iter()
            .filter(|t| matches!(t.status, RemovalTargetStatus::Removed) == removed)
            .map(|t| t.rows.len())
            .sum()
    }
}

/// One line per target — diagnostic text for logs, not user copy.
impl fmt::Display for RemovalReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}: central_removed={} record_deleted={}",
            self.scope, self.central_removed, self.record_deleted
        )?;
        for target in &self.targets {
            let status = match &target.status {
                RemovalTargetStatus::Removed => "removed".to_string(),
                RemovalTargetStatus::Failed { error } => format!("failed: {}", error),
            };
            let tools: Vec<&str> = target.rows.iter().map(RowRef::tool).collect();
            write!(
                f,
                "\n  {} [{}] -> {}",
                target.path.display(),
                tools.join(", "),
                status
            )?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Planning
// ---------------------------------------------------------------------------

/// Read-only: resolve every path `scope` touches, deduped by path with the
/// rows describing each path attached. Every root a scope needs travels
/// inside the scope itself ([`RemovalScope::SkillTool`] carries the home
/// that decides Tool installedness), so planning takes no ambient roots.
/// Unlocked internal seam.
pub(crate) fn plan(store: &SkillStore, scope: &RemovalScope) -> Result<RemovalPlan> {
    let mut builder = TargetBuilder::default();
    let mut skill: Option<SkillRecord> = None;

    match scope {
        RemovalScope::Skill { skill_id } => {
            skill = store.get_skill_by_id(skill_id)?;
            for row in store.list_skill_targets(skill_id)? {
                builder.push_global(&row);
            }
            let assignments = store.list_project_skill_assignments_by_skill(skill_id)?;
            push_assignments(store, &mut builder, assignments)?;
        }
        RemovalScope::SkillGlobal { skill_id } => {
            for row in store.list_skill_targets(skill_id)? {
                builder.push_global(&row);
            }
        }
        RemovalScope::SkillTool {
            skill_id,
            tool_key,
            home,
        } => {
            let group_keys = global_group_keys(home, tool_key);
            if let Some(group_keys) = group_keys {
                for row in store.list_skill_targets(skill_id)? {
                    if group_keys.iter().any(|k| k == &row.tool) {
                        builder.push_global(&row);
                    }
                }
            }
        }
        RemovalScope::Project { project_id } => {
            let assignments = store.list_project_skill_assignments(project_id)?;
            push_assignments(store, &mut builder, assignments)?;
        }
        RemovalScope::ProjectTool {
            project_id,
            tool_key,
        } => {
            let assignments =
                store.list_project_skill_assignments_for_project_tool(project_id, tool_key)?;
            push_assignments(store, &mut builder, assignments)?;
        }
        RemovalScope::ProjectSkillTool {
            project_id,
            skill_id,
            tool_key,
        } => {
            let assignment = store.get_project_skill_assignment(project_id, skill_id, tool_key)?;
            push_assignments(store, &mut builder, assignment.into_iter().collect())?;
        }
        RemovalScope::EveryGlobalTarget => {
            for skill in store.list_skills()? {
                for row in store.list_skill_targets(&skill.id)? {
                    builder.push_global(&row);
                }
            }
        }
    }

    Ok(RemovalPlan {
        scope: scope.clone(),
        targets: builder.finish(),
        skill,
    })
}

/// The global tool keys one Tool's artifact is shared with: every tool
/// resolving to the same global skills dir. `None` means "nothing to do" —
/// no member of the group is installed for this operator, so the artifact is
/// not ours to touch (an uninstalled tool's directory is left alone). An
/// unknown key stands for itself: its rows still carry their own paths.
fn global_group_keys(home: &Path, tool_key: &str) -> Option<Vec<String>> {
    let Some(adapter) = adapter_by_key(tool_key) else {
        return Some(vec![tool_key.to_string()]);
    };
    let group = adapters_sharing_skills_dir(adapter);
    if !group.iter().any(|a| is_installed_in(home, a)) {
        return None;
    }
    Some(group.into_iter().map(|a| a.key().to_string()).collect())
}

/// Resolve each assignment row's project-scope artifact path. A row whose
/// project or tool can no longer be resolved has no locatable artifact, so
/// it is not planned (and not settled) — the project-side callers own that
/// case.
fn push_assignments(
    store: &SkillStore,
    builder: &mut TargetBuilder,
    assignments: Vec<crate::core::skill_store::ProjectSkillAssignmentRecord>,
) -> Result<()> {
    let mut projects: HashMap<String, Option<ProjectRecord>> = HashMap::new();
    for assignment in assignments {
        let project = match projects.get(&assignment.project_id) {
            Some(cached) => cached.clone(),
            None => {
                let looked_up = store.get_project_by_id(&assignment.project_id)?;
                projects.insert(assignment.project_id.clone(), looked_up.clone());
                looked_up
            }
        };
        let Some(project) = project else {
            log::warn!(
                "artifact removal: assignment {} names missing project {}",
                assignment.id,
                assignment.project_id
            );
            continue;
        };
        let Some(adapter) = adapter_by_key(&assignment.tool) else {
            log::warn!(
                "artifact removal: assignment {} names unknown tool {}",
                assignment.id,
                assignment.tool
            );
            continue;
        };
        // The stored skill name locates the artifact; the skill row is
        // consulted only when the column is empty (pre-V6 rows).
        let skill_name = if assignment.skill_name.is_empty() {
            match store.get_skill_by_id(&assignment.skill_id)? {
                Some(skill) => skill.name,
                None => {
                    log::warn!(
                        "artifact removal: assignment {} has no skill name to locate its artifact",
                        assignment.id
                    );
                    continue;
                }
            }
        } else {
            assignment.skill_name.clone()
        };
        let path = resolve_project_sync_target(Path::new(&project.path), adapter, &skill_name);
        builder.push(
            path,
            RowRef::Assignment {
                id: assignment.id,
                project_id: assignment.project_id,
                skill_id: assignment.skill_id,
                tool: assignment.tool,
            },
        );
    }
    Ok(())
}

/// Accumulates rows into targets deduped by path — a shared skills dir means
/// one artifact with several rows.
#[derive(Default)]
struct TargetBuilder {
    targets: Vec<RemovalTarget>,
}

impl TargetBuilder {
    fn push(&mut self, path: PathBuf, row: RowRef) {
        match self.targets.iter_mut().find(|t| t.path == path) {
            Some(target) => target.rows.push(row),
            None => self.targets.push(RemovalTarget {
                path,
                rows: vec![row],
            }),
        }
    }

    fn push_global(&mut self, row: &crate::core::skill_store::SkillTargetRecord) {
        self.push(
            PathBuf::from(&row.target_path),
            RowRef::GlobalTarget {
                id: row.id.clone(),
                skill_id: row.skill_id.clone(),
                tool: row.tool.clone(),
            },
        );
    }

    fn finish(self) -> Vec<RemovalTarget> {
        self.targets
    }
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

/// The one presence rule: absence is only `NotFound`. `symlink_metadata`
/// succeeding means present, so a broken symlink counts as present (and must
/// be removed); any *other* stat error (EACCES on a parent, EIO, ENOTDIR)
/// also counts as present, because it does not prove absence — removal is
/// attempted, fails, and the row is kept with Sync status `error` (ADR-0002)
/// rather than deleted blind.
fn is_present(path: &Path) -> bool {
    match path.symlink_metadata() {
        Ok(_) => true,
        Err(err) => err.kind() != std::io::ErrorKind::NotFound,
    }
}

/// Execute a plan. Each target is removed once and its rows settled: deleted
/// on success, kept with Sync status `error` on failure (ADR-0002). For the
/// `Skill` scope the central copy and the `skills` row follow, but only when
/// every target succeeded — a failure keeps the whole skill so the operator
/// can retry. Unlocked internal seam.
pub(crate) fn execute_unlocked(store: &SkillStore, plan: RemovalPlan) -> Result<RemovalReport> {
    let mut targets: Vec<RemovalTargetOutcome> = Vec::with_capacity(plan.targets.len());

    for target in plan.targets {
        let status = if is_present(&target.path) {
            match remove_path_any(&target.path) {
                Ok(()) => RemovalTargetStatus::Removed,
                Err(err) => RemovalTargetStatus::Failed {
                    error: format!("{:#}", err),
                },
            }
        } else {
            RemovalTargetStatus::Removed
        };

        for row in &target.rows {
            match &status {
                RemovalTargetStatus::Removed => delete_row(store, row)?,
                RemovalTargetStatus::Failed { error } => settle_row_as_error(store, row, error)?,
            }
        }

        targets.push(RemovalTargetOutcome {
            path: target.path,
            rows: target.rows,
            status,
        });
    }

    let any_failed = targets
        .iter()
        .any(|t| matches!(t.status, RemovalTargetStatus::Failed { .. }));

    let mut central_removed = false;
    let mut record_deleted = false;
    if let Some(skill) = plan.skill {
        if !any_failed {
            let central = Path::new(&skill.central_path);
            if central.exists() {
                std::fs::remove_dir_all(central)
                    .with_context(|| format!("remove central copy {:?}", central))?;
                central_removed = true;
            }
            store.delete_skill(&skill.id)?;
            record_deleted = true;
        }
    }

    Ok(RemovalReport {
        scope: plan.scope,
        targets,
        central_removed,
        record_deleted,
    })
}

fn delete_row(store: &SkillStore, row: &RowRef) -> Result<()> {
    match row {
        RowRef::GlobalTarget { skill_id, tool, .. } => store.delete_skill_target(skill_id, tool),
        RowRef::Assignment {
            project_id,
            skill_id,
            tool,
            ..
        } => store.remove_project_skill_assignment(project_id, skill_id, tool),
    }
}

fn settle_row_as_error(store: &SkillStore, row: &RowRef, error: &str) -> Result<()> {
    match row {
        RowRef::GlobalTarget { id, .. } => {
            store.transition_skill_target(id, TargetTransition::SyncFailed { error })
        }
        RowRef::Assignment { id, .. } => {
            store.transition_assignment(id, AssignmentTransition::SyncFailed { error })
        }
    }
}

// ---------------------------------------------------------------------------
// Entry points (each takes the mutation guard)
// ---------------------------------------------------------------------------

fn planned(store: &SkillStore, scope: RemovalScope) -> Result<RemovalReport> {
    let plan = plan(store, &scope)?;
    execute_unlocked(store, plan)
}

/// Delete a Managed skill everywhere: every global and project artifact,
/// then the central copy and the record.
///
/// When any artifact could not be removed, its row is kept with Sync status
/// `error`, the skill itself is kept (so the failure is retryable), and the
/// typed `SignalError::DeleteCleanupFailed` carries what is left behind.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
pub fn remove_skill(store: &SkillStore, skill_id: &str) -> Result<RemovalReport> {
    mutation_guard::serialized(|| {
        let report = planned(
            store,
            RemovalScope::Skill {
                skill_id: skill_id.to_string(),
            },
        )?;
        let failures = report.failures();
        if !failures.is_empty() {
            anyhow::bail!(SignalError::DeleteCleanupFailed { failures });
        }
        Ok(report)
    })
}

/// Remove every global Sync target of one Managed skill.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
pub fn unsync_skill_targets(store: &SkillStore, skill_id: &str) -> Result<RemovalReport> {
    mutation_guard::serialized(|| {
        planned(
            store,
            RemovalScope::SkillGlobal {
                skill_id: skill_id.to_string(),
            },
        )
    })
}

/// Remove every global Sync target of every Managed skill.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
pub fn unsync_all_skill_targets(store: &SkillStore) -> Result<RemovalReport> {
    mutation_guard::serialized(|| planned(store, RemovalScope::EveryGlobalTarget))
}

/// Remove one skill's global Sync target for one Tool, across the tools
/// sharing that Tool's skills directory.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
pub fn unsync_skill_from_tool(
    store: &SkillStore,
    home: &Path,
    skill_id: &str,
    tool_key: &str,
) -> Result<RemovalReport> {
    mutation_guard::serialized(|| {
        planned(
            store,
            RemovalScope::SkillTool {
                skill_id: skill_id.to_string(),
                tool_key: tool_key.to_string(),
                home: home.to_path_buf(),
            },
        )
    })
}

#[cfg(test)]
#[path = "tests/artifact_removal.rs"]
mod tests;
