//! Artifact removal: taking Sync targets off disk and settling their rows.
//!
//! One module for every scope an operator can ask to remove — one Managed
//! skill, one skill × Tool pair, one Project, one Project × Tool pair, one
//! Project × skill pair, or every global target — because "take the artifact off disk, then settle
//! the row that describes it" is one rule, not five. The deletion
//! counterpart of `global_sync.rs` / `project_sync.rs`, with the same
//! plan/execute/report split:
//!
//! * [`plan`] only reads the store: it resolves every path a scope touches
//!   and attaches the rows that describe each path. Tools sharing a skills
//!   directory share one artifact, so one [`RemovalTarget`] carries several
//!   [`RowRef`]s — the path is removed once and every member row is settled.
//! * [`execute_unlocked`] touches the filesystem with **one occupant rule**
//!   and **one settlement rule**:
//!   - occupant: right before each removal, `core::ownership::check` reads
//!     what is at the path against the evidence of the rows describing it
//!     (ADR-0007). Only an occupant one of them proves its own — a link
//!     resolving to the skill's Library copy, or a copy still holding the
//!     identity the row deployed — is removed. Anything else stays where it
//!     is and is reported [`RemovalTargetStatus::LeftInPlace`]: the rows
//!     settle as if their artifact were gone (it is), never by deleting
//!     another folder. An absent path is a successful removal; an occupant
//!     that cannot be read is a failure (the rows are kept `error`).
//!   - admission: every removal is admitted by the entry's physical identity
//!     (`core::removal_fence`) — never inside the central library (the
//!     configured central repo the caller passes as `central_dir`, plus every
//!     recorded copy), however a folder on the way is aliased or spelled —
//!     and only inside a skills folder the row that proved the occupant its
//!     own is managed in (`Claim::Within`: a global row's current and former
//!     registry dirs, `placement::managed_global_dirs`; an assignment's
//!     project skills dir), so a stored path outside them is refused
//!     (`ResolvesElsewhere`). A refusal is a failure like any other (the row
//!     is kept `error`).
//!     The `Skill` scope's own central copy is the one exception, removed
//!     under the record's authority (`sync_engine::remove_central_copy`).
//!   - settlement: on success every attached row is deleted — or, under
//!     [`Settlement::Disable`] (`core::skill_disable`, the only caller), kept
//!     with Sync status `disabled` as the operator's durable "not on this
//!     Tool"; on failure every attached row is **kept** with Sync status
//!     `error` carrying the diagnostic, so the failure stays observable and
//!     the operator can retry (ADR-0002). Rows are never deleted blind, and a
//!     row is never marked `disabled` while its artifact is still there.
//!   - claim: a row without a claim on its path (never materialised, or
//!     given up by a successful Disable or a Keep answer — `synced_at` NULL,
//!     see `SkillTargetRecord::owns_artifact`) has no evidence at all,
//!     whatever its status — a `disabled` marker, or one a failed Enable
//!     turned `error` — so planning sets it aside as a marker: no path is
//!     removed for it. A row's claim alone never removes anything: the
//!     occupant rule above decides. Delete clears the row alone
//!     ([`RemovalReport::cleared_markers`]) and Disable records it
//!     `disabled`, both only when every target of the scope succeeded;
//!     until then it keeps its status and `last_error`.
//! * Per-target outcomes are report data ([`RemovalReport`]); only a store
//!   failure fails the whole operation.
//!
//! Ordering matters for the `Skill` scope: `SkillStore` runs with
//! `PRAGMA foreign_keys = ON` and both `skill_targets.skill_id` and
//! `project_skill_assignments.skill_id` are `ON DELETE CASCADE`, so deleting
//! the `skills` row erases the rows that locate the artifacts. Planning
//! therefore resolves every path *before* the record goes, and the central
//! copy plus the record are removed last — and only when every target
//! succeeded, so a failed target never loses the row that describes it.
//!
//! Legacy-orphan rows (assignments whose skill row is already gone) are
//! reachable through the `Project` scopes, which walk assignments from the
//! project side.
//!
//! Locking: the operator-facing entry point here is [`remove_skill`]
//! (Delete), which takes the mutation guard; [`plan`], [`execute_unlocked`]
//! and [`execute_settling_unlocked`] are unlocked `pub(crate)` seams (see
//! `mutation_guard`) composed by `project_ops`, `project_sync` and
//! `skill_disable`. There is no unrecorded unsync: an operator's removal of
//! a global target is a Disable (`core::skill_disable`).

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::core::{
    environment::HostRoots,
    errors::{CommandError, SignalError},
    mutation_guard,
    ownership::{self, Deployed, Occupancy},
    placement::{self, row_folder, ScopeKind},
    project_sync::resolve_assignment_artifact,
    removal_fence::{Claim, Fence, ProtectedLibrary},
    skill_health::{self, SkillOutcomeKind},
    skill_store::{
        AssignmentTransition, ProjectRecord, ProjectSkillAssignmentRecord, SkillRecord, SkillStore,
        SkillTargetRecord, TargetTransition,
    },
    sync_engine::{remove_central_copy, remove_path_any, typed_removal_failure},
    tool_adapters::{
        adapter_by_key, ensure_path_within_tool_dirs, installed_set_in, project_skills_dir_in,
    },
};

/// What an operator asked to remove. Planning input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemovalScope {
    /// Every Sync target of one Managed skill — global target rows *and*
    /// project assignment rows — plus its central copy and its record.
    /// Deleting a Managed skill. `roots` locate the skills folders its global
    /// rows may lie in (`placement::managed_global_dirs`).
    Skill { skill_id: String, roots: HostRoots },
    /// Every **global** Sync target of one Managed skill; project artifacts
    /// are untouched. Planned by a skill-grain Disable (with a `ProjectSkill`
    /// scope per project for the assignments).
    SkillGlobal { skill_id: String, roots: HostRoots },
    /// One skill × folder pair at global scope: every row of the skill that
    /// describes the folder `tool_key`'s row does (`placement::row_folder` —
    /// rows written once per shared-dir sibling before 1.4.0 included). When
    /// no harness sharing that folder is installed any more, the rows are
    /// planned by their own stored paths, fenced by the registry's deletion
    /// rule. Its host roots also decide Tool installedness.
    SkillTool {
        skill_id: String,
        tool_key: String,
        roots: HostRoots,
    },
    /// Every Sync target of one Project. Planned by
    /// `project_ops::remove_project_and_artifacts`.
    Project { project_id: String },
    /// One Project × Tool pair: the project's assignment rows keyed exactly
    /// `tool_key` — a harness's legacy rows, or a folder's rows when
    /// `tool_key` is its folder key. Which keys a dropped harness takes with
    /// it is `project_ops::configure_project_tools`' decision
    /// (`placement::is_folder_row`). Planned by
    /// `project_ops::remove_project_tool_and_artifacts`.
    ProjectTool {
        project_id: String,
        tool_key: String,
    },
    /// One Project × skill × folder triple — the assignment row of that
    /// folder (and any row written per sibling before 1.4.0). Planned by
    /// `project_sync::unassign_and_remove_artifacts`.
    ProjectSkillTool {
        project_id: String,
        skill_id: String,
        tool_key: String,
    },
    /// One Project × skill pair across every Tool it is assigned to — every
    /// assignment row of one skill in one project. Bulk unassign; planned
    /// by `project_sync::unassign_skill_from_project`.
    ProjectSkill {
        project_id: String,
        skill_id: String,
    },
    /// Every global Sync target of every Managed skill. No operator action
    /// plans it any more (library-wide undeploy is a Disable of every
    /// skill); kept as the removal module's widest global scope.
    #[cfg_attr(
        not(test),
        allow(dead_code, reason = "no operator action plans it; its tests still do")
    )]
    EveryGlobalTarget { roots: HostRoots },
}

/// Where a row lives — the two tables that record Sync targets.
///
/// `id` / `tool` are the raw row reference settlement acts on (a pre-1.4.0
/// row written once per shared-dir sibling keeps its own key). `folder_key`
/// is the Deployment folder the row describes (`placement::row_folder`): the
/// backend's answer to "which Deployment is this", so a report reader counts
/// one Deployment per (skill, folder) — and per project for an assignment —
/// however many legacy rows describe it, and never derives a folder itself.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum RowRef {
    /// A `skill_targets` row (global scope).
    GlobalTarget {
        id: String,
        skill_id: String,
        tool: String,
        folder_key: String,
    },
    /// A `project_skill_assignments` row (project scope).
    Assignment {
        id: String,
        project_id: String,
        skill_id: String,
        tool: String,
        folder_key: String,
    },
}

impl RowRef {
    pub fn tool(&self) -> &str {
        match self {
            RowRef::GlobalTarget { tool, .. } | RowRef::Assignment { tool, .. } => tool,
        }
    }

    /// The reference to a global target row.
    pub fn global(id: &str, skill_id: &str, tool: &str) -> Self {
        RowRef::GlobalTarget {
            id: id.to_string(),
            skill_id: skill_id.to_string(),
            tool: tool.to_string(),
            folder_key: row_folder(ScopeKind::Global, tool),
        }
    }

    /// The reference to a project assignment row.
    pub fn assignment(id: &str, project_id: &str, skill_id: &str, tool: &str) -> Self {
        RowRef::Assignment {
            id: id.to_string(),
            project_id: project_id.to_string(),
            skill_id: skill_id.to_string(),
            tool: tool.to_string(),
            folder_key: row_folder(ScopeKind::Project, tool),
        }
    }
}

/// One filesystem path to remove, with every row that describes it. Tools
/// sharing a skills directory produce one target with several rows.
#[derive(Debug, Clone)]
pub struct RemovalTarget {
    pub path: PathBuf,
    pub rows: Vec<RowRef>,
    /// What each row recorded, index-aligned with `rows`: the evidence the
    /// occupant at `path` is checked against when it is removed, and the
    /// managed skills folders the row's path may lie in. Kept aligned through
    /// [`RemovalTarget::retain_rows`].
    evidence: Vec<RowEvidence>,
    /// Refused at planning, before anything at `path` is read: an assignment
    /// whose stored name is not one folder name (`placement::deployment_path`)
    /// has no artifact inside its project skills dir, so `path` is only where
    /// the name pointed (for the report). Its rows fail and are kept `error`.
    refused: Option<SignalError>,
}

/// One row's own authority over its path: what it deployed, and where.
#[derive(Debug, Clone)]
struct RowEvidence {
    deployed: Deployed,
    /// The skills folders the row's artifact may lie in — a global row's
    /// current and former registry dirs of its harness and folder
    /// (`placement::managed_global_dirs`), an assignment's project skills dir.
    /// A removal is admitted physically inside them (`Claim::Within`), so a
    /// stored path outside every managed folder is refused, never removed.
    managed: Vec<PathBuf>,
}

/// What occupies a target's path, as its rows' evidence tells it now.
enum TargetOccupant<'t> {
    Absent,
    /// One row proves it its own; its managed folders are the removal's claim.
    Ours {
        within: &'t [PathBuf],
    },
    Foreign,
}

impl RemovalTarget {
    /// Keep only the rows `keep` selects, with their own evidence: a row
    /// dropped from the target lends nothing to the rows that stay.
    pub(crate) fn retain_rows(
        &mut self,
        mut keep: impl FnMut(&RowRef) -> Result<bool>,
    ) -> Result<()> {
        let mut rows = Vec::with_capacity(self.rows.len());
        let mut evidence = Vec::with_capacity(self.evidence.len());
        for (row, own) in self.rows.drain(..).zip(self.evidence.drain(..)) {
            if keep(&row)? {
                rows.push(row);
                evidence.push(own);
            }
        }
        self.rows = rows;
        self.evidence = evidence;
        Ok(())
    }

    /// The occupant check over every row's own evidence, now: ours when
    /// one row proves it its own (each row speaks only for itself — and its
    /// managed folders alone claim the removal), absent when nothing is
    /// there, else foreign. A read failure that no row's evidence settled is
    /// an `Err`.
    fn occupant(&self) -> Result<TargetOccupant<'_>> {
        let mut verdict = TargetOccupant::Absent;
        let mut failure = None;
        for own in &self.evidence {
            match ownership::check(&self.path, &own.deployed) {
                Ok(Occupancy::Ours) => {
                    return Ok(TargetOccupant::Ours {
                        within: &own.managed,
                    })
                }
                Ok(Occupancy::Absent) => {}
                Ok(Occupancy::Foreign) => verdict = TargetOccupant::Foreign,
                Err(err) => failure = Some(err),
            }
        }
        match failure {
            Some(err) => Err(err),
            None => Ok(verdict),
        }
    }
}

/// Everything a scope resolves to, read before any row is deleted.
#[derive(Debug)]
pub struct RemovalPlan {
    pub scope: RemovalScope,
    pub targets: Vec<RemovalTarget>,
    /// Rows in scope that own no artifact (`owns_artifact` in `skill_store`,
    /// whatever their status): the app never materialised anything at their
    /// path, so no path is planned for them — whatever sits there is not ours. They
    /// are settled as rows alone (see [`RemovalReport::cleared_markers`]).
    pub markers: Vec<RowRef>,
    /// Set only for [`RemovalScope::Skill`]: the record whose central copy
    /// and row are removed after every target succeeded. `None` when the
    /// skill row is already gone (legacy shape) or for any other scope.
    pub skill: Option<SkillRecord>,
}

/// Failures are classified at settlement, after writing the full chain to
/// each attached row's diagnostic.
#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RemovalTargetStatus {
    /// Removed, or already absent.
    Removed,
    /// Something occupies the path that no attached row can show is the
    /// artifact it deployed (ADR-0007): a folder put where the app's link
    /// was, a copy edited in place, a copy a pre-1.4.0 row never recorded.
    /// It was kept untouched, and the rows settled as their artifact gone.
    LeftInPlace,
    /// Every attached row was kept with status `error` and the original chain.
    Failed { error: CommandError },
}

#[derive(Debug, serde::Serialize, specta::Type)]
pub struct RemovalTargetOutcome {
    pub path: PathBuf,
    pub rows: Vec<RowRef>,
    pub status: RemovalTargetStatus,
}

#[derive(Debug, serde::Serialize, specta::Type)]
pub struct RemovalReport {
    /// The scope the report settled, for diagnostics only; `None` for an
    /// empty merge.
    #[serde(skip)]
    #[specta(skip)]
    pub scope: Option<RemovalScope>,
    pub targets: Vec<RemovalTargetOutcome>,
    /// The central copy was deleted (only ever true for the `Skill` scope).
    pub central_removed: bool,
    /// The `skills` row was deleted (cascading targets and assignments).
    pub record_deleted: bool,
    /// Rows owning no artifact (never materialised, whatever their status)
    /// deleted without touching any path — the
    /// app never materialised their artifact. Only the delete settlement
    /// clears them, and only when every target of the scope succeeded (a
    /// kept skill or project keeps its markers too).
    pub cleared_markers: Vec<RowRef>,
    /// Rows owning no artifact that this call recorded `disabled` without
    /// touching any path (only the Disable settlement, including the marker
    /// row a pair Disable records for a pair that had none): the operator's
    /// "not on this Tool" was recorded although nothing was removed.
    pub disabled_markers: Vec<RowRef>,
}

impl RemovalReport {
    /// Per-target diagnostics for internal removal policy and logs, never IPC.
    pub fn failures(&self) -> Vec<String> {
        self.targets
            .iter()
            .filter_map(|target| match &target.status {
                RemovalTargetStatus::Failed { error } => {
                    Some(format!("{}: {error}", target.path.display()))
                }
                RemovalTargetStatus::Removed | RemovalTargetStatus::LeftInPlace => None,
            })
            .collect()
    }

    /// Combine settled reports in input order, without flattening shared rows.
    /// The first scope is retained for diagnostics only; an empty merge has
    /// none. Deletion flags indicate whether any input deleted it.
    pub fn merge(reports: impl IntoIterator<Item = RemovalReport>) -> RemovalReport {
        let mut reports = reports.into_iter();
        let mut merged = reports.next().unwrap_or(RemovalReport {
            scope: None,
            targets: Vec::new(),
            central_removed: false,
            record_deleted: false,
            cleared_markers: Vec::new(),
            disabled_markers: Vec::new(),
        });
        for report in reports {
            merged.targets.extend(report.targets);
            merged.cleared_markers.extend(report.cleared_markers);
            merged.disabled_markers.extend(report.disabled_markers);
            merged.central_removed |= report.central_removed;
            merged.record_deleted |= report.record_deleted;
        }
        merged
    }

    /// Rows whose artifact was removed (and whose row was therefore settled).
    pub fn removed_rows(&self) -> usize {
        self.count_rows(|status| matches!(status, RemovalTargetStatus::Removed))
    }

    /// Rows kept with Sync status `error` because their artifact stayed.
    pub fn failed_rows(&self) -> usize {
        self.count_rows(|status| matches!(status, RemovalTargetStatus::Failed { .. }))
    }

    /// Rows settled while the folder at their path, not theirs, was kept.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn left_in_place_rows(&self) -> usize {
        self.count_rows(|status| matches!(status, RemovalTargetStatus::LeftInPlace))
    }

    fn count_rows(&self, counted: impl Fn(&RemovalTargetStatus) -> bool) -> usize {
        self.targets
            .iter()
            .filter(|t| counted(&t.status))
            .map(|t| t.rows.len())
            .sum()
    }
}

/// One line per target — diagnostic text for logs, not user copy.
impl fmt::Display for RemovalReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}: central_removed={} record_deleted={} removed_rows={} failed_rows={}",
            self.scope,
            self.central_removed,
            self.record_deleted,
            self.removed_rows(),
            self.failed_rows()
        )?;
        for target in &self.targets {
            let status = match &target.status {
                RemovalTargetStatus::Removed => "removed".to_string(),
                RemovalTargetStatus::LeftInPlace => "left in place".to_string(),
                RemovalTargetStatus::Failed { error } => format!("failed: {:#}", error),
            };
            let tools: Vec<&str> = target.rows.iter().map(RowRef::tool).collect();
            write!(
                f,
                "\n  {} [{}] -> {}",
                target.path.display(),
                tools.join(", "),
                status
            )?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Planning
// ---------------------------------------------------------------------------

/// Read-only: resolve every path `scope` touches, deduped by path with the
/// rows describing each path attached, each row with the managed skills
/// folders its removal is claimed in. Every root a scope needs travels
/// inside the scope itself (a global scope carries the host roots its rows'
/// managed folders resolve under, and [`RemovalScope::SkillTool`]'s also
/// decide Tool installedness), so planning takes no ambient roots.
/// Unlocked internal seam.
pub(crate) fn plan(store: &SkillStore, scope: &RemovalScope) -> Result<RemovalPlan> {
    let mut builder = TargetBuilder::new(store);
    let mut skill: Option<SkillRecord> = None;

    match scope {
        RemovalScope::Skill { skill_id, roots } => {
            skill = store.get_skill_by_id(skill_id)?;
            for row in store.list_skill_targets(skill_id)? {
                builder.push_global(&row, roots)?;
            }
            let assignments = store.list_project_skill_assignments_by_skill(skill_id)?;
            builder.push_assignments(assignments)?;
        }
        RemovalScope::SkillGlobal { skill_id, roots } => {
            for row in store.list_skill_targets(skill_id)? {
                builder.push_global(&row, roots)?;
            }
        }
        RemovalScope::SkillTool {
            skill_id,
            tool_key,
            roots,
        } => {
            let folder = row_folder(ScopeKind::Global, tool_key);
            let rows: Vec<_> = store
                .list_skill_targets(skill_id)?
                .into_iter()
                .filter(|row| row_folder(ScopeKind::Global, &row.tool) == folder)
                .collect();
            if folder_locatable(roots, &folder) {
                for row in &rows {
                    builder.push_global(row, roots)?;
                }
            } else {
                // No harness sharing the folder is on this machine any more,
                // so the registry can no longer locate it — but each row
                // records the path of an artifact *we* created. Cleaning up
                // after ourselves is not "touching an uninstalled tool's
                // directory": plan the rows by their own stored paths. Because
                // such a path is a stored string rather than a registry-derived
                // one, it is fenced by the registry's deletion rule — a row
                // pointing outside every Tool skills dir is refused, and
                // refusing planning keeps every row.
                for row in &rows {
                    ensure_path_within_tool_dirs(roots, Path::new(&row.target_path))?;
                    builder.push_global(row, roots)?;
                }
            }
        }
        RemovalScope::Project { project_id } => {
            let assignments = store.list_project_skill_assignments(project_id)?;
            builder.push_assignments(assignments)?;
        }
        RemovalScope::ProjectTool {
            project_id,
            tool_key,
        } => {
            let assignments =
                store.list_project_skill_assignments_for_project_tool(project_id, tool_key)?;
            builder.push_assignments(assignments)?;
        }
        RemovalScope::ProjectSkillTool {
            project_id,
            skill_id,
            tool_key,
        } => {
            let folder = row_folder(ScopeKind::Project, tool_key);
            let assignments = store
                .list_project_skill_assignments(project_id)?
                .into_iter()
                .filter(|a| {
                    &a.skill_id == skill_id && row_folder(ScopeKind::Project, &a.tool) == folder
                })
                .collect();
            builder.push_assignments(assignments)?;
        }
        RemovalScope::ProjectSkill {
            project_id,
            skill_id,
        } => {
            let assignments = store
                .list_project_skill_assignments(project_id)?
                .into_iter()
                .filter(|assignment| &assignment.skill_id == skill_id)
                .collect();
            builder.push_assignments(assignments)?;
        }
        RemovalScope::EveryGlobalTarget { roots } => {
            for skill in store.list_skills()? {
                for row in store.list_skill_targets(&skill.id)? {
                    builder.push_global(&row, roots)?;
                }
            }
        }
    }

    let (targets, markers) = builder.finish();
    Ok(RemovalPlan {
        scope: scope.clone(),
        targets,
        markers,
        skill,
    })
}

/// Every artifact path the app owns, resolved exactly as Delete plans it
/// (global target rows by their recorded path, project assignments by
/// `resolve_assignment_artifact`): the paths a removal report can name.
/// Read-only; what Reveal admits (`core::log_reveal::resolve_reveal_item`).
/// `roots` only shape the plan's claims, never which paths it names.
pub(crate) fn recorded_target_paths(store: &SkillStore, roots: &HostRoots) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for skill in store.list_skills()? {
        let scope = RemovalScope::Skill {
            skill_id: skill.id,
            roots: roots.clone(),
        };
        paths.extend(
            plan(store, &scope)?
                .targets
                .into_iter()
                // A refused row names no artifact of ours.
                .filter(|target| target.refused.is_none())
                .map(|target| target.path),
        );
    }
    Ok(paths)
}

/// Whether the registry can still locate the global folder `folder`: some
/// harness whose own folder it is, is installed for this operator. When none
/// is, no path may be *derived* for it and the rows' own recorded paths are
/// planned instead, fenced. An unknown key stands for itself: its rows still
/// carry their own paths.
fn folder_locatable(roots: &HostRoots, folder: &str) -> bool {
    if adapter_by_key(folder).is_none() {
        return true;
    }
    placement::folder_reachable(ScopeKind::Global, folder, &installed_set_in(roots))
}

/// Accumulates rows into targets deduped by path — a shared skills dir means
/// one artifact with several rows — each with the evidence its row recorded
/// (`core::ownership::Deployed`, read from the row and its skill's record),
/// and sets markers aside (rows without a claim on their path, whatever their
/// status): they describe no artifact of ours, so they never join (or create)
/// a target.
struct TargetBuilder<'a> {
    store: &'a SkillStore,
    targets: Vec<RemovalTarget>,
    markers: Vec<RowRef>,
    /// Skill records by id, read once: the Library copy a row's link must
    /// resolve to, and an un-backfilled assignment's name.
    skills: HashMap<String, Option<SkillRecord>>,
    projects: HashMap<String, Option<ProjectRecord>>,
}

impl<'a> TargetBuilder<'a> {
    fn new(store: &'a SkillStore) -> Self {
        Self {
            store,
            targets: Vec::new(),
            markers: Vec::new(),
            skills: HashMap::new(),
            projects: HashMap::new(),
        }
    }

    fn skill(&mut self, skill_id: &str) -> Result<Option<SkillRecord>> {
        if let Some(cached) = self.skills.get(skill_id) {
            return Ok(cached.clone());
        }
        let looked_up = self.store.get_skill_by_id(skill_id)?;
        self.skills.insert(skill_id.to_string(), looked_up.clone());
        Ok(looked_up)
    }

    fn project(&mut self, project_id: &str) -> Result<Option<ProjectRecord>> {
        if let Some(cached) = self.projects.get(project_id) {
            return Ok(cached.clone());
        }
        let looked_up = self.store.get_project_by_id(project_id)?;
        self.projects
            .insert(project_id.to_string(), looked_up.clone());
        Ok(looked_up)
    }

    fn push(&mut self, path: PathBuf, row: RowRef, own: RowEvidence) {
        match self.targets.iter_mut().find(|t| t.path == path) {
            Some(target) => {
                target.rows.push(row);
                target.evidence.push(own);
            }
            None => self.targets.push(RemovalTarget {
                path,
                rows: vec![row],
                evidence: vec![own],
                refused: None,
            }),
        }
    }

    /// A row refused at planning (`refusal`, a typed condition): its own
    /// target, failed at execution with nothing touched.
    fn push_refused(&mut self, path: PathBuf, row: RowRef, refusal: SignalError) {
        self.targets.push(RemovalTarget {
            path,
            rows: vec![row],
            evidence: vec![RowEvidence {
                deployed: Deployed::none(),
                managed: Vec::new(),
            }],
            refused: Some(refusal),
        });
    }

    /// A global row by its stored path, claimed inside the skills folders
    /// its harness and folder are managed in under `roots`.
    fn push_global(&mut self, row: &SkillTargetRecord, roots: &HostRoots) -> Result<()> {
        let row_ref = RowRef::global(&row.id, &row.skill_id, &row.tool);
        if !row.owns_artifact() {
            self.markers.push(row_ref);
        } else {
            let skill = self.skill(&row.skill_id)?;
            let own = RowEvidence {
                deployed: Deployed::of_target(row, skill.as_ref()),
                managed: placement::managed_global_dirs(roots, &row.tool),
            };
            self.push(PathBuf::from(&row.target_path), row_ref, own);
        }
        Ok(())
    }

    /// Resolve each assignment row's project-scope artifact path. A row whose
    /// project or tool can no longer be resolved has no locatable artifact,
    /// so it is not planned (and not settled) — the project-side callers own
    /// that case.
    fn push_assignments(&mut self, assignments: Vec<ProjectSkillAssignmentRecord>) -> Result<()> {
        for assignment in assignments {
            let row_ref = RowRef::assignment(
                &assignment.id,
                &assignment.project_id,
                &assignment.skill_id,
                &assignment.tool,
            );
            if !assignment.owns_artifact() {
                self.markers.push(row_ref);
                continue;
            }
            let Some(project) = self.project(&assignment.project_id)? else {
                log::warn!(
                    "artifact removal: assignment {} names missing project {}",
                    assignment.id,
                    assignment.project_id
                );
                continue;
            };
            let Some(adapter) = adapter_by_key(&assignment.tool) else {
                log::warn!(
                    "artifact removal: assignment {} names unknown tool {}",
                    assignment.id,
                    assignment.tool
                );
                continue;
            };
            let skill = self.skill(&assignment.skill_id)?;
            // One naming rule for project artifacts: the stored assignment name
            // (`project_sync::assignment_artifact_name`); the live skill row is
            // its fallback supplier, read only for an un-backfilled row. A name
            // that is not one folder name is refused here, nothing read.
            let resolved =
                resolve_assignment_artifact(Path::new(&project.path), adapter, &assignment, || {
                    Ok(skill.as_ref().map(|skill| skill.name.clone()))
                });
            let resolved = match resolved {
                Ok(resolved) => resolved,
                Err(err) => {
                    match (
                        placement::refused_place(&err),
                        err.downcast_ref::<SignalError>(),
                    ) {
                        (Some(place), Some(refusal)) => {
                            let refusal = refusal.clone();
                            self.push_refused(place, row_ref, refusal);
                            continue;
                        }
                        _ => return Err(err),
                    }
                }
            };
            let Some(path) = resolved else {
                log::warn!(
                    "artifact removal: assignment {} has no skill name to locate its artifact",
                    assignment.id
                );
                continue;
            };
            let own = RowEvidence {
                deployed: Deployed::of_assignment(&assignment, skill.as_ref()),
                managed: project_skills_dir_in(Path::new(&project.path), adapter)
                    .into_iter()
                    .collect(),
            };
            self.push(path, row_ref, own);
        }
        Ok(())
    }

    fn finish(self) -> (Vec<RemovalTarget>, Vec<RowRef>) {
        (self.targets, self.markers)
    }
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

/// The one presence rule: absence is only `NotFound`. `symlink_metadata`
/// succeeding means present, so a broken symlink counts as present (and must
/// be removed); any *other* stat error (EACCES on a parent, EIO, ENOTDIR)
/// also counts as present, because it does not prove absence — removal is
/// attempted, fails, and the row is kept with Sync status `error` (ADR-0002)
/// rather than deleted blind.
fn is_present(path: &Path) -> bool {
    match path.symlink_metadata() {
        Ok(_) => true,
        Err(err) => err.kind() != std::io::ErrorKind::NotFound,
    }
}

/// Execute a plan. Each target is removed once and its rows settled: deleted
/// on success, kept with Sync status `error` on failure (ADR-0002). For the
/// `Skill` scope the central copy and the `skills` row follow, but only when
/// every target succeeded — a failure keeps the whole skill so the operator
/// can retry. Unlocked internal seam.
pub(crate) fn execute_unlocked(
    store: &SkillStore,
    central_dir: &Path,
    plan: RemovalPlan,
) -> Result<RemovalReport> {
    execute_settling_unlocked(store, central_dir, plan, Settlement::Delete)
}

/// How the rows of a successfully removed target are settled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Settlement {
    /// Rows are deleted: unassign, project removal, Delete.
    Delete,
    /// Rows are kept with Sync status `disabled`: the durable "not on this
    /// Tool" (`core::skill_disable`). Never deletes a skill record.
    Disable,
}

/// [`execute_unlocked`] with the success settlement chosen by the caller.
/// Failure settlement is the same for both (`error`, ADR-0002), and the
/// `Skill` scope's central copy and record are removed only under
/// [`Settlement::Delete`]. Unlocked internal seam.
pub(crate) fn execute_settling_unlocked(
    store: &SkillStore,
    central_dir: &Path,
    plan: RemovalPlan,
    settlement: Settlement,
) -> Result<RemovalReport> {
    let mut targets: Vec<RemovalTargetOutcome> = Vec::with_capacity(plan.targets.len());
    let library = ProtectedLibrary::new(store, central_dir);

    for target in plan.targets {
        // The occupant rule, at the moment of removal: only what a row shows
        // is its own artifact is removed — and only inside a skills folder
        // that row is managed in (a malformed stored path elsewhere is
        // refused, `ResolvesElsewhere`, and its rows kept `error`); a foreign
        // occupant stays.
        let result = match target.refused.clone() {
            Some(refusal) => Err(anyhow::anyhow!(refusal)),
            None => match target.occupant() {
                Ok(TargetOccupant::Absent) => Ok(RemovalTargetStatus::Removed),
                Ok(TargetOccupant::Ours { within }) => remove_path_any(
                    &target.path,
                    &Fence::new(&library).claiming(Claim::Within(within)),
                )
                .map(|()| RemovalTargetStatus::Removed),
                Ok(TargetOccupant::Foreign) => Ok(RemovalTargetStatus::LeftInPlace),
                // Not knowing the occupant is never "foreign" (the row would
                // settle as if its artifact were gone) nor "ours": it fails like a
                // removal that could not run, and the row is kept (ADR-0002).
                Err(err) => Err(typed_removal_failure(&target.path, err)),
            },
        };

        let status = match result {
            Ok(status) => {
                // The artifact is gone or was never this row's: either way
                // the row's claim ends here (ADR-0007).
                for row in &target.rows {
                    match settlement {
                        Settlement::Delete => delete_row(store, row)?,
                        Settlement::Disable => settle_row_as_disabled(store, row)?,
                    }
                }
                status
            }
            Err(error) => {
                let detail = format!("{error:#}");
                for row in &target.rows {
                    settle_row_as_error(store, row, &detail)?;
                }
                RemovalTargetStatus::Failed {
                    error: CommandError::from_anyhow(error),
                }
            }
        };

        targets.push(RemovalTargetOutcome {
            path: target.path,
            rows: target.rows,
            status,
        });
    }

    let any_failed = targets
        .iter()
        .any(|t| matches!(t.status, RemovalTargetStatus::Failed { .. }));

    // Markers own no artifact: no path is touched for them, and until the
    // scope settles they keep their status and `last_error` (a failed Enable's
    // `TARGET_EXISTS` stays observable). Only when nothing failed (so a kept
    // skill or project keeps its recorded intent, and a sibling's failed
    // shared artifact is never hidden behind a `disabled` row): Delete clears
    // their rows with the scope; Disable records them `disabled` (row only,
    // `synced_at` untouched, so they stay non-owners) so the operator's "not
    // on this Tool" is durable and Enable retries them.
    let mut cleared_markers = Vec::new();
    let mut disabled_markers = Vec::new();
    if !any_failed {
        for marker in plan.markers {
            match settlement {
                Settlement::Delete => {
                    delete_row(store, &marker)?;
                    cleared_markers.push(marker);
                }
                Settlement::Disable => {
                    settle_row_as_disabled(store, &marker)?;
                    disabled_markers.push(marker);
                }
            }
        }
    }

    let mut central_removed = false;
    let mut record_deleted = false;
    if let (Some(skill), Settlement::Delete) = (plan.skill, settlement) {
        if any_failed {
            // A kept Delete settles here as the skill's health, with the
            // first failed target's classified error (the report carries
            // every one).
            if let Some(error) = targets.iter().find_map(|t| match &t.status {
                RemovalTargetStatus::Failed { error } => Some(error),
                RemovalTargetStatus::Removed | RemovalTargetStatus::LeftInPlace => None,
            }) {
                skill_health::note_outcome(
                    store,
                    &skill.id,
                    SkillOutcomeKind::DeleteIncomplete,
                    error,
                );
            }
        } else {
            // The one removal of central bytes, authorised by the record
            // rather than the fence above; a permission failure is typed
            // like any removal's (BACKLOG #76).
            let central = Path::new(&skill.central_path);
            if is_present(central) {
                remove_central_copy(central, &skill.id, &library)
                    .with_context(|| format!("remove central copy {:?}", central))?;
                central_removed = true;
            }
            store.delete_skill(&skill.id)?;
            record_deleted = true;
        }
    }

    Ok(RemovalReport {
        scope: Some(plan.scope),
        targets,
        central_removed,
        record_deleted,
        cleared_markers,
        disabled_markers,
    })
}

fn delete_row(store: &SkillStore, row: &RowRef) -> Result<()> {
    match row {
        RowRef::GlobalTarget { skill_id, tool, .. } => store.delete_skill_target(skill_id, tool),
        RowRef::Assignment {
            project_id,
            skill_id,
            tool,
            ..
        } => store.remove_project_skill_assignment(project_id, skill_id, tool),
    }
}

fn settle_row_as_disabled(store: &SkillStore, row: &RowRef) -> Result<()> {
    match row {
        RowRef::GlobalTarget { id, .. } => {
            store.transition_skill_target(id, TargetTransition::Disabled)
        }
        RowRef::Assignment { id, .. } => {
            store.transition_assignment(id, AssignmentTransition::Disabled)
        }
    }
}

fn settle_row_as_error(store: &SkillStore, row: &RowRef, error: &str) -> Result<()> {
    match row {
        RowRef::GlobalTarget { id, .. } => {
            store.transition_skill_target(id, TargetTransition::SyncFailed { error })
        }
        RowRef::Assignment { id, .. } => {
            store.transition_assignment(id, AssignmentTransition::SyncFailed { error })
        }
    }
}

/// Why the redeploy may remove an old artifact (`core::redeploy`).
#[derive(Clone, Copy, Debug)]
pub(crate) enum SupersededAuthority<'a> {
    /// Only an occupant one of these proves ours — the old row's own record,
    /// or the Library's Content identity — through the occupant check, now.
    Evidence(&'a [Deployed]),
    /// The operator answered "Delete your edited copy" for exactly this path.
    Answered,
}

/// What [`remove_superseded_artifact_unlocked`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Superseded {
    /// Removed now.
    Removed,
    /// Nothing was there.
    Absent,
    /// Something is there that no evidence proves ours: untouched.
    NotOurs,
}

/// Remove an old artifact a redeploy's row switch superseded
/// (`core::redeploy`). No row is settled: the row already records the new
/// place, and the journal entry is the caller's to clear.
///
/// The occupant rule runs here, at the moment of removal: under
/// [`SupersededAuthority::Evidence`] the occupant is removed only when one of
/// the evidences proves it ours (`core::ownership::check`); anything else is
/// [`Superseded::NotOurs`] and stays. The removal is admitted physically like
/// every removal — never the central library — and only inside `managed`,
/// the skills folders the old row is managed in
/// (`placement::managed_global_dirs`, its harness's current and former
/// dirs): a malformed path elsewhere is refused (`ResolvesElsewhere`).
/// Unlocked internal seam.
pub(crate) fn remove_superseded_artifact_unlocked(
    store: &SkillStore,
    central_dir: &Path,
    path: &Path,
    managed: &[PathBuf],
    authority: SupersededAuthority<'_>,
) -> Result<Superseded> {
    if !is_present(path) {
        return Ok(Superseded::Absent);
    }
    if let SupersededAuthority::Evidence(evidence) = authority {
        let mut ours = false;
        for deployed in evidence {
            if ownership::check(path, deployed)? == Occupancy::Ours {
                ours = true;
                break;
            }
        }
        if !ours {
            return Ok(Superseded::NotOurs);
        }
    }
    let library = ProtectedLibrary::new(store, central_dir);
    remove_path_any(path, &Fence::new(&library).claiming(Claim::Within(managed)))?;
    Ok(Superseded::Removed)
}

/// Move Library folder's Clean step (`core::library_move`): remove a moved
/// skill's **former** central copy at `former_copy`, after the switch
/// recorded its new copy — so the store no longer records this path, and
/// `central_dir` is the *new* configured central repo. The clean calls it
/// only on a copy it has renamed aside inside the old Library folder
/// (`<former_root>/.skills-hub-clean-<uuid>/<name>`) and just proved
/// identical to the new copy; the claim is that set-aside folder.
///
/// Admitted physically like every removal: never the (new) library, never a
/// copy the store still records or the folder holding one (a skill left
/// behind as Unlocatable keeps its record, so its folder stays protected),
/// and claimed to lie inside the folder that held the copy — an aliased
/// folder on the way that leads elsewhere is refused. The old Library root
/// `former_root` is never removed, nor anything holding it: a record that
/// names the root itself (or a folder around it) as a copy is refused
/// (`CentralLibrary`), so the root and anything unrecorded in it stay.
/// Presence follows the module rule. Unlocked internal seam.
pub(crate) fn remove_former_central_copy_unlocked(
    store: &SkillStore,
    central_dir: &Path,
    former_root: &Path,
    former_copy: &Path,
) -> Result<()> {
    let Some(holder) = former_copy.parent().filter(|_| former_copy.is_absolute()) else {
        anyhow::bail!(
            "refusing to remove {:?}: not a recorded copy path",
            former_copy
        );
    };
    // A link is removed as itself and never reaches its referent; a real
    // folder must not be, or hold, the old root.
    let is_link = std::fs::symlink_metadata(former_copy).is_ok_and(|m| m.file_type().is_symlink());
    if !is_link {
        if let (Ok(copy), Ok(root)) = (former_copy.canonicalize(), former_root.canonicalize()) {
            if root.starts_with(&copy) {
                anyhow::bail!(SignalError::RemovalRefused {
                    path: former_copy.to_string_lossy().into_owned(),
                    resolved: copy.to_string_lossy().into_owned(),
                    reason: crate::core::errors::RemovalRefusal::CentralLibrary,
                });
            }
        }
    }
    if is_present(former_copy) {
        let claim = [holder.to_path_buf()];
        let library = ProtectedLibrary::new(store, central_dir);
        remove_path_any(
            former_copy,
            &Fence::new(&library).claiming(Claim::Within(&claim)),
        )?;
    }
    Ok(())
}

/// Move Library folder's rollback (`core::library_move`): remove a copy it
/// staged at `staged` inside the destination `dest_root` before the switch,
/// when a later copy failed. Nothing is recorded there yet; the library
/// (`central_dir`, the still-configured one, and every recorded copy) stays
/// protected and the removal is claimed inside `dest_root`. Unlocked
/// internal seam.
pub(crate) fn remove_staged_library_copy_unlocked(
    store: &SkillStore,
    central_dir: &Path,
    dest_root: &Path,
    staged: &Path,
) -> Result<()> {
    if is_present(staged) {
        let claim = [dest_root.to_path_buf()];
        let library = ProtectedLibrary::new(store, central_dir);
        remove_path_any(
            staged,
            &Fence::new(&library).claiming(Claim::Within(&claim)),
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Entry points (each takes the mutation guard)
// ---------------------------------------------------------------------------

fn planned(store: &SkillStore, central_dir: &Path, scope: RemovalScope) -> Result<RemovalReport> {
    let plan = plan(store, &scope)?;
    execute_unlocked(store, central_dir, plan)
}

/// Delete a Managed skill everywhere: every global and project artifact,
/// then the central copy and the record.
///
/// When any artifact could not be removed, its row is kept with Sync status
/// `error`, the skill itself is kept (so the failure is retryable), and the
/// report carries the failed targets and `record_deleted: false`.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
pub fn remove_skill(
    store: &SkillStore,
    central_dir: &Path,
    roots: &HostRoots,
    skill_id: &str,
) -> Result<RemovalReport> {
    mutation_guard::serialized(|| {
        let report = planned(
            store,
            central_dir,
            RemovalScope::Skill {
                skill_id: skill_id.to_string(),
                roots: roots.clone(),
            },
        )?;
        Ok(report)
    })
}

#[cfg(test)]
#[path = "tests/artifact_removal.rs"]
mod tests;
