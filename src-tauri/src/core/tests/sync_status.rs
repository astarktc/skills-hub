use crate::core::sync_status::{
    aggregate, next_status, Observation, ProjectSyncStatus, SyncMode, SyncStatus,
};

// ---------------------------------------------------------------------------
// next_status: pure decision table (no temp dirs, no DB)
// ---------------------------------------------------------------------------

fn obs<'a>(
    source_present: bool,
    target_present: bool,
    mode: SyncMode,
    current: SyncStatus,
    source_hash: Option<&'a str>,
    recorded_hash: Option<&'a str>,
) -> Observation<'a> {
    Observation {
        source_present,
        target_present,
        mode,
        current,
        source_hash,
        recorded_hash,
    }
}

#[test]
fn next_status_table() {
    use SyncMode::*;
    use SyncStatus::*;
    // (case name, observation, expected)
    let cases: Vec<(&str, Observation<'_>, SyncStatus)> = vec![
        // Source absent wins over everything, whatever the current status.
        (
            "source absent / synced",
            obs(false, true, Symlink, Synced, None, None),
            Missing,
        ),
        (
            "source absent / pending",
            obs(false, false, Copy, Pending, Some("h"), None),
            Missing,
        ),
        (
            "source absent / error",
            obs(false, true, Copy, Error, Some("h"), Some("h")),
            Missing,
        ),
        // Target absent: only a previously deployed row becomes missing.
        (
            "target absent / synced",
            obs(true, false, Symlink, Synced, None, None),
            Missing,
        ),
        (
            "target absent / stale",
            obs(true, false, Copy, Stale, Some("h"), None),
            Missing,
        ),
        (
            "target absent / missing stays",
            obs(true, false, Symlink, Missing, None, None),
            Missing,
        ),
        (
            "target absent / pending unchanged",
            obs(true, false, Symlink, Pending, None, None),
            Pending,
        ),
        (
            "target absent / error unchanged",
            obs(true, false, Copy, Error, Some("h"), None),
            Error,
        ),
        // Both present, link modes: only recovery from missing changes anything.
        (
            "symlink both present / missing recovers",
            obs(true, true, Symlink, Missing, None, None),
            Synced,
        ),
        (
            "junction both present / missing recovers",
            obs(true, true, Junction, Missing, None, None),
            Synced,
        ),
        (
            "symlink both present / synced unchanged",
            obs(true, true, Symlink, Synced, None, None),
            Synced,
        ),
        (
            "symlink both present / error unchanged",
            obs(true, true, Symlink, Error, None, None),
            Error,
        ),
        (
            "symlink both present / pending unchanged",
            obs(true, true, Symlink, Pending, None, None),
            Pending,
        ),
        // Both present, copy mode: content hashes decide.
        (
            "copy hashes equal / synced",
            obs(true, true, Copy, Synced, Some("h"), Some("h")),
            Synced,
        ),
        (
            "copy hashes differ / stale",
            obs(true, true, Copy, Synced, Some("h2"), Some("h1")),
            Stale,
        ),
        (
            "copy recorded hash absent / stale",
            obs(true, true, Copy, Synced, Some("h"), None),
            Stale,
        ),
        (
            "copy hashes equal / error recovers",
            obs(true, true, Copy, Error, Some("h"), Some("h")),
            Synced,
        ),
        (
            "copy hashes equal / missing recovers",
            obs(true, true, Copy, Missing, Some("h"), Some("h")),
            Synced,
        ),
        (
            "copy hashes differ / missing becomes stale",
            obs(true, true, Copy, Missing, Some("h"), None),
            Stale,
        ),
        (
            "copy source hash unavailable / unchanged",
            obs(true, true, Copy, Stale, None, None),
            Stale,
        ),
        (
            "copy source hash unavailable / missing unchanged",
            obs(true, true, Copy, Missing, None, None),
            Missing,
        ),
    ];

    for (name, observation, expected) in cases {
        assert_eq!(next_status(&observation), expected, "case: {name}");
    }
}

// ---------------------------------------------------------------------------
// aggregate: precedence table (error/missing > stale > pending > synced)
// ---------------------------------------------------------------------------

#[test]
fn aggregate_precedence_table() {
    use ProjectSyncStatus as P;
    use SyncStatus::*;
    let cases: Vec<(&str, Vec<SyncStatus>, P)> = vec![
        ("no assignments", vec![], P::Empty),
        ("all synced", vec![Synced, Synced], P::Synced),
        ("single pending", vec![Pending], P::Pending),
        (
            "pending beats synced",
            vec![Synced, Pending, Synced],
            P::Pending,
        ),
        (
            "stale beats pending",
            vec![Pending, Stale, Synced],
            P::Stale,
        ),
        ("error beats stale", vec![Stale, Error, Pending], P::Error),
        ("missing counts as error", vec![Synced, Missing], P::Error),
        ("missing beats stale", vec![Stale, Missing], P::Error),
    ];
    for (name, statuses, expected) in cases {
        assert_eq!(aggregate(statuses.into_iter()), expected, "case: {name}");
    }
}

// ---------------------------------------------------------------------------
// Stored-string seam: round-trip and legacy values
// ---------------------------------------------------------------------------

#[test]
fn sync_status_round_trips_through_stored_strings() {
    for status in SyncStatus::ALL {
        assert_eq!(
            SyncStatus::from_stored(status.as_str()),
            Some(status),
            "{status:?}"
        );
    }
    // Legacy vocabulary of global skill targets: "ok" was the only status ever written.
    assert_eq!(SyncStatus::from_stored("ok"), Some(SyncStatus::Synced));
    assert_eq!(SyncStatus::from_stored("bogus"), None);
    assert_eq!(SyncStatus::from_stored(""), None);
    // Serde wire form equals the stored form.
    assert_eq!(
        serde_json::to_string(&SyncStatus::Missing).unwrap(),
        "\"missing\""
    );
}

#[test]
fn sync_mode_round_trips_through_stored_strings() {
    for mode in SyncMode::ALL {
        assert_eq!(SyncMode::from_stored(mode.as_str()), Some(mode), "{mode:?}");
    }
    assert_eq!(SyncMode::from_stored("auto"), None);
    assert_eq!(SyncMode::from_stored("bogus"), None);
    assert_eq!(serde_json::to_string(&SyncMode::Copy).unwrap(), "\"copy\"");
}

#[test]
fn project_sync_status_wire_form() {
    assert_eq!(
        serde_json::to_string(&ProjectSyncStatus::Empty).unwrap(),
        "\"none\""
    );
    assert_eq!(
        serde_json::to_string(&ProjectSyncStatus::Error).unwrap(),
        "\"error\""
    );
}
