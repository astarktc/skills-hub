//! The Add flow's one batch command (D12, D20): install several candidates of
//! one source — a GitHub listing or a local folder — and deploy each to the
//! chosen Tools, answering **one** report.
//!
//! Per candidate, in caller order:
//!
//! 1. **Acquire** outside the mutation guard (network / disk copy into a
//!    staging dir beside the central repo; `installer::stage_*`, which reach
//!    git bytes only through `git_acquisition::acquire`).
//! 2. **Finalize + first sync** inside one `mutation_guard::serialized`
//!    section: the staged bytes become a Managed skill, then the global sync
//!    batch's unlocked seam deploys it (`global_sync::sync_skills_to_tools_unlocked`)
//!    with the same-content policy and the request's per-pair decisions.
//!
//! A folder in a target's way is **collected, not asked**: it is the pair's
//! `TARGET_EXISTS` target row (no row recorded — D9), carrying the path, the
//! folder's modified time and `same_content`. The frontend raises one ask
//! over every collided row of the report and retries them through
//! `sync_skills_to_tools` with per-pair overrides — the skills exist by then,
//! so the retry is an ordinary sync batch and its rows replace the asked ones
//! in this report (one ask, one report shape). The request's own
//! `overrides` let a caller that already holds a decision (keyed by skill
//! name, since no id exists yet) settle it in the first pass.
//!
//! **Cancel** is honoured between candidates, after an acquisition and
//! again once the guard is held (a Cancel that arrived while the candidate
//! waited for another mutation), never inside finalize/sync: a candidate the batch did not reach, or whose
//! staged bytes were discarded because the operator cancelled, is
//! `Cancelled` (D10) — nothing of it was finalized (the staging dir's RAII
//! removes the bytes). A candidate already finalized completes its sync.
//!
//! Continue-and-report: a candidate that fails acquisition or finalize is
//! `Failed` and the rest proceed; target failures are report data. The batch
//! itself never errors.

use std::path::{Path, PathBuf};

use super::cancel_token::CancelToken;
use super::credentials::CredentialStore;
use super::errors::CommandError;
use super::git_acquisition::{GitSelection, GitSourceResolution, GithubApi, HttpGithubApi};
use super::global_sync::{
    sync_skills_to_tools_unlocked, BatchOverride, BatchPolicy, BatchSkill, BatchTargetOutcome,
    OverrideDecision,
};
use super::install_finalize::InstallResult;
use super::installer::{stage_git_selection, stage_local_selection, InstallerPaths, StagedInstall};
use super::mutation_guard;
use super::skill_store::SkillStore;

/// Where the candidates come from. Crosses the wire as itself:
/// `{ kind: "git", url, resolution }` or `{ kind: "local", root }`.
#[derive(Clone, Debug, serde::Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InstallSource {
    /// A GitHub repo URL and the listing's source decision (branch / path),
    /// reused so the install makes no second refs lookup.
    Git {
        url: String,
        resolution: Option<GitSourceResolution>,
    },
    /// A local folder the candidates' subpaths are relative to.
    Local { root: String },
}

/// One selected candidate.
#[derive(Clone, Debug, serde::Deserialize, specta::Type)]
pub struct InstallCandidate {
    /// The listing's subpath (`"."` for the source root).
    pub subpath: String,
    /// The operator's name for a single pick; `null` keeps the skill's own.
    pub name_override: Option<String>,
}

/// A decision taken before the first pass for one (skill, Tool) pair, keyed
/// by the skill's name (it has no id until it is installed). Applies to any
/// Tool sharing the named Tool's skills dir, like `BatchOverride`.
#[derive(Clone, Debug, serde::Deserialize, specta::Type)]
pub struct InstallOverride {
    pub skill_name: String,
    pub tool: String,
    pub decision: OverrideDecision,
}

/// The whole Add action.
#[derive(Clone, Debug, serde::Deserialize, specta::Type)]
pub struct InstallRequest {
    pub source: InstallSource,
    pub candidates: Vec<InstallCandidate>,
    /// Tools to deploy each installed skill to (the global sync batch's
    /// rules: installedness, shared-dir dedupe, Disabled gate).
    pub tools: Vec<String>,
    /// Replace a target whose content is identical to the skill's.
    pub overwrite_if_same_content: bool,
    pub overrides: Vec<InstallOverride>,
    /// Deploy at all (auto-sync on). `false` installs only.
    pub deploy: bool,
}

/// What happened to one candidate.
#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum InstallCandidateStatus {
    /// Finalized as a Managed skill; `targets` is its first sync (empty when
    /// the request did not deploy or named no Tools). A `TARGET_EXISTS` row
    /// is the askable collision.
    Installed {
        skill: InstallResult,
        targets: Vec<BatchTargetOutcome>,
    },
    /// Acquisition or finalize failed; nothing of this candidate remains.
    Failed { error: CommandError },
    /// The operator cancelled before this candidate was finalized; nothing
    /// of it remains (D10 — its own kind, never a failure).
    Cancelled,
}

#[derive(Debug, serde::Serialize, specta::Type)]
pub struct InstallCandidateOutcome {
    pub subpath: String,
    /// The installed skill's name, or the name the candidate was asked
    /// under when it was not installed.
    pub name: String,
    pub status: InstallCandidateStatus,
}

#[derive(Debug, Default, serde::Serialize, specta::Type)]
pub struct InstallBatchReport {
    pub candidates: Vec<InstallCandidateOutcome>,
}

/// Progress tick emitted before each candidate ("Adding (i/n) name").
pub struct InstallProgress<'a> {
    /// 1-based.
    pub index: usize,
    pub total: usize,
    pub name: &'a str,
}

/// Install a selection as one batch (see the module doc).
///
/// Mutation entry point: each candidate's finalize + first sync is
/// serialised against every other Sync-target mutation; acquisition is
/// deliberately outside the guard.
pub fn install_selection(
    paths: &InstallerPaths,
    store: &SkillStore,
    credentials: &dyn CredentialStore,
    request: &InstallRequest,
    cancel: Option<&CancelToken>,
    now: i64,
    on_progress: impl FnMut(InstallProgress),
) -> InstallBatchReport {
    install_selection_with(
        paths,
        store,
        request,
        &HttpGithubApi::new(super::settings::github_token_or_none(credentials)),
        cancel,
        now,
        on_progress,
    )
}

/// [`install_selection`] with the GitHub adapter injected (tests).
pub(crate) fn install_selection_with(
    paths: &InstallerPaths,
    store: &SkillStore,
    request: &InstallRequest,
    api: &dyn GithubApi,
    cancel: Option<&CancelToken>,
    now: i64,
    mut on_progress: impl FnMut(InstallProgress),
) -> InstallBatchReport {
    let cancelled = || cancel.is_some_and(|token| token.is_cancelled());
    let total = request.candidates.len();
    let mut report = InstallBatchReport::default();
    for (index, candidate) in request.candidates.iter().enumerate() {
        let label = candidate_label(&request.source, candidate);
        let outcome = |name: String, status| InstallCandidateOutcome {
            subpath: candidate.subpath.clone(),
            name,
            status,
        };
        if cancelled() {
            report
                .candidates
                .push(outcome(label, InstallCandidateStatus::Cancelled));
            continue;
        }
        on_progress(InstallProgress {
            index: index + 1,
            total,
            name: &label,
        });

        let staged = match stage(paths, store, &request.source, candidate, api, cancel) {
            Ok(staged) => staged,
            Err(error) => {
                let status = match CommandError::from_anyhow(error) {
                    CommandError::Cancelled => InstallCandidateStatus::Cancelled,
                    error => InstallCandidateStatus::Failed { error },
                };
                report.candidates.push(outcome(label, status));
                continue;
            }
        };
        // Cancelled while acquiring: the staged bytes are dropped (RAII),
        // never finalized.
        if cancelled() {
            drop(staged);
            report
                .candidates
                .push(outcome(label, InstallCandidateStatus::Cancelled));
            continue;
        }

        // Checked again once the guard is held: a Cancel pressed while this
        // candidate waited for another mutation wins before `finalize`
        // consumes the staging (dropped here, RAII). Once finalize starts,
        // the candidate completes its first sync.
        let settled = mutation_guard::serialized(|| {
            if cancelled() {
                drop(staged);
                return Ok(None);
            }
            settle_unlocked(paths, store, request, staged, now).map(Some)
        });
        report.candidates.push(match settled {
            Ok(None) => outcome(label, InstallCandidateStatus::Cancelled),
            Ok(Some((skill, targets))) => outcome(
                skill.name.clone(),
                InstallCandidateStatus::Installed { skill, targets },
            ),
            Err(error) => outcome(
                label,
                InstallCandidateStatus::Failed {
                    error: CommandError::from_anyhow(error),
                },
            ),
        });
    }
    report
}

/// The acquire half for one candidate.
fn stage(
    paths: &InstallerPaths,
    store: &SkillStore,
    source: &InstallSource,
    candidate: &InstallCandidate,
    api: &dyn GithubApi,
    cancel: Option<&CancelToken>,
) -> anyhow::Result<StagedInstall> {
    match source {
        InstallSource::Git { url, resolution } => stage_git_selection(
            paths,
            store,
            url,
            GitSelection {
                subpath: &candidate.subpath,
                resolution: resolution.as_ref(),
            },
            candidate.name_override.clone(),
            cancel,
            api,
        ),
        InstallSource::Local { root } => stage_local_selection(
            paths,
            Path::new(root),
            &candidate.subpath,
            candidate.name_override.clone(),
        ),
    }
}

/// Finalize + first sync, under the guard the caller holds. A finalize
/// failure is the candidate's; a sync failure is report data on its rows.
fn settle_unlocked(
    paths: &InstallerPaths,
    store: &SkillStore,
    request: &InstallRequest,
    staged: StagedInstall,
    now: i64,
) -> anyhow::Result<(InstallResult, Vec<BatchTargetOutcome>)> {
    let skill = staged.finalize(store)?;
    let targets = if request.deploy && !request.tools.is_empty() {
        let policy = BatchPolicy {
            overwrite: false,
            overwrite_if_same_content: request.overwrite_if_same_content,
            overrides: request
                .overrides
                .iter()
                .filter(|o| o.skill_name == skill.name)
                .map(|o| BatchOverride {
                    skill_id: skill.skill_id.clone(),
                    tool_key: o.tool.clone(),
                    decision: o.decision,
                })
                .collect(),
        };
        sync_skills_to_tools_unlocked(
            &paths.home,
            store,
            &[BatchSkill {
                skill_id: skill.skill_id.clone(),
                skill_name: skill.name.clone(),
                source_path: skill.central_path.clone(),
            }],
            &request.tools,
            &policy,
            now,
            |_| {},
        )
    } else {
        Vec::new()
    };
    Ok((skill, targets))
}

/// What a candidate is called before it is installed: the operator's name,
/// else the subpath's last segment, else the source's own name.
fn candidate_label(source: &InstallSource, candidate: &InstallCandidate) -> String {
    if let Some(name) = candidate.name_override.as_deref().filter(|n| !n.is_empty()) {
        return name.to_string();
    }
    let last = |path: &str| {
        path.trim_end_matches('/')
            .rsplit('/')
            .next()
            .map(|s| s.trim_end_matches(".git").to_string())
            .filter(|s| !s.is_empty())
    };
    if candidate.subpath != "." {
        if let Some(segment) = last(&candidate.subpath) {
            return segment;
        }
    }
    match source {
        InstallSource::Git { url, .. } => last(url),
        InstallSource::Local { root } => PathBuf::from(root)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned()),
    }
    .unwrap_or_else(|| candidate.subpath.clone())
}

#[cfg(test)]
#[path = "tests/install_batch.rs"]
mod tests;
