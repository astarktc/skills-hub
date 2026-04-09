---
status: resolved
trigger: "7 integration tests in tests/gitignore.rs failing -- remove_block leaves marker block intact"
created: 2026-04-09T00:00:00Z
updated: 2026-04-09T00:01:00Z
---

## Current Focus

hypothesis: CONFIRMED -- the test's index-scanning remove_block has a bug where start/end detection collide in the same loop iteration
test: Runtime trace with debug prints confirmed the exact failure mechanism
expecting: Replacing the test's algorithm with the production state-machine algorithm will fix all 7 tests
next_action: Replace the test's remove_block with the production version from commands/projects.rs

## Symptoms

expected: `remove_block` should strip the Skills Hub marker block from gitignore content
actual: The marker block remains after `remove_block` is called. 7 tests fail.
errors: marker should be removed, roundtrip assertions fail
reproduction: `cd src-tauri && cargo test --test gitignore`
started: Tests exist but implementation diverged

## Eliminated

- hypothesis: The production code in commands/projects.rs is also broken
  evidence: The production code uses a correct state-machine approach (in_block flag). The test code uses a different, buggy index-scanning approach.
  timestamp: 2026-04-09T00:00:00Z

- hypothesis: MARKER constant mismatch
  evidence: MARKER = "# Skills Hub" matches the marker line via contains()
  timestamp: 2026-04-09T00:00:00Z

## Evidence

- timestamp: 2026-04-09T00:00:00Z
  checked: Two different algorithms exist -- test vs production
  found: Test file (lines 42-76) uses index-scanning with start/end variables. Production code in commands/projects.rs (lines 441-486) uses state-machine with in_block flag. The algorithms are completely different.
  implication: The test was supposed to be a "faithful reimplementation" but diverged.

- timestamp: 2026-04-09T00:01:00Z
  checked: Runtime trace of the test's remove_block with actual test inputs
  found: ROOT CAUSE CONFIRMED. When the marker line has a preceding blank line, `start` is set to `i-1` (the blank line index). But the end-detection check `i > start.unwrap()` runs on the SAME loop iteration where `start` is set (because i is the marker line index, and start was set to i-1, so i > i-1 is true). The marker line "# Skills Hub ..." does NOT start with '/', so `!line.starts_with('/')` is true, triggering `end = Some(i)`. Result: drain only removes the blank line, leaving the marker and all pattern lines intact.
  implication: The bug is specifically in the interaction between "include preceding blank line" (setting start=i-1) and the end-detection running in the same iteration (since i > i-1). This only manifests when there IS a preceding blank line. Without a preceding blank line, start=i and i > i is false, so end detection does not run on the marker line itself -- which is why test_remove_block_no_preceding_blank_line and test_remove_block_entire_file_is_block PASS.

## Resolution

root_cause: The test file's `remove_block` (lines 42-76) is a stale, buggy copy of an earlier algorithm. It uses index-scanning where start/end detection collide in the same loop iteration when a preceding blank line exists. When start is set to i-1 (blank line before marker), the end-detection condition `i > start.unwrap()` is immediately true for the marker line itself (i > i-1), and since the marker line doesn't start with '/', end is set there. Only the blank line gets drained; the marker block remains. The production code in commands/projects.rs uses a correct state-machine approach with an in_block flag that doesn't have this problem.
fix: Replaced the test's remove_block (stale index-scanning algorithm) with the correct state-machine algorithm matching the production code in commands/projects.rs.
verification: All 16 gitignore integration tests pass (was 9 pass / 7 fail). Full Rust test suite passes with zero failures.
files_changed: [src-tauri/tests/gitignore.rs]
