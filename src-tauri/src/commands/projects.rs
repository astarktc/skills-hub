use specta::Type;
use tauri::State;

use crate::core::environment::home_dir;
use crate::core::gitignore::{self, IgnoreUpdateOptions};
use crate::core::project_ops::{
    self, ProjectDto, ProjectSkillAssignmentDto, ProjectToolDto, ProjectView,
};
use crate::core::project_sync::{self, AssignTargetStatus, ToggleOutcome};
use crate::core::skill_store::{ProjectSkillAssignmentRecord, SkillStore};

use super::CommandError;
use crate::core::clock::now_ms;

/// Everything the project world shows for one project, as one wire value:
/// the project row (counts and aggregate status included), its configured
/// Tools, and its assignments with the reconcile flag.
///
/// Every project mutation returns this, so the frontend applies one result
/// instead of chasing the mutation with follow-up reads. It is always built
/// *after* the mutation released the mutation guard, so its reconcile pass
/// can take the guard normally.
#[derive(serde::Serialize, Clone, Type)]
pub struct ProjectViewDto {
    pub project: ProjectDto,
    pub tools: Vec<ProjectToolDto>,
    pub assignments: Vec<ProjectSkillAssignmentDto>,
    /// `false` means a Sync-target mutation was in flight and the reconcile
    /// pass was skipped rather than queued: the rows are the stored ones,
    /// not re-derived from disk. Never read it as healthy.
    pub reconciled: bool,
}

fn to_project_view_dto(view: ProjectView) -> ProjectViewDto {
    ProjectViewDto {
        project: view.project,
        tools: view.tools,
        assignments: view
            .assignments
            .assignments
            .into_iter()
            .map(to_assignment_dto)
            .collect(),
        reconciled: view.assignments.reconciled,
    }
}

/// The view of `project_id`, read outside any critical section.
fn view_of(store: &SkillStore, project_id: &str) -> anyhow::Result<ProjectViewDto> {
    Ok(to_project_view_dto(project_ops::project_view(
        store, project_id,
    )?))
}

#[tauri::command]
#[specta::specta]
pub async fn register_project(
    store: State<'_, SkillStore>,
    path: String,
) -> Result<ProjectViewDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let home = home_dir()?;
        let project = project_ops::register_project_path(&store, &home, &path, now_ms())?;
        view_of(&store, &project.id)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

/// Remove a project and every artifact it owns. The project it named is
/// gone, so the fresh view is the *remaining* project list.
#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn remove_project(
    store: State<'_, SkillStore>,
    projectId: String,
) -> Result<Vec<ProjectDto>, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        project_ops::remove_project_with_cleanup(&store, &projectId)?;
        project_ops::list_project_dtos(&store)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

/// The read counterpart of the mutation views: what selecting a project
/// loads.
#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn get_project_view(
    store: State<'_, SkillStore>,
    projectId: String,
) -> Result<ProjectViewDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || view_of(&store, &projectId))
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
) -> Result<ProjectViewDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let home = home_dir()?;
        project_ops::update_project_path(&store, &home, &projectId, &path, now_ms())?;
        view_of(&store, &projectId)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

/// Replace the project's configured tool set and, when `gitignore` is given,
/// update its ignore files afterwards. Core owns the ordering
/// (`project_ops::configure_project_tools`). Removing a tool cascades to its
/// assignments, so the returned view already reflects that cascade.
#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn configure_project_tools(
    store: State<'_, SkillStore>,
    projectId: String,
    tools: Vec<String>,
    gitignore: Option<IgnoreUpdateOptions>,
) -> Result<ProjectViewDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        project_ops::configure_project_tools(&store, &projectId, &tools, gitignore)?;
        view_of(&store, &projectId)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

/// Which way a toggle went, with the resulting view.
#[derive(serde::Serialize, Clone, Type)]
pub struct ToggleAssignmentResultDto {
    pub view: ProjectViewDto,
    /// True when the skill is now assigned to the tool, false when the
    /// assignment was removed.
    pub assigned: bool,
}

/// Assign or unassign one skill × project Tool pair — the backend decides
/// which from its own rows, so no caller mirrors assignment existence.
#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn toggle_project_skill_assignment(
    store: State<'_, SkillStore>,
    projectId: String,
    skillId: String,
    tool: String,
) -> Result<ToggleAssignmentResultDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let outcome =
            project_sync::toggle_skill_assignment(&store, &projectId, &skillId, &tool, now_ms())?;
        Ok::<_, anyhow::Error>(ToggleAssignmentResultDto {
            view: view_of(&store, &projectId)?,
            assigned: outcome == ToggleOutcome::Assigned,
        })
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
fn to_resync_summary_dto(summary: project_sync::ResyncSummary) -> ResyncSummaryDto {
    ResyncSummaryDto {
        project_id: summary.project_id,
        synced: summary.synced,
        failed: summary.failed,
        errors: summary.errors,
    }
}

/// A resync's counts and errors alongside the project's fresh view.
#[derive(serde::Serialize, Clone, Type)]
pub struct ResyncProjectResultDto {
    pub view: ProjectViewDto,
    pub summary: ResyncSummaryDto,
}

#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn resync_project(
    store: State<'_, SkillStore>,
    projectId: String,
) -> Result<ResyncProjectResultDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let summary = project_sync::resync_project(&store, &projectId, now_ms())?;
        Ok::<_, anyhow::Error>(ResyncProjectResultDto {
            view: view_of(&store, &projectId)?,
            summary: to_resync_summary_dto(summary),
        })
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

/// Per-project counts and errors plus the refreshed project list. A single
/// project's assignments are not returned here — the caller re-reads the
/// view of whichever project it is showing.
#[derive(serde::Serialize, Clone, Type)]
pub struct ResyncAllResultDto {
    pub summaries: Vec<ResyncSummaryDto>,
    pub projects: Vec<ProjectDto>,
}

#[tauri::command]
#[specta::specta]
pub async fn resync_all_projects(
    store: State<'_, SkillStore>,
) -> Result<ResyncAllResultDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let summaries = project_sync::resync_all_projects(&store, now_ms())?;
        Ok::<_, anyhow::Error>(ResyncAllResultDto {
            summaries: summaries.into_iter().map(to_resync_summary_dto).collect(),
            projects: project_ops::list_project_dtos(&store)?,
        })
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

/// The fan-out's fresh view plus the tools it could not assign. Tools that
/// were already assigned are silent (nothing changed for them); the view
/// carries every assignment that now exists.
#[derive(serde::Serialize, Clone, Type)]
pub struct BulkAssignResultDto {
    pub view: ProjectViewDto,
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
    projectId: String,
    skillId: String,
) -> Result<BulkAssignResultDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let outcomes =
            project_sync::assign_skill_to_project_tools(&store, &projectId, &skillId, now_ms())?;

        let mut failed = Vec::new();
        for outcome in outcomes {
            match outcome.status {
                AssignTargetStatus::Assigned { .. } | AssignTargetStatus::AlreadyAssigned => {}
                AssignTargetStatus::Failed { error } => failed.push(BulkAssignErrorDto {
                    tool: outcome.tool_key,
                    error: CommandError::from_anyhow(error),
                }),
            }
        }
        Ok::<_, anyhow::Error>(BulkAssignResultDto {
            view: view_of(&store, &projectId)?,
            failed,
        })
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
    gitignore: IgnoreUpdateOptions,
) -> Result<(), CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        gitignore::update_for_project(&store, &projectId, gitignore)
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

        let project = project_ops::require_project(&store, &projectId)?;
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
