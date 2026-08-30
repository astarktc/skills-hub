use tauri::State;
use ts_rs::TS;
use uuid::Uuid;

use crate::core::errors::SignalError;
use crate::core::project_ops::{self, ProjectDto, ProjectSkillAssignmentDto, ProjectToolDto};
use crate::core::project_sync;
use crate::core::skill_store::{ProjectToolRecord, SkillStore};
use crate::SyncMutex;

use super::{expand_home_path, now_ms, CommandError};

#[tauri::command]
pub async fn register_project(
    store: State<'_, SkillStore>,
    path: String,
) -> Result<ProjectDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        project_ops::register_project_path(&store, &path, now_ms(), expand_home_path)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
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
pub async fn list_projects(store: State<'_, SkillStore>) -> Result<Vec<ProjectDto>, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || project_ops::list_project_dtos(&store))
        .await
        .map_err(CommandError::internal)?
        .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn update_project_path(
    store: State<'_, SkillStore>,
    projectId: String,
    path: String,
) -> Result<ProjectDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        project_ops::update_project_path(&store, &projectId, &path, now_ms(), expand_home_path)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn add_project_tool(
    store: State<'_, SkillStore>,
    projectId: String,
    tool: String,
) -> Result<(), CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        // Validate that the project exists before inserting
        store.get_project_by_id(&projectId)?.ok_or_else(|| {
            anyhow::anyhow!(SignalError::NotFound {
                kind: "project".to_string(),
                id: projectId.clone(),
            })
        })?;

        // Validate that the tool key corresponds to a known tool adapter
        if crate::core::tool_adapters::adapter_by_key(&tool).is_none() {
            anyhow::bail!("unknown tool: {}", tool);
        }

        let record = ProjectToolRecord {
            id: Uuid::new_v4().to_string(),
            project_id: projectId,
            tool,
        };
        store.add_project_tool(&record)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn remove_project_tool(
    store: State<'_, SkillStore>,
    sync_mutex: State<'_, SyncMutex>,
    projectId: String,
    tool: String,
) -> Result<(), CommandError> {
    let store = store.inner().clone();
    let mutex = sync_mutex.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _lock = mutex.0.lock().unwrap_or_else(|e| e.into_inner());
        project_ops::remove_tool_with_cleanup(&store, &projectId, &tool)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
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
        let project = store.get_project_by_id(&projectId)?.ok_or_else(|| {
            anyhow::anyhow!(SignalError::NotFound {
                kind: "project".to_string(),
                id: projectId.clone(),
            })
        })?;
        let skill = store
            .get_skill_by_id(&skillId)?
            .ok_or_else(|| anyhow::anyhow!("skill not found: {}", skillId))?;

        // Check for existing assignment before attempting insert
        if store
            .get_project_skill_assignment(&projectId, &skillId, &tool)?
            .is_some()
        {
            anyhow::bail!(SignalError::AssignmentExists {
                project: projectId,
                skill: skillId,
                tool,
            });
        }

        let _lock = mutex.0.lock().unwrap_or_else(|e| e.into_inner());
        let now = now_ms();
        let record = project_sync::assign_and_sync(&store, &project, &skill, &tool, now)?;

        Ok::<_, anyhow::Error>(ProjectSkillAssignmentDto {
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
        })
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
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

#[tauri::command]
#[allow(non_snake_case)]
pub async fn list_project_skill_assignments(
    store: State<'_, SkillStore>,
    projectId: String,
) -> Result<Vec<ProjectSkillAssignmentDto>, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let records = project_sync::list_assignments_with_staleness(&store, &projectId)?;
        Ok::<_, anyhow::Error>(
            records
                .into_iter()
                .map(|r| ProjectSkillAssignmentDto {
                    id: r.id,
                    project_id: r.project_id,
                    skill_id: r.skill_id,
                    skill_name: r.skill_name,
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
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[derive(serde::Serialize, Clone, TS)]
#[ts(export)]
pub struct ResyncSummaryDto {
    pub project_id: String,
    pub synced: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

#[tauri::command]
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

#[derive(serde::Serialize, Clone, TS)]
#[ts(export)]
pub struct BulkAssignResultDto {
    pub assigned: Vec<ProjectSkillAssignmentDto>,
    pub failed: Vec<BulkAssignErrorDto>,
}

#[derive(serde::Serialize, Clone, TS)]
#[ts(export)]
pub struct BulkAssignErrorDto {
    pub tool: String,
    pub error: CommandError,
}

#[tauri::command]
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
        let project = store.get_project_by_id(&projectId)?.ok_or_else(|| {
            anyhow::anyhow!(SignalError::NotFound {
                kind: "project".to_string(),
                id: projectId.clone(),
            })
        })?;
        let skill = store.get_skill_by_id(&skillId)?.ok_or_else(|| {
            anyhow::anyhow!(SignalError::NotFound {
                kind: "skill".to_string(),
                id: skillId.clone(),
            })
        })?;
        let tools = store.list_project_tools(&projectId)?;

        let _lock = mutex.0.lock().unwrap_or_else(|e| e.into_inner());
        let now = now_ms();

        let mut assigned: Vec<ProjectSkillAssignmentDto> = Vec::new();
        let mut failed: Vec<BulkAssignErrorDto> = Vec::new();

        for tool_record in &tools {
            // Skip if already assigned
            if store
                .get_project_skill_assignment(&projectId, &skillId, &tool_record.tool)?
                .is_some()
            {
                continue;
            }
            match project_sync::assign_and_sync(&store, &project, &skill, &tool_record.tool, now) {
                Ok(record) => {
                    assigned.push(ProjectSkillAssignmentDto {
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
                    });
                }
                Err(e) => {
                    failed.push(BulkAssignErrorDto {
                        tool: tool_record.tool.clone(),
                        error: CommandError::from_anyhow(e),
                    });
                }
            }
        }

        Ok::<_, anyhow::Error>(BulkAssignResultDto { assigned, failed })
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn update_project_gitignore(
    store: State<'_, SkillStore>,
    projectId: String,
    addToGitignore: bool,
    addToExclude: bool,
) -> Result<(), CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        use std::path::Path;

        let project = store.get_project_by_id(&projectId)?.ok_or_else(|| {
            anyhow::anyhow!(SignalError::NotFound {
                kind: "project".to_string(),
                id: projectId.clone(),
            })
        })?;

        let project_path = Path::new(&project.path);
        if !project_path.is_dir() {
            anyhow::bail!("project directory does not exist: {}", project.path);
        }

        let tools = store.list_project_tools(&projectId)?;
        let adapters: Vec<_> = tools
            .iter()
            .filter_map(|t| crate::core::tool_adapters::adapter_by_key(&t.tool))
            .collect();
        let patterns = crate::core::gitignore::patterns_for_tools(adapters.iter());

        crate::core::gitignore::update_project_ignore_files(
            project_path,
            &patterns,
            addToGitignore,
            addToExclude,
        )
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_project_gitignore_status(
    store: State<'_, SkillStore>,
    projectId: String,
) -> Result<GitignoreStatusDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        use std::fs;
        use std::path::Path;

        let project = store.get_project_by_id(&projectId)?.ok_or_else(|| {
            anyhow::anyhow!(SignalError::NotFound {
                kind: "project".to_string(),
                id: projectId.clone(),
            })
        })?;

        let project_path = Path::new(&project.path);
        let marker = "# Skills Hub";

        let in_gitignore = {
            let p = project_path.join(".gitignore");
            p.exists() && fs::read_to_string(&p).unwrap_or_default().contains(marker)
        };

        let in_exclude = {
            let p = project_path.join(".git").join("info").join("exclude");
            p.exists() && fs::read_to_string(&p).unwrap_or_default().contains(marker)
        };

        Ok::<_, anyhow::Error>(GitignoreStatusDto {
            in_gitignore,
            in_exclude,
        })
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[derive(serde::Serialize, Clone, TS)]
#[ts(export)]
pub struct GitignoreStatusDto {
    pub in_gitignore: bool,
    pub in_exclude: bool,
}
