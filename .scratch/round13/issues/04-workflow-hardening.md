# 04 Harden the four GitHub workflows

Status: done — pending parent commit
Lane: B
Source: BACKLOG #31

In `.github/workflows/{ci,release,auto-tag,update-featured-skills}.yml`: pin every `uses:` to a full commit SHA with a `# vX.Y.Z` trailing comment (resolve SHAs with `gh api repos/<owner>/<repo>/git/ref/tags/<tag>` or `git ls-remote`; verify each resolves); `dtolnay/rust-toolchain@<sha>` needs `with: toolchain: stable`; add top-level `permissions: contents: read` where absent (keep existing job-level broader grants — release needs `contents: write`, auto-tag/featured need their write scopes); add `persist-credentials: false` to every checkout unless that job pushes with the default token (auto-tag, update-featured-skills push — leave those true and say so in a comment). Validate YAML with `npx -y @action-validator/cli` or `actionlint` if available; otherwise `node -e` yaml parse.

## Done when

All four workflows have SHA pins + permissions; ci.yml still green (parent verifies after push).

## Comments

- 2026-09-16 (lane B child): Pinned every `uses:` in the four workflows to a commit SHA with a trailing version
  comment. Stayed within the majors already in use (checkout/setup-node/artifacts v4, rust-cache v2, gh-release v2)
  so CI behaviour is unchanged; SHAs are what each floating major tag resolved to today (annotated `v2` for
  rust-cache was peeled to its commit). `dtolnay/rust-toolchain@stable` is a *branch*, not a tag — pinned the master
  commit (= tag `v1`) and added `with: toolchain: stable`.

  | Action | SHA | Version |
  |---|---|---|
  | actions/checkout | `11d5960a326750d5838078e36cf38b85af677262` | v4.4.0 |
  | actions/setup-node | `49933ea5288caeca8642d1e84afbd3f7d6820020` | v4.4.0 |
  | dtolnay/rust-toolchain | `02cb101ec7c40f2c49e1d9714d64511d8e1b74de` | v1 (master, 2026-09-12) |
  | Swatinem/rust-cache | `6323deb102c322ba6fcbdcafc7e3dddab59af2b6` | v2.9.2 (peeled) |
  | actions/upload-artifact | `ea165f8d65b6e75b540449e92b4886f43607fa02` | v4.6.2 |
  | actions/download-artifact | `d3f86a106a0bac45b974a628896c90dbdf5c8093` | v4.3.0 |
  | softprops/action-gh-release | `3bb12739c298aeb8a4eeaf626c5b8d85266b0e65` | v2.6.2 |

  Permissions: ci.yml gained top-level `contents: read`; release.yml top-level lowered from `contents: write` to
  `contents: read` (the build matrix only uploads workflow artifacts; `assemble-updater-json` already carried a
  job-level `contents: write`, which the gh-release step uses). auto-tag.yml / update-featured-skills.yml keep their
  write grants (single-job workflows that push) with a comment saying why. `persist-credentials: false` on every
  checkout in ci.yml and release.yml; explicit `persist-credentials: true` + comment in auto-tag.yml and
  update-featured-skills.yml because those jobs `git push` with the default token.
  Evidence: `npx -y @action-validator/cli` reports all four files valid; python `yaml.safe_load` parses all four;
  `grep uses:` shows no floating tag left. Note: newer majors exist (checkout v7, setup-node v7, upload-artifact v7,
  download-artifact v8, gh-release v3) — bumping them is a separate, behaviour-changing ticket.

