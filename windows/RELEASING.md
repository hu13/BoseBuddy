# Releasing QCBuddy

## Once: the update signing key

Installed copies only accept updates signed with the key whose public half
is in `app/src-tauri/tauri.conf.json` (`plugins.updater.pubkey`).

- The key pair was made with `npx tauri signer generate` and is kept outside
  the repo: `%USERPROFILE%\.qcbuddy-updater\` (`qcbuddy-updater.key`,
  `.key.pub`, `password.txt`). **Keep an offline copy (e.g. a password
  manager).** If the private key is lost, installed copies reject every
  future update and users have to reinstall by hand once.
- The release workflow reads it from two repository secrets:

  ```powershell
  Get-Content -Raw "$env:USERPROFILE\.qcbuddy-updater\qcbuddy-updater.key" | gh secret set TAURI_SIGNING_PRIVATE_KEY --repo hu13/BoseBuddy
  Get-Content -Raw "$env:USERPROFILE\.qcbuddy-updater\password.txt" | gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --repo hu13/BoseBuddy
  ```

## Each release

1. Bump the version in `app/src-tauri/tauri.conf.json`, `app/package.json`
   (`npm version x.y.z --no-git-tag-version`), `app/src-tauri/Cargo.toml` and
   `core/Cargo.toml`; add a `## X.Y.Z` section to `CHANGELOG.md` with only
   what changed since the last release. Open a pull request from a
   branch starting with `release/qcbuddy` (for example,
   `release/qcbuddy-v0.1.3`) in this repository into `main`.
   The release page shows that section plus a short install footer
   (`app/scripts/release-notes.mjs`); the release fails early if the section
   is missing.
2. Merge the PR. `qcbuddy-release.yml` checks the versions and the
   changelog, then releases each platform independently, side by side:
   - **Windows** (tests, signed installer, MSIX): tags the exact merge
     commit as `qcbuddy-vX.Y.Z` and creates a **draft** pre-release with
     `QCBuddy_X.Y.Z_x64-setup.exe` and its `.sig`, `latest-windows.json`
     (the update feed entry for this version) and `SHA256SUMS.txt`.
   - **macOS** (tests, universal DMG): tags the same commit as
     `qcbuddy-macos-vX.Y.Z` and creates its own draft with
     `QCBuddy_X.Y.Z_universal.dmg`, a signed `.app.tar.gz` updater archive,
     its `.sig`, `latest-darwin.json`, and `SHA256SUMS.txt` (see macOS DMG).
   If one platform fails, the other still gets its draft; fix the cause and
   **Re-run failed jobs**.
   The tag version comes from `tauri.conf.json`, not the branch name.
   Closed-but-unmerged PRs and other source branches do not release.
   An existing tag is never moved: it must point to the same commit or
   the workflow fails. No extra token is needed; tagging and release
   creation happen in the same run.
3. Download the installer from the draft and check it (`HARDWARE-CHECKLIST.md`,
   install and update rows).
4. **Publish** the draft. `windows-publish-update.yml` copies its
   platform feed (`latest-windows.json` or `latest-darwin.json`) to the permanent `qcbuddy-latest` release; installed
   copies are offered the update at their next check (at most 6 hours).
## Manual release fallback
Tag the intended commit with
`git tag -a qcbuddy-vX.Y.Z -m "QCBuddy X.Y.Z (Windows)"` and push that
tag. Tag pushes still run the same release workflow.

## The update feed

- Installed copies read
  `https://github.com/hu13/BoseBuddy/releases/download/qcbuddy-latest/latest-{{target}}.json`;
  Windows reads `latest-windows.json`, macOS reads `latest-darwin.json`.
  Each platform can release on its own schedule.
- `qcbuddy-latest` is a pre-release that is never marked "Latest" (the
  macOS app's releases are). Don't delete it.
- **Roll back:** run "Windows update feed" by hand (Actions → Run workflow)
  with an older tag. Copies already on the newer version stay there (the
  updater only offers newer versions), but nobody else is offered it.
- Drafts are never offered: the feed changes only when a release is
  published.
- **Check by hand:** `QCBuddy.exe --check-for-updates` (the running copy
  checks and shows the result in the flyout), or right-click the tray icon →
  **Check for updates**.

## Microsoft Store

The Store copy is the same build in an MSIX package (`app/msix/`,
`app/scripts/package-msix.ps1`), tray app only: the Store rejects a package
with a hidden app (`AppListEntry="none"`) unless Microsoft grants a
HeadlessAppBypass waiver, so the CLI (`qcb`) is in the GitHub installer only.
The Store signs and updates it: the Store copy never reads the update feed,
and **Check for updates** opens its Store page instead (`core/src/package.rs`).

**Once:**

- The listing text, privacy policy URL and other fields are in
  [`STORE-LISTING.md`](STORE-LISTING.md); the policy is
  [`PRIVACY.md`](PRIVACY.md). Update them when features or data handling
  change.
- In Partner Center, the product must be an **MSIX or PWA app** (an "EXE or
  MSI app" product only takes an installer URL; its type can't be changed).
- Copy QCBuddy → **Product identity** into repository variables:

  ```sh
  gh variable set MSIX_IDENTITY_NAME --repo hu13/BoseBuddy --body "<Package/Identity/Name>"
  gh variable set MSIX_PUBLISHER --repo hu13/BoseBuddy --body "<Package/Identity/Publisher, CN=...>"
  gh variable set MSIX_PUBLISHER_DISPLAY_NAME --repo hu13/BoseBuddy --body "<Package/Properties/PublisherDisplayName>"
  ```

  Without them the release still builds, with a placeholder MSIX for local
  testing that the Store rejects.

**Between releases:** Actions → **Windows MSIX** → Run workflow builds a
`.msix` from any branch, without tagging or releasing. Or build one on the
test PC: `make msix` from the repo root (or `npm run msix` in `app`) builds
the app and packages it with QCBuddy's identity, in
`app\src-tauri\target\msix`. `make msix-install` also installs it, unsigned
(Developer Mode on). Double-clicking the unsigned `.msix` doesn't work
(0x800B010A); upload that same unsigned file to Partner Center.

**Each release**, after step 2 above:

1. Download `QCBuddy_X.Y.Z.0_x64.msix` from the release run's artifacts
   (Actions → QCBuddy release → `qcbuddy-msix-X.Y.Z`).
2. Check it on the test PC (`HARDWARE-CHECKLIST.md`) by installing the
   unsigned package from the run: unzip the `.msix` (it's a zip) and, with
   Developer Mode on, `Add-AppxPackage -Register <folder>\AppxManifest.xml`.
   Uninstall the GitHub copy first, or both run side by side with separate
   settings. Optionally run the Windows App Certification Kit on the `.msix`.
3. Partner Center → QCBuddy → new submission → **Packages**: upload the
   `.msix`. Partner Center warns that `runFullTrust` is a restricted
   capability; under **Submission options → Restricted capabilities**, paste
   the justification from [`STORE-LISTING.md`](STORE-LISTING.md). Then
   submit. Certification usually takes 1–3 business days.

The Store wants four-part versions ending in `.0` (`X.Y.Z.0`), each higher
than the last; a rejected submission needs a version bump.

**Package locations:** the Store copy's log and presets are under
`%LOCALAPPDATA%\Packages\<package family name>\LocalCache\Local\QC Buddy`
(Windows redirects the app's `%LOCALAPPDATA%\QC Buddy`).

## macOS DMG

`macos-release.yml` builds the Tauri app for macOS as a universal (Apple
silicon and Intel) `QCBuddy_X.Y.Z_universal.dmg`. `qcbuddy-release.yml` runs
it alongside the Windows build; it tags the commit `qcbuddy-macos-vX.Y.Z`
and creates a separate draft, "QCBuddy X.Y.Z (macOS)", which is checked and
published on its own. Publishing it updates only `latest-darwin.json`; the Windows feed stays
unchanged. The shared `windows-publish-update.yml` workflow handles both
platforms and rejects drafts, mismatched versions, and missing updater assets. Actions →
**macOS release** → Run workflow builds a test DMG from any branch (artifact
only), or, with a `qcbuddy-macos-vX.Y.Z` tag, rebuilds that tag and replaces
the files on its release.

Starting with 0.1.6, releases also include a universal `.app.tar.gz` and its
Tauri signature. `latest-darwin.json` points both `darwin-aarch64` and
`darwin-x86_64` to that archive. The app verifies the signature before
replacing itself and restarting. The DMG remains available for first installs.
Existing macOS builds with the updater enabled can use this feed too.

The macOS release job requires the same `TAURI_SIGNING_PRIVATE_KEY` and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secrets used for Windows. Updater signing
is separate from Apple code signing; never change the updater public key in
an already released app. Artifact-only test DMG builds do not require the key.

Publishing the draft promotes its feed; creating a draft does not. To roll
back the offered macOS version, run **QCBuddy update feeds** with a previously
published `qcbuddy-macos-vX.Y.Z` tag that contains `latest-darwin.json`.
Before publishing, test **Check for updates → Install and restart** from an
older installed app, verify the new version after restart and Bluetooth access,
and confirm the feed still serves the Windows entry independently.

**Signing (optional):** without these secrets the app is ad-hoc signed, and
Gatekeeper blocks its first launch until it's allowed under System Settings
→ Privacy & Security. With them, Tauri signs it with a Developer ID and
notarizes it:

```sh
base64 -i DeveloperID.p12 | gh secret set APPLE_CERTIFICATE --repo hu13/BoseBuddy
gh secret set APPLE_CERTIFICATE_PASSWORD --repo hu13/BoseBuddy      # the .p12's password
gh secret set APPLE_SIGNING_IDENTITY --repo hu13/BoseBuddy          # "Developer ID Application: Name (TEAMID)"
gh secret set APPLE_ID --repo hu13/BoseBuddy                        # notarization
gh secret set APPLE_PASSWORD --repo hu13/BoseBuddy                  # an app-specific password
gh secret set APPLE_TEAM_ID --repo hu13/BoseBuddy
```
