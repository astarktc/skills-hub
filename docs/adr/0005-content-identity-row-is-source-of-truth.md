# The skill row's `content_hash` is the source of truth for Content identity; hand-editing the central copy is unsupported

**Content identity** (see CONTEXT.md) is the hash that says whether a copy still matches a Managed
skill's central copy. Round 10 (Q2) moved every read of it onto the `skills` row: the hash is
computed by finalize and by Edit — the two paths that land bytes in `~/.skillshub/<skill>` — and
every later reader (Propagation, project reconcile, the same-content checks in the global sync batch
and Onboarding import) reads the row, backfilling it once when absent. That leaves one way for the
row and the directory to disagree: an operator opening `~/.skillshub/<skill>` in an editor. We
decided: **the row is authoritative. Bytes written to the central copy by anything other than the
app are unsupported; `content_identity::same_content` trusts the row for the managed side
deliberately, and no read path re-hashes a managed central copy to check it.** Every Source kind
already has a sanctioned door through which its bytes change, and each of those doors records the
new identity:

| Source kind (see **Provenance**) | Where the operator edits | What rewrites the central copy and its hash |
|---|---|---|
| `git` | The upstream repository (or a fork of it, then Re-point) | Update / Refresh, Restore |
| `local` | The source folder | Update / Refresh, Restore, Re-point (local) |
| `imported` / central-only (ADR-0003) | The in-app **Edit** | Edit, which settles through the Update module |

The consequence the backlog flagged is accepted: after an out-of-band edit, a target that was
byte-identical to the *recorded* copy still passes `overwrite_if_same_content`, so the next sync
replaces it with the edited bytes. That is the app doing exactly what its record says — the target
matched the skill as recorded, and the recorded skill is what a sync deploys.

## Considered options

- **Reconcile-driven re-hash: hash the central copy on read and invalidate or repair the row when
  it differs.** This is the read-time hashing round 10 removed (`project_sync`'s read-time backfill
  and `global_sync`'s both-sides re-hash) — every listing, reconcile pass and same-content check
  would walk the skill tree again, and the row would stop meaning anything on its own. It also
  cannot decide what a mismatch *means*: silently re-recording the edited bytes blesses a change
  no door produced, while flagging it invents a fourth state (recorded, on disk, upstream, and now
  "unknown") for a situation the app never causes. Rejected: it reintroduces the cost round 10
  paid to remove, to serve a workflow that is not supported.
- **Treat an external central edit as an Edit (ADR-0004).** An Edit is a *recorded* operator
  change the app can replay after upstream bytes replace the copy; a free-form directory edit has
  no row, no base value and no replay, so the next Update would silently discard it. Rejected: a
  change the app cannot replay is not an Edit, and pretending otherwise loses it later rather than
  now.
- **Support hand edits by making the central copy itself the source (drop the hash column).** The
  `local` kind already exists for "a folder the operator maintains"; a git skill's central copy is
  a *derived* artifact of its upstream. Rejected: it collapses the source/copy distinction that
  Provenance, Refresh eligibility and Re-point are built on.

## Consequences

- `content_identity` stays the only module that hashes; its managed read has one owner (the row)
  and one backfill (a row with no hash yet, from before the column existed or after a hash I/O
  failure at finalize). Unknown identity is never interpreted as drift.
- A skill an operator wants to change by hand is one of: a `local` skill (edit the source folder,
  then Update), a git skill (fork upstream, Re-point), or an `imported`/central-only skill (in-app
  Edit). Documentation and support answers point at those doors, never at `~/.skillshub`.
- Should an Edit kind beyond invocation mode be wanted, it is added to the Edit model
  (ADR-0004) — a recorded, replayable change — not by loosening this decision.
