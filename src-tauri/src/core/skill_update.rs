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
    git_acquisition::{
        acquire, parse_github_url, AcquireRequest, Acquired, GitSource, GithubApi, SkillIntent,
    },
    install_finalize::{finalize_update, StagingDir},
    installer::{ensure_installable_skill_dir, InstallerPaths},
    propagation::{propagate_unlocked, PropagationReport},
    provenance::{is_refreshable, Provenance},
    skill_store::{SkillRecord, SkillStore},
    sync_engine::copy_dir_recursive,
};

/// The three byte adapters. Git acquisition and local folders are staged
/// outside the guard; a direct Edit settles the central copy in place. Restore
/// is not a fourth adapter: when acquisition finds the central copy absent it
/// stages the same bytes, and finalize rebuilds the copy at the recorded path.
pub(crate) enum UpdateBytes {
    Acquired {
        staged: StagingDir,
        revision: Option<String>,
    },
    LocalFolder {
        staged: StagingDir,
    },
    EditInPlace {
        clear: bool,
    },
}

/// The only fields an Update may change on the row besides what finalize
/// owns: the source it was acquired from. Everything else is re-read under
/// the guard so unrelated current fields always win — except that a Re-point
/// also drops what belonged to the old source (`source_revision`,
/// `imported_from_tool`; see `apply_unlocked`).
struct SourceProposal {
    source_ref: Option<String>,
    source_subpath: Option<String>,
    source_type: String,
}

impl SourceProposal {
    fn unchanged(record: &SkillRecord) -> Self {
        Self {
            source_ref: record.source_ref.clone(),
            source_subpath: record.source_subpath.clone(),
            source_type: record.source_type.clone(),
        }
    }
}

/// Acquisition facts, not authority to upsert a record. `expected` is the
/// provenance acquisition read; `proposal` carries only proposed source
/// changes. Built only through `local`, `edit`, `acquire_update` and
/// `acquire_git_repoint`.
pub(crate) struct UpdateRequest {
    expected: SkillRecord,
    proposal: SourceProposal,
    bytes: UpdateBytes,
    repoint: bool,
}

impl UpdateRequest {
    /// Local byte adapter: stage outside the guard, just like git acquisition.
    /// With `repoint` the folder becomes the skill's source whatever its
    /// provenance was (`local`, the folder, no subpath); that proposal is only
    /// persisted by apply after staging succeeds.
    pub(crate) fn local(record: SkillRecord, source: &Path, repoint: bool) -> Result<Self> {
        if !source.exists() {
            anyhow::bail!(SignalError::SourcePathMissing {
                path: source.to_string_lossy().into_owned()
            });
        }
        let central = Path::new(&record.central_path);
        let parent = central.parent().context("invalid central path")?;
        ensure_central_repo(parent)?;
        let staged = StagingDir::new_in(parent);
        copy_dir_recursive(source, staged.path())?;
        let proposal = if repoint {
            SourceProposal {
                source_type: Provenance::Local.as_str().to_string(),
                source_ref: Some(source.to_string_lossy().into_owned()),
                source_subpath: None,
            }
        } else {
            SourceProposal::unchanged(&record)
        };
        Ok(Self {
            expected: record,
            proposal,
            bytes: UpdateBytes::LocalFolder { staged },
            repoint,
        })
    }

    /// Direct Edit: no bytes to acquire and no source change; the central copy
    /// is settled in place under the guard (`clear` drops the Edit row).
    pub(crate) fn edit(record: SkillRecord, clear: bool) -> Self {
        Self {
            proposal: SourceProposal::unchanged(&record),
            expected: record,
            bytes: UpdateBytes::EditInPlace { clear },
            repoint: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
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
    current.source_ref = request.proposal.source_ref;
    current.source_subpath = request.proposal.source_subpath;
    current.source_type = request.proposal.source_type;
    if request.repoint {
        // A Re-point replaces the source outright: nothing of the old one
        // survives. A git acquisition's revision is recorded by finalize; a
        // folder has none. A skill that gains an external source is no
        // longer imported, so its found-in Tool history goes too.
        current.source_revision = None;
        current.imported_from_tool = None;
    }
    let now = now_ms();
    let (updated, edit_conflict) = match request.bytes {
        UpdateBytes::EditInPlace { clear } => {
            super::skill_edits::settle_direct_unlocked(store, &current, clear)?;
            current.updated_at = now;
            content_identity::record(store, &mut current)?;
            (current, None)
        }
        UpdateBytes::Acquired { staged, revision } => {
            settle_staged(store, &current, staged, revision)?
        }
        UpdateBytes::LocalFolder { staged } => settle_staged(store, &current, staged, None)?,
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

/// Land staged bytes through finalize's failure-atomic window, replaying the
/// Edit inside it (ADR-0004).
fn settle_staged(
    store: &SkillStore,
    current: &SkillRecord,
    staged: StagingDir,
    revision: Option<String>,
) -> Result<(
    SkillRecord,
    Option<super::skill_edits::InvocationEditConflict>,
)> {
    finalize_update(store, current, staged, revision, |updated| {
        super::skill_edits::replay_unlocked(store, updated)
    })
}

/// Acquire source bytes for an Update of the skill's **own** source, without
/// settling rows or targets. A skill with no external source is refused
/// typed; a Re-point, which brings its own source, is [`acquire_git_repoint`]
/// or [`UpdateRequest::local`] with `repoint = true`.
pub(crate) fn acquire_update(
    paths: &InstallerPaths,
    store: &SkillStore,
    skill_id: &str,
    cancel: Option<&CancelToken>,
    api: &dyn GithubApi,
    ttl_ms: i64,
) -> Result<UpdateRequest> {
    let record = require_record(store, skill_id)?;

    // The Provenance rule: a skill with no external source has nothing to
    // acquire, whatever the state of its central copy. The central copy's
    // own presence is deliberately *not* checked: a `git`/`local` skill
    // whose central copy is gone is exactly what Restore re-acquires, and
    // finalize rebuilds it at the recorded path.
    if !is_refreshable(&record) {
        anyhow::bail!(not_refreshable(&record));
    }

    match Provenance::parse(&record.source_type) {
        Some(Provenance::Local) => {
            let source = PathBuf::from(record.source_ref.as_ref().context("missing local source")?);
            UpdateRequest::local(record, &source, false)
        }
        Some(Provenance::Git) => {
            let repo_url = record
                .source_ref
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("missing source_ref for git skill"))?;
            let source = parse_github_url(repo_url);
            let intent = SkillIntent::StoredRecord {
                name: &record.name,
                subpath: record.source_subpath.as_deref(),
            };
            let (staged, acquired) =
                stage_git(paths, &record, &source, intent, cancel, api, ttl_ms)?;
            let mut proposal = SourceProposal::unchanged(&record);
            // Acquisition owns the branch/path split. Finalize carries this
            // resolved path into the record, including legacy backfills.
            proposal.source_subpath = resolved_subpath(acquired.resolved_subpath);
            Ok(UpdateRequest {
                expected: record,
                proposal,
                bytes: UpdateBytes::Acquired {
                    staged,
                    revision: Some(acquired.revision),
                },
                repoint: false,
            })
        }
        // Imported/unknown were refused by admission above.
        _ => anyhow::bail!(not_refreshable(&record)),
    }
}

/// Acquire a Re-point's new git source for any Managed skill — `git`,
/// `local` or `imported` alike (the skill's current provenance is
/// irrelevant: the source is being replaced). The URL was validated by the
/// Re-point module. The skill is resolved in the new source by its existing
/// name, and the staged bytes must pass install's manifest gate. The new
/// source is carried only in the proposal: nothing is written until apply
/// settles it through finalize.
#[allow(clippy::too_many_arguments)]
pub(crate) fn acquire_git_repoint(
    paths: &InstallerPaths,
    store: &SkillStore,
    skill_id: &str,
    cancel: Option<&CancelToken>,
    api: &dyn GithubApi,
    ttl_ms: i64,
    url: &str,
    source: &GitSource,
) -> Result<UpdateRequest> {
    let record = require_record(store, skill_id)?;
    let intent = SkillIntent::ByName(Some(&record.name));
    let (staged, acquired) = stage_git(paths, &record, source, intent, cancel, api, ttl_ms)?;
    ensure_installable_skill_dir(staged.path())?;
    let proposal = SourceProposal {
        source_type: Provenance::Git.as_str().to_string(),
        source_ref: Some(url.to_string()),
        source_subpath: resolved_subpath(acquired.resolved_subpath),
    };
    Ok(UpdateRequest {
        expected: record,
        proposal,
        bytes: UpdateBytes::Acquired {
            staged,
            revision: Some(acquired.revision),
        },
        repoint: true,
    })
}

/// Acquire git bytes into a Staging dir beside the skill's central copy.
fn stage_git(
    paths: &InstallerPaths,
    record: &SkillRecord,
    source: &GitSource,
    intent: SkillIntent<'_>,
    cancel: Option<&CancelToken>,
    api: &dyn GithubApi,
    ttl_ms: i64,
) -> Result<(StagingDir, Acquired)> {
    let central_path = PathBuf::from(&record.central_path);
    let central_parent = central_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid central path"))?;
    // The central repo itself may be gone too (a Restore after the whole
    // library folder was lost); the staging dir needs its parent.
    ensure_central_repo(central_parent)?;

    // Build new content in a sibling staging dir; finalize swaps it in.
    let staged = StagingDir::new_in(central_parent);
    let acquired = acquire(
        &AcquireRequest {
            source,
            intent,
            dest: staged.path(),
            cache_dir: &paths.cache_dir,
            ttl_ms,
            cancel,
            allow_fast_path: true,
        },
        api,
    )?;
    Ok((staged, acquired))
}

/// The repository root is recorded as no subpath.
fn resolved_subpath(resolved: Option<String>) -> Option<String> {
    resolved.filter(|subpath| subpath != ".")
}

fn require_record(store: &SkillStore, skill_id: &str) -> Result<SkillRecord> {
    store.get_skill_by_id(skill_id)?.ok_or_else(|| {
        anyhow::anyhow!(SignalError::NotFound {
            kind: "skill".to_string(),
            id: skill_id.to_string(),
        })
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
