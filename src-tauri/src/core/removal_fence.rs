//! Physical admission for every destructive filesystem operation (BACKLOG #107).
//!
//! A path names an entry: its leaf, in the directory its parent resolves to.
//! Resolution follows every alias on the way — a symlinked or junctioned
//! ancestor — so a lexical fence (`starts_with` a Tool's skills dir) says
//! nothing about which bytes a removal reaches: with a Tool's skills dir
//! linked onto the central repo, `<tool dir>/<skill>` *is* the central skill,
//! and `symlink_metadata` on it describes a real directory. Admission
//! therefore decides on the entry's **physical identity**: the canonical
//! parent plus the leaf name, the leaf never resolved (removing a leaf link
//! removes the link, never its referent). The admitted entry is removed at
//! that physical location, so the entry checked is the entry removed.
//!
//! Two boundaries, both checked on a present entry:
//!
//! * **The protected library** ([`ProtectedLibrary`]): a removal never
//!   reaches a Managed skill's central copy — not the copy itself, nothing
//!   inside it, nothing around it — nor the central repo folder holding the
//!   copies, nor anything around that. An overwrite also spares its own
//!   source ([`Fence::sparing`]), so a replacement never removes the bytes it
//!   replaces with.
//! * **The claim** ([`Claim`]): where the caller says the entry lives —
//!   inside one of a set of directories (each resolved through its own
//!   aliases, since an operator may link a Tool's skills dir elsewhere), or
//!   directly inside a directory the app owns, which must be a real
//!   directory itself.
//!
//! A refusal is the typed `SignalError::RemovalRefused`; nothing is touched.
//! Admission is not atomic with the removal: a concurrent writer replacing a
//! directory in between is out of scope.
//!
//! Callers never use this module directly to delete: the one removal
//! primitive is `sync_engine::remove_path_any`, which takes a [`Fence`].

use std::cell::OnceCell;
use std::fs::FileType;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::errors::{RemovalRefusal, SignalError};
use super::skill_store::SkillStore;

/// The central library a removal must never reach.
///
/// The library is what the store records: every Managed skill's central copy
/// (protected with everything inside it) and the central repo folder holding
/// it — a central copy is always `<central repo>/<name>` (`install_finalize`,
/// `central_repo::move_central_repo`) — protected as a folder: never removed
/// itself or with anything around it. What else that folder holds is not the
/// library, and the folder is only ever *derived* from the copies, so its
/// other contents are not fenced (a Tool dir may legitimately sit beside a
/// copy). It is read once, lazily, at the first admission of a present entry,
/// so building one where nothing ends up removed costs nothing.
pub(crate) struct ProtectedLibrary<'a> {
    store: Option<&'a SkillStore>,
    roots: OnceCell<Roots>,
}

/// The library in physical form.
#[derive(Default)]
struct Roots {
    /// Central copies: no removal may be one, inside one, or around one.
    copies: Vec<PathBuf>,
    /// Folders holding them: no removal may be one or around one.
    holders: Vec<PathBuf>,
}

impl<'a> ProtectedLibrary<'a> {
    /// The central library as `store` records it.
    pub(crate) fn recorded(store: &'a SkillStore) -> Self {
        Self {
            store: Some(store),
            roots: OnceCell::new(),
        }
    }

    /// No recorded library — for a caller with no store (the git cache, an
    /// unmanaged test source). A [`Fence`] may still spare a source.
    pub(crate) fn unrecorded() -> Self {
        Self {
            store: None,
            roots: OnceCell::new(),
        }
    }

    /// The library in physical form. A store failure fails the admission:
    /// refusing is recoverable, a blind removal is not.
    fn roots(&self) -> Result<&Roots> {
        if let Some(roots) = self.roots.get() {
            return Ok(roots);
        }
        let mut roots = Roots::default();
        if let Some(store) = self.store {
            for skill in store
                .list_skills()
                .context("read the central library before a removal")?
            {
                let central = Path::new(&skill.central_path);
                // A relative or empty record locates nothing, and a
                // filesystem root as a copy would protect everything.
                if !central.is_absolute() || central.parent().is_none() {
                    continue;
                }
                push_unique(&mut roots.copies, physical_form(central));
                if let Some(holder) = central.parent() {
                    push_unique(&mut roots.holders, physical_form(holder));
                }
            }
        }
        Ok(self.roots.get_or_init(|| roots))
    }
}

fn push_unique(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.contains(&path) {
        paths.push(path);
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
    /// Physically a direct child of this directory, which must be a real
    /// directory — not a link or junction: a root the app creates and owns
    /// (the git cache).
    ChildOfOwned(&'a Path),
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
    if !path.is_absolute() {
        anyhow::bail!("refusing to remove a relative path: {:?}", path);
    }
    let (Some(parent), Some(leaf)) = (path.parent(), path.file_name()) else {
        anyhow::bail!("refusing to remove {:?}: it names no entry", path);
    };
    let parent = match parent.canonicalize() {
        Ok(parent) => parent,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Entry::Absent),
        Err(err) => return Err(err).with_context(|| format!("resolve {:?}", parent)),
    };
    let physical = parent.join(leaf);
    let file_type = match std::fs::symlink_metadata(&physical) {
        Ok(meta) => meta.file_type(),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Entry::Absent),
        Err(err) => return Err(err).with_context(|| format!("stat {:?}", physical)),
    };

    let refuse = |reason: RemovalRefusal| {
        anyhow::anyhow!(SignalError::RemovalRefused {
            path: path.to_string_lossy().into_owned(),
            resolved: physical.to_string_lossy().into_owned(),
            reason,
        })
    };
    // Inside `root` (or `root` itself), and around it.
    let inside = |root: &Path| physical.starts_with(root);
    let around = |root: &Path| root.starts_with(&physical);
    let library = fence.library.roots()?;
    let reaches_library = fence
        .source
        .map(physical_form)
        .iter()
        .chain(&library.copies)
        .any(|copy| inside(copy) || around(copy))
        || library.holders.iter().any(|holder| around(holder));
    if reaches_library {
        return Err(refuse(RemovalRefusal::CentralLibrary));
    }
    let claimed = match fence.claim {
        Claim::Unclaimed => true,
        Claim::Within(dirs) => dirs
            .iter()
            .any(|dir| parent.starts_with(physical_form(dir))),
        Claim::ChildOfOwned(dir) => {
            let real_dir = std::fs::symlink_metadata(dir)
                .is_ok_and(|meta| meta.is_dir() && !meta.file_type().is_symlink());
            real_dir && dir.canonicalize().is_ok_and(|dir| dir == parent)
        }
    };
    if !claimed {
        return Err(refuse(RemovalRefusal::ResolvesElsewhere));
    }
    Ok(Entry::Present {
        physical,
        file_type,
    })
}

/// `path` as the filesystem resolves it: its longest existing prefix
/// canonicalised, the rest appended as spelled — so a protected root or a
/// claimed directory that does not exist yet still compares in the same form
/// as an admitted entry (on Windows, the verbatim `\\?\` form).
fn physical_form(path: &Path) -> PathBuf {
    let mut existing = path;
    let mut rest = Vec::new();
    loop {
        if let Ok(resolved) = existing.canonicalize() {
            return rest.iter().rev().fold(resolved, |out, name| out.join(name));
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_os_string());
                existing = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

#[cfg(test)]
#[path = "tests/removal_fence.rs"]
mod tests;
