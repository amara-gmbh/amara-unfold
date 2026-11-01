# Build from source

The repository is sufficient to inspect, test, and build amara Unfold. A
compiled `.app` is a release artifact and is intentionally not committed to the
source tree.

## Pinned release environment

The 2026-07-19 development gate was verified with:

- macOS on Apple silicon and Xcode Command Line Tools;
- Rust/Cargo 1.97.0 from `rust-toolchain.toml`;
- Node.js 25.9.0 and npm 11.12.1;
- dependency graphs pinned by `Cargo.lock` and `package-lock.json`.

The security review candidate requires Node.js 22.13 or newer. `package.json`
declares the minimum supported Node.js version. Exact byte-for-byte reproducibility of a
signed/notarized Apple bundle is not yet claimed; source-to-function
reproducibility is the present release gate.

## Clean build

Choose a local, non-synced build directory before running these commands.
Do not install dependencies or build inside OneDrive, iCloud Drive, Dropbox,
or another file-sync root. `node_modules`, Cargo `target`, frontend `dist`,
and test output contain many generated files; `.gitignore` affects Git, not
the cloud client's sync policy. If source is stored in a synced folder, stage
a clean local copy without those generated directories and build that copy.
Keep source changes canonical and never copy generated output back to the
synced source tree.

```sh
# From the root of the reviewed source tree:
npm ci
npm run audit:public
npm run format:check
npm run build
npm test
npm run lint
npm run build:app:debug
```

The development bundle is created at
`target/debug/bundle/macos/Amara Unfold.app`. The configured ad-hoc signing
identity seals the complete bundle; the command then runs strict code-signature
verification and fails if the seal is invalid. Do not add `--no-sign`: a
linker-signed executable alone does not seal the bundle's resources or Info.plist.
It can be inspected and run locally but must not be represented as the
publisher's official Developer ID signed or notarized release.

## Local candidate and installation procedure

Keep a verified source recovery and a restorable copy of the currently installed
app before a local update. Build only in the non-synced workspace. The debug
command builds the renderer and native executable, seals the complete bundle
with the configured ad-hoc identity, and verifies that seal. A successful build
using existing dependencies and Cargo caches is not a clean-machine or fresh
dependency-install certification.

Copy the completed bundle to a separate local review directory and freeze it
before acceptance. Record the executable SHA-256 and a complete bundle file
inventory, not just the version label: separate local builds can retain the
same version/build number. Run `codesign --verify --deep --strict` on the frozen
copy and on a fresh restoration of its recovery ZIP. Do not repair or rebuild
that copy during independent review; a changed hash requires a new review.

Use normal native controls with synthetic exports and a separate empty output
folder. Verify recognition, explicit destination selection, extraction, balanced
quality, complete newly generated event text and both source/Markdown provenance.
In the macOS picker, **Command-Shift-G**, the visible path field, **Return** and
**Open** are sufficient; verify the folder name before confirming. If navigation
fails, keep the case blocked and resolve the visible picker interaction rather
than injecting application state or calling an extraction command behind the UI.

Close and verify the exact test instance after the bounded run. Preserve other
users' instances and unsaved work, and coordinate independent native review to
avoid sharing the same app surface concurrently. A retained review bundle needs
an owner and cleanup trigger.

Only a proven candidate proceeds to an authorized local installation. Verify
installed bundle parity and strict signature, then launch the installed path
and confirm the synthetic event's full readback and provenance. Archive the
final source and installed bundle with hashes, recovery checks and acceptance
limits. This closes only the tested workflows; the existing-route matrix still
governs remaining importer reliability work. Local installation is not public
distribution or notarization.

## Independent verification

On a clean second machine, record `rustc --version`, `node --version`, the exact
source revision, and SHA-256 of the resulting bundle archive. Run the source,
history, dependency, test, and lint gates before comparing behavior. Official
release archives must be signed with a Developer ID Application certificate,
notarized, stapled, checked with Gatekeeper, and published with checksums tied
to the source revision.

## Source package and release decisions — 2026-09-07

A prepared source ZIP is an inspection/build candidate, not a published release.
It excludes Git history, private continuity, raw data, generated exports and
compiled applications. Audit the final extracted payload, retain its SHA-256
and file inventory, and publish only through the documented release process. The
existing local mirror history is audited separately; a pass for configured
signatures is not a comprehensive secret audit or proof of a remote's contents.

Use the pinned Rust 1.97 toolchain for this snapshot. The workspace's older
rust-version declaration does not certify the complete resolved dependency
graph on that minimum compiler. A clean-machine minimum-version check has not
been performed for this candidate.

The earlier workbook security batch removed the unused direct quick-xml
dependency. The current `Cargo.lock` resolves both quick-xml `0.41.0` and
`0.42.0` through different dependency paths; it does not contain only one
quick-xml version. Historical scoped advisory checks do not establish the
current security status of the complete graph. Review the matching lockfile
with current advisory data rather than treating a version line as a fresh audit.

WDIO 9 components and their expectation peer are updated together; the Tauri
service's older exact webdriverio pin is constrained to 9.31.6 so the graph
contains only the fixed deepmerge-ts 8.0.2. A single existing native harness
launch check verifies the peer/service integration; broader importer acceptance
is separate. Unrelated frontend and production npm versions remain unchanged.

The current JavaScript requirement is Node.js `22.13` or newer. The checked-in
lockfile resolves `@puppeteer/browsers` `3.2.6`, glob `14.0.3`, and minimatch
`11.0.3` using the overrides in `package.json`. It contains no extract-zip
package, so the old extract-zip/Node-20 compatibility paragraph no longer
describes this source revision.

Some overrides cross upstream declared major-version ranges. Their interfaces
need the focused dependency-override checks and native-test verification
described below. Lockfile inspection is not a fresh npm advisory audit or a
claim that every dependency is vulnerability-free. Preserve npm peer checks; do
not use force or legacy peer resolution to conceal incompatibilities.

Version `2026.9.21` is a limited-feature GitHub release candidate. The public
source route is the canonical GitHub repository after its final privacy/history
audit and intentional visibility change. Source publication does not make an
ad-hoc macOS bundle an official downloadable binary.

For an official downloadable Mac application, use the organization's Developer ID
Application certificate and corresponding private key in the signing Mac's
Keychain, plus an owner-configured notarytool Keychain profile. Keep credentials
out of source and chat. Harden, sign, submit to Apple, staple the ticket and
verify Gatekeeper on the final archive before distribution. Source publication
does not require Apple distribution credentials. See
[Apple's Developer ID guidance](https://developer.apple.com/developer-id/) and
[notarization workflow](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow).

## Security review candidate — 1 November 2026

The local review candidate updates the ZIP and spreadsheet dependencies and
the JavaScript test tools. Its complete audit and compatibility checks are
still in progress. In particular, dependency overrides that cross major
versions need native-test verification; do not treat this candidate as a
finished release or as a blanket security certification.

The candidate uses security-motivated dependency overrides for the WebDriver
browser utilities, glob and minimatch. Run `npm run test:dependency-overrides`
with the full test suite to exercise the interfaces used by those tools. Some
overrides cross upstream declared major-version ranges; a passing check and
native E2E run reduce compatibility uncertainty, yet do not replace upstream
support or independent security review.
