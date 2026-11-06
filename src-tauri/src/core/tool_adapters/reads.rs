//! Shared-folder read facts: which shared skills folders a harness reads, at
//! each scope, what wins when one skill name is in two of them, what the
//! harness does with a duplicate, and which other harnesses' folders it also
//! reads.
//!
//! Every value is a registry literal (`ToolAdapter::reads`) citing its row in
//! `docs/design-inputs/2026-11-05-harness-candidates-and-shared-dir-reads.md`
//! § Part 2 (the **brief**). LIKELY and UNKNOWN evidence is `Unknown`.
//! Changing a fact means editing that brief row and its source in the same
//! commit (spec Q20). The facts stay in the backend; the one thing derived
//! from them that reaches the UI is the cross-read warning's pairs
//! (`catalog::ToolCatalogEntry::duplicate_readers`).

use serde::Serialize;

use super::RegistryPath;

/// Whether a harness reads one shared skills folder (`~/.agents/skills`,
/// `~/.claude/skills`, or their project equivalents).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharedRead {
    /// Read by default (a setting may turn it off: still `Yes`).
    Yes,
    /// Read only under the stated condition. Never warns.
    Conditional(&'static str),
    /// Read only after the operator opts in. Never warns.
    OptIn,
    /// Not read (the brief cites an absence source).
    No,
    /// No verified source (the brief says LIKELY or UNKNOWN).
    Unknown,
}

/// The two shared folders at one scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScopeReads {
    /// `.agents/skills` (under home globally, under the project root).
    pub agents: SharedRead,
    /// `.claude/skills` (under home globally, under the project root).
    pub claude: SharedRead,
}

/// What wins when one skill name is in the harness's own folder and a shared
/// folder it reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Precedence {
    Own,
    Shared,
    /// Both copies are listed.
    Both,
    Unknown,
}

/// What a harness does when it reads the same skill twice — the consequence
/// the cross-read warning names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateBehaviour {
    /// Every copy is listed.
    ShowsAll,
    /// One copy wins; the other is hidden (often with a warning).
    Shadows,
    /// An ambiguous name loads neither copy: invocation by name breaks (fx).
    LoadsNone,
    /// Differing copies are flagged as a conflict (Cortex Code).
    FlagsConflict,
    Unknown,
}

/// How a harness reads one `also_reads` folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadMode {
    /// Read by the current release by default (a deprecated folder the
    /// current release still reads is `Default`). The only mode that warns.
    Default,
    /// Read only by a legacy release of the harness (Kimi's `kimi-cli`).
    Legacy,
    /// Read only after the operator opts in.
    #[allow(
        dead_code,
        reason = "no existing row has an opt-in cross-read yet; later rows (tickets 05–07) may"
    )]
    OptIn,
    /// Read only under the stated condition.
    Conditional(&'static str),
    /// The brief could not establish whether it is read.
    Unknown,
}

/// One more folder a harness reads, beyond the two shared ones: usually
/// another harness's own folder (Cursor reads `~/.codex/skills`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlsoRead {
    /// Root-relative, like every registry path (global scope).
    pub dir: RegistryPath,
    pub mode: ReadMode,
}

/// Every shared-folder read fact of one registry row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadFacts {
    pub global: ScopeReads,
    pub project: ScopeReads,
    pub precedence: Precedence,
    pub duplicates: DuplicateBehaviour,
    /// Other global folders it reads, each with its mode. A row's own
    /// deploy dir is read by default unless an entry here says otherwise
    /// (Kimi's `.config/agents/skills` is read only by legacy `kimi-cli`).
    pub also_reads: &'static [AlsoRead],
}

/// `~/.agents/skills`, the AGENTS convention's global folder.
pub const AGENTS_SKILLS: RegistryPath = RegistryPath::home(".agents/skills");
/// `~/.claude/skills`, Claude Code's global folder, read by many harnesses.
pub const CLAUDE_SKILLS: RegistryPath = RegistryPath::home(".claude/skills");
/// `<project>/.agents/skills`, the AGENTS convention's project folder.
pub const AGENTS_PROJECT_DIR: &str = ".agents/skills";
/// `<project>/.claude/skills`, Claude Code's project folder.
pub const CLAUDE_PROJECT_DIR: &str = ".claude/skills";

impl ReadFacts {
    /// Facts for a row the brief has no evidence for (or retires).
    pub const UNKNOWN: ReadFacts = ReadFacts {
        global: ScopeReads::UNKNOWN,
        project: ScopeReads::UNKNOWN,
        precedence: Precedence::Unknown,
        duplicates: DuplicateBehaviour::Unknown,
        also_reads: &[],
    };

    /// Whether the harness reads the global folder `dir` **by default** —
    /// the only kind of read the cross-read warning counts. `own` is the
    /// row's own deploy dir, read by default unless `also_reads` marks it.
    pub fn reads_by_default(&self, dir: &RegistryPath, own: &RegistryPath) -> bool {
        if dir.same_location(&AGENTS_SKILLS) {
            return self.global.agents == SharedRead::Yes;
        }
        if dir.same_location(&CLAUDE_SKILLS) {
            return self.global.claude == SharedRead::Yes;
        }
        if let Some(entry) = self.also_reads.iter().find(|a| a.dir.same_location(dir)) {
            return entry.mode == ReadMode::Default;
        }
        dir.same_location(own)
    }
}

impl ScopeReads {
    pub const UNKNOWN: ScopeReads = ScopeReads {
        agents: SharedRead::Unknown,
        claude: SharedRead::Unknown,
    };
}
