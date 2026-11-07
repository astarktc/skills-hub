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
//! `migrate_github_token_to_credential_store` moves a pre-1.2.18 row there,
//! deleting the row securely (zeroed + compacted). A compaction that fails
//! after the row's deletion committed is an error, and a non-secret
//! cleanup-pending marker (`SkillStore::delete_setting_securely`) makes
//! startup and the next token Save/Remove finish it.

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::credentials::{self, CredentialStore};
use super::environment::HostRoots;
use super::errors::SignalError;
use super::placement::{self, PlacementSettings, ScopeKind};
use super::skill_store::SkillStore;
use super::tool_adapters::{adapter_by_key, catalog_rows, global_tool_entries, installed_keys};

/// Storage keys — spelled here and nowhere else.
mod keys {
    pub const CENTRAL_REPO_PATH: &str = "central_repo_path";
    pub const GIT_CACHE_CLEANUP_DAYS: &str = "git_cache_cleanup_days";
    pub const GIT_CACHE_TTL_SECS: &str = "git_cache_ttl_secs";
    /// Pre-1.2.18 plaintext token row. Only the startup migration reads it;
    /// the token now lives in the `CredentialStore`.
    pub const LEGACY_GITHUB_TOKEN: &str = "github_token";
    pub const AUTO_SYNC_ENABLED: &str = "auto_sync_enabled";
    /// The saved selection as Deployment folder keys (1.4.0 on).
    pub const GLOBAL_SELECTED_FOLDERS: &str = "global_selected_folders_v2";
    /// The pre-1.4.0 selection of harness keys. Read only while
    /// `GLOBAL_SELECTED_FOLDERS` is absent, and translated to folder keys
    /// (`migrate_tool_selection_to_folders` writes the translation once);
    /// never written again, so a downgrade still finds it.
    pub const GLOBAL_SELECTED_TOOLS: &str = "global_selected_tools_v1";
    pub const SCAN_SELECTED_TOOLS_ONLY: &str = "scan_selected_tools_only";
    pub const UI_ZOOM_LEVEL: &str = "ui_zoom_level";
    pub const FEATURED_SKILLS_CACHE: &str = "featured_skills_cache";
    pub const INSTALLED_TOOLS: &str = "installed_tools_v1";
    /// Move Library folder's journal of a switched move whose Re-point and
    /// Clean have not settled yet (`core::library_move` owns its format).
    pub const LIBRARY_MOVE_JOURNAL: &str = "library_move_journal";
    /// The report of the last interrupted Library move a resume finished,
    /// until the frontend takes it (`core::library_move` owns its format).
    pub const LIBRARY_MOVE_RESUMED: &str = "library_move_resumed_report";
    /// The report of the redeploy journal finish the launch check ran, until
    /// the frontend takes it (`core::redeploy` owns its format).
    pub const REDEPLOY_FINISHED: &str = "redeploy_finished_report";
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
    /// `None` = never configured (distinct from an empty selection). The
    /// selection is stored as Deployment folder keys; this is every harness a
    /// selected folder serves (`core::placement`) — folder by folder in the
    /// stored order, each folder's harnesses in registry order — so two
    /// harnesses sharing one folder are on or off together. Keys the registry
    /// no longer knows are pruned on read (round 12 D2).
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
    // The Library folder is not a setting write: Move Library folder
    // (`core::library_move`) is its only writer, through `switch_library`.
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
    let global_selection = read_folder_selection(store)?;
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
        global_selected_tools: global_selection
            .configured()
            .map(|f| harnesses_served_by(&f)),
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
    let selection = read_folder_selection(store)?
        .configured()
        .map(|folders| harnesses_served_by(&folders));
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

/// The Deployment folders a global sync writes to, as folder keys: the
/// operator's recorded selection when they have configured one, otherwise the
/// folders serving every detected tool. Detection is a fallback for a
/// never-configured install, never an override of intent.
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
pub fn effective_global_tool_targets(store: &SkillStore, roots: &HostRoots) -> Result<Vec<String>> {
    Ok(match read_folder_selection(store)? {
        StoredSelection::Configured(selected) => selected,
        StoredSelection::Unconfigured => {
            folders_serving(&installed_keys(&global_tool_entries(roots)))
        }
        StoredSelection::Corrupt { key, detail } => bail!(SignalError::SettingCorrupt {
            key: key.to_string(),
            detail,
        }),
    })
}

/// The folder keys serving `harnesses` at global scope, in first-seen order —
/// a key the registry does not know is dropped (logged). A folder is on when
/// any harness it serves is named (spec Q26).
fn folders_serving(harnesses: &[String]) -> Vec<String> {
    let mut folders: Vec<String> = Vec::new();
    for harness in harnesses {
        match placement::serving_folder(ScopeKind::Global, harness, &PlacementSettings::default()) {
            Some(folder) if !folders.iter().any(|f| f == folder) => {
                folders.push(folder.to_string())
            }
            Some(_) => {}
            None => {
                log::warn!("[settings] dropping unknown tool key {harness:?} from the selection")
            }
        }
    }
    folders
}

/// Every harness a selected folder serves — folder by folder in the stored
/// order, each folder's harnesses in registry order: the harness-shaped
/// spelling of a folder selection the wire carries.
fn harnesses_served_by(folders: &[String]) -> Vec<String> {
    folders
        .iter()
        .flat_map(|folder| {
            catalog_rows().filter(move |adapter| {
                placement::serving_folder(
                    ScopeKind::Global,
                    adapter.key(),
                    &PlacementSettings::default(),
                ) == Some(folder.as_str())
            })
        })
        .map(|adapter| adapter.key().to_string())
        .collect()
}

/// Translate a pre-1.4.0 harness selection (`global_selected_tools_v1`) into
/// the folder selection once: when no folder selection is stored and the old
/// row parses, write its translation (a folder is on when any harness it
/// serves was selected). A corrupt old row is left alone — it keeps reading as
/// `SETTING_CORRUPT` until the operator saves the selection, which writes the
/// new key. Answers whether a translation was written. Runs at startup; the
/// reader translates on the fly until it has.
pub fn migrate_tool_selection_to_folders(store: &SkillStore) -> Result<bool> {
    if store.get_setting(keys::GLOBAL_SELECTED_FOLDERS)?.is_some() {
        return Ok(false);
    }
    match read_legacy_tool_selection(store)? {
        StoredSelection::Configured(folders) => {
            write_str(
                store,
                keys::GLOBAL_SELECTED_FOLDERS,
                &serde_json::to_string(&folders)?,
            )?;
            Ok(true)
        }
        StoredSelection::Unconfigured | StoredSelection::Corrupt { .. } => Ok(false),
    }
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
            // Stored as the folders serving the named harnesses (or folders).
            write_str(
                store,
                keys::GLOBAL_SELECTED_FOLDERS,
                &serde_json::to_string(&folders_serving(&selected_tools))?,
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
///
/// With no legacy row, the operation still finishes any secure cleanup an
/// earlier deletion left pending (`SkillStore::finish_pending_secure_cleanup`:
/// the row is gone but its compaction failed); a failure there fails the
/// operation too.
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
        // Compacts, which also completes any earlier pending cleanup.
        store.delete_setting_securely(keys::LEGACY_GITHUB_TOKEN)?;
    } else {
        store.finish_pending_secure_cleanup()?;
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
    /// compacted. Returned only when the compaction succeeded: if the row's
    /// deletion committed but the compaction failed, the migration is an
    /// error instead (the keychain holds the verified token, the row is gone,
    /// and a non-secret cleanup-pending marker makes the next startup's
    /// `SkillStore::finish_pending_secure_cleanup` — or the next token
    /// Save/Remove — finish it; that startup's migration then reports
    /// `NothingToMigrate`).
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
/// cleanup is an error, never `Migrated`, and is retried through the cleanup
/// marker (see `TokenMigration::Migrated`) — never by restoring the
/// plaintext row. Copies outside the live file —
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

/// Move Library folder's switch — the one writer of the Library folder
/// setting: in one transaction, point every moved skill (`(skill_id, new
/// central_path)`) at its new copy, record `new_root` as the configured
/// central repo, and record `journal` — what Re-point and Clean still owe —
/// so a crash after the switch is finished, never forgotten. Either all of
/// it lands or none of it does.
pub(crate) fn switch_library(
    store: &SkillStore,
    moves: &[(String, String)],
    new_root: &Path,
    journal: &str,
    now: i64,
) -> Result<()> {
    store.switch_library_records(
        moves,
        &[
            (keys::CENTRAL_REPO_PATH, new_root.to_string_lossy().as_ref()),
            (keys::LIBRARY_MOVE_JOURNAL, journal),
        ],
        now,
    )
}

/// The journal of a switched Library move whose Re-point and Clean have not
/// settled (see [`switch_library`]), if any.
pub(crate) fn pending_library_move(store: &SkillStore) -> Result<Option<String>> {
    store.get_setting(keys::LIBRARY_MOVE_JOURNAL)
}

/// The switched Library move's Re-point and Clean settled: drop its journal
/// — and, for a move a resume finished, record its report (`resumed`) in
/// the same transaction, so what the resume did is never lost between the
/// two.
pub(crate) fn finish_library_move(store: &SkillStore, resumed: Option<&str>) -> Result<()> {
    let record: Vec<(&str, &str)> = resumed
        .map(|report| (keys::LIBRARY_MOVE_RESUMED, report))
        .into_iter()
        .collect();
    store.write_settings(&[keys::LIBRARY_MOVE_JOURNAL], &record)
}

/// The report of a resumed Library move (see [`finish_library_move`]),
/// taken — read and removed in one transaction — if one is stored.
pub(crate) fn take_resumed_library_move(store: &SkillStore) -> Result<Option<String>> {
    store.take_setting(keys::LIBRARY_MOVE_RESUMED)
}

/// Record the report of the redeploy journal finish the launch check ran
/// (`core::redeploy::finish_journal`), replacing an earlier one nobody took.
pub(crate) fn record_redeploy_report(store: &SkillStore, report: &str) -> Result<()> {
    store.write_settings(&[], &[(keys::REDEPLOY_FINISHED, report)])
}

/// The recorded launch-check report (see [`record_redeploy_report`]), taken —
/// read and removed in one transaction — if one is stored.
pub(crate) fn take_redeploy_report(store: &SkillStore) -> Result<Option<String>> {
    store.take_setting(keys::REDEPLOY_FINISHED)
}

/// Test seam: overwrite a pending Library move's journal with `raw`, as an
/// edited or corrupted database would hold it.
#[cfg(test)]
pub(crate) fn overwrite_library_move_journal(store: &SkillStore, raw: &str) -> Result<()> {
    store.set_setting(keys::LIBRARY_MOVE_JOURNAL, raw)
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

/// A stored selection as read from its row. `Unconfigured` (absent or blank)
/// is the only state that may fall back to detection; `Configured` carries
/// folder keys only; `Corrupt` is a row that exists but is not a JSON string
/// array (`key` names that row).
#[derive(Debug, Clone, PartialEq, Eq)]
enum StoredSelection {
    Unconfigured,
    Configured(Vec<String>),
    Corrupt { key: &'static str, detail: String },
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

/// The one reader of the saved selection, as folder keys: the folder row when
/// it exists, else the pre-1.4.0 harness row translated
/// ([`read_legacy_tool_selection`]). Keys that are no global folder are
/// dropped here (logged once per read), so neither the modal nor a sync batch
/// ever sees a folder that does not exist; the operator's next save persists
/// the pruned set.
fn read_folder_selection(store: &SkillStore) -> Result<StoredSelection> {
    let key = keys::GLOBAL_SELECTED_FOLDERS;
    let Some(raw) = store.get_setting(key)? else {
        return read_legacy_tool_selection(store);
    };
    Ok(match parse_selection_row(key, &raw) {
        StoredSelection::Configured(stored) => {
            let (known, unknown): (Vec<String>, Vec<String>) = stored
                .into_iter()
                .partition(|folder| is_global_folder(folder));
            for folder in &unknown {
                log::warn!("[settings] dropping unknown folder key {folder:?} from {key}");
            }
            StoredSelection::Configured(known)
        }
        other => other,
    })
}

/// The pre-1.4.0 selection row (`global_selected_tools_v1`, harness keys),
/// translated to the folders serving them ([`folders_serving`]).
fn read_legacy_tool_selection(store: &SkillStore) -> Result<StoredSelection> {
    let key = keys::GLOBAL_SELECTED_TOOLS;
    let Some(raw) = store.get_setting(key)? else {
        return Ok(StoredSelection::Unconfigured);
    };
    Ok(match parse_selection_row(key, &raw) {
        StoredSelection::Configured(harnesses) => {
            StoredSelection::Configured(folders_serving(&harnesses))
        }
        other => other,
    })
}

fn parse_selection_row(key: &'static str, raw: &str) -> StoredSelection {
    if raw.trim().is_empty() {
        return StoredSelection::Unconfigured;
    }
    match serde_json::from_str::<Vec<String>>(raw) {
        Ok(keys) => StoredSelection::Configured(keys),
        Err(err) => StoredSelection::Corrupt {
            key,
            detail: err.to_string(),
        },
    }
}

/// Whether `key` is a global Deployment folder key: a folder serving some
/// registry entry.
fn is_global_folder(key: &str) -> bool {
    catalog_rows().any(|adapter| {
        placement::serving_folder(
            ScopeKind::Global,
            adapter.key(),
            &PlacementSettings::default(),
        ) == Some(key)
    })
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
