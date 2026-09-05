use crate::core::provenance::{is_refreshable, Provenance};
use crate::core::skill_store::SkillRecord;

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
