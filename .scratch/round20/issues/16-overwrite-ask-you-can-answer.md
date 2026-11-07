# 16: An overwrite ask you can answer knowingly

Status: ready-for-agent

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

- [ ] **Provenance on the wire row:** owned by a recorded Deployment (`synced_at` set, physical identity) vs foreign,
      beside `path`, `modifiedAt`, `sameContent`. Classified where `sync_engine` observes the occupant. Bindings
      regenerated.
- [ ] **Content view / diff** of the occupant against the skill, reusing the file browser (`useSkillDocument` over
      `skillFiles`) with a backend read of the occupant fenced like `reveal_path`.
- [ ] **Reveal in Finder** per row, through `admit_reveal_path` (extended to admit a reported occupant path; a test
      pins that nothing else is admitted).
- [ ] **Button copy states consequences**, e.g. "Keep theirs — {skill} stays off {folder}; Enable later to replace it".
      The edited-copy row's "delete your edited copy" / "leave it (shows twice in …)" uses the readers 10 reports.
- [ ] **Overwritten pairs** are an outcome kind beside `declined`, recorded through `recordRun`, folded once in
      `reportOutcome` (`compositeHeadline` included). The overwrite-ask settlement no longer drops them as plain
      `synced`.
- [ ] **An ask arriving after Cancel** says the skill was already being added.
- [ ] **One operator action still raises one ask** (D12): collisions and edited copies share it.
- [ ] **Tests:** `overwriteAsk` and `reportOutcome` folds, the door's settlement of overwritten rows, the
      `admit_reveal_path` fence, i18n coverage.
- BACKLOG #70 left the queue when this ticket opened.
- [ ] `npm run version:check && npm run check` is green.

## Comments

- 2026-11-06 (orchestrator): carried over from the ticket 18 review, round 2.
  - **Astra B / Sol N1:** a dismissed Propagation collision must keep its row and path provenance in Notifications
    and repair through the row-bound route (`resolve_target_collisions`), not a batch Retry against the current
    destination. If it does start a fresh deploy instead, the copy must say so, including what remains at the old
    path.
  - **Sol N3:** a stale answer (`NOT_FOUND`) reads "the Deployment changed since you were asked, so your answer
    wasn't applied", never `target:<path>`.
