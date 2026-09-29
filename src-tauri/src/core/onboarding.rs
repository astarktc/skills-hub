use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Serialize;
use specta::Type;

use super::content_identity::{self, Source};
use super::skill_discovery::InvalidManifestReason;
use super::skill_store::SkillStore;
use super::tool_adapters::{
    default_tool_adapters, is_installed_in, scan_tool_dir, skills_dir_in, DetectedSkill,
    ToolAdapter,
};

#[derive(Clone, Debug, Serialize, Type)]
pub struct OnboardingVariant {
    pub tool: String,
    pub name: String,
    pub path: PathBuf,
    pub fingerprint: Option<String>,
    pub is_link: bool,
    pub link_target: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Type)]
pub struct OnboardingGroup {
    pub name: String,
    pub variants: Vec<OnboardingVariant>,
    pub has_conflict: bool,
    /// A Managed skill already carries this name: importing the group would
    /// collide. The review offers "Replace this Tool's copy with the managed
    /// skill" (the selection's `replace`) or "Leave" instead of an import.
    pub name_is_managed: bool,
    /// The Managed skill holding the name; `Some` exactly when
    /// `name_is_managed`.
    pub managed_skill_id: Option<String>,
}

/// A directory in a scanned Tool's skills dir that is not an importable
/// skill: listed so the review can show it disabled, with the reason.
#[derive(Clone, Debug, Serialize, Type)]
pub struct OnboardingInvalidEntry {
    pub tool: String,
    pub name: String,
    pub path: PathBuf,
    pub reason: InvalidManifestReason,
}

#[derive(Clone, Debug, Serialize, Type)]
pub struct OnboardingPlan {
    pub total_tools_scanned: usize,
    /// Name-groups ("skills") in `groups`.
    pub group_count: usize,
    /// Copies across every group: one per (Tool, directory) variant.
    pub copy_count: usize,
    /// Sorted by name.
    pub groups: Vec<OnboardingGroup>,
    /// Directories the `SKILL.md` gate refused, in scan order; never part of
    /// a group and never importable.
    pub invalid: Vec<OnboardingInvalidEntry>,
}

/// Which Tools the onboarding scan visits. Resolved at the command seam
/// from the "only scan selected tools" setting and the global selection;
/// core applies it and reports the scope's size as `total_tools_scanned`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum OnboardingScanScope {
    /// Every detected Tool (the setting is off, or on with no saved selection).
    #[default]
    Installed,
    /// Exactly these registry keys, detected or not: an absent skills dir
    /// simply scans to nothing, and the selection is the operator's word.
    Selected(Vec<String>),
}

impl OnboardingScanScope {
    fn includes(&self, home: &Path, adapter: &ToolAdapter) -> bool {
        match self {
            OnboardingScanScope::Installed => is_installed_in(home, adapter),
            OnboardingScanScope::Selected(keys) => keys.iter().any(|k| k == adapter.key()),
        }
    }
}

/// Scan the Tools in `scope` under `home` for unmanaged skills, excluding
/// anything already living in (or linked into) `central_dir` and every
/// target the store already manages.
pub fn build_onboarding_plan(
    home: &Path,
    central_dir: &Path,
    store: &SkillStore,
    scope: &OnboardingScanScope,
) -> Result<OnboardingPlan> {
    let managed_targets = store
        .list_all_skill_target_paths()
        .unwrap_or_default()
        .into_iter()
        .map(|(tool, path)| managed_target_key(&tool, Path::new(&path)))
        .collect::<std::collections::HashSet<_>>();
    let managed_names = managed_names(store)?;
    build_onboarding_plan_in_home(
        home,
        scope,
        Some(central_dir),
        Some(&managed_targets),
        &managed_names,
    )
}

/// Every name a Managed skill holds → its id: the record's name and its
/// central directory's name (the one finalize's collision check reads).
fn managed_names(store: &SkillStore) -> Result<HashMap<String, String>> {
    let mut names = HashMap::new();
    for skill in store.list_skills()? {
        if let Some(dir_name) = Path::new(&skill.central_path).file_name() {
            names
                .entry(dir_name.to_string_lossy().into_owned())
                .or_insert_with(|| skill.id.clone());
        }
        names.insert(skill.name.clone(), skill.id.clone());
    }
    Ok(names)
}

fn build_onboarding_plan_in_home(
    home: &Path,
    scope: &OnboardingScanScope,
    exclude_root: Option<&Path>,
    exclude_managed_targets: Option<&std::collections::HashSet<String>>,
    managed_names: &HashMap<String, String>,
) -> Result<OnboardingPlan> {
    let adapters = default_tool_adapters();
    let mut all_detected: Vec<DetectedSkill> = Vec::new();
    let mut invalid: Vec<OnboardingInvalidEntry> = Vec::new();
    let mut scanned = 0usize;

    for adapter in adapters {
        if !scope.includes(home, adapter) {
            continue;
        }
        scanned += 1;
        let scan = scan_tool_dir(adapter, &skills_dir_in(home, adapter))?;
        all_detected.extend(filter_detected(
            scan.skills,
            exclude_root,
            exclude_managed_targets,
        ));
        // The same exclusions as the skills: a path the app owns is never
        // reported, valid or not.
        invalid.extend(
            scan.invalid
                .into_iter()
                .filter(|dir| is_unmanaged(&dir.entry, exclude_root, exclude_managed_targets))
                .map(|dir| OnboardingInvalidEntry {
                    tool: dir.entry.tool.as_key().to_string(),
                    name: dir.entry.name,
                    path: dir.entry.path,
                    reason: dir.reason,
                }),
        );
    }

    let mut grouped: HashMap<String, Vec<OnboardingVariant>> = HashMap::new();
    for skill in all_detected.iter() {
        let fingerprint = content_identity::read(Source::Directory(&skill.path));
        let entry = grouped.entry(skill.name.clone()).or_default();
        entry.push(OnboardingVariant {
            tool: skill.tool.as_key().to_string(),
            name: skill.name.clone(),
            path: skill.path.clone(),
            fingerprint,
            is_link: skill.is_link,
            link_target: skill.link_target.clone(),
        });
    }

    let mut groups: Vec<OnboardingGroup> = grouped
        .into_iter()
        .map(|(name, variants)| {
            let mut uniq = variants
                .iter()
                .filter_map(|v| v.fingerprint.as_ref())
                .collect::<std::collections::HashSet<_>>()
                .len();
            if uniq == 0 {
                uniq = 1;
            }
            let managed_skill_id = managed_names.get(&name).cloned();
            OnboardingGroup {
                name,
                has_conflict: uniq > 1,
                variants,
                name_is_managed: managed_skill_id.is_some(),
                managed_skill_id,
            }
        })
        .collect();
    groups.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(OnboardingPlan {
        total_tools_scanned: scanned,
        group_count: groups.len(),
        copy_count: all_detected.len(),
        groups,
        invalid,
    })
}

fn filter_detected(
    detected: Vec<DetectedSkill>,
    exclude_root: Option<&Path>,
    exclude_managed_targets: Option<&std::collections::HashSet<String>>,
) -> Vec<DetectedSkill> {
    detected
        .into_iter()
        .filter(|skill| is_unmanaged(skill, exclude_root, exclude_managed_targets))
        .collect()
}

/// Not inside (or linked into) the central repo, and not a recorded target.
fn is_unmanaged(
    skill: &DetectedSkill,
    exclude_root: Option<&Path>,
    exclude_managed_targets: Option<&std::collections::HashSet<String>>,
) -> bool {
    if let Some(exclude_root) = exclude_root {
        if is_under(&skill.path, exclude_root) {
            return false;
        }
        if let Some(target) = &skill.link_target {
            if is_under(target, exclude_root) {
                return false;
            }
        }
    }
    if let Some(exclude) = exclude_managed_targets {
        if exclude.contains(&managed_target_key(skill.tool.as_key(), &skill.path)) {
            return false;
        }
    }
    true
}

fn is_under(path: &Path, base: &Path) -> bool {
    path.starts_with(base)
}

fn managed_target_key(tool: &str, path: &Path) -> String {
    let tool = tool.to_ascii_lowercase();
    let normalized = normalize_path_for_key(path);
    format!("{tool}\n{normalized}")
}

fn normalize_path_for_key(path: &Path) -> String {
    let normalized: PathBuf = path.components().collect();
    let s = normalized.to_string_lossy().to_string();
    #[cfg(windows)]
    {
        s.to_lowercase()
    }
    #[cfg(not(windows))]
    {
        s
    }
}

#[cfg(test)]
#[path = "tests/onboarding.rs"]
mod tests;
