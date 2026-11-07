//! Tests for `core::redeploy` — moving global Deployments to where placement
//! says they belong (round 20 ticket 10), ported from the former
//! `target_relocation` tests where they still apply. The registry corrections
//! 1.3.5 ships (Antigravity, Kimi Code, Kilo, Xum, Devin) are exercised
//! directly; the `.agents` moves held for 1.4.0 (Codex, Goose) run on a
//! registry shadow ([`agents_shape`]) because they are what collapses several
//! harnesses' rows into one folder.
//!
//! Every case runs against a temp home, a temp database and a central repo
//! beside it (`removal_fence::central_repo_of`), with every harness dir
//! outside the central repo. Installedness is faked through
//! `mark_installed_in`. Nothing touches the operator's home.

// Most link cases are Unix-only (`std::os::unix::fs::symlink`); what they
// import is unused on Windows, where the copy cases run.
#![cfg_attr(not(unix), allow(unused_imports, dead_code))]

use std::fs;
use std::path::{Path, PathBuf};

use crate::core::content_identity::{self, Source};
use crate::core::environment::HostRoots;
use crate::core::errors::{CommandError, DestinationRefusal};
use crate::core::global_sync::OverrideDecision;
use crate::core::placement::PlacementSettings;
use crate::core::redeploy::{
    finish_journal, preview, redeploy, redeploy_unlocked, take_finished_journal, test_seams,
    EditedCopyDecision, JournalStatus, RedeployAnswer, RedeployCause, RedeployKept,
    RedeployLeftReason, RedeployPreview, RedeployReport, RedeploySkip, RedeployStatus,
};
use crate::core::removal_fence::{central_repo_of, EntryId};
use crate::core::skill_store::{SkillRecord, SkillStore, SkillTargetRecord};
use crate::core::sync_status::{SyncMode, SyncStatus};
use crate::core::tool_adapters::{adapter_by_key, mark_installed_in};

const NOW: i64 = 5000;

macro_rules! skipped {
    ($status:expr, $reason:pat) => {
        matches!($status, RedeployStatus::Skipped { reason: $reason })
    };
}

struct Fixture {
    _dir: tempfile::TempDir,
    roots: HostRoots,
    store: SkillStore,
    central: PathBuf,
}

impl Fixture {
    fn home(&self) -> &Path {
        &self.roots.home
    }
    fn at(&self, rel: &str) -> PathBuf {
        let mut path = self.home().to_path_buf();
        path.extend(rel.split('/'));
        path
    }
    fn central_dir(&self) -> PathBuf {
        central_repo_of(&self.store)
    }
    fn install(&self, key: &str) {
        mark_installed_in(&self.roots, adapter_by_key(key).expect("adapter"));
    }
    fn run(&self, answers: &[RedeployAnswer]) -> RedeployReport {
        redeploy(
            &self.store,
            &self.roots,
            &self.central_dir(),
            answers,
            NOW,
            |_| {},
        )
        .expect("redeploy runs")
    }
    fn preview(&self) -> RedeployPreview {
        preview(&self.store, &self.roots, &PlacementSettings::default()).expect("preview")
    }
    fn rows(&self) -> Vec<SkillTargetRecord> {
        self.store.list_skill_targets("alpha").expect("rows")
    }
    fn row(&self, tool: &str) -> Option<SkillTargetRecord> {
        self.store.get_skill_target("alpha", tool).expect("row")
    }
    /// Seed a global row of `alpha` for `tool` at `path`.
    fn seed(&self, tool: &str, path: &Path, mode: SyncMode, status: SyncStatus, claim: bool) {
        let content_hash = (mode == SyncMode::Copy && claim)
            .then(|| content_identity::read(Source::Directory(&self.central)))
            .flatten();
        self.store
            .upsert_skill_target(&SkillTargetRecord {
                id: format!("row-{tool}"),
                skill_id: "alpha".to_string(),
                tool: tool.to_string(),
                target_path: path.to_string_lossy().into_owned(),
                mode,
                status,
                last_error: None,
                synced_at: claim.then_some(1),
                content_hash,
            })
            .expect("seed row");
    }
    /// A live link row of `tool` at `rel`, with its link into the Library.
    #[cfg(unix)]
    fn link_row(&self, tool: &str, rel: &str) -> PathBuf {
        let path = self.at(rel);
        link(&self.central, &path);
        self.seed(tool, &path, SyncMode::Symlink, SyncStatus::Synced, true);
        path
    }
    /// A live copy row of `tool` at `rel`, recording the Library's identity.
    fn copy_row(&self, tool: &str, rel: &str) -> PathBuf {
        let path = self.at(rel);
        copy_of(&self.central, &path);
        self.seed(tool, &path, SyncMode::Copy, SyncStatus::Synced, true);
        path
    }
}

/// One managed skill `alpha` in the central repo; `installed` harnesses.
fn fixture(installed: &[&str]) -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let roots = HostRoots::under_home(&dir.path().join("home"));
    fs::create_dir_all(&roots.home).expect("home");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("schema");
    let central = central_repo_of(&store).join("alpha");
    fs::create_dir_all(&central).expect("central skill");
    fs::write(central.join("SKILL.md"), "---\nname: alpha\n---\n").expect("SKILL.md");
    fs::write(central.join("a.txt"), "v1").expect("a.txt");
    store
        .upsert_skill(&SkillRecord {
            id: "alpha".to_string(),
            name: "alpha".to_string(),
            description: None,
            source_type: "local".to_string(),
            source_ref: None,
            source_subpath: None,
            source_revision: None,
            central_path: central.to_string_lossy().into_owned(),
            content_hash: None,
            created_at: 1,
            updated_at: 1,
            last_sync_at: None,
            last_seen_at: 1,
            status: "ok".to_string(),
            imported_from_tool: None,
        })
        .expect("seed skill");
    let f = Fixture {
        _dir: dir,
        roots,
        store,
        central,
    };
    for key in installed {
        f.install(key);
    }
    f
}

/// The `.agents` moves held for 1.4.0 (release split; ticket 12): Codex and
/// Goose deploy to `~/.agents/skills`, their old dirs former dirs, Codex still
/// reading its deprecated folder. 1.3.5 ships without them, but it is the
/// shape that collapses several harnesses' rows into one folder (Q24) — the
/// redeploy must handle it the day ticket 12 lands — so these tests run on
/// it as a registry shadow.
fn agents_shape() {
    use crate::core::tool_adapters::reads::{AlsoRead, ReadMode};
    use crate::core::tool_adapters::{test_overrides, RegistryPath, ToolAdapter};
    const CODEX_FORMER: &[RegistryPath] = &[RegistryPath::home(".codex/skills")];
    const CODEX_ALSO: &[AlsoRead] = &[AlsoRead {
        dir: RegistryPath::home(".codex/skills"),
        mode: ReadMode::Default,
    }];
    const GOOSE_FORMER: &[RegistryPath] = &[RegistryPath::home(".config/goose/skills")];
    let codex = adapter_by_key("codex").expect("codex").clone();
    test_overrides::shadow(ToolAdapter {
        skills_dir: RegistryPath::home(".agents/skills"),
        former_skills_dirs: CODEX_FORMER,
        reads: crate::core::tool_adapters::reads::ReadFacts {
            also_reads: CODEX_ALSO,
            ..codex.reads
        },
        ..codex
    });
    let goose = adapter_by_key("goose").expect("goose").clone();
    test_overrides::shadow(ToolAdapter {
        skills_dir: RegistryPath::home(".agents/skills"),
        former_skills_dirs: GOOSE_FORMER,
        ..goose
    });
}

/// [`fixture`] on the [`agents_shape`].
fn agents_fixture(installed: &[&str]) -> Fixture {
    agents_shape();
    fixture(installed)
}

#[cfg(unix)]
fn link(source: &Path, target: &Path) {
    fs::create_dir_all(target.parent().unwrap()).expect("link parent");
    std::os::unix::fs::symlink(source, target).expect("symlink");
}

fn copy_of(source: &Path, target: &Path) {
    fs::create_dir_all(target).expect("copy dir");
    for entry in fs::read_dir(source).expect("read source") {
        let entry = entry.unwrap();
        fs::copy(entry.path(), target.join(entry.file_name())).expect("copy file");
    }
}

fn present(path: &Path) -> bool {
    path.symlink_metadata().is_ok()
}

fn only_move(report: &RedeployReport) -> &crate::core::redeploy::RedeployMove {
    assert_eq!(report.moves.len(), 1, "{report:#?}");
    &report.moves[0]
}

// ---------------------------------------------------------------------------
// The move: deploy first, switch, remove what is provably ours
// ---------------------------------------------------------------------------

/// Codex's correction (Q12, held for 1.4.0 — [`agents_shape`]): its link in
/// `~/.codex/skills` moves to `~/.agents/skills`, the folder that serves it
/// then; the row becomes that folder's row and the old link — ours — is
/// removed.
#[cfg(unix)]
#[test]
fn a_corrected_link_moves_to_its_folder_and_the_old_link_is_removed() {
    let f = agents_fixture(&["codex"]);
    let old = f.link_row("codex", ".codex/skills/alpha");

    let report = f.run(&[]);

    let moved = only_move(&report);
    assert!(matches!(moved.status, RedeployStatus::Moved), "{moved:?}");
    assert_eq!(moved.folder_key, "agents_skills");
    assert_eq!(moved.cause, RedeployCause::Correction);
    let new = f.at(".agents/skills/alpha");
    assert_eq!(fs::read_link(&new).unwrap(), f.central);
    assert!(!present(&old), "the old link is ours and removed");
    assert!(
        f.at(".codex/skills").is_dir(),
        "the old folder itself stays"
    );
    assert!(f.row("codex").is_none(), "the legacy row is converted");
    let row = f.row("agents_skills").expect("the folder's row");
    assert_eq!(row.id, "row-codex", "the row keeps its identity");
    assert_eq!(Path::new(&row.target_path), new);
    assert_eq!(row.status, SyncStatus::Synced);
    assert_eq!(row.synced_at, Some(NOW));
    assert!(report.journal.is_empty() && report.left_behind.is_empty());
    assert!(f.store.list_redeploy_journal().unwrap().is_empty());
}

/// Deploy first: the old artifact is still there when the new one is
/// recorded (a crash after the switch leaves a duplicate, never a gap).
#[cfg(unix)]
#[test]
fn the_old_artifact_is_present_until_the_new_one_is_recorded() {
    let f = agents_fixture(&["codex"]);
    let old = f.link_row("codex", ".codex/skills/alpha");

    test_seams::crash_after_switch();
    let crashed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        redeploy_unlocked(
            &f.store,
            &f.roots,
            &f.central_dir(),
            &PlacementSettings::default(),
            &[],
            NOW,
            |_| {},
        )
    }))
    .is_err();

    assert!(crashed);
    assert!(present(&old), "the old link is still there");
    assert!(present(&f.at(".agents/skills/alpha")), "the new one too");
    let row = f.row("agents_skills").expect("switched");
    assert_eq!(Path::new(&row.target_path), f.at(".agents/skills/alpha"));
    let journal = f.store.list_redeploy_journal().unwrap();
    assert_eq!(journal.len(), 1, "the removal is owed");
    assert_eq!(Path::new(&journal[0].old_path), old);
}

/// A legacy copy row recorded no identity (pre-1.4.0): its copy is removed
/// only because it is byte-identical to the Library.
#[test]
fn an_unrecorded_old_copy_identical_to_the_library_is_removed() {
    let f = fixture(&["augment"]);
    let old = f.at(".augment/rules/alpha");
    copy_of(&f.central, &old);
    f.seed("augment", &old, SyncMode::Copy, SyncStatus::Synced, true);
    let mut row = f.row("augment").unwrap();
    row.content_hash = None;
    f.store.upsert_skill_target(&row).unwrap();

    let report = f.run(&[]);

    assert!(matches!(only_move(&report).status, RedeployStatus::Moved));
    assert!(!present(&old), "identical to the Library: a duplicate");
    assert!(present(&f.at(".augment/skills/alpha")));
    assert!(f.at(".augment/rules").is_dir(), "the rules dir stays");
}

/// Ours (or identical content) already at the new place is adopted, not
/// asked about.
#[cfg(unix)]
#[test]
fn identical_content_at_the_new_place_is_adopted() {
    let f = agents_fixture(&["codex"]);
    let old = f.link_row("codex", ".codex/skills/alpha");
    copy_of(&f.central, &f.at(".agents/skills/alpha"));

    let report = f.run(&[]);

    assert!(
        matches!(only_move(&report).status, RedeployStatus::Adopted),
        "{report:#?}"
    );
    assert!(!present(&old));
    assert!(present(&f.at(".agents/skills/alpha")));
    assert_eq!(
        Path::new(&f.row("agents_skills").unwrap().target_path),
        f.at(".agents/skills/alpha")
    );
}

/// A row already in the folder's place with the folder's key is no move.
#[test]
fn a_deployment_already_in_place_is_no_move() {
    let f = fixture(&["claude_code"]);
    f.copy_row("claude_code", ".claude/skills/alpha");

    assert_eq!(f.preview(), RedeployPreview::default());
    assert!(f.run(&[]).moves.is_empty());
    assert_eq!(
        f.row("claude_code").unwrap().synced_at,
        Some(1),
        "untouched"
    );
}

/// Idempotent: a second run finds nothing to do.
#[cfg(unix)]
#[test]
fn a_second_run_is_a_no_op() {
    let f = agents_fixture(&["codex", "kilo_code"]);
    f.link_row("codex", ".codex/skills/alpha");
    f.link_row("kilo_code", ".kilocode/skills/alpha");
    assert_eq!(f.preview().global.correction, 2);

    assert_eq!(f.run(&[]).moves.len(), 2);

    assert_eq!(f.preview(), RedeployPreview::default());
    let second = f.run(&[]);
    assert!(
        second.moves.is_empty() && second.journal.is_empty(),
        "{second:#?}"
    );
}

/// Rows of a harness that is not installed keep their old meaning: the
/// redeploy does not move them (nor does the preview count them).
#[cfg(unix)]
#[test]
fn a_harness_that_is_not_installed_is_left_as_recorded() {
    let f = agents_fixture(&[]);
    let old = f.link_row("codex", ".codex/skills/alpha");

    assert_eq!(f.preview(), RedeployPreview::default());
    assert!(f.run(&[]).moves.is_empty());
    assert!(present(&old));
    assert!(f.row("codex").is_some());
}

// ---------------------------------------------------------------------------
// The overwrite ask: a different folder at the new place
// ---------------------------------------------------------------------------

fn foreign_at(f: &Fixture, rel: &str) -> PathBuf {
    let path = f.at(rel);
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("SKILL.md"), "someone else's").unwrap();
    path
}

/// A different folder in the way is a `TARGET_EXISTS` row: nothing moves.
#[cfg(unix)]
#[test]
fn a_different_folder_at_the_new_place_is_asked_about() {
    let f = agents_fixture(&["codex"]);
    let old = f.link_row("codex", ".codex/skills/alpha");
    let foreign = foreign_at(&f, ".agents/skills/alpha");

    let report = f.run(&[]);

    match &only_move(&report).status {
        RedeployStatus::Failed {
            error: CommandError::TargetExists {
                path, same_content, ..
            },
        } => {
            assert_eq!(Path::new(path), foreign);
            assert!(!same_content);
        }
        other => panic!("{other:?}"),
    }
    assert!(present(&old), "nothing moved");
    assert_eq!(
        fs::read_to_string(foreign.join("SKILL.md")).unwrap(),
        "someone else's"
    );
    assert!(f.row("codex").is_some(), "the row is unchanged");
}

/// The answer the ask raised for Codex's move: its one row and the place
/// it showed.
fn occupied(f: &Fixture, decision: OverrideDecision) -> RedeployAnswer {
    RedeployAnswer::Occupied {
        skill_id: "alpha".into(),
        folder_key: "agents_skills".into(),
        row_ids: vec!["row-codex".into()],
        path: f.at(".agents/skills/alpha").to_string_lossy().into_owned(),
        decision,
    }
}

/// Keep leaves the move undone and pending: both folders and the row stay.
#[cfg(unix)]
#[test]
fn keep_leaves_the_move_undone_and_pending() {
    let f = agents_fixture(&["codex"]);
    let old = f.link_row("codex", ".codex/skills/alpha");
    let foreign = foreign_at(&f, ".agents/skills/alpha");

    let report = f.run(&[occupied(&f, OverrideDecision::Keep)]);

    assert!(matches!(
        only_move(&report).status,
        RedeployStatus::Kept {
            reason: RedeployKept::Declined
        }
    ));
    assert!(present(&old));
    assert!(foreign.join("SKILL.md").is_file());
    let row = f.row("codex").expect("unchanged");
    assert_eq!(row.status, SyncStatus::Synced);
    assert_eq!(f.preview().global.correction, 1, "still pending");
}

/// Overwrite replaces the folder in the way, then the move completes.
#[cfg(unix)]
#[test]
fn overwrite_replaces_the_folder_in_the_way() {
    let f = agents_fixture(&["codex"]);
    let old = f.link_row("codex", ".codex/skills/alpha");
    foreign_at(&f, ".agents/skills/alpha");

    let report = f.run(&[occupied(&f, OverrideDecision::Overwrite)]);

    assert!(matches!(only_move(&report).status, RedeployStatus::Moved));
    assert!(report.refused_answers.is_empty(), "{report:#?}");
    assert_eq!(
        fs::read_link(f.at(".agents/skills/alpha")).unwrap(),
        f.central
    );
    assert!(!present(&old));
    assert_eq!(
        fs::read_dir(f.at(".agents/skills")).unwrap().count(),
        1,
        "the folder in the way is gone, nothing staged is left"
    );
    assert!(f.store.list_redeploy_journal().unwrap().is_empty());
}

/// An answer applies to the path its ask showed, and only there.
#[cfg(unix)]
#[test]
fn an_answer_naming_another_path_is_stale_and_touches_nothing() {
    let f = agents_fixture(&["codex"]);
    let old = f.link_row("codex", ".codex/skills/alpha");
    let foreign = foreign_at(&f, ".agents/skills/alpha");

    let report = f.run(&[RedeployAnswer::Occupied {
        skill_id: "alpha".into(),
        folder_key: "agents_skills".into(),
        row_ids: vec!["row-codex".into()],
        path: f.at("elsewhere/alpha").to_string_lossy().into_owned(),
        decision: OverrideDecision::Overwrite,
    }]);

    assert!(matches!(
        &only_move(&report).status,
        RedeployStatus::Failed {
            error: CommandError::NotFound { .. }
        }
    ));
    assert!(present(&old));
    assert!(foreign.join("SKILL.md").is_file());
}

// ---------------------------------------------------------------------------
// The edited-copy question
// ---------------------------------------------------------------------------

/// A copy edited in place is never removed silently: the move completes, the
/// old copy stays, journaled, and is asked about with its readers.
fn edited_copy(f: &Fixture) -> PathBuf {
    let old = f.copy_row("codex", ".codex/skills/alpha");
    fs::write(old.join("local.txt"), "only here").unwrap();
    old
}

#[test]
fn an_edited_old_copy_is_asked_about_and_kept_until_answered() {
    let f = agents_fixture(&["codex", "cursor"]);
    let old = edited_copy(&f);

    let report = f.run(&[]);

    assert!(matches!(
        only_move(&report).status,
        RedeployStatus::LeftOld { edited: true }
    ));
    assert_eq!(report.edited_copies.len(), 1);
    let question = &report.edited_copies[0];
    assert_eq!(Path::new(&question.path), old);
    assert_eq!(question.tool, "codex");
    // Codex still reads its deprecated folder; Cursor reads Codex's folder.
    assert_eq!(question.readers, vec!["cursor", "codex"]);
    assert!(old.join("local.txt").is_file(), "the edit survives");
    let journal = f.store.list_redeploy_journal().unwrap();
    assert_eq!(journal.len(), 1);
    assert_eq!(journal[0].state, "edited");
    assert_eq!(f.preview().journal, 1, "the line keeps it");
}

/// The answer to the one edited copy awaiting it: its journal entry and the
/// path the ask showed.
fn edited_answer(f: &Fixture, decision: EditedCopyDecision) -> RedeployAnswer {
    let journal = f.store.list_redeploy_journal().unwrap();
    assert_eq!(journal.len(), 1, "one copy awaits");
    RedeployAnswer::EditedCopy {
        entry_id: journal[0].id.clone(),
        path: f.at(".codex/skills/alpha").to_string_lossy().into_owned(),
        decision,
    }
}

#[test]
fn delete_removes_the_edited_copy() {
    let f = agents_fixture(&["codex"]);
    let old = edited_copy(&f);
    f.run(&[]);

    let report = f.run(&[edited_answer(&f, EditedCopyDecision::Delete)]);

    assert!(report.refused_answers.is_empty(), "{report:#?}");
    assert_eq!(report.journal.len(), 1);
    assert_eq!(report.journal[0].status, JournalStatus::Finished);
    assert!(!present(&old));
    assert!(f.store.list_redeploy_journal().unwrap().is_empty());
    assert_eq!(f.preview(), RedeployPreview::default());
}

#[test]
fn leave_disowns_the_edited_copy_and_names_its_readers() {
    let f = agents_fixture(&["codex"]);
    let old = edited_copy(&f);
    f.run(&[]);

    let report = f.run(&[edited_answer(&f, EditedCopyDecision::Leave)]);

    assert_eq!(report.journal[0].status, JournalStatus::LeftBehind);
    assert_eq!(report.left_behind.len(), 1);
    assert_eq!(report.left_behind[0].reason, RedeployLeftReason::Edited);
    assert_eq!(report.left_behind[0].readers, vec!["codex"]);
    assert!(old.join("local.txt").is_file());
    assert!(f.store.list_redeploy_journal().unwrap().is_empty());
    assert_eq!(f.preview(), RedeployPreview::default());
}

/// An answer reaches only an entry that was awaiting it: a copy edited
/// after an answer was given is asked again, never deleted on a stale yes.
#[test]
fn an_edited_copy_answer_naming_another_path_deletes_nothing() {
    let f = agents_fixture(&["codex"]);
    let old = edited_copy(&f);
    f.run(&[]);

    let report = f.run(&[RedeployAnswer::EditedCopy {
        entry_id: f.store.list_redeploy_journal().unwrap()[0].id.clone(),
        path: f.at(".codex/skills/other").to_string_lossy().into_owned(),
        decision: EditedCopyDecision::Delete,
    }]);

    assert_eq!(report.journal[0].status, JournalStatus::AwaitingAnswer);
    assert!(old.join("local.txt").is_file());
    assert_eq!(report.refused_answers.len(), 1, "{report:#?}");
    assert!(matches!(
        &report.refused_answers[0].error,
        CommandError::NotFound { kind, .. } if kind == "redeploy_question"
    ));
}

/// Astra round 1, finding 2 (answer identity): an Overwrite answered for one
/// row never reaches a row recreated since at the same skill, folder and
/// place — the question it answered is gone. Refused, nothing touched.
#[cfg(unix)]
#[test]
fn an_overwrite_for_a_row_recreated_since_is_refused() {
    let f = agents_fixture(&["codex"]);
    let old = f.link_row("codex", ".codex/skills/alpha");
    let foreign = foreign_at(&f, ".agents/skills/alpha");
    let asked = f.run(&[]);
    assert_eq!(only_move(&asked).rows[0].id, "row-codex");
    // The row is deleted and recreated (same skill, key and place).
    f.store.delete_skill_target("alpha", "codex").unwrap();
    let mut recreated = f.store.list_skill_targets("alpha").unwrap();
    assert!(recreated.is_empty());
    recreated.push(SkillTargetRecord {
        id: "replacement-row".into(),
        skill_id: "alpha".into(),
        tool: "codex".into(),
        target_path: old.to_string_lossy().into_owned(),
        mode: SyncMode::Symlink,
        status: SyncStatus::Synced,
        last_error: None,
        synced_at: Some(2),
        content_hash: None,
    });
    f.store.upsert_skill_target(&recreated[0]).unwrap();

    let report = f.run(&[occupied(&f, OverrideDecision::Overwrite)]);

    assert!(
        matches!(
            &only_move(&report).status,
            RedeployStatus::Failed {
                error: CommandError::NotFound { kind, .. }
            } if kind == "redeploy_question"
        ),
        "{report:#?}"
    );
    assert_eq!(
        fs::read_to_string(foreign.join("SKILL.md")).unwrap(),
        "someone else's",
        "the folder in the way is untouched"
    );
    assert!(present(&old));
    assert_eq!(f.row("codex").unwrap().id, "replacement-row");
    assert!(f.store.list_redeploy_journal().unwrap().is_empty());
}

/// Astra round 1, finding 2 (answer identity): a Delete answered for one
/// journal entry never deletes a later entry's copy at the same key and
/// path. The later copy is asked about again; the answer is refused.
#[test]
fn an_old_delete_answer_never_deletes_a_later_entrys_copy() {
    let f = agents_fixture(&["codex"]);
    let old = edited_copy(&f);
    f.run(&[]);
    let first = edited_answer(&f, EditedCopyDecision::Delete);
    // Leave settles that entry; then the harness's old folder holds another
    // edited copy, recorded again, and the next run journals it anew.
    f.run(&[edited_answer(&f, EditedCopyDecision::Leave)]);
    fs::write(old.join("local.txt"), "edited again").unwrap();
    f.store
        .upsert_skill_target(&SkillTargetRecord {
            id: "row-codex-again".into(),
            skill_id: "alpha".into(),
            tool: "codex".into(),
            target_path: old.to_string_lossy().into_owned(),
            mode: SyncMode::Copy,
            status: SyncStatus::Synced,
            last_error: None,
            synced_at: Some(2),
            content_hash: content_identity::read(Source::Directory(&f.central)),
        })
        .unwrap();
    let report = f.run(&[]);
    assert_eq!(report.edited_copies.len(), 1, "{report:#?}");
    let later = report.edited_copies[0].entry_id.clone();
    assert_eq!(Path::new(&report.edited_copies[0].path), old);

    let report = f.run(&[first]);

    assert_eq!(report.journal.len(), 1);
    assert_eq!(report.journal[0].entry_id, later);
    assert_eq!(report.journal[0].status, JournalStatus::AwaitingAnswer);
    assert_eq!(report.refused_answers.len(), 1, "{report:#?}");
    assert_eq!(
        fs::read_to_string(old.join("local.txt")).unwrap(),
        "edited again",
        "the later copy is untouched"
    );
}

// ---------------------------------------------------------------------------
// Disabled and never-deployed rows; the Q24 collapse
// ---------------------------------------------------------------------------

/// A Disabled row is re-recorded at the new place with nothing written, so
/// Enable deploys it there.
#[test]
fn a_disabled_row_is_re_recorded_with_nothing_written() {
    let f = agents_fixture(&["codex"]);
    let old = f.at(".codex/skills/alpha");
    f.seed(
        "codex",
        &old,
        SyncMode::Symlink,
        SyncStatus::Disabled,
        false,
    );

    let report = f.run(&[]);

    assert!(skipped!(&only_move(&report).status, RedeploySkip::Disabled));
    let row = f.row("agents_skills").expect("re-recorded");
    assert_eq!(row.status, SyncStatus::Disabled);
    assert_eq!(Path::new(&row.target_path), f.at(".agents/skills/alpha"));
    assert!(!row.owns_artifact());
    assert!(!present(&f.at(".agents/skills/alpha")), "nothing written");
    assert!(report.lost_disables.is_empty());
}

/// A row that never deployed (a failed first sync's `error` row) owns
/// nothing: re-recorded, its failure kept, nothing written.
#[test]
fn a_never_deployed_row_is_re_recorded_with_nothing_written() {
    let f = fixture(&["kilo_code"]);
    let old = f.at(".kilocode/skills/alpha");
    fs::create_dir_all(&old).unwrap();
    fs::write(old.join("SKILL.md"), "someone else's").unwrap();
    f.seed(
        "kilo_code",
        &old,
        SyncMode::Symlink,
        SyncStatus::Error,
        false,
    );

    let report = f.run(&[]);

    assert!(skipped!(
        &only_move(&report).status,
        RedeploySkip::NotDeployed
    ));
    let row = f.row("kilo_code").expect("re-recorded");
    assert_eq!(row.status, SyncStatus::Error);
    assert_eq!(Path::new(&row.target_path), f.at(".kilo/skills/alpha"));
    assert!(
        old.join("SKILL.md").is_file(),
        "a folder we never wrote stays"
    );
    assert!(!present(&f.at(".kilo/skills/alpha")));
}

/// Q24: rows collapsing into one folder — the folder keeps the skill when
/// any harness it serves had it live, and each disable that no longer holds
/// is reported, naming the harness.
#[cfg(unix)]
#[test]
fn rows_collapsing_into_one_folder_keep_the_skill_live_and_report_lost_disables() {
    let f = agents_fixture(&["codex", "goose"]);
    let codex = f.link_row("codex", ".codex/skills/alpha");
    f.seed(
        "goose",
        &f.at(".config/goose/skills/alpha"),
        SyncMode::Symlink,
        SyncStatus::Disabled,
        false,
    );

    let report = f.run(&[]);

    let moved = only_move(&report);
    assert_eq!(moved.folder_key, "agents_skills");
    assert_eq!(moved.rows.len(), 2);
    assert!(matches!(moved.status, RedeployStatus::Moved));
    assert_eq!(f.rows().len(), 1, "one folder, one row");
    let row = f.row("agents_skills").unwrap();
    assert_eq!(row.status, SyncStatus::Synced);
    assert_eq!(report.lost_disables.len(), 1);
    assert_eq!(report.lost_disables[0].tool, "goose");
    assert_eq!(report.lost_disables[0].folder_key, "agents_skills");
    assert!(!present(&codex));
}

/// Q24: when every harness the folder serves had it disabled, the folder
/// records it disabled — and no disable is lost.
#[test]
fn rows_all_disabled_collapse_into_a_disabled_folder_row() {
    let f = agents_fixture(&["codex", "goose"]);
    for (tool, rel) in [
        ("codex", ".codex/skills/alpha"),
        ("goose", ".config/goose/skills/alpha"),
    ] {
        f.seed(
            tool,
            &f.at(rel),
            SyncMode::Symlink,
            SyncStatus::Disabled,
            false,
        );
    }

    let report = f.run(&[]);

    assert!(skipped!(&only_move(&report).status, RedeploySkip::Disabled));
    assert_eq!(f.rows().len(), 1);
    assert_eq!(f.row("agents_skills").unwrap().status, SyncStatus::Disabled);
    assert!(report.lost_disables.is_empty());
}

/// A legacy row whose folder's own row already holds the skill there is
/// absorbed: the folder row stays, the legacy artifact is removed.
#[cfg(unix)]
#[test]
fn a_legacy_row_merges_into_the_folders_row_already_in_place() {
    let f = agents_fixture(&["codex", "agents_skills"]);
    let codex = f.link_row("codex", ".codex/skills/alpha");
    f.link_row("agents_skills", ".agents/skills/alpha");

    let report = f.run(&[]);

    assert!(matches!(only_move(&report).status, RedeployStatus::Adopted));
    assert_eq!(f.rows().len(), 1);
    assert_eq!(f.row("agents_skills").unwrap().id, "row-agents_skills");
    assert!(!present(&codex));
}

/// A skill whose Disable failed (rows kept `error`, artifact still there):
/// nothing is written or re-recorded for it — the artifact keeps its row.
#[cfg(unix)]
#[test]
fn a_disabled_skill_with_an_artifact_left_is_not_touched() {
    let f = agents_fixture(&["codex"]);
    let old = f.link_row("codex", ".codex/skills/alpha");
    let mut row = f.row("codex").unwrap();
    row.status = SyncStatus::Error;
    f.store.upsert_skill_target(&row).unwrap();
    f.store.set_skill_disabled_at("alpha", Some(1)).unwrap();

    let report = f.run(&[]);

    assert!(skipped!(&only_move(&report).status, RedeploySkip::Disabled));
    assert!(present(&old));
    assert!(!present(&f.at(".agents/skills/alpha")));
    assert_eq!(f.row("codex").unwrap().status, SyncStatus::Error);
}

// ---------------------------------------------------------------------------
// Kimi and Amp: an old artifact another live row records survives
// ---------------------------------------------------------------------------

/// Kimi Code left Amp's `~/.config/agents/skills` (Q12). Before 1.4.0 a skill
/// deployed to both had one artifact there and a row per harness: Kimi's
/// row moves to `~/.kimi-code/skills`, and the old artifact — Amp's live
/// Deployment — survives, as does Amp's row.
#[test]
fn amps_artifact_survives_kimis_move() {
    let f = fixture(&["amp", "kimi_cli"]);
    let shared = f.copy_row("amp", ".config/agents/skills/alpha");
    f.seed(
        "kimi_cli",
        &shared,
        SyncMode::Copy,
        SyncStatus::Synced,
        true,
    );

    let report = f.run(&[]);

    let moved = only_move(&report);
    assert_eq!(moved.folder_key, "kimi_cli");
    assert!(matches!(moved.status, RedeployStatus::Moved), "{moved:?}");
    assert!(shared.join("SKILL.md").is_file(), "Amp's artifact survives");
    let amp = f.row("amp").expect("Amp's row");
    assert_eq!(Path::new(&amp.target_path), shared);
    assert_eq!(amp.status, SyncStatus::Synced);
    let kimi = f.row("kimi_cli").expect("Kimi's row");
    assert_eq!(
        Path::new(&kimi.target_path),
        f.at(".kimi-code/skills/alpha")
    );
    assert!(present(&f.at(".kimi-code/skills/alpha")));
}

// ---------------------------------------------------------------------------
// Migration: legacy rows converted at the path they already have
// ---------------------------------------------------------------------------

/// Two rows written once per harness on one shared folder (the pre-1.4.0
/// shape) collapse into the folder's one row with nothing written: a
/// migration, not a correction.
#[cfg(unix)]
#[test]
fn per_harness_rows_on_one_folder_collapse_with_nothing_written() {
    crate::core::tool_adapters::test_overrides::kimi_on_amps_folder();
    let f = fixture(&["amp", "kimi_cli"]);
    let shared = f.link_row("amp", ".config/agents/skills/alpha");
    f.seed(
        "kimi_cli",
        &shared,
        SyncMode::Symlink,
        SyncStatus::Synced,
        true,
    );
    let preview = f.preview();
    assert_eq!(preview.global.migration, 1);
    assert_eq!(preview.global.correction, 0);

    let report = f.run(&[]);

    let moved = only_move(&report);
    assert_eq!(moved.cause, RedeployCause::Migration);
    assert!(matches!(moved.status, RedeployStatus::Unchanged));
    assert_eq!(f.rows().len(), 1);
    assert_eq!(
        f.row("amp").unwrap().synced_at,
        Some(1),
        "nothing rewritten"
    );
    assert_eq!(fs::read_link(&shared).unwrap(), f.central);
}

// ---------------------------------------------------------------------------
// The journal: a crash seeded at each step, then finished
// ---------------------------------------------------------------------------

fn crash(f: &Fixture, arm: fn()) {
    arm();
    let crashed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        redeploy_unlocked(
            &f.store,
            &f.roots,
            &f.central_dir(),
            &PlacementSettings::default(),
            &[],
            NOW,
            |_| {},
        )
    }))
    .is_err();
    assert!(crashed, "interrupted");
}

/// Crash after the deploy, before the switch: the new link exists, nothing
/// is recorded. The next run adopts it (ours) and finishes the move.
#[test]
fn a_crash_before_the_switch_is_finished_by_the_next_run() {
    let f = agents_fixture(&["codex"]);
    let old = f.copy_row("codex", ".codex/skills/alpha");
    crash(&f, test_seams::crash_before_switch);
    assert!(present(&old) && present(&f.at(".agents/skills/alpha")));
    assert!(f.row("codex").is_some(), "nothing switched");

    let report = f.run(&[]);

    assert!(matches!(only_move(&report).status, RedeployStatus::Adopted));
    assert!(!present(&old));
    assert!(f.row("agents_skills").is_some());
}

/// Crash after the switch: the journal owes the removal. The launch check's
/// journal finish removes it and records a report taken once.
#[test]
fn a_crash_after_the_switch_is_finished_by_the_launch_check() {
    let f = agents_fixture(&["codex"]);
    let old = f.copy_row("codex", ".codex/skills/alpha");
    crash(&f, test_seams::crash_after_switch);

    let report = finish_journal(&f.store, &f.roots, &f.central_dir(), NOW)
        .unwrap()
        .expect("something settled");

    assert_eq!(report.journal.len(), 1);
    assert_eq!(report.journal[0].status, JournalStatus::Finished);
    assert!(!present(&old));
    assert!(f.store.list_redeploy_journal().unwrap().is_empty());
    assert!(take_finished_journal(&f.store).unwrap().is_some());
    assert!(
        take_finished_journal(&f.store).unwrap().is_none(),
        "taken once"
    );
    assert_eq!(f.preview(), RedeployPreview::default());
    assert!(finish_journal(&f.store, &f.roots, &f.central_dir(), NOW)
        .unwrap()
        .is_none());
}

/// Crash after the removal, before the entry was dropped: the old artifact
/// is gone, so the entry finishes with nothing to do.
#[test]
fn a_crash_after_the_removal_finishes_with_nothing_to_remove() {
    let f = agents_fixture(&["codex"]);
    let old = f.copy_row("codex", ".codex/skills/alpha");
    crash(&f, test_seams::crash_after_switch);
    fs::remove_dir_all(&old).unwrap();

    let report = f.run(&[]);

    assert_eq!(report.journal[0].status, JournalStatus::Finished);
    assert!(report.moves.is_empty());
}

/// The journal authorises nothing by its paths alone: with the new place
/// serving, an old path taken by another folder is reported left behind and
/// nothing is touched.
#[cfg(unix)]
#[test]
fn a_journal_entry_that_no_longer_matches_is_reported_and_touches_nothing() {
    let f = agents_fixture(&["codex"]);
    let old = f.link_row("codex", ".codex/skills/alpha");
    crash(&f, test_seams::crash_after_switch);
    // Someone replaced the old link with a folder of their own.
    fs::remove_file(&old).unwrap();
    fs::create_dir_all(&old).unwrap();
    fs::write(old.join("SKILL.md"), "theirs").unwrap();

    let report = f.run(&[]);

    assert_eq!(report.journal[0].status, JournalStatus::LeftBehind);
    assert_eq!(report.left_behind[0].reason, RedeployLeftReason::Replaced);
    assert_eq!(fs::read_to_string(old.join("SKILL.md")).unwrap(), "theirs");
    assert!(f.store.list_redeploy_journal().unwrap().is_empty());
}

/// The journal is dropped only on positive evidence (round 4 disposition):
/// with the new artifact gone, the old one may be the only copy, so it is
/// kept and so is its entry — `unfinished`, counted, retried. Once the new
/// place serves again (a later sync writes it), the entry finishes.
#[cfg(unix)]
#[test]
fn a_journal_entry_whose_new_artifact_is_gone_stays_unfinished() {
    let f = agents_fixture(&["codex"]);
    let old = f.link_row("codex", ".codex/skills/alpha");
    crash(&f, test_seams::crash_after_switch);
    let new = f.at(".agents/skills/alpha");
    fs::remove_file(&new).unwrap();

    let report = finish_journal(&f.store, &f.roots, &f.central_dir(), NOW)
        .unwrap()
        .unwrap();

    assert_eq!(report.journal[0].status, JournalStatus::Unfinished);
    assert!(report.left_behind.is_empty(), "{report:#?}");
    assert!(present(&old), "it may be the only copy");
    assert_eq!(f.store.list_redeploy_journal().unwrap().len(), 1, "kept");
    assert_eq!(f.preview().journal, 1, "counted");

    link(&f.central, &new);
    let report = f.run(&[]);

    assert_eq!(report.journal[0].status, JournalStatus::Finished);
    assert!(!present(&old));
    assert!(f.store.list_redeploy_journal().unwrap().is_empty());
}

/// An unreachable new place (its folder moved away) authorises nothing: the
/// entry is kept, never dropped as left behind, and finishes once the place
/// is reachable again (every OS: copies).
#[test]
fn a_journal_entry_whose_new_place_is_unreachable_is_kept_until_it_serves() {
    let f = fixture(&["mux"]);
    let old = f.copy_row("mux", ".mux/skills/alpha");
    crash(&f, test_seams::crash_after_switch);
    let new = f.at(".xum/skills/alpha");
    assert!(serves_at(&new));
    fs::rename(f.at(".xum/skills"), f.at(".xum/away")).unwrap();

    let report = finish_journal(&f.store, &f.roots, &f.central_dir(), NOW)
        .unwrap()
        .expect("reported");

    assert_eq!(report.journal[0].status, JournalStatus::Unfinished);
    assert!(report.left_behind.is_empty(), "{report:#?}");
    assert!(serves_at(&old), "the old copy is kept");
    assert_eq!(f.preview().journal, 1, "pending");

    fs::rename(f.at(".xum/away"), f.at(".xum/skills")).unwrap();
    let report = finish_journal(&f.store, &f.roots, &f.central_dir(), NOW)
        .unwrap()
        .expect("settled");

    assert_eq!(report.journal[0].status, JournalStatus::Finished);
    assert!(!present(&old));
    assert!(f.store.list_redeploy_journal().unwrap().is_empty());
}

/// The other positive evidence: the skill record is gone. Its entry goes;
/// the old artifact still on disk is reported left behind, untouched.
#[test]
fn a_journal_entry_whose_skill_is_gone_is_dropped_touching_nothing() {
    let f = fixture(&["mux"]);
    let old = f.copy_row("mux", ".mux/skills/alpha");
    crash(&f, test_seams::crash_after_switch);
    f.store.delete_skill("alpha").unwrap();

    let report = finish_journal(&f.store, &f.roots, &f.central_dir(), NOW)
        .unwrap()
        .expect("settled");

    assert_eq!(report.journal[0].status, JournalStatus::LeftBehind);
    assert_eq!(report.left_behind[0].reason, RedeployLeftReason::Stale);
    assert!(serves_at(&old), "untouched");
    assert!(f.store.list_redeploy_journal().unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// Audit 3 F14–F16
// ---------------------------------------------------------------------------

/// F14 (aliased roots): `~/.agents/skills` is an alias of `~/.codex/skills`,
/// so the old link and the new place are one entry. The move is a re-record
/// only: nothing is written, and the one artifact is never removed.
#[cfg(unix)]
#[test]
fn f14_aliased_old_and_new_roots_are_one_entry_and_nothing_is_removed() {
    let f = agents_fixture(&["codex"]);
    let old = f.link_row("codex", ".codex/skills/alpha");
    link(&f.at(".codex/skills"), &f.at(".agents/skills"));

    let report = f.run(&[]);

    assert!(matches!(
        only_move(&report).status,
        RedeployStatus::Unchanged
    ));
    assert!(present(&old), "the one artifact survives");
    assert_eq!(fs::read_link(&old).unwrap(), f.central);
    let row = f.row("agents_skills").unwrap();
    assert_eq!(Path::new(&row.target_path), f.at(".agents/skills/alpha"));
    assert_eq!(row.status, SyncStatus::Synced);
    assert!(f.store.list_redeploy_journal().unwrap().is_empty());
    assert!(f.central.join("a.txt").is_file());
}

/// F14 (truthful status): the old artifact is gone and a foreign folder sits
/// at the new place. The move cannot complete; the row never reads `synced`
/// over a missing folder, and the foreign bytes are untouched.
#[cfg(unix)]
#[test]
fn f14_a_missing_old_artifact_with_a_foreign_destination_is_not_reported_synced() {
    let f = agents_fixture(&["codex"]);
    let old = f.link_row("codex", ".codex/skills/alpha");
    fs::remove_file(&old).unwrap();
    let foreign = foreign_at(&f, ".agents/skills/alpha");

    let report = f.run(&[]);

    assert!(matches!(
        &only_move(&report).status,
        RedeployStatus::Failed {
            error: CommandError::TargetExists { .. }
        }
    ));
    let row = f.row("codex").unwrap();
    assert_eq!(row.status, SyncStatus::Error);
    assert!(row.owns_artifact(), "the claim is kept (ADR-0002)");
    assert_eq!(
        fs::read_to_string(foreign.join("SKILL.md")).unwrap(),
        "someone else's"
    );
}

/// F14 (nested alias; Astra round 1, finding 1): `~/.kilo/skills` is an alias
/// of the old copy `~/.kilocode/skills/alpha` itself, so the new place would
/// lie inside the old Deployment — and removing the old copy would take the
/// new one with it. The move is refused before anything is written, never
/// reported moved; the old copy, its bytes and its row survive.
#[cfg(unix)]
#[test]
fn f14_a_new_folder_aliased_into_the_old_copy_is_refused() {
    let f = fixture(&["kilo_code"]);
    let old = f.copy_row("kilo_code", ".kilocode/skills/alpha");
    fs::create_dir_all(f.at(".kilo")).unwrap();
    link(&old, &f.at(".kilo/skills"));

    let report = f.run(&[]);

    assert!(
        matches!(
            &only_move(&report).status,
            RedeployStatus::Failed {
                error: CommandError::DestinationRefused {
                    reason: DestinationRefusal::OverlapsDeployment,
                    resolved,
                    ..
                }
            } if Path::new(resolved) == old
        ),
        "{report:#?}"
    );
    assert_eq!(fs::read_to_string(old.join("a.txt")).unwrap(), "v1");
    assert!(!present(&old.join("alpha")), "nothing written inside it");
    let row = f.row("kilo_code").unwrap();
    assert_eq!(Path::new(&row.target_path), old);
    assert_eq!(row.status, SyncStatus::Synced);
    assert!(f.store.list_redeploy_journal().unwrap().is_empty());
    assert_eq!(f.preview().global.correction, 1, "still pending");
}

/// Lays the Kilo nesting out after a crash left its journal entry: the new
/// Deployment (moved, keeping its identity) now lies inside the old copy,
/// reached through `~/.kilo/skills` aliased onto it.
#[cfg(unix)]
fn nest_the_new_kilo_deployment_in_the_old_copy(f: &Fixture, old: &Path) -> PathBuf {
    let new = f.at(".kilo/skills/alpha");
    fs::rename(&new, old.join("alpha")).unwrap();
    fs::remove_dir(f.at(".kilo/skills")).unwrap();
    link(old, &f.at(".kilo/skills"));
    assert_eq!(
        fs::read_link(&new).unwrap(),
        f.central,
        "reached through it"
    );
    new
}

/// F14 (nested alias) at cleanup: the journal finish never removes an old
/// copy the journaled new Deployment lies inside. Both survive; the old copy
/// is reported left behind (`holds_deployment`), never finished.
#[cfg(unix)]
#[test]
fn f14_the_journal_finish_keeps_an_old_copy_holding_the_new_deployment() {
    let f = fixture(&["kilo_code"]);
    let old = f.copy_row("kilo_code", ".kilocode/skills/alpha");
    crash(&f, test_seams::crash_after_switch);
    let new = nest_the_new_kilo_deployment_in_the_old_copy(&f, &old);

    let report = finish_journal(&f.store, &f.roots, &f.central_dir(), NOW)
        .unwrap()
        .expect("settled");

    assert_eq!(report.journal[0].status, JournalStatus::LeftBehind);
    assert_eq!(
        report.left_behind[0].reason,
        RedeployLeftReason::HoldsDeployment
    );
    assert!(present(&new), "the new Deployment survives");
    assert_eq!(fs::read_to_string(old.join("a.txt")).unwrap(), "v1");
    assert_eq!(
        Path::new(&f.row("kilo_code").unwrap().target_path),
        new,
        "the row still records it"
    );
}

/// F14 (nested alias) for an answered Delete: an edited old copy the new
/// Deployment came to lie inside is kept even though the operator answered
/// Delete for it — the answer never reaches another Deployment.
#[cfg(unix)]
#[test]
fn f14_an_answered_delete_keeps_an_old_copy_holding_the_new_deployment() {
    let f = fixture(&["kilo_code"]);
    let old = f.copy_row("kilo_code", ".kilocode/skills/alpha");
    fs::write(old.join("local.txt"), "only here").unwrap();
    let report = f.run(&[]);
    assert_eq!(report.edited_copies.len(), 1, "{report:#?}");
    let asked = &report.edited_copies[0];
    let answer = RedeployAnswer::EditedCopy {
        entry_id: asked.entry_id.clone(),
        path: asked.path.clone(),
        decision: EditedCopyDecision::Delete,
    };
    let new = nest_the_new_kilo_deployment_in_the_old_copy(&f, &old);

    let report = f.run(&[answer]);

    assert_eq!(report.journal[0].status, JournalStatus::LeftBehind);
    assert_eq!(
        report.left_behind[0].reason,
        RedeployLeftReason::HoldsDeployment
    );
    assert!(present(&new), "the new Deployment survives");
    assert_eq!(
        fs::read_to_string(old.join("local.txt")).unwrap(),
        "only here"
    );
}

/// F14 (nested alias, the other way): the old copy lies inside the folder at
/// the new place (`~/.kilocode/skills` aliased into it). Even an Overwrite
/// answer for that folder is refused before anything is touched — replacing
/// it would remove the old Deployment.
#[cfg(unix)]
#[test]
fn f14_a_destination_holding_the_old_copy_is_refused_even_on_overwrite() {
    let f = fixture(&["kilo_code"]);
    let holder = f.at(".kilo/skills/alpha");
    fs::create_dir_all(&holder).unwrap();
    fs::remove_dir_all(f.at(".kilocode/skills")).ok();
    link(&holder, &f.at(".kilocode/skills"));
    let old = f.copy_row("kilo_code", ".kilocode/skills/alpha");
    assert!(holder.join("alpha/a.txt").is_file());

    let report = f.run(&[RedeployAnswer::Occupied {
        skill_id: "alpha".into(),
        folder_key: "kilo_code".into(),
        row_ids: vec!["row-kilo_code".into()],
        path: holder.to_string_lossy().into_owned(),
        decision: OverrideDecision::Overwrite,
    }]);

    assert!(
        matches!(
            &only_move(&report).status,
            RedeployStatus::Failed {
                error: CommandError::DestinationRefused {
                    reason: DestinationRefusal::OverlapsDeployment,
                    ..
                }
            }
        ),
        "{report:#?}"
    );
    assert_eq!(fs::read_to_string(old.join("a.txt")).unwrap(), "v1");
    assert_eq!(Path::new(&f.row("kilo_code").unwrap().target_path), old);
}

// ---------------------------------------------------------------------------
// Non-destructive adoption and the staged replacement (Astra round 1,
// finding 3)
// ---------------------------------------------------------------------------

/// The physical identity of the entry at `path`.
#[cfg(unix)]
fn inode(path: &Path) -> u64 {
    use std::os::unix::fs::MetadataExt;
    fs::symlink_metadata(path).unwrap().ino()
}

/// Astra's reproduction: the old recorded copy is gone and an identical copy
/// sits at the new place. It is adopted as it is — never removed and
/// re-created — so an interruption right after the destination step leaves
/// it in place, and the next run records it.
#[cfg(unix)]
#[test]
fn an_identical_copy_at_the_new_place_is_adopted_where_it_is() {
    let f = agents_fixture(&["codex"]);
    let old = f.copy_row("codex", ".codex/skills/alpha");
    fs::remove_dir_all(&old).unwrap();
    let new = f.at(".agents/skills/alpha");
    copy_of(&f.central, &new);
    let identity = inode(&new);

    crash(&f, test_seams::crash_before_switch);
    assert_eq!(inode(&new), identity, "untouched by the interrupted run");
    assert!(new.join("a.txt").is_file());

    let report = f.run(&[]);

    assert!(
        matches!(only_move(&report).status, RedeployStatus::Adopted),
        "{report:#?}"
    );
    assert_eq!(inode(&new), identity, "recorded as it is");
    let row = f.row("agents_skills").unwrap();
    assert_eq!(Path::new(&row.target_path), new);
    assert_eq!(row.mode, SyncMode::Copy);
    assert_eq!(
        row.content_hash,
        content_identity::read(Source::Directory(&f.central))
    );
}

/// Deep Agents Code takes copies only: a legacy link row at its place is
/// ours in the wrong mode, so the move is a genuine replacement.
#[cfg(unix)]
fn deep_agents_link(f: &Fixture) -> PathBuf {
    f.link_row("deep_agents_code", ".deepagents/agent/skills/alpha")
}

/// What the Deep Agents folder holds besides the skill (stage leftovers).
#[cfg(unix)]
fn deep_agents_extras(f: &Fixture) -> Vec<String> {
    fs::read_dir(f.at(".deepagents/agent/skills"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name != "alpha")
        .collect()
}

#[cfg(unix)]
fn assert_deep_agents_copy(f: &Fixture, dest: &Path) {
    let meta = fs::symlink_metadata(dest).unwrap();
    assert!(meta.is_dir() && !meta.file_type().is_symlink(), "a copy");
    assert_eq!(fs::read_to_string(dest.join("a.txt")).unwrap(), "v1");
    assert!(
        deep_agents_extras(f).is_empty(),
        "{:?}",
        deep_agents_extras(f)
    );
}

/// The replacement completes: the link becomes a copy, nothing staged stays.
#[cfg(unix)]
#[test]
fn a_link_where_copies_are_taken_is_replaced_by_a_copy() {
    let f = fixture(&["deep_agents_code"]);
    let dest = deep_agents_link(&f);

    let report = f.run(&[]);

    assert!(matches!(only_move(&report).status, RedeployStatus::Moved));
    assert_deep_agents_copy(&f, &dest);
    let row = f.row("deep_agents_code").unwrap();
    assert_eq!(row.mode, SyncMode::Copy);
    assert!(row.content_hash.is_some());
    assert!(f.store.list_redeploy_journal().unwrap().is_empty());
    assert_eq!(f.preview(), RedeployPreview::default());
}

/// Interrupted before the swap: the link still serves; the launch check
/// drops what was staged, and the next run completes the move.
#[cfg(unix)]
#[test]
fn a_replacement_interrupted_before_the_swap_keeps_the_link_and_completes() {
    let f = fixture(&["deep_agents_code"]);
    let dest = deep_agents_link(&f);
    crash(&f, test_seams::crash_before_swap);
    assert_eq!(fs::read_link(&dest).unwrap(), f.central, "still deployed");
    assert_eq!(f.preview().journal, 1);

    let finished = finish_journal(&f.store, &f.roots, &f.central_dir(), NOW)
        .unwrap()
        .expect("settled");
    assert_eq!(finished.journal[0].status, JournalStatus::Finished);
    assert_eq!(fs::read_link(&dest).unwrap(), f.central);
    assert!(deep_agents_extras(&f).is_empty());

    assert!(matches!(
        only_move(&f.run(&[])).status,
        RedeployStatus::Moved
    ));
    assert_deep_agents_copy(&f, &dest);
}

/// Interrupted right after the atomic exchange: the new copy is in place,
/// the old link at the staged name beside it. The launch check removes the
/// link; the next run records the copy as it is.
#[cfg(unix)]
#[test]
fn a_replacement_interrupted_after_the_swap_keeps_the_copy_and_completes() {
    let f = fixture(&["deep_agents_code"]);
    let dest = deep_agents_link(&f);
    crash(&f, test_seams::crash_after_swap);
    assert!(dest.join("a.txt").is_file(), "deployed in the new place");
    assert!(!fs::symlink_metadata(&dest)
        .unwrap()
        .file_type()
        .is_symlink());

    finish_journal(&f.store, &f.roots, &f.central_dir(), NOW)
        .unwrap()
        .expect("settled");
    assert_deep_agents_copy(&f, &dest);

    assert!(matches!(
        only_move(&f.run(&[])).status,
        RedeployStatus::Adopted
    ));
    assert_eq!(f.row("deep_agents_code").unwrap().mode, SyncMode::Copy);
    assert_eq!(f.preview(), RedeployPreview::default());
}

/// Whether `status` is the refusal to replace the folder at `dest` without an
/// atomic exchange.
fn refused_unsafe(status: &RedeployStatus, dest: &Path) -> bool {
    matches!(
        status,
        RedeployStatus::Failed {
            error: CommandError::DestinationRefused {
                path,
                reason: DestinationRefusal::CannotReplaceSafely,
                ..
            },
        } if Path::new(path) == dest
    )
}

/// Without an atomic exchange (Windows always; SMB, exFAT, …), the link
/// where copies are taken is never replaced: the occupant is not touched,
/// nothing staged stays, and the move is refused and still pending — the
/// swap's interruption point is never reached.
#[cfg(unix)]
#[test]
fn without_an_exchange_the_only_deployment_is_never_replaced() {
    let f = fixture(&["deep_agents_code"]);
    let dest = deep_agents_link(&f);
    let before = EntryId::of(&dest).unwrap();
    test_seams::force_no_exchange();
    test_seams::crash_after_swap();

    let report = f.run(&[]);

    assert!(
        refused_unsafe(&only_move(&report).status, &dest),
        "{report:#?}"
    );
    assert_eq!(fs::read_link(&dest).unwrap(), f.central, "still deployed");
    assert_eq!(EntryId::of(&dest).unwrap(), before, "the same entry");
    assert!(deep_agents_extras(&f).is_empty(), "nothing staged stays");
    assert!(f.store.list_redeploy_journal().unwrap().is_empty());
    let row = f.row("deep_agents_code").unwrap();
    assert_eq!(row.status, SyncStatus::Synced);
    assert_eq!(row.mode, SyncMode::Symlink);
    assert_eq!(f.preview().global.correction, 1, "still pending");
}

/// An Overwrite over a folder in the way is staged the same way: interrupted
/// after the swap, the launch check removes that folder under the answer.
#[cfg(unix)]
#[test]
fn an_overwrite_interrupted_after_the_swap_is_finished_under_the_answer() {
    let f = agents_fixture(&["codex"]);
    let old = f.link_row("codex", ".codex/skills/alpha");
    foreign_at(&f, ".agents/skills/alpha");
    test_seams::crash_after_swap();
    let crashed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        redeploy_unlocked(
            &f.store,
            &f.roots,
            &f.central_dir(),
            &PlacementSettings::default(),
            &[occupied(&f, OverrideDecision::Overwrite)],
            NOW,
            |_| {},
        )
    }))
    .is_err();
    assert!(crashed);
    assert_eq!(
        fs::read_link(f.at(".agents/skills/alpha")).unwrap(),
        f.central
    );
    assert!(present(&old), "the old link still serves too");

    finish_journal(&f.store, &f.roots, &f.central_dir(), NOW)
        .unwrap()
        .expect("settled");

    assert_eq!(
        fs::read_dir(f.at(".agents/skills")).unwrap().count(),
        1,
        "the folder answered Overwrite for is gone"
    );
    let report = f.run(&[]);
    assert!(matches!(only_move(&report).status, RedeployStatus::Adopted));
    assert!(!present(&old));
}

/// Xum's copy moving onto a folder in the way, answered Overwrite, with the
/// staged replacement interrupted where `arm` says.
fn xum_overwrite_interrupted(arm: fn()) -> (Fixture, PathBuf, PathBuf, RedeployAnswer) {
    let f = fixture(&["mux"]);
    let old = f.copy_row("mux", ".mux/skills/alpha");
    let dest = foreign_at(&f, ".xum/skills/alpha");
    let answer = xum_overwrite(&dest);
    arm();
    let crashed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        redeploy_unlocked(
            &f.store,
            &f.roots,
            &f.central_dir(),
            &PlacementSettings::default(),
            std::slice::from_ref(&answer),
            NOW,
            |_| {},
        )
    }))
    .is_err();
    assert!(crashed, "interrupted");
    assert!(old.join("a.txt").is_file(), "the old copy still serves");
    (f, old, dest, answer)
}

fn xum_entries(f: &Fixture) -> usize {
    fs::read_dir(f.at(".xum/skills")).unwrap().count()
}

/// Interrupted before the swap: the folder in the way is still there; the
/// launch check drops what was staged, and the answered run completes the
/// move.
#[cfg(unix)]
#[test]
fn an_overwrite_interrupted_before_the_swap_is_undone_at_launch() {
    let (f, old, dest, answer) = xum_overwrite_interrupted(test_seams::crash_before_swap);
    assert_eq!(
        fs::read_to_string(dest.join("SKILL.md")).unwrap(),
        "someone else's"
    );

    finish_journal(&f.store, &f.roots, &f.central_dir(), NOW)
        .unwrap()
        .expect("settled");

    assert_eq!(xum_entries(&f), 1, "nothing staged is left");
    assert_eq!(
        fs::read_to_string(dest.join("SKILL.md")).unwrap(),
        "someone else's"
    );
    let report = f.run(&[answer]);
    assert!(matches!(only_move(&report).status, RedeployStatus::Moved));
    assert_eq!(fs::read_to_string(dest.join("a.txt")).unwrap(), "v1");
    assert_eq!(xum_entries(&f), 1);
    assert!(!present(&old));
}

/// Interrupted right after the atomic exchange: the skill is in its new
/// place; the launch check removes the folder answered Overwrite for, and
/// the next run records the skill as it is.
#[cfg(unix)]
#[test]
fn an_overwrite_interrupted_after_the_swap_is_finished_at_launch() {
    let (f, old, dest, _) = xum_overwrite_interrupted(test_seams::crash_after_swap);
    assert_eq!(fs::read_to_string(dest.join("a.txt")).unwrap(), "v1");

    finish_journal(&f.store, &f.roots, &f.central_dir(), NOW)
        .unwrap()
        .expect("settled");

    assert_eq!(
        xum_entries(&f),
        1,
        "the folder answered Overwrite for is gone"
    );
    let report = f.run(&[]);
    assert!(
        matches!(only_move(&report).status, RedeployStatus::Adopted),
        "{report:#?}"
    );
    assert!(!present(&old));
    assert!(f.store.list_redeploy_journal().unwrap().is_empty());
}

/// What an Overwrite refused for want of an exchange leaves: the old copy
/// and the folder in the way as they were, nothing staged, nothing
/// journaled, the row not switched, the move still pending.
fn assert_overwrite_refused(f: &Fixture, report: &RedeployReport, old: &Path, dest: &Path) {
    assert!(
        refused_unsafe(&only_move(report).status, dest),
        "{report:#?}"
    );
    assert_eq!(fs::read_to_string(old.join("a.txt")).unwrap(), "v1");
    assert_eq!(
        fs::read_to_string(dest.join("SKILL.md")).unwrap(),
        "someone else's"
    );
    assert_eq!(xum_entries(f), 1, "nothing staged stays");
    assert!(f.store.list_redeploy_journal().unwrap().is_empty());
    let row = f.row("mux").unwrap();
    assert_eq!(row.status, SyncStatus::Synced, "the old copy still serves");
    assert_eq!(Path::new(&row.target_path), old, "not switched");
    assert_eq!(f.preview().global.correction, 1, "still pending");
}

/// Without an atomic exchange an Overwrite is refused before the folder in
/// the way is touched — even while the old copy serves: redeploy has no
/// two-rename fallback (round 20 t10 review, round 4 disposition). Forced
/// here on every OS, so the Windows behaviour runs everywhere.
#[test]
fn an_overwrite_without_an_exchange_is_refused_and_touches_nothing() {
    let f = fixture(&["mux"]);
    let old = f.copy_row("mux", ".mux/skills/alpha");
    let dest = foreign_at(&f, ".xum/skills/alpha");
    let (old_id, dest_id) = (EntryId::of(&old).unwrap(), EntryId::of(&dest).unwrap());
    test_seams::force_no_exchange();
    test_seams::crash_after_swap();

    let report = f.run(&[xum_overwrite(&dest)]);

    assert_overwrite_refused(&f, &report, &old, &dest);
    assert_eq!(EntryId::of(&old).unwrap(), old_id);
    assert_eq!(EntryId::of(&dest).unwrap(), dest_id);
}

/// Windows has no atomic exchange at all: with no test hook, an Overwrite is
/// refused there the same way.
#[cfg(windows)]
#[test]
fn windows_refuses_an_overwrite_for_want_of_an_exchange() {
    let f = fixture(&["mux"]);
    let old = f.copy_row("mux", ".mux/skills/alpha");
    let dest = foreign_at(&f, ".xum/skills/alpha");

    let report = f.run(&[xum_overwrite(&dest)]);

    assert_overwrite_refused(&f, &report, &old, &dest);
}

fn xum_overwrite(dest: &Path) -> RedeployAnswer {
    RedeployAnswer::Occupied {
        skill_id: "alpha".into(),
        folder_key: "mux".into(),
        row_ids: vec!["row-mux".into()],
        path: dest.to_string_lossy().into_owned(),
        decision: OverrideDecision::Overwrite,
    }
}

/// A usable copy of the skill that is not the Library's (an older edit).
fn edited_skill_at(f: &Fixture, rel: &str) -> PathBuf {
    let path = f.at(rel);
    copy_of(&f.central, &path);
    fs::write(path.join("a.txt"), "v0 edited").unwrap();
    path
}

/// Round 20 t10 review, finding 7 (Astra's probe): the old place is gone and
/// the new place holds a different, usable copy — the only working one.
/// Without an exchange the occupant is not touched: refused, pending,
/// nothing staged stays, and the swap's interruption point is never reached.
#[test]
fn an_overwrite_with_no_old_copy_and_no_exchange_never_touches_the_only_copy() {
    let f = fixture(&["mux"]);
    let old = f.copy_row("mux", ".mux/skills/alpha");
    fs::remove_dir_all(&old).unwrap();
    let dest = edited_skill_at(&f, ".xum/skills/alpha");
    let before = EntryId::of(&dest).unwrap();
    test_seams::force_no_exchange();
    test_seams::crash_after_swap();

    let report = f.run(&[xum_overwrite(&dest)]);

    assert!(
        refused_unsafe(&only_move(&report).status, &dest),
        "{report:#?}"
    );
    assert_eq!(EntryId::of(&dest).unwrap(), before, "the same folder");
    assert_eq!(fs::read_to_string(dest.join("a.txt")).unwrap(), "v0 edited");
    assert_eq!(xum_entries(&f), 1, "nothing staged stays");
    assert!(f.store.list_redeploy_journal().unwrap().is_empty());
    let row = f.row("mux").unwrap();
    assert_eq!(row.status, SyncStatus::Error, "not synced anywhere (F14)");
    assert_eq!(Path::new(&row.target_path), old, "not switched");
    assert_eq!(f.preview().global.correction, 1, "still pending");
}

/// Finding 7, the one interruption point a refusal leaves (after staging,
/// before the exchange is tried): the only copy stays in place. The old
/// place is gone, so aborting would leave nothing serving the skill: the
/// launch check keeps what was staged and its entry unfinished (finding 14).
#[test]
fn an_overwrite_with_no_old_copy_interrupted_before_the_swap_keeps_the_only_copy() {
    let f = fixture(&["mux"]);
    let old = f.copy_row("mux", ".mux/skills/alpha");
    fs::remove_dir_all(&old).unwrap();
    let dest = edited_skill_at(&f, ".xum/skills/alpha");
    let before = EntryId::of(&dest).unwrap();
    test_seams::force_no_exchange();
    test_seams::crash_before_swap();
    let crashed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        redeploy_unlocked(
            &f.store,
            &f.roots,
            &f.central_dir(),
            &PlacementSettings::default(),
            &[xum_overwrite(&dest)],
            NOW,
            |_| {},
        )
    }))
    .is_err();
    assert!(crashed, "interrupted");
    assert_eq!(EntryId::of(&dest).unwrap(), before);

    finish_journal(&f.store, &f.roots, &f.central_dir(), NOW)
        .unwrap()
        .expect("settled");

    assert_eq!(EntryId::of(&dest).unwrap(), before, "the same folder");
    assert_eq!(fs::read_to_string(dest.join("a.txt")).unwrap(), "v0 edited");
    assert_staged_kept_unfinished(&f);
}

/// What the launch check keeps of a replacement cut before the swap when
/// the old place no longer serves the skill: the staged copy beside the
/// occupant, and its `replacing` entry, unfinished and counted.
fn assert_staged_kept_unfinished(f: &Fixture) -> PathBuf {
    let journal = f.store.list_redeploy_journal().unwrap();
    assert_eq!(journal.len(), 1, "the entry stays: {journal:#?}");
    let staged = PathBuf::from(&journal[0].new_path);
    assert!(staged.starts_with(f.at(".xum/skills")), "{staged:?}");
    assert!(serves_at(&staged), "the staged copy is kept");
    assert_eq!(xum_entries(f), 2, "the occupant and the staged copy");
    assert_eq!(f.preview().journal, 1, "counted, retried");
    staged
}

/// Round 5 (Astra), finding 14: the recorded Mux copy is gone before the
/// redeploy, the new place holds an unrelated folder, the operator answers
/// Overwrite, and the run is cut before the swap. Nothing serves the skill
/// at the old place, so the launch check keeps the staged copy and its entry
/// `Unfinished` instead of discarding the only usable copy. The answer
/// belongs to the ask of that one run: a later run asks again, and the
/// answered run completes the replacement through the exchange; the stale
/// entry is then finished (the new place serves) and its staged copy goes.
#[cfg(unix)]
#[test]
fn r5_pre_swap_missing_old_unusable_destination_keeps_recovery() {
    let f = fixture(&["mux"]);
    let old = f.copy_row("mux", ".mux/skills/alpha");
    fs::remove_dir_all(&old).unwrap();
    let dest = foreign_at(&f, ".xum/skills/alpha");
    let answer = xum_overwrite(&dest);
    test_seams::crash_before_swap();
    let crashed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        redeploy_unlocked(
            &f.store,
            &f.roots,
            &f.central_dir(),
            &PlacementSettings::default(),
            std::slice::from_ref(&answer),
            NOW,
            |_| {},
        )
    }))
    .is_err();
    assert!(crashed, "interrupted");

    let finished = finish_journal(&f.store, &f.roots, &f.central_dir(), NOW)
        .unwrap()
        .expect("reported");
    assert_eq!(finished.journal.len(), 1);
    assert_eq!(finished.journal[0].status, JournalStatus::Unfinished);
    assert!(finished.left_behind.is_empty(), "{finished:#?}");
    let staged = assert_staged_kept_unfinished(&f);
    assert_eq!(
        fs::read_to_string(dest.join("SKILL.md")).unwrap(),
        "someone else's",
        "the occupant is untouched"
    );

    // A new run is a new operator action: the folder in the way is asked
    // about again, and the kept entry stays unfinished.
    let asked = f.run(&[]);
    assert!(
        matches!(
            &only_move(&asked).status,
            RedeployStatus::Failed { error: CommandError::TargetExists { path, .. } }
                if Path::new(path) == dest
        ),
        "{asked:#?}"
    );
    assert_eq!(asked.journal[0].status, JournalStatus::Unfinished);
    assert!(serves_at(&staged));

    // The answered run replaces the folder through the exchange.
    let report = f.run(&[answer]);
    assert!(
        matches!(only_move(&report).status, RedeployStatus::Moved),
        "{report:#?}"
    );
    assert!(report.refused_answers.is_empty(), "{report:#?}");
    assert!(serves_at(&dest), "the skill is in its new place");
    let row = f.row("mux").unwrap();
    assert_eq!(Path::new(&row.target_path), dest);
    assert_eq!(row.status, SyncStatus::Synced);

    // The new place serves now: the stale entry's staged copy goes.
    let finished = finish_journal(&f.store, &f.roots, &f.central_dir(), NOW)
        .unwrap()
        .expect("settled");
    assert_eq!(finished.journal[0].status, JournalStatus::Finished);
    assert!(!present(&staged));
    assert_eq!(xum_entries(&f), 1);
    assert!(f.store.list_redeploy_journal().unwrap().is_empty());
    assert_eq!(f.preview(), RedeployPreview::default());
}

// ---------------------------------------------------------------------------
// Round 4 (Astra): ancestor aliases through the destination (findings 11, 12)
// ---------------------------------------------------------------------------

/// Whether the skill is served at `path`: its bytes read through it.
fn serves_at(path: &Path) -> bool {
    fs::read_to_string(path.join("a.txt")).is_ok_and(|text| text == "v1")
}

/// What each placement is, physically, before a refused move: the entry and
/// the bytes read through it.
#[cfg(unix)]
fn placement_of(path: &Path) -> (Option<EntryId>, Option<String>) {
    (
        EntryId::of(path).unwrap(),
        fs::read_to_string(path.join("a.txt"))
            .or_else(|_| fs::read_to_string(path.join("SKILL.md")))
            .ok(),
    )
}

/// `f.at(rel)` spelled through the destination `~/.xum/skills/alpha`
/// (`…/alpha/../../..` is the home folder).
#[cfg(unix)]
fn through_xum_alpha(f: &Fixture, rel: &str) -> PathBuf {
    let mut path = f.at(".xum/skills/alpha/../../..");
    path.extend(rel.split('/'));
    path
}

/// Finding 11 (Astra's first probe): the old copy's parent `~/.mux/skills`
/// is a link to `~/.xum/skills/alpha/../../../.mux-backing/skills` — a real
/// copy, reached through the folder in the way. With no exchange the move is
/// refused before any rename: both placements unchanged.
#[cfg(unix)]
#[test]
fn f11_a_copy_reached_through_the_destination_is_refused_without_an_exchange() {
    let f = fixture(&["mux"]);
    let dest = foreign_at(&f, ".xum/skills/alpha");
    fs::create_dir_all(f.at(".mux-backing/skills")).unwrap();
    fs::remove_dir_all(f.at(".mux/skills")).ok();
    link(
        &through_xum_alpha(&f, ".mux-backing/skills"),
        &f.at(".mux/skills"),
    );
    let old = f.copy_row("mux", ".mux/skills/alpha");
    assert!(f.at(".mux-backing/skills/alpha/a.txt").is_file());
    let before = (placement_of(&old), placement_of(&dest));
    test_seams::force_no_exchange();
    test_seams::crash_after_swap();

    let report = f.run(&[xum_overwrite(&dest)]);

    assert_overwrite_refused(&f, &report, &old, &dest);
    assert_eq!((placement_of(&old), placement_of(&dest)), before);
    assert!(serves_at(&old));
}

/// Finding 11 (Astra's second probe): the old link's text is exactly the
/// skill's recorded Library copy path, but that path runs through an alias
/// (`~/.libalias`) that passes through the folder in the way. With no
/// exchange the move is refused before any rename: both placements
/// unchanged.
#[cfg(unix)]
#[test]
fn f11_a_library_alias_through_the_destination_is_refused_without_an_exchange() {
    let f = fixture(&["mux"]);
    let dest = foreign_at(&f, ".xum/skills/alpha");
    // `~/.libalias` → the folder holding the home, spelled through `dest`.
    let above_home = f.home().parent().unwrap();
    link(&f.at(".xum/skills/alpha/../../../.."), &f.at(".libalias"));
    let aliased = f
        .at(".libalias")
        .join(f.central.strip_prefix(above_home).unwrap());
    let mut skill = f.store.get_skill_by_id("alpha").unwrap().unwrap();
    skill.central_path = aliased.to_string_lossy().into_owned();
    f.store.upsert_skill(&skill).unwrap();
    let old = f.at(".mux/skills/alpha");
    link(&aliased, &old);
    f.seed("mux", &old, SyncMode::Symlink, SyncStatus::Synced, true);
    assert_eq!(fs::read_link(&old).unwrap(), aliased, "the app's text");
    assert!(serves_at(&old));
    let before = (placement_of(&old), placement_of(&dest));
    test_seams::force_no_exchange();
    test_seams::crash_after_swap();

    let report = f.run(&[xum_overwrite(&dest)]);

    assert_overwrite_refused(&f, &report, &old, &dest);
    assert_eq!((placement_of(&old), placement_of(&dest)), before);
    assert_eq!(fs::read_link(&old).unwrap(), aliased);
}

/// Finding 12's layout: both harness folders are aliases spelled through the
/// folder in the way — `~/.xum/skills` → `H/actual/alpha/..` (so the
/// destination `~/.xum/skills/alpha` is `H/actual/alpha`, someone else's)
/// and `~/.mux/skills` → `H/actual/alpha/../mux`, holding the old copy.
/// Answered Overwrite.
#[cfg(unix)]
fn f12_layout() -> (Fixture, PathBuf, PathBuf, RedeployAnswer) {
    let f = fixture(&["mux"]);
    let actual = f.at("actual");
    foreign_at(&f, "actual/alpha");
    fs::create_dir_all(actual.join("mux")).unwrap();
    for (alias, text) in [
        (".xum/skills", actual.join("alpha/..")),
        (".mux/skills", actual.join("alpha/../mux")),
    ] {
        fs::remove_dir_all(f.at(alias)).ok();
        link(&text, &f.at(alias));
    }
    let old = f.copy_row("mux", ".mux/skills/alpha");
    let dest = f.at(".xum/skills/alpha");
    assert_eq!(
        fs::read_to_string(dest.join("SKILL.md")).unwrap(),
        "someone else's"
    );
    let answer = xum_overwrite(&dest);
    (f, old, dest, answer)
}

/// Finding 12's invariant: a state where neither placement serves the skill
/// keeps its journal entry.
#[cfg(unix)]
fn assert_never_unserved_and_forgotten(f: &Fixture, old: &Path, dest: &Path) {
    let journaled = f.store.list_redeploy_journal().unwrap().len();
    assert!(
        serves_at(old) || serves_at(dest) || journaled > 0,
        "old serves {}, new serves {}, journal entries {journaled}",
        serves_at(old),
        serves_at(dest)
    );
}

/// Finding 12 with no exchange: refused before any rename, both aliases
/// still resolving, both placements unchanged.
#[cfg(unix)]
#[test]
fn f12_aliases_through_the_destination_are_refused_without_an_exchange() {
    let (f, old, dest, answer) = f12_layout();
    let before = (placement_of(&old), placement_of(&dest));
    test_seams::force_no_exchange();

    let report = f.run(&[answer]);

    assert!(
        refused_unsafe(&only_move(&report).status, &dest),
        "{report:#?}"
    );
    assert_eq!((placement_of(&old), placement_of(&dest)), before);
    assert!(serves_at(&old));
    assert!(f.store.list_redeploy_journal().unwrap().is_empty());
}

/// Finding 12 with the exchange: the new place is never empty, so neither
/// alias ever dangles, and the swap completes — the skill served at every
/// step, and no state forgets the journal while it is unserved.
#[cfg(unix)]
#[test]
fn f12_aliases_through_the_destination_complete_with_the_exchange() {
    let (f, old, dest, answer) = f12_layout();

    let report = f.run(&[answer]);

    assert!(
        matches!(
            only_move(&report).status,
            RedeployStatus::Moved | RedeployStatus::LeftOld { .. }
        ),
        "{report:#?}"
    );
    assert!(serves_at(&dest), "{report:#?}");
    assert_never_unserved_and_forgotten(&f, &old, &dest);
}

/// Finding 12, cut at each step of the staged replacement and finished at
/// launch: no state leaves the skill unserved with its journal entry gone.
#[cfg(unix)]
#[test]
fn f12_a_cut_replacement_never_forgets_an_unserved_skill() {
    for arm in [
        test_seams::crash_before_swap as fn(),
        test_seams::crash_after_swap,
        test_seams::crash_before_switch,
        test_seams::crash_after_switch,
    ] {
        let (f, old, dest, answer) = f12_layout();
        arm();
        let crashed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            redeploy_unlocked(
                &f.store,
                &f.roots,
                &f.central_dir(),
                &PlacementSettings::default(),
                std::slice::from_ref(&answer),
                NOW,
                |_| {},
            )
        }))
        .is_err();
        assert!(crashed, "interrupted");
        assert_never_unserved_and_forgotten(&f, &old, &dest);

        finish_journal(&f.store, &f.roots, &f.central_dir(), NOW).unwrap();
        assert_never_unserved_and_forgotten(&f, &old, &dest);
        f.run(&[]);
        assert_never_unserved_and_forgotten(&f, &old, &dest);
    }
}

/// F15 (destination aliased onto the central repo): the folder serving Codex
/// is an alias of the Library. The deploy is refused before anything is
/// written; the central bytes and the old link survive.
#[cfg(unix)]
#[test]
fn f15_a_destination_aliased_onto_the_central_repo_is_refused() {
    let f = agents_fixture(&["codex"]);
    let old = f.link_row("codex", ".codex/skills/alpha");
    fs::create_dir_all(f.at(".agents")).unwrap();
    link(&f.central_dir(), &f.at(".agents/skills"));

    let report = f.run(&[]);

    assert!(
        matches!(
            &only_move(&report).status,
            RedeployStatus::Failed {
                error: CommandError::DestinationRefused {
                    reason: DestinationRefusal::InLibrary,
                    ..
                }
            }
        ),
        "{report:#?}"
    );
    assert_eq!(fs::read_to_string(f.central.join("a.txt")).unwrap(), "v1");
    assert!(present(&old));
    assert!(f.row("codex").is_some());
    let central_entries: Vec<_> = fs::read_dir(f.central_dir()).unwrap().collect();
    assert_eq!(central_entries.len(), 1, "nothing written into the Library");
}

/// F15 (the old root aliased onto the central repo): Codex's old folder is an
/// alias of the Library, so its "old copy" is the central copy itself. The
/// move completes, and the old copy is kept: the new link resolves into it
/// (`holds_deployment`, checked first), and the removal fence would refuse it
/// anyway. The central bytes survive, reported left behind.
#[cfg(unix)]
#[test]
fn f15_an_old_root_aliased_onto_the_central_repo_never_loses_central_bytes() {
    let f = agents_fixture(&["codex"]);
    fs::create_dir_all(f.at(".codex")).unwrap();
    link(&f.central_dir(), &f.at(".codex/skills"));
    let old = f.at(".codex/skills/alpha");
    f.seed("codex", &old, SyncMode::Copy, SyncStatus::Synced, true);

    let report = f.run(&[]);

    assert_eq!(fs::read_to_string(f.central.join("a.txt")).unwrap(), "v1");
    assert_eq!(report.left_behind.len(), 1, "{report:#?}");
    assert_eq!(
        report.left_behind[0].reason,
        RedeployLeftReason::HoldsDeployment
    );
    assert_eq!(
        fs::read_link(f.at(".agents/skills/alpha")).unwrap(),
        f.central
    );
}

/// F16 (Keep over a copy with local-only edits): Keep leaves the move undone,
/// and the copy holding the only bytes of the operator's edit is untouched,
/// its row unchanged.
#[test]
fn f16_keep_over_a_copy_with_local_only_edits_keeps_the_edits() {
    let f = agents_fixture(&["codex"]);
    let old = edited_copy(&f);
    foreign_at(&f, ".agents/skills/alpha");

    let report = f.run(&[occupied(&f, OverrideDecision::Keep)]);

    assert!(matches!(
        only_move(&report).status,
        RedeployStatus::Kept { .. }
    ));
    assert_eq!(
        fs::read_to_string(old.join("local.txt")).unwrap(),
        "only here"
    );
    let row = f.row("codex").unwrap();
    assert_eq!(row.status, SyncStatus::Synced);
    assert!(row.owns_artifact());
    assert!(f.store.list_redeploy_journal().unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// Report and preview shape
// ---------------------------------------------------------------------------

/// The report crosses the wire as itself, with its serde tags.
#[cfg(unix)]
#[test]
fn the_report_crosses_the_wire_with_its_tags() {
    let f = agents_fixture(&["codex", "goose"]);
    f.link_row("codex", ".codex/skills/alpha");
    f.seed(
        "goose",
        &f.at(".config/goose/skills/alpha"),
        SyncMode::Symlink,
        SyncStatus::Disabled,
        false,
    );

    let json = serde_json::to_value(f.run(&[])).unwrap();

    assert_eq!(json["moves"][0]["status"]["status"], "moved");
    assert_eq!(json["moves"][0]["cause"], "correction");
    assert_eq!(json["moves"][0]["folder_key"], "agents_skills");
    assert_eq!(json["lost_disables"][0]["tool"], "goose");
}

#[cfg(unix)]
#[test]
fn the_preview_counts_moves_and_names_their_harnesses() {
    let f = agents_fixture(&["codex", "kilo_code", "claude_code"]);
    f.link_row("codex", ".codex/skills/alpha");
    f.link_row("kilo_code", ".kilocode/skills/alpha");
    f.link_row("claude_code", ".claude/skills/alpha");

    let preview = f.preview();

    assert_eq!(preview.global.correction, 2);
    assert_eq!(preview.global.migration, 0);
    assert_eq!(preview.journal, 0);
    assert_eq!(preview.harnesses, vec!["codex", "kilo_code"]);
}

/// A name that is not one folder name has no place in any folder: refused
/// before anything is touched.
#[cfg(unix)]
#[test]
fn a_skill_name_that_is_not_a_folder_name_is_refused() {
    let f = agents_fixture(&["codex"]);
    let old = f.link_row("codex", ".codex/skills/alpha");
    let mut skill = f.store.get_skill_by_id("alpha").unwrap().unwrap();
    skill.name = "../escape".into();
    f.store.upsert_skill(&skill).unwrap();

    let report = f.run(&[]);

    assert!(matches!(
        &only_move(&report).status,
        RedeployStatus::Failed {
            error: CommandError::InvalidPath { .. }
        }
    ));
    assert!(present(&old));
}

/// Legacy rows convert only through the redeploy (ADR-0007 rule 2): the
/// global batch never re-records a row that owns an artifact elsewhere at
/// its new place — that would orphan the old artifact — and skips it with
/// the typed reason instead.
#[cfg(unix)]
#[test]
fn the_global_batch_leaves_a_pending_move_to_the_redeploy() {
    use crate::core::global_sync::{
        sync_skills_to_tools, BatchPolicy, BatchSkill, BatchTargetStatus,
    };
    let f = agents_fixture(&["codex"]);
    let old = f.link_row("codex", ".codex/skills/alpha");

    let outcomes = sync_skills_to_tools(
        &f.roots,
        &f.store,
        &f.central_dir(),
        &[BatchSkill {
            skill_id: "alpha".into(),
            skill_name: "alpha".into(),
            source_path: f.central.clone(),
        }],
        &["codex".to_string()],
        &BatchPolicy {
            overwrite: false,
            overwrite_if_same_content: true,
            overrides: vec![],
        },
        NOW,
        |_| {},
    );

    assert!(
        matches!(
            outcomes.as_slice(),
            [o] if matches!(
                &o.status,
                BatchTargetStatus::Skipped { error: CommandError::RedeployPending { tool, path } }
                    if tool == "codex" && Path::new(path) == old
            )
        ),
        "{outcomes:?}"
    );
    assert!(!present(&f.at(".agents/skills/alpha")), "nothing written");
    assert_eq!(Path::new(&f.row("codex").unwrap().target_path), old);
    assert_eq!(f.preview().global.correction, 1, "still the redeploy's");
}

/// An edited copy awaiting its answer is asked again by the next run, not
/// reported as a launch-check run of its own on every launch.
#[test]
fn the_launch_check_records_nothing_for_an_edited_copy_still_awaiting() {
    let f = agents_fixture(&["codex"]);
    edited_copy(&f);
    f.run(&[]);

    assert!(finish_journal(&f.store, &f.roots, &f.central_dir(), NOW)
        .unwrap()
        .is_none());
    assert!(take_finished_journal(&f.store).unwrap().is_none());
    assert_eq!(
        f.store.list_redeploy_journal().unwrap().len(),
        1,
        "still owed"
    );
}

// ---------------------------------------------------------------------------
// The corrections 1.3.5 ships (Q12 minus the `.agents` moves)
// ---------------------------------------------------------------------------

/// Kilo's correction: its link in `~/.kilocode/skills` moves to
/// `~/.kilo/skills`, keyed by its own folder, and the old link is removed.
#[cfg(unix)]
#[test]
fn kilos_link_moves_to_kilo_skills() {
    let f = fixture(&["kilo_code"]);
    let old = f.link_row("kilo_code", ".kilocode/skills/alpha");

    let report = f.run(&[]);

    assert!(matches!(only_move(&report).status, RedeployStatus::Moved));
    assert_eq!(only_move(&report).folder_key, "kilo_code");
    let new = f.at(".kilo/skills/alpha");
    assert_eq!(fs::read_link(&new).unwrap(), f.central);
    assert!(!present(&old));
    let row = f.row("kilo_code").unwrap();
    assert_eq!(row.id, "row-kilo_code");
    assert_eq!(Path::new(&row.target_path), new);
}

/// Xum's and Antigravity's corrections, with copies (every platform): a copy
/// recording the Library's identity is a provable duplicate and is removed.
#[test]
fn xum_and_antigravity_copies_move_to_their_current_folders() {
    let f = fixture(&["mux", "antigravity"]);
    let xum = f.copy_row("mux", ".mux/skills/alpha");
    let antigravity = f.copy_row("antigravity", ".gemini/antigravity/global_skills/alpha");

    let report = f.run(&[]);

    assert_eq!(report.moves.len(), 2, "{report:#?}");
    assert!(report
        .moves
        .iter()
        .all(|m| matches!(m.status, RedeployStatus::Moved)));
    assert!(!present(&xum) && !present(&antigravity));
    assert_eq!(
        Path::new(&f.row("mux").unwrap().target_path),
        f.at(".xum/skills/alpha")
    );
    assert_eq!(
        Path::new(&f.row("antigravity").unwrap().target_path),
        f.at(".gemini/config/skills/alpha")
    );
    assert!(f.at(".mux/skills").is_dir(), "the old folder itself stays");
}

/// Devin's correction lands where the registry resolves its folder on this
/// OS: `~/.config/devin/skills`, or `%APPDATA%\devin\skills` on Windows
/// (the roaming override; proven by the Windows CI leg).
#[test]
fn devins_copy_moves_to_its_folder_on_this_os() {
    let f = fixture(&["windsurf"]);
    let old = f.copy_row("windsurf", ".codeium/windsurf/skills/alpha");

    let report = f.run(&[]);

    assert!(matches!(only_move(&report).status, RedeployStatus::Moved));
    let devin = adapter_by_key("windsurf").unwrap();
    let expected = devin.skills_dir.resolve(&f.roots).join("alpha");
    if cfg!(windows) {
        assert!(expected.starts_with(&f.roots.roaming), "{expected:?}");
    } else {
        assert_eq!(expected, f.at(".config/devin/skills/alpha"));
    }
    assert_eq!(Path::new(&f.row("windsurf").unwrap().target_path), expected);
    assert!(expected.join("SKILL.md").is_file());
    assert!(!present(&old));
}

/// The operator's machine on 1.3.5 (Claude Code, Codex, Pi and `.agents`,
/// every row in its own folder): nothing to move, so no launch offer.
#[test]
fn the_operators_machine_has_nothing_to_move() {
    let f = fixture(&["claude_code", "codex", "pi", "agents_skills"]);
    f.copy_row("claude_code", ".claude/skills/alpha");
    f.copy_row("codex", ".codex/skills/alpha");
    f.copy_row("pi", ".pi/agent/skills/alpha");
    f.copy_row("agents_skills", ".agents/skills/alpha");

    assert_eq!(f.preview(), RedeployPreview::default());
    assert!(f.run(&[]).moves.is_empty());
}
