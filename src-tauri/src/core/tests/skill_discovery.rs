use std::fs;
use std::path::Path;

use super::{discover_skills, DiscoveredSkill, Validity};

fn write_skill(base: &Path, rel: &str, name: &str) {
    fs::create_dir_all(base.join(rel)).unwrap();
    fs::write(
        base.join(rel).join("SKILL.md"),
        format!("---\nname: {}\ndescription: {} desc\n---\n", name, name),
    )
    .unwrap();
}

fn subpaths(list: &[DiscoveredSkill]) -> Vec<String> {
    list.iter().map(|c| c.subpath.clone()).collect()
}

fn find<'a>(list: &'a [DiscoveredSkill], subpath: &str) -> &'a DiscoveredSkill {
    list.iter()
        .find(|c| c.subpath == subpath)
        .unwrap_or_else(|| panic!("candidate {subpath} missing from {:?}", subpaths(list)))
}

// ── Strategy: root SKILL.md ──

#[test]
fn root_skill_md_is_the_dot_candidate() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("SKILL.md"),
        "---\nname: Root\ndescription: root desc\n---\n",
    )
    .unwrap();

    let list = discover_skills(dir.path());
    assert_eq!(subpaths(&list), vec![".".to_string()]);
    let root = find(&list, ".");
    assert_eq!(root.name, "Root");
    assert_eq!(root.description.as_deref(), Some("root desc"));
    assert_eq!(root.validity, Validity::Valid);
}

#[test]
fn root_with_broken_skill_md_is_invalid_with_reason_and_fixed_name() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("my-folder");
    fs::create_dir_all(&base).unwrap();
    fs::write(base.join("SKILL.md"), "name: no frontmatter\n").unwrap();

    let list = discover_skills(&base);
    let root = find(&list, ".");
    assert_eq!(root.name, "root-skill");
    assert_eq!(
        root.validity,
        Validity::InvalidSkillMd("invalid_frontmatter")
    );
    assert_eq!(root.validity.reason(), Some("invalid_frontmatter"));
    assert!(!root.validity.is_valid());
    assert!(root.validity.is_installable());
}

#[test]
fn root_without_skill_md_has_no_dot_candidate() {
    let dir = tempfile::tempdir().unwrap();
    write_skill(dir.path(), "skills/a", "A");
    let list = discover_skills(dir.path());
    assert!(!subpaths(&list).contains(&".".to_string()));
}

// ── Strategy: known scan bases ──

#[test]
fn scan_base_children_are_candidates_with_validity() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    write_skill(base, "skills/a", "A");
    fs::create_dir_all(base.join("skills/b")).unwrap();
    fs::create_dir_all(base.join("skills/c")).unwrap();
    fs::write(base.join("skills/c/SKILL.md"), "name: C\n").unwrap();
    fs::create_dir_all(base.join("skills/d")).unwrap();
    fs::write(base.join("skills/d/SKILL.md"), "---\ndescription: D\n---\n").unwrap();
    fs::write(base.join("skills/README.md"), "not a dir").unwrap();

    let list = discover_skills(base);
    assert_eq!(
        subpaths(&list),
        vec!["skills/a", "skills/b", "skills/c", "skills/d"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>()
    );

    let a = find(&list, "skills/a");
    assert_eq!(a.name, "A");
    assert_eq!(a.validity, Validity::Valid);

    let b = find(&list, "skills/b");
    assert_eq!(b.name, "b");
    assert_eq!(b.validity, Validity::MissingSkillMd);
    assert_eq!(b.validity.reason(), Some("missing_skill_md"));
    assert!(!b.validity.is_installable());

    let c = find(&list, "skills/c");
    assert_eq!(c.name, "c");
    assert_eq!(c.validity, Validity::InvalidSkillMd("invalid_frontmatter"));

    let d = find(&list, "skills/d");
    assert_eq!(d.validity, Validity::InvalidSkillMd("missing_name"));
}

#[test]
fn every_known_scan_base_is_scanned() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    write_skill(base, "skills/.curated/cur", "cur");
    write_skill(base, "skills/.experimental/exp", "exp");
    write_skill(base, "skills/.system/sys", "sys");
    write_skill(base, ".claude/skills/cl", "cl");

    let list = discover_skills(base);
    let mut got = subpaths(&list);
    got.sort();
    assert_eq!(
        got,
        vec![
            ".claude/skills/cl",
            "skills/.curated/cur",
            "skills/.experimental/exp",
            "skills/.system/sys",
        ]
    );
}

#[test]
fn claude_skills_child_without_skill_md_is_valid_with_plugin_description() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    fs::create_dir_all(base.join(".claude/skills/plugin-skill")).unwrap();
    fs::create_dir_all(base.join(".claude-plugin")).unwrap();
    fs::write(
        base.join(".claude-plugin/plugin.json"),
        r#"{"description":"from plugin.json"}"#,
    )
    .unwrap();

    let list = discover_skills(base);
    let c = find(&list, ".claude/skills/plugin-skill");
    assert_eq!(c.name, "plugin-skill");
    assert_eq!(c.description.as_deref(), Some("from plugin.json"));
    assert_eq!(c.validity, Validity::Valid);
}

#[test]
fn invalid_skill_md_falls_back_to_folder_name_and_plugin_description() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    fs::create_dir_all(base.join("skills/broken")).unwrap();
    fs::write(base.join("skills/broken/SKILL.md"), "---\nno end\n").unwrap();
    fs::create_dir_all(base.join(".claude-plugin")).unwrap();
    fs::write(
        base.join(".claude-plugin/plugin.json"),
        r#"{"description":"plugin desc"}"#,
    )
    .unwrap();

    let list = discover_skills(base);
    let c = find(&list, "skills/broken");
    assert_eq!(c.name, "broken");
    assert_eq!(c.description.as_deref(), Some("plugin desc"));
    assert_eq!(c.validity, Validity::InvalidSkillMd("invalid_frontmatter"));
}

// ── Strategy: root-level skills and skill containers ──

#[test]
fn root_level_skill_dirs_are_found_and_plain_dirs_are_not_listed() {
    let dir = tempfile::tempdir().unwrap();
    write_skill(dir.path(), "technical-writer", "technical-writer");
    write_skill(dir.path(), "python-expert", "python-expert");
    fs::create_dir_all(dir.path().join("not-a-skill")).unwrap();

    let list = discover_skills(dir.path());
    assert_eq!(
        subpaths(&list),
        vec!["python-expert".to_string(), "technical-writer".to_string()]
    );
}

#[test]
fn named_skill_containers_are_scanned_but_generic_dirs_only_via_recursion() {
    let dir = tempfile::tempdir().unwrap();
    write_skill(dir.path(), "agent-pack/hidden-skill", "hidden");
    write_skill(dir.path(), "agent-skills/visible-skill", "visible");

    let list = discover_skills(dir.path());
    // Recursion (depth 5) also reaches agent-pack/hidden-skill; the container
    // stage alone would only find agent-skills/visible-skill.
    assert_eq!(
        subpaths(&list),
        vec![
            "agent-pack/hidden-skill".to_string(),
            "agent-skills/visible-skill".to_string()
        ]
    );
}

// ── Strategy: marketplace.json ──

#[test]
fn marketplace_plugins_contribute_their_skills() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    fs::create_dir_all(base.join(".claude-plugin")).unwrap();
    fs::write(
        base.join(".claude-plugin/marketplace.json"),
        r#"{"plugins":[{"name":"backend","source":"./plugins/backend"},{"name":"direct","source":"./plugins/direct"},{"name":"escape","source":"../outside"}]}"#,
    )
    .unwrap();
    write_skill(base, "plugins/backend/skills/api-design", "api-design");
    write_skill(base, "plugins/direct/child-skill", "child");
    // A plugin dir named "skills" must not be scanned twice.
    fs::create_dir_all(base.join("plugins/direct/skills")).unwrap();

    let list = discover_skills(base);
    assert_eq!(
        subpaths(&list),
        vec![
            "plugins/backend/skills/api-design".to_string(),
            "plugins/direct/child-skill".to_string()
        ]
    );
    assert_eq!(
        find(&list, "plugins/backend/skills/api-design").name,
        "api-design"
    );
    assert_eq!(
        find(&list, "plugins/direct/child-skill").validity,
        Validity::Valid
    );
}

#[test]
fn marketplace_is_ignored_when_missing_or_malformed() {
    let dir = tempfile::tempdir().unwrap();
    assert!(discover_skills(dir.path()).is_empty());

    fs::create_dir_all(dir.path().join(".claude-plugin")).unwrap();
    fs::write(
        dir.path().join(".claude-plugin/marketplace.json"),
        "{ not json",
    )
    .unwrap();
    assert!(discover_skills(dir.path()).is_empty());
}

// ── Strategy: recursive fallback ──

#[test]
fn recursive_fallback_finds_deeply_nested_skills() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    write_skill(base, "plugins/backend/skills/api-design", "api-design");
    write_skill(base, "plugins/frontend/skills/tailwind", "tailwind");
    write_skill(base, "a/b/c/d/e", "depth5");

    let list = discover_skills(base);
    assert_eq!(
        subpaths(&list),
        vec![
            "plugins/backend/skills/api-design".to_string(),
            "a/b/c/d/e".to_string(),
            "plugins/frontend/skills/tailwind".to_string(),
        ]
    );
}

#[test]
fn recursive_fallback_respects_max_depth() {
    let dir = tempfile::tempdir().unwrap();
    write_skill(dir.path(), "a/b/c/d/e/f", "too-deep");
    assert!(discover_skills(dir.path()).is_empty());
}

#[test]
fn recursive_fallback_skips_heavy_and_hidden_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    write_skill(base, "node_modules/pkg/skill", "nm");
    write_skill(base, ".git/skill", "git");
    write_skill(base, "dist/skill", "dist");
    write_skill(base, ".hidden/skill", "hidden");
    write_skill(base, "real/skill", "real");

    let list = discover_skills(base);
    assert_eq!(subpaths(&list), vec!["real/skill".to_string()]);
}

#[test]
fn recursive_hit_with_broken_skill_md_is_invalid() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("x/y/broken")).unwrap();
    fs::write(dir.path().join("x/y/broken/SKILL.md"), "nope\n").unwrap();

    let list = discover_skills(dir.path());
    let c = find(&list, "x/y/broken");
    assert_eq!(c.name, "broken");
    assert_eq!(c.validity, Validity::InvalidSkillMd("invalid_frontmatter"));
}

// ── Dedup and ordering ──

#[test]
fn candidates_are_deduplicated_by_subpath_and_sorted_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    // Reachable via scan base, marketplace, and recursion all at once.
    fs::create_dir_all(base.join(".claude-plugin")).unwrap();
    fs::write(
        base.join(".claude-plugin/marketplace.json"),
        r#"{"plugins":[{"name":"root","source":"."}]}"#,
    )
    .unwrap();
    write_skill(base, "skills/zeta", "Zeta");
    write_skill(base, "skills/alpha", "Alpha");
    // Reachable via root-level stage and recursion.
    write_skill(base, "mid", "Mid");
    fs::write(base.join("SKILL.md"), "---\nname: Root\n---\n").unwrap();

    let list = discover_skills(base);
    assert_eq!(
        subpaths(&list),
        vec![
            "skills/alpha".to_string(),
            "mid".to_string(),
            ".".to_string(),
            "skills/zeta".to_string(),
        ]
    );
    let names: Vec<&str> = list.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["Alpha", "Mid", "Root", "Zeta"]);
}

#[test]
fn same_name_candidates_are_ordered_by_subpath() {
    let dir = tempfile::tempdir().unwrap();
    write_skill(dir.path(), "skills/b", "same");
    write_skill(dir.path(), "skills/a", "same");
    let list = discover_skills(dir.path());
    assert_eq!(
        subpaths(&list),
        vec!["skills/a".to_string(), "skills/b".to_string()]
    );
}

// ── SKILL.md reading ──

#[test]
fn skill_md_lookup_is_case_insensitive() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("skills/upper")).unwrap();
    fs::write(
        dir.path().join("skills/upper/skill.MD"),
        "---\nname: Upper\n---\n",
    )
    .unwrap();
    let list = discover_skills(dir.path());
    assert_eq!(find(&list, "skills/upper").name, "Upper");
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

    let (name, desc) = super::parse_skill_md(&p).unwrap();
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

    let (name, desc) = super::parse_skill_md(&p).unwrap();
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
    let (_, desc) = super::parse_skill_md(&p).unwrap();
    assert_eq!(desc.as_deref(), Some("one two"));
}

#[test]
fn parse_skill_md_with_reason_reports_each_failure() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("SKILL.md");
    assert_eq!(
        super::parse_skill_md_with_reason(&p).unwrap_err(),
        "read_failed"
    );
    fs::write(&p, "name: x\n").unwrap();
    assert_eq!(
        super::parse_skill_md_with_reason(&p).unwrap_err(),
        "invalid_frontmatter"
    );
    fs::write(&p, "---\nname: x\n").unwrap();
    assert_eq!(
        super::parse_skill_md_with_reason(&p).unwrap_err(),
        "invalid_frontmatter"
    );
    fs::write(&p, "---\ndescription: x\n---\n").unwrap();
    assert_eq!(
        super::parse_skill_md_with_reason(&p).unwrap_err(),
        "missing_name"
    );
}

// ── Invocation mode ──

#[test]
fn invocation_mode_maps_every_frontmatter_shape() {
    use super::{parse_invocation_mode, InvocationMode as M};

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
        super::invocation_mode_for_dir(dir.path()),
        super::InvocationMode::UserAndModel
    );
    // Case-insensitive lookup, restriction honoured.
    fs::write(
        dir.path().join("Skill.md"),
        "---\nname: x\nuser-invocable: false\n---\n",
    )
    .unwrap();
    assert_eq!(
        super::invocation_mode_for_dir(dir.path()),
        super::InvocationMode::ModelOnly
    );
}

// ── The admission rule (`require_skill_md`) ──

#[test]
fn require_skill_md_admits_a_dir_with_a_manifest_and_returns_its_path() {
    let dir = tempfile::tempdir().unwrap();
    write_skill(dir.path(), "ok", "ok-skill");

    let manifest = super::require_skill_md(&dir.path().join("ok")).expect("admitted");
    assert_eq!(manifest, dir.path().join("ok").join("SKILL.md"));

    // Case-insensitive, like the rest of discovery.
    fs::create_dir_all(dir.path().join("cased")).unwrap();
    fs::write(
        dir.path().join("cased").join("Skill.md"),
        "---\nname: c\n---\n",
    )
    .unwrap();
    super::require_skill_md(&dir.path().join("cased")).expect("admitted");
}

#[test]
fn require_skill_md_refuses_a_dir_without_one_using_the_typed_condition() {
    let dir = tempfile::tempdir().unwrap();
    // The shape issue #8 was about: a directory under a tool's skills dir
    // that was discovered but carries no manifest.
    let bare = dir.path().join(".claude/skills/bare");
    fs::create_dir_all(&bare).unwrap();
    fs::write(bare.join("notes.md"), "not a manifest").unwrap();

    let err = super::require_skill_md(&bare).expect_err("must refuse");
    match err.downcast_ref::<crate::core::errors::SignalError>() {
        Some(crate::core::errors::SignalError::SkillInvalid { reason }) => {
            assert_eq!(reason, "missing_skill_md");
        }
        other => panic!("expected SkillInvalid, got {other:?}"),
    }
}
