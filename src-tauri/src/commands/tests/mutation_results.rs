//! Real mutation + wire-result composition, without a Tauri window or operator roots.
use super::*;
use crate::core::{
    installer::install_local_skill,
    propagation::PropagationStatus,
    refresh::{refresh_managed_skills_with, RefreshReport, SkillRefreshStatus},
    repoint::{repoint_skill_source_with, RepointTarget},
    skill_edits::set_invocation_override,
    skill_store::SkillTargetRecord,
    skill_update::UpdateRequest,
};
use std::{fs, path::Path};

struct Fixture {
    _dir: tempfile::TempDir,
    paths: InstallerPaths,
    store: SkillStore,
    id: String,
    source: PathBuf,
    central: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let paths = InstallerPaths {
            home: dir.path().join("home"),
            central_dir: dir.path().join("central"),
            cache_dir: dir.path().join("cache"),
        };
        fs::create_dir_all(&paths.home).unwrap();
        let source = dir.path().join("source");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "---\nname: alpha\n---\nold\n").unwrap();
        let store = SkillStore::new(dir.path().join("test.db"));
        store.ensure_schema().unwrap();
        let installed = install_local_skill(&paths, &store, &source, None).unwrap();
        Self {
            _dir: dir,
            paths,
            store,
            id: installed.skill_id,
            source,
            central: installed.central_path,
        }
    }
    fn update(&self, reassert: bool) -> RefreshReport {
        refresh_managed_skills_core(
            &self.paths,
            &self.store,
            RefreshSelection::Ids(vec![self.id.clone()]),
            RefreshPolicy {
                reassert_auto_sync: reassert,
            },
            None,
            5000,
            |_| {},
        )
        .unwrap()
    }
    fn result(&self, report: RefreshReport) -> SkillMutationResultDto {
        SkillMutationResultDto::from_report(&self.store, report).unwrap()
    }
    fn blocked_target(&self) {
        let tool = crate::core::tool_adapters::adapter_by_key("cursor").unwrap();
        crate::core::tool_adapters::mark_installed_in(&self.paths.home, tool);
        let blocker = self.paths.home.join("blocked");
        fs::write(&blocker, "not a directory").unwrap();
        self.store
            .upsert_skill_target(&SkillTargetRecord {
                id: "blocked".into(),
                skill_id: self.id.clone(),
                tool: "cursor".into(),
                target_path: blocker.join("skill").to_string_lossy().into_owned(),
                mode: SyncMode::Copy,
                status: SyncStatus::Synced,
                last_error: None,
                synced_at: Some(1),
            })
            .unwrap();
    }
}

#[test]
fn update_and_restore_catalog_include_replayed_override_and_reasserted_targets() {
    for restore in [false, true] {
        let f = Fixture::new();
        set_invocation_override(&f.paths, &f.store, &f.id, Some(InvocationMode::UserOnly)).unwrap();
        let tool = crate::core::tool_adapters::adapter_by_key("claude_code").unwrap();
        crate::core::tool_adapters::mark_installed_in(&f.paths.home, tool);
        fs::write(
            f.source.join("SKILL.md"),
            "---\nname: alpha\ndescription: new description\n---\nnew\n",
        )
        .unwrap();
        if restore {
            fs::remove_dir_all(&f.central).unwrap();
        }
        let result = f.result(f.update(true));
        assert!(
            matches!(result.report.skills.as_slice(), [outcome] if matches!(outcome.status, SkillRefreshStatus::Refreshed { .. }))
        );
        let row = &result.skills[0];
        assert_eq!(row.id, f.id);
        assert_eq!(row.description.as_deref(), Some("new description"));
        assert_eq!(row.invocation_mode, InvocationMode::UserOnly);
        assert_eq!(
            row.invocation_override.as_ref().unwrap().mode,
            InvocationMode::UserOnly
        );
        assert_eq!(row.unlocatable, None);
        assert!(row.refreshable);
        assert!(row.detachable);
        assert_eq!(row.targets.len(), 1);
        assert_eq!(row.targets[0].tool, "claude_code");
        assert_eq!(row.targets[0].status, SyncStatus::Synced);
    }
}

#[test]
fn update_failure_returns_full_catalog_not_just_report_members() {
    let f = Fixture::new();
    let other = install_local_skill(&f.paths, &f.store, &f.source, Some("beta".into())).unwrap();
    fs::remove_dir_all(&f.source).unwrap();
    let result = f.result(f.update(false));
    assert!(
        matches!(result.report.skills.as_slice(), [outcome] if matches!(outcome.status, SkillRefreshStatus::Failed { .. }))
    );
    assert_eq!(result.skills.len(), 2);
    assert!(result.skills.iter().any(|s| s.id == other.skill_id));
    for row in result.skills {
        assert_eq!(row.unlocatable, Some(UnlocatableState::SourceMissing));
        assert!(row.detachable);
    }
}

#[test]
fn local_repoint_catalog_has_new_source_and_settled_target_failure() {
    let f = Fixture::new();
    f.blocked_target();
    let new_source = f.paths.home.join("new-source");
    fs::rename(&f.source, &new_source).unwrap();
    let report = repoint_skill_source_with(
        &f.paths,
        &f.store,
        &f.id,
        RepointTarget::Local {
            path: new_source.to_string_lossy().into_owned(),
        },
        RefreshPolicy::default(),
        None,
        5000,
        |_| {},
        &RepointApi,
    )
    .unwrap();
    let result = f.result(report);
    assert!(
        matches!(result.report.skills.as_slice(), [outcome] if matches!(&outcome.status,
            SkillRefreshStatus::Refreshed { targets, reassert_error: None, .. }
            if matches!(targets.as_slice(), [target] if matches!(target.status, PropagationStatus::Failed { .. }))
        ))
    );
    assert_eq!(result.skills[0].source_ref.as_deref(), new_source.to_str());
    assert_eq!(result.skills[0].unlocatable, None);
    assert_eq!(result.skills[0].targets[0].status, SyncStatus::Error);
}

struct RepointApi;
impl crate::core::git_acquisition::GithubApi for RepointApi {
    fn matching_refs(
        &self,
        _: &crate::core::git_acquisition::GithubRepo,
        _: &str,
    ) -> anyhow::Result<Vec<String>> {
        Ok(vec!["main".into()])
    }
    fn branch_sha(&self, _: &crate::core::git_acquisition::GithubCoords) -> anyhow::Result<String> {
        Ok("new-sha".into())
    }
    fn download_directory(
        &self,
        _: &crate::core::git_acquisition::GithubCoords,
        dest: &Path,
        _: Option<&CancelToken>,
    ) -> anyhow::Result<()> {
        fs::create_dir_all(dest)?;
        fs::write(
            dest.join("SKILL.md"),
            "---\nname: alpha\ndescription: git bytes\n---\n",
        )?;
        Ok(())
    }
}

#[test]
fn git_repoint_catalog_has_replaced_source_and_repaired_central() {
    let f = Fixture::new();
    let mut row = f.store.get_skill_by_id(&f.id).unwrap().unwrap();
    row.source_type = "git".into();
    row.source_ref = Some("https://github.com/old/repo/tree/main/skills/old".into());
    row.source_subpath = Some("skills/old".into());
    f.store.upsert_skill(&row).unwrap();
    fs::remove_dir_all(&f.central).unwrap();
    let url = "https://github.com/new/repo/tree/main/skills/alpha";
    let report = repoint_skill_source_with(
        &f.paths,
        &f.store,
        &f.id,
        RepointTarget::Git { url: url.into() },
        RefreshPolicy::default(),
        None,
        5000,
        |_| {},
        &RepointApi,
    )
    .unwrap();
    let result = f.result(report);
    assert!(
        matches!(result.report.skills.as_slice(), [outcome] if matches!(outcome.status, SkillRefreshStatus::Refreshed { .. }))
    );
    assert_eq!(result.skills[0].source_ref.as_deref(), Some(url));
    assert_eq!(result.skills[0].description.as_deref(), Some("git bytes"));
    assert_eq!(result.skills[0].unlocatable, None);
    assert!(!result.skills[0].detachable);
}

#[test]
fn edit_catalog_is_post_propagation_and_includes_unrelated_rows() {
    let f = Fixture::new();
    f.blocked_target();
    let other = install_local_skill(&f.paths, &f.store, &f.source, Some("beta".into())).unwrap();
    let outcome =
        set_invocation_override(&f.paths, &f.store, &f.id, Some(InvocationMode::UserOnly)).unwrap();
    // Changed after the operation: result composition must read fresh, not reuse entry.
    let mut beta = f.store.get_skill_by_id(&other.skill_id).unwrap().unwrap();
    beta.description = Some("changed elsewhere".into());
    f.store.upsert_skill(&beta).unwrap();
    let result = InvocationEditResultDto::from_outcome(&f.store, outcome).unwrap();
    assert_eq!(result.report.skill_id, f.id);
    assert!(matches!(
        result.report.propagation.targets[0].status,
        PropagationStatus::Failed { .. }
    ));
    let alpha = result.skills.iter().find(|s| s.id == f.id).unwrap();
    assert_eq!(alpha.invocation_mode, InvocationMode::UserOnly);
    assert_eq!(
        alpha.invocation_override.as_ref().unwrap().mode,
        InvocationMode::UserOnly
    );
    assert_eq!(alpha.targets[0].status, SyncStatus::Error);
    assert_eq!(result.skills.len(), 2);
    assert_eq!(
        result
            .skills
            .iter()
            .find(|s| s.id == other.skill_id)
            .unwrap()
            .description
            .as_deref(),
        Some("changed elsewhere")
    );
}

#[test]
fn acquisition_skips_return_current_catalog_and_wire_reason() {
    for gone in [false, true] {
        let f = Fixture::new();
        let report = refresh_managed_skills_with(
            &f.paths,
            &f.store,
            RefreshSelection::Ids(vec![f.id.clone()]),
            RefreshPolicy::default(),
            None,
            5000,
            |_| {},
            &|id, _| {
                let mut row = f.store.get_skill_by_id(id)?.unwrap();
                let request = UpdateRequest::local(row.clone(), &f.source, false)?;
                if gone {
                    f.store.delete_skill(id)?;
                } else {
                    row.source_ref = Some(
                        f.paths
                            .home
                            .join("new-source")
                            .to_string_lossy()
                            .into_owned(),
                    );
                    row.description = Some("current row".into());
                    f.store.upsert_skill(&row)?;
                }
                Ok(request)
            },
        )
        .unwrap();
        let result = f.result(report);
        assert!(
            matches!(result.report.skills.as_slice(), [outcome] if matches!(outcome.status, SkillRefreshStatus::SkippedAcquisition { .. }))
        );
        let wire = serde_json::to_value(&result).unwrap();
        assert_eq!(
            wire["report"]["skills"][0]["status"],
            serde_json::json!({
                "status": "skipped_acquisition", "reason": if gone { "skill_gone" } else { "stale_acquisition" }
            })
        );
        if gone {
            assert!(result.skills.is_empty());
        } else {
            assert_eq!(result.skills.len(), 1);
            assert_eq!(result.skills[0].description.as_deref(), Some("current row"));
            assert_eq!(
                result.skills[0].source_ref.as_deref(),
                f.paths.home.join("new-source").to_str()
            );
        }
    }
}

#[test]
fn catalog_failure_after_settlement_is_an_error_never_an_empty_success() {
    for edit in [false, true] {
        let f = Fixture::new();
        let outcome =
            set_invocation_override(&f.paths, &f.store, &f.id, Some(InvocationMode::UserOnly))
                .unwrap();
        let report = f.update(false);
        rusqlite::Connection::open(f.store.db_path())
            .unwrap()
            .execute_batch("PRAGMA foreign_keys = OFF; DROP TABLE skill_targets;")
            .unwrap();
        let error = if edit {
            InvocationEditResultDto::from_outcome(&f.store, outcome).unwrap_err()
        } else {
            SkillMutationResultDto::from_report(&f.store, report).unwrap_err()
        };
        let error = CommandError::from_anyhow(error);
        assert!(matches!(error, CommandError::Other { .. }));
        assert!(error.to_string().contains("list sync targets"));
    }
}
