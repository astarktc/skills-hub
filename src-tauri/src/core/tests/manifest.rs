use super::*;
use std::fs;

#[test]
fn complete_column_zero_fences_match_presentation_corpus() {
    #[derive(Deserialize)]
    struct Case {
        label: String,
        raw: String,
        meta: Option<std::collections::HashMap<String, String>>,
        mode: InvocationMode,
    }
    // The presentation adapter consumes these same literal inputs/expectations.
    let cases: Vec<Case> = serde_json::from_str(include_str!(
        "../../../../src/lib/manifestPresentation.corpus.json"
    ))
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("SKILL.md");
    for case in cases {
        fs::write(&path, &case.raw).unwrap();
        let expected = case
            .meta
            .as_ref()
            .map(|meta| (meta["name"].clone(), meta.get("description").cloned()));
        assert_eq!(parse_skill_md(&path), expected, "{}", case.label);
        assert_eq!(
            parse_invocation_mode(&case.raw),
            case.mode,
            "{}",
            case.label
        );
        assert_eq!(
            invocation_mode_for_dir(dir.path()),
            case.mode,
            "{}",
            case.label
        );
        let base = read_invocation_lines(&case.raw);
        assert_eq!(base.had_frontmatter, case.meta.is_some(), "{}", case.label);
        assert_eq!(base.mode(), case.mode, "{}", case.label);
        assert_eq!(
            write_invocation_mode(&case.raw, case.mode),
            case.raw,
            "{}",
            case.label
        );
        let edited = write_invocation_mode(&case.raw, InvocationMode::UserOnly);
        assert_eq!(
            parse_invocation_mode(&edited),
            InvocationMode::UserOnly,
            "{}",
            case.label
        );
        assert_eq!(
            restore_invocation_lines(&edited, &base),
            case.raw,
            "{}",
            case.label
        );

        // Real consumers, not a Manifest stub: discovery, finalize and catalog
        // all agree about the very same bytes (including malformed fences).
        let candidates = crate::core::skill_discovery::discover_skills(dir.path());
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].validity.is_valid(), expected.is_some());
        assert_eq!(
            candidates[0].name,
            expected.as_ref().map_or("root-skill", |(name, _)| name)
        );
        assert_eq!(
            candidates[0].description,
            expected.as_ref().and_then(|(_, desc)| desc.clone())
        );
        let central = tempfile::tempdir().unwrap();
        let store = crate::core::skill_store::SkillStore::new(central.path().join("test.db"));
        store.ensure_schema().unwrap();
        let staged = crate::core::install_finalize::StagingDir::new_in(central.path());
        fs::create_dir_all(staged.path()).unwrap();
        fs::write(staged.path().join("SKILL.md"), &case.raw).unwrap();
        let installed = crate::core::install_finalize::finalize_install(
            &store,
            central.path(),
            staged,
            crate::core::install_finalize::NameIntent::Derived("fallback".into()),
            crate::core::install_finalize::SkillProvenance::imported(None),
        )
        .unwrap();
        let entry = crate::core::skill_catalog::managed_skill_entry(&store, &installed.skill_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            entry.skill.name,
            expected.as_ref().map_or("fallback", |(name, _)| name)
        );
        assert_eq!(entry.skill.description, expected.and_then(|(_, desc)| desc));
        assert_eq!(entry.invocation_mode, case.mode);
    }
}

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
fn required_reads_are_typed_while_optional_reads_remain_permissive() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("SKILL.md");
    for shape in ["missing", "invalid_utf8", "directory"] {
        match shape {
            "invalid_utf8" => fs::write(&path, [0xff]).unwrap(),
            "directory" => {
                fs::remove_file(&path).unwrap();
                fs::create_dir(&path).unwrap();
            }
            _ => {}
        }
        assert_eq!(parse_skill_md_with_reason(&path), Err("read_failed"));
        assert_eq!(parse_skill_md(&path), None);
        assert_eq!(
            invocation_mode_for_dir(dir.path()),
            InvocationMode::UserAndModel
        );
        let error = read_manifest(&path).unwrap_err();
        assert!(error.downcast_ref::<std::io::Error>().is_some());
        assert!(matches!(
            crate::commands::error::CommandError::from_anyhow(error),
            crate::commands::error::CommandError::SkillManifestIo { .. }
        ));
    }
}

#[cfg(unix)]
#[test]
fn file_edits_preserve_permissions_and_same_mode_preserves_file_identity() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("SKILL.md");
    let original = "---\nname: alpha\nuser-invocable: false  \n---\nBody\n";
    fs::write(&path, original).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    let before = fs::metadata(&path).unwrap();
    apply_to_file(&path, |text| {
        write_invocation_mode(text, InvocationMode::ModelOnly)
    })
    .unwrap();
    let after = fs::metadata(&path).unwrap();
    assert_eq!(after.ino(), before.ino());
    assert_eq!(after.modified().unwrap(), before.modified().unwrap());
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    apply_to_file(&path, |text| {
        write_invocation_mode(text, InvocationMode::UserOnly)
    })
    .unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "---\nname: alpha\nuser-invocable: true\ndisable-model-invocation: true\n---\nBody\n"
    );
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
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
