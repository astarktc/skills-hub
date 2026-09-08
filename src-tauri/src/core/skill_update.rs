//! Update owns admission, central settlement (including Edit replay), and
//! Propagation. Acquisition chooses bytes outside the Mutation guard; apply
//! re-admits them against the current row before touching the central copy.
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::{
    cancel_token::CancelToken,
    central_repo::ensure_central_repo,
    clock::now_ms,
    content_identity,
    errors::SignalError,
    git_acquisition::{acquire, parse_github_url, AcquireRequest, GithubApi, SkillIntent},
    install_finalize::{finalize_update, StagingDir},
    installer::InstallerPaths,
    propagation::{propagate_unlocked, PropagationReport},
    provenance::{is_refreshable, Provenance},
    skill_store::{SkillRecord, SkillStore},
    sync_engine::copy_dir_recursive,
};

/// The four byte adapters. Local folders are staged by this module; Restore
/// carries rebuilt bytes when acquisition found the central copy absent.
pub(crate) enum UpdateBytes {
    GitAcquired {
        staged: StagingDir,
        revision: Option<String>,
    },
    LocalFolder {
        path: PathBuf,
    },
    EditInPlace,
    RestoreRebuild {
        staged: StagingDir,
        revision: Option<String>,
    },
}

/// Acquisition facts, not authority to upsert a record. `expected` is the
/// provenance acquisition read; `record` carries only proposed source changes.
pub(crate) struct UpdateRequest {
    pub expected: SkillRecord,
    pub record: SkillRecord,
    pub bytes: UpdateBytes,
    pub repoint: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateSkip {
    SkillGone,
    StaleAcquisition,
}

pub(crate) enum ApplyOutcome {
    Updated(UpdateOutcome),
    Skipped { reason: UpdateSkip },
}

/// Successful Update keeps its report shape; skips are separate report data.
pub(crate) struct UpdateOutcome {
    pub skill_id: String,
    pub name: String,
    pub content_hash: Option<String>,
    pub source_revision: Option<String>,
    pub propagation: PropagationReport,
    pub edit_conflict: Option<super::skill_edits::InvocationEditConflict>,
}

/// Settle these bytes and every existing target. Caller holds the guard.
/// Re-read the row even for Re-point: unrelated current fields always win.
pub(crate) fn apply_unlocked(
    paths: &InstallerPaths,
    store: &SkillStore,
    request: UpdateRequest,
) -> Result<ApplyOutcome> {
    let Some(mut current) = store.get_skill_by_id(&request.expected.id)? else {
        return Ok(ApplyOutcome::Skipped {
            reason: UpdateSkip::SkillGone,
        });
    };
    if !request.repoint
        && (current.source_ref != request.expected.source_ref
            || current.source_subpath != request.expected.source_subpath
            || current.source_type != request.expected.source_type)
    {
        return Ok(ApplyOutcome::Skipped {
            reason: UpdateSkip::StaleAcquisition,
        });
    }
    current.source_ref = request.record.source_ref;
    current.source_subpath = request.record.source_subpath;
    current.source_type = request.record.source_type;
    let now = now_ms();
    let (updated, edit_conflict) = match request.bytes {
        UpdateBytes::EditInPlace => {
            current.updated_at = now;
            content_identity::record(store, &mut current)?;
            (current, None)
        }
        bytes => {
            let (staged, revision) = match bytes {
                UpdateBytes::GitAcquired { staged, revision }
                | UpdateBytes::RestoreRebuild { staged, revision } => (staged, revision),
                UpdateBytes::LocalFolder { path } => {
                    let parent = Path::new(&current.central_path)
                        .parent()
                        .context("invalid central path")?;
                    ensure_central_repo(parent)?;
                    let staged = StagingDir::new_in(parent);
                    copy_dir_recursive(&path, staged.path())?;
                    (staged, None)
                }
                UpdateBytes::EditInPlace => unreachable!(),
            };
            finalize_update(store, &current, staged, revision, |updated| {
                super::skill_edits::replay_unlocked(store, updated)
            })?
        }
    };
    let propagation = propagate_unlocked(store, paths, &updated.id, now)?;
    Ok(ApplyOutcome::Updated(UpdateOutcome {
        skill_id: updated.id,
        name: updated.name,
        content_hash: updated.content_hash,
        source_revision: updated.source_revision,
        propagation,
        edit_conflict,
    }))
}

/// Acquire source bytes without settling rows or targets. The source override
/// is validated by the Re-point adapter before reaching this seam.
#[allow(clippy::too_many_arguments)]
pub(crate) fn acquire_update(
    paths: &InstallerPaths,
    store: &SkillStore,
    skill_id: &str,
    cancel: Option<&CancelToken>,
    api: &dyn GithubApi,
    ttl_ms: i64,
    source_override: Option<(&str, &super::git_acquisition::GitSource)>,
) -> Result<UpdateRequest> {
    let mut record = store.get_skill_by_id(skill_id)?.ok_or_else(|| {
        anyhow::anyhow!(SignalError::NotFound {
            kind: "skill".to_string(),
            id: skill_id.to_string(),
        })
    })?;

    let expected = record.clone();

    // The Provenance rule: a skill with no external source has nothing to
    // acquire, whatever the state of its central copy. The central copy's
    // own presence is deliberately *not* checked: a `git`/`local` skill
    // whose central copy is gone is exactly what Restore re-acquires, and
    // finalize rebuilds it at the recorded path.
    if !is_refreshable(&record) {
        anyhow::bail!(not_refreshable(&record));
    }

    let central_path = PathBuf::from(record.central_path.clone());
    let restore = !central_path.exists();
    let central_parent = central_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid central path"))?
        .to_path_buf();
    // The central repo itself may be gone too (a Restore after the whole
    // library folder was lost); the staging dir needs its parent.
    ensure_central_repo(&central_parent)?;

    // Build new content in a sibling staging dir; finalize swaps it in.
    let staged = StagingDir::new_in(&central_parent);
    let staging_dir = staged.path().to_path_buf();

    let mut new_revision: Option<String> = None;

    match Provenance::parse(&record.source_type) {
        Some(Provenance::Git) => {
            let repo_url = record
                .source_ref
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("missing source_ref for git skill"))?;
            let source = source_override
                .map(|(_, source)| source.clone())
                .unwrap_or_else(|| parse_github_url(repo_url));

            // Stored paths are explicit selections. URL paths stay on the source
            // so acquisition can correct their branch boundary before using them.
            let intent = if source_override.is_some() {
                SkillIntent::NamedSkill(Some(&record.name))
            } else if let Some(subpath) = record.source_subpath.as_deref() {
                SkillIntent::Subpath(subpath)
            } else {
                SkillIntent::NamedSkillOrWholeRepo(&record.name)
            };

            let acquired = acquire(
                &AcquireRequest {
                    source: &source,
                    intent,
                    stored_subpath: if source_override.is_some() {
                        None
                    } else {
                        record.source_subpath.as_deref()
                    },
                    dest: &staging_dir,
                    cache_dir: &paths.cache_dir,
                    ttl_ms,
                    cancel,
                    allow_fast_path: true,
                },
                api,
            )?;
            new_revision = Some(acquired.revision);

            if let Some((url, _)) = source_override {
                if !super::skill_discovery::is_skill_dir(&staging_dir) {
                    anyhow::bail!(SignalError::SkillInvalid {
                        reason: "missing_skill_md".into()
                    });
                }
                record.source_ref = Some(url.to_string());
            }
            // Acquisition owns the branch/path split. Finalize carries this
            // resolved path into the record, including legacy backfills.
            record.source_subpath = acquired.resolved_subpath.filter(|subpath| subpath != ".");
        }
        Some(Provenance::Local) => {
            let source = record
                .source_ref
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("missing source_ref for local skill"))?;
            let source_path = PathBuf::from(source);
            if !source_path.exists() {
                anyhow::bail!(SignalError::SourcePathMissing {
                    path: source_path.to_string_lossy().into_owned()
                });
            }
            if restore {
                copy_dir_recursive(&source_path, &staging_dir)
                    .with_context(|| format!("copy {:?} -> {:?}", source_path, staging_dir))?;
            }
        }
        // Excluded by the predicate above; restated so the match is total.
        Some(Provenance::Imported) | None => anyhow::bail!(not_refreshable(&record)),
    }

    let bytes = if restore {
        UpdateBytes::RestoreRebuild {
            staged,
            revision: new_revision,
        }
    } else if Provenance::parse(&record.source_type) == Some(Provenance::Local) {
        UpdateBytes::LocalFolder {
            path: PathBuf::from(record.source_ref.as_ref().context("missing local source")?),
        }
    } else {
        UpdateBytes::GitAcquired {
            staged,
            revision: new_revision,
        }
    };
    Ok(UpdateRequest {
        expected,
        record,
        bytes,
        repoint: source_override.is_some(),
    })
}

/// The typed condition for an Update of a skill that has no external source
/// (`provenance::is_refreshable` said no).
fn not_refreshable(record: &super::skill_store::SkillRecord) -> SignalError {
    SignalError::NotRefreshable {
        name: record.name.clone(),
    }
}

#[cfg(test)]
#[path = "tests/skill_update.rs"]
mod tests;
