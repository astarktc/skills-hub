use specta::Type;
use tauri::State;

use crate::core::environment::home_dir;
use crate::core::errors::SignalError;
use crate::core::gitignore::{self, IgnoreUpdateOptions};
use crate::core::project_ops::{self, ProjectDto, ProjectSkillAssignmentDto, ProjectToolDto};
use crate::core::project_sync::{self, AssignTargetStatus};
use crate::core::skill_store::{ProjectSkillAssignmentRecord, SkillStore};
use crate::SyncMutex;

use super::{now_ms, CommandError};

#[tauri::command]
#[specta::specta]
pub async fn register_project(
    store: State<'_, SkillStore>,
    path: String,
) -> Result<ProjectDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let home = home_dir()?;
        project_ops::register_project_path(&store, &home, &path, now_ms())
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn remove_project(
    store: State<'_, SkillStore>,
    sync_mutex: State<'_, SyncMutex>,
    projectId: String,
) -> Result<(), CommandError> {
    let store = store.inner().clone();
    let mutex = sync_mutex.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _lock = mutex.0.lock().unwrap_or_else(|e| e.into_inner());
        project_ops::remove_project_with_cleanup(&store, &projectId)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[specta::specta]
pub async fn list_projects(store: State<'_, SkillStore>) -> Result<Vec<ProjectDto>, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || project_ops::list_project_dtos(&store))
        .await
        .map_err(CommandError::internal)?
        .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn update_project_path(
    store: State<'_, SkillStore>,
    projectId: String,
    path: String,
) -> Result<ProjectDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let home = home_dir()?;
        project_ops::update_project_path(&store, &home, &projectId, &path, now_ms())
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

/// Replace the project's configured tool set and, when `gitignore` is given,
/// update its ignore files afterwards. Core owns the ordering
/// (`project_ops::configure_project_tools`); returns the resulting tools.
#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn configure_project_tools(
    store: State<'_, SkillStore>,
    sync_mutex: State<'_, SyncMutex>,
    projectId: String,
    tools: Vec<String>,
    gitignore: Option<IgnoreUpdateOptions>,
) -> Result<Vec<ProjectToolDto>, CommandError> {
    let store = store.inner().clone();
    let mutex = sync_mutex.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _lock = mutex.0.lock().unwrap_or_else(|e| e.into_inner());
        project_ops::configure_project_tools(&store, &projectId, &tools, gitignore)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn list_project_tools(
    store: State<'_, SkillStore>,
    projectId: String,
) -> Result<Vec<ProjectToolDto>, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let records = store.list_project_tools(&projectId)?;
        Ok::<_, anyhow::Error>(
            records
                .into_iter()
                .map(|r| ProjectToolDto {
                    id: r.id,
                    project_id: r.project_id,
                    tool: r.tool,
                })
                .collect(),
        )
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn add_project_skill_assignment(
    store: State<'_, SkillStore>,
    sync_mutex: State<'_, SyncMutex>,
    projectId: String,
    skillId: String,
    tool: String,
) -> Result<ProjectSkillAssignmentDto, CommandError> {
    let store = store.inner().clone();
    let mutex = sync_mutex.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _lock = mutex.0.lock().unwrap_or_else(|e| e.into_inner());
        let record = project_sync::assign_skill_to_project_tool(
            &store,
            &projectId,
            &skillId,
            &tool,
            now_ms(),
        )?;
        Ok::<_, anyhow::Error>(to_assignment_dto(record))
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn remove_project_skill_assignment(
    store: State<'_, SkillStore>,
    sync_mutex: State<'_, SyncMutex>,
    projectId: String,
    skillId: String,
    tool: String,
) -> Result<(), CommandError> {
    let store = store.inner().clone();
    let mutex = sync_mutex.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let project = store.get_project_by_id(&projectId)?.ok_or_else(|| {
            anyhow::anyhow!(SignalError::NotFound {
                kind: "project".to_string(),
                id: projectId.clone(),
            })
        })?;
        let skill = store
            .get_skill_by_id(&skillId)?
            .ok_or_else(|| anyhow::anyhow!("skill not found: {}", skillId))?;

        let _lock = mutex.0.lock().unwrap_or_else(|e| e.into_inner());
        project_sync::unassign_and_cleanup(&store, &project, &skill, &tool)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

fn to_assignment_dto(record: ProjectSkillAssignmentRecord) -> ProjectSkillAssignmentDto {
    ProjectSkillAssignmentDto {
        id: record.id,
        project_id: record.project_id,
        skill_id: record.skill_id,
        skill_name: record.skill_name,
        tool: record.tool,
        mode: record.mode,
        status: record.status,
        last_error: record.last_error,
        synced_at: record.synced_at,
        content_hash: record.content_hash,
        created_at: record.created_at,
    }
}

#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn list_project_skill_assignments(
    store: State<'_, SkillStore>,
    projectId: String,
) -> Result<Vec<ProjectSkillAssignmentDto>, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let records = project_sync::list_assignments_with_staleness(&store, &projectId)?;
        Ok::<_, anyhow::Error>(records.into_iter().map(to_assignment_dto).collect())
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[derive(serde::Serialize, Clone, Type)]
pub struct ResyncSummaryDto {
    pub project_id: String,
    pub synced: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn resync_project(
    store: State<'_, SkillStore>,
    sync_mutex: State<'_, SyncMutex>,
    projectId: String,
) -> Result<ResyncSummaryDto, CommandError> {
    let store = store.inner().clone();
    let mutex = sync_mutex.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _lock = mutex.0.lock().unwrap_or_else(|e| e.into_inner());
        let now = now_ms();
        let summary = project_sync::resync_project(&store, &projectId, now)?;
        Ok::<_, anyhow::Error>(ResyncSummaryDto {
            project_id: summary.project_id,
            synced: summary.synced,
            failed: summary.failed,
            errors: summary.errors,
        })
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[specta::specta]
pub async fn resync_all_projects(
    store: State<'_, SkillStore>,
    sync_mutex: State<'_, SyncMutex>,
) -> Result<Vec<ResyncSummaryDto>, CommandError> {
    let store = store.inner().clone();
    let mutex = sync_mutex.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _lock = mutex.0.lock().unwrap_or_else(|e| e.into_inner());
        let now = now_ms();
        let summaries = project_sync::resync_all_projects(&store, now)?;
        Ok::<_, anyhow::Error>(
            summaries
                .into_iter()
                .map(|s| ResyncSummaryDto {
                    project_id: s.project_id,
                    synced: s.synced,
                    failed: s.failed,
                    errors: s.errors,
                })
                .collect(),
        )
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[derive(serde::Serialize, Clone, Type)]
pub struct BulkAssignResultDto {
    pub assigned: Vec<ProjectSkillAssignmentDto>,
    pub failed: Vec<BulkAssignErrorDto>,
}

#[derive(serde::Serialize, Clone, Type)]
pub struct BulkAssignErrorDto {
    pub tool: String,
    pub error: CommandError,
}

#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn bulk_assign_skill(
    store: State<'_, SkillStore>,
    sync_mutex: State<'_, SyncMutex>,
    projectId: String,
    skillId: String,
) -> Result<BulkAssignResultDto, CommandError> {
    let store = store.inner().clone();
    let mutex = sync_mutex.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _lock = mutex.0.lock().unwrap_or_else(|e| e.into_inner());
        let outcomes =
            project_sync::assign_skill_to_project_tools(&store, &projectId, &skillId, now_ms())?;

        // Already-assigned tools are silent on the wire (neither list).
        let mut result = BulkAssignResultDto {
            assigned: Vec::new(),
            failed: Vec::new(),
        };
        for outcome in outcomes {
            match outcome.status {
                AssignTargetStatus::Assigned { record } => {
                    result.assigned.push(to_assignment_dto(*record))
                }
                AssignTargetStatus::AlreadyAssigned => {}
                AssignTargetStatus::Failed { error } => result.failed.push(BulkAssignErrorDto {
                    tool: outcome.tool_key,
                    error: CommandError::from_anyhow(error),
                }),
            }
        }
        Ok::<_, anyhow::Error>(result)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn update_project_gitignore(
    store: State<'_, SkillStore>,
    projectId: String,
    addToGitignore: bool,
    addToExclude: bool,
) -> Result<(), CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        gitignore::update_for_project(
            &store,
            &projectId,
            IgnoreUpdateOptions {
                add_to_gitignore: addToGitignore,
                add_to_exclude: addToExclude,
            },
        )
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn get_project_gitignore_status(
    store: State<'_, SkillStore>,
    projectId: String,
) -> Result<GitignoreStatusDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        use std::path::Path;

        let project = store.get_project_by_id(&projectId)?.ok_or_else(|| {
            anyhow::anyhow!(SignalError::NotFound {
                kind: "project".to_string(),
                id: projectId.clone(),
            })
        })?;

        let status = crate::core::gitignore::project_ignore_status(Path::new(&project.path));

        Ok::<_, anyhow::Error>(GitignoreStatusDto {
            in_gitignore: status.in_gitignore,
            in_exclude: status.in_exclude,
        })
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[derive(serde::Serialize, Clone, Type)]
pub struct GitignoreStatusDto {
    pub in_gitignore: bool,
    pub in_exclude: bool,
}
