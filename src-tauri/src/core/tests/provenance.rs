use crate::core::provenance::{
    is_refreshable, refresh_eligibility, Provenance, RefreshEligibility,
};
use crate::core::skill_store::SkillRecord;
use crate::core::unlocatable::UnlocatableState;

fn record(source_type: &str) -> SkillRecord {
    SkillRecord {
        id: "id".to_string(),
        name: "name".to_string(),
        description: None,
        source_type: source_type.to_string(),
        source_ref: None,
        source_subpath: None,
        source_revision: None,
        central_path: "/central/name".to_string(),
        content_hash: None,
        created_at: 1,
        updated_at: 1,
        last_sync_at: None,
        last_seen_at: 1,
        status: "ok".to_string(),
        imported_from_tool: None,
    }
}

#[test]
fn the_three_provenances_round_trip_through_their_stored_spelling() {
    for kind in [Provenance::Git, Provenance::Local, Provenance::Imported] {
        assert_eq!(Provenance::parse(kind.as_str()), Some(kind));
    }
    assert_eq!(Provenance::parse("github"), None);
}

#[test]
fn git_and_local_skills_are_refreshable_and_an_imported_skill_is_not() {
    assert!(is_refreshable(&record("git")));
    assert!(is_refreshable(&record("local")));
    assert!(!is_refreshable(&record("imported")));
    assert!(
        !is_refreshable(&record("something-else")),
        "an unknown provenance has no acquisition path"
    );
}

/// Batch membership layers the Unlocatable states over the provenance rule:
/// no external source → not a member (never "skipped"), whatever the disk
/// says; a source → member when locatable, skipped with its state when not.
#[test]
fn refresh_eligibility_separates_not_a_member_from_skipped() {
    let tmp = tempfile::tempdir().unwrap();
    let central = tmp.path().join("central/name");
    std::fs::create_dir_all(&central).unwrap();
    let folder = tmp.path().join("folder");
    std::fs::create_dir_all(&folder).unwrap();

    let mut imported_no_central = record("imported");
    imported_no_central.central_path = "/nowhere/name".to_string();
    assert_eq!(
        refresh_eligibility(&imported_no_central),
        RefreshEligibility::NotAMember,
        "an imported skill is never in the batch, so it is never skipped either"
    );

    let mut git = record("git");
    git.source_ref = Some("https://github.com/o/r".to_string());
    git.central_path = central.to_string_lossy().to_string();
    assert_eq!(refresh_eligibility(&git), RefreshEligibility::Refreshable);
    git.central_path = "/nowhere/name".to_string();
    assert_eq!(
        refresh_eligibility(&git),
        RefreshEligibility::Unlocatable(UnlocatableState::CentralMissing)
    );
    assert!(
        is_refreshable(&git),
        "Restore is an Update: the provenance half still says yes"
    );

    let mut local = record("local");
    local.central_path = central.to_string_lossy().to_string();
    local.source_ref = Some(folder.to_string_lossy().to_string());
    assert_eq!(refresh_eligibility(&local), RefreshEligibility::Refreshable);
    local.source_ref = Some(tmp.path().join("moved").to_string_lossy().to_string());
    assert_eq!(
        refresh_eligibility(&local),
        RefreshEligibility::Unlocatable(UnlocatableState::SourceMissing)
    );
    local.central_path = "/nowhere/name".to_string();
    assert_eq!(
        refresh_eligibility(&local),
        RefreshEligibility::Unlocatable(UnlocatableState::SourceMissing),
        "both gone: the source wins, because Re-point's Update rebuilds central"
    );
}
