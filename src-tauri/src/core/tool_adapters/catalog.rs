//! Presentation-ready Tool catalogs.
//!
//! The command tier maps these entries 1:1 onto `ToolInfoDto`; every rule
//! about which adapters appear, how virtual groups absorb their constituent
//! tools, how shared skills dirs group, and what "installed" means lives here
//! so it is testable against a temp home.

use super::{
    adapters_sharing_skills_dir, catalog_rows, constituents_of, is_installed_in,
    reader_installed_in, readers_of, HostRoots, ToolAdapter,
};

/// One row of a tool list, with every backend-owned fact already resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCatalogEntry {
    pub key: &'static str,
    pub label: &'static str,
    pub installed: bool,
    /// Keys of every entry in this list sharing this entry's skills dir — the
    /// global dir for the global catalog, the project dir for the project
    /// catalog — in registry order, including the entry itself (len >= 1).
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
}

/// One reader-only harness listed under the entry whose folder it reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderEntry {
    pub key: &'static str,
    pub label: &'static str,
    pub installed: bool,
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
    catalog_rows()
        .map(|adapter| ToolCatalogEntry {
            key: adapter.key(),
            label: adapter.display_name,
            installed: is_installed_in(roots, adapter),
            shared_with: adapters_sharing_skills_dir(adapter)
                .into_iter()
                .map(ToolAdapter::key)
                .collect(),
            constituents: vec![],
            readers: adapter
                .as_virtual_group()
                .map(|group| {
                    readers_of(group)
                        .into_iter()
                        .map(|reader| ReaderEntry {
                            key: reader.key,
                            label: reader.display_name,
                            installed: reader_installed_in(roots, reader),
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
        })
        .collect()
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
                shared_with: listed
                    .iter()
                    .filter(|a| a.project_skills_dir == adapter.project_skills_dir)
                    .map(|a| a.key())
                    .collect(),
                constituents,
                readers: vec![],
                placement: None,
            }
        })
        .collect()
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
