---
phase: quick-260409-sqk
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/core/skill_lock.rs
  - src-tauri/src/core/mod.rs
  - src-tauri/src/core/installer.rs
  - src-tauri/src/core/tests/skill_lock.rs
autonomous: true
must_haves:
  truths:
    - "Skills imported via onboarding that were originally installed by npx skills add get source_type 'git' with full repo URL"
    - "Skills imported from paths that are NOT symlinks into ~/.agents/skills/ remain source_type 'local' unchanged"
    - "Missing or malformed ~/.agents/.skill-lock.json causes no error and no change to import behavior"
  artifacts:
    - path: "src-tauri/src/core/skill_lock.rs"
      provides: "Lock file parser and enrichment lookup"
      exports: ["SkillLockEntry", "try_enrich_from_skill_lock"]
    - path: "src-tauri/src/core/tests/skill_lock.rs"
      provides: "Unit tests for lock file parsing and enrichment"
  key_links:
    - from: "src-tauri/src/core/installer.rs"
      to: "src-tauri/src/core/skill_lock.rs"
      via: "try_enrich_from_skill_lock() call in install_local_skill()"
      pattern: "skill_lock::try_enrich_from_skill_lock"
---

<objective>
Enrich the onboarding import flow so skills originally installed via `npx skills add` (symlinked from tool dirs into `~/.agents/skills/`) are imported with full git provenance from `~/.agents/.skill-lock.json` instead of `source_type: "local"`.

Purpose: Imported skills retain their git origin, enabling future updates from source repos.
Output: New `skill_lock.rs` module, modified `install_local_skill()` in `installer.rs`.
</objective>

<execution_context>
@/home/alexwsl/skills-hub/.claude/get-shit-done/workflows/execute-plan.md
@/home/alexwsl/skills-hub/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/quick/260409-sqk-enrich-onboarding-import-with-skill-lock/260409-sqk-CONTEXT.md
@src-tauri/src/core/installer.rs
@src-tauri/src/core/mod.rs
@src-tauri/src/core/tool_adapters/mod.rs (detect_link pattern)
@src-tauri/src/commands/mod.rs (import_existing_skill command)

<interfaces>
<!-- Key types and contracts the executor needs. -->

From src-tauri/src/core/installer.rs (install_local_skill, lines 28-85):

```rust
pub fn install_local_skill<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    store: &SkillStore,
    source_path: &Path,   // path in tool dir, may be symlink
    name: Option<String>,
) -> Result<InstallResult>
// Currently always sets: source_type: "local", source_ref: source_path, source_subpath: None
```

From src-tauri/src/core/skill_store.rs (SkillRecord fields to enrich):

```rust
pub struct SkillRecord {
    // ...
    pub source_type: String,      // "local" -> "git" when enriched
    pub source_ref: Option<String>,    // filesystem path -> repo URL when enriched
    pub source_subpath: Option<String>, // None -> parent of skillPath when enriched
    pub source_revision: Option<String>, // remains None (lock file has no branch/tag)
    // ...
}
```

Lock file structure (~/.agents/.skill-lock.json):

```json
{
  "version": 3,
  "skills": {
    "agent-browser": {
      "source": "anthropics/skills",
      "sourceType": "github",
      "sourceUrl": "https://github.com/anthropics/skills.git",
      "skillPath": "skills/agent-browser/SKILL.md",
      "skillFolderHash": "abc123..."
    }
  }
}
```

Field mapping:

- sourceUrl -> source_ref
- parent_dir(skillPath) -> source_subpath (e.g., "skills/agent-browser" from "skills/agent-browser/SKILL.md")
- source_type becomes "git"
- source_revision remains None (lock file doesn't track branch/tag)
  </interfaces>
  </context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Create skill_lock.rs module with lock file parser and enrichment lookup</name>
  <files>src-tauri/src/core/skill_lock.rs, src-tauri/src/core/mod.rs, src-tauri/src/core/tests/skill_lock.rs</files>
  <behavior>
    - Test: parse valid lock file JSON -> returns HashMap of skill entries with correct fields
    - Test: parse lock file with missing "skills" key -> returns empty/None
    - Test: parse lock file with unknown extra fields -> succeeds (forward-compatible)
    - Test: try_enrich_from_skill_lock with source_path that is a symlink into ~/.agents/skills/foo -> returns Some(SkillLockEntry) with sourceUrl and derived subpath
    - Test: try_enrich_from_skill_lock with source_path that is NOT a symlink -> returns None
    - Test: try_enrich_from_skill_lock with source_path symlinked to ~/.agents/skills/foo but foo not in lock file -> returns None
    - Test: try_enrich_from_skill_lock with nonexistent lock file -> returns None (no error)
    - Test: skillPath "skills/agent-browser/SKILL.md" -> source_subpath "skills/agent-browser"
    - Test: skillPath "SKILL.md" (root-level) -> source_subpath None
  </behavior>
  <action>
Create `src-tauri/src/core/skill_lock.rs` with:

1. Serde structs for the lock file (only the fields we need):

   ```rust
   #[derive(Deserialize)]
   struct SkillLockFile {
       skills: Option<HashMap<String, SkillLockRaw>>,
   }
   #[derive(Deserialize)]
   #[serde(rename_all = "camelCase")]
   struct SkillLockRaw {
       source_url: Option<String>,
       skill_path: Option<String>,
       // other fields ignored via #[serde(flatten)] or just not listed
   }
   ```

   Use `#[serde(default)]` on the outer struct so missing `skills` key yields None instead of parse error.

2. A public return struct:

   ```rust
   pub struct SkillLockEntry {
       pub source_url: String,
       pub source_subpath: Option<String>,
   }
   ```

3. A public function `try_enrich_from_skill_lock(source_path: &Path) -> Option<SkillLockEntry>` that:
   a. Calls `std::fs::read_link(source_path)`. If Err (not a symlink), return None.
   b. Resolves the link target. If the target is relative, resolve it against source_path's parent.
   c. Checks if the resolved target is under `~/.agents/skills/` using `dirs::home_dir()`. If not, return None.
   d. Extracts the skill name from the path component immediately after `~/.agents/skills/` (i.e., the directory name).
   e. Reads `~/.agents/.skill-lock.json`. If file missing or unreadable, return None.
   f. Parses JSON via serde. If parse fails, return None (defensive).
   g. Looks up the extracted skill name in `skills` map. If not found, return None.
   h. Extracts `source_url`. If missing, return None.
   i. Derives `source_subpath` from `skill_path`: take parent directory of the path. If the parent is empty or "." (meaning SKILL.md is at repo root), set to None. Otherwise, convert to string with forward slashes.
   j. Returns `Some(SkillLockEntry { source_url, source_subpath })`.

4. Register the module in `src-tauri/src/core/mod.rs`: add `pub mod skill_lock;`

5. Create tests at `src-tauri/src/core/tests/skill_lock.rs` with `#[cfg(test)] #[path = "tests/skill_lock.rs"] mod tests;` at the bottom of `skill_lock.rs`. Tests use `tempfile` for filesystem fixtures. For symlink tests, use `std::os::unix::fs::symlink` (the app targets WSL2/macOS/Linux per constraints).
   </action>
   <verify>
   <automated>cd /home/alexwsl/skills-hub && cargo test -p app_lib skill_lock -- --nocapture 2>&1 | tail -20</automated>
   </verify>
   <done>All skill_lock tests pass: parsing valid/invalid lock files, symlink resolution matching, subpath derivation, and graceful handling of missing files.</done>
   </task>

<task type="auto">
  <name>Task 2: Wire enrichment into install_local_skill and verify end-to-end</name>
  <files>src-tauri/src/core/installer.rs, src-tauri/src/core/tests/installer.rs</files>
  <action>
Modify `install_local_skill()` in `installer.rs` to call `try_enrich_from_skill_lock` before creating the `SkillRecord`.

1. Add import at top of installer.rs: `use super::skill_lock::try_enrich_from_skill_lock;`

2. In `install_local_skill()`, after line 56 (after `let description = ...`) and before the `SkillRecord` construction (line 60), add:

   ```rust
   // Enrich with git provenance from ~/.agents/.skill-lock.json if source is a
   // symlink into ~/.agents/skills/ (skills installed via `npx skills add`).
   let (source_type, source_ref, source_subpath) =
       if let Some(lock_entry) = try_enrich_from_skill_lock(source_path) {
           ("git".to_string(), Some(lock_entry.source_url), lock_entry.source_subpath)
       } else {
           ("local".to_string(), Some(source_path.to_string_lossy().to_string()), None)
       };
   ```

3. Update the `SkillRecord` construction to use the enriched values:
   - `source_type,` (was `source_type: "local".to_string()`)
   - `source_ref,` (was `source_ref: Some(source_path.to_string_lossy().to_string())`)
   - `source_subpath,` (was `source_subpath: None`)

4. Add a test in `src-tauri/src/core/tests/installer.rs`:

   ```rust
   #[test]
   fn install_local_skill_enriches_from_skill_lock() {
       // Setup: create ~/.agents/skills/test-skill/ with SKILL.md
       // Setup: create ~/.agents/.skill-lock.json with entry for "test-skill"
       // Setup: create a symlink from a "tool dir" pointing to the agents skill dir
       // Call install_local_skill with the symlink path
       // Assert: record.source_type == "git"
       // Assert: record.source_ref == lock file's sourceUrl
       // Assert: record.source_subpath == derived from lock file's skillPath
   }
   ```

   Use tempfile for all directories. Create a fake `~/.agents/` structure inside the temp dir. Override the home directory detection by having the symlink target be an absolute path and checking the resolved path. NOTE: Since `try_enrich_from_skill_lock` uses `dirs::home_dir()` for the real home, the integration test should create the actual structure under a temp dir and use symlinks pointing there. The function checks `resolved_target.starts_with(home.join(".agents/skills"))`, so the test must either: (a) mock the home dir lookup, or (b) test the enrichment function directly with explicit paths. Prefer option (b) -- test `try_enrich_from_skill_lock` thoroughly in the skill_lock tests (Task 1), and for this integration test, verify that `install_local_skill` calls the enrichment by testing with a non-symlink path and confirming it stays "local".

5. Run `npm run check` to ensure clippy, fmt, build, and all tests pass.
   </action>
   <verify>
   <automated>cd /home/alexwsl/skills-hub && npm run check 2>&1 | tail -30</automated>
   </verify>
   <done>install_local_skill enriches symlinked npx-skills imports with git provenance from the lock file. Non-symlink imports remain source_type "local". All checks pass (lint, build, clippy, tests).</done>
   </task>

</tasks>

<threat_model>

## Trust Boundaries

| Boundary           | Description                                                                 |
| ------------------ | --------------------------------------------------------------------------- |
| Lock file read     | Untrusted JSON from ~/.agents/.skill-lock.json (written by npx CLI, not us) |
| Symlink resolution | Filesystem symlink targets could point anywhere                             |

## STRIDE Threat Register

| Threat ID | Category      | Component     | Disposition | Mitigation Plan                                                                                                                                                                                                                                |
| --------- | ------------- | ------------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| T-sqk-01  | T (Tampering) | skill_lock.rs | accept      | Lock file is user-local (~/.agents/), same trust level as the skills themselves. A malicious lock file could set source_url to any URL, but this only affects the metadata stored in SQLite -- no network requests are made during enrichment. |
| T-sqk-02  | S (Spoofing)  | skill_lock.rs | mitigate    | Validate symlink target is under ~/.agents/skills/ before trusting the lock file lookup. This prevents arbitrary symlinks from triggering enrichment.                                                                                          |
| T-sqk-03  | D (DoS)       | skill_lock.rs | mitigate    | Lock file is read with fs::read_to_string (bounded by file size). Serde parsing is defensive -- malformed JSON returns None, no panic.                                                                                                         |

</threat_model>

<verification>
1. `cargo test -p app_lib skill_lock` -- all lock file parsing and enrichment tests pass
2. `cargo test -p app_lib installer` -- all existing installer tests still pass, new enrichment test passes
3. `npm run check` -- full suite passes (lint, build, clippy, fmt, tests)
</verification>

<success_criteria>

- A skill imported via onboarding whose source path is a symlink into ~/.agents/skills/ AND has a matching entry in ~/.agents/.skill-lock.json gets source_type="git", source_ref=lock file's sourceUrl, source_subpath=derived from skillPath
- A skill imported from a regular (non-symlink) directory retains source_type="local" with the filesystem path as source_ref
- Missing or malformed lock file causes zero errors and falls back to "local" behavior
- All existing tests continue to pass unchanged
  </success_criteria>

<output>
After completion, create `.planning/quick/260409-sqk-enrich-onboarding-import-with-skill-lock/260409-sqk-SUMMARY.md`
</output>
