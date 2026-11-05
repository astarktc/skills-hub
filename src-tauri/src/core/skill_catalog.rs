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
use std::collections::HashMap;
use std::path::Path;

use crate::core::{
    manifest::{invocation_mode_for_dir, InvocationMode},
    project_ops::project_name_from_path,
    provenance::is_refreshable,
    skill_health::{all_skill_health, skill_health, SkillHealth},
    skill_store::{SkillProjectDeploymentRecord, SkillRecord, SkillStore, SkillTargetRecord},
    sync_status::SyncStatus,
    unlocatable::{is_detachable, unlocatable_state, UnlocatableState},
};

/// One project a Managed skill is deployed to, as the library row needs it:
/// the project's identity and name, and the skill's assignment rows there.
/// Counts (how many projects hold a live deployment, how many failed) are the
/// frontend fold's, never carried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillProject {
    pub project_id: String,
    /// The project's display name (its folder's basename, `project_name_from_path`).
    pub project_name: String,
    /// One per assignment row of this skill in the project, by Tool key.
    pub deployments: Vec<SkillProjectDeployment>,
}

/// One project assignment row's Tool and lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillProjectDeployment {
    pub tool: String,
    pub status: SyncStatus,
    /// Set once a sync of ours succeeded there (ownership): a never-synced
    /// `error` row owns nothing and is not a live deployment.
    pub synced_at: Option<i64>,
}

/// Group the flat deployment rows (one query) into each skill's projects,
/// ordered by project name, then id.
fn projects_by_skill(
    rows: Vec<SkillProjectDeploymentRecord>,
) -> HashMap<String, Vec<SkillProject>> {
    let mut out: HashMap<String, Vec<SkillProject>> = HashMap::new();
    for row in rows {
        let projects = out.entry(row.skill_id).or_default();
        let deployment = SkillProjectDeployment {
            tool: row.tool,
            status: row.status,
            synced_at: row.synced_at,
        };
        match projects.iter_mut().find(|p| p.project_id == row.project_id) {
            Some(project) => project.deployments.push(deployment),
            None => projects.push(SkillProject {
                project_name: project_name_from_path(&row.project_path),
                project_id: row.project_id,
                deployments: vec![deployment],
            }),
        }
    }
    for projects in out.values_mut() {
        projects.sort_by(|a, b| {
            a.project_name
                .to_lowercase()
                .cmp(&b.project_name.to_lowercase())
                .then_with(|| a.project_id.cmp(&b.project_id))
        });
    }
    out
}

/// One Managed skill as the library list needs it.
#[derive(Debug, Clone)]
pub struct ManagedSkillEntry {
    pub skill: SkillRecord,
    /// Who may invoke the skill, read from the central copy's `SKILL.md`
    /// frontmatter at list time (not persisted).
    pub invocation_mode: InvocationMode,
    pub invocation_override: Option<super::skill_edits::InvocationOverride>,
    /// Every global Sync target row of this skill, in store order —
    /// `disabled` rows included: they are the operator's record.
    pub targets: Vec<SkillTargetRecord>,
    /// When the skill was disabled at skill grain, `None` when it is not
    /// (`SkillStore::skill_disabled_at`; see **Disabled** in `CONTEXT.md`).
    pub disabled_at: Option<i64>,
    /// Whether Refresh / Update can re-acquire this skill — the Provenance
    /// rule (`provenance::is_refreshable`), answered here so the UI shows or
    /// hides Update without re-deriving it.
    pub refreshable: bool,
    /// Whether the app can still locate this skill's recorded paths, and
    /// which is gone when not (`unlocatable::unlocatable_state`) — computed
    /// here at list time, never stored.
    pub unlocatable: Option<UnlocatableState>,
    /// Whether Detach would be accepted for this skill (`unlocatable::is_detachable`):
    /// the card offers it, for a `source_missing` skill, exactly when so.
    pub detachable: bool,
    /// The skill's stored last outcome (`core::skill_health`), `None` when
    /// healthy. Distinct from each target's own status and `last_error`.
    pub health: Option<SkillHealth>,
    /// Every project the skill has an assignment row in, with those rows
    /// (`disabled` included), read for the whole catalog in one query.
    pub projects: Vec<SkillProject>,
}

/// Assemble the Managed-skill catalog: every Managed skill with its Sync
/// targets and invocation mode.
pub fn managed_skill_catalog(store: &SkillStore) -> Result<Vec<ManagedSkillEntry>> {
    let skills = store.list_skills().context("list managed skills")?;
    let disabled = store.disabled_skills().context("list disabled skills")?;
    let mut health = all_skill_health(store).context("list skill health")?;
    let mut projects = projects_by_skill(
        store
            .list_skill_project_deployments(None)
            .context("list project deployments")?,
    );
    skills
        .into_iter()
        .map(|skill| {
            let disabled_at = disabled.get(&skill.id).copied();
            let health = health.remove(&skill.id);
            let projects = projects.remove(&skill.id).unwrap_or_default();
            build_entry(store, skill, disabled_at, health, projects)
        })
        .collect()
}

/// Resolve only the requested skill; Save must not scan the library under the guard.
pub fn managed_skill_entry(store: &SkillStore, id: &str) -> Result<Option<ManagedSkillEntry>> {
    store
        .get_skill_by_id(id)?
        .map(|skill| {
            let disabled_at = store.skill_disabled_at(&skill.id)?;
            let health = skill_health(store, &skill.id)?;
            let projects = projects_by_skill(
                store
                    .list_skill_project_deployments(Some(&skill.id))
                    .context("list project deployments")?,
            )
            .remove(&skill.id)
            .unwrap_or_default();
            build_entry(store, skill, disabled_at, health, projects)
        })
        .transpose()
}

fn build_entry(
    store: &SkillStore,
    skill: SkillRecord,
    disabled_at: Option<i64>,
    health: Option<SkillHealth>,
    projects: Vec<SkillProject>,
) -> Result<ManagedSkillEntry> {
    let targets = store
        .list_skill_targets(&skill.id)
        .with_context(|| format!("list sync targets for skill {}", skill.id))?;
    let invocation_mode = invocation_mode_for_dir(Path::new(&skill.central_path));
    let invocation_override = super::skill_edits::invocation_override(store, &skill.id)?;
    let refreshable = is_refreshable(&skill);
    let unlocatable = unlocatable_state(&skill);
    let detachable = is_detachable(&skill);
    Ok(ManagedSkillEntry {
        skill,
        invocation_mode,
        invocation_override,
        targets,
        disabled_at,
        refreshable,
        unlocatable,
        detachable,
        health,
        projects,
    })
}

#[cfg(test)]
#[path = "tests/skill_catalog.rs"]
mod tests;
