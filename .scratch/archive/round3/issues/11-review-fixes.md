# 11: Review-panel fixes (round 3)

Status: done — 93641d7

**What to build:** The cross-reviewer findings from the ticket-09 panel that are cheap and improve correctness or readability, plus the CHANGELOG entry the bump needs.

(a) **Never-narrow holds even without a cache record.** When a clone dir has a `.git` but its `.skills-hub-cache.json` is missing or unparseable, the entry's checkout is treated as `Full` (unknown shape → refetch as the full tree, a widening), never as `requested(subpath)`; a metadata write failure is logged at WARN instead of silently dropped. Test: clone present, sidecar deleted, sparse request → the working tree ends up full and the new record says `full`.

(b) **The success toast of an action can carry a message.** `RunActionOptions.successToast` may resolve to `string | { title, message? }`; `runAction` passes the message to `notify`'s message slot. The import flow's forced-source-Tool lines (`status.importSourceToolForced`) travel as the message, so they render as separate readable lines in the toast description and in the notification panel (`.notif-message` already wraps).

(c) **A batch with failures is a warning, not a success.** The Refresh (all) summary (`N skills refreshed, M failed`) is notified as `warning` when `M > 0` (so it lingers 5 s and counts as unread), `success` otherwise. Same rule for any other batch summary that reports a failure count (unsync-partial, import with failed groups) — find them via the `status.*`/`errors.*` summary keys and apply the one rule.

(d) **Dead key slot.** `git_cache::hash_key_parts` loses its always-`None` third parameter; the digest for existing full-clone entries must not change (keep the trailing separator; a test pins the digest of a known input before and after).

(e) **Docs.** CONTEXT.md **Notification**: "shown as a toast — or folded into one batch toast when an action reports many — and kept in the session's history; the backend log is partial forensics (backend-logged events only), not a record of Notifications." `.scratch/round3/spec.md` Q4: the single entry point sentence becomes "every notification is *recorded* through the reporter; `notify` toasts and records, `showActionErrors` toasts once and records each". `CHANGELOG.md` `[Unreleased]` gets the round's operator-visible entries (persistent error toasts + notification bell/panel + Copy all; Open log folder; import always overwrites the source Tool's original; one clone per Add on non-GitHub hosts; typed removal errors; `version:set` covers both lockfiles) under Added / Changed / Fixed / Internal — the bump commit renames it to `[1.2.3]`.

Source: `$TMPDIR/r3-review-{fable,opus,sol}.md`; spec Q1, Q5, Q6, Q4.

**Blocked by:** None (can start immediately)

- [x] Core test for (a) green; `write_meta` failure logs
- [x] Reporter hook test: `successToast` returning `{ title, message }` reaches `toast.success(title, { description: message })` and the history row has the message; `useAddSkillFlow` test asserts the forced line is the message, not part of the title
- [x] Hook test: a refresh report with `failed > 0` produces a `warning` notification
- [x] Digest-stability test for (d)
- [x] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Status reconciliation: done (pending merge) → done — 93641d7. Evidence: Cited 93641d7 integrated as a745d9e; remaining slices 3d31568,9dd4e20,12f5d91,07fbfd3 reached v1.2.3. The old pending-merge claim is obsolete; src-tauri/src/core/git_cache.rs:195 treats an unreadable sidecar as Full.
- 2026-09-15 — Previous status wording (historical, not a current merge/publication claim): done (pending merge)

Branch `r3/11-review-fixes`, five commits on top of `4d17493`, one per lettered item:

| Item | Commit |
|---|---|
| (a) | `93641d7` fix(git-cache): treat a clone without a readable record as full, never narrow |
| (d) | `f344598` refactor(git-cache): drop hash_key_parts' always-None third slot |
| (b) | `cb3aa59` feat(reporter): a completion toast may carry a message |
| (c) | `6e8b90b` fix(refresh): a batch that finished with failures is a warning, not a success |
| (e) | `cc31ef5` docs: Notification is toasted-or-batched and recorded; the backend log is partial forensics; changelog for the round |

**(a)** `fetch_through_cache` now distinguishes "no clone" from "clone, unreadable record" via `clone_present(repo_dir)` (`.git` exists): the latter takes `Checkout::Full`, so the refetch (`clone_or_pull`, which already reshapes to full) can only widen. Three tests, red first: sidecar deleted → sparse request leaves `skills/b` + root `SKILL.md` in place and the new record says `full`; sidecar unparseable → same; a blocked sidecar path (directory) → the fetch still succeeds (the WARN branch runs). There is no log-capture harness in the crate, so the WARN itself is not asserted — I did not add a logging dependency for it.

**(d)** Pinned the explore preview key against an independently computed digest (`printf 'URL\nSKILL\n' | shasum -a 256`) alongside the existing repo-key pin, then dropped the parameter; both pins green before and after. The trailing separator stays; the doc comment now says why.

**(b)** `CompletionToast = string | { kind?: "success" | "warning"; title; message? }` on `RunActionOptions.successToast` (literal and function-return). The one-shot state widened to `CompletionToast | null`; `setSuccessToastMessage(string | null)`'s public signature is unchanged (the state setter accepts a string). `importSuccessToast` returns `{ title, message }` with `message` undefined when nothing was forced. **Deviation/verify note:** `.notif-message` has `white-space: pre-wrap` (App.css:1017) but sonner's `[data-description]` has no `white-space` rule (`node_modules/sonner/dist/index.mjs`), so `\n` would collapse in the toast — added one App.css rule `[data-sonner-toast] [data-description] { white-space: pre-wrap; }`. The `kind` slot landed in this commit (its test too) since it is one type; (c) only uses it.

**(c)** Picked the smaller surface: `kind` on the `CompletionToast` value, so `runAction` keeps owning the lifecycle and the summary stays ordered after `showActionErrors` (one path, not two). Applied to: Refresh (all) `status.refreshSummary`, unsync-all `unsyncPartial`, and import with failed groups. **Deviation:** import had no count-carrying summary — "Import completed." as a *warning* would be a contradiction — so I added `status.importPartial` ("{{imported}} skills imported, {{failed}} failed.", en + zh), mirroring `refreshSummary`. Resync (`projects.resyncPartial`) and bulk-assign (`projects.bulkAssignFailed`) already notified as `warning`; unchanged.

**(e)** CONTEXT.md and spec Q4 as specified. Also the same finding's other copies (sol #3 cited `commands/mod.rs:195-196` with CONTEXT.md): the `open_log_folder` doc comment (regenerated binding committed — doc-comment-only diff), and two frontend comments that called the log "the post-restart record" (`useStatusReporter.ts`, `useSettingsState.ts`). CHANGELOG `[Unreleased]` filled under Added / Changed / Fixed / Internal, `[1.2.2]` tone; the bump renames it.

Gate: `npm run version:check` → `Version OK (1.2.2)`; `npm run check` → exit 0 (172 vitest, 445 cargo, clippy `-D warnings` clean, fmt clean, bindings undrifted). The known `acquisitions_overlap…` flake did not fire.

