use std::path::Path;

use anyhow::{bail, Context, Result};
use walkdir::{DirEntry, WalkDir};

const IGNORE_NAMES: [&str; 4] = [".git", ".DS_Store", "Thumbs.db", ".gitignore"];
const MAX_FILE_SIZE: u64 = 1_048_576; // 1 MB

pub(crate) fn is_ignored(entry: &DirEntry) -> bool {
    let file_name = entry.file_name().to_string_lossy();
    // `.skills-hub-manifest-*` is the temp file of `manifest`'s atomic
    // write; an abandoned one must be invisible to hashing (identity) and to
    // file listings alike, so the rule lives in this one predicate.
    IGNORE_NAMES.iter().any(|name| name == &file_name.as_ref())
        || file_name.starts_with(".skills-hub-manifest-")
}

pub struct FileEntry {
    pub path: String,
    pub size: u64,
}

pub fn list_files(central_path: &Path) -> Result<Vec<FileEntry>> {
    let mut entries: Vec<FileEntry> = Vec::new();

    for entry in WalkDir::new(central_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_ignored(e))
    {
        let entry = entry?;
        if !entry.file_type().is_file() || is_ignored(&entry) {
            continue;
        }

        let relative = entry
            .path()
            .strip_prefix(central_path)
            .with_context(|| format!("strip prefix {:?}", entry.path()))?;

        let metadata = entry.metadata()?;
        entries.push(FileEntry {
            path: relative.to_string_lossy().to_string(),
            size: metadata.len(),
        });
    }

    // Sort: SKILL.md first, then alphabetical
    entries.sort_by(|a, b| {
        let a_is_skill = a.path.eq_ignore_ascii_case("skill.md");
        let b_is_skill = b.path.eq_ignore_ascii_case("skill.md");
        match (a_is_skill, b_is_skill) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.path.to_lowercase().cmp(&b.path.to_lowercase()),
        }
    });

    Ok(entries)
}

pub fn read_file(central_path: &Path, relative_path: &str) -> Result<String> {
    // Path traversal protection
    if relative_path.contains("..") {
        bail!("Invalid file path: path traversal not allowed");
    }

    let full_path = central_path.join(relative_path);
    let canonical = full_path
        .canonicalize()
        .with_context(|| format!("resolve path: {:?}", full_path))?;
    let canonical_base = central_path
        .canonicalize()
        .with_context(|| format!("resolve base: {:?}", central_path))?;

    if !canonical.starts_with(&canonical_base) {
        bail!("Invalid file path: outside skill directory");
    }

    let metadata =
        std::fs::metadata(&canonical).with_context(|| format!("read metadata: {:?}", canonical))?;

    if metadata.len() > MAX_FILE_SIZE {
        bail!(
            "File too large to display ({:.1} KB, max 1 MB)",
            metadata.len() as f64 / 1024.0
        );
    }

    let bytes = std::fs::read(&canonical).with_context(|| format!("read file: {:?}", canonical))?;

    String::from_utf8(bytes)
        .map_err(|_| anyhow::anyhow!("File is not valid UTF-8 text and cannot be displayed"))
}

// ---------------------------------------------------------------------------
// A reported occupant's files, against the skill (round 20 #70)
// ---------------------------------------------------------------------------

/// How one file of a folder in a target's way compares with the skill's
/// Library copy, by its path inside each and its bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum FileComparison {
    /// The same bytes at the same path in both.
    Same,
    /// At the same path in both, with different bytes.
    Changed,
    /// Only in the folder in the way.
    OnlyHere,
    /// Only in the skill: the folder in the way lacks it.
    OnlyInSkill,
}

/// One file of a reported occupant, or one the skill has and it lacks.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct OccupantFileEntry {
    pub path: String,
    /// The occupant's size (the skill's, for a file only the skill has).
    pub size: u64,
    pub compared: FileComparison,
}

/// One file's text on each side; `None` where that side has no such file.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, specta::Type)]
pub struct OccupantFile {
    pub occupant: Option<String>,
    pub skill: Option<String>,
}

/// The files of the folder at `occupant` — a folder the app reported in an
/// ask's way (`core::occupants`; the command admits nothing else) — each
/// compared with the skill's Library copy at `skill` (`None` when the skill
/// record is gone: every file is the occupant's alone), followed by the
/// skill's files the occupant lacks. Read-only, through the same listing as
/// the file browser (links inside are never followed).
pub fn list_occupant_files(
    occupant: &Path,
    skill: Option<&Path>,
) -> Result<Vec<OccupantFileEntry>> {
    let theirs = list_files(occupant)?;
    let ours = match skill {
        Some(skill) if skill.is_dir() => list_files(skill)?,
        _ => Vec::new(),
    };
    let mut entries: Vec<OccupantFileEntry> = theirs
        .iter()
        .map(|file| {
            let compared = match (skill, ours.iter().any(|mine| mine.path == file.path)) {
                (Some(skill), true) => {
                    let same = std::fs::read(occupant.join(&file.path))
                        .ok()
                        .zip(std::fs::read(skill.join(&file.path)).ok())
                        .is_some_and(|(a, b)| a == b);
                    if same {
                        FileComparison::Same
                    } else {
                        FileComparison::Changed
                    }
                }
                _ => FileComparison::OnlyHere,
            };
            OccupantFileEntry {
                path: file.path.clone(),
                size: file.size,
                compared,
            }
        })
        .collect();
    entries.extend(
        ours.into_iter()
            .filter(|mine| !theirs.iter().any(|file| file.path == mine.path))
            .map(|mine| OccupantFileEntry {
                path: mine.path,
                size: mine.size,
                compared: FileComparison::OnlyInSkill,
            }),
    );
    Ok(entries)
}

/// One file of a reported occupant beside the skill's file at the same path,
/// each read under [`read_file`]'s containment rule; a side without the file
/// is `None`.
pub fn read_occupant_file(
    occupant: &Path,
    skill: Option<&Path>,
    relative_path: &str,
) -> Result<OccupantFile> {
    let side = |root: &Path| -> Result<Option<String>> {
        if !root.join(relative_path).exists() {
            return Ok(None);
        }
        read_file(root, relative_path).map(Some)
    };
    if relative_path.contains("..") || Path::new(relative_path).is_absolute() {
        bail!("Invalid file path: path traversal not allowed");
    }
    Ok(OccupantFile {
        occupant: side(occupant)?,
        skill: match skill {
            Some(skill) => side(skill)?,
            None => None,
        },
    })
}

#[cfg(test)]
#[path = "tests/skill_files.rs"]
mod tests;
