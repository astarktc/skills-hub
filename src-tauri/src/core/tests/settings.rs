use std::path::PathBuf;

use crate::core::settings::{
    apply_setting, featured_skills_cache, git_cache_cleanup_days, git_cache_ttl_secs, github_token,
    load_settings, record_installed_tools, resolve_central_repo_path, set_featured_skills_cache,
    ui_zoom_level, SettingUpdate, DEFAULT_AUTO_SYNC_ENABLED, DEFAULT_GIT_CACHE_CLEANUP_DAYS,
    DEFAULT_GIT_CACHE_TTL_SECS, DEFAULT_SCAN_SELECTED_TOOLS_ONLY, DEFAULT_UI_ZOOM_LEVEL,
    GIT_CACHE_CLEANUP_DAYS_RANGE, GIT_CACHE_TTL_SECS_RANGE, UI_ZOOM_LEVEL_RANGE,
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
    let s = load_settings(&store, &home).unwrap();

    assert_eq!(
        s.central_repo_path,
        home.join(".skillshub").to_string_lossy().to_string()
    );
    assert_eq!(s.git_cache_cleanup_days, DEFAULT_GIT_CACHE_CLEANUP_DAYS);
    assert_eq!(s.git_cache_ttl_secs, DEFAULT_GIT_CACHE_TTL_SECS);
    assert_eq!(s.github_token, "");
    assert_eq!(s.auto_sync_enabled, DEFAULT_AUTO_SYNC_ENABLED);
    assert_eq!(s.global_selected_tools, None);
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
        let (_dir, store) = make_store();
        raw(&store, "github_token", stored);
        assert_eq!(github_token(&store).unwrap(), expected, "raw {stored:?}");
    }
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
            load_settings(&store, home).unwrap().auto_sync_enabled,
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
            load_settings(&store, home)
                .unwrap()
                .scan_selected_tools_only,
            expected,
            "raw {stored:?}"
        );
    }
}

#[test]
fn global_selected_tools_parses_json_and_defaults_to_unconfigured() {
    let (dir, store) = make_store();
    let home = dir.path();
    for (stored, expected) in [
        (
            r#"["claude_code","cursor"]"#,
            Some(vec!["claude_code".to_string(), "cursor".to_string()]),
        ),
        ("[]", Some(vec![])),
        ("not json", None),
        ("{\"a\":1}", None),
        ("[1,2]", None),
        ("", None),
    ] {
        raw(&store, "global_selected_tools_v1", stored);
        assert_eq!(
            load_settings(&store, home).unwrap().global_selected_tools,
            expected,
            "raw {stored:?}"
        );
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
        load_settings(&store, &home).unwrap().central_repo_path,
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
        let s = apply_setting(&store, home, SettingUpdate::GitCacheCleanupDays(input)).unwrap();
        assert_eq!(s.git_cache_cleanup_days, expected, "input {input}");
        assert_eq!(git_cache_cleanup_days(&store), expected, "input {input}");
    }
}

#[test]
fn apply_git_cache_ttl_secs_clamps_and_round_trips() {
    let (dir, store) = make_store();
    let home = dir.path();
    for (input, expected) in [(120, 120), (-1, 0), (7200, 3600)] {
        let s = apply_setting(&store, home, SettingUpdate::GitCacheTtlSecs(input)).unwrap();
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
        let s = apply_setting(&store, home, SettingUpdate::UiZoomLevel(input)).unwrap();
        assert_eq!(s.ui_zoom_level, expected, "input {input}");
        assert_eq!(ui_zoom_level(&store), expected, "input {input}");
    }
}

#[test]
fn apply_github_token_trims_and_clears() {
    let (dir, store) = make_store();
    let home = dir.path();

    let s = apply_setting(
        &store,
        home,
        SettingUpdate::GithubToken("  ghp_secret \n".to_string()),
    )
    .unwrap();
    assert_eq!(s.github_token, "ghp_secret");
    assert_eq!(github_token(&store).unwrap().as_deref(), Some("ghp_secret"));

    let s = apply_setting(&store, home, SettingUpdate::GithubToken("   ".to_string())).unwrap();
    assert_eq!(s.github_token, "");
    assert_eq!(github_token(&store).unwrap(), None);
}

#[test]
fn apply_auto_sync_round_trips() {
    let (dir, store) = make_store();
    let home = dir.path();
    for value in [false, true, false] {
        let s = apply_setting(&store, home, SettingUpdate::AutoSyncEnabled(value)).unwrap();
        assert_eq!(s.auto_sync_enabled, value);
        assert_eq!(
            load_settings(&store, home).unwrap().auto_sync_enabled,
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
fn apply_central_repo_path_requires_absolute_path() {
    let (dir, store) = make_store();
    let err = apply_setting(
        &store,
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
        })
        .unwrap();

    let new_base = dir.path().join("moved");
    apply_setting(
        &store,
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
