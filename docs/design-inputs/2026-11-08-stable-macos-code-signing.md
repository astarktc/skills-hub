# Stable macOS code signing for release builds (BACKLOG #120)

*Brief from Quartermaster (Plane QM-352, policy QM ADR-0016), 2026-11-08. The operator approved it. Re-verify
each claim against the repo before acting on it.*

## Why

Every Skills Hub update makes macOS ask for its permissions again. The release `.app` is ad-hoc signed:
on the MBP, `codesign -d -r- "/Applications/Skills Hub.app"` gives `designated => cdhash H"…"`, with
`Signature=adhoc`. macOS records privacy (TCC) grants and Keychain ACLs against the designated
requirement. A cdhash requirement changes with every build, so each release is a new program to macOS.
Signed with a stable identity, the requirement becomes
`identifier "com.skillshub.app" and certificate root = H"<cert sha1>"`, and it survives every release.
A self-signed certificate is enough: codesign, TCC and the Keychain need no trust, and updater-installed
apps are not quarantined. A paid Developer ID is out of scope.

The Muesli and T3 Code forks already work this way. They are built on the MBP and signed by the
self-signed identity `Muesli QM Local Signing` (procedure: `~/Projects/forks/t3code/fork-infra/sign-mac.sh`
and its README section "Code signing"; recipe: `~/Projects/forks/muesli/fork-infra/README.md`
§ "Signing identity").

## Decision already made (QM ADR-0016)

Skills Hub is built in GitHub Actions, so it gets **its own identity**, held only in this repo's secrets
plus the operator's LastPass backup. **Do not reuse the MBP identity.** A leaked CI secret must not be able
to sign a fake T3 Code or Muesli that inherits their microphone, screen and Keychain grants.

## What exists

- `.github/workflows/release.yml` already has an "Import Apple certificate (codesign)" step. It reads
  `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD` and `APPLE_SIGNING_IDENTITY`. None of them is set yet
  (confirm with `gh secret list`), so the macOS build logs "构建未签名版本" (building unsigned) and ships ad-hoc.
- When `APPLE_SIGNING_IDENTITY` is unset, the step infers an identity with `security find-identity -v`.
  `-v` lists valid identities only, so it skips an untrusted self-signed certificate. **Set
  `APPLE_SIGNING_IDENTITY` to the certificate's SHA-1.** codesign accepts an untrusted identity given by hash.
- `TAURI_SIGNING_PRIVATE_KEY` is the updater's minisign key. It is unrelated, so leave it alone.

## Work

1. **Create the identity** (CN `Skills Hub QM CI Signing`, O `Alex Stark (local)`, 10 years). Use the recipe
   in the Muesli README with the CN changed, including OpenSSL's `-legacy` flag for the `.p12`. **Never print
   the key, the `.p12` or its password**: pipe them straight into `gh secret set` and into files.
2. **Set the three secrets** with `gh secret set NAME < file`. `APPLE_CERTIFICATE` is the base64 of the
   `.p12`; `APPLE_CERTIFICATE_PASSWORD` is its password; `APPLE_SIGNING_IDENTITY` is the 40-hex SHA-1.
3. **Leave the backup for the operator**: put the `.p12` and a password file in `~/Downloads/skills-hub-signing/`
   (dir 700, files 600) and tell him. He stores them in a LastPass item named "Skills Hub QM CI Signing" and
   then deletes the folder. Do not import the key into the MBP keychain, and remove every other copy you made.
4. **Hardened runtime.** Tauri 2 signs with the hardened runtime by default (`bundle.macOS.hardenedRuntime`).
   The Muesli fork had to drop it, because code without a Team ID cannot load bundled non-system frameworks
   under library validation. Skills Hub bundles no frameworks, so it probably works as is. Prove it on a
   release candidate, and set `hardenedRuntime: false` only if the app fails to launch. Nothing is notarized
   (no `APPLE_ID`/`APPLE_API_KEY` secrets).
5. **Fail closed.** Change the macOS build so that a missing `APPLE_SIGNING_IDENTITY` fails the release
   instead of quietly shipping ad-hoc. One unsigned release would reset every user grant again.
6. **Verify** on the installed release (both arches build in CI, and the MBP installs aarch64):
   `codesign -d -r- "/Applications/Skills Hub.app"` must give the identifier + `certificate root = H"<sha1>"`
   requirement, and `codesign --verify --deep --strict` must pass. Acceptance: the first signed release
   re-prompts once (expected, the one-time switch), and the next release prompts for nothing.
7. **Document** the identity (name, SHA-1, where the backup lives, what losing it costs) where this repo
   documents releases. Give the operator the SHA-1 and the release tag so Quartermaster can close QM-352.

## Caveat found on T3 Code (2026-11-08): legacy Keychain items still prompt without a Team ID

The stable identity fixed TCC and the Keychain item's ACL on T3 Code, but a deploy still raised one
Keychain prompt. That item is a legacy file-keychain item (Electron's "t3code Safe Storage"). Besides the
ACL, such an item carries a **partition list**, and code without a Team ID gets the per-build partition ID
`cdhash:<hash>`. A self-signed certificate never yields a Team ID, so every new build is outside the list
and prompts once (`security dump-keychain -a` shows the item's `partition_id` entry growing by one cdhash per
build). So before or alongside step 1, check whether Skills Hub stores anything in the login Keychain: the
`keyring` crate, `security-framework`'s generic-password calls, or a Tauri keychain/stronghold plugin. If it
stores nothing, the self-signed identity is the whole fix. If it uses legacy items, ask the operator whether
Quartermaster's QM-352 decision moved the apps to an Apple-issued certificate with a Team ID, and use the
same kind of identity.

## Cost and risk

- The first signed release costs one round of prompts on each Mac, then never again while the identity lives.
- Losing the identity (secrets deleted and no backup) means a new hash, which pays that round again.
- Windows is unaffected.
