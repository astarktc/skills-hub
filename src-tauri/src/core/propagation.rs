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

use super::environment::HostRoots;
use super::errors::{CommandError, SignalError};
use super::installer::InstallerPaths;
use super::placement::{self, one_row_per_folder, row_folder, ScopeKind};
use super::project_sync::AssignmentSyncContext;
use super::removal_fence::ProtectedLibrary;
use super::skill_store::{
    AssignmentTransition, ProjectSkillAssignmentRecord, SkillRecord, SkillStore, SkillTargetRecord,
    TargetTransition,
};
use super::sync_engine;
use super::sync_status::{SyncMode, SyncStatus};
use super::tool_adapters::{adapter_by_key, catalog_rows, installed_set_in, ToolAdapter};

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
        // Off everywhere until enabled: every folder is reported, none is touched.
        for row in one_row_per_folder(ScopeKind::Global, store.list_skill_targets(skill_id)?) {
            report
                .targets
                .push(skipped_global(&row.tool, PropagationSkip::Disabled));
        }
        for assignment in one_row_per_folder(
            ScopeKind::Project,
            store.list_project_skill_assignments_by_skill(skill_id)?,
        ) {
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
        &paths.central_dir,
        &paths.roots,
        &central_path,
        skill_id,
        now,
        &mut report,
    )?;
    propagate_project_rows(store, &paths.central_dir, &skill, now, &mut report)?;
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

/// Global target rows, handled one **Deployment folder at a time**: a folder
/// is one artifact, so the bytes are written once and every row describing it
/// is settled — one row per folder, plus any row written once per shared-dir
/// sibling before rows were per folder (`placement::row_folder` reads it as
/// the folder), until the redeploy collapses them. One outcome is reported per
/// folder, named by its folder key. A row that never materialised gets a first
/// deploy and records nothing beyond its own row.
fn propagate_global_rows(
    store: &SkillStore,
    central_dir: &Path,
    roots: &HostRoots,
    central_path: &Path,
    skill_id: &str,
    now: i64,
    report: &mut PropagationReport,
) -> Result<()> {
    let mut groups: Vec<(String, Vec<SkillTargetRecord>)> = Vec::new();
    for row in store.list_skill_targets(skill_id)? {
        let folder = row_folder(ScopeKind::Global, &row.tool);
        match groups.iter_mut().find(|(f, _)| *f == folder) {
            Some((_, rows)) => rows.push(row),
            None => groups.push((folder, vec![row])),
        }
    }
    if groups.is_empty() {
        return Ok(());
    }
    let installed = installed_set_in(roots);

    for (folder, rows) in &groups {
        // One disabled row disables the folder: writing it for a sibling
        // would re-materialise it where the disabled harness reads.
        if rows.iter().any(|row| row.status == SyncStatus::Disabled) {
            report
                .targets
                .push(skipped_global(folder, PropagationSkip::Disabled));
            continue;
        }
        let Some(adapter) = placement::capability_entry(ScopeKind::Global, folder) else {
            report.targets.push(skipped_global(
                folder,
                PropagationSkip::UnknownTool {
                    tool: folder.clone(),
                },
            ));
            continue;
        };
        // The folder is reachable when any harness whose own folder it is, is
        // installed (its rows are what is recorded here, whatever key wrote them).
        let reachable = catalog_rows()
            .filter(|a| row_folder(ScopeKind::Global, a.key()) == *folder)
            .any(|a| installed.contains(a.key()));
        if !reachable {
            report.targets.push(skipped_global(
                folder,
                PropagationSkip::ToolNotInstalled {
                    tool: folder.clone(),
                },
            ));
            continue;
        }

        // A link already follows the central copy — but only a link of ours:
        // a row that never materialised (a failed first sync's `error` row,
        // `owns_artifact` false) has no link yet, so it gets a first sync.
        if !rows
            .iter()
            .any(|row| needs_new_bytes(row.mode, adapter) || !row.owns_artifact())
        {
            report
                .targets
                .push(skipped_global(folder, PropagationSkip::LinkFollowsSource));
            continue;
        }

        // The folder's artifact is ours to replace only when a row
        // materialised it (`owns_artifact`); otherwise this is a first sync
        // and foreign bytes there are `TARGET_EXISTS`.
        let owned = rows.iter().any(|row| row.owns_artifact());
        let driver = rows
            .iter()
            .find(|row| row.tool == *folder)
            .unwrap_or(&rows[0]);

        let source_missing = !central_path.is_dir();
        let result = if source_missing {
            Err(missing_source(central_path))
        } else {
            sync_engine::sync_dir_for_tool_with_overwrite(
                adapter,
                central_path,
                super::content_identity::Source::Managed { store, skill_id },
                Path::new(&driver.target_path),
                owned,
                &ProtectedLibrary::new(store, central_dir),
            )
        };

        let status = match result {
            Ok(outcome) => {
                let target_path = outcome.target_path.to_string_lossy().to_string();
                for row in rows {
                    store.transition_skill_target(
                        &row.id,
                        TargetTransition::SyncCompleted {
                            mode: outcome.mode_used,
                            target_path: &target_path,
                            synced_at: now,
                        },
                    )?;
                }
                PropagationStatus::Synced {
                    mode_used: outcome.mode_used,
                }
            }
            Err(error) => {
                // A failed first sync records nothing new (D9): a
                // `TARGET_EXISTS` is answered by the operator's ask, any other
                // failure settles the folder's rows `error`.
                let detail = format!("{:#}", error);
                for row in rows {
                    store.transition_skill_target(
                        &row.id,
                        TargetTransition::SyncFailed { error: &detail },
                    )?;
                }
                PropagationStatus::Failed {
                    error: CommandError::from_anyhow(error),
                    first_deploy: !owned,
                }
            }
        };
        report.targets.push(PropagationOutcome {
            scope: PropagationScope::Global {
                tool: folder.clone(),
            },
            status,
        });
    }
    Ok(())
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
    central_dir: &Path,
    skill: &SkillRecord,
    now: i64,
    report: &mut PropagationReport,
) -> Result<()> {
    // One folder per project, one sync: rows written once per shared-dir
    // sibling before rows were per folder follow their folder's row.
    let mut groups: Vec<(String, String, Vec<ProjectSkillAssignmentRecord>)> = Vec::new();
    for assignment in store.list_project_skill_assignments_by_skill(&skill.id)? {
        let folder = row_folder(ScopeKind::Project, &assignment.tool);
        match groups
            .iter_mut()
            .find(|(p, f, _)| *p == assignment.project_id && *f == folder)
        {
            Some((_, _, rows)) => rows.push(assignment),
            None => groups.push((assignment.project_id.clone(), folder, vec![assignment])),
        }
    }
    for (project_id, folder, rows) in groups {
        let scope = PropagationScope::Project {
            project_id,
            tool: folder.clone(),
        };
        let pick = rows
            .iter()
            .position(|row| row.status == SyncStatus::Disabled)
            .or_else(|| rows.iter().position(|row| row.tool == folder))
            .unwrap_or(0);
        let driver = &rows[pick];
        let status = propagate_one_assignment(store, central_dir, skill, now, driver)?;
        settle_siblings(store, driver, &rows, &status)?;
        report.targets.push(PropagationOutcome { scope, status });
    }
    Ok(())
}

/// Settle the other rows of the driver's folder as the driver settled.
fn settle_siblings(
    store: &SkillStore,
    driver: &ProjectSkillAssignmentRecord,
    rows: &[ProjectSkillAssignmentRecord],
    status: &PropagationStatus,
) -> Result<()> {
    let siblings = rows.iter().filter(|row| row.id != driver.id);
    match status {
        PropagationStatus::Synced { .. } => {
            let Some(settled) = store.get_project_skill_assignment(
                &driver.project_id,
                &driver.skill_id,
                &driver.tool,
            )?
            else {
                return Ok(());
            };
            for row in siblings {
                store.transition_assignment(
                    &row.id,
                    AssignmentTransition::SyncCompleted {
                        mode: settled.mode,
                        synced_at: settled.synced_at.unwrap_or_default(),
                        content_hash: settled.content_hash.as_deref(),
                    },
                )?;
            }
        }
        PropagationStatus::Failed { .. } => {
            let detail = store
                .get_project_skill_assignment(&driver.project_id, &driver.skill_id, &driver.tool)?
                .and_then(|settled| settled.last_error)
                .unwrap_or_default();
            for row in siblings {
                store.transition_assignment(
                    &row.id,
                    AssignmentTransition::SyncFailed { error: &detail },
                )?;
            }
        }
        PropagationStatus::Skipped { .. } => {}
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
    central_dir: &Path,
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
            central_dir,
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
