# Round 8 — v1.2.8 backlog slice (safety + regressions)

Base: `main` = `51dcae6` (v1.2.7). Source: `.scratch/round7/backlog.md` items 1, 3, 4, 5, 8 and the grilling rulings appended
there (2026-09-09). Hygiene items 2, 6, 9 ride along only where a slice item touches the same code; 7, 10, 11 stay in backlog.

Rulings: two file-disjoint lanes (Rust safety = 1, 3, 4, 8; frontend = 5); Rust lane Astra medium, frontend lane Astra low;
single Fable reviewer on the integrated diff; item 3 honours acquire-first (corrected split rides in the acquisition result,
persisted by finalize's existing backfill); item 1 = per-skill sweep in `move_old_central_aside`, 7-day age bound, no marker;
item 4 = delete the just-landed dir on upsert failure (no backup); item 5 = derived view, no effect; item 8 = every
`github_token(store)?` degrades to token-free like Refresh via one helper. Ships as v1.2.8.

| Lane | Ticket | Model |
| --- | --- | --- |
| A | `issues/01-rust-safety.md` | Astra medium |
| B | `issues/02-detail-view.md` | Astra low |

## Closure — 2026-09-15

Shipped: v1.2.8 / 96cd33c. Tickets: 2 terminal (2 done), 0 still open (none).
Residue → BACKLOG: #19. Dropped by name: none.
