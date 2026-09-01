use std::fs;
use std::path::{Path, PathBuf};

use crate::core::errors::SignalError;
use crate::core::installer::InstallerPaths;
use crate::core::skill_store::{
    ProjectRecord, ProjectSkillAssignmentRecord, SkillStore, SkillTargetRecord,
};

fn make_store() -> (tempfile::TempDir, SkillStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    (dir, store)
}

/// Installer roots isolated under one temp dir: an empty home (no tool is
/// installed), a central repo, and a git cache.
fn make_paths() -> (tempfile::TempDir, InstallerPaths) {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = InstallerPaths {
        home: dir.path().join("home"),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.home).unwrap();
    (dir, paths)
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
fn installs_local_skill_and_updates_from_source() {
    let (_dir, store) = make_store();
    let (_roots, paths) = make_paths();

    let source = tempfile::tempdir().unwrap();
    fs::write(source.path().join("SKILL.md"), b"---\nname: x\n---\n").unwrap();
    fs::write(source.path().join("a.txt"), b"v1").unwrap();

    let res = super::install_local_skill(&paths, &store, source.path(), Some("local1".to_string()))
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
    let up = super::update_managed_skill_from_source(&paths, &store, &res.skill_id).unwrap();
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
        "target path should exist and contain the synced file"
    );
    assert_eq!(fs::read(target.join("a.txt")).unwrap(), b"v2");

    let err =
        match super::install_local_skill(&paths, &store, source.path(), Some("local1".to_string()))
        {
            Ok(_) => panic!("expected error"),
            Err(e) => e,
        };
    // The collision crosses core as a typed signal, never as prose.
    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::SkillExists {
            name: "local1".to_string()
        })
    );
    // A rejected install leaves no staging residue in the central repo.
    let entries: Vec<String> = fs::read_dir(&paths.central_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(entries, vec!["local1".to_string()]);
}

#[test]
fn lists_and_installs_git_skills_without_network() {
    let (_dir, store) = make_store();
    let (_roots, paths) = make_paths();

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

    let candidates =
        super::list_git_skills(&paths, &store, repo_dir.path().to_string_lossy().as_ref()).unwrap();
    let subpaths: Vec<String> = candidates.into_iter().map(|c| c.subpath).collect();
    assert!(subpaths.contains(&".".to_string()));
    assert!(subpaths.iter().any(|s| s.ends_with("skills/a")));

    let res = super::install_git_skill_from_selection(
        &paths,
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
    let (_dir, store) = make_store();
    let (_roots, paths) = make_paths();

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
        &paths,
        &store,
        repo_dir.path().to_string_lossy().as_ref(),
        None,
        None,
    ) {
        Ok(_) => panic!("expected error"),
        Err(e) => e,
    };
    assert!(matches!(
        err.downcast_ref::<SignalError>(),
        Some(SignalError::MultiSkills)
    ));
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
    let (_dir, store) = make_store();
    let (_roots, paths) = make_paths();

    let base = tempfile::tempdir().unwrap();
    fs::create_dir_all(base.path().join("skills/a")).unwrap();
    fs::create_dir_all(base.path().join("skills/b")).unwrap();
    fs::write(
        base.path().join("skills/a/SKILL.md"),
        "---\nname: Local A\n---\n",
    )
    .unwrap();

    let res =
        super::install_local_skill_from_selection(&paths, &store, base.path(), "skills/a", None)
            .unwrap();
    assert!(res.central_path.exists());
    let skill = store.get_skill_by_id(&res.skill_id).unwrap().unwrap();
    assert_eq!(skill.name, "Local A");

    let err = match super::install_local_skill_from_selection(
        &paths,
        &store,
        base.path(),
        "skills/b",
        None,
    ) {
        Ok(_) => panic!("expected error"),
        Err(e) => e,
    };
    assert!(matches!(
        err.downcast_ref::<SignalError>(),
        Some(SignalError::SkillInvalid { reason }) if reason == "missing_skill_md"
    ));
}

/// Issue #28: when a git subpath is "skills", the derived name should be replaced by the
/// SKILL.md name to avoid path duplication (e.g. `~/.claude/skills/skills/`).
#[test]
fn install_git_skill_uses_skill_md_name_over_subpath_skills() {
    let (_dir, store) = make_store();
    let (_roots, paths) = make_paths();

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
        &paths,
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
    let (_dir, store) = make_store();
    let (_roots, paths) = make_paths();

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
        &paths,
        &store,
        repo_dir.path().to_string_lossy().as_ref(),
        "awesome_agent_skills",
        None,
    ) {
        Ok(_) => panic!("expected invalid skill path"),
        Err(e) => e,
    };
    assert!(matches!(
        err.downcast_ref::<SignalError>(),
        Some(SignalError::SkillInvalid { reason }) if reason == "missing_skill_md"
    ));
}

#[test]
fn install_git_skill_selection_accepts_specific_child_under_container() {
    let (_dir, store) = make_store();
    let (_roots, paths) = make_paths();

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
        &paths,
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
    let (_dir, store) = make_store();
    let (_roots, paths) = make_paths();

    let repo_dir = tempfile::tempdir().unwrap();
    let skills_dir = repo_dir.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();
    fs::write(skills_dir.join("SKILL.md"), "---\nname: md-name\n---\n").unwrap();
    let repo = init_git_repo(repo_dir.path());
    commit_all(&repo, "add skill");

    let res = super::install_git_skill_from_selection(
        &paths,
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
    let (_dir, store) = make_store();
    let (_roots, paths) = make_paths();

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
        &paths,
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
    let (_dir, store) = make_store();
    let (_roots, paths) = make_paths();

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
        &paths,
        &store,
        repo_dir.path().to_string_lossy().as_ref(),
        None,
        None,
    ) {
        Ok(_) => panic!("expected MULTI_SKILLS error"),
        Err(e) => e,
    };
    assert!(matches!(
        err.downcast_ref::<SignalError>(),
        Some(SignalError::MultiSkills)
    ));
}

/// Issue #18: list_git_skills should discover skills in root-level subdirectories.
#[test]
fn list_git_skills_finds_root_level_skills() {
    let (_dir, store) = make_store();
    let (_roots, paths) = make_paths();

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

    let candidates =
        super::list_git_skills(&paths, &store, repo_dir.path().to_string_lossy().as_ref()).unwrap();

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
    let (_dir, store) = make_store();
    let (_roots, paths) = make_paths();

    let source = tempfile::tempdir().unwrap();
    fs::write(source.path().join("SKILL.md"), b"---\nname: plain\n---\n").unwrap();
    fs::write(source.path().join("readme.txt"), b"hello").unwrap();

    let res = super::install_local_skill(
        &paths,
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

#[test]
fn list_git_skills_discovers_deeply_nested_via_recursive_fallback() {
    let (_dir, store) = make_store();
    let (_roots, paths) = make_paths();

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

    let candidates =
        super::list_git_skills(&paths, &store, repo_dir.path().to_string_lossy().as_ref()).unwrap();

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
    let (_dir, store) = make_store();
    let (_roots, paths) = make_paths();

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

    let candidates =
        super::list_git_skills(&paths, &store, repo_dir.path().to_string_lossy().as_ref()).unwrap();
    let names: Vec<String> = candidates.iter().map(|c| c.name.clone()).collect();
    assert!(names.contains(&"Skill A".to_string()));
    assert!(names.contains(&"Skill B".to_string()));

    // The multi-skill detection used by install/update sees the same two.
    let count = super::installable_skills_in_repo(repo_dir.path()).len();
    assert_eq!(count, 2);
}

// ── Listing adapters over skill discovery ──

/// Git listing policy: anything with skill bytes is offered (a broken SKILL.md
/// still installs, named after its folder); a dir with no SKILL.md under a
/// scan base is not; a broken root is named `root-skill`.
#[test]
fn git_candidates_admit_installable_only_and_carry_no_validity() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    fs::write(base.join("SKILL.md"), "no frontmatter\n").unwrap();
    fs::create_dir_all(base.join("skills/good")).unwrap();
    fs::write(base.join("skills/good/SKILL.md"), "---\nname: Good\n---\n").unwrap();
    fs::create_dir_all(base.join("skills/broken")).unwrap();
    fs::write(
        base.join("skills/broken/SKILL.md"),
        "---\ndescription: x\n---\n",
    )
    .unwrap();
    fs::create_dir_all(base.join("skills/empty")).unwrap();

    let list = super::git_candidates_in(base, None);
    let pairs: Vec<(&str, &str)> = list
        .iter()
        .map(|c| (c.name.as_str(), c.subpath.as_str()))
        .collect();
    assert_eq!(
        pairs,
        vec![
            ("Good", "skills/good"),
            ("broken", "skills/broken"),
            ("root-skill", "."),
        ]
    );
}

#[test]
fn git_candidates_for_folder_url_are_scoped_and_repo_relative() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    fs::create_dir_all(base.join("skills/outside")).unwrap();
    fs::write(
        base.join("skills/outside/SKILL.md"),
        "---\nname: Outside\n---\n",
    )
    .unwrap();
    fs::create_dir_all(base.join("pack/skills/inside")).unwrap();
    fs::write(
        base.join("pack/skills/inside/SKILL.md"),
        "---\nname: Inside\n---\n",
    )
    .unwrap();

    // Folder that is a container: only its skills, with repo-relative subpaths.
    let list = super::git_candidates_in(base, Some("pack"));
    let pairs: Vec<(&str, &str)> = list
        .iter()
        .map(|c| (c.name.as_str(), c.subpath.as_str()))
        .collect();
    assert_eq!(pairs, vec![("Inside", "pack/skills/inside")]);

    // Folder that is itself a skill: the single candidate.
    let list = super::git_candidates_in(base, Some("pack/skills/inside"));
    let pairs: Vec<(&str, &str)> = list
        .iter()
        .map(|c| (c.name.as_str(), c.subpath.as_str()))
        .collect();
    assert_eq!(pairs, vec![("Inside", "pack/skills/inside")]);

    // Missing folder: nothing.
    assert!(super::git_candidates_in(base, Some("nope")).is_empty());
}

/// Local listing policy: every candidate is shown with validity, the root
/// included.
#[test]
fn list_local_skills_reports_root_validity() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("picked-folder");
    fs::create_dir_all(&base).unwrap();
    fs::write(base.join("SKILL.md"), "---\ndescription: x\n---\n").unwrap();

    let list = super::list_local_skills(&base).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].subpath, ".");
    assert_eq!(list[0].name, "root-skill");
    assert!(!list[0].valid);
    assert_eq!(list[0].reason.as_deref(), Some("missing_name"));
}

/// The update flow's name backfill and the fetch path's multi-skill check both
/// rely on this view: deep hits count, the repo root and non-skill dirs under
/// a scan base do not.
#[test]
fn installable_skills_in_repo_excludes_root_and_missing_skill_md() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    fs::write(base.join("SKILL.md"), "---\nname: Root\n---\n").unwrap();
    fs::create_dir_all(base.join("skills/empty")).unwrap();
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

    let candidates = super::installable_skills_in_repo(base);
    let names: Vec<&str> = candidates.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["API Design", "Tailwind"]);
    assert_eq!(candidates[0].subpath, "plugins/a/skills/api-design");
}

#[test]
fn list_git_skills_finds_root_skill_container_layout() {
    let (_dir, store) = make_store();
    let (_roots, paths) = make_paths();

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

    let candidates =
        super::list_git_skills(&paths, &store, repo_dir.path().to_string_lossy().as_ref()).unwrap();

    let candidate = candidates
        .iter()
        .find(|c| c.name == "technical-writer")
        .expect("technical-writer should be discovered");
    assert_eq!(candidate.subpath, "custom-agent-skills/technical-writer");
    assert_eq!(candidate.description.as_deref(), Some("docs"));
}

/// After `update_managed_skill_from_source`, copy-mode project assignments
/// (including Cursor) must receive updated content. Symlink-mode assignments
/// should be skipped (they auto-update via the central path).
#[test]
fn update_resyncs_project_copy_assignments() {
    let (_dir, store) = make_store();
    let (_roots, paths) = make_paths();

    // 1. Create a local skill source with a.txt = "v1"
    let source = tempfile::tempdir().unwrap();
    fs::write(
        source.path().join("SKILL.md"),
        b"---\nname: proj-test\n---\n",
    )
    .unwrap();
    fs::write(source.path().join("a.txt"), b"v1").unwrap();

    let res =
        super::install_local_skill(&paths, &store, source.path(), Some("proj-test".to_string()))
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

    // 3. Create the tool skills directory structure under the project,
    // at the same paths project sync writes to (project_relative_skills_dir).
    // Cursor: .agents/skills/proj-test/
    let cursor_target = project_root
        .path()
        .join(".agents")
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
    let up = super::update_managed_skill_from_source(&paths, &store, &res.skill_id).unwrap();

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
