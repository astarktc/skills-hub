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
use super::placement::{self, row_folder, ScopeKind};
use super::project_sync::{deploy_adapter, AssignmentSyncContext};
use super::removal_fence::ProtectedLibrary;
use super::skill_store::{
    AssignmentTransition, ProjectSkillAssignmentRecord, SkillRecord, SkillStore, TargetTransition,
};
use super::sync_engine;
use super::sync_status::{SyncMode, SyncStatus};
use super::tool_adapters::{catalog_rows, installed_set_in, ToolAdapter};

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

/// Global target rows, **one row at a time**: each row is synced at its own
/// recorded path, with its own ownership, and settles only itself. Rows a
/// pre-1.4.0 app wrote once per shared-dir sibling (Amp and Kimi on
/// `~/.config/agents/skills`) are each their own row here until the redeploy
/// collapses them; nothing is lent or settled between them.
///
/// Two facts are the row's folder's (`placement::row_folder`) rather than its
/// key's: the capability it is deployed with (`placement::capability_entry` —
/// a group's capability stays its own fact) and whether any harness reading
/// it is installed. And one disabled row of the folder keeps every row of it
/// skipped, as on every Sync-target path: writing the shared artifact for one
/// row would re-materialise it where the disabled harness reads. That rule
/// only ever withholds a write; it never grants one.
fn propagate_global_rows(
    store: &SkillStore,
    central_dir: &Path,
    roots: &HostRoots,
    central_path: &Path,
    skill_id: &str,
    now: i64,
    report: &mut PropagationReport,
) -> Result<()> {
    let rows = store.list_skill_targets(skill_id)?;
    if rows.is_empty() {
        return Ok(());
    }
    let installed = installed_set_in(roots);

    for row in &rows {
        let folder = row_folder(ScopeKind::Global, &row.tool);
        let folder_disabled = rows.iter().any(|other| {
            other.status == SyncStatus::Disabled
                && row_folder(ScopeKind::Global, &other.tool) == folder
        });
        if folder_disabled {
            report
                .targets
                .push(skipped_global(&row.tool, PropagationSkip::Disabled));
            continue;
        }
        let Some(adapter) = placement::capability_entry(ScopeKind::Global, &folder) else {
            report.targets.push(skipped_global(
                &row.tool,
                PropagationSkip::UnknownTool {
                    tool: row.tool.clone(),
                },
            ));
            continue;
        };
        // Reachable when any harness whose own folder it is, is installed:
        // the row is that folder's, whatever key wrote it.
        let reachable = catalog_rows()
            .filter(|a| row_folder(ScopeKind::Global, a.key()) == folder)
            .any(|a| installed.contains(a.key()));
        if !reachable {
            report.targets.push(skipped_global(
                &row.tool,
                PropagationSkip::ToolNotInstalled {
                    tool: row.tool.clone(),
                },
            ));
            continue;
        }

        // A link already follows the central copy — but only a link of ours:
        // a row that never materialised (a failed first sync's `error` row,
        // `owns_artifact` false) has no link yet, so it gets a first sync,
        // and foreign bytes there are `TARGET_EXISTS`. The artifact is ours
        // to replace only when this row materialised it.
        let owned = row.owns_artifact();
        if owned && !needs_new_bytes(row.mode, adapter) {
            report.targets.push(skipped_global(
                &row.tool,
                PropagationSkip::LinkFollowsSource,
            ));
            continue;
        }

        let source_missing = !central_path.is_dir();
        let result = if source_missing {
            Err(missing_source(central_path))
        } else {
            sync_engine::sync_dir_for_tool_with_overwrite(
                adapter,
                central_path,
                super::content_identity::Source::Managed { store, skill_id },
                Path::new(&row.target_path),
                owned,
                &ProtectedLibrary::new(store, central_dir),
            )
        };

        let status = match result {
            Ok(outcome) => {
                let target_path = outcome.target_path.to_string_lossy().to_string();
                store.transition_skill_target(
                    &row.id,
                    TargetTransition::SyncCompleted {
                        mode: outcome.mode_used,
                        target_path: &target_path,
                        synced_at: now,
                    },
                )?;
                PropagationStatus::Synced {
                    mode_used: outcome.mode_used,
                }
            }
            Err(error) => {
                let detail = format!("{:#}", error);
                store.transition_skill_target(
                    &row.id,
                    TargetTransition::SyncFailed { error: &detail },
                )?;
                PropagationStatus::Failed {
                    error: CommandError::from_anyhow(error),
                    first_deploy: !owned,
                }
            }
        };
        report.targets.push(PropagationOutcome {
            scope: PropagationScope::Global {
                tool: row.tool.clone(),
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
    for assignment in store.list_project_skill_assignments_by_skill(&skill.id)? {
        let scope = PropagationScope::Project {
            project_id: assignment.project_id.clone(),
            tool: assignment.tool.clone(),
        };
        let status = propagate_one_assignment(store, central_dir, skill, now, &assignment)?;
        report.targets.push(PropagationOutcome { scope, status });
    }
    Ok(())
}

/// One project assignment: decide whether it needs new bytes, then sync it
/// through `project_sync::sync_assignment_target` (which locates the artifact
/// by its stored name and records `SyncCompleted`) and settle a failure here.
/// The adapter is resolved here, once — the capability of the project folder
/// the row describes (`project_sync::deploy_adapter`, through placement),
/// never a pre-1.4.0 member key's own; the skill was read once for the whole
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
    let Ok(adapter) = deploy_adapter(&assignment.tool) else {
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
