//! Typed settings policy over the `settings` key/value table.
//!
//! `SkillStore::get_setting` / `set_setting` are the raw SQLite adapter; this
//! module is the only caller. It owns every storage key, the parse /
//! default / bound rule for each setting, the `AppSettings` DTO the frontend
//! reads (bounds included, so the UI clamps from data), and the
//! `SettingUpdate` command the frontend writes. Malformed or legacy stored
//! values always parse to the setting's default — never an error.
//!
//! Core never reads the environment: callers pass `fallback_root` (the
//! operator's home in production) for the central repo default.

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::central_repo::{ensure_central_repo, move_central_repo};
use super::skill_store::SkillStore;

/// Storage keys — spelled here and nowhere else.
mod keys {
    pub const CENTRAL_REPO_PATH: &str = "central_repo_path";
    pub const GIT_CACHE_CLEANUP_DAYS: &str = "git_cache_cleanup_days";
    pub const GIT_CACHE_TTL_SECS: &str = "git_cache_ttl_secs";
    pub const GITHUB_TOKEN: &str = "github_token";
    pub const AUTO_SYNC_ENABLED: &str = "auto_sync_enabled";
    pub const GLOBAL_SELECTED_TOOLS: &str = "global_selected_tools_v1";
    pub const SCAN_SELECTED_TOOLS_ONLY: &str = "scan_selected_tools_only";
    pub const UI_ZOOM_LEVEL: &str = "ui_zoom_level";
    pub const FEATURED_SKILLS_CACHE: &str = "featured_skills_cache";
}

/// Default central repo dir name under `fallback_root`.
const CENTRAL_DIR_NAME: &str = ".skillshub";

pub const DEFAULT_GIT_CACHE_CLEANUP_DAYS: i64 = 30;
pub const DEFAULT_GIT_CACHE_TTL_SECS: i64 = 60;
pub const DEFAULT_AUTO_SYNC_ENABLED: bool = true;
pub const DEFAULT_SCAN_SELECTED_TOOLS_ONLY: bool = true;
pub const DEFAULT_UI_ZOOM_LEVEL: f64 = 1.0;

/// Inclusive integer bound, shipped to the frontend inside `AppSettings`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct IntRange {
    pub min: i64,
    pub max: i64,
}

impl IntRange {
    fn contains(&self, v: i64) -> bool {
        (self.min..=self.max).contains(&v)
    }
    fn clamp(&self, v: i64) -> i64 {
        v.clamp(self.min, self.max)
    }
}

/// Inclusive float bound, shipped to the frontend inside `AppSettings`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct FloatRange {
    pub min: f64,
    pub max: f64,
}

impl FloatRange {
    fn contains(&self, v: f64) -> bool {
        v.is_finite() && (self.min..=self.max).contains(&v)
    }
}

pub const GIT_CACHE_CLEANUP_DAYS_RANGE: IntRange = IntRange { min: 0, max: 3650 };
pub const GIT_CACHE_TTL_SECS_RANGE: IntRange = IntRange { min: 0, max: 3600 };
pub const UI_ZOOM_LEVEL_RANGE: FloatRange = FloatRange { min: 0.5, max: 3.0 };

/// Every bound the frontend needs to clamp input before sending it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SettingsBounds {
    pub git_cache_cleanup_days: IntRange,
    pub git_cache_ttl_secs: IntRange,
    pub ui_zoom_level: FloatRange,
}

pub const BOUNDS: SettingsBounds = SettingsBounds {
    git_cache_cleanup_days: GIT_CACHE_CLEANUP_DAYS_RANGE,
    git_cache_ttl_secs: GIT_CACHE_TTL_SECS_RANGE,
    ui_zoom_level: UI_ZOOM_LEVEL_RANGE,
};

/// Snapshot of every backend-persisted setting, already parsed, defaulted
/// and (for the central repo path) resolved.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct AppSettings {
    /// Resolved central skills repo root (override or default).
    pub central_repo_path: String,
    pub git_cache_cleanup_days: i64,
    pub git_cache_ttl_secs: i64,
    /// Empty string when no token is stored.
    pub github_token: String,
    pub auto_sync_enabled: bool,
    /// `None` = never configured (distinct from an empty selection).
    pub global_selected_tools: Option<Vec<String>>,
    pub scan_selected_tools_only: bool,
    pub ui_zoom_level: f64,
    pub bounds: SettingsBounds,
}

/// One setting write, as sent by the frontend: `{ key, value }`.
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "key", content = "value", rename_all = "snake_case")]
pub enum SettingUpdate {
    /// Absolute path; `~` expansion happens at the command seam. Moving the
    /// repo relocates every managed skill directory.
    CentralRepoPath(String),
    /// Clamped into `GIT_CACHE_CLEANUP_DAYS_RANGE`.
    GitCacheCleanupDays(i64),
    /// Clamped into `GIT_CACHE_TTL_SECS_RANGE`.
    GitCacheTtlSecs(i64),
    /// Trimmed; blank clears the token.
    GithubToken(String),
    AutoSyncEnabled(bool),
    GlobalToolConfig {
        selected_tools: Vec<String>,
        scan_selected_only: bool,
    },
    /// Clamped into `UI_ZOOM_LEVEL_RANGE`; non-finite falls back to default.
    UiZoomLevel(f64),
}

// ---------------------------------------------------------------------------
// Load
// ---------------------------------------------------------------------------

/// Read and parse every setting. Only a storage failure is an error;
/// malformed values parse to their defaults.
pub fn load_settings(store: &SkillStore, fallback_root: &Path) -> Result<AppSettings> {
    Ok(AppSettings {
        central_repo_path: resolve_central_repo_path(store, fallback_root)?
            .to_string_lossy()
            .to_string(),
        git_cache_cleanup_days: git_cache_cleanup_days(store),
        git_cache_ttl_secs: git_cache_ttl_secs(store),
        github_token: github_token(store)?.unwrap_or_default(),
        auto_sync_enabled: read_bool(store, keys::AUTO_SYNC_ENABLED, DEFAULT_AUTO_SYNC_ENABLED)?,
        global_selected_tools: read_string_list(store, keys::GLOBAL_SELECTED_TOOLS)?,
        scan_selected_tools_only: read_bool(
            store,
            keys::SCAN_SELECTED_TOOLS_ONLY,
            DEFAULT_SCAN_SELECTED_TOOLS_ONLY,
        )?,
        ui_zoom_level: ui_zoom_level(store),
        bounds: BOUNDS,
    })
}

/// Resolve the central skills repo root: the explicit override wins;
/// otherwise `.skillshub` under `fallback_root` (the operator's home in
/// production, with the app data dir as a last resort — the command seam
/// picks it). A blank stored override counts as unset.
pub fn resolve_central_repo_path(store: &SkillStore, fallback_root: &Path) -> Result<PathBuf> {
    let stored = store.get_setting(keys::CENTRAL_REPO_PATH)?;
    Ok(match stored.as_deref().map(str::trim) {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        _ => fallback_root.join(CENTRAL_DIR_NAME),
    })
}

/// Days before an unused git cache clone is deleted; `0` disables cleanup.
/// Storage failures and malformed values read as the default.
pub fn git_cache_cleanup_days(store: &SkillStore) -> i64 {
    read_bounded_i64(
        store,
        keys::GIT_CACHE_CLEANUP_DAYS,
        GIT_CACHE_CLEANUP_DAYS_RANGE,
        DEFAULT_GIT_CACHE_CLEANUP_DAYS,
    )
}

/// Seconds a git cache clone is considered fresh (no re-fetch).
/// Storage failures and malformed values read as the default.
pub fn git_cache_ttl_secs(store: &SkillStore) -> i64 {
    read_bounded_i64(
        store,
        keys::GIT_CACHE_TTL_SECS,
        GIT_CACHE_TTL_SECS_RANGE,
        DEFAULT_GIT_CACHE_TTL_SECS,
    )
}

/// GitHub token, trimmed; `None` when unset or blank.
pub fn github_token(store: &SkillStore) -> Result<Option<String>> {
    Ok(store
        .get_setting(keys::GITHUB_TOKEN)?
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty()))
}

/// Webview zoom factor. Storage failures, malformed and out-of-range values
/// read as the default so a bad row can never render an unusable window.
pub fn ui_zoom_level(store: &SkillStore) -> f64 {
    store
        .get_setting(keys::UI_ZOOM_LEVEL)
        .ok()
        .flatten()
        .and_then(|raw| raw.trim().parse::<f64>().ok())
        .filter(|v| UI_ZOOM_LEVEL_RANGE.contains(*v))
        .unwrap_or(DEFAULT_UI_ZOOM_LEVEL)
}

// ---------------------------------------------------------------------------
// Apply
// ---------------------------------------------------------------------------

/// Persist one setting (normalised into its bounds) and return the resulting
/// full snapshot so the caller adopts the effective value, not the requested
/// one.
pub fn apply_setting(
    store: &SkillStore,
    fallback_root: &Path,
    update: SettingUpdate,
) -> Result<AppSettings> {
    match update {
        SettingUpdate::CentralRepoPath(path) => {
            set_central_repo_path(store, fallback_root, Path::new(&path))?;
        }
        SettingUpdate::GitCacheCleanupDays(days) => {
            write_str(
                store,
                keys::GIT_CACHE_CLEANUP_DAYS,
                &GIT_CACHE_CLEANUP_DAYS_RANGE.clamp(days).to_string(),
            )?;
        }
        SettingUpdate::GitCacheTtlSecs(secs) => {
            write_str(
                store,
                keys::GIT_CACHE_TTL_SECS,
                &GIT_CACHE_TTL_SECS_RANGE.clamp(secs).to_string(),
            )?;
        }
        SettingUpdate::GithubToken(token) => {
            write_str(store, keys::GITHUB_TOKEN, token.trim())?;
        }
        SettingUpdate::AutoSyncEnabled(enabled) => {
            write_bool(store, keys::AUTO_SYNC_ENABLED, enabled)?;
        }
        SettingUpdate::GlobalToolConfig {
            selected_tools,
            scan_selected_only,
        } => {
            write_str(
                store,
                keys::GLOBAL_SELECTED_TOOLS,
                &serde_json::to_string(&selected_tools)?,
            )?;
            write_bool(store, keys::SCAN_SELECTED_TOOLS_ONLY, scan_selected_only)?;
        }
        SettingUpdate::UiZoomLevel(level) => {
            let effective = if level.is_finite() {
                level.clamp(UI_ZOOM_LEVEL_RANGE.min, UI_ZOOM_LEVEL_RANGE.max)
            } else {
                DEFAULT_UI_ZOOM_LEVEL
            };
            write_str(store, keys::UI_ZOOM_LEVEL, &effective.to_string())?;
        }
    }
    load_settings(store, fallback_root)
}

/// Point the central repo at `new_base` (must be absolute), creating it and
/// relocating every managed skill directory when the root actually changes.
fn set_central_repo_path(store: &SkillStore, fallback_root: &Path, new_base: &Path) -> Result<()> {
    if !new_base.is_absolute() {
        anyhow::bail!("storage path must be absolute");
    }
    ensure_central_repo(new_base)?;

    let current_base = resolve_central_repo_path(store, fallback_root)?;
    if current_base != new_base {
        move_central_repo(store, new_base)?;
    }
    write_str(
        store,
        keys::CENTRAL_REPO_PATH,
        new_base.to_string_lossy().as_ref(),
    )
}

// ---------------------------------------------------------------------------
// Internal persisted state that is not a user setting (still keyed here so
// key naming lives in one place).
// ---------------------------------------------------------------------------

/// Last successfully fetched featured-skills payload (raw JSON), if any.
pub fn featured_skills_cache(store: &SkillStore) -> Option<String> {
    store
        .get_setting(keys::FEATURED_SKILLS_CACHE)
        .ok()
        .flatten()
}

pub fn set_featured_skills_cache(store: &SkillStore, json: &str) -> Result<()> {
    write_str(store, keys::FEATURED_SKILLS_CACHE, json)
}

// ---------------------------------------------------------------------------
// Parse / write primitives
// ---------------------------------------------------------------------------

fn read_bounded_i64(store: &SkillStore, key: &str, range: IntRange, default: i64) -> i64 {
    store
        .get_setting(key)
        .ok()
        .flatten()
        .and_then(|raw| raw.trim().parse::<i64>().ok())
        .filter(|v| range.contains(*v))
        .unwrap_or(default)
}

fn read_bool(store: &SkillStore, key: &str, default: bool) -> Result<bool> {
    Ok(store
        .get_setting(key)?
        .and_then(|raw| match raw.trim().to_ascii_lowercase().as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        })
        .unwrap_or(default))
}

fn read_string_list(store: &SkillStore, key: &str) -> Result<Option<Vec<String>>> {
    Ok(store
        .get_setting(key)?
        .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok()))
}

fn write_str(store: &SkillStore, key: &str, value: &str) -> Result<()> {
    store.set_setting(key, value)
}

fn write_bool(store: &SkillStore, key: &str, value: bool) -> Result<()> {
    write_str(store, key, if value { "true" } else { "false" })
}

#[cfg(test)]
#[path = "tests/settings.rs"]
mod tests;
