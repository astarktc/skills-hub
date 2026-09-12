//! Skill discovery: find every skill candidate in a directory tree.
//!
//! One scan ladder serves the git listing, the local listing and the update
//! flow's name backfill. Each strategy contributes directories; every hit is
//! inspected the same way ([`inspect`]) and the ladder ends with one dedup
//! (first strategy to reach a subpath wins) and one sort (by name, then
//! subpath). Strategies, in order:
//!
//! 1. The root itself, when it carries a `SKILL.md` (subpath `.`).
//! 2. Known scan bases (`skills/*`, `.claude/skills/*`, ...): *every* child
//!    directory is a candidate, so a broken or missing `SKILL.md` under a
//!    declared skills dir surfaces with a reason instead of vanishing.
//! 3. Root-level skill dirs (`repo/my-skill/SKILL.md`) and root-level skill
//!    containers (`repo/*skill*/my-skill/SKILL.md`).
//! 4. Plugins declared in `.claude-plugin/marketplace.json`.
//! 5. A bounded recursive walk (depth 5) for layouts nobody anticipated.
//!
//! Validity is reported, never enforced during discovery: callers decide
//! what to show ([`Validity::is_installable`] vs [`Validity::is_valid`]).
//! The one place validity *is* enforced is [`require_skill_md`], the
//! admission rule every install path applies to a source directory.
//! No message here is user copy — reasons are stable machine tokens.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Deserialize;

use super::errors::SignalError;
// Compatibility exports for existing callers; Manifest is the only implementation.
pub use super::manifest::InvocationMode;
pub(crate) use super::manifest::{find_skill_md, parse_skill_md, parse_skill_md_with_reason};
use super::skill_matching::MatchableSkill;

/// Directories relative to a root that are declared skill homes.
const SKILL_SCAN_BASES: [&str; 5] = [
    "skills",
    "skills/.curated",
    "skills/.experimental",
    "skills/.system",
    ".claude/skills",
];

/// Directories the recursive walk never enters (performance + correctness).
const SKIP_DIRS: [&str; 7] = [
    "node_modules",
    ".git",
    "dist",
    "build",
    "target",
    ".next",
    ".cache",
];

/// Maximum depth of the recursive fallback walk (root is depth 0).
const MAX_RECURSIVE_DEPTH: usize = 5;

/// Why a discovered directory is or is not a usable skill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Validity {
    /// `SKILL.md` parsed (has a `name`), or the dir lives under `.claude/skills/`
    /// where a manifest is optional.
    Valid,
    /// A `SKILL.md` exists but is unusable; the token is one of
    /// `read_failed`, `invalid_frontmatter`, `missing_name`.
    InvalidSkillMd(&'static str),
    /// No `SKILL.md` at all (only reported for children of a scan base).
    MissingSkillMd,
}

impl Validity {
    /// Manifest present and well-formed (or optional). The local listing's
    /// notion of "valid".
    pub fn is_valid(self) -> bool {
        matches!(self, Validity::Valid)
    }

    /// There are skill bytes to install: a `SKILL.md` exists (even if broken)
    /// or the dir is a `.claude/skills/` child. The git listing's admission
    /// rule, and the same predicate the git install path enforces.
    pub fn is_installable(self) -> bool {
        !matches!(self, Validity::MissingSkillMd)
    }

    /// Machine token explaining an invalid candidate; `None` when valid.
    pub fn reason(self) -> Option<&'static str> {
        match self {
            Validity::Valid => None,
            Validity::InvalidSkillMd(reason) => Some(reason),
            Validity::MissingSkillMd => Some("missing_skill_md"),
        }
    }
}

/// One skill candidate found under a discovery root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredSkill {
    /// Path relative to the discovery root; `.` for the root itself.
    pub subpath: String,
    /// `SKILL.md` `name`, else the folder name (`root-skill` for the root).
    pub name: String,
    /// `SKILL.md` `description`, else `.claude-plugin/plugin.json`'s.
    pub description: Option<String>,
    pub validity: Validity,
}

impl MatchableSkill for DiscoveredSkill {
    fn name(&self) -> &str {
        &self.name
    }
    fn subpath(&self) -> &str {
        &self.subpath
    }
}

/// Discover every skill candidate under `root`.
///
/// Returns one entry per distinct subpath, sorted by name then subpath.
/// A root that does not exist yields an empty list.
pub fn discover_skills(root: &Path) -> Vec<DiscoveredSkill> {
    let mut dirs: Vec<PathBuf> = Vec::new();

    // 1) The root itself.
    if has_skill_md(root) || is_claude_skill_dir(root) {
        dirs.push(root.to_path_buf());
    }

    // 2) Known scan bases: every non-hidden child directory is a candidate,
    //    except a child that is itself a scan base (`skills/.curated` under `skills`).
    let scan_bases: Vec<PathBuf> = SKILL_SCAN_BASES.iter().map(|b| root.join(b)).collect();
    for base in &scan_bases {
        if let Ok(rd) = std::fs::read_dir(base) {
            for entry in rd.flatten() {
                let p = entry.path();
                if p.is_dir()
                    && !is_hidden_dir_name(&entry.file_name().to_string_lossy())
                    && !scan_bases.contains(&p)
                {
                    dirs.push(p);
                }
            }
        }
    }

    // 3) Root-level skills and root-level skill containers.
    if let Ok(rd) = std::fs::read_dir(root) {
        for entry in rd.flatten() {
            let p = entry.path();
            if !p.is_dir() {
                continue;
            }
            let dir_name = entry.file_name();
            let dir_name = dir_name.to_string_lossy();
            if is_hidden_dir_name(&dir_name) || is_known_root_scan_dir(&dir_name) {
                continue;
            }
            if is_skill_dir(&p) {
                dirs.push(p);
            } else if is_skill_container_dir_name(&dir_name) {
                push_skill_dirs_from_base(&mut dirs, &p);
            }
        }
    }

    // 4) Marketplace plugins.
    dirs.extend(scan_marketplace_skills(&parse_marketplace_json(root)));

    // 5) Bounded recursive fallback (the root was handled by strategy 1).
    dirs.extend(
        find_skill_dirs_recursive(root, 0, MAX_RECURSIVE_DEPTH)
            .into_iter()
            .filter(|p| p != root),
    );

    let mut seen: HashSet<String> = HashSet::new();
    let mut unique: Vec<(String, PathBuf)> = Vec::new();
    for dir in dirs {
        let subpath = relative_subpath(root, &dir);
        if seen.insert(subpath.clone()) {
            unique.push((subpath, dir));
        }
    }

    // A candidate's identity is the directory, not the subpath: a repo that
    // publishes one skill under `skills/<name>` and links it from
    // `.claude/skills/<name>` (and `.codex/skills/<name>`, ...) holds one
    // skill, not one per alias — counted separately they would make its own
    // name ambiguous. Aliases collapse into the real directory (the one whose
    // subpath is where it actually lives); a link whose target discovery
    // never visits stays a candidate of its own.
    let canonical_root = root.canonicalize().ok();
    let is_real = |subpath: &str, canonical: &Path| {
        canonical_root
            .as_ref()
            .is_some_and(|croot| croot.join(subpath) == canonical)
    };
    let mut kept: Vec<(PathBuf, String, PathBuf)> = Vec::new();
    for (subpath, dir) in unique {
        let canonical = dir.canonicalize().unwrap_or_else(|_| dir.clone());
        match kept.iter_mut().find(|(c, _, _)| *c == canonical) {
            Some(slot) => {
                if is_real(&subpath, &canonical) {
                    *slot = (canonical, subpath, dir);
                }
            }
            None => kept.push((canonical, subpath, dir)),
        }
    }

    let mut out: Vec<DiscoveredSkill> = kept
        .into_iter()
        .map(|(_, subpath, dir)| inspect(root, &dir, subpath))
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.subpath.cmp(&b.subpath)));
    out
}

/// Read one candidate directory: name, description and validity.
fn inspect(root: &Path, dir: &Path, subpath: String) -> DiscoveredSkill {
    // Fallback name when SKILL.md yields none. The root gets a fixed name
    // rather than its folder: a git cache dir is a content hash.
    let folder_name = || {
        if subpath == "." {
            return "root-skill".to_string();
        }
        dir.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    };
    let (name, description, validity) = match find_skill_md(dir) {
        Some(skill_md) => match parse_skill_md_with_reason(&skill_md) {
            Ok((name, desc)) => (name, desc, Validity::Valid),
            Err(reason) => (
                folder_name(),
                read_plugin_description(root),
                Validity::InvalidSkillMd(reason),
            ),
        },
        None if is_claude_skill_dir(dir) => (
            folder_name(),
            read_plugin_description(root),
            Validity::Valid,
        ),
        None => (folder_name(), None, Validity::MissingSkillMd),
    };
    DiscoveredSkill {
        subpath,
        name,
        description,
        validity,
    }
}

fn relative_subpath(root: &Path, dir: &Path) -> String {
    let rel = dir
        .strip_prefix(root)
        .unwrap_or(dir)
        .to_string_lossy()
        .to_string();
    if rel.is_empty() {
        ".".to_string()
    } else {
        rel
    }
}

// ── SKILL.md lookup and parsing ──

/// Check if a directory contains a SKILL.md file (case-insensitive).
pub(crate) fn has_skill_md(dir: &Path) -> bool {
    find_skill_md(dir).is_some()
}

/// The admission rule for turning a directory into a Managed skill: a source
/// without a `SKILL.md` is not an installable skill (fixes #8 — directories
/// that were "discovered" under a tool's skills dir but carry no manifest).
/// Returns the manifest path so callers that parse it do not look twice, and
/// raises the typed `SignalError::SkillInvalid { reason: "missing_skill_md" }`
/// the frontend localizes.
pub(crate) fn require_skill_md(dir: &Path) -> Result<PathBuf> {
    find_skill_md(dir).ok_or_else(|| {
        anyhow::anyhow!(SignalError::SkillInvalid {
            reason: "missing_skill_md".to_string(),
        })
    })
}

/// Check if a directory is a skill dir: has SKILL.md or is a `.claude/skills/` child.
pub(crate) fn is_skill_dir(p: &Path) -> bool {
    p.is_dir() && (has_skill_md(p) || is_claude_skill_dir(p))
}

/// A directory under `.claude/skills/` is a skill even without SKILL.md.
fn is_claude_skill_dir(p: &Path) -> bool {
    if let Some(parent) = p.parent() {
        let parent_str = parent.to_string_lossy();
        if parent_str.ends_with(".claude/skills") || parent_str.ends_with(".claude\\skills") {
            return p.is_dir();
        }
    }
    false
}

/// Description fallback from `.claude-plugin/plugin.json` at the root.
fn read_plugin_description(root: &Path) -> Option<String> {
    let plugin_json = root.join(".claude-plugin/plugin.json");
    let content = std::fs::read_to_string(plugin_json).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    json.get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

// ── Strategy helpers ──

fn is_hidden_dir_name(name: &str) -> bool {
    name.starts_with('.')
}

fn is_known_root_scan_dir(name: &str) -> bool {
    SKILL_SCAN_BASES
        .iter()
        .filter_map(|base| base.split('/').next())
        .any(|base| base == name)
}

fn is_skill_container_dir_name(name: &str) -> bool {
    name.to_ascii_lowercase().contains("skill")
}

fn push_skill_dirs_from_base(out: &mut Vec<PathBuf>, base_dir: &Path) {
    if let Ok(rd) = std::fs::read_dir(base_dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if is_skill_dir(&p) {
                out.push(p);
            }
        }
    }
}

/// Recursively find directories containing SKILL.md, up to `max_depth` levels
/// deep (e.g. wshobson/agents keeps skills at depth 4:
/// `plugins/*/skills/*/SKILL.md`).
fn find_skill_dirs_recursive(dir: &Path, depth: usize, max_depth: usize) -> Vec<PathBuf> {
    if depth > max_depth {
        return vec![];
    }
    let mut results = Vec::new();
    if has_skill_md(dir) {
        results.push(dir.to_path_buf());
    }
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if !p.is_dir() {
                continue;
            }
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if SKIP_DIRS.iter().any(|&s| name_str == s) {
                continue;
            }
            // Skip hidden dirs except `.claude`, which hosts a scan base.
            if name_str.starts_with('.') && name_str != ".claude" {
                continue;
            }
            results.extend(find_skill_dirs_recursive(&p, depth + 1, max_depth));
        }
    }
    results
}

#[derive(Deserialize)]
struct MarketplaceManifest {
    plugins: Option<Vec<MarketplacePlugin>>,
}

#[derive(Deserialize)]
struct MarketplacePlugin {
    source: Option<String>,
}

/// Parse `.claude-plugin/marketplace.json` into the plugin dirs it declares.
/// Returns `root`-based paths (not canonicalized, so they strip back to
/// subpaths even when `root` sits behind a symlink) that exist on disk and
/// resolve inside `root`.
fn parse_marketplace_json(root: &Path) -> Vec<PathBuf> {
    let manifest_path = root.join(".claude-plugin/marketplace.json");
    let content = match std::fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let manifest: MarketplaceManifest = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(_) => return vec![],
    };
    let plugins = match manifest.plugins {
        Some(p) => p,
        None => return vec![],
    };
    let repo_root = match root.canonicalize() {
        Ok(p) => p,
        Err(_) => return vec![],
    };

    plugins
        .iter()
        .filter_map(|plugin| {
            let source = plugin.source.as_ref()?;
            let cleaned = source.strip_prefix("./").unwrap_or(source);
            let plugin_dir = root.join(cleaned);
            let resolved = plugin_dir.canonicalize().ok()?;
            if resolved.starts_with(&repo_root) && resolved.is_dir() {
                Some(plugin_dir)
            } else {
                None
            }
        })
        .collect()
}

/// Skill dirs inside marketplace plugin dirs: `plugin/skills/*/SKILL.md` and
/// direct children `plugin/*/SKILL.md`.
fn scan_marketplace_skills(plugin_dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut results = Vec::new();
    for plugin_dir in plugin_dirs {
        if let Ok(rd) = std::fs::read_dir(plugin_dir.join("skills")) {
            for entry in rd.flatten() {
                let p = entry.path();
                if p.is_dir() && has_skill_md(&p) {
                    results.push(p);
                }
            }
        }
        if let Ok(rd) = std::fs::read_dir(plugin_dir) {
            for entry in rd.flatten() {
                let p = entry.path();
                if entry.file_name().to_string_lossy() == "skills" {
                    continue; // Already scanned above.
                }
                if p.is_dir() && has_skill_md(&p) {
                    results.push(p);
                }
            }
        }
    }
    results
}

#[cfg(test)]
#[path = "tests/skill_discovery.rs"]
mod tests;
