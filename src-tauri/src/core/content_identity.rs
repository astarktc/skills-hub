use std::path::Path;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use super::{
    skill_files::is_ignored,
    skill_store::{SkillRecord, SkillStore},
};

/// Managed copies trust the row. Unmanaged onboarding candidates have no row yet.
#[derive(Clone, Copy)]
pub enum Source<'a> {
    Managed {
        store: &'a SkillStore,
        skill_id: &'a str,
    },
    Directory(&'a Path),
}

/// Record freshly landed bytes together with the supplied skill record.
/// Hash I/O failure leaves identity unknown; a database failure remains fatal.
pub fn record(store: &SkillStore, record: &mut SkillRecord) -> Result<()> {
    record.content_hash = read(Source::Directory(Path::new(&record.central_path)));
    store.upsert_skill(record)
}

/// Read identity, backfilling a missing managed hash once. Unknown identity must
/// not be interpreted as drift. This is the only warning site for read failures,
/// and it warns on every failure: reconcile may ask repeatedly while an I/O
/// fault persists, so a persistent fault repeats in the log. That flood is
/// accepted over keeping process-global dedupe state in core.
pub fn read(source: Source<'_>) -> Option<String> {
    fn resolve(source: Source<'_>) -> Result<Option<String>> {
        match source {
            Source::Directory(path) => hash_dir(path).map(Some),
            Source::Managed { store, skill_id } => {
                let Some(record) = store.get_skill_by_id(skill_id)? else {
                    return Ok(None);
                };
                if record.content_hash.is_some() {
                    return Ok(record.content_hash);
                }
                let hash = hash_dir(Path::new(&record.central_path))?;
                store.update_skill_content_hash(skill_id, &hash)?;
                Ok(Some(hash))
            }
        }
    }
    match resolve(source) {
        Ok(hash) => hash,
        Err(error) => {
            log::warn!("[content identity] identity unavailable: {error:#}");
            None
        }
    }
}

/// What comparing two identities established. Only `Same` and `Different`
/// are answers; `Unknown` (either side absent or unreadable) is not drift and
/// never authorizes a destructive step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Comparison {
    Same,
    Different,
    Unknown,
}

/// Compare a source with a target directory. Targets are always re-hashed; a
/// managed source goes through the stored read. A missing target (a dangling
/// link included) is `Unknown`.
pub fn compare(source: Source<'_>, target: &Path) -> Comparison {
    if !target.exists() {
        return Comparison::Unknown;
    }
    match (read(source), read(Source::Directory(target))) {
        (Some(source), Some(target)) if source == target => Comparison::Same,
        (Some(_), Some(_)) => Comparison::Different,
        _ => Comparison::Unknown,
    }
}

/// Compare a target directory with an identity recorded earlier — what a
/// Deployment row recorded as deployed (`core::ownership`). The target is
/// re-hashed now; a missing or unreadable target is `Unknown`, never a match.
pub fn compare_recorded(recorded: &str, target: &Path) -> Comparison {
    if !target.is_dir() {
        return Comparison::Unknown;
    }
    match hash_dir(target) {
        Ok(hash) if hash == recorded => Comparison::Same,
        Ok(_) => Comparison::Different,
        Err(error) => {
            log::warn!("[content identity] identity of {target:?} unavailable: {error:#}");
            Comparison::Unknown
        }
    }
}

/// The conservative two-state answer: `true` only for a known `Same`, so an
/// unknown identity reads as "not the same" — right for keep / refuse-to-
/// overwrite decisions, never for a deletion (use [`compare`]).
pub fn same_content(source: Source<'_>, target: &Path) -> bool {
    compare(source, target) == Comparison::Same
}

/// Hash skill content, excluding internal symlinks entirely: neither their names
/// nor their targets contribute to identity. Links are never followed, matching
/// `sync_engine::copy_dir_recursive`'s exclusion from copies.
/// A skill consisting only of symlinks hashes as empty.
fn hash_dir(path: &Path) -> Result<String> {
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
#[path = "tests/content_identity.rs"]
mod tests;
