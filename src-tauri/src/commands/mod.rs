pub mod error;
pub mod projects;

use anyhow::Context;

use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::State;
use ts_rs::TS;

use std::sync::Arc;

use crate::core::cache_cleanup::{
    cleanup_git_cache_dirs, get_git_cache_cleanup_days as get_git_cache_cleanup_days_core,
    get_git_cache_ttl_secs as get_git_cache_ttl_secs_core,
    set_git_cache_cleanup_days as set_git_cache_cleanup_days_core,
    set_git_cache_ttl_secs as set_git_cache_ttl_secs_core,
};
use crate::core::cancel_token::CancelToken;
use crate::core::central_repo::{ensure_central_repo, resolve_central_repo_path};
use crate::core::errors::SignalError;
use crate::core::featured_skills::{fetch_featured_skills, FeaturedSkill};
use crate::core::github_search::{search_github_repos, RepoSummary};
use crate::core::global_sync::{BatchOverride, BatchPolicy, BatchSkill, BatchTargetStatus};
use crate::core::installer::{
    clone_for_explore_preview, install_git_skill, install_git_skill_from_selection,
    install_local_skill, install_local_skill_from_selection, list_git_skills, list_local_skills,
    update_managed_skill_from_source, GitSkillCandidate, InstallResult, LocalSkillCandidate,
};
use crate::core::onboarding::{build_onboarding_plan, OnboardingPlan};
use crate::core::skill_store::SkillStore;
use crate::core::skills_search::{
    search_skills_online as search_skills_online_core, OnlineSkillResult,
};
use crate::core::sync_engine::{copy_dir_recursive, remove_path_any};
use crate::core::tool_adapters::{
    adapters_sharing_skills_dir, default_tool_adapters, is_tool_installed, resolve_default_path,
    ToolId, AGENTS_STANDARD_KEYS,
};

pub use error::CommandError;

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ToolInfoDto {
    pub key: String,
    pub label: String,
    pub installed: bool,
    pub skills_dir: String,
    /// Keys of every global tool sharing this tool's skills dir, in adapter
    /// order, including this tool itself (len >= 1). The backend owns the
    /// shared-dir invariant; the frontend only presents it.
    pub shared_with: Vec<String>,
    /// Display labels of the constituent tools absorbed into this entry when
    /// it is a virtual group (project-scope AgentsStandard); empty for real
    /// tools. The backend owns group membership; the frontend only presents it.
    pub constituents: Vec<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ToolStatusDto {
    pub tools: Vec<ToolInfoDto>,
    pub installed: Vec<String>,
    pub newly_installed: Vec<String>,
}

#[tauri::command]
pub async fn get_tool_status(store: State<'_, SkillStore>) -> Result<ToolStatusDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let adapters = crate::core::tool_adapters::default_tool_adapters();
        let mut tools: Vec<ToolInfoDto> = Vec::new();
        let mut installed: Vec<String> = Vec::new();

        for adapter in &adapters {
            // AgentsStandard is project-only -- excluded from global tool list
            if adapter.id == ToolId::AgentsStandard {
                continue;
            }
            let ok = is_tool_installed(adapter)?;
            let key = adapter.id.as_key().to_string();
            let skills_dir = resolve_default_path(adapter)?.to_string_lossy().to_string();
            let shared_with: Vec<String> = adapters_sharing_skills_dir(adapter)
                .iter()
                .filter(|a| a.id != ToolId::AgentsStandard)
                .map(|a| a.id.as_key().to_string())
                .collect();
            tools.push(ToolInfoDto {
                key: key.clone(),
                label: adapter.display_name.to_string(),
                installed: ok,
                skills_dir,
                shared_with,
                constituents: vec![],
            });
            if ok {
                installed.push(key);
            }
        }

        installed.dedup();

        let prev: Vec<String> = store
            .get_setting("installed_tools_v1")?
            .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok())
            .unwrap_or_default();

        let prev_set: std::collections::HashSet<String> = prev.into_iter().collect();
        let newly_installed: Vec<String> = installed
            .iter()
            .filter(|k| !prev_set.contains(*k))
            .cloned()
            .collect();

        // Persist current set (best effort).
        let _ = store.set_setting(
            "installed_tools_v1",
            &serde_json::to_string(&installed).unwrap_or_else(|_| "[]".to_string()),
        );

        Ok::<_, anyhow::Error>(ToolStatusDto {
            tools,
            installed,
            newly_installed,
        })
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
pub async fn get_project_tool_status() -> Result<ToolStatusDto, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let adapters = default_tool_adapters();
        let mut tools: Vec<ToolInfoDto> = Vec::new();
        let mut installed: Vec<String> = Vec::new();

        for adapter in &adapters {
            let key = adapter.id.as_key();

            // Skip individual constituent tools -- they're absorbed into AgentsStandard
            if AGENTS_STANDARD_KEYS.contains(&key) {
                continue;
            }

            if adapter.id == ToolId::AgentsStandard {
                // Installed if ANY of the 9 constituent detect dirs exist
                let group_installed = adapters
                    .iter()
                    .filter(|a| AGENTS_STANDARD_KEYS.contains(&a.id.as_key()))
                    .any(|a| is_tool_installed(a).unwrap_or(false));

                let skills_dir = resolve_default_path(adapter)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();

                let constituents: Vec<String> = adapters
                    .iter()
                    .filter(|a| AGENTS_STANDARD_KEYS.contains(&a.id.as_key()))
                    .map(|a| a.display_name.to_string())
                    .collect();

                tools.push(ToolInfoDto {
                    key: "agents_skills".to_string(),
                    label: adapter.display_name.to_string(),
                    installed: group_installed,
                    skills_dir,
                    // Project scope: dir sharing is already absorbed into the
                    // single AgentsStandard entry, so every entry is its own
                    // group.
                    shared_with: vec!["agents_skills".to_string()],
                    constituents,
                });
                if group_installed {
                    installed.push("agents_skills".to_string());
                }
            } else {
                let ok = is_tool_installed(adapter)?;
                let key_str = key.to_string();
                let skills_dir = resolve_default_path(adapter)?.to_string_lossy().to_string();
                tools.push(ToolInfoDto {
                    key: key_str.clone(),
                    label: adapter.display_name.to_string(),
                    installed: ok,
                    skills_dir,
                    shared_with: vec![key_str.clone()],
                    constituents: vec![],
                });
                if ok {
                    installed.push(key_str);
                }
            }
        }

        installed.dedup();

        Ok::<_, anyhow::Error>(ToolStatusDto {
            tools,
            installed,
            newly_installed: vec![],
        })
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
pub async fn get_onboarding_plan(
    app: tauri::AppHandle,
    store: State<'_, SkillStore>,
) -> Result<OnboardingPlan, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || build_onboarding_plan(&app, &store))
        .await
        .map_err(CommandError::internal)?
        .map_err(CommandError::from_anyhow)
}

#[tauri::command]
pub async fn get_git_cache_cleanup_days(store: State<'_, SkillStore>) -> Result<i64, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        Ok::<_, anyhow::Error>(get_git_cache_cleanup_days_core(&store))
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
pub async fn set_git_cache_cleanup_days(
    store: State<'_, SkillStore>,
    days: i64,
) -> Result<i64, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || set_git_cache_cleanup_days_core(&store, days))
        .await
        .map_err(CommandError::internal)?
        .map_err(CommandError::from_anyhow)
}

#[tauri::command]
pub async fn clear_git_cache_now(app: tauri::AppHandle) -> Result<usize, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        cleanup_git_cache_dirs(&app, std::time::Duration::from_secs(0))
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
pub async fn get_git_cache_ttl_secs(store: State<'_, SkillStore>) -> Result<i64, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        Ok::<_, anyhow::Error>(get_git_cache_ttl_secs_core(&store))
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
pub async fn set_git_cache_ttl_secs(
    store: State<'_, SkillStore>,
    secs: i64,
) -> Result<i64, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || set_git_cache_ttl_secs_core(&store, secs))
        .await
        .map_err(CommandError::internal)?
        .map_err(CommandError::from_anyhow)
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct InstallResultDto {
    pub skill_id: String,
    pub name: String,
    pub central_path: String,
    pub content_hash: Option<String>,
}

pub(crate) fn expand_home_path(input: &str) -> Result<std::path::PathBuf, anyhow::Error> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        anyhow::bail!("storage path is empty");
    }
    if trimmed == "~" {
        let home = dirs::home_dir().context("failed to resolve home directory")?;
        return Ok(home);
    }
    if let Some(stripped) = trimmed.strip_prefix("~/") {
        let home = dirs::home_dir().context("failed to resolve home directory")?;
        return Ok(home.join(stripped));
    }
    Ok(std::path::PathBuf::from(trimmed))
}

#[tauri::command]
pub async fn get_central_repo_path(
    app: tauri::AppHandle,
    store: State<'_, SkillStore>,
) -> Result<String, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let path = resolve_central_repo_path(&app, &store)?;
        ensure_central_repo(&path)?;
        Ok::<_, anyhow::Error>(path.to_string_lossy().to_string())
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
pub async fn set_central_repo_path(
    app: tauri::AppHandle,
    store: State<'_, SkillStore>,
    path: String,
) -> Result<String, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let new_base = expand_home_path(&path)?;
        if !new_base.is_absolute() {
            anyhow::bail!("storage path must be absolute");
        }
        ensure_central_repo(&new_base)?;

        let current_base = resolve_central_repo_path(&app, &store)?;
        let skills = store.list_skills()?;
        if current_base == new_base {
            store.set_setting("central_repo_path", new_base.to_string_lossy().as_ref())?;
            return Ok::<_, anyhow::Error>(new_base.to_string_lossy().to_string());
        }

        if !skills.is_empty() {
            for skill in skills {
                let old_path = std::path::PathBuf::from(&skill.central_path);
                if !old_path.exists() {
                    anyhow::bail!("central path not found: {:?}", old_path);
                }
                let file_name = old_path
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("invalid central path: {:?}", old_path))?;
                let new_path = new_base.join(file_name);
                if new_path.exists() {
                    anyhow::bail!("target path already exists: {:?}", new_path);
                }

                if let Err(err) = std::fs::rename(&old_path, &new_path) {
                    copy_dir_recursive(&old_path, &new_path)
                        .with_context(|| format!("copy {:?} -> {:?}", old_path, new_path))?;
                    std::fs::remove_dir_all(&old_path)
                        .with_context(|| format!("cleanup {:?}", old_path))?;
                    // Surface rename error in logs for troubleshooting.
                    log::warn!("rename failed, fallback used: {}", err);
                }

                let mut updated = skill.clone();
                updated.central_path = new_path.to_string_lossy().to_string();
                updated.updated_at = now_ms();
                store.upsert_skill(&updated)?;
            }
        }

        store.set_setting("central_repo_path", new_base.to_string_lossy().as_ref())?;
        Ok::<_, anyhow::Error>(new_base.to_string_lossy().to_string())
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn install_local(
    app: tauri::AppHandle,
    store: State<'_, SkillStore>,
    sourcePath: String,
    name: Option<String>,
) -> Result<InstallResultDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = install_local_skill(&app, &store, sourcePath.as_ref(), name)?;
        Ok::<_, anyhow::Error>(to_install_dto(result))
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
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
        let result =
            install_local_skill_from_selection(&app, &store, base.as_ref(), &subpath, name)?;
        Ok::<_, anyhow::Error>(to_install_dto(result))
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn install_git(
    app: tauri::AppHandle,
    store: State<'_, SkillStore>,
    cancel: State<'_, Arc<CancelToken>>,
    repoUrl: String,
    name: Option<String>,
) -> Result<InstallResultDto, CommandError> {
    let store = store.inner().clone();
    cancel.reset();
    let cancel_token = Arc::clone(cancel.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let result = install_git_skill(&app, &store, &repoUrl, name, Some(&cancel_token))?;
        Ok::<_, anyhow::Error>(to_install_dto(result))
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn list_git_skills_cmd(
    app: tauri::AppHandle,
    store: State<'_, SkillStore>,
    repoUrl: String,
) -> Result<Vec<GitSkillCandidate>, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || list_git_skills(&app, &store, &repoUrl))
        .await
        .map_err(CommandError::internal)?
        .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn install_git_selection(
    app: tauri::AppHandle,
    store: State<'_, SkillStore>,
    repoUrl: String,
    subpath: String,
    name: Option<String>,
) -> Result<InstallResultDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = install_git_skill_from_selection(&app, &store, &repoUrl, &subpath, name)?;
        Ok::<_, anyhow::Error>(to_install_dto(result))
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

/// One skill in a batch sync request.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct BatchSyncSkillDto {
    pub skill_id: String,
    pub name: String,
    pub source_path: String,
}

/// Force-overwrite for one (skill, tool) pair; applies to any target tool
/// sharing the named tool's skills dir.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct BatchSyncOverrideDto {
    pub skill_id: String,
    pub tool: String,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
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
#[derive(Debug, Serialize, TS)]
#[serde(tag = "status", rename_all = "snake_case")]
#[ts(export)]
pub enum SyncTargetStatusDto {
    Synced { mode_used: String },
    Skipped { error: CommandError },
    Failed { error: CommandError },
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SyncTargetResultDto {
    pub skill_id: String,
    pub skill_name: String,
    pub tool: String,
    pub status: SyncTargetStatusDto,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct BatchSyncReportDto {
    pub results: Vec<SyncTargetResultDto>,
    pub synced: u32,
    pub skipped: u32,
    pub failed: u32,
}

/// Progress tick streamed over the command's channel before each attempted
/// (skill, tool) pair.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
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

        let outcomes = crate::core::global_sync::sync_skills_to_tools(
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
            let status = match outcome.status {
                BatchTargetStatus::Synced { outcome } => {
                    report.synced += 1;
                    SyncTargetStatusDto::Synced {
                        mode_used: outcome.mode_used.as_str().to_string(),
                    }
                }
                BatchTargetStatus::Skipped { error } => {
                    report.skipped += 1;
                    SyncTargetStatusDto::Skipped {
                        error: CommandError::from(error),
                    }
                }
                BatchTargetStatus::Failed { error } => {
                    report.failed += 1;
                    SyncTargetStatusDto::Failed {
                        error: CommandError::from(error),
                    }
                }
            };
            report.results.push(SyncTargetResultDto {
                skill_id: outcome.skill_id,
                skill_name: outcome.skill_name,
                tool: outcome.tool_key,
                status,
            });
        }
        Ok::<_, anyhow::Error>(report)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn unsync_skill_from_tool(
    store: State<'_, SkillStore>,
    skillId: String,
    tool: String,
) -> Result<(), CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        crate::core::global_sync::unsync_skill_from_tool_with_records(&store, &tool, &skillId)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct UpdateResultDto {
    pub skill_id: String,
    pub name: String,
    pub content_hash: Option<String>,
    pub source_revision: Option<String>,
    pub updated_targets: Vec<String>,
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn update_managed_skill(
    app: tauri::AppHandle,
    store: State<'_, SkillStore>,
    skillId: String,
) -> Result<UpdateResultDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let res = update_managed_skill_from_source(&app, &store, &skillId)?;
        Ok::<_, anyhow::Error>(UpdateResultDto {
            skill_id: res.skill_id,
            name: res.name,
            content_hash: res.content_hash,
            source_revision: res.source_revision,
            updated_targets: res.updated_targets,
        })
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
pub async fn search_github(
    store: State<'_, SkillStore>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<RepoSummary>, CommandError> {
    let store = store.inner().clone();
    let limit = limit.unwrap_or(10) as usize;
    tauri::async_runtime::spawn_blocking(move || {
        let token = store.get_setting("github_token")?.unwrap_or_default();
        let token_opt = if token.is_empty() {
            None
        } else {
            Some(token.as_str())
        };
        search_github_repos(&query, limit, token_opt)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
pub async fn get_github_token(store: State<'_, SkillStore>) -> Result<String, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        Ok::<_, anyhow::Error>(store.get_setting("github_token")?.unwrap_or_default())
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
pub async fn set_github_token(
    store: State<'_, SkillStore>,
    token: String,
) -> Result<(), CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            store.set_setting("github_token", "")?;
        } else {
            store.set_setting("github_token", trimmed)?;
        }
        Ok::<_, anyhow::Error>(())
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
pub async fn get_auto_sync_enabled(store: State<'_, SkillStore>) -> Result<bool, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let val = store
            .get_setting("auto_sync_enabled")?
            .unwrap_or_else(|| "true".to_string());
        Ok::<_, anyhow::Error>(val == "true")
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
pub async fn set_auto_sync_enabled(
    store: State<'_, SkillStore>,
    enabled: bool,
) -> Result<(), CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        store.set_setting("auto_sync_enabled", if enabled { "true" } else { "false" })?;
        Ok::<_, anyhow::Error>(())
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct GlobalToolConfigDto {
    pub selected_tools: Option<Vec<String>>,
    pub scan_selected_only: bool,
}

pub(crate) fn get_global_tool_config_impl(
    store: &SkillStore,
) -> anyhow::Result<GlobalToolConfigDto> {
    let selected_tools = store
        .get_setting("global_selected_tools_v1")?
        .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok());
    let scan_selected_only = store
        .get_setting("scan_selected_tools_only")?
        .map(|v| v == "true")
        .unwrap_or(true);
    Ok(GlobalToolConfigDto {
        selected_tools,
        scan_selected_only,
    })
}

pub(crate) fn set_global_tool_config_impl(
    store: &SkillStore,
    selected_tools: &[String],
    scan_selected_only: bool,
) -> anyhow::Result<()> {
    store.set_setting(
        "global_selected_tools_v1",
        &serde_json::to_string(selected_tools).unwrap_or_else(|_| "[]".to_string()),
    )?;
    store.set_setting(
        "scan_selected_tools_only",
        if scan_selected_only { "true" } else { "false" },
    )?;
    Ok(())
}

#[tauri::command]
pub async fn get_global_tool_config(
    store: State<'_, SkillStore>,
) -> Result<GlobalToolConfigDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || get_global_tool_config_impl(&store))
        .await
        .map_err(CommandError::internal)?
        .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn set_global_tool_config(
    store: State<'_, SkillStore>,
    selectedTools: Vec<String>,
    scanSelectedOnly: bool,
) -> Result<(), CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        set_global_tool_config_impl(&store, &selectedTools, scanSelectedOnly)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_ui_zoom_level(store: State<'_, SkillStore>) -> Result<f64, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let val = store
            .get_setting("ui_zoom_level")
            .map_err(CommandError::from_anyhow)?;
        Ok::<_, CommandError>(val.and_then(|v| v.parse::<f64>().ok()).unwrap_or(1.0))
    })
    .await
    .map_err(CommandError::internal)?
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn set_ui_zoom_level(
    store: State<'_, SkillStore>,
    zoomLevel: f64,
) -> Result<(), CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let clamped = zoomLevel.clamp(0.5, 3.0);
        store
            .set_setting("ui_zoom_level", &clamped.to_string())
            .map_err(CommandError::from_anyhow)
    })
    .await
    .map_err(CommandError::internal)?
}

#[tauri::command]
pub async fn unsync_all_skills(store: State<'_, SkillStore>) -> Result<usize, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let skills = store.list_skills()?;
        let mut removed_count: usize = 0;
        for skill in &skills {
            let targets = store.list_skill_targets(&skill.id)?;
            for target in &targets {
                let _ = crate::core::sync_engine::remove_path_any(std::path::Path::new(
                    &target.target_path,
                ));
            }
            removed_count += targets.len();
        }
        store.delete_all_skill_targets()?;
        Ok::<_, anyhow::Error>(removed_count)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn unsync_skill(
    store: State<'_, SkillStore>,
    skillId: String,
) -> Result<usize, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let targets = store.list_skill_targets(&skillId)?;
        for target in &targets {
            let _ = crate::core::sync_engine::remove_path_any(std::path::Path::new(
                &target.target_path,
            ));
        }
        let count = targets.len();
        store.delete_skill_targets(&skillId)?;
        Ok::<_, anyhow::Error>(count)
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn import_existing_skill(
    app: tauri::AppHandle,
    store: State<'_, SkillStore>,
    sourcePath: String,
    name: Option<String>,
) -> Result<InstallResultDto, CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let source = std::path::Path::new(&sourcePath);
        // Validate SKILL.md exists before importing (fixes #8: prevents importing
        // directories that were "discovered" but lack a valid SKILL.md).
        if !source.join("SKILL.md").exists() {
            anyhow::bail!(SignalError::SkillInvalid {
                reason: "missing_skill_md".to_string(),
            });
        }
        let result = install_local_skill(&app, &store, source, name)?;
        Ok::<_, anyhow::Error>(to_install_dto(result))
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[tauri::command]
pub async fn remove_skill_source(path: String) -> Result<(), CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let target = std::path::PathBuf::from(&path);

        // Safety: only allow deletion of paths under known tool skill directories.
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot resolve home directory"))?;
        let adapters = default_tool_adapters();
        let is_safe = adapters.iter().any(|adapter| {
            let tool_skills_dir = home.join(adapter.relative_skills_dir);
            target.starts_with(&tool_skills_dir)
        });
        if !is_safe {
            anyhow::bail!("path is not under a known tool skills directory: {}", path);
        }

        remove_path_any(&target)?;
        Ok::<_, anyhow::Error>(())
    })
    .await
    .map_err(CommandError::internal)?
    .map_err(CommandError::from_anyhow)
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
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
    pub targets: Vec<SkillTargetDto>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SkillTargetDto {
    pub tool: String,
    pub mode: String,
    pub status: String,
    pub target_path: String,
    pub synced_at: Option<i64>,
}

#[tauri::command]
pub fn get_managed_skills(
    store: State<'_, SkillStore>,
) -> Result<Vec<ManagedSkillDto>, CommandError> {
    get_managed_skills_impl(store.inner())
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn delete_managed_skill(
    store: State<'_, SkillStore>,
    skillId: String,
) -> Result<(), CommandError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        log::debug!("[delete_managed_skill] skillId={}", skillId);

        let record = store.get_skill_by_id(&skillId)?;
        let skill_name = record.as_ref().map(|s| s.name.clone());

        // Remove global tool directory symlinks/copies
        let targets = store.list_skill_targets(&skillId)?;
        let mut remove_failures: Vec<String> = Vec::new();
        for target in targets {
            if let Err(err) = remove_path_any(std::path::Path::new(&target.target_path)) {
                remove_failures.push(format!("{}: {}", target.target_path, err));
            }
        }

        // INFR-03: Clean up project directory artifacts before cascade delete
        if let Some(ref name) = skill_name {
            let project_assignments = store.list_project_skill_assignments_by_skill(&skillId)?;
            for assignment in &project_assignments {
                if assignment.status == "synced"
                    || assignment.status == "stale"
                    || assignment.status == "error"
                {
                    if let Ok(Some(project)) = store.get_project_by_id(&assignment.project_id) {
                        if let Some(adapter) =
                            crate::core::tool_adapters::adapter_by_key(&assignment.tool)
                        {
                            let project_path = std::path::Path::new(&project.path);
                            let target = project_path
                                .join(crate::core::tool_adapters::project_relative_skills_dir(
                                    &adapter,
                                ))
                                .join(name);
                            if let Err(e) = remove_path_any(&target) {
                                remove_failures.push(format!("{}: {}", target.display(), e));
                            }
                        }
                    }
                }
            }
        }

        if let Some(skill) = record {
            let path = std::path::PathBuf::from(skill.central_path);
            if path.exists() {
                std::fs::remove_dir_all(&path)?;
            }
            store.delete_skill(&skillId)?;
        }

        if !remove_failures.is_empty() {
            // Typed condition; user copy lives in the frontend catalog.
            anyhow::bail!(crate::core::errors::SignalError::DeleteCleanupFailed {
                failures: remove_failures,
            });
        }

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

pub(crate) fn now_ms() -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    now.as_millis() as i64
}

fn get_managed_skills_impl(store: &SkillStore) -> Result<Vec<ManagedSkillDto>, CommandError> {
    let skills = store.list_skills().map_err(CommandError::internal)?;
    Ok(skills
        .into_iter()
        .map(|skill| {
            let targets = store
                .list_skill_targets(&skill.id)
                .unwrap_or_default()
                .into_iter()
                .map(|target| SkillTargetDto {
                    tool: target.tool,
                    mode: target.mode,
                    status: target.status,
                    target_path: target.target_path,
                    synced_at: target.synced_at,
                })
                .collect();

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
                targets,
            }
        })
        .collect())
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
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

#[derive(Debug, Serialize, TS)]
#[ts(export)]
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

#[derive(Debug, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SkillFileEntry {
    pub path: String,
    pub size: u64,
}

#[tauri::command]
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
        let path = clone_for_explore_preview(
            &app,
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
pub fn cancel_current_operation(cancel: State<'_, Arc<CancelToken>>) -> Result<(), CommandError> {
    cancel.cancel();
    Ok(())
}

#[tauri::command]
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
