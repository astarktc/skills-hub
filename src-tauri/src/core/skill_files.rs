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
    Ok(list_files_within(central_path, usize::MAX)?.0)
}

/// [`list_files`], keeping at most `max` files (in walk order, then sorted);
/// the flag says whether the folder holds a file beyond them.
fn list_files_within(central_path: &Path, max: usize) -> Result<(Vec<FileEntry>, bool)> {
    let mut entries: Vec<FileEntry> = Vec::new();
    let mut truncated = false;

    for entry in WalkDir::new(central_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_ignored(e))
    {
        let entry = match entry {
            Ok(entry) => entry,
            // Past the cap only the next file matters, not a failed read.
            Err(_) if entries.len() >= max => continue,
            Err(err) => return Err(err.into()),
        };
        if !entry.file_type().is_file() || is_ignored(&entry) {
            continue;
        }
        if entries.len() >= max {
            truncated = true;
            break;
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

    Ok((entries, truncated))
}

/// `relative_path` under `base` resolved by [`read_file`]'s containment rule
/// (no `..`, no absolute path, the resolved path inside the resolved base):
/// `None` when there is no such file there, or it resolves out of `base`.
fn contained_file(base: &Path, relative_path: &str) -> Option<std::path::PathBuf> {
    if relative_path.contains("..") || Path::new(relative_path).is_absolute() {
        return None;
    }
    let canonical = base.join(relative_path).canonicalize().ok()?;
    let canonical_base = base.canonicalize().ok()?;
    (canonical.starts_with(&canonical_base) && canonical.is_file()).then_some(canonical)
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

/// A reported occupant's files against the skill: `truncated` when the
/// folder holds more files than [`OCCUPANT_FILES_MAX`] and only the first of
/// them (plus every file the skill has, probed directly) are listed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, specta::Type)]
pub struct OccupantFileListing {
    pub entries: Vec<OccupantFileEntry>,
    pub truncated: bool,
}

/// How many files of a folder in the way are listed at most.
const OCCUPANT_FILES_MAX: usize = 500;

/// The files of the folder at `occupant` — a folder the app reported in an
/// ask's way (`core::occupants`; the command admits nothing else) — each
/// compared with the skill's Library copy at `skill` (`None` when the skill
/// record is gone: every file is the occupant's alone), followed by the
/// skill's files the occupant lacks. Read-only, through the same listing as
/// the file browser (links inside are never followed).
///
/// The occupant's listing is bounded; whether it has each of the skill's
/// files is probed directly (under [`read_file`]'s containment rule), so a
/// file cut by the bound is never reported missing — it is listed, compared,
/// after the bounded part.
pub fn list_occupant_files(occupant: &Path, skill: Option<&Path>) -> Result<OccupantFileListing> {
    // The folder in the way is anyone's: a link there may lead to a large
    // tree, so the listing is bounded (a skill is a handful of files).
    let (theirs, truncated) = list_files_within(occupant, OCCUPANT_FILES_MAX)?;
    let ours = match skill {
        Some(skill) if skill.is_dir() => list_files(skill)?,
        _ => Vec::new(),
    };
    let compare = |path: &str| match skill {
        Some(skill) => match same_bytes(&occupant.join(path), &skill.join(path)) {
            Ok(true) => FileComparison::Same,
            _ => FileComparison::Changed,
        },
        None => FileComparison::OnlyHere,
    };
    let mut entries: Vec<OccupantFileEntry> = theirs
        .iter()
        .map(|file| OccupantFileEntry {
            path: file.path.clone(),
            size: file.size,
            compared: if ours.iter().any(|mine| mine.path == file.path) {
                compare(&file.path)
            } else {
                FileComparison::OnlyHere
            },
        })
        .collect();
    for mine in ours {
        if theirs.iter().any(|file| file.path == mine.path) {
            continue;
        }
        // Not among the listed files: the occupant may still have it, past
        // the bound (or as a link the walk does not follow).
        entries.push(match contained_file(occupant, &mine.path) {
            Some(at) => OccupantFileEntry {
                size: std::fs::metadata(at).map(|m| m.len()).unwrap_or(mine.size),
                compared: compare(&mine.path),
                path: mine.path,
            },
            None => OccupantFileEntry {
                path: mine.path,
                size: mine.size,
                compared: FileComparison::OnlyInSkill,
            },
        });
    }
    Ok(OccupantFileListing { entries, truncated })
}

/// Whether two files hold the same bytes: lengths first (different lengths
/// are never read), then both streamed through fixed-size buffers — no size
/// limit, and never a whole file in memory (display has its own, [`read_file`]).
fn same_bytes(a: &Path, b: &Path) -> std::io::Result<bool> {
    use std::io::Read;
    if std::fs::metadata(a)?.len() != std::fs::metadata(b)?.len() {
        return Ok(false);
    }
    let (mut a, mut b) = (std::fs::File::open(a)?, std::fs::File::open(b)?);
    let mut buf_a = vec![0u8; 64 * 1024];
    let mut buf_b = vec![0u8; 64 * 1024];
    loop {
        let n = fill(&mut a, &mut buf_a)?;
        if n != fill(&mut b, &mut buf_b)? || buf_a[..n] != buf_b[..n] {
            return Ok(false);
        }
        if n == 0 {
            return Ok(true);
        }
    }

    /// Reads until `buf` is full or the file ends; the bytes read.
    fn fill(file: &mut std::fs::File, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut read = 0;
        while read < buf.len() {
            match file.read(&mut buf[read..]) {
                Ok(0) => break,
                Ok(n) => read += n,
                Err(err) if err.kind() == std::io::ErrorKind::Interrupted => {}
                Err(err) => return Err(err),
            }
        }
        Ok(read)
    }
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
