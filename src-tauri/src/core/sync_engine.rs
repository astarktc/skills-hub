use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::content_identity::{self, Source};
use super::errors::SignalError;
use super::tool_adapters::ToolAdapter;

/// Typed condition: the sync target already exists and overwrite was not
/// permitted. Raised through `anyhow` chains so callers (e.g.
/// `global_sync::classify_sync_error`) recover it by downcast — never by
/// matching the message text (see ADR 0001).
#[derive(Debug)]
pub struct TargetExistsError {
    pub target: PathBuf,
    /// What the overwrite ask shows about the folder in the way (D12),
    /// observed where the collision is raised so every producer's
    /// `TARGET_EXISTS` row carries it.
    pub occupant: Occupant,
}

/// The folder in a sync target's way, as the overwrite ask displays it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Occupant {
    /// The existing folder's modification time (ms since the epoch); `None`
    /// when the filesystem cannot say.
    pub modified_at: Option<i64>,
    /// Whether its content is byte-identical to the skill's (the content
    /// identity rule). The frontend never re-derives it.
    pub same_content: bool,
}

impl Occupant {
    /// Observe the occupant of `target` against the skill's identity. A
    /// Managed skill's identity is its stored one (`Source::Managed`, the
    /// same read the overwrite policy decided with), never a re-hash of the
    /// central copy; `Source::Directory` is for genuinely unmanaged bytes.
    pub fn observe(identity: Source<'_>, target: &Path) -> Self {
        let modified_at = std::fs::metadata(target)
            .or_else(|_| std::fs::symlink_metadata(target))
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|since| since.as_millis() as i64);
        Self {
            modified_at,
            same_content: content_identity::same_content(identity, target),
        }
    }
}

impl TargetExistsError {
    fn at(identity: Source<'_>, target: &Path) -> Self {
        Self {
            target: target.to_path_buf(),
            occupant: Occupant::observe(identity, target),
        }
    }
}

impl fmt::Display for TargetExistsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "target already exists: {:?}", self.target)
    }
}

impl std::error::Error for TargetExistsError {}

/// The mode actually used for a target — defined with the rest of the
/// lifecycle vocabulary in `sync_status`, re-exported here for the engine's
/// callers.
pub use super::sync_status::SyncMode;

#[derive(Clone, Debug, serde::Serialize, specta::Type)]
pub struct SyncOutcome {
    pub mode_used: SyncMode,
    pub target_path: PathBuf,
    pub replaced: bool,
}

/// Where a sync may create the directories a target needs.
#[derive(Clone, Copy, Debug)]
enum TargetParents<'a> {
    /// Create every missing ancestor — a Tool's global skills dir may
    /// legitimately not exist yet.
    CreateMissing,
    /// Create only descendants of a root that must already be a directory
    /// at creation time ([`create_dirs_within`]) — a Project target, whose
    /// root is never recreated.
    WithinRoot(&'a Path),
}

impl TargetParents<'_> {
    fn create(self, dir: &Path) -> Result<()> {
        match self {
            TargetParents::CreateMissing => {
                std::fs::create_dir_all(dir).with_context(|| format!("create dir {:?}", dir))
            }
            TargetParents::WithinRoot(root) => create_dirs_within(root, dir),
        }
    }
}

/// `identity` names the skill whose bytes sit at `source`; it is read only
/// when a folder is in the way, for the ask's `same_content` fact.
#[cfg(test)]
pub fn sync_dir_hybrid(source: &Path, identity: Source<'_>, target: &Path) -> Result<SyncOutcome> {
    hybrid(source, identity, target, TargetParents::CreateMissing)
}

fn hybrid(
    source: &Path,
    identity: Source<'_>,
    target: &Path,
    parents: TargetParents<'_>,
) -> Result<SyncOutcome> {
    if target.exists() {
        if is_same_link(target, source) {
            return Ok(SyncOutcome {
                mode_used: SyncMode::Symlink,
                target_path: target.to_path_buf(),
                replaced: false,
            });
        }
        anyhow::bail!(TargetExistsError::at(identity, target));
    }

    ensure_parent_dir(target, parents)?;

    if try_link_dir(source, target).is_ok() {
        return Ok(SyncOutcome {
            mode_used: SyncMode::Symlink,
            target_path: target.to_path_buf(),
            replaced: false,
        });
    }

    #[cfg(windows)]
    if try_junction(source, target).is_ok() {
        return Ok(SyncOutcome {
            mode_used: SyncMode::Junction,
            target_path: target.to_path_buf(),
            replaced: false,
        });
    }

    copy_dir_with_parents(source, target, parents)?;
    Ok(SyncOutcome {
        mode_used: SyncMode::Copy,
        target_path: target.to_path_buf(),
        replaced: false,
    })
}

#[cfg(test)]
pub fn sync_dir_hybrid_with_overwrite(
    source: &Path,
    identity: Source<'_>,
    target: &Path,
    overwrite: bool,
) -> Result<SyncOutcome> {
    hybrid_with_overwrite(
        source,
        identity,
        target,
        overwrite,
        TargetParents::CreateMissing,
    )
}

fn hybrid_with_overwrite(
    source: &Path,
    identity: Source<'_>,
    target: &Path,
    overwrite: bool,
    parents: TargetParents<'_>,
) -> Result<SyncOutcome> {
    // A missing source must fail like it does in copy mode; without this a
    // dangling symlink would be created and recorded as a successful sync.
    if !source.is_dir() {
        anyhow::bail!(SignalError::InvalidPath {
            path: source.to_string_lossy().into_owned(),
            reason: "missing".to_string(),
        });
    }
    let mut did_replace = false;
    if std::fs::symlink_metadata(target).is_ok() {
        if is_same_link(target, source) {
            return Ok(SyncOutcome {
                mode_used: SyncMode::Symlink,
                target_path: target.to_path_buf(),
                replaced: false,
            });
        }

        if overwrite {
            remove_path_any(target)
                .with_context(|| format!("remove existing target {:?}", target))?;
            did_replace = true;
        } else {
            anyhow::bail!(TargetExistsError::at(identity, target));
        }
    }

    // reuse normal flow
    hybrid(source, identity, target, parents).map(|mut out| {
        out.replaced = did_replace;
        out
    })
}

#[cfg(test)]
pub fn sync_dir_copy_with_overwrite(
    source: &Path,
    identity: Source<'_>,
    target: &Path,
    overwrite: bool,
) -> Result<SyncOutcome> {
    copy_with_overwrite(
        source,
        identity,
        target,
        overwrite,
        TargetParents::CreateMissing,
    )
}

fn copy_with_overwrite(
    source: &Path,
    identity: Source<'_>,
    target: &Path,
    overwrite: bool,
    parents: TargetParents<'_>,
) -> Result<SyncOutcome> {
    let mut did_replace = false;
    if std::fs::symlink_metadata(target).is_ok() {
        if overwrite {
            remove_path_any(target)
                .with_context(|| format!("remove existing target {:?}", target))?;
            did_replace = true;
        } else {
            anyhow::bail!(TargetExistsError::at(identity, target));
        }
    }

    ensure_parent_dir(target, parents)?;
    copy_dir_with_parents(source, target, parents)?;

    Ok(SyncOutcome {
        mode_used: SyncMode::Copy,
        target_path: target.to_path_buf(),
        replaced: did_replace,
    })
}

/// The tool-aware sync entry point: tools that cannot consume a symlinked
/// skills dir (`ToolAdapter::supports_symlink == false`) are always copied so
/// the skills stay usable after sync; every other tool gets the hybrid
/// symlink -> junction -> copy fallback. `identity` is the skill's content
/// identity (`Source::Managed` for a Managed skill), read only to describe
/// a folder in the way (`TargetExistsError::occupant`).
pub fn sync_dir_for_tool_with_overwrite(
    adapter: &ToolAdapter,
    source: &Path,
    identity: Source<'_>,
    target: &Path,
    overwrite: bool,
) -> Result<SyncOutcome> {
    sync_dir_for_tool(
        adapter,
        source,
        identity,
        target,
        overwrite,
        TargetParents::CreateMissing,
    )
}

/// [`sync_dir_for_tool_with_overwrite`] for a target beneath `root` (a
/// Project root): every directory the write needs — the target's parents and
/// the copy fallback's own tree — is created through [`create_dirs_within`],
/// so a root that disappears at any point before a creation is refused
/// (`InvalidPath` `missing`) instead of being recreated. Nothing is written
/// outside `root`.
pub(crate) fn sync_dir_for_tool_within_root(
    adapter: &ToolAdapter,
    source: &Path,
    identity: Source<'_>,
    target: &Path,
    overwrite: bool,
    root: &Path,
) -> Result<SyncOutcome> {
    sync_dir_for_tool(
        adapter,
        source,
        identity,
        target,
        overwrite,
        TargetParents::WithinRoot(root),
    )
}

fn sync_dir_for_tool(
    adapter: &ToolAdapter,
    source: &Path,
    identity: Source<'_>,
    target: &Path,
    overwrite: bool,
    parents: TargetParents<'_>,
) -> Result<SyncOutcome> {
    if !adapter.supports_symlink {
        return copy_with_overwrite(source, identity, target, overwrite, parents);
    }
    hybrid_with_overwrite(source, identity, target, overwrite, parents)
}

fn ensure_parent_dir(path: &Path, parents: TargetParents<'_>) -> Result<()> {
    if let Some(parent) = path.parent() {
        parents.create(parent)?;
    }
    Ok(())
}

/// Create `dir` and its missing ancestors **below** `root`, never `root`
/// itself or anything above it. `root` must be a directory (a link to one
/// counts) when the walk starts, and each level is made by one
/// non-recursive `mkdir`, so a root that disappears at any moment — before
/// the walk or between two levels — makes the next `mkdir` fail rather than
/// recreate it. That refusal, like a root already gone, is the typed
/// `InvalidPath { path: root, reason: "missing" }` (an intermediate level that
/// vanished under a present root names that level instead). A `dir` that is
/// not a plain descendant of `root` is a caller bug and refused with prose.
/// An existing level must be a directory; anything else in the way is an
/// I/O error, as with `create_dir_all`.
pub(crate) fn create_dirs_within(root: &Path, dir: &Path) -> Result<()> {
    create_dirs_within_observed(root, dir, &mut |_level| {
        #[cfg(test)]
        test_hooks::before_bounded_mkdir(_level);
    })
}

/// [`create_dirs_within`] with `before_mkdir` called just before each
/// level's `mkdir` — the seam its tests use to make a root vanish exactly
/// at creation time.
fn create_dirs_within_observed(
    root: &Path,
    dir: &Path,
    before_mkdir: &mut dyn FnMut(&Path),
) -> Result<()> {
    let relative = dir
        .strip_prefix(root)
        .map_err(|_| anyhow::anyhow!("refusing to create {:?} outside {:?}", dir, root))?;
    if relative
        .components()
        .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        anyhow::bail!("refusing to create {:?}: it escapes {:?}", dir, root);
    }
    let missing = |path: &Path| SignalError::InvalidPath {
        path: path.to_string_lossy().into_owned(),
        reason: "missing".to_string(),
    };
    if !root.is_dir() {
        anyhow::bail!(missing(root));
    }
    let mut level = root.to_path_buf();
    for component in relative.components() {
        let parent = level.clone();
        level.push(component);
        before_mkdir(&level);
        match std::fs::create_dir(&level) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists && level.is_dir() => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                anyhow::bail!(missing(if root.is_dir() { &parent } else { root }));
            }
            Err(err) => return Err(err).with_context(|| format!("create dir {:?}", level)),
        }
    }
    Ok(())
}

/// Test-only hook into [`create_dirs_within`]: lets a test make a Project
/// root disappear at the exact moment a sync creates a directory under it.
#[cfg(test)]
pub(crate) mod test_hooks {
    use std::cell::RefCell;
    use std::path::Path;

    type Hook = Box<dyn FnMut(&Path)>;

    thread_local! {
        static BEFORE_BOUNDED_MKDIR: RefCell<Option<Hook>> = RefCell::new(None);
    }

    /// Installs `hook` on this thread until the returned guard drops.
    pub(crate) fn before_bounded_mkdir_scoped(hook: impl FnMut(&Path) + 'static) -> HookGuard {
        BEFORE_BOUNDED_MKDIR.with(|cell| *cell.borrow_mut() = Some(Box::new(hook)));
        HookGuard
    }

    pub(super) fn before_bounded_mkdir(level: &Path) {
        BEFORE_BOUNDED_MKDIR.with(|cell| {
            if let Some(hook) = cell.borrow_mut().as_mut() {
                hook(level);
            }
        });
    }

    pub(crate) struct HookGuard;

    impl Drop for HookGuard {
        fn drop(&mut self) {
            BEFORE_BOUNDED_MKDIR.with(|cell| *cell.borrow_mut() = None);
        }
    }
}

pub(crate) fn remove_path_any(path: &Path) -> Result<()> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err).with_context(|| format!("stat {:?}", path)),
    };
    let ft = meta.file_type();

    // A symlink (even one pointing to a directory) should be removed with remove_file, which deletes the link itself
    if ft.is_symlink() {
        std::fs::remove_file(path).with_context(|| format!("remove symlink {:?}", path))?;
        return Ok(());
    }
    if ft.is_dir() {
        std::fs::remove_dir_all(path).with_context(|| format!("remove dir {:?}", path))?;
        return Ok(());
    }
    std::fs::remove_file(path).with_context(|| format!("remove file {:?}", path))?;
    Ok(())
}

fn is_same_link(link_path: &Path, target: &Path) -> bool {
    if let Ok(existing) = std::fs::read_link(link_path) {
        return existing == target;
    }
    false
}

fn try_link_dir(source: &Path, target: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, target)
            .with_context(|| format!("symlink {:?} -> {:?}", target, source))?;
        Ok(())
    }

    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(source, target)
            .with_context(|| format!("symlink {:?} -> {:?}", target, source))?;
        return Ok(());
    }

    #[cfg(not(any(unix, windows)))]
    anyhow::bail!("symlink not supported on this platform");
}

#[cfg(windows)]
fn try_junction(source: &Path, target: &Path) -> Result<()> {
    junction::create(source, target)
        .with_context(|| format!("junction {:?} -> {:?}", target, source))?;
    Ok(())
}

fn should_skip_copy(entry: &walkdir::DirEntry) -> bool {
    entry.file_name() == ".git"
}

/// Copy skill content without `.git` or internal symlinks. Internal links are
/// neither preserved nor followed: they are not content, matching `hash_dir`.
pub fn copy_dir_recursive(source: &Path, target: &Path) -> Result<()> {
    copy_dir_with_parents(source, target, TargetParents::CreateMissing)
}

/// [`copy_dir_recursive`] creating every directory through `parents`.
fn copy_dir_with_parents(source: &Path, target: &Path, parents: TargetParents<'_>) -> Result<()> {
    let profile = std::env::var("SKILLS_HUB_PROFILE_IO")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let started = std::time::Instant::now();
    let mut copied_files: u64 = 0;
    let mut copied_bytes: u64 = 0;

    for entry in walkdir::WalkDir::new(source)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !should_skip_copy(entry))
    {
        let entry = entry?;
        if should_skip_copy(&entry) {
            continue;
        }
        if entry.file_type().is_symlink() {
            // Internal links are not content; never copy or follow their targets.
            continue;
        }
        let relative = entry.path().strip_prefix(source)?;
        let target_path = target.join(relative);

        if entry.file_type().is_dir() {
            parents.create(&target_path)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target_path.parent() {
                parents.create(parent)?;
            }
            let bytes = std::fs::copy(entry.path(), &target_path)
                .with_context(|| format!("copy file {:?} -> {:?}", entry.path(), target_path))?;
            if profile {
                copied_files += 1;
                copied_bytes = copied_bytes.saturating_add(bytes);
            }
        }
    }
    if profile {
        log::info!(
            "[sync_engine] copy_dir_recursive {} files, {} bytes in {}s (src={:?} dst={:?})",
            copied_files,
            copied_bytes,
            started.elapsed().as_secs_f32(),
            source,
            target
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/sync_engine.rs"]
mod tests;
