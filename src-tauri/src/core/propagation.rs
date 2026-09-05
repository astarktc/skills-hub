//! Propagation: bringing every Sync target of one Managed skill into line
//! after its central copy changed.
//!
//! One module spans both scopes — the skill's global target rows and its
//! project assignment rows — because "the bytes changed, make every target
//! match" is one rule, not two. Propagation reads its own rows: callers hand
//! it `(store, paths, skill_id, content_hash, now)` and nothing else.
//!
//! Three decisions live here and nowhere else:
//!
//! 1. **Does this target need new bytes?** Only when its Sync mode can drift
//!    (a copy) or its Tool cannot consume a link at all
//!    ([`needs_new_bytes`], the one `force_copy` predicate). A link already
//!    follows the central copy, so it is reported
//!    [`PropagationSkip::LinkFollowsSource`] and left untouched.
//! 2. **How do the bytes get there?** Only through
//!    [`sync_engine::sync_dir_for_tool_with_overwrite`], the capability-aware
//!    entry point the batch engines use. It may re-materialise a drifting
//!    copy as a link on a symlink-capable Tool; the row records the mode
//!    actually used, so it stays truthful (and can no longer drift).
//! 3. **What does a target's outcome mean?** Every target resolves to
//!    synced / skipped / failed as *report data*
//!    (continue-and-report): one target's failure never fails the operation.
//!    Only reading the rows can fail the operation.
//!
//! Row settlement goes through the typed transitions on both tables
//! ([`TargetTransition`], [`AssignmentTransition`]) — never a hand-built
//! record literal.
//!
//! Unlocked internal seam: callers reach it through an entry point that has
//! already taken the mutation guard (see `mutation_guard`).

use std::path::{Path, PathBuf};

use anyhow::Result;

use super::errors::SignalError;
use super::installer::InstallerPaths;
use super::skill_store::{
    AssignmentTransition, ProjectSkillAssignmentRecord, SkillStore, SkillTargetRecord,
    TargetTransition,
};
use super::sync_engine;
use super::sync_status::SyncMode;
use super::tool_adapters::{
    adapter_by_key, adapters_sharing_skills_dir, is_installed_in, ToolAdapter,
};

/// Which Sync target an outcome is about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropagationScope {
    Global { tool: String },
    Project { project_id: String, tool: String },
}

/// Why a target needed no work. Not a failure: skipping is the correct
/// outcome for a link, an uninstalled Tool, or a project that is not there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropagationSkip {
    /// The target is a link (or junction) into the central copy, which was
    /// just refreshed in place — the target is already current.
    LinkFollowsSource,
    /// The Tool is no longer installed for this operator.
    ToolNotInstalled { tool: String },
    /// The row names a tool key the registry does not know.
    UnknownTool { tool: String },
    /// The project row is gone, or its directory no longer exists on disk.
    ProjectUnavailable { project_id: String },
}

#[derive(Debug)]
pub enum PropagationStatus {
    Synced { mode_used: SyncMode },
    Skipped { reason: PropagationSkip },
    Failed { error: anyhow::Error },
}

#[derive(Debug)]
pub struct PropagationOutcome {
    pub scope: PropagationScope,
    pub status: PropagationStatus,
}

/// Every Sync target of one Managed skill, with what happened to it.
#[derive(Debug, Default)]
pub struct PropagationReport {
    pub targets: Vec<PropagationOutcome>,
}

/// The one `force_copy` predicate: a target needs its bytes written again
/// when its Sync mode can drift (a copy) or its Tool cannot consume a link.
pub(crate) fn needs_new_bytes(mode: SyncMode, adapter: &ToolAdapter) -> bool {
    mode.can_drift() || !adapter.supports_symlink
}

/// Re-materialise every Sync target of `skill_id` from its central copy.
///
/// `content_hash` is the freshly finalized central hash, recorded on copies
/// (only copies can drift, so links record none). Unlocked internal seam.
pub(crate) fn propagate_unlocked(
    store: &SkillStore,
    paths: &InstallerPaths,
    skill_id: &str,
    content_hash: Option<&str>,
    now: i64,
) -> Result<PropagationReport> {
    let skill = store.get_skill_by_id(skill_id)?.ok_or_else(|| {
        anyhow::anyhow!(SignalError::NotFound {
            kind: "skill".to_string(),
            id: skill_id.to_string(),
        })
    })?;
    let central_path = PathBuf::from(&skill.central_path);

    let mut report = PropagationReport::default();
    propagate_global_rows(
        store,
        &paths.home,
        &central_path,
        skill_id,
        now,
        &mut report,
    )?;
    propagate_project_rows(
        store,
        &central_path,
        skill_id,
        &skill.name,
        content_hash,
        now,
        &mut report,
    )?;
    Ok(report)
}

/// The typed condition for "the bytes we are supposed to propagate are not
/// there" — raised per target as report data, never as prose.
fn missing_source(central_path: &Path) -> anyhow::Error {
    anyhow::anyhow!(SignalError::InvalidPath {
        path: central_path.to_string_lossy().into_owned(),
        reason: "missing".to_string(),
    })
}

// ---------------------------------------------------------------------------
// Global scope
// ---------------------------------------------------------------------------

/// Global target rows, handled one **shared skills dir group at a time**:
/// tools sharing a global skills dir share one artifact, so the bytes are
/// written once and every member row is settled.
fn propagate_global_rows(
    store: &SkillStore,
    home: &Path,
    central_path: &Path,
    skill_id: &str,
    now: i64,
    report: &mut PropagationReport,
) -> Result<()> {
    let rows = store.list_skill_targets(skill_id)?;
    let mut handled: Vec<String> = Vec::new();

    for row in &rows {
        if handled.contains(&row.tool) {
            continue;
        }
        handled.push(row.tool.clone());

        let Some(adapter) = adapter_by_key(&row.tool) else {
            report.targets.push(skipped_global(
                &row.tool,
                PropagationSkip::UnknownTool {
                    tool: row.tool.clone(),
                },
            ));
            continue;
        };

        // The group: this skill's rows whose tools share the global skills
        // dir (the dir, not the key, is a target's identity). Which tools
        // those are is the registry's answer; the rows are matched by key.
        let sharing: Vec<&str> = adapters_sharing_skills_dir(adapter)
            .into_iter()
            .map(ToolAdapter::key)
            .collect();
        let group: Vec<(&SkillTargetRecord, &'static ToolAdapter)> = rows
            .iter()
            .filter(|r| sharing.contains(&r.tool.as_str()))
            .filter_map(|r| adapter_by_key(&r.tool).map(|a| (r, a)))
            .collect();
        for (member, _) in &group {
            if !handled.contains(&member.tool) {
                handled.push(member.tool.clone());
            }
        }

        let (installed, absent): (Vec<_>, Vec<_>) = group
            .into_iter()
            .partition(|(_, a)| is_installed_in(home, a));
        for (member, _) in &absent {
            report.targets.push(skipped_global(
                &member.tool,
                PropagationSkip::ToolNotInstalled {
                    tool: member.tool.clone(),
                },
            ));
        }
        let Some((driver, _)) = installed.first() else {
            continue;
        };

        if !installed
            .iter()
            .any(|(member, a)| needs_new_bytes(member.mode, a))
        {
            for (member, _) in &installed {
                report.targets.push(skipped_global(
                    &member.tool,
                    PropagationSkip::LinkFollowsSource,
                ));
            }
            continue;
        }

        // The group's most restrictive member decides how the shared
        // artifact is written: one copy-only Tool makes the whole dir a copy.
        let driving_adapter = installed
            .iter()
            .find(|(_, a)| !a.supports_symlink)
            .map(|(_, a)| *a)
            .unwrap_or(adapter);

        let source_missing = !central_path.is_dir();
        let result = if source_missing {
            Err(missing_source(central_path))
        } else {
            sync_engine::sync_dir_for_tool_with_overwrite(
                driving_adapter,
                central_path,
                Path::new(&driver.target_path),
                true,
            )
        };

        match result {
            Ok(outcome) => {
                let target_path = outcome.target_path.to_string_lossy().to_string();
                for (member, _) in &installed {
                    store.transition_skill_target(
                        &member.id,
                        TargetTransition::SyncCompleted {
                            mode: outcome.mode_used,
                            target_path: &target_path,
                            synced_at: now,
                        },
                    )?;
                    report.targets.push(PropagationOutcome {
                        scope: PropagationScope::Global {
                            tool: member.tool.clone(),
                        },
                        status: PropagationStatus::Synced {
                            mode_used: outcome.mode_used,
                        },
                    });
                }
            }
            Err(error) => {
                let detail = format!("{:#}", error);
                // Each member row is its own Sync target, so each one is
                // settled and reported as failed. A missing central copy is
                // the same typed condition for every row; a sync failure's
                // full chain belongs to the row that was attempted, and the
                // rest carry its diagnostic text.
                let mut original = Some(error);
                for (member, _) in &installed {
                    store.transition_skill_target(
                        &member.id,
                        TargetTransition::SyncFailed { error: &detail },
                    )?;
                    let error = if source_missing {
                        missing_source(central_path)
                    } else {
                        original
                            .take()
                            .unwrap_or_else(|| anyhow::anyhow!("{}", detail))
                    };
                    report.targets.push(PropagationOutcome {
                        scope: PropagationScope::Global {
                            tool: member.tool.clone(),
                        },
                        status: PropagationStatus::Failed { error },
                    });
                }
            }
        }
    }
    Ok(())
}

fn skipped_global(tool: &str, reason: PropagationSkip) -> PropagationOutcome {
    PropagationOutcome {
        scope: PropagationScope::Global {
            tool: tool.to_string(),
        },
        status: PropagationStatus::Skipped { reason },
    }
}

// ---------------------------------------------------------------------------
// Project scope
// ---------------------------------------------------------------------------

fn propagate_project_rows(
    store: &SkillStore,
    central_path: &Path,
    skill_id: &str,
    skill_name: &str,
    content_hash: Option<&str>,
    now: i64,
    report: &mut PropagationReport,
) -> Result<()> {
    for assignment in store.list_project_skill_assignments_by_skill(skill_id)? {
        let scope = PropagationScope::Project {
            project_id: assignment.project_id.clone(),
            tool: assignment.tool.clone(),
        };
        let status = propagate_one_assignment(
            store,
            central_path,
            skill_name,
            content_hash,
            now,
            &assignment,
        )?;
        report.targets.push(PropagationOutcome { scope, status });
    }
    Ok(())
}

fn propagate_one_assignment(
    store: &SkillStore,
    central_path: &Path,
    skill_name: &str,
    content_hash: Option<&str>,
    now: i64,
    assignment: &ProjectSkillAssignmentRecord,
) -> Result<PropagationStatus> {
    let Some(adapter) = adapter_by_key(&assignment.tool) else {
        return Ok(PropagationStatus::Skipped {
            reason: PropagationSkip::UnknownTool {
                tool: assignment.tool.clone(),
            },
        });
    };
    let unavailable = PropagationSkip::ProjectUnavailable {
        project_id: assignment.project_id.clone(),
    };
    let Some(project) = store.get_project_by_id(&assignment.project_id)? else {
        return Ok(PropagationStatus::Skipped {
            reason: unavailable,
        });
    };
    let project_path = PathBuf::from(&project.path);
    if !project_path.exists() {
        return Ok(PropagationStatus::Skipped {
            reason: unavailable,
        });
    }
    if !needs_new_bytes(assignment.mode, adapter) {
        return Ok(PropagationStatus::Skipped {
            reason: PropagationSkip::LinkFollowsSource,
        });
    }

    let target =
        super::project_sync::resolve_project_sync_target(&project_path, adapter, skill_name);
    let result = if central_path.is_dir() {
        sync_engine::sync_dir_for_tool_with_overwrite(adapter, central_path, &target, true)
    } else {
        Err(missing_source(central_path))
    };

    match result {
        Ok(outcome) => {
            // Only copies can drift, so only copies record a hash.
            let hash = if outcome.mode_used.can_drift() {
                content_hash
            } else {
                None
            };
            store.transition_assignment(
                &assignment.id,
                AssignmentTransition::SyncCompleted {
                    mode: outcome.mode_used,
                    synced_at: now,
                    content_hash: hash,
                },
            )?;
            Ok(PropagationStatus::Synced {
                mode_used: outcome.mode_used,
            })
        }
        Err(error) => {
            let detail = format!("{:#}", error);
            store.transition_assignment(
                &assignment.id,
                AssignmentTransition::SyncFailed { error: &detail },
            )?;
            Ok(PropagationStatus::Failed { error })
        }
    }
}

#[cfg(test)]
#[path = "tests/propagation.rs"]
mod tests;
