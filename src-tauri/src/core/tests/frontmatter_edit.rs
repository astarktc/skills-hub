use super::*;

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
    let before = crate::core::content_hash::hash_dir(dir.path()).unwrap();
    std::fs::write(dir.path().join(".skills-hub-manifest-abandoned"), "partial").unwrap();
    assert_eq!(
        crate::core::content_hash::hash_dir(dir.path()).unwrap(),
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
