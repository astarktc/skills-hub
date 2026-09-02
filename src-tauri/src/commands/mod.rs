pub mod error;
pub mod projects;

use anyhow::Context;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::ipc::Channel;
use tauri::{Manager, State};

use std::path::PathBuf;
use std::sync::Arc;

use crate::core::cache_cleanup::cleanup_git_cache_dirs;
use crate::core::cancel_token::CancelToken;
use crate::core::clock::now_ms;
use crate::core::environment::{expand_home_path, home_dir};
use crate::core::featured_skills::{fetch_featured_skills, FeaturedSkill};
use crate::core::global_sync::{
    BatchOverride, BatchPolicy, BatchSkill, BatchTargetOutcome, BatchTargetStatus,
};
use crate::core::installer::{
    clone_for_explore_preview, install_git_skill_from_selection,
    install_local_skill_from_selection, list_git_skills, list_local_skills, GitSkillListing,
    InstallResult, InstallerPaths, LocalSkillCandidate,
};
use crate::core::onboarding::{build_onboarding_plan, OnboardingPlan};
use crate::core::onboarding_import::{
    import_onboarding_selection as import_onboarding_selection_core, ImportGroupStatus,
    ImportPhase, ImportPolicy, ImportSelection, OriginalStatus,
};
use crate::core::propagation::{PropagationScope, PropagationSkip, PropagationStatus};
use crate::core::refresh::{
    refresh_managed_skills as refresh_managed_skills_core, RefreshPhase, RefreshPolicy,
    RefreshSelection, SkillRefreshStatus,
};
use crate::core::settings::{
    apply_setting, load_settings, record_installed_tools, AppSettings, SettingUpdate,
};
use crate::core::skill_catalog::{managed_skill_catalog, ManagedSkillEntry};
use crate::core::skill_discovery::InvocationMode;
use crate::core::skill_store::SkillStore;
use crate::core::skills_search::{
    search_skills_online as search_skills_online_core, OnlineSkillResult,
};
use crate::core::sync_status::{SyncMode, SyncStatus};
use crate::core::tool_adapters::{
    global_tool_entries, installed_keys, project_tool_entries, ToolCatalogEntry,
};

pub use error::CommandError;

/// Production environment adapter for the central repo: `.skillshub` lives
/// under the operator's home, or under the app data dir when no home can be
/// resolved. The only place the wiring tier reads Tauri paths for core.
pub(crate) fn resolve_central_repo_path_for_app(
    app: &tauri::AppHandle,
    store: &SkillStore,
) -> Result<PathBuf, anyhow::Error> {
    crate::core::settings::resolve_central_repo_path(store, &settings_fallback_root(app)?)
}

/// Root under which the default central repo lives (see
/// `settings::resolve_central_repo_path`): home, else the app data dir.
fn settings_fallback_root(app: &tauri::AppHandle) -> Result<PathBuf, anyhow::Error> {
    match home_dir() {
        Ok(home) => Ok(home),
        Err(_) => app
            .path()
            .app_data_dir()
            .context("failed to resolve app data dir"),
    }
}

/// Resolve every root the installer needs once, at the command seam.
fn installer_paths(
    app: &tauri::AppHandle,
    store: &SkillStore,
) -> Result<InstallerPaths, anyhow::Error> {
    Ok(InstallerPaths {
        home: home_dir()?,
        central_dir: resolve_central_repo_path_for_app(app, store)?,
        cache_dir: app
            .path()
            .app_cache_dir()
            .context("failed to resolve app cache dir")?,
    })
}

#[derive(Debug, Serialize, Type)]
pub struct ToolInfoDto {
    pub key: String,
    pub label: String,
    pub installed: bool,
    /// Keys of every listed tool sharing this tool's skills dir (global dir
    /// for the global list, project dir for the project list), in adapter
    /// order, including this tool itself (len >= 1). The backend owns the
    /// shared-dir invariant; the frontend only presents it. The dir itself is
    /// not on the wire — see `ToolCatalogEntry`.
    pub shared_with: Vec<String>,
    /// Display labels of the constituent tools absorbed into this entry when
    /// it is a virtual group (project-scope AgentsStandard); empty for real
    /// tools. The backend owns group membership; the frontend only presents it.
    pub constituents: Vec<String>,
}

#[derive(Debug, Serialize, Type)]
pub struct ToolStatusDto {
    pub tools: Vec<ToolInfoDto>,
    pub installed: Vec<String>,
    pub newly_installed: Vec<String>,
}

impl From<ToolCatalogEntry> for ToolInfoDto {
    fn from(entry: ToolCatalogEntry) -> Self {
        ToolInfoDto {
            key: entry.key.to_string(),
            label: entry.label.to_string(),
            installed: entry.installed,
            shared_with: entry.shared_with.iter().map(|k| k.to_string()).collect(),
            constituents: entry.constituents.iter().map(|k| k.to_string()).collect(),
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_tool_status(store: State<'_, SkillStore>) -> Result<ToolStatusDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let home = home_dir()?;
        let entries = global_tool_entries(&home);
        let installed = installed_keys(&entries);
        let newly_installed = record_installed_tools(&store, &installed)?;
        Ok::<_, anyhow::Error>(ToolStatusDto {
            tools: entries.into_iter().map(ToolInfoDto::from).collect(),
            installed,
            newly_installed,
        })
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[specta::specta]
pub async fn get_project_tool_status() -> Result<ToolStatusDto, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let home = home_dir()?;
        let entries = project_tool_entries(&home);
        let installed = installed_keys(&entries);
        Ok::<_, anyhow::Error>(ToolStatusDto {
            tools: entries.into_iter().map(ToolInfoDto::from).collect(),
            installed,
            newly_installed: vec![],
        })
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[specta::specta]
pub async fn get_onboarding_plan(
    app: tauri::AppHandle,
    store: State<'_, SkillStore>,
) -> Result<OnboardingPlan, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let home = home_dir()?;
        let central = resolve_central_repo_path_for_app(&app, &store)?;
        build_onboarding_plan(&home, &central, &store)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[specta::specta]
pub async fn clear_git_cache_now(app: tauri::AppHandle) -> Result<usize, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        cleanup_git_cache_dirs(&app, std::time::Duration::from_secs(0))
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[derive(Debug, Serialize, Type)]
pub struct InstallResultDto {
    pub skill_id: String,
    pub name: String,
    pub central_path: String,
    pub content_hash: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn get_settings(
    app: tauri::AppHandle,
    store: State<'_, SkillStore>,
) -> Result<AppSettings, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        load_settings(&store, &settings_fallback_root(&app)?)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

/// Persist one setting and return the effective snapshot. `~` in a central
/// repo path is expanded here — the only environment-aware step — before
/// the policy layer validates and applies it.
#[tauri::command]
#[specta::specta]
pub async fn update_setting(
    app: tauri::AppHandle,
    store: State<'_, SkillStore>,
    update: SettingUpdate,
) -> Result<AppSettings, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let update = match update {
            SettingUpdate::CentralRepoPath(raw) => SettingUpdate::CentralRepoPath(
                expand_home_path(&raw)?.to_string_lossy().to_string(),
            ),
            other => other,
        };
        apply_setting(&store, &settings_fallback_root(&app)?, update)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn list_local_skills_cmd(
    basePath: String,
) -> Result<Vec<LocalSkillCandidate>, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = std::path::PathBuf::from(basePath);
        list_local_skills(&path)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn install_local_selection(
    app: tauri::AppHandle,
    store: State<'_, SkillStore>,
    basePath: String,
    subpath: String,
    name: Option<String>,
) -> Result<InstallResultDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let base = std::path::PathBuf::from(basePath);
        let paths = installer_paths(&app, &store)?;
        let result =
            install_local_skill_from_selection(&paths, &store, base.as_ref(), &subpath, name)?;
        Ok::<_, anyhow::Error>(to_install_dto(result))
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn list_git_skills_cmd(
    app: tauri::AppHandle,
    store: State<'_, SkillStore>,
    repoUrl: String,
    targetName: Option<String>,
) -> Result<GitSkillListing, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let paths = installer_paths(&app, &store)?;
        list_git_skills(&paths, &store, &repoUrl, targetName.as_deref())
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn install_git_selection(
    app: tauri::AppHandle,
    store: State<'_, SkillStore>,
    cancel: State<'_, Arc<CancelToken>>,
    repoUrl: String,
    subpath: String,
    name: Option<String>,
) -> Result<InstallResultDto, CommandError> {
    let store = store.inner().clone();
    let cancel = cancel.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        cancel.reset();
        let paths = installer_paths(&app, &store)?;
        let result = install_git_skill_from_selection(
            &paths,
            &store,
            &repoUrl,
            &subpath,
            name,
            Some(&cancel),
        )?;
        Ok::<_, anyhow::Error>(to_install_dto(result))
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

/// One skill in a batch sync request.
#[derive(Debug, Clone, Deserialize, Type)]
pub struct BatchSyncSkillDto {
    pub skill_id: String,
    pub name: String,
    pub source_path: String,
}

/// Force-overwrite for one (skill, tool) pair; applies to any target tool
/// sharing the named tool's skills dir.
#[derive(Debug, Clone, Deserialize, Type)]
pub struct BatchSyncOverrideDto {
    pub skill_id: String,
    pub tool: String,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct BatchSyncPolicyDto {
    #[serde(default)]
    pub overwrite: bool,
    #[serde(default)]
    pub overwrite_if_same_content: bool,
    #[serde(default)]
    pub overrides: Vec<BatchSyncOverrideDto>,
}

/// Per-(skill, tool) result. `skipped` is the expected-and-ignorable class
/// (tool absent, dir unwritable); `failed` is everything else. Both carry
/// the typed error so call sites choose what to surface.
#[derive(Debug, Serialize, Type)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SyncTargetStatusDto {
    Synced { mode_used: SyncMode },
    Skipped { error: CommandError },
    Failed { error: CommandError },
}

#[derive(Debug, Serialize, Type)]
pub struct SyncTargetResultDto {
    pub skill_id: String,
    pub skill_name: String,
    pub tool: String,
    pub status: SyncTargetStatusDto,
}

#[derive(Debug, Serialize, Type)]
pub struct BatchSyncReportDto {
    pub results: Vec<SyncTargetResultDto>,
    pub synced: u32,
    pub skipped: u32,
    pub failed: u32,
}

/// Progress tick streamed over the command's channel before each attempted
/// (skill, tool) pair.
#[derive(Debug, Clone, Serialize, Type)]
pub struct SyncProgressDto {
    pub index: u32,
    pub total: u32,
    pub skill_name: String,
    pub tool: String,
}

/// Sync N skills to M tools in one call. The backend owns the whole
/// choreography — installedness filtering, shared-dir dedupe, overwrite
/// policy, DB record fan-out — and returns a per-target report; per-target
/// failures are data, not command errors.
#[tauri::command]
#[specta::specta]
pub async fn sync_skills_to_tools(
    store: State<'_, SkillStore>,
    skills: Vec<BatchSyncSkillDto>,
    tools: Vec<String>,
    policy: BatchSyncPolicyDto,
    on_progress: Channel<SyncProgressDto>,
) -> Result<BatchSyncReportDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let batch_skills: Vec<BatchSkill> = skills
            .into_iter()
            .map(|s| BatchSkill {
                skill_id: s.skill_id,
                skill_name: s.name,
                source_path: std::path::PathBuf::from(s.source_path),
            })
            .collect();
        let batch_policy = BatchPolicy {
            overwrite: policy.overwrite,
            overwrite_if_same_content: policy.overwrite_if_same_content,
            overrides: policy
                .overrides
                .into_iter()
                .map(|o| BatchOverride {
                    skill_id: o.skill_id,
                    tool_key: o.tool,
                    overwrite: o.overwrite,
                })
                .collect(),
        };

        let home = home_dir()?;
        let outcomes = crate::core::global_sync::sync_skills_to_tools(
            &home,
            &store,
            &batch_skills,
            &tools,
            &batch_policy,
            now_ms(),
            |p| {
                let _ = on_progress.send(SyncProgressDto {
                    index: p.index as u32,
                    total: p.total as u32,
                    skill_name: p.skill_name.to_string(),
                    tool: p.tool_key.to_string(),
                });
            },
        );

        let mut report = BatchSyncReportDto {
            results: Vec::with_capacity(outcomes.len()),
            synced: 0,
            skipped: 0,
            failed: 0,
        };
        for outcome in outcomes {
            let result = to_sync_target_result_dto(outcome);
            match result.status {
                SyncTargetStatusDto::Synced { .. } => report.synced += 1,
                SyncTargetStatusDto::Skipped { .. } => report.skipped += 1,
                SyncTargetStatusDto::Failed { .. } => report.failed += 1,
            }
            report.results.push(result);
        }
        Ok::<_, anyhow::Error>(report)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

/// The one wire shape for a (skill, tool) sync outcome — shared by the sync
/// batch and the Onboarding import report.
fn to_sync_target_result_dto(outcome: BatchTargetOutcome) -> SyncTargetResultDto {
    let status = match outcome.status {
        BatchTargetStatus::Synced { outcome } => SyncTargetStatusDto::Synced {
            mode_used: outcome.mode_used,
        },
        BatchTargetStatus::Skipped { error } => SyncTargetStatusDto::Skipped {
            error: CommandError::from(error),
        },
        BatchTargetStatus::Failed { error } => SyncTargetStatusDto::Failed {
            error: CommandError::from(error),
        },
    };
    SyncTargetResultDto {
        skill_id: outcome.skill_id,
        skill_name: outcome.skill_name,
        tool: outcome.tool_key,
        status,
    }
}

/// Which Sync target a removal outcome is about.
#[derive(Debug, Serialize, Type)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum RemovalScopeDto {
    Global,
    Project { project_id: String },
}

/// Per-row removal result. `failed` means the artifact is still on disk and
/// the row was kept with Sync status `error` (ADR-0002) — report data, not a
/// command error.
#[derive(Debug, Serialize, Type)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RemovalTargetStatusDto {
    Removed,
    Failed { error: CommandError },
}

#[derive(Debug, Serialize, Type)]
pub struct RemovalTargetDto {
    pub scope: RemovalScopeDto,
    pub tool: String,
    pub path: String,
    pub status: RemovalTargetStatusDto,
}

#[derive(Debug, Serialize, Type)]
pub struct RemovalReportDto {
    pub targets: Vec<RemovalTargetDto>,
    /// Rows whose artifact was removed (and whose row was deleted).
    pub removed: u32,
    /// Rows kept with Sync status `error` because their artifact stayed.
    pub failed: u32,
}

/// One DTO row per settled row: a shared skills dir removes one artifact but
/// settles every member row, and the frontend reports per tool.
fn to_removal_report_dto(report: crate::core::artifact_removal::RemovalReport) -> RemovalReportDto {
    use crate::core::artifact_removal::RemovalTargetStatus;

    let mut dto = RemovalReportDto {
        targets: Vec::new(),
        removed: report.removed_rows() as u32,
        failed: report.failed_rows() as u32,
    };
    for target in report.targets {
        let path = target.path.to_string_lossy().to_string();
        for row in target.rows {
            let status = match &target.status {
                RemovalTargetStatus::Removed => RemovalTargetStatusDto::Removed,
                RemovalTargetStatus::Failed { error } => RemovalTargetStatusDto::Failed {
                    error: CommandError::from_anyhow(anyhow::anyhow!("{}", error)),
                },
            };
            let scope = match row.project_id() {
                None => RemovalScopeDto::Global,
                Some(project_id) => RemovalScopeDto::Project {
                    project_id: project_id.to_string(),
                },
            };
            dto.targets.push(RemovalTargetDto {
                scope,
                tool: row.tool().to_string(),
                path: path.clone(),
                status,
            });
        }
    }
    dto
}

#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn unsync_skill_from_tool(
    store: State<'_, SkillStore>,
    skillId: String,
    tool: String,
) -> Result<RemovalReportDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let home = home_dir()?;
        let report =
            crate::core::artifact_removal::unsync_skill_from_tool(&store, &home, &skillId, &tool)?;
        Ok::<_, anyhow::Error>(to_removal_report_dto(report))
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

/// Which Sync target a Propagation outcome is about.
#[derive(Debug, Serialize, Type)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum PropagationScopeDto {
    Global { tool: String },
    Project { project_id: String, tool: String },
}

/// Why a Sync target needed no work. Not a failure — skipping is the correct
/// outcome for a link, an uninstalled Tool, or an absent Project.
#[derive(Debug, Serialize, Type)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum PropagationSkipDto {
    LinkFollowsSource,
    ToolNotInstalled { tool: String },
    UnknownTool { tool: String },
    ProjectUnavailable { project_id: String },
}

#[derive(Debug, Serialize, Type)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PropagationStatusDto {
    Synced { mode_used: SyncMode },
    Skipped { reason: PropagationSkipDto },
    Failed { error: CommandError },
}

#[derive(Debug, Serialize, Type)]
pub struct PropagationTargetDto {
    pub scope: PropagationScopeDto,
    pub status: PropagationStatusDto,
}

/// Per-skill result of a Refresh batch. A skill whose bytes could not be
/// acquired is `failed` — its Sync targets were left alone.
#[derive(Debug, Serialize, Type)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SkillRefreshStatusDto {
    Refreshed {
        content_hash: Option<String>,
        source_revision: Option<String>,
        targets: Vec<PropagationTargetDto>,
    },
    Failed {
        error: CommandError,
    },
}

#[derive(Debug, Serialize, Type)]
pub struct SkillRefreshResultDto {
    pub skill_id: String,
    pub skill_name: String,
    pub status: SkillRefreshStatusDto,
}

#[derive(Debug, Serialize, Type)]
pub struct RefreshReportDto {
    pub skills: Vec<SkillRefreshResultDto>,
    pub refreshed: u32,
    pub failed: u32,
    /// Sync targets that failed across every refreshed skill.
    pub target_failures: u32,
}

#[derive(Debug, Clone, Copy, Deserialize, Type)]
pub struct RefreshPolicyDto {
    /// Also sync each refreshed skill to installed Tools it is not on yet
    /// (the auto-sync invariant, re-asserted). The caller decides — the
    /// backend never reads the auto-sync setting behind its back.
    #[serde(default)]
    pub reassert_auto_sync: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum RefreshPhaseDto {
    Acquiring,
    Applying,
}

/// Progress tick streamed before each per-skill step of each Refresh phase.
#[derive(Debug, Clone, Serialize, Type)]
pub struct RefreshProgressDto {
    pub index: u32,
    pub total: u32,
    pub skill_name: String,
    pub phase: RefreshPhaseDto,
}

/// Re-acquire Managed skills from their sources, finalize them, and
/// propagate to every Sync target — one batch, one report. `skillIds` of
/// `None` means every Managed skill (Refresh all); a single-skill Update is
/// a batch of one. Per-skill and per-target failures are report data, not
/// command errors.
#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn refresh_managed_skills(
    app: tauri::AppHandle,
    store: State<'_, SkillStore>,
    cancel: State<'_, Arc<CancelToken>>,
    skillIds: Option<Vec<String>>,
    policy: RefreshPolicyDto,
    on_progress: Channel<RefreshProgressDto>,
) -> Result<RefreshReportDto, CommandError> {
    let store = store.inner().clone();
    let cancel = cancel.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        cancel.reset();
        let paths = installer_paths(&app, &store)?;
        let selection = match skillIds {
            Some(ids) => RefreshSelection::Ids(ids),
            None => RefreshSelection::All,
        };
        let report = refresh_managed_skills_core(
            &paths,
            &store,
            selection,
            RefreshPolicy {
                reassert_auto_sync: policy.reassert_auto_sync,
            },
            Some(&cancel),
            now_ms(),
            |p| {
                let _ = on_progress.send(RefreshProgressDto {
                    index: p.index as u32,
                    total: p.total as u32,
                    skill_name: p.skill_name.to_string(),
                    phase: match p.phase {
                        RefreshPhase::Acquiring => RefreshPhaseDto::Acquiring,
                        RefreshPhase::Applying => RefreshPhaseDto::Applying,
                    },
                });
            },
        )?;
        Ok::<_, anyhow::Error>(to_refresh_report_dto(report))
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

fn to_refresh_report_dto(report: crate::core::refresh::RefreshReport) -> RefreshReportDto {
    let mut dto = RefreshReportDto {
        skills: Vec::with_capacity(report.skills.len()),
        refreshed: 0,
        failed: 0,
        target_failures: 0,
    };
    for skill in report.skills {
        let status = match skill.status {
            SkillRefreshStatus::Refreshed {
                content_hash,
                source_revision,
                targets,
            } => {
                dto.refreshed += 1;
                let targets: Vec<PropagationTargetDto> = targets
                    .into_iter()
                    .map(|target| PropagationTargetDto {
                        scope: match target.scope {
                            PropagationScope::Global { tool } => {
                                PropagationScopeDto::Global { tool }
                            }
                            PropagationScope::Project { project_id, tool } => {
                                PropagationScopeDto::Project { project_id, tool }
                            }
                        },
                        status: match target.status {
                            PropagationStatus::Synced { mode_used } => {
                                PropagationStatusDto::Synced { mode_used }
                            }
                            PropagationStatus::Skipped { reason } => {
                                PropagationStatusDto::Skipped {
                                    reason: match reason {
                                        PropagationSkip::LinkFollowsSource => {
                                            PropagationSkipDto::LinkFollowsSource
                                        }
                                        PropagationSkip::ToolNotInstalled { tool } => {
                                            PropagationSkipDto::ToolNotInstalled { tool }
                                        }
                                        PropagationSkip::UnknownTool { tool } => {
                                            PropagationSkipDto::UnknownTool { tool }
                                        }
                                        PropagationSkip::ProjectUnavailable { project_id } => {
                                            PropagationSkipDto::ProjectUnavailable { project_id }
                                        }
                                    },
                                }
                            }
                            PropagationStatus::Failed { error } => {
                                dto.target_failures += 1;
                                PropagationStatusDto::Failed {
                                    error: CommandError::from_anyhow(error),
                                }
                            }
                        },
                    })
                    .collect();
                SkillRefreshStatusDto::Refreshed {
                    content_hash,
                    source_revision,
                    targets,
                }
            }
            SkillRefreshStatus::Failed { error } => {
                dto.failed += 1;
                SkillRefreshStatusDto::Failed {
                    error: CommandError::from_anyhow(error),
                }
            }
        };
        dto.skills.push(SkillRefreshResultDto {
            skill_id: skill.skill_id,
            skill_name: skill.skill_name,
            status,
        });
    }
    dto
}

#[tauri::command]
#[specta::specta]
pub async fn unsync_all_skills(
    store: State<'_, SkillStore>,
) -> Result<RemovalReportDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let report = crate::core::artifact_removal::unsync_all_skill_targets(&store)?;
        Ok::<_, anyhow::Error>(to_removal_report_dto(report))
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn unsync_skill(
    store: State<'_, SkillStore>,
    skillId: String,
) -> Result<RemovalReportDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let report = crate::core::artifact_removal::unsync_skill_targets(&store, &skillId)?;
        Ok::<_, anyhow::Error>(to_removal_report_dto(report))
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

/// One name-group the operator chose to import. The backend re-reads the
/// onboarding plan to learn which paths the group owns, so this carries only
/// the choice itself.
#[derive(Debug, Clone, Deserialize, Type)]
pub struct OnboardingSelectionDto {
    pub group_name: String,
    pub chosen_path: String,
    pub name: Option<String>,
}

/// What happens to the originals: sync the imported skill to `tools`
/// (`None` = every installed Tool), or remove the originals.
#[derive(Debug, Clone, Deserialize, Type)]
pub struct ImportPolicyDto {
    #[serde(default)]
    pub auto_sync: bool,
    #[serde(default)]
    pub tools: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ImportPhaseDto {
    Admitting,
    Applying,
}

/// Progress tick streamed before each phase of each imported group.
#[derive(Debug, Clone, Serialize, Type)]
pub struct ImportProgressDto {
    pub index: u32,
    pub total: u32,
    pub group_name: String,
    pub phase: ImportPhaseDto,
}

/// What happened to one original directory (auto-sync off). `kept_divergent`
/// means the directory's content differs from the imported skill, so it was
/// deliberately left in place — report data, not a command error.
#[derive(Debug, Serialize, Type)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ImportOriginalStatusDto {
    Removed,
    KeptDivergent,
    Failed { error: CommandError },
}

#[derive(Debug, Serialize, Type)]
pub struct ImportOriginalDto {
    pub path: String,
    pub tool: String,
    pub status: ImportOriginalStatusDto,
}

/// Per-group result. `targets` carries the sync outcomes (auto-sync on) and
/// `originals` the settled originals (auto-sync off); the other is empty.
#[derive(Debug, Serialize, Type)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ImportGroupStatusDto {
    Imported {
        skill_id: String,
        skill_name: String,
        targets: Vec<SyncTargetResultDto>,
        originals: Vec<ImportOriginalDto>,
    },
    Failed {
        error: CommandError,
    },
}

#[derive(Debug, Serialize, Type)]
pub struct ImportGroupOutcomeDto {
    pub group_name: String,
    pub status: ImportGroupStatusDto,
}

#[derive(Debug, Serialize, Type)]
pub struct ImportReportDto {
    pub groups: Vec<ImportGroupOutcomeDto>,
    pub imported: u32,
    pub failed: u32,
}

/// Import pre-existing Tool skills the operator selected, in one call: admit
/// each chosen variant, finalize it as a Managed skill, then sync it
/// (auto-sync on) or remove the byte-identical originals (auto-sync off).
/// Per-group, per-target and per-original failures are report data.
#[tauri::command]
#[specta::specta]
pub async fn import_onboarding_selection(
    app: tauri::AppHandle,
    store: State<'_, SkillStore>,
    selections: Vec<OnboardingSelectionDto>,
    policy: ImportPolicyDto,
    on_progress: Channel<ImportProgressDto>,
) -> Result<ImportReportDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let paths = installer_paths(&app, &store)?;
        let selections: Vec<ImportSelection> = selections
            .into_iter()
            .map(|s| ImportSelection {
                group_name: s.group_name,
                chosen_path: PathBuf::from(s.chosen_path),
                name: s.name,
            })
            .collect();
        let report = import_onboarding_selection_core(
            &paths,
            &store,
            &selections,
            &ImportPolicy {
                auto_sync: policy.auto_sync,
                tools: policy.tools,
            },
            now_ms(),
            |p| {
                let _ = on_progress.send(ImportProgressDto {
                    index: p.index as u32,
                    total: p.total as u32,
                    group_name: p.group_name.to_string(),
                    phase: match p.phase {
                        ImportPhase::Admitting => ImportPhaseDto::Admitting,
                        ImportPhase::Applying => ImportPhaseDto::Applying,
                    },
                });
            },
        )?;
        Ok::<_, anyhow::Error>(to_import_report_dto(report))
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

fn to_import_report_dto(report: crate::core::onboarding_import::ImportReport) -> ImportReportDto {
    let mut dto = ImportReportDto {
        groups: Vec::with_capacity(report.groups.len()),
        imported: 0,
        failed: 0,
    };
    for group in report.groups {
        let status = match group.status {
            ImportGroupStatus::Imported {
                skill_id,
                skill_name,
                targets,
                originals,
            } => {
                dto.imported += 1;
                ImportGroupStatusDto::Imported {
                    skill_id,
                    skill_name,
                    targets: targets.into_iter().map(to_sync_target_result_dto).collect(),
                    originals: originals
                        .into_iter()
                        .map(|original| ImportOriginalDto {
                            path: original.path.to_string_lossy().to_string(),
                            tool: original.tool,
                            status: match original.status {
                                OriginalStatus::Removed => ImportOriginalStatusDto::Removed,
                                OriginalStatus::KeptDivergent => {
                                    ImportOriginalStatusDto::KeptDivergent
                                }
                                OriginalStatus::Failed { error } => {
                                    ImportOriginalStatusDto::Failed {
                                        error: CommandError::from_anyhow(error),
                                    }
                                }
                            },
                        })
                        .collect(),
                }
            }
            ImportGroupStatus::Failed { error } => {
                dto.failed += 1;
                ImportGroupStatusDto::Failed {
                    error: CommandError::from_anyhow(error),
                }
            }
        };
        dto.groups.push(ImportGroupOutcomeDto {
            group_name: group.group_name,
            status,
        });
    }
    dto
}

#[derive(Debug, Serialize, Type)]
pub struct ManagedSkillDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub source_type: String,
    pub source_ref: Option<String>,
    pub central_path: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_sync_at: Option<i64>,
    pub status: String,
    /// Who may invoke the skill, read from the central copy's `SKILL.md`
    /// frontmatter at list time (not persisted).
    pub invocation_mode: InvocationMode,
    pub targets: Vec<SkillTargetDto>,
}

#[derive(Debug, Serialize, Type)]
pub struct SkillTargetDto {
    pub tool: String,
    pub mode: SyncMode,
    pub status: SyncStatus,
    pub target_path: String,
    pub synced_at: Option<i64>,
}

#[tauri::command]
#[specta::specta]
pub fn get_managed_skills(
    store: State<'_, SkillStore>,
) -> Result<Vec<ManagedSkillDto>, CommandError> {
    let catalog = managed_skill_catalog(store.inner()).map_err(CommandError::from_anyhow)?;
    Ok(catalog.into_iter().map(ManagedSkillDto::from).collect())
}

#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn delete_managed_skill(
    store: State<'_, SkillStore>,
    skillId: String,
) -> Result<(), CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let report = crate::core::artifact_removal::remove_skill(&store, &skillId)?;
        log::debug!("[delete_managed_skill] {}", report);
        Ok::<_, anyhow::Error>(())
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

fn to_install_dto(result: InstallResult) -> InstallResultDto {
    InstallResultDto {
        skill_id: result.skill_id,
        name: result.name,
        central_path: result.central_path.to_string_lossy().to_string(),
        content_hash: result.content_hash,
    }
}

impl From<ManagedSkillEntry> for ManagedSkillDto {
    fn from(entry: ManagedSkillEntry) -> Self {
        let ManagedSkillEntry {
            skill,
            invocation_mode,
            targets,
        } = entry;
        ManagedSkillDto {
            id: skill.id,
            name: skill.name,
            description: skill.description,
            source_type: skill.source_type,
            source_ref: skill.source_ref,
            central_path: skill.central_path,
            created_at: skill.created_at,
            updated_at: skill.updated_at,
            last_sync_at: skill.last_sync_at,
            status: skill.status,
            invocation_mode,
            targets: targets
                .into_iter()
                .map(|target| SkillTargetDto {
                    tool: target.tool,
                    mode: target.mode,
                    status: target.status,
                    target_path: target.target_path,
                    synced_at: target.synced_at,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize, Type)]
pub struct FeaturedSkillDto {
    pub slug: String,
    pub name: String,
    pub summary: String,
    pub downloads: u64,
    pub stars: u64,
    pub source_url: String,
}

impl From<FeaturedSkill> for FeaturedSkillDto {
    fn from(s: FeaturedSkill) -> Self {
        Self {
            slug: s.slug,
            name: s.name,
            summary: s.summary,
            downloads: s.downloads,
            stars: s.stars,
            source_url: s.source_url,
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_featured_skills(
    store: State<'_, SkillStore>,
) -> Result<Vec<FeaturedSkillDto>, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let skills = fetch_featured_skills(&store)?;
        Ok::<_, anyhow::Error>(skills.into_iter().map(FeaturedSkillDto::from).collect())
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[derive(Debug, Serialize, Type)]
pub struct OnlineSkillDto {
    pub name: String,
    pub installs: u64,
    pub source: String,
    pub source_url: String,
}

impl From<OnlineSkillResult> for OnlineSkillDto {
    fn from(r: OnlineSkillResult) -> Self {
        Self {
            name: r.name,
            installs: r.installs,
            source: r.source,
            source_url: r.source_url,
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn search_skills_online(
    query: String,
    limit: Option<u32>,
) -> Result<Vec<OnlineSkillDto>, CommandError> {
    let limit = limit.unwrap_or(20) as usize;
    tauri::async_runtime::spawn_blocking(move || {
        let results = search_skills_online_core(&query, limit)?;
        Ok::<_, anyhow::Error>(results.into_iter().map(OnlineSkillDto::from).collect())
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SkillFileEntry {
    pub path: String,
    pub size: u64,
}

#[tauri::command]
#[specta::specta]
pub async fn list_skill_files(central_path: String) -> Result<Vec<SkillFileEntry>, CommandError> {
    let path = std::path::PathBuf::from(&central_path);
    tauri::async_runtime::spawn_blocking(move || {
        let entries = crate::core::skill_files::list_files(&path)?;
        Ok::<_, anyhow::Error>(
            entries
                .into_iter()
                .map(|e| SkillFileEntry {
                    path: e.path,
                    size: e.size,
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
pub async fn read_skill_file(
    central_path: String,
    file_path: String,
) -> Result<String, CommandError> {
    let base = std::path::PathBuf::from(&central_path);
    tauri::async_runtime::spawn_blocking(move || {
        crate::core::skill_files::read_file(&base, &file_path)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn clone_explore_skill(
    sourceUrl: String,
    skillName: Option<String>,
    app: tauri::AppHandle,
    store: State<'_, SkillStore>,
    cancel: State<'_, Arc<CancelToken>>,
) -> Result<String, CommandError> {
    let store = store.inner().clone();
    let cancel = cancel.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        cancel.reset();
        let paths = installer_paths(&app, &store).map_err(CommandError::from_anyhow)?;
        let path = clone_for_explore_preview(
            &paths,
            &store,
            &sourceUrl,
            skillName.as_deref(),
            Some(&cancel),
        )
        .map_err(CommandError::from_anyhow)?;
        Ok(path.to_string_lossy().to_string())
    })
    .await
    .map_err(CommandError::internal)?
}

#[tauri::command]
#[specta::specta]
pub fn cancel_current_operation(cancel: State<'_, Arc<CancelToken>>) -> Result<(), CommandError> {
    cancel.cancel();
    Ok(())
}

#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn hide_explore_skill(
    store: State<'_, SkillStore>,
    sourceUrl: String,
) -> Result<(), CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || store.hide_explore_skill(&sourceUrl))
        .await
        .map_err(CommandError::internal)?
        .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[specta::specta]
#[allow(non_snake_case)]
pub async fn unhide_explore_skill(
    store: State<'_, SkillStore>,
    sourceUrl: String,
) -> Result<(), CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || store.unhide_explore_skill(&sourceUrl))
        .await
        .map_err(CommandError::internal)?
        .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[specta::specta]
pub async fn get_hidden_explore_skills(
    store: State<'_, SkillStore>,
) -> Result<Vec<String>, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || store.list_hidden_explore_skills())
        .await
        .map_err(CommandError::internal)?
        .map_err(CommandError::from_anyhow)
}

#[cfg(test)]
#[path = "tests/commands.rs"]
mod tests;
