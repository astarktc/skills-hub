use anyhow::{bail, Context, Result};
use serde::Serialize;
use specta::Type;
use std::path::Path;
use uuid::Uuid;

use super::environment::expand_home_path_in;
use super::errors::SignalError;
use super::gitignore::{self, IgnoreUpdateOptions};
use super::project_sync;
use super::skill_store::{ProjectRecord, ProjectToolRecord, SkillStore};
use super::sync_engine;
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

/// Best-effort removal of one project-scope synced artifact.
///
/// The single shape every cleanup path shares: resolve the tool's project
/// skills dir, then remove the entry named `skill_name`. Presence is decided
/// by `symlink_metadata`, not `exists`, because the usual orphan is a broken
/// symlink. An unknown tool key or an empty skill name resolves to nothing to
/// remove.
///
/// Returns whether an artifact was found and removed. Failures are logged and
/// reported as `false` rather than propagated — cleanup is best-effort by
/// design, so one stuck path must not block a tool or project removal.
fn remove_project_artifact(project_path: &str, tool: &str, skill_name: &str) -> bool {
    if skill_name.is_empty() {
        return false;
    }
    let Some(adapter) = tool_adapters::adapter_by_key(tool) else {
        return false;
    };
    let target =
        project_sync::resolve_project_sync_target(Path::new(project_path), adapter, skill_name);
    if target.symlink_metadata().is_err() {
        return false;
    }
    match sync_engine::remove_path_any(&target) {
        Ok(()) => true,
        Err(err) => {
            log::warn!("failed to remove project artifact {:?}: {:#}", target, err);
            false
        }
    }
}

pub fn project_name_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

pub fn to_project_dto(record: &ProjectRecord, store: &SkillStore) -> Result<ProjectDto> {
    let tool_count = store.count_project_tools(&record.id)?;
    let skill_count = store.count_project_unique_skills(&record.id)?;
    let assignment_count = store.count_project_assignments(&record.id)?;
    let sync_status = store.aggregate_project_sync_status(&record.id)?;
    Ok(ProjectDto {
        id: record.id.clone(),
        path: record.path.clone(),
        name: project_name_from_path(&record.path),
        created_at: record.created_at,
        updated_at: record.updated_at,
        tool_count,
        skill_count,
        assignment_count,
        sync_status,
        path_exists: std::path::Path::new(&record.path).is_dir(),
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

pub fn remove_tool_with_cleanup(store: &SkillStore, project_id: &str, tool: &str) -> Result<()> {
    let project = store.get_project_by_id(project_id)?.ok_or_else(|| {
        anyhow::anyhow!(SignalError::NotFound {
            kind: "project".to_string(),
            id: project_id.to_string(),
        })
    })?;

    let assignments = store.list_project_skill_assignments_for_project_tool(project_id, tool)?;

    for assignment in &assignments {
        match store.get_skill_by_id(&assignment.skill_id) {
            Ok(Some(skill)) => {
                if let Err(e) = project_sync::unassign_and_cleanup(store, &project, &skill, tool) {
                    log::warn!(
                        "remove_tool_with_cleanup: failed to unassign skill {} for tool {}: {:#}",
                        assignment.skill_id,
                        tool,
                        e
                    );
                }
            }
            Ok(None) => {
                // Skill record missing from DB -- orphaned assignment.
                // Do best-effort filesystem cleanup via adapter path resolution.
                log::warn!(
                    "remove_tool_with_cleanup: skill {} not found; cleaning up orphaned assignment for tool {}",
                    assignment.skill_id,
                    tool
                );
                // Use the stored skill name (not the UUID) for filesystem cleanup.
                remove_project_artifact(&project.path, tool, &assignment.skill_name);
                // Clean up the DB record directly
                if let Err(e) =
                    store.remove_project_skill_assignment(&project.id, &assignment.skill_id, tool)
                {
                    log::warn!("failed to remove orphaned assignment record: {:#}", e);
                }
            }
            Err(e) => {
                log::warn!(
                    "remove_tool_with_cleanup: error looking up skill {}: {:#}",
                    assignment.skill_id,
                    e
                );
                // Best-effort: clean up the assignment record to avoid orphaned rows
                if let Err(e2) =
                    store.remove_project_skill_assignment(&project.id, &assignment.skill_id, tool)
                {
                    log::warn!(
                        "failed to remove assignment record after lookup error: {:#}",
                        e2
                    );
                }
            }
        }
    }

    store.remove_project_tool(project_id, tool)?;
    Ok(())
}

/// Make `tools` the project's configured tool set, then (optionally) update
/// its ignore files. Owns the ordering the ignore writer depends on: patterns
/// are derived from the *persisted* tools, so tools are written first and the
/// managed block is rewritten afterwards — callers cannot get the sequence
/// wrong. Tools already configured keep their records; removed tools go
/// through [`remove_tool_with_cleanup`]. Unknown tool keys fail before any
/// write. Returns the resulting tool list.
pub fn configure_project_tools(
    store: &SkillStore,
    project_id: &str,
    tools: &[String],
    ignore: Option<IgnoreUpdateOptions>,
) -> Result<Vec<ProjectToolDto>> {
    store.get_project_by_id(project_id)?.ok_or_else(|| {
        anyhow::anyhow!(SignalError::NotFound {
            kind: "project".to_string(),
            id: project_id.to_string(),
        })
    })?;
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
    for record in &persisted {
        if !tools.contains(&record.tool) {
            remove_tool_with_cleanup(store, project_id, &record.tool)?;
        }
    }

    if let Some(options) = ignore {
        gitignore::update_for_project(store, project_id, options)?;
    }

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

pub fn remove_project_with_cleanup(store: &SkillStore, project_id: &str) -> Result<()> {
    let project = store.get_project_by_id(project_id)?.ok_or_else(|| {
        anyhow::anyhow!(SignalError::NotFound {
            kind: "project".to_string(),
            id: project_id.to_string(),
        })
    })?;

    let assignments = store.list_project_skill_assignments(project_id)?;

    for assignment in &assignments {
        if assignment.status.has_deployed_artifact() {
            match store.get_skill_by_id(&assignment.skill_id) {
                Ok(Some(skill)) => {
                    remove_project_artifact(&project.path, &assignment.tool, &skill.name);
                }
                Ok(None) => {
                    // Skill record gone: fall back to the stored skill name.
                    if !remove_project_artifact(
                        &project.path,
                        &assignment.tool,
                        &assignment.skill_name,
                    ) {
                        log::warn!(
                            "skill {} not found during project cleanup; \
                             orphaned symlink may remain in project {:?} for tool {}",
                            assignment.skill_id,
                            project.path,
                            assignment.tool
                        );
                    }
                }
                Err(e) => {
                    log::warn!(
                        "error looking up skill {} during project cleanup: {}",
                        assignment.skill_id,
                        e
                    );
                }
            }
        }
    }

    store.delete_project(project_id)?;
    Ok(())
}

pub fn list_project_dtos(store: &SkillStore) -> Result<Vec<ProjectDto>> {
    let records = store.list_projects()?;
    let mut dtos = Vec::with_capacity(records.len());
    for record in &records {
        dtos.push(to_project_dto(record, store)?);
    }
    Ok(dtos)
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
    let record = store.get_project_by_id(project_id)?.ok_or_else(|| {
        anyhow::anyhow!(SignalError::NotFound {
            kind: "project".to_string(),
            id: project_id.to_string(),
        })
    })?;
    to_project_dto(&record, store)
}

#[cfg(test)]
#[path = "tests/project_ops.rs"]
mod tests;
