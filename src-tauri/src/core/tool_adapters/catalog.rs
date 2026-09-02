//! Presentation-ready Tool catalogs.
//!
//! The command tier maps these entries 1:1 onto `ToolInfoDto`; every rule
//! about which adapters appear, how virtual groups absorb their constituent
//! tools, how shared skills dirs group, and what "installed" means lives here
//! so it is testable against a temp home.

use std::path::Path;

use super::{
    adapters_sharing_skills_dir, constituents_of, is_installed_in, ToolAdapter, TOOL_ADAPTERS,
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
    /// The dir itself is deliberately absent: it is absolute under the
    /// operator's home at global scope but project-relative at project scope,
    /// so one field could not carry both honestly, and no consumer needs the
    /// path — only the grouping it induces.
    pub shared_with: Vec<&'static str>,
    /// Display names of the constituent tools absorbed into this entry when
    /// it is a virtual group; empty for real tools.
    pub constituents: Vec<&'static str>,
}

/// Global-scope catalog: every real tool (virtual groups are project-only),
/// grouped by shared global skills dir.
pub fn global_tool_entries(home: &Path) -> Vec<ToolCatalogEntry> {
    TOOL_ADAPTERS
        .iter()
        .filter(|adapter| !adapter.is_virtual_group())
        .map(|adapter| ToolCatalogEntry {
            key: adapter.key(),
            label: adapter.display_name,
            installed: is_installed_in(home, adapter),
            shared_with: adapters_sharing_skills_dir(adapter)
                .into_iter()
                .filter(|a| !a.is_virtual_group())
                .map(ToolAdapter::key)
                .collect(),
            constituents: vec![],
        })
        .collect()
}

/// Project-scope catalog: constituent tools are absorbed into their virtual
/// group's entry, which counts as installed when any constituent is. Entries
/// are grouped by shared project skills dir.
pub fn project_tool_entries(home: &Path) -> Vec<ToolCatalogEntry> {
    let listed: Vec<&'static ToolAdapter> = TOOL_ADAPTERS
        .iter()
        .filter(|adapter| adapter.group.is_none())
        .collect();

    listed
        .iter()
        .map(|adapter| {
            let (installed, constituents) = match adapter.as_virtual_group() {
                Some(group) => {
                    let members: Vec<&'static ToolAdapter> = constituents_of(group).collect();
                    (
                        members.iter().any(|a| is_installed_in(home, a)),
                        members.iter().map(|a| a.display_name).collect(),
                    )
                }
                None => (is_installed_in(home, adapter), vec![]),
            };
            ToolCatalogEntry {
                key: adapter.key(),
                label: adapter.display_name,
                installed,
                shared_with: listed
                    .iter()
                    .filter(|a| {
                        a.project_relative_skills_dir == adapter.project_relative_skills_dir
                    })
                    .map(|a| a.key())
                    .collect(),
                constituents,
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
