//! Presentation-ready Tool catalogs.
//!
//! The command tier maps these entries 1:1 onto `ToolInfoDto`; every rule
//! about which adapters appear, how virtual groups absorb their constituent
//! tools, how shared skills dirs group, and what "installed" means lives here
//! so it is testable against a temp home.

use std::path::{Path, PathBuf};

use super::{
    adapters_sharing_skills_dir, constituents_of, is_installed_in, AgentsStandardRead,
    GlobalLayout, Precedence, ToolAdapter, TOOL_ADAPTERS,
};

/// One row of a tool list, with every backend-owned fact already resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCatalogEntry {
    pub key: &'static str,
    pub label: &'static str,
    pub installed: bool,
    /// Keys of every entry in this list sharing this entry's skills dir — the
    /// global dir it deploys to (under the `GlobalLayout`) for the global
    /// catalog, the project dir for the project catalog — in registry order,
    /// including the entry itself (len >= 1).
    ///
    /// The dir itself is deliberately absent: it is absolute under the
    /// operator's home at global scope but project-relative at project scope,
    /// so one field could not carry both honestly, and no consumer needs the
    /// path — only the grouping it induces.
    pub shared_with: Vec<&'static str>,
    /// Display names of the constituent tools absorbed into this entry when
    /// it is a virtual group; empty for real tools.
    pub constituents: Vec<&'static str>,
    /// **Cross-read**, resolved: keys of the *installed* entries, other than
    /// this one and its `shared_with`, whose deployment dir this harness also
    /// reads — its registry `also_reads` dirs, plus the AGENTS-standard dir
    /// when it VERIFIED-reads that one. A skill deployed to this entry and to
    /// one of these shows twice in this harness. Empty at project scope.
    pub also_reads: Vec<&'static str>,
    /// The registry's VERIFIED `~/.agents/skills` read; `None` at project scope.
    pub agents_standard_read: Option<AgentsStandardRead>,
    /// An `OwnWins` harness the layout moved to the AGENTS-standard dir whose
    /// own dir still holds a copy, not recorded as ours, of a skill we
    /// deployed to the AGENTS-standard dir: that copy shadows ours there.
    pub shadowed_by_own_dir: bool,
}

/// Global-scope catalog: every registry entry, grouped by the global dir it
/// deploys to under `layout`.
///
/// A virtual group is listed here **alongside** its constituents: globally it
/// absorbs nothing at project grain, it is simply one more directory some
/// tools deploy to or read. So it carries no constituent roster — its sharers
/// (Codex, Goose, OpenHands; every AGENTS-standard reader under the
/// preference) are its `shared_with` — its label is the plain
/// `display_name`, and its installedness is its own detect dir like any
/// other entry.
///
/// `recorded_targets` are the artifact paths the store records (every target
/// row); only `shadowed_by_own_dir` reads them.
pub fn global_tool_entries(
    home: &Path,
    layout: GlobalLayout,
    recorded_targets: &[PathBuf],
) -> Vec<ToolCatalogEntry> {
    let installed: Vec<bool> = TOOL_ADAPTERS
        .iter()
        .map(|adapter| is_installed_in(home, adapter))
        .collect();
    TOOL_ADAPTERS
        .iter()
        .zip(&installed)
        .map(|(adapter, &is_installed)| ToolCatalogEntry {
            key: adapter.key(),
            label: adapter.display_name,
            installed: is_installed,
            shared_with: adapters_sharing_skills_dir(adapter, layout)
                .into_iter()
                .map(ToolAdapter::key)
                .collect(),
            constituents: vec![],
            also_reads: resolved_cross_reads(adapter, layout, &installed),
            agents_standard_read: adapter.reads_agents_standard_globally,
            shadowed_by_own_dir: shadowed_by_own_dir(home, adapter, layout, recorded_targets),
        })
        .collect()
}

/// Installed global entries' keys, in registry order — installedness does not
/// depend on the layout or the store.
pub fn installed_global_keys(home: &Path) -> Vec<String> {
    TOOL_ADAPTERS
        .iter()
        .filter(|adapter| is_installed_in(home, adapter))
        .map(|adapter| adapter.key().to_string())
        .collect()
}

fn resolved_cross_reads(
    adapter: &ToolAdapter,
    layout: GlobalLayout,
    installed: &[bool],
) -> Vec<&'static str> {
    let own = adapter.global_relative_skills_dir(layout);
    let read: Vec<&str> = adapter.other_global_reads().collect();
    TOOL_ADAPTERS
        .iter()
        .zip(installed)
        .filter(|(other, &is_installed)| {
            let dir = other.global_relative_skills_dir(layout);
            is_installed && other.id != adapter.id && dir != own && read.contains(&dir)
        })
        .map(|(other, _)| other.key())
        .collect()
}

fn shadowed_by_own_dir(
    home: &Path,
    adapter: &ToolAdapter,
    layout: GlobalLayout,
    recorded_targets: &[PathBuf],
) -> bool {
    let own_wins = matches!(
        adapter.reads_agents_standard_globally,
        Some(AgentsStandardRead {
            precedence: Precedence::OwnWins,
            ..
        })
    );
    let deployed = adapter.global_relative_skills_dir(layout);
    if !own_wins || deployed == adapter.relative_skills_dir {
        return false;
    }
    let deployed_dir = home.join(deployed);
    let own_dir = home.join(adapter.relative_skills_dir);
    recorded_targets
        .iter()
        .filter(|path| path.parent() == Some(deployed_dir.as_path()))
        .filter_map(|path| path.file_name())
        .any(|name| {
            let own_copy = own_dir.join(name);
            own_copy.symlink_metadata().is_ok() && !recorded_targets.contains(&own_copy)
        })
}

/// Project-scope catalog: constituent tools are absorbed into their virtual
/// group's entry, which counts as installed when any constituent is. Entries
/// are grouped by shared project skills dir. A global-only Tool (no project
/// dir) is absent.
pub fn project_tool_entries(home: &Path) -> Vec<ToolCatalogEntry> {
    let listed: Vec<&'static ToolAdapter> = TOOL_ADAPTERS
        .iter()
        .filter(|adapter| adapter.group.is_none() && adapter.project_relative_skills_dir.is_some())
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
                label: adapter.project_display_name(),
                installed,
                shared_with: listed
                    .iter()
                    .filter(|a| {
                        a.project_relative_skills_dir == adapter.project_relative_skills_dir
                    })
                    .map(|a| a.key())
                    .collect(),
                constituents,
                also_reads: vec![],
                agents_standard_read: None,
                shadowed_by_own_dir: false,
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
