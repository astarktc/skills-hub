//! The Managed-skill catalog: the presentation-ready list of Managed skills
//! with their Sync targets and manifest-derived invocation mode already
//! resolved. The skills-side counterpart of the Tool catalog
//! (`tool_adapters::global_tool_entries`): core assembles it, the command
//! tier only maps it to DTOs.
//!
//! Failure policy: a target query that fails **fails the catalog call**. A
//! listing that silently drops a skill's Sync targets would tell the operator
//! the skill is synced nowhere and invite a second sync over a live artifact;
//! an error the UI can show is strictly better than a wrong list. Only the
//! invocation mode degrades quietly, because it is derived from the central
//! copy's `SKILL.md` at list time and a missing or unreadable manifest is an
//! ordinary state, not a failure (`InvocationMode::default()`).

use anyhow::{Context, Result};
use std::path::Path;

use crate::core::{
    skill_discovery::{invocation_mode_for_dir, InvocationMode},
    skill_store::{SkillRecord, SkillStore, SkillTargetRecord},
};

/// One Managed skill as the library list needs it.
#[derive(Debug, Clone)]
pub struct ManagedSkillEntry {
    pub skill: SkillRecord,
    /// Who may invoke the skill, read from the central copy's `SKILL.md`
    /// frontmatter at list time (not persisted).
    pub invocation_mode: InvocationMode,
    /// Every global Sync target row of this skill, in store order.
    pub targets: Vec<SkillTargetRecord>,
}

/// Assemble the Managed-skill catalog: every Managed skill with its Sync
/// targets and invocation mode.
pub fn managed_skill_catalog(store: &SkillStore) -> Result<Vec<ManagedSkillEntry>> {
    let skills = store.list_skills().context("list managed skills")?;
    let mut entries = Vec::with_capacity(skills.len());
    for skill in skills {
        let targets = store
            .list_skill_targets(&skill.id)
            .with_context(|| format!("list sync targets for skill {}", skill.id))?;
        let invocation_mode = invocation_mode_for_dir(Path::new(&skill.central_path));
        entries.push(ManagedSkillEntry {
            skill,
            invocation_mode,
            targets,
        });
    }
    Ok(entries)
}

#[cfg(test)]
#[path = "tests/skill_catalog.rs"]
mod tests;
