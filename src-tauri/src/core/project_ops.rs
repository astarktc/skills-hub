use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::skill_store::{ProjectRecord, SkillStore};
use super::sync_engine;
use super::tool_adapters;

#[derive(Debug, Clone, Serialize)]
pub struct ProjectDto {
    pub id: String,
    pub path: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub tool_count: usize,
    pub assignment_count: usize,
    pub sync_status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectToolDto {
    pub id: String,
    pub project_id: String,
    pub tool: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectSkillAssignmentDto {
    pub id: String,
    pub project_id: String,
    pub skill_id: String,
    pub tool: String,
    pub mode: String,
    pub status: String,
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

pub fn to_project_dto(record: &ProjectRecord, store: &SkillStore) -> Result<ProjectDto> {
    let tool_count = store.count_project_tools(&record.id)?;
    let assignment_count = store.count_project_assignments(&record.id)?;
    let sync_status = store.aggregate_project_sync_status(&record.id)?;
    Ok(ProjectDto {
        id: record.id.clone(),
        path: record.path.clone(),
        name: project_name_from_path(&record.path),
        created_at: record.created_at,
        updated_at: record.updated_at,
        tool_count,
        assignment_count,
        sync_status,
    })
}

pub fn register_project_path(
    store: &SkillStore,
    path: &str,
    now_ms: i64,
    expand_home: impl Fn(&str) -> Result<PathBuf>,
) -> Result<ProjectDto> {
    let expanded = expand_home(path)?;
    let canonical = std::fs::canonicalize(&expanded)
        .with_context(|| format!("failed to resolve path: {:?}", expanded))?;

    if !canonical.is_dir() {
        bail!("path is not a directory: {:?}", canonical);
    }

    let path_str = canonical.to_string_lossy().to_string();

    if store.get_project_by_path(&path_str)?.is_some() {
        bail!("DUPLICATE_PROJECT|{}", path_str);
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

pub fn remove_project_with_cleanup(store: &SkillStore, project_id: &str) -> Result<()> {
    let project = store
        .get_project_by_id(project_id)?
        .ok_or_else(|| anyhow::anyhow!("project not found: {}", project_id))?;

    let assignments = store.list_project_skill_assignments(project_id)?;

    for assignment in &assignments {
        if assignment.status == "synced" || assignment.status == "stale" {
            match store.get_skill_by_id(&assignment.skill_id) {
                Ok(Some(skill)) => {
                    if let Some(adapter) = tool_adapters::adapter_by_key(&assignment.tool) {
                        let project_path = Path::new(&project.path);
                        let target = project_path
                            .join(adapter.relative_skills_dir)
                            .join(&skill.name);
                        if let Err(e) = sync_engine::remove_path_any(&target) {
                            log::warn!("failed to remove {:?}: {}", target, e);
                        }
                    }
                }
                Ok(None) => {
                    log::warn!(
                        "skill {} not found during project cleanup; \
                         orphaned symlink may remain in project {:?} for tool {}",
                        assignment.skill_id,
                        project.path,
                        assignment.tool
                    );
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

#[cfg(test)]
#[path = "tests/project_ops.rs"]
mod tests;
