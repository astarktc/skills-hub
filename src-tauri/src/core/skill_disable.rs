//! Disable / Enable: the operator's durable "not on this Tool".
//!
//! **Disabled** (see `CONTEXT.md`) is a Sync-target state at two grains:
//!
//! * **per (skill, folder)**, global scope only: the Deployment folder serving
//!   the named harness (`core::placement`) has its artifact removed and its
//!   row kept with Sync status `disabled` — a folder with no row yet gets a
//!   `disabled` marker row keyed by the folder, so a Tool the skill was never
//!   on can be switched off before a Deploy reaches it. A marker
//!   owns nothing at its path (`SkillTargetRecord::owns_artifact`: only a
//!   successful sync of ours, recorded as `synced_at`, makes a row an owner,
//!   whatever its status), so no removal (Delete included) touches that path
//!   and no re-sync overwrites it, even after a failed Enable turned the row
//!   `error`; a marker is admitted only once the Tool's shared artifact
//!   settled;
//! * **per skill**: every global target *and* every project assignment of the
//!   skill is removed and kept `disabled`, and `skills.disabled_at` records
//!   "off everywhere until enabled" — a new assignment made meanwhile is
//!   recorded `disabled` too (`project_sync`).
//!
//! Every automatic pass — Propagation, the global sync
//! batch (hence Onboarding import's first sync), project sync, reconcile and
//! target relocation — skips a disabled target and reports it `Skipped`, never
//! `Failed`. Nothing but [`enable_targets`] re-materialises it.
//!
//! Two rules decide everything here:
//!
//! 1. **Removal goes through `artifact_removal`.** Disable plans the existing
//!    scopes (`SkillTool`; `SkillGlobal` + one `ProjectSkill` per project) and
//!    executes them with [`Settlement::Disable`]: a row flips to `disabled`
//!    only after its artifact is gone; a failed removal keeps the row `error`
//!    (ADR-0002) so the operator can retry. A row owning no artifact (never
//!    materialised, e.g. `error` after a failed Enable) flips to `disabled`
//!    with no path touched. This module never removes a path.
//! 2. **Enable is a first sync again.** It clears the marker and re-syncs
//!    through the existing batches — `global_sync` for global targets (the
//!    disabled rows in scope are deleted and carried to the batch as an
//!    in-memory first-sync plan of their own keys, so it records each afresh
//!    under its own key and touches no other row of their folders) and
//!    `project_sync::first_sync_assignments_unlocked` for assignments — with
//!    overwrite off, so `TARGET_EXISTS` can occur and is report data. A
//!    global pair whose folder is in the way stays `disabled` until the
//!    operator answers the one overwrite ask (D12) — the retry is the same
//!    scope with per-pair decisions ([`enable_targets_with`]); any other
//!    failed pair is the batch's `error` row; an assignment keeps its row
//!    `error` and is retried through `project_sync::resolve_collisions`.
//!    Rows a pre-1.4.0 app wrote once per shared-dir sibling are each their
//!    own row here: nothing is lent or settled between them.
//!
//! Both answer with report types that cross the wire as themselves:
//! [`RemovalReport`] for Disable, [`EnableReport`] for Enable. Only a store
//! failure or an unknown skill / Tool fails the whole operation.
//!
//! Locking: [`disable_targets`] and [`enable_targets`] are mutation entry
//! points (they take the guard); everything they call is an unlocked seam.
//! Enable of a whole skill does not lift a *pair* disable recorded before the
//! skill was disabled separately: both are the same `disabled` row, so the
//! skill grain re-enables every disabled row of the skill.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Result;
use uuid::Uuid;

use super::artifact_removal::{self, RemovalPlan, RemovalReport, RemovalScope, RowRef, Settlement};
use super::environment::HostRoots;
use super::errors::{CommandError, SignalError};
use super::global_sync::{
    first_sync_rows_unlocked, sync_skills_to_tools_unlocked, BatchOverride, BatchPolicy,
    BatchSkill, BatchTargetOutcome, BatchTargetStatus, FirstSyncGrain, FirstSyncRow,
    GlobalSyncError,
};
use super::mutation_guard;
use super::placement::{self, row_folder, PlacementSettings, ScopeKind};
use super::project_sync::{first_sync_assignments_unlocked, ProjectSyncReport};
use super::skill_store::{SkillRecord, SkillStore, TargetTransition};
use super::sync_status::SyncStatus;
use super::tool_adapters::{adapter_by_key, skills_dir_in};

/// What the operator asked to disable or enable. Crosses the wire as itself:
/// `{ kind: "skill_tool", skill_id, tool }`, `{ kind: "skill", skill_id }`
/// or `{ kind: "skills", skill_ids }`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DisableScope {
    /// One (skill, folder) pair at global scope: `tool` names a harness (or a
    /// folder key), and the Deployment folder serving it is what turns off —
    /// one folder, one row (`core::placement`).
    SkillTool { skill_id: String, tool: String },
    /// One skill, everywhere: global targets and project assignments.
    Skill { skill_id: String },
    /// Several skills at skill grain, in order. An id with no skill row is
    /// dropped (the listing that produced it may be stale).
    Skills { skill_ids: Vec<String> },
}

/// What Enable re-synced.
#[derive(Debug, Default, serde::Serialize, specta::Type)]
pub struct EnableReport {
    /// Global targets, as the global sync batch reported them (a first sync:
    /// `TARGET_EXISTS` is a `failed` row, a skill-disabled pair a `skipped`
    /// `TARGET_DISABLED`).
    pub global: Vec<BatchTargetOutcome>,
    /// Project assignments re-synced by a skill-grain Enable.
    pub projects: ProjectSyncReport,
}

/// Disable `scope`: remove its artifacts and keep their rows `disabled`.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
pub fn disable_targets(
    store: &SkillStore,
    central_dir: &Path,
    roots: &HostRoots,
    scope: &DisableScope,
    now: i64,
) -> Result<RemovalReport> {
    mutation_guard::serialized(|| disable_unlocked(store, central_dir, roots, scope, now))
}

/// Enable `scope`: clear the Disabled marker and re-sync as a first sync.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
/// Without overrides — the first Enable; test fixtures' door (the command
/// always states its overrides through [`enable_targets_with`]).
#[cfg(test)]
pub fn enable_targets(
    store: &SkillStore,
    central_dir: &Path,
    roots: &HostRoots,
    scope: &DisableScope,
    now: i64,
) -> Result<EnableReport> {
    enable_targets_with(store, central_dir, roots, scope, &[], now)
}

/// [`enable_targets`] carrying the operator's answers to the overwrite ask
/// (D12) for global pairs whose first sync met a folder in the way: the ask
/// is raised over the first Enable's `TARGET_EXISTS` rows, and its retry is
/// the same scope again with per-pair decisions — `Overwrite` replaces the
/// folder, `Keep` leaves the pair `disabled` and reports it `Declined`.
/// Project assignments are retried through `project_sync`'s collision
/// resolution (their rows are `error`, not `disabled`).
///
/// Mutation entry point: serialised against every other Sync-target mutation.
pub fn enable_targets_with(
    store: &SkillStore,
    central_dir: &Path,
    roots: &HostRoots,
    scope: &DisableScope,
    overrides: &[BatchOverride],
    now: i64,
) -> Result<EnableReport> {
    mutation_guard::serialized(|| enable_unlocked(store, central_dir, roots, scope, overrides, now))
}

// ---------------------------------------------------------------------------
// Disable
// ---------------------------------------------------------------------------

fn disable_unlocked(
    store: &SkillStore,
    central_dir: &Path,
    roots: &HostRoots,
    scope: &DisableScope,
    now: i64,
) -> Result<RemovalReport> {
    match scope {
        DisableScope::SkillTool { skill_id, tool } => {
            let skill = require_skill(store, skill_id)?;
            disable_pair(store, central_dir, roots, &skill, tool)
        }
        DisableScope::Skill { skill_id } => {
            let skill = require_skill(store, skill_id)?;
            disable_skill(store, central_dir, &skill, now)
        }
        DisableScope::Skills { skill_ids } => {
            let mut reports = Vec::new();
            for skill_id in skill_ids {
                if let Some(skill) = store.get_skill_by_id(skill_id)? {
                    reports.push(disable_skill(store, central_dir, &skill, now)?);
                }
            }
            Ok(RemovalReport::merge(reports))
        }
    }
}

/// The global folder `tool` names: the folder serving it when the registry
/// knows it as a harness (`placement::serving_folder`), else the folder its
/// rows describe (an unknown key stands for itself).
fn global_folder(tool: &str) -> String {
    placement::serving_folder(ScopeKind::Global, tool, &PlacementSettings::default())
        .map(str::to_string)
        .unwrap_or_else(|| row_folder(ScopeKind::Global, tool))
}

/// Whether the skill has any row describing the global folder `folder`.
fn has_folder_row(store: &SkillStore, skill_id: &str, folder: &str) -> Result<bool> {
    Ok(store
        .list_skill_targets(skill_id)?
        .iter()
        .any(|row| row_folder(ScopeKind::Global, &row.tool) == folder))
}

/// One pair: plan the `SkillTool` scope over the folder serving `tool`,
/// settle removed rows `disabled`, and record a marker row for the folder
/// when it had none.
fn disable_pair(
    store: &SkillStore,
    central_dir: &Path,
    roots: &HostRoots,
    skill: &SkillRecord,
    tool: &str,
) -> Result<RemovalReport> {
    let folder = global_folder(tool);
    let entry = adapter_by_key(&folder);
    let has_row = has_folder_row(store, &skill.id, &folder)?;
    // A marker needs the registry to say where the target would be; a stored
    // row of an unknown Tool still carries its own path.
    if entry.is_none() && !has_row {
        anyhow::bail!(SignalError::UnknownTool {
            tool: tool.to_string(),
        });
    }

    let plan = artifact_removal::plan(
        store,
        &RemovalScope::SkillTool {
            skill_id: skill.id.clone(),
            tool_key: folder.clone(),
            roots: roots.clone(),
        },
    )?;
    let mut report = execute_disable(store, central_dir, plan)?;

    if let (Some(entry), false) = (entry, has_row) {
        // Admitted only when the folder's artifact settled: a failed removal
        // keeps a row `error` with the artifact still where this folder's
        // harnesses read it, so a `disabled` marker would assert what is
        // false. A retry that removes it records the marker then. The row may
        // also have appeared meanwhile; it is never overwritten.
        if report.failed_rows() == 0 && !has_folder_row(store, &skill.id, &folder)? {
            let target_path = skills_dir_in(roots, entry).join(&skill.name);
            let mode = placement::folder_mode(ScopeKind::Global, &folder).first_attempt();
            let id = Uuid::new_v4().to_string();
            store.transition_skill_target(
                &id,
                TargetTransition::DisabledRecorded {
                    skill_id: &skill.id,
                    tool: &folder,
                    mode,
                    target_path: &target_path.to_string_lossy(),
                },
            )?;
            report
                .disabled_markers
                .push(RowRef::global(&id, &skill.id, &folder));
        }
    }
    Ok(report)
}

/// One skill everywhere: its global targets, then one `ProjectSkill` scope per
/// project it is assigned in, each executed with the Disable settlement; then
/// the skill-level marker. The marker records the operator's intent and is
/// set even when a removal failed — the failed rows stay `error` (retryable
/// by disabling again) and no automatic pass re-materialises the rest.
fn disable_skill(
    store: &SkillStore,
    central_dir: &Path,
    skill: &SkillRecord,
    now: i64,
) -> Result<RemovalReport> {
    let mut reports = vec![execute_disable(
        store,
        central_dir,
        artifact_removal::plan(
            store,
            &RemovalScope::SkillGlobal {
                skill_id: skill.id.clone(),
            },
        )?,
    )?];
    let projects: BTreeSet<String> = store
        .list_project_skill_assignments_by_skill(&skill.id)?
        .into_iter()
        .map(|assignment| assignment.project_id)
        .collect();
    for project_id in projects {
        let plan = artifact_removal::plan(
            store,
            &RemovalScope::ProjectSkill {
                project_id,
                skill_id: skill.id.clone(),
            },
        )?;
        reports.push(execute_disable(store, central_dir, plan)?);
    }
    if store.skill_disabled_at(&skill.id)?.is_none() {
        store.set_skill_disabled_at(&skill.id, Some(now))?;
    }
    Ok(RemovalReport::merge(reports))
}

/// Execute a plan with the Disable settlement, after dropping rows that are
/// already `disabled`: disabling twice is a no-op, and the report counts only
/// the targets and markers this call settled.
fn execute_disable(
    store: &SkillStore,
    central_dir: &Path,
    mut plan: RemovalPlan,
) -> Result<RemovalReport> {
    let mut markers = Vec::with_capacity(plan.markers.len());
    for marker in plan.markers.drain(..) {
        if !row_is_disabled(store, &marker)? {
            markers.push(marker);
        }
    }
    plan.markers = markers;
    for target in &mut plan.targets {
        let mut kept = Vec::with_capacity(target.rows.len());
        for row in target.rows.drain(..) {
            if !row_is_disabled(store, &row)? {
                kept.push(row);
            }
        }
        target.rows = kept;
    }
    plan.targets.retain(|target| !target.rows.is_empty());
    artifact_removal::execute_settling_unlocked(store, central_dir, plan, Settlement::Disable)
}

fn row_is_disabled(store: &SkillStore, row: &RowRef) -> Result<bool> {
    let status = match row {
        RowRef::GlobalTarget { skill_id, tool, .. } => store
            .get_skill_target(skill_id, tool)?
            .map(|record| record.status),
        RowRef::Assignment {
            project_id,
            skill_id,
            tool,
            ..
        } => store
            .get_project_skill_assignment(project_id, skill_id, tool)?
            .map(|record| record.status),
    };
    Ok(status == Some(SyncStatus::Disabled))
}

// ---------------------------------------------------------------------------
// Enable
// ---------------------------------------------------------------------------

fn enable_unlocked(
    store: &SkillStore,
    central_dir: &Path,
    roots: &HostRoots,
    scope: &DisableScope,
    overrides: &[BatchOverride],
    now: i64,
) -> Result<EnableReport> {
    match scope {
        DisableScope::SkillTool { skill_id, tool } => {
            let skill = require_skill(store, skill_id)?;
            enable_pair(store, central_dir, roots, &skill, tool, overrides, now)
        }
        DisableScope::Skill { skill_id } => {
            let skill = require_skill(store, skill_id)?;
            enable_skill(store, central_dir, roots, &skill, overrides, now)
        }
        DisableScope::Skills { skill_ids } => {
            let mut report = EnableReport::default();
            for skill_id in skill_ids {
                if let Some(skill) = store.get_skill_by_id(skill_id)? {
                    let one = enable_skill(store, central_dir, roots, &skill, overrides, now)?;
                    report.global.extend(one.global);
                    report.projects.absorb(one.projects);
                }
            }
            Ok(report)
        }
    }
}

/// One pair. A skill disabled at skill grain stays off: only a skill-grain
/// Enable lifts that, so the pair is reported skipped and nothing changes.
/// Otherwise every `disabled` row of the folder serving `tool` is taken out
/// of the store and planned for a first sync under its own key
/// ([`take_disabled`]), as Enable always cleared and re-synced them; a folder
/// with no disabled row is synced as an ordinary first sync.
fn enable_pair(
    store: &SkillStore,
    central_dir: &Path,
    roots: &HostRoots,
    skill: &SkillRecord,
    tool: &str,
    overrides: &[BatchOverride],
    now: i64,
) -> Result<EnableReport> {
    let folder = global_folder(tool);
    if store.skill_disabled_at(&skill.id)?.is_some() {
        return Ok(EnableReport {
            global: vec![BatchTargetOutcome {
                skill_id: skill.id.clone(),
                skill_name: skill.name.clone(),
                tool_key: folder.clone(),
                folder_key: folder.clone(),
                status: BatchTargetStatus::Skipped {
                    error: CommandError::from(GlobalSyncError::TargetDisabled {
                        tool_key: folder.clone(),
                    }),
                },
            }],
            projects: ProjectSyncReport::default(),
        });
    }
    let plan = take_disabled(store, &skill.id, |row_folder| row_folder == folder)?;
    Ok(EnableReport {
        global: first_sync_global(
            store,
            central_dir,
            roots,
            skill,
            FirstSync::Pair {
                folder: &folder,
                plan: &plan,
            },
            overrides,
            now,
        ),
        projects: ProjectSyncReport::default(),
    })
}

/// Take every `disabled` global row of the skill whose folder `wanted`
/// selects out of the store — a successful Disable removed its artifact, and
/// the batch records it afresh, as Enable always did — and answer the
/// in-memory first-sync plan that carries each row's identity (its own key)
/// to the batch, so the batch re-records exactly those rows and never another
/// row of their folders.
fn take_disabled(
    store: &SkillStore,
    skill_id: &str,
    wanted: impl Fn(&str) -> bool,
) -> Result<Vec<FirstSyncRow>> {
    let mut plan = Vec::new();
    for row in store.list_skill_targets(skill_id)? {
        if row.status != SyncStatus::Disabled || !wanted(&row_folder(ScopeKind::Global, &row.tool))
        {
            continue;
        }
        store.delete_skill_target(skill_id, &row.tool)?;
        plan.push(FirstSyncRow {
            skill_id: skill_id.to_string(),
            key: row.tool,
        });
    }
    Ok(plan)
}

/// One skill: clear the skill-level marker, then re-sync every disabled row —
/// the global ones through the batch ([`take_disabled`] first), the
/// assignments through `project_sync` — each row by itself.
fn enable_skill(
    store: &SkillStore,
    central_dir: &Path,
    roots: &HostRoots,
    skill: &SkillRecord,
    overrides: &[BatchOverride],
    now: i64,
) -> Result<EnableReport> {
    store.set_skill_disabled_at(&skill.id, None)?;

    let plan = take_disabled(store, &skill.id, |_| true)?;
    let assignments: Vec<_> = store
        .list_project_skill_assignments_by_skill(&skill.id)?
        .into_iter()
        .filter(|assignment| assignment.status == SyncStatus::Disabled)
        .collect();

    Ok(EnableReport {
        global: if plan.is_empty() {
            Vec::new()
        } else {
            first_sync_global(
                store,
                central_dir,
                roots,
                skill,
                FirstSync::Rows(&plan),
                overrides,
                now,
            )
        },
        projects: first_sync_assignments_unlocked(store, central_dir, &assignments, now)?,
    })
}

/// What Enable's global first sync acts on.
enum FirstSync<'a> {
    /// The rows [`take_disabled`] took, each under its own key.
    Rows(&'a [FirstSyncRow]),
    /// A pair: its taken rows ([`FirstSyncGrain::Pair`]: the folder turns
    /// back on), or — none were disabled — the folder as an ordinary first
    /// sync.
    Pair {
        folder: &'a str,
        plan: &'a [FirstSyncRow],
    },
}

/// The global sync batch as a first sync: overwrite off, identical content
/// replaced silently (the same policy as every operator-initiated sync),
/// plus the operator's per-row answers to an earlier ask for this skill.
fn first_sync_global(
    store: &SkillStore,
    central_dir: &Path,
    roots: &HostRoots,
    skill: &SkillRecord,
    what: FirstSync<'_>,
    overrides: &[BatchOverride],
    now: i64,
) -> Vec<BatchTargetOutcome> {
    let skills = [BatchSkill {
        skill_id: skill.id.clone(),
        skill_name: skill.name.clone(),
        source_path: std::path::PathBuf::from(&skill.central_path),
    }];
    let policy = BatchPolicy {
        overwrite: false,
        overwrite_if_same_content: true,
        overrides: overrides
            .iter()
            .filter(|o| o.skill_id == skill.id)
            .cloned()
            .collect(),
    };
    let outcomes = match what {
        FirstSync::Rows(plan) if !plan.is_empty() => first_sync_rows_unlocked(
            roots,
            store,
            central_dir,
            &skills,
            plan,
            FirstSyncGrain::Skill,
            &policy,
            now,
        ),
        FirstSync::Pair { plan, .. } if !plan.is_empty() => first_sync_rows_unlocked(
            roots,
            store,
            central_dir,
            &skills,
            plan,
            FirstSyncGrain::Pair,
            &policy,
            now,
        ),
        FirstSync::Rows(_) => Vec::new(),
        FirstSync::Pair { folder, .. } => sync_skills_to_tools_unlocked(
            roots,
            store,
            central_dir,
            &skills,
            &[folder.to_string()],
            &policy,
            now,
            |_| {},
        ),
    };
    // A folder in the way is asked about, never recorded as a failure (D9);
    // until the operator answers, the row keeps the state it was enabled from:
    // `disabled`, re-recorded as a marker owning nothing at that path (the
    // taken row was deleted so the batch could record afresh). Any other
    // failure is the batch's own `error` row.
    for outcome in &outcomes {
        if let BatchTargetStatus::Failed {
            error: CommandError::TargetExists { path, .. },
        } = &outcome.status
        {
            let mode =
                placement::folder_mode(ScopeKind::Global, &outcome.folder_key).first_attempt();
            if let Err(err) = store.transition_skill_target(
                &Uuid::new_v4().to_string(),
                TargetTransition::DisabledRecorded {
                    skill_id: &skill.id,
                    tool: &outcome.tool_key,
                    mode,
                    target_path: path,
                },
            ) {
                log::warn!(
                    "keeping {} disabled on {} after a collision failed: {err:#}",
                    skill.id,
                    outcome.tool_key
                );
            }
        }
    }
    outcomes
}

fn require_skill(store: &SkillStore, skill_id: &str) -> Result<SkillRecord> {
    store.get_skill_by_id(skill_id)?.ok_or_else(|| {
        anyhow::anyhow!(SignalError::NotFound {
            kind: "skill".to_string(),
            id: skill_id.to_string(),
        })
    })
}

#[cfg(test)]
#[path = "tests/skill_disable.rs"]
mod tests;
