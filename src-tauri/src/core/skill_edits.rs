//! Edits layered on the central copy, replayed before Propagation.
//! The edit row is the source of truth; bytes follow it. Persist a set/replay
//! before writing bytes. Clear restores bytes before deleting the row.
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

use super::{
    clock::now_ms,
    content_hash::hash_dir,
    errors::SignalError,
    frontmatter_edit::{self, InvocationLines},
    installer::InstallerPaths,
    mutation_guard,
    propagation::{propagate_unlocked, PropagationStatus},
    skill_catalog::{managed_skill_entry, ManagedSkillEntry},
    skill_discovery::{find_skill_md, InvocationMode},
    skill_store::{SkillEditKind, SkillEditRecord, SkillRecord, SkillStore},
};

#[derive(Clone, Debug, Serialize, specta::Type)]
pub struct InvocationOverride {
    pub mode: InvocationMode,
    pub base_mode: InvocationMode,
    pub conflict: bool,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
pub struct InvocationEditConflict {
    pub base_mode: InvocationMode,
    pub upstream_mode: InvocationMode,
    pub override_mode: InvocationMode,
}

fn edit_mode(edit: &SkillEditRecord) -> Result<InvocationMode> {
    InvocationMode::from_key(&edit.value).context("read invocation edit mode")
}

fn base_lines(edit: &SkillEditRecord) -> Result<InvocationLines> {
    serde_json::from_str(&edit.base_value).context("read invocation edit base")
}

pub fn invocation_override(
    store: &SkillStore,
    skill_id: &str,
) -> Result<Option<InvocationOverride>> {
    store
        .get_skill_edit(skill_id, SkillEditKind::InvocationMode)?
        .map(|edit| {
            Ok(InvocationOverride {
                mode: edit_mode(&edit)?,
                base_mode: base_lines(&edit)?.mode(),
                conflict: edit.conflict,
            })
        })
        .transpose()
}

fn manifest(record: &SkillRecord) -> Result<PathBuf> {
    let central = Path::new(&record.central_path);
    if !central.exists() {
        anyhow::bail!(SignalError::CentralPathMissing {
            path: record.central_path.clone()
        });
    }
    find_skill_md(central).ok_or_else(|| {
        anyhow::anyhow!(SignalError::CentralPathMissing {
            path: central.join("SKILL.md").to_string_lossy().into_owned()
        })
    })
}

fn record_hash(store: &SkillStore, record: &mut SkillRecord) -> Result<()> {
    record.content_hash = Some(hash_dir(Path::new(&record.central_path))?);
    record.updated_at = now_ms();
    store.upsert_skill(record)
}

pub fn set_invocation_override(
    paths: &InstallerPaths,
    store: &SkillStore,
    skill_id: &str,
    mode: Option<InvocationMode>,
) -> Result<ManagedSkillEntry> {
    mutation_guard::serialized(|| {
        let mut record = store.get_skill_by_id(skill_id)?.ok_or_else(|| {
            anyhow::anyhow!(SignalError::NotFound {
                kind: "skill".into(),
                id: skill_id.into()
            })
        })?;
        let path = manifest(&record)?;
        let existing = store.get_skill_edit(skill_id, SkillEditKind::InvocationMode)?;
        match mode {
            Some(mode) => {
                let base_value = match existing {
                    Some(edit) => edit.base_value,
                    None => serde_json::to_string(&frontmatter_edit::read_invocation_lines(
                        &frontmatter_edit::read_manifest(&path)?,
                    ))?,
                };
                store.upsert_skill_edit(&SkillEditRecord {
                    skill_id: skill_id.into(),
                    kind: SkillEditKind::InvocationMode,
                    value: mode.as_key().into(),
                    base_value,
                    conflict: false,
                    applied_at: now_ms(),
                })?;
                frontmatter_edit::apply_to_file(&path, |text| {
                    frontmatter_edit::write_invocation_mode(text, mode)
                })?;
            }
            None => {
                if let Some(edit) = existing {
                    let base = base_lines(&edit)?;
                    frontmatter_edit::apply_to_file(&path, |text| {
                        frontmatter_edit::restore_invocation_lines(text, &base)
                    })?;
                    store.delete_skill_edit(skill_id, SkillEditKind::InvocationMode)?;
                }
            }
        }
        record_hash(store, &mut record)?;
        let report = propagate_unlocked(
            store,
            paths,
            skill_id,
            record.content_hash.as_deref(),
            now_ms(),
        )?;
        for target in report.targets {
            if let PropagationStatus::Failed { error } = target.status {
                log::warn!(
                    "invocation edit propagation {skill_id} {:?}: {error:#}",
                    target.scope
                );
            }
        }
        managed_skill_entry(store, skill_id)?.context("edited skill missing from catalog")
    })
}

/// The caller has finalized upstream bytes and holds the Mutation guard.
pub(crate) fn replay_unlocked(
    store: &SkillStore,
    record: &mut SkillRecord,
) -> Result<Option<InvocationEditConflict>> {
    let Some(mut edit) = store.get_skill_edit(&record.id, SkillEditKind::InvocationMode)? else {
        return Ok(None);
    };
    let path = manifest(record)?;
    let upstream =
        frontmatter_edit::read_invocation_lines(&frontmatter_edit::read_manifest(&path)?);
    let base_mode = base_lines(&edit)?.mode();
    let upstream_mode = upstream.mode();
    let override_mode = edit_mode(&edit)?;
    // An unresolved conflict remains flagged across later unchanged Updates.
    edit.conflict |= upstream_mode != base_mode;
    let conflict = (upstream_mode != base_mode).then_some(InvocationEditConflict {
        base_mode,
        upstream_mode,
        override_mode,
    });
    edit.base_value = serde_json::to_string(&upstream)?;
    edit.applied_at = now_ms();
    store.upsert_skill_edit(&edit)?;
    frontmatter_edit::apply_to_file(&path, |text| {
        frontmatter_edit::write_invocation_mode(text, override_mode)
    })?;
    record_hash(store, record)?;
    Ok(conflict)
}

#[cfg(test)]
#[path = "tests/skill_edits.rs"]
mod tests;
