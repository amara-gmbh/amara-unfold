# amara Unfold transfer package

Start here after copying this package to another Mac.

amara Unfold is a local, offline archive utility. It converts supported raw
personal-data exports into readable, user-owned Markdown and builds a derived
timeline over those files. It does not require an amara account, cloud service,
database server, Node.js, or Rust to run the supplied binaries.

## Package contents

- `Amara Unfold.app.zip` — macOS desktop application.
- `amara-unfold` — command-line extractor, validator, and timeline builder.
- `LLM-INSTRUCTIONS.md` — exact operating contract for an assisting LLM.
- `COMPONENTS.md` — architecture, dependencies, formats, and limitations.
- `HOW-TO-USE.md` — end-user workflow.
- `CLAUDE-LOCAL-APP-APPROVAL.md` — independent local-app UAT sign-off sheet.
- `LOCAL-APP-UAT-STATUS.txt` and `.json` — package-time local-app UAT status
  snapshot without event bodies.
- `PRIVACY.md` — security and local-data boundaries.
- [`open-adapters.md`](open-adapters.md) — supported raw-export contracts.
- `BUILD-INFO.txt` and `manifest.json` — facts about this build.
- `CHECKSUMS-SHA256.txt` — integrity values for every transferred file.
- `LICENSE` — GNU AGPL v3 or later.

No personal archive, extracted Markdown, SQLite database, password, token, or
secret belongs in this package.

## Compatibility

This package is currently for **Apple Silicon (`arm64`) Macs**. The app is an
unsigned development build. It uses only macOS system frameworks at runtime,
including AppKit and WebKit. The CLI is also an `arm64` Mach-O binary.

Check the destination Mac before opening anything:

```sh
uname -m
sw_vers
```

`uname -m` must report `arm64`. A future Intel or universal build must be
packaged separately and labelled accordingly.

## Verify and open

Run these commands from the unzipped transfer directory:

```sh
shasum -a 256 -c CHECKSUMS-SHA256.txt
ditto -x -k "Amara Unfold.app.zip" .
chmod +x amara-unfold
./amara-unfold adapters
```

Every checksum must report `OK`. Do not bypass macOS security warnings for a
package whose checksum does not match.

The app is not Developer ID signed or notarized. First try Control-clicking
`Amara Unfold.app`, choosing **Open**, and confirming once. If the package was
quarantined during transfer, an operator may remove quarantine only after the
checksums have been verified:

```sh
xattr -dr com.apple.quarantine "Amara Unfold.app"
open "Amara Unfold.app"
```

## First-use proof

1. Open the app and confirm Home, Scan, Memories, and Settings appear.
2. Open Settings and confirm network access and telemetry both report off.
3. Use a copied or synthetic sample folder first.
4. Select a separate empty output folder.
5. Confirm the source files remain unchanged.
6. Confirm the output contains service packages and
   `Amara-Unfold-Timeline/index.md`.

For local app intake, prefer narrow exports first: Apple Mail
**Mailbox › Export Mailbox…** followed by **Use found Mail export** or
**Choose Mail export** in Unfold, Apple Music local-library metadata, and
ordinary exported photo/video folders. Do not grant broad macOS permissions
such as Full Disk Access for the supported Apple Mail route; exported `.mbox`
packages are ordinary local folders.

The Mail card’s **Prepare Mail export** helper creates
`~/Desktop/Amara Unfold Mail Exports`, opens that folder, and opens Mail. It is
only a save-target helper; it does not read Mail or start extraction. It writes
a short README into the prepared folder, shows the waiting folder in the Mail
card, and auto-checks briefly after this explicit action so **Export found** can
appear when the `.mbox` is saved.

For source-tree UAT, `npm run uat:mail-export:watch` can be left running while a
tester creates the Mail export. It repeats only the no-content preflight while
waiting and starts extraction only after a real `.mbox` export is present.

The local app index is normally stored at:

```text
~/Library/Application Support/com.amaramemory.unfold/amara-unfold.db
```

That database contains private local paths and metadata. It is created on the
destination Mac and is deliberately not included in this transfer package.

## Source status for testers

Treat this as the package promise:

- Working primary paths: normal photo/video folders, Apple Photos exports,
  Apple Mail `.mbox` exports, Apple Music local-library export action, Google
  Takeout routes, Airbnb JSON, banking CSV/XLSX, structured diary DOCX, and
  Google Maps/places exports.
- Working but explain carefully: Apple Calendar `.ics` exports, Apple Contacts
  `.vcf` exports and the secondary Apple Music TSV route. Existing photo-event
  Markdown is recognized as already-open Markdown; this build does not offer an
  extraction command for it.
- Not available in this build: Outlook PST, direct Immich-library intake,
  Apple Messages / iMessage, Apple Notes, and Apple Journal. Do not present
  these as working or "coming soon"; this build does not read those sources.

## Binary-only boundary

The supplied binaries are enough to run the current functionality. They are
not enough to modify, rebuild, audit, or port the application. For code changes,
obtain the matching amara Unfold source tree and follow the source-build section
in `COMPONENTS.md`.

Read `LLM-INSTRUCTIONS.md` before allowing an LLM to operate the package.
