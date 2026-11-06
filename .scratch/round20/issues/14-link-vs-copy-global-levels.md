# 14: Link vs copy at global scope and per folder

Status: ready-for-agent

**Blocked by:** 12.

**Spec:** `.scratch/round20/spec.md` § Wave B spec → § Link vs copy; grill Q21; user stories 42–46. Absorbs BACKLOG
**#84** (global half; the per-project level is 15's).

**What to build:** link stays the default. The operator can choose copy for global scope as a whole, and per folder
card (a harness's own folder, or a shared folder for all its readers). Each card shows its effective mode and where
that comes from. A copy-only harness shows link disabled with its reason. Changing a level moves the Deployments it
governs through the redeploy.

## Acceptance criteria

- [ ] **Placement resolves deploy mode:** folder override › scope setting › built-in link. The registry capability is a
      floor: a copy-only folder resolves to copy, and link is unavailable with its reason.
- [ ] **Copy-only demotion (Q21).** A copy-only harness (`supports_symlink: false`: Deep Agents Code) that would be
      served by a link-mode shared folder is served by its own folder. Setting `.agents` to copy serves it from `.agents`.
      The "Do not" rule holds: a group's capability is never the AND of its members.
- [ ] **Settings:**
  - global-scope mode and per-folder overrides are typed settings;
  - the all-projects mode exists here, applied in 15;
  - each is written through its own previewing command that redeploys in one guarded section.
- [ ] **UI:** a Link / Copy control on each card, plus the global-scope control, each with its origin. Confirmations
      state "would move N".
- [ ] **Sync stays name-blind.** `sync_engine` receives the resolved mode as a capability.
- [ ] **Tests:**
  - placement mode resolution, every level and the floor;
  - demotion and re-promotion of Deep Agents Code;
  - a mode change moves links ↔ copies through the redeploy (identical-or-ask);
  - frontend folds for origin and disabled reasons.
- [ ] **CONTEXT.md:** "Deploy mode" gains its levels and origin. "Harness capability" is updated.
- [ ] `npm run version:check && npm run check` is green.

## Notes

- Review: Astra and Sol in parallel. A link → copy switch replaces artifacts through the overwrite path (Q28).
