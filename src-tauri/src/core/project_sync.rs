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
        .ok_or_else(|| anyhow::anyhow!("skill not found: {}", assignment.skill_id))?;
    let adapter = tool_adapters::adapter_by_key(&assignment.tool)
        .ok_or_else(|| anyhow::anyhow!("unknown tool: {}", assignment.tool))?;

    let source = Path::new(&skill.central_path);
    let target = resolve_project_sync_target(
        Path::new(&project.path),
        adapter.relative_skills_dir,
        &skill.name,
    );

    let outcome = sync_engine::sync_dir_for_tool_with_overwrite(
        &assignment.tool,
        source,
        &target,
        overwrite,
    )?;

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
        &assignment.id,
        "synced",
        None,
        Some(now),
        Some(mode_str),
        hash.as_deref(),
    )?;

    Ok(())
}

pub fn resync_project(store: &SkillStore, project_id: &str, now: i64) -> Result<ResyncSummary> {
    let project = store
        .get_project_by_id(project_id)?
        .ok_or_else(|| anyhow::anyhow!("project not found: {}", project_id))?;
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
                let _ = store.update_assignment_status(
                    &assignment.id,
                    "error",
                    Some(&format!("{:#}", e)),
                    None,
                    None,
                    None,
                );
                summary.failed += 1;
                summary.errors.push(err_msg);
            }
        }
    }

    Ok(summary)
}

pub fn resync_all_projects(store: &SkillStore, now: i64) -> Result<Vec<ResyncSummary>> {
    let projects = store.list_projects()?;
    let mut summaries = Vec::with_capacity(projects.len());

    for project in &projects {
        let summary = resync_project(store, &project.id, now)?;
        summaries.push(summary);
    }

    Ok(summaries)
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
