use specta::Type;
use tauri::State;

use crate::core::environment::home_dir;
use crate::core::gitignore::{self, IgnoreUpdateOptions};
use crate::core::project_ops::{
    self, ProjectDto, ProjectSkillAssignmentDto, ProjectToolDto, ProjectView,
};
use crate::core::project_sync::{self, ProjectSyncReport, ToggleOutcome};
use crate::core::skill_store::{ProjectSkillAssignmentRecord, SkillStore};

use super::CommandError;
use crate::core::artifact_removal::RemovalReport;
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

/// What removing a project settled: the project list *after* the removal
/// and the per-target report. When every artifact went, the project is
/// absent from `projects`; when one stayed, the project is still listed
/// (its rows kept with Sync status `error`, ADR-0002) and `report` names
/// each path that could not be removed — report data, not a command error.
#[derive(serde::Serialize, Type)]
pub struct RemoveProjectResultDto {
    pub projects: Vec<ProjectDto>,
    pub report: RemovalReport,
}

/// Remove a project and every artifact it owns. The project it named is
/// (normally) gone, so the fresh view is the *remaining* project list.
#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn remove_project(
    store: State<'_, SkillStore>,
    projectId: String,
) -> Result<RemoveProjectResultDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let report = project_ops::remove_project_and_artifacts(&store, &projectId)?;
        Ok::<_, anyhow::Error>(RemoveProjectResultDto {
            projects: project_ops::list_project_dtos(&store)?,
            report,
        })
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

/// What configuring a project's tool set settled: the resulting view and
/// the removal report for every tool dropped from the set, merged into one
/// (a tool whose artifact stayed is still in `view.tools` — its row was
/// kept for a retry — and its failure is in `report`). A configuration
/// that dropped nothing carries an empty report.
#[derive(serde::Serialize, Type)]
pub struct ConfigureProjectToolsResultDto {
    pub view: ProjectViewDto,
    pub report: RemovalReport,
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
) -> Result<ConfigureProjectToolsResultDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let removals = project_ops::configure_project_tools(&store, &projectId, &tools, gitignore)?;
        let report = RemovalReport::merge(removals);
        Ok::<_, anyhow::Error>(ConfigureProjectToolsResultDto {
            view: view_of(&store, &projectId)?,
            report,
        })
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

/// Which way a toggle went, with the resulting view and that direction's
/// report: the project-sync report of one on assign (a sync failure is
/// report data — the row is kept with status `error`), the removal report on
/// unassign.
#[derive(serde::Serialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToggleAssignmentResultDto {
    Assigned {
        view: ProjectViewDto,
        report: ProjectSyncReport,
    },
    Unassigned {
        view: ProjectViewDto,
        report: RemovalReport,
    },
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
        let view = view_of(&store, &projectId)?;
        Ok::<_, anyhow::Error>(match outcome {
            ToggleOutcome::Assigned { report } => {
                ToggleAssignmentResultDto::Assigned { view, report }
            }
            ToggleOutcome::Unassigned { report } => {
                ToggleAssignmentResultDto::Unassigned { view, report }
            }
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

/// A re-sync's report alongside the project's fresh view.
#[derive(serde::Serialize, Clone, Type)]
pub struct ResyncProjectResultDto {
    pub view: ProjectViewDto,
    pub report: ProjectSyncReport,
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
        let report = project_sync::resync_project(&store, &projectId, now_ms())?;
        Ok::<_, anyhow::Error>(ResyncProjectResultDto {
            view: view_of(&store, &projectId)?,
            report,
        })
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

/// One report spanning every project plus the refreshed project list. A
/// single project's assignments are not returned here — the caller re-reads
/// the view of whichever project it is showing.
#[derive(serde::Serialize, Clone, Type)]
pub struct ResyncAllResultDto {
    pub report: ProjectSyncReport,
    pub projects: Vec<ProjectDto>,
}

#[tauri::command]
#[specta::specta]
pub async fn resync_all_projects(
    store: State<'_, SkillStore>,
) -> Result<ResyncAllResultDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let report = project_sync::resync_all_projects(&store, now_ms())?;
        Ok::<_, anyhow::Error>(ResyncAllResultDto {
            report,
            projects: project_ops::list_project_dtos(&store)?,
        })
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

/// The fan-out's fresh view plus its report: one item per configured Tool
/// (`synced`, `already_assigned`, or `failed` with the typed error).
#[derive(serde::Serialize, Clone, Type)]
pub struct BulkAssignResultDto {
    pub view: ProjectViewDto,
    pub report: ProjectSyncReport,
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
        let report =
            project_sync::assign_skill_to_project_tools(&store, &projectId, &skillId, now_ms())?;
        Ok::<_, anyhow::Error>(BulkAssignResultDto {
            view: view_of(&store, &projectId)?,
            report,
        })
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

/// Bulk unassign's fresh view plus the removal report for every assignment
/// row of the skill in the project (a row whose artifact stayed is kept with
/// status `error` and named in `report`, ADR-0002).
#[derive(serde::Serialize, Type)]
pub struct BulkUnassignResultDto {
    pub view: ProjectViewDto,
    pub report: RemovalReport,
}

/// Unassign one skill from every Tool of one project — the inverse of
/// `bulk_assign_skill`.
#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn bulk_unassign_skill(
    store: State<'_, SkillStore>,
    projectId: String,
    skillId: String,
) -> Result<BulkUnassignResultDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let report = project_sync::unassign_skill_from_project(&store, &projectId, &skillId)?;
        Ok::<_, anyhow::Error>(BulkUnassignResultDto {
            view: view_of(&store, &projectId)?,
            report,
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
