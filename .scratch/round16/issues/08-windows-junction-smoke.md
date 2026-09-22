# 08 — Windows junction fallback executed once (BACKLOG #18, facet a)

Status: ready-for-human
Blocked by: 07 (needs the 1.2.16 release build)
Spec: `.scratch/round16/spec.md` — Dispositions #18.

## Why

`sync_engine.rs:63` (`symlink → junction → copy`) has never executed: CI runs `cargo test` on `ubuntu-latest`
only and no test reaches `try_junction`. This is not Cursor-specific — it is the Windows leg for every tool.
Facet (b), Cursor discovering a skill through a junction, is dropped by name (no Cursor on any operator host;
vendor docs say symlinked dirs are supported).

## Checklist (operator, on the Windows host)

1. Confirm **Developer Mode is off** (Settings → For developers) and run as a non-elevated user — otherwise
   `std::os::windows::fs::symlink_dir` succeeds and the junction leg never fires.
2. Install `Skills-Hub-v1.2.16-Windows-x64.exe` (or arm64) from the GitHub release.
3. Add any small skill (GitHub URL or local folder) and sync it to one installed tool (Claude Code is fine).
4. In the tool's skills directory (e.g. `%USERPROFILE%\.claude\skills`), run `dir /AL` — the skill entry must
   show `<JUNCTION>`; in the app the target's mode should read junction (or the Synced badge with the junction
   tooltip — note what is shown).
5. Unsync it; `dir /AL` shows the entry gone.
6. Paste the `dir /AL` output and a one-line result under `## Comments` here (dated). If symlink was used
   instead, say so — that is evidence too (Developer Mode was on).

## Comments
