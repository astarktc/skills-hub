# macOS code signing

The macOS releases are signed with the operator's Apple Development certificate (BACKLOG #120, from 1.3.6). The
design input is `docs/design-inputs/2026-11-08-stable-macos-code-signing.md` (QM-352, QM ADR-0017).

## Why

An ad-hoc signature's designated requirement is the build's cdhash, so each release was a new program to macOS. Privacy
(TCC) grants and Keychain ACLs record the designated requirement, so every update asked for them again. The GitHub
token is a legacy login-keychain item (`keyring` crate, `core/credentials.rs`). Such an item also carries a
**partition list**, which admits code by Team ID, or by a per-build `cdhash:` for code without a Team ID. A self-signed
certificate has no Team ID, so only an Apple-issued certificate stops the Keychain prompt after each update.

## The identity

| | |
| --- | --- |
| Certificate | `Apple Development: <operator> (SGXP9K48V2)` of the operator's free Apple personal team **N66437Y9GD** (the team that signs T3 Code) |
| SHA-1 | `B76D3A5009BCC43DE6386CDB08B13D41EB72058D` |
| Valid to | **2027-11-08** |
| Issuer | Apple Worldwide Developer Relations CA **G3**, which `release.yml` downloads from Apple and checks against a pinned SHA-256 |
| Where | the MBP's login keychain (created by Xcode), and this repo's Actions secrets |
| Backup | none needed: Xcode re-issues a certificate of the same team at any time, and the requirement pins the team |

The secrets:

- `APPLE_CERTIFICATE`: the base64 `.p12`, holding the certificate and its private key.
- `APPLE_CERTIFICATE_PASSWORD`: the `.p12` password.
- `APPLE_SIGNING_IDENTITY`: the 40-hex SHA-1.

The build keychain's password is random per job. `TAURI_SIGNING_PRIVATE_KEY` is the updater's minisign key, which is
unrelated.

## The designated requirement

Every release app carries this requirement:

```
identifier "com.skillshub.app" and anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.1] exists and certificate leaf[subject.OU] = "N66437Y9GD"
```

It pins the team, not one certificate, so a renewed certificate resets nothing. Tauri's bundler can't pass a
requirement, so for the macOS build `release.yml` puts `scripts/macos-codesign/` first on `PATH`. Its `codesign` shim
adds `-r=` when the bundler signs the `.app`; every other call passes through unchanged. The team id and bundle id are
`MACOS_TEAM_ID` / `MACOS_BUNDLE_ID` in `release.yml`.

`release.yml` checks the following on both the built app and the app inside the updater archive:

- `codesign --verify --deep --strict` passes;
- `TeamIdentifier` is the team;
- `codesign -d -r-` gives the requirement above, which codesign prints with `/* exists */` and the OU unquoted.

The hardened runtime stays on (Tauri's default). The app bundles no non-system framework, so library validation has
nothing to refuse. Nothing is notarized.

**Fail closed.** A macOS release job fails in any of these cases:

- a secret is missing;
- the identity is not valid in the build keychain;
- the certificate is not the team's, or it has expired;
- the shim is not first on `PATH`;
- a check above fails.

One ad-hoc release would reset every user's grants. The job warns when the certificate expires within 30 days.

## Yearly renewal (before 2027-11-08)

1. Xcode → Settings → Accounts → (Personal Team) → Manage Certificates → **+** → Apple Development.
2. Export the new identity with its private key and update the three secrets: `gh secret set NAME < file`, never
   printing the key, the `.p12` or its password. Then delete every exported file.
3. Update the SHA-1 and expiry date in this file.

Nothing else changes, and no grant resets, because the requirement pins the team.

**Costs.** Switching to a different team (for example a paid Developer Program team) changes `MACOS_TEAM_ID`. Like the
switch from ad-hoc in 1.3.6, it costs one round of prompts on every Mac: privacy grants, plus one Keychain "Always
Allow" for the GitHub token.
