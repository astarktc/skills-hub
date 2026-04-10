use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

/// A parsed entry from ~/.agents/.skill-lock.json with fields needed for enrichment.
pub struct SkillLockEntry {
    pub source_url: String,
    pub source_subpath: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct SkillLockFile {
    #[serde(default)]
    skills: Option<HashMap<String, SkillLockRaw>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct SkillLockRaw {
    source_url: Option<String>,
    skill_path: Option<String>,
}

/// Parse a skill lock file and return a map of skill name -> SkillLockEntry.
/// Returns None if the file is missing, unreadable, or malformed.
pub fn parse_lock_file(path: &Path) -> Option<HashMap<String, SkillLockEntry>> {
    let content = std::fs::read_to_string(path).ok()?;
    let lock_file: SkillLockFile = serde_json::from_str(&content).ok()?;
    let skills = lock_file.skills?;
    if skills.is_empty() {
        return None;
    }

    let mut result = HashMap::new();
    for (name, raw) in skills {
        if let Some(source_url) = raw.source_url {
            let source_subpath = raw.skill_path.as_deref().and_then(derive_subpath);
            result.insert(
                name,
                SkillLockEntry {
                    source_url,
                    source_subpath,
                },
            );
        }
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Derive the source_subpath from a skillPath value.
/// Takes the parent directory of the path. If the parent is empty or "."
/// (meaning the file is at repo root), returns None.
pub fn derive_subpath(skill_path: &str) -> Option<String> {
    let path = std::path::Path::new(skill_path);
    let parent = path.parent()?;
    let parent_str = parent.to_string_lossy();
    if parent_str.is_empty() || parent_str == "." {
        None
    } else {
        // Normalize to forward slashes
        Some(parent_str.replace('\\', "/"))
    }
}

/// Try to enrich a source path with git provenance from the skill lock file.
/// Uses the real home directory from dirs::home_dir().
pub fn try_enrich_from_skill_lock(source_path: &Path) -> Option<SkillLockEntry> {
    let home = dirs::home_dir()?;
    try_enrich_from_skill_lock_with_home(source_path, &home)
}

/// Testable variant that accepts an explicit home directory.
/// Checks if source_path is a symlink pointing into {home}/.agents/skills/,
/// then looks up the skill name in {home}/.agents/.skill-lock.json.
pub fn try_enrich_from_skill_lock_with_home(
    source_path: &Path,
    home: &Path,
) -> Option<SkillLockEntry> {
    // Step a: read_link -- if not a symlink, return None
    let link_target = std::fs::read_link(source_path).ok()?;

    // Step b: resolve relative symlinks against source_path's parent
    let resolved = if link_target.is_relative() {
        source_path
            .parent()
            .map(|p| p.join(&link_target))
            .unwrap_or(link_target)
    } else {
        link_target
    };
    // Canonicalize to resolve any ".." components
    let resolved = std::fs::canonicalize(&resolved).ok()?;

    // Step c: check if resolved target is under {home}/.agents/skills/
    let agents_skills = home.join(".agents").join("skills");
    let agents_skills_canonical = std::fs::canonicalize(&agents_skills).ok()?;
    if !resolved.starts_with(&agents_skills_canonical) {
        return None;
    }

    // Step d: extract skill name (first component after .agents/skills/)
    let relative = resolved.strip_prefix(&agents_skills_canonical).ok()?;
    let skill_name = relative.components().next()?.as_os_str().to_string_lossy();

    // Step e: read the lock file
    let lock_path = home.join(".agents").join(".skill-lock.json");
    let entries = parse_lock_file(&lock_path)?;

    // Step f-j: look up and return
    entries.into_iter().find_map(|(name, entry)| {
        if name == skill_name.as_ref() {
            Some(entry)
        } else {
            None
        }
    })
}

#[cfg(test)]
#[path = "tests/skill_lock.rs"]
mod tests;
