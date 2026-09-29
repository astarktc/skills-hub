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
//!   *plus* every Tool holding a variant byte-identical to the chosen one
//!   (the chosen variant's own Tool always among them), whether or not the
//!   policy names them — otherwise a deselected Tool would keep its original
//!   as an untracked duplicate (a real dir in one Tool with another Tool's
//!   symlink into it is the common shape). Identity is checked against the
//!   finalized central copy. Internal symlinks are not content: hashing ignores
//!   both their names and targets, and copying neither preserves nor follows
//!   them, so they do not make the chosen original diverge from its own copy.
//!   The sync batch rechecks the same-content
//!   policy before replacing a target; a stale plan never authorizes a
//!   force-overwrite. The Tools the policy did not name are reported
//!   as `forced_tools` so the UI can say why a deselected Tool received a
//!   link. A divergent sibling is neither force-included nor overwritten:
//!   it is left in place and reported once — as the pair's askable
//!   `TARGET_EXISTS` target row when its Tool is a target, else as a kept
//!   original, exactly as on the auto-sync-off path.
//! * **Auto-sync off**: the operator decides per run (`remove_originals`).
//!   When true, every original of the group (the chosen variant's own path
//!   included — it now lives in the central repo) is removed *only* when it
//!   is byte-identical to the finalized central copy; when false, nothing is
//!   touched and each identical original is reported `LeftInPlace`. Either
//!   way a divergent sibling is left in place and reported: sharing a name
//!   never proved it was the same skill.
//! * **Managed-name groups** (the plan's `name_is_managed`): never finalized
//!   — the name is taken. Without the operator's `replace` the group is
//!   `Skipped` with a typed reason and nothing is touched. With it, the
//!   Managed skill is synced to every Tool of the group through the global
//!   sync batch's same-content policy: an identical copy is replaced, a
//!   divergent one answers the pair's `TARGET_EXISTS` row — the askable row
//!   the frontend's one overwrite ask retries with an Overwrite override. A
//!   plan is never an authorization to force-overwrite.
//!
//! Continue-and-report: a group that fails admission or finalize is a
//! per-group `Failed` outcome and the rest of the batch proceeds; per-target
//! and per-original outcomes are report data. Only reading the store (the
//! plan) can fail the operation.

use std::path::{Path, PathBuf};

use anyhow::Result;

use super::content_identity::{self, Source};
use super::errors::{CommandError, SignalError};
use super::global_sync::{
    sync_skills_to_tools_unlocked, BatchPolicy, BatchSkill, BatchTargetOutcome,
};
use super::installer::{install_imported_skill, InstallerPaths};
use super::mutation_guard;
use super::onboarding::{
    build_onboarding_plan, OnboardingGroup, OnboardingScanScope, OnboardingVariant,
};
use super::skill_discovery::require_skill_md;
use super::skill_store::SkillStore;
use super::sync_engine::remove_path_any;
use super::tool_adapters::ensure_path_within_tool_dirs;

/// One name-group the operator chose to import, as the frontend states it.
#[derive(Clone, Debug)]
pub struct ImportSelection {
    pub group_name: String,
    /// The variant whose bytes become the Managed skill. Validated against
    /// the freshly built plan — an unknown path fails the group.
    pub chosen_path: PathBuf,
    /// Operator-supplied name; the group name is used when absent.
    pub name: Option<String>,
    /// For a managed-name group: replace the Tools' copies with the Managed
    /// skill. `false` is "no decision" — the group is skipped. Ignored for a
    /// group whose name is not managed.
    pub replace: bool,
}

/// The batch's policy: what happens to the originals.
#[derive(Clone, Debug)]
pub struct ImportPolicy {
    /// On: sync the imported skill to `tools`. Off: settle the originals
    /// per `remove_originals`.
    pub auto_sync: bool,
    /// Auto-sync off: remove the byte-identical originals (`true`) or leave
    /// them and report them `LeftInPlace` (`false`). Ignored when auto-sync
    /// is on — the sync batch takes the identical originals over.
    pub remove_originals: bool,
    /// Tools to sync to when `auto_sync` is on; `None` means every installed
    /// Tool. Ignored when `auto_sync` is off.
    pub tools: Option<Vec<String>>,
    /// The scope the presented plan was built with: the import re-derives
    /// the plan under the same scope, so the originals it takes over or
    /// removes are exactly the ones the operator reviewed.
    pub scan_scope: OnboardingScanScope,
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

/// What happened to one original directory.
#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum OriginalStatus {
    /// Byte-identical to the central copy (or already gone) — removed
    /// (auto-sync off, `remove_originals`).
    Removed,
    /// Byte-identical to the central copy and left where it was because the
    /// operator chose not to remove originals (auto-sync off).
    LeftInPlace,
    /// Content differs from the central copy, so it was left in place
    /// (either policy).
    KeptDivergent,
    /// The path was refused or could not be removed. Report data.
    Failed { error: CommandError },
}

#[derive(Debug, serde::Serialize, specta::Type)]
pub struct OriginalOutcome {
    pub path: PathBuf,
    pub tool: String,
    pub status: OriginalStatus,
}

#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ImportGroupStatus {
    Imported {
        skill_id: String,
        skill_name: String,
        /// Sync targets (auto-sync on); empty when auto-sync is off.
        targets: Vec<BatchTargetOutcome>,
        /// The Tools synced beyond the policy's Tools because they held a
        /// variant byte-identical to the chosen one (auto-sync on), in the
        /// order they were appended to the target set; empty when the
        /// policy already named every one of them or auto-sync is off.
        forced_tools: Vec<String>,
        /// Originals settled, per Tool: every variant still present when
        /// auto-sync is off (removed, left in place or kept divergent); only
        /// the divergent siblings (kept in place) when auto-sync is on — the
        /// identical ones are the `targets`.
        originals: Vec<OriginalOutcome>,
    },
    /// A managed-name group the operator chose to replace: the Managed
    /// skill was synced to every Tool holding a copy. A divergent copy is a
    /// `TARGET_EXISTS` target row — askable, never force-overwritten here.
    Replaced {
        skill_id: String,
        skill_name: String,
        targets: Vec<BatchTargetOutcome>,
    },
    /// Not imported by design; nothing of this group was touched.
    Skipped { reason: ImportSkipReason },
    /// Admission or finalize failed; nothing of this group was touched.
    Failed { error: CommandError },
}

/// Why a selected group was not imported, when that is not a failure.
#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum ImportSkipReason {
    /// A Managed skill already holds the group's name and the selection
    /// carried no `replace` decision.
    NameManaged { skill_id: String },
}

#[derive(Debug, serde::Serialize, specta::Type)]
pub struct ImportGroupOutcome {
    pub group_name: String,
    pub status: ImportGroupStatus,
}

#[derive(Debug, Default, serde::Serialize, specta::Type)]
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
    let plan = build_onboarding_plan(&paths.home, &paths.central_dir, store, &policy.scan_scope)?;

    let total = selections.len();
    let mut report = ImportReport::default();
    for (index, selection) in selections.iter().enumerate() {
        on_progress(ImportProgress {
            index: index + 1,
            total,
            group_name: &selection.group_name,
            phase: ImportPhase::Admitting,
        });

        let applying = ImportProgress {
            index: index + 1,
            total,
            group_name: &selection.group_name,
            phase: ImportPhase::Applying,
        };
        let status = match admit(&plan.groups, selection) {
            Ok(Admission::Import(group)) => {
                on_progress(applying);
                mutation_guard::serialized(|| {
                    apply_one_unlocked(paths, store, selection, group, policy, now)
                })
            }
            Ok(Admission::Replace { group, skill_id }) => {
                on_progress(applying);
                mutation_guard::serialized(|| {
                    replace_with_managed_unlocked(paths, store, group, skill_id, now)
                })
            }
            // Decided at admission: nothing to apply, no applying tick.
            Ok(Admission::Skip(reason)) => ImportGroupStatus::Skipped { reason },
            Err(error) => ImportGroupStatus::Failed {
                error: CommandError::from_anyhow(error),
            },
        };

        report.groups.push(ImportGroupOutcome {
            group_name: selection.group_name.clone(),
            status,
        });
    }
    Ok(report)
}

/// What admission decided for one selection.
enum Admission<'a> {
    /// Finalize the chosen variant as a new Managed skill.
    Import(&'a OnboardingGroup),
    /// The name is managed and the operator chose Replace: sync the Managed
    /// skill over the group's copies.
    Replace {
        group: &'a OnboardingGroup,
        skill_id: &'a str,
    },
    /// Nothing to apply.
    Skip(ImportSkipReason),
}

/// Resolve the selection against the freshly built plan and check the chosen
/// variant is admissible. Reads only.
fn admit<'a>(
    groups: &'a [OnboardingGroup],
    selection: &ImportSelection,
) -> Result<Admission<'a>, anyhow::Error> {
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
    // A managed name is decided here, never by finalize's collision. An
    // operator-supplied different name is an ordinary import (finalize still
    // refuses a name that is taken).
    let keeps_group_name = selection
        .name
        .as_deref()
        .is_none_or(|name| name == group.name);
    match group.managed_skill_id.as_deref() {
        Some(skill_id) if keeps_group_name && selection.replace => {
            Ok(Admission::Replace { group, skill_id })
        }
        Some(skill_id) if keeps_group_name => Ok(Admission::Skip(ImportSkipReason::NameManaged {
            skill_id: skill_id.to_string(),
        })),
        _ => Ok(Admission::Import(group)),
    }
}

/// Replace every copy in a managed-name group with the Managed skill: one
/// global sync batch over the group's Tools, same-content policy, no
/// override. An identical copy is replaced in place; a divergent one is the
/// pair's `TARGET_EXISTS` row (the frontend's overwrite ask retries it with
/// an Overwrite override through `sync_skills_to_tools`). The caller holds
/// the mutation guard.
fn replace_with_managed_unlocked(
    paths: &InstallerPaths,
    store: &SkillStore,
    group: &OnboardingGroup,
    skill_id: &str,
    now: i64,
) -> ImportGroupStatus {
    let record = match store.get_skill_by_id(skill_id) {
        Ok(Some(record)) => record,
        Ok(None) => {
            return ImportGroupStatus::Failed {
                error: CommandError::from_anyhow(anyhow::anyhow!(SignalError::NotFound {
                    kind: "skill".to_string(),
                    id: skill_id.to_string(),
                })),
            }
        }
        Err(error) => {
            return ImportGroupStatus::Failed {
                error: CommandError::from_anyhow(error),
            }
        }
    };
    let mut tools: Vec<String> = Vec::new();
    for variant in &group.variants {
        if !tools.contains(&variant.tool) {
            tools.push(variant.tool.clone());
        }
    }
    let skills = [BatchSkill {
        skill_id: record.id.clone(),
        skill_name: record.name.clone(),
        source_path: PathBuf::from(&record.central_path),
    }];
    let batch_policy = BatchPolicy {
        overwrite: false,
        overwrite_if_same_content: true,
        overrides: Vec::new(),
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
    ImportGroupStatus::Replaced {
        skill_id: record.id,
        skill_name: record.name,
        targets,
    }
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
    let found_in_tool = chosen_variant(group, selection).map(|variant| variant.tool.as_str());
    let installed = match install_imported_skill(
        paths,
        store,
        &selection.chosen_path,
        Some(name),
        found_in_tool,
    ) {
        Ok(result) => result,
        Err(error) => {
            return ImportGroupStatus::Failed {
                error: CommandError::from_anyhow(error),
            }
        }
    };

    let (targets, forced_tools, originals) = if policy.auto_sync {
        sync_imported_unlocked(paths, store, &installed, group, policy, now)
    } else {
        (
            Vec::new(),
            Vec::new(),
            group
                .variants
                .iter()
                .filter_map(|variant| {
                    if policy.remove_originals {
                        Some(settle_original(
                            &paths.home,
                            store,
                            &installed.skill_id,
                            &variant.path,
                            &variant.tool,
                        ))
                    } else {
                        leave_original(store, &installed.skill_id, &variant.path, &variant.tool)
                    }
                })
                .collect(),
        )
    };
    // Finalised, then an original could not be settled: that settles here as
    // the new skill's health (its targets' failures are their own `error`
    // rows).
    if let Some(error) = originals
        .iter()
        .find_map(|original| match &original.status {
            OriginalStatus::Failed { error } => Some(error),
            _ => None,
        })
    {
        super::skill_health::note_outcome(
            store,
            &installed.skill_id,
            super::skill_health::SkillOutcomeKind::ImportFailed,
            error,
        );
    }

    ImportGroupStatus::Imported {
        skill_id: installed.skill_id,
        skill_name: installed.name,
        targets,
        forced_tools,
        originals,
    }
}

/// Auto-sync on: fan the freshly imported skill out to the requested Tools
/// and every Tool holding a variant byte-identical to the chosen one. The
/// target set is `policy.tools ∪ {identical variants' Tools}`: each of those
/// Tools is synced using the same-content policy — never a force-overwrite —
/// so a deselected Tool does not keep an untracked identical duplicate.
/// A variant whose current content differs from the finalized central copy is not
/// touched: it is reported `KeptDivergent`, as the auto-sync-off path
/// reports it. Returns the outcomes, the Tools included beyond the policy,
/// and the divergent originals.
///
/// The identical Tools are appended *after* the policy's Tools so the
/// batch's shared-skills-dir dedupe keeps its caller-order semantics: a
/// forced Tool sharing its dir with a policy Tool is covered by that Tool's
/// record fan-out, exactly as before.
fn sync_imported_unlocked(
    paths: &InstallerPaths,
    store: &SkillStore,
    installed: &super::installer::InstallResult,
    group: &OnboardingGroup,
    policy: &ImportPolicy,
    now: i64,
) -> (Vec<BatchTargetOutcome>, Vec<String>, Vec<OriginalOutcome>) {
    // No per-import selection means the operator's recorded global selection,
    // not raw detection — the same rule the Refresh re-assert and the sync
    // fan-out follow (`settings::effective_global_tool_targets`). Detection is
    // only the fallback for a never-configured install.
    // A settings read that fails must fail **closed**: falling back to raw
    // detection here would write to every detected Tool — the exact defect
    // this rule exists to prevent — on an error path the operator never sees.
    // Writing nowhere is recoverable; writing everywhere is what they
    // reported as a bug.
    let mut tools = match policy.tools.clone() {
        Some(selected) => selected,
        None => {
            super::settings::effective_global_tool_targets(store, &paths.home).unwrap_or_default()
        }
    };
    let mut identical_tools: Vec<String> = Vec::new();
    let mut originals: Vec<OriginalOutcome> = Vec::new();
    for variant in &group.variants {
        if content_identity::same_content(
            Source::Managed {
                store,
                skill_id: &installed.skill_id,
            },
            &variant.path,
        ) {
            if !identical_tools.contains(&variant.tool) {
                identical_tools.push(variant.tool.clone());
            }
        } else {
            originals.push(OriginalOutcome {
                path: variant.path.clone(),
                tool: variant.tool.clone(),
                status: OriginalStatus::KeptDivergent,
            });
        }
    }
    let forced_tools: Vec<String> = identical_tools
        .iter()
        .filter(|tool_key| !tools.contains(tool_key))
        .cloned()
        .collect();
    tools.extend(forced_tools.iter().cloned());
    let skills = [BatchSkill {
        skill_id: installed.skill_id.clone(),
        skill_name: installed.name.clone(),
        source_path: installed.central_path.clone(),
    }];
    let batch_policy = BatchPolicy {
        overwrite: false,
        overwrite_if_same_content: true,
        overrides: Vec::new(),
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
    // A divergent sibling sitting where a target lands is that target's
    // `TARGET_EXISTS` row — the askable one (D12). Reported once, there:
    // it is not also a kept original.
    let collided: Vec<&Path> = targets
        .iter()
        .filter_map(|outcome| match &outcome.status {
            super::global_sync::BatchTargetStatus::Failed {
                error: CommandError::TargetExists { path, .. },
            } => Some(Path::new(path.as_str())),
            _ => None,
        })
        .collect();
    originals.retain(|original| !collided.contains(&original.path.as_path()));
    (targets, forced_tools, originals)
}

/// The group's variant the selection chose. `admit` proved it is one of
/// them; `None` is unreachable after admission and is passed through as
/// "unknown", never as an empty Tool.
fn chosen_variant<'a>(
    group: &'a OnboardingGroup,
    selection: &ImportSelection,
) -> Option<&'a OnboardingVariant> {
    group
        .variants
        .iter()
        .find(|variant| variant.path == selection.chosen_path)
}

/// Auto-sync off, originals kept: touch nothing and say what stays —
/// `LeftInPlace` when byte-identical to the central copy, `KeptDivergent`
/// otherwise. An original already gone is not reported: nothing is left.
fn leave_original(
    store: &SkillStore,
    skill_id: &str,
    path: &Path,
    tool: &str,
) -> Option<OriginalOutcome> {
    path.symlink_metadata().ok()?;
    let status = if content_identity::same_content(Source::Managed { store, skill_id }, path) {
        OriginalStatus::LeftInPlace
    } else {
        OriginalStatus::KeptDivergent
    };
    Some(OriginalOutcome {
        path: path.to_path_buf(),
        tool: tool.to_string(),
        status,
    })
}

/// Auto-sync off: remove one original, but only when it is byte-identical to
/// the central copy. The registry owns which paths may be deleted at all
/// ([`ensure_path_within_tool_dirs`]); its refusal is reported, never thrown.
fn settle_original(
    home: &Path,
    store: &SkillStore,
    skill_id: &str,
    path: &Path,
    tool: &str,
) -> OriginalOutcome {
    let status = match ensure_path_within_tool_dirs(home, path) {
        Err(error) => OriginalStatus::Failed {
            error: CommandError::from_anyhow(error),
        },
        Ok(()) if path.symlink_metadata().is_err() => OriginalStatus::Removed,
        Ok(()) if !content_identity::same_content(Source::Managed { store, skill_id }, path) => {
            OriginalStatus::KeptDivergent
        }
        Ok(()) => match remove_path_any(path) {
            Ok(()) => OriginalStatus::Removed,
            Err(error) => OriginalStatus::Failed {
                error: CommandError::from_anyhow(error),
            },
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
