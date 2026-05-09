---
status: diagnosed
trigger: "macos-damaged-app-signing: Skills Hub v1.1.0 macOS .dmg produces 'app is damaged' error due to signing misconfiguration"
created: 2026-04-10T00:00:00Z
updated: 2026-04-10T00:00:00Z
---

## Current Focus

hypothesis: The "damaged" error is the standard macOS Gatekeeper quarantine behavior for unsigned apps downloaded from the internet, NOT a broken/partial code signature. The app was built genuinely unsigned (no Apple codesign applied). The workflow's Apple signing infrastructure is dead code inherited from upstream that has no effect since this fork has no APPLE_CERTIFICATE secret configured. The fix is (a) removing the dead signing steps from the workflow and (b) documenting the xattr workaround for users -- OR adding a --no-sign flag to the macOS Tauri build to ensure Tauri's bundler doesn't apply an ad-hoc signature.
test: Verify whether Tauri 2 bundler applies ad-hoc codesign automatically on macOS even when APPLE_SIGNING_IDENTITY is unset
expecting: If Tauri 2 does ad-hoc sign, the "damaged" error is from a broken ad-hoc sig on the DMG contents after extraction. If no signing at all, it is purely quarantine.
next_action: Return diagnosis -- enough evidence to characterize the root cause and fix directions

## Symptoms

expected: App opens normally from Applications folder after DMG install
actual: "Skills Hub.app is damaged and can't be opened" -- macOS Gatekeeper code signature validation failure
errors: macOS Gatekeeper code signature validation failure
reproduction: Download v1.1.0 DMG, drag app to Applications, try to open
started: v1.1.0 release -- CI workflow showed steps attempting Apple signing

## Eliminated

- hypothesis: Apple signing secrets (APPLE_CERTIFICATE) are configured and produce a broken signature
  evidence: CI logs show APPLE_CERTIFICATE is empty, "Import Apple certificate" step skipped with "未配置 APPLE_CERTIFICATE, 跳过 codesign 导入". APPLE_SIGNING_IDENTITY_INPUT is also empty. Build step confirms "未配置 APPLE_SIGNING_IDENTITY, 构建未签名版本" and runs `unset APPLE_SIGNING_IDENTITY`. Verify codesign step also skipped.
  timestamp: 2026-04-10

- hypothesis: The workflow actively invoked codesign with wrong/missing certificate
  evidence: Grep of full CI logs shows zero instances of "Running Command" codesign. The Tauri bundler output only shows "Bundling Skills Hub.app", "Bundling .dmg", "Finished 2 bundles", "Finished 1 updater signature" -- no signing messages.
  timestamp: 2026-04-10

- hypothesis: qufei1993 upstream-specific identifiers are embedded in the distributed binary
  evidence: No qufei1993 references found in any config file. tauri.conf.json identifier is "com.skillshub.app" (generic). Updater endpoint already points to astarktc/skills-hub. Updater pubkey was already replaced in commit b8a172c.
  timestamp: 2026-04-10

## Evidence

- timestamp: 2026-04-10
  checked: .github/workflows/release.yml -- Apple signing steps
  found: The workflow has a full Apple certificate import step (lines 145-194) that imports a .p12 cert, creates a keychain, and exports APPLE_SIGNING_IDENTITY. However, it gracefully skips when APPLE_CERTIFICATE secret is empty (line 155-158). The macOS build step (lines 196-213) also checks APPLE_SIGNING_IDENTITY and unsets it if empty (lines 205-209).
  implication: The signing infrastructure is dead code in this fork -- it runs but does nothing because the secrets are not configured.

- timestamp: 2026-04-10
  checked: CI run 24225621164 (v1.1.0 release) -- actual log output for aarch64-apple-darwin job
  found: (1) TAURI_SIGNING_PRIVATE_KEY IS configured (shown as \*\*\* in logs -- secret exists for updater signing). (2) APPLE_CERTIFICATE is empty. (3) APPLE_SIGNING_IDENTITY_INPUT is empty. (4) "Import Apple certificate" step printed "未配置 APPLE_CERTIFICATE, 跳过 codesign 导入" and exited 0. (5) Build step printed "未配置 APPLE_SIGNING_IDENTITY, 构建未签名版本" and ran `unset APPLE_SIGNING_IDENTITY`. (6) "Verify codesign" step printed "未配置 APPLE_CERTIFICATE, 跳过验签" and exited 0.
  implication: No Apple code signing was performed. The app is genuinely unsigned.

- timestamp: 2026-04-10
  checked: Tauri bundler output during build
  found: Bundler output: "Bundling Skills Hub.app", "Bundling Skills Hub_1.1.0_aarch64.dmg", "Bundling .app.tar.gz", "Finished 2 bundles", "Finished 1 updater signature". No "Signing" messages. Zero "Running Command" codesign invocations.
  implication: Tauri 2 bundler did not invoke codesign during the build. However, Tauri 2 is known to apply an ad-hoc signature (`codesign --force --deep --sign -`) automatically when APPLE_SIGNING_IDENTITY is not set. This happens silently and may not appear in the log output at this verbosity level.

- timestamp: 2026-04-10
  checked: src-tauri/tauri.conf.json
  found: No macOS-specific signing config. No signingIdentity, no entitlements, no team ID. Bundle config is minimal: active=true, targets="all", createUpdaterArtifacts=true. Identifier is "com.skillshub.app".
  implication: Tauri config does not request signing. Any signing behavior comes from environment variables or Tauri's default bundler behavior.

- timestamp: 2026-04-10
  checked: No .entitlements files exist in the project
  found: Glob for \*.entitlements returned empty
  implication: No custom entitlements are being embedded

- timestamp: 2026-04-10
  checked: Git diff between fork and upstream release.yml
  found: Fork adds Linux build, fixes codesign verification to macOS-only (was running on all platforms), adds changelog extraction fallback, normalizes semver. The Apple signing steps are UNCHANGED from upstream.
  implication: Signing infrastructure was inherited wholesale from upstream author (qufei1993) who has the Apple Developer certificate. This fork never customized or removed it.

## Resolution

root_cause: The "Skills Hub.app is damaged" error is caused by macOS Gatekeeper quarantine on an unsigned (or ad-hoc signed) app downloaded from the internet. The v1.1.0 CI build produced a macOS .dmg with NO Apple Developer code signature because: (1) this fork (astarktc/skills-hub) does not have APPLE_CERTIFICATE, APPLE_CERTIFICATE_PASSWORD, APPLE_SIGNING_IDENTITY, or KEYCHAIN_PASSWORD secrets configured; (2) the release workflow correctly skips the certificate import and codesign verification steps when these are absent; (3) but the resulting app bundle either has no signature at all or only an ad-hoc signature from Tauri 2's bundler default behavior. Either way, macOS Gatekeeper flags apps downloaded from the internet that lack a valid Developer ID signature, and the com.apple.quarantine xattr triggers the "damaged" dialog. This is EXPECTED behavior for unsigned macOS apps -- it is not a broken build.

fix:
verification:
files_changed: []
