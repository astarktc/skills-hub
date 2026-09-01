//! Global-tool sync orchestration: overwrite policy, writability probing, and
//! DB record fan-out layered on top of `sync_engine`. The global-skills
//! counterpart of `project_sync.rs`.

use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::Result;
use uuid::Uuid;

use crate::core::{
    content_hash,
    skill_store::{SkillStore, SkillTargetRecord},
    sync_engine::{self, SyncOutcome},
    tool_adapters::{
        adapter_by_key, adapters_sharing_skills_dir, is_installed_in, skills_dir_in, ToolAdapter,
    },
};

/// Typed failures the frontend reacts to specially. The command layer maps
/// these onto `commands::error::CommandError` wire variants; core tests
/// assert on the variants, not strings.
#[derive(Debug)]
pub enum GlobalSyncError {
    ToolNotInstalled {
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

/// True when `target` exists and hashes identically to `source`.
pub fn target_has_same_content(source: &Path, target: &Path) -> bool {
    if !target.exists() {
        return false;
    }
    match (
        content_hash::hash_dir(source),
        content_hash::hash_dir(target),
    ) {
        (Ok(s), Ok(t)) => s == t,
        _ => false,
    }
}

/// Deterministic single-pair sync: probe writability of `tool_root`, apply
/// the overwrite policy, sync, and upsert a `SkillTargetRecord` for each
/// tool in `record_tools`. The batch engine
/// ([`sync_skills_to_planned_tools`]) drives this per attempted pair; tests
/// drive it directly.
#[allow(clippy::too_many_arguments)]
pub fn sync_skill_into_root(
    store: &SkillStore,
    adapter: &ToolAdapter,
    tool_root: &Path,
    source: &Path,
    skill_id: &str,
    skill_name: &str,
    policy: &OverwritePolicy,
    record_tools: &[ToolAdapter],
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
        || (policy.overwrite_if_same_content && target_has_same_content(source, &target));

    let outcome = sync_engine::sync_dir_for_tool_with_overwrite(
        adapter.id.as_key(),
        source,
        &target,
        overwrite,
    )
    .map_err(|err| classify_sync_error(err, adapter, tool_root, &target))?;

    // Some tools share the same global skills directory; keep DB records consistent across them.
    for a in record_tools {
        let record = SkillTargetRecord {
            id: Uuid::new_v4().to_string(),
            skill_id: skill_id.to_string(),
            tool: a.id.as_key().to_string(),
            target_path: outcome.target_path.to_string_lossy().to_string(),
            mode: outcome.mode_used.as_str().to_string(),
            status: "ok".to_string(),
            last_error: None,
            synced_at: Some(now),
        };
        store.upsert_skill_target(&record)?;
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

/// Remove a skill's sync target for a tool, updating every tool that shares
/// the same global skills directory. Environment probing (installedness
/// under `home`) lives here; the deterministic removal is in
/// [`remove_targets_for_tools`].
pub fn unsync_skill_from_tool_with_records(
    home: &Path,
    store: &SkillStore,
    tool_key: &str,
    skill_id: &str,
) -> Result<()> {
    let group_tool_keys: Vec<String> =
        if let Some(adapter) = crate::core::tool_adapters::adapter_by_key(tool_key) {
            let group = adapters_sharing_skills_dir(&adapter);
            // If none of the group tools are installed, do nothing (treat as already not effective).
            if !group.iter().any(|a| is_installed_in(home, a)) {
                return Ok(());
            }
            group
                .into_iter()
                .map(|a| a.id.as_key().to_string())
                .collect()
        } else {
            vec![tool_key.to_string()]
        };

    remove_targets_for_tools(store, skill_id, &group_tool_keys)
}

/// Remove the filesystem target once (shared dir ⇒ shared target path) and
/// delete the DB record for each tool in `tool_keys`.
pub fn remove_targets_for_tools(
    store: &SkillStore,
    skill_id: &str,
    tool_keys: &[String],
) -> Result<()> {
    let mut removed = false;
    for k in tool_keys {
        if let Some(target) = store.get_skill_target(skill_id, k)? {
            if !removed {
                sync_engine::remove_path_any(Path::new(&target.target_path))?;
                removed = true;
            }
            store.delete_skill_target(skill_id, k)?;
        }
    }
    Ok(())
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

/// Force-overwrite for one (skill, tool) pair. Applies to any target tool
/// sharing the named tool's skills dir (the dir, not the key, is the real
/// identity of a sync target).
#[derive(Debug, Clone)]
pub struct BatchOverride {
    pub skill_id: String,
    pub tool_key: String,
    pub overwrite: bool,
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
    pub adapter: ToolAdapter,
    pub root: PathBuf,
    pub installed: bool,
    pub record_tools: Vec<ToolAdapter>,
}

/// Per-(skill, tool) result. `Skipped` is the expected-and-ignorable class
/// (tool absent, dir unwritable) — callers decide whether to surface it;
/// `Failed` is everything else. Both carry the typed error, so no
/// information is lost by the classification.
#[derive(Debug)]
pub enum BatchTargetStatus {
    Synced { outcome: SyncOutcome },
    Skipped { error: GlobalSyncError },
    Failed { error: GlobalSyncError },
}

#[derive(Debug)]
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
            let installed = is_installed_in(home, &adapter);
            let root = skills_dir_in(home, &adapter);
            let record_tools: Vec<ToolAdapter> = adapters_sharing_skills_dir(&adapter)
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
/// pair never aborts the batch.
pub fn sync_skills_to_planned_tools(
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
                    error: GlobalSyncError::ToolNotInstalled {
                        tool_key: tool_key.to_string(),
                    },
                },
            });
        }
    }

    let total = skills.len() * attempted.len();
    let mut index = 0usize;
    for skill in skills {
        for target in &attempted {
            index += 1;
            let tool_key = target.adapter.id.as_key();
            on_progress(BatchProgress {
                index,
                total,
                skill_name: &skill.skill_name,
                tool_key,
            });

            let overwrite = policy.overwrite
                || policy.overrides.iter().any(|o| {
                    o.overwrite
                        && o.skill_id == skill.skill_id
                        && adapter_by_key(&o.tool_key).is_some_and(|oa| {
                            oa.relative_skills_dir == target.adapter.relative_skills_dir
                        })
                });
            let pair_policy = OverwritePolicy {
                overwrite,
                overwrite_if_same_content: policy.overwrite_if_same_content,
            };

            let status = match sync_skill_into_root(
                store,
                &target.adapter,
                &target.root,
                &skill.source_path,
                &skill.skill_id,
                &skill.skill_name,
                &pair_policy,
                &target.record_tools,
                now,
            ) {
                Ok(outcome) => BatchTargetStatus::Synced { outcome },
                Err(error @ GlobalSyncError::ToolNotWritable { .. }) => {
                    BatchTargetStatus::Skipped { error }
                }
                Err(error) => BatchTargetStatus::Failed { error },
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

/// Sync N skills to M tools in one call: environment probing under `home`
/// ([`plan_batch_tool_targets`]) composed with the deterministic engine
/// ([`sync_skills_to_planned_tools`]). Planning failures surface as `Failed`
/// outcomes per skill; the function itself never errors.
pub fn sync_skills_to_tools(
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
                let msg = format!("{}", error);
                for skill in skills {
                    outcomes.push(BatchTargetOutcome {
                        skill_id: skill.skill_id.clone(),
                        skill_name: skill.skill_name.clone(),
                        tool_key: tool_key.clone(),
                        status: BatchTargetStatus::Failed {
                            error: GlobalSyncError::Other(anyhow::anyhow!("{}", msg)),
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
