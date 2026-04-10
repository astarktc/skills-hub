use std::fs;

use tempfile::TempDir;

/// Helper: create a valid lock file JSON with one or more skill entries.
fn write_lock_file(dir: &std::path::Path, skills: &[(&str, &str, &str)]) {
    let mut entries = String::new();
    for (i, (name, source_url, skill_path)) in skills.iter().enumerate() {
        if i > 0 {
            entries.push(',');
        }
        entries.push_str(&format!(
            r#""{name}": {{ "source": "owner/repo", "sourceType": "github", "sourceUrl": "{source_url}", "skillPath": "{skill_path}", "skillFolderHash": "abc123" }}"#,
        ));
    }
    let json = format!(r#"{{ "version": 3, "skills": {{ {entries} }} }}"#);
    fs::write(dir.join(".skill-lock.json"), json).unwrap();
}

#[test]
fn parse_valid_lock_file_returns_entries() {
    let dir = TempDir::new().unwrap();
    write_lock_file(
        dir.path(),
        &[(
            "agent-browser",
            "https://github.com/anthropics/skills.git",
            "skills/agent-browser/SKILL.md",
        )],
    );

    let result = super::parse_lock_file(&dir.path().join(".skill-lock.json"));
    assert!(result.is_some(), "should parse valid lock file");
    let map = result.unwrap();
    assert!(map.contains_key("agent-browser"));
    let entry = &map["agent-browser"];
    assert_eq!(entry.source_url, "https://github.com/anthropics/skills.git");
    assert_eq!(
        entry.source_subpath.as_deref(),
        Some("skills/agent-browser")
    );
}

#[test]
fn parse_lock_file_missing_skills_key_returns_none() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join(".skill-lock.json"), r#"{ "version": 3 }"#).unwrap();

    let result = super::parse_lock_file(&dir.path().join(".skill-lock.json"));
    // Missing "skills" key means no entries -- should return None or empty
    assert!(
        result.is_none() || result.as_ref().is_some_and(|m| m.is_empty()),
        "missing skills key should yield empty/None"
    );
}

#[test]
fn parse_lock_file_with_extra_fields_succeeds() {
    let dir = TempDir::new().unwrap();
    let json = r#"{
        "version": 3,
        "unknownField": true,
        "skills": {
            "my-skill": {
                "source": "owner/repo",
                "sourceType": "github",
                "sourceUrl": "https://github.com/owner/repo.git",
                "skillPath": "skills/my-skill/SKILL.md",
                "skillFolderHash": "abc123",
                "extraField": "should be ignored"
            }
        }
    }"#;
    fs::write(dir.path().join(".skill-lock.json"), json).unwrap();

    let result = super::parse_lock_file(&dir.path().join(".skill-lock.json"));
    assert!(result.is_some(), "extra fields should not break parsing");
    let map = result.unwrap();
    assert!(map.contains_key("my-skill"));
}

#[test]
fn try_enrich_symlink_into_agents_skills_returns_entry() {
    let dir = TempDir::new().unwrap();
    let agents_dir = dir.path().join(".agents");
    let skills_dir = agents_dir.join("skills");
    let skill_dir = skills_dir.join("test-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), "---\nname: test-skill\n---\n").unwrap();

    write_lock_file(
        &agents_dir,
        &[(
            "test-skill",
            "https://github.com/owner/repo.git",
            "skills/test-skill/SKILL.md",
        )],
    );

    // Create a symlink in a "tool dir" pointing into agents skills
    let tool_dir = dir.path().join("tool");
    fs::create_dir_all(&tool_dir).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&skill_dir, tool_dir.join("test-skill")).unwrap();

    let result =
        super::try_enrich_from_skill_lock_with_home(&tool_dir.join("test-skill"), dir.path());
    assert!(
        result.is_some(),
        "should find lock entry for symlinked skill"
    );
    let entry = result.unwrap();
    assert_eq!(entry.source_url, "https://github.com/owner/repo.git");
    assert_eq!(entry.source_subpath.as_deref(), Some("skills/test-skill"));
}

#[test]
fn try_enrich_non_symlink_returns_none() {
    let dir = TempDir::new().unwrap();
    let regular_dir = dir.path().join("regular-skill");
    fs::create_dir_all(&regular_dir).unwrap();
    fs::write(regular_dir.join("SKILL.md"), "---\nname: x\n---\n").unwrap();

    let result = super::try_enrich_from_skill_lock_with_home(&regular_dir, dir.path());
    assert!(result.is_none(), "non-symlink should return None");
}

#[test]
fn try_enrich_symlink_not_in_lock_file_returns_none() {
    let dir = TempDir::new().unwrap();
    let agents_dir = dir.path().join(".agents");
    let skills_dir = agents_dir.join("skills");
    let skill_dir = skills_dir.join("unlisted-skill");
    fs::create_dir_all(&skill_dir).unwrap();

    // Lock file exists but does NOT contain "unlisted-skill"
    write_lock_file(
        &agents_dir,
        &[(
            "other-skill",
            "https://github.com/owner/repo.git",
            "skills/other-skill/SKILL.md",
        )],
    );

    let tool_dir = dir.path().join("tool");
    fs::create_dir_all(&tool_dir).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&skill_dir, tool_dir.join("unlisted-skill")).unwrap();

    let result =
        super::try_enrich_from_skill_lock_with_home(&tool_dir.join("unlisted-skill"), dir.path());
    assert!(
        result.is_none(),
        "skill not in lock file should return None"
    );
}

#[test]
fn try_enrich_nonexistent_lock_file_returns_none() {
    let dir = TempDir::new().unwrap();
    let agents_dir = dir.path().join(".agents");
    let skills_dir = agents_dir.join("skills");
    let skill_dir = skills_dir.join("some-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    // No lock file created

    let tool_dir = dir.path().join("tool");
    fs::create_dir_all(&tool_dir).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&skill_dir, tool_dir.join("some-skill")).unwrap();

    let result =
        super::try_enrich_from_skill_lock_with_home(&tool_dir.join("some-skill"), dir.path());
    assert!(
        result.is_none(),
        "missing lock file should return None gracefully"
    );
}

#[test]
fn skill_path_derives_subpath_correctly() {
    // "skills/agent-browser/SKILL.md" -> "skills/agent-browser"
    assert_eq!(
        super::derive_subpath("skills/agent-browser/SKILL.md").as_deref(),
        Some("skills/agent-browser")
    );

    // "SKILL.md" (root-level) -> None
    assert_eq!(super::derive_subpath("SKILL.md"), None);

    // "deep/nested/path/SKILL.md" -> "deep/nested/path"
    assert_eq!(
        super::derive_subpath("deep/nested/path/SKILL.md").as_deref(),
        Some("deep/nested/path")
    );
}

#[test]
fn try_enrich_symlink_outside_agents_skills_returns_none() {
    let dir = TempDir::new().unwrap();
    // Create a directory NOT under .agents/skills/
    let other_dir = dir.path().join("other-location").join("some-skill");
    fs::create_dir_all(&other_dir).unwrap();

    let tool_dir = dir.path().join("tool");
    fs::create_dir_all(&tool_dir).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&other_dir, tool_dir.join("some-skill")).unwrap();

    let result =
        super::try_enrich_from_skill_lock_with_home(&tool_dir.join("some-skill"), dir.path());
    assert!(
        result.is_none(),
        "symlink outside ~/.agents/skills/ should return None"
    );
}
