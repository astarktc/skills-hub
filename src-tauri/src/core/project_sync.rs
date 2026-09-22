use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::core::{
    artifact_removal, content_identity,
    errors::{CommandError, SignalError},
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
/// than once (see `gitignore.rs` and the Artifact removal callers in `project_ops.rs`).
pub(crate) fn resolve_project_sync_target(
    project_path: &Path,
    adapter: &ToolAdapter,
    skill_name: &str,
) -> PathBuf {
    project_path
        .join(adapter.project_relative_skills_dir)
        .join(skill_name)
}

/// Which name locates a Project assignment's artifact on disk.
///
/// The rule is the **stored** `assignment.skill_name` — the name the artifact
/// was materialised under — never the live `skill.name`. A Managed skill's
/// name can change after the assignment row was written (a store update
/// today; a finalize that renames tomorrow), and the artifact on disk does
/// not follow: only the stored name still points at it. Propagation, re-sync,
/// the reconcile pass and Artifact removal all answer the question here, so
/// they cannot disagree — reading the live name in any of them would strand
/// the old artifact and grow a second one under the new name.
///
/// The live-name fallback fires **only** for an empty stored name: a row
/// that predates the `skill_name` column (schema V6) and whose backfill
/// found no skill row to copy the name from. Only then is `live_name`
/// consulted — the Managed skill's current name, supplied by the caller
/// (from the record it already holds, or a store lookup) — as a best
/// effort: the artifact was materialised under whatever name the skill had
/// at the time, which the live name may no longer be. A non-empty stored
/// name is never overridden. `None` means the artifact cannot be located.
pub(crate) fn assignment_artifact_name(
    assignment: &ProjectSkillAssignmentRecord,
    live_name: impl FnOnce() -> Result<Option<String>>,
) -> Result<Option<String>> {
    if !assignment.skill_name.is_empty() {
        return Ok(Some(assignment.skill_name.clone()));
    }
    live_name()
}

/// The path of a Project assignment's artifact: the project's Tool skills
/// dir joined with [`assignment_artifact_name`] (`live_name` is its
/// fallback supplier). `None` when the name cannot be resolved.
pub(crate) fn resolve_assignment_artifact(
    project_path: &Path,
    adapter: &ToolAdapter,
    assignment: &ProjectSkillAssignmentRecord,
    live_name: impl FnOnce() -> Result<Option<String>>,
) -> Result<Option<PathBuf>> {
    Ok(assignment_artifact_name(assignment, live_name)?
        .map(|name| resolve_project_sync_target(project_path, adapter, &name)))
}

/// What one Project assignment's sync needs and its caller has already
/// resolved: the assignment's Tool (its registry record) and its Managed
/// skill (the source of the bytes and, for an un-backfilled row, the live
/// name), the project root the artifact lives under, and the operation's
/// policy. Resolved once per assignment by the caller and handed down, so
/// no helper looks the adapter or the skill up a second time.
pub(crate) struct AssignmentSyncContext<'a> {
    pub store: &'a SkillStore,
    pub project_path: &'a Path,
    pub adapter: &'static ToolAdapter,
    pub skill: &'a SkillRecord,
    pub overwrite: bool,
    pub now: i64,
}

/// Sync one Project assignment through the capability-aware entry point and
/// record `SyncCompleted` on its row — the one place a project assignment
/// reaches [`sync_engine::sync_dir_for_tool_with_overwrite`], for the first
/// sync ([`assign_and_settle`]), a re-sync ([`sync_single_assignment`]) and
/// Propagation alike. The target is located by [`resolve_assignment_artifact`].
///
/// One hash rule lives here: only a copy records a content hash, because a
/// link follows the central copy and cannot drift. `central_hash` *supplies*
/// the hash and is consulted only when the mode used can drift. The callers
/// differ in where the hash comes from, not in the rule — Propagation passes
/// the freshly finalized central hash (computed once for every target, and
/// absent only when hashing the central copy failed), project sync hashes the
/// source on demand.
///
/// A sync failure is returned as is, the row untouched: each caller settles
/// `SyncFailed` under its own policy (error, count, or report data).
pub(crate) fn sync_assignment_target(
    ctx: &AssignmentSyncContext<'_>,
    assignment: &ProjectSkillAssignmentRecord,
    central_hash: impl FnOnce() -> Option<String>,
) -> Result<sync_engine::SyncOutcome> {
    let source = Path::new(&ctx.skill.central_path);
    let target = resolve_assignment_artifact(ctx.project_path, ctx.adapter, assignment, || {
        Ok(Some(ctx.skill.name.clone()))
    })?
    .ok_or_else(|| {
        anyhow::anyhow!(SignalError::NotFound {
            kind: "skill".to_string(),
            id: assignment.skill_id.clone(),
        })
    })?;

    let outcome =
        sync_engine::sync_dir_for_tool_with_overwrite(ctx.adapter, source, &target, ctx.overwrite)?;

    let hash = if outcome.mode_used.can_drift() {
        central_hash()
    } else {
        None
    };
    ctx.store.transition_assignment(
        &assignment.id,
        AssignmentTransition::SyncCompleted {
            mode: outcome.mode_used,
            synced_at: ctx.now,
            content_hash: hash.as_deref(),
        },
    )?;
    Ok(outcome)
}

/// The registry record for an assignment's Tool, or the typed `UnknownTool`.
fn require_adapter(tool_key: &str) -> Result<&'static ToolAdapter> {
    tool_adapters::adapter_by_key(tool_key).ok_or_else(|| {
        anyhow::anyhow!(SignalError::UnknownTool {
            tool: tool_key.to_string(),
        })
    })
}

/// Test fixture over [`assign_and_settle`]: create one assignment row and
/// run its first sync, returning the row (a sync failure is recorded on it
/// with `status = error`). Production reaches the first sync through the
/// fan-out ([`assign_skill_to_tools`]), which keeps the settled failure as
/// report data.
#[cfg(test)]
pub(crate) fn assign_and_sync(
    store: &SkillStore,
    project: &ProjectRecord,
    skill: &SkillRecord,
    tool_key: &str,
    now: i64,
) -> Result<ProjectSkillAssignmentRecord> {
    assign_and_settle(store, project, skill, tool_key, now).map(|(record, _)| record)
}

/// Create one assignment row and run its first sync. A sync failure is
/// recorded on the row (`status = error`, `last_error`) and returned beside
/// it, classified here, right after its `{:#}` chain was written to the
/// row's `last_error` — the report-row settlement point (ADR-0001, round
/// 15). Only a refusal before the row exists (unknown tool) or a store
/// failure is an `Err`.
///
/// Unlocked internal seam: callers reach it through an entry point that has
/// already taken the mutation guard (`mutation_guard`).
fn assign_and_settle(
    store: &SkillStore,
    project: &ProjectRecord,
    skill: &SkillRecord,
    tool_key: &str,
    now: i64,
) -> Result<(ProjectSkillAssignmentRecord, Option<CommandError>)> {
    // Refuse before a row exists: an unknown tool gets no assignment.
    let adapter = require_adapter(tool_key)?;

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

    let ctx = AssignmentSyncContext {
        store,
        project_path: Path::new(&project.path),
        adapter,
        skill,
        overwrite: false,
        now,
    };
    match sync_assignment_target(&ctx, &record, || {
        content_identity::read(content_identity::Source::Managed {
            store,
            skill_id: &skill.id,
        })
    }) {
        Ok(_) => {
            let updated = store
                .get_project_skill_assignment(&project.id, &skill.id, tool_key)?
                .unwrap_or(record);
            Ok((updated, None))
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
            Ok((updated, Some(CommandError::from_anyhow(e))))
        }
    }
}

// ---------------------------------------------------------------------------
// The project-sync report: what every project-world sync answers with —
// toggle-on (a batch of one), bulk assign, re-sync of one project and of
// every project. Crosses the wire as itself (ADR-0001, round-15 amendment):
// failures are `CommandError`s classified where the row settled, and the
// consumer derives any counts.
// ---------------------------------------------------------------------------

/// One skill × project Tool pair's result.
#[derive(Clone, Debug, serde::Serialize, specta::Type)]
pub struct ProjectSyncOutcome {
    /// The assignment row this outcome describes. Absent (`null` on the
    /// wire) only when no row exists — the assignment itself was refused
    /// (unknown tool) or the store failed before a row could be created.
    pub assignment_id: Option<String>,
    pub skill_id: String,
    pub skill_name: String,
    /// Registry key of the project Tool.
    pub tool: String,
    pub status: ProjectSyncOutcomeStatus,
}

#[derive(Clone, Debug, serde::Serialize, specta::Type)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ProjectSyncOutcomeStatus {
    /// The artifact was (re-)materialised and the row is `synced`.
    Synced,
    /// Bulk assign only: the pair was already assigned; nothing changed.
    AlreadyAssigned,
    /// The sync did not happen. When `assignment_id` is present the row
    /// exists with Sync status `error` and this chain in `last_error`.
    Failed { error: CommandError },
}

#[derive(Clone, Debug, Default, serde::Serialize, specta::Type)]
pub struct ProjectSyncReport {
    pub items: Vec<ProjectSyncOutcome>,
}

impl ProjectSyncReport {
    /// Outcomes whose status is `Synced` — for `Display`, logs and tests.
    pub fn synced(&self) -> usize {
        self.items
            .iter()
            .filter(|o| matches!(o.status, ProjectSyncOutcomeStatus::Synced))
            .count()
    }

    /// Outcomes whose status is `Failed` — for `Display`, logs and tests.
    pub fn failed(&self) -> usize {
        self.items
            .iter()
            .filter(|o| matches!(o.status, ProjectSyncOutcomeStatus::Failed { .. }))
            .count()
    }
}

/// One line per item — diagnostic text for logs, not user copy.
impl std::fmt::Display for ProjectSyncReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "synced={} failed={}", self.synced(), self.failed())?;
        for item in &self.items {
            let status = match &item.status {
                ProjectSyncOutcomeStatus::Synced => "synced".to_string(),
                ProjectSyncOutcomeStatus::AlreadyAssigned => "already assigned".to_string(),
                ProjectSyncOutcomeStatus::Failed { error } => format!("failed: {error}"),
            };
            write!(
                f,
                "\n  {} [{}] ({}) -> {}",
                item.skill_name,
                item.tool,
                item.assignment_id.as_deref().unwrap_or("no row"),
                status
            )?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Project-scope fan-out: one skill × N tools with per-target outcomes as
// data (the project counterpart of `global_sync::sync_skills_to_planned_tools`).
// `assign_skill_to_tools` is the deterministic engine; the two `*_project_*`
// entry points add the store lookups (typed `NotFound`) in front of it.
// ---------------------------------------------------------------------------

/// Deterministic engine: for each tool key in caller order, skip tools the
/// skill is already assigned to (`AlreadyAssigned`), otherwise assign and
/// sync. Failures are isolated per tool — one bad tool never aborts the
/// batch. A *sync* failure leaves its row with status `error` and is
/// reported `Failed` with that row's id; a refused assignment (unknown
/// tool) or a store failure is `Failed` with the id of whatever row exists.
pub(crate) fn assign_skill_to_tools(
    store: &SkillStore,
    project: &ProjectRecord,
    skill: &SkillRecord,
    tool_keys: &[String],
    now: i64,
) -> ProjectSyncReport {
    let items = tool_keys
        .iter()
        .map(|tool_key| {
            let (assignment_id, status) =
                match store.get_project_skill_assignment(&project.id, &skill.id, tool_key) {
                    Ok(Some(existing)) => {
                        (Some(existing.id), ProjectSyncOutcomeStatus::AlreadyAssigned)
                    }
                    Ok(None) => match assign_and_settle(store, project, skill, tool_key, now) {
                        Ok((record, None)) => (Some(record.id), ProjectSyncOutcomeStatus::Synced),
                        Ok((record, Some(error))) => {
                            (Some(record.id), ProjectSyncOutcomeStatus::Failed { error })
                        }
                        // Refused or a store failure: report whichever row the
                        // failure left behind (none for a refusal).
                        Err(error) => (
                            store
                                .get_project_skill_assignment(&project.id, &skill.id, tool_key)
                                .ok()
                                .flatten()
                                .map(|row| row.id),
                            ProjectSyncOutcomeStatus::Failed {
                                error: CommandError::from_anyhow(error),
                            },
                        ),
                    },
                    Err(error) => (
                        None,
                        ProjectSyncOutcomeStatus::Failed {
                            error: CommandError::from_anyhow(error),
                        },
                    ),
                };
            ProjectSyncOutcome {
                assignment_id,
                skill_id: skill.id.clone(),
                skill_name: skill.name.clone(),
                tool: tool_key.clone(),
                status,
            }
        })
        .collect();
    ProjectSyncReport { items }
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
) -> Result<ProjectSyncReport> {
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

/// Assign one skill to one tool: the single-target view of the same engine —
/// a report of one, `Synced` or `Failed` (a sync failure keeps its row with
/// status `error`; an unknown tool leaves none). `AlreadyAssigned` cannot
/// reach the caller: it is the typed `AssignmentExists` invariant, and the
/// toggle entry point has just read the row absent inside the same critical
/// section.
///
/// Unlocked internal seam — [`toggle_skill_assignment`] is the entry point
/// that takes the guard around it.
pub(crate) fn assign_skill_to_project_tool_unlocked(
    store: &SkillStore,
    project_id: &str,
    skill_id: &str,
    tool_key: &str,
    now: i64,
) -> Result<ProjectSyncReport> {
    let (project, skill) = lookup_project_and_skill(store, project_id, skill_id)?;
    let report = assign_skill_to_tools(store, &project, &skill, &[tool_key.to_string()], now);
    if report
        .items
        .iter()
        .any(|o| matches!(o.status, ProjectSyncOutcomeStatus::AlreadyAssigned))
    {
        anyhow::bail!(SignalError::AssignmentExists {
            project: project_id.to_string(),
            skill: skill_id.to_string(),
            tool: tool_key.to_string(),
        });
    }
    Ok(report)
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
    let ctx = AssignmentSyncContext {
        store,
        project_path: Path::new(&project.path),
        adapter: require_adapter(&assignment.tool)?,
        skill: &skill,
        overwrite,
        now,
    };
    sync_assignment_target(&ctx, assignment, || {
        content_identity::read(content_identity::Source::Managed {
            store,
            skill_id: &skill.id,
        })
    })?;
    Ok(())
}

/// Re-materialise every Sync target of one project.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
pub fn resync_project(store: &SkillStore, project_id: &str, now: i64) -> Result<ProjectSyncReport> {
    mutation_guard::serialized(|| resync_project_unlocked(store, project_id, now))
}

/// Per-assignment failures are report data, classified after the row was
/// settled `error` with the chain in `last_error`; only the project lookup
/// and the assignment listing (store failures) fail the whole re-sync.
pub(crate) fn resync_project_unlocked(
    store: &SkillStore,
    project_id: &str,
    now: i64,
) -> Result<ProjectSyncReport> {
    let project = project_ops::require_project(store, project_id)?;
    let assignments = store.list_project_skill_assignments(project_id)?;
    let mut report = ProjectSyncReport::default();

    for assignment in &assignments {
        let status = match sync_single_assignment(store, &project, assignment, true, now) {
            Ok(()) => ProjectSyncOutcomeStatus::Synced,
            Err(e) => {
                if let Err(settle) = store.transition_assignment(
                    &assignment.id,
                    AssignmentTransition::SyncFailed {
                        error: &format!("{:#}", e),
                    },
                ) {
                    log::warn!(
                        "resync: could not settle assignment {} as error: {:#}",
                        assignment.id,
                        settle
                    );
                }
                ProjectSyncOutcomeStatus::Failed {
                    error: CommandError::from_anyhow(e),
                }
            }
        };
        report.items.push(ProjectSyncOutcome {
            assignment_id: Some(assignment.id.clone()),
            skill_id: assignment.skill_id.clone(),
            skill_name: reported_skill_name(store, assignment),
            tool: assignment.tool.clone(),
            status,
        });
    }

    Ok(report)
}

/// The name a report shows for an assignment: the stored name the artifact
/// was materialised under, else (an un-backfilled legacy row) the live
/// skill name — the same rule as [`assignment_artifact_name`]. Empty only
/// when neither exists.
fn reported_skill_name(store: &SkillStore, assignment: &ProjectSkillAssignmentRecord) -> String {
    assignment_artifact_name(assignment, || {
        Ok(store
            .get_skill_by_id(&assignment.skill_id)?
            .map(|skill| skill.name))
    })
    .ok()
    .flatten()
    .unwrap_or_default()
}

/// Re-materialise every Sync target of every project, as one report
/// spanning them all (the caller re-reads whichever project it shows).
///
/// Mutation entry point: serialised against every other Sync-target mutation.
/// The per-project step is the unlocked seam — the guard is not reentrant.
/// Nothing composes the whole batch, so it has no unlocked seam of its own.
/// Per-assignment failures are report data; a store failure listing a
/// project's assignments fails the whole operation.
pub fn resync_all_projects(store: &SkillStore, now: i64) -> Result<ProjectSyncReport> {
    mutation_guard::serialized(|| {
        let mut report = ProjectSyncReport::default();
        for project in store.list_projects()? {
            let project_report = resync_project_unlocked(store, &project.id, now)?;
            report.items.extend(project_report.items);
        }
        Ok(report)
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
            resolve_assignment_artifact(Path::new(&project.path), adapter, assignment, || {
                Ok(Some(skill.name.clone()))
            })
            .ok()
            .flatten()
            .is_some_and(|target| target.exists() || target.symlink_metadata().is_ok())
        }
        _ => false,
    };

    // Only copies can drift, and hashing is only worth it when both sides exist.
    let source_hash = if assignment.mode.can_drift() && source_present && target_present {
        content_identity::read(content_identity::Source::Managed {
            store,
            skill_id: &skill.id,
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
    let reconciled = mutation_guard::try_serialized(|| {
        reconcile_listing_unlocked(store, project_id, &mut assignments)
    })
    .is_some();
    Ok(AssignmentListing {
        assignments,
        reconciled,
    })
}

/// The reconcile pass itself: observe every row and write the ones whose
/// status changed. Unlocked internal seam — the caller holds the guard.
pub(crate) fn reconcile_listing_unlocked(
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
#[derive(Debug)]
pub enum ToggleOutcome {
    /// A batch of one: `Synced`, or `Failed` with the row kept `error`.
    Assigned { report: ProjectSyncReport },
    Unassigned {
        report: artifact_removal::RemovalReport,
    },
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
            let report = unassign_and_remove_artifacts(store, &project, &skill, tool_key)?;
            return Ok(ToggleOutcome::Unassigned { report });
        }
        let report =
            assign_skill_to_project_tool_unlocked(store, project_id, skill_id, tool_key, now)?;
        Ok(ToggleOutcome::Assigned { report })
    })
}

/// Unassign one skill from every Tool of one project — the inverse of
/// [`assign_skill_to_project_tools`] (bulk unassign). Plans
/// [`artifact_removal::RemovalScope::ProjectSkill`] and executes it once:
/// each row is deleted on success and kept with Sync status `error` on a
/// failed removal (ADR-0002); per-target failures are report data. An
/// unknown project or skill is the typed `NotFound`.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
/// It calls only unlocked seams — the guard is not reentrant.
pub fn unassign_skill_from_project(
    store: &SkillStore,
    project_id: &str,
    skill_id: &str,
) -> Result<artifact_removal::RemovalReport> {
    mutation_guard::serialized(|| {
        let (project, skill) = lookup_project_and_skill(store, project_id, skill_id)?;
        let scope = artifact_removal::RemovalScope::ProjectSkill {
            project_id: project.id,
            skill_id: skill.id,
        };
        let plan = artifact_removal::plan(store, &scope)?;
        artifact_removal::execute_unlocked(store, plan)
    })
}

/// Artifact removal for one assignment row: plan the
/// [`RemovalScope::ProjectSkillTool`] scope and execute it once.
/// The row is settled by the module (deleted on success, kept with Sync
/// status `error` on failure, ADR-0002); target failures remain report data.
///
/// Unlocked internal seam: callers reach it through an entry point that has
/// already taken the mutation guard.
pub(crate) fn unassign_and_remove_artifacts(
    store: &SkillStore,
    project: &ProjectRecord,
    skill: &SkillRecord,
    tool_key: &str,
) -> Result<artifact_removal::RemovalReport> {
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
    artifact_removal::execute_unlocked(store, plan)
}

#[cfg(test)]
#[path = "tests/project_sync.rs"]
mod tests;
