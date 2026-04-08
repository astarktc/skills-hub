use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::core::{
    content_hash,
    skill_store::{ProjectRecord, ProjectSkillAssignmentRecord, SkillRecord, SkillStore},
    sync_engine::{self, SyncMode},
    tool_adapters,
};

pub fn resolve_project_sync_target(
    project_path: &Path,
    relative_skills_dir: &str,
    skill_name: &str,
) -> PathBuf {
    project_path.join(relative_skills_dir).join(skill_name)
}

pub fn sync_mode_to_str(mode: &SyncMode) -> &'static str {
    match mode {
        SyncMode::Auto => "auto",
        SyncMode::Symlink => "symlink",
        SyncMode::Junction => "junction",
        SyncMode::Copy => "copy",
    }
}

pub fn assign_and_sync(
    store: &SkillStore,
    project: &ProjectRecord,
    skill: &SkillRecord,
    tool_key: &str,
    now: i64,
) -> Result<ProjectSkillAssignmentRecord> {
    let adapter = tool_adapters::adapter_by_key(tool_key)
        .ok_or_else(|| anyhow::anyhow!("unknown tool: {}", tool_key))?;

    let record = ProjectSkillAssignmentRecord {
        id: uuid::Uuid::new_v4().to_string(),
        project_id: project.id.clone(),
        skill_id: skill.id.clone(),
        tool: tool_key.to_string(),
        mode: "symlink".to_string(),
        status: "pending".to_string(),
        last_error: None,
        synced_at: None,
        content_hash: None,
        created_at: now,
    };
    store.add_project_skill_assignment(&record)?;

    let source = Path::new(&skill.central_path);
    let target = resolve_project_sync_target(
        Path::new(&project.path),
        adapter.relative_skills_dir,
        &skill.name,
    );

    match sync_engine::sync_dir_for_tool_with_overwrite(tool_key, source, &target, false) {
        Ok(outcome) => {
            let mode_str = sync_mode_to_str(&outcome.mode_used);
            let hash = if matches!(outcome.mode_used, SyncMode::Copy) {
                Some(
                    content_hash::hash_dir(source)
                        .context("computing content hash for copy-mode target")?,
                )
            } else {
                None
            };
            store.update_assignment_status(
                &record.id,
                "synced",
                None,
                Some(now),
                Some(mode_str),
                hash.as_deref(),
            )?;
            let updated = store
                .get_project_skill_assignment(&project.id, &skill.id, tool_key)?
                .unwrap_or(record);
            Ok(updated)
        }
        Err(e) => {
            let err_msg = format!("{:#}", e);
            store.update_assignment_status(
                &record.id,
                "error",
                Some(&err_msg),
                None,
                None,
                None,
            )?;
            let updated = store
                .get_project_skill_assignment(&project.id, &skill.id, tool_key)?
                .unwrap_or(record);
            Ok(updated)
        }
    }
}

pub fn unassign_and_cleanup(
    store: &SkillStore,
    project: &ProjectRecord,
    skill: &SkillRecord,
    tool_key: &str,
) -> Result<()> {
    let adapter = tool_adapters::adapter_by_key(tool_key)
        .ok_or_else(|| anyhow::anyhow!("unknown tool: {}", tool_key))?;

    let target = resolve_project_sync_target(
        Path::new(&project.path),
        adapter.relative_skills_dir,
        &skill.name,
    );

    if target.exists() || target.symlink_metadata().is_ok() {
        match sync_engine::remove_path_any(&target) {
            Ok(()) => {
                store.remove_project_skill_assignment(&project.id, &skill.id, tool_key)?;
                Ok(())
            }
            Err(e) => {
                // Filesystem removal failed -- keep record with error status
                if let Some(assignment) =
                    store.get_project_skill_assignment(&project.id, &skill.id, tool_key)?
                {
                    let _ = store.update_assignment_status(
                        &assignment.id,
                        "error",
                        Some(&format!("{:#}", e)),
                        None,
                        None,
                        None,
                    );
                }
                Err(e)
            }
        }
    } else {
        // Target doesn't exist -- just clean up the DB record
        store.remove_project_skill_assignment(&project.id, &skill.id, tool_key)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/project_sync.rs"]
mod tests;
