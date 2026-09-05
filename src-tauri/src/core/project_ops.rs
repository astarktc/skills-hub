use anyhow::{bail, Context, Result};
use serde::Serialize;
use specta::Type;
use std::path::Path;
use uuid::Uuid;

use super::artifact_removal::{self, RemovalReport, RemovalScope};
use super::environment::expand_home_path_in;
use super::errors::SignalError;
use super::gitignore::{self, IgnoreUpdateOptions};
use super::mutation_guard;
use super::project_sync::{self, AssignmentListing};
use super::skill_store::{ProjectAggregate, ProjectRecord, ProjectToolRecord, SkillStore};
use super::sync_status::{ProjectSyncStatus, SyncMode, SyncStatus};
use super::tool_adapters;

#[derive(Debug, Clone, Serialize, Type)]
pub struct ProjectDto {
    pub id: String,
    pub path: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub tool_count: usize,
    pub skill_count: usize,
    pub assignment_count: usize,
    pub sync_status: ProjectSyncStatus,
    pub path_exists: bool,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct ProjectToolDto {
    pub id: String,
    pub project_id: String,
    pub tool: String,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct ProjectSkillAssignmentDto {
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

pub fn project_name_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

/// The one "this project must exist" lookup. Every project operation and
/// every project command reaches for the record through here, so the typed
/// `NotFound` condition is raised in exactly one place.
pub fn require_project(store: &SkillStore, project_id: &str) -> Result<ProjectRecord> {
    store.get_project_by_id(project_id)?.ok_or_else(|| {
        anyhow::anyhow!(SignalError::NotFound {
            kind: "project".to_string(),
            id: project_id.to_string(),
        })
    })
}

pub fn to_project_dto(record: &ProjectRecord, store: &SkillStore) -> Result<ProjectDto> {
    Ok(project_dto_from_parts(
        record,
        store.project_aggregate(&record.id)?,
    ))
}

/// Compose the wire row from a record and its already-read aggregate, so the
/// listing can build N rows from one grouped read.
fn project_dto_from_parts(record: &ProjectRecord, aggregate: ProjectAggregate) -> ProjectDto {
    ProjectDto {
        id: record.id.clone(),
        path: record.path.clone(),
        name: project_name_from_path(&record.path),
        created_at: record.created_at,
        updated_at: record.updated_at,
        tool_count: aggregate.tool_count,
        skill_count: aggregate.skill_count,
        assignment_count: aggregate.assignment_count,
        sync_status: aggregate.sync_status,
        path_exists: std::path::Path::new(&record.path).is_dir(),
    }
}

/// A project's configured Tools as wire rows.
pub fn project_tool_dtos(store: &SkillStore, project_id: &str) -> Result<Vec<ProjectToolDto>> {
    Ok(store
        .list_project_tools(project_id)?
        .into_iter()
        .map(|r| ProjectToolDto {
            id: r.id,
            project_id: r.project_id,
            tool: r.tool,
        })
        .collect())
}

/// Everything the project world shows for one project: its row (counts and
/// aggregate status included), its configured Tools, and its reconciled
/// assignments.
///
/// Read-only, and deliberately **not** a mutation entry point: it must run
/// *after* a mutation released the guard, because the reconcile pass inside
/// [`project_sync::list_assignments_with_staleness`] try-locks. Building it
/// inside a critical section would always report `reconciled: false`.
#[derive(Debug)]
pub struct ProjectView {
    pub project: ProjectDto,
    pub tools: Vec<ProjectToolDto>,
    pub assignments: AssignmentListing,
}

pub fn project_view(store: &SkillStore, project_id: &str) -> Result<ProjectView> {
    let record = require_project(store, project_id)?;
    Ok(ProjectView {
        project: to_project_dto(&record, store)?,
        tools: project_tool_dtos(store, project_id)?,
        assignments: project_sync::list_assignments_with_staleness(store, project_id)?,
    })
}

/// Register `path` (may start with `~`, expanded against `home`) as a project.
pub fn register_project_path(
    store: &SkillStore,
    home: &Path,
    path: &str,
    now_ms: i64,
) -> Result<ProjectDto> {
    let expanded = expand_home_path_in(home, path)?;
    let canonical = std::fs::canonicalize(&expanded)
        .with_context(|| format!("failed to resolve path: {:?}", expanded))?;

    if !canonical.is_dir() {
        bail!(SignalError::InvalidPath {
            path: canonical.to_string_lossy().to_string(),
            reason: "not_a_directory".to_string(),
        });
    }

    let path_str = canonical.to_string_lossy().to_string();

    if store.get_project_by_path(&path_str)?.is_some() {
        bail!(SignalError::DuplicateProject { path: path_str });
    }

    let record = ProjectRecord {
        id: Uuid::new_v4().to_string(),
        path: path_str,
        created_at: now_ms,
        updated_at: now_ms,
    };
    store.register_project(&record)?;
    to_project_dto(&record, store)
}

/// Remove one configured Tool from a project: the
/// [`RemovalScope::ProjectTool`] plan, executed once, then this caller's
/// final policy — the project-tool row is removed **only** when every
/// artifact went. A tool whose artifact stayed keeps its row (and its `error`
/// assignment rows) so the operator can retry the same removal; the failures
/// are returned in the report for the caller to raise.
///
/// Rows the plan could not locate an artifact for (unknown tool key, missing
/// skill name) are left alone by the module; an unknown tool key cannot reach
/// here because [`configure_project_tools_unlocked`] rejects it first.
///
/// Unlocked internal seam: callers reach it through an entry point that has
/// already taken the mutation guard (`mutation_guard`).
pub(crate) fn remove_tool_with_cleanup(
    store: &SkillStore,
    project_id: &str,
    tool: &str,
) -> Result<RemovalReport> {
    require_project(store, project_id)?;

    let scope = RemovalScope::ProjectTool {
        project_id: project_id.to_string(),
        tool_key: tool.to_string(),
    };
    let plan = artifact_removal::plan(store, &scope)?;
    let report = artifact_removal::execute_unlocked(store, plan)?;

    if report.failures().is_empty() {
        store.remove_project_tool(project_id, tool)?;
    } else {
        log::warn!(
            "remove_tool_with_cleanup: keeping project tool row {}/{} for retry: {}",
            project_id,
            tool,
            report
        );
    }
    Ok(report)
}

/// Make `tools` the project's configured tool set, then (optionally) update
/// its ignore files. Owns the ordering the ignore writer depends on: patterns
/// are derived from the *persisted* tools, so tools are written first and the
/// managed block is rewritten afterwards — callers cannot get the sequence
/// wrong. Tools already configured keep their records; removed tools go
/// through [`remove_tool_with_cleanup`]. Unknown tool keys fail before any
/// write. Returns the resulting tool list.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
/// Both composed steps use their unlocked seams — the guard is not reentrant.
pub fn configure_project_tools(
    store: &SkillStore,
    project_id: &str,
    tools: &[String],
    ignore: Option<IgnoreUpdateOptions>,
) -> Result<Vec<ProjectToolDto>> {
    mutation_guard::serialized(|| {
        configure_project_tools_unlocked(store, project_id, tools, ignore)
    })
}

pub(crate) fn configure_project_tools_unlocked(
    store: &SkillStore,
    project_id: &str,
    tools: &[String],
    ignore: Option<IgnoreUpdateOptions>,
) -> Result<Vec<ProjectToolDto>> {
    require_project(store, project_id)?;
    for tool in tools {
        if tool_adapters::adapter_by_key(tool).is_none() {
            bail!(SignalError::UnknownTool { tool: tool.clone() });
        }
    }

    let persisted = store.list_project_tools(project_id)?;
    for tool in tools {
        if !persisted.iter().any(|record| &record.tool == tool) {
            store.add_project_tool(&ProjectToolRecord {
                id: Uuid::new_v4().to_string(),
                project_id: project_id.to_string(),
                tool: tool.clone(),
            })?;
        }
    }
    // Continue semantics: one tool whose artifacts could not be removed must
    // not stop the others (or the ignore update). Its failures are collected
    // and raised once, after the configuration is otherwise applied.
    let mut failures: Vec<String> = Vec::new();
    for record in &persisted {
        if !tools.contains(&record.tool) {
            failures.extend(remove_tool_with_cleanup(store, project_id, &record.tool)?.failures());
        }
    }

    if let Some(options) = ignore {
        gitignore::update_for_project_unlocked(store, project_id, options)?;
    }

    if !failures.is_empty() {
        bail!(SignalError::DeleteCleanupFailed { failures });
    }

    project_tool_dtos(store, project_id)
}

/// Artifact removal for a whole project, then the project record: the
/// [`RemovalScope::Project`] plan, executed once, then this caller's final
/// policy — mirroring the whole-skill rule of ADR-0002: the project row goes
/// only when every artifact went. If any removal failed, the project and its
/// `error` assignment rows are kept and the typed `DeleteCleanupFailed` names
/// what is still on disk, so a retry can re-plan exactly those paths.
///
/// Assignment rows the plan skipped (no locatable artifact: unknown tool key,
/// no skill name) are deleted with the project — `delete_project` cascades
/// `project_skill_assignments` — because there is nothing left to retry for
/// them.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
/// No composite operation removes a project, so it has no unlocked seam.
pub fn remove_project_with_cleanup(store: &SkillStore, project_id: &str) -> Result<()> {
    mutation_guard::serialized(|| {
        require_project(store, project_id)?;

        let scope = RemovalScope::Project {
            project_id: project_id.to_string(),
        };
        let plan = artifact_removal::plan(store, &scope)?;
        let report = artifact_removal::execute_unlocked(store, plan)?;

        let failures = report.failures();
        if !failures.is_empty() {
            bail!(SignalError::DeleteCleanupFailed { failures });
        }

        store.delete_project(project_id)?;
        Ok(())
    })
}

/// Every project as a wire row. Two grouped queries plus the project read,
/// whatever N is — the counts and status folds arrive from
/// [`SkillStore::project_aggregates`], never per project.
pub fn list_project_dtos(store: &SkillStore) -> Result<Vec<ProjectDto>> {
    let records = store.list_projects()?;
    let mut aggregates = store.project_aggregates()?;
    Ok(records
        .iter()
        .map(|record| {
            let aggregate = aggregates.remove(&record.id).unwrap_or_default();
            project_dto_from_parts(record, aggregate)
        })
        .collect())
}

/// Re-point a project at `new_path` (may start with `~`, expanded against `home`).
pub fn update_project_path(
    store: &SkillStore,
    home: &Path,
    project_id: &str,
    new_path: &str,
    now_ms: i64,
) -> Result<ProjectDto> {
    let expanded = expand_home_path_in(home, new_path)?;
    let canonical = std::fs::canonicalize(&expanded)
        .with_context(|| format!("failed to resolve path: {:?}", expanded))?;

    if !canonical.is_dir() {
        bail!(SignalError::InvalidPath {
            path: canonical.to_string_lossy().to_string(),
            reason: "not_a_directory".to_string(),
        });
    }

    let path_str = canonical.to_string_lossy().to_string();

    // Check for duplicates (different project using this path)
    if let Some(existing) = store.get_project_by_path(&path_str)? {
        if existing.id != project_id {
            bail!(SignalError::DuplicateProject { path: path_str });
        }
    }

    store.update_project_path(project_id, &path_str, now_ms)?;
    let record = require_project(store, project_id)?;
    to_project_dto(&record, store)
}

#[cfg(test)]
#[path = "tests/project_ops.rs"]
mod tests;
