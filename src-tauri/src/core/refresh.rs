//! Refresh (all): re-acquire Managed skills from their sources, finalize
//! them, and propagate — as one backend-owned batch over a skill set.
//!
//! Two phases, deliberately separate:
//!
//! 1. **Acquire** every selected skill's bytes into its Staging dir, outside
//!    the mutation guard. Sequential today, but structured as one
//!    self-contained result per skill, so a bounded parallel pool drops in
//!    without touching phase two.
//! 2. **Apply** each acquired skill under the mutation guard: finalize, then
//!    Propagation. The guard is taken *per skill*, not once for the batch, so
//!    listings and other mutations are not blocked for the whole run.
//!
//! With [`RefreshPolicy::reassert_auto_sync`] the apply phase also re-asserts
//! the auto-sync invariant — every Managed skill is synced to every installed
//! Tool — so a Tool the skill was never on gets it now.
//!
//! Everything is report data: a skill that fails acquisition is reported and
//! excluded from phase two (and from the re-assert); a Sync target that fails
//! is reported by Propagation. Only reading the skill list can fail the
//! operation.

use anyhow::Result;

use super::global_sync::{
    sync_skills_to_tools_unlocked, BatchPolicy, BatchSkill, BatchTargetStatus,
};
use super::installer::{
    acquire_managed_skill_update, finalize_and_propagate_unlocked, AcquiredUpdate, InstallerPaths,
};
use super::mutation_guard;
use super::propagation::{
    PropagationOutcome, PropagationScope, PropagationSkip, PropagationStatus,
};
use super::skill_store::SkillStore;
use super::tool_adapters::{global_tool_entries, installed_keys};

/// Which Managed skills to refresh.
#[derive(Clone, Debug)]
pub enum RefreshSelection {
    All,
    Ids(Vec<String>),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RefreshPolicy {
    /// Also sync each refreshed skill to installed Tools it is not on yet —
    /// the auto-sync invariant, re-asserted. Off leaves the target set alone.
    pub reassert_auto_sync: bool,
}

/// Which half of the batch a progress tick is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshPhase {
    Acquiring,
    Applying,
}

/// Progress tick emitted before each per-skill step of each phase.
pub struct RefreshProgress<'a> {
    /// 1-based index within the phase.
    pub index: usize,
    pub total: usize,
    pub skill_name: &'a str,
    pub phase: RefreshPhase,
}

#[derive(Debug)]
pub enum SkillRefreshStatus {
    /// Acquired, finalized and propagated. `targets` is Propagation's report.
    Refreshed {
        content_hash: Option<String>,
        source_revision: Option<String>,
        targets: Vec<PropagationOutcome>,
    },
    /// Acquisition or finalize failed; this skill's targets were left alone.
    Failed { error: anyhow::Error },
}

#[derive(Debug)]
pub struct SkillRefreshOutcome {
    pub skill_id: String,
    pub skill_name: String,
    pub status: SkillRefreshStatus,
}

#[derive(Debug, Default)]
pub struct RefreshReport {
    pub skills: Vec<SkillRefreshOutcome>,
}

/// Refresh a set of Managed skills (or all of them).
///
/// Mutation entry point: phase two serialises each skill's finalize +
/// Propagation against every other Sync-target mutation. Acquisition is
/// deliberately outside the guard.
pub fn refresh_managed_skills(
    paths: &InstallerPaths,
    store: &SkillStore,
    selection: RefreshSelection,
    policy: RefreshPolicy,
    now: i64,
    mut on_progress: impl FnMut(RefreshProgress),
) -> Result<RefreshReport> {
    let selected = select_skills(store, &selection)?;
    let total = selected.len();

    // Phase 1 — acquire (unlocked, slow I/O).
    let mut acquired: Vec<(String, String, Result<AcquiredUpdate>)> = Vec::with_capacity(total);
    for (index, (skill_id, skill_name)) in selected.into_iter().enumerate() {
        on_progress(RefreshProgress {
            index: index + 1,
            total,
            skill_name: &skill_name,
            phase: RefreshPhase::Acquiring,
        });
        let result = acquire_managed_skill_update(paths, store, &skill_id);
        acquired.push((skill_id, skill_name, result));
    }

    // Phase 2 — apply (finalize + propagate) one skill at a time under the guard.
    let mut report = RefreshReport::default();
    for (index, (skill_id, skill_name, result)) in acquired.into_iter().enumerate() {
        let update = match result {
            Ok(update) => update,
            Err(error) => {
                // Reported and excluded from propagation and the re-assert.
                report.skills.push(SkillRefreshOutcome {
                    skill_id,
                    skill_name,
                    status: SkillRefreshStatus::Failed { error },
                });
                continue;
            }
        };
        on_progress(RefreshProgress {
            index: index + 1,
            total,
            skill_name: &skill_name,
            phase: RefreshPhase::Applying,
        });
        let status =
            mutation_guard::serialized(|| apply_one_unlocked(paths, store, update, policy, now));
        report.skills.push(SkillRefreshOutcome {
            skill_id,
            skill_name,
            status,
        });
    }
    Ok(report)
}

/// `(id, name)` for every selected skill. An id with no row is dropped rather
/// than failing the batch — the listing that produced it may be stale.
fn select_skills(
    store: &SkillStore,
    selection: &RefreshSelection,
) -> Result<Vec<(String, String)>> {
    match selection {
        RefreshSelection::All => Ok(store
            .list_skills()?
            .into_iter()
            .map(|s| (s.id, s.name))
            .collect()),
        RefreshSelection::Ids(ids) => {
            let mut out = Vec::with_capacity(ids.len());
            for id in ids {
                if let Some(skill) = store.get_skill_by_id(id)? {
                    out.push((skill.id, skill.name));
                }
            }
            Ok(out)
        }
    }
}

/// Finalize one acquired skill and bring its Sync targets into line. The
/// caller holds the mutation guard.
fn apply_one_unlocked(
    paths: &InstallerPaths,
    store: &SkillStore,
    update: AcquiredUpdate,
    policy: RefreshPolicy,
    now: i64,
) -> SkillRefreshStatus {
    let outcome = match finalize_and_propagate_unlocked(paths, store, update) {
        Ok(outcome) => outcome,
        Err(error) => return SkillRefreshStatus::Failed { error },
    };
    let mut targets = outcome.propagation.targets;
    if policy.reassert_auto_sync {
        match reassert_auto_sync_unlocked(
            paths,
            store,
            &outcome.skill_id,
            &outcome.name,
            now,
            &targets,
        ) {
            Ok(extra) => targets.extend(extra),
            Err(error) => log::warn!(
                "failed to re-assert auto-sync for {}: {:#}",
                outcome.name,
                error
            ),
        }
    }
    SkillRefreshStatus::Refreshed {
        content_hash: outcome.content_hash,
        source_revision: outcome.source_revision,
        targets,
    }
}

/// The auto-sync invariant, re-asserted for one skill: sync it to every
/// installed Tool it has no target row for. Existing targets were already
/// brought into line by Propagation, so they are not touched again here;
/// a directory that is in the way without a row is reported, not clobbered
/// (`overwrite_if_same_content`).
fn reassert_auto_sync_unlocked(
    paths: &InstallerPaths,
    store: &SkillStore,
    skill_id: &str,
    skill_name: &str,
    now: i64,
    already: &[PropagationOutcome],
) -> Result<Vec<PropagationOutcome>> {
    let skill = match store.get_skill_by_id(skill_id)? {
        Some(skill) => skill,
        None => return Ok(Vec::new()),
    };
    let existing: Vec<&str> = already
        .iter()
        .filter_map(|outcome| match &outcome.scope {
            PropagationScope::Global { tool } => Some(tool.as_str()),
            PropagationScope::Project { .. } => None,
        })
        .collect();
    let missing: Vec<String> = installed_keys(&global_tool_entries(&paths.home))
        .into_iter()
        .filter(|key| !existing.contains(&key.as_str()))
        .collect();
    if missing.is_empty() {
        return Ok(Vec::new());
    }

    let skills = [BatchSkill {
        skill_id: skill.id.clone(),
        skill_name: skill_name.to_string(),
        source_path: std::path::PathBuf::from(&skill.central_path),
    }];
    let policy = BatchPolicy {
        overwrite: false,
        overwrite_if_same_content: true,
        overrides: Vec::new(),
    };
    let outcomes =
        sync_skills_to_tools_unlocked(&paths.home, store, &skills, &missing, &policy, now, |_| {});

    Ok(outcomes
        .into_iter()
        .map(|outcome| PropagationOutcome {
            scope: PropagationScope::Global {
                tool: outcome.tool_key.clone(),
            },
            status: match outcome.status {
                BatchTargetStatus::Synced { outcome } => PropagationStatus::Synced {
                    mode_used: outcome.mode_used,
                },
                BatchTargetStatus::Skipped { error } => match error {
                    super::global_sync::GlobalSyncError::ToolNotInstalled { tool_key } => {
                        PropagationStatus::Skipped {
                            reason: PropagationSkip::ToolNotInstalled { tool: tool_key },
                        }
                    }
                    // A skips-because-unwritable is still a failure to report:
                    // the operator asked for this Tool to carry the skill.
                    other => PropagationStatus::Failed {
                        error: anyhow::Error::new(other),
                    },
                },
                BatchTargetStatus::Failed { error } => PropagationStatus::Failed {
                    error: anyhow::Error::new(error),
                },
            },
        })
        .collect())
}

#[cfg(test)]
#[path = "tests/refresh.rs"]
mod tests;
