use std::path::Path;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use walkdir::{DirEntry, WalkDir};

const IGNORE_NAMES: [&str; 4] = [".git", ".DS_Store", "Thumbs.db", ".gitignore"];

fn is_ignored(entry: &DirEntry) -> bool {
    let file_name = entry.file_name().to_string_lossy();
    IGNORE_NAMES.iter().any(|name| name == &file_name.as_ref())
        || file_name.starts_with(".skills-hub-manifest-")
}

/// Hash skill content, excluding internal symlinks entirely: neither their names
/// nor their targets contribute to identity. Links are never followed, matching
/// `sync_engine::copy_dir_recursive`'s exclusion from copies.
/// A skill consisting only of symlinks hashes as empty.
pub fn hash_dir(path: &Path) -> Result<String> {
    let mut hasher = Sha256::new();

    for entry in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !is_ignored(entry))
    {
        let entry = entry?;
        if is_ignored(&entry) || entry.file_type().is_symlink() {
            continue;
        }

        let relative = entry
            .path()
            .strip_prefix(path)
            .with_context(|| format!("strip prefix {:?}", entry.path()))?;
        hasher.update(relative.to_string_lossy().as_bytes());

        if entry.file_type().is_file() {
            let bytes = std::fs::read(entry.path())
                .with_context(|| format!("read file {:?}", entry.path()))?;
            hasher.update(bytes);
        }
    }

    let digest = hasher.finalize();
    Ok(hex::encode(digest))
}

#[cfg(test)]
#[path = "tests/content_hash.rs"]
mod tests;
