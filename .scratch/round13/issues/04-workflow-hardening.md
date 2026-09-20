# 04 Harden the four GitHub workflows

Status: open
Lane: B
Source: BACKLOG #31

In `.github/workflows/{ci,release,auto-tag,update-featured-skills}.yml`: pin every `uses:` to a full commit SHA with a `# vX.Y.Z` trailing comment (resolve SHAs with `gh api repos/<owner>/<repo>/git/ref/tags/<tag>` or `git ls-remote`; verify each resolves); `dtolnay/rust-toolchain@<sha>` needs `with: toolchain: stable`; add top-level `permissions: contents: read` where absent (keep existing job-level broader grants — release needs `contents: write`, auto-tag/featured need their write scopes); add `persist-credentials: false` to every checkout unless that job pushes with the default token (auto-tag, update-featured-skills push — leave those true and say so in a comment). Validate YAML with `npx -y @action-validator/cli` or `actionlint` if available; otherwise `node -e` yaml parse.

## Done when

All four workflows have SHA pins + permissions; ci.yml still green (parent verifies after push).

## Comments
