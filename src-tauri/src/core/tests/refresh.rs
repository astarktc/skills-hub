//! Tests for `core::refresh` — the Refresh (all) batch: acquire every
//! selected skill, then finalize + propagate each under the mutation guard.
//!
//! Every case runs against a temp home / central dir / DB; installedness is
//! faked by creating a Tool's detect dir under the temp home.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

use crate::core::cancel_token::CancelToken;
use crate::core::errors::SignalError;
use crate::core::installer::{install_imported_skill, install_local_skill, InstallerPaths};
use crate::core::propagation::{PropagationOutcome, PropagationScope, PropagationStatus};
use crate::core::refresh::{
    merge_reassert, refresh_managed_skills, refresh_managed_skills_with, RefreshPhase,
    RefreshPolicy, RefreshSelection, SkillRefreshStatus,
};
use crate::core::skill_store::SkillStore;
use crate::core::tool_adapters::adapter_by_key;
use crate::core::unlocatable::UnlocatableState;

struct RepointApi;

impl crate::core::git_acquisition::GithubApi for RepointApi {
    fn matching_refs(
        &self,
        _: &crate::core::git_acquisition::GithubRepo,
        _: &str,
    ) -> anyhow::Result<Vec<String>> {
        Ok(vec!["main".into(), "feature/x".into()])
    }

    fn branch_sha(&self, _: &crate::core::git_acquisition::GithubCoords) -> anyhow::Result<String> {
        Ok("new-sha".into())
    }

    fn download_directory(
        &self,
        coords: &crate::core::git_acquisition::GithubCoords,
        dest: &Path,
        _: Option<&CancelToken>,
    ) -> anyhow::Result<()> {
        fs::create_dir_all(dest)?;
        fs::write(
            dest.join("SKILL.md"),
            format!("---\nname: alpha\n---\nnew bytes\n{}", coords.tree_url()),
        )?;
        Ok(())
    }
}

#[test]
fn git_repoint_does_not_use_old_subpath_suffix_for_new_url() {
    use crate::core::git_acquisition::{GithubApi, GithubCoords, GithubRepo};

    #[derive(Default)]
    struct RecordingApi(Mutex<Vec<(String, String)>>);
    impl GithubApi for RecordingApi {
        fn matching_refs(&self, repo: &GithubRepo, prefix: &str) -> anyhow::Result<Vec<String>> {
            RepointApi.matching_refs(repo, prefix)
        }
        fn branch_sha(&self, coords: &GithubCoords) -> anyhow::Result<String> {
            self.0
                .lock()
                .unwrap()
                .push((coords.branch.clone(), coords.subpath.clone()));
            RepointApi.branch_sha(coords)
        }
        fn download_directory(
            &self,
            coords: &GithubCoords,
            dest: &Path,
            cancel: Option<&CancelToken>,
        ) -> anyhow::Result<()> {
            self.0
                .lock()
                .unwrap()
                .push((coords.branch.clone(), coords.subpath.clone()));
            RepointApi.download_directory(coords, dest, cancel)
        }
    }

    let f = git_repoint_fixture();
    let api = RecordingApi::default();
    let report = super::repoint_git_skill_with(
        &f.paths,
        &f.store,
        &f.skill_id,
        "https://github.com/owner/repo/tree/main/skills/old",
        RefreshPolicy::default(),
        None,
        3000,
        &api,
    )
    .unwrap();
    assert!(matches!(
        report.skills[0].status,
        SkillRefreshStatus::Refreshed { .. }
    ));
    assert_eq!(
        *api.0.lock().unwrap(),
        vec![
            ("main".into(), "skills/old".into()),
            ("main".into(), "skills/old".into()),
        ]
    );
    let record = f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap();
    assert_eq!(record.source_subpath.as_deref(), Some("skills/old"));
}

#[test]
fn git_repoint_slash_branch_persists_resolved_path_and_original_url() {
    let f = git_repoint_fixture();
    let url = "https://github.com/owner/repo/tree/feature/x/skills/alpha";
    let report = super::repoint_git_skill_with(
        &f.paths,
        &f.store,
        &f.skill_id,
        url,
        RefreshPolicy::default(),
        None,
        3000,
        &RepointApi,
    )
    .unwrap();
    assert!(matches!(
        report.skills[0].status,
        SkillRefreshStatus::Refreshed { .. }
    ));
    let record = f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap();
    assert_eq!(record.source_ref.as_deref(), Some(url));
    assert_eq!(record.source_subpath.as_deref(), Some("skills/alpha"));
    assert!(
        fs::read_to_string(Path::new(&record.central_path).join("SKILL.md"))
            .unwrap()
            .contains(url)
    );
}

#[test]
fn git_repoint_acquires_before_rewriting_and_rebuilds_central() {
    let f = fixture();
    let tool = adapter_by_key("claude_code").unwrap();
    fs::create_dir_all(f.paths.home.join(tool.relative_detect_dir)).unwrap();
    refresh(
        &f,
        RefreshPolicy {
            reassert_auto_sync: true,
        },
    );
    let mut original = f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap();
    original.source_type = "git".into();
    original.source_ref = Some("https://github.com/owner/repo/tree/main/old".into());
    original.source_subpath = Some("old".into());
    f.store.upsert_skill(&original).unwrap();
    fs::remove_dir_all(&original.central_path).unwrap();
    let url = "https://github.com/owner/repo/tree/main/new";
    let report = super::repoint_git_skill_with(
        &f.paths,
        &f.store,
        &f.skill_id,
        url,
        RefreshPolicy::default(),
        None,
        3000,
        &RepointApi,
    )
    .unwrap();
    assert!(matches!(
        report.skills[0].status,
        SkillRefreshStatus::Refreshed { .. }
    ));
    let after = f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap();
    assert_eq!(after.id, original.id);
    assert_eq!(after.source_ref.as_deref(), Some(url));
    assert_eq!(after.source_subpath.as_deref(), Some("new"));
    assert_eq!(after.source_revision.as_deref(), Some("new-sha"));
    let SkillRefreshStatus::Refreshed { targets, .. } = &report.skills[0].status else {
        unreachable!()
    };
    assert_eq!(targets.len(), 1);
    assert!(
        matches!(&targets[0].scope, PropagationScope::Global { tool } if tool == "claude_code")
    );
    assert!(f
        .paths
        .home
        .join(tool.relative_skills_dir)
        .join("alpha/SKILL.md")
        .is_file());
    assert!(
        fs::read_to_string(Path::new(&after.central_path).join("SKILL.md"))
            .unwrap()
            .ends_with(url)
    );
}

#[test]
fn git_repoint_accepts_add_flow_github_skill_links() {
    for url in [
        "https://github.com/owner/repo/blob/main/skills/foo/SKILL.md",
        "http://github.com/owner/repo/tree/main/skills/foo",
        "github.com/owner/repo/tree/main/skills/foo",
        "https://github.com/owner/repo.git/tree/main/skills/foo/",
    ] {
        let f = git_repoint_fixture();
        let report = super::repoint_git_skill_with(
            &f.paths,
            &f.store,
            &f.skill_id,
            url,
            RefreshPolicy::default(),
            None,
            3000,
            &RepointApi,
        )
        .unwrap_or_else(|err| panic!("{url}: {err:#}"));
        assert!(matches!(
            report.skills[0].status,
            SkillRefreshStatus::Refreshed { .. }
        ));
        let after = f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap();
        assert_eq!(after.source_ref.as_deref(), Some(url));
        assert_eq!(after.source_subpath.as_deref(), Some("skills/foo"));
        assert!(
            fs::read_to_string(Path::new(&after.central_path).join("SKILL.md"))
                .unwrap()
                .ends_with("https://github.com/owner/repo/tree/main/skills/foo")
        );
    }
}

#[test]
fn git_repoint_refuses_non_git_without_changing_the_record() {
    let f = fixture();
    let before = format!("{:?}", f.store.get_skill_by_id(&f.skill_id).unwrap());
    let error = super::repoint_git_skill_with(
        &f.paths,
        &f.store,
        &f.skill_id,
        "https://github.com/owner/repo/tree/main/new",
        RefreshPolicy::default(),
        None,
        3000,
        &RepointApi,
    )
    .unwrap_err();
    assert!(error.downcast_ref::<SignalError>().is_some());
    assert_eq!(
        format!("{:?}", f.store.get_skill_by_id(&f.skill_id).unwrap()),
        before
    );
}

#[test]
fn git_repoint_rejects_malformed_url_before_acquisition() {
    let f = fixture();
    let mut record = f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap();
    record.source_type = "git".into();
    f.store.upsert_skill(&record).unwrap();
    for url in [
        "owner/repo",
        "owner/repo/tree/main/new",
        "https://example.com/owner/repo",
        "https://github.com/owner/repo/tree",
        "https://github.com/owner/repo/issues",
        "https://github.com/owner/repo/tree/main/../secret",
        "https://github.com/owner/repo/tree/main//skill",
        "https://github.com/owner/repo/tree/main/skill?raw=1",
        "https://github.com/owner/repo/tree/main/skill#heading",
        "https://github.com/owner/repo/tree/main/%2e%2e",
        "https://github.com/owner/repo/tree/main/a b",
        "https://github.com/owner/.git",
    ] {
        let result = super::repoint_git_skill_with(
            &f.paths,
            &f.store,
            &f.skill_id,
            url,
            RefreshPolicy::default(),
            None,
            3000,
            &RepointApi,
        );
        assert!(
            matches!(
                result.unwrap_err().downcast_ref::<SignalError>(),
                Some(SignalError::InvalidGithubUrl { .. })
            ),
            "{url}"
        );
    }
}

fn git_repoint_fixture() -> Fixture {
    let f = fixture();
    let mut record = f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap();
    record.source_type = "git".into();
    record.source_ref = Some("https://github.com/owner/repo/tree/main/old".into());
    record.source_subpath = Some("old".into());
    f.store.upsert_skill(&record).unwrap();
    f
}

#[test]
fn git_repoint_honours_auto_sync_reassert_policy() {
    for reassert_auto_sync in [false, true] {
        let f = git_repoint_fixture();
        let tool = adapter_by_key("claude_code").unwrap();
        fs::create_dir_all(f.paths.home.join(tool.relative_detect_dir)).unwrap();
        assert!(f.store.list_skill_targets(&f.skill_id).unwrap().is_empty());
        let report = super::repoint_git_skill_with(
            &f.paths,
            &f.store,
            &f.skill_id,
            "https://github.com/owner/repo/tree/main/new",
            RefreshPolicy { reassert_auto_sync },
            None,
            3000,
            &RepointApi,
        )
        .unwrap();
        assert!(matches!(
            report.skills[0].status,
            SkillRefreshStatus::Refreshed {
                reassert_error: None,
                ..
            }
        ));
        let targets = f.store.list_skill_targets(&f.skill_id).unwrap();
        assert_eq!(targets.len(), usize::from(reassert_auto_sync));
        assert_eq!(
            f.paths
                .home
                .join(tool.relative_skills_dir)
                .join("alpha/SKILL.md")
                .is_file(),
            reassert_auto_sync
        );
    }
}

struct EmptyRepointApi;
impl crate::core::git_acquisition::GithubApi for EmptyRepointApi {
    fn matching_refs(
        &self,
        repo: &crate::core::git_acquisition::GithubRepo,
        prefix: &str,
    ) -> anyhow::Result<Vec<String>> {
        RepointApi.matching_refs(repo, prefix)
    }

    fn branch_sha(&self, _: &crate::core::git_acquisition::GithubCoords) -> anyhow::Result<String> {
        Ok("sha".into())
    }
    fn download_directory(
        &self,
        _: &crate::core::git_acquisition::GithubCoords,
        dest: &Path,
        _: Option<&CancelToken>,
    ) -> anyhow::Result<()> {
        fs::create_dir_all(dest)?;
        fs::write(dest.join("README.md"), "not a skill")?;
        Ok(())
    }
}

#[test]
fn git_repoint_non_skill_directory_never_replaces_a_working_skill() {
    let f = git_repoint_fixture();
    let before = format!("{:?}", f.store.get_skill_by_id(&f.skill_id).unwrap());
    let report = super::repoint_git_skill_with(
        &f.paths,
        &f.store,
        &f.skill_id,
        "https://github.com/owner/repo/tree/main/docs",
        RefreshPolicy::default(),
        None,
        3000,
        &EmptyRepointApi,
    )
    .unwrap();
    let SkillRefreshStatus::Failed { error } = &report.skills[0].status else {
        panic!("{report:?}")
    };
    assert!(matches!(
        error.downcast_ref::<SignalError>(),
        Some(SignalError::SkillInvalid { .. })
    ));
    assert_eq!(
        format!("{:?}", f.store.get_skill_by_id(&f.skill_id).unwrap()),
        before
    );
}

struct MissingRepointApi;
impl crate::core::git_acquisition::GithubApi for MissingRepointApi {
    fn matching_refs(
        &self,
        repo: &crate::core::git_acquisition::GithubRepo,
        prefix: &str,
    ) -> anyhow::Result<Vec<String>> {
        RepointApi.matching_refs(repo, prefix)
    }

    fn branch_sha(&self, _: &crate::core::git_acquisition::GithubCoords) -> anyhow::Result<String> {
        Ok("sha".into())
    }
    fn download_directory(
        &self,
        _: &crate::core::git_acquisition::GithubCoords,
        _: &Path,
        _: Option<&CancelToken>,
    ) -> anyhow::Result<()> {
        Err(crate::core::github_download::GithubApiError {
            status: 404,
            reset_minutes: None,
            url: "https://api.github.com/repos/other/repo/contents/missing".into(),
        }
        .into())
    }
}

#[test]
fn git_repoint_404_preserves_every_record_field_and_central_bytes() {
    let f = git_repoint_fixture();
    let before = format!("{:?}", f.store.get_skill_by_id(&f.skill_id).unwrap());
    let report = super::repoint_git_skill_with(
        &f.paths,
        &f.store,
        &f.skill_id,
        "https://github.com/other/repo/tree/main/missing",
        RefreshPolicy::default(),
        None,
        3000,
        &MissingRepointApi,
    )
    .unwrap();
    let SkillRefreshStatus::Failed { error } = &report.skills[0].status else {
        panic!("{report:?}")
    };
    assert!(matches!(
        error.downcast_ref::<SignalError>(),
        Some(SignalError::GithubSkillNotFound { .. })
    ));
    assert_eq!(
        format!("{:?}", f.store.get_skill_by_id(&f.skill_id).unwrap()),
        before
    );
    let record = f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap();
    assert_eq!(
        fs::read_to_string(Path::new(&record.central_path).join("a.txt")).unwrap(),
        "v1"
    );
}

#[test]
fn git_repoint_to_another_repo_propagates_every_existing_scope() {
    use crate::core::skill_store::{
        ProjectRecord, ProjectSkillAssignmentRecord, SkillTargetRecord,
    };
    use crate::core::sync_status::{SyncMode, SyncStatus};
    let f = git_repoint_fixture();
    let tool = adapter_by_key("claude_code").unwrap();
    fs::create_dir_all(f.paths.home.join(tool.relative_detect_dir)).unwrap();
    let target = f.paths.home.join(tool.relative_skills_dir).join("alpha");
    f.store
        .upsert_skill_target(&SkillTargetRecord {
            id: "global".into(),
            skill_id: f.skill_id.clone(),
            tool: tool.key().into(),
            target_path: target.to_string_lossy().into(),
            mode: SyncMode::Copy,
            status: SyncStatus::Synced,
            last_error: None,
            synced_at: Some(1),
        })
        .unwrap();
    let project = f.paths.home.join("project");
    fs::create_dir_all(&project).unwrap();
    f.store
        .register_project(&ProjectRecord {
            id: "project".into(),
            path: project.to_string_lossy().into(),
            created_at: 1,
            updated_at: 1,
        })
        .unwrap();
    f.store
        .add_project_skill_assignment(&ProjectSkillAssignmentRecord {
            id: "assignment".into(),
            project_id: "project".into(),
            skill_id: f.skill_id.clone(),
            skill_name: "alpha".into(),
            tool: tool.key().into(),
            mode: SyncMode::Copy,
            status: SyncStatus::Synced,
            last_error: None,
            synced_at: Some(1),
            content_hash: None,
            created_at: 1,
        })
        .unwrap();
    let url = "https://github.com/new-owner/new-repo/tree/develop/new-skill";
    let report = super::repoint_git_skill_with(
        &f.paths,
        &f.store,
        &f.skill_id,
        url,
        RefreshPolicy::default(),
        None,
        3000,
        &RepointApi,
    )
    .unwrap();
    let SkillRefreshStatus::Refreshed { targets, .. } = &report.skills[0].status else {
        panic!("{report:?}")
    };
    assert_eq!(targets.len(), 2);
    assert!(targets
        .iter()
        .all(|target| !matches!(target.status, PropagationStatus::Failed { .. })));
    assert!(targets
        .iter()
        .any(|target| matches!(target.scope, PropagationScope::Global { .. })));
    assert!(targets
        .iter()
        .any(|target| matches!(target.scope, PropagationScope::Project { .. })));
    assert!(fs::read_to_string(target.join("SKILL.md"))
        .unwrap()
        .ends_with(url));
    let record = f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap();
    assert_eq!(record.source_ref.as_deref(), Some(url));
    assert_eq!(record.source_subpath.as_deref(), Some("new-skill"));
}

/// Seed a real local clone under the replacement URL's cache identity, so
/// repository-name resolution exercises acquisition without external networking.
fn seed_repoint_repository(f: &Fixture, matching_name: bool) {
    use crate::core::git_cache::{
        fetch_through_cache, repo_cache_key, CacheKeyInputs, FetchRequest,
    };
    let repo = fixture_repo();
    if matching_name {
        fs::write(
            repo.path().join("skills/a/SKILL.md"),
            "---\nname: alpha\n---\nnew bytes",
        )
        .unwrap();
    } else {
        fs::rename(
            repo.path().join("skills/a"),
            repo.path().join("skills/unrelated"),
        )
        .unwrap();
    }
    fs::create_dir_all(repo.path().join("skills/b")).unwrap();
    fs::write(
        repo.path().join("skills/b/SKILL.md"),
        "---\nname: beta\n---\n",
    )
    .unwrap();
    git(&["add", "-A"], repo.path());
    git(&["commit", "-q", "-m", "two skills"], repo.path());
    let url = repo.path().to_string_lossy();
    let (cached, _) = fetch_through_cache(
        &f.paths.cache_dir,
        &FetchRequest {
            clone_url: &url,
            branch: None,
            subpath: None,
            ttl_ms: 3600000,
            cancel: None,
        },
    )
    .unwrap();
    let key = repo_cache_key(&CacheKeyInputs {
        clone_url: "https://github.com/new/repo.git",
        branch: None,
    });
    fs::rename(&cached, cached.parent().unwrap().join(key)).unwrap();
}

#[test]
fn git_repoint_root_manifest_stores_no_subpath() {
    use crate::core::git_cache::{
        fetch_through_cache, repo_cache_key, CacheKeyInputs, FetchRequest,
    };
    let f = git_repoint_fixture();
    let repo = fixture_repo();
    fs::write(
        repo.path().join("SKILL.md"),
        "---\nname: alpha\n---\nroot bytes",
    )
    .unwrap();
    git(&["add", "-A"], repo.path());
    git(&["commit", "-q", "-m", "root skill"], repo.path());
    let (cached, _) = fetch_through_cache(
        &f.paths.cache_dir,
        &FetchRequest {
            clone_url: &repo.path().to_string_lossy(),
            branch: Some("main"),
            subpath: None,
            ttl_ms: 3600000,
            cancel: None,
        },
    )
    .unwrap();
    let key = repo_cache_key(&CacheKeyInputs {
        clone_url: "https://github.com/new/repo.git",
        branch: Some("main"),
    });
    fs::rename(&cached, cached.parent().unwrap().join(key)).unwrap();
    let report = super::repoint_git_skill_with(
        &f.paths,
        &f.store,
        &f.skill_id,
        "https://github.com/new/repo/blob/main/SKILL.md",
        RefreshPolicy::default(),
        None,
        3000,
        &RepointApi,
    )
    .unwrap();
    assert!(
        matches!(
            report.skills[0].status,
            SkillRefreshStatus::Refreshed { .. }
        ),
        "{report:?}"
    );
    let after = f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap();
    assert_eq!(after.source_subpath, None);
    assert!(
        fs::read_to_string(Path::new(&after.central_path).join("SKILL.md"))
            .unwrap()
            .ends_with("root bytes")
    );
}

#[test]
fn git_repoint_repo_url_resolves_by_the_existing_skill_name() {
    let f = git_repoint_fixture();
    seed_repoint_repository(&f, true);
    for url in [
        "https://github.com/new/repo",
        "http://github.com/new/repo",
        "github.com/new/repo",
        "https://github.com/new/repo.git",
    ] {
        let report = super::repoint_git_skill_with(
            &f.paths,
            &f.store,
            &f.skill_id,
            url,
            RefreshPolicy::default(),
            None,
            3000,
            &RepointApi,
        )
        .unwrap();
        assert!(
            matches!(
                report.skills[0].status,
                SkillRefreshStatus::Refreshed { .. }
            ),
            "{report:?}"
        );
        let record = f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap();
        assert_eq!(record.source_ref.as_deref(), Some(url));
        assert_eq!(record.source_subpath.as_deref(), Some("skills/a"));
        assert!(
            fs::read_to_string(Path::new(&record.central_path).join("SKILL.md"))
                .unwrap()
                .contains("new bytes")
        );
    }
}

#[test]
fn git_repoint_ambiguous_repo_preserves_the_record_byte_for_byte() {
    let f = git_repoint_fixture();
    seed_repoint_repository(&f, false);
    let before = format!("{:?}", f.store.get_skill_by_id(&f.skill_id).unwrap());
    let report = super::repoint_git_skill_with(
        &f.paths,
        &f.store,
        &f.skill_id,
        "https://github.com/new/repo",
        RefreshPolicy::default(),
        None,
        3000,
        &RepointApi,
    )
    .unwrap();
    let SkillRefreshStatus::Failed { error } = &report.skills[0].status else {
        panic!("{report:?}")
    };
    assert_eq!(
        error.downcast_ref::<SignalError>(),
        Some(&SignalError::MultiSkills)
    );
    assert_eq!(
        format!("{:?}", f.store.get_skill_by_id(&f.skill_id).unwrap()),
        before
    );
}

struct Fixture {
    _dir: tempfile::TempDir,
    paths: InstallerPaths,
    store: SkillStore,
    source: tempfile::TempDir,
    skill_id: String,
}

/// One local-source Managed skill installed into a temp central repo.
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = InstallerPaths {
        home: dir.path().join("home"),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.home).expect("create home");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");

    let source = tempfile::tempdir().expect("source tempdir");
    fs::write(source.path().join("SKILL.md"), "---\nname: alpha\n---\n").expect("write SKILL.md");
    fs::write(source.path().join("a.txt"), "v1").expect("write a.txt");
    let installed = install_local_skill(&paths, &store, source.path(), Some("alpha".to_string()))
        .expect("install");

    Fixture {
        _dir: dir,
        paths,
        store,
        source,
        skill_id: installed.skill_id,
    }
}

fn refresh(f: &Fixture, policy: RefreshPolicy) -> crate::core::refresh::RefreshReport {
    refresh_managed_skills(
        &f.paths,
        &f.store,
        RefreshSelection::All,
        policy,
        None,
        3000,
        |_| {},
    )
    .expect("refresh reads its own skill list")
}

#[test]
fn a_refreshed_skill_gets_its_new_bytes_and_reports_its_targets() {
    let f = fixture();
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    fs::create_dir_all(f.paths.home.join(claude.relative_detect_dir)).expect("install tool");
    fs::write(f.source.path().join("a.txt"), "v2").expect("write a.txt");

    let mut phases: Vec<(RefreshPhase, String)> = Vec::new();
    let report = refresh_managed_skills(
        &f.paths,
        &f.store,
        RefreshSelection::Ids(vec![f.skill_id.clone()]),
        RefreshPolicy::default(),
        None,
        3000,
        |p| phases.push((p.phase, p.skill_name.to_string())),
    )
    .expect("refresh");

    assert!(matches!(
        report.skills.as_slice(),
        [outcome] if matches!(outcome.status, SkillRefreshStatus::Refreshed { .. })
    ));
    let central = PathBuf::from(
        f.store
            .get_skill_by_id(&f.skill_id)
            .expect("query")
            .expect("skill")
            .central_path,
    );
    assert_eq!(
        fs::read_to_string(central.join("a.txt")).expect("read central"),
        "v2"
    );
    assert_eq!(
        phases,
        vec![
            (RefreshPhase::Acquiring, "alpha".to_string()),
            (RefreshPhase::Applying, "alpha".to_string()),
        ],
        "progress ticks come from the backend, one per phase step"
    );
}

#[test]
fn a_skill_that_fails_acquisition_is_reported_and_never_finalized() {
    let f = fixture();
    let central_before = f
        .store
        .get_skill_by_id(&f.skill_id)
        .expect("query")
        .expect("skill");
    // The local source is gone: acquisition cannot produce bytes. Named
    // explicitly (a single Update) — Refresh (all) would skip it instead.
    let source_path = f.source.path().to_path_buf();
    fs::remove_dir_all(&source_path).expect("remove source");

    let report = refresh_managed_skills(
        &f.paths,
        &f.store,
        RefreshSelection::Ids(vec![f.skill_id.clone()]),
        RefreshPolicy::default(),
        None,
        3000,
        |_| {},
    )
    .expect("refresh");

    assert!(
        matches!(
            report.skills.as_slice(),
            [outcome] if matches!(outcome.status, SkillRefreshStatus::Failed { .. })
        ),
        "got {:?}",
        report
    );
    let after = f
        .store
        .get_skill_by_id(&f.skill_id)
        .expect("query")
        .expect("skill");
    assert_eq!(
        after.updated_at, central_before.updated_at,
        "a skill that failed acquisition must not be finalized"
    );
    assert_eq!(
        fs::read_to_string(PathBuf::from(&after.central_path).join("a.txt")).expect("read central"),
        "v1",
        "the central copy is untouched"
    );
}

#[test]
fn reassert_auto_sync_creates_a_target_the_skill_was_never_on() {
    let f = fixture();
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    fs::create_dir_all(f.paths.home.join(claude.relative_detect_dir)).expect("install tool");
    assert!(
        f.store
            .list_skill_targets(&f.skill_id)
            .expect("query")
            .is_empty(),
        "the skill starts on no Tool"
    );

    refresh(
        &f,
        RefreshPolicy {
            reassert_auto_sync: true,
        },
    );

    let row = f
        .store
        .get_skill_target(&f.skill_id, "claude_code")
        .expect("query")
        .expect("the re-assert creates the missing target");
    assert!(PathBuf::from(&row.target_path).exists());
}

#[test]
fn without_the_reassert_policy_a_missing_target_stays_missing() {
    let f = fixture();
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    fs::create_dir_all(f.paths.home.join(claude.relative_detect_dir)).expect("install tool");

    refresh(&f, RefreshPolicy::default());

    assert!(
        f.store
            .get_skill_target(&f.skill_id, "claude_code")
            .expect("query")
            .is_none(),
        "refresh alone never creates a Sync target"
    );
    assert!(!f.paths.home.join(".claude/skills/alpha").exists());
}

/// A store failure inside the re-assert is report data, not a log line: the
/// skill stays `Refreshed` (finalize and Propagation did succeed) and carries
/// the error so the batch report can count it.
#[test]
fn a_failed_reassert_is_reported_alongside_the_refreshed_status() {
    let targets = vec![outcome_for("claude_code")];

    let (kept, error) = merge_reassert(targets, Err(anyhow::anyhow!("store is gone")));

    assert_eq!(kept.len(), 1, "Propagation's own outcomes survive");
    assert_eq!(
        format!(
            "{:#}",
            error.expect("the failure is carried, never dropped")
        ),
        "store is gone"
    );
}

#[test]
fn a_successful_reassert_extends_the_targets_and_reports_no_error() {
    let (kept, error) = merge_reassert(
        vec![outcome_for("claude_code")],
        Ok(vec![outcome_for("codex")]),
    );

    let tools: Vec<String> = kept
        .iter()
        .map(|o| match &o.scope {
            PropagationScope::Global { tool } => tool.clone(),
            other => panic!("expected a global scope, got {other:?}"),
        })
        .collect();
    assert_eq!(tools, vec!["claude_code".to_string(), "codex".to_string()]);
    assert!(error.is_none());
}

fn outcome_for(tool: &str) -> PropagationOutcome {
    PropagationOutcome {
        scope: PropagationScope::Global {
            tool: tool.to_string(),
        },
        status: PropagationStatus::Synced {
            mode_used: crate::core::sync_status::SyncMode::Symlink,
        },
    }
}

#[test]
fn a_skill_that_fails_acquisition_is_excluded_from_the_reassert() {
    let f = fixture();
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    fs::create_dir_all(f.paths.home.join(claude.relative_detect_dir)).expect("install tool");
    fs::remove_dir_all(f.source.path()).expect("remove source");

    refresh(
        &f,
        RefreshPolicy {
            reassert_auto_sync: true,
        },
    );

    assert!(
        f.store
            .get_skill_target(&f.skill_id, "claude_code")
            .expect("query")
            .is_none(),
        "a skill whose bytes could not be acquired must not be synced anywhere"
    );
}

// ---------------------------------------------------------------------------
// Phase one — the bounded acquisition pool
// ---------------------------------------------------------------------------

/// N local-source Managed skills named `s0..s{n-1}` in one temp fixture.
struct PoolFixture {
    _dir: tempfile::TempDir,
    paths: InstallerPaths,
    store: SkillStore,
    _sources: Vec<tempfile::TempDir>,
    names: Vec<String>,
}

fn pool_fixture(n: usize) -> PoolFixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = InstallerPaths {
        home: dir.path().join("home"),
        central_dir: dir.path().join("central"),
        cache_dir: dir.path().join("cache"),
    };
    fs::create_dir_all(&paths.home).expect("create home");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");

    let mut sources = Vec::new();
    let mut names = Vec::new();
    for i in 0..n {
        let name = format!("s{i}");
        let source = tempfile::tempdir().expect("source tempdir");
        fs::write(
            source.path().join("SKILL.md"),
            format!("---\nname: {name}\n---\n"),
        )
        .expect("write SKILL.md");
        install_local_skill(&paths, &store, source.path(), Some(name.clone())).expect("install");
        sources.push(source);
        names.push(name);
    }
    PoolFixture {
        _dir: dir,
        paths,
        store,
        _sources: sources,
        names,
    }
}

/// The real (fast, local-source) acquisition with an artificial per-skill
/// latency, so the pool's completion order and its cancellation window are
/// observable.
fn slow_acquire<'a>(
    f: &'a PoolFixture,
    delay: impl Fn(&str) -> Duration + Sync + 'a,
) -> impl Fn(&str, Option<&CancelToken>) -> anyhow::Result<crate::core::installer::AcquiredUpdate>
       + Sync
       + 'a {
    move |skill_id, cancel| {
        let name = f
            .store
            .get_skill_by_id(skill_id)
            .expect("query")
            .expect("skill")
            .name;
        std::thread::sleep(delay(&name));
        crate::core::installer::acquire_managed_skill_update_with(
            &f.paths,
            &f.store,
            skill_id,
            cancel,
            &crate::core::git_acquisition::HttpGithubApi::new(None),
            0,
        )
    }
}

/// A door every acquisition passes through, counting how many are inside at
/// once. Until overlap has been witnessed, an acquisition waits at the door
/// for a second one to arrive — a rendezvous only a pool can complete, so
/// concurrency is proven by what was observed, never by how long the batch
/// took. The wait is bounded so a sequential regression fails the assertion
/// below instead of hanging the gate; once it has timed out nobody waits.
struct OverlapDoor {
    in_flight: Mutex<usize>,
    arrived: Condvar,
    max_in_flight: AtomicUsize,
    gave_up: AtomicBool,
}

impl OverlapDoor {
    const RENDEZVOUS_BOUND: Duration = Duration::from_secs(10);

    fn new() -> Self {
        Self {
            in_flight: Mutex::new(0),
            arrived: Condvar::new(),
            max_in_flight: AtomicUsize::new(0),
            gave_up: AtomicBool::new(false),
        }
    }

    fn overlap_witnessed(&self) -> bool {
        self.max_in_flight.load(Ordering::SeqCst) >= 2
    }

    fn enter(&self) {
        let mut inside = self.in_flight.lock().expect("door lock");
        *inside += 1;
        self.arrived.notify_all();
        while *inside < 2 && !self.overlap_witnessed() && !self.gave_up.load(Ordering::SeqCst) {
            let (guard, timeout) = self
                .arrived
                .wait_timeout(inside, Self::RENDEZVOUS_BOUND)
                .expect("door lock");
            inside = guard;
            if timeout.timed_out() {
                self.gave_up.store(true, Ordering::SeqCst);
            }
        }
        self.max_in_flight.fetch_max(*inside, Ordering::SeqCst);
    }

    fn leave(&self) {
        *self.in_flight.lock().expect("door lock") -= 1;
    }
}

/// Eight skills through a pool of four: at some point two acquisitions are
/// inside the door together. A sequential batch would never have two in
/// flight, whatever the load on the machine.
#[test]
fn acquisitions_overlap_instead_of_running_one_at_a_time() {
    let f = pool_fixture(8);
    let door = OverlapDoor::new();
    let inner = slow_acquire(&f, |_| Duration::ZERO);
    let acquire = |skill_id: &str, cancel: Option<&CancelToken>| {
        door.enter();
        let result = inner(skill_id, cancel);
        door.leave();
        result
    };

    let report = refresh_managed_skills_with(
        &f.paths,
        &f.store,
        RefreshSelection::All,
        RefreshPolicy::default(),
        None,
        3000,
        |_| {},
        &acquire,
    )
    .expect("refresh");

    assert_eq!(report.skills.len(), 8);
    assert!(
        report
            .skills
            .iter()
            .all(|o| matches!(o.status, SkillRefreshStatus::Refreshed { .. })),
        "every skill is refreshed: {report:?}"
    );
    let observed = door.max_in_flight.load(Ordering::SeqCst);
    assert!(
        observed >= 2,
        "acquisitions ran one at a time: at most {observed} in flight"
    );
}

/// Progress reads as completion order, not dispatch order: the slowest skill
/// is dispatched first and still ticks last, and the indices count completions
/// 1..n.
#[test]
fn acquire_progress_counts_completions_in_completion_order() {
    let f = pool_fixture(4);
    // s0 is dispatched first and is by far the slowest.
    let acquire = slow_acquire(&f, |name| {
        Duration::from_millis(if name == "s0" { 400 } else { 20 })
    });

    let mut ticks: Vec<(usize, String)> = Vec::new();
    refresh_managed_skills_with(
        &f.paths,
        &f.store,
        RefreshSelection::All,
        RefreshPolicy::default(),
        None,
        3000,
        |p| {
            if p.phase == RefreshPhase::Acquiring {
                ticks.push((p.index, p.skill_name.to_string()));
            }
        },
        &acquire,
    )
    .expect("refresh");

    assert_eq!(
        ticks.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
        vec![1, 2, 3, 4],
        "acquire ticks count completions"
    );
    assert_eq!(
        ticks.last().map(|(_, name)| name.as_str()),
        Some("s0"),
        "the slowest skill ticks last even though it was dispatched first: {ticks:?}"
    );
    assert_eq!(f.names.len(), 4);
}

/// Cancellation observed mid-batch: nothing is finalized (no `Applying` tick,
/// no central copy rewritten) and every skill is reported as cancelled.
#[test]
fn cancelling_mid_batch_finalizes_nothing() {
    let f = pool_fixture(6);
    let token = CancelToken::new();
    let completed = Mutex::new(0usize);
    let inner = slow_acquire(&f, |_| Duration::from_millis(30));
    let acquire = |skill_id: &str, cancel: Option<&CancelToken>| {
        let result = inner(skill_id, cancel);
        let mut done = completed.lock().expect("lock");
        *done += 1;
        if *done == 2 {
            token.cancel();
        }
        result
    };
    let before: Vec<(String, i64)> = f
        .store
        .list_skills()
        .expect("list")
        .into_iter()
        .map(|s| (s.id, s.updated_at))
        .collect();

    let mut phases: Vec<RefreshPhase> = Vec::new();
    let report = refresh_managed_skills_with(
        &f.paths,
        &f.store,
        RefreshSelection::All,
        RefreshPolicy::default(),
        Some(&token),
        3000,
        |p| phases.push(p.phase),
        &acquire,
    )
    .expect("a cancelled batch still reports");

    assert_eq!(report.skills.len(), 6, "every selected skill is reported");
    assert!(
        !phases.contains(&RefreshPhase::Applying),
        "a cancelled batch never enters the apply phase: {phases:?}"
    );
    assert!(
        report.skills.iter().all(|o| matches!(
            &o.status,
            SkillRefreshStatus::Failed { error }
                if error.downcast_ref::<SignalError>() == Some(&SignalError::Cancelled)
        )),
        "cancelled skills are reported as cancelled: {report:?}"
    );
    let after: Vec<(String, i64)> = f
        .store
        .list_skills()
        .expect("list")
        .into_iter()
        .map(|s| (s.id, s.updated_at))
        .collect();
    assert_eq!(before, after, "no skill was finalized");
}

// ---------------------------------------------------------------------------
// Same-repository skills under the pool
// ---------------------------------------------------------------------------

fn git(args: &[&str], cwd: &Path) {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.com")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.com")
        .output()
        .expect("run git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A local repository holding one skill under `skills/a`, cloned by path.
fn fixture_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    git(&["init", "-q", "-b", "main", "."], dir.path());
    fs::create_dir_all(dir.path().join("skills/a")).expect("mkdir");
    fs::write(
        dir.path().join("skills/a/SKILL.md"),
        "---\nname: shared\n---\n",
    )
    .expect("write");
    git(&["add", "-A"], dir.path());
    git(&["commit", "-q", "-m", "init"], dir.path());
    dir
}

/// Two Managed skills from the same repository are acquired concurrently by
/// the pool. `git_cache`'s per-key lock is what keeps that safe: both land the
/// same revision and the cache entry's metadata is intact.
#[test]
fn two_skills_from_one_repository_share_the_cache_without_corrupting_it() {
    let f = pool_fixture(0);
    let repo = fixture_repo();
    let url = repo.path().to_string_lossy().to_string();
    for name in ["one", "two"] {
        crate::core::installer::install_git_skill_from_selection(
            &f.paths,
            &f.store,
            &url,
            "skills/a",
            Some(name.to_string()),
            None,
        )
        .expect("install from the fixture repo");
    }

    let report = refresh_managed_skills(
        &f.paths,
        &f.store,
        RefreshSelection::All,
        RefreshPolicy::default(),
        None,
        3000,
        |_| {},
    )
    .expect("refresh");

    let revisions: Vec<String> = report
        .skills
        .iter()
        .map(|o| match &o.status {
            SkillRefreshStatus::Refreshed {
                source_revision, ..
            } => source_revision
                .clone()
                .expect("a git skill records its revision"),
            other => panic!("expected a refreshed skill, got {other:?}"),
        })
        .collect();
    assert_eq!(revisions.len(), 2);
    assert_eq!(
        revisions[0], revisions[1],
        "both skills come from the same commit"
    );

    let cache_root = f.paths.cache_dir.join("skills-hub-git-cache");
    let entries: Vec<PathBuf> = fs::read_dir(&cache_root)
        .expect("read cache root")
        .map(|e| e.expect("entry").path())
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "one clone URL + subpath is one cache entry"
    );
    let meta = fs::read_to_string(entries[0].join(".skills-hub-cache.json"))
        .expect("the cache metadata survives concurrent fetches");
    assert!(
        meta.contains(&revisions[0]),
        "cache metadata records the fetched head: {meta}"
    );
}

/// An `imported` Managed skill taken over from `claude_code`'s skills dir,
/// installed the way Onboarding import records it.
fn install_imported(paths: &InstallerPaths, store: &SkillStore, name: &str) -> String {
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    let found = paths.home.join(claude.relative_skills_dir).join(name);
    fs::create_dir_all(&found).expect("tool skill dir");
    fs::write(found.join("SKILL.md"), format!("---\nname: {name}\n---\n")).expect("write");
    install_imported_skill(
        paths,
        store,
        &found,
        Some(name.to_string()),
        Some("claude_code"),
    )
    .expect("import")
    .skill_id
}

/// Refresh (all) over one `git` and one `imported` skill: the imported one
/// is not a member of the batch — not acquired, not applied, not reported —
/// so the report counts exactly one skill, not "1 refreshed, 1 skipped".
#[test]
fn refresh_all_never_puts_an_imported_skill_in_the_batch() {
    let f = pool_fixture(0);
    let repo = fixture_repo();
    let url = repo.path().to_string_lossy().to_string();
    let git_skill = crate::core::installer::install_git_skill_from_selection(
        &f.paths,
        &f.store,
        &url,
        "skills/a",
        Some("from-git".to_string()),
        None,
    )
    .expect("install from the fixture repo");
    let imported_id = install_imported(&f.paths, &f.store, "taken-over");
    let imported_before = f
        .store
        .get_skill_by_id(&imported_id)
        .expect("query")
        .expect("imported record");

    let acquired: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let mut progress: Vec<(RefreshPhase, String)> = Vec::new();
    let report = refresh_managed_skills_with(
        &f.paths,
        &f.store,
        RefreshSelection::All,
        RefreshPolicy {
            reassert_auto_sync: true,
        },
        None,
        3000,
        |p| progress.push((p.phase, p.skill_name.to_string())),
        &|skill_id, cancel| {
            acquired.lock().unwrap().push(skill_id.to_string());
            crate::core::installer::acquire_managed_skill_update_with(
                &f.paths,
                &f.store,
                skill_id,
                cancel,
                &crate::core::git_acquisition::HttpGithubApi::new(None),
                0,
            )
        },
    )
    .expect("refresh");

    assert_eq!(
        acquired.into_inner().unwrap(),
        vec![git_skill.skill_id.clone()],
        "only the git skill reaches the acquire pool"
    );
    assert_eq!(report.skills.len(), 1, "got {:?}", report);
    assert_eq!(report.skills[0].skill_id, git_skill.skill_id);
    assert!(matches!(
        report.skills[0].status,
        SkillRefreshStatus::Refreshed { .. }
    ));
    assert!(
        progress
            .iter()
            .all(|(_, name)| name == "from-git" && progress.len() == 2),
        "progress ticks name the batch's one member only: {:?}",
        progress
    );
    // The auto-sync re-assert ran for the git skill only: the imported skill
    // got no target and its record was not touched.
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    fs::create_dir_all(f.paths.home.join(claude.relative_detect_dir)).expect("install tool");
    assert!(
        f.store
            .list_skill_targets(&imported_id)
            .expect("query")
            .is_empty(),
        "reassert_auto_sync never runs for a non-member"
    );
    let imported_after = f
        .store
        .get_skill_by_id(&imported_id)
        .expect("query")
        .expect("imported record");
    assert_eq!(imported_after.updated_at, imported_before.updated_at);
}

/// A single Update names the skill explicitly; for an imported skill the
/// answer is a typed refusal, not a silent empty report.
#[test]
fn a_single_update_of_an_imported_skill_is_refused_with_a_typed_condition() {
    let f = pool_fixture(0);
    let imported_id = install_imported(&f.paths, &f.store, "taken-over");

    let report = refresh_managed_skills(
        &f.paths,
        &f.store,
        RefreshSelection::Ids(vec![imported_id.clone()]),
        RefreshPolicy::default(),
        None,
        3000,
        |_| {},
    )
    .expect("refresh");

    let [outcome] = report.skills.as_slice() else {
        panic!("one outcome for the one requested skill: {:?}", report);
    };
    let SkillRefreshStatus::Failed { error } = &outcome.status else {
        panic!("an imported skill cannot be updated: {:?}", outcome);
    };
    assert_eq!(
        error.downcast_ref::<SignalError>(),
        Some(&SignalError::NotRefreshable {
            name: "taken-over".to_string(),
        }),
        "got {error:#}"
    );
    assert!(
        f.paths
            .central_dir
            .join("taken-over")
            .join("SKILL.md")
            .is_file(),
        "the central copy is untouched"
    );
}

// ---------------------------------------------------------------------------
// Unlocatable skills
// ---------------------------------------------------------------------------

/// Refresh (all) over one healthy `local` skill, one whose source folder is
/// gone and one whose central copy is gone: only the healthy one reaches the
/// acquire pool; the two unlocatable ones are reported *skipped* by name
/// with their state, and — auto-sync on — get no Sync target minted (a
/// central-missing skill's target would be a dangling link).
#[test]
fn refresh_all_skips_unlocatable_skills_and_mints_no_targets_for_them() {
    let f = pool_fixture(3);
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    fs::create_dir_all(f.paths.home.join(claude.relative_detect_dir)).expect("install tool");
    let ids: Vec<String> = f
        .store
        .list_skills()
        .expect("list")
        .into_iter()
        .map(|s| s.id)
        .collect();
    let by_name = |name: &str| {
        f.store
            .list_skills()
            .expect("list")
            .into_iter()
            .find(|s| s.name == name)
            .expect("skill")
    };
    // s1: source folder gone (moved away).
    let mut source_gone = by_name("s1");
    source_gone.source_ref = Some(
        f.paths
            .home
            .join("moved-away")
            .to_string_lossy()
            .to_string(),
    );
    f.store.upsert_skill(&source_gone).expect("upsert");
    // s2: central copy gone.
    let central_gone = by_name("s2");
    fs::remove_dir_all(&central_gone.central_path).expect("remove central");

    let acquired: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let mut progress: Vec<(RefreshPhase, String)> = Vec::new();
    let report = refresh_managed_skills_with(
        &f.paths,
        &f.store,
        RefreshSelection::All,
        RefreshPolicy {
            reassert_auto_sync: true,
        },
        None,
        3000,
        |p| progress.push((p.phase, p.skill_name.to_string())),
        &|skill_id, cancel| {
            acquired.lock().unwrap().push(skill_id.to_string());
            crate::core::installer::acquire_managed_skill_update_with(
                &f.paths,
                &f.store,
                skill_id,
                cancel,
                &crate::core::git_acquisition::HttpGithubApi::new(None),
                0,
            )
        },
    )
    .expect("refresh");

    let healthy = by_name("s0");
    assert_eq!(
        acquired.into_inner().unwrap(),
        vec![healthy.id.clone()],
        "only the healthy skill reaches the acquire pool"
    );
    assert!(
        progress.iter().all(|(_, name)| name == "s0"),
        "progress ticks name the batch's members only: {progress:?}"
    );
    assert_eq!(ids.len(), 3);
    assert_eq!(
        report.skills.len(),
        3,
        "every skill is accounted for: {report:?}"
    );
    let status_of = |name: &str| {
        &report
            .skills
            .iter()
            .find(|o| o.skill_name == name)
            .unwrap_or_else(|| panic!("{name} is in the report: {report:?}"))
            .status
    };
    assert!(matches!(
        status_of("s0"),
        SkillRefreshStatus::Refreshed { .. }
    ));
    assert!(matches!(
        status_of("s1"),
        SkillRefreshStatus::Skipped {
            state: UnlocatableState::SourceMissing
        }
    ));
    assert!(matches!(
        status_of("s2"),
        SkillRefreshStatus::Skipped {
            state: UnlocatableState::CentralMissing
        }
    ));
    // The auto-sync re-assert ran for the healthy skill only.
    assert!(f
        .store
        .get_skill_target(&healthy.id, "claude_code")
        .expect("query")
        .is_some());
    for name in ["s1", "s2"] {
        assert!(
            f.store
                .list_skill_targets(&by_name(name).id)
                .expect("query")
                .is_empty(),
            "no target is minted for a skipped skill ({name})"
        );
    }
    assert!(!f.paths.home.join(".claude/skills/s2").exists());
}

/// Restore is the single-skill Update of a `git` skill whose central copy is
/// gone: it is re-acquired from its repository, the central copy is rebuilt
/// at the recorded path, and Propagation follows — the Tool's link, dangling
/// a moment ago, resolves again.
#[test]
fn restore_of_a_git_skill_with_no_central_copy_rebuilds_it_and_propagation_follows() {
    let f = pool_fixture(0);
    let repo = fixture_repo();
    let url = repo.path().to_string_lossy().to_string();
    let installed = crate::core::installer::install_git_skill_from_selection(
        &f.paths,
        &f.store,
        &url,
        "skills/a",
        Some("from-git".to_string()),
        None,
    )
    .expect("install from the fixture repo");
    let claude = adapter_by_key("claude_code").expect("claude_code adapter");
    fs::create_dir_all(f.paths.home.join(claude.relative_detect_dir)).expect("install tool");
    // Put the skill on the Tool (a link into the central copy).
    refresh_managed_skills(
        &f.paths,
        &f.store,
        RefreshSelection::All,
        RefreshPolicy {
            reassert_auto_sync: true,
        },
        None,
        3000,
        |_| {},
    )
    .expect("refresh");
    let link = PathBuf::from(
        f.store
            .get_skill_target(&installed.skill_id, "claude_code")
            .expect("query")
            .expect("target")
            .target_path,
    );
    assert!(link.join("SKILL.md").is_file());

    fs::remove_dir_all(&installed.central_path).expect("lose the central copy");
    assert!(
        !link.join("SKILL.md").exists(),
        "the Tool's link dangles once the central copy is gone"
    );

    let report = refresh_managed_skills(
        &f.paths,
        &f.store,
        RefreshSelection::Ids(vec![installed.skill_id.clone()]),
        RefreshPolicy::default(),
        None,
        3000,
        |_| {},
    )
    .expect("refresh");

    let [outcome] = report.skills.as_slice() else {
        panic!("one outcome for the one requested skill: {report:?}");
    };
    let SkillRefreshStatus::Refreshed { targets, .. } = &outcome.status else {
        panic!("Restore re-acquires and finalizes: {outcome:?}");
    };
    assert!(
        installed.central_path.join("SKILL.md").is_file(),
        "the central copy is rebuilt at its recorded path"
    );
    assert!(
        targets.iter().any(|t| matches!(
            &t.scope,
            PropagationScope::Global { tool } if tool == "claude_code"
        )),
        "Propagation reported the Tool's target: {targets:?}"
    );
    assert!(
        link.join("SKILL.md").is_file(),
        "the Tool's link resolves again"
    );
    let record = f
        .store
        .get_skill_by_id(&installed.skill_id)
        .expect("query")
        .expect("record");
    assert_eq!(
        record.central_path,
        installed.central_path.to_string_lossy()
    );
}
