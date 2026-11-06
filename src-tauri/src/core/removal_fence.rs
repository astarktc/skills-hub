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
//! A refusal is the typed `SignalError::RemovalRefused`; nothing is touched.
//!
//! **Accepted limit:** admission is a check followed by a removal, not one
//! atomic operation. A concurrent writer that replaces a directory on the
//! way between the two (swapping a real folder for a link) is out of scope:
//! the fence defends against the layout the operator left on disk, not
//! against a process racing the app.
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
/// (`/`, a trailing `..`) is a caller bug, refused with prose.
pub(crate) fn admit(path: &Path, fence: &Fence<'_>) -> Result<Entry> {
    let Some(entry) = Located::at(path)? else {
        return Ok(Entry::Absent);
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
    Ok(entry.into_present())
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
    Ok(entry.into_present())
}

/// Refuse when the existing directory `dir` overlaps `library` — is it, lies
/// inside it or around it — exactly as a removal of `dir` itself would be
/// refused. For a caller about to remove many entries below `dir` (the git
/// cache's age-out) that must not run at all when its root is the library's.
pub(crate) fn ensure_clear_of_library(dir: &Path, library: &ProtectedLibrary<'_>) -> Result<()> {
    admit(dir, &Fence::new(library)).map(|_| ())
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
