//! Target relocation: the once-per-launch pass that moves global Sync
//! targets out of a Tool's *former* skills directory into its current one.
//!
//! When the registry corrects a Tool's global skills dir (Augment: from
//! `~/.augment/rules`, its rules dir, to `~/.augment/skills` in 1.2.18),
//! existing target rows still record the old path — and nothing else moves
//! them: Propagation writes to a row's stored path and skips links outright,
//! and global sync only re-records a pair the operator syncs again (leaving
//! the old artifact behind). This pass closes that gap, driven entirely by
//! the registry fact [`ToolAdapter::former_relative_skills_dirs`]; no tool
//! is named here.
//!
//! For each global row whose artifact sits directly in a former dir:
//!
//! 1. the old artifact must be present (`symlink_metadata`) — an absent one
//!    has nothing to move, and materialising anyway could create a skills
//!    dir for a Tool that is gone; the row stays and stays removable
//!    (former dirs are inside the deletion rule);
//! 2. the skill is materialised in the current dir from its central copy
//!    through the capability-aware sync entry point, replacing an existing
//!    target only when its content is identical — foreign bytes there keep
//!    the row where it is (logged, retried next launch);
//! 3. every row sharing that artifact is settled with `SyncCompleted` at the
//!    new path;
//! 4. the old artifact is removed through `artifact_removal`'s fenced seam.
//!    A failure there is logged; the row already describes the new target.
//!
//! Idempotent: a relocated row is no longer in a former dir.

use std::path::{Path, PathBuf};

use anyhow::Result;

use super::artifact_removal::remove_superseded_artifact_unlocked;
use super::content_identity::{self, Source};
use super::mutation_guard;
use super::skill_store::{SkillRecord, SkillStore, TargetTransition};
use super::sync_engine::{self, SyncOutcome};
use super::tool_adapters::{adapter_by_key, former_skills_dirs_in, skills_dir_in, ToolAdapter};

/// How many global target rows the pass moved, and how many it found in a
/// former dir but had to leave there.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RelocationSummary {
    pub relocated: usize,
    pub left_in_place: usize,
}

/// Relocate every global Sync target still recorded in a Tool's former
/// skills dir.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
pub fn relocate_former_global_targets(
    store: &SkillStore,
    home: &Path,
    now: i64,
) -> Result<RelocationSummary> {
    mutation_guard::serialized(|| relocate_unlocked(store, home, now))
}

/// One former artifact of one skill, materialised (or not) once and shared
/// by every row recording it.
struct Move {
    old: PathBuf,
    adapter: &'static ToolAdapter,
    outcome: Option<SyncOutcome>,
}

pub(crate) fn relocate_unlocked(
    store: &SkillStore,
    home: &Path,
    now: i64,
) -> Result<RelocationSummary> {
    let mut summary = RelocationSummary::default();
    for skill in store.list_skills()? {
        let mut moves: Vec<Move> = Vec::new();
        for row in store.list_skill_targets(&skill.id)? {
            let Some(adapter) = adapter_by_key(&row.tool) else {
                continue;
            };
            let old = PathBuf::from(&row.target_path);
            let in_former_dir = former_skills_dirs_in(home, adapter)
                .iter()
                .any(|dir| old.parent() == Some(dir.as_path()));
            if !in_former_dir {
                continue;
            }

            let index = match moves.iter().position(|m| m.old == old) {
                Some(index) => index,
                None => {
                    let outcome = materialise(store, home, adapter, &skill, &old);
                    moves.push(Move {
                        old: old.clone(),
                        adapter,
                        outcome,
                    });
                    moves.len() - 1
                }
            };

            match &moves[index].outcome {
                Some(outcome) => {
                    store.transition_skill_target(
                        &row.id,
                        TargetTransition::SyncCompleted {
                            mode: outcome.mode_used,
                            target_path: &outcome.target_path.to_string_lossy(),
                            synced_at: now,
                        },
                    )?;
                    log::info!(
                        "relocated {} for {} from {} to {}",
                        skill.name,
                        row.tool,
                        old.display(),
                        outcome.target_path.display()
                    );
                    summary.relocated += 1;
                }
                None => summary.left_in_place += 1,
            }
        }

        for relocated in moves.iter().filter(|m| m.outcome.is_some()) {
            if let Err(err) =
                remove_superseded_artifact_unlocked(home, relocated.adapter, &relocated.old)
            {
                log::warn!(
                    "relocated {} but could not remove its former target {}: {:#}",
                    skill.name,
                    relocated.old.display(),
                    err
                );
            }
        }
    }
    Ok(summary)
}

/// Write `skill` into the Tool's current skills dir under the old
/// artifact's name. `None` (logged) when the move must not happen now.
fn materialise(
    store: &SkillStore,
    home: &Path,
    adapter: &ToolAdapter,
    skill: &SkillRecord,
    old: &Path,
) -> Option<SyncOutcome> {
    if old.symlink_metadata().is_err() {
        log::info!(
            "former target {} of {} is absent; row left in place",
            old.display(),
            skill.name
        );
        return None;
    }
    let central = Path::new(&skill.central_path);
    if !central.is_dir() {
        log::warn!(
            "cannot relocate {}: central copy {} is missing",
            skill.name,
            central.display()
        );
        return None;
    }
    let name = old.file_name()?;
    let target = skills_dir_in(home, adapter).join(name);
    let overwrite = content_identity::same_content(
        Source::Managed {
            store,
            skill_id: &skill.id,
        },
        &target,
    );
    match sync_engine::sync_dir_for_tool_with_overwrite(adapter, central, &target, overwrite) {
        Ok(outcome) => Some(outcome),
        Err(err) => {
            log::warn!(
                "cannot relocate {} to {}: {:#}; row left at {}",
                skill.name,
                target.display(),
                err,
                old.display()
            );
            None
        }
    }
}

#[cfg(test)]
#[path = "tests/target_relocation.rs"]
mod tests;
