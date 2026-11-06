//! Placement: where and how a Deployment should live.
//!
//! One pure answer, per scope, over the registry (its rows, their read facts
//! and capabilities), the settings and the installed set. Nothing here reads
//! the store or the filesystem: the caller hands in the [`InstalledSet`]
//! (`tool_adapters::installed_set_in` detects it) and the
//! [`PlacementSettings`], and every write path asks here instead of
//! re-deriving a folder from a harness.
//!
//! - A **Deployment folder** is one distinct skills dir at one scope. Its
//!   **folder key** is a registry entry key: `agents_skills` for `.agents`,
//!   `claude_code` for `.claude`, the harness's own key otherwise; a dir several
//!   entries share takes the virtual group entry that owns it, else the first
//!   entry in registry order ([`own_folder`]). Rows, the saved selection and the
//!   catalogs carry folder keys; the frontend never derives one from a path.
//! - **Which folder serves a harness** ([`serving_folder`]). Today every harness
//!   is served by its own folder, as the registry has it; the shared-folder
//!   rule (spec Q15) lands here, behind the same interface.
//! - **Per folder** ([`place`]): its path, its deploy mode (today the registry
//!   capability) and who sees it — the detected harnesses it serves, the other
//!   detected harnesses that read it by default, the detected reader-only
//!   harnesses it serves, and every undetected member kept apart for the
//!   Harnesses pane.
//!
//! Stored rows are read through [`row_folder`] (a row describes the folder of
//! the dir it was written to) and collapsed by [`one_row_per_folder`], so rows
//! written once per shared-dir sibling before 1.4.0 (Amp and Kimi on
//! `~/.config/agents/skills`) read as one folder until the redeploy collapses
//! them.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::environment::HostRoots;
use super::skill_store::{
    ProjectSkillAssignmentRecord, SkillProjectDeploymentRecord, SkillTargetRecord,
};
use super::sync_status::{SyncMode, SyncStatus};
use super::tool_adapters::reads::{
    ReadFacts, SharedRead, AGENTS_PROJECT_DIR, AGENTS_SKILLS, CLAUDE_PROJECT_DIR, CLAUDE_SKILLS,
};
use super::tool_adapters::{
    adapter_by_key, catalog_rows, project_skills_dir_in, reader_only_harnesses, ToolAdapter, ToolId,
};

/// The two deployment scopes, without their roots (folder identity needs none).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeKind {
    Global,
    Project,
}

/// A scope with the root its folder paths resolve under.
#[derive(Clone, Copy, Debug)]
pub enum PlacementScope<'a> {
    /// Under the operator's host roots.
    Global(&'a HostRoots),
    /// Under one project's root folder.
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "the project folder cards and overrides read it (tickets 13, 15)"
        )
    )]
    Project(&'a Path),
}

impl PlacementScope<'_> {
    pub fn kind(&self) -> ScopeKind {
        match self {
            PlacementScope::Global(_) => ScopeKind::Global,
            PlacementScope::Project(_) => ScopeKind::Project,
        }
    }
}

/// The detected registry entries and reader-only harnesses, by key — an input,
/// never probed here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InstalledSet {
    keys: BTreeSet<String>,
}

impl InstalledSet {
    pub fn from_keys<I, S>(keys: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            keys: keys.into_iter().map(Into::into).collect(),
        }
    }

    pub fn contains(&self, key: &str) -> bool {
        self.keys.contains(key)
    }
}

/// The settings placement reads. Empty today: every harness is served by its
/// own folder. The shared-folder preference and the both-folders choice
/// (ticket 12) and the deploy-mode levels (ticket 14) arrive as fields here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlacementSettings {}

/// How a folder is deployed into: a link into the Library (symlink, junction on
/// Windows, copy as the last fallback) or a copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeployMode {
    Link,
    Copy,
}

impl DeployMode {
    /// The Sync mode a deploy in this mode tries first — what a row that never
    /// materialised records.
    pub fn first_attempt(self) -> SyncMode {
        match self {
            DeployMode::Link => SyncMode::Symlink,
            DeployMode::Copy => SyncMode::Copy,
        }
    }
}

/// One harness's answer: the folder that serves it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HarnessPlacement {
    pub key: &'static str,
    pub folder_key: &'static str,
    pub path: PathBuf,
}

/// One Deployment folder at one scope, with everyone who sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeploymentFolder {
    pub key: &'static str,
    pub path: PathBuf,
    pub mode: DeployMode,
    /// Detected registry entries this folder serves, in registry order.
    pub served: Vec<&'static str>,
    /// Other detected entries and reader-only harnesses that read this folder
    /// by default while another folder serves them, in registry order.
    pub readers: Vec<&'static str>,
    /// Detected reader-only harnesses served through this folder.
    pub reader_only: Vec<&'static str>,
    /// Undetected entries and reader-only harnesses this folder serves (the
    /// Harnesses pane keeps them reachable; no other surface names them).
    pub undetected: Vec<&'static str>,
}

/// Placement's answer for one scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    pub scope: ScopeKind,
    /// Every registry entry the scope admits, in registry order.
    pub harnesses: Vec<HarnessPlacement>,
    /// Every folder serving at least one of them, in order of first use.
    pub folders: Vec<DeploymentFolder>,
}

impl Placement {
    /// The folder serving `harness`, `None` for a key the scope does not admit.
    pub fn folder_of(&self, harness: &str) -> Option<&DeploymentFolder> {
        let placed = self.harnesses.iter().find(|h| h.key == harness)?;
        self.folder(placed.folder_key)
    }

    pub fn folder(&self, key: &str) -> Option<&DeploymentFolder> {
        self.folders.iter().find(|folder| folder.key == key)
    }
}

/// Placement for one scope: each harness → its folder, each folder → its path,
/// mode and who sees it.
pub fn place(
    scope: PlacementScope<'_>,
    installed: &InstalledSet,
    settings: &PlacementSettings,
) -> Placement {
    let kind = scope.kind();
    let rows = scope_rows(kind);
    let mut harnesses = Vec::new();
    let mut folders: Vec<DeploymentFolder> = Vec::new();
    for adapter in &rows {
        let Some(folder_key) = serving_folder(kind, adapter.key(), settings) else {
            continue;
        };
        let Some(entry) = adapter_by_key(folder_key) else {
            continue;
        };
        let Some(path) = folder_path(&scope, entry) else {
            continue;
        };
        harnesses.push(HarnessPlacement {
            key: adapter.key(),
            folder_key,
            path: path.clone(),
        });
        let index = match folders.iter().position(|f| f.key == folder_key) {
            Some(index) => index,
            None => {
                folders.push(DeploymentFolder {
                    key: folder_key,
                    path,
                    mode: folder_mode(kind, folder_key),
                    served: Vec::new(),
                    readers: Vec::new(),
                    reader_only: Vec::new(),
                    undetected: Vec::new(),
                });
                folders.len() - 1
            }
        };
        let folder = &mut folders[index];
        if installed.contains(adapter.key()) {
            folder.served.push(adapter.key());
        } else {
            folder.undetected.push(adapter.key());
        }
    }

    let readers = reader_only_harnesses();
    for reader in &readers {
        let group_entry = reader.group.entry_id().as_key();
        let Some(folder_key) = serving_folder(kind, group_entry, settings) else {
            continue;
        };
        if let Some(folder) = folders.iter_mut().find(|f| f.key == folder_key) {
            if installed.contains(reader.key) {
                folder.reader_only.push(reader.key);
            } else {
                folder.undetected.push(reader.key);
            }
        }
    }

    for folder in &mut folders {
        let Some(entry) = adapter_by_key(folder.key) else {
            continue;
        };
        for adapter in &rows {
            let own_key = serving_folder(kind, adapter.key(), settings);
            if !installed.contains(adapter.key()) || own_key == Some(folder.key) {
                continue;
            }
            let Some(own) = own_key.and_then(adapter_by_key) else {
                continue;
            };
            if reads_folder(kind, &adapter.reads, entry, own) {
                folder.readers.push(adapter.key());
            }
        }
        for reader in &readers {
            let group_entry = reader.group.entry_id().as_key();
            let own_key = serving_folder(kind, group_entry, settings);
            if !installed.contains(reader.key) || own_key == Some(folder.key) {
                continue;
            }
            let Some(own) = own_key.and_then(adapter_by_key) else {
                continue;
            };
            if reads_folder(kind, &reader.reads, entry, own) {
                folder.readers.push(reader.key);
            }
        }
    }

    Placement {
        scope: kind,
        harnesses,
        folders,
    }
}

/// The folder serving `harness` at `scope`: `None` for a key the registry does
/// not know, or a harness with no skills dir at that scope. Today the harness's
/// own folder ([`own_folder`]); the shared-folder rule replaces this body.
pub fn serving_folder(
    scope: ScopeKind,
    harness: &str,
    _settings: &PlacementSettings,
) -> Option<&'static str> {
    own_folder(scope, harness)
}

/// The folder key of `key`'s own skills dir at `scope` — the identity of the
/// folder a row keyed `key` was written to. `None` for an unknown key, or a
/// harness with no skills dir at that scope.
pub fn own_folder(scope: ScopeKind, key: &str) -> Option<&'static str> {
    let adapter = adapter_by_key(key)?;
    if scope == ScopeKind::Project && adapter.project_skills_dir.is_none() {
        return None;
    }
    if is_agents_dir(scope, adapter) {
        return Some(ToolId::AgentsStandard.as_key());
    }
    if is_claude_dir(scope, adapter) {
        return Some(ToolId::ClaudeCode.as_key());
    }
    let sharers = sharers_of(scope, adapter);
    sharers
        .iter()
        .find(|a| a.as_virtual_group().is_some())
        .or(sharers.first())
        .map(|a| a.key())
        .or(Some(adapter.key()))
}

/// The folder a stored row describes: [`own_folder`] of its key, or the key
/// itself when the registry does not know it (an unknown key stays tolerated
/// and stands for itself).
pub fn row_folder(scope: ScopeKind, key: &str) -> String {
    own_folder(scope, key)
        .map(str::to_string)
        .unwrap_or_else(|| key.to_string())
}

/// The registry entry whose capability governs how `folder_key`'s folder is
/// deployed into: a virtual group's own entry for a folder it owns (a group's
/// capability is its own fact, never its constituents'), otherwise the first
/// copy-only entry sharing the dir, else the folder's own entry.
pub fn capability_entry(scope: ScopeKind, folder_key: &str) -> Option<&'static ToolAdapter> {
    let entry = adapter_by_key(folder_key)?;
    if entry.as_virtual_group().is_some() {
        return Some(entry);
    }
    Some(
        sharers_of(scope, entry)
            .into_iter()
            .find(|a| !a.supports_symlink)
            .unwrap_or(entry),
    )
}

/// The deploy mode of `folder_key`'s folder: today the registry capability
/// ([`capability_entry`]).
pub fn folder_mode(scope: ScopeKind, folder_key: &str) -> DeployMode {
    match capability_entry(scope, folder_key) {
        Some(adapter) if !adapter.supports_symlink => DeployMode::Copy,
        _ => DeployMode::Link,
    }
}

/// The registry rows a scope admits, in registry order (shadow-aware in tests).
fn scope_rows(scope: ScopeKind) -> Vec<&'static ToolAdapter> {
    catalog_rows()
        .filter(|a| scope == ScopeKind::Global || a.project_skills_dir.is_some())
        .collect()
}

/// The entries whose skills dir at `scope` is `adapter`'s, in registry order.
fn sharers_of(scope: ScopeKind, adapter: &ToolAdapter) -> Vec<&'static ToolAdapter> {
    scope_rows(scope)
        .into_iter()
        .filter(|a| match scope {
            ScopeKind::Global => a.skills_dir.same_location(&adapter.skills_dir),
            ScopeKind::Project => a.project_skills_dir == adapter.project_skills_dir,
        })
        .collect()
}

fn is_agents_dir(scope: ScopeKind, adapter: &ToolAdapter) -> bool {
    match scope {
        ScopeKind::Global => adapter.skills_dir.same_location(&AGENTS_SKILLS),
        ScopeKind::Project => adapter.project_skills_dir == Some(AGENTS_PROJECT_DIR),
    }
}

fn is_claude_dir(scope: ScopeKind, adapter: &ToolAdapter) -> bool {
    match scope {
        ScopeKind::Global => adapter.skills_dir.same_location(&CLAUDE_SKILLS),
        ScopeKind::Project => adapter.project_skills_dir == Some(CLAUDE_PROJECT_DIR),
    }
}

fn folder_path(scope: &PlacementScope<'_>, entry: &ToolAdapter) -> Option<PathBuf> {
    match scope {
        PlacementScope::Global(roots) => Some(entry.skills_dir.resolve(roots)),
        PlacementScope::Project(root) => project_skills_dir_in(root, entry),
    }
}

/// Whether a harness with read facts `facts`, served by `own`, reads the folder
/// of `entry` by default at `scope`. Globally the registry's read rule
/// (`ReadFacts::reads_by_default`); in a project only the two shared folders
/// are read facts (a project has no `also_reads`).
fn reads_folder(
    scope: ScopeKind,
    facts: &ReadFacts,
    entry: &ToolAdapter,
    own: &ToolAdapter,
) -> bool {
    match scope {
        ScopeKind::Global => facts.reads_by_default(&entry.skills_dir, &own.skills_dir),
        ScopeKind::Project => {
            if is_agents_dir(scope, entry) {
                facts.project.agents == SharedRead::Yes
            } else if is_claude_dir(scope, entry) {
                facts.project.claude == SharedRead::Yes
            } else {
                false
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Reading stored rows as folders
// ---------------------------------------------------------------------------

/// A stored Deployment row, as [`one_row_per_folder`] reads it.
pub trait FolderRow {
    /// What the row belongs to beside its folder: the skill (global rows), or
    /// the project and the skill (project rows).
    fn owner(&self) -> String;
    fn tool(&self) -> &str;
    fn status(&self) -> SyncStatus;
    fn set_tool(&mut self, tool: String);
}

/// One row per (owner, folder): rows written once per shared-dir sibling read
/// as their folder. Each group's representative is a `disabled` row when any is
/// (one disabled sibling keeps the whole folder off, the batch and Propagation's
/// rule), else the row already keyed by the folder, else the first; it comes out
/// keyed by the folder key, in the order its group first appeared. Rows of a
/// folder of their own pass through unchanged.
pub fn one_row_per_folder<R: FolderRow>(scope: ScopeKind, rows: Vec<R>) -> Vec<R> {
    group_by_folder(scope, rows)
        .into_iter()
        .map(|(folder, mut members)| {
            let mut row = members.swap_remove(representative(&members, &folder));
            row.set_tool(folder);
            row
        })
        .collect()
}

/// One (owner, folder)'s stored rows as the write paths act on them. Folder
/// grouping is **logical** (which folder a row describes, which row stands for
/// it); artifact ownership is **physical**: only rows proven to name the
/// driver's artifact share its ownership and settlement.
#[derive(Debug)]
pub struct FolderRows<R> {
    /// The folder key every row here describes.
    pub folder: String,
    /// The row standing for the folder, by [`one_row_per_folder`]'s rule (a
    /// `disabled` row when any is, else the row keyed by the folder, else the
    /// first) — the folder's state, and the artifact a write acts on.
    pub driver: R,
    /// Other rows proven to name the driver's physical artifact: an owner
    /// among them authorises replacing it, and they settle with the driver.
    pub same_artifact: Vec<R>,
    /// Rows of the folder naming another artifact (a pre-1.4.0 sibling written
    /// under another skill name): each owns only its own artifact, and no
    /// write of the folder's artifact touches or settles it — the redeploy
    /// (ticket 10) reconciles it. Removal still plans it by its own row.
    pub independent: Vec<R>,
}

impl<R> FolderRows<R> {
    /// The driver and every row naming its artifact.
    pub fn artifact_rows(&self) -> impl Iterator<Item = &R> {
        std::iter::once(&self.driver).chain(self.same_artifact.iter())
    }
}

/// Group `rows` by (owner, folder) and split each group by artifact:
/// `same_artifact(driver, row)` decides, per row, whether it names the
/// driver's physical artifact (the caller supplies the physical test — this
/// module reads no filesystem). Groups come out in the order they first
/// appeared; rows keep their order within a group.
pub fn folder_rows<R: FolderRow>(
    scope: ScopeKind,
    rows: Vec<R>,
    same_artifact: impl Fn(&R, &R) -> bool,
) -> Vec<FolderRows<R>> {
    group_by_folder(scope, rows)
        .into_iter()
        .map(|(folder, mut members)| {
            let driver = members.remove(representative(&members, &folder));
            let (same, independent) = members
                .into_iter()
                .partition(|row| same_artifact(&driver, row));
            FolderRows {
                folder,
                driver,
                same_artifact: same,
                independent,
            }
        })
        .collect()
}

/// Rows grouped by (owner, folder), each group keyed by its folder, in order
/// of first appearance.
fn group_by_folder<R: FolderRow>(scope: ScopeKind, rows: Vec<R>) -> Vec<(String, Vec<R>)> {
    let mut groups: Vec<(String, String, Vec<R>)> = Vec::new();
    for row in rows {
        let owner = row.owner();
        let folder = row_folder(scope, row.tool());
        match groups
            .iter_mut()
            .find(|(o, f, _)| *o == owner && *f == folder)
        {
            Some((_, _, members)) => members.push(row),
            None => groups.push((owner, folder, vec![row])),
        }
    }
    groups
        .into_iter()
        .map(|(_, folder, members)| (folder, members))
        .collect()
}

/// The index of the row standing for `folder` among `members` (non-empty): a
/// `disabled` row when any is (one disabled row keeps the folder off), else
/// the row keyed by the folder, else the first. Among several candidates the
/// row keyed by the folder comes first.
fn representative<R: FolderRow>(members: &[R], folder: &str) -> usize {
    let disabled = |r: &R| r.status() == SyncStatus::Disabled;
    members
        .iter()
        .position(|r| disabled(r) && r.tool() == folder)
        .or_else(|| members.iter().position(disabled))
        .or_else(|| members.iter().position(|r| r.tool() == folder))
        .unwrap_or(0)
}

impl FolderRow for SkillTargetRecord {
    fn owner(&self) -> String {
        self.skill_id.clone()
    }
    fn tool(&self) -> &str {
        &self.tool
    }
    fn status(&self) -> SyncStatus {
        self.status
    }
    fn set_tool(&mut self, tool: String) {
        self.tool = tool;
    }
}

impl FolderRow for ProjectSkillAssignmentRecord {
    fn owner(&self) -> String {
        format!("{}\n{}", self.project_id, self.skill_id)
    }
    fn tool(&self) -> &str {
        &self.tool
    }
    fn status(&self) -> SyncStatus {
        self.status
    }
    fn set_tool(&mut self, tool: String) {
        self.tool = tool;
    }
}

impl FolderRow for SkillProjectDeploymentRecord {
    fn owner(&self) -> String {
        format!("{}\n{}", self.project_id, self.skill_id)
    }
    fn tool(&self) -> &str {
        &self.tool
    }
    fn status(&self) -> SyncStatus {
        self.status
    }
    fn set_tool(&mut self, tool: String) {
        self.tool = tool;
    }
}

#[cfg(test)]
#[path = "tests/placement.rs"]
mod tests;
