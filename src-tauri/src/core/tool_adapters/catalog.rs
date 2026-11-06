//! Presentation-ready Tool catalogs.
//!
//! The command tier maps these entries 1:1 onto `ToolInfoDto`; every rule
//! about which adapters appear, how virtual groups absorb their constituent
//! tools, how shared skills dirs group, and what "installed" means lives here
//! so it is testable against a temp home.

use super::reads::{DuplicateBehaviour, ReadFacts};
use super::{
    catalog_rows, constituents_of, is_installed_in, reader_installed_in, readers_of, HostRoots,
    RegistryPath, ToolAdapter,
};
use crate::core::placement::{own_folder, serving_folder, PlacementSettings, ScopeKind};

/// One row of a tool list, with every backend-owned fact already resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCatalogEntry {
    pub key: &'static str,
    pub label: &'static str,
    pub installed: bool,
    /// The folder key of the Deployment folder serving this entry at the
    /// catalog's scope (`core::placement::serving_folder`): its rows, the
    /// saved selection and the project matrix are keyed by it, and the
    /// frontend matches a harness to its rows through it, never through a path.
    pub folder_key: &'static str,
    /// Keys of every entry in this list sharing this entry's skills dir — the
    /// global dir for the global catalog, the project dir for the project
    /// catalog — in registry order, including the entry itself (len >= 1):
    /// the entries whose own folder (`core::placement::own_folder`) is its.
    ///
    /// The dir itself is not a field of the entry: it is home-relative at
    /// global scope but project-relative at project scope, so one field could
    /// not carry both honestly. The global catalog states it in `placement`
    /// (for display); grouping always reads this list, never the path.
    pub shared_with: Vec<&'static str>,
    /// Display names of the constituent tools absorbed into this entry when
    /// it is a virtual group — its reader-only harnesses included, after its
    /// adapters — at project scope only; empty for real tools and globally.
    pub constituents: Vec<&'static str>,
    /// The reader-only harnesses reading this entry's folder, with their
    /// detected state — a virtual group's entry in the global catalog only
    /// (at project scope they are absorbed into `constituents`).
    pub readers: Vec<ReaderEntry>,
    /// Where this Tool lives on the operator's machine and how Skills Hub
    /// deploys into it — the global catalog only (`None` in the project
    /// catalog, whose dirs are project-relative and whose detection is a
    /// group's union).
    pub placement: Option<GlobalPlacement>,
    /// Set when this harness, deployed to beside one of the `keys`, would see
    /// a skill twice — the cross-read warning's pairs, global catalog only
    /// ([`DuplicateReads`]). `None` when there is no such pair.
    pub duplicate_readers: Option<DuplicateReads>,
}

/// The cross-read warning's pairs for one installed harness R: the other
/// installed entries whose deploy dir R reads **by default** (`Yes`, never a
/// legacy, opt-in, conditional or unknown read — `ReadFacts::reads_by_default`)
/// while R also reads its own deploy dir by default. A shared skills dir
/// sibling (`shared_with`) is not a pair: one folder, deployed to once.
/// Resolved from the registry alone; the frontend only intersects `keys`
/// with the selection, never re-deriving a read from a dir.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateReads {
    /// The other installed entries' keys, in registry order (len >= 1).
    pub keys: Vec<&'static str>,
    /// What R does with the second copy (the warning's consequence).
    pub behaviour: DuplicateBehaviour,
}

/// One reader-only harness listed under the entry whose folder it reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderEntry {
    pub key: &'static str,
    pub label: &'static str,
    pub installed: bool,
    /// Its cross-read pairs ([`DuplicateReads`]), resolved as an adapter's
    /// with the group entry's folder as its own deploy dir: set when, with
    /// that entry deployed to, it would see a skill twice through one of the
    /// `keys` (fx reads `~/.claude/skills` too).
    pub duplicate_readers: Option<DuplicateReads>,
}

/// A global catalog entry's placement facts, straight from the registry,
/// spelled for display (`RegistryPath::display`: `~/.claude/skills` under
/// home, the absolute path under another root).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobalPlacement {
    /// The global skills dir (`~/.claude/skills`).
    pub skills_dir: String,
    /// The dirs any one of whose presence marks the Tool installed, in
    /// registry order.
    pub detect_dirs: Vec<String>,
    /// The registry capability (`ToolAdapter::supports_symlink`): `true`
    /// deploys a link (symlink → junction → copy fallback), `false` a copy.
    pub supports_symlink: bool,
}

/// Global-scope catalog: every registry entry, grouped by shared global
/// skills dir.
///
/// A virtual group is listed here **alongside** its constituents: globally it
/// absorbs nothing (Cursor reads `~/.cursor/skills`, Codex `~/.codex/skills`),
/// it is simply one more directory some tools also read. So it carries no
/// constituent roster — the roster renders exactly when ticking this box
/// covers those tools — its label is the plain `display_name`, and its
/// installedness is its own detect dir like any other entry, or any detected
/// reader-only harness of it (`is_installed_in`).
pub fn global_tool_entries(roots: &HostRoots) -> Vec<ToolCatalogEntry> {
    let installed_rows: Vec<&'static ToolAdapter> = catalog_rows()
        .filter(|adapter| is_installed_in(roots, adapter))
        .collect();
    catalog_rows()
        .map(|adapter| ToolCatalogEntry {
            key: adapter.key(),
            label: adapter.display_name,
            installed: is_installed_in(roots, adapter),
            folder_key: folder_key_of(ScopeKind::Global, adapter),
            shared_with: catalog_rows()
                .filter(|other| {
                    own_folder(ScopeKind::Global, other.key())
                        == own_folder(ScopeKind::Global, adapter.key())
                })
                .map(ToolAdapter::key)
                .collect(),
            constituents: vec![],
            readers: adapter
                .as_virtual_group()
                .map(|group| {
                    readers_of(group)
                        .into_iter()
                        .map(|reader| {
                            let installed = reader_installed_in(roots, reader);
                            ReaderEntry {
                                key: reader.key,
                                label: reader.display_name,
                                installed,
                                duplicate_readers: installed
                                    .then(|| {
                                        pairs_of(
                                            &reader.reads,
                                            &adapter.skills_dir,
                                            &installed_rows,
                                        )
                                    })
                                    .flatten(),
                            }
                        })
                        .collect()
                })
                .unwrap_or_default(),
            placement: Some(GlobalPlacement {
                skills_dir: adapter.skills_dir.display(roots),
                detect_dirs: adapter
                    .detect_dirs
                    .iter()
                    .map(|dir| dir.display(roots))
                    .collect(),
                supports_symlink: adapter.supports_symlink,
            }),
            duplicate_readers: duplicate_reads(adapter, &installed_rows),
        })
        .collect()
}

/// [`DuplicateReads`] for the adapter `reader` among the `installed` entries.
fn duplicate_reads(
    reader: &ToolAdapter,
    installed: &[&'static ToolAdapter],
) -> Option<DuplicateReads> {
    if !installed.iter().any(|a| a.key() == reader.key()) {
        return None;
    }
    pairs_of(&reader.reads, &reader.skills_dir, installed)
}

/// The pairs of an installed harness whose read facts are `facts` and whose
/// deploy dir is `own` (an adapter's own folder, or for a reader-only harness
/// its group entry's): the other `installed` entries whose deploy dir it
/// reads by default, beside `own` — `None` unless it reads `own` by default.
fn pairs_of(
    facts: &ReadFacts,
    own: &RegistryPath,
    installed: &[&'static ToolAdapter],
) -> Option<DuplicateReads> {
    if !facts.reads_by_default(own, own) {
        return None;
    }
    let keys: Vec<&'static str> = installed
        .iter()
        .filter(|other| !other.skills_dir.same_location(own))
        .filter(|other| facts.reads_by_default(&other.skills_dir, own))
        .map(|other| other.key())
        .collect();
    (!keys.is_empty()).then_some(DuplicateReads {
        keys,
        behaviour: facts.duplicates,
    })
}

/// Project-scope catalog: constituent tools are absorbed into their virtual
/// group's entry, which counts as installed when any constituent is. Entries
/// are grouped by shared project skills dir. A tool with no project skills
/// dir is not listed.
pub fn project_tool_entries(roots: &HostRoots) -> Vec<ToolCatalogEntry> {
    let listed: Vec<&'static ToolAdapter> = catalog_rows()
        .filter(|adapter| adapter.group.is_none() && adapter.project_skills_dir.is_some())
        .collect();

    listed
        .iter()
        .map(|adapter| {
            let (installed, constituents) = match adapter.as_virtual_group() {
                Some(group) => {
                    let members: Vec<&'static ToolAdapter> = constituents_of(group).collect();
                    let readers = readers_of(group);
                    (
                        members.iter().any(|a| is_installed_in(roots, a))
                            || readers.iter().any(|r| reader_installed_in(roots, r)),
                        members
                            .iter()
                            .map(|a| a.display_name)
                            .chain(readers.iter().map(|r| r.display_name))
                            .collect(),
                    )
                }
                None => (is_installed_in(roots, adapter), vec![]),
            };
            ToolCatalogEntry {
                key: adapter.key(),
                label: adapter.display_name,
                installed,
                folder_key: folder_key_of(ScopeKind::Project, adapter),
                shared_with: listed
                    .iter()
                    .filter(|a| a.project_skills_dir == adapter.project_skills_dir)
                    .map(|a| a.key())
                    .collect(),
                constituents,
                readers: vec![],
                placement: None,
                duplicate_readers: None,
            }
        })
        .collect()
}

/// The folder serving `adapter` at `scope` (an entry the scope lists always
/// has one; its own key is the fallback the registry tests rule out).
fn folder_key_of(scope: ScopeKind, adapter: &ToolAdapter) -> &'static str {
    serving_folder(scope, adapter.key(), &PlacementSettings::default()).unwrap_or(adapter.key())
}

/// Keys of the installed entries, in catalog order.
pub fn installed_keys(entries: &[ToolCatalogEntry]) -> Vec<String> {
    entries
        .iter()
        .filter(|e| e.installed)
        .map(|e| e.key.to_string())
        .collect()
}

#[cfg(test)]
#[path = "../tests/tool_catalog.rs"]
mod tests;
