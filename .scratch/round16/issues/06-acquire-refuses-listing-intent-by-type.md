# 06 — `acquire` cannot be handed a listing-only intent (BACKLOG #15)

Status: ready-for-agent
Spec: `.scratch/round16/spec.md` — D8. Orchestrator ticket.

## Work

`core/git_acquisition.rs:88` `GitSelection { subpath: Option<&str>, resolution }` — `None` means "list
candidates" (`resolution.rs:62` maps it to `Intent::Listing`). Make `SkillIntent::Selection`'s subpath
non-optional (`&str`) so `acquire` cannot receive a listing; the listing path (`git_acquisition.rs:284`
`Resolved::new(source, SkillIntent::Selection(GitSelection::default()), api)`) constructs its own intent
(a private `Intent::Listing` or a separate `SkillIntent::Listing` variant that `acquire` rejects at the type
level by taking a narrower type — choose the smallest change that makes the misuse unrepresentable). No runtime
refusal, no new error variant. `installer.rs:404` (always has a subpath) adapts trivially. Tests compile-only
plus the existing suite.
