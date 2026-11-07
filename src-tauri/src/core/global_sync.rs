//! Global-tool sync orchestration: overwrite policy, writability probing, and
//! one Deployment row per (skill, folder) layered on top of `sync_engine`. The
//! global-skills counterpart of `project_sync.rs`. Which folder serves a
//! requested harness, and how it is deployed into, is placement's answer
//! (`core::placement`); nothing here re-derives a folder from a dir.

use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::Result;
use uuid::Uuid;

use crate::core::{
    content_identity::{self, Source},
    environment::HostRoots,
    errors::{CommandError, SignalError},
    mutation_guard,
    ownership::Deployed,
    placement::{
        self, is_folder_row, row_folder, InstalledSet, PlacementScope, PlacementSettings, ScopeKind,
    },
    removal_fence::ProtectedLibrary,
    skill_store::{SkillRecord, SkillStore, SkillTargetRecord, TargetTransition},
    sync_engine::{self, Replace, SyncOutcome},
    sync_status::{SyncMode, SyncStatus},
    tool_adapters::{adapter_by_key, installed_set_in, ToolAdapter},
};

/// Typed sync failures, classified into `CommandError` at batch settlement.
/// Single-pair callers retain the internal error and its diagnostic chain.
#[derive(Debug)]
pub enum GlobalSyncError {
    ToolNotInstalled {
        tool_key: String,
    },
    /// The pair is Disabled (pair or skill grain) — a skip, never attempted.
    TargetDisabled {
        tool_key: String,
    },
    TargetExists {
        target_path: PathBuf,
        /// The folder in the way, as the overwrite ask shows it (D12).
        occupant: sync_engine::Occupant,
    },
    ToolNotWritable {
        tool_display_name: String,
        skills_dir: PathBuf,
    },
    Other(anyhow::Error),
}

impl fmt::Display for GlobalSyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GlobalSyncError::ToolNotInstalled { tool_key } => {
                write!(f, "tool not installed: {}", tool_key)
            }
            GlobalSyncError::TargetDisabled { tool_key } => {
                write!(f, "target disabled: {}", tool_key)
            }
            GlobalSyncError::TargetExists { target_path, .. } => {
                write!(f, "target already exists: {}", target_path.display())
            }
            GlobalSyncError::ToolNotWritable {
                tool_display_name,
                skills_dir,
            } => write!(
                f,
                "skills dir not writable for {}: {}",
                tool_display_name,
                skills_dir.display()
            ),
            GlobalSyncError::Other(err) => write!(f, "{:#}", err),
        }
    }
}

impl std::error::Error for GlobalSyncError {}

impl From<anyhow::Error> for GlobalSyncError {
    fn from(err: anyhow::Error) -> Self {
        GlobalSyncError::Other(err)
    }
}

/// How an existing sync target may be replaced, beside the occupant rule
/// every row follows (an occupant the row's evidence proves its own is
/// replaced; ADR-0007).
pub struct OverwritePolicy {
    /// Replace the target unconditionally: the operator's Overwrite (D12).
    pub overwrite: bool,
    /// Also replace a target whose content is identical to the skill's
    /// (safe refresh, e.g. re-linking an identical copy), decided when the
    /// target is met.
    pub overwrite_if_same_content: bool,
}

/// Test door to [`sync_into_root`] for a row with no prior identity.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn sync_skill_into_root(
    store: &SkillStore,
    central_dir: &Path,
    adapter: &ToolAdapter,
    tool_root: &Path,
    source: &Path,
    skill_id: &str,
    skill_name: &str,
    policy: &OverwritePolicy,
    deployed: &Deployed,
    key: &str,
    now: i64,
) -> Result<SyncOutcome, GlobalSyncError> {
    sync_into_root(
        store,
        central_dir,
        adapter,
        tool_root,
        source,
        skill_id,
        skill_name,
        policy,
        deployed,
        RowIdentity { key, id: None },
        now,
    )
}

/// Which row a sync records: its key, and — for a row Enable took out of the
/// store to re-record it (`FirstSyncRow`) — the id it had, so the row keeps
/// its identity across the re-record. A stored row keeps its own id whatever
/// `id` says (the record updates it in place); a new row with no `id` gets a
/// fresh one.
#[derive(Clone, Copy)]
struct RowIdentity<'a> {
    key: &'a str,
    id: Option<&'a str>,
}

impl RowIdentity<'_> {
    fn id_for_insert(&self) -> String {
        self.id
            .map(str::to_string)
            .unwrap_or_else(|| Uuid::new_v4().to_string())
    }
}

/// Deterministic single-row sync: refuse a `skill_name` that is not one
/// folder name, probe writability of `tool_root`, apply the overwrite policy
/// and the occupant rule (`deployed`: what the row keyed `row.key` recorded at
/// this destination, [`Deployed::none`] for a new row) when a folder is met —
/// replacing it only physically inside `tool_root` — sync
/// `tool_root/skill_name`, and record the `Synced` row keyed `row.key` — with
/// the identity a copy deployed — through [`TargetTransition::Recorded`]: the
/// row's own key, or the folder key for a folder's new row (one folder, one
/// new row; never a row per harness sharing it). `adapter` is the registry
/// entry whose capability governs the folder (`placement::capability_entry`).
/// The batch engine ([`sync_skills_to_planned_tools`]) drives this per row;
/// tests drive it through `sync_skill_into_root`. Unlocked internal seam —
/// callers reach it through an entry point that has already taken the
/// mutation guard.
#[allow(clippy::too_many_arguments)]
fn sync_into_root(
    store: &SkillStore,
    central_dir: &Path,
    adapter: &ToolAdapter,
    tool_root: &Path,
    source: &Path,
    skill_id: &str,
    skill_name: &str,
    policy: &OverwritePolicy,
    deployed: &Deployed,
    row: RowIdentity<'_>,
    now: i64,
) -> Result<SyncOutcome, GlobalSyncError> {
    // The skill's place in the folder, refused before anything touches the
    // disk when its name is not one folder name (`placement::deployment_path`).
    let target = placement::deployment_path(tool_root, skill_name)?;

    // Pre-check: ensure the skills directory is writable (fixes #20 — Windows OS error 5).
    if let Err(err) = std::fs::create_dir_all(tool_root) {
        if err.kind() == std::io::ErrorKind::PermissionDenied {
            return Err(GlobalSyncError::ToolNotWritable {
                tool_display_name: adapter.display_name.to_string(),
                skills_dir: tool_root.to_path_buf(),
            });
        }
        return Err(GlobalSyncError::Other(anyhow::anyhow!(
            "failed to create skills dir {:?}: {}",
            tool_root,
            err
        )));
    }

    let replace = if policy.overwrite {
        Replace::confirmed()
    } else {
        Replace::if_ours(deployed).or_same_content(policy.overwrite_if_same_content)
    };
    // Whatever the rule admitted, an occupant is replaced only physically
    // inside the planned registry skills dir the destination was derived in.
    let managed = [tool_root.to_path_buf()];

    let outcome = sync_engine::sync_dir_for_tool_with_overwrite(
        adapter,
        source,
        Source::Managed { store, skill_id },
        &target,
        replace.within(&managed),
        &ProtectedLibrary::new(store, central_dir),
    )
    .map_err(|err| classify_sync_error(err, adapter, tool_root, &target))?;

    // This row alone settles: harnesses sharing its folder read it through
    // their folder key, never through a row of their own. A copy records the
    // identity it deployed (the skill's stored one, ADR-0005): the evidence a
    // later removal or replacement checks the copy against.
    let content_hash = outcome
        .mode_used
        .can_drift()
        .then(|| content_identity::read(Source::Managed { store, skill_id }))
        .flatten();
    let target_path = outcome.target_path.to_string_lossy();
    store.transition_skill_target(
        &row.id_for_insert(),
        TargetTransition::Recorded {
            skill_id,
            tool: row.key,
            mode: outcome.mode_used,
            target_path: &target_path,
            synced_at: now,
            content_hash: content_hash.as_deref(),
        },
    )?;

    Ok(outcome)
}

fn classify_sync_error(
    err: anyhow::Error,
    adapter: &ToolAdapter,
    tool_root: &Path,
    target: &Path,
) -> GlobalSyncError {
    // Typed condition raised by `sync_engine` — recovered by downcast through
    // the anyhow chain, never by matching message text (ADR 0001).
    if let Some(exists) = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<sync_engine::TargetExistsError>())
    {
        return GlobalSyncError::TargetExists {
            target_path: target.to_path_buf(),
            occupant: exists.occupant,
        };
    }

    // A removal the fence refused, or one a permission failure stopped,
    // already carries its typed condition (path + next step): keep it rather
    // than flattening it into "the Tool's skills dir is not writable"
    // (1.3.3 release review). `CommandError::from_anyhow` recovers it.
    // (`downcast_ref` on the anyhow error looks through context layers, as
    // `from_anyhow` does; `chain()` would see only the context wrappers.)
    if matches!(
        err.downcast_ref::<crate::core::errors::SignalError>(),
        Some(
            crate::core::errors::SignalError::RemovalPermissionDenied { .. }
                | crate::core::errors::SignalError::RemovalRefused { .. }
        )
    ) {
        return GlobalSyncError::Other(err);
    }

    // External OS failures: inspect the underlying `io::Error` kind instead of
    // sniffing platform-specific message strings. The raw-code check covers
    // Windows ERROR_ACCESS_DENIED (5) in case a wrapper surfaces it with a
    // kind other than `PermissionDenied` (e.g. `Uncategorized`); it is gated
    // to Windows because raw code 5 means EIO on Unix.
    if err.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io_err| {
                io_err.kind() == std::io::ErrorKind::PermissionDenied
                    || (cfg!(windows) && io_err.raw_os_error() == Some(5))
            })
    }) {
        return GlobalSyncError::ToolNotWritable {
            tool_display_name: adapter.display_name.to_string(),
            skills_dir: tool_root.to_path_buf(),
        };
    }

    GlobalSyncError::Other(err)
}

// ---------------------------------------------------------------------------
// Batch sync: the single fan-out engine behind the `sync_skills_to_tools`
// command. Skills × folders in one call; folder dedupe, installedness
// filtering, per-target overwrite policy, and per-target outcomes all live
// here so no caller re-implements the choreography.
// ---------------------------------------------------------------------------

/// One skill to sync in a batch.
#[derive(Debug, Clone)]
pub struct BatchSkill {
    pub skill_id: String,
    pub skill_name: String,
    pub source_path: PathBuf,
}

/// The operator's answer to a `TARGET_EXISTS` ask for one pair (D12).
/// Crosses the wire as itself: `"overwrite"` or `"keep"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum OverrideDecision {
    /// Replace the folder in the way with the managed skill.
    Overwrite,
    /// Keep the existing folder: the pair is not synced and is recorded
    /// `disabled` (a marker owning nothing at its path), so no later pass
    /// re-asks or re-materialises it until the operator enables it.
    Keep,
}

/// The operator's decision for one (skill, row) pair: `tool_key` is the key
/// of the row the ask was raised for (the report row's `tool_key`), and the
/// answer applies to that row alone (`answer_for`). `path` is the path the ask
/// showed (the `TARGET_EXISTS` row's): the answer applies only while the row's
/// destination is still that path, else it is stale and nothing is touched.
/// `None` only for a decision taken before any ask (the Add flow's
/// pre-install overrides).
#[derive(Debug, Clone)]
pub struct BatchOverride {
    pub skill_id: String,
    pub tool_key: String,
    pub decision: OverrideDecision,
    pub path: Option<PathBuf>,
}

/// Batch-level default policy plus per-(skill, tool) overrides.
#[derive(Debug, Default)]
pub struct BatchPolicy {
    pub overwrite: bool,
    pub overwrite_if_same_content: bool,
    pub overrides: Vec<BatchOverride>,
}

/// A probed Deployment folder: its folder key, the registry entry whose
/// capability governs it, its resolved skills root, whether the folder is
/// reachable (any harness it serves is installed) and the detected set a
/// legacy row's own harness is looked up in (`placement::is_folder_row`).
/// Produced by [`plan_batch_tool_targets`] (placement over the environment);
/// consumed by the deterministic [`sync_skills_to_planned_tools`], which tests
/// drive with fabricated roots.
#[derive(Debug)]
pub struct PlannedToolTarget {
    pub adapter: &'static ToolAdapter,
    pub folder_key: String,
    pub root: PathBuf,
    pub installed: bool,
    pub detected: InstalledSet,
}

/// Per-(skill, tool) result. `Skipped` is the expected-and-ignorable class
/// (tool absent, dir unwritable) — callers decide whether to surface it;
/// `Failed` is everything else. Both carry the typed error, so no
/// information is lost by the classification. `Declined` is the operator's
/// own answer, neither a skip nor a failure (D10).
#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum BatchTargetStatus {
    Synced {
        outcome: SyncOutcome,
    },
    Skipped {
        error: CommandError,
    },
    Failed {
        error: CommandError,
    },
    /// The operator answered the `TARGET_EXISTS` ask with "Keep existing"
    /// (`OverrideDecision::Keep`): the folder in the way was left alone and
    /// the pair is recorded `disabled`. Only this batch's override produces
    /// it; a later pass reports the same pair as a Disabled skip.
    Declined,
}

/// One row's result. `tool_key` is the raw row the outcome settled (a
/// pre-1.4.0 shared-dir sibling's row keeps its own key) — the key an
/// overwrite answer names. `folder_key` is the Deployment folder that row
/// describes (`placement::row_folder`): the backend's answer to "which
/// Deployment is this", so a report reader counts one Deployment per (skill,
/// folder) however many legacy rows describe it, and never derives a folder.
#[derive(Debug, serde::Serialize, specta::Type)]
pub struct BatchTargetOutcome {
    pub skill_id: String,
    pub skill_name: String,
    pub tool_key: String,
    pub folder_key: String,
    pub status: BatchTargetStatus,
}

/// Progress tick emitted before each attempted (skill, tool) pair.
pub struct BatchProgress<'a> {
    /// 1-based index over attempted pairs.
    pub index: usize,
    pub total: usize,
    pub skill_name: &'a str,
    pub tool_key: &'a str,
}

/// Place each requested key under `roots`: the folder serving it
/// (`placement::place`, the layout read once for the whole batch), that
/// folder's root, the entry whose capability governs it, and whether any
/// harness it serves is installed. A key may name a harness or a folder (a
/// folder key is its own entry's key). Unknown keys become planning entries
/// via `Err`, which the batch turns into `Failed` outcomes.
pub fn plan_batch_tool_targets(
    roots: &HostRoots,
    tool_keys: &[String],
) -> Vec<Result<PlannedToolTarget, (String, GlobalSyncError)>> {
    let detected = installed_set_in(roots);
    let layout = placement::place(
        PlacementScope::Global(roots),
        &detected,
        &PlacementSettings::default(),
    );
    tool_keys
        .iter()
        .map(|key| {
            let unknown = || {
                (
                    key.clone(),
                    GlobalSyncError::Other(anyhow::anyhow!("unknown tool: {}", key)),
                )
            };
            adapter_by_key(key).ok_or_else(unknown)?;
            let folder = layout.folder_of(key).ok_or_else(unknown)?;
            let adapter =
                placement::capability_entry(ScopeKind::Global, folder.key).ok_or_else(unknown)?;
            Ok(PlannedToolTarget {
                adapter,
                folder_key: folder.key.to_string(),
                root: folder.path.clone(),
                installed: !folder.served.is_empty(),
                detected: detected.clone(),
            })
        })
        .collect()
}

/// One row of Enable's first-sync plan: a `disabled` row Enable deleted, by
/// the identity it is re-recorded under (its skill and its own key). The
/// batch acts on exactly these rows — no other row of their folders, so a
/// record outside the Enable's scope is never rewritten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FirstSyncRow {
    pub skill_id: String,
    pub key: String,
    /// The id the taken row had: the row is re-recorded under it, so it
    /// keeps its identity across Enable's delete and re-record.
    pub id: String,
}

/// Which Enable a first-sync plan comes from: what it re-materialises beside
/// the taken rows, as Enable always did at that grain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FirstSyncGrain {
    /// A pair: the operator turned the folder back on. A taken legacy row
    /// whose own harness is gone is dropped silently, and a reachable folder
    /// none of whose taken rows is reached gets its one new row.
    Pair,
    /// A skill: every taken row is re-synced under its own key, and one whose
    /// own harness is gone is reported `ToolNotInstalled`.
    Skill,
}

/// Which rows the batch acts on in each attempted folder.
#[derive(Clone, Copy)]
enum RowSelection<'a> {
    /// Every stored row of the folder ([`batch_rows`]).
    Stored,
    /// Exactly the rows of Enable's first-sync plan.
    FirstSync(&'a [FirstSyncRow], FirstSyncGrain),
}

/// One row a batch acts on: an existing row's key, a planned row's key (with
/// the id it is re-recorded under), or the folder key of the folder's
/// row-to-be.
struct BatchRow {
    key: String,
    id: Option<String>,
}

impl BatchRow {
    fn identity(&self) -> RowIdentity<'_> {
        RowIdentity {
            key: &self.key,
            id: self.id.as_deref(),
        }
    }
}

/// A row is acted on only where its own reach holds: a folder row through
/// its folder (the target is attempted, so it is), a legacy row through its
/// own harness, as before rows were per folder (`placement::is_folder_row`).
fn acts_on(target: &PlannedToolTarget, key: &str) -> bool {
    is_folder_row(ScopeKind::Global, key) || target.detected.contains(key)
}

/// The rows of `skill` the batch acts on for `target`'s folder.
///
/// - [`RowSelection::Stored`]: every stored row describing the folder
///   (`placement::row_folder`) that is reachable by its own rule
///   ([`acts_on`]) — a legacy row whose own harness is not installed is left
///   untouched, as it was before; when no stored row is acted on, the one new
///   row keyed by the folder (no row is added per harness sharing it).
/// - [`RowSelection::FirstSync`]: the planned rows of the folder, each under
///   its own key. A planned legacy row whose harness is not installed comes
///   back in the second list at skill grain, to be reported
///   `ToolNotInstalled` as before; at pair grain it is dropped, and when no
///   planned row is reached the folder's one new row is synced instead
///   ([`FirstSyncGrain`]).
fn batch_rows(
    rows: &[SkillTargetRecord],
    target: &PlannedToolTarget,
    skill: &BatchSkill,
    selection: RowSelection<'_>,
) -> (Vec<BatchRow>, Vec<BatchRow>) {
    match selection {
        RowSelection::Stored => {
            let acted: Vec<BatchRow> = rows
                .iter()
                .filter(|row| row_folder(ScopeKind::Global, &row.tool) == target.folder_key)
                .filter(|row| acts_on(target, &row.tool))
                .map(|row| BatchRow {
                    key: row.tool.clone(),
                    id: None,
                })
                .collect();
            if acted.is_empty() {
                (
                    vec![BatchRow {
                        key: target.folder_key.clone(),
                        id: None,
                    }],
                    Vec::new(),
                )
            } else {
                (acted, Vec::new())
            }
        }
        RowSelection::FirstSync(plan, grain) => {
            let (acted, unreached): (Vec<BatchRow>, Vec<BatchRow>) = plan
                .iter()
                .filter(|row| {
                    row.skill_id == skill.skill_id
                        && row_folder(ScopeKind::Global, &row.key) == target.folder_key
                })
                .map(|row| BatchRow {
                    key: row.key.clone(),
                    id: Some(row.id.clone()),
                })
                .partition(|row| acts_on(target, &row.key));
            match grain {
                FirstSyncGrain::Skill => (acted, unreached),
                FirstSyncGrain::Pair if acted.is_empty() => (
                    vec![BatchRow {
                        key: target.folder_key.clone(),
                        id: None,
                    }],
                    Vec::new(),
                ),
                FirstSyncGrain::Pair => (acted, Vec::new()),
            }
        }
    }
}

/// Deterministic batch engine: dedupe targets by folder key (a folder is
/// planned once, whichever of its harnesses was asked for), emit `Skipped`
/// for a folder no installed harness reads, then act on **each row** of each
/// attempted folder by itself ([`batch_rows`]): its own key, the Disabled
/// rule, its own overwrite policy and its own settlement, written where the
/// registry places the skill now — the folder's root plus the skill's current
/// name, whatever path the row recorded before. Nothing is lent or settled
/// from one row to another; a folder with no row acted on gets one new row
/// keyed by the folder. Failures are isolated per row — one bad row never
/// aborts the batch. Unlocked internal seam — callers reach it through an
/// entry point that has already taken the mutation guard. Production reaches
/// it through [`sync_skills_to_tools_unlocked`]; tests drive it directly.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn sync_skills_to_planned_tools(
    store: &SkillStore,
    central_dir: &Path,
    skills: &[BatchSkill],
    targets: &[PlannedToolTarget],
    policy: &BatchPolicy,
    now: i64,
    on_progress: impl FnMut(BatchProgress),
) -> Vec<BatchTargetOutcome> {
    run_planned(
        store,
        central_dir,
        skills,
        targets,
        RowSelection::Stored,
        policy,
        now,
        on_progress,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_planned(
    store: &SkillStore,
    central_dir: &Path,
    skills: &[BatchSkill],
    targets: &[PlannedToolTarget],
    selection: RowSelection<'_>,
    policy: &BatchPolicy,
    now: i64,
    mut on_progress: impl FnMut(BatchProgress),
) -> Vec<BatchTargetOutcome> {
    let mut outcomes: Vec<BatchTargetOutcome> = Vec::new();

    // Partition: one target per folder; attempted = reachable, the rest skip.
    let mut seen_folders: Vec<&str> = Vec::new();
    let mut attempted: Vec<&PlannedToolTarget> = Vec::new();
    let mut skipped_not_installed: Vec<&PlannedToolTarget> = Vec::new();
    for target in targets {
        if seen_folders.contains(&target.folder_key.as_str()) {
            continue; // the folder is already planned
        }
        seen_folders.push(target.folder_key.as_str());
        if target.installed {
            attempted.push(target);
        } else {
            skipped_not_installed.push(target);
        }
    }

    let not_installed = |skill: &BatchSkill, tool_key: &str, folder_key: &str| BatchTargetOutcome {
        skill_id: skill.skill_id.clone(),
        skill_name: skill.skill_name.clone(),
        tool_key: tool_key.to_string(),
        folder_key: folder_key.to_string(),
        status: BatchTargetStatus::Skipped {
            error: CommandError::from(GlobalSyncError::ToolNotInstalled {
                tool_key: tool_key.to_string(),
            }),
        },
    };

    for target in &skipped_not_installed {
        let folder_key = target.folder_key.as_str();
        for skill in skills {
            match selection {
                RowSelection::Stored => outcomes.push(not_installed(skill, folder_key, folder_key)),
                // Each planned row is reported under its own key.
                // A pair names its folder, as the operator asked for it.
                RowSelection::FirstSync(_, FirstSyncGrain::Pair) => {
                    outcomes.push(not_installed(skill, folder_key, folder_key))
                }
                RowSelection::FirstSync(plan, FirstSyncGrain::Skill) => outcomes.extend(
                    plan.iter()
                        .filter(|row| {
                            row.skill_id == skill.skill_id
                                && row_folder(ScopeKind::Global, &row.key) == folder_key
                        })
                        .map(|row| not_installed(skill, &row.key, folder_key)),
                ),
            }
        }
    }

    let total = skills.len() * attempted.len();
    let mut index = 0usize;
    for skill in skills {
        let rows = store.list_skill_targets(&skill.skill_id);
        // The record a row's evidence names (its Library copy); a read
        // failure leaves the evidence without one (nothing is proved ours).
        let record = store.get_skill_by_id(&skill.skill_id).ok().flatten();
        let skill_disabled = store
            .skill_disabled_at(&skill.skill_id)
            .map(|at| at.is_some());
        for target in &attempted {
            index += 1;
            on_progress(BatchProgress {
                index,
                total,
                skill_name: &skill.skill_name,
                tool_key: &target.folder_key,
            });

            // The rows of this folder, and the Disabled rule over them: the
            // skill is off at skill grain, or a row of the folder is
            // `disabled` — writing the shared artifact for any one of its rows
            // would re-materialise it where the disabled harness reads. The
            // rule only withholds; it never grants a write.
            let ((acted, unreached), folder_off) = match (&rows, &skill_disabled) {
                (Ok(rows), Ok(skill_disabled)) => {
                    let folder_off = *skill_disabled
                        || rows.iter().any(|row| {
                            row.status == SyncStatus::Disabled
                                && row_folder(ScopeKind::Global, &row.tool) == target.folder_key
                        });
                    (batch_rows(rows, target, skill, selection), folder_off)
                }
                (Err(error), _) | (_, Err(error)) => {
                    outcomes.push(BatchTargetOutcome {
                        skill_id: skill.skill_id.clone(),
                        skill_name: skill.skill_name.clone(),
                        tool_key: target.folder_key.clone(),
                        folder_key: target.folder_key.clone(),
                        status: BatchTargetStatus::Failed {
                            error: CommandError::internal(format!("{error:#}")),
                        },
                    });
                    continue;
                }
            };

            for row in &unreached {
                outcomes.push(not_installed(skill, &row.key, &target.folder_key));
            }
            for row in &acted {
                let status = if folder_off {
                    BatchTargetStatus::Skipped {
                        error: CommandError::from(GlobalSyncError::TargetDisabled {
                            tool_key: row.key.clone(),
                        }),
                    }
                } else {
                    let deployed = match &rows {
                        Ok(stored) => row_evidence(stored, target, skill, row, record.as_ref()),
                        Err(_) => Deployed::none(),
                    };
                    sync_row(
                        store,
                        central_dir,
                        target,
                        skill,
                        row,
                        &acted,
                        policy,
                        &deployed,
                        now,
                    )
                };
                outcomes.push(BatchTargetOutcome {
                    skill_id: skill.skill_id.clone(),
                    skill_name: skill.skill_name.clone(),
                    tool_key: row.key.clone(),
                    folder_key: target.folder_key.clone(),
                    status,
                });
            }
        }
    }

    outcomes
}

/// What the stored row keyed `row.key` recorded **at the skill's place in
/// the folder** ([`destination`]) — the only path its evidence speaks for. A
/// row recorded at another path (a skill renamed since), or no row yet, has
/// none there.
fn row_evidence(
    stored: &[SkillTargetRecord],
    target: &PlannedToolTarget,
    skill: &BatchSkill,
    row: &BatchRow,
    record: Option<&SkillRecord>,
) -> Deployed {
    let destination = destination(target, skill);
    stored
        .iter()
        .find(|stored| stored.tool == row.key && Path::new(&stored.target_path) == destination)
        .map(|stored| Deployed::of_target(stored, record))
        .unwrap_or_default()
}

/// One row of an attempted folder: apply the operator's answer for exactly
/// this row ([`answer_for`]), else the batch policy and the occupant rule
/// over the row's own evidence (`deployed`), sync at the skill's place in the
/// folder (the registry root plus its current name) and settle the row under
/// its own key.
#[allow(clippy::too_many_arguments)]
fn sync_row(
    store: &SkillStore,
    central_dir: &Path,
    target: &PlannedToolTarget,
    skill: &BatchSkill,
    row: &BatchRow,
    rows: &[BatchRow],
    policy: &BatchPolicy,
    deployed: &Deployed,
    now: i64,
) -> BatchTargetStatus {
    // A name that is not one folder name has no place in the folder: refused
    // before anything is touched or recorded (`placement::deployment_path`).
    if let Err(refusal) = placement::deployment_path(&target.root, &skill.skill_name) {
        return BatchTargetStatus::Failed {
            error: CommandError::from_anyhow(refusal),
        };
    }
    let answer = answer_for(policy, skill, target, row, rows);
    // An answer applies to the path its ask showed, and only there: a
    // destination that moved since (the skill renamed, the folder relocated)
    // makes it stale — nothing at either path is touched, nothing recorded.
    if let Some(asked) = answer.and_then(|answer| answer.path.as_deref()) {
        if asked != destination(target, skill) {
            return BatchTargetStatus::Failed {
                error: CommandError::from(SignalError::NotFound {
                    kind: "target".to_string(),
                    id: asked.to_string_lossy().into_owned(),
                }),
            };
        }
    }
    let answer = answer.map(|answer| answer.decision);
    if answer == Some(OverrideDecision::Keep) {
        return record_kept(store, target, skill, row);
    }
    let pair_policy = OverwritePolicy {
        overwrite: policy.overwrite || answer == Some(OverrideDecision::Overwrite),
        overwrite_if_same_content: policy.overwrite_if_same_content,
    };
    match sync_into_root(
        store,
        central_dir,
        target.adapter,
        &target.root,
        &skill.source_path,
        &skill.skill_id,
        &skill.skill_name,
        &pair_policy,
        deployed,
        row.identity(),
        now,
    ) {
        Ok(outcome) => BatchTargetStatus::Synced { outcome },
        Err(error) => {
            // Every failure but an unanswered collision settles the row
            // `error` (D9); a collision records nothing until the operator
            // answers the ask (D12).
            if !matches!(error, GlobalSyncError::TargetExists { .. }) {
                record_failed(store, target, skill, row, &error);
            }
            match error {
                GlobalSyncError::ToolNotWritable { .. } => BatchTargetStatus::Skipped {
                    error: CommandError::from(error),
                },
                error => BatchTargetStatus::Failed {
                    error: CommandError::from(error),
                },
            }
        }
    }
}

/// Where the skill belongs in `target`'s folder now: the registry root plus
/// its current name — what every row of the folder is written and recorded at
/// (a name that is not one folder name is refused before a write or record,
/// [`sync_row`]).
fn destination(target: &PlannedToolTarget, skill: &BatchSkill) -> PathBuf {
    target.root.join(&skill.skill_name)
}

/// The operator's answer (D12) that applies to `row`: an override naming the
/// row's own key — the key the ask's report row carried — so an answer reaches
/// exactly the row, and the path, it was asked about. An override naming
/// another key of the same folder (a harness rather than the folder key)
/// applies only where that cannot be ambiguous: the folder has a single row
/// and no override names it exactly.
fn answer_for<'p>(
    policy: &'p BatchPolicy,
    skill: &BatchSkill,
    target: &PlannedToolTarget,
    row: &BatchRow,
    rows: &[BatchRow],
) -> Option<&'p BatchOverride> {
    let for_skill = || {
        policy
            .overrides
            .iter()
            .filter(|o| o.skill_id == skill.skill_id)
    };
    if let Some(exact) = for_skill().find(|o| o.tool_key == row.key) {
        return Some(exact);
    }
    if rows.len() != 1 {
        return None;
    }
    for_skill().find(|o| row_folder(ScopeKind::Global, &o.tool_key) == target.folder_key)
}

/// The mode a sync of `target`'s folder tries first — what a row that never
/// materialised records.
fn first_attempt_mode(target: &PlannedToolTarget) -> SyncMode {
    if target.adapter.supports_symlink {
        SyncMode::Symlink
    } else {
        SyncMode::Copy
    }
}

/// A failed sync settles its row here (D9, symmetric with ADR-0002): the row
/// is recorded `error` with the diagnostic chain — a new row at the skill's
/// place in the folder, owning nothing (`synced_at` NULL); an existing one
/// keeps its key, path and ownership. A
/// store failure here is logged: the report row already carries the sync
/// failure, which is what the operator acts on.
fn record_failed(
    store: &SkillStore,
    target: &PlannedToolTarget,
    skill: &BatchSkill,
    row: &BatchRow,
    error: &GlobalSyncError,
) {
    let detail = error.to_string();
    if let Err(err) = store.transition_skill_target(
        &row.identity().id_for_insert(),
        TargetTransition::FailedRecorded {
            skill_id: &skill.skill_id,
            tool: &row.key,
            mode: first_attempt_mode(target),
            target_path: &destination(target, skill).to_string_lossy(),
            error: &detail,
        },
    ) {
        log::warn!(
            "recording the failed sync of {} to {} failed: {err:#}",
            skill.skill_id,
            row.key
        );
    }
}

/// The operator kept the folder in the way (`OverrideDecision::Keep`): the
/// row is recorded `disabled` under its own key — a new row at the skill's
/// place in the folder, a marker owning nothing there; an existing row only
/// flips — and reported `Declined`. A store failure fails
/// the row as report data.
fn record_kept(
    store: &SkillStore,
    target: &PlannedToolTarget,
    skill: &BatchSkill,
    row: &BatchRow,
) -> BatchTargetStatus {
    match store.transition_skill_target(
        &row.identity().id_for_insert(),
        TargetTransition::DisabledRecorded {
            skill_id: &skill.skill_id,
            tool: &row.key,
            mode: first_attempt_mode(target),
            target_path: &destination(target, skill).to_string_lossy(),
        },
    ) {
        Ok(()) => BatchTargetStatus::Declined,
        Err(err) => BatchTargetStatus::Failed {
            error: CommandError::internal(format!("{err:#}")),
        },
    }
}

/// Sync N skills to M tools in one call: environment probing under `roots`
/// ([`plan_batch_tool_targets`]) composed with the deterministic engine
/// ([`sync_skills_to_planned_tools`]). Planning failures surface as `Failed`
/// outcomes per skill; the function itself never errors.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
#[allow(clippy::too_many_arguments)]
pub fn sync_skills_to_tools(
    roots: &HostRoots,
    store: &SkillStore,
    central_dir: &Path,
    skills: &[BatchSkill],
    tool_keys: &[String],
    policy: &BatchPolicy,
    now: i64,
    on_progress: impl FnMut(BatchProgress),
) -> Vec<BatchTargetOutcome> {
    mutation_guard::serialized(|| {
        sync_skills_to_tools_unlocked(
            roots,
            store,
            central_dir,
            skills,
            tool_keys,
            policy,
            now,
            on_progress,
        )
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn sync_skills_to_tools_unlocked(
    roots: &HostRoots,
    store: &SkillStore,
    central_dir: &Path,
    skills: &[BatchSkill],
    tool_keys: &[String],
    policy: &BatchPolicy,
    now: i64,
    on_progress: impl FnMut(BatchProgress),
) -> Vec<BatchTargetOutcome> {
    plan_and_run(
        roots,
        store,
        central_dir,
        skills,
        tool_keys,
        RowSelection::Stored,
        policy,
        now,
        on_progress,
    )
}

/// Enable's global first sync (`core::skill_disable`): the batch over the
/// folders of `plan`'s rows, acting on exactly those rows — each recorded
/// under its own key at the skill's place in its folder, a legacy one only
/// where its own harness is installed, as before rows were per folder
/// ([`FirstSyncGrain`] says what the Enable's grain adds). No other stored
/// row of those folders is synced or settled. Unlocked internal seam.
#[allow(clippy::too_many_arguments)]
pub(crate) fn first_sync_rows_unlocked(
    roots: &HostRoots,
    store: &SkillStore,
    central_dir: &Path,
    skills: &[BatchSkill],
    plan: &[FirstSyncRow],
    grain: FirstSyncGrain,
    policy: &BatchPolicy,
    now: i64,
) -> Vec<BatchTargetOutcome> {
    let mut folders: Vec<String> = Vec::new();
    for row in plan {
        let folder = row_folder(ScopeKind::Global, &row.key);
        if !folders.contains(&folder) {
            folders.push(folder);
        }
    }
    plan_and_run(
        roots,
        store,
        central_dir,
        skills,
        &folders,
        RowSelection::FirstSync(plan, grain),
        policy,
        now,
        |_| {},
    )
}

#[allow(clippy::too_many_arguments)]
fn plan_and_run(
    roots: &HostRoots,
    store: &SkillStore,
    central_dir: &Path,
    skills: &[BatchSkill],
    tool_keys: &[String],
    selection: RowSelection<'_>,
    policy: &BatchPolicy,
    now: i64,
    on_progress: impl FnMut(BatchProgress),
) -> Vec<BatchTargetOutcome> {
    let mut targets: Vec<PlannedToolTarget> = Vec::new();
    let mut outcomes: Vec<BatchTargetOutcome> = Vec::new();
    for plan in plan_batch_tool_targets(roots, tool_keys) {
        match plan {
            Ok(target) => targets.push(target),
            Err((tool_key, error)) => {
                // One planning failure per skill, mirroring attempted shape.
                let error = CommandError::from(error);
                for skill in skills {
                    outcomes.push(BatchTargetOutcome {
                        skill_id: skill.skill_id.clone(),
                        skill_name: skill.skill_name.clone(),
                        folder_key: row_folder(ScopeKind::Global, &tool_key),
                        tool_key: tool_key.clone(),
                        status: BatchTargetStatus::Failed {
                            error: error.clone(),
                        },
                    });
                }
            }
        }
    }
    outcomes.extend(run_planned(
        store,
        central_dir,
        skills,
        &targets,
        selection,
        policy,
        now,
        on_progress,
    ));
    outcomes
}

#[cfg(test)]
#[path = "tests/global_sync.rs"]
mod tests;
