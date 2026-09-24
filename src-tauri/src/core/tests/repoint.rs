//! Tests for `core::repoint` — Re-point over a target enum: every Managed
//! skill (`git`, `local`, `imported`) toward a GitHub URL or a local folder,
//! through the single-skill Update.
//!
//! Every case runs against a temp home / central dir / DB; the git arm uses
//! an injected `GithubApi` (or a seeded clone cache), never the network.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::core::cancel_token::CancelToken;
use crate::core::errors::{CommandError, SignalError};
use crate::core::git_acquisition::GithubApi;
use crate::core::installer::{install_imported_skill, install_local_skill, InstallerPaths};
use crate::core::manifest::{read_invocation_lines, InvocationMode};
use crate::core::propagation::{PropagationScope, PropagationStatus};
use crate::core::provenance::{is_refreshable, refresh_eligibility, RefreshEligibility};
use crate::core::refresh::{
    refresh_managed_skills, RefreshPolicy, RefreshReport, RefreshSelection, SkillRefreshStatus,
};
use crate::core::repoint::{repoint_skill_source_with, RepointTarget};
use crate::core::skill_store::{SkillRecord, SkillStore};
use crate::core::tool_adapters::{adapter_by_key, mark_installed_in};
use crate::core::unlocatable::{unlocatable_state, UnlocatableState};

/// The git arm with the call shape the pre-enum tests were written against.
#[allow(clippy::too_many_arguments)]
fn repoint_git(
    paths: &InstallerPaths,
    store: &SkillStore,
    skill_id: &str,
    url: &str,
    policy: RefreshPolicy,
    cancel: Option<&CancelToken>,
    now: i64,
    api: &(dyn GithubApi + Sync),
) -> anyhow::Result<RefreshReport> {
    repoint_skill_source_with(
        paths,
        store,
        skill_id,
        RepointTarget::Git { url: url.into() },
        policy,
        cancel,
        now,
        |_| {},
        api,
    )
}

/// The local arm. The API is never consulted; `RepointApi` stands in.
fn repoint_local(
    paths: &InstallerPaths,
    store: &SkillStore,
    skill_id: &str,
    folder: &Path,
    policy: RefreshPolicy,
) -> anyhow::Result<RefreshReport> {
    repoint_skill_source_with(
        paths,
        store,
        skill_id,
        RepointTarget::Local {
            path: folder.to_string_lossy().into_owned(),
        },
        policy,
        None,
        3000,
        |_| {},
        &RepointApi,
    )
}

fn refresh(f: &Fixture, policy: RefreshPolicy) -> RefreshReport {
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

fn record(f: &Fixture) -> SkillRecord {
    f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap()
}

fn assert_refreshed(report: &RefreshReport) {
    assert!(
        matches!(
            report.skills.as_slice(),
            [outcome] if matches!(outcome.status, SkillRefreshStatus::Refreshed { .. })
        ),
        "{report:?}"
    );
}

// ---------------------------------------------------------------------------
// Git target (ported from the git-only Re-point)
// ---------------------------------------------------------------------------

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
    let report = repoint_git(
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
    let report = repoint_git(
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
    mark_installed_in(&f.paths.home, tool);
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
    let report = repoint_git(
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
        let report = repoint_git(
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
        let result = repoint_git(
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
        mark_installed_in(&f.paths.home, tool);
        assert!(f.store.list_skill_targets(&f.skill_id).unwrap().is_empty());
        let report = repoint_git(
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
    let report = repoint_git(
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
    // Re-point shares install's manifest gate (`ensure_installable_skill_dir`),
    // so it refuses with install's token.
    assert!(
        matches!(
            error,
            CommandError::SkillInvalid { reason } if reason == "missing_skill_md"
        ),
        "{error:#}"
    );
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
    let report = repoint_git(
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
    assert!(matches!(error, CommandError::GithubSkillNotFound { .. }));
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
    mark_installed_in(&f.paths.home, tool);
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
    let report = repoint_git(
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
    let report = repoint_git(
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
        let report = repoint_git(
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
    let report = repoint_git(
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
    assert!(matches!(error, CommandError::MultiSkills));
    assert_eq!(
        format!("{:?}", f.store.get_skill_by_id(&f.skill_id).unwrap()),
        before
    );
}

struct Fixture {
    _dir: tempfile::TempDir,
    paths: InstallerPaths,
    store: SkillStore,
    _source: tempfile::TempDir,
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
        _source: source,
        skill_id: installed.skill_id,
    }
}

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

// ---------------------------------------------------------------------------
// Local target (ported from the local-only Re-point of an Unlocatable skill)
// ---------------------------------------------------------------------------

/// The fixture's `local` skill after its folder moved: the record still
/// names the old folder, which is gone; the new one holds `a.txt = v2`.
fn moved_source(f: Fixture) -> (Fixture, PathBuf) {
    let old = PathBuf::from(record(&f).source_ref.unwrap());
    let new = f.paths.home.join("Documents/new-place");
    fs::create_dir_all(new.parent().unwrap()).unwrap();
    // A rename across temp dirs may cross filesystems; copy then delete.
    crate::core::sync_engine::copy_dir_recursive(&old, &new).unwrap();
    fs::remove_dir_all(&old).unwrap();
    fs::write(new.join("a.txt"), "v2").unwrap();
    (f, new)
}

fn central(f: &Fixture) -> PathBuf {
    PathBuf::from(record(f).central_path)
}

#[test]
fn local_repoint_honours_auto_sync_reassert_policy() {
    for reassert_auto_sync in [false, true] {
        let (f, new) = moved_source(fixture());
        let tool = adapter_by_key("claude_code").unwrap();
        mark_installed_in(&f.paths.home, tool);
        assert!(f.store.list_skill_targets(&f.skill_id).unwrap().is_empty());
        let report = repoint_local(
            &f.paths,
            &f.store,
            &f.skill_id,
            &new,
            RefreshPolicy { reassert_auto_sync },
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

/// Re-point is one operation: the record names the new folder and the
/// central copy holds its bytes when the call returns, with the Update's
/// outcome as report data.
#[test]
fn local_repoint_rewrites_the_source_and_lands_the_new_folders_bytes() {
    let (f, new) = moved_source(fixture());
    assert_eq!(
        unlocatable_state(&record(&f)),
        Some(UnlocatableState::SourceMissing)
    );

    let report = repoint_local(
        &f.paths,
        &f.store,
        &f.skill_id,
        &new,
        RefreshPolicy::default(),
    )
    .expect("re-point");

    assert_refreshed(&report);
    assert_eq!(report.skills[0].skill_id, f.skill_id);
    let after = record(&f);
    assert_eq!(after.source_ref.as_deref(), Some(new.to_str().unwrap()));
    assert_eq!(after.source_type, "local", "still a local skill");
    assert_eq!(unlocatable_state(&after), None);
    assert_eq!(
        fs::read_to_string(central(&f).join("a.txt")).unwrap(),
        "v2",
        "the Update copied from the new folder"
    );
}

/// The operator's `~` is expanded against the home core was handed.
#[test]
fn local_repoint_expands_a_home_relative_folder() {
    let (f, new) = moved_source(fixture());
    let report = repoint_skill_source_with(
        &f.paths,
        &f.store,
        &f.skill_id,
        RepointTarget::Local {
            path: "~/Documents/new-place".into(),
        },
        RefreshPolicy::default(),
        None,
        3000,
        |_| {},
        &RepointApi,
    )
    .unwrap();
    assert_refreshed(&report);
    assert_eq!(
        record(&f).source_ref.as_deref(),
        Some(new.to_str().unwrap())
    );
}

#[test]
fn local_repoint_validates_the_new_folder_the_way_add_does() {
    let (f, _new) = moved_source(fixture());
    let before = format!("{:?}", record(&f));
    let repoint = |folder: &Path| {
        repoint_local(
            &f.paths,
            &f.store,
            &f.skill_id,
            folder,
            RefreshPolicy::default(),
        )
    };

    // Not there.
    let err = repoint(&f.paths.home.join("nowhere")).expect_err("a missing folder is refused");
    assert!(matches!(
        err.downcast_ref::<SignalError>(),
        Some(SignalError::SourcePathMissing { .. })
    ));

    // No SKILL.md.
    let empty = f.paths.home.join("Documents/empty");
    fs::create_dir_all(&empty).unwrap();
    let err = repoint(&empty).expect_err("a folder without SKILL.md is refused");
    assert!(matches!(
        err.downcast_ref::<SignalError>(),
        Some(SignalError::SkillInvalid { .. })
    ));

    // Inside a Tool's skills dir: a Tool's copy is not a source.
    let in_tool = f.paths.home.join(".claude/skills/alpha");
    fs::create_dir_all(&in_tool).unwrap();
    fs::write(in_tool.join("SKILL.md"), "---\nname: alpha\n---\n").unwrap();
    let err = repoint(&in_tool).expect_err("a Tool-dir folder is refused");
    assert_eq!(
        err.downcast_ref::<SignalError>(),
        Some(&SignalError::LocalSourceInsideToolDir {
            path: in_tool.to_string_lossy().to_string(),
            tool: "claude_code".to_string(),
        })
    );

    assert_eq!(
        format!("{:?}", record(&f)),
        before,
        "a refused re-point changes nothing"
    );
    assert_eq!(
        fs::read_to_string(central(&f).join("a.txt")).unwrap(),
        "v1",
        "a refused re-point runs no Update"
    );
}

#[test]
fn an_unknown_skill_is_refused_typed_for_either_target() {
    let f = fixture();
    let dir = f.paths.home.join("Documents/any");
    for target in [
        RepointTarget::Local {
            path: dir.to_string_lossy().into_owned(),
        },
        RepointTarget::Git {
            url: "https://github.com/owner/repo".into(),
        },
    ] {
        let err = repoint_skill_source_with(
            &f.paths,
            &f.store,
            "no-such-id",
            target,
            RefreshPolicy::default(),
            None,
            3000,
            |_| {},
            &RepointApi,
        )
        .expect_err("unknown id");
        assert!(matches!(
            err.downcast_ref::<SignalError>(),
            Some(SignalError::NotFound { .. })
        ));
    }
}

// ---------------------------------------------------------------------------
// Crossing provenance (D4): every {git, local, imported} → {git, local}
// ---------------------------------------------------------------------------

/// A skill folder outside every Tool's skills dir with fresh bytes.
fn new_folder(f: &Fixture, body: &str) -> PathBuf {
    let folder = f.paths.home.join("Documents/maintained/alpha");
    fs::create_dir_all(&folder).unwrap();
    fs::write(
        folder.join("SKILL.md"),
        format!("---\nname: alpha\n---\n{body}"),
    )
    .unwrap();
    folder
}

/// The fixture's skill turned `git` with a recorded revision.
fn as_git(f: &Fixture) -> SkillRecord {
    let mut row = record(f);
    row.source_type = "git".into();
    row.source_ref = Some("https://github.com/owner/repo/tree/main/old".into());
    row.source_subpath = Some("old".into());
    row.source_revision = Some("old-sha".into());
    f.store.upsert_skill(&row).unwrap();
    row
}

/// The fixture's skill turned `imported` from `claude_code`, the way
/// Onboarding import records one: no source, found-in Tool kept.
fn as_imported(f: &Fixture) -> SkillRecord {
    let claude = adapter_by_key("claude_code").unwrap();
    let found = f.paths.home.join(claude.relative_skills_dir).join("alpha");
    fs::create_dir_all(&found).unwrap();
    fs::write(found.join("SKILL.md"), "---\nname: alpha\n---\n").unwrap();
    let id = install_imported_skill(
        &f.paths,
        &f.store,
        &found,
        Some("alpha-imported".to_string()),
        Some("claude_code"),
    )
    .unwrap()
    .skill_id;
    let row = f.store.get_skill_by_id(&id).unwrap().unwrap();
    assert_eq!(row.source_type, "imported");
    assert_eq!(row.imported_from_tool.as_deref(), Some("claude_code"));
    assert_eq!(refresh_eligibility(&row), RefreshEligibility::NotAMember);
    row
}

fn assert_local_source(row: &SkillRecord, folder: &Path) {
    assert_eq!(row.source_type, "local");
    assert_eq!(row.source_ref.as_deref(), folder.to_str());
    assert_eq!(row.source_subpath, None);
    assert_eq!(row.source_revision, None);
    assert_eq!(row.imported_from_tool, None);
    assert!(is_refreshable(row));
    assert_eq!(refresh_eligibility(row), RefreshEligibility::Refreshable);
}

fn assert_git_source(row: &SkillRecord, url: &str) {
    assert_eq!(row.source_type, "git");
    assert_eq!(row.source_ref.as_deref(), Some(url));
    assert_eq!(row.source_subpath.as_deref(), Some("skills/alpha"));
    assert_eq!(row.source_revision.as_deref(), Some("new-sha"));
    assert_eq!(row.imported_from_tool, None);
    assert!(is_refreshable(row));
    assert_eq!(refresh_eligibility(row), RefreshEligibility::Refreshable);
}

const NEW_URL: &str = "https://github.com/new/home/tree/main/skills/alpha";

#[test]
fn git_to_local_records_the_folder_and_drops_every_git_fact() {
    let f = fixture();
    as_git(&f);
    let folder = new_folder(&f, "folder bytes");

    let report = repoint_local(
        &f.paths,
        &f.store,
        &f.skill_id,
        &folder,
        RefreshPolicy::default(),
    )
    .unwrap();

    assert_refreshed(&report);
    assert_local_source(&record(&f), &folder);
    assert!(fs::read_to_string(central(&f).join("SKILL.md"))
        .unwrap()
        .ends_with("folder bytes"));
}

#[test]
fn local_to_git_records_the_acquisition_as_add_does() {
    let f = fixture();
    let report = repoint_git(
        &f.paths,
        &f.store,
        &f.skill_id,
        NEW_URL,
        RefreshPolicy::default(),
        None,
        3000,
        &RepointApi,
    )
    .unwrap();

    assert_refreshed(&report);
    assert_git_source(&record(&f), NEW_URL);
    assert!(fs::read_to_string(central(&f).join("SKILL.md"))
        .unwrap()
        .contains("new bytes"));
}

#[test]
fn imported_to_local_stops_being_imported_and_becomes_refreshable() {
    let f = fixture();
    let imported = as_imported(&f);
    let folder = new_folder(&f, "folder bytes");

    let report = repoint_local(
        &f.paths,
        &f.store,
        &imported.id,
        &folder,
        RefreshPolicy::default(),
    )
    .unwrap();

    assert_refreshed(&report);
    let after = f.store.get_skill_by_id(&imported.id).unwrap().unwrap();
    assert_local_source(&after, &folder);
    assert_eq!(
        after.central_path, imported.central_path,
        "same central copy"
    );
    assert!(
        fs::read_to_string(Path::new(&after.central_path).join("SKILL.md"))
            .unwrap()
            .ends_with("folder bytes")
    );
    // A member of Refresh (all) now — it is acquired, not silently absent.
    fs::write(folder.join("SKILL.md"), "---\nname: alpha\n---\nlater").unwrap();
    let batch = refresh_managed_skills(
        &f.paths,
        &f.store,
        RefreshSelection::All,
        RefreshPolicy::default(),
        None,
        4000,
        |_| {},
    )
    .unwrap();
    assert!(batch
        .skills
        .iter()
        .any(|o| o.skill_id == imported.id
            && matches!(o.status, SkillRefreshStatus::Refreshed { .. })));
    assert!(
        fs::read_to_string(Path::new(&after.central_path).join("SKILL.md"))
            .unwrap()
            .ends_with("later")
    );
}

#[test]
fn imported_to_git_stops_being_imported_and_becomes_refreshable() {
    let f = fixture();
    let imported = as_imported(&f);

    let report = repoint_git(
        &f.paths,
        &f.store,
        &imported.id,
        NEW_URL,
        RefreshPolicy::default(),
        None,
        3000,
        &RepointApi,
    )
    .unwrap();

    assert_refreshed(&report);
    let after = f.store.get_skill_by_id(&imported.id).unwrap().unwrap();
    assert_git_source(&after, NEW_URL);
    assert!(
        fs::read_to_string(Path::new(&after.central_path).join("SKILL.md"))
            .unwrap()
            .contains("new bytes")
    );
}

/// The new source is carried only in the acquired record: a Re-point that
/// fails at finalize — here the skill-row write — leaves the git skill's
/// provenance, revision and bytes exactly as they were.
#[test]
fn a_failed_local_repoint_of_a_git_skill_changes_nothing() {
    let f = fixture();
    let before = as_git(&f);
    let folder = new_folder(&f, "folder bytes");
    rusqlite::Connection::open(f.store.db_path())
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER fail_repoint BEFORE INSERT ON skills
            WHEN NEW.source_type != (SELECT source_type FROM skills WHERE id = NEW.id)
            BEGIN SELECT RAISE(ABORT, 'test failed repoint'); END;",
        )
        .unwrap();

    let report = repoint_local(
        &f.paths,
        &f.store,
        &f.skill_id,
        &folder,
        RefreshPolicy::default(),
    )
    .unwrap();

    assert!(
        matches!(report.skills[0].status, SkillRefreshStatus::Failed { .. }),
        "{report:?}"
    );
    let after = record(&f);
    assert_eq!(format!("{after:?}"), format!("{before:?}"));
    assert_eq!(after.source_type, "git");
    assert_eq!(after.source_revision.as_deref(), Some("old-sha"));
    assert_eq!(fs::read_to_string(central(&f).join("a.txt")).unwrap(), "v1");
    assert!(!fs::read_to_string(central(&f).join("SKILL.md"))
        .unwrap()
        .contains("folder bytes"));
}

/// Edit V1 replays inside finalize on a Re-point as on any Update: the
/// operator's invocation choice survives the source crossing to a folder.
#[test]
fn a_git_to_local_repoint_replays_the_invocation_edit_onto_the_new_bytes() {
    let f = fixture();
    as_git(&f);
    crate::core::skill_edits::set_invocation_override(
        &f.paths,
        &f.store,
        &f.skill_id,
        Some(InvocationMode::UserOnly),
    )
    .unwrap();
    let folder = new_folder(&f, "folder bytes");

    let report = repoint_local(
        &f.paths,
        &f.store,
        &f.skill_id,
        &folder,
        RefreshPolicy::default(),
    )
    .unwrap();

    assert!(
        matches!(
            report.skills[0].status,
            SkillRefreshStatus::Refreshed {
                edit_conflict: None,
                ..
            }
        ),
        "{report:?}"
    );
    let text = fs::read_to_string(central(&f).join("SKILL.md")).unwrap();
    assert!(text.ends_with("folder bytes"), "{text}");
    assert_eq!(
        read_invocation_lines(&text).mode(),
        InvocationMode::UserOnly
    );
    assert_local_source(&record(&f), &folder);
    assert!(
        fs::read_to_string(folder.join("SKILL.md"))
            .unwrap()
            .ends_with("---\nfolder bytes"),
        "the operator's folder is never touched"
    );
}
