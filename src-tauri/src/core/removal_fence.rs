//! Physical admission for every destructive filesystem operation (BACKLOG #107).
//!
//! A path names an entry: its leaf, in the directory its parent resolves to.
//! Resolution follows every alias on the way — a symlinked or junctioned
//! ancestor — so a lexical fence (`starts_with` a Tool's skills dir) says
//! nothing about which bytes a removal reaches: with a Tool's skills dir
//! linked onto the central repo, `<tool dir>/<skill>` *is* the central skill,
//! and `symlink_metadata` on it describes a real directory. Spelling says
//! nothing either: on a case-insensitive volume `<tool dir>/ALPHA` is the
//! central `alpha`. Admission therefore decides on the entry's **physical
//! identity**, as the filesystem reports it:
//!
//! * the parent is resolved through every link or junction;
//! * a leaf that is a link (a junction counts) is never resolved — removing it
//!   removes the link, never its referent — and is located as the resolved
//!   parent plus its name;
//! * any other leaf is resolved in full, which gives its real spelling.
//!
//! Every comparison is then by identity, not by spelling: on Unix the device
//! and inode of each directory on the way (so a case-folding volume or
//! directory cannot alias a protected root), on Windows the final path the OS
//! reports for it (`canonicalize`, real spelling). The admitted entry is
//! removed at that physical location, so the entry checked is the entry
//! removed.
//!
//! Two boundaries, both checked on a present entry:
//!
//! * **The protected library** ([`ProtectedLibrary`]): a removal never
//!   reaches the configured central repo — not the folder, nothing inside it,
//!   nothing around it — nor, for a historical or split library, a recorded
//!   central copy (or anything inside or around it) or the folder holding
//!   one (or anything around it). An overwrite also spares its own source
//!   ([`Fence::sparing`]), so a replacement never removes the bytes it
//!   replaces with.
//! * **The claim** ([`Claim`]): where the caller says the entry lives —
//!   inside one of a set of directories (each resolved through its own
//!   aliases, since an operator may link a Tool's skills dir elsewhere), or
//!   directly inside a directory the app owns ([`OwnedDir`]), reached from a
//!   trusted boundary through real directories only.
//!
//! A directory entry is removed recursively, so admission refuses one that
//! is itself a **mount point** (`MountPoint`) — a filesystem mounted at it
//! would have its contents deleted:
//!
//! * it sits on another device than its (resolved) parent folder (Unix) — a
//!   disk image or share attached at a skill folder reports one device for
//!   the entry and everything inside it, so only this comparison shows it;
//! * on Linux the kernel's mount table lists it (a same-device bind mount).
//!
//! On Windows a mounted folder (a volume mount point, like a junction) is a
//! name-surrogate reparse point, which std reports as a link: the entry is
//! located and removed as a link, never recursed into.
//!
//! Admission also walks what lies **beneath** it — without following any
//! link or junction, exactly as the removal itself
//! (`std::fs::remove_dir_all`) never follows one — and refuses
//! (`ContainsMount`) when the recursion would leave the entry through a
//! mount:
//!
//! * a directory inside it has the identity of a protected directory — the
//!   library's, an overwrite's source, or any directory around them, or the
//!   entry itself or a directory around it. A bind mount shares the identity
//!   of the directory it mounts, so this catches a same-device bind mount of
//!   the library without mount IDs (on Windows, where identity is the final
//!   path, a mounted folder is a reparse point the walk never enters);
//! * a directory inside it sits on another device than the entry (Unix): a
//!   mounted filesystem inside a skill folder is never the skill's;
//! * on Linux, `/proc/self/mountinfo` lists a mount point below it — this
//!   also catches a bind mount of a folder *inside* the library, whose
//!   identity names no protected directory. The table is read as bytes and
//!   compared as paths (a mount point need not be UTF-8), and a table that
//!   cannot be read or parsed fails closed (`MountTableUnreadable`): not
//!   knowing the mounts is never "no mounts".
//!
//! A refusal is the typed `SignalError::RemovalRefused`; nothing is touched.
//!
//! **Accepted limit:** admission is a check followed by a removal, not one
//! atomic operation. A concurrent writer that replaces a directory on the
//! way between the two (swapping a real folder for a link), or mounts a
//! filesystem inside the entry after its walk, is out of scope: the fence
//! defends against the layout the operator left on disk, not against a
//! process racing the app.
//!
//! Callers never use this module directly to delete: the one removal
//! primitive is `sync_engine::remove_path_any`, which takes a [`Fence`].

use std::cell::OnceCell;
use std::fs::FileType;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};

use super::errors::{RemovalRefusal, SignalError};
use super::skill_store::SkillStore;

/// The central library a removal must never reach.
///
/// The library is the **configured central repo** — the folder the command
/// seam resolves (`InstallerPaths::central_dir`,
/// `resolve_central_repo_path_for_app`), protected with its whole subtree:
/// no removal is it, inside it, or around it, whatever else it holds
/// (staging folders, the Explore cache, a copy not yet recorded) — plus what
/// the store records, for a library split across folders or left behind by a
/// move: every Managed skill's central copy (protected with everything
/// inside it) and the folder holding it (never removed itself or with
/// anything around it). It is read once, lazily, at the first admission of a
/// present entry, so building one where nothing ends up removed costs
/// nothing.
pub(crate) struct ProtectedLibrary<'a> {
    store: Option<&'a SkillStore>,
    central_dir: Option<&'a Path>,
    roots: OnceCell<Roots>,
}

/// The library in physical form.
#[derive(Default)]
struct Roots {
    /// The configured central repo: no removal may be it, inside it, or
    /// around it.
    configured: Option<Lineage>,
    /// Every recorded central copy, by skill id: no removal may be one,
    /// inside one, or around one.
    copies: Vec<(String, Lineage)>,
    /// The folders holding them: no removal may be one or around one.
    holders: Vec<Lineage>,
}

impl Roots {
    /// Every lineage the library names — the configured repo, each recorded
    /// copy and each folder holding one.
    fn lineages(&self) -> impl Iterator<Item = &Lineage> {
        self.configured
            .iter()
            .chain(self.copies.iter().map(|(_, copy)| copy))
            .chain(&self.holders)
    }
}

impl<'a> ProtectedLibrary<'a> {
    /// The configured central repo `central_dir` and the central library as
    /// `store` records it.
    pub(crate) fn new(store: &'a SkillStore, central_dir: &'a Path) -> Self {
        Self {
            store: Some(store),
            central_dir: Some(central_dir),
            roots: OnceCell::new(),
        }
    }

    /// No library at all — for an engine test with an unmanaged source. A
    /// [`Fence`] may still spare a source. Production always protects one.
    #[cfg(test)]
    pub(crate) fn unrecorded() -> Self {
        Self {
            store: None,
            central_dir: None,
            roots: OnceCell::new(),
        }
    }

    /// The library in physical form. A store or resolution failure fails the
    /// admission: refusing is recoverable, a blind removal is not.
    fn roots(&self) -> Result<&Roots> {
        if let Some(roots) = self.roots.get() {
            return Ok(roots);
        }
        let mut roots = Roots::default();
        if let Some(central_dir) = self.central_dir {
            if !central_dir.is_absolute() {
                anyhow::bail!(
                    "refusing to remove anything: the central repo {:?} is not an absolute path",
                    central_dir
                );
            }
            roots.configured = Some(Lineage::of(central_dir)?);
        }
        if let Some(store) = self.store {
            for skill in store
                .list_skills()
                .context("read the central library before a removal")?
            {
                let central = Path::new(&skill.central_path);
                // A relative or empty record locates nothing, and a
                // filesystem root as a copy would protect everything.
                let Some(holder) = central.parent().filter(|_| central.is_absolute()) else {
                    continue;
                };
                roots.copies.push((skill.id, Lineage::of(central)?));
                let holder = Lineage::of(holder)?;
                if !roots.holders.contains(&holder) {
                    roots.holders.push(holder);
                }
            }
        }
        Ok(self.roots.get_or_init(|| roots))
    }
}

/// A directory the app creates and owns, reached from a trusted boundary
/// through real directories only.
///
/// The boundary is a root the command seam resolves (the app cache dir): its
/// own ancestors resolve however the operator's system lays them out, and are
/// trusted. The boundary itself and every component below it down to this
/// directory are the app's — each must be a real directory, never a link or
/// junction, or an alias anywhere on the app's own path (`cache -> outside`,
/// `skills-hub-git-cache -> outside`) would hand the app a directory it does
/// not own. The resolved form is what every later removal compares against
/// ([`Claim::ChildOfOwned`]), and admission re-checks the chain.
#[derive(Debug)]
pub(crate) struct OwnedDir<'a> {
    boundary: &'a Path,
    relative: &'a Path,
    physical: PathBuf,
}

impl<'a> OwnedDir<'a> {
    /// Resolve `boundary/relative` as an owned directory: `None` when any
    /// level is absent (nothing in it to remove); refused as
    /// `ResolvesElsewhere` when a level is a link or junction.
    pub(crate) fn resolve(boundary: &'a Path, relative: &'a Path) -> Result<Option<Self>> {
        if !boundary.is_absolute() {
            anyhow::bail!(
                "refusing an owned directory under a relative root: {:?}",
                boundary
            );
        }
        if relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        {
            anyhow::bail!(
                "refusing an owned directory {:?}: it escapes {:?}",
                relative,
                boundary
            );
        }
        let as_written = boundary.join(relative);
        let refuse = |level: &Path| {
            let resolved = as_written
                .canonicalize()
                .unwrap_or_else(|_| level.to_path_buf());
            anyhow::anyhow!(SignalError::RemovalRefused {
                path: as_written.to_string_lossy().into_owned(),
                resolved: resolved.to_string_lossy().into_owned(),
                reason: RemovalRefusal::ResolvesElsewhere,
            })
        };
        let real_dir = |level: &Path| -> Result<Option<()>> {
            match std::fs::symlink_metadata(level) {
                Ok(meta) if meta.file_type().is_symlink() => Err(refuse(level)),
                Ok(meta) if meta.is_dir() => Ok(Some(())),
                Ok(_) => anyhow::bail!("{:?} is not a directory", level),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(err) => Err(err).with_context(|| format!("stat {:?}", level)),
            }
        };
        if real_dir(boundary)?.is_none() {
            return Ok(None);
        }
        let mut physical = boundary
            .canonicalize()
            .with_context(|| format!("resolve {:?}", boundary))?;
        for component in relative.components() {
            physical.push(component);
            if real_dir(&physical)?.is_none() {
                return Ok(None);
            }
        }
        // No level below the boundary is a link, so this only gives the real
        // spelling — the form an admitted entry's resolved parent takes.
        let physical = physical
            .canonicalize()
            .with_context(|| format!("resolve {:?}", physical))?;
        Ok(Some(Self {
            boundary,
            relative,
            physical,
        }))
    }

    /// The directory where it physically is.
    pub(crate) fn path(&self) -> &Path {
        &self.physical
    }
}

/// Where a caller says the entry it removes lives.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Claim<'a> {
    /// No claim beyond the protected library: the path is one the app
    /// recorded (a target row) or derived from the registry.
    Unclaimed,
    /// Physically inside one of these directories, each resolved through its
    /// own aliases (an operator may link a Tool's skills dir elsewhere).
    Within(&'a [PathBuf]),
    /// Physically a direct child of this owned directory (the git cache),
    /// whose chain from its boundary is re-checked at admission.
    ChildOfOwned(&'a OwnedDir<'a>),
}

/// Everything admission checks a removal against.
pub(crate) struct Fence<'a> {
    library: &'a ProtectedLibrary<'a>,
    source: Option<&'a Path>,
    claim: Claim<'a>,
}

impl<'a> Fence<'a> {
    /// Protect `library`, with no claim.
    pub(crate) fn new(library: &'a ProtectedLibrary<'a>) -> Self {
        Self {
            library,
            source: None,
            claim: Claim::Unclaimed,
        }
    }

    /// Also protect `source` — an overwrite's own bytes.
    pub(crate) fn sparing(self, source: &'a Path) -> Self {
        Self {
            source: Some(source),
            ..self
        }
    }

    /// Require the entry to live where `claim` says.
    pub(crate) fn claiming(self, claim: Claim<'a>) -> Self {
        Self { claim, ..self }
    }
}

/// What admission found at a path.
#[derive(Debug)]
pub(crate) enum Entry {
    /// Nothing there (its parent or its leaf is `NotFound`): nothing to remove.
    Absent,
    /// An admitted entry at its physical location, with its own (never
    /// followed) file type.
    Present {
        physical: PathBuf,
        file_type: FileType,
    },
}

/// Admit the removal of the entry `path` names against `fence`.
///
/// Absence is only `NotFound` (of the parent or of the leaf); any other
/// resolution or stat error is returned, never read as absence. A refusal is
/// `SignalError::RemovalRefused`; a path that is relative or names no entry
/// (`/`, a trailing `..`) is a caller bug, refused with prose. A directory's
/// subtree is walked before it is admitted: a mount inside it is refused.
pub(crate) fn admit(path: &Path, fence: &Fence<'_>) -> Result<Entry> {
    let Some((entry, source)) = admit_location(path, fence)? else {
        return Ok(Entry::Absent);
    };
    let library = fence.library.roots()?;
    entry.ensure_no_mount_inside(path, source.iter().chain(library.lineages()))?;
    Ok(entry.into_present())
}

/// Where the entry `path` names is, admitted against the library, the
/// fence's source and its claim — not yet what lies beneath it. With the
/// source's lineage, for the walk.
fn admit_location(path: &Path, fence: &Fence<'_>) -> Result<Option<(Located, Option<Lineage>)>> {
    let Some(entry) = Located::at(path)? else {
        return Ok(None);
    };
    let library = fence.library.roots()?;
    let source = fence.source.map(Lineage::of).transpose()?;
    let reaches_library = source
        .iter()
        .chain(&library.configured)
        .chain(library.copies.iter().map(|(_, copy)| copy))
        .any(|root| entry.inside(root) || entry.around(root))
        || library.holders.iter().any(|holder| entry.around(holder));
    if reaches_library {
        return Err(entry.refuse(path, RemovalRefusal::CentralLibrary));
    }
    let claimed = match fence.claim {
        Claim::Unclaimed => true,
        Claim::Within(dirs) => {
            let mut claimed = false;
            for dir in dirs {
                let dir = Lineage::of(dir)?;
                if dir.head().is_some_and(|dir| entry.parent.ids.contains(dir)) {
                    claimed = true;
                    break;
                }
            }
            claimed
        }
        Claim::ChildOfOwned(owned) => {
            // The chain is re-checked now: an alias swapped in since the
            // directory was resolved is refused, not followed.
            match OwnedDir::resolve(owned.boundary, owned.relative)? {
                Some(now) if now.physical == owned.physical => {
                    entry.parent.ids.first() == Some(&identity(&now.physical)?)
                }
                _ => false,
            }
        }
    };
    if !claimed {
        return Err(entry.refuse(path, RemovalRefusal::ResolvesElsewhere));
    }
    Ok(Some((entry, source)))
}

/// Admit the removal of skill `skill_id`'s own central copy at `path` — the
/// Delete settlement's last step, and the one removal that reaches central
/// bytes. Its authorization is the record, not a [`Fence`] (which forbids
/// every other removal exactly these bytes): `path` is the skill's recorded
/// `central_path`. It is still located physically, and refused
/// (`CentralLibrary`) when it reaches the rest of the library — it is or
/// contains the configured central repo or a folder holding copies, or it
/// overlaps another skill's copy — so a malformed record never takes the
/// library with it. A link at the recorded path is removed as itself.
pub(crate) fn admit_central_copy(
    path: &Path,
    skill_id: &str,
    library: &ProtectedLibrary<'_>,
) -> Result<Entry> {
    let Some(entry) = Located::at(path)? else {
        return Ok(Entry::Absent);
    };
    let library = library.roots()?;
    let reaches_rest = library.configured.iter().any(|root| entry.around(root))
        || library.holders.iter().any(|holder| entry.around(holder))
        || library
            .copies
            .iter()
            .filter(|(id, _)| id != skill_id)
            .any(|(_, copy)| entry.inside(copy) || entry.around(copy));
    if reaches_rest {
        return Err(entry.refuse(path, RemovalRefusal::CentralLibrary));
    }
    entry.ensure_no_mount_inside(path, library.lineages())?;
    Ok(entry.into_present())
}

/// Refuse when the existing directory `dir` overlaps `library` — is it, lies
/// inside it or around it — exactly as a removal of `dir` itself would be
/// refused. For a caller about to remove many entries below `dir` (the git
/// cache's age-out) that must not run at all when its root is the library's.
/// Its subtree is not walked here: each entry removed below it is, when it
/// is admitted.
pub(crate) fn ensure_clear_of_library(dir: &Path, library: &ProtectedLibrary<'_>) -> Result<()> {
    admit_location(dir, &Fence::new(library)).map(|_| ())
}

/// A present entry, located physically.
struct Located {
    /// Where it is: the resolved parent plus the name for a link, the entry
    /// resolved in full (its real spelling) for anything else.
    physical: PathBuf,
    /// Its own, never followed, file type.
    file_type: FileType,
    /// The resolved parent's lineage.
    parent: Lineage,
    /// Its own identity — `None` for a link, which is never a library
    /// directory: it can lie inside the library but never be or contain a
    /// part of it.
    id: Option<Id>,
}

impl Located {
    /// Locate the entry `path` names; `None` when its parent or leaf is
    /// `NotFound`.
    fn at(path: &Path) -> Result<Option<Self>> {
        if !path.is_absolute() {
            anyhow::bail!("refusing to remove a relative path: {:?}", path);
        }
        let (Some(parent), Some(leaf)) = (path.parent(), path.file_name()) else {
            anyhow::bail!("refusing to remove {:?}: it names no entry", path);
        };
        let not_found = |err: &std::io::Error| err.kind() == std::io::ErrorKind::NotFound;
        let parent = match parent.canonicalize() {
            Ok(parent) => parent,
            Err(err) if not_found(&err) => return Ok(None),
            Err(err) => return Err(err).with_context(|| format!("resolve {:?}", parent)),
        };
        let spelled = parent.join(leaf);
        let file_type = match std::fs::symlink_metadata(&spelled) {
            Ok(meta) => meta.file_type(),
            Err(err) if not_found(&err) => return Ok(None),
            Err(err) => return Err(err).with_context(|| format!("stat {:?}", spelled)),
        };
        // A link stays a link: located by its parent, never resolved.
        // Anything else is resolved in full — it crosses no link, so this
        // only gives its real spelling (`<tool dir>/ALPHA` is
        // `central/alpha` on a case-insensitive volume).
        let (physical, id) = if file_type.is_symlink() {
            (spelled, None)
        } else {
            let physical = match spelled.canonicalize() {
                Ok(physical) => physical,
                Err(err) if not_found(&err) => return Ok(None),
                Err(err) => return Err(err).with_context(|| format!("resolve {:?}", spelled)),
            };
            let id = identity(&physical)?;
            (physical, Some(id))
        };
        Ok(Some(Self {
            physical,
            file_type,
            parent: Lineage::of_existing(&parent)?,
            id,
        }))
    }

    /// The entry is `root` or lies inside it.
    fn inside(&self, root: &Lineage) -> bool {
        root.head()
            .is_some_and(|root| self.id.as_ref() == Some(root) || self.parent.ids.contains(root))
    }

    /// `root` (or where it would be) is the entry or lies inside it.
    fn around(&self, root: &Lineage) -> bool {
        self.id.as_ref().is_some_and(|id| root.ids.contains(id))
    }

    /// Refuse a directory entry that is a mount point (`MountPoint`: on
    /// another device than its parent, or — Linux — listed in the mount
    /// table), or whose mount table cannot be read (`MountTableUnreadable`).
    /// Then walk its subtree as its recursive removal would — never through a
    /// link or junction — and refuse (`ContainsMount`, with the offending
    /// directory as `resolved`) when a directory in it is a mount: it has the
    /// identity of a directory in one of the `protected` lineages, of the
    /// entry, or of a directory around the entry; it sits on another device;
    /// or (Linux) the mount table lists a mount point below the entry. A link
    /// or a file entry has no subtree to walk.
    fn ensure_no_mount_inside<'r>(
        &self,
        path: &Path,
        protected: impl Iterator<Item = &'r Lineage>,
    ) -> Result<()> {
        let Some(own) = self.id.as_ref().filter(|_| self.file_type.is_dir()) else {
            return Ok(());
        };
        let refuse_at = |at: &Path, reason| {
            anyhow::anyhow!(SignalError::RemovalRefused {
                path: path.to_string_lossy().into_owned(),
                resolved: at.to_string_lossy().into_owned(),
                reason,
            })
        };
        let refuse = |at: &Path| refuse_at(at, RemovalRefusal::ContainsMount);
        // A filesystem mounted at the entry itself shares one device with
        // everything inside it, so only the entry against its folder shows it.
        if self
            .parent
            .ids
            .first()
            .is_some_and(|parent| !own.same_device(parent))
        {
            return Err(refuse_at(&self.physical, RemovalRefusal::MountPoint));
        }
        // The kernel's mount table (Linux) also lists a same-device bind mount
        // of a folder inside the library. Not knowing is never "none".
        #[cfg(unix)]
        match mounts::table() {
            Ok(None) => {}
            Ok(Some(table)) => {
                let Some(points) = mounts::mount_points(&table) else {
                    return Err(refuse_at(
                        &self.physical,
                        RemovalRefusal::MountTableUnreadable,
                    ));
                };
                if points.iter().any(|point| point == &self.physical) {
                    return Err(refuse_at(&self.physical, RemovalRefusal::MountPoint));
                }
                if let Some(below) = points
                    .iter()
                    .find(|point| point.starts_with(&self.physical))
                {
                    return Err(refuse(below));
                }
            }
            // The I/O error is diagnostic text, never a cause in the chain:
            // the removal primitive would read an `EACCES` there as the
            // entry's own permission failure.
            Err(err) => {
                return Err(
                    refuse_at(&self.physical, RemovalRefusal::MountTableUnreadable)
                        .context(format!("could not read the mount table: {err}")),
                );
            }
        }
        let forbidden: Vec<&Id> = protected
            .flat_map(|lineage| lineage.ids.iter())
            .chain(&self.parent.ids)
            .chain(std::iter::once(own))
            .collect();
        let not_found = |err: &std::io::Error| err.kind() == std::io::ErrorKind::NotFound;
        let mut pending = vec![self.physical.clone()];
        while let Some(dir) = pending.pop() {
            let children = match std::fs::read_dir(&dir) {
                Ok(children) => children,
                Err(err) if not_found(&err) => continue,
                Err(err) => return Err(err).with_context(|| format!("read {:?}", dir)),
            };
            for child in children {
                let child = child.with_context(|| format!("read {:?}", dir))?;
                // Never followed: a link (a junction counts) is removed as
                // itself, and is neither a directory nor walked.
                let file_type = child
                    .file_type()
                    .with_context(|| format!("stat {:?}", child.path()))?;
                if !file_type.is_dir() {
                    continue;
                }
                let child = child.path();
                let id = match subtree_identity(&child) {
                    Ok(id) => id,
                    Err(err) if not_found(&err) => continue,
                    Err(err) => return Err(err).with_context(|| format!("stat {:?}", child)),
                };
                if forbidden.contains(&&id) || !id.same_device(own) {
                    return Err(refuse(&child));
                }
                pending.push(child);
            }
        }
        Ok(())
    }

    fn refuse(&self, path: &Path, reason: RemovalRefusal) -> anyhow::Error {
        anyhow::anyhow!(SignalError::RemovalRefused {
            path: path.to_string_lossy().into_owned(),
            resolved: self.physical.to_string_lossy().into_owned(),
            reason,
        })
    }

    fn into_present(self) -> Entry {
        Entry::Present {
            physical: self.physical,
            file_type: self.file_type,
        }
    }
}

/// A directory's physical identity, as the filesystem reports it.
#[cfg(unix)]
#[derive(Clone, Debug, PartialEq, Eq)]
struct Id {
    dev: u64,
    ino: u64,
}

/// A directory's physical identity: on Windows the final path the OS
/// reports for an already-resolved path (its real spelling).
#[cfg(not(unix))]
#[derive(Clone, Debug, PartialEq, Eq)]
struct Id(PathBuf);

/// The identity of the entry at the resolved path `path` (no link on the way,
/// so following and not following agree).
fn identity(path: &Path) -> Result<Id> {
    #[cfg(test)]
    if let Some(id) = test_seams::pretended(path) {
        return Ok(id);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let meta = std::fs::metadata(path).with_context(|| format!("stat {:?}", path))?;
        Ok(Id {
            dev: meta.dev(),
            ino: meta.ino(),
        })
    }
    #[cfg(not(unix))]
    {
        Ok(Id(path.to_path_buf()))
    }
}

impl Id {
    /// Whether both lie on one device (Unix `st_dev`). Windows has no stable
    /// device number in std; a mounted folder there is a reparse point that
    /// std reports as a link — removed as itself at the entry, never entered
    /// by the walk.
    fn same_device(&self, other: &Id) -> bool {
        #[cfg(unix)]
        {
            self.dev == other.dev
        }
        #[cfg(not(unix))]
        {
            let _ = other;
            true
        }
    }
}

/// The identity of a directory found walking an admitted entry's subtree,
/// read without following it (it is a real directory, never a link).
fn subtree_identity(path: &Path) -> std::io::Result<Id> {
    #[cfg(test)]
    if let Some(id) = test_seams::pretended(path) {
        return Ok(id);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let meta = std::fs::symlink_metadata(path)?;
        Ok(Id {
            dev: meta.dev(),
            ino: meta.ino(),
        })
    }
    #[cfg(not(unix))]
    {
        Ok(Id(path.to_path_buf()))
    }
}

/// The mount points the kernel lists for this process (Linux), as raw bytes:
/// a mount point is a Unix path, any bytes but NUL, never necessarily UTF-8.
#[cfg(unix)]
mod mounts {
    use std::path::PathBuf;

    /// This process's mount table: on Linux `/proc/self/mountinfo`, read as
    /// bytes; elsewhere there is none (`Ok(None)` — the portable identity and
    /// device rules carry admission). A table that cannot be read is an
    /// error, never an empty table.
    pub(super) fn table() -> std::io::Result<Option<Vec<u8>>> {
        #[cfg(test)]
        if let Some(pretended) = super::test_seams::pretended_mount_table() {
            return pretended.map(Some);
        }
        #[cfg(target_os = "linux")]
        {
            std::fs::read("/proc/self/mountinfo").map(Some)
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(None)
        }
    }

    /// Every mount point in `mountinfo` (proc(5) format: the fifth
    /// space-separated field of each line, octal-escaped), in table order;
    /// `None` when it is not a mount table — empty (a real one always lists
    /// `/`), or a line without a fifth field.
    pub(super) fn mount_points(mountinfo: &[u8]) -> Option<Vec<PathBuf>> {
        let points = mountinfo
            .split(|&byte| byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| line.split(|&byte| byte == b' ').nth(4).map(unescape))
            .collect::<Option<Vec<_>>>()?;
        (!points.is_empty()).then_some(points)
    }

    /// Undo the kernel's `\ooo` escaping of space, tab, newline and `\`.
    fn unescape(bytes: &[u8]) -> PathBuf {
        use std::os::unix::ffi::OsStringExt;
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            let octal = bytes.get(i + 1..i + 4).filter(|digits| {
                bytes[i] == b'\\' && digits.iter().all(|d| (b'0'..=b'7').contains(d))
            });
            match octal {
                Some(digits) => {
                    out.push(digits.iter().fold(0u8, |n, d| (n << 3) | (d - b'0')));
                    i += 4;
                }
                None => {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
        }
        PathBuf::from(std::ffi::OsString::from_vec(out))
    }
}

/// Where a directory physically is: the identity of its deepest existing
/// level (itself, when it exists) and of every directory above it, nearest
/// first. A root that does not exist yet still has a lineage — its existing
/// ancestors — so nothing around where it would be is removable either.
#[derive(Debug, PartialEq, Eq)]
struct Lineage {
    /// Whether the whole path exists (`ids[0]` is then the path itself).
    complete: bool,
    ids: Vec<Id>,
}

impl Lineage {
    /// `path` resolved through every alias, as far as it resolves. A level
    /// that cannot be resolved for any reason (absent, or unreadable) is
    /// treated as not there: nothing inside a folder the app cannot reach can
    /// be admitted anyway, and everything around it stays protected.
    fn of(path: &Path) -> Result<Self> {
        let mut existing = path;
        let mut complete = true;
        loop {
            if let Ok(resolved) = existing.canonicalize() {
                let mut lineage = Self::of_existing(&resolved)?;
                lineage.complete = complete;
                return Ok(lineage);
            }
            complete = false;
            match existing.parent() {
                Some(parent) => existing = parent,
                None => {
                    return Ok(Self {
                        complete: false,
                        ids: Vec::new(),
                    })
                }
            }
        }
    }

    /// The lineage of an already-resolved, existing path.
    fn of_existing(resolved: &Path) -> Result<Self> {
        Ok(Self {
            complete: true,
            ids: resolved.ancestors().map(identity).collect::<Result<_>>()?,
        })
    }

    /// The path itself, when it exists.
    fn head(&self) -> Option<&Id> {
        self.ids.first().filter(|_| self.complete)
    }
}

/// Where a path physically is, for a boundary decision that removes nothing
/// itself (Move Library folder's admission, aliasing and retention checks):
/// the identity of the path resolved through every alias (when it exists)
/// and of every directory above it — the same [`Lineage`] removal admission
/// compares. A path that does not exist is placed by its deepest existing
/// ancestor. Comparisons are by identity (device + inode on Unix, the final
/// path on Windows), never by spelling.
#[derive(Debug)]
pub(crate) struct Place(Lineage);

impl Place {
    /// Place `path`, following every link on the way and at its leaf.
    pub(crate) fn of(path: &Path) -> Result<Self> {
        Lineage::of(path).map(Self)
    }

    /// Whether the path itself exists.
    pub(crate) fn exists(&self) -> bool {
        self.0.head().is_some()
    }

    /// Whether this is `other` or lies inside it (for a path that does not
    /// exist: would lie inside it). `other` must exist.
    pub(crate) fn within(&self, other: &Place) -> bool {
        other.0.head().is_some_and(|head| self.0.ids.contains(head))
    }

    /// Whether both exist and are one directory.
    pub(crate) fn same(&self, other: &Place) -> bool {
        matches!((self.0.head(), other.0.head()), (Some(a), Some(b)) if a == b)
    }

    /// Whether either is, holds or lies inside the other.
    pub(crate) fn overlaps(&self, other: &Place) -> bool {
        self.within(other) || other.within(self)
    }
}

#[cfg(test)]
#[path = "tests/removal_fence.rs"]
mod tests;

/// Test support: the configured central repo of a test whose store lives in
/// its own temp dir — `central` beside the database, where no fixture keeps a
/// Tool or Project dir (a Tool dir inside the central repo is not a real
/// layout, and the library's whole subtree is protected).
#[cfg(test)]
pub(crate) fn central_repo_of(store: &SkillStore) -> PathBuf {
    store.db_path().with_file_name("central")
}

/// Test seams over what the mount checks read: a test declares a directory —
/// the entry or one inside it — to be a mount, as the filesystem would report
/// one, or sets the mount table (or makes it unreadable), without the
/// privileges a real mount needs.
#[cfg(test)]
pub(crate) mod test_seams {
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    use super::Id;

    thread_local! {
        static PRETENDED: RefCell<Vec<(PathBuf, Id)>> = const { RefCell::new(Vec::new()) };
    }

    /// Undoes one pretended mount when dropped.
    pub(crate) struct MountGuard(PathBuf);

    impl Drop for MountGuard {
        fn drop(&mut self) {
            PRETENDED.with(|cell| cell.borrow_mut().retain(|(dir, _)| dir != &self.0));
        }
    }

    fn pretend(dir: &Path, id: Id) -> MountGuard {
        let dir = dir.canonicalize().expect("a pretended mount point exists");
        PRETENDED.with(|cell| cell.borrow_mut().push((dir.clone(), id)));
        MountGuard(dir)
    }

    /// `dir` reports the identity of `mounted`, as a bind mount of `mounted`
    /// at `dir` does.
    pub(crate) fn mount_identity_of(dir: &Path, mounted: &Path) -> MountGuard {
        let mounted = mounted.canonicalize().expect("the mounted dir exists");
        pretend(dir, super::identity(&mounted).expect("identity"))
    }

    /// `dir` reports another device, as a mounted filesystem does.
    #[cfg(unix)]
    pub(crate) fn mount_other_device(dir: &Path) -> MountGuard {
        let real = super::identity(&dir.canonicalize().expect("exists")).expect("identity");
        pretend(
            dir,
            Id {
                dev: real.dev.wrapping_add(1),
                ino: real.ino,
            },
        )
    }

    #[cfg(unix)]
    thread_local! {
        /// `Some(None)`: the table cannot be read; `Some(Some(bytes))`: it
        /// reads `bytes`.
        static MOUNT_TABLE: RefCell<Option<Option<Vec<u8>>>> = const { RefCell::new(None) };
    }

    /// Undoes a pretended mount table when dropped.
    #[cfg(unix)]
    pub(crate) struct TableGuard(());

    #[cfg(unix)]
    impl Drop for TableGuard {
        fn drop(&mut self) {
            MOUNT_TABLE.with(|cell| *cell.borrow_mut() = None);
        }
    }

    /// The mount table reads `bytes` (mountinfo format), on every Unix.
    #[cfg(unix)]
    pub(crate) fn mount_table(bytes: &[u8]) -> TableGuard {
        MOUNT_TABLE.with(|cell| *cell.borrow_mut() = Some(Some(bytes.to_vec())));
        TableGuard(())
    }

    /// The mount table cannot be read, as with procfs masked or unmounted.
    #[cfg(unix)]
    pub(crate) fn unreadable_mount_table() -> TableGuard {
        MOUNT_TABLE.with(|cell| *cell.borrow_mut() = Some(None));
        TableGuard(())
    }

    #[cfg(unix)]
    pub(super) fn pretended_mount_table() -> Option<std::io::Result<Vec<u8>>> {
        MOUNT_TABLE.with(|cell| {
            cell.borrow().as_ref().map(|table| {
                table.clone().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "pretended: the mount table cannot be read",
                    )
                })
            })
        })
    }

    pub(super) fn pretended(path: &Path) -> Option<Id> {
        PRETENDED.with(|cell| {
            cell.borrow()
                .iter()
                .find(|(dir, _)| dir == path)
                .map(|(_, id)| id.clone())
        })
    }
}
