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

use super::clock::now_ms;
use super::content_identity;
use super::errors::SignalError;
use super::provenance::Provenance;
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
///
/// The variants name the *policy*, not the provenance: `UserProvided` means
/// "honor this name as-is", `Derived` means "prefer SKILL.md's name over it".
/// That is why the local-install flows pass a folder-derived name as
/// `UserProvided` — for a directory on the operator's disk the folder name *is*
/// the skill's identity (it is what every tool already shows), so renaming it
/// behind the operator's back would be the surprise. The local *selection*
/// flow does apply the SKILL.md preference, but earlier: it parses the manifest
/// itself and passes the resulting name here (`installer.rs`), because it must
/// reject an unparseable `SKILL.md` before staging anything.
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

/// Where the staged bytes came from, as recorded on the `SkillRecord`. One
/// of the three Provenances (`core::provenance`): `git`, `local`, `imported`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillProvenance {
    pub source_type: String,
    pub source_ref: Option<String>,
    pub source_subpath: Option<String>,
    pub source_revision: Option<String>,
    /// Display-only history for an `imported` skill: the Tool it was found
    /// in. Never a source.
    pub imported_from_tool: Option<String>,
}

impl SkillProvenance {
    /// A skill copied from an independent local directory the operator
    /// maintains; that directory stays its source.
    pub fn local(source_path: &Path) -> Self {
        SkillProvenance {
            source_type: Provenance::Local.as_str().to_string(),
            source_ref: Some(source_path.to_string_lossy().to_string()),
            source_subpath: None,
            source_revision: None,
            imported_from_tool: None,
        }
    }

    /// A skill fetched from a git repository. `revision` is `None` when the
    /// bytes came via the GitHub API download path (no commit is known).
    pub fn git(repo_url: &str, subpath: Option<String>, revision: Option<String>) -> Self {
        SkillProvenance {
            source_type: Provenance::Git.as_str().to_string(),
            source_ref: Some(repo_url.to_string()),
            source_subpath: subpath,
            source_revision: Some(revision.unwrap_or_else(|| "api-download".to_string())),
            imported_from_tool: None,
        }
    }

    /// A skill taken over from a Tool's skills directory. It has no external
    /// source — the central copy is its truth (ADR-0003) — so `source_ref`
    /// is `None`; `found_in_tool` is kept as display-only history when the
    /// import knows it — never an empty placeholder.
    pub fn imported(found_in_tool: Option<&str>) -> Self {
        SkillProvenance {
            source_type: Provenance::Imported.as_str().to_string(),
            source_ref: None,
            source_subpath: None,
            source_revision: None,
            imported_from_tool: found_in_tool.map(str::to_string),
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
    ///
    /// A failed fallback copy removes the partial `dest` before propagating:
    /// `Drop` only cleans the staging path, so without this a half-written
    /// skill directory would be left inside the central repo under the skill's
    /// final name — where the next install reads it as a name collision.
    fn move_into(self, dest: &Path) -> Result<()> {
        if let Err(err) = std::fs::rename(&self.path, dest) {
            if let Err(copy_err) = copy_dir_recursive(&self.path, dest) {
                if let Err(cleanup_err) = std::fs::remove_dir_all(dest) {
                    if cleanup_err.kind() != std::io::ErrorKind::NotFound {
                        log::warn!(
                            "[install] failed to remove partial {:?} after copy failure: {}",
                            dest,
                            cleanup_err
                        );
                    }
                }
                return Err(copy_err)
                    .with_context(|| format!("fallback copy {:?} -> {:?}", self.path, dest));
            }
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
    let mut record = SkillRecord {
        id: Uuid::new_v4().to_string(),
        name,
        description,
        source_type: provenance.source_type,
        source_ref: provenance.source_ref,
        source_subpath: provenance.source_subpath,
        source_revision: provenance.source_revision,
        central_path: central_path.to_string_lossy().to_string(),
        content_hash: None,
        created_at: now,
        updated_at: now,
        last_sync_at: None,
        last_seen_at: now,
        status: "ok".to_string(),
        imported_from_tool: provenance.imported_from_tool,
    };
    if let Err(err) = content_identity::record(store, &mut record) {
        return Err(match std::fs::remove_dir_all(&central_path) {
            Ok(()) => err,
            Err(cleanup_err) if cleanup_err.kind() == std::io::ErrorKind::NotFound => err,
            Err(cleanup_err) => err.context(format!(
                "failed to remove untracked install retained at {:?}: {}",
                central_path, cleanup_err
            )),
        });
    }

    Ok(InstallResult {
        skill_id: record.id,
        name: record.name,
        central_path,
        content_hash: record.content_hash,
    })
}

/// Replace a managed skill's content with the staged bytes and refresh its
/// record (identity, name, provenance, and timestamps other than `updated_at`
/// are preserved; `revision` overrides the stored one when known). Returns
/// the settled record and the step's result.
///
/// A central copy that is already gone is not a failure: the staged bytes
/// simply land at the recorded path — that is how Restore rebuilds an
/// Unlocatable skill's central copy.
///
/// Keep the previous bytes beside the destination until the row and step are
/// settled. Failures restore bytes and the previous row; a failed byte rollback
/// retains and names the backup. This is failure-atomic, not crash-atomic.
///
/// Settle derived state while the previous bytes are still recoverable. The
/// step owns rollback of its auxiliary rows; finalize owns bytes and the skill
/// row. Snapshot the persisted row before landing: acquisition may have
/// overridden provenance in its input record (Re-point).
pub fn finalize_update<T>(
    store: &SkillStore,
    record: &SkillRecord,
    staged: StagingDir,
    revision: Option<String>,
    settle: impl FnOnce(&mut SkillRecord) -> Result<T>,
) -> Result<(SkillRecord, T)> {
    let previous = store
        .get_skill_by_id(&record.id)?
        .context("skill missing before finalize")?;
    let central_path = PathBuf::from(&record.central_path);
    let backup = move_old_central_aside(&central_path)?;
    if let Err(err) = staged.move_into(&central_path) {
        return Err(rollback_update(&central_path, backup.as_deref(), err));
    }

    let now = now_ms();
    let (_, description) = read_skill_md_meta(&central_path);
    let mut updated = SkillRecord {
        description: description.or_else(|| record.description.clone()),
        source_revision: revision.or_else(|| record.source_revision.clone()),
        content_hash: None,
        updated_at: now,
        last_seen_at: now,
        status: "ok".to_string(),
        ..record.clone()
    };
    if let Err(err) = content_identity::record(store, &mut updated) {
        return Err(rollback_update(&central_path, backup.as_deref(), err));
    }
    let settled = match settle(&mut updated) {
        Ok(settled) => settled,
        Err(err) => {
            let err = rollback_update(&central_path, backup.as_deref(), err);
            return Err(match store.upsert_skill(&previous) {
                Ok(()) => err,
                Err(restore_err) => err.context(format!(
                    "restore pre-finalize skill row {} failed: {restore_err:#}",
                    record.id
                )),
            });
        }
    };
    if let Some(backup) = backup {
        if let Err(err) = std::fs::remove_dir_all(&backup) {
            // Bytes and row already agree: cleanup must not turn success into failure.
            log::warn!(
                "[install] failed to remove old central backup {:?}: {}",
                backup,
                err
            );
        }
    }
    Ok((updated, settled))
}

fn move_old_central_aside(central: &Path) -> Result<Option<PathBuf>> {
    sweep_old_central_backups(central);
    match std::fs::symlink_metadata(central) {
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err).with_context(|| format!("inspect central dir {:?}", central)),
    }
    // A hidden UUID sibling stays outside ordinary skill discovery, unlike
    // `<name>.old-<timestamp>`. Check even UUID collisions (including dangling
    // symlinks), so no existing skill/recovery directory can be overwritten.
    // Finalize runs under the caller's mutation guard, like staging/install.
    let backup = loop {
        let candidate = central.with_file_name(format!(".skills-hub-old-{}", Uuid::new_v4()));
        match std::fs::symlink_metadata(&candidate) {
            Ok(_) => continue,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => break candidate,
            Err(err) => {
                return Err(err).with_context(|| format!("inspect backup path {:?}", candidate))
            }
        }
    };
    std::fs::rename(central, &backup)
        .with_context(|| format!("move old central dir {:?} aside to {:?}", central, backup))?;
    stamp_backup_created(&backup);
    Ok(Some(backup))
}

/// `rename` preserves the directory's mtime, which would date the backup from
/// the skill's last content change rather than from now — and the sweep's
/// recovery window is measured from creation. Best-effort: a backup that
/// cannot be stamped keeps its old mtime and may be swept early; that is
/// logged, never a finalize failure.
fn stamp_backup_created(backup: &Path) {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_FLAG_BACKUP_SEMANTICS opens a directory handle; FILE_WRITE_ATTRIBUTES
        // grants SetFileTime without requesting directory-content writes.
        options.custom_flags(0x02000000).access_mode(0x100);
    }
    let stamped = options
        .open(backup)
        .and_then(|file| file.set_modified(std::time::SystemTime::now()));
    if let Err(err) = stamped {
        log::warn!(
            "[install] could not stamp backup {:?} creation time: {}",
            backup,
            err
        );
    }
}

/// Best-effort, parent-local recovery cleanup; inspect links themselves, never
/// their targets. Recent backups remain available for manual recovery.
fn sweep_old_central_backups(central: &Path) {
    let Some(parent) = central.parent() else {
        return;
    };
    let entries = match std::fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(err) => {
            log::warn!(
                "[install] cannot sweep old central backups in {:?}: {}",
                parent,
                err
            );
            return;
        }
    };
    let now = std::time::SystemTime::now();
    let max_age = std::time::Duration::from_secs(7 * 24 * 60 * 60);
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                log::warn!(
                    "[install] cannot inspect backup sibling in {:?}: {}",
                    parent,
                    err
                );
                continue;
            }
        };
        if !entry
            .file_name()
            .to_string_lossy()
            .starts_with(".skills-hub-old-")
        {
            continue;
        }
        let path = entry.path();
        let Ok(metadata) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        if !now.duration_since(modified).is_ok_and(|age| age > max_age) {
            continue;
        }
        let removal = if metadata.is_dir() || metadata.is_symlink() {
            // remove_dir_all removes a symlink itself without following it,
            // including directory links on Windows (where remove_file fails).
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        if let Err(err) = removal {
            log::warn!(
                "[install] failed to remove old central backup {:?}: {}",
                path,
                err
            );
        }
    }
}

/// Preserve the original failure as the error's source, even if recovery fails.
fn rollback_update(
    central: &Path,
    backup: Option<&Path>,
    original: anyhow::Error,
) -> anyhow::Error {
    match roll_back_update(central, backup) {
        Ok(()) => original,
        Err(err) => original.context(format!("rollback: {err:#}")).context(
            SignalError::FinalizeRollbackFailed {
                central: central.to_string_lossy().into_owned(),
                backup: backup.map(|path| path.to_string_lossy().into_owned()),
            },
        ),
    }
}

/// Remove partial replacement bytes and restore the previous central copy.
fn roll_back_update(central: &Path, backup: Option<&Path>) -> Result<()> {
    // move_into normally removes a failed partial copy, but cleanup can fail.
    // Retry before restoring so old bytes never merge with partial new bytes.
    match std::fs::remove_dir_all(central) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err).context("remove replacement central dir"),
    }
    if let Some(backup) = backup {
        std::fs::rename(backup, central).context("restore old central dir")?;
    }
    Ok(())
}

/// `(name, description)` from the directory's SKILL.md frontmatter, if any.
fn read_skill_md_meta(dir: &Path) -> (Option<String>, Option<String>) {
    match find_skill_md(dir).and_then(|md| parse_skill_md(&md)) {
        Some((name, description)) => (Some(name), description),
        None => (None, None),
    }
}

#[cfg(test)]
#[path = "tests/install_finalize.rs"]
mod tests;
