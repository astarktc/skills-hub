# 08 — Windows junction fallback executed once (BACKLOG #18, facet a)

Status: done — 80d3e9d + e3fc6ed (forced-junction tests in a windows-latest CI job) and operator field evidence 2026-11-05
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
- 2026-11-05 — **done.** Field evidence (operator's Windows 11 PC `gpc`, Developer Mode off, non-elevated, installed
  1.3.1 from the GitHub exe; read by the orchestrator over SSH): Explore → `grill-me` deployed to Claude Code and Pi;
  `dir /AL %USERPROFILE%\.claude\skills` shows `<JUNCTION> grill-me [C:\Users\Alexlocal\.skillshub\grill-me]`; both
  rows `mode = junction`, `status = synced`, `synced_at` set. Step 5 (unsync) would have **failed** on 1.3.1:
  `remove_path_any` removed links with `remove_file`, which Windows refuses for a junction — found by the new Windows
  tests and fixed in 1.3.2 (`80d3e9d`, removal by `remove_dir`, the Library copy untouched). Standing proof: 1.3.2's
  `windows-latest` CI job (`e3fc6ed`) runs nine forced-junction tests (creation, presence, removal, dangling junction,
  Disable/Delete settlement, overwrite, content identity) on every push — green on CI run 42112761270. Facet (b) stays
  dropped by name.

