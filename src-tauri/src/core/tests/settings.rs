use std::path::PathBuf;

use crate::core::credentials::{CredentialStore, MemoryStore};
use crate::core::errors::SignalError;
use crate::core::settings::{
    apply_setting, effective_global_tool_targets, featured_skills_cache, git_cache_cleanup_days,
    git_cache_ttl_secs, github_token, load_settings, migrate_github_token_to_credential_store,
    onboarding_scan_selection, record_installed_tools, resolve_central_repo_path,
    set_featured_skills_cache, ui_zoom_level, SettingUpdate, TokenMigration,
    DEFAULT_AUTO_SYNC_ENABLED, DEFAULT_GIT_CACHE_CLEANUP_DAYS, DEFAULT_GIT_CACHE_TTL_SECS,
    DEFAULT_SCAN_SELECTED_TOOLS_ONLY, DEFAULT_UI_ZOOM_LEVEL, GIT_CACHE_CLEANUP_DAYS_RANGE,
    GIT_CACHE_TTL_SECS_RANGE, UI_ZOOM_LEVEL_RANGE,
};
use crate::core::skill_store::{SkillRecord, SkillStore};

fn make_store() -> (tempfile::TempDir, SkillStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    (dir, store)
}

/// Write a raw stored value, bypassing the policy layer, to simulate legacy
/// or hand-edited rows.
fn raw(store: &SkillStore, key: &str, value: &str) {
    store.set_setting(key, value).expect("raw set_setting");
}

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

#[test]
fn load_on_empty_store_yields_defaults_and_bounds() {
    let (dir, store) = make_store();
    let home = dir.path().join("home");
    let s = load_settings(&store, &MemoryStore::new(), &home).unwrap();

    assert_eq!(
        s.central_repo_path,
        home.join(".skillshub").to_string_lossy().to_string()
    );
    assert_eq!(s.git_cache_cleanup_days, DEFAULT_GIT_CACHE_CLEANUP_DAYS);
    assert_eq!(s.git_cache_ttl_secs, DEFAULT_GIT_CACHE_TTL_SECS);
    assert_eq!(s.github_token_set, Some(false));
    assert_eq!(s.auto_sync_enabled, DEFAULT_AUTO_SYNC_ENABLED);
    assert_eq!(s.global_selected_tools, None);
    assert!(!s.global_selected_tools_corrupt);
    assert_eq!(s.scan_selected_tools_only, DEFAULT_SCAN_SELECTED_TOOLS_ONLY);
    assert_eq!(s.ui_zoom_level, DEFAULT_UI_ZOOM_LEVEL);

    assert_eq!(
        s.bounds.git_cache_cleanup_days,
        GIT_CACHE_CLEANUP_DAYS_RANGE
    );
    assert_eq!(s.bounds.git_cache_ttl_secs, GIT_CACHE_TTL_SECS_RANGE);
    assert_eq!(s.bounds.ui_zoom_level, UI_ZOOM_LEVEL_RANGE);
}

// ---------------------------------------------------------------------------
// Table: malformed / legacy stored values parse to the default
// ---------------------------------------------------------------------------

#[test]
fn malformed_git_cache_cleanup_days_reads_as_default() {
    for bad in [
        "",
        "   ",
        "abc",
        "-1",
        "3651",
        "1.5",
        "999999999999999999999",
    ] {
        let (_dir, store) = make_store();
        raw(&store, "git_cache_cleanup_days", bad);
        assert_eq!(
            git_cache_cleanup_days(&store),
            DEFAULT_GIT_CACHE_CLEANUP_DAYS,
            "raw {bad:?}"
        );
    }
}

#[test]
fn valid_git_cache_cleanup_days_reads_trimmed_value() {
    for (stored, expected) in [("0", 0), (" 7 ", 7), ("3650", 3650)] {
        let (_dir, store) = make_store();
        raw(&store, "git_cache_cleanup_days", stored);
        assert_eq!(git_cache_cleanup_days(&store), expected, "raw {stored:?}");
    }
}

#[test]
fn malformed_git_cache_ttl_secs_reads_as_default() {
    for bad in ["", "x", "-5", "3601", "60s"] {
        let (_dir, store) = make_store();
        raw(&store, "git_cache_ttl_secs", bad);
        assert_eq!(
            git_cache_ttl_secs(&store),
            DEFAULT_GIT_CACHE_TTL_SECS,
            "raw {bad:?}"
        );
    }
}

#[test]
fn valid_git_cache_ttl_secs_reads_value() {
    for (stored, expected) in [("0", 0), ("3600", 3600), ("\n120\n", 120)] {
        let (_dir, store) = make_store();
        raw(&store, "git_cache_ttl_secs", stored);
        assert_eq!(git_cache_ttl_secs(&store), expected, "raw {stored:?}");
    }
}

#[test]
fn github_token_reads_trimmed_and_empty_as_none() {
    for (stored, expected) in [
        ("", None),
        ("   ", None),
        ("ghp_abc", Some("ghp_abc".to_string())),
        ("  ghp_abc\n", Some("ghp_abc".to_string())),
    ] {
        let credentials = MemoryStore::new();
        credentials.set("github_token", stored).unwrap();
        assert_eq!(
            github_token(&credentials).unwrap(),
            expected,
            "stored {stored:?}"
        );
        assert_eq!(
            super::github_token_or_none(&credentials),
            expected,
            "stored {stored:?}"
        );
    }
    assert_eq!(github_token(&MemoryStore::new()).unwrap(), None);
}

#[test]
fn token_read_failure_is_typed_degrades_for_acquisition_and_reads_as_unknown_presence() {
    let (dir, store) = make_store();
    let credentials = MemoryStore::failing();
    let err = github_token(&credentials).unwrap_err();
    assert!(matches!(
        err.downcast_ref::<SignalError>(),
        Some(SignalError::CredentialStoreUnavailable { .. })
    ));
    assert_eq!(super::github_token_or_none(&credentials), None);
    // The rest of Settings still loads; presence is unknown, not "unset".
    let s = load_settings(&store, &credentials, dir.path()).unwrap();
    assert_eq!(s.github_token_set, None);
}

/// A token that exists but whose read is denied is reported as unknown —
/// never as absent (review S1).
#[test]
fn existing_token_with_denied_read_reports_unknown_presence() {
    let (dir, store) = make_store();
    let credentials = MemoryStore::new();
    credentials.set("github_token", "ghp_exists").unwrap();
    credentials.set_reads_fail(true);
    let s = load_settings(&store, &credentials, dir.path()).unwrap();
    assert_eq!(s.github_token_set, None);
}

#[test]
fn settings_table_failure_still_fails_load() {
    let (dir, store) = make_store();
    rusqlite::Connection::open(dir.path().join("test.db"))
        .unwrap()
        .execute_batch("DROP TABLE settings;")
        .unwrap();
    assert!(load_settings(&store, &MemoryStore::new(), dir.path()).is_err());
}

#[test]
fn auto_sync_enabled_parses_bools_and_defaults_on_garbage() {
    let (dir, store) = make_store();
    let home = dir.path();
    for (stored, expected) in [
        ("true", true),
        ("false", false),
        (" TRUE ", true),
        ("False", false),
        ("yes", DEFAULT_AUTO_SYNC_ENABLED),
        ("", DEFAULT_AUTO_SYNC_ENABLED),
        ("1", DEFAULT_AUTO_SYNC_ENABLED),
    ] {
        raw(&store, "auto_sync_enabled", stored);
        assert_eq!(
            load_settings(&store, &MemoryStore::new(), home)
                .unwrap()
                .auto_sync_enabled,
            expected,
            "raw {stored:?}"
        );
    }
}

#[test]
fn scan_selected_tools_only_parses_bools_and_defaults_on_garbage() {
    let (dir, store) = make_store();
    let home = dir.path();
    for (stored, expected) in [
        ("true", true),
        ("false", false),
        ("nope", DEFAULT_SCAN_SELECTED_TOOLS_ONLY),
        ("", DEFAULT_SCAN_SELECTED_TOOLS_ONLY),
    ] {
        raw(&store, "scan_selected_tools_only", stored);
        assert_eq!(
            load_settings(&store, &MemoryStore::new(), home)
                .unwrap()
                .scan_selected_tools_only,
            expected,
            "raw {stored:?}"
        );
    }
}

#[test]
fn global_selected_tools_parses_json_and_flags_corrupt_rows() {
    let (dir, store) = make_store();
    let home = dir.path();
    // (stored, displayed selection, corrupt flag). Display keeps the
    // "malformed parses to default" contract; the flag says why it is None.
    for (stored, expected, corrupt) in [
        (
            r#"["claude_code","cursor"]"#,
            Some(vec!["claude_code".to_string(), "cursor".to_string()]),
            false,
        ),
        ("[]", Some(vec![]), false),
        ("", None, false),
        ("   ", None, false),
        ("not json", None, true),
        ("{\"a\":1}", None, true),
        ("[1,2]", None, true),
    ] {
        raw(&store, "global_selected_tools_v1", stored);
        let s = load_settings(&store, &MemoryStore::new(), home).unwrap();
        assert_eq!(s.global_selected_tools, expected, "raw {stored:?}");
        assert_eq!(s.global_selected_tools_corrupt, corrupt, "raw {stored:?}");
    }
}

#[test]
fn global_selected_tools_prunes_keys_the_registry_no_longer_knows() {
    let (dir, store) = make_store();
    let home = dir.path().join("home");
    raw(
        &store,
        "global_selected_tools_v1",
        r#"["ghost","claude_code","retired_tool"]"#,
    );
    assert_eq!(
        load_settings(&store, &MemoryStore::new(), &home)
            .unwrap()
            .global_selected_tools,
        Some(vec!["claude_code".to_string()])
    );
    assert_eq!(
        effective_global_tool_targets(&store, &home).unwrap(),
        vec!["claude_code".to_string()]
    );
    // Pruning is a read-side view; the row is repaired by the next save, not here.
    assert_eq!(
        store
            .get_setting("global_selected_tools_v1")
            .unwrap()
            .as_deref(),
        Some(r#"["ghost","claude_code","retired_tool"]"#)
    );
}

// ---------------------------------------------------------------------------
// Effective global sync target set: intent beats detection
// ---------------------------------------------------------------------------

/// Fake a Tool installation by creating its detect dir under `home`.
fn install_tool(home: &std::path::Path, key: &str) {
    let adapter =
        crate::core::tool_adapters::adapter_by_key(key).unwrap_or_else(|| panic!("adapter {key}"));
    crate::core::tool_adapters::mark_installed_in(home, adapter);
}

#[test]
fn effective_global_tool_targets_prefers_the_recorded_selection_over_detection() {
    let (dir, store) = make_store();
    let home = dir.path().join("home");
    install_tool(&home, "claude_code");
    install_tool(&home, "codex");

    // Never configured: detection is the fallback.
    assert_eq!(
        effective_global_tool_targets(&store, &home).unwrap(),
        vec!["claude_code".to_string(), "codex".to_string()]
    );

    // A selection is taken verbatim, even when it names an installed Tool the
    // operator deselected.
    raw(&store, "global_selected_tools_v1", r#"["claude_code"]"#);
    assert_eq!(
        effective_global_tool_targets(&store, &home).unwrap(),
        vec!["claude_code".to_string()]
    );

    // An empty selection means "sync nowhere" — never a fallback to detection.
    raw(&store, "global_selected_tools_v1", "[]");
    assert!(effective_global_tool_targets(&store, &home)
        .unwrap()
        .is_empty());

    // A selected-but-uninstalled key survives: the set is never intersected
    // with detection, so downstream can report it as a stale-selection skip.
    raw(&store, "global_selected_tools_v1", r#"["cursor","codex"]"#);
    assert_eq!(
        effective_global_tool_targets(&store, &home).unwrap(),
        vec!["cursor".to_string(), "codex".to_string()]
    );

    // A blank row is "never configured", hence detection again.
    raw(&store, "global_selected_tools_v1", "  ");
    assert_eq!(
        effective_global_tool_targets(&store, &home).unwrap(),
        vec!["claude_code".to_string(), "codex".to_string()]
    );
}

#[test]
fn effective_global_tool_targets_refuses_a_corrupt_selection() {
    let (dir, store) = make_store();
    let home = dir.path().join("home");
    install_tool(&home, "claude_code");
    for stored in ["not json", "{\"a\":1}", "[1,2]"] {
        raw(&store, "global_selected_tools_v1", stored);
        let err = effective_global_tool_targets(&store, &home)
            .expect_err("corrupt selection must not fall back to detection");
        match err.downcast_ref::<SignalError>() {
            Some(SignalError::SettingCorrupt { key, detail }) => {
                assert_eq!(key, "global_selected_tools_v1");
                assert!(!detail.is_empty(), "raw {stored:?}");
            }
            other => panic!("raw {stored:?}: expected SettingCorrupt, got {other:?}"),
        }
    }
}

#[test]
fn malformed_ui_zoom_level_reads_as_default() {
    for bad in ["", "abc", "NaN", "inf", "0.1", "3.5", "-1"] {
        let (_dir, store) = make_store();
        raw(&store, "ui_zoom_level", bad);
        assert_eq!(ui_zoom_level(&store), DEFAULT_UI_ZOOM_LEVEL, "raw {bad:?}");
    }
}

#[test]
fn valid_ui_zoom_level_reads_value() {
    for (stored, expected) in [("0.5", 0.5), ("1.25", 1.25), (" 3 ", 3.0), ("1", 1.0)] {
        let (_dir, store) = make_store();
        raw(&store, "ui_zoom_level", stored);
        assert_eq!(ui_zoom_level(&store), expected, "raw {stored:?}");
    }
}

#[test]
fn central_repo_path_override_wins_and_blank_is_unset() {
    let (dir, store) = make_store();
    let home = dir.path().join("home");
    let custom = dir.path().join("custom");

    raw(
        &store,
        "central_repo_path",
        custom.to_string_lossy().as_ref(),
    );
    assert_eq!(resolve_central_repo_path(&store, &home).unwrap(), custom);
    assert_eq!(
        load_settings(&store, &MemoryStore::new(), &home)
            .unwrap()
            .central_repo_path,
        custom.to_string_lossy().to_string()
    );

    for blank in ["", "   "] {
        raw(&store, "central_repo_path", blank);
        assert_eq!(
            resolve_central_repo_path(&store, &home).unwrap(),
            home.join(".skillshub"),
            "raw {blank:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Table: apply clamps into bounds and round-trips
// ---------------------------------------------------------------------------

#[test]
fn apply_git_cache_cleanup_days_clamps_and_round_trips() {
    let (dir, store) = make_store();
    let home = dir.path();
    for (input, expected) in [(45, 45), (-10, 0), (99999, 3650), (0, 0), (3650, 3650)] {
        let s = apply_setting(
            &store,
            &MemoryStore::new(),
            home,
            SettingUpdate::GitCacheCleanupDays(input),
        )
        .unwrap();
        assert_eq!(s.git_cache_cleanup_days, expected, "input {input}");
        assert_eq!(git_cache_cleanup_days(&store), expected, "input {input}");
    }
}

#[test]
fn apply_git_cache_ttl_secs_clamps_and_round_trips() {
    let (dir, store) = make_store();
    let home = dir.path();
    for (input, expected) in [(120, 120), (-1, 0), (7200, 3600)] {
        let s = apply_setting(
            &store,
            &MemoryStore::new(),
            home,
            SettingUpdate::GitCacheTtlSecs(input),
        )
        .unwrap();
        assert_eq!(s.git_cache_ttl_secs, expected, "input {input}");
        assert_eq!(git_cache_ttl_secs(&store), expected, "input {input}");
    }
}

#[test]
fn apply_ui_zoom_level_clamps_and_rejects_non_finite() {
    let (dir, store) = make_store();
    let home = dir.path();
    for (input, expected) in [
        (1.25, 1.25),
        (0.1, 0.5),
        (10.0, 3.0),
        (f64::NAN, DEFAULT_UI_ZOOM_LEVEL),
        (f64::INFINITY, DEFAULT_UI_ZOOM_LEVEL),
    ] {
        let s = apply_setting(
            &store,
            &MemoryStore::new(),
            home,
            SettingUpdate::UiZoomLevel(input),
        )
        .unwrap();
        assert_eq!(s.ui_zoom_level, expected, "input {input}");
        assert_eq!(ui_zoom_level(&store), expected, "input {input}");
    }
}

#[test]
fn apply_github_token_writes_the_credential_store_not_the_settings_row() {
    let (dir, store) = make_store();
    let home = dir.path();
    let credentials = MemoryStore::new();

    let s = apply_setting(
        &store,
        &credentials,
        home,
        SettingUpdate::GithubToken("  ghp_secret \n".to_string()),
    )
    .unwrap();
    assert_eq!(s.github_token_set, Some(true));
    assert_eq!(
        credentials.get("github_token").unwrap().as_deref(),
        Some("ghp_secret")
    );
    assert_eq!(store.get_setting("github_token").unwrap(), None);

    let s = apply_setting(
        &store,
        &credentials,
        home,
        SettingUpdate::GithubToken("   ".to_string()),
    )
    .unwrap();
    assert_eq!(s.github_token_set, Some(false));
    assert_eq!(credentials.get("github_token").unwrap(), None);
    assert_eq!(store.get_setting("github_token").unwrap(), None);
}

#[test]
fn apply_github_token_store_failure_is_typed_and_writes_nothing() {
    let (dir, store) = make_store();
    let err = apply_setting(
        &store,
        &MemoryStore::failing(),
        dir.path(),
        SettingUpdate::GithubToken("ghp_secret".to_string()),
    )
    .unwrap_err();
    assert!(matches!(
        err.downcast_ref::<SignalError>(),
        Some(SignalError::CredentialStoreUnavailable { .. })
    ));
    assert_eq!(store.get_setting("github_token").unwrap(), None);
}

#[test]
fn token_migration_moves_the_row_then_deletes_it_and_is_idempotent() {
    let (_dir, store) = make_store();
    let credentials = MemoryStore::new();
    raw(&store, "github_token", "  ghp_legacy\n");

    assert_eq!(
        migrate_github_token_to_credential_store(&store, &credentials).unwrap(),
        TokenMigration::Migrated
    );
    assert_eq!(
        credentials.get("github_token").unwrap().as_deref(),
        Some("ghp_legacy")
    );
    assert_eq!(store.get_setting("github_token").unwrap(), None);

    assert_eq!(
        migrate_github_token_to_credential_store(&store, &credentials).unwrap(),
        TokenMigration::NothingToMigrate
    );
    assert_eq!(
        credentials.get("github_token").unwrap().as_deref(),
        Some("ghp_legacy")
    );
}

#[test]
fn token_migration_drops_a_blank_row_without_touching_the_store() {
    let (_dir, store) = make_store();
    // A failing store proves the blank path never calls it.
    let credentials = MemoryStore::failing();
    raw(&store, "github_token", "   ");
    assert_eq!(
        migrate_github_token_to_credential_store(&store, &credentials).unwrap(),
        TokenMigration::DroppedBlank
    );
    assert_eq!(store.get_setting("github_token").unwrap(), None);
}

#[test]
fn token_migration_store_failure_keeps_the_row() {
    let (_dir, store) = make_store();
    raw(&store, "github_token", "ghp_legacy");
    let err =
        migrate_github_token_to_credential_store(&store, &MemoryStore::failing()).unwrap_err();
    assert!(matches!(
        err.downcast_ref::<SignalError>(),
        Some(SignalError::CredentialStoreUnavailable { .. })
    ));
    assert_eq!(
        store.get_setting("github_token").unwrap().as_deref(),
        Some("ghp_legacy")
    );
}

/// A store that accepts writes but reads back something else (a stale or
/// foreign entry) must not let the migration delete the row.
#[test]
fn token_migration_read_back_mismatch_keeps_the_row() {
    struct Lossy;
    impl CredentialStore for Lossy {
        fn get(&self, _key: &str) -> anyhow::Result<Option<String>> {
            Ok(Some("something-else".to_string()))
        }
        fn set(&self, _key: &str, _value: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn delete(&self, _key: &str) -> anyhow::Result<()> {
            Ok(())
        }
    }
    let (_dir, store) = make_store();
    raw(&store, "github_token", "ghp_legacy");
    let err = migrate_github_token_to_credential_store(&store, &Lossy).unwrap_err();
    assert!(matches!(
        err.downcast_ref::<SignalError>(),
        Some(SignalError::CredentialStoreUnavailable { .. })
    ));
    assert_eq!(
        store.get_setting("github_token").unwrap().as_deref(),
        Some("ghp_legacy")
    );
}

// ---------------------------------------------------------------------------
// Legacy row lifecycle: secure cleanup (review B2) and settlement by an
// explicit Save/Remove (review B1). `make_store` is a file-backed SQLite DB.
// ---------------------------------------------------------------------------

fn assert_typed_store_error(err: &anyhow::Error) {
    assert!(
        matches!(
            err.downcast_ref::<SignalError>(),
            Some(SignalError::CredentialStoreUnavailable { .. })
        ),
        "expected CredentialStoreUnavailable, got {err:#}"
    );
}

/// Every database file SQLite may have written: the DB and any rollback
/// journal / WAL sibling.
fn db_file_bytes(dir: &tempfile::TempDir) -> Vec<(PathBuf, Vec<u8>)> {
    ["test.db", "test.db-journal", "test.db-wal", "test.db-shm"]
        .iter()
        .map(|name| dir.path().join(name))
        .filter(|path| path.exists())
        .map(|path| {
            let bytes = std::fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect()
}

fn db_files_contain(dir: &tempfile::TempDir, needle: &str) -> bool {
    db_file_bytes(dir).iter().any(|(_, bytes)| {
        bytes
            .windows(needle.len())
            .any(|window| window == needle.as_bytes())
    })
}

/// Make every DELETE of the legacy row fail, the way a read-only or
/// corrupt database would, while reads keep working.
fn block_legacy_row_delete(dir: &tempfile::TempDir) {
    rusqlite::Connection::open(dir.path().join("test.db"))
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER block_token_delete BEFORE DELETE ON settings
             WHEN old.key = 'github_token'
             BEGIN SELECT RAISE(ABORT, 'delete blocked by test'); END;",
        )
        .unwrap();
}

const CLEANUP_MARKER: &str = "github_token.cleanup_pending";

fn cleanup_marker(store: &SkillStore) -> Option<String> {
    store.get_setting(CLEANUP_MARKER).unwrap()
}

/// Leave `sentinel` in freed pages the way an unsecured deletion does:
/// insert it under `key`, then DELETE it with `secure_delete` off. Padded
/// past a page so the bytes land in overflow pages, which go to the freelist
/// (a small freed cell is reused by the very next small insert).
fn leave_freed_bytes(dir: &tempfile::TempDir, key: &str, sentinel: &str) {
    let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
    conn.pragma_update(None, "secure_delete", false).unwrap();
    let value = format!("{}{sentinel}", "x".repeat(16 * 1024));
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)",
        rusqlite::params![key, value],
    )
    .unwrap();
    conn.execute("DELETE FROM settings WHERE key = ?1", [key])
        .unwrap();
    drop(conn);
    assert!(
        db_files_contain(dir, sentinel),
        "precondition: {sentinel} left in freed pages"
    );
}

#[test]
fn token_migration_removes_the_plaintext_from_the_database_bytes() {
    let (dir, store) = make_store();
    // An earlier value of the row (a pre-1.2.18 token change) and the current
    // one; a plain DELETE leaves both recoverable from freed pages.
    let old = "SENTINEL_OLD_ghp_0123456789abcdef";
    let current = "SENTINEL_NEW_ghp_fedcba9876543210";
    raw(&store, "github_token", old);
    raw(&store, "github_token", current);
    // Sensitivity: the sentinel really is in the file before cleanup.
    assert!(
        db_files_contain(&dir, current),
        "precondition: sentinel on disk"
    );

    let credentials = MemoryStore::new();
    assert_eq!(
        migrate_github_token_to_credential_store(&store, &credentials).unwrap(),
        TokenMigration::Migrated
    );

    // Logically gone ...
    assert_eq!(store.get_setting("github_token").unwrap(), None);
    // ... and physically gone from every DB file (connections are closed
    // after each call, so the files are settled).
    for sentinel in [old, current] {
        assert!(
            !db_files_contain(&dir, sentinel),
            "{sentinel} still present in {:?}",
            db_file_bytes(&dir)
                .iter()
                .map(|(p, _)| p.clone())
                .collect::<Vec<_>>()
        );
    }
    // The rest of the database survived the compaction.
    raw(&store, "auto_sync_enabled", "false");
    assert!(
        !load_settings(&store, &credentials, dir.path())
            .unwrap()
            .auto_sync_enabled
    );
}

#[test]
fn token_migration_cleanup_failure_is_an_error_not_migrated() {
    let (dir, store) = make_store();
    raw(&store, "github_token", "ghp_legacy");
    block_legacy_row_delete(&dir);
    let credentials = MemoryStore::new();

    let err = migrate_github_token_to_credential_store(&store, &credentials).unwrap_err();
    assert!(
        format!("{err:#}").contains("delete blocked by test"),
        "{err:#}"
    );
    // The keychain copy was verified, the row stays for the next launch.
    assert_eq!(
        credentials.get("github_token").unwrap().as_deref(),
        Some("ghp_legacy")
    );
    assert_eq!(
        store.get_setting("github_token").unwrap().as_deref(),
        Some("ghp_legacy")
    );
    // The DELETE and the marker share one transaction: a failed DELETE
    // leaves no marker either.
    assert_eq!(cleanup_marker(&store), None);
}

/// B2a: the row's deletion committed but the compaction failed. The
/// migration is an error (never `Migrated`), the verified keychain copy
/// stays, and the non-secret marker survives so the next startup finishes
/// the cleanup although no legacy row is left to trigger it.
#[test]
fn failed_compaction_keeps_a_marker_that_the_next_startup_completes() {
    let (dir, store) = make_store();
    let old = "SENTINEL_OLD_ghp_1111111111111111";
    let current = "SENTINEL_NEW_ghp_2222222222222222";
    // An earlier token left in a freed region by a pre-1.2.18 change.
    leave_freed_bytes(&dir, "github_token", old);
    raw(&store, "github_token", current);
    let credentials = MemoryStore::new();

    crate::core::skill_store::compaction_fault::fail_next();
    let err = migrate_github_token_to_credential_store(&store, &credentials).unwrap_err();
    assert!(
        format!("{err:#}").contains("compaction fault injected by test"),
        "{err:#}"
    );
    assert_eq!(store.get_setting("github_token").unwrap(), None);
    assert_eq!(cleanup_marker(&store).as_deref(), Some("1"));
    assert_eq!(
        credentials.get("github_token").unwrap().as_deref(),
        Some(current)
    );
    // `secure_delete` was in force inside the transaction: the live row's
    // bytes are zeroed. The earlier value in a freed region is what the
    // compaction still owes.
    assert!(!db_files_contain(&dir, current), "live row not zeroed");
    assert!(
        db_files_contain(&dir, old),
        "precondition: a historical copy awaits the compaction"
    );

    // Next startup: pending cleanup first, then the migration.
    assert!(store.finish_pending_secure_cleanup().unwrap());
    assert_eq!(
        migrate_github_token_to_credential_store(&store, &credentials).unwrap(),
        TokenMigration::NothingToMigrate
    );
    assert_eq!(cleanup_marker(&store), None);
    for sentinel in [old, current] {
        assert!(
            !db_files_contain(&dir, sentinel),
            "{sentinel} still on disk"
        );
    }
    assert_eq!(
        credentials.get("github_token").unwrap().as_deref(),
        Some(current)
    );
    // Nothing left to do on the launch after that.
    assert!(!store.finish_pending_secure_cleanup().unwrap());
}

/// B2a: an explicit Save or Remove finishes a pending cleanup even when no
/// legacy row is pending.
#[test]
fn save_and_remove_finish_a_stale_cleanup_marker() {
    let (dir, store) = make_store();
    let credentials = MemoryStore::new();
    for (update, sentinel) in [
        ("ghp_B", "SENTINEL_SAVE_ghp_3333333333333333"),
        ("", "SENTINEL_REMOVE_ghp_4444444444444444"),
    ] {
        leave_freed_bytes(&dir, "github_token", sentinel);
        raw(&store, CLEANUP_MARKER, "1");

        apply_setting(
            &store,
            &credentials,
            dir.path(),
            SettingUpdate::GithubToken(update.to_string()),
        )
        .unwrap();

        assert_eq!(cleanup_marker(&store), None, "marker left by {update:?}");
        assert!(
            !db_files_contain(&dir, sentinel),
            "{sentinel} still on disk after {update:?}"
        );
    }
    assert_eq!(credentials.get("github_token").unwrap(), None);
}

/// B2b: in WAL mode, a checkpoint that a reader keeps from finishing
/// reports `busy` in its result row, not as a query error. That is an
/// incomplete cleanup: an error that keeps the marker, completed once the
/// reader lets go.
#[test]
fn busy_wal_checkpoint_is_an_error_and_the_retry_completes_it() {
    let (dir, store) = make_store();
    let db = dir.path().join("test.db");
    let mode: String = rusqlite::Connection::open(&db)
        .unwrap()
        .query_row("PRAGMA journal_mode = WAL;", [], |row| row.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
    let old = "SENTINEL_OLD_ghp_5555555555555555";
    let current = "SENTINEL_NEW_ghp_6666666666666666";
    raw(&store, "github_token", old);
    raw(&store, "github_token", current);
    let credentials = MemoryStore::new();

    // A reader holding a pre-deletion snapshot across the first attempt.
    let reader = rusqlite::Connection::open(&db).unwrap();
    reader.execute_batch("BEGIN;").unwrap();
    let _: i64 = reader
        .query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))
        .unwrap();

    let err = migrate_github_token_to_credential_store(&store, &credentials).unwrap_err();
    assert!(
        format!("{err:#}").contains("WAL checkpoint could not complete"),
        "{err:#}"
    );
    assert_eq!(store.get_setting("github_token").unwrap(), None);
    assert_eq!(cleanup_marker(&store).as_deref(), Some("1"));
    assert_eq!(
        credentials.get("github_token").unwrap().as_deref(),
        Some(current)
    );
    assert!(
        db_files_contain(&dir, current) || db_files_contain(&dir, old),
        "precondition: the busy checkpoint left token bytes on disk"
    );

    drop(reader);
    assert!(store.finish_pending_secure_cleanup().unwrap());
    assert_eq!(cleanup_marker(&store), None);
    for sentinel in [old, current] {
        assert!(
            !db_files_contain(&dir, sentinel),
            "{sentinel} still in {:?}",
            db_file_bytes(&dir)
                .iter()
                .map(|(p, _)| p.clone())
                .collect::<Vec<_>>()
        );
    }
}

/// A migration whose keychain write landed but whose read-back was denied
/// keeps the row (N1: set succeeds → get errors).
#[test]
fn token_migration_read_back_denied_keeps_the_row() {
    let (_dir, store) = make_store();
    raw(&store, "github_token", "ghp_legacy");
    let credentials = MemoryStore::new();
    credentials.set_reads_fail(true);

    let err = migrate_github_token_to_credential_store(&store, &credentials).unwrap_err();
    assert_typed_store_error(&err);
    assert_eq!(
        store.get_setting("github_token").unwrap().as_deref(),
        Some("ghp_legacy")
    );
    credentials.set_reads_fail(false);
    assert_eq!(
        credentials.get("github_token").unwrap().as_deref(),
        Some("ghp_legacy")
    );
}

/// B1 (a): the startup migration failed and kept token A in the table; the
/// operator later saves B. The next launch must not overwrite B with A.
#[test]
fn save_after_failed_migration_settles_the_row_so_restart_keeps_the_new_token() {
    let (dir, store) = make_store();
    let legacy = "SENTINEL_LEGACY_ghp_aaaaaaaaaaaa";
    raw(&store, "github_token", legacy);
    assert_typed_store_error(
        &migrate_github_token_to_credential_store(&store, &MemoryStore::failing()).unwrap_err(),
    );
    assert!(store.get_setting("github_token").unwrap().is_some());

    // Access is granted now; the operator saves B.
    let credentials = MemoryStore::new();
    let s = apply_setting(
        &store,
        &credentials,
        dir.path(),
        SettingUpdate::GithubToken("ghp_B".to_string()),
    )
    .unwrap();
    assert_eq!(s.github_token_set, Some(true));
    assert_eq!(store.get_setting("github_token").unwrap(), None);
    assert!(
        !db_files_contain(&dir, legacy),
        "legacy plaintext left on disk"
    );

    // Restart.
    assert_eq!(
        migrate_github_token_to_credential_store(&store, &credentials).unwrap(),
        TokenMigration::NothingToMigrate
    );
    assert_eq!(
        credentials.get("github_token").unwrap().as_deref(),
        Some("ghp_B")
    );
}

/// B1 (b): the migration wrote A to the keychain but its read-back failed,
/// so the row stayed. A later Remove must not be undone by the next launch.
#[test]
fn remove_after_partial_migration_settles_the_row_so_restart_stays_empty() {
    let (dir, store) = make_store();
    raw(&store, "github_token", "ghp_A");
    let credentials = MemoryStore::new();
    credentials.set_reads_fail(true);
    assert_typed_store_error(
        &migrate_github_token_to_credential_store(&store, &credentials).unwrap_err(),
    );

    // Remove succeeds even while reads are still denied; presence is unknown.
    let s = apply_setting(
        &store,
        &credentials,
        dir.path(),
        SettingUpdate::GithubToken(String::new()),
    )
    .unwrap();
    assert_eq!(s.github_token_set, None);
    assert_eq!(store.get_setting("github_token").unwrap(), None);

    // Restart with access restored.
    credentials.set_reads_fail(false);
    assert_eq!(
        migrate_github_token_to_credential_store(&store, &credentials).unwrap(),
        TokenMigration::NothingToMigrate
    );
    assert_eq!(credentials.get("github_token").unwrap(), None);
}

/// A replacement that cannot be read back while a legacy row is pending is
/// unverified: the row is kept (deleting it would trade a known token for an
/// unconfirmed one) and the operation fails with the typed error.
#[test]
fn save_with_pending_row_and_denied_read_back_fails_and_keeps_the_row() {
    let (dir, store) = make_store();
    raw(&store, "github_token", "ghp_A");
    let credentials = MemoryStore::new();
    credentials.set_reads_fail(true);

    let err = apply_setting(
        &store,
        &credentials,
        dir.path(),
        SettingUpdate::GithubToken("ghp_B".to_string()),
    )
    .unwrap_err();
    assert_typed_store_error(&err);
    assert_eq!(
        store.get_setting("github_token").unwrap().as_deref(),
        Some("ghp_A")
    );
}

/// S1 write-success/read-denied with no pending row: the save is not an
/// error, but presence is reported as unknown so the UI does not claim a
/// verified token.
#[test]
fn save_with_denied_read_back_reports_unknown_presence() {
    let (dir, store) = make_store();
    let credentials = MemoryStore::new();
    credentials.set_reads_fail(true);

    let s = apply_setting(
        &store,
        &credentials,
        dir.path(),
        SettingUpdate::GithubToken("ghp_B".to_string()),
    )
    .unwrap();
    assert_eq!(s.github_token_set, None);
    credentials.set_reads_fail(false);
    assert_eq!(
        credentials.get("github_token").unwrap().as_deref(),
        Some("ghp_B")
    );
}

/// B1 (c): the keychain change lands but the legacy row cannot be removed.
/// The operation reports the failure instead of success; the keychain holds
/// what the operator asked for, and the surviving row is the remaining risk
/// (the next launch's migration would act on it).
#[test]
fn legacy_row_cleanup_failure_fails_save_and_remove() {
    let (dir, store) = make_store();
    raw(&store, "github_token", "ghp_A");
    block_legacy_row_delete(&dir);
    let credentials = MemoryStore::new();

    let err = apply_setting(
        &store,
        &credentials,
        dir.path(),
        SettingUpdate::GithubToken("ghp_B".to_string()),
    )
    .unwrap_err();
    assert!(
        format!("{err:#}").contains("delete blocked by test"),
        "{err:#}"
    );
    assert_eq!(
        credentials.get("github_token").unwrap().as_deref(),
        Some("ghp_B")
    );
    assert_eq!(
        store.get_setting("github_token").unwrap().as_deref(),
        Some("ghp_A")
    );

    let err = apply_setting(
        &store,
        &credentials,
        dir.path(),
        SettingUpdate::GithubToken("  ".to_string()),
    )
    .unwrap_err();
    assert!(
        format!("{err:#}").contains("delete blocked by test"),
        "{err:#}"
    );
    assert_eq!(credentials.get("github_token").unwrap(), None);
    assert_eq!(
        store.get_setting("github_token").unwrap().as_deref(),
        Some("ghp_A")
    );
}

// ---------------------------------------------------------------------------
// Onboarding scan selection (reads only the settings table)
// ---------------------------------------------------------------------------

#[test]
fn onboarding_scan_selection_table() {
    let valid = Some(vec!["claude_code".to_string()]);
    for (flag, stored, expected) in [
        // Flag off: always every detected Tool.
        (Some("false"), None, None),
        (Some("false"), Some("[]"), None),
        (Some("false"), Some(r#"["claude_code"]"#), None),
        (Some("false"), Some("not json"), None),
        // Flag on (explicit, and by default).
        (Some("true"), None, None),
        (Some("true"), Some("[]"), Some(vec![])),
        (Some("true"), Some(r#"["claude_code"]"#), valid.clone()),
        (Some("true"), Some("not json"), None),
        (None, Some(r#"["claude_code"]"#), valid.clone()),
        (None, Some("not json"), None),
    ] {
        let (_dir, store) = make_store();
        if let Some(flag) = flag {
            raw(&store, "scan_selected_tools_only", flag);
        }
        if let Some(stored) = stored {
            raw(&store, "global_selected_tools_v1", stored);
        }
        assert_eq!(
            onboarding_scan_selection(&store).unwrap(),
            expected,
            "flag {flag:?}, stored {stored:?}"
        );
    }
}

#[test]
fn apply_auto_sync_round_trips() {
    let (dir, store) = make_store();
    let home = dir.path();
    for value in [false, true, false] {
        let s = apply_setting(
            &store,
            &MemoryStore::new(),
            home,
            SettingUpdate::AutoSyncEnabled(value),
        )
        .unwrap();
        assert_eq!(s.auto_sync_enabled, value);
        assert_eq!(
            load_settings(&store, &MemoryStore::new(), home)
                .unwrap()
                .auto_sync_enabled,
            value
        );
    }
}

#[test]
fn apply_global_tool_config_round_trips_and_keeps_empty_selection() {
    let (dir, store) = make_store();
    let home = dir.path();

    let selected = vec!["claude_code".to_string(), "cursor".to_string()];
    let s = apply_setting(
        &store,
        &MemoryStore::new(),
        home,
        SettingUpdate::GlobalToolConfig {
            selected_tools: selected.clone(),
            scan_selected_only: false,
        },
    )
    .unwrap();
    assert_eq!(s.global_selected_tools, Some(selected.clone()));
    assert!(!s.scan_selected_tools_only);

    // Empty selection is a deliberate choice, distinct from "never configured".
    let s = apply_setting(
        &store,
        &MemoryStore::new(),
        home,
        SettingUpdate::GlobalToolConfig {
            selected_tools: vec![],
            scan_selected_only: true,
        },
    )
    .unwrap();
    assert_eq!(s.global_selected_tools, Some(vec![]));
    assert!(s.scan_selected_tools_only);
}

#[test]
fn apply_global_tool_config_refuses_an_unknown_tool_key_and_leaves_the_row() {
    let (dir, store) = make_store();
    let home = dir.path();
    raw(&store, "global_selected_tools_v1", r#"["claude_code"]"#);

    let err = apply_setting(
        &store,
        &MemoryStore::new(),
        home,
        SettingUpdate::GlobalToolConfig {
            selected_tools: vec!["claude_code".to_string(), "ghost".to_string()],
            scan_selected_only: true,
        },
    )
    .expect_err("unknown key must be refused");
    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::UnknownTool {
            tool: "ghost".to_string()
        })
    );
    assert_eq!(
        store
            .get_setting("global_selected_tools_v1")
            .unwrap()
            .as_deref(),
        Some(r#"["claude_code"]"#)
    );
    assert_eq!(
        load_settings(&store, &MemoryStore::new(), home)
            .unwrap()
            .scan_selected_tools_only,
        DEFAULT_SCAN_SELECTED_TOOLS_ONLY
    );
}

#[test]
fn apply_central_repo_path_requires_absolute_path() {
    let (dir, store) = make_store();
    let err = apply_setting(
        &store,
        &MemoryStore::new(),
        dir.path(),
        SettingUpdate::CentralRepoPath("relative/dir".to_string()),
    )
    .unwrap_err();
    assert!(err.to_string().contains("absolute"), "{err}");
}

#[test]
fn apply_central_repo_path_creates_dir_and_persists() {
    let (dir, store) = make_store();
    let home = dir.path().join("home");
    let target = dir.path().join("new-central");
    assert!(!target.exists());

    let s = apply_setting(
        &store,
        &MemoryStore::new(),
        &home,
        SettingUpdate::CentralRepoPath(target.to_string_lossy().to_string()),
    )
    .unwrap();
    assert_eq!(s.central_repo_path, target.to_string_lossy().to_string());
    assert!(target.is_dir());
    assert_eq!(resolve_central_repo_path(&store, &home).unwrap(), target);
}

#[test]
fn apply_central_repo_path_moves_managed_skills() {
    let (dir, store) = make_store();
    let home = dir.path().join("home");
    let old_base = home.join(".skillshub");
    let skill_dir = old_base.join("my-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), "# hi").unwrap();

    let now = 1_700_000_000_000;
    store
        .upsert_skill(&SkillRecord {
            id: "s1".to_string(),
            name: "my-skill".to_string(),
            description: None,
            source_type: "local".to_string(),
            source_ref: None,
            source_subpath: None,
            source_revision: None,
            central_path: skill_dir.to_string_lossy().to_string(),
            content_hash: None,
            created_at: now,
            updated_at: now,
            last_sync_at: None,
            last_seen_at: now,
            status: "active".to_string(),
            imported_from_tool: None,
        })
        .unwrap();

    let new_base = dir.path().join("moved");
    apply_setting(
        &store,
        &MemoryStore::new(),
        &home,
        SettingUpdate::CentralRepoPath(new_base.to_string_lossy().to_string()),
    )
    .unwrap();

    let moved = new_base.join("my-skill");
    assert!(moved.join("SKILL.md").exists());
    assert!(!skill_dir.exists());
    let record = store.get_skill_by_id("s1").unwrap().expect("skill");
    assert_eq!(PathBuf::from(&record.central_path), moved);
    assert!(record.updated_at >= now);
}

#[test]
fn apply_central_repo_path_same_path_is_a_noop_move() {
    let (dir, store) = make_store();
    let home = dir.path().join("home");
    let base = home.join(".skillshub");
    let s = apply_setting(
        &store,
        &MemoryStore::new(),
        &home,
        SettingUpdate::CentralRepoPath(base.to_string_lossy().to_string()),
    )
    .unwrap();
    assert_eq!(s.central_repo_path, base.to_string_lossy().to_string());
    assert!(base.is_dir());
}

// ---------------------------------------------------------------------------
// Internal persisted state that is not a user setting
// ---------------------------------------------------------------------------

#[test]
fn featured_skills_cache_round_trips() {
    let (_dir, store) = make_store();
    assert_eq!(featured_skills_cache(&store), None);
    set_featured_skills_cache(&store, "{\"skills\":[]}").unwrap();
    assert_eq!(
        featured_skills_cache(&store).as_deref(),
        Some("{\"skills\":[]}")
    );
}

#[test]
fn record_installed_tools_reports_only_tools_not_seen_before() {
    let (_dir, store) = make_store();
    let first = record_installed_tools(&store, &["claude".into(), "cursor".into()]).unwrap();
    assert_eq!(first, vec!["claude".to_string(), "cursor".to_string()]);

    let second = record_installed_tools(&store, &["claude".into(), "pi".into()]).unwrap();
    assert_eq!(second, vec!["pi".to_string()]);

    // A tool that disappeared and comes back is new again.
    let third = record_installed_tools(&store, &["cursor".into()]).unwrap();
    assert_eq!(third, vec!["cursor".to_string()]);
}

#[test]
fn record_installed_tools_treats_malformed_stored_value_as_empty() {
    let (_dir, store) = make_store();
    store.set_setting("installed_tools_v1", "not json").unwrap();
    let newly = record_installed_tools(&store, &["claude".into()]).unwrap();
    assert_eq!(newly, vec!["claude".to_string()]);
}

// ---------------------------------------------------------------------------
// Wire contract
// ---------------------------------------------------------------------------

#[test]
fn setting_update_deserializes_adjacently_tagged() {
    let u: SettingUpdate =
        serde_json::from_str(r#"{"key":"git_cache_cleanup_days","value":12}"#).unwrap();
    assert!(matches!(u, SettingUpdate::GitCacheCleanupDays(12)));

    let u: SettingUpdate = serde_json::from_str(
        r#"{"key":"global_tool_config","value":{"selected_tools":["cursor"],"scan_selected_only":false}}"#,
    )
    .unwrap();
    match u {
        SettingUpdate::GlobalToolConfig {
            selected_tools,
            scan_selected_only,
        } => {
            assert_eq!(selected_tools, vec!["cursor".to_string()]);
            assert!(!scan_selected_only);
        }
        other => panic!("unexpected {other:?}"),
    }

    let u: SettingUpdate =
        serde_json::from_str(r#"{"key":"central_repo_path","value":"/tmp/x"}"#).unwrap();
    assert!(matches!(u, SettingUpdate::CentralRepoPath(p) if p == "/tmp/x"));
}
