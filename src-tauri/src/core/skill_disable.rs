//! Disable / Enable: the operator's durable "not on this Tool".
//!
//! **Disabled** (see `CONTEXT.md`) is a Sync-target state at two grains:
//!
//! * **per (skill, Tool)**, global scope only: the pair's global artifact is
//!   removed and its row kept with Sync status `disabled` — a pair with no
//!   row yet gets a `disabled` marker row, so a Tool the skill was never on
//!   can be switched off before a sync or re-assert reaches it. A marker
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
//! Every automatic pass — Propagation, the Refresh re-assert, the global sync
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
//!    cleared rows are deleted so the batch records them afresh) and
//!    `project_sync::first_sync_assignments_unlocked` for assignments — with
//!    overwrite off, so `TARGET_EXISTS` can occur and is report data.
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
use super::errors::{CommandError, SignalError};
use super::global_sync::{
    sync_skills_to_tools_unlocked, BatchPolicy, BatchSkill, BatchTargetOutcome, BatchTargetStatus,
    GlobalSyncError,
};
use super::mutation_guard;
use super::project_sync::{first_sync_assignments_unlocked, ProjectSyncReport};
use super::skill_store::{SkillRecord, SkillStore, TargetTransition};
use super::sync_status::{SyncMode, SyncStatus};
use super::tool_adapters::{adapter_by_key, adapters_sharing_skills_dir, skills_dir_in};

/// What the operator asked to disable or enable. Crosses the wire as itself:
/// `{ kind: "skill_tool", skill_id, tool }`, `{ kind: "skill", skill_id }`
/// or `{ kind: "skills", skill_ids }`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DisableScope {
    /// One (skill, Tool) pair at global scope (fanned out across the Tool's
    /// shared skills dir by `artifact_removal`).
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
    home: &Path,
    scope: &DisableScope,
    now: i64,
) -> Result<RemovalReport> {
    mutation_guard::serialized(|| disable_unlocked(store, home, scope, now))
}

/// Enable `scope`: clear the Disabled marker and re-sync as a first sync.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
pub fn enable_targets(
    store: &SkillStore,
    home: &Path,
    scope: &DisableScope,
    now: i64,
) -> Result<EnableReport> {
    mutation_guard::serialized(|| enable_unlocked(store, home, scope, now))
}

// ---------------------------------------------------------------------------
// Disable
// ---------------------------------------------------------------------------

fn disable_unlocked(
    store: &SkillStore,
    home: &Path,
    scope: &DisableScope,
    now: i64,
) -> Result<RemovalReport> {
    match scope {
        DisableScope::SkillTool { skill_id, tool } => {
            let skill = require_skill(store, skill_id)?;
            disable_pair(store, home, &skill, tool)
        }
        DisableScope::Skill { skill_id } => {
            let skill = require_skill(store, skill_id)?;
            disable_skill(store, &skill, now)
        }
        DisableScope::Skills { skill_ids } => {
            let mut reports = Vec::new();
            for skill_id in skill_ids {
                if let Some(skill) = store.get_skill_by_id(skill_id)? {
                    reports.push(disable_skill(store, &skill, now)?);
                }
            }
            Ok(RemovalReport::merge(reports))
        }
    }
}

/// One pair: plan the `SkillTool` scope, settle removed rows `disabled`, and
/// record a marker row for the named Tool when it had none.
fn disable_pair(
    store: &SkillStore,
    home: &Path,
    skill: &SkillRecord,
    tool: &str,
) -> Result<RemovalReport> {
    let adapter = adapter_by_key(tool);
    let has_row = store.get_skill_target(&skill.id, tool)?.is_some();
    // A marker needs the registry to say where the target would be; a stored
    // row of an unknown Tool still carries its own path.
    if adapter.is_none() && !has_row {
        anyhow::bail!(SignalError::UnknownTool {
            tool: tool.to_string(),
        });
    }

    let plan = artifact_removal::plan(
        store,
        &RemovalScope::SkillTool {
            skill_id: skill.id.clone(),
            tool_key: tool.to_string(),
            home: home.to_path_buf(),
        },
    )?;
    let report = execute_disable(store, plan)?;

    if let (Some(adapter), false) = (adapter, has_row) {
        // Admitted only when the shared artifact settled: a failed removal
        // keeps a sibling's row `error` with the artifact still where this
        // Tool reads it, so a `disabled` marker would assert what is false
        // (and hide the sibling's failure behind the shared-dir skip). A
        // retry that removes it records the marker then. The row may also
        // have appeared meanwhile; it is never overwritten.
        if report.failed_rows() == 0 && store.get_skill_target(&skill.id, tool)?.is_none() {
            let target_path = skills_dir_in(home, adapter).join(&skill.name);
            let mode = if adapter.supports_symlink {
                SyncMode::Symlink
            } else {
                SyncMode::Copy
            };
            store.transition_skill_target(
                &Uuid::new_v4().to_string(),
                TargetTransition::DisabledRecorded {
                    skill_id: &skill.id,
                    tool,
                    mode,
                    target_path: &target_path.to_string_lossy(),
                },
            )?;
        }
    }
    Ok(report)
}

/// One skill everywhere: its global targets, then one `ProjectSkill` scope per
/// project it is assigned in, each executed with the Disable settlement; then
/// the skill-level marker. The marker records the operator's intent and is
/// set even when a removal failed — the failed rows stay `error` (retryable
/// by disabling again) and no automatic pass re-materialises the rest.
fn disable_skill(store: &SkillStore, skill: &SkillRecord, now: i64) -> Result<RemovalReport> {
    let mut reports = vec![execute_disable(
        store,
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
        reports.push(execute_disable(store, plan)?);
    }
    if store.skill_disabled_at(&skill.id)?.is_none() {
        store.set_skill_disabled_at(&skill.id, Some(now))?;
    }
    Ok(RemovalReport::merge(reports))
}

/// Execute a plan with the Disable settlement, after dropping rows that are
/// already `disabled`: disabling twice is a no-op, and the report counts only
/// the targets this call settled.
fn execute_disable(store: &SkillStore, mut plan: RemovalPlan) -> Result<RemovalReport> {
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
    artifact_removal::execute_settling_unlocked(store, plan, Settlement::Disable)
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
    home: &Path,
    scope: &DisableScope,
    now: i64,
) -> Result<EnableReport> {
    match scope {
        DisableScope::SkillTool { skill_id, tool } => {
            let skill = require_skill(store, skill_id)?;
            enable_pair(store, home, &skill, tool, now)
        }
        DisableScope::Skill { skill_id } => {
            let skill = require_skill(store, skill_id)?;
            enable_skill(store, home, &skill, now)
        }
        DisableScope::Skills { skill_ids } => {
            let mut report = EnableReport::default();
            for skill_id in skill_ids {
                if let Some(skill) = store.get_skill_by_id(skill_id)? {
                    let one = enable_skill(store, home, &skill, now)?;
                    report.global.extend(one.global);
                    report.projects.items.extend(one.projects.items);
                }
            }
            Ok(report)
        }
    }
}

/// One pair. A skill disabled at skill grain stays off: only a skill-grain
/// Enable lifts that, so the pair is reported skipped and nothing changes.
/// Otherwise the disabled rows of the Tool's shared-dir group are cleared
/// (the batch's Disabled rule would otherwise still skip the shared artifact)
/// and the pair is synced as a first sync.
fn enable_pair(
    store: &SkillStore,
    home: &Path,
    skill: &SkillRecord,
    tool: &str,
    now: i64,
) -> Result<EnableReport> {
    if store.skill_disabled_at(&skill.id)?.is_some() {
        return Ok(EnableReport {
            global: vec![BatchTargetOutcome {
                skill_id: skill.id.clone(),
                skill_name: skill.name.clone(),
                tool_key: tool.to_string(),
                status: BatchTargetStatus::Skipped {
                    error: CommandError::from(GlobalSyncError::TargetDisabled {
                        tool_key: tool.to_string(),
                    }),
                },
            }],
            projects: ProjectSyncReport::default(),
        });
    }
    let group: Vec<String> = match adapter_by_key(tool) {
        Some(adapter) => {
            let mut keys: Vec<String> = adapters_sharing_skills_dir(adapter)
                .into_iter()
                .map(|a| a.key().to_string())
                .collect();
            if !keys.iter().any(|key| key == tool) {
                keys.push(tool.to_string());
            }
            keys
        }
        None => vec![tool.to_string()],
    };
    for row in store.list_skill_targets(&skill.id)? {
        if row.status == SyncStatus::Disabled && group.contains(&row.tool) {
            store.delete_skill_target(&skill.id, &row.tool)?;
        }
    }
    Ok(EnableReport {
        global: first_sync_global(store, home, skill, &[tool.to_string()], now),
        projects: ProjectSyncReport::default(),
    })
}

/// One skill: clear the skill-level marker, then re-sync every disabled row —
/// the global ones through the batch (rows deleted first so the batch records
/// them afresh), the assignments through `project_sync`.
fn enable_skill(
    store: &SkillStore,
    home: &Path,
    skill: &SkillRecord,
    now: i64,
) -> Result<EnableReport> {
    store.set_skill_disabled_at(&skill.id, None)?;

    let mut tools = Vec::new();
    for row in store.list_skill_targets(&skill.id)? {
        if row.status == SyncStatus::Disabled {
            store.delete_skill_target(&skill.id, &row.tool)?;
            tools.push(row.tool);
        }
    }
    let assignments: Vec<_> = store
        .list_project_skill_assignments_by_skill(&skill.id)?
        .into_iter()
        .filter(|assignment| assignment.status == SyncStatus::Disabled)
        .collect();

    Ok(EnableReport {
        global: if tools.is_empty() {
            Vec::new()
        } else {
            first_sync_global(store, home, skill, &tools, now)
        },
        projects: first_sync_assignments_unlocked(store, &assignments, now)?,
    })
}

/// The global sync batch as a first sync: overwrite off, identical content
/// replaced silently (the same policy as every operator-initiated sync).
fn first_sync_global(
    store: &SkillStore,
    home: &Path,
    skill: &SkillRecord,
    tools: &[String],
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
        overrides: Vec::new(),
    };
    sync_skills_to_tools_unlocked(home, store, &skills, tools, &policy, now, |_| {})
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
