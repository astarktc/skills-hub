# 06 — `acquire` cannot be handed a listing-only intent (BACKLOG #15)

Status: done — aee54af
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

## Comments

- 2026-09-22 — done in `aee54af`. `GitSelection.subpath: &str`; `Resolved::listing(source, api)` (private to
  `git_acquisition`) replaces `Resolved::new(Selection(default))`; `Intent::Listing` deleted along with **two**
  runtime guards the type now makes unreachable — `resolve_subpath`'s "listing intent requires the candidate
  listing seam" bail and `install_git_selection_with`'s "install requires an explicit selection" `.context`.
  `cargo test --all` 647 (unchanged — type-level change), clippy clean, bindings untouched.
