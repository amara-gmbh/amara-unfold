# How to use amara Unfold

amara Unfold inspects folders on your computer and builds a private index. It
does not move originals or send anything to amara Memory.

## Why use amara Unfold?

A downloaded archive gives you data, but rarely gives you a usable memory of
your life. Years of dates, places, photos, messages, listening, travel, and
other traces can remain hidden inside technical folders and disconnected
formats. Unfold turns only the sources you choose into readable open files,
places them into a common timeline, and preserves the evidence behind every
record.

The result can help you rediscover what exists, understand which traces matter
to you, and conserve selected memories without depending on the company that
created the original export.

### Is this just another metadata extractor?

No. Extraction is one necessary step, not the finished product. A conventional
extractor commonly reads technical fields from one format and leaves you with a
dump to interpret yourself. Unfold normalizes different sources into the same
open event model, preserves readable content and provenance, validates quality,
and builds a shared timeline plus people/place/entity and connection indexes.
The result is designed to remain useful beyond Unfold—to you directly, to local
tools, and to future memory experiences.

## What can you do afterward?

1. Read, search, edit, and curate the Markdown directly.
2. Point a separate, locally running LLM or document tool you trust at selected
   files. Ask about periods, themes, journeys, interests, or changes. The
   richness can be surprising once formerly scattered sources are readable
   together. Verify interpretations against the cited records, and make sure
   the chosen tool does not upload or cloud-index the files.

amara Memory is planned and not available yet. This release contains no Memory
application or import bridge, and no Memory handoff can be performed from this
build. The open files remain useful independently. The present build also
contains no bundled LLM, cloud sync or amara account.

## Open the current macOS build

The initial build is an unsigned development application:

```text
target/debug/bundle/macos/Amara Unfold.app
```

Open it from Finder, or from the repository root:

```sh
open "target/debug/bundle/macos/Amara Unfold.app"
```

If macOS warns that it is unsigned, use **Control-click → Open** and confirm.
Signed distribution is intentionally a later release step.

## What works in this build

Use this section as the release promise. If a source is not listed here, treat
it as not available unless the app recognizes a matching exported folder/file.

### Working primary paths

| Source               | What the user chooses                  | Notes                                                                                                                                                                |
| -------------------- | -------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Photos/videos folder | A normal folder containing media       | Reads file metadata and hashes locally.                                                                                                                              |
| Apple Photos         | A folder exported from Photos.app      | Export unmodified originals first; Unfold does not open the live Photos library.                                                                                     |
| Apple Mail           | An exported `.mbox` package            | Use Mailbox › Export Mailbox… in Mail first. No Full Disk Access route.                                                                                              |
| Apple Music          | The Music library route at export time | macOS may ask for Music Automation only when creating Markdown.                                                                                                      |
| Google Takeout       | A Takeout ZIP or extracted folder      | Recognizes supported Photos, Gmail, YouTube, and KMZ/Maps shapes.                                                                                                    |
| Airbnb               | Airbnb JSON export folder              | Emits every reservation status plus readable messages/reviews; `AIRBNB-COVERAGE.md` accounts for every inspected category and explains records kept out of Timeline. |
| Banking              | CSV/XLSX/XLS/ODS transaction export    | Emits transaction events and counterparties.                                                                                                                         |
| Diary                | Supported structured DOCX diary        | Keeps entries local and traceable.                                                                                                                                   |
| Google Maps / places | Supported CSV/JSON/KMZ files           | Emits saved-place / POI events.                                                                                                                                      |

### Working export routes that need plain wording

| Source                  | What the user chooses                  | Say this                                                            |
| ----------------------- | -------------------------------------- | ------------------------------------------------------------------- |
| Apple Calendar          | Exported `.ics` file/folder            | Supported via export, not live Calendar database access.            |
| Apple Contacts          | Exported `.vcf`/vCard file/folder      | Supported via export, not live Contacts database access.            |
| Apple Music TSV         | Exported TSV                           | Secondary executable route for prepared libraries.                  |
| Existing photo Markdown | Existing Open Event Markdown directory | Recognized as already-open Markdown; no extraction command offered. |

### Not available in this build

These sources may be mentioned only as privacy-sensitive categories. They are
not working import routes and should not be described as "coming soon":

- Apple Messages / iMessage
- Apple Notes
- Apple Journal
- Outlook PST
- Immich

## Extract an archive or local app export

1. On Home, choose **Choose a folder** (or **Scan another folder** when an
   archive is already present). On Scan, use **Choose raw file** or **Choose raw
   folder** for the corresponding native picker.
2. Select a raw export, mixed Takeout folder, existing Markdown archive, or
   exported local-app package such as an Apple Mail `.mbox`.
3. Review every locally recognized route. A mixed Google Takeout can expose
   Google Photos, Gmail, YouTube, and KMZ as separate routes.
4. Choose a local destination folder.
5. For a single recognized route, choose **Create Markdown files**. When
   several routes are recognized, choose **Create all N exports** for all of
   them or **Create selected export** for the selected route.
6. Leave the app open while it reports the active export, records read, Markdown files written,
   unchanged events, and errors. After extraction, Unfold builds one cross-source timeline.

The source remains in place. ZIP, MBOX, XLSX, JSON, CSV, TSV, KMZ, images, and
videos are parsed locally. Symbolic links are not followed outside the chosen
folder, and compressed Takeouts do not need to be uploaded or manually unpacked.

To stop safely, choose **Stop extraction**. Event files already completed remain
valid; rerunning the adapter recognizes their stable event IDs and does not
create duplicates, including when an earlier export used the legacy filename
format.

Packages are grouped under `<destination>/<service>/<export-date>/`. Each one contains:

- `README.md` explaining the open contract;
- `manifest.yaml` with counts, adapter ID, source path, and `networkUsed: false`;
- `events/YYYY/MM/*.md` with schema-v1 YAML frontmatter, readable bodies, and
  filenames containing the event title plus a stable short ID.

The Markdown works without Unfold. A possible future amara Memory connection
is a plan, not an available or guaranteed import route.

## Use local app sources

Scan also offers **Suggested folders and app exports**. This is the second
intake path: instead of downloading a Takeout, you can point Unfold at a local
app export or a narrow local folder that already exists on your Mac.

Unfold keeps this deliberately boring:

- nothing is inspected until you choose a card or folder;
- sources are read once, locally, into the destination you choose;
- originals are not changed;
- no network API, sync service, or cloud account is contacted;
- Unfold keeps no background connection to the app after export.

### Apple Mail

The recommended Mail route is export-first:

1. In Apple Mail, select a mailbox.
2. Choose **Mailbox › Export Mailbox…**.
3. Save the exported `.mbox` package somewhere local. The **Prepare Mail
   export** helper creates and opens a Desktop folder named
   `Amara Unfold Mail Exports` so you have an obvious save target.
4. In Unfold, choose **Use found Mail export** if the card already found the
   saved `.mbox`, or choose **Choose Mail export** and select that `.mbox`
   package.
   If you exported while Unfold was already open, return to Scan and click
   **Refresh app exports** if the card has not updated yet.

This follows Apple’s own Mail User Guide: Apple documents that Mail exports
selected mailboxes as `.mbox` packages via **Mailbox › Export Mailbox…**. See
[Import or export mailboxes in Mail on Mac](https://support.apple.com/guide/mail/mlhlp1030/mac).

The **Prepare Mail export** helper opens Mail and the export landing folder so
you can perform Apple’s export step. It does not read Mail, grant permission, or
start extraction.

If an exported `.mbox` package is already on Desktop, Documents, Downloads, or
iCloud Drive, Unfold may show **Export found**. That discovery checks only the
folder name and top-level export shape; message bodies are not read until you
choose the export and create Markdown.

Unfold does not ask users to grant Full Disk Access for Apple Mail, and the
Apple Mail adapter does not support the protected live `~/Library/Mail` store.
The supported end-user route is the exported `.mbox` package.

### Apple Music

**Use Music library** points at the standard current-user Music library location
and reads local metadata once. For Music.app library metadata, macOS may ask for
Music Automation access at export time. Unfold does not contact Apple Music or
the network, keeps no background Music.app connection, and asks macOS to forget
the Automation approval after the export where supported. If macOS keeps the
approval, revoke it in System Settings › Privacy & Security › Automation.

The result is Markdown-first: `APPLE-MUSIC-OVERVIEW.md`, an alphabetical
`library/` track catalog, `PLAYLISTS-OVERVIEW.md`, one readable file per
playlist under `playlists/`, and `APPLE-MUSIC-COVERAGE.md`. Playlist files
preserve track membership and show each linked track's library `Date Added`.
Music.app does not expose a trustworthy playlist creation date, so Unfold never
invents one.

Tracks are not emitted as 15,000 separate Timeline cards. Instead, Unfold writes
one combined Timeline record for each genuine `Date Added` day and lists the
tracks added on that date. `Last Played` remains secondary catalog metadata; it
never creates or reorders Timeline entries. Track/playlist catalogs remain
outside Timeline. The Apple/iTunes `1904-01-01` empty-date sentinel is excluded.
Release year, folder names, file timestamps, and extraction time never become
personal Timeline dates.

For a chosen folder of loose audio files, embedded title/artist/album/genre/year
metadata is used where macOS exposes it, with visible filename/folder fallbacks.
Those files form a timeless catalog/reference unless they contain an explicit
supported Music activity date; filesystem creation/modification time is never a
substitute.

### Immich

Immich is listed under **Not available yet**. This build does not offer a
selectable Immich application route and does not inspect Immich locations.

### Calendar and Contacts

Calendar and Contacts routes are available for readable exported/local files
(`.ics` calendars and `.vcf` contacts). Modern macOS/iCloud stores do not always
expose clean files at their internal default locations, so exported folders are
the most predictable user path.

### Messages, Notes, and Journal

Messages, Notes, and Journal are not available in this build. They are
privacy-sensitive categories, not working import buttons. Unfold does not read
their private stores and does not ask for broad access to them.

### Future modules to build

These are ideas for future community work, not capabilities in this build:

- **Podcasts:** a local, user-controlled module for conserving selected
  subscriptions, saved episodes, listening history, playback progress, and
  local episode metadata. Podcasts often carry the interests, routines,
  discoveries, and chapters of a life, so the module should help people see
  those traces without contacting a podcast service.
- **NotebookLM:** an offline adapter may be considered once a stable, open
  export contract exists. It remains unsupported today.

## Use the generated timeline

Every completed extraction refreshes `<destination>/Amara-Unfold-Timeline/`.
This folder is derived entirely from the event Markdown and contains:

- `index.md` as the human starting point;
- `days/YYYY/MM/YYYY-MM-DD.md` with every event mapped to its supported day;
- month and year pages for navigation and partial dates;
- `undated.md` for records that cannot honestly be assigned a day;
- `catalog.csv` with every event path, title, date, entity ID, and review status;
- `connections.md` for shared dates, actors, and places;
- consolidated people, merchant, artist, channel, and place indexes;
- `quality-report.md` for portability and review gates.

Multi-day stays appear on each covered day. A same-date intersection is labelled
as context, not proof that two records describe the same activity. Unknown people
remain unresolved identifiers until a resolver or person confirms them.
When several export archives contain the same stable event IDs, All events
deduplicates those records. Its Months metric counts distinct supported
`YYYY-MM` groups in that merged catalog rather than adding each archive's page
count, so rerunning a source cannot inflate the summary.

Event IDs—not filenames—provide the stable schema identity. Event filenames are
also intended to be useful in Finder: sortable date/time and source come first,
then a readable title and the stable short ID. Use `index.md`, the day pages, or
`catalog.csv` when navigating across many sources.

## Understand the workspace

### Home

Home summarizes indexed files, photos, videos, storage size, sources, and the
number of photo events found. **Local only** is always visible so the current
privacy state is unambiguous.

### Scan

Scan shows the current four-step flow: choose source, confirm format, choose
destination, create Markdown. It also shows the two intake paths:

- **Choose raw file** or **Choose raw folder** for Takeouts, archives, existing
  Markdown, and exported packages;
- **Choose from an application** for supported local app routes such as Apple
  Mail, Apple Music, Calendar, Contacts, and Photos exports. Immich and the
  other unavailable routes remain visibly separate and cannot be selected.

Recent scan issues identify what was skipped. They are stored locally and are
not uploaded.

### All events

All events remembers every completed export archive instead of replacing the
previous archive when another source is converted. When separate export folders
live beside each other, Unfold restores them into one view and suppresses exact
reruns with the same event IDs. Each event remains tied to its own local
Markdown folder.

Event cards identify the provider first—such as **Diary**, **Airbnb**, or
**Google Maps**—followed by the event type and other useful details. A readable
preview is always visible; expand **Read complete event** to read the full local
Markdown body.

### Photo drafts

Photo drafts groups indexed media by month and then by date. Each memory draft is a
deterministic metadata event, not generated story text or a photo viewer. The
first draft opens automatically. Expand any draft to inspect:

- the capture-time span, GPS grouping, camera model, and distinct-file count;
- filenames and local paths supporting the cluster;
- whether the date came from EXIF or a file timestamp;
- conservative folder-name hints such as a place or event folder;
- photo and video counts.

Numeric folders such as `13` are treated as archive structure, not place names.
The app labels metadata as observed fact and explicitly says when it has not
named an event or identified the people shown.

Use **Export open Markdown** to choose a local folder. Unfold writes a readable
`README.md` and one amara Unfold Open Event Schema v1 event file per indexed
media file under `photos/library/YYYY/MM/DD/`. The export includes SHA-256
provenance hashes for the open schema and retains Unfold's BLAKE3 index hashes.
It does not copy the photos. amara Memory is planned and unavailable; this
export does not establish compatibility with a released Memory application.

### Source history

Source history now lives inside Home and the local index instead of a separate
top-level module. Removing a source from the local index deletes only indexed
rows, scan history, and generated memory candidates. It never deletes or changes
the original folder or any file inside it.

### Settings

Settings reports effective privacy state, network/telemetry status, and the
local database path. In this release both network access and telemetry are off.

### Help

Help is the in-app end-user guide. It explains the four-step workflow, each
workspace module, supported and unsupported sources, generated Markdown and
timeline files, privacy and source-removal behavior, troubleshooting, FAQs, and
the current beta limitations. Use **Start in Scan** at the top of Help to move
directly into the intake workflow.

Help also explains the project's community purpose: personal history should not
remain trapped in digital fenced gardens. Unfold converts available exports into
open files as a service to everyone, runs fully locally with zero outbound
traffic, and is licensed under AGPL-3.0-or-later. People who want to contribute
new adapters or submodules can write to `info@amara.gmbh`.

Its central promise is: **Discover your digital traces. Understand what matters.
Conserve your memories in open files.** This is discovery without surveillance,
scoring, profiling, or a hidden dossier. You choose every source, every module,
and every output; seeing a trace never means you must conserve it. Podcasts are
listed clearly as a future module candidate rather than a current capability.

## Where the local index lives

On macOS the database normally lives at:

```text
~/Library/Application Support/com.amaramemory.unfold/amara-unfold.db
```

The database contains personal paths and metadata and should be treated as
private data. Removing a source in the app deletes only that source's indexed
rows. Deleting the database removes every index, but neither action deletes
original media.

## Current limitations

- no upload, cloud sync, or amara Memory bridge;
- no face recognition or AI-written stories;
- video duration is not yet extracted;
- HEIC files are recognized, but dimensions/EXIF depend on decoder support;
- the SQLite index is local but not application-level encrypted;
- no polished, signed installer yet.
- NotebookLM Takeouts are recognized as an unsupported service gap and are not
  yet converted.

These limits are explicit so the first public version never promises behavior
that has not been implemented and tested.

## If a scan reports a problem

1. Read **Recent scan issues** in Scan.
2. Confirm the selected folder still exists and is readable by your macOS account.
3. Try a small synthetic or copied test folder—not your irreplaceable archive.
4. Run the automated deep test described in [testing.md](testing.md).
5. If the deep test passes but the folder does not, report the exact error
   without publishing private paths or media.
