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
use super::global_sync::{BatchTargetOutcome, BatchTargetStatus, OverrideDecision};
use super::installer::InstallerPaths;
use super::mutation_guard;
use super::ownership::{self, Deployed, Occupancy};
use super::placement::{self, row_folder, ScopeKind};
use super::project_sync::{deploy_adapter, resolve_assignment_artifact, AssignmentSyncContext};
use super::removal_fence::ProtectedLibrary;
use super::skill_store::{
    AssignmentTransition, ProjectSkillAssignmentRecord, SkillRecord, SkillStore, TargetTransition,
};
use super::sync_engine::{self, Replace};
use super::sync_status::{SyncMode, SyncStatus};
use super::tool_adapters::{installed_set_in, ToolAdapter};

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

/// One row's outcome. `scope` names the raw row (a pre-1.4.0 shared-dir
/// sibling's row keeps its own key); `folder_key` is the Deployment folder
/// that row describes (`placement::row_folder`) — the backend's answer to
/// "which Deployment is this", so a report reader counts one Deployment per
/// (scope, project, skill, folder) however many legacy rows describe it, and
/// never derives a folder itself.
#[derive(Debug, serde::Serialize, specta::Type)]
pub struct PropagationOutcome {
    pub scope: PropagationScope,
    pub folder_key: String,
    pub status: PropagationStatus,
}

impl PropagationOutcome {
    /// The outcome of the row `scope` names, with the folder it describes.
    pub fn new(scope: PropagationScope, status: PropagationStatus) -> Self {
        let folder_key = match &scope {
            PropagationScope::Global { tool } => row_folder(ScopeKind::Global, tool),
            PropagationScope::Project { tool, .. } => row_folder(ScopeKind::Project, tool),
        };
        Self {
            scope,
            folder_key,
            status,
        }
    }
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

/// Whether a row's target needs nothing because it is the link the row wrote:
/// its mode cannot drift on a Tool that consumes links, and the occupant check
/// (`core::ownership::check`, now) proves the entry at `path` is that link,
/// resolving to the skill's Library copy. A row without a claim, an absent
/// link, a folder put in its place, or an occupant that cannot be read is not
/// — it goes through the occupant decision instead, which writes an absent
/// link and asks about anything else (ADR-0007).
fn link_follows_source(
    mode: SyncMode,
    adapter: &ToolAdapter,
    path: &Path,
    deployed: &Deployed,
) -> bool {
    if needs_new_bytes(mode, adapter) {
        return false;
    }
    let is_link = std::fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink());
    is_link && matches!(ownership::check(path, deployed), Ok(Occupancy::Ours))
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

    let mut report = PropagationReport::default();
    if store.skill_disabled_at(skill_id)?.is_some() {
        // Off everywhere until enabled: every row is reported, none is touched.
        for row in store.list_skill_targets(skill_id)? {
            report
                .targets
                .push(skipped_global(&row.tool, PropagationSkip::Disabled));
        }
        for assignment in store.list_project_skill_assignments_by_skill(skill_id)? {
            report.targets.push(PropagationOutcome::new(
                PropagationScope::Project {
                    project_id: assignment.project_id,
                    tool: assignment.tool,
                },
                PropagationStatus::Skipped {
                    reason: PropagationSkip::Disabled,
                },
            ));
        }
        return Ok(report);
    }
    propagate_global_rows(
        store,
        &paths.central_dir,
        &paths.roots,
        &skill,
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
/// The capability a row is deployed with is its folder's
/// (`placement::capability_entry` — a group's capability stays its own fact).
/// Whether it is reachable is the row's own rule (`placement::row_reachable`):
/// a folder row through any installed harness its folder serves, a legacy row
/// through its own harness alone. And one disabled row of the folder keeps every row of it
/// skipped, as on every Sync-target path: writing the shared artifact for one
/// row would re-materialise it where the disabled harness reads. That rule
/// only ever withholds a write; it never grants one.
fn propagate_global_rows(
    store: &SkillStore,
    central_dir: &Path,
    roots: &HostRoots,
    skill: &SkillRecord,
    now: i64,
    report: &mut PropagationReport,
) -> Result<()> {
    let skill_id = skill.id.as_str();
    let central_path = Path::new(&skill.central_path);
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
        // Reachable by the row's own rule (`placement::row_reachable`): a
        // folder row when any harness its folder serves is installed, a
        // legacy row only when its own harness is — as before rows were per
        // folder, so an old row of an uninstalled harness is never written
        // because a sibling of its folder is installed.
        if !placement::row_reachable(ScopeKind::Global, &row.tool, &installed) {
            report.targets.push(skipped_global(
                &row.tool,
                PropagationSkip::ToolNotInstalled {
                    tool: row.tool.clone(),
                },
            ));
            continue;
        }

        // A link already follows the central copy — but only the link the
        // row wrote, verified now (`link_follows_source`): a link row whose
        // path another folder took, or whose link went, takes the occupant
        // decision below like any other row. A row that never materialised (a
        // failed first sync's `error` row, `owns_artifact` false) has no link
        // yet, so it gets a first sync, and foreign bytes there are
        // `TARGET_EXISTS`. Whatever occupies the path is replaced only when
        // the row's evidence proves it the row's own when it is met
        // (`core::ownership`): our stale copy, still holding what the row
        // deployed, is; a copy edited in place or a folder put where our link
        // was is asked about. The path is the row's stored one, so a
        // replacement is also claimed inside the row's managed skills folders
        // (`placement::managed_global_dirs`): a malformed record never makes
        // Propagation replace a folder elsewhere.
        let owned = row.owns_artifact();
        let deployed = Deployed::of_target(row, Some(skill));
        let target_path = Path::new(&row.target_path);
        if link_follows_source(row.mode, adapter, target_path, &deployed) {
            report.targets.push(skipped_global(
                &row.tool,
                PropagationSkip::LinkFollowsSource,
            ));
            continue;
        }

        let managed = placement::managed_global_dirs(roots, &row.tool);
        let source_missing = !central_path.is_dir();
        let result = if source_missing {
            Err(missing_source(central_path))
        } else {
            sync_engine::sync_dir_for_tool_with_overwrite(
                adapter,
                central_path,
                super::content_identity::Source::Managed { store, skill_id },
                target_path,
                Replace::if_ours(&deployed).within(&managed),
                &ProtectedLibrary::new(store, central_dir),
            )
        };

        let status = match result {
            Ok(outcome) => {
                let target_path = outcome.target_path.to_string_lossy().to_string();
                // A copy records the identity it now holds: the freshly
                // finalized one Propagation read for every target.
                let content_hash = outcome
                    .mode_used
                    .can_drift()
                    .then(|| skill.content_hash.clone())
                    .flatten();
                store.transition_skill_target(
                    &row.id,
                    TargetTransition::SyncCompleted {
                        mode: outcome.mode_used,
                        target_path: &target_path,
                        synced_at: now,
                        content_hash: content_hash.as_deref(),
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
        report.targets.push(PropagationOutcome::new(
            PropagationScope::Global {
                tool: row.tool.clone(),
            },
            status,
        ));
    }
    Ok(())
}

/// The operator's answer to the overwrite ask for one **global** row whose
/// Propagation met a folder it cannot prove its own (a `TARGET_EXISTS`
/// Propagation outcome, D12). It names the row (`skill_id`, `tool`: the
/// outcome's raw row key) **and the path the ask showed** (`path`: the
/// `TARGET_EXISTS` row's path, the row's recorded `target_path`), so the
/// answer applies to exactly the row and the folder it was asked about —
/// never to the global batch's destination for the skill, which may be
/// another folder. Crosses the wire as itself.
#[derive(Clone, Debug, serde::Deserialize, specta::Type)]
pub struct TargetDecision {
    pub skill_id: String,
    pub tool: String,
    pub path: String,
    pub decision: OverrideDecision,
}

/// Settle the overwrite ask's answers for Propagation's global rows (D12):
/// the retry half of Refresh / Update / Restore / Re-point / Edit for a
/// global Deployment. Each answer is checked against its row **now**:
///
/// - the row is gone, or no longer records the asked path (it moved since the
///   ask), or its skill is gone: `Failed` `NotFound` (`target`, the asked
///   path) — nothing is touched;
/// - the row (or a row of its folder) is `disabled`, or the skill is:
///   `Skipped` `TARGET_DISABLED`;
/// - the asked path holds the row's own artifact again (`core::ownership`):
///   no answer is needed — it is brought into line as Propagation would
///   (`Replace::if_ours`), whatever the answer;
/// - `Keep`: the folder stays; the row stays at its recorded path, `disabled`,
///   its claim given up (ADR-0007) — reported `Declined`;
/// - `Overwrite`: the folder at the asked path is replaced — at that path
///   only, admitted inside the row's managed skills folders
///   (`placement::managed_global_dirs`) — and the row records what it
///   deployed there (`SyncCompleted`, the identity of a copy), so the next
///   Propagation finds it its own. A failure settles the row `error`.
///
/// One outcome per decision, in the batch's shape (`BatchTargetOutcome`,
/// `tool_key` the row's key). Store failures fail the whole call.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
pub fn resolve_target_collisions(
    store: &SkillStore,
    paths: &InstallerPaths,
    decisions: &[TargetDecision],
    now: i64,
) -> Result<Vec<BatchTargetOutcome>> {
    mutation_guard::serialized(|| {
        decisions
            .iter()
            .map(|decision| resolve_target_collision(store, paths, decision, now))
            .collect()
    })
}

fn resolve_target_collision(
    store: &SkillStore,
    paths: &InstallerPaths,
    decision: &TargetDecision,
    now: i64,
) -> Result<BatchTargetOutcome> {
    let skill = store.get_skill_by_id(&decision.skill_id)?;
    let outcome = |status| BatchTargetOutcome {
        skill_id: decision.skill_id.clone(),
        skill_name: skill.as_ref().map(|s| s.name.clone()).unwrap_or_default(),
        tool_key: decision.tool.clone(),
        folder_key: row_folder(ScopeKind::Global, &decision.tool),
        status,
    };
    let asked = Path::new(&decision.path);
    let row = store
        .get_skill_target(&decision.skill_id, &decision.tool)?
        .filter(|row| Path::new(&row.target_path) == asked);
    let (Some(row), Some(skill)) = (row, skill.clone()) else {
        return Ok(outcome(BatchTargetStatus::Failed {
            error: CommandError::from(SignalError::NotFound {
                kind: "target".to_string(),
                id: decision.path.clone(),
            }),
        }));
    };

    let folder = row_folder(ScopeKind::Global, &row.tool);
    let folder_disabled = store.list_skill_targets(&skill.id)?.iter().any(|other| {
        other.status == SyncStatus::Disabled && row_folder(ScopeKind::Global, &other.tool) == folder
    });
    if folder_disabled || store.skill_disabled_at(&skill.id)?.is_some() {
        return Ok(outcome(BatchTargetStatus::Skipped {
            error: CommandError::TargetDisabled {
                tool: row.tool.clone(),
            },
        }));
    }
    let Some(adapter) = placement::capability_entry(ScopeKind::Global, &folder) else {
        return Ok(outcome(BatchTargetStatus::Failed {
            error: CommandError::from(SignalError::UnknownTool {
                tool: row.tool.clone(),
            }),
        }));
    };

    let deployed = Deployed::of_target(&row, Some(&skill));
    let ours = matches!(ownership::check(asked, &deployed), Ok(Occupancy::Ours));
    if !ours && decision.decision == OverrideDecision::Keep {
        store.transition_skill_target(&row.id, TargetTransition::Disabled)?;
        return Ok(outcome(BatchTargetStatus::Declined));
    }

    let managed = placement::managed_global_dirs(&paths.roots, &row.tool);
    let replace = if ours {
        Replace::if_ours(&deployed)
    } else {
        Replace::confirmed()
    }
    .within(&managed);
    let central_path = Path::new(&skill.central_path);
    let result = if central_path.is_dir() {
        sync_engine::sync_dir_for_tool_with_overwrite(
            adapter,
            central_path,
            super::content_identity::Source::Managed {
                store,
                skill_id: &skill.id,
            },
            asked,
            replace,
            &ProtectedLibrary::new(store, &paths.central_dir),
        )
    } else {
        Err(missing_source(central_path))
    };
    match result {
        Ok(synced) => {
            let content_hash = synced
                .mode_used
                .can_drift()
                .then(|| {
                    super::content_identity::read(super::content_identity::Source::Managed {
                        store,
                        skill_id: &skill.id,
                    })
                })
                .flatten();
            store.transition_skill_target(
                &row.id,
                TargetTransition::SyncCompleted {
                    mode: synced.mode_used,
                    target_path: &row.target_path,
                    synced_at: now,
                    content_hash: content_hash.as_deref(),
                },
            )?;
            Ok(outcome(BatchTargetStatus::Synced { outcome: synced }))
        }
        Err(error) => {
            let detail = format!("{:#}", error);
            store.transition_skill_target(
                &row.id,
                TargetTransition::SyncFailed { error: &detail },
            )?;
            Ok(outcome(BatchTargetStatus::Failed {
                error: CommandError::from_anyhow(error),
            }))
        }
    }
}

fn skipped_global(tool: &str, reason: PropagationSkip) -> PropagationOutcome {
    PropagationOutcome::new(
        PropagationScope::Global {
            tool: tool.to_string(),
        },
        PropagationStatus::Skipped { reason },
    )
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
        report.targets.push(PropagationOutcome::new(scope, status));
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
    // Only the link the row wrote, verified now, follows the source; any
    // other occupant takes the occupant decision, and a row that never
    // materialised gets a first sync (see the global arm).
    let deployed = Deployed::of_assignment(assignment, Some(skill));
    let artifact = resolve_assignment_artifact(&project_path, adapter, assignment, || {
        Ok(Some(skill.name.clone()))
    })?;
    if artifact
        .as_deref()
        .is_some_and(|path| link_follows_source(assignment.mode, adapter, path, &deployed))
    {
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
            // Only an artifact the row's evidence proves its own is
            // replaced, checked when it is met (see the global arm); a row
            // that never materialised gets a first sync.
            replace: Replace::if_ours(&deployed),
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
