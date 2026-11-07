//! The occupant check (ADR-0007 rule 1), at the filesystem level. The seams
//! that call it are pinned in their own modules' tests (artifact removal,
//! project sync, Propagation, the global batch, Disable / Delete).

use std::fs;
use std::path::Path;

use super::{check, Deployed, Occupancy};
use crate::core::content_identity::{read, Source};

fn skill_dir(at: &Path, body: &str) {
    fs::create_dir_all(at).unwrap();
    fs::write(at.join("SKILL.md"), body).unwrap();
}

fn identity(at: &Path) -> String {
    read(Source::Directory(at)).unwrap()
}

#[test]
fn an_absent_path_is_absent_whatever_the_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("skills/alpha");
    assert_eq!(check(&path, &Deployed::none()).unwrap(), Occupancy::Absent);
    let library = dir.path().join("library/alpha");
    let deployed = Deployed::recorded(true, Some(&library), Some("hash"));
    assert_eq!(check(&path, &deployed).unwrap(), Occupancy::Absent);
}

#[test]
fn a_row_without_a_claim_owns_nothing_even_an_identical_copy() {
    let dir = tempfile::tempdir().unwrap();
    let library = dir.path().join("library/alpha");
    skill_dir(&library, "v1");
    let path = dir.path().join("skills/alpha");
    skill_dir(&path, "v1");
    let hash = identity(&path);
    let unclaimed = Deployed::recorded(false, Some(&library), Some(&hash));
    assert_eq!(check(&path, &unclaimed).unwrap(), Occupancy::Foreign);
}

#[test]
fn a_copy_is_ours_only_while_it_holds_the_recorded_identity() {
    let dir = tempfile::tempdir().unwrap();
    let library = dir.path().join("library/alpha");
    skill_dir(&library, "v2");
    let path = dir.path().join("skills/alpha");
    skill_dir(&path, "v1");
    // Deployed v1; the Library copy moved on to v2 since (a stale copy).
    let deployed = Deployed::recorded(true, Some(&library), Some(&identity(&path)));
    assert_eq!(check(&path, &deployed).unwrap(), Occupancy::Ours);

    // Edited in place: no longer what was deployed.
    fs::write(path.join("SKILL.md"), "v1, edited").unwrap();
    assert_eq!(check(&path, &deployed).unwrap(), Occupancy::Foreign);
}

#[test]
fn a_copy_without_a_recorded_identity_is_not_provably_ours() {
    let dir = tempfile::tempdir().unwrap();
    let library = dir.path().join("library/alpha");
    skill_dir(&library, "v1");
    let path = dir.path().join("skills/alpha");
    skill_dir(&path, "v1");
    // A pre-1.4.0 global row: a claim, no identity.
    let deployed = Deployed::recorded(true, Some(&library), None);
    assert_eq!(check(&path, &deployed).unwrap(), Occupancy::Foreign);
}

#[test]
fn a_file_is_never_ours() {
    let dir = tempfile::tempdir().unwrap();
    let library = dir.path().join("library/alpha");
    skill_dir(&library, "v1");
    let path = dir.path().join("skills/alpha");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "a file").unwrap();
    let deployed = Deployed::recorded(true, Some(&library), Some("any"));
    assert_eq!(check(&path, &deployed).unwrap(), Occupancy::Foreign);
}

#[cfg(unix)]
mod links {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn a_link_to_the_library_copy_is_ours() {
        let dir = tempfile::tempdir().unwrap();
        let library = dir.path().join("library/alpha");
        skill_dir(&library, "v1");
        let path = dir.path().join("skills/alpha");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        symlink(&library, &path).unwrap();
        let deployed = Deployed::recorded(true, Some(&library), None);
        assert_eq!(check(&path, &deployed).unwrap(), Occupancy::Ours);
    }

    #[test]
    fn a_link_reaching_the_copy_through_another_spelling_is_ours_by_identity() {
        let dir = tempfile::tempdir().unwrap();
        let library = dir.path().join("library/alpha");
        skill_dir(&library, "v1");
        let alias = dir.path().join("alias");
        symlink(dir.path().join("library"), &alias).unwrap();
        let path = dir.path().join("skills/alpha");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        symlink(alias.join("alpha"), &path).unwrap();
        let deployed = Deployed::recorded(true, Some(&library), None);
        assert_eq!(check(&path, &deployed).unwrap(), Occupancy::Ours);
    }

    #[test]
    fn a_link_elsewhere_is_foreign_even_to_identical_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let library = dir.path().join("library/alpha");
        skill_dir(&library, "v1");
        let elsewhere = dir.path().join("elsewhere/alpha");
        skill_dir(&elsewhere, "v1");
        let path = dir.path().join("skills/alpha");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        symlink(&elsewhere, &path).unwrap();
        let hash = identity(&library);
        let deployed = Deployed::recorded(true, Some(&library), Some(&hash));
        assert_eq!(check(&path, &deployed).unwrap(), Occupancy::Foreign);
    }

    #[test]
    fn a_dangling_link_is_ours_only_when_it_spells_the_library_copy() {
        let dir = tempfile::tempdir().unwrap();
        let library = dir.path().join("library/alpha");
        let path = dir.path().join("skills/alpha");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        // The Library copy is gone (an Unlocatable skill): the link the app
        // wrote still names it exactly.
        symlink(&library, &path).unwrap();
        let deployed = Deployed::recorded(true, Some(&library), None);
        assert_eq!(check(&path, &deployed).unwrap(), Occupancy::Ours);

        fs::remove_file(&path).unwrap();
        symlink(dir.path().join("gone/alpha"), &path).unwrap();
        assert_eq!(check(&path, &deployed).unwrap(), Occupancy::Foreign);
    }

    #[test]
    fn a_link_is_foreign_when_the_skill_record_is_gone() {
        let dir = tempfile::tempdir().unwrap();
        let library = dir.path().join("library/alpha");
        skill_dir(&library, "v1");
        let path = dir.path().join("skills/alpha");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        symlink(&library, &path).unwrap();
        let deployed = Deployed::recorded(true, None, None);
        assert_eq!(check(&path, &deployed).unwrap(), Occupancy::Foreign);
    }
}

// ---------------------------------------------------------------------------
// The rule at every destructive seam (core integration, both scopes): a
// Deployment's folder is removed or replaced only while it is still what the
// row deployed (ADR-0007; round 20 t09 review round 3: Astra 10 and 11,
// Sol 102).
// ---------------------------------------------------------------------------

mod seams {
    use std::fs;
    use std::path::{Path, PathBuf};

    use crate::core::artifact_removal::{remove_skill, RemovalTargetStatus};
    use crate::core::content_identity::{self, Source};
    use crate::core::environment::HostRoots;
    use crate::core::errors::{CommandError, RemovalRefusal};
    use crate::core::global_sync::{
        sync_skills_to_tools, BatchPolicy, BatchSkill, BatchTargetOutcome, BatchTargetStatus,
        OverrideDecision,
    };
    use crate::core::installer::{install_local_skill, InstallerPaths};
    use crate::core::project_sync::{
        assign_skill_to_project_tools, resolve_collisions, resync_project, AssignmentDecision,
        ProjectSyncOutcomeStatus, ProjectSyncReport,
    };
    use crate::core::propagation::{
        propagate_unlocked, resolve_target_collisions, PropagationSkip, PropagationStatus,
        TargetDecision,
    };
    use crate::core::removal_fence::central_repo_of;
    use crate::core::skill_store::{
        ProjectRecord, ProjectToolRecord, SkillStore, SkillTargetRecord,
    };
    use crate::core::sync_status::SyncStatus;
    use crate::core::tool_adapters::{adapter_by_key, mark_installed_in};

    const NOW: i64 = 5000;

    struct Fixture {
        _dir: tempfile::TempDir,
        paths: InstallerPaths,
        store: SkillStore,
        _source: tempfile::TempDir,
        skill_id: String,
        project: ProjectRecord,
    }

    /// One Managed skill `alpha`, `tool` installed, and a project with `tool`
    /// configured.
    fn fixture(tool: &str) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let paths = InstallerPaths {
            roots: HostRoots::under_home(&dir.path().join("home")),
            central_dir: dir.path().join("central"),
            cache_dir: dir.path().join("cache"),
        };
        fs::create_dir_all(&paths.roots.home).unwrap();
        mark_installed_in(&paths.roots, adapter_by_key(tool).unwrap());
        let store = SkillStore::new(dir.path().join("test.db"));
        store.ensure_schema().unwrap();
        let source = tempfile::tempdir().unwrap();
        fs::write(
            source.path().join("SKILL.md"),
            "---\nname: alpha\n---\nv1\n",
        )
        .unwrap();
        let skill_id = install_local_skill(&paths, &store, source.path(), Some("alpha".into()))
            .unwrap()
            .skill_id;
        let project_path = dir.path().join("proj");
        fs::create_dir_all(&project_path).unwrap();
        let project = ProjectRecord {
            id: "p1".into(),
            path: project_path.to_string_lossy().into_owned(),
            created_at: 1,
            updated_at: 1,
        };
        store.register_project(&project).unwrap();
        store
            .add_project_tool(&ProjectToolRecord {
                id: format!("pt-{tool}"),
                project_id: "p1".into(),
                tool: tool.into(),
            })
            .unwrap();
        Fixture {
            _dir: dir,
            paths,
            store,
            _source: source,
            skill_id,
            project,
        }
    }

    impl Fixture {
        fn central(&self) -> PathBuf {
            PathBuf::from(
                self.store
                    .get_skill_by_id(&self.skill_id)
                    .unwrap()
                    .unwrap()
                    .central_path,
            )
        }
        fn global(&self, rel: &str) -> PathBuf {
            self.paths.roots.home.join(rel).join("alpha")
        }
        fn assigned(&self, rel: &str) -> PathBuf {
            Path::new(&self.project.path).join(rel).join("alpha")
        }
        fn sync(&self, tool: &str) -> Vec<BatchTargetOutcome> {
            sync_skills_to_tools(
                &self.paths.roots,
                &self.store,
                &central_repo_of(&self.store),
                &[BatchSkill {
                    skill_id: self.skill_id.clone(),
                    skill_name: "alpha".into(),
                    source_path: self.central(),
                }],
                &[tool.to_string()],
                &BatchPolicy {
                    overwrite: false,
                    overwrite_if_same_content: true,
                    overrides: Vec::new(),
                },
                NOW,
                |_| {},
            )
        }
        fn assign(&self) -> ProjectSyncReport {
            assign_skill_to_project_tools(
                &self.store,
                &central_repo_of(&self.store),
                "p1",
                &self.skill_id,
                NOW,
            )
            .unwrap()
        }
        fn resync(&self) -> ProjectSyncReport {
            resync_project(&self.store, &central_repo_of(&self.store), "p1", NOW).unwrap()
        }
        /// The Library copy moves on (an Update): new bytes and their identity.
        fn update_library(&self) {
            fs::write(
                self.central().join("SKILL.md"),
                "---\nname: alpha\n---\nv2\n",
            )
            .unwrap();
            let hash = content_identity::read(Source::Directory(&self.central())).unwrap();
            self.store
                .update_skill_content_hash(&self.skill_id, &hash)
                .unwrap();
        }
        fn propagate(&self) -> Vec<(String, PropagationStatus)> {
            crate::core::mutation_guard::serialized(|| {
                propagate_unlocked(&self.store, &self.paths, &self.skill_id, NOW)
            })
            .unwrap()
            .targets
            .into_iter()
            .map(|t| (t.folder_key, t.status))
            .collect()
        }
        fn delete(&self) -> crate::core::artifact_removal::RemovalReport {
            remove_skill(
                &self.store,
                &central_repo_of(&self.store),
                &self.paths.roots,
                &self.skill_id,
            )
            .unwrap()
        }
        /// Answer Propagation's global ask for the row keyed `tool`, naming
        /// the path the ask showed.
        fn answer_global(
            &self,
            tool: &str,
            path: &Path,
            decision: OverrideDecision,
        ) -> Vec<BatchTargetOutcome> {
            resolve_target_collisions(
                &self.store,
                &self.paths,
                &[TargetDecision {
                    skill_id: self.skill_id.clone(),
                    tool: tool.into(),
                    path: path.to_string_lossy().into_owned(),
                    decision,
                }],
                NOW,
            )
            .unwrap()
        }
        fn answer_project(&self, tool: &str, decision: OverrideDecision) -> ProjectSyncReport {
            resolve_collisions(
                &self.store,
                &central_repo_of(&self.store),
                &[AssignmentDecision {
                    project_id: "p1".into(),
                    skill_id: self.skill_id.clone(),
                    tool: tool.into(),
                    decision,
                }],
                NOW,
            )
            .unwrap()
        }
        fn target(&self, tool: &str) -> SkillTargetRecord {
            self.store
                .get_skill_target(&self.skill_id, tool)
                .unwrap()
                .unwrap()
        }
        /// Point the global row keyed `tool` at `path` (a stored record the
        /// registry would not write today).
        fn record_target_at(&self, tool: &str, path: &Path) {
            let mut row = self.target(tool);
            row.target_path = path.to_string_lossy().into_owned();
            self.store.upsert_skill_target(&row).unwrap();
        }
        fn global_status(&self, tool: &str) -> Option<SyncStatus> {
            self.store
                .get_skill_target(&self.skill_id, tool)
                .unwrap()
                .map(|row| row.status)
        }
        fn assignment(&self, tool: &str) -> crate::core::skill_store::ProjectSkillAssignmentRecord {
            self.store
                .get_project_skill_assignment("p1", &self.skill_id, tool)
                .unwrap()
                .unwrap()
        }
    }

    /// The operator's folder, put where the app's artifact was.
    fn replace_with_foreign_folder(path: &Path) {
        if path
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
        {
            #[cfg(windows)]
            fs::remove_dir(path).unwrap();
            #[cfg(not(windows))]
            fs::remove_file(path).unwrap();
        } else if path.exists() {
            fs::remove_dir_all(path).unwrap();
        }
        fs::create_dir_all(path).unwrap();
        fs::write(path.join("SKILL.md"), "the operator's own").unwrap();
    }

    fn is_foreign(path: &Path) -> bool {
        fs::read_to_string(path.join("SKILL.md")).is_ok_and(|text| text == "the operator's own")
    }

    fn is_link(path: &Path) -> bool {
        path.symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
    }

    fn asked(status: &ProjectSyncOutcomeStatus) -> bool {
        matches!(
            status,
            ProjectSyncOutcomeStatus::Failed {
                error: CommandError::TargetExists { .. }
            }
        )
    }

    fn batch_asked(outcomes: &[BatchTargetOutcome]) -> bool {
        matches!(
            outcomes,
            [outcome] if matches!(
                outcome.status,
                BatchTargetStatus::Failed { error: CommandError::TargetExists { .. } }
            )
        )
    }

    fn propagation_asked(outcomes: &[(String, PropagationStatus)]) -> bool {
        matches!(
            outcomes,
            [(
                _,
                PropagationStatus::Failed {
                    error: CommandError::TargetExists { .. },
                    ..
                }
            )]
        )
    }

    // -- A link still pointing at the Library copy: removed or replaced as before

    #[test]
    fn a_link_still_resolving_to_the_library_copy_is_resynced_and_removed() {
        let f = fixture("claude_code");
        assert!(matches!(
            f.sync("claude_code").as_slice(),
            [o] if matches!(o.status, BatchTargetStatus::Synced { .. })
        ));
        f.assign();
        let global = f.global(".claude/skills");
        let assigned = f.assigned(".claude/skills");
        if cfg!(unix) {
            assert!(is_link(&global) && is_link(&assigned));
        }

        let resync = f.resync();
        assert!(
            matches!(
                resync.items.as_slice(),
                [item] if matches!(item.status, ProjectSyncOutcomeStatus::Synced)
            ),
            "{resync:?}"
        );

        let report = f.delete();
        assert!(report.record_deleted, "{report}");
        assert_eq!(report.removed_rows(), 2, "{report}");
        assert!(
            global.symlink_metadata().is_err(),
            "the global link is removed"
        );
        assert!(
            assigned.symlink_metadata().is_err(),
            "the project link is removed"
        );
    }

    // -- An owned link replaced by a foreign folder: an ask, never a replacement

    #[test]
    fn an_owned_project_link_replaced_by_a_foreign_folder_goes_to_the_ask_on_resync() {
        let f = fixture("claude_code");
        f.assign();
        let assigned = f.assigned(".claude/skills");
        replace_with_foreign_folder(&assigned);

        let report = f.resync();

        assert!(
            matches!(report.items.as_slice(), [item] if asked(&item.status)),
            "an ask, never Synced: {report:?}"
        );
        assert!(is_foreign(&assigned), "the operator's folder survives");
        let row = f.assignment("claude_code");
        assert_eq!(row.status, SyncStatus::Error);
        assert!(row.owns_artifact(), "the collision keeps the row's claim");

        // Delete before answering: the folder is not the row's, so it stays.
        let deleted = f.delete();
        assert!(deleted.record_deleted, "{deleted}");
        assert!(matches!(
            deleted.targets.as_slice(),
            [target] if matches!(target.status, RemovalTargetStatus::LeftInPlace)
        ));
        assert!(is_foreign(&assigned));
    }

    /// The ask's answers apply to a claiming row whose path another folder
    /// took: Keep gives the path up and leaves the folder; Overwrite replaces
    /// it with the app's link again.
    #[test]
    fn the_ask_raised_over_a_claiming_row_is_answered_for_that_row() {
        for decision in [OverrideDecision::Keep, OverrideDecision::Overwrite] {
            let f = fixture("claude_code");
            f.assign();
            let assigned = f.assigned(".claude/skills");
            replace_with_foreign_folder(&assigned);
            assert!(asked(&f.resync().items[0].status));

            let answer = resolve_collisions(
                &f.store,
                &central_repo_of(&f.store),
                &[AssignmentDecision {
                    project_id: "p1".into(),
                    skill_id: f.skill_id.clone(),
                    tool: "claude_code".into(),
                    decision,
                }],
                NOW,
            )
            .unwrap();

            let row = f.assignment("claude_code");
            match decision {
                OverrideDecision::Keep => {
                    assert!(matches!(
                        answer.items[0].status,
                        ProjectSyncOutcomeStatus::Declined
                    ));
                    assert!(is_foreign(&assigned), "kept");
                    assert_eq!(row.status, SyncStatus::Disabled);
                    assert!(!row.owns_artifact(), "Keep gives the path up");
                    let deleted = f.delete();
                    assert!(deleted.targets.is_empty(), "{deleted}");
                    assert!(is_foreign(&assigned), "Delete never reaches it");
                }
                OverrideDecision::Overwrite => {
                    assert!(
                        matches!(answer.items[0].status, ProjectSyncOutcomeStatus::Synced),
                        "{answer:?}"
                    );
                    assert!(!is_foreign(&assigned), "replaced on the operator's answer");
                    assert_eq!(row.status, SyncStatus::Synced);
                }
            }
        }
    }

    #[test]
    fn an_owned_global_link_replaced_by_a_foreign_folder_goes_to_the_ask_on_sync() {
        let f = fixture("claude_code");
        f.sync("claude_code");
        let global = f.global(".claude/skills");
        replace_with_foreign_folder(&global);

        let again = f.sync("claude_code");

        assert!(batch_asked(&again), "{again:?}");
        assert!(is_foreign(&global));
        let deleted = f.delete();
        assert!(matches!(
            deleted.targets.as_slice(),
            [target] if matches!(target.status, RemovalTargetStatus::LeftInPlace)
        ));
        assert!(is_foreign(&global), "Delete leaves it in place");
    }

    /// A link row follows the source only while its link is there: a folder
    /// that took the place of an owned link is asked about through
    /// Propagation at both scopes — never skipped as `LinkFollowsSource`,
    /// never reported Synced — and stays (round 20 t18 review: Astra 1 =
    /// Sol 102).
    #[test]
    fn propagation_asks_about_a_folder_that_replaced_an_owned_link_at_both_scopes() {
        let f = fixture("claude_code");
        f.sync("claude_code");
        f.assign();
        let global = f.global(".claude/skills");
        let assigned = f.assigned(".claude/skills");
        replace_with_foreign_folder(&global);
        replace_with_foreign_folder(&assigned);
        f.update_library();

        let outcomes = f.propagate();

        assert_eq!(outcomes.len(), 2, "{outcomes:?}");
        for (_, status) in &outcomes {
            assert!(
                matches!(
                    status,
                    PropagationStatus::Failed {
                        error: CommandError::TargetExists { .. },
                        first_deploy: false,
                    }
                ),
                "TARGET_EXISTS at both scopes: {outcomes:?}"
            );
        }
        assert!(is_foreign(&global) && is_foreign(&assigned));
        assert!(
            f.target("claude_code").owns_artifact(),
            "the ask keeps the claim"
        );
    }

    /// The answers to that ask, at both scopes. Keep leaves the folder and
    /// gives the row's path up; Overwrite replaces it with the app's link, and
    /// the next Propagation finds the link its own — no second ask.
    #[test]
    fn a_replaced_link_asked_about_through_propagation_settles_by_the_answer() {
        for decision in [OverrideDecision::Keep, OverrideDecision::Overwrite] {
            let f = fixture("claude_code");
            f.sync("claude_code");
            f.assign();
            let global = f.global(".claude/skills");
            let assigned = f.assigned(".claude/skills");
            replace_with_foreign_folder(&global);
            replace_with_foreign_folder(&assigned);
            f.update_library();
            assert!(f
                .propagate()
                .iter()
                .all(|(_, s)| matches!(s, PropagationStatus::Failed { .. })));

            let global_answer = f.answer_global("claude_code", &global, decision);
            let project_answer = f.answer_project("claude_code", decision);

            match decision {
                OverrideDecision::Keep => {
                    assert!(
                        matches!(global_answer[0].status, BatchTargetStatus::Declined),
                        "{global_answer:?}"
                    );
                    assert!(matches!(
                        project_answer.items[0].status,
                        ProjectSyncOutcomeStatus::Declined
                    ));
                    assert!(is_foreign(&global) && is_foreign(&assigned), "kept");
                    let row = f.target("claude_code");
                    assert_eq!(row.status, SyncStatus::Disabled);
                    assert!(!row.owns_artifact(), "Keep gives the path up");
                    assert_eq!(Path::new(&row.target_path), global, "where it was asked");
                    let again = f.propagate();
                    assert!(
                        again.iter().all(|(_, s)| matches!(
                            s,
                            PropagationStatus::Skipped {
                                reason: PropagationSkip::Disabled
                            }
                        )),
                        "{again:?}"
                    );
                    let deleted = f.delete();
                    assert!(deleted.targets.is_empty(), "{deleted}");
                    assert!(is_foreign(&global) && is_foreign(&assigned));
                }
                OverrideDecision::Overwrite => {
                    assert!(
                        matches!(global_answer[0].status, BatchTargetStatus::Synced { .. }),
                        "{global_answer:?}"
                    );
                    assert!(matches!(
                        project_answer.items[0].status,
                        ProjectSyncOutcomeStatus::Synced
                    ));
                    assert!(!is_foreign(&global) && !is_foreign(&assigned));
                    assert_eq!(f.target("claude_code").status, SyncStatus::Synced);
                    f.update_library();
                    let again = f.propagate();
                    assert!(
                        again.iter().all(|(_, s)| matches!(
                            s,
                            PropagationStatus::Skipped {
                                reason: PropagationSkip::LinkFollowsSource
                            }
                        )),
                        "no second ask: {again:?}"
                    );
                }
            }
        }
    }

    /// Round 20 t18 review, Sol 101: a global Propagation ask names the row's
    /// recorded path; its answer applies there and nowhere else. A copy row
    /// records `alpha`, the skill is now `alpha-renamed`, and folders the app
    /// did not write sit at both paths. Overwrite replaces the asked folder
    /// only — the one at the batch's destination survives, and the row stays
    /// where it was asked about. The next Propagation finds the copy its own.
    #[test]
    fn a_propagation_overwrite_replaces_only_the_asked_path() {
        let f = fixture("deep_agents_code");
        f.sync("deep_agents_code");
        let mut renamed = f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap();
        renamed.name = "alpha-renamed".into();
        f.store.upsert_skill(&renamed).unwrap();
        let recorded = f.global(".deepagents/agent/skills");
        let destination = recorded.with_file_name("alpha-renamed");
        replace_with_foreign_folder(&recorded);
        replace_with_foreign_folder(&destination);
        f.update_library();

        let outcomes = f.propagate();
        let [(
            _,
            PropagationStatus::Failed {
                error: CommandError::TargetExists { path, .. },
                ..
            },
        )] = outcomes.as_slice()
        else {
            panic!("one ask: {outcomes:?}");
        };
        assert_eq!(Path::new(path), recorded, "the ask names the recorded path");

        let answer = f.answer_global(
            "deep_agents_code",
            Path::new(path),
            OverrideDecision::Overwrite,
        );

        assert!(
            matches!(answer[0].status, BatchTargetStatus::Synced { .. }),
            "{answer:?}"
        );
        assert!(
            fs::read_to_string(recorded.join("SKILL.md"))
                .unwrap()
                .contains("v2"),
            "the asked folder is replaced"
        );
        assert!(is_foreign(&destination), "the unasked folder survives");
        let row = f.target("deep_agents_code");
        assert_eq!(
            Path::new(&row.target_path),
            recorded,
            "the row stays where asked"
        );
        assert_eq!(row.status, SyncStatus::Synced);
        assert_eq!(
            row.content_hash,
            content_identity::read(Source::Directory(&f.central())),
            "the row records what it deployed"
        );

        let again = f.propagate();
        assert!(
            matches!(again.as_slice(), [(_, PropagationStatus::Synced { .. })]),
            "no second ask: {again:?}"
        );
        assert!(is_foreign(&destination));
    }

    /// Keep leaves both folders, records the row `disabled` at its recorded
    /// path with its claim given up, and nothing at the destination is
    /// touched or claimed.
    #[test]
    fn a_propagation_keep_disables_the_asked_row_where_it_is() {
        let f = fixture("deep_agents_code");
        f.sync("deep_agents_code");
        let mut renamed = f.store.get_skill_by_id(&f.skill_id).unwrap().unwrap();
        renamed.name = "alpha-renamed".into();
        f.store.upsert_skill(&renamed).unwrap();
        let recorded = f.global(".deepagents/agent/skills");
        let destination = recorded.with_file_name("alpha-renamed");
        replace_with_foreign_folder(&recorded);
        replace_with_foreign_folder(&destination);
        f.update_library();
        assert!(propagation_asked(&f.propagate()));

        let answer = f.answer_global("deep_agents_code", &recorded, OverrideDecision::Keep);

        assert!(
            matches!(answer[0].status, BatchTargetStatus::Declined),
            "{answer:?}"
        );
        assert!(is_foreign(&recorded) && is_foreign(&destination));
        let row = f.target("deep_agents_code");
        assert_eq!(Path::new(&row.target_path), recorded);
        assert_eq!(row.status, SyncStatus::Disabled);
        assert!(!row.owns_artifact());
        assert_eq!(f.store.list_skill_targets(&f.skill_id).unwrap().len(), 1);
    }

    /// An answer is checked against the row now: a row that no longer records
    /// the asked path (it moved since the ask) is not answered — nothing is
    /// touched, the row is unchanged.
    #[test]
    fn an_answer_for_a_path_the_row_no_longer_records_touches_nothing() {
        let f = fixture("deep_agents_code");
        f.sync("deep_agents_code");
        let recorded = f.global(".deepagents/agent/skills");
        let elsewhere = recorded.with_file_name("alpha-renamed");
        replace_with_foreign_folder(&recorded);
        replace_with_foreign_folder(&elsewhere);
        f.update_library();
        assert!(propagation_asked(&f.propagate()));
        let before = f.target("deep_agents_code");

        for decision in [OverrideDecision::Overwrite, OverrideDecision::Keep] {
            let answer = f.answer_global("deep_agents_code", &elsewhere, decision);
            assert!(
                matches!(
                    &answer[0].status,
                    BatchTargetStatus::Failed { error: CommandError::NotFound { kind, .. } }
                        if kind == "target"
                ),
                "{answer:?}"
            );
        }
        assert!(is_foreign(&recorded) && is_foreign(&elsewhere));
        let after = f.target("deep_agents_code");
        assert_eq!(
            (after.status, after.synced_at, &after.target_path),
            (before.status, before.synced_at, &before.target_path)
        );
    }

    /// A corrected-folder legacy record: an Augment link row still records
    /// the former `~/.augment/rules` folder, an operator's folder took its
    /// place, and another sits at the current `~/.augment/skills`. The ask
    /// names the former path; Overwrite re-links there only, the current
    /// folder's occupant survives, and the next Propagation follows the link.
    #[test]
    fn a_legacy_record_in_a_former_folder_is_answered_at_its_recorded_path() {
        let f = fixture("augment");
        f.sync("augment");
        let current = f.global(".augment/skills");
        let former = f.global(".augment/rules");
        // The row as 1.2.18 left it: the former folder, a link it wrote, now
        // taken by the operator's folder.
        f.record_target_at("augment", &former);
        replace_with_foreign_folder(&former);
        replace_with_foreign_folder(&current);
        f.update_library();

        let outcomes = f.propagate();
        let [(
            _,
            PropagationStatus::Failed {
                error: CommandError::TargetExists { path, .. },
                ..
            },
        )] = outcomes.as_slice()
        else {
            panic!("one ask: {outcomes:?}");
        };
        assert_eq!(Path::new(path), former);

        let answer = f.answer_global("augment", &former, OverrideDecision::Overwrite);

        assert!(
            matches!(answer[0].status, BatchTargetStatus::Synced { .. }),
            "{answer:?}"
        );
        assert!(!is_foreign(&former), "re-linked at the asked path");
        assert!(
            is_foreign(&current),
            "the current folder's occupant survives"
        );
        assert_eq!(Path::new(&f.target("augment").target_path), former);
        let again = f.propagate();
        assert!(
            matches!(
                again.as_slice(),
                [(
                    _,
                    PropagationStatus::Skipped {
                        reason: PropagationSkip::LinkFollowsSource
                    }
                )]
            ),
            "no second ask: {again:?}"
        );
    }

    // -- Only inside a skills folder the app manages (round 20 t18 review,
    //    Astra 2)

    /// A Deep Agents copy whose stored path is `~/Documents/alpha` (no flow
    /// writes one; a malformed record) still holds what the row deployed.
    /// Delete refuses it (`ResolvesElsewhere`), keeps the bytes, the row
    /// (`error`, ADR-0002) and the skill.
    #[test]
    fn a_stored_path_outside_every_managed_folder_is_refused_by_delete() {
        let f = fixture("deep_agents_code");
        f.sync("deep_agents_code");
        let deployed = f.global(".deepagents/agent/skills");
        let stray = f.paths.roots.home.join("Documents/alpha");
        fs::create_dir_all(stray.parent().unwrap()).unwrap();
        fs::rename(&deployed, &stray).unwrap();
        f.record_target_at("deep_agents_code", &stray);

        let report = f.delete();

        assert!(
            !report.record_deleted && !report.central_removed,
            "{report}"
        );
        assert!(
            matches!(
                &report.targets[..],
                [target] if matches!(
                    &target.status,
                    RemovalTargetStatus::Failed {
                        error: CommandError::RemovalRefused {
                            reason: RemovalRefusal::ResolvesElsewhere,
                            ..
                        }
                    }
                )
            ),
            "{report}"
        );
        assert!(stray.join("SKILL.md").is_file(), "the bytes are kept");
        assert_eq!(f.global_status("deep_agents_code"), Some(SyncStatus::Error));
        assert!(f.target("deep_agents_code").owns_artifact());
        assert!(f.store.get_skill_by_id(&f.skill_id).unwrap().is_some());
    }

    /// The same record through Propagation: its stale copy is the row's own,
    /// but a stored path outside every managed folder is never replaced.
    #[test]
    fn a_stored_path_outside_every_managed_folder_is_never_replaced_by_propagation() {
        let f = fixture("deep_agents_code");
        f.sync("deep_agents_code");
        let deployed = f.global(".deepagents/agent/skills");
        let stray = f.paths.roots.home.join("Documents/alpha");
        fs::create_dir_all(stray.parent().unwrap()).unwrap();
        fs::rename(&deployed, &stray).unwrap();
        f.record_target_at("deep_agents_code", &stray);
        f.update_library();

        let outcomes = f.propagate();

        assert!(
            matches!(
                outcomes.as_slice(),
                [(
                    _,
                    PropagationStatus::Failed {
                        error: CommandError::RemovalRefused {
                            reason: RemovalRefusal::ResolvesElsewhere,
                            ..
                        },
                        ..
                    }
                )]
            ),
            "{outcomes:?}"
        );
        assert!(
            fs::read_to_string(stray.join("SKILL.md"))
                .unwrap()
                .contains("v1"),
            "the old bytes are kept"
        );
        assert_eq!(f.global_status("deep_agents_code"), Some(SyncStatus::Error));
    }

    // -- Copies (a copy-only harness): the deployed identity is the evidence

    /// The copy subtlety: Propagation replaces the app's own stale copy (it
    /// still holds what the row deployed, the old version) and records the
    /// new identity — at both scopes.
    #[test]
    fn propagation_replaces_its_own_stale_copy_at_both_scopes() {
        let f = fixture("deep_agents_code");
        f.sync("deep_agents_code");
        f.assign();
        let global = f.global(".deepagents/agent/skills");
        let assigned = f.assigned(".deepagents/skills");
        assert!(!is_link(&global) && !is_link(&assigned), "copies");
        f.update_library();

        let outcomes = f.propagate();

        assert_eq!(outcomes.len(), 2, "{outcomes:?}");
        assert!(
            outcomes
                .iter()
                .all(|(_, s)| matches!(s, PropagationStatus::Synced { .. })),
            "{outcomes:?}"
        );
        for path in [&global, &assigned] {
            assert!(fs::read_to_string(path.join("SKILL.md"))
                .unwrap()
                .contains("v2"));
        }
        let new_identity = content_identity::read(Source::Directory(&f.central()));
        let row = f
            .store
            .get_skill_target(&f.skill_id, "deep_agents_code")
            .unwrap()
            .unwrap();
        assert_eq!(row.content_hash, new_identity, "the global row records v2");
        assert_eq!(f.assignment("deep_agents_code").content_hash, new_identity);
    }

    #[test]
    fn a_copy_replaced_by_a_foreign_folder_goes_to_the_ask_through_propagation() {
        for scope in ["global", "project"] {
            let f = fixture("deep_agents_code");
            let path = if scope == "global" {
                f.sync("deep_agents_code");
                f.global(".deepagents/agent/skills")
            } else {
                f.assign();
                f.assigned(".deepagents/skills")
            };
            replace_with_foreign_folder(&path);
            f.update_library();

            let outcomes = f.propagate();

            assert!(propagation_asked(&outcomes), "{scope}: {outcomes:?}");
            assert!(is_foreign(&path), "{scope}: the folder survives");
        }
    }

    /// A copy edited in place no longer holds what the row deployed: resync
    /// and Propagation ask, Delete leaves it in place — at both scopes.
    #[test]
    fn a_copy_edited_in_place_is_not_ours() {
        let f = fixture("deep_agents_code");
        f.sync("deep_agents_code");
        f.assign();
        let global = f.global(".deepagents/agent/skills");
        let assigned = f.assigned(".deepagents/skills");
        for path in [&global, &assigned] {
            fs::write(path.join("SKILL.md"), "---\nname: alpha\n---\nmy edits\n").unwrap();
        }
        let edited = |path: &Path| {
            fs::read_to_string(path.join("SKILL.md")).is_ok_and(|t| t.contains("my edits"))
        };

        let resync = f.resync();
        assert!(asked(&resync.items[0].status), "{resync:?}");
        assert!(edited(&assigned));

        f.update_library();
        let outcomes = f.propagate();
        assert_eq!(outcomes.len(), 2, "{outcomes:?}");
        assert!(
            outcomes.iter().all(|(_, s)| matches!(
                s,
                PropagationStatus::Failed {
                    error: CommandError::TargetExists { .. },
                    ..
                }
            )),
            "{outcomes:?}"
        );
        assert!(edited(&global) && edited(&assigned));

        let deleted = f.delete();
        assert!(deleted.record_deleted, "{deleted}");
        assert_eq!(deleted.left_in_place_rows(), 2, "{deleted}");
        assert!(edited(&global) && edited(&assigned), "never deleted");
    }

    /// The pre-1.4.0 copy row (no recorded identity): only a link resolving
    /// to the Library copy is provably ours, so its copy is left by Delete.
    #[test]
    fn a_global_copy_with_no_recorded_identity_is_left_by_delete() {
        let f = fixture("deep_agents_code");
        f.sync("deep_agents_code");
        let global = f.global(".deepagents/agent/skills");
        let conn = rusqlite::Connection::open(f.store.db_path()).unwrap();
        conn.execute_batch("UPDATE skill_targets SET content_hash = NULL;")
            .unwrap();

        let deleted = f.delete();

        assert!(deleted.record_deleted, "{deleted}");
        assert_eq!(deleted.left_in_place_rows(), 1, "{deleted}");
        assert!(global.join("SKILL.md").is_file(), "the copy stays");
        assert!(f.global_status("deep_agents_code").is_none());
    }

    /// A project Disable whose removal fails keeps the assignment's claim
    /// (ADR-0002); its retry removes the artifact and only then gives up.
    #[cfg(unix)]
    #[test]
    fn a_project_disable_whose_removal_failed_keeps_ownership() {
        use crate::core::skill_disable::{disable_targets, DisableScope};
        use std::os::unix::fs::PermissionsExt;

        let f = fixture("claude_code");
        f.assign();
        let assigned = f.assigned(".claude/skills");
        let parent = assigned.parent().unwrap().to_path_buf();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o555)).unwrap();
        let probe = parent.join("probe");
        if fs::create_dir(&probe).is_ok() {
            // Root ignores permission bits: nothing to test here.
            fs::remove_dir(&probe).unwrap();
            fs::set_permissions(&parent, fs::Permissions::from_mode(0o755)).unwrap();
            return;
        }
        let disable = |f: &Fixture| {
            disable_targets(
                &f.store,
                &central_repo_of(&f.store),
                &f.paths.roots,
                &DisableScope::Skill {
                    skill_id: f.skill_id.clone(),
                },
                NOW,
            )
            .unwrap()
        };
        let failed = disable(&f);
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(failed.failed_rows(), 1, "{failed}");
        let row = f.assignment("claude_code");
        assert_eq!(row.status, SyncStatus::Error);
        assert!(row.owns_artifact(), "a failed removal keeps ownership");
        assert!(is_link(&assigned));

        let retried = disable(&f);
        assert_eq!(retried.removed_rows(), 1, "{retried}");
        assert!(assigned.symlink_metadata().is_err());
        let row = f.assignment("claude_code");
        assert_eq!(row.status, SyncStatus::Disabled);
        assert!(!row.owns_artifact());
    }

    /// A link row still resolving to the Library copy follows the source
    /// through Propagation: skipped, never replaced.
    #[test]
    fn a_link_row_follows_the_source_through_propagation() {
        let f = fixture("claude_code");
        f.assign();
        f.update_library();
        let outcomes = f.propagate();
        assert!(matches!(
            outcomes.as_slice(),
            [(
                _,
                PropagationStatus::Skipped {
                    reason: PropagationSkip::LinkFollowsSource
                }
            )]
        ));
    }
}
