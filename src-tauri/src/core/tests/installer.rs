use std::fs;
use std::path::{Path, PathBuf};

use crate::core::skill_store::{
    ProjectRecord, ProjectSkillAssignmentRecord, SkillStore, SkillTargetRecord,
};

fn make_store() -> (tempfile::TempDir, SkillStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    (dir, store)
}

fn set_central_path(store: &SkillStore, central: &Path) {
    store
        .set_setting("central_repo_path", central.to_string_lossy().as_ref())
        .unwrap();
}

fn init_git_repo(dir: &Path) -> git2::Repository {
    let repo = git2::Repository::init(dir).unwrap();
    let sig = git2::Signature::now("t", "t@example.com").unwrap();

    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    let tree_id = index.write_tree().unwrap();
    {
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();
    }
    repo
}

fn commit_all(repo: &git2::Repository, msg: &str) -> git2::Oid {
    let sig = git2::Signature::now("t", "t@example.com").unwrap();
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();

    let parent = repo
        .head()
        .ok()
        .and_then(|h| h.target())
        .and_then(|oid| repo.find_commit(oid).ok());
    match parent {
        Some(p) => repo
            .commit(Some("HEAD"), &sig, &sig, msg, &tree, &[&p])
            .unwrap(),
        None => repo
            .commit(Some("HEAD"), &sig, &sig, msg, &tree, &[])
            .unwrap(),
    }
}

#[test]
fn parses_github_urls() {
    let p = super::parse_github_url("https://github.com/owner/repo");
    assert_eq!(p.clone_url, "https://github.com/owner/repo.git");
    assert!(p.branch.is_none());
    assert!(p.subpath.is_none());

    let p = super::parse_github_url("anthropics/skills");
    assert_eq!(p.clone_url, "https://github.com/anthropics/skills.git");
    assert!(p.branch.is_none());
    assert!(p.subpath.is_none());

    let p = super::parse_github_url("github.com/owner/repo");
    assert_eq!(p.clone_url, "https://github.com/owner/repo.git");
    assert!(p.branch.is_none());
    assert!(p.subpath.is_none());

    let p = super::parse_github_url("https://github.com/owner/repo/tree/main/skills/x");
    assert_eq!(p.clone_url, "https://github.com/owner/repo.git");
    assert_eq!(p.branch.as_deref(), Some("main"));
    assert_eq!(p.subpath.as_deref(), Some("skills/x"));

    let p = super::parse_github_url("owner/repo/tree/main/skills/x");
    assert_eq!(p.clone_url, "https://github.com/owner/repo.git");
    assert_eq!(p.branch.as_deref(), Some("main"));
    assert_eq!(p.subpath.as_deref(), Some("skills/x"));

    let p = super::parse_github_url("https://github.com/owner/repo/blob/main/skills/x/SKILL.md");
    assert_eq!(p.clone_url, "https://github.com/owner/repo.git");
    assert_eq!(p.branch.as_deref(), Some("main"));
    assert_eq!(p.subpath.as_deref(), Some("skills/x"));

    let p = super::parse_github_url("https://github.com/owner/repo/blob/main/SKILL.md");
    assert_eq!(p.clone_url, "https://github.com/owner/repo.git");
    assert_eq!(p.branch.as_deref(), Some("main"));
    assert_eq!(p.subpath.as_deref(), Some("."));

    let p = super::parse_github_url("/local/path/to/repo");
    assert_eq!(p.clone_url, "/local/path/to/repo");
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
fn installs_local_skill_and_updates_from_source() {
    let app = tauri::test::mock_app();
    let (_dir, store) = make_store();

    let central_root = tempfile::tempdir().unwrap();
    set_central_path(&store, central_root.path());

    let source = tempfile::tempdir().unwrap();
    fs::write(source.path().join("SKILL.md"), b"---\nname: x\n---\n").unwrap();
    fs::write(source.path().join("a.txt"), b"v1").unwrap();

    let res = super::install_local_skill(
        app.handle(),
        &store,
        source.path(),
        Some("local1".to_string()),
    )
    .unwrap();
    assert!(res.central_path.exists());

    let skill = store.get_skill_by_id(&res.skill_id).unwrap().unwrap();
    assert_eq!(skill.name, "local1");

    // add a copy target so update will resync it
    let target_root = tempfile::tempdir().unwrap();
    let target = target_root.path().join("target");
    let t = SkillTargetRecord {
        id: "t1".to_string(),
        skill_id: res.skill_id.clone(),
        tool: "unknown_tool".to_string(),
        target_path: target.to_string_lossy().to_string(),
        mode: "copy".to_string(),
        status: "ok".to_string(),
        last_error: None,
        synced_at: None,
    };
    store.upsert_skill_target(&t).unwrap();

    fs::write(source.path().join("a.txt"), b"v2").unwrap();
    let up = super::update_managed_skill_from_source(app.handle(), &store, &res.skill_id).unwrap();
    assert_eq!(up.skill_id, res.skill_id);
    assert!(up.updated_targets.contains(&"unknown_tool".to_string()));
    assert!(PathBuf::from(
        store
            .get_skill_by_id(&res.skill_id)
            .unwrap()
            .unwrap()
            .central_path
    )
    .exists());
    assert!(
        target.join("a.txt").exists(),
        "目标路径应存在并包含同步后的文件"
    );
    assert_eq!(fs::read(target.join("a.txt")).unwrap(), b"v2");

    let err = match super::install_local_skill(
        app.handle(),
        &store,
        source.path(),
        Some("local1".to_string()),
    ) {
        Ok(_) => panic!("expected error"),
        Err(e) => e,
    };
    assert!(format!("{:#}", err).contains("skill already exists"));
}

#[test]
fn lists_and_installs_git_skills_without_network() {
    let app = tauri::test::mock_app();
    let (_dir, store) = make_store();
    let central_root = tempfile::tempdir().unwrap();
    set_central_path(&store, central_root.path());

    let repo_dir = tempfile::tempdir().unwrap();
    fs::write(repo_dir.path().join("SKILL.md"), "---\nname: Root\n---\n").unwrap();
    fs::create_dir_all(repo_dir.path().join("skills/a")).unwrap();
    fs::write(
        repo_dir.path().join("skills/a/SKILL.md"),
        "---\nname: A\n---\n",
    )
    .unwrap();
    let repo = init_git_repo(repo_dir.path());
    commit_all(&repo, "add skills");

    let candidates = super::list_git_skills(
        app.handle(),
        &store,
        repo_dir.path().to_string_lossy().as_ref(),
    )
    .unwrap();
    let subpaths: Vec<String> = candidates.into_iter().map(|c| c.subpath).collect();
    assert!(subpaths.contains(&".".to_string()));
    assert!(subpaths.iter().any(|s| s.ends_with("skills/a")));

    let res = super::install_git_skill_from_selection(
        app.handle(),
        &store,
        repo_dir.path().to_string_lossy().as_ref(),
        "skills/a",
        None,
    )
    .unwrap();
    assert!(res.central_path.exists());
}

#[test]
fn install_git_skill_errors_on_multi_skills_repo_root() {
    let app = tauri::test::mock_app();
    let (_dir, store) = make_store();
    let central_root = tempfile::tempdir().unwrap();
    set_central_path(&store, central_root.path());

    let repo_dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(repo_dir.path().join("skills/a")).unwrap();
    fs::create_dir_all(repo_dir.path().join("skills/b")).unwrap();
    fs::write(
        repo_dir.path().join("skills/a/SKILL.md"),
        "---\nname: A\n---\n",
    )
    .unwrap();
    fs::write(
        repo_dir.path().join("skills/b/SKILL.md"),
        "---\nname: B\n---\n",
    )
    .unwrap();
    let repo = init_git_repo(repo_dir.path());
    commit_all(&repo, "multi skills");

    let err = match super::install_git_skill(
        app.handle(),
        &store,
        repo_dir.path().to_string_lossy().as_ref(),
        None,
        None,
    ) {
        Ok(_) => panic!("expected error"),
        Err(e) => e,
    };
    assert!(format!("{:#}", err).contains("MULTI_SKILLS|"));
}

#[test]
fn lists_local_skills_with_invalid_entries() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    fs::create_dir_all(base.join("skills/a")).unwrap();
    fs::create_dir_all(base.join("skills/b")).unwrap();
    fs::create_dir_all(base.join("skills/c")).unwrap();
    fs::create_dir_all(base.join("skills/d")).unwrap();

    fs::write(base.join("skills/a/SKILL.md"), "---\nname: A\n---\n").unwrap();
    fs::write(base.join("skills/c/SKILL.md"), "name: C\n").unwrap();
    fs::write(base.join("skills/d/SKILL.md"), "---\ndescription: D\n---\n").unwrap();

    let list = super::list_local_skills(base).unwrap();

    let find = |subpath: &str| list.iter().find(|c| c.subpath == subpath).cloned();

    let a = find("skills/a").expect("skills/a");
    assert!(a.valid);
    assert_eq!(a.name, "A");

    let b = find("skills/b").expect("skills/b");
    assert!(!b.valid);
    assert_eq!(b.reason.as_deref(), Some("missing_skill_md"));

    let c = find("skills/c").expect("skills/c");
    assert!(!c.valid);
    assert_eq!(c.reason.as_deref(), Some("invalid_frontmatter"));

    let d = find("skills/d").expect("skills/d");
    assert!(!d.valid);
    assert_eq!(d.reason.as_deref(), Some("missing_name"));
}

#[test]
fn install_local_selection_validates_skill_md() {
    let app = tauri::test::mock_app();
    let (_dir, store) = make_store();

    let central_root = tempfile::tempdir().unwrap();
    set_central_path(&store, central_root.path());

    let base = tempfile::tempdir().unwrap();
    fs::create_dir_all(base.path().join("skills/a")).unwrap();
    fs::create_dir_all(base.path().join("skills/b")).unwrap();
    fs::write(
        base.path().join("skills/a/SKILL.md"),
        "---\nname: Local A\n---\n",
    )
    .unwrap();

    let res = super::install_local_skill_from_selection(
        app.handle(),
        &store,
        base.path(),
        "skills/a",
        None,
    )
    .unwrap();
    assert!(res.central_path.exists());
    let skill = store.get_skill_by_id(&res.skill_id).unwrap().unwrap();
    assert_eq!(skill.name, "Local A");

    let err = match super::install_local_skill_from_selection(
        app.handle(),
        &store,
        base.path(),
        "skills/b",
        None,
    ) {
        Ok(_) => panic!("expected error"),
        Err(e) => e,
    };
    assert!(format!("{:#}", err).contains("SKILL_INVALID|missing_skill_md"));
}

/// Issue #28: when a git subpath is "skills", the derived name should be replaced by the
/// SKILL.md name to avoid path duplication (e.g. `~/.claude/skills/skills/`).
#[test]
fn install_git_skill_uses_skill_md_name_over_subpath_skills() {
    let app = tauri::test::mock_app();
    let (_dir, store) = make_store();
    let central_root = tempfile::tempdir().unwrap();
    set_central_path(&store, central_root.path());

    // Build a repo with skills/<folder> where the folder is named "skills" (simulating
    // a URL like https://github.com/owner/repo/tree/main/skills).
    let repo_dir = tempfile::tempdir().unwrap();
    let skills_dir = repo_dir.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();
    fs::write(
        skills_dir.join("SKILL.md"),
        "---\nname: my-real-skill\ndescription: A real skill\n---\n",
    )
    .unwrap();
    fs::write(skills_dir.join("helper.txt"), b"data").unwrap();
    let repo = init_git_repo(repo_dir.path());
    commit_all(&repo, "add skill in skills dir");

    // install_git_skill_from_selection with subpath "skills" (no user-provided name)
    let res = super::install_git_skill_from_selection(
        app.handle(),
        &store,
        repo_dir.path().to_string_lossy().as_ref(),
        "skills",
        None,
    )
    .unwrap();

    // The name should be "my-real-skill" from SKILL.md, NOT "skills" from the subpath.
    assert_eq!(res.name, "my-real-skill");
    assert!(res.central_path.ends_with("my-real-skill"));
    assert!(res.central_path.join("SKILL.md").exists());

    let skill = store.get_skill_by_id(&res.skill_id).unwrap().unwrap();
    assert_eq!(skill.name, "my-real-skill");
    assert_eq!(skill.description.as_deref(), Some("A real skill"));
}

#[test]
fn install_git_skill_rejects_container_subpath_without_skill_md() {
    let app = tauri::test::mock_app();
    let (_dir, store) = make_store();
    let central_root = tempfile::tempdir().unwrap();
    set_central_path(&store, central_root.path());

    let repo_dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(
        repo_dir
            .path()
            .join("awesome_agent_skills/technical-writer"),
    )
    .unwrap();
    fs::write(
        repo_dir
            .path()
            .join("awesome_agent_skills/technical-writer/SKILL.md"),
        "---\nname: technical-writer\n---\n",
    )
    .unwrap();
    let repo = init_git_repo(repo_dir.path());
    commit_all(&repo, "add container skill");

    let err = match super::install_git_skill_from_selection(
        app.handle(),
        &store,
        repo_dir.path().to_string_lossy().as_ref(),
        "awesome_agent_skills",
        None,
    ) {
        Ok(_) => panic!("expected invalid skill path"),
        Err(e) => e,
    };
    assert!(format!("{:#}", err).contains("SKILL_INVALID|missing_skill_md"));
}

#[test]
fn install_git_skill_selection_accepts_specific_child_under_container() {
    let app = tauri::test::mock_app();
    let (_dir, store) = make_store();
    let central_root = tempfile::tempdir().unwrap();
    set_central_path(&store, central_root.path());

    let repo_dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(
        repo_dir
            .path()
            .join("awesome_agent_skills/technical-writer"),
    )
    .unwrap();
    fs::write(
        repo_dir
            .path()
            .join("awesome_agent_skills/technical-writer/SKILL.md"),
        "---\nname: technical-writer\ndescription: docs\n---\n",
    )
    .unwrap();
    let repo = init_git_repo(repo_dir.path());
    commit_all(&repo, "add container skill");

    let res = super::install_git_skill_from_selection(
        app.handle(),
        &store,
        repo_dir.path().to_string_lossy().as_ref(),
        "awesome_agent_skills/technical-writer",
        None,
    )
    .unwrap();

    assert_eq!(res.name, "technical-writer");
    assert!(res.central_path.join("SKILL.md").exists());
}

/// Issue #28: when user explicitly provides a name, SKILL.md should NOT override it.
#[test]
fn install_git_skill_respects_user_provided_name() {
    let app = tauri::test::mock_app();
    let (_dir, store) = make_store();
    let central_root = tempfile::tempdir().unwrap();
    set_central_path(&store, central_root.path());

    let repo_dir = tempfile::tempdir().unwrap();
    let skills_dir = repo_dir.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();
    fs::write(skills_dir.join("SKILL.md"), "---\nname: md-name\n---\n").unwrap();
    let repo = init_git_repo(repo_dir.path());
    commit_all(&repo, "add skill");

    let res = super::install_git_skill_from_selection(
        app.handle(),
        &store,
        repo_dir.path().to_string_lossy().as_ref(),
        "skills",
        Some("user-custom-name".to_string()),
    )
    .unwrap();

    // User-provided name takes priority.
    assert_eq!(res.name, "user-custom-name");
}

/// Issue #28: install_git_skill (non-selection variant) also uses SKILL.md name.
#[test]
fn install_git_skill_derives_name_from_skill_md() {
    let app = tauri::test::mock_app();
    let (_dir, store) = make_store();
    let central_root = tempfile::tempdir().unwrap();
    set_central_path(&store, central_root.path());

    let repo_dir = tempfile::tempdir().unwrap();
    fs::write(
        repo_dir.path().join("SKILL.md"),
        "---\nname: proper-name\ndescription: desc\n---\n",
    )
    .unwrap();
    let repo = init_git_repo(repo_dir.path());
    commit_all(&repo, "init");

    // The repo name (derived from path) will be something like a temp dir name.
    // After install, the name should be "proper-name" from SKILL.md.
    let res = super::install_git_skill(
        app.handle(),
        &store,
        repo_dir.path().to_string_lossy().as_ref(),
        None,
        None,
    )
    .unwrap();

    assert_eq!(res.name, "proper-name");
    assert!(res.central_path.ends_with("proper-name"));
}

/// Issue #18: repos with skills in root-level subdirectories (no `skills/` parent)
/// should be detected as multi-skill repos.
#[test]
fn install_git_skill_detects_root_level_multi_skills() {
    let app = tauri::test::mock_app();
    let (_dir, store) = make_store();
    let central_root = tempfile::tempdir().unwrap();
    set_central_path(&store, central_root.path());

    // Build a repo with skills directly in root subdirectories (no skills/ parent)
    let repo_dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(repo_dir.path().join("skill-a")).unwrap();
    fs::create_dir_all(repo_dir.path().join("skill-b")).unwrap();
    fs::write(
        repo_dir.path().join("skill-a/SKILL.md"),
        "---\nname: Skill A\n---\n",
    )
    .unwrap();
    fs::write(
        repo_dir.path().join("skill-b/SKILL.md"),
        "---\nname: Skill B\n---\n",
    )
    .unwrap();
    let repo = init_git_repo(repo_dir.path());
    commit_all(&repo, "add root-level skills");

    // install_git_skill should detect multiple skills and bail with MULTI_SKILLS
    let err = match super::install_git_skill(
        app.handle(),
        &store,
        repo_dir.path().to_string_lossy().as_ref(),
        None,
        None,
    ) {
        Ok(_) => panic!("expected MULTI_SKILLS error"),
        Err(e) => e,
    };
    assert!(format!("{:#}", err).contains("MULTI_SKILLS|"));
}

/// Issue #18: list_git_skills should discover skills in root-level subdirectories.
#[test]
fn list_git_skills_finds_root_level_skills() {
    let app = tauri::test::mock_app();
    let (_dir, store) = make_store();
    let central_root = tempfile::tempdir().unwrap();
    set_central_path(&store, central_root.path());

    let repo_dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(repo_dir.path().join("my-skill-1")).unwrap();
    fs::create_dir_all(repo_dir.path().join("my-skill-2")).unwrap();
    fs::create_dir_all(repo_dir.path().join("not-a-skill")).unwrap();
    fs::write(
        repo_dir.path().join("my-skill-1/SKILL.md"),
        "---\nname: First\n---\n",
    )
    .unwrap();
    fs::write(
        repo_dir.path().join("my-skill-2/SKILL.md"),
        "---\nname: Second\n---\n",
    )
    .unwrap();
    // not-a-skill has no SKILL.md — should NOT be discovered
    let repo = init_git_repo(repo_dir.path());
    commit_all(&repo, "add root-level skills");

    let candidates = super::list_git_skills(
        app.handle(),
        &store,
        repo_dir.path().to_string_lossy().as_ref(),
    )
    .unwrap();

    let names: Vec<String> = candidates.iter().map(|c| c.name.clone()).collect();
    assert!(names.contains(&"First".to_string()), "should find First");
    assert!(names.contains(&"Second".to_string()), "should find Second");
    // "not-a-skill" should NOT appear
    assert!(
        !candidates.iter().any(|c| c.subpath.contains("not-a-skill")),
        "should not find not-a-skill"
    );
}

/// Non-symlink local skills retain source_type "local" (enrichment is skipped).
#[test]
fn install_local_skill_non_symlink_stays_local() {
    let app = tauri::test::mock_app();
    let (_dir, store) = make_store();
    let central_root = tempfile::tempdir().unwrap();
    set_central_path(&store, central_root.path());

    let source = tempfile::tempdir().unwrap();
    fs::write(source.path().join("SKILL.md"), b"---\nname: plain\n---\n").unwrap();
    fs::write(source.path().join("readme.txt"), b"hello").unwrap();

    let res = super::install_local_skill(
        app.handle(),
        &store,
        source.path(),
        Some("plain-skill".to_string()),
    )
    .unwrap();

    let skill = store.get_skill_by_id(&res.skill_id).unwrap().unwrap();
    assert_eq!(skill.source_type, "local", "non-symlink should stay local");
    assert!(
        skill.source_ref.is_some(),
        "source_ref should be the filesystem path"
    );
    assert!(
        skill.source_subpath.is_none(),
        "source_subpath should be None for local"
    );
}

// ── Deep discovery tests ──

#[test]
fn find_skill_dirs_recursive_finds_deeply_nested_skills() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();

    // Create wshobson/agents-like structure: plugins/<plugin>/skills/<skill>/SKILL.md (depth 4)
    let depths = [
        "plugins/backend/skills/api-design",
        "plugins/frontend/skills/tailwind-design",
        "plugins/accessibility/skills/wcag-audit",
    ];
    for d in &depths {
        fs::create_dir_all(base.join(d)).unwrap();
        fs::write(
            base.join(d).join("SKILL.md"),
            format!("---\nname: {}\n---\n", d.rsplit('/').next().unwrap()),
        )
        .unwrap();
    }

    let found = super::find_skill_dirs_recursive(base, 0, 5);
    assert_eq!(found.len(), 3, "should find all 3 deeply nested skills");
    for d in &depths {
        assert!(
            found.iter().any(|p: &PathBuf| p.ends_with(d)),
            "should find {}",
            d
        );
    }
}

#[test]
fn find_skill_dirs_recursive_respects_max_depth() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();

    // Skill at depth 5 (within limit)
    let d5 = "a/b/c/d/e";
    fs::create_dir_all(base.join(d5)).unwrap();
    fs::write(base.join(d5).join("SKILL.md"), "---\nname: deep5\n---\n").unwrap();

    // Skill at depth 6 (beyond limit)
    let d6 = "a/b/c/d/e/f";
    fs::create_dir_all(base.join(d6)).unwrap();
    fs::write(base.join(d6).join("SKILL.md"), "---\nname: deep6\n---\n").unwrap();

    let found = super::find_skill_dirs_recursive(base, 0, 5);
    assert!(
        found.iter().any(|p: &PathBuf| p.ends_with(d5)),
        "should find skill at depth 5"
    );
    assert!(
        !found.iter().any(|p: &PathBuf| p.ends_with(d6)),
        "should NOT find skill at depth 6"
    );
}

#[test]
fn find_skill_dirs_recursive_skips_excluded_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();

    // Valid skill
    fs::create_dir_all(base.join("valid/my-skill")).unwrap();
    fs::write(
        base.join("valid/my-skill/SKILL.md"),
        "---\nname: valid\n---\n",
    )
    .unwrap();

    // Skills inside dirs that should be skipped
    let skip_dirs = [
        "node_modules",
        ".git",
        "dist",
        "build",
        "target",
        ".next",
        ".cache",
    ];
    for skip in &skip_dirs {
        let skip_path = base.join(skip).join("hidden-skill");
        fs::create_dir_all(&skip_path).unwrap();
        fs::write(skip_path.join("SKILL.md"), "---\nname: hidden\n---\n").unwrap();
    }

    let found = super::find_skill_dirs_recursive(base, 0, 5);
    assert_eq!(
        found.len(),
        1,
        "should only find the valid skill, not those in excluded dirs"
    );
    assert!(found[0].ends_with("valid/my-skill"));
}

#[test]
fn parse_marketplace_json_extracts_plugin_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();

    // Create .claude-plugin/marketplace.json
    fs::create_dir_all(base.join(".claude-plugin")).unwrap();
    let manifest = serde_json::json!({
        "plugins": [
            {"name": "api-scaffolding", "source": "./plugins/api-scaffolding"},
            {"name": "tailwind-design", "source": "./plugins/tailwind-design"},
            {"name": "no-source"}
        ]
    });
    fs::write(
        base.join(".claude-plugin/marketplace.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    // Create the plugin dirs on disk
    fs::create_dir_all(base.join("plugins/api-scaffolding")).unwrap();
    fs::create_dir_all(base.join("plugins/tailwind-design")).unwrap();

    let dirs = super::parse_marketplace_json(base);
    assert_eq!(dirs.len(), 2);
    assert!(dirs
        .iter()
        .any(|p: &PathBuf| p.ends_with("plugins/api-scaffolding")));
    assert!(dirs
        .iter()
        .any(|p: &PathBuf| p.ends_with("plugins/tailwind-design")));
}

#[test]
fn parse_marketplace_json_returns_empty_for_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let dirs = super::parse_marketplace_json(dir.path());
    assert!(dirs.is_empty());
}

#[test]
fn parse_marketplace_json_returns_empty_for_malformed_json() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join(".claude-plugin")).unwrap();
    fs::write(
        dir.path().join(".claude-plugin/marketplace.json"),
        "not json {{{",
    )
    .unwrap();
    let dirs = super::parse_marketplace_json(dir.path());
    assert!(dirs.is_empty());
}

#[test]
fn list_git_skills_discovers_deeply_nested_via_recursive_fallback() {
    let app = tauri::test::mock_app();
    let (_dir, store) = make_store();
    let central_root = tempfile::tempdir().unwrap();
    set_central_path(&store, central_root.path());

    // Build wshobson/agents-like repo with NO standard skill dirs
    let repo_dir = tempfile::tempdir().unwrap();
    let skills = [
        "plugins/backend/skills/api-design",
        "plugins/frontend/skills/tailwind",
    ];
    for s in &skills {
        fs::create_dir_all(repo_dir.path().join(s)).unwrap();
        fs::write(
            repo_dir.path().join(s).join("SKILL.md"),
            format!("---\nname: {}\n---\n", s.rsplit('/').next().unwrap()),
        )
        .unwrap();
    }
    let repo = init_git_repo(repo_dir.path());
    commit_all(&repo, "add nested skills");

    let candidates = super::list_git_skills(
        app.handle(),
        &store,
        repo_dir.path().to_string_lossy().as_ref(),
    )
    .unwrap();

    assert!(
        candidates.len() >= 2,
        "should find at least 2 deeply nested skills, found {}",
        candidates.len()
    );
    let names: Vec<String> = candidates.iter().map(|c| c.name.clone()).collect();
    assert!(names.contains(&"api-design".to_string()));
    assert!(names.contains(&"tailwind".to_string()));
}

#[test]
fn count_skills_in_repo_counts_deeply_nested() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();

    // No skills in standard locations, but 3 deeply nested
    let skills = [
        "plugins/a/skills/s1",
        "plugins/b/skills/s2",
        "plugins/c/skills/s3",
    ];
    for s in &skills {
        fs::create_dir_all(base.join(s)).unwrap();
        fs::write(base.join(s).join("SKILL.md"), "---\nname: x\n---\n").unwrap();
    }

    let count = super::count_skills_in_repo(base);
    assert_eq!(count, 3, "should count all 3 deeply nested skills");
}

#[test]
fn scan_skill_candidates_in_dir_finds_deeply_nested() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();

    let skills = [
        ("plugins/a/skills/api-design", "API Design"),
        ("plugins/b/skills/tailwind", "Tailwind"),
    ];
    for (path, name) in &skills {
        fs::create_dir_all(base.join(path)).unwrap();
        fs::write(
            base.join(path).join("SKILL.md"),
            format!("---\nname: {}\n---\n", name),
        )
        .unwrap();
    }

    let candidates = super::scan_skill_candidates_in_dir(base);
    assert_eq!(candidates.len(), 2, "should find 2 deep candidates");
    let names: Vec<&str> = candidates.iter().map(|c| c.0.as_str()).collect();
    assert!(names.contains(&"API Design"));
    assert!(names.contains(&"Tailwind"));
}

#[test]
fn list_local_skills_discovers_deeply_nested() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();

    let skills = [
        "plugins/backend/skills/api-design",
        "plugins/frontend/skills/tailwind",
    ];
    for s in &skills {
        fs::create_dir_all(base.join(s)).unwrap();
        fs::write(
            base.join(s).join("SKILL.md"),
            format!("---\nname: {}\n---\n", s.rsplit('/').next().unwrap()),
        )
        .unwrap();
    }

    let list = super::list_local_skills(base).unwrap();
    assert!(
        list.len() >= 2,
        "should find at least 2 deeply nested skills, found {}",
        list.len()
    );
    let names: Vec<String> = list.iter().map(|c| c.name.clone()).collect();
    assert!(names.contains(&"api-design".to_string()));
    assert!(names.contains(&"tailwind".to_string()));
}

#[test]
fn existing_shallow_repos_still_work() {
    // Verify that repos with standard skill dirs continue working unchanged
    let app = tauri::test::mock_app();
    let (_dir, store) = make_store();
    let central_root = tempfile::tempdir().unwrap();
    set_central_path(&store, central_root.path());

    let repo_dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(repo_dir.path().join("skills/a")).unwrap();
    fs::create_dir_all(repo_dir.path().join("skills/b")).unwrap();
    fs::write(
        repo_dir.path().join("skills/a/SKILL.md"),
        "---\nname: Skill A\n---\n",
    )
    .unwrap();
    fs::write(
        repo_dir.path().join("skills/b/SKILL.md"),
        "---\nname: Skill B\n---\n",
    )
    .unwrap();
    let repo = init_git_repo(repo_dir.path());
    commit_all(&repo, "add standard skills");

    let candidates = super::list_git_skills(
        app.handle(),
        &store,
        repo_dir.path().to_string_lossy().as_ref(),
    )
    .unwrap();
    let names: Vec<String> = candidates.iter().map(|c| c.name.clone()).collect();
    assert!(names.contains(&"Skill A".to_string()));
    assert!(names.contains(&"Skill B".to_string()));

    // count_skills_in_repo should still work
    let count = super::count_skills_in_repo(repo_dir.path());
    assert_eq!(count, 2);
}

#[test]
fn list_git_skills_finds_root_skill_container_layout() {
    let app = tauri::test::mock_app();
    let (_dir, store) = make_store();
    let central_root = tempfile::tempdir().unwrap();
    set_central_path(&store, central_root.path());

    let repo_dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(repo_dir.path().join("custom-agent-skills/technical-writer")).unwrap();
    fs::write(
        repo_dir
            .path()
            .join("custom-agent-skills/technical-writer/SKILL.md"),
        "---\nname: technical-writer\ndescription: docs\n---\n",
    )
    .unwrap();
    let repo = init_git_repo(repo_dir.path());
    commit_all(&repo, "add container skill");

    let candidates = super::list_git_skills(
        app.handle(),
        &store,
        repo_dir.path().to_string_lossy().as_ref(),
    )
    .unwrap();

    let candidate = candidates
        .iter()
        .find(|c| c.name == "technical-writer")
        .expect("technical-writer should be discovered");
    assert_eq!(candidate.subpath, "custom-agent-skills/technical-writer");
    assert_eq!(candidate.description.as_deref(), Some("docs"));
}

#[test]
fn collect_skill_dirs_finds_skills_under_explicit_container() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("technical-writer")).unwrap();
    fs::create_dir_all(dir.path().join("not-a-skill")).unwrap();
    fs::write(
        dir.path().join("technical-writer/SKILL.md"),
        "---\nname: technical-writer\n---\n",
    )
    .unwrap();

    let dirs = super::collect_skill_dirs(dir.path());
    let rels: Vec<String> = dirs
        .iter()
        .map(|p| {
            p.strip_prefix(dir.path())
                .unwrap_or(p)
                .to_string_lossy()
                .to_string()
        })
        .collect();
    assert_eq!(rels, vec!["technical-writer".to_string()]);
}

#[test]
fn collect_skill_dirs_finds_multiple_skills_under_explicit_container() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("technical-writer")).unwrap();
    fs::create_dir_all(dir.path().join("python-expert")).unwrap();
    fs::create_dir_all(dir.path().join("not-a-skill")).unwrap();
    fs::write(
        dir.path().join("technical-writer/SKILL.md"),
        "---\nname: technical-writer\n---\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("python-expert/SKILL.md"),
        "---\nname: python-expert\n---\n",
    )
    .unwrap();

    let dirs = super::collect_skill_dirs(dir.path());
    let rels: Vec<String> = dirs
        .iter()
        .map(|p| {
            p.strip_prefix(dir.path())
                .unwrap_or(p)
                .to_string_lossy()
                .to_string()
        })
        .collect();
    assert_eq!(
        rels,
        vec!["python-expert".to_string(), "technical-writer".to_string()]
    );
}

#[test]
fn collect_skill_dirs_scans_named_skill_containers_but_not_generic_dirs() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("agent-pack/hidden-skill")).unwrap();
    fs::create_dir_all(dir.path().join("agent-skills/visible-skill")).unwrap();
    fs::write(
        dir.path().join("agent-pack/hidden-skill/SKILL.md"),
        "---\nname: hidden\n---\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("agent-skills/visible-skill/SKILL.md"),
        "---\nname: visible\n---\n",
    )
    .unwrap();

    let dirs = super::collect_skill_dirs(dir.path());
    let rels: Vec<String> = dirs
        .iter()
        .map(|p| {
            p.strip_prefix(dir.path())
                .unwrap_or(p)
                .to_string_lossy()
                .to_string()
        })
        .collect();
    assert_eq!(
        rels,
        vec![
            "agent-pack/hidden-skill".to_string(),
            "agent-skills/visible-skill".to_string()
        ]
    );
}

#[test]
fn collect_skill_dirs_deduplicates_known_root_containers() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("skills/technical-writer")).unwrap();
    fs::write(
        dir.path().join("skills/technical-writer/SKILL.md"),
        "---\nname: technical-writer\n---\n",
    )
    .unwrap();

    let dirs = super::collect_skill_dirs(dir.path());
    assert_eq!(dirs.len(), 1);
    assert!(dirs[0].ends_with("skills/technical-writer"));
}

/// After `update_managed_skill_from_source`, copy-mode project assignments
/// (including Cursor) must receive updated content. Symlink-mode assignments
/// should be skipped (they auto-update via the central path).
#[test]
fn update_resyncs_project_copy_assignments() {
    let app = tauri::test::mock_app();
    let (_dir, store) = make_store();

    let central_root = tempfile::tempdir().unwrap();
    set_central_path(&store, central_root.path());

    // 1. Create a local skill source with a.txt = "v1"
    let source = tempfile::tempdir().unwrap();
    fs::write(
        source.path().join("SKILL.md"),
        b"---\nname: proj-test\n---\n",
    )
    .unwrap();
    fs::write(source.path().join("a.txt"), b"v1").unwrap();

    let res = super::install_local_skill(
        app.handle(),
        &store,
        source.path(),
        Some("proj-test".to_string()),
    )
    .unwrap();

    // 2. Register a project (using a tempdir as the project root)
    let project_root = tempfile::tempdir().unwrap();
    let now = 1000i64;
    let project = ProjectRecord {
        id: "p1".to_string(),
        path: project_root.path().to_string_lossy().to_string(),
        created_at: now,
        updated_at: now,
    };
    store.register_project(&project).unwrap();

    // 3. Create the tool skills directory structure under the project
    // Cursor: .cursor/skills/proj-test/
    let cursor_target = project_root
        .path()
        .join(".cursor")
        .join("skills")
        .join("proj-test");
    fs::create_dir_all(&cursor_target).unwrap();
    fs::write(cursor_target.join("a.txt"), b"v1").unwrap();

    // Claude Code: .claude/skills/proj-test/
    let claude_target = project_root
        .path()
        .join(".claude")
        .join("skills")
        .join("proj-test");
    fs::create_dir_all(&claude_target).unwrap();
    fs::write(claude_target.join("a.txt"), b"v1").unwrap();

    // 4. Insert a copy-mode assignment for cursor
    let copy_assignment = ProjectSkillAssignmentRecord {
        id: "pa-copy".to_string(),
        project_id: "p1".to_string(),
        skill_id: res.skill_id.clone(),
        skill_name: "proj-test".to_string(),
        tool: "cursor".to_string(),
        mode: "copy".to_string(),
        status: "synced".to_string(),
        last_error: None,
        synced_at: Some(now),
        content_hash: None,
        created_at: now,
    };
    store
        .add_project_skill_assignment(&copy_assignment)
        .unwrap();

    // 5. Insert a symlink-mode assignment for claude_code
    let symlink_assignment = ProjectSkillAssignmentRecord {
        id: "pa-sym".to_string(),
        project_id: "p1".to_string(),
        skill_id: res.skill_id.clone(),
        skill_name: "proj-test".to_string(),
        tool: "claude_code".to_string(),
        mode: "symlink".to_string(),
        status: "synced".to_string(),
        last_error: None,
        synced_at: Some(now),
        content_hash: None,
        created_at: now,
    };
    store
        .add_project_skill_assignment(&symlink_assignment)
        .unwrap();

    // 6. Modify source to "v2" and update the skill
    fs::write(source.path().join("a.txt"), b"v2").unwrap();
    let up = super::update_managed_skill_from_source(app.handle(), &store, &res.skill_id).unwrap();

    // 7. Assert: copy-mode (cursor) project target has updated content
    assert_eq!(
        fs::read(cursor_target.join("a.txt")).unwrap(),
        b"v2",
        "copy-mode project target should have updated content"
    );

    // 8. Assert: updated_targets includes a project: prefixed entry for cursor
    assert!(
        up.updated_targets
            .iter()
            .any(|t| t.starts_with("project:") && t.contains("cursor")),
        "updated_targets should include project:p1:cursor, got: {:?}",
        up.updated_targets
    );

    // 9. Assert: symlink assignment is NOT in updated_targets
    assert!(
        !up.updated_targets
            .iter()
            .any(|t| t.starts_with("project:") && t.contains("claude_code")),
        "symlink assignment should not be in updated_targets, got: {:?}",
        up.updated_targets
    );

    // 10. Assert: DB assignment record has updated content_hash and status "synced"
    let assignments = store
        .list_project_skill_assignments_by_skill(&res.skill_id)
        .unwrap();
    let copy_rec = assignments.iter().find(|a| a.id == "pa-copy").unwrap();
    assert_eq!(copy_rec.status, "synced");
    assert!(
        copy_rec.content_hash.is_some(),
        "content_hash should be set after re-sync"
    );
    assert!(
        copy_rec.synced_at.unwrap() > now,
        "synced_at should be updated"
    );
}
