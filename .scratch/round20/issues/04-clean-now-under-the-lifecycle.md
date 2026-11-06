# 04: Clean Now under the lifecycle

Status: ready-for-agent

**Blocked by:** 03 (same files: `useSettingsState`, `commands/mod.rs`; run in the same lane after it).

**Spec:** `.scratch/round20/spec.md` § Clean Now; user stories 36–38.

**What to build:** Clean Now (Settings → clear the git cache) runs under the action lifecycle. The cleanup never deletes
a cache entry an acquisition is using, and the age-out at startup follows the same rule. Today's code:
`useSettingsState.ts::handleClearGitCacheNow` (outside `runAction`), `commands/mod.rs::clear_git_cache_now`,
`core/cache_cleanup.rs`, and the `core/git_cache.rs::key_lock` per-key locks.

## Acceptance criteria

- [ ] **The lifecycle.** Clean Now runs under `runAction`: it's refused with the typed reason while another action runs,
      and its control carries the reason (`focusableWhenDisabled`).
- [ ] **The in-use rule.**
  - The cleanup maps each cache entry to its key and takes that key's lock with a **try-lock** before removing the
    entry. A held lock means the entry is skipped and counted `in_use`, never removed and never waited on.
  - The startup age-out follows the same rule.
  - It doesn't take the Mutation guard.
- [ ] **The response.** The command answers `{ cleared, in_use }` (a specta DTO, bindings regenerated). The toast reads
      "Cleared N cached repos; M in use, skipped", and the in-use clause is omitted at zero.
- [ ] **Tests.** A held key lock leaves the entry on disk and counts it; the age-out obeys the same rule; there's a hook
      test for the refusal and the toast copy. The existing removal-fence guarantees for cache cleanup still hold
      (`OwnedDir`, the library-overlap refusal).
- [ ] The fixture backend answers the new shape.
- [ ] `npm run version:check && npm run check` passes.

## Comments
