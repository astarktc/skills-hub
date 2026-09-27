//! Typed settings policy over the `settings` key/value table.
//!
//! `SkillStore::get_setting` / `set_setting` are the raw SQLite adapter; this
//! module is the only caller. It owns every storage key, the parse /
//! default / bound rule for each setting, the `AppSettings` DTO the frontend
//! reads (bounds included, so the UI clamps from data), and the
//! `SettingUpdate` command the frontend writes. Malformed or legacy stored
//! values parse to the setting's default — never an error — with one
//! exception: the global tool selection drives sync writes, so a corrupt row
//! is a typed refusal (`SignalError::SettingCorrupt`) on the write-driving
//! read, not a silent fallback to "every detected tool" (round 12 D1).
//!
//! Core never reads the environment: callers pass `fallback_root` (the
//! operator's home in production) for the central repo default, and the
//! `CredentialStore` the GitHub token lives in (the OS keychain in
//! production). The token is the one setting not kept in the settings table;
//! `migrate_github_token_to_credential_store` moves a pre-1.2.18 row there.

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::central_repo::{ensure_central_repo, move_central_repo};
use super::credentials::{self, CredentialStore};
use super::errors::SignalError;
use super::skill_store::SkillStore;
use super::tool_adapters::{adapter_by_key, global_tool_entries, installed_keys};

/// Storage keys — spelled here and nowhere else.
mod keys {
    pub const CENTRAL_REPO_PATH: &str = "central_repo_path";
    pub const GIT_CACHE_CLEANUP_DAYS: &str = "git_cache_cleanup_days";
    pub const GIT_CACHE_TTL_SECS: &str = "git_cache_ttl_secs";
    /// Pre-1.2.18 plaintext token row. Only the startup migration reads it;
    /// the token now lives in the `CredentialStore`.
    pub const LEGACY_GITHUB_TOKEN: &str = "github_token";
    pub const AUTO_SYNC_ENABLED: &str = "auto_sync_enabled";
    pub const GLOBAL_SELECTED_TOOLS: &str = "global_selected_tools_v1";
    pub const SCAN_SELECTED_TOOLS_ONLY: &str = "scan_selected_tools_only";
    pub const UI_ZOOM_LEVEL: &str = "ui_zoom_level";
    pub const FEATURED_SKILLS_CACHE: &str = "featured_skills_cache";
    pub const INSTALLED_TOOLS: &str = "installed_tools_v1";
}

/// Default central repo dir name under `fallback_root`.
const CENTRAL_DIR_NAME: &str = ".skillshub";

pub const DEFAULT_GIT_CACHE_CLEANUP_DAYS: i64 = 30;
pub const DEFAULT_GIT_CACHE_TTL_SECS: i64 = 60;
pub const DEFAULT_AUTO_SYNC_ENABLED: bool = true;
pub const DEFAULT_SCAN_SELECTED_TOOLS_ONLY: bool = true;
pub const DEFAULT_UI_ZOOM_LEVEL: f64 = 1.0;

/// Inclusive integer bound, shipped to the frontend inside `AppSettings`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
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
///
/// specta types `f64` as `number | null` because serde_json writes
/// NaN/±Infinity as `null`; these bounds are finite constants, so the
/// field-level override asserts the plain `number` the frontend clamps with.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
pub struct FloatRange {
    #[specta(type = specta_typescript::Number)]
    pub min: f64,
    #[specta(type = specta_typescript::Number)]
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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct AppSettings {
    /// Resolved central skills repo root (override or default).
    pub central_repo_path: String,
    pub git_cache_cleanup_days: i64,
    pub git_cache_ttl_secs: i64,
    /// Whether a GitHub token is stored: `Some(true)` set, `Some(false)`
    /// unset, `None` when the credential store could not be read (logged) —
    /// unreadable is not absent, and the rest of Settings still loads. The
    /// secret itself never crosses the wire.
    pub github_token_set: Option<bool>,
    pub auto_sync_enabled: bool,
    /// `None` = never configured (distinct from an empty selection). Keys
    /// the Tool registry no longer knows are pruned on read (round 12 D2).
    pub global_selected_tools: Option<Vec<String>>,
    /// True when the stored selection exists but could not be parsed. The
    /// selection above is then `None` for display only: a global sync refuses
    /// with `SETTING_CORRUPT` until the operator saves the selection again,
    /// which rewrites the row (round 12 D1).
    pub global_selected_tools_corrupt: bool,
    pub scan_selected_tools_only: bool,
    /// Always finite (clamped into `UI_ZOOM_LEVEL_RANGE`), hence `number`
    /// rather than specta's default `number | null` for `f64`.
    #[specta(type = specta_typescript::Number)]
    pub ui_zoom_level: f64,
    pub bounds: SettingsBounds,
}

/// One setting write, as sent by the frontend: `{ key, value }`.
#[derive(Debug, Clone, PartialEq, Deserialize, Type)]
#[serde(tag = "key", content = "value", rename_all = "snake_case")]
pub enum SettingUpdate {
    /// Absolute path; `~` expansion happens at the command seam. Moving the
    /// repo relocates every managed skill directory.
    CentralRepoPath(String),
    /// Clamped into `GIT_CACHE_CLEANUP_DAYS_RANGE`.
    GitCacheCleanupDays(i64),
    /// Clamped into `GIT_CACHE_TTL_SECS_RANGE`.
    GitCacheTtlSecs(i64),
    /// Trimmed and written to the `CredentialStore`; blank clears the token.
    GithubToken(String),
    AutoSyncEnabled(bool),
    GlobalToolConfig {
        selected_tools: Vec<String>,
        scan_selected_only: bool,
    },
    /// Clamped into `UI_ZOOM_LEVEL_RANGE`; non-finite falls back to default.
    /// Typed `number` (not specta's `number | null` for `f64`): `null` would
    /// fail deserialization, so it is not a value the frontend may send.
    UiZoomLevel(#[specta(type = specta_typescript::Number)] f64),
}

// ---------------------------------------------------------------------------
// Load
// ---------------------------------------------------------------------------

/// Read and parse every setting. Only a storage failure is an error;
/// malformed values parse to their defaults.
pub fn load_settings(
    store: &SkillStore,
    credentials: &dyn CredentialStore,
    fallback_root: &Path,
) -> Result<AppSettings> {
    let global_selection = read_tool_selection(store, keys::GLOBAL_SELECTED_TOOLS)?;
    Ok(AppSettings {
        central_repo_path: resolve_central_repo_path(store, fallback_root)?
            .to_string_lossy()
            .to_string(),
        git_cache_cleanup_days: git_cache_cleanup_days(store),
        git_cache_ttl_secs: git_cache_ttl_secs(store),
        github_token_set: match github_token(credentials) {
            Ok(token) => Some(token.is_some()),
            Err(err) => {
                log::warn!("[settings] cannot read GitHub token presence: {err:#}");
                None
            }
        },
        auto_sync_enabled: read_bool(store, keys::AUTO_SYNC_ENABLED, DEFAULT_AUTO_SYNC_ENABLED)?,
        global_selected_tools: global_selection.configured(),
        global_selected_tools_corrupt: matches!(global_selection, StoredSelection::Corrupt { .. }),
        scan_selected_tools_only: read_bool(
            store,
            keys::SCAN_SELECTED_TOOLS_ONLY,
            DEFAULT_SCAN_SELECTED_TOOLS_ONLY,
        )?,
        ui_zoom_level: ui_zoom_level(store),
        bounds: BOUNDS,
    })
}

/// The saved Tool selection an onboarding scan is scoped to: `Some` when
/// "only scan selected tools" is on and a selection is saved; `None` means
/// every detected Tool (a corrupt selection reads as unsaved here — the sync
/// path refuses it separately). Reads only the settings table, never the
/// credential store.
pub fn onboarding_scan_selection(store: &SkillStore) -> Result<Option<Vec<String>>> {
    let selection = read_tool_selection(store, keys::GLOBAL_SELECTED_TOOLS)?.configured();
    let scan_selected_only = read_bool(
        store,
        keys::SCAN_SELECTED_TOOLS_ONLY,
        DEFAULT_SCAN_SELECTED_TOOLS_ONLY,
    )?;
    Ok(selection.filter(|_| scan_selected_only))
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

/// The same freshness window in milliseconds, the unit `core::git_cache`
/// takes. `0` keeps its meaning: never serve a cached clone.
pub fn git_cache_ttl_ms(store: &SkillStore) -> i64 {
    git_cache_ttl_secs(store).saturating_mul(1000)
}

/// GitHub token from the credential store, trimmed; `None` when unset or
/// blank. A store failure is `SignalError::CredentialStoreUnavailable`.
pub fn github_token(credentials: &dyn CredentialStore) -> Result<Option<String>> {
    Ok(credentials
        .get(credentials::keys::GITHUB_TOKEN)?
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty()))
}

/// Acquisition can continue unauthenticated when the token cannot be read.
/// Writes go through `apply_setting`, which surfaces store failures.
pub fn github_token_or_none(credentials: &dyn CredentialStore) -> Option<String> {
    match github_token(credentials) {
        Ok(token) => token,
        Err(err) => {
            log::warn!("[settings] cannot read GitHub token; continuing unauthenticated: {err:#}");
            None
        }
    }
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

/// The tools a global sync writes to: the operator's recorded selection when
/// they have configured one, otherwise every detected tool. Detection is a
/// fallback for a never-configured install, never an override of intent.
///
/// An empty selection is a selection — `Some(vec![])` means "sync nowhere",
/// deliberately distinct from `None`. The set is **not** intersected with
/// detection either: a selected-but-uninstalled key stays in it and is
/// reported downstream as a skip (`GlobalSyncError::ToolNotInstalled`), which
/// is the operator's only signal that their selection has gone stale.
///
/// A stored selection that cannot be parsed is a refusal
/// (`SignalError::SettingCorrupt`), never a fallback to detection: the
/// operator repairs it by saving the selection again.
pub fn effective_global_tool_targets(store: &SkillStore, home: &Path) -> Result<Vec<String>> {
    Ok(
        match read_tool_selection(store, keys::GLOBAL_SELECTED_TOOLS)? {
            StoredSelection::Configured(selected) => selected,
            StoredSelection::Unconfigured => installed_keys(&global_tool_entries(home)),
            StoredSelection::Corrupt { detail } => bail!(SignalError::SettingCorrupt {
                key: keys::GLOBAL_SELECTED_TOOLS.to_string(),
                detail,
            }),
        },
    )
}

// ---------------------------------------------------------------------------
// Apply
// ---------------------------------------------------------------------------

/// Persist one setting (normalised into its bounds) and return the resulting
/// full snapshot so the caller adopts the effective value, not the requested
/// one.
pub fn apply_setting(
    store: &SkillStore,
    credentials: &dyn CredentialStore,
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
        SettingUpdate::GithubToken(token) => set_github_token(store, credentials, &token)?,
        SettingUpdate::AutoSyncEnabled(enabled) => {
            write_bool(store, keys::AUTO_SYNC_ENABLED, enabled)?;
        }
        SettingUpdate::GlobalToolConfig {
            selected_tools,
            scan_selected_only,
        } => {
            // The UI only offers registry keys; the guard keeps the stored
            // row honest for any other caller (round 12 D2).
            if let Some(tool) = selected_tools
                .iter()
                .find(|key| adapter_by_key(key).is_none())
            {
                bail!(SignalError::UnknownTool { tool: tool.clone() });
            }
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
    load_settings(store, credentials, fallback_root)
}

/// Save (non-blank, trimmed) or remove (blank) the GitHub token, then settle
/// any pending legacy settings row in the same operation: a row a failed
/// startup migration left behind would otherwise overwrite this explicit
/// choice on the next launch (or keep the old plaintext around).
///
/// Replacement: `set`, then — only when a legacy row is pending — read the
/// new value back and securely delete the row. Removal: `delete`, then
/// securely delete any pending row. Every failure propagates; in particular
/// a row that cannot be removed fails the operation even though the
/// keychain already holds what the operator asked for (the next launch's
/// migration would then still act on the row — the remaining risk, reported
/// rather than hidden). With no legacy row, a read-back failure is not an
/// error here: `load_settings` reports presence as unknown (`None`).
fn set_github_token(
    store: &SkillStore,
    credentials: &dyn CredentialStore,
    token: &str,
) -> Result<()> {
    let legacy_row_pending = store.get_setting(keys::LEGACY_GITHUB_TOKEN)?.is_some();
    match token.trim() {
        "" => credentials.delete(credentials::keys::GITHUB_TOKEN)?,
        token => {
            credentials.set(credentials::keys::GITHUB_TOKEN, token)?;
            if legacy_row_pending {
                verify_stored_github_token(credentials, token)?;
            }
        }
    }
    if legacy_row_pending {
        store.delete_setting_securely(keys::LEGACY_GITHUB_TOKEN)?;
    }
    Ok(())
}

/// Read the token back and require it to equal what was just written.
fn verify_stored_github_token(credentials: &dyn CredentialStore, expected: &str) -> Result<()> {
    if github_token(credentials)?.as_deref() != Some(expected) {
        bail!(SignalError::CredentialStoreUnavailable {
            detail: "token read back from the credential store did not match the write".to_string(),
        });
    }
    Ok(())
}

/// Outcome of the one-time move of the plaintext token row to the
/// credential store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenMigration {
    /// No legacy row: already migrated, or never set.
    NothingToMigrate,
    /// The row held a token; it is now in the credential store (read back
    /// and verified) and the row is gone — securely deleted, database
    /// compacted.
    Migrated,
    /// The row was blank; it was deleted without touching the store.
    DroppedBlank,
}

/// Move a pre-1.2.18 `github_token` settings row into `credentials`.
/// Runs at every startup and is idempotent: the row is deleted only after the
/// store write has been read back unchanged, so any store failure (a denied
/// or locked Keychain prompt, no Secret Service) leaves the row untouched for
/// the next launch and surfaces `SignalError::CredentialStoreUnavailable`.
///
/// The row is removed with `SkillStore::delete_setting_securely`, so the
/// plaintext leaves the live database file (not just the table); a failed
/// cleanup is an error, never `Migrated`. Copies outside the live file —
/// backups, filesystem snapshots, storage remnants — are out of reach.
/// An explicit Save/Remove settles a pending row too (`set_github_token`).
pub fn migrate_github_token_to_credential_store(
    store: &SkillStore,
    credentials: &dyn CredentialStore,
) -> Result<TokenMigration> {
    let Some(raw) = store.get_setting(keys::LEGACY_GITHUB_TOKEN)? else {
        return Ok(TokenMigration::NothingToMigrate);
    };
    let token = raw.trim();
    if token.is_empty() {
        store.delete_setting_securely(keys::LEGACY_GITHUB_TOKEN)?;
        return Ok(TokenMigration::DroppedBlank);
    }
    credentials.set(credentials::keys::GITHUB_TOKEN, token)?;
    verify_stored_github_token(credentials, token)?;
    store.delete_setting_securely(keys::LEGACY_GITHUB_TOKEN)?;
    Ok(TokenMigration::Migrated)
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

/// Persist the currently installed global tool keys and report which of them
/// were not recorded before (the "newly installed" set the UI announces). A
/// malformed stored value counts as nothing recorded; the write is best
/// effort so a failed write never fails the status read.
pub fn record_installed_tools(store: &SkillStore, installed: &[String]) -> Result<Vec<String>> {
    let prev: std::collections::HashSet<String> = store
        .get_setting(keys::INSTALLED_TOOLS)?
        .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok())
        .unwrap_or_default()
        .into_iter()
        .collect();
    let newly_installed = installed
        .iter()
        .filter(|k| !prev.contains(*k))
        .cloned()
        .collect();
    let _ = write_str(
        store,
        keys::INSTALLED_TOOLS,
        &serde_json::to_string(installed).unwrap_or_else(|_| "[]".to_string()),
    );
    Ok(newly_installed)
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

/// A stored Tool selection as read from its row. `Unconfigured` (absent or
/// blank) is the only state that may fall back to detection; `Configured`
/// carries registry keys only; `Corrupt` is a row that exists but is not a
/// JSON string array.
#[derive(Debug, Clone, PartialEq, Eq)]
enum StoredSelection {
    Unconfigured,
    Configured(Vec<String>),
    Corrupt { detail: String },
}

impl StoredSelection {
    /// The display shape: `Corrupt` reads as `None`, the wire flag says why.
    fn configured(&self) -> Option<Vec<String>> {
        match self {
            StoredSelection::Configured(keys) => Some(keys.clone()),
            _ => None,
        }
    }
}

/// The one reader of a Tool-selection row. Keys the registry no longer knows
/// are dropped here (logged once per read), so neither the modal nor a sync
/// batch ever sees a tool that does not exist; the operator's next save
/// persists the pruned set.
fn read_tool_selection(store: &SkillStore, key: &str) -> Result<StoredSelection> {
    let Some(raw) = store.get_setting(key)? else {
        return Ok(StoredSelection::Unconfigured);
    };
    if raw.trim().is_empty() {
        return Ok(StoredSelection::Unconfigured);
    }
    let keys = match serde_json::from_str::<Vec<String>>(&raw) {
        Ok(keys) => keys,
        Err(err) => {
            return Ok(StoredSelection::Corrupt {
                detail: err.to_string(),
            })
        }
    };
    let (known, unknown): (Vec<String>, Vec<String>) = keys
        .into_iter()
        .partition(|tool| adapter_by_key(tool).is_some());
    for tool in &unknown {
        log::warn!("[settings] dropping unknown tool key {tool:?} from {key}");
    }
    Ok(StoredSelection::Configured(known))
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
