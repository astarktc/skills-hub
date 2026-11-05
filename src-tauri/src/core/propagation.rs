//! Propagation: bringing every Sync target of one Managed skill into line
//! after its central copy changed.
//!
//! One module spans both scopes — the skill's global target rows and its
//! project assignment rows — because "the bytes changed, make every target
//! match" is one rule, not two. Propagation reads its own rows: callers hand
//! it `(store, paths, skill_id, now)` and nothing else.
//!
//! Three decisions live here and nowhere else:
//!
//! 1. **Does this target need new bytes?** Only when its Sync mode can drift
//!    (a copy) or its Tool cannot consume a link at all
//!    ([`needs_new_bytes`], the one `force_copy` predicate). A link already
//!    follows the central copy, so it is reported
//!    [`PropagationSkip::LinkFollowsSource`] and left untouched.
//! 2. **How do the bytes get there?** Only through
//!    [`sync_engine::sync_dir_for_tool_with_overwrite`], the capability-aware
//!    entry point the batch engines use (a project assignment reaches it
//!    through `project_sync::sync_assignment_target`, the one function that
//!    syncs an assignment and records `SyncCompleted`). It may
//!    re-materialise a drifting copy as a link on a symlink-capable Tool; the
//!    row records the mode actually used, so it stays truthful (and can no
//!    longer drift).
//! 3. **What does a target's outcome mean?** Every target resolves to
//!    synced / skipped / failed as *report data*
//!    (continue-and-report): one target's failure never fails the operation.
//!    Only reading the rows can fail the operation.
//!
//! Row settlement goes through the typed transitions on both tables
//! ([`TargetTransition`], [`AssignmentTransition`]) — never a hand-built
//! record literal.
//!
//! Unlocked internal seam: callers reach it through an entry point that has
//! already taken the mutation guard (see `mutation_guard`).

use std::path::{Path, PathBuf};

use anyhow::Result;
use uuid::Uuid;

use super::errors::{CommandError, SignalError};
use super::installer::InstallerPaths;
use super::project_sync::AssignmentSyncContext;
use super::settings;
use super::skill_store::{
    AssignmentTransition, ProjectSkillAssignmentRecord, SkillRecord, SkillStore, SkillTargetRecord,
    TargetTransition,
};
use super::sync_engine;
use super::sync_status::{SyncMode, SyncStatus};
use super::tool_adapters::{
    adapter_by_key, adapters_sharing_skills_dir, global_skills_dir_in, is_installed_in,
    superseded_skills_dirs_in, ToolAdapter,
};

/// Which Sync target an outcome is about.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, specta::Type)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum PropagationScope {
    Global { tool: String },
    Project { project_id: String, tool: String },
}

/// Why a target needed no work. Not a failure: skipping is the correct
/// outcome for a link, an uninstalled Tool, or a project that is not there.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, specta::Type)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum PropagationSkip {
    /// The target is a link (or junction) into the central copy, which was
    /// just refreshed in place — the target is already current.
    LinkFollowsSource,
    /// The Tool is no longer installed for this operator.
    ToolNotInstalled { tool: String },
    /// The row names a tool key the registry does not know.
    UnknownTool { tool: String },
    /// The project row is gone, or its directory no longer exists on disk.
    ProjectUnavailable { project_id: String },
    /// The operator disabled this target — its own row, a row sharing its
    /// skills dir, or the whole skill (see **Disabled** in `CONTEXT.md`).
    /// Nothing but Enable re-materialises it.
    Disabled,
    /// A first sync into the dir the shared group deploys to, held back
    /// while an owned member's artifact still sits in a superseded dir (a
    /// relocation that could not finish): writing now would leave that
    /// harness reading a second copy. The row is not written; the error is
    /// `RELOCATION_PENDING`, naming the Tool and the path still to move.
    RelocationPending { error: CommandError },
}

#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PropagationStatus {
    Synced {
        mode_used: SyncMode,
    },
    Skipped {
        reason: PropagationSkip,
    },
    /// `first_deploy` is true when the row had never materialised an
    /// artifact before this pass (`owns_artifact` false: a failed first
    /// sync's `error` row) — the failed write was a first deploy, not an
    /// update of a copy the app already owned.
    Failed {
        error: CommandError,
        first_deploy: bool,
    },
}

#[derive(Debug, serde::Serialize, specta::Type)]
pub struct PropagationOutcome {
    pub scope: PropagationScope,
    pub status: PropagationStatus,
}

/// Every Sync target of one Managed skill, with what happened to it.
#[derive(Debug, Default, serde::Serialize, specta::Type)]
pub struct PropagationReport {
    pub targets: Vec<PropagationOutcome>,
}

/// The one `force_copy` predicate: a target needs its bytes written again
/// when its Sync mode can drift (a copy) or its Tool cannot consume a link.
pub(crate) fn needs_new_bytes(mode: SyncMode, adapter: &ToolAdapter) -> bool {
    mode.can_drift() || !adapter.supports_symlink
}

/// Re-materialise every Sync target of `skill_id` from its central copy.
///
/// Content identity is read once, including legacy-row backfill, and recorded
/// on copies (only copies can drift). Unlocked internal seam.
pub(crate) fn propagate_unlocked(
    store: &SkillStore,
    paths: &InstallerPaths,
    skill_id: &str,
    now: i64,
) -> Result<PropagationReport> {
    let mut skill = store.get_skill_by_id(skill_id)?.ok_or_else(|| {
        anyhow::anyhow!(SignalError::NotFound {
            kind: "skill".to_string(),
            id: skill_id.to_string(),
        })
    })?;
    skill.content_hash =
        super::content_identity::read(super::content_identity::Source::Managed { store, skill_id });
    let central_path = PathBuf::from(&skill.central_path);

    let mut report = PropagationReport::default();
    if store.skill_disabled_at(skill_id)?.is_some() {
        // Off everywhere until enabled: every row is reported, none is touched.
        for row in store.list_skill_targets(skill_id)? {
            report
                .targets
                .push(skipped_global(&row.tool, PropagationSkip::Disabled));
        }
        for assignment in store.list_project_skill_assignments_by_skill(skill_id)? {
            report.targets.push(PropagationOutcome {
                scope: PropagationScope::Project {
                    project_id: assignment.project_id,
                    tool: assignment.tool,
                },
                status: PropagationStatus::Skipped {
                    reason: PropagationSkip::Disabled,
                },
            });
        }
        return Ok(report);
    }
    propagate_global_rows(
        store,
        &paths.home,
        &central_path,
        skill_id,
        now,
        &mut report,
    )?;
    propagate_project_rows(store, &skill, now, &mut report)?;
    Ok(report)
}

/// The typed condition for "the bytes we are supposed to propagate are not
/// there" — raised per target as report data, never as prose.
fn missing_source(central_path: &Path) -> anyhow::Error {
    anyhow::anyhow!(SignalError::InvalidPath {
        path: central_path.to_string_lossy().into_owned(),
        reason: "missing".to_string(),
    })
}

// ---------------------------------------------------------------------------
// Global scope
// ---------------------------------------------------------------------------

/// Global target rows, handled one **shared skills dir group at a time**:
/// tools sharing a global skills dir share one artifact, so the bytes are
/// written once and every member row is settled.
fn propagate_global_rows(
    store: &SkillStore,
    home: &Path,
    central_path: &Path,
    skill_id: &str,
    now: i64,
    report: &mut PropagationReport,
) -> Result<()> {
    let rows: Vec<_> = store
        .list_skill_targets(skill_id)?
        .into_iter()
        .map(|row| {
            let adapter = adapter_by_key(&row.tool);
            (row, adapter)
        })
        .collect();
    let mut handled: Vec<String> = Vec::new();
    let layout = settings::global_layout(store);

    for (row, adapter) in &rows {
        if handled.contains(&row.tool) {
            continue;
        }
        handled.push(row.tool.clone());

        let Some(adapter) = *adapter else {
            let reason = if row.status == SyncStatus::Disabled {
                PropagationSkip::Disabled
            } else {
                PropagationSkip::UnknownTool {
                    tool: row.tool.clone(),
                }
            };
            report.targets.push(skipped_global(&row.tool, reason));
            continue;
        };

        // The group: this skill's rows whose tools share the global skills
        // dir (the dir, not the key, is a target's identity). Which tools
        // those are is the registry's answer; the rows are matched by key.
        // The row's own Tool is a member by definition — the registry
        // answers from its static entries while the row resolved through
        // `adapter_by_key`, and the two may disagree (a test shadow), so the
        // membership is not left to the registry's reflexivity.
        let sharers = adapters_sharing_skills_dir(adapter, layout);
        let sharing: Vec<&str> = sharers.iter().map(|a| a.key()).collect();
        let group: Vec<(&SkillTargetRecord, &'static ToolAdapter)> = rows
            .iter()
            .filter(|(r, _)| r.tool == row.tool || sharing.contains(&r.tool.as_str()))
            .filter_map(|(r, a)| a.map(|a| (r, a)))
            .collect();
        for (member, _) in &group {
            if !handled.contains(&member.tool) {
                handled.push(member.tool.clone());
            }
        }

        // One disabled member disables the shared artifact: writing it for a
        // sibling would re-materialise it where the disabled Tool reads.
        if group
            .iter()
            .any(|(member, _)| member.status == SyncStatus::Disabled)
        {
            for (member, _) in &group {
                report
                    .targets
                    .push(skipped_global(&member.tool, PropagationSkip::Disabled));
            }
            continue;
        }

        let (installed, absent): (Vec<_>, Vec<_>) = group
            .into_iter()
            .partition(|(_, a)| is_installed_in(home, a));
        for (member, _) in &absent {
            report.targets.push(skipped_global(
                &member.tool,
                PropagationSkip::ToolNotInstalled {
                    tool: member.tool.clone(),
                },
            ));
        }
        if installed.is_empty() {
            continue;
        }

        // Each member's **write target**: an owned row's artifact is where its
        // row says — rows at different stored paths are different artifacts
        // (a relocation that could not finish leaves Cursor's copy in its own
        // dir while Codex's sits in `~/.agents/skills`), so each is updated in
        // place and no row is ever settled at another row's path; only the
        // guarded relocation seam moves an owned row. A row that owns nothing
        // has no artifact to keep: its first sync goes where the group deploys
        // now, under the same artifact name.
        let root = global_skills_dir_in(home, adapter, layout);
        // An owned member whose artifact still sits in one of its Tool's
        // superseded dirs: until the guarded relocation moves it, no first
        // sync goes into the group's dir (the member's harness would read
        // both copies). Its own partition still updates in place.
        let pending_owner: Option<&SkillTargetRecord> = installed
            .iter()
            .find(|(member, a)| {
                let stored = Path::new(&member.target_path);
                member.owns_artifact()
                    && stored.parent().is_some_and(|parent| {
                        superseded_skills_dirs_in(home, a, layout)
                            .iter()
                            .any(|dir| dir == parent)
                    })
                    && stored.symlink_metadata().is_ok()
            })
            .map(|(member, _)| *member);
        let mut partitions: Vec<(PathBuf, GroupMembers)> = Vec::new();
        for (member, a) in installed {
            let stored = Path::new(&member.target_path);
            let at = match stored.file_name() {
                Some(name) if !member.owns_artifact() => root.join(name),
                _ => stored.to_path_buf(),
            };
            match partitions.iter_mut().find(|(path, _)| *path == at) {
                Some((_, members)) => members.push((member, a)),
                None => partitions.push((at, vec![(member, a)])),
            }
        }

        for (at, members) in partitions {
            // A shared skills dir is one artifact (D34 amendment): a first
            // sync into the group's dir is recorded for every *installed*
            // sharer, exactly as the global batch records its `record_tools`
            // — the sharers this skill has no row for yet are settled here
            // beside the group's rows. An update of an owned artifact (or a
            // first sync of anything but the group's dir) records nothing
            // new. A sibling minted here is never visited again in this pass
            // (`rows` is a snapshot; `handled` names it besides).
            let first_sync_into_root = at.parent() == Some(root.as_path())
                && !members.iter().any(|(member, _)| member.owns_artifact());
            if let (true, Some(owner)) = (first_sync_into_root, pending_owner) {
                let error = CommandError::RelocationPending {
                    tool: owner.tool.clone(),
                    path: owner.target_path.clone(),
                };
                for (member, _) in &members {
                    report.targets.push(skipped_global(
                        &member.tool,
                        PropagationSkip::RelocationPending {
                            error: error.clone(),
                        },
                    ));
                }
                continue;
            }
            let siblings: Vec<&'static ToolAdapter> = if first_sync_into_root {
                sharers
                    .iter()
                    .copied()
                    .filter(|a| !rows.iter().any(|(r, _)| r.tool == a.key()))
                    .filter(|a| !handled.iter().any(|tool| tool == a.key()))
                    .filter(|a| is_installed_in(home, a))
                    .collect()
            } else {
                Vec::new()
            };
            for sibling in &siblings {
                handled.push(sibling.key().to_string());
            }
            let write = ArtifactWrite {
                store,
                central_path,
                skill_id,
                now,
                at: &at,
                default_adapter: adapter,
            };
            write.settle(&members, &siblings, report)?;
        }
    }
    Ok(())
}

/// One partition of a shared-dir group: the rows whose artifact is `at`.
type GroupMembers<'r> = Vec<(&'r SkillTargetRecord, &'static ToolAdapter)>;

/// Write one global artifact and settle every row describing it.
struct ArtifactWrite<'a> {
    store: &'a SkillStore,
    central_path: &'a Path,
    skill_id: &'a str,
    now: i64,
    /// The artifact's path: the members' stored path (owned) or where the
    /// group deploys now (a first sync).
    at: &'a Path,
    /// The group's own Tool, writing unless a member is copy-only.
    default_adapter: &'static ToolAdapter,
}

impl ArtifactWrite<'_> {
    /// `members` all describe the artifact at `self.at`; `siblings` are the
    /// installed sharers a first sync into the group's dir records beside
    /// them.
    fn settle(
        &self,
        members: &[(&SkillTargetRecord, &'static ToolAdapter)],
        siblings: &[&'static ToolAdapter],
        report: &mut PropagationReport,
    ) -> Result<()> {
        let Self {
            store,
            central_path,
            skill_id,
            now,
            ..
        } = *self;
        let at_str = self.at.to_string_lossy().into_owned();

        // A link already follows the central copy — but only a link of ours:
        // a row that never materialised (a failed first sync's `error` row,
        // `owns_artifact` false) has no link yet, so it gets a first sync.
        if !members
            .iter()
            .any(|(member, a)| needs_new_bytes(member.mode, a) || !member.owns_artifact())
        {
            for (member, _) in members {
                report.targets.push(skipped_global(
                    &member.tool,
                    PropagationSkip::LinkFollowsSource,
                ));
            }
            return Ok(());
        }

        // The shared artifact is ours to replace only when a member row
        // materialised it (`owns_artifact`); otherwise this is a first sync
        // and foreign bytes there are `TARGET_EXISTS`.
        let overwrite = members.iter().any(|(member, _)| member.owns_artifact());

        // The group's most restrictive member — a sibling about to be
        // recorded included — decides how the shared artifact is written:
        // one copy-only Tool makes the whole dir a copy.
        let driving_adapter = members
            .iter()
            .map(|(_, a)| *a)
            .chain(siblings.iter().copied())
            .find(|a| !a.supports_symlink)
            .unwrap_or(self.default_adapter);

        let source_missing = !central_path.is_dir();
        let result = if source_missing {
            Err(missing_source(central_path))
        } else {
            sync_engine::sync_dir_for_tool_with_overwrite(
                driving_adapter,
                central_path,
                super::content_identity::Source::Managed { store, skill_id },
                self.at,
                overwrite,
            )
        };

        match result {
            Ok(outcome) => {
                let target_path = outcome.target_path.to_string_lossy().to_string();
                for (member, _) in members {
                    store.transition_skill_target(
                        &member.id,
                        TargetTransition::SyncCompleted {
                            mode: outcome.mode_used,
                            target_path: &target_path,
                            synced_at: now,
                        },
                    )?;
                    report.targets.push(PropagationOutcome {
                        scope: PropagationScope::Global {
                            tool: member.tool.clone(),
                        },
                        status: PropagationStatus::Synced {
                            mode_used: outcome.mode_used,
                        },
                    });
                }
                for sibling in siblings {
                    store.transition_skill_target(
                        &Uuid::new_v4().to_string(),
                        TargetTransition::Recorded {
                            skill_id,
                            tool: sibling.key(),
                            mode: outcome.mode_used,
                            target_path: &target_path,
                            synced_at: now,
                        },
                    )?;
                    report.targets.push(PropagationOutcome {
                        scope: PropagationScope::Global {
                            tool: sibling.key().to_string(),
                        },
                        status: PropagationStatus::Synced {
                            mode_used: outcome.mode_used,
                        },
                    });
                }
            }
            Err(error) => {
                let detail = format!("{:#}", error);
                // The sibling rule of a failed first sync is the global
                // batch's (`global_sync::sync_skills_to_planned_tools`): a
                // `TARGET_EXISTS` records nothing until the operator answers
                // the ask (D12) — its confirmed retry goes through that batch,
                // which records the sharers then; any other failure records
                // every installed sharer `error`, owning nothing
                // (`record_failed`, D9).
                let collision = error
                    .chain()
                    .any(|cause| cause.is::<sync_engine::TargetExistsError>());
                let failed_siblings: &[&'static ToolAdapter] =
                    if collision { &[] } else { siblings };
                // Each member row is its own Sync target, so each one is
                // settled and reported as failed. A missing central copy is
                // the same typed condition for every row; a sync failure's
                // full chain belongs to the row that was attempted, and the
                // rest carry its diagnostic text.
                let mut original = Some(error);
                for (member, _) in members {
                    store.transition_skill_target(
                        &member.id,
                        TargetTransition::SyncFailed { error: &detail },
                    )?;
                    let error = if source_missing {
                        missing_source(central_path)
                    } else {
                        original
                            .take()
                            .unwrap_or_else(|| anyhow::anyhow!("{}", detail))
                    };
                    report.targets.push(PropagationOutcome {
                        scope: PropagationScope::Global {
                            tool: member.tool.clone(),
                        },
                        status: PropagationStatus::Failed {
                            error: CommandError::from_anyhow(error),
                            first_deploy: !member.owns_artifact(),
                        },
                    });
                }
                // A row that never materialised records the mode a sync
                // would try first, as the batch's `would_be` does.
                let would_be_mode = if driving_adapter.supports_symlink {
                    SyncMode::Symlink
                } else {
                    SyncMode::Copy
                };
                for sibling in failed_siblings {
                    store.transition_skill_target(
                        &Uuid::new_v4().to_string(),
                        TargetTransition::FailedRecorded {
                            skill_id,
                            tool: sibling.key(),
                            mode: would_be_mode,
                            target_path: &at_str,
                            error: &detail,
                        },
                    )?;
                    let error = if source_missing {
                        missing_source(central_path)
                    } else {
                        anyhow::anyhow!("{}", detail)
                    };
                    report.targets.push(PropagationOutcome {
                        scope: PropagationScope::Global {
                            tool: sibling.key().to_string(),
                        },
                        status: PropagationStatus::Failed {
                            error: CommandError::from_anyhow(error),
                            first_deploy: true,
                        },
                    });
                }
            }
        }
        Ok(())
    }
}

fn skipped_global(tool: &str, reason: PropagationSkip) -> PropagationOutcome {
    PropagationOutcome {
        scope: PropagationScope::Global {
            tool: tool.to_string(),
        },
        status: PropagationStatus::Skipped { reason },
    }
}

// ---------------------------------------------------------------------------
// Project scope
// ---------------------------------------------------------------------------

fn propagate_project_rows(
    store: &SkillStore,
    skill: &SkillRecord,
    now: i64,
    report: &mut PropagationReport,
) -> Result<()> {
    for assignment in store.list_project_skill_assignments_by_skill(&skill.id)? {
        let scope = PropagationScope::Project {
            project_id: assignment.project_id.clone(),
            tool: assignment.tool.clone(),
        };
        let status = propagate_one_assignment(store, skill, now, &assignment)?;
        report.targets.push(PropagationOutcome { scope, status });
    }
    Ok(())
}

/// One project assignment: decide whether it needs new bytes, then sync it
/// through `project_sync::sync_assignment_target` (which locates the artifact
/// by its stored name and records `SyncCompleted`) and settle a failure here.
/// The adapter is resolved here, once; the skill was read once for the whole
/// Propagation — both travel down in the [`AssignmentSyncContext`].
fn propagate_one_assignment(
    store: &SkillStore,
    skill: &SkillRecord,
    now: i64,
    assignment: &ProjectSkillAssignmentRecord,
) -> Result<PropagationStatus> {
    if assignment.status == SyncStatus::Disabled {
        return Ok(PropagationStatus::Skipped {
            reason: PropagationSkip::Disabled,
        });
    }
    let Some(adapter) = adapter_by_key(&assignment.tool) else {
        return Ok(PropagationStatus::Skipped {
            reason: PropagationSkip::UnknownTool {
                tool: assignment.tool.clone(),
            },
        });
    };
    let unavailable = PropagationSkip::ProjectUnavailable {
        project_id: assignment.project_id.clone(),
    };
    let Some(project) = store.get_project_by_id(&assignment.project_id)? else {
        return Ok(PropagationStatus::Skipped {
            reason: unavailable,
        });
    };
    // The project-root rule shared with every project sync: an existing
    // directory, never merely an existing path.
    let project_path = PathBuf::from(&project.path);
    if !super::project_ops::project_root_present(&project_path) {
        return Ok(PropagationStatus::Skipped {
            reason: unavailable,
        });
    }
    // Only a link of ours follows the source; a row that never materialised
    // gets a first sync (see the global arm).
    if assignment.owns_artifact() && !needs_new_bytes(assignment.mode, adapter) {
        return Ok(PropagationStatus::Skipped {
            reason: PropagationSkip::LinkFollowsSource,
        });
    }

    // The freshly finalized central hash is the supplier: computed once for
    // every target, and absent only when hashing the central copy failed.
    let central_path = Path::new(&skill.central_path);
    let result = if central_path.is_dir() {
        let ctx = AssignmentSyncContext {
            store,
            project_path: &project_path,
            adapter,
            skill,
            // Only an artifact of ours is replaced; a row that never
            // materialised gets a first sync (`owns_artifact`).
            overwrite: assignment.owns_artifact(),
            now,
        };
        super::project_sync::sync_assignment_target(&ctx, assignment, || skill.content_hash.clone())
    } else {
        Err(missing_source(central_path))
    };

    match result {
        Ok(outcome) => Ok(PropagationStatus::Synced {
            mode_used: outcome.mode_used,
        }),
        Err(error) => {
            let detail = format!("{:#}", error);
            store.transition_assignment(
                &assignment.id,
                AssignmentTransition::SyncFailed { error: &detail },
            )?;
            Ok(PropagationStatus::Failed {
                error: CommandError::from_anyhow(error),
                first_deploy: !assignment.owns_artifact(),
            })
        }
    }
}

#[cfg(test)]
#[path = "tests/propagation.rs"]
mod tests;
