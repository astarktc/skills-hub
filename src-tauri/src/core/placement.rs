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
//! the dir it was written to) and one rule, [`is_folder_row`]: a row keyed by
//! its own folder key is a **folder row** (reachable through any installed
//! harness its folder serves); a row with any other key is a **legacy row**
//! and keeps 1.3.4's meaning in every mutation — its own harness's
//! installedness, Enable's lifecycle, project-tool removal — until the
//! redeploy converts it (ADR-0007). For display and counting only, the rows
//! of one folder are folded by [`fold_folder_rows`] / [`fold_status`] into
//! facts about the whole folder ([`FolderFacts`]), so rows written once per
//! shared-dir sibling before 1.4.0 (Amp and Kimi on `~/.config/agents/skills`)
//! read as one folder. Mutations never group rows: each acts on every stored
//! row by itself, with its own ownership and settlement.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

use super::environment::HostRoots;
use super::errors::SignalError;
use super::skill_store::{
    ProjectSkillAssignmentRecord, SkillProjectDeploymentRecord, SkillTargetRecord,
};
use super::sync_status::{SyncMode, SyncStatus};
use super::tool_adapters::reads::{
    ReadFacts, SharedRead, AGENTS_PROJECT_DIR, AGENTS_SKILLS, CLAUDE_PROJECT_DIR, CLAUDE_SKILLS,
};
use super::tool_adapters::{
    adapter_by_key, catalog_rows, deletable_skills_dirs_in, former_skills_dirs_in,
    project_skills_dir_in, reader_only_harnesses, skills_dir_in, ToolAdapter, ToolId,
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

/// The skills folders a global row keyed `key` may lie in — the only places
/// the app removes or replaces its folder (ADR-0007: "only inside a skills
/// folder it manages"): the current and former registry skills dirs of the
/// row's own harness and of the folder it describes ([`row_folder`]). A key
/// the registry no longer knows names no folder of its own, so its row may
/// lie in any registry skills dir (`tool_adapters::deletable_skills_dirs_in`,
/// the registry's deletion rule). The removal and replacement fences claim
/// these physically (`removal_fence::Claim::Within`).
pub fn managed_global_dirs(roots: &HostRoots, key: &str) -> Vec<PathBuf> {
    let folder = row_folder(ScopeKind::Global, key);
    let entries: Vec<&ToolAdapter> = [adapter_by_key(key), adapter_by_key(&folder)]
        .into_iter()
        .flatten()
        .collect();
    if entries.is_empty() {
        return deletable_skills_dirs_in(roots);
    }
    let mut dirs: Vec<PathBuf> = Vec::new();
    for entry in entries {
        for dir in
            std::iter::once(skills_dir_in(roots, entry)).chain(former_skills_dirs_in(roots, entry))
        {
            if !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
    }
    dirs
}

/// The `INVALID_PATH` reason [`deployment_path`] refuses a name with.
pub(crate) const NOT_A_FOLDER_NAME: &str = "not_a_folder_name";

/// Whether `name` can name a Deployment's folder: exactly one normal path
/// component, spelled as itself — not empty, not `.` or `..`, not absolute or
/// drive-relative, and with no separator of any platform (`/`, `\`) or NUL.
/// The one skill-name rule: the Library applies it too, from install onward
/// (`install_finalize::library_path`, `unlocatable::unlocatable_state`).
pub(crate) fn is_folder_name(name: &str) -> bool {
    if name.contains(['/', '\\', '\0']) {
        return false;
    }
    let mut components = Path::new(name).components();
    matches!(
        (components.next(), components.next()),
        (Some(Component::Normal(only)), None) if only == OsStr::new(name)
    )
}

/// Where the skill (or project assignment) named `name` lives in the skills
/// folder `skills_dir`: the one place a name becomes a Deployment's path, so a
/// derived destination is always a direct child of the folder it was derived
/// in (ADR-0007: "only inside a skills folder it manages"). A name that is not
/// one folder name ([`is_folder_name`] — `../../Documents/alpha`, an absolute
/// path) is refused before anything touches the disk: the typed
/// `InvalidPath { reason: "not_a_folder_name" }`, whose `path` is the place
/// the name would have reached, joined lexically for the report only
/// ([`refused_place`]) — never a destination.
pub(crate) fn deployment_path(skills_dir: &Path, name: &str) -> anyhow::Result<PathBuf> {
    if is_folder_name(name) {
        return Ok(skills_dir.join(name));
    }
    anyhow::bail!(SignalError::InvalidPath {
        path: skills_dir.join(name).to_string_lossy().into_owned(),
        reason: NOT_A_FOLDER_NAME.to_string(),
    })
}

/// The place a [`deployment_path`] refusal names, when `err` is one: what a
/// report shows for the refused row, and what a protection set ("keep what
/// this reaches") may include to stay cautious. Never a write or removal
/// destination.
pub(crate) fn refused_place(err: &anyhow::Error) -> Option<PathBuf> {
    match err.downcast_ref::<SignalError>() {
        Some(SignalError::InvalidPath { path, reason }) if reason == NOT_A_FOLDER_NAME => {
            Some(PathBuf::from(path))
        }
        _ => None,
    }
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
// Folder rows and legacy rows — the one rule mutations read
// ---------------------------------------------------------------------------

/// Whether a stored row keyed `key` is a **folder row** at `scope`: its key is
/// the folder key of the dir it was written to (`own_folder(scope, key) ==
/// key`). Every row this app writes from 1.4.0 on is one, and so is every row
/// of a harness whose skills dir no other entry shares.
///
/// A row with any other key — a pre-1.4.0 per-sharer row (Kimi Code's on
/// Amp's `~/.config/agents/skills`, Trae CN's on `.trae/skills`) or a key the
/// registry no longer knows — is a **legacy row**, and keeps its old meaning in
/// every mutation until the redeploy converts it (ADR-0007 rule 2): its
/// destination, its own harness's installedness, Enable's lifecycle and the
/// project-tool removal all read as on 1.3.4. A folder row is reachable when
/// any installed harness is served by its folder ([`folder_reachable`]); both
/// kinds take their capability through placement.
pub fn is_folder_row(scope: ScopeKind, key: &str) -> bool {
    own_folder(scope, key) == Some(key)
}

/// Whether the global folder `folder` is reachable on this machine: any
/// harness it serves is installed (`installed` is the caller's detected set).
/// What a **folder row** is gated on; a legacy row is gated on its own
/// harness ([`is_folder_row`]).
pub fn folder_reachable(scope: ScopeKind, folder: &str, installed: &InstalledSet) -> bool {
    scope_rows(scope)
        .into_iter()
        .filter(|adapter| {
            serving_folder(scope, adapter.key(), &PlacementSettings::default()) == Some(folder)
        })
        .any(|adapter| installed.contains(adapter.key()))
}

/// Whether the stored row keyed `key` is reachable for a mutation at
/// `scope`: a folder row through its folder ([`folder_reachable`]), a legacy
/// row through its own harness — exactly as before rows were per folder.
pub fn row_reachable(scope: ScopeKind, key: &str, installed: &InstalledSet) -> bool {
    if is_folder_row(scope, key) {
        folder_reachable(scope, key, installed)
    } else {
        installed.contains(key)
    }
}

// ---------------------------------------------------------------------------
// Reading stored rows as folders — presentation and counting only
// ---------------------------------------------------------------------------

/// What [`fold_status`] reads of one stored row: its status, and whether it
/// owns the artifact at its path (`synced_at` set).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowState {
    pub status: SyncStatus,
    pub owns_artifact: bool,
}

/// One Deployment folder's state, folded over **all** its stored rows — facts
/// about the whole folder, never borrowed from one row for another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FolderFacts {
    /// What the folder shows, failure ahead of muted (the order `skillHealth`
    /// ranks: failed › stale › muted): `error` › `missing` › `stale` ›
    /// `disabled` › `pending` › `synced`. A row whose removal failed stays
    /// visible beside a sibling Disable did settle.
    pub status: SyncStatus,
    /// The operator's Disable holds on the folder: some row is `disabled`.
    /// Every Sync-target path withholds a write to such a folder (see
    /// **Disabled** in `CONTEXT.md`), so its failure is repaired by finishing
    /// the Disable or by Enable — never a re-sync the gate would skip.
    pub disabled: bool,
    /// Some row that is not `disabled` owns its artifact — what a Disable or
    /// a Delete of the folder would remove. Presentation only: it counts an
    /// artifact for a confirmation and is never any row's ownership.
    pub deployed: bool,
}

impl FolderFacts {
    /// What the folder contributes to a project total
    /// (`sync_status::aggregate`): its displayed status, except that a
    /// `disabled` folder with a live artifact still on disk counts as
    /// deployed there — a total never reads empty over a link it can see.
    pub fn total_status(&self) -> SyncStatus {
        if self.status == SyncStatus::Disabled && self.deployed {
            SyncStatus::Synced
        } else {
            self.status
        }
    }
}

/// The displayed rank of a row status (higher shows): failure › stale ›
/// muted › in flight › healthy.
fn display_rank(status: SyncStatus) -> u8 {
    match status {
        SyncStatus::Error => 6,
        SyncStatus::Missing => 5,
        SyncStatus::Stale => 4,
        SyncStatus::Disabled => 3,
        SyncStatus::Pending => 2,
        SyncStatus::Synced => 1,
    }
}

/// The one fold of a Deployment folder's rows ([`FolderFacts`]) — what the
/// skill catalog, the project listing and the project total read, so no
/// surface can disagree with another about a folder. Every row is reached by
/// the per-row resync / Retry / Disable of its folder, so whatever this shows,
/// one of those actions settles. `None` for no rows.
pub fn fold_status(rows: impl IntoIterator<Item = RowState>) -> Option<FolderFacts> {
    let mut facts: Option<FolderFacts> = None;
    for row in rows {
        let live_owner = row.status != SyncStatus::Disabled && row.owns_artifact;
        let disabled = row.status == SyncStatus::Disabled;
        facts = Some(match facts {
            None => FolderFacts {
                status: row.status,
                disabled,
                deployed: live_owner,
            },
            Some(current) => FolderFacts {
                status: if display_rank(row.status) > display_rank(current.status) {
                    row.status
                } else {
                    current.status
                },
                disabled: current.disabled || disabled,
                deployed: current.deployed || live_owner,
            },
        });
    }
    facts
}

/// A stored Deployment row, as [`fold_folder_rows`] reads it.
pub trait FolderRow {
    /// What the row belongs to beside its folder: the skill (global rows), or
    /// the project and the skill (project rows).
    fn owner(&self) -> String;
    fn tool(&self) -> &str;
    fn state(&self) -> RowState;
    fn set_tool(&mut self, tool: String);
}

/// One folder of one owner, for display: a row of it standing for the
/// folder's identity — its first row whose status is the folder's displayed
/// status, re-keyed by the folder key, every other field its own (never a
/// sibling's) — and the [`FolderFacts`] of all its rows. Reads as its row
/// (`Deref`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folded<R> {
    pub row: R,
    pub facts: FolderFacts,
}

impl<R> std::ops::Deref for Folded<R> {
    type Target = R;
    fn deref(&self) -> &R {
        &self.row
    }
}

/// One entry per (owner, folder), for display and counting — never for a
/// mutation, which always acts on every stored row by itself. In the order
/// each group first appeared; a folder with a single row passes it through,
/// re-keyed.
pub fn fold_folder_rows<R: FolderRow>(scope: ScopeKind, rows: Vec<R>) -> Vec<Folded<R>> {
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
        .filter_map(|(_, folder, members)| {
            let facts = fold_status(members.iter().map(FolderRow::state))?;
            let mut row = members
                .into_iter()
                .find(|row| row.state().status == facts.status)?;
            row.set_tool(folder);
            Some(Folded { row, facts })
        })
        .collect()
}

impl FolderRow for SkillTargetRecord {
    fn owner(&self) -> String {
        self.skill_id.clone()
    }
    fn tool(&self) -> &str {
        &self.tool
    }
    fn state(&self) -> RowState {
        RowState {
            status: self.status,
            owns_artifact: self.owns_artifact(),
        }
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
    fn state(&self) -> RowState {
        RowState {
            status: self.status,
            owns_artifact: self.owns_artifact(),
        }
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
    fn state(&self) -> RowState {
        RowState {
            status: self.status,
            owns_artifact: self.synced_at.is_some(),
        }
    }
    fn set_tool(&mut self, tool: String) {
        self.tool = tool;
    }
}

#[cfg(test)]
#[path = "tests/placement.rs"]
mod tests;
