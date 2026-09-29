//! Global-tool sync orchestration: overwrite policy, writability probing, and
//! DB record fan-out layered on top of `sync_engine`. The global-skills
//! counterpart of `project_sync.rs`.

use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::Result;
use uuid::Uuid;

use crate::core::{
    content_identity::{self, Source},
    errors::CommandError,
    mutation_guard,
    skill_store::{SkillStore, TargetTransition},
    sync_engine::{self, SyncOutcome},
    sync_status::{SyncMode, SyncStatus},
    tool_adapters::{
        adapter_by_key, adapters_sharing_skills_dir, is_installed_in, skills_dir_in, ToolAdapter,
    },
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
            GlobalSyncError::TargetExists { target_path } => {
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
/// the overwrite policy, sync, and record a `Synced` target row for each
/// tool in `record_tools` through [`TargetTransition::Recorded`]. The batch engine
/// ([`sync_skills_to_planned_tools`]) drives this per attempted pair; tests
/// drive it directly. Unlocked internal seam — callers reach it through an
/// entry point that has already taken the mutation guard.
#[allow(clippy::too_many_arguments)]
pub(crate) fn sync_skill_into_root(
    store: &SkillStore,
    adapter: &ToolAdapter,
    tool_root: &Path,
    source: &Path,
    skill_id: &str,
    skill_name: &str,
    policy: &OverwritePolicy,
    record_tools: &[&'static ToolAdapter],
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

    let outcome =
        sync_engine::sync_dir_for_tool_with_overwrite(adapter, source, &target, overwrite)
            .map_err(|err| classify_sync_error(err, adapter, tool_root, &target))?;

    // Some tools share the same global skills directory; keep DB records consistent across them.
    let target_path = outcome.target_path.to_string_lossy();
    for a in record_tools {
        store.transition_skill_target(
            &Uuid::new_v4().to_string(),
            TargetTransition::Recorded {
                skill_id,
                tool: a.id.as_key(),
                mode: outcome.mode_used,
                target_path: &target_path,
                synced_at: now,
            },
        )?;
    }

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
    if err.chain().any(|cause| {
        cause
            .downcast_ref::<sync_engine::TargetExistsError>()
            .is_some()
    }) {
        return GlobalSyncError::TargetExists {
            target_path: target.to_path_buf(),
        };
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
// command. Skills × tools in one call; shared-dir dedupe, installedness
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

/// The operator's decision for one (skill, tool) pair. Applies to any target
/// tool sharing the named tool's skills dir (the dir, not the key, is the
/// real identity of a sync target).
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

/// A probed tool target: adapter, resolved skills root, installedness, and
/// the installed tools sharing that root (DB record fan-out). Produced by
/// [`plan_batch_tool_targets`] (environment probing); consumed by the
/// deterministic [`sync_skills_to_planned_tools`], which tests drive with
/// fabricated roots.
#[derive(Debug)]
pub struct PlannedToolTarget {
    pub adapter: &'static ToolAdapter,
    pub root: PathBuf,
    pub installed: bool,
    pub record_tools: Vec<&'static ToolAdapter>,
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

/// Probe the environment under `home` for each requested tool key: resolve
/// the adapter, its skills root, installedness, and the installed shared-dir
/// group. Unknown keys become planning entries via `Err`, which the batch
/// turns into `Failed` outcomes.
pub fn plan_batch_tool_targets(
    home: &Path,
    tool_keys: &[String],
) -> Vec<Result<PlannedToolTarget, (String, GlobalSyncError)>> {
    tool_keys
        .iter()
        .map(|key| {
            let adapter = adapter_by_key(key).ok_or_else(|| {
                (
                    key.clone(),
                    GlobalSyncError::Other(anyhow::anyhow!("unknown tool: {}", key)),
                )
            })?;
            let installed = is_installed_in(home, adapter);
            let root = skills_dir_in(home, adapter);
            let record_tools: Vec<&'static ToolAdapter> = adapters_sharing_skills_dir(adapter)
                .into_iter()
                .filter(|a| is_installed_in(home, a))
                .collect();
            Ok(PlannedToolTarget {
                adapter,
                root,
                installed,
                record_tools,
            })
        })
        .collect()
}

/// Deterministic batch engine: dedupe installed targets by skills root
/// (first in caller order wins; shared-dir tools are covered via each
/// target's `record_tools`), emit `Skipped` for not-installed tools, apply
/// the per-pair overwrite policy, and isolate failures per target — one bad
/// pair never aborts the batch. Unlocked internal seam — callers reach it
/// through an entry point that has already taken the mutation guard.
pub(crate) fn sync_skills_to_planned_tools(
    store: &SkillStore,
    skills: &[BatchSkill],
    targets: &[PlannedToolTarget],
    policy: &BatchPolicy,
    now: i64,
    mut on_progress: impl FnMut(BatchProgress),
) -> Vec<BatchTargetOutcome> {
    let mut outcomes: Vec<BatchTargetOutcome> = Vec::new();

    // Partition: attempted = installed, deduped by root; the rest skip.
    let mut seen_roots: Vec<&Path> = Vec::new();
    let mut attempted: Vec<&PlannedToolTarget> = Vec::new();
    let mut skipped_not_installed: Vec<&PlannedToolTarget> = Vec::new();
    for target in targets {
        if !target.installed {
            skipped_not_installed.push(target);
            continue;
        }
        if seen_roots.contains(&target.root.as_path()) {
            continue; // covered by an earlier target's record_tools fan-out
        }
        seen_roots.push(target.root.as_path());
        attempted.push(target);
    }

    for target in &skipped_not_installed {
        let tool_key = target.adapter.id.as_key();
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
            let tool_key = target.adapter.id.as_key();
            on_progress(BatchProgress {
                index,
                total,
                skill_name: &skill.skill_name,
                tool_key,
            });

            let blocked = match &gate {
                Ok(gate) => gate
                    .blocks(target.adapter)
                    .then(|| BatchTargetStatus::Skipped {
                        error: CommandError::from(GlobalSyncError::TargetDisabled {
                            tool_key: tool_key.to_string(),
                        }),
                    }),
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

            // An override names a tool key; it applies to this target when
            // that tool shares the target's skills dir (the registry's answer).
            let decision = |wanted: OverrideDecision| {
                policy.overrides.iter().any(|o| {
                    o.decision == wanted
                        && o.skill_id == skill.skill_id
                        && adapters_sharing_skills_dir(target.adapter)
                            .iter()
                            .any(|a| a.key() == o.tool_key)
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
                target.adapter,
                &target.root,
                &skill.source_path,
                &skill.skill_id,
                &skill.skill_name,
                &pair_policy,
                &target.record_tools,
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

/// A failed sync settles its rows here (D9, symmetric with ADR-0002): every
/// Tool of the target's shared-dir group is recorded `error` with the
/// diagnostic chain — a new row owns nothing (`synced_at` NULL), an existing
/// one keeps its ownership. A store failure here is logged: the report row
/// already carries the sync failure, which is what the operator acts on.
fn record_failed(
    store: &SkillStore,
    target: &PlannedToolTarget,
    skill: &BatchSkill,
    error: &GlobalSyncError,
) {
    let (target_path, mode) = would_be(target, skill);
    let detail = error.to_string();
    for adapter in &target.record_tools {
        if let Err(err) = store.transition_skill_target(
            &Uuid::new_v4().to_string(),
            TargetTransition::FailedRecorded {
                skill_id: &skill.skill_id,
                tool: adapter.key(),
                mode,
                target_path: &target_path,
                error: &detail,
            },
        ) {
            log::warn!(
                "recording the failed sync of {} to {} failed: {err:#}",
                skill.skill_id,
                adapter.key()
            );
        }
    }
}

/// The operator kept the folder in the way (`OverrideDecision::Keep`): the
/// pair is recorded `disabled` for the attempted Tool — a marker that owns
/// nothing at its path, and that the Disabled rule extends to the Tool's
/// shared-dir siblings — and reported `Declined`. A store failure fails the
/// pair as report data.
fn record_kept(
    store: &SkillStore,
    target: &PlannedToolTarget,
    skill: &BatchSkill,
) -> BatchTargetStatus {
    let (target_path, mode) = would_be(target, skill);
    let tool_key = target.adapter.key();
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
/// is skipped when its skill is disabled at skill grain, or when **any** row
/// of the skill describing the target's artifact is `Disabled` — the
/// target's own Tool or any Tool sharing its skills dir, because a shared
/// artifact written for a sibling is the disabled Tool's artifact too.
struct DisabledGate {
    skill_disabled: bool,
    disabled_tools: Vec<String>,
}

impl DisabledGate {
    fn read(store: &SkillStore, skill_id: &str) -> Result<Self> {
        Ok(Self {
            skill_disabled: store.skill_disabled_at(skill_id)?.is_some(),
            disabled_tools: store
                .list_skill_targets(skill_id)?
                .into_iter()
                .filter(|row| row.status == SyncStatus::Disabled)
                .map(|row| row.tool)
                .collect(),
        })
    }

    fn blocks(&self, adapter: &ToolAdapter) -> bool {
        self.skill_disabled
            || self.disabled_tools.iter().any(|tool| {
                tool == adapter.key()
                    || adapters_sharing_skills_dir(adapter)
                        .iter()
                        .any(|a| a.key() == tool)
            })
    }
}

/// Sync N skills to M tools in one call: environment probing under `home`
/// ([`plan_batch_tool_targets`]) composed with the deterministic engine
/// ([`sync_skills_to_planned_tools`]). Planning failures surface as `Failed`
/// outcomes per skill; the function itself never errors.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
pub fn sync_skills_to_tools(
    home: &Path,
    store: &SkillStore,
    skills: &[BatchSkill],
    tool_keys: &[String],
    policy: &BatchPolicy,
    now: i64,
    on_progress: impl FnMut(BatchProgress),
) -> Vec<BatchTargetOutcome> {
    mutation_guard::serialized(|| {
        sync_skills_to_tools_unlocked(home, store, skills, tool_keys, policy, now, on_progress)
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn sync_skills_to_tools_unlocked(
    home: &Path,
    store: &SkillStore,
    skills: &[BatchSkill],
    tool_keys: &[String],
    policy: &BatchPolicy,
    now: i64,
    on_progress: impl FnMut(BatchProgress),
) -> Vec<BatchTargetOutcome> {
    let mut targets: Vec<PlannedToolTarget> = Vec::new();
    let mut outcomes: Vec<BatchTargetOutcome> = Vec::new();
    for plan in plan_batch_tool_targets(home, tool_keys) {
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
    outcomes.extend(sync_skills_to_planned_tools(
        store,
        skills,
        &targets,
        policy,
        now,
        on_progress,
    ));
    outcomes
}

#[cfg(test)]
#[path = "tests/global_sync.rs"]
mod tests;
