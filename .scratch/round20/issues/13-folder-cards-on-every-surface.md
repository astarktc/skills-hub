# 13: Folder cards on every other surface, and Disable per folder

Status: ready-for-agent

**Blocked by:** 12.

**Spec:** `.scratch/round20/spec.md` § Wave B spec → § Settings and the Harnesses pane ("the same cards on every
surface"), § Deployment rows are per folder (Disable per folder); grill Q18, Q19, Q25; user stories 33–37.

**What to build:** everywhere the app asks "which harnesses?", it shows the same folder model as the Harnesses pane:

- the Add dialog's "Deploy after adding" picker;
- the inspector's Targets section;
- the "Deploy your N skills to … now?" ask;
- My Skills' row dots.

Disable on a shared folder applies to the folder, and its confirmation names everyone who loses the skill.

## Acceptance criteria

- [ ] **Add dialog.** The picker lists folder cards (folder + "Serves …", detected harnesses only) instead of harness
      checkboxes, and `useAddSkillFlow` installs to folder keys.
- [ ] **Inspector Targets.** One entry per folder, its detected harnesses grouped under it. Deploy and Retry act per
      folder through the door (`actions.run`).
- [ ] **`DeployHarnessAsk`** names the folder and the detected harnesses it serves.
- [ ] **My Skills rows.** One dot per folder (`SyncedStrip`). `skillHealth`'s target-level rows are per folder.
      `isLiveTarget` is unchanged in meaning.
- [ ] **Disable per folder (Q19).**
  - `DisableScope`'s pair grain is (skill, folder).
  - The confirmation names every detected harness the folder serves and every other installed reader, and says one
    reader can't be excluded while another needs the folder.
  - `bulkCounts` counts what the settlement touches.
- [ ] **Unavoidable duplicates (Q18).** A skill on both shared folders is marked on its row and in the inspector where
      it hurts (fx `LoadsNone`, Cortex Code `FlagsConflict`).
- [ ] **No surface derives a folder** from a path or rebuilds a card. Everything comes from `useToolCatalog`.
- [ ] **Tests:** hook and pure-fold tests (`skillHealth`, `bulkCounts`, the add flow, the presentation fold), the door's
      Disable confirmation, i18n coverage.
- [ ] **Fixture:** every surface is shown in the new scenario.
- [ ] `npm run version:check && npm run check` is green.

## Comments

- 2026-11-06 (orchestrator): carried over from the ticket 09 review, round 5 (Sol NB1–NB3). Fold these in.
  - **NB1:** derive a folder's failure wording from its folder facts (`status`, `disabled`, `deployed`), never from
    the representative row's `synced_at`. A folder with one synced row and one never-synced error row must not read
    "the first deploy did not complete".
  - **NB2:** give partly disabled project folders explicit presentation and actions: the matrix cell offers "Finish
    disabling" alongside "Sync this". The run report's wording should acknowledge both the successful sync and the
    disable that is still pending, never "Not deployed" while a copy is on disk.
  - **NB3:** show "Removed … · Undo" only when the refreshed folder facts confirm the Disable finished.
