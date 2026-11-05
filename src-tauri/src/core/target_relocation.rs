//! Target relocation: the once-per-launch pass that moves global Sync
//! targets out of a Tool's *superseded* skills directories into the one it
//! deploys to — a former dir the registry corrected, or the dir the #83
//! preference ("prefer the AGENTS standard folder", `GlobalLayout`) moved
//! the Tool away from. Saving the preference runs the same pass
//! ([`apply_global_layout`]).
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
//! For each global row whose artifact sits directly in a superseded dir
//! (`tool_adapters::superseded_skills_dirs_in`):
//!
//! 0. the move must pass the global batch's Disabled admission for the
//!    **destination** group (`global_sync::DisabledGate`): a skill disabled
//!    at skill grain, or a `disabled` row (owning or not) of any Tool sharing
//!    the dir the row would move into, keeps the row and its artifact where
//!    they are — a move must never re-materialise a skill where a disabled
//!    Tool reads, and no marker is cleared. The preview applies the same rule;
//! 1. the old artifact must be present (`symlink_metadata`) — an absent one
//!    has nothing to move, and materialising anyway could create a skills
//!    dir for a Tool that is gone; the row stays and stays removable
//!    (former dirs are inside the deletion rule). The one exception is our
//!    own identical bytes already at the destination: an interrupted move,
//!    which the pass completes;
//! 2. the skill is materialised in the current dir from its central copy
//!    through the capability-aware sync entry point, replacing an existing
//!    target only when its content is identical — foreign bytes there keep
//!    the row where it is (logged, retried next launch);
//! 3. the old artifact is removed through `artifact_removal`'s fenced seam —
//!    **unless any other row still owns it** — a global row of any skill or
//!    a project assignment (`artifact_removal::artifact_is_owned_except`,
//!    the one ownership inventory): a former dir may be another Tool's live
//!    dir (Kimi's former `~/.config/agents/skills` is Amp's, brief I4), and
//!    a project rooted at home owns `~/.augment/skills/<name>` exactly where
//!    the global Augment row did. A removal **failure rolls the move back**
//!    (ADR-0002): the destination it created goes, and the rows stay at the
//!    source, owning it, as `error` — left in place and reported
//!    ([`RelocationSummary::incomplete`]), retried next launch;
//! 4. every row sharing that artifact is settled with `SyncCompleted` at the
//!    new path — a row stored under a former product name's key
//!    ([`ToolAdapter::former_keys`]: Clawdbot, MoltBot → OpenClaw) is
//!    re-keyed to its Tool first, unless the skill already has a row under
//!    that Tool (then it stays, removable).
//!
//! Idempotent: a relocated row is no longer in a former dir.

use std::path::{Path, PathBuf};

use anyhow::Result;

use super::artifact_removal::{
    artifact_is_owned, artifact_is_owned_except, remove_relocation_destination_unlocked,
    remove_superseded_artifact_unlocked,
};
use super::content_identity::{self, Source};
use super::errors::CommandError;
use super::global_sync::DisabledGate;
use super::mutation_guard;
use super::settings;
use super::skill_store::{SkillRecord, SkillStore, SkillTargetRecord, TargetTransition};
use super::sync_engine::{self, SyncOutcome};
use super::sync_status::SyncStatus;
use super::tool_adapters::{
    adapter_by_key, adapter_succeeding_key, adapters_sharing_skills_dir, global_skills_dir_in,
    superseded_skills_dirs_in, GlobalLayout, ToolAdapter,
};

/// How many global target rows the pass moved, and how many it found in a
/// superseded dir but had to leave there.
#[derive(Debug, Default, PartialEq, Eq, serde::Serialize, specta::Type)]
pub struct RelocationSummary {
    pub relocated: usize,
    pub left_in_place: usize,
    /// The left-in-place rows whose move was **rolled back** because their
    /// source could not be removed — each kept at its source as `error`,
    /// with the removal's classified error. The next launch retries them.
    pub incomplete: Vec<RelocationIncomplete>,
}

/// One row a move left at its source after a failed source removal.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, specta::Type)]
pub struct RelocationIncomplete {
    pub skill_id: String,
    /// Registry key of the row's Tool.
    pub tool: String,
    /// The source path the row still records and owns.
    pub path: String,
    pub error: CommandError,
}

/// Relocate every global Sync target still recorded in a dir its Tool no
/// longer deploys to under the `GlobalLayout` in effect (a former dir, or
/// the dir the #83 preference moved it away from). The once-per-launch pass.
/// On an existing install that never saved the preference the layout is own
/// dirs (`settings::global_layout`), so this pass then moves registry-truth
/// relocations only — never a reader's own-dir target on the default.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
pub fn relocate_former_global_targets(
    store: &SkillStore,
    home: &Path,
    now: i64,
) -> Result<RelocationSummary> {
    mutation_guard::serialized(|| {
        relocate_unlocked(store, home, settings::global_layout(store), now)
    })
}

/// Save the #83 preference: persist `layout` and relocate every target it
/// supersedes, in one guarded section — an AGENTS-standard reader's targets
/// move into `~/.agents/skills` when it turns on, and back to the harness's
/// own dir when it turns off (by the same rule: byte-identical replace, else
/// leave + log). Returns what moved, for the UI's confirmation to report.
///
/// Mutation entry point: serialised against every other Sync-target mutation.
pub fn apply_global_layout(
    store: &SkillStore,
    home: &Path,
    layout: GlobalLayout,
    now: i64,
) -> Result<RelocationSummary> {
    mutation_guard::serialized(|| {
        settings::write_prefer_agents_standard_dir(store, layout.prefer_agents_standard)?;
        relocate_unlocked(store, home, layout, now)
    })
}

/// How many global target rows [`apply_global_layout`] would try to move
/// under `layout` — the count the preference's confirmation states. Read-only;
/// the same admission as the pass ([`admission`]), so a row the pass would
/// leave in place is never counted (a row whose move then fails is reported
/// as left in place).
pub fn preview_relocation(store: &SkillStore, home: &Path, layout: GlobalLayout) -> Result<usize> {
    let mut count = 0;
    for skill in store.list_skills()? {
        let gate = DisabledGate::read(store, &skill.id)?;
        for row in store.list_skill_targets(&skill.id)? {
            if let Some(Admission::Move(_)) = admission(store, home, layout, &gate, &row)? {
                if Path::new(&row.target_path).symlink_metadata().is_ok() {
                    count += 1;
                }
            }
        }
    }
    Ok(count)
}

/// A former key's row (`ToolAdapter::former_keys`) cannot be re-keyed while
/// the skill already has a row under the Tool's key (`UNIQUE(skill_id,
/// tool)`): it stays where it is, removable.
fn rekey_blocked(
    store: &SkillStore,
    skill_id: &str,
    row: &SkillTargetRecord,
    adapter: &ToolAdapter,
) -> Result<bool> {
    Ok(row.tool != adapter.key() && store.get_skill_target(skill_id, adapter.key())?.is_some())
}

/// The row's Tool when the row is a relocation candidate under `layout`: it
/// owns a live artifact (not Disabled, materialised by us) sitting directly
/// in one of its Tool's superseded dirs.
fn superseded_row(
    home: &Path,
    layout: GlobalLayout,
    row: &SkillTargetRecord,
) -> Option<&'static ToolAdapter> {
    // A Disabled row has no artifact to move and must not gain one: it
    // keeps its recorded path until Enable re-syncs the pair. A row that
    // never materialised (`owns_artifact`) has nothing of ours there either:
    // moving would delete someone else's former folder.
    if row.status == SyncStatus::Disabled || !row.owns_artifact() {
        return None;
    }
    // A retired key's row (Clawdbot, MoltBot) belongs to the Tool it was a
    // former name of, and moves with it out of that Tool's former dirs.
    let adapter = adapter_by_key(&row.tool).or_else(|| adapter_succeeding_key(&row.tool))?;
    let old = Path::new(&row.target_path);
    superseded_skills_dirs_in(home, adapter, layout)
        .iter()
        .any(|dir| old.parent() == Some(dir.as_path()))
        .then_some(adapter)
}

/// What the pass does with one candidate row.
enum Admission {
    /// Materialise in the Tool's current dir and settle the row there.
    Move(&'static ToolAdapter),
    /// The row and its artifact stay where they are (`left_in_place`).
    Stay(&'static str),
}

/// The one admission rule of the pass and its preview. `None`: not a
/// candidate (see [`superseded_row`]). A candidate stays when
///
/// - the **destination** group is Disabled for the skill — the global
///   batch's rule ([`DisabledGate`]): the skill is disabled at skill grain,
///   or any Tool sharing the dir the row would move into has a `disabled`
///   row, owning or not. A move there would re-materialise the skill where a
///   disabled Tool reads; only Enable does that, and no marker is cleared;
/// - its former key's re-key is blocked ([`rekey_blocked`]).
fn admission(
    store: &SkillStore,
    home: &Path,
    layout: GlobalLayout,
    gate: &DisabledGate,
    row: &SkillTargetRecord,
) -> Result<Option<Admission>> {
    let Some(adapter) = superseded_row(home, layout, row) else {
        return Ok(None);
    };
    if gate.blocks_group(adapter, &adapters_sharing_skills_dir(adapter, layout)) {
        return Ok(Some(Admission::Stay(
            "the skill is disabled where it would move",
        )));
    }
    if rekey_blocked(store, &row.skill_id, row, adapter)? {
        return Ok(Some(Admission::Stay(
            "the skill already has a row under its Tool's key",
        )));
    }
    Ok(Some(Admission::Move(adapter)))
}

/// One superseded artifact of one skill moving into one destination dir,
/// with every admitted row recording it and that row's Tool. The first
/// row's Tool materialises (and fences) the move.
struct Move {
    old: PathBuf,
    dest_dir: PathBuf,
    adapter: &'static ToolAdapter,
    rows: Vec<(SkillTargetRecord, &'static ToolAdapter)>,
}

pub(crate) fn relocate_unlocked(
    store: &SkillStore,
    home: &Path,
    layout: GlobalLayout,
    now: i64,
) -> Result<RelocationSummary> {
    let mut summary = RelocationSummary::default();
    for skill in store.list_skills()? {
        relocate_skill_unlocked(store, home, layout, &skill, now, &mut summary)?;
    }
    Ok(summary)
}

/// The pass over one skill's rows. Also the seam a global first sync hands
/// an owned row to when it would otherwise re-point that row away from the
/// artifact it still owns (`global_sync::sync_skills_to_tools_unlocked`).
/// Unlocked internal seam.
pub(crate) fn relocate_skill_unlocked(
    store: &SkillStore,
    home: &Path,
    layout: GlobalLayout,
    skill: &SkillRecord,
    now: i64,
    summary: &mut RelocationSummary,
) -> Result<()> {
    // Read once per skill: the pass changes paths, never a Disabled status.
    let gate = DisabledGate::read(store, &skill.id)?;
    let mut moves: Vec<Move> = Vec::new();
    for row in store.list_skill_targets(&skill.id)? {
        let old = PathBuf::from(&row.target_path);
        let adapter = match admission(store, home, layout, &gate, &row)? {
            None => continue,
            Some(Admission::Stay(reason)) => {
                log::info!(
                    "{}'s {} row stays at {}: {reason}",
                    skill.name,
                    row.tool,
                    old.display()
                );
                summary.left_in_place += 1;
                continue;
            }
            Some(Admission::Move(adapter)) => adapter,
        };
        let dest_dir = global_skills_dir_in(home, adapter, layout);
        match moves
            .iter_mut()
            .find(|m| m.old == old && m.dest_dir == dest_dir)
        {
            Some(existing) => existing.rows.push((row, adapter)),
            None => moves.push(Move {
                old,
                dest_dir,
                adapter,
                rows: vec![(row, adapter)],
            }),
        }
    }
    for relocation in moves {
        settle_move(store, home, layout, skill, relocation, now, summary)?;
    }
    Ok(())
}

/// One move, ordered so that an interruption between any two steps leaves
/// either the row at its source with its artifact (plus, at most, our
/// identical bytes at the destination, which the next launch replaces), or
/// the row at an absent source with our bytes at the destination (which the
/// next launch adopts, [`materialise`]) — never an artifact no row records:
///
/// 1. materialise the destination;
/// 2. remove the source — unless anything besides the moving rows still
///    owns it (`artifact_removal::artifact_is_owned_except`: another
///    skill's or Tool's global row, a project assignment; a former dir may
///    be another Tool's live dir, brief I4);
/// 3. settle the rows at the destination (re-keying a former name's row).
///
/// A failed source removal **rolls the move back** (ADR-0002): the
/// destination this move wrote is removed (unless a row owns it — a sharer
/// already deployed there), and every moving row stays at its source, owning it, as
/// `error` with the removal's chain — counted left in place, the reason in
/// [`RelocationSummary::incomplete`] — so the next launch retries it.
fn settle_move(
    store: &SkillStore,
    home: &Path,
    layout: GlobalLayout,
    skill: &SkillRecord,
    relocation: Move,
    now: i64,
    summary: &mut RelocationSummary,
) -> Result<()> {
    let Move {
        old,
        dest_dir,
        adapter,
        rows,
    } = relocation;
    let Some(outcome) = materialise(store, adapter, skill, &old, &dest_dir) else {
        summary.left_in_place += rows.len();
        return Ok(());
    };

    let moving: Vec<&str> = rows.iter().map(|(row, _)| row.id.as_str()).collect();
    if artifact_is_owned_except(store, &old, &moving)? {
        log::info!(
            "relocating {} keeps {}: another row still owns it",
            skill.name,
            old.display()
        );
    } else if let Err(error) = remove_superseded_artifact_unlocked(home, adapter, layout, &old) {
        let detail = format!("{error:#}");
        log::warn!(
            "cannot relocate {}: its former target {} could not be removed: {detail}; \
             rolled back, row left in place",
            skill.name,
            old.display()
        );
        // Whatever was at the destination, `materialise` just wrote our
        // bytes there; unless a row records them, they go.
        let dest = &outcome.target_path;
        if !artifact_is_owned(store, dest)? {
            if let Err(err) = remove_relocation_destination_unlocked(home, adapter, layout, dest) {
                log::warn!(
                    "rolling back {}'s relocation could not remove {}: {err:#}",
                    skill.name,
                    dest.display()
                );
            }
        }
        let error = CommandError::from_anyhow(error);
        for (row, _) in &rows {
            store.transition_skill_target(
                &row.id,
                TargetTransition::SyncFailed { error: &detail },
            )?;
            summary.incomplete.push(RelocationIncomplete {
                skill_id: skill.id.clone(),
                tool: row.tool.clone(),
                path: row.target_path.clone(),
                error: error.clone(),
            });
        }
        summary.left_in_place += rows.len();
        return Ok(());
    }

    let target_path = outcome.target_path.to_string_lossy();
    for (row, row_adapter) in &rows {
        // A former key's row is re-keyed to its Tool as it moves.
        if row.tool != row_adapter.key() {
            store.rekey_skill_target(&row.id, row_adapter.key())?;
        }
        store.transition_skill_target(
            &row.id,
            TargetTransition::SyncCompleted {
                mode: outcome.mode_used,
                target_path: &target_path,
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
    Ok(())
}

/// Write `skill` into `dest_dir` (where the Tool deploys under the layout)
/// under the old artifact's name. `None` (logged) when the move must not
/// happen now.
fn materialise(
    store: &SkillStore,
    adapter: &ToolAdapter,
    skill: &SkillRecord,
    old: &Path,
    dest_dir: &Path,
) -> Option<SyncOutcome> {
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
    let target = dest_dir.join(name);
    let overwrite = content_identity::same_content(
        Source::Managed {
            store,
            skill_id: &skill.id,
        },
        &target,
    );
    // An absent source has nothing to move — unless our identical bytes
    // already wait at the destination: a move interrupted after its source
    // was removed, which this pass completes rather than leave them unowned.
    if old.symlink_metadata().is_err() && !overwrite {
        log::info!(
            "former target {} of {} is absent; row left in place",
            old.display(),
            skill.name
        );
        return None;
    }
    let identity = Source::Managed {
        store,
        skill_id: &skill.id,
    };
    match sync_engine::sync_dir_for_tool_with_overwrite(
        adapter, central, identity, &target, overwrite,
    ) {
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
