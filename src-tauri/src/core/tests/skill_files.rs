use std::fs;

use super::*;

fn write(root: &Path, files: &[(&str, &str)]) {
    for (path, content) in files {
        let at = root.join(path);
        fs::create_dir_all(at.parent().unwrap()).unwrap();
        fs::write(at, content).unwrap();
    }
}

/// The occupant's files compared with the skill's by path and bytes, then
/// the skill's files it lacks.
#[test]
fn an_occupants_files_are_compared_with_the_skill() {
    let tmp = tempfile::tempdir().unwrap();
    let occupant = tmp.path().join("occupant");
    let skill = tmp.path().join("skill");
    write(
        &occupant,
        &[
            ("SKILL.md", "theirs"),
            ("notes.md", "same"),
            ("extra.txt", "x"),
        ],
    );
    write(
        &skill,
        &[
            ("SKILL.md", "ours"),
            ("notes.md", "same"),
            ("scripts/run.sh", "echo"),
        ],
    );

    let listing = list_occupant_files(&occupant, Some(&skill)).unwrap();
    assert!(!listing.truncated);
    let entries = listing.entries;
    let compared: Vec<(&str, FileComparison)> = entries
        .iter()
        .map(|e| (e.path.as_str(), e.compared))
        .collect();
    assert_eq!(
        compared,
        vec![
            ("SKILL.md", FileComparison::Changed),
            ("extra.txt", FileComparison::OnlyHere),
            ("notes.md", FileComparison::Same),
            (
                Path::new("scripts").join("run.sh").to_str().unwrap(),
                FileComparison::OnlyInSkill
            ),
        ]
    );

    let file = read_occupant_file(&occupant, Some(&skill), "SKILL.md").unwrap();
    assert_eq!(file.occupant.as_deref(), Some("theirs"));
    assert_eq!(file.skill.as_deref(), Some("ours"));
    let missing = read_occupant_file(&occupant, Some(&skill), "extra.txt").unwrap();
    assert_eq!(missing.skill, None);
    assert!(read_occupant_file(&occupant, Some(&skill), "../skill/SKILL.md").is_err());
}

/// Without a skill copy to compare with, every file is the occupant's alone.
#[test]
fn without_the_skill_every_file_is_the_occupants() {
    let tmp = tempfile::tempdir().unwrap();
    let occupant = tmp.path().join("occupant");
    write(&occupant, &[("SKILL.md", "theirs")]);
    let entries = list_occupant_files(&occupant, None).unwrap().entries;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].compared, FileComparison::OnlyHere);
}

/// Past the listing's bound the occupant still has the skill's files: each is
/// probed directly, never reported missing, and the listing says it is cut.
#[test]
fn a_bounded_listing_never_reports_an_existing_file_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let occupant = tmp.path().join("occupant");
    let skill = tmp.path().join("skill");
    let names: Vec<String> = (0..=OCCUPANT_FILES_MAX)
        .map(|i| format!("f{i:04}.md"))
        .collect();
    let files: Vec<(&str, &str)> = names.iter().map(|n| (n.as_str(), "same")).collect();
    write(&occupant, &files);
    write(&skill, &files);

    let listing = list_occupant_files(&occupant, Some(&skill)).unwrap();
    assert!(listing.truncated);
    assert_eq!(listing.entries.len(), OCCUPANT_FILES_MAX + 1);
    assert!(listing
        .entries
        .iter()
        .all(|e| e.compared == FileComparison::Same));

    // Exactly at the bound nothing is cut.
    fs::remove_file(occupant.join("f0000.md")).unwrap();
    let listing = list_occupant_files(&occupant, Some(&skill)).unwrap();
    assert!(!listing.truncated);
    let missing: Vec<&str> = listing
        .entries
        .iter()
        .filter(|e| e.compared == FileComparison::OnlyInSkill)
        .map(|e| e.path.as_str())
        .collect();
    assert_eq!(missing, vec!["f0000.md"]);
}

/// The probe for a skill file keeps `read_file`'s containment: a link in the
/// occupant leading out of it is not the occupant's file.
#[cfg(unix)]
#[test]
fn the_probe_never_follows_a_link_out_of_the_occupant() {
    let tmp = tempfile::tempdir().unwrap();
    let occupant = tmp.path().join("occupant");
    let skill = tmp.path().join("skill");
    write(&occupant, &[("SKILL.md", "theirs")]);
    write(&skill, &[("SKILL.md", "ours"), ("notes.md", "n")]);
    std::os::unix::fs::symlink(skill.join("notes.md"), occupant.join("notes.md")).unwrap();

    let entries = list_occupant_files(&occupant, Some(&skill))
        .unwrap()
        .entries;
    let notes = entries.iter().find(|e| e.path == "notes.md").unwrap();
    assert_eq!(notes.compared, FileComparison::OnlyInSkill);
}

/// Byte equality streams: same length with different bytes (past one buffer)
/// is changed, equal bytes are the same, and a different length is decided
/// without opening either file.
#[test]
fn same_bytes_compares_by_length_then_streamed_bytes() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    let big = vec![7u8; 200 * 1024];
    let mut other = big.clone();
    *other.last_mut().unwrap() = 8;
    fs::write(&a, &big).unwrap();
    fs::write(&b, &other).unwrap();
    assert!(!same_bytes(&a, &b).unwrap());
    fs::write(&b, &big).unwrap();
    assert!(same_bytes(&a, &b).unwrap());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // Unreadable: equal lengths must open it (an error); different
        // lengths never do.
        fs::set_permissions(&b, fs::Permissions::from_mode(0o000)).unwrap();
        if fs::File::open(&b).is_err() {
            assert!(same_bytes(&a, &b).is_err());
            fs::write(&a, b"short").unwrap();
            assert!(!same_bytes(&a, &b).unwrap());
        }
        fs::set_permissions(&b, fs::Permissions::from_mode(0o644)).unwrap();
    }
}
