## Map of the territory

- Review point: `HEAD` is `943f85c4ca55777e89cb59bd9860ea70d116f93a` (`943f85c`, “translate remaining Chinese comments and test assertion messages to English”). The recent refactoring epic concentrated on the command/core seam (`ee4f826`), tagged command errors (`ac3395c`), backend-owned global sync (`8484538`), shared Tool configuration (`6ba5ae4`), the `App.tsx` carve into per-world hooks (`fc89d1f`/`bb77d83`), generated IPC DTOs (`e69ac42`), the shared modal shell (`936a835`), and hook tests (`be9a74c`). I weighted those areas, then probed Tool path policy, Project cleanup, settings/startup, and Managed skill lifecycle code that the epic did not fully deepen.
- The frontend tier has `src/App.tsx` as binder: it owns navigation and composes reporter → sync → library → settings/explore/add-flow hooks (`src/App.tsx:33-36`, `src/App.tsx:112-134`). The Projects world is separately bound inside `src/components/projects/ProjectsPage.tsx`, through `useProjectState` (`src/components/projects/ProjectsPage.tsx:16-19`).
- The Tauri command adapter is split between `src-tauri/src/commands/mod.rs` and `src-tauri/src/commands/projects.rs`; the project invariant says this tier is wiring only and business behavior belongs in `core/` (`AGENTS.md:85-87`). Core implementation is divided among Managed skill installation, global sync, Project operations/sync, Tool adapters, and the SQLite `SkillStore` (`src-tauri/src/core/mod.rs:1-21`).
- The strongest existing seam is global sync: one batch command maps DTOs and progress into `global_sync::sync_skills_to_tools` (`src-tauri/src/commands/mod.rs:559-641`), while planning, shared skills dir deduplication, overwrite policy, record fan-out, and per-target outcomes stay behind the core interface (`src-tauri/src/core/global_sync.rs:257-506`). This has high depth and should remain the model.
- IPC types and command errors form a settled generated seam. `CommandError` is translated only at the command adapter; the frontend owns user prose. This review does not reopen ADR-0001 (`docs/adr/0001-tagged-command-error-contract.md:1-18`).

## Findings

### 1. Tool path policy leaks through the interface, and Project cleanup already selects the wrong path family

**Files**

- `src-tauri/src/core/tool_adapters/mod.rs`
- `src-tauri/src/core/project_ops.rs`
- `src-tauri/src/core/project_sync.rs`
- `src-tauri/src/core/gitignore.rs`
- `src-tauri/src/commands/mod.rs`

**Problem**

The Tool adapter module is shallow at its most important seam. Its interface exposes raw global path fields (`relative_skills_dir`, `relative_detect_dir`) while a separate function supplies the Project path. Every caller must therefore know whether it is operating in global or Project scope. Virtual group, Constituent tools, Shared skills dir group, installedness, and presentation-view policy also leak into the command adapter. That loses locality: Tool policy is not actually owned by the Tool module.

The deletion test is already visible in reverse. Although `project_relative_skills_dir` exists, path-selection complexity has reappeared in callers, and different callers make different choices.

**Evidence**

- `ToolAdapter` publishes both raw path fields at `src-tauri/src/core/tool_adapters/mod.rs:120-127`; Project mapping is a separate exhaustive match at `src-tauri/src/core/tool_adapters/mod.rs:482-527`.
- Project sync consistently uses `project_relative_skills_dir` at `src-tauri/src/core/project_sync.rs:45-50`, `src-tauri/src/core/project_sync.rs:117-122`, and `src-tauri/src/core/project_sync.rs:363-368`.
- Project cleanup instead joins `adapter.relative_skills_dir` at `src-tauri/src/core/project_ops.rs:135-141`, `src-tauri/src/core/project_ops.rs:188-193`, and `src-tauri/src/core/project_ops.rs:200-205`.
- The gitignore module records that using the global field for Project paths previously produced entries that did not match Project sync for Windsurf, Pi, Goose, and Augment (`src-tauri/src/core/gitignore.rs:4-8`); it now correctly delegates at `src-tauri/src/core/gitignore.rs:27-34`.
- Global and Project Tool views are assembled with different grouping rules inside the command adapter (`src-tauri/src/commands/mod.rs:68-112` and `src-tauri/src/commands/mod.rs:133-199`), despite the wiring-only invariant.

**Proposed deepening**

Deepen the Tool module into the single owner of Tool metadata and scope-specific path resolution. Keep the exhaustive `ToolId` enum, but make raw path representation private. Let callers request a resolved global skills root, resolved Project skills root/target, detect root, global Tool view, or Project Tool view. The module should also own Virtual group expansion and Shared skills dir group membership. The command adapter should only translate the resulting core values to generated DTOs.

Do not introduce a trait merely for abstraction: there is one metadata registry today, so “one adapter = hypothetical seam.” The real seam is scope-specific Tool policy, with many callers. Table-driven tests should assert both scopes and cleanup targets for every Tool.

**Benefits**

- **Locality:** one module owns the global-vs-Project decision; a path change cannot require auditing arbitrary callers.
- **Leverage:** sync, cleanup, gitignore, onboarding, status views, and future Tool additions all use the same interface.
- **Tests:** the Tool interface becomes the test surface; exhaustive table tests can prove every Tool’s global path, Project path, grouping, and cleanup target without exercising command wiring.

**Strength:** **Strong**

**ADR/invariant conflicts:** No ADR conflict. Preserve the `ToolId` exhaustiveness and README update requirement at `AGENTS.md:63-66`; the refactor should reduce, not weaken, those checks. Preserve Cursor copy behavior.

### 2. The Project module’s behavior is split across an untestable visual adapter, a wide hook interface, command wiring, and two core modules

**Files**

- `src/components/projects/ProjectsPage.tsx`
- `src/components/projects/useProjectState.ts`
- `src-tauri/src/commands/projects.rs`
- `src-tauri/src/core/project_ops.rs`
- `src-tauri/src/core/project_sync.rs`

**Problem**

Understanding one Project workflow requires bouncing across five modules. The registration → Tool configuration → gitignore ordering rule lives in the visual adapter, while data refresh and per-cell concurrency live in the hook, bulk assignment and gitignore derivation live in command wiring, and cleanup/sync live in separate core modules. The current seams do not align with the Project domain concept.

“The interface is the test surface” exposes the pain directly: repository policy allows frontend tests at hook seams and deliberately has no JSX tests (`AGENTS.md:80-83`), yet the load-bearing ordering behavior sits above `useProjectState`, in `ProjectsPage`. The hook itself exposes internal modal flags and raw setters alongside domain actions, making its interface nearly mirror its implementation state.

**Evidence**

- `ProjectState` exposes six data collections, three loading values, six modal values, thirteen domain/loading actions, and six raw modal setters (`src/components/projects/useProjectState.ts:14-56`).
- The critical ordering rule is documented and implemented with `pendingGitignoreRef` in `ProjectsPage` (`src/components/projects/ProjectsPage.tsx:21-29`): registration stores options (`src/components/projects/ProjectsPage.tsx:31-63`), then Tool persistence is calculated and performed before a raw `invoke("update_project_gitignore")` (`src/components/projects/ProjectsPage.tsx:66-103`).
- `useProjectState` loops one command per Tool and then refetches (`src/components/projects/useProjectState.ts:325-365`), so callers must understand partial-progress behavior.
- The command adapter validates projects and Tools, creates records, and assigns UUIDs in `add_project_tool` (`src-tauri/src/commands/projects.rs:69-101`); it also owns the full bulk-assignment loop and partial-failure accumulation (`src-tauri/src/commands/projects.rs:346-414`).
- Gitignore behavior is also orchestrated in the command adapter by loading the Project and Tools, resolving adapters, deriving patterns, and writing files (`src-tauri/src/commands/projects.rs:417-453`).
- The remaining implementation is split between Project registration/cleanup in `project_ops` (`src-tauri/src/core/project_ops.rs:78-235`) and assignment/resync/staleness in `project_sync` (`src-tauri/src/core/project_sync.rs:21-400`).

**Proposed deepening**

Create one deeper Project module interface around domain intents: register/update/remove a Project, replace its configured Tool set, apply ignore-file choices after Tool configuration, assign/unassign/bulk-assign a Managed skill, and resync. Put filesystem/DB ordering and partial-result rules behind that core interface. Keep `commands/projects.rs` as a thin adapter.

On the frontend, keep plain `useState` but deepen `useProjectState` so callers invoke intent-level actions rather than coordinating command order or toggling its internal modal representation. Visual-only open/close state may stay local to the visual adapter; the registration/configuration workflow should cross the hook seam so it is testable there.

**Benefits**

- **Locality:** Project invariants, Tool-set replacement, cleanup, and gitignore ordering become discoverable in one module instead of five.
- **Leverage:** every Project mutation gets consistent locking, refresh, and partial-result behavior.
- **Tests:** core tests exercise the complete Project operation through its interface; hook tests exercise the registration/configuration sequence without JSX. Tests no longer need to reach past the interface into raw `invoke` calls.

**Strength:** **Strong**

**ADR/invariant conflicts:** No ADR conflict. This reinforces the wiring-only command invariant. It must retain generated DTOs (`AGENTS.md:57-61`), plain hook state/no state library (`AGENTS.md:73-79`), `~` expansion, and the backend-owned Virtual group rules.

### 3. Managed skill lifecycle has no single module interface; delete behavior is trapped in command wiring

**Files**

- `src-tauri/src/core/installer.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/core/skill_store.rs`
- `src-tauri/src/core/project_sync.rs`
- `src-tauri/src/core/global_sync.rs`

**Problem**

A Managed skill’s lifecycle is not local. Installation and update live in `installer`; global deployment lives in `global_sync`; Project deployment lives in `project_sync`; persistence lives in `SkillStore`; but deletion’s multi-location filesystem/DB choreography lives directly in the command adapter. This makes the command interface—not a core module interface—the only usable test surface for deletion, contrary to the wiring-only invariant.

The deletion test for the current arrangement is revealing: deleting the command body would not delete complexity; it would force global-target cleanup, Project-target cleanup, central-repo removal, DB deletion, and failure aggregation into another caller. That behavior deserves a deep core module.

**Evidence**

- Core exposes lifecycle entry points for local install, git install, and update at `src-tauri/src/core/installer.rs:39-115`, `src-tauri/src/core/installer.rs:117-213`, and `src-tauri/src/core/installer.rs:735-978`.
- `delete_managed_skill` performs global target removal, Project assignment traversal/path resolution, central directory removal, DB deletion, and failure aggregation inside command wiring (`src-tauri/src/commands/mod.rs:986-1050`).
- The deletion implementation inspects Project assignment status literals before cleanup (`src-tauri/src/commands/mod.rs:1006-1031`), coupling lifecycle behavior to Project sync representation.
- The command test module tests only the low-level `remove_path_any` helper around this area (`src-tauri/src/commands/tests/commands.rs:142-171`); there is no core deletion interface analogous to the core installation entry points.
- Installation finalization is also repeated: regular git install builds and persists a `SkillRecord` (`src-tauri/src/core/installer.rs:117-212`), while selected-candidate git install repeats central-path checks, naming/rename, hashing, record construction, and persistence (`src-tauri/src/core/installer.rs:1299-1403`).

**Proposed deepening**

Deepen `installer` (or rename it to a Managed skill lifecycle module) so one core interface owns create, refresh, and delete. Internally, separate source acquisition from common finalization and cleanup, but keep those as internal seams unless real variation exists. There are already at least two real source adapters—local directory and git source—so that seam is earned; GitHub HTTP and git clone are also two real acquisition adapters behind the git path.

Move deletion into this core module and return a structured result or typed `SignalError`; the command adapter should only convert it to `CommandError`. Keep global sync fan-out in its already-deep module, called through its interface where lifecycle behavior needs it.

**Benefits**

- **Locality:** creation, refresh, record finalization, and deletion invariants are maintained together.
- **Leverage:** all install entry paths share naming, validation, hash, provenance, persistence, and rollback behavior.
- **Tests:** core tests can cover full deletion and common install finalization with temporary directories and a test store, rather than testing a low-level helper or Tauri command body.

**Strength:** **Strong**

**ADR/invariant conflicts:** Preserve ADR-0001: core raises typed conditions and the command adapter performs `CommandError` conversion. Preserve backend-owned global sync and Cursor’s forced-copy rule.

### 4. Project skill assignment status is a stringly interface spread across storage, transitions, aggregation, DTOs, and rendering

**Files**

- `src-tauri/src/core/skill_store.rs`
- `src-tauri/src/core/project_sync.rs`
- `src-tauri/src/core/project_ops.rs`
- `src/bindings/ProjectSkillAssignmentDto.ts`
- `src/bindings/ProjectDto.ts`
- `src/components/projects/AssignmentMatrix.tsx`

**Problem**

The Project skill assignment state machine has no owning module interface. Status and mode are `String` at the store seam, transition rules are literals in `project_sync`, aggregate precedence is another string match in `SkillStore`, cleanup eligibility is another literal check in `project_ops`, and TypeScript receives unrestricted strings. Callers must learn every literal and transition to use the module correctly, so the interface is nearly as complex as the implementation.

Under the deletion test, removing any one literal-handling helper simply moves the same state-machine knowledge to its callers. A typed status module would instead concentrate that knowledge.

**Evidence**

- `ProjectSkillAssignmentRecord` exposes `mode: String` and `status: String` (`src-tauri/src/core/skill_store.rs:156-166`); `update_assignment_status` accepts unvalidated `&str` values (`src-tauri/src/core/skill_store.rs:858-882`).
- Assignment creation and sync outcomes write `pending`, `synced`, and `error` (`src-tauri/src/core/project_sync.rs:31-90`); staleness evaluation separately transitions among `missing`, `stale`, and `synced` (`src-tauri/src/core/project_sync.rs:257-348`).
- Project-level precedence is independently encoded as error/missing > stale > pending > synced in `aggregate_project_sync_status` (`src-tauri/src/core/skill_store.rs:993-1035`).
- Cleanup separately recognizes only `synced` and `stale` (`src-tauri/src/core/project_ops.rs:177-186`).
- Generated DTOs preserve both assignment status and aggregate sync status as unrestricted `string` (`src/bindings/ProjectSkillAssignmentDto.ts:3`, `src/bindings/ProjectDto.ts:3`).
- The matrix sends any assignment status directly into a CSS class and special-cases only `error` (`src/components/projects/AssignmentMatrix.tsx:419-438`).

**Proposed deepening**

Introduce typed Project assignment mode/status values and place transition, cleanup-eligibility, and aggregate-precedence behavior behind their module interface. The SQLite adapter should serialize/parse those values at its seam; generated DTOs should carry a generated enum/union; the matrix should render statuses exhaustively rather than accepting arbitrary strings. Preserve existing stored strings for migration compatibility.

**Benefits**

- **Locality:** valid states and transitions live in one place.
- **Leverage:** storage, sync, cleanup, aggregation, and rendering all share compiler-checked semantics.
- **Tests:** one transition/aggregation table covers the state machine; Rust and TypeScript compilation expose missing cases through the same interface.

**Strength:** **Worth exploring**

**ADR/invariant conflicts:** No ADR conflict. This fits the generated-DTO invariant. A DB schema change is not necessarily required, but parsing legacy values and migration behavior must be considered (`AGENTS.md:68`).

### 5. The add/import hook gained implementation locality, but its external interface still mirrors form and modal internals

**Files**

- `src/hooks/useAddSkillFlow.ts`
- `src/App.tsx`
- `src/hooks/useAddSkillFlow.test.ts`
- `src/hooks/useSkillLibrary.ts`
- `src/hooks/useSyncOrchestration.ts`

**Problem**

The recent `App.tsx` carve usefully moved behavior into a dedicated module, but `useAddSkillFlow` exposes nearly every field, boolean, setter, and event handler. It also consumes eight members of the sync hook plus the full reporter interface. This is a wide seam: App must understand the internal representation of every add/import modal, and tests must construct broad fake adapters.

The deletion test gives a nuanced result: deleting the hook would move substantial orchestration back into App, so the module earns its existence. The opportunity is to deepen it, not split it into more shallow hooks.

**Evidence**

- `AddSkillFlowDeps` requires the reporter, eight selected sync members, and two library members (`src/hooks/useAddSkillFlow.ts:20-32`); the implementation immediately unpacks that wide interface (`src/hooks/useAddSkillFlow.ts:42-68`).
- The hook owns fifteen related state values for plan selection, modal visibility, local/git forms, candidate selection, and Explore installation (`src/hooks/useAddSkillFlow.ts:70-101`).
- Its returned interface exposes those raw values/setters plus more than twenty handlers (`src/hooks/useAddSkillFlow.ts:796-837`).
- App wires those members individually into the add, import, local-pick, and git-pick visual adapters (`src/App.tsx:401-454`, `src/App.tsx:464-478`, `src/App.tsx:538-584`).
- Hook tests recreate reporter, sync, and library adapters before they can exercise the interface (`src/hooks/useAddSkillFlow.test.ts:63-107`).
- Sync and library have similarly broad return surfaces (`src/hooks/useSyncOrchestration.ts:345-374`, `src/hooks/useSkillLibrary.ts:475-497`), so cross-world `Pick` lists are likely to grow as behavior grows.

**Proposed deepening**

Keep a single add/import module, but move its visual adapters alongside its state implementation and expose only intent-level entry points to App (open manual add, install an Explore item, close/cancel the active flow) plus a compact view result if App genuinely needs one. Internal form setters and candidate-selection details should not cross the external seam. Narrow reporter/sync/library dependencies to small intent functions rather than wide slices of other hooks’ returned representations.

Do not add a state library or React context. This is a seam-placement change within the established plain-hook architecture.

**Benefits**

- **Locality:** changing a form field, candidate step, or modal sequence no longer requires editing App’s wiring.
- **Leverage:** install→deploy behavior remains reusable while the caller learns fewer ordering and representation details.
- **Tests:** tests drive user/domain intents through the same small interface App uses; setup fakes fewer unrelated hook members.

**Strength:** **Worth exploring**

**ADR/invariant conflicts:** No ADR conflict. Must preserve the no-state-library rule and App-as-binder rule (`AGENTS.md:73-79`); this deepens those modules rather than replacing the pattern.

### 6. Persisted settings have no typed policy module, so keys, defaults, bounds, and parsing are distributed

**Files**

- `src/hooks/useSettingsState.ts`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/core/cache_cleanup.rs`
- `src-tauri/src/core/skill_store.rs`
- `src-tauri/src/lib.rs`

**Problem**

The generic `SkillStore::get_setting/set_setting` interface pushes settings semantics into callers. Cache bounds/defaults live in one core module, zoom parsing/clamping lives in command wiring and startup wiring, global Tool-selection keys live in command wiring, and the frontend repeats cache bounds. There is no module that owns the persisted settings contract.

The deletion test favors a deep typed settings module: without it, the complexity demonstrably exists in every caller; with it, keys/defaults/validation/serialization disappear from those callers.

**Evidence**

- Cache defaults and maximums are defined and validated in `cache_cleanup` (`src-tauri/src/core/cache_cleanup.rs:13-17`, `src-tauri/src/core/cache_cleanup.rs:24-53`).
- The frontend independently clamps the same cache values to 3650 and 3600 (`src/hooks/useSettingsState.ts:171-202`).
- Global Tool configuration reads/writes raw keys and JSON in the command adapter (`src-tauri/src/commands/mod.rs:759-795`).
- Zoom command wiring reads the raw `ui_zoom_level` string with a default and separately clamps writes (`src-tauri/src/commands/mod.rs:825-850`).
- Startup independently reads and parses the same key/default before first paint (`src-tauri/src/lib.rs:42-51`).
- `useSettingsState` has separate effects for central path, cache cleanup age, cache TTL, GitHub token, and zoom (`src/hooks/useSettingsState.ts:79-120`) and directly embeds zoom presets (`src/hooks/useSettingsState.ts:6`, `src/hooks/useSettingsState.ts:123-142`).

**Proposed deepening**

Add a typed persisted-settings module behind a small interface that owns keys, defaults, validation, and serialization for central repo path, cache policy, GitHub token, zoom, auto-sync, and global Tool selection. `SkillStore` remains the concrete SQLite adapter; startup and command wiring both call the typed module. Keep visual-only theme/local-storage preference in the frontend unless it is intentionally part of the same persisted contract.

Do not add a generic repository trait with only one implementation: “one adapter = hypothetical seam.” The useful depth is typed policy over the existing store adapter.

**Benefits**

- **Locality:** changing a default, bound, key, or encoding is one edit.
- **Leverage:** startup, commands, cleanup, and frontend DTO values share one validated contract.
- **Tests:** table tests cover defaults, malformed legacy values, clamping/rejection, and round trips through the typed interface; startup no longer carries an independent parser.

**Strength:** **Worth exploring**

**ADR/invariant conflicts:** No ADR conflict. Central repo `~` expansion and DB migration behavior must remain intact.

## Top recommendation

Tackle **Finding 1: deepen Tool path policy** first. It has the best pain-to-scope ratio and the strongest evidence of lost locality: the repo recently documented and fixed a global-vs-Project path mistake in gitignore (`src-tauri/src/core/gitignore.rs:4-8`), while Project cleanup still bypasses the Project resolver in three places (`src-tauri/src/core/project_ops.rs:135-141`, `src-tauri/src/core/project_ops.rs:188-205`). A private, scope-aware Tool interface would remove the caller choice that makes this class of error possible, improve every sync/cleanup/onboarding/status caller, and create an exhaustive test surface without reopening any settled ADR.
