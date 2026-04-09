use anyhow::Context;
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
pub async fn remove_project(
    store: State<'_, SkillStore>,
    sync_mutex: State<'_, SyncMutex>,
    projectId: String,
) -> Result<(), String> {
    let store = store.inner().clone();
    let mutex = sync_mutex.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _lock = mutex.0.lock().unwrap_or_else(|e| e.into_inner());
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
pub async fn update_project_path(
    store: State<'_, SkillStore>,
    projectId: String,
    path: String,
) -> Result<ProjectDto, String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        project_ops::update_project_path(&store, &projectId, &path, now_ms(), expand_home_path)
    })
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

        // Check for existing assignment before attempting insert
        if store
            .get_project_skill_assignment(&projectId, &skillId, &tool)?
            .is_some()
        {
            anyhow::bail!("ASSIGNMENT_EXISTS|{}:{}:{}", projectId, skillId, tool);
        }

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
        let records = project_sync::list_assignments_with_staleness(&store, &projectId)?;
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

#[derive(serde::Serialize, Clone)]
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
) -> Result<ResyncSummaryDto, String> {
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
    .map_err(|e| format!("{}", e))?
    .map_err(format_anyhow_error)
}

#[tauri::command]
pub async fn resync_all_projects(
    store: State<'_, SkillStore>,
    sync_mutex: State<'_, SyncMutex>,
) -> Result<Vec<ResyncSummaryDto>, String> {
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
    .map_err(|e| format!("{}", e))?
    .map_err(format_anyhow_error)
}

#[derive(serde::Serialize, Clone)]
pub struct BulkAssignResultDto {
    pub assigned: Vec<ProjectSkillAssignmentDto>,
    pub failed: Vec<BulkAssignErrorDto>,
}

#[derive(serde::Serialize, Clone)]
pub struct BulkAssignErrorDto {
    pub tool: String,
    pub error: String,
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn bulk_assign_skill(
    store: State<'_, SkillStore>,
    sync_mutex: State<'_, SyncMutex>,
    projectId: String,
    skillId: String,
) -> Result<BulkAssignResultDto, String> {
    let store = store.inner().clone();
    let mutex = sync_mutex.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let project = store
            .get_project_by_id(&projectId)?
            .ok_or_else(|| anyhow::anyhow!("NOT_FOUND|project:{}", projectId))?;
        let skill = store
            .get_skill_by_id(&skillId)?
            .ok_or_else(|| anyhow::anyhow!("NOT_FOUND|skill:{}", skillId))?;
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
                        error: format!("{:#}", e),
                    });
                }
            }
        }

        Ok::<_, anyhow::Error>(BulkAssignResultDto { assigned, failed })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(format_anyhow_error)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn update_project_gitignore(
    store: State<'_, SkillStore>,
    projectId: String,
    addToGitignore: bool,
    addToExclude: bool,
) -> Result<(), String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        use std::fs;
        use std::path::Path;

        let project = store
            .get_project_by_id(&projectId)?
            .ok_or_else(|| anyhow::anyhow!("project not found: {}", projectId))?;

        let project_path = Path::new(&project.path);
        if !project_path.is_dir() {
            anyhow::bail!("project directory does not exist: {}", project.path);
        }

        // Collect unique relative_skills_dir patterns from the project's configured tools.
        let tools = store.list_project_tools(&projectId)?;
        let mut patterns: Vec<String> = Vec::new();
        for tool_record in &tools {
            if let Some(adapter) = crate::core::tool_adapters::adapter_by_key(&tool_record.tool) {
                let pattern = format!("/{}/", adapter.relative_skills_dir);
                if !patterns.contains(&pattern) {
                    patterns.push(pattern);
                }
            }
        }

        if patterns.is_empty() {
            return Ok(());
        }

        let gitignore_block = format!(
            "\n# Skills Hub — managed skill directories\n{}\n",
            patterns.join("\n")
        );

        let marker = "# Skills Hub";

        // Helper: remove the Skills Hub block from content.
        // The block is: an optional preceding blank line, the marker comment line,
        // and all immediately following lines that start with '/' (our gitignore
        // patterns). Stops at the first non-pattern line to avoid draining
        // unrelated content that happens to follow without a blank separator.
        let remove_block = |content: &str| -> String {
            let mut lines: Vec<&str> = content.lines().collect();
            let mut start = None;
            let mut end = None;
            for (i, line) in lines.iter().enumerate() {
                if line.contains(marker) {
                    // Include preceding blank line if present
                    start = Some(if i > 0 && lines[i - 1].trim().is_empty() {
                        i - 1
                    } else {
                        i
                    });
                }
                if start.is_some() && end.is_none() && i > start.unwrap() {
                    // Block continues while lines are our gitignore patterns (start with '/')
                    if line.trim().is_empty() || !line.starts_with('/') {
                        end = Some(i);
                        break;
                    }
                }
            }
            // If we found start but not end, the block runs to EOF
            if start.is_some() && end.is_none() {
                end = Some(lines.len());
            }
            if let (Some(s), Some(e)) = (start, end) {
                lines.drain(s..e);
            }
            let result = lines.join("\n");
            if result.is_empty() {
                result
            } else {
                format!("{}\n", result)
            }
        };

        // .gitignore
        let gitignore_path = project_path.join(".gitignore");
        if addToGitignore {
            let existing = if gitignore_path.exists() {
                fs::read_to_string(&gitignore_path)
                    .with_context(|| format!("failed to read {}", gitignore_path.display()))?
            } else {
                String::new()
            };

            if !existing.contains(marker) {
                let mut content = existing;
                if !content.ends_with('\n') && !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(&gitignore_block);
                fs::write(&gitignore_path, content)
                    .with_context(|| format!("failed to write {}", gitignore_path.display()))?;
            }
        } else if gitignore_path.exists() {
            let existing = fs::read_to_string(&gitignore_path)
                .with_context(|| format!("failed to read {}", gitignore_path.display()))?;
            if existing.contains(marker) {
                let cleaned = remove_block(&existing);
                fs::write(&gitignore_path, cleaned)
                    .with_context(|| format!("failed to write {}", gitignore_path.display()))?;
            }
        }

        // .git/info/exclude
        let exclude_path = project_path.join(".git").join("info").join("exclude");
        if addToExclude {
            if let Some(parent) = exclude_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("failed to create {}", parent.display()))?;
                }
            }

            let existing = if exclude_path.exists() {
                fs::read_to_string(&exclude_path)
                    .with_context(|| format!("failed to read {}", exclude_path.display()))?
            } else {
                String::new()
            };

            if !existing.contains(marker) {
                let mut content = existing;
                if !content.ends_with('\n') && !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(&gitignore_block);
                fs::write(&exclude_path, content)
                    .with_context(|| format!("failed to write {}", exclude_path.display()))?;
            }
        } else if exclude_path.exists() {
            let existing = fs::read_to_string(&exclude_path)
                .with_context(|| format!("failed to read {}", exclude_path.display()))?;
            if existing.contains(marker) {
                let cleaned = remove_block(&existing);
                fs::write(&exclude_path, cleaned)
                    .with_context(|| format!("failed to write {}", exclude_path.display()))?;
            }
        }

        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(format_anyhow_error)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_project_gitignore_status(
    store: State<'_, SkillStore>,
    projectId: String,
) -> Result<GitignoreStatusDto, String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        use std::fs;
        use std::path::Path;

        let project = store
            .get_project_by_id(&projectId)?
            .ok_or_else(|| anyhow::anyhow!("project not found: {}", projectId))?;

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
    .map_err(|e| e.to_string())?
    .map_err(format_anyhow_error)
}

#[derive(serde::Serialize, Clone)]
pub struct GitignoreStatusDto {
    pub in_gitignore: bool,
    pub in_exclude: bool,
}
