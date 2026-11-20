use super::*;

#[test]
fn a_reported_occupant_is_admitted_exactly_as_reported() {
    let base = PathBuf::from(format!("/occupants-test/{}", uuid::Uuid::new_v4()));
    let occupant = base.join("skills/alpha");
    assert!(!is_reported(&occupant));
    report(&occupant);
    assert!(is_reported(&occupant));
    // A trailing separator spells the same entry.
    assert!(is_reported(Path::new(&format!("{}/", occupant.display()))));
    // Nothing inside it, above it or beside it is reported.
    assert!(!is_reported(&occupant.join("SKILL.md")));
    assert!(!is_reported(&base.join("skills")));
    assert!(!is_reported(&base.join("skills/beta")));
    assert!(reported().contains(&occupant));
}

#[test]
fn only_the_most_recent_occupants_are_remembered() {
    let base = PathBuf::from(format!("/occupants-cap/{}", uuid::Uuid::new_v4()));
    let first = base.join("first");
    report(&first);
    for index in 0..CAPACITY {
        report(&base.join(format!("later-{index}")));
    }
    assert!(!is_reported(&first));
    assert!(is_reported(&base.join(format!("later-{}", CAPACITY - 1))));
}
