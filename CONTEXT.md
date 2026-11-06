# Skills Hub

Desktop app that installs AI Agent Skills once and deploys them to many AI coding harnesses. This glossary is the canonical vocabulary; terms crystallise here as design decisions land (see `docs/adr/`).

The human-facing words below were set on 2026-11-02 (round 19, D45). Code identifiers, DB columns and error codes keep
their former names (`Tool`, `sync`, `refresh`, `assignment`); an entry whose code differs carries a **Code name** line,
and its _Avoid_ line keeps the former word so a reader coming from the code finds it.

## Language

**Harness**:
An AI coding harness that reads a skills directory (Claude Code, Cursor, Codex, …), identified by a stable key — the thing Skills Hub deploys skills into. Every fact about a harness (its dirs, group membership, capabilities) lives in its single registry record. Where a harness first appears on a surface a newcomer meets, the copy names examples once ("harnesses — Claude Code, Cursor, Codex, …").
**Code name**: Tool — `ToolId`, `ToolAdapter` in `core/tool_adapters/`, `ToolInfoDto`, DB `tool` columns, `TOOL_*` error codes.
_Avoid_: tool (what a harness's users call a model-callable function), agent (a sub-agent, or the model), adapter (the code object serving a harness), client, IDE

**Detected harness** (installedness):
A Harness one of whose detect dirs — its own configuration root(s), several when the vendor moved it across releases (Kimi: `~/.kimi-code`, earlier `~/.kimi`) — exists under the operator's home and is not a **skills-only footprint**: a detect dir holding nothing but the path down to the harness's skills dir (`~/.kiro/skills/x` and nothing else), which is what a skill deployer leaves behind (`npx skills add`, ego-browser, Skills Hub itself once the harness is uninstalled), not evidence of the harness. An empty detect dir counts; anything beside the skills path counts; a detect dir the skills dir does not live under (Amp's `~/.config/amp` beside the shared `~/.config/agents/skills`) counts by presence. A shared skills convention dir is never a detect dir — that is the footprint itself — except for a virtual group entry, whose dir *is* the convention (`~/.agents` legitimately holds only `skills/`). One rule, read by the Harness catalog, the global deploy batch's skip rule and the onboarding scan's `Installed` scope. "Detected" is the human-facing word.
**Code name**: `tool_adapters::is_installed_in` (installed tool).
_Avoid_: detected tool, harness present, harness exists (a footprint exists too)

**Harness capability**:
A per-harness fact that changes how deploying behaves for that harness, recorded on its registry record rather than tested by name in deploy code (today: `supports_symlink` — `false` forces copy mode for that entry; Cursor carried it until Cursor 2.5 fixed symlink discovery; today only Deep Agents Code is copy-only, since it rejects a link resolving outside its skill roots). A capability describes one registry entry: a virtual group carries its own capability and does not inherit its constituents'.
**Code name**: a field of `ToolAdapter` (tool capability), read by `sync_engine::sync_dir_for_tool_with_overwrite`.
_Avoid_: tool capability, special case, harness quirk, Cursor mode

**Virtual group**:
A harness entry that stands in for several harnesses sharing one project-scope skills convention (today: the AGENTS standard's `.agents/skills`, labelled `.agents/skills (N harnesses)` in a project, N counted from its constituents and reader-only harnesses). It appears in harness lists in place of its constituent harnesses and deploys by its own capability (symlink-capable), whatever its constituents' individual capabilities are.
**Code name**: `ToolAdapter::as_virtual_group`; the frontend's `projectToolLabel` composes the counted label.
_Avoid_: meta-tool, umbrella harness

**Constituent harnesses**:
The harnesses absorbed into a virtual group entry. The group counts as installed when any constituent is; group membership is owned by the backend and only presented by the frontend.
**Code name**: the `group` field of `ToolAdapter` (constituent tools).
_Avoid_: constituent tools, member harnesses, sub-harnesses

**Reader-only harness**:
A Harness that reads a virtual group's shared folder (`~/.agents/skills`, `<project>/.agents/skills`) and that Skills Hub serves only through that folder, with no deployment target of its own (Zed, Warp, fx, …) — some have their own skills folder (fx's `~/.fx/skills`), which Skills Hub never deploys to and never accepts as a Library folder. Skills Hub detects it and lists it as a reader of that folder's entry ("Read by …", detected or not) but never deploys to it: it has no switch, is never in the saved selection or the new-harness ask, and owns no Deployment. At project scope it is absorbed into the group like a constituent harness and counts in the group's "(N harnesses)". The first, minimal form of "a Deployment is a folder; harnesses are its readers" (#109).
**Code name**: `ReaderOnlyHarness`, `READER_ONLY_HARNESSES` (`core/tool_adapters/mod.rs`), `ToolCatalogEntry::readers`, `ToolInfoDto.readers`.
_Avoid_: passive harness, virtual harness, follower

**Deployment folder**:
A skills folder Skills Hub writes skills to, at one scope (global, or one Project): one distinct physical skills dir. It **serves** harnesses — the harnesses whose Deployments live there — and other harnesses may also read it. Identified by its **folder key**, a registry entry key: `agents_skills` for `.agents/skills`, `claude_code` for `.claude/skills`, otherwise the harness's own key; a dir several entries share takes the virtual group entry that owns it, else the first entry in registry order (Amp and Kimi Code's `~/.config/agents/skills` is `amp`; Trae and Trae CN's `<project>/.trae/skills` is `trae`). Deployment rows, the saved selection and the catalogs carry folder keys: a Deployment is recorded once per (skill, folder) — once per (project, skill, folder) in a project — never once per harness that shares the folder. Rows written once per shared-dir sibling before 1.4.0 are read as their folder until the redeploy collapses them. That grouping is for display, counting and selection, **never ownership** (`placement::folder_rows`): a write of the folder acts on its **driver** row's recorded path with the driver's own overwrite permission — the driver is a row that owns its artifact whenever any row does, so no sibling ever lends its ownership — and the folder is off only when a row on that path is disabled. After a successful write, settlement extends only to rows recording that very path, byte for byte (the app has just written exactly it). A row recording any other path — another name, or another spelling of the same place (an alias, a hard link, a case variant) — is independent: no resync, Propagation, collision answer or Enable touches, settles or consults it, and it is left for the redeploy. Physical identity never widens a decision: only a successful write of a row's own recorded path makes it an owner. The frontend matches a harness to its rows through `ToolInfoDto.folder_key` (`ProjectToolDto.folder_key` in a project), never through a path.
**Code name**: folder key — `placement::own_folder` / `serving_folder` / `row_folder`, `placement::one_row_per_folder`, `ToolInfoDto.folder_key`.
_Avoid_: target dir, shared dir (unqualified), destination

**Placement**:
Where and how a Deployment should live, answered for one scope from the registry (rows, read facts, capabilities), the settings and the Detected set — and nothing else: no store, no filesystem. Per harness: the Deployment folder serving it and its path. Per folder: its deploy mode (today the registry capability — a virtual group's own, else copy when any entry sharing the dir is copy-only) and who sees it — the detected harnesses it serves, the other detected harnesses that read it by default, the detected reader-only harnesses it serves, and the undetected members kept apart for the Harnesses pane. Every write path asks placement and never re-derives a folder. Today every harness is served by its own folder; the shared-folder preference changes the serving rule here.
**Code name**: `core/placement.rs` — `place`, `Placement`, `DeploymentFolder`, `InstalledSet` (detected by `tool_adapters::installed_set_in`), `PlacementSettings`.
_Avoid_: target resolution, routing, layout (as a noun for this)

**Shared skills dir group**:
The harnesses one Deployment folder serves at a scope — their skills directories resolve to the same location, so deploying to one deploys to all, and the folder has one row per skill. Owned by the backend (the entries sharing a folder key); the frontend only presents it.
**Code name**: `ToolInfoDto.shared_with`.
_Avoid_: dir alias, linked harnesses

**Former skills dir**:
A global skills directory the registry once named for a Harness and has since corrected (Augment: `~/.augment/rules`, its rules dir, until 1.2.18; now `~/.augment/skills`). A registry fact per entry, empty for almost every harness. Never scanned, never deployed into; **Deployment relocation** (once per launch under the Mutation guard) moves each global Deployment still recorded directly inside one into the current dir from the central copy — replacing an existing artifact there only when byte-identical — settles the row at the new path and removes the superseded artifact through Artifact removal's fenced seam; an absent old artifact, a missing central copy or foreign bytes at the new path leave the row where it is, logged, retried next launch. A former dir stays inside the deletion rule so such a row's artifact can always be removed.
**Code name**: `ToolAdapter::former_skills_dirs`; the relocation is Target relocation, `core/target_relocation.rs`.
_Avoid_: legacy path, old dir, migration (that word is the DB's)

**Harness catalog**:
The presentation-ready harness list for one scope (global or project) with installedness, shared-dir groups and virtual-group constituents already resolved against the operator's home. Assembled by the backend; commands only map it to DTOs.
**Code name**: Tool catalog — `tool_adapters::global_tool_entries` / `project_tool_entries`; the frontend's `useToolCatalog`.
_Avoid_: tool catalog, tool status (that's the DTO carrying a catalog), harness list

**Managed skill**:
A skill installed through Skills Hub and tracked in its database — a member of the Library — eligible for deployment to harnesses.
_Avoid_: installed skill (ambiguous with a harness being installed)

**Library** / **Library folder**:
The set of Managed skills (the area that lists it stays "My Skills"), and the folder holding their central copies (`~/.skillshub` by default, configurable in Settings). A skill's **central copy** keeps that name.
**Code name**: central repo — `central_repo_path`, `central_path`, `core/central_repo.rs`.
_Avoid_: Skills Storage Path, storage path, central repo (human-facing), store

**Global** / **Project**:
The two deployment scopes — a harness's own skills directory under the operator's home, or a registered Project's folder.
_Avoid_: user-wide, local (that is a Provenance)

**Manifest**:
A skill's `SKILL.md` document: metadata and invocation rules in its frontmatter, followed by the instructions that a harness reads. An Edit changes the intended fields while preserving unrelated text, formatting and body bytes.
_Avoid_: config file, skill header

**Edit**:
An operator change layered on a Managed skill's central copy, recorded in `skill_edits` and replayed inside every finalize's failure-atomic window (Update / Update all / Restore / Re-point) — a failed replay rolls the central copy, the skill row and the Edit row back together; on conflict — upstream changed the same thing to something other than the Edit's value — the Edit wins and the row is flagged until the operator re-chooses or clears, or until upstream converges to the Edit's value, which clears it (`docs/adr/0004-edits-replay-inside-finalize.md`). V1 kind: invocation mode.
_Avoid_: fork (reserved for a future whole-copy model), hand edit

**Fork**:
Reserved for a future whole-copy operator variant of an upstream skill; not built.
_Avoid_: Edit (the current layered change)

**Update** (one skill) / **Update all**:
One operation at two grains: re-acquire updatable Managed skills (see Provenance) from their source, finalize (replaying Edits inside finalize's window) and bring every **existing** Deployment up to date (Propagation), as one backend batch; a single skill's Update is a batch of one. Restore, Change source (Re-point) and Detach stay their own verbs. "App updates" (the app's own releases) keeps its name.
- **Update module.** Settles four byte adapters — git-acquired Staging dir, local folder, Edit-in-place, and Restore-rebuild — re-admitting acquisition against the current skill and source before landing anything (an explicit Re-point may replace the source); Restore rebuilds an absent central copy.
- **Membership.** An imported skill is not a member of Update all — it is neither acquired nor reported, not "skipped". An Unlocatable skill is a member the batch does not dispatch: it is reported skipped with its state, and no Deployment is ever minted for it; naming it explicitly (Update) proceeds instead, which is how Restore rebuilds a missing central copy. A skill whose last Delete was incomplete (`delete_incomplete`) is never updated — reported skipped with that typed reason, whether the batch is all skills or names it (Update, Re-point); finishing the Delete is the only way forward (D35).
- **Phases.** Acquisition runs outside the Mutation guard over a bounded pool (4 workers, progress ticking in completion order), then each skill is finalized and propagated under the guard, taken per skill. Per-skill and per-target outcomes are report data. Cancellation stops dispatching new acquisitions and finalizes nothing: once observed, no skill reaches phase two.
- **Arrival-only Auto-deploy.** Auto-deploy plays no part: it decides only where a new skill lands, so an Update only brings existing Deployments up to date and never creates one (D34; before it, Update also re-asserted "every skill on every selected harness").
**Code name**: Update is `core/skill_update.rs` / `update_managed_skill`; Update all is Refresh (all) — `core/refresh.rs`, `refresh_managed_skills`.
_Avoid_: refresh, refresh all, re-sync, re-install, pull, re-deploy (that is the project operation)

**Content identity**:
The hash that says whether a copy still matches its central copy; computed at finalize/Edit, read from the row everywhere else, and backfilled by the Content identity module when absent. The row is authoritative: bytes changed in the central copy by anything other than the app are unsupported, and no read path re-hashes a managed copy to check (`docs/adr/0005-content-identity-row-is-source-of-truth.md`).
_Avoid_: content hash (the column), checksum, hand edit (of the central copy)

**Deployment** (Deploy, Deployed):
One Managed skill materialised in one place — a harness's global skills directory or a Project's — and the act of putting it there. Verbs: **Deploy** (to harnesses, globally), **Deploy all** (a source group's hollow pairs), **Deploy this** (one project cell), **Deploy to project**, **Re-deploy** (one project, or all). A deployment is **Deployed**, **Not deployed yet**, an **Out of date copy**, **Copy missing**, **Failed** or **Disabled** (see Deployment status); a pair with no deployment is **Not deployed**, a durable *never put here* that only an explicit Deploy (Deploy, Deploy this, Deploy all, Deploy to project, Enable) fills — nothing fills it on its own, and the difference from Disabled is that those verbs fill a hollow pair and skip a disabled one. A deploy batch attempts each (skill, harness) pair, and each resolves to deployed, skipped, failed or declined as report data, never as a command error. A first deploy onto a path something else already occupies follows one **overwrite rule**: byte-identical content is replaced silently (the batch's same-content policy, every call site); different content settles as a `TARGET_EXISTS` failure carrying the path, the folder's modified time and whether it differs from the skill, which the operator answers in the **Overwrite ask**. Propagation (an already-recorded deployment) always overwrites.
**Code name**: Sync target / sync — `sync_*`, `global_sync.rs`, `skill_targets` (global rows), `project_assignments` (project rows, an *assignment*), `BatchTargetOutcome`.
_Avoid_: sync, synced, sync target, sync pair, assignment / assign (as human-facing words), destination, install (that is Add), not synced

**Overwrite ask**:
The one question an operator action (an Add, a Deploy, an Enable, an Update's Propagation, a Deploy to project or Re-deploy) raises about every Deployment whose first deploy found a folder with different content in the way: each pair listed once — skill → harness → path, the folder's modified time, whether it differs — and answered per pair. Overwrite retries the pair with a per-pair override and its row replaces the asked one in the report; **Keep existing** records the pair Disabled (a project deployment too) and reports it **declined** — the operator's answer, neither a skip nor a failure; a dismissed ask sends nothing, and its pairs read "to review" in the Run. One action asks once, however many reports it settles.
_Avoid_: overwrite confirmation (per pair), collision dialog, conflict (that is an Edit's)

**Effective global harness set**:
The Deployment folders a global deploy writes to: the operator's recorded selection when they configured one — an empty selection included, which means "deploy nowhere" — otherwise the folders serving every Detected harness. The selection is stored as folder keys (`global_selected_folders_v2`; a pre-1.4.0 harness selection is translated once — a folder is on when any harness it serves was selected, a corrupt old value stays a refusal until saved), and is shown as every harness a selected folder serves, so two harnesses sharing one folder are on or off together. Detection is the fallback for a never-configured install, never an override of intent, and the set is not intersected with detection: a selected folder no detected harness reads stays in it and is reported as a skip, which is the operator's only signal that the selection has gone stale.
**Code name**: effective global target set — `settings::effective_global_tool_targets` (folder keys), mirrored in the frontend as `effectiveSyncTargetIds` (harness keys).
_Avoid_: effective global target set, installed harnesses (as a deploy set), detected harnesses

**Staging dir**:
A scratch directory inside the Library folder that an install or update flow fills with a skill's bytes before the finalize step moves it into place; discarded automatically if the flow fails first.
_Avoid_: temp dir, download dir

**Finalize (install)**:
The single last mile that turns a staging dir into a managed skill: final-name choice (SKILL.md `name` beats a derived name; an operator-provided name always wins), typed collision check, move, description + content hash (always computed; absent only on an I/O failure), record upsert, and — for an update — Edit replay before the previous bytes are released. Flows acquire bytes; only finalize records them.
_Avoid_: register, materialise

**Skill discovery**:
The single scan ladder (`core/skill_discovery.rs::discover_skills`) that turns a directory tree into skill candidates: root `SKILL.md`, known scan bases, root-level skills/containers, `marketplace.json` plugins, then a depth-5 walk; dedup by subpath, collapse aliases of the same canonical directory (preferring the real-directory subpath), then sort by name. Every candidate carries a validity (valid / invalid `SKILL.md` with reason / missing `SKILL.md`); listings and the update backfill are adapters that decide what to admit.
_Avoid_: scan, collect skill dirs

**Skill candidate**:
One discovered directory with its subpath, name, description and validity. "Installable" means it has skill bytes (a `SKILL.md`, even a broken one) — the git side's admission rule and what install enforces; "valid" means the manifest parsed — the local picker's rule. A `.claude/skills/` child without a manifest is *discovered* (listed, with its missing-manifest reason) but neither installable nor valid (round 13, BACKLOG #11).
_Avoid_: skill entry, hit

**Deployment status**:
The lifecycle of one Deployment (a global row or a project row): `pending` (**Not deployed yet**) → `synced` (**Deployed**) / `stale` (**Out of date copy** — the deployed copy drifted from the central copy, not "upstream is newer") / `missing` (**Copy missing**) / `error` (**Failed**), plus `disabled` (**Disabled**). A typed enum whose stored and wire spelling are those strings; the store parses it at its seam (legacy `ok` reads as `synced`; an unrecognised value surfaces as `error` with the raw value as diagnostic — the store never coerces it to healthy). Status changes are typed transitions, and the "what should it be" decision (`next_status`) is pure so a reconcile pass can plan before it writes. The reconcile pass (run by the project listing) may re-derive any row's status from what it observes on disk — source/target presence and, for copies, the content hash — and write the canonical string; that is how a legacy or `error` row recovers, and it is grounded in observation, not in the stored value. The one exception is `disabled` (see Disabled): it records the operator's choice, not an observation, so reconcile never changes it.
**Code name**: Sync status — `SyncStatus` (`core/sync_status.rs`), `AssignmentTransition`.
_Avoid_: sync status, stale (human-facing), not synced, state, health, "ok"

**Disabled**:
The operator's durable "not on this harness": a Deployment state that every automatic pass honours, recorded at two grains.
- **Per (skill, folder)**, at global scope: Disabling a harness disables the Deployment folder serving it — its artifact is removed and its row kept `disabled`; a folder with no row yet gets a `disabled` marker row, so a harness the skill was never on can be switched off before a deploy reaches it — recorded only once the folder's artifact is gone (a failed removal mints no marker).
- **Ownership** is independent of status: a row owns the artifact at its path only once a deploy of ours succeeded there (`synced_at` set). A row that was never deployed (the pair marker, a project deployment recorded for a disabled skill, and the same rows after a failed Enable turned them `error`) is **intent only**: it owns nothing at its path, so no removal ever touches that path and no re-deploy or Propagation overwrites it (Delete clears the row alone; Disable records it `disabled`; until then it keeps its status and diagnostic); only a successful deploy makes a row the owner of an artifact.
- **Per skill**: every global Deployment *and* every project Deployment of the skill is removed and kept `disabled`, and `skills.disabled_at` records "off everywhere until enabled"; a Deploy to project made meanwhile is recorded `disabled`, not deployed. A row flips to `disabled` only after its artifact is gone (Artifact removal's settlement; a failed removal keeps it `error`, ADR-0002).
- **Every automatic pass skips it.** Propagation, the global deploy batch (so Onboarding import's first deploy), project deploys, reconcile and Deployment relocation all leave a disabled Deployment alone — and every row of a skill disabled at skill grain, including one a failed Disable kept `error` — and report it **skipped** with the `Disabled` reason (`TARGET_DISABLED` in a deploy batch), never failed. A Deployment folder is one artifact: while any row describing it is disabled, the batch never writes it, whichever harness asked.
- **Only Enable lifts it**: the marker is cleared and the Deployments re-deployed as a first deploy, so `TARGET_EXISTS` can occur and is report data (a global pair with a folder in the way stays `disabled` until the operator answers the overwrite ask, whose retry is the same Enable with a per-pair decision; any other failed global pair is an `error` row that owns nothing). A pair-grain Enable of a skill disabled at skill grain is skipped; only a skill-grain Enable lifts that.
- Every operator removal of a global Deployment is a Disable. **Unsync is not an operator verb** (removed in 1.3.0 with "Uninstall from tool directories"): there is no removal that the next Update undoes, and only Delete removes a Deployment without recording intent.
**Code name**: `core/skill_disable.rs` (`disable_skill_targets` / `enable_skill_targets`), status `disabled`.
_Avoid_: unsync, uninstall from harness, off (for the state), excluded

**Skill health**:
A Managed skill's stored **last outcome** — what is wrong with the skill itself now, as opposed to *deployment health* (one Deployment's own status and `last_error`). One slot on the skill row (`skills.last_outcome_kind` / `last_outcome_error` / `last_outcome_at`, `core/skill_health.rs`): the kind (`fetch_failed` — an Update could not acquire or settle the bytes; `restore_failed` — the same for a skill whose central copy is gone; `repoint_failed`; `edit_conflict` — an Update kept the operator's Edit over disagreeing upstream bytes; `delete_incomplete` — a Delete kept the skill because an artifact could not be removed; `import_failed` — an Onboarding import finalised the skill but could not settle an original), the typed `CommandError` of the report row it came from, and when it settled. Written in core **where the outcome settles**, never for an error that fails the whole command, never for a cancellation; a later failure replaces it, and a success clears only its own family (a settled Update, Restore or Re-point clears the three acquisition kinds; a saved Edit, or an Update whose Edit no longer conflicts, clears `edit_conflict`; a completed Delete removes the row). There is no run log. Its Deployment counterpart: a first deploy that fails to materialise records an `error` row that owns nothing (a `TARGET_EXISTS` records nothing until the operator answers the ask), and "Keep existing" records the pair Disabled. The catalog carries it as `ManagedSkill.health`.
_Avoid_: status (that's a Deployment's), last error (that's a row's diagnostic), history, target health

**Deploy mode**:
How an artifact was materialised — `symlink`, `junction` (Windows fallback) or `copy` — recorded on the row because only copies can drift from their source. Shown as `link` / `copy`.
**Code name**: Sync mode — `SyncMode` (`core/sync_status.rs`).
_Avoid_: sync mode, link type, strategy

**Auto-deploy**:
The one setting that decides where a **new** skill lands — "Deploy added and imported skills to your selected harnesses": on, an Add or an Onboarding import deploys the skill to the selected harnesses (a first deploy); off, it is only added to the Library. Enable is not an arrival and never reads it: Enable always re-deploys the skill's disabled Deployments. It governs nothing after arrival — an Update never adds a Deployment (D34; before it, Update also re-asserted "every skill on every selected harness"). A shared skills directory is one Deployment folder with one row per skill, whichever of its harnesses was deployed to (round 20 retired the D34 amendment's per-sharer records). It shows where it acts: Add's summary, the Import review, the Harnesses settings — not My Skills.
**Code name**: `auto_sync_enabled`.
_Avoid_: auto-sync, re-assert, auto-sync invariant

**Propagation**:
Re-materialising every Deployment of one Managed skill after its central copy changed, in both scopes (global rows and project rows), honouring each harness's capability and each row's Deploy mode (`core/propagation.rs`). A link of ours already follows the central copy, so it is left untouched and reported `Skipped { LinkFollowsSource }` (a row that never materialised — a failed first deploy's `error` row — has no link yet and gets a first deploy, which never overwrites what sits at its path and records nothing beyond the folder's own row); a Deployment folder is written once and reported once, at its driver row's recorded path: it is ours to replace only when the driver materialised it, it needs new bytes when any row recording that very path records a copy, and those rows — never a row recording another path — settle with it; only a Deployment that can drift gets new bytes, through the capability-aware deploy entry point — which may re-materialise a drifting copy as a link on a symlink-capable harness. The row records the Deploy mode actually used, so it stays truthful. Every Deployment resolves to deployed / skipped / failed as report data — one Deployment's failure never fails the operation. Update and Update all acquire bytes and finalize; only Propagation touches Deployments.
_Avoid_: re-deploy (that's a project operation), update (that's the operator action that triggers it), fan-out (that's the initial deploy batch), re-sync

**Provenance**:
Where a Managed skill's bytes come from — one of three kinds recorded on the skill. `git`: an external repository. `local`: an independent folder the operator maintains, outside every harness's skills directory. `imported`: taken over from a harness's skills directory by Onboarding import. A git or local skill is **updatable** — Update all and Update re-acquire it from its source — and a local skill's folder is never touched by any operation. An imported skill has no external source: the central copy is its source of truth, it is never a member of an Update all batch and offers no Update, and the harness it was found in is kept as display-only history — never a path the app reads from or writes to (`docs/adr/0003-imported-skill-has-no-external-source.md`). Provenance is a state, not a permanent property: a Re-point gives any skill a git or local source (an imported skill thereby stops being imported and becomes updatable; its found-in history is cleared), and Detach is the only road back to imported. A harness copy whose skill lock names an upstream is a git skill: a discovered upstream is a real source. "Is this skill updatable?" is one rule, owned by the backend and only presented by the frontend.
**Code name**: refreshable (`NOT_REFRESHABLE`); the stored spelling is the source type.
_Avoid_: refreshable (human-facing), source type (that's the stored spelling), origin, "local" for a skill taken over from a harness

**Unlocatable skill**:
A Managed skill the app can no longer find on disk, in one of two states judged from its recorded paths every time the library is listed and never stored: **source missing** — a local skill (see Provenance) whose folder is gone — or **central copy missing** — its central copy is gone, so every harness's link for it is dangling. A skill whose folder and central copy are both gone is source missing, because that repair fixes both. Update all skips it and says so; a Restore is an Update. Its repairs: **Re-point** (see its own entry — the source-missing repair opens Change source with a local folder preselected, but any target is allowed), **Detach** (it becomes imported — the central copy is its truth from now on, so it is offered only while that copy exists), **Restore** (re-acquire a git or local skill from its source and rebuild the central copy), or Remove.
**Code name**: `source_missing` / `central_missing`.
_Avoid_: stale row, broken skill, orphan, missing skill (which path is missing is the whole point)

**Re-point**:
Replacing a Managed skill's source in place while preserving its global and project Deployments — one operation over one target, offered to every Managed skill whatever its Provenance (the card's **Change source…**). Two targets: a **local folder** outside every harness's skills directory (validated the way Add validates one — present, holds a `SKILL.md`, not a harness's copy, which is Onboarding import's door), or a **GitHub URL** (full repository or skill URL: HTTP, HTTPS or schemeless `github.com/…`, tree/blob links and a `.git` suffix, but not `owner/repo` shorthand). The new source is carried only in the acquired Update request — a folder target is copied, a git target acquired first — and the single-skill Update settles source and bytes together through finalize, replaying Edits and propagating to the Deployments that exist, so a Re-point never leaves the skill in a Deployment state a plain Update would change; a refused target or failed acquisition/finalize changes nothing. On settle the record's Provenance follows the target: a local target records `local`, the folder, no subpath and no revision; a git target records `git` with the subpath and revision acquisition resolved, as Add does; both clear the imported found-in history, so a re-pointed imported skill is imported no longer. Becoming imported is Detach, not a Re-point.
**Code name**: `core/repoint.rs`, `repoint_skill_source`, `RepointTarget::{Git, Local}`.
_Avoid_: re-add, source migration, git Re-point / local Re-point as two operations (they are two targets of one)

**Artifact removal**:
Taking a Deployment off disk and settling its row, planned by scope and executed once with one presence rule and one failure rule: a row whose artifact could not be removed is kept with status `error` (never deleted blind) so the failure stays observable; rows are deleted only on successful removal (`docs/adr/0002-keep-row-with-error-on-failed-artifact-removal.md`). The scopes are `Skill` (everything about one Managed skill, including its central copy and record), `SkillGlobal`, `SkillTool`, `Project`, `ProjectTool`, `ProjectSkill` (one skill's every Deployment in one Project — the bulk Remove from all), `ProjectSkillTool` and `EveryGlobalTarget`; a scope carries the roots its own planning needs (only `SkillTool` needs the operator's home — every other scope resolves paths from stored rows). `SkillTool` plans every row describing the Deployment folder its key names (pre-1.4.0 sibling rows included) and uses that home to ask whether the folder is still locatable; when the registry can no longer locate it (no harness sharing it is detected) it plans those rows by their **own recorded** `target_path`, fenced by the registry's deletion rule — deriving a path for an absent harness stays forbidden, removing the artifact we recorded does not. A row that owns no artifact (never deployed, whatever its status — see Disabled) is set aside by planning as a marker: no path is removed for it; when the whole scope succeeded the delete settlement clears the row alone (`cleared_markers` in the report) and the Disable settlement records it `disabled`. Callers apply their own final policy by reading the report; Disable is the one caller that settles a successful removal by keeping the row `disabled` instead of deleting it (see Disabled). Every removal — here, a deploy's overwrite, Import's originals and the git cache — is **admitted by physical identity**: the path's parent resolved through every link or junction, a leaf link never (any other leaf to its real spelling — on a case-insensitive volume `ALPHA` is `alpha`), compared by identity rather than spelling, and the entry removed where it physically is; one that is, lies inside or contains the configured central repo (its whole subtree — staging folders and unrecorded copies included) or a recorded central copy, contains a folder holding copies (a harness dir linked onto the library), or lies outside the folder it claims (the git cache must be reached from the app cache dir through real folders), is refused (`REMOVAL_REFUSED`) and nothing is touched; a directory that is itself a mount point (on another device than its folder, or listed in the Linux mount table) is refused, and one is also walked beneath (never through a link) before anything in it goes, and refused when the recursion would cross a mount — a folder inside it with the identity of the library, of the entry or of a folder around either (a bind mount), one on another device, or (Linux) any mount point the kernel lists below it; a Linux mount table that cannot be read or parsed refuses the directory (fail closed); a git cache that overlaps the library is not cleaned at all (`core/removal_fence.rs`, BACKLOG #107). Admission is a check before the removal, not atomic with it: a process swapping a folder for a link, or mounting one inside the entry, in between is out of scope. Delete's own central copy is the one removal of central bytes, authorised by the skill's record.
**Code name**: `core/artifact_removal.rs`; the scopes keep `Tool` in their names.
_Avoid_: cleanup, unsync, unassign — in new names (Disable and Remove from project are the operator actions that plan a removal)

**Onboarding import**:
Adopting a skill that already exists in a harness's skills directory as a Managed skill (`core/onboarding_import.rs`).
- **Choice.** The operator picks one variant per name-group — for a consistent group the default is a real directory rather than a link to one, so a link is never what gets copied — and the chosen variant is finalized with `imported` Provenance (the harness it came from is history, not a source).
- **Scope.** The scan behind the review has a scope, resolved at the command seam from "only scan for existing skills within selected harnesses": on with a saved selection, exactly the selected harnesses (detected or not — the selection is the operator's word); otherwise every Detected harness. The import re-derives the plan under the same scope, so the originals it takes over or removes are exactly the ones the operator reviewed — which is why Review always fetches the plan afresh and the banner's plan reloads whenever the harness selection is saved: a plan from before a selection change describes nothing the import would act on.
- **Auto-deploy on.** The skill is deployed to the requested harnesses (a first deploy), and the import **takes over every byte-identical original in the group**: every harness holding a variant with the chosen variant's bytes is among the deployments whether or not the selection names it, and its copy — the import source, or a duplicate of it — is overwritten in place, so no harness keeps an untracked duplicate; the report names each harness included beyond the selection.
- **Auto-deploy off.** The operator decides per run whether the group's originals are removed or left in place (`remove_originals`, a required input); either way the report lists every original per harness — removed, left in place, or kept divergent.
- **Only identical bytes.** Which paths a group owns is re-read from a fresh onboarding plan, never taken from the selection. Under either policy only originals byte-identical to the finalized central copy are taken over or removed — a divergent sibling is left in place and reported, because sharing a name never proved it was the same skill — **unless the operator chose to remove the other copies** of that group (`remove_divergent`, the review's keep / remove choice for a Conflict): then every copy **known** to differ from the chosen one is removed first (within the harness dirs, once per physical path, reported for every harness naming it), so a harness the deploy reaches gets the link there. Deletion needs two successfully read identities that are unequal (`content_identity::compare` → `Different`): a copy whose comparison is unknown (an unreadable file, unreadable metadata) is left in place and reported `IDENTITY_UNKNOWN`, and the chosen source is never a candidate.
- **What the review compares.** *Match / Conflict* compare a group's copies with **each other**; a managed-name group compares each copy with **the Managed skill** (`same_as_managed`, the content identity rule). The plan carries each copy's description and newest modification time so a Conflict's copies can be told apart before choosing.
- **Manifest gate.** Only a directory whose `SKILL.md` parses to a valid manifest is a candidate; every other directory in a scanned harness's skills dir is listed apart with a typed reason (`missing_skill_md`, `invalid_frontmatter`, `missing_name`, `read_failed`) and is never importable.
- **Name already managed** (`name_is_managed`): never finalized. Without the operator's **Replace** it is skipped with a typed reason and nothing is touched; with Replace the Managed skill is deployed over the group's copies through the global deploy batch — an identical copy is replaced, a divergent one answers an askable `TARGET_EXISTS` row (the plan never authorizes a force-overwrite).
- One group's failure never stops the batch; per-group and per-target outcomes are report data.
_Avoid_: migration, adopt, absorb

**Project view**:
The fresh state of one Project that every project mutation returns — its row (with counts and aggregate status), its configured harnesses, and its reconciled Deployments, plus whether the reconcile pass actually ran. The project world applies it instead of re-reading after a mutation.
**Code name**: `ProjectViewDto { project, tools, assignments, reconciled }`.
_Avoid_: project state, project snapshot, refresh

**Mutation guard**:
The process-wide rule that Deployment mutations run one at a time (`core/mutation_guard.rs`): each operation entry point wraps its own body in `serialized`, the mutex is private to that module, and internals are unlocked `pub(crate)` seams — so an entry point never calls another entry point. A reader that must not queue behind a mutation uses `try_serialized`; the project listing's reconcile pass does, and reports `reconciled: false` when it skipped.
_Avoid_: sync mutex, lock, transaction

**Source resolution**:
Determining the repository, branch and skill subpath meant by a git source, including URLs where a slash could belong to either the branch or the path. A selected listing carries that answer into installation; a repaired recorded source becomes authoritative only when its acquired bytes are successfully finalized.
_Avoid_: URL splitting, branch guessing

**Git acquisition**:
Landing a skill's bytes from a git source in a directory, once, for every flow that needs it (`core/git_acquisition.rs`): given a parsed source and an intent it answers with the bytes, the revision and the strategy used. Ambiguous GitHub branch/path boundaries use the recorded subpath suffix first, otherwise the longest matching branch prefix, retaining the first-segment split when discovery fails or finds no match. Two adapters meet at one seam — the GitHub Contents API fast path (fetches the branch SHA first, so the recorded revision is the real commit) and the clone through the git cache (sparse when a subpath is known). A GitHub 404 or 403 is an answer for the operator, raised typed and never retried as a clone; other API failures fall back. Both adapters follow an **upstream in-repo symlink** at acquire time, every time (`core/repo_subpath.rs`): the link's target is resolved against the link's own directory within the repository root, chains are followed to a small bound, and a target that is absolute or leaves the repository is refused typed before anything at it is read. The recorded subpath stays the alias the operator chose — the resolved path is diagnostics only — so a maintainer re-pointing the alias is followed on the next Update; on the clone side the cache entry is widened by the target.
The API fast path follows an upstream link only at the leaf of the subpath; a link on an earlier path component is a typed not-found, never retried as a clone.
_Avoid_: download, fetch (that's the cache's job), clone (that's one of the two adapters)

**Notification**:
One user-visible outcome of an action — a kind (error, warning, success or info), a title, an optional message and the time it was raised. It belongs to a **Run**: a run raises one toast — its headline, or its first item — whose lifetime the reporter owns (an error stays until closed; a run whose outcome includes a failure stays until closed or until the next run's toast replaces it — a run's toast always replaces the previous run's; a skipped or to-review outcome stays 5 s; a declined one, a success or info flashes for 2 s), none while the panel is open (opening it takes every toast off the screen), and its notifications are kept together in the session's history, where errors and warnings count as unread until the operator opens the panel. The history is in memory for the session only; the backend log (see Open log folder in Settings) is partial forensics — backend-logged events only — not a record of Notifications.
_Avoid_: alert, log line (the backend log is a different thing), toast (that's the transient rendering of one)

**Run** (run report):
One operator action as the operator hears about it afterwards: its **composite headline** — the count line, "12 updated · 5 failed · 6 deployments failed · 4 skipped · 1 Edit to review", zero counts omitted — over the items its reports produced, each item carrying its own repair (Show, Retry, Overwrite…, Remove…, Change source…, Settings → GitHub). The outcome kinds a run counts are done, failed, deployments failed, skipped, declined, cancelled and to review; **declined** (the operator kept a folder) and **cancelled** (stopped before it ran) are never failures. A run is one group in the Notifications panel for the session.
**Code name**: the "deployments failed" kind is `targetsFailed`.
_Avoid_: job, log, summary toast, batch (that is the backend command)

**Header progress**:
Where a running action shows itself, and the only place: what it is doing in words, done/total, and Cancel only while the operation can still stop — in the area's header, or at the foot of the dialog whose action it is. The app stays browsable while it runs; a second mutation is refused with the reason, never queued.
_Avoid_: loading overlay, spinner, busy state

**Project deployment status**:
The precedence fold of a project's Deployment statuses shown on the project list: failed/copy missing > out of date copy > not deployed yet > deployed, "no deployments" when the project has none.
**Code name**: `ProjectSyncStatus` (`error`/`missing` > `stale` > `pending` > `synced`, `none`), `aggregate`.
_Avoid_: project sync status, project health, overall status

**Project deploy report**:
The per-Deployment outcome list every project-scope deploy answers with — Deploy to project for one cell (a batch of one) or a bulk Deploy to all, and Re-deploy of one or every project. Each item names the Deployment row when one exists, the skill, the harness, and its outcome: deployed / already deployed / skipped (disabled) / declined / failed with its error; a Project folder missing is one skipped project, not an item per Deployment. A deploy failure inside a freshly created row is reported *with* that row's id: the row is kept with status `error` (the red cell) and the operator hears about it. Failures are typed `CommandError` values classified where the row settles; counters are derived by the frontend fold, never carried. The removal side (taking a cell off, bulk Remove from all, dropping a harness) answers with an Artifact removal report instead.
**Code name**: Project sync report — `ProjectSyncReport { items, skipped_projects }`, item statuses `synced` / `already_assigned` / `skipped { disabled }` / `declined` / `failed { error }`.
_Avoid_: project sync report, resync summary, bulk assign errors, sync counts

**Project folder missing**:
A registered Project whose root folder no longer exists. Every project deploy skips it whole — reported once as a skipped project, its Deployment rows untouched — and never recreates the folder or anything in it; removal, Remove from project and Disable still work on its rows. Its repairs are Update path and Remove project.
**Code name**: `folder_missing`, `project_ops::project_folder_exists`.
_Avoid_: broken project, orphan project, stale path

**World**:
One area of app state and the actions on it, owned by one hook (the skills world, the project world, the settings world); the app composes the worlds and passes their state down.
_Avoid_: store, slice, context

**Visual world**:
The single visual language every surface is drawn in — palette, type, spacing, radii and motion — expressed entirely through Design tokens and Primitives, in both Themes (`docs/adr/0006-styling-architecture.md`).
_Avoid_: world (unqualified — that is a state area), skin, style, design system

**Theme**:
The resolved rendering of the Visual world, light or dark; the operator's preference may also be system, which the settings world resolves. Dark is designed first; light is derived from it.
_Avoid_: mode, colour scheme, world

**Design token**:
A named design value (a colour role, size, radius, duration) with one value per Theme; the only way a surface expresses the Visual world.
_Avoid_: token (unqualified — the GitHub token is a credential), CSS variable, hard-coded colour

**Primitive**:
A reusable interface building block (button, field, dialog, menu, checkbox, switch, tabs, chip, …) whose `variant`, `tone` and `size` are its entire styling interface, and which owns its focus, disabled and pending states, motion and accessibility wiring.
_Avoid_: component (too broad), widget, shadcn component

**Inspector**:
The focused Managed skill shown beside My Skills (a sheet on a narrow window): its Skill health with inline repairs, its Deployments, source, Edits, project Deployments and files. Focusing a skill chooses what the inspector shows; it is addressable (`#/skills/<id>`) but it is not a page you navigate to.
_Avoid_: detail view, skill page, drawer

**Selection**:
The items checked on one surface — My Skills, the project matrix, Explore, a picker each own theirs; a selection never travels between surfaces and never enters the address. Distinct from the focused subject (the one item the Inspector shows).
_Avoid_: checked items, multi-select, focus

**Bulk rail**:
The action bar My Skills shows while it has a Selection, acting on it. Every verb over several skills states its counts — skills, harnesses, global Deployments, project Deployments — and asks before it runs.
_Avoid_: toolbar, batch menu, action bar
