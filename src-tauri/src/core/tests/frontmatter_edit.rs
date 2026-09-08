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
fn every_mode_round_trips_without_frontmatter() {
    let text = "# Skill\nBody\n";
    for mode in [
        InvocationMode::UserAndModel,
        InvocationMode::UserOnly,
        InvocationMode::ModelOnly,
        InvocationMode::Neither,
    ] {
        let base = read_invocation_lines(text);
        let edited = write_invocation_mode(text, mode);
        assert_eq!(parse_invocation_mode(&edited), mode);
        assert_eq!(restore_invocation_lines(&edited, &base), text);
    }
}
