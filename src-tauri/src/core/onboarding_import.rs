//! Onboarding import: adopting skills that already live in a Tool's skills
//! directory as Managed skills — one backend operation, one report.
//!
//! The operator picks one variant per name-group (`ImportSelection`) and the
//! auto-sync policy (`ImportPolicy`); everything else is decided here:
//!
//! * **Which paths a group owns** is re-read from a fresh
//!   [`build_onboarding_plan`], never taken from the caller: a stale UI must
//!   not be able to name a path that is no longer part of the group (and get
//!   it deleted). The selection carries only `(group_name, chosen_path)`.
//! * **Admission** (does the chosen variant look like a skill?) runs outside
//!   the mutation guard — it only reads.
//! * **Apply** runs under [`mutation_guard::serialized`], one hold *per
//!   group* rather than one for the whole batch, so other mutations are not
//!   blocked for the length of the run. Inside the guard only the unlocked
//!   seams are used (`sync_skills_to_tools_unlocked`, `remove_path_any`) —
//!   an entry point never calls another entry point.
//! * **Auto-sync on**: the finalized skill is synced to the requested Tools
//!   *plus* the chosen variant's own Tool (the source Tool), whether or not
//!   the policy names it — otherwise a deselected source Tool would keep its
//!   original as an untracked copy. The source Tool carries a
//!   force-overwrite override — that copy *is* the import source and its
//!   bytes are already in the central repo, so replacing it in place with
//!   the Sync target is safe. When the policy did not name the source Tool,
//!   the group reports it as `forced_source_tool` so the UI can say why a
//!   deselected Tool received a link.
//! * **Auto-sync off**: every original of the group (the chosen variant's own
//!   path included — it now lives in the central repo) is removed *only* when
//!   it is byte-identical to the finalized central copy. A divergent sibling
//!   is left in place and reported: sharing a name never proved it was the
//!   same skill.
//!
//! Continue-and-report: a group that fails admission or finalize is a
//! per-group `Failed` outcome and the rest of the batch proceeds; per-target
//! and per-original outcomes are report data. Only reading the store (the
//! plan) can fail the operation.

use std::path::{Path, PathBuf};

use anyhow::Result;

use super::errors::SignalError;
use super::global_sync::{
    sync_skills_to_tools_unlocked, target_has_same_content, BatchOverride, BatchPolicy, BatchSkill,
    BatchTargetOutcome,
};
use super::installer::{install_local_skill, InstallerPaths};
use super::mutation_guard;
use super::onboarding::{build_onboarding_plan, OnboardingGroup};
use super::skill_discovery::require_skill_md;
use super::skill_store::SkillStore;
use super::sync_engine::remove_path_any;
use super::tool_adapters::{ensure_path_within_tool_dirs, global_tool_entries, installed_keys};

/// One name-group the operator chose to import, as the frontend states it.
#[derive(Clone, Debug)]
pub struct ImportSelection {
    pub group_name: String,
    /// The variant whose bytes become the Managed skill. Validated against
    /// the freshly built plan — an unknown path fails the group.
    pub chosen_path: PathBuf,
    /// Operator-supplied name; the group name is used when absent.
    pub name: Option<String>,
}

/// The batch's policy: what happens to the originals.
#[derive(Clone, Debug, Default)]
pub struct ImportPolicy {
    /// On: sync the imported skill to `tools`. Off: remove the originals.
    pub auto_sync: bool,
    /// Tools to sync to when `auto_sync` is on; `None` means every installed
    /// Tool. Ignored when `auto_sync` is off.
    pub tools: Option<Vec<String>>,
}

/// Which half of a group's import a progress tick is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportPhase {
    /// Resolving the group and validating the chosen variant (unlocked).
    Admitting,
    /// Finalize plus sync or original removal (under the mutation guard).
    Applying,
}

/// Progress tick emitted before each phase of each group.
pub struct ImportProgress<'a> {
    /// 1-based index over selections.
    pub index: usize,
    pub total: usize,
    pub group_name: &'a str,
    pub phase: ImportPhase,
}

/// What happened to one original directory (auto-sync off).
#[derive(Debug)]
pub enum OriginalStatus {
    /// Byte-identical to the central copy (or already gone) — removed.
    Removed,
    /// Content differs from the central copy, so it was left in place.
    KeptDivergent,
    /// The path was refused or could not be removed. Report data.
    Failed { error: anyhow::Error },
}

#[derive(Debug)]
pub struct OriginalOutcome {
    pub path: PathBuf,
    pub tool: String,
    pub status: OriginalStatus,
}

#[derive(Debug)]
pub enum ImportGroupStatus {
    Imported {
        skill_id: String,
        skill_name: String,
        /// Sync targets (auto-sync on); empty when auto-sync is off.
        targets: Vec<BatchTargetOutcome>,
        /// The source Tool's key when it was synced beyond the policy's
        /// Tools (auto-sync on, source Tool deselected); `None` when the
        /// policy already named it or auto-sync is off.
        forced_source_tool: Option<String>,
        /// Originals settled (auto-sync off); empty when auto-sync is on.
        originals: Vec<OriginalOutcome>,
    },
    /// Admission or finalize failed; nothing of this group was touched.
    Failed { error: anyhow::Error },
}

#[derive(Debug)]
pub struct ImportGroupOutcome {
    pub group_name: String,
    pub status: ImportGroupStatus,
}

#[derive(Debug, Default)]
pub struct ImportReport {
    pub groups: Vec<ImportGroupOutcome>,
}

/// Import the operator's onboarding selections.
///
/// Mutation entry point: each group's apply step is serialised against every
/// other Sync-target mutation; admission is deliberately outside the guard.
pub fn import_onboarding_selection(
    paths: &InstallerPaths,
    store: &SkillStore,
    selections: &[ImportSelection],
    policy: &ImportPolicy,
    now: i64,
    mut on_progress: impl FnMut(ImportProgress),
) -> Result<ImportReport> {
    // The authority on which paths a group owns. Reading it is the only
    // failure that fails the whole operation.
    let plan = build_onboarding_plan(&paths.home, &paths.central_dir, store)?;

    let total = selections.len();
    let mut report = ImportReport::default();
    for (index, selection) in selections.iter().enumerate() {
        on_progress(ImportProgress {
            index: index + 1,
            total,
            group_name: &selection.group_name,
            phase: ImportPhase::Admitting,
        });

        let status = match admit(&plan.groups, selection) {
            Ok(group) => {
                on_progress(ImportProgress {
                    index: index + 1,
                    total,
                    group_name: &selection.group_name,
                    phase: ImportPhase::Applying,
                });
                mutation_guard::serialized(|| {
                    apply_one_unlocked(paths, store, selection, group, policy, now)
                })
            }
            Err(error) => ImportGroupStatus::Failed { error },
        };

        report.groups.push(ImportGroupOutcome {
            group_name: selection.group_name.clone(),
            status,
        });
    }
    Ok(report)
}

/// Resolve the selection against the freshly built plan and check the chosen
/// variant is admissible. Reads only.
fn admit<'a>(
    groups: &'a [OnboardingGroup],
    selection: &ImportSelection,
) -> Result<&'a OnboardingGroup, anyhow::Error> {
    let group = groups
        .iter()
        .find(|group| group.name == selection.group_name)
        .ok_or_else(|| {
            anyhow::anyhow!(SignalError::NotFound {
                kind: "onboarding_group".to_string(),
                id: selection.group_name.clone(),
            })
        })?;
    if !group
        .variants
        .iter()
        .any(|variant| variant.path == selection.chosen_path)
    {
        anyhow::bail!(SignalError::NotFound {
            kind: "onboarding_variant".to_string(),
            id: selection.chosen_path.to_string_lossy().into_owned(),
        });
    }
    // Skill discovery owns the admission rule: no `SKILL.md`, no skill.
    require_skill_md(&selection.chosen_path)?;
    Ok(group)
}

/// Finalize one group's chosen variant and settle its originals. The caller
/// holds the mutation guard.
fn apply_one_unlocked(
    paths: &InstallerPaths,
    store: &SkillStore,
    selection: &ImportSelection,
    group: &OnboardingGroup,
    policy: &ImportPolicy,
    now: i64,
) -> ImportGroupStatus {
    let name = selection.name.clone().unwrap_or_else(|| group.name.clone());
    let installed = match install_local_skill(paths, store, &selection.chosen_path, Some(name)) {
        Ok(result) => result,
        Err(error) => return ImportGroupStatus::Failed { error },
    };

    let (targets, forced_source_tool, originals) = if policy.auto_sync {
        let (targets, forced_source_tool) =
            sync_imported_unlocked(paths, store, &installed, group, selection, policy, now);
        (targets, forced_source_tool, Vec::new())
    } else {
        (
            Vec::new(),
            None,
            group
                .variants
                .iter()
                .map(|variant| {
                    settle_original(
                        &paths.home,
                        &installed.central_path,
                        &variant.path,
                        &variant.tool,
                    )
                })
                .collect(),
        )
    };

    ImportGroupStatus::Imported {
        skill_id: installed.skill_id,
        skill_name: installed.name,
        targets,
        forced_source_tool,
        originals,
    }
}

/// Auto-sync on: fan the freshly imported skill out to the requested Tools
/// and the source Tool. The target set is `policy.tools ∪ {source Tool}`:
/// the chosen variant's own Tool is always synced and force-overwritten —
/// the original at that path *is* the source, and its bytes are already in
/// the central repo — so a deselected source Tool never keeps an untracked
/// copy. Returns the outcomes plus the source Tool's key when it was
/// included beyond the policy.
///
/// The source Tool is appended *after* the policy's Tools so the batch's
/// shared-skills-dir dedupe keeps its caller-order semantics: a source Tool
/// sharing its dir with a policy Tool is covered by that Tool's record
/// fan-out, exactly as before.
fn sync_imported_unlocked(
    paths: &InstallerPaths,
    store: &SkillStore,
    installed: &super::installer::InstallResult,
    group: &OnboardingGroup,
    selection: &ImportSelection,
    policy: &ImportPolicy,
    now: i64,
) -> (Vec<BatchTargetOutcome>, Option<String>) {
    let mut tools = policy
        .tools
        .clone()
        .unwrap_or_else(|| installed_keys(&global_tool_entries(&paths.home)));
    let source_tool = group
        .variants
        .iter()
        .find(|variant| variant.path == selection.chosen_path)
        .map(|variant| variant.tool.clone());
    let forced_source_tool = source_tool
        .clone()
        .filter(|tool_key| !tools.contains(tool_key));
    tools.extend(forced_source_tool.clone());
    let overrides = source_tool
        .map(|tool_key| {
            vec![BatchOverride {
                skill_id: installed.skill_id.clone(),
                tool_key,
                overwrite: true,
            }]
        })
        .unwrap_or_default();
    let skills = [BatchSkill {
        skill_id: installed.skill_id.clone(),
        skill_name: installed.name.clone(),
        source_path: installed.central_path.clone(),
    }];
    let batch_policy = BatchPolicy {
        overwrite: false,
        overwrite_if_same_content: true,
        overrides,
    };
    let targets = sync_skills_to_tools_unlocked(
        &paths.home,
        store,
        &skills,
        &tools,
        &batch_policy,
        now,
        |_| {},
    );
    (targets, forced_source_tool)
}

/// Auto-sync off: remove one original, but only when it is byte-identical to
/// the central copy. The registry owns which paths may be deleted at all
/// ([`ensure_path_within_tool_dirs`]); its refusal is reported, never thrown.
fn settle_original(home: &Path, central: &Path, path: &Path, tool: &str) -> OriginalOutcome {
    let status = match ensure_path_within_tool_dirs(home, path) {
        Err(error) => OriginalStatus::Failed { error },
        Ok(()) if path.symlink_metadata().is_err() => OriginalStatus::Removed,
        Ok(()) if !target_has_same_content(central, path) => OriginalStatus::KeptDivergent,
        Ok(()) => match remove_path_any(path) {
            Ok(()) => OriginalStatus::Removed,
            Err(error) => OriginalStatus::Failed { error },
        },
    };
    OriginalOutcome {
        path: path.to_path_buf(),
        tool: tool.to_string(),
        status,
    }
}

#[cfg(test)]
#[path = "tests/onboarding_import.rs"]
mod tests;
