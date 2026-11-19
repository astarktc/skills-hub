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

    let entries = list_occupant_files(&occupant, Some(&skill)).unwrap();
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
    let entries = list_occupant_files(&occupant, None).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].compared, FileComparison::OnlyHere);
}
