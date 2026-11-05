use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Serialize;
use specta::Type;

use super::artifact_removal::artifact_identity;
use super::content_identity::{self, Source};
use super::skill_discovery::InvalidManifestReason;
use super::skill_store::SkillStore;
use super::tool_adapters::{
    default_tool_adapters, is_installed_in, scan_tool_dir, skills_dir_in, DetectedSkill,
    ToolAdapter,
};

#[derive(Clone, Debug, Serialize, Type)]
pub struct OnboardingVariant {
    /// The Tool the copy is attributed to: the first Tool in scope, in
    /// registry order, whose own skills dir holds it.
    pub tool: String,
    /// Every harness in scope that loads this copy (registry order): each
    /// in-scope Tool whose own skills dir holds it (`~/.agents/skills`: the
    /// AGENTS-standard entry, Codex, Goose, OpenHands) and each that reads
    /// that dir besides (a VERIFIED AGENTS-standard reader, a cross-read —
    /// `ToolAdapter::reads_global_dir`). Under Installed scope only detected
    /// harnesses; under Selected scope the selected ones that are detected,
    /// plus the attributed Tool itself. One physical copy is one variant: the
    /// roster names its consumers without duplicating it — a Tool skills dir
    /// symlinked to another's is the same physical dir, so both dirs'
    /// readers are unioned here.
    pub readers: Vec<String>,
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
    // Owned rows only (`synced_at` set), keyed by physical artifact identity
    // alone: a shared dir's copy is attributed to one sharer, while the row
    // owning it may be another's (a managed Codex copy in `~/.agents/skills`),
    // and a Tool skills dir may be a symlink to another's.
    let managed_targets = store
        .list_all_skill_target_paths()
        .unwrap_or_default()
        .into_iter()
        .map(|(_tool, path)| managed_target_key(Path::new(&path)))
        .collect::<HashSet<_>>();
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
    exclude_managed_targets: Option<&HashSet<PathBuf>>,
    managed_names: &HashMap<String, String>,
) -> Result<OnboardingPlan> {
    let adapters = default_tool_adapters();
    // Each detected copy with the index of the scanned root it came from.
    let mut all_detected: Vec<(usize, DetectedSkill)> = Vec::new();
    let mut invalid: Vec<OnboardingInvalidEntry> = Vec::new();
    let mut scanned = 0usize;
    let mut roots: Vec<ScannedRoot> = Vec::new();

    for adapter in adapters {
        if !scope.includes(home, adapter) {
            continue;
        }
        scanned += 1;
        // A physical dir several Tools reach is scanned once — a dir they
        // share (`~/.agents/skills`: the AGENTS-standard entry, Codex, Goose,
        // OpenHands), or one Tool's skills dir symlinked to another's — and
        // its copies are attributed to the first Tool in scope, in registry
        // order: one copy is one variant, never one per sharer or alias.
        let dir = skills_dir_in(home, adapter);
        let identity = std::fs::canonicalize(&dir).unwrap_or_else(|_| dir.components().collect());
        if let Some(root) = roots.iter_mut().find(|root| root.identity == identity) {
            if !root.relative_dirs.contains(&adapter.relative_skills_dir) {
                root.relative_dirs.push(adapter.relative_skills_dir);
            }
            continue;
        }
        let scan = scan_tool_dir(adapter, &dir)?;
        let root_index = roots.len();
        roots.push(ScannedRoot {
            identity,
            relative_dirs: vec![adapter.relative_skills_dir],
        });
        all_detected.extend(
            filter_detected(scan.skills, exclude_root, exclude_managed_targets)
                .into_iter()
                .map(|skill| (root_index, skill)),
        );
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
    for (root_index, skill) in all_detected.iter() {
        let fingerprint = content_identity::read(Source::Directory(&skill.path));
        let entry = grouped.entry(skill.name.clone()).or_default();
        entry.push(OnboardingVariant {
            tool: skill.tool.as_key().to_string(),
            readers: readers_of(home, scope, skill, &roots[*root_index].relative_dirs),
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

/// One physical skills dir the scan visited: its identity (canonicalised,
/// lexical when it cannot be resolved) and every registry skills dir
/// (home-relative) in scope that reaches it — its own, and any aliased to it.
struct ScannedRoot {
    identity: PathBuf,
    relative_dirs: Vec<&'static str>,
}

/// The in-scope harnesses loading `skill`'s copy (see
/// [`OnboardingVariant::readers`]): every harness reading any of the
/// registry dirs that reach the copy's physical root (`relative_dirs`). The
/// attributed Tool always reads it.
fn readers_of(
    home: &Path,
    scope: &OnboardingScanScope,
    skill: &DetectedSkill,
    relative_dirs: &[&'static str],
) -> Vec<String> {
    let attributed = skill.tool.as_key();
    default_tool_adapters()
        .iter()
        .filter(|a| {
            a.key() == attributed || relative_dirs.iter().any(|dir| a.reads_global_dir(dir))
        })
        .filter(|a| a.key() == attributed || (scope.includes(home, a) && is_installed_in(home, a)))
        .map(|a| a.key().to_string())
        .collect()
}

fn filter_detected(
    detected: Vec<DetectedSkill>,
    exclude_root: Option<&Path>,
    exclude_managed_targets: Option<&HashSet<PathBuf>>,
) -> Vec<DetectedSkill> {
    detected
        .into_iter()
        .filter(|skill| is_unmanaged(skill, exclude_root, exclude_managed_targets))
        .collect()
}

/// Not inside (or linked into) the central repo, and not the physical
/// artifact an owned target row records — whichever Tool the copy is
/// attributed to, and through whichever alias of its dir it was reached.
fn is_unmanaged(
    skill: &DetectedSkill,
    exclude_root: Option<&Path>,
    exclude_managed_targets: Option<&HashSet<PathBuf>>,
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
        if exclude.contains(&managed_target_key(&skill.path)) {
            return false;
        }
    }
    true
}

fn is_under(path: &Path, base: &Path) -> bool {
    path.starts_with(base)
}

/// The exclusion key: Artifact removal's one physical identity
/// (`artifact_removal::artifact_identity` — canonical parent + leaf, never
/// the link's referent; lexical when the parent cannot be resolved).
fn managed_target_key(path: &Path) -> PathBuf {
    artifact_identity(path)
}

#[cfg(test)]
#[path = "tests/onboarding.rs"]
mod tests;
