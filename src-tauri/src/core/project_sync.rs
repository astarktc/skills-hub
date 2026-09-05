use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::core::{
    artifact_removal, content_hash,
    errors::SignalError,
    mutation_guard, project_ops,
    skill_store::{
        AssignmentTransition, ProjectRecord, ProjectSkillAssignmentRecord, SkillRecord, SkillStore,
    },
    sync_engine,
    sync_status::{next_status, Observation, SyncMode, SyncStatus},
    tool_adapters::{self, ToolAdapter},
};

/// The single place that joins a project root with a tool's skills dir.
///
/// Takes the adapter (not a bare dir string) so the project-scope mapping
/// (`ToolAdapter::project_relative_skills_dir`) is chosen here and callers cannot reach for
/// the global `relative_skills_dir` by mistake — that mix-up has shipped more
/// than once (see `gitignore.rs` and the cleanup paths in `project_ops.rs`).
pub fn resolve_project_sync_target(
    project_path: &Path,
    adapter: &ToolAdapter,
    skill_name: &str,
) -> PathBuf {
    project_path
        .join(adapter.project_relative_skills_dir)
        .join(skill_name)
}

/// Unlocked internal seam: callers reach it through an entry point that has
/// already taken the mutation guard (`mutation_guard`).
pub(crate) fn assign_and_sync(
    store: &SkillStore,
    project: &ProjectRecord,
    skill: &SkillRecord,
    tool_key: &str,
    now: i64,
) -> Result<ProjectSkillAssignmentRecord> {
    let adapter = tool_adapters::adapter_by_key(tool_key).ok_or_else(|| {
        anyhow::anyhow!(SignalError::UnknownTool {
            tool: tool_key.to_string(),
        })
    })?;

    let record = ProjectSkillAssignmentRecord {
        id: uuid::Uuid::new_v4().to_string(),
        project_id: project.id.clone(),
        skill_id: skill.id.clone(),
        skill_name: skill.name.clone(),
        tool: tool_key.to_string(),
        mode: SyncMode::Symlink,
        status: SyncStatus::Pending,
        last_error: None,
        synced_at: None,
        content_hash: None,
        created_at: now,
    };
    store.add_project_skill_assignment(&record)?;

    let source = Path::new(&skill.central_path);
    let target = resolve_project_sync_target(Path::new(&project.path), adapter, &skill.name);

    match sync_engine::sync_dir_for_tool_with_overwrite(adapter, source, &target, false) {
        Ok(outcome) => {
            let hash = hash_after_sync(outcome.mode_used, source);
            store.transition_assignment(
                &record.id,
                AssignmentTransition::SyncCompleted {
                    mode: outcome.mode_used,
                    synced_at: now,
                    content_hash: hash.as_deref(),
                },
            )?;
            let updated = store
                .get_project_skill_assignment(&project.id, &skill.id, tool_key)?
                .unwrap_or(record);
            Ok(updated)
        }
        Err(e) => {
            let err_msg = format!("{:#}", e);
            store.transition_assignment(
                &record.id,
                AssignmentTransition::SyncFailed { error: &err_msg },
            )?;
            let updated = store
                .get_project_skill_assignment(&project.id, &skill.id, tool_key)?
                .unwrap_or(record);
            Ok(updated)
        }
    }
}

/// The source content hash to record after a successful sync: only copies
/// can drift, so links record nothing. A hashing failure is logged and
/// leaves the hash unknown (the next reconcile pass then reports `Stale`).
fn hash_after_sync(mode_used: SyncMode, source: &Path) -> Option<String> {
    if !mode_used.can_drift() {
        return None;
    }
    match content_hash::hash_dir(source) {
        Ok(h) => Some(h),
        Err(e) => {
            log::warn!("failed to compute content hash after sync: {:#}", e);
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Project-scope fan-out: one skill × N tools with per-target outcomes as
// data (the project counterpart of `global_sync::sync_skills_to_planned_tools`).
// `assign_skill_to_tools` is the deterministic engine; the two `*_project_*`
// entry points add the store lookups (typed `NotFound`) in front of it.
// ---------------------------------------------------------------------------

/// Per-tool result of a fan-out. A *sync* failure is not a fan-out failure:
/// `assign_and_sync` records it on the assignment row (`SyncStatus::Error`),
/// so it arrives as `Assigned` with that record. `Failed` is reserved for
/// the assignment itself not happening (unknown tool, store error).
#[derive(Debug)]
pub enum AssignTargetStatus {
    Assigned {
        record: Box<ProjectSkillAssignmentRecord>,
    },
    AlreadyAssigned,
    Failed {
        error: anyhow::Error,
    },
}

#[derive(Debug)]
pub struct AssignTargetOutcome {
    pub tool_key: String,
    pub status: AssignTargetStatus,
}

/// Deterministic engine: for each tool key in caller order, skip tools the
/// skill is already assigned to, otherwise assign and sync. Failures are
/// isolated per tool — one bad tool never aborts the batch.
pub(crate) fn assign_skill_to_tools(
    store: &SkillStore,
    project: &ProjectRecord,
    skill: &SkillRecord,
    tool_keys: &[String],
    now: i64,
) -> Vec<AssignTargetOutcome> {
    tool_keys
        .iter()
        .map(|tool_key| {
            let status = match store.get_project_skill_assignment(&project.id, &skill.id, tool_key)
            {
                Ok(Some(_)) => AssignTargetStatus::AlreadyAssigned,
                Ok(None) => match assign_and_sync(store, project, skill, tool_key, now) {
                    Ok(record) => AssignTargetStatus::Assigned {
                        record: Box::new(record),
                    },
                    Err(error) => AssignTargetStatus::Failed { error },
                },
                Err(error) => AssignTargetStatus::Failed { error },
            };
            AssignTargetOutcome {
                tool_key: tool_key.clone(),
                status,
            }
        })
        .collect()
}

/// The one "this project and this skill must both exist" lookup, raising the
/// typed `NotFound` for whichever is missing. Public so no command body has
/// to re-derive the idiom.
pub fn lookup_project_and_skill(
    store: &SkillStore,
    project_id: &str,
    skill_id: &str,
) -> Result<(ProjectRecord, SkillRecord)> {
    let project = project_ops::require_project(store, project_id)?;
    let skill = store.get_skill_by_id(skill_id)?.ok_or_else(|| {
        anyhow::anyhow!(SignalError::NotFound {
            kind: "skill".to_string(),
            id: skill_id.to_string(),
        })
    })?;
    Ok((project, skill))
}

/// Assign one skill to every tool persisted for the project (the
/// `bulk_assign_skill` command). Only the lookups can error; per-tool
/// results are data.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
/// No composite operation composes it, so it has no unlocked seam.
pub fn assign_skill_to_project_tools(
    store: &SkillStore,
    project_id: &str,
    skill_id: &str,
    now: i64,
) -> Result<Vec<AssignTargetOutcome>> {
    mutation_guard::serialized(|| {
        let (project, skill) = lookup_project_and_skill(store, project_id, skill_id)?;
        let tool_keys: Vec<String> = store
            .list_project_tools(project_id)?
            .into_iter()
            .map(|t| t.tool)
            .collect();
        Ok(assign_skill_to_tools(
            store, &project, &skill, &tool_keys, now,
        ))
    })
}

/// Assign one skill to one tool: the single-target view of the same engine,
/// where `AlreadyAssigned` is the typed `AssignmentExists` condition.
///
/// Unlocked internal seam — [`toggle_skill_assignment`] is the entry point
/// that takes the guard around it.
pub(crate) fn assign_skill_to_project_tool_unlocked(
    store: &SkillStore,
    project_id: &str,
    skill_id: &str,
    tool_key: &str,
    now: i64,
) -> Result<ProjectSkillAssignmentRecord> {
    let (project, skill) = lookup_project_and_skill(store, project_id, skill_id)?;
    let outcome = assign_skill_to_tools(store, &project, &skill, &[tool_key.to_string()], now)
        .pop()
        .expect("one tool key yields one outcome");
    match outcome.status {
        AssignTargetStatus::Assigned { record } => Ok(*record),
        AssignTargetStatus::AlreadyAssigned => {
            Err(anyhow::anyhow!(SignalError::AssignmentExists {
                project: project_id.to_string(),
                skill: skill_id.to_string(),
                tool: tool_key.to_string(),
            }))
        }
        AssignTargetStatus::Failed { error } => Err(error),
    }
}

pub struct ResyncSummary {
    pub project_id: String,
    pub synced: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

pub(crate) fn sync_single_assignment(
    store: &SkillStore,
    project: &ProjectRecord,
    assignment: &ProjectSkillAssignmentRecord,
    overwrite: bool,
    now: i64,
) -> Result<()> {
    let skill = store
        .get_skill_by_id(&assignment.skill_id)?
        .ok_or_else(|| {
            anyhow::anyhow!(SignalError::NotFound {
                kind: "skill".to_string(),
                id: assignment.skill_id.clone(),
            })
        })?;
    let adapter = tool_adapters::adapter_by_key(&assignment.tool).ok_or_else(|| {
        anyhow::anyhow!(SignalError::UnknownTool {
            tool: assignment.tool.clone(),
        })
    })?;

    let source = Path::new(&skill.central_path);
    let target = resolve_project_sync_target(Path::new(&project.path), adapter, &skill.name);

    let outcome =
        sync_engine::sync_dir_for_tool_with_overwrite(adapter, source, &target, overwrite)?;

    let hash = hash_after_sync(outcome.mode_used, source);
    store.transition_assignment(
        &assignment.id,
        AssignmentTransition::SyncCompleted {
            mode: outcome.mode_used,
            synced_at: now,
            content_hash: hash.as_deref(),
        },
    )?;

    Ok(())
}

/// Re-materialise every Sync target of one project.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
pub fn resync_project(store: &SkillStore, project_id: &str, now: i64) -> Result<ResyncSummary> {
    mutation_guard::serialized(|| resync_project_unlocked(store, project_id, now))
}

pub(crate) fn resync_project_unlocked(
    store: &SkillStore,
    project_id: &str,
    now: i64,
) -> Result<ResyncSummary> {
    let project = project_ops::require_project(store, project_id)?;
    let assignments = store.list_project_skill_assignments(project_id)?;
    let mut summary = ResyncSummary {
        project_id: project_id.to_string(),
        synced: 0,
        failed: 0,
        errors: vec![],
    };

    for assignment in &assignments {
        match sync_single_assignment(store, &project, assignment, true, now) {
            Ok(()) => summary.synced += 1,
            Err(e) => {
                let err_msg = format!("{}: {:#}", assignment.id, e);
                let _ = store.transition_assignment(
                    &assignment.id,
                    AssignmentTransition::SyncFailed {
                        error: &format!("{:#}", e),
                    },
                );
                summary.failed += 1;
                summary.errors.push(err_msg);
            }
        }
    }

    Ok(summary)
}

/// Re-materialise every Sync target of every project.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
/// The per-project step is the unlocked seam — the guard is not reentrant.
/// Nothing composes the whole batch, so it has no unlocked seam of its own.
pub fn resync_all_projects(store: &SkillStore, now: i64) -> Result<Vec<ResyncSummary>> {
    mutation_guard::serialized(|| {
        let projects = store.list_projects()?;
        let mut summaries = Vec::with_capacity(projects.len());

        for project in &projects {
            match resync_project_unlocked(store, &project.id, now) {
                Ok(summary) => summaries.push(summary),
                Err(e) => {
                    log::warn!(
                        "resync_all: failed to resync project {}: {:#}",
                        project.id,
                        e
                    );
                    summaries.push(ResyncSummary {
                        project_id: project.id.clone(),
                        synced: 0,
                        failed: 0,
                        errors: vec![format!("project-level error: {:#}", e)],
                    });
                }
            }
        }

        Ok(summaries)
    })
}

/// One assignment's on-disk facts, resolved by `observe_assignment`. Owns the
/// source hash so the borrowed `Observation` can point into it.
struct Observed {
    source_present: bool,
    target_present: bool,
    source_hash: Option<String>,
}

/// Plan step: read the environment for one assignment. Backfills the skill's
/// cached content hash when a copy-mode row needs it (legacy skill rows have
/// `content_hash = NULL`).
fn observe_assignment(
    store: &SkillStore,
    project: Option<&ProjectRecord>,
    skill: Option<&SkillRecord>,
    assignment: &ProjectSkillAssignmentRecord,
) -> Observed {
    let Some(skill) = skill else {
        return Observed {
            source_present: false,
            target_present: false,
            source_hash: None,
        };
    };
    let source = Path::new(&skill.central_path);
    let source_present = source.exists();

    let target_present = match (project, tool_adapters::adapter_by_key(&assignment.tool)) {
        (Some(project), Some(adapter)) => {
            let target =
                resolve_project_sync_target(Path::new(&project.path), adapter, &skill.name);
            target.exists() || target.symlink_metadata().is_ok()
        }
        _ => false,
    };

    // Only copies can drift, and hashing is only worth it when both sides exist.
    let source_hash = if assignment.mode.can_drift() && source_present && target_present {
        skill.content_hash.clone().or_else(|| {
            let h = content_hash::hash_dir(source).ok();
            if let Some(ref hash_val) = h {
                let _ = store.update_skill_content_hash(&skill.id, hash_val);
            }
            h
        })
    } else {
        None
    };

    Observed {
        source_present,
        target_present,
        source_hash,
    }
}

/// Execute step: apply `next_status` to one assignment, writing only when the
/// status changes. Returns the record as it now stands.
fn reconcile_assignment(
    store: &SkillStore,
    mut assignment: ProjectSkillAssignmentRecord,
    observed: &Observed,
) -> ProjectSkillAssignmentRecord {
    let decided = next_status(&Observation {
        source_present: observed.source_present,
        target_present: observed.target_present,
        mode: assignment.mode,
        current: assignment.status,
        source_hash: observed.source_hash.as_deref(),
        recorded_hash: assignment.content_hash.as_deref(),
    });
    if decided == assignment.status {
        return assignment;
    }
    let confirmed_hash = if decided == SyncStatus::Synced && assignment.mode.can_drift() {
        observed.source_hash.as_deref()
    } else {
        None
    };
    let _ = store.transition_assignment(
        &assignment.id,
        AssignmentTransition::Reconciled {
            status: decided,
            content_hash: confirmed_hash,
        },
    );
    assignment.status = decided;
    assignment.last_error = None;
    assignment.content_hash = confirmed_hash.map(str::to_string);
    assignment
}

/// A project's assignment rows plus whether the reconcile pass ran.
///
/// `reconciled == false` means a Sync-target mutation was in flight, so the
/// rows are the stored ones, un-reconciled. It is *not* a health signal: a
/// consumer must not read it as "everything is fine".
#[derive(Debug)]
pub struct AssignmentListing {
    pub assignments: Vec<ProjectSkillAssignmentRecord>,
    pub reconciled: bool,
}

/// List a project's assignments with their status reconciled against the
/// filesystem (source/target presence, copy drift). Rows whose observed
/// status differs from the stored one are updated in place.
///
/// The reconcile pass writes rows, so it runs under the mutation guard — but
/// it *try*-locks: a listing must stay responsive while a mutation runs, so
/// when the guard is busy the stored rows are returned untouched with
/// `reconciled: false`.
pub fn list_assignments_with_staleness(
    store: &SkillStore,
    project_id: &str,
) -> Result<AssignmentListing> {
    let mut assignments = store.list_project_skill_assignments(project_id)?;
    // Rows are reconciled in place, so the skipped path needs no fallback copy.
    let reconciled =
        mutation_guard::try_serialized(|| reconcile_listing(store, project_id, &mut assignments))
            .is_some();
    Ok(AssignmentListing {
        assignments,
        reconciled,
    })
}

/// The reconcile pass itself: observe every row and write the ones whose
/// status changed. Unlocked internal seam — the caller holds the guard.
fn reconcile_listing(
    store: &SkillStore,
    project_id: &str,
    assignments: &mut Vec<ProjectSkillAssignmentRecord>,
) {
    // Pre-fetch skill records with deduplication (one DB query per unique skill_id)
    let mut skill_cache: HashMap<String, Option<SkillRecord>> = HashMap::new();
    for a in assignments.iter() {
        skill_cache
            .entry(a.skill_id.clone())
            .or_insert_with(|| store.get_skill_by_id(&a.skill_id).ok().flatten());
    }

    // Pre-fetch project record once (not per iteration)
    let project_record = store.get_project_by_id(project_id).ok().flatten();

    *assignments = std::mem::take(assignments)
        .into_iter()
        .map(|assignment| {
            let skill = skill_cache
                .get(&assignment.skill_id)
                .and_then(|s| s.as_ref());
            let observed = observe_assignment(store, project_record.as_ref(), skill, &assignment);
            reconcile_assignment(store, assignment, &observed)
        })
        .collect();
}

/// Which way a toggle went. The decision is the backend's: it reads its own
/// assignment rows under the guard, so no caller has to mirror assignment
/// existence to choose between assigning and unassigning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleOutcome {
    Assigned,
    Unassigned,
}

/// Assign one skill to one project Tool, or unassign it when the row is
/// already there — whichever the stored state calls for.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
/// Reading the row and acting on it happen in the same critical section, so
/// the decision cannot be raced by a concurrent mutation.
pub fn toggle_skill_assignment(
    store: &SkillStore,
    project_id: &str,
    skill_id: &str,
    tool_key: &str,
    now: i64,
) -> Result<ToggleOutcome> {
    mutation_guard::serialized(|| {
        // Both branches use the unlocked seams — the guard is not reentrant.
        if store
            .get_project_skill_assignment(project_id, skill_id, tool_key)?
            .is_some()
        {
            let (project, skill) = lookup_project_and_skill(store, project_id, skill_id)?;
            unassign_and_cleanup(store, &project, &skill, tool_key)?;
            return Ok(ToggleOutcome::Unassigned);
        }
        assign_skill_to_project_tool_unlocked(store, project_id, skill_id, tool_key, now)?;
        Ok(ToggleOutcome::Assigned)
    })
}

/// Artifact removal for one assignment row: plan the
/// [`RemovalScope::ProjectSkillTool`] scope, execute it once, then apply this
/// caller's final policy — a failure is an error, because unassigning one
/// skill from one tool has nothing else to report.
///
/// The row is settled by the module (deleted on success, kept with Sync
/// status `error` on failure, ADR-0002) *before* the typed
/// `DeleteCleanupFailed` is raised, so the operator sees the `error` row and
/// the path that stayed.
///
/// Unlocked internal seam: callers reach it through an entry point that has
/// already taken the mutation guard.
pub(crate) fn unassign_and_cleanup(
    store: &SkillStore,
    project: &ProjectRecord,
    skill: &SkillRecord,
    tool_key: &str,
) -> Result<()> {
    if tool_adapters::adapter_by_key(tool_key).is_none() {
        anyhow::bail!(SignalError::UnknownTool {
            tool: tool_key.to_string(),
        });
    }

    let scope = artifact_removal::RemovalScope::ProjectSkillTool {
        project_id: project.id.clone(),
        skill_id: skill.id.clone(),
        tool_key: tool_key.to_string(),
    };
    let plan = artifact_removal::plan(store, &scope)?;
    let report = artifact_removal::execute_unlocked(store, plan)?;

    let failures = report.failures();
    if !failures.is_empty() {
        anyhow::bail!(SignalError::DeleteCleanupFailed { failures });
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/project_sync.rs"]
mod tests;
