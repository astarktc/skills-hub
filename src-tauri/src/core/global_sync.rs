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
    errors::CommandError,
    mutation_guard,
    placement::{self, row_folder, PlacementScope, PlacementSettings, ScopeKind},
    removal_fence::ProtectedLibrary,
    skill_store::{SkillStore, TargetTransition},
    sync_engine::{self, SyncOutcome},
    sync_status::SyncMode,
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

/// How an existing sync target may be replaced.
pub struct OverwritePolicy {
    /// Replace the target unconditionally.
    pub overwrite: bool,
    /// Replace the target only when its content hash matches the source
    /// (safe refresh, e.g. re-linking an identical copy).
    pub overwrite_if_same_content: bool,
}

/// Deterministic single-pair sync: probe writability of `tool_root`, apply
/// the overwrite policy, sync, and record the one `Synced` Deployment row of
/// the folder `folder_key` through [`TargetTransition::Recorded`]. `adapter`
/// is the registry entry whose capability governs the folder
/// (`placement::capability_entry`). The batch engine
/// ([`sync_planned_tools_lifting`]) drives this per attempted pair; tests
/// drive it directly. Unlocked internal seam — callers reach it through an
/// entry point that has already taken the mutation guard.
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
    folder_key: &str,
    now: i64,
) -> Result<SyncOutcome, GlobalSyncError> {
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

    let target = tool_root.join(skill_name);
    let overwrite = policy.overwrite
        || (policy.overwrite_if_same_content
            && content_identity::same_content(Source::Managed { store, skill_id }, &target));

    let outcome = sync_engine::sync_dir_for_tool_with_overwrite(
        adapter,
        source,
        Source::Managed { store, skill_id },
        &target,
        overwrite,
        &ProtectedLibrary::new(store, central_dir),
    )
    .map_err(|err| classify_sync_error(err, adapter, tool_root, &target))?;

    // One folder, one row: harnesses sharing it read this row through their
    // folder key, never through a row of their own.
    let target_path = outcome.target_path.to_string_lossy();
    store.transition_skill_target(
        &Uuid::new_v4().to_string(),
        TargetTransition::Recorded {
            skill_id,
            tool: folder_key,
            mode: outcome.mode_used,
            target_path: &target_path,
            synced_at: now,
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

/// The operator's decision for one (skill, tool) pair. Applies to the folder
/// the named key's row describes (`placement::row_folder`): the folder, not
/// the harness, is the identity of a Deployment.
#[derive(Debug, Clone)]
pub struct BatchOverride {
    pub skill_id: String,
    pub tool_key: String,
    pub decision: OverrideDecision,
}

/// Batch-level default policy plus per-(skill, tool) overrides.
#[derive(Debug, Default)]
pub struct BatchPolicy {
    pub overwrite: bool,
    pub overwrite_if_same_content: bool,
    pub overrides: Vec<BatchOverride>,
}

/// A probed Deployment folder: its folder key, the registry entry whose
/// capability governs it, its resolved skills root, and whether any harness it
/// serves is installed. Produced by [`plan_batch_tool_targets`] (placement
/// over the environment); consumed by the deterministic
/// [`sync_planned_tools_lifting`], which tests drive with fabricated roots.
#[derive(Debug)]
pub struct PlannedToolTarget {
    pub adapter: &'static ToolAdapter,
    pub folder_key: String,
    pub root: PathBuf,
    pub installed: bool,
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

#[derive(Debug, serde::Serialize, specta::Type)]
pub struct BatchTargetOutcome {
    pub skill_id: String,
    pub skill_name: String,
    pub tool_key: String,
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
    let layout = placement::place(
        PlacementScope::Global(roots),
        &installed_set_in(roots),
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
            })
        })
        .collect()
}

/// The batch engine with nothing lifted — test fixtures' door, driven with
/// fabricated roots ([`sync_planned_tools_lifting`]).
#[cfg(test)]
pub(crate) fn sync_skills_to_planned_tools(
    store: &SkillStore,
    central_dir: &Path,
    skills: &[BatchSkill],
    targets: &[PlannedToolTarget],
    policy: &BatchPolicy,
    now: i64,
    on_progress: impl FnMut(BatchProgress),
) -> Vec<BatchTargetOutcome> {
    sync_planned_tools_lifting(
        store,
        central_dir,
        skills,
        targets,
        policy,
        &[],
        now,
        on_progress,
    )
}

/// Deterministic batch engine: dedupe targets by folder key (a folder is
/// written once, whichever of its harnesses was asked for, and every outcome
/// names the folder), emit `Skipped` for a folder no installed harness reads,
/// apply the Disabled rule and the per-pair overwrite policy, and isolate
/// failures per target — one bad pair never aborts the batch. Unlocked
/// internal seam — callers reach it through an entry point that has already
/// taken the mutation guard.
///
/// `lifted` (Enable only, else empty) names the folders whose Disabled state
/// Enable has just lifted (`skill_disable`): a row still `disabled` there — an
/// independent row recording another path, never the folder's own (Enable
/// cleared those) — does not block the folder. A skill disabled at skill
/// grain still blocks everything.
#[allow(clippy::too_many_arguments)]
fn sync_planned_tools_lifting(
    store: &SkillStore,
    central_dir: &Path,
    skills: &[BatchSkill],
    targets: &[PlannedToolTarget],
    policy: &BatchPolicy,
    lifted: &[String],
    now: i64,
    mut on_progress: impl FnMut(BatchProgress),
) -> Vec<BatchTargetOutcome> {
    let mut outcomes: Vec<BatchTargetOutcome> = Vec::new();

    // Partition: one target per folder; attempted = installed, the rest skip.
    let mut seen_folders: Vec<&str> = Vec::new();
    let mut attempted: Vec<&PlannedToolTarget> = Vec::new();
    let mut skipped_not_installed: Vec<&PlannedToolTarget> = Vec::new();
    for target in targets {
        if seen_folders.contains(&target.folder_key.as_str()) {
            continue; // the folder is already planned: one folder, one row
        }
        seen_folders.push(target.folder_key.as_str());
        if target.installed {
            attempted.push(target);
        } else {
            skipped_not_installed.push(target);
        }
    }

    for target in &skipped_not_installed {
        let tool_key = target.folder_key.as_str();
        for skill in skills {
            outcomes.push(BatchTargetOutcome {
                skill_id: skill.skill_id.clone(),
                skill_name: skill.skill_name.clone(),
                tool_key: tool_key.to_string(),
                status: BatchTargetStatus::Skipped {
                    error: CommandError::from(GlobalSyncError::ToolNotInstalled {
                        tool_key: tool_key.to_string(),
                    }),
                },
            });
        }
    }

    let total = skills.len() * attempted.len();
    let mut index = 0usize;
    for skill in skills {
        // Read once per skill: a store failure fails this skill's pairs as
        // report data, never the batch (the engine itself never errors).
        let gate = DisabledGate::read(store, &skill.skill_id);
        for target in &attempted {
            index += 1;
            let tool_key = target.folder_key.as_str();
            on_progress(BatchProgress {
                index,
                total,
                skill_name: &skill.skill_name,
                tool_key,
            });

            let blocked = match &gate {
                Ok(gate) => {
                    gate.blocks(&target.folder_key, lifted)
                        .then(|| BatchTargetStatus::Skipped {
                            error: CommandError::from(GlobalSyncError::TargetDisabled {
                                tool_key: tool_key.to_string(),
                            }),
                        })
                }
                Err(error) => Some(BatchTargetStatus::Failed {
                    error: CommandError::internal(format!("{error:#}")),
                }),
            };
            if let Some(status) = blocked {
                outcomes.push(BatchTargetOutcome {
                    skill_id: skill.skill_id.clone(),
                    skill_name: skill.skill_name.clone(),
                    tool_key: tool_key.to_string(),
                    status,
                });
                continue;
            }

            // An override names a key; it applies to this target when that
            // key's row describes the target's folder.
            let decision = |wanted: OverrideDecision| {
                policy.overrides.iter().any(|o| {
                    o.decision == wanted
                        && o.skill_id == skill.skill_id
                        && row_folder(ScopeKind::Global, &o.tool_key) == target.folder_key
                })
            };
            if decision(OverrideDecision::Keep) {
                outcomes.push(BatchTargetOutcome {
                    skill_id: skill.skill_id.clone(),
                    skill_name: skill.skill_name.clone(),
                    tool_key: tool_key.to_string(),
                    status: record_kept(store, target, skill),
                });
                continue;
            }
            let overwrite = policy.overwrite || decision(OverrideDecision::Overwrite);
            let pair_policy = OverwritePolicy {
                overwrite,
                overwrite_if_same_content: policy.overwrite_if_same_content,
            };

            let status = match sync_skill_into_root(
                store,
                central_dir,
                target.adapter,
                &target.root,
                &skill.source_path,
                &skill.skill_id,
                &skill.skill_name,
                &pair_policy,
                &target.folder_key,
                now,
            ) {
                Ok(outcome) => BatchTargetStatus::Synced { outcome },
                // Asked first (D12): no row until the operator decides.
                Err(error @ GlobalSyncError::TargetExists { .. }) => BatchTargetStatus::Failed {
                    error: CommandError::from(error),
                },
                Err(error) => {
                    record_failed(store, target, skill, &error);
                    match error {
                        GlobalSyncError::ToolNotWritable { .. } => BatchTargetStatus::Skipped {
                            error: CommandError::from(error),
                        },
                        error => BatchTargetStatus::Failed {
                            error: CommandError::from(error),
                        },
                    }
                }
            };
            outcomes.push(BatchTargetOutcome {
                skill_id: skill.skill_id.clone(),
                skill_name: skill.skill_name.clone(),
                tool_key: tool_key.to_string(),
                status,
            });
        }
    }

    outcomes
}

/// Where an attempted target's artifact is (or would be), and the mode a
/// sync would try first — what a row that never materialised records.
fn would_be(target: &PlannedToolTarget, skill: &BatchSkill) -> (String, SyncMode) {
    let path = target.root.join(&skill.skill_name);
    let mode = if target.adapter.supports_symlink {
        SyncMode::Symlink
    } else {
        SyncMode::Copy
    };
    (path.to_string_lossy().into_owned(), mode)
}

/// A failed sync settles its row here (D9, symmetric with ADR-0002): the
/// folder's row is recorded `error` with the diagnostic chain — a new row owns
/// nothing (`synced_at` NULL), an existing one keeps its ownership. A store
/// failure here is logged: the report row already carries the sync failure,
/// which is what the operator acts on.
fn record_failed(
    store: &SkillStore,
    target: &PlannedToolTarget,
    skill: &BatchSkill,
    error: &GlobalSyncError,
) {
    let (target_path, mode) = would_be(target, skill);
    let detail = error.to_string();
    if let Err(err) = store.transition_skill_target(
        &Uuid::new_v4().to_string(),
        TargetTransition::FailedRecorded {
            skill_id: &skill.skill_id,
            tool: &target.folder_key,
            mode,
            target_path: &target_path,
            error: &detail,
        },
    ) {
        log::warn!(
            "recording the failed sync of {} to {} failed: {err:#}",
            skill.skill_id,
            target.folder_key
        );
    }
}

/// The operator kept the folder in the way (`OverrideDecision::Keep`): the
/// folder's row is recorded `disabled` — a marker that owns nothing at its
/// path — and reported `Declined`. A store failure fails the pair as report
/// data.
fn record_kept(
    store: &SkillStore,
    target: &PlannedToolTarget,
    skill: &BatchSkill,
) -> BatchTargetStatus {
    let (target_path, mode) = would_be(target, skill);
    let tool_key = target.folder_key.as_str();
    match store.transition_skill_target(
        &Uuid::new_v4().to_string(),
        TargetTransition::DisabledRecorded {
            skill_id: &skill.skill_id,
            tool: tool_key,
            mode,
            target_path: &target_path,
        },
    ) {
        Ok(()) => BatchTargetStatus::Declined,
        Err(err) => BatchTargetStatus::Failed {
            error: CommandError::internal(format!("{err:#}")),
        },
    }
}

/// The Disabled rule of the batch (see **Disabled** in `CONTEXT.md`): a pair
/// is skipped when its skill is disabled at skill grain, or when the target's
/// folder is off (`placement::FolderRows::disabled`): a row of the skill on
/// the folder driver's recorded path is `Disabled` — the folder's own row, or
/// a row a shared-dir sibling wrote on that same path before rows were per
/// folder. An independent row (another recorded path) never blocks it.
struct DisabledGate {
    skill_disabled: bool,
    disabled_folders: Vec<String>,
}

impl DisabledGate {
    fn read(store: &SkillStore, skill_id: &str) -> Result<Self> {
        Ok(Self {
            skill_disabled: store.skill_disabled_at(skill_id)?.is_some(),
            disabled_folders: placement::folder_rows(
                ScopeKind::Global,
                store.list_skill_targets(skill_id)?,
            )
            .into_iter()
            .filter(|group| group.disabled())
            .map(|group| group.folder)
            .collect(),
        })
    }

    fn blocks(&self, folder_key: &str, lifted: &[String]) -> bool {
        self.skill_disabled
            || (self.disabled_folders.iter().any(|f| f == folder_key)
                && !lifted.iter().any(|f| f == folder_key))
    }
}

/// Sync N skills to M tools in one call: environment probing under `roots`
/// ([`plan_batch_tool_targets`]) composed with the deterministic engine
/// ([`sync_planned_tools_lifting`]). Planning failures surface as `Failed`
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
    sync_skills_to_tools_lifting_unlocked(
        roots,
        store,
        central_dir,
        skills,
        tool_keys,
        policy,
        &[],
        now,
        on_progress,
    )
}

/// [`sync_skills_to_tools_unlocked`] for Enable, lifting the Disabled state of
/// the folders `lifted` names ([`sync_planned_tools_lifting`]).
#[allow(clippy::too_many_arguments)]
pub(crate) fn sync_skills_to_tools_lifting_unlocked(
    roots: &HostRoots,
    store: &SkillStore,
    central_dir: &Path,
    skills: &[BatchSkill],
    tool_keys: &[String],
    policy: &BatchPolicy,
    lifted: &[String],
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
                        tool_key: tool_key.clone(),
                        status: BatchTargetStatus::Failed {
                            error: error.clone(),
                        },
                    });
                }
            }
        }
    }
    outcomes.extend(sync_planned_tools_lifting(
        store,
        central_dir,
        skills,
        &targets,
        policy,
        lifted,
        now,
        on_progress,
    ));
    outcomes
}

#[cfg(test)]
#[path = "tests/global_sync.rs"]
mod tests;
