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
        adapters_sharing_skills_dir, is_tool_installed, resolve_default_path, ToolAdapter,
    },
};

/// Typed failures the frontend reacts to specially. The command layer maps
/// these to its wire prefixes (`TOOL_NOT_INSTALLED|…`, `TARGET_EXISTS|…`,
/// `TOOL_NOT_WRITABLE|…`); core tests assert on the variants, not strings.
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

/// Sync a skill into a tool's global skills directory and record the result
/// for every installed tool sharing that directory.
///
/// Environment probing (is the tool installed, where is its skills dir, which
/// installed tools share it) happens here; the deterministic work is in
/// [`sync_skill_into_root`], which tests drive directly.
pub fn sync_skill_to_tool_with_records(
    store: &SkillStore,
    adapter: &ToolAdapter,
    source: &Path,
    skill_id: &str,
    skill_name: &str,
    policy: &OverwritePolicy,
    now: i64,
) -> Result<SyncOutcome, GlobalSyncError> {
    if !is_tool_installed(adapter)? {
        return Err(GlobalSyncError::ToolNotInstalled {
            tool_key: adapter.id.as_key().to_string(),
        });
    }
    let tool_root = resolve_default_path(adapter)?;
    let mut record_tools: Vec<ToolAdapter> = Vec::new();
    for a in adapters_sharing_skills_dir(adapter) {
        if is_tool_installed(&a)? {
            record_tools.push(a);
        }
    }
    sync_skill_into_root(
        store,
        adapter,
        &tool_root,
        source,
        skill_id,
        skill_name,
        policy,
        &record_tools,
        now,
    )
}

/// Deterministic half of [`sync_skill_to_tool_with_records`]: probe
/// writability of `tool_root`, apply the overwrite policy, sync, and upsert a
/// `SkillTargetRecord` for each tool in `record_tools`.
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
    let msg = format!("{:#}", err);
    if msg.contains("target already exists") {
        GlobalSyncError::TargetExists {
            target_path: target.to_path_buf(),
        }
    } else if msg.contains("os error 5")
        || msg.contains("Access is denied")
        || msg.contains("Permission denied")
    {
        GlobalSyncError::ToolNotWritable {
            tool_display_name: adapter.display_name.to_string(),
            skills_dir: tool_root.to_path_buf(),
        }
    } else {
        GlobalSyncError::Other(err)
    }
}

/// Remove a skill's sync target for a tool, updating every tool that shares
/// the same global skills directory. Environment probing lives here; the
/// deterministic removal is in [`remove_targets_for_tools`].
pub fn unsync_skill_from_tool_with_records(
    store: &SkillStore,
    tool_key: &str,
    skill_id: &str,
) -> Result<()> {
    let group_tool_keys: Vec<String> =
        if let Some(adapter) = crate::core::tool_adapters::adapter_by_key(tool_key) {
            let group = adapters_sharing_skills_dir(&adapter);
            // If none of the group tools are installed, do nothing (treat as already not effective).
            let mut any_installed = false;
            for a in &group {
                if is_tool_installed(a)? {
                    any_installed = true;
                    break;
                }
            }
            if !any_installed {
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

#[cfg(test)]
#[path = "tests/global_sync.rs"]
mod tests;
