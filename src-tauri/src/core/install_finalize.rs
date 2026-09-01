//! The single place where staged skill bytes become a managed skill.
//!
//! Every install flow (local path, git URL, git/local selection) does the same
//! last mile: pick the final directory name (SKILL.md's `name` wins over a
//! derived name), refuse collisions, move the bytes into the central repo,
//! read the description, hash the content, and record the `SkillRecord`.
//! Flows own only the *acquire* half — put the bytes into a [`StagingDir`],
//! then hand it to [`finalize_install`]. The update flow stages the same way
//! and hands off to [`finalize_update`], which swaps the content in place and
//! keeps the record's identity.
//!
//! Collisions are raised as `SignalError::SkillExists { name }` so the command
//! seam maps them to a typed wire variant; no message here is user copy.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use uuid::Uuid;

use super::content_hash::hash_dir;
use super::errors::SignalError;
use super::installer::now_ms;
use super::skill_discovery::{find_skill_md, parse_skill_md};
use super::skill_store::{SkillRecord, SkillStore};
use super::sync_engine::copy_dir_recursive;

/// Outcome of a completed install.
#[derive(Debug)]
pub struct InstallResult {
    pub skill_id: String,
    pub name: String,
    pub central_path: PathBuf,
    pub content_hash: Option<String>,
}

/// How the caller arrived at the requested skill name. A derived name (from a
/// URL, subpath, or folder) yields to SKILL.md's `name` when that is free; a
/// user-provided name is always honored (fixes #28: a subpath of `skills`
/// otherwise collides with tool directory names).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NameIntent {
    UserProvided(String),
    Derived(String),
}

impl NameIntent {
    /// Name the caller asked for, before SKILL.md preference is applied.
    pub fn requested(&self) -> &str {
        match self {
            NameIntent::UserProvided(name) | NameIntent::Derived(name) => name,
        }
    }
}

/// Where the staged bytes came from, as recorded on the `SkillRecord`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillProvenance {
    pub source_type: String,
    pub source_ref: Option<String>,
    pub source_subpath: Option<String>,
    pub source_revision: Option<String>,
}

impl SkillProvenance {
    /// A skill copied from a local directory.
    pub fn local(source_path: &Path) -> Self {
        SkillProvenance {
            source_type: "local".to_string(),
            source_ref: Some(source_path.to_string_lossy().to_string()),
            source_subpath: None,
            source_revision: None,
        }
    }

    /// A skill fetched from a git repository. `revision` is `None` when the
    /// bytes came via the GitHub API download path (no commit is known).
    pub fn git(repo_url: &str, subpath: Option<String>, revision: Option<String>) -> Self {
        SkillProvenance {
            source_type: "git".to_string(),
            source_ref: Some(repo_url.to_string()),
            source_subpath: subpath,
            source_revision: Some(revision.unwrap_or_else(|| "api-download".to_string())),
        }
    }
}

/// A scratch directory inside the central repo that acquire steps fill and
/// finalize consumes by renaming it into place. Living as a sibling of the
/// final path keeps the move on one filesystem. If it is dropped unconsumed
/// (any failure between staging and finalize), it is removed.
#[derive(Debug)]
pub struct StagingDir {
    path: PathBuf,
}

impl StagingDir {
    /// Reserve a fresh, unique staging path under `central_dir`. The directory
    /// itself is not created; acquire steps create it as they write.
    pub fn new_in(central_dir: &Path) -> Self {
        StagingDir {
            path: central_dir.join(format!(".skills-hub-staging-{}", Uuid::new_v4())),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Move the staged content to `dest` (copy + delete on cross-device rename
    /// failure). The guard is consumed; nothing is left at the staging path.
    fn move_into(self, dest: &Path) -> Result<()> {
        if let Err(err) = std::fs::rename(&self.path, dest) {
            copy_dir_recursive(&self.path, dest)
                .with_context(|| format!("fallback copy {:?} -> {:?}", self.path, dest))?;
            log::warn!(
                "[install] rename {:?} -> {:?} failed, copied instead: {}",
                self.path,
                dest,
                err
            );
        }
        Ok(())
    }
}

impl Drop for StagingDir {
    fn drop(&mut self) {
        if self.path.exists() {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

/// Resolve `central_dir/name`, raising the typed `SkillExists` collision when
/// something already lives there. Flows call this before acquiring bytes so a
/// doomed install never downloads; [`finalize_install`] re-checks it as the
/// authority.
pub fn ensure_name_available(central_dir: &Path, name: &str) -> Result<PathBuf> {
    let central_path = central_dir.join(name);
    if central_path.exists() {
        anyhow::bail!(SignalError::SkillExists {
            name: name.to_string(),
        });
    }
    Ok(central_path)
}

/// Materialize a staged skill as a new managed skill: resolve the final name,
/// move the bytes into the central repo, and record it.
pub fn finalize_install(
    store: &SkillStore,
    central_dir: &Path,
    staged: StagingDir,
    name: NameIntent,
    provenance: SkillProvenance,
) -> Result<InstallResult> {
    let requested_path = ensure_name_available(central_dir, name.requested())?;
    let (md_name, description) = read_skill_md_meta(staged.path());

    let (name, central_path) = match (&name, md_name) {
        (NameIntent::Derived(requested), Some(better)) if better != *requested => {
            let better_path = central_dir.join(&better);
            if better_path.exists() {
                (requested.clone(), requested_path)
            } else {
                (better, better_path)
            }
        }
        _ => (name.requested().to_string(), requested_path),
    };

    staged.move_into(&central_path)?;

    let now = now_ms();
    let content_hash = compute_content_hash(&central_path);
    let record = SkillRecord {
        id: Uuid::new_v4().to_string(),
        name,
        description,
        source_type: provenance.source_type,
        source_ref: provenance.source_ref,
        source_subpath: provenance.source_subpath,
        source_revision: provenance.source_revision,
        central_path: central_path.to_string_lossy().to_string(),
        content_hash: content_hash.clone(),
        created_at: now,
        updated_at: now,
        last_sync_at: None,
        last_seen_at: now,
        status: "ok".to_string(),
    };
    store.upsert_skill(&record)?;

    Ok(InstallResult {
        skill_id: record.id,
        name: record.name,
        central_path,
        content_hash,
    })
}

/// Replace a managed skill's content with the staged bytes and refresh its
/// record (identity, name, provenance, and timestamps other than `updated_at`
/// are preserved; `revision` overrides the stored one when known). Returns
/// the upserted record.
pub fn finalize_update(
    store: &SkillStore,
    record: &SkillRecord,
    staged: StagingDir,
    revision: Option<String>,
) -> Result<SkillRecord> {
    let central_path = PathBuf::from(&record.central_path);
    std::fs::remove_dir_all(&central_path)
        .with_context(|| format!("failed to remove old central dir {:?}", central_path))?;
    staged.move_into(&central_path)?;

    let now = now_ms();
    let content_hash = compute_content_hash(&central_path);
    let (_, description) = read_skill_md_meta(&central_path);
    let updated = SkillRecord {
        description: description.or_else(|| record.description.clone()),
        source_revision: revision.or_else(|| record.source_revision.clone()),
        content_hash,
        updated_at: now,
        last_seen_at: now,
        status: "ok".to_string(),
        ..record.clone()
    };
    store.upsert_skill(&updated)?;
    Ok(updated)
}

/// `(name, description)` from the directory's SKILL.md frontmatter, if any.
fn read_skill_md_meta(dir: &Path) -> (Option<String>, Option<String>) {
    match find_skill_md(dir).and_then(|md| parse_skill_md(&md)) {
        Some((name, description)) => (Some(name), description),
        None => (None, None),
    }
}

fn compute_content_hash(path: &Path) -> Option<String> {
    if should_compute_content_hash() {
        hash_dir(path).ok()
    } else {
        None
    }
}

fn should_compute_content_hash() -> bool {
    if cfg!(debug_assertions) {
        return true;
    }
    std::env::var("SKILLS_HUB_COMPUTE_HASH")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

#[cfg(test)]
#[path = "tests/install_finalize.rs"]
mod tests;
