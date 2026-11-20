# 16: An overwrite ask you can answer knowingly

Status: claimed

**Blocked by:** 10 (the edited-copy row kind; everything else could start earlier, but this ticket lands after 10).

**Spec:** `.scratch/round20/spec.md` § Wave B spec → § Overwrite ask (#70); grill Q23; user stories 53–57. Absorbs
BACKLOG **#70**.

**What to build:** the one overwrite ask now tells the operator, per row:

- who put the folder there (Skills Hub, or not ours);
- what is in it, with a content view or a diff against the skill;
- a Reveal in Finder action;
- what each button will do.

Overwritten rows are recorded in the run, like declined ones. The redeploy's edited-copy question (10) gets the same
treatment as a second row kind in the same ask.

## Acceptance criteria

- [x] **Provenance on the wire row:** owned by a recorded Deployment (`synced_at` set, physical identity) vs foreign,
      beside `path`, `modifiedAt`, `sameContent`. Classified where `sync_engine` observes the occupant. Bindings
      regenerated.
- [x] **Content view / diff** of the occupant against the skill, reusing the file browser (`useSkillDocument` over
      `skillFiles`) with a backend read of the occupant fenced like `reveal_path`.
- [x] **Reveal in Finder** per row, through `admit_reveal_path` (extended to admit a reported occupant path; a test
      pins that nothing else is admitted).
- [x] **Button copy states consequences**, e.g. "Keep theirs — {skill} stays off {folder}; Enable later to replace it".
      The edited-copy row's "delete your edited copy" / "leave it (shows twice in …)" uses the readers 10 reports.
- [x] **Overwritten pairs** are an outcome kind beside `declined`, recorded through `recordRun`, folded once in
      `reportOutcome` (`compositeHeadline` included). The overwrite-ask settlement no longer drops them as plain
      `synced`.
- [x] **An ask arriving after Cancel** says the skill was already being added.
- [x] **One operator action still raises one ask** (D12): collisions and edited copies share it.
- [x] **Tests:** `overwriteAsk` and `reportOutcome` folds, the door's settlement of overwritten rows, the
      `admit_reveal_path` fence, i18n coverage.
- BACKLOG #70 left the queue when this ticket opened.
- [x] `npm run version:check && npm run check` is green.

## Comments

- 2026-11-06 (orchestrator): carried over from the ticket 18 review, round 2.
  - **Astra B / Sol N1:** a dismissed Propagation collision must keep its row and path provenance in Notifications
    and repair through the row-bound route (`resolve_target_collisions`), not a batch Retry against the current
    destination. If it does start a fresh deploy instead, the copy must say so, including what remains at the old
    path.
  - **Sol N3:** a stale answer (`NOT_FOUND`) reads "the Deployment changed since you were asked, so your answer
    wasn't applied", never `target:<path>`.
- 2026-11-07 (orchestrator): carried over from the ticket 10 review, round 1.
  - **Sol 102:** readers of a copy left behind read as "still visible in …" unless a duplicate is proven. The backend
    keeps old-copy readers separate from proven duplicate readers.
  - **Sol 104 / Astra 4:** a move with both a folder in the way and an edited old copy asks both questions in one ask
    for that one operator action.
- 2026-11-07 (orchestrator): carried over from the ticket 10 review, round 2.
  - **Sol 112:** two edited-copy questions at one path can be told apart, with distinct accessible labels.
  - **Astra:** the retry-report merge associates settlement by `entry_id`, not by `(skill, tool, path)`.
- 2026-11-18 (orchestrator, trial clone): claimed. Opus 5.5 high implements in a worktree; GPT-6 Astra (high) reviews
  (Q28). This ticket was built in a throwaway clone of this repository, as the control lane of a four-lane workflow
  trial (Quartermaster QM-365), and ported here by cherry-pick. Commit ids and "PR #1" in the next entry are the
  clone's; here `35a2a02` is `0f83a5c7`, `be08e2f` is `62b37e58` and `87d17ae` is `0aa016bf`.
- 2026-11-18 (orchestrator, trial clone): done in the clone, merged there as `5aa4f73` (PR #1, head `be08e2f`).
  - Implemented by Opus 5.5 (high). Provenance travels on `TARGET_EXISTS.owner`, set by `ownership::recorded_owner`
    and read in `sync_engine::Occupant::observe`. `core/occupants.rs` records each occupant the backend reported.
    `admit_reveal_path` and the new `list_occupant_files` / `read_occupant_file` admit exactly those paths.
    `overwritten` is a new outcome kind, folded in `reportOutcome` and recorded through `recordRun`.
  - The Comments items are covered:
    - Astra B / Sol N1: a dismissed row is answered through `resolve_target_collisions`.
    - Sol N3: the stale-answer copy.
    - Sol 102: `duplicate_readers` is kept separate from the readers the old copy stays visible in.
    - Sol 104 / Astra 4: `EditedCopy.ahead` puts the edited old copy in the same ask as the folder in the way.
    - Sol 112: `askRowLabels`.
    - Astra: the retry merge matches by `MovedRow.entry_id`.
  - Review (`.scratch/reviews/round20-t16-overwrite-ask-review.md`):
    - Round 1, GPT-6 Astra (high): SHIP with three P2 findings, all fixed in this PR (`35a2a02`, `be08e2f`): the
      truncated listing, bounded comparison, and the copy on edited-copy questions asked ahead of their move.
    - Round 2: SHIP.
  - The orchestrator fixed a Windows-only compile error in a test (`87d17ae`).
  - CI is green on `be08e2f`. The local gate on `be08e2f` passed with 1,503 frontend tests and 1,226 Rust tests,
    1 ignored. `main` was already in the branch, so the merged tree is `be08e2f`'s.
  - Follow-ups:
    - Sol 111 (`redeploy_question` copy) stays with ticket 12.
    - The pre-existing unfenced `list_skill_files` / `read_skill_file` → BACKLOG #121.
