use super::*;
use std::fs;

#[test]
fn corpus_round_trips_preserving_unrelated_bytes_and_no_op() {
    let corpus = [
        "",
        "# Body\r\n",
        "---\nname: skill\n---\nBody",
        "---\ndisable-model-invocation: yes  \nname: skill\n---\nBody\n",
        "---\nuser-invocable: 'no'  \n---\n",
        "---\nuser-invocable: false\nname: skill\ndisable-model-invocation: true\n---\nBody",
        "---\nmetadata:\n  user-invocable: false\n  disable-model-invocation: true\n---\nBody",
        "---\r\nuser-invocable: false  \r\nname: skill\r\n---\r\nBody\r\n",
        "---\n---",
        "---\nname: skill\n---\n---\ntrailing\n",
        "---\nuser-invocable: no\nname: skill\nuser-invocable: yes\n---\n",
        "---\nname: unfinished\n",
        "---\r\n---\r\nBody",
        "---\nuser-invocable: false\r\ndisable-model-invocation: false\n---",
    ];
    for text in corpus {
        assert_eq!(
            write_invocation_mode(text, parse_invocation_mode(text)),
            text
        );
        for mode in [
            InvocationMode::UserAndModel,
            InvocationMode::UserOnly,
            InvocationMode::ModelOnly,
            InvocationMode::Neither,
        ] {
            let edited = write_invocation_mode(text, mode);
            assert_eq!(parse_invocation_mode(&edited), mode, "{text:?}");
            assert_eq!(
                restore_invocation_lines(&edited, &read_invocation_lines(text)),
                text,
                "{text:?}"
            );
        }
    }
}

#[test]
fn indented_fence_is_description_content_for_every_parser_and_writer() {
    let text = "---\nname: fenced\ndescription: |\n  before\n  ---\n  after\ndisable-model-invocation: true\nuser-invocable: true\n---  \nBody\n---\nunchanged\n";
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("SKILL.md");
    std::fs::write(&path, text).unwrap();
    assert_eq!(
        crate::core::skill_discovery::parse_skill_md_with_reason(&path).unwrap(),
        ("fenced".into(), Some("before\n---\nafter".into()))
    );
    assert_eq!(parse_invocation_mode(text), InvocationMode::UserOnly);
    let base = read_invocation_lines(text);
    assert!(base.had_frontmatter);
    assert_eq!(base.mode(), InvocationMode::UserOnly);
    let edited = write_invocation_mode(text, InvocationMode::ModelOnly);
    assert_eq!(
        edited,
        text.replace(
            "disable-model-invocation: true",
            "disable-model-invocation: false"
        )
        .replace("user-invocable: true", "user-invocable: false")
    );
    assert_eq!(parse_invocation_mode(&edited), InvocationMode::ModelOnly);
    assert_eq!(restore_invocation_lines(&edited, &base), text);
}

#[test]
fn replaces_top_level_lines_in_place_and_appends_only_missing_keys() {
    let text = "---\r\nuser-invocable: false  \r\nmetadata:\r\n  disable-model-invocation: false\r\n# comment\r\n---\r\nBody\n---\n";
    assert_eq!(write_invocation_mode(text, InvocationMode::UserOnly), "---\r\nuser-invocable: true\r\nmetadata:\r\n  disable-model-invocation: false\r\n# comment\r\ndisable-model-invocation: true\r\n---\r\nBody\n---\n");
}

#[test]
fn rename_failure_preserves_bytes_and_cleans_temp() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("SKILL.md");
    let backup = dir.path().join("original");
    std::fs::write(&path, "original bytes").unwrap();
    let error = apply_to_file(&path, |_| {
        // Change the target after the read: rename cannot replace a directory.
        std::fs::rename(&path, &backup).unwrap();
        std::fs::create_dir(&path).unwrap();
        "new bytes".into()
    })
    .unwrap_err();
    assert_eq!(std::fs::read_to_string(backup).unwrap(), "original bytes");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    assert!(error.downcast_ref::<std::io::Error>().is_some());
    assert!(matches!(
        error.downcast_ref::<super::super::errors::SignalError>(),
        Some(super::super::errors::SignalError::SkillManifestIo { .. })
    ));
}

#[test]
fn abandoned_temp_is_not_content() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("SKILL.md"), "original bytes").unwrap();
    let before = crate::core::content_identity::read(
        crate::core::content_identity::Source::Directory(dir.path()),
    )
    .unwrap();
    std::fs::write(dir.path().join(".skills-hub-manifest-abandoned"), "partial").unwrap();
    assert_eq!(
        crate::core::content_identity::read(crate::core::content_identity::Source::Directory(
            dir.path()
        ))
        .unwrap(),
        before
    );
}

#[test]
fn clear_preserves_later_frontmatter_additions() {
    let base = read_invocation_lines("Body\n");
    for addition in [
        "name: added\n",
        "# comment\n",
        "metadata:\n  nested: yes\n",
        "\n",
    ] {
        let edited = write_invocation_mode("Body\n", InvocationMode::UserOnly);
        let edited = edited.replacen("---\n", &format!("---\n{addition}"), 1);
        assert_eq!(
            restore_invocation_lines(&edited, &base),
            format!("---\n{addition}---\nBody\n")
        );
    }
}

#[test]
fn parses_skill_md_frontmatter() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("SKILL.md");
    fs::write(
        &p,
        r#"---
name: "My Skill"
description: "Desc"
---

body
"#,
    )
    .unwrap();

    let (name, desc) = parse_skill_md(&p).unwrap();
    assert_eq!(name, "My Skill");
    assert_eq!(desc.as_deref(), Some("Desc"));
}

#[test]
fn parses_skill_md_frontmatter_literal_description() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("SKILL.md");
    fs::write(
        &p,
        r#"---
name: technical-writer
description: |
  Creates clear documentation, API references, guides, and
  technical content for developers and users.
author: awesome-llm-apps
---

body
"#,
    )
    .unwrap();

    let (name, desc) = parse_skill_md(&p).unwrap();
    assert_eq!(name, "technical-writer");
    assert_eq!(
        desc.as_deref(),
        Some("Creates clear documentation, API references, guides, and\ntechnical content for developers and users.")
    );
}

#[test]
fn parses_skill_md_frontmatter_folded_description() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("SKILL.md");
    fs::write(&p, "---\nname: folded\ndescription: >\n  one\n  two\n---\n").unwrap();
    let (_, desc) = parse_skill_md(&p).unwrap();
    assert_eq!(desc.as_deref(), Some("one two"));
}

#[test]
fn parse_skill_md_with_reason_reports_each_failure() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("SKILL.md");
    assert_eq!(parse_skill_md_with_reason(&p).unwrap_err(), "read_failed");
    fs::write(&p, "name: x\n").unwrap();
    assert_eq!(
        parse_skill_md_with_reason(&p).unwrap_err(),
        "invalid_frontmatter"
    );
    fs::write(&p, "---\nname: x\n").unwrap();
    assert_eq!(
        parse_skill_md_with_reason(&p).unwrap_err(),
        "invalid_frontmatter"
    );
    fs::write(&p, "---\ndescription: x\n---\n").unwrap();
    assert_eq!(parse_skill_md_with_reason(&p).unwrap_err(), "missing_name");
}

// ── Invocation mode ──

#[test]
fn invocation_mode_maps_every_frontmatter_shape() {
    use super::InvocationMode as M;

    let cases: Vec<(&str, M)> = vec![
        // No frontmatter at all, and unterminated frontmatter: default.
        ("just a body\n", M::UserAndModel),
        ("---\nname: x\n", M::UserAndModel),
        ("", M::UserAndModel),
        // Keys absent.
        ("---\nname: x\ndescription: d\n---\nbody\n", M::UserAndModel),
        // Explicit permissive values.
        (
            "---\nname: x\ndisable-model-invocation: false\nuser-invocable: true\n---\n",
            M::UserAndModel,
        ),
        // User only.
        (
            "---\nname: x\ndisable-model-invocation: true\n---\n",
            M::UserOnly,
        ),
        (
            "---\nname: x\ndisable-model-invocation: yes\n---\n",
            M::UserOnly,
        ),
        (
            "---\nname: x\ndisable-model-invocation: ON\n---\n",
            M::UserOnly,
        ),
        (
            "---\nname: x\ndisable-model-invocation: 1\n---\n",
            M::UserOnly,
        ),
        (
            "---\nname: x\ndisable-model-invocation: \"true\"\n---\n",
            M::UserOnly,
        ),
        // Model only.
        ("---\nname: x\nuser-invocable: false\n---\n", M::ModelOnly),
        ("---\nname: x\nuser-invocable: no\n---\n", M::ModelOnly),
        ("---\nname: x\nuser-invocable: 0\n---\n", M::ModelOnly),
        // Both restrictions: nobody can invoke it.
        (
            "---\nname: x\ndisable-model-invocation: true\nuser-invocable: false\n---\n",
            M::Neither,
        ),
        // Malformed values fall back to each key's default.
        (
            "---\nname: x\ndisable-model-invocation: maybe\nuser-invocable: sometimes\n---\n",
            M::UserAndModel,
        ),
        (
            "---\nname: x\ndisable-model-invocation:\n---\n",
            M::UserAndModel,
        ),
        // Nested mappings never contribute top-level keys.
        (
            "---\nname: x\nmetadata:\n  disable-model-invocation: true\n---\n",
            M::UserAndModel,
        ),
        // A restriction after the closing marker is body text, not frontmatter.
        (
            "---\nname: x\n---\ndisable-model-invocation: true\n",
            M::UserAndModel,
        ),
    ];

    for (raw, expected) in cases {
        assert_eq!(parse_invocation_mode(raw), expected, "input: {raw:?}");
    }
}

#[test]
fn invocation_mode_for_dir_defaults_without_readable_skill_md() {
    let dir = tempfile::tempdir().unwrap();
    // No SKILL.md at all.
    assert_eq!(
        invocation_mode_for_dir(dir.path()),
        InvocationMode::UserAndModel
    );
    // Case-insensitive lookup, restriction honoured.
    fs::write(
        dir.path().join("Skill.md"),
        "---\nname: x\nuser-invocable: false\n---\n",
    )
    .unwrap();
    assert_eq!(
        invocation_mode_for_dir(dir.path()),
        InvocationMode::ModelOnly
    );
}
