use tauri::State;
use uuid::Uuid;

use crate::core::project_ops::{self, ProjectDto, ProjectSkillAssignmentDto, ProjectToolDto};
use crate::core::project_sync;
use crate::core::skill_store::{ProjectToolRecord, SkillStore};
use crate::SyncMutex;

use super::{expand_home_path, format_anyhow_error, now_ms};

#[tauri::command]
pub async fn register_project(
    store: State<'_, SkillStore>,
    path: String,
) -> Result<ProjectDto, String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        project_ops::register_project_path(&store, &path, now_ms(), expand_home_path)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(format_anyhow_error)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn remove_project(store: State<'_, SkillStore>, projectId: String) -> Result<(), String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        project_ops::remove_project_with_cleanup(&store, &projectId)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(format_anyhow_error)
}

#[tauri::command]
pub async fn list_projects(store: State<'_, SkillStore>) -> Result<Vec<ProjectDto>, String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || project_ops::list_project_dtos(&store))
        .await
        .map_err(|e| e.to_string())?
        .map_err(format_anyhow_error)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn add_project_tool(
    store: State<'_, SkillStore>,
    projectId: String,
    tool: String,
) -> Result<(), String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        // Validate that the project exists before inserting
        store
            .get_project_by_id(&projectId)?
            .ok_or_else(|| anyhow::anyhow!("project not found: {}", projectId))?;

        let record = ProjectToolRecord {
            id: Uuid::new_v4().to_string(),
            project_id: projectId,
            tool,
        };
        store.add_project_tool(&record)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(format_anyhow_error)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn remove_project_tool(
    store: State<'_, SkillStore>,
    projectId: String,
    tool: String,
) -> Result<(), String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || store.remove_project_tool(&projectId, &tool))
        .await
        .map_err(|e| e.to_string())?
        .map_err(format_anyhow_error)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn list_project_tools(
    store: State<'_, SkillStore>,
    projectId: String,
) -> Result<Vec<ProjectToolDto>, String> {
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
    .map_err(|e| e.to_string())?
    .map_err(format_anyhow_error)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn add_project_skill_assignment(
    store: State<'_, SkillStore>,
    sync_mutex: State<'_, SyncMutex>,
    projectId: String,
    skillId: String,
    tool: String,
) -> Result<ProjectSkillAssignmentDto, String> {
    let store = store.inner().clone();
    let mutex = sync_mutex.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let project = store
            .get_project_by_id(&projectId)?
            .ok_or_else(|| anyhow::anyhow!("project not found: {}", projectId))?;
        let skill = store
            .get_skill_by_id(&skillId)?
            .ok_or_else(|| anyhow::anyhow!("skill not found: {}", skillId))?;

        let _lock = mutex.0.lock().unwrap_or_else(|e| e.into_inner());
        let now = now_ms();
        let record = project_sync::assign_and_sync(&store, &project, &skill, &tool, now)?;

        Ok::<_, anyhow::Error>(ProjectSkillAssignmentDto {
            id: record.id,
            project_id: record.project_id,
            skill_id: record.skill_id,
            tool: record.tool,
            mode: record.mode,
            status: record.status,
            last_error: record.last_error,
            synced_at: record.synced_at,
            content_hash: record.content_hash,
            created_at: record.created_at,
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(format_anyhow_error)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn remove_project_skill_assignment(
    store: State<'_, SkillStore>,
    sync_mutex: State<'_, SyncMutex>,
    projectId: String,
    skillId: String,
    tool: String,
) -> Result<(), String> {
    let store = store.inner().clone();
    let mutex = sync_mutex.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let project = store
            .get_project_by_id(&projectId)?
            .ok_or_else(|| anyhow::anyhow!("project not found: {}", projectId))?;
        let skill = store
            .get_skill_by_id(&skillId)?
            .ok_or_else(|| anyhow::anyhow!("skill not found: {}", skillId))?;

        let _lock = mutex.0.lock().unwrap_or_else(|e| e.into_inner());
        project_sync::unassign_and_cleanup(&store, &project, &skill, &tool)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(format_anyhow_error)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn list_project_skill_assignments(
    store: State<'_, SkillStore>,
    projectId: String,
) -> Result<Vec<ProjectSkillAssignmentDto>, String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let records = store.list_project_skill_assignments(&projectId)?;
        Ok::<_, anyhow::Error>(
            records
                .into_iter()
                .map(|r| ProjectSkillAssignmentDto {
                    id: r.id,
                    project_id: r.project_id,
                    skill_id: r.skill_id,
                    tool: r.tool,
                    mode: r.mode,
                    status: r.status,
                    last_error: r.last_error,
                    synced_at: r.synced_at,
                    content_hash: r.content_hash,
                    created_at: r.created_at,
                })
                .collect(),
        )
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(format_anyhow_error)
}
