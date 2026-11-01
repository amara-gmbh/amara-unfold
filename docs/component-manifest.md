# amara Unfold component and runtime manifest

## Product contract

amara Unfold is an open-source, local-first desktop and command-line tool for
turning raw personal-data exports into open Markdown. It also maintains an
optional local SQLite media index and creates deterministic timeline and entity
indexes over exported events.

It is not a photo viewer, hosted service, sync engine, face recognizer, or
closed data store. No amara account is required. No runtime network feature or
telemetry is configured. See the [privacy model](privacy.md) for the limits of
the network-status declaration.

The generated Markdown may be used with a separate locally running LLM or
document tool selected by the user, but no LLM is bundled or controlled by
Unfold. amara Memory is planned and not available yet. This release contains
no Memory application or import bridge. The open files remain independently
useful, without a future product being required to read them.

## Distributed binaries

### Desktop application

`Amara Unfold.app.zip` contains a Tauri 2 application with:

- the Rust native host and extraction code;
- the compiled React renderer;
- bundled SQLite support;
- schema migrations;
- adapter-recognition catalog;
- icons and amara brand assets.

Current binary facts:

- publisher: `amara information security GmbH` (`https://amara.gmbh`);
- product version: `2026.9.21` (build `202609210800`);
- bundle identifier: `com.amaramemory.unfold`;
- target: macOS Apple Silicon `arm64`;
- minimum macOS compatibility: unverified for this preview;
  `bundle.macOS.minimumSystemVersion` is not configured, so the earlier macOS 11
  design target is not an enforced or tested support guarantee;
- signing: ad-hoc development seal only; not Developer ID signed or notarized;
- runtime frameworks: macOS AppKit, WebKit, Foundation, CoreServices,
  CoreGraphics, and other Apple system frameworks;
- external database/runtime service: none.

### Command-line application

`amara-unfold` is a separate `arm64` Mach-O executable using the same Rust
adapter library as the desktop application. It supports:

```text
amara-unfold adapters
amara-unfold extract <adapter-id> --source <path> --output <folder>
amara-unfold timeline <export-folder>
amara-unfold validate <package-folder>
```

The CLI is included because it gives a new operator or LLM a deterministic,
scriptable recovery and verification path without automating the GUI.

## Runtime storage

The app creates its SQLite database on the destination Mac at the operating
system application-data location, normally:

```text
~/Library/Application Support/com.amaramemory.unfold/amara-unfold.db
```

SQLite is bundled into the application. Do not install a system SQLite server.

The database is a rebuildable private index, not the canonical archive. Source
exports remain canonical raw evidence. Generated Markdown remains portable
user-owned output.

## Source components

The matching source tree contains:

| Component               | Responsibility                                                     |
| ----------------------- | ------------------------------------------------------------------ |
| `apps/desktop`          | React UI, Tauri host, native folder picker, typed IPC              |
| `crates/core`           | Shared models/contracts; network-policy placeholder |
| `crates/adapters`       | Offline takeout parsers, Markdown writer, timeline builder, CLI    |
| `crates/scanner`        | Cancellable traversal without following symbolic links             |
| `crates/extractors`     | Filesystem, image, and EXIF metadata extraction                    |
| `crates/db`             | Bundled SQLite repository and migrations                           |
| `crates/memory`         | Conservative evidence-based media candidates                       |
| `adapters/catalog.json` | Open dataset-recognition contract                                  |
| `docs`                  | Architecture, privacy, operation, testing, and transfer contracts  |

The native app and CLI share `crates/adapters`; they must not develop separate
normalization behavior.

### Interface appearance contract

The desktop interface uses Segoe UI, with Arial and Helvetica as fallbacks,
for navigation, content, Timeline dates, and large year numerals. Dark mode
uses one coffee surface (`#181713`) and three intentional text levels:

- near-white emphasis (`#faf9f5`) only for page titles, headline metrics, the
  current next-action heading, and recognized-format heading;
- normal warm cream (`#ded8cc`) for ordinary headings, controls, labels, and
  readable content;
- muted warm cream (`#b8b2a4`) for explanatory and secondary copy.

New components must use the semantic `--emphasis`, `--text`/`--ink`, and
`--muted` tokens instead of adding hard-coded white text.

Structured control groups must use a consistent visible
label-plus-control pattern and equal internal spacing. The production-width
Home promise uses three intentional lines; its conservation sentence remains
whole when space allows and returns to normal wrapping below the wide-layout
breakpoint. Paired Home summary panels use equal columns and content-sized
height; dashboard summaries must not repeat headline metrics as a second hero
or expose implementation paths merely to fill empty card space.

All events distinguishes archive layers from providers. Each completed export
folder is one removable layer; Provider is a catalog filter that groups entries
by source type and may therefore have a smaller count. Removing a layer hides
it from the local app catalog without deleting its Markdown or raw source.
All event-summary figures use the same stable-ID-deduplicated catalog regardless
of archive size. The visible invariant is `All events = Dated + Undated +
Reference notes`; month pages are distinct date groups and are not an event
category.

## Data formats

### Raw inputs

Current executable routes cover official or structured exports for photo/video
folders, Apple Mail, Apple Music, Apple Calendar, Apple Contacts,
Airbnb, banking CSV/XLSX, diary DOCX, Gmail MBOX, YouTube/YouTube Music,
Google Photos, Google Maps CSV/JSON, and KMZ.

Internal Outlook PST and Immich adapter code is not registered or advertised in
this release. The UI lists both under **Not available yet**.

Banking CSV/XLSX is emitted as one exact-date Markdown movement file per active
banking day, plus a compact month-precision overview. Daily files preserve the
usable transaction rows in readable blocks; the manifest accounts included
source rows separately from emitted Timeline files.

Run `amara-unfold adapters` for the binary's authoritative adapter-ID list.
Read the [open adapter contract](open-adapters.md) before choosing one.

### Service output

Generated packages use open files only:

- Markdown;
- YAML frontmatter;
- compact CSV/YAML indexes where validation or navigation requires them;
- SHA-256 provenance hashes;
- deterministic paths and IDs.

No original media is copied into event packages.

### Timeline output

`Amara-Unfold-Timeline` is a derived destination-level index containing:

- day, month, year, and undated Markdown pages;
- `catalog.csv`;
- people, merchant, artist, channel, and place indexes;
- `connections.md`;
- `quality-report.md`.

Connections are evidence-only. Same-date intersections are candidates, not
claims of causation, companionship, or identity.

## Runtime security boundary

- The app receives only minimal Tauri window and native-dialog capabilities.
- Filesystem access begins with an explicit native folder selection.
- Symbolic links are not followed beyond that selected tree.
- Runtime network access and telemetry are false.
- Original files are never deleted or rewritten.
- Removing a source in the app removes only local index records.
- The SQLite database is private but not application-level encrypted.

## Running versus developing

### Running the transferred binaries

Required:

- Apple Silicon Mac;
- compatible macOS system frameworks;
- permission to read the chosen source and write the chosen destination.

Not required:

- Node.js;
- npm packages;
- Rust or Cargo;
- Xcode Command Line Tools;
- Homebrew;
- Python;
- external SQLite;
- internet access;
- an LLM.

### Building from matching source

Required:

- Xcode Command Line Tools;
- Node.js `22.13` or newer;
- npm dependencies from `package-lock.json`;
- pinned Rust toolchain with project minimum Rust `1.82`.

```sh
npm install
npm run check
npm run test:deep
```

The debug app bundle is created at:

```text
target/debug/bundle/macos/Amara Unfold.app
```

The CLI is built with:

```sh
cargo build -p amara-unfold-adapters --bin amara-unfold
```

## Known distribution limitations

- The current package is Apple Silicon only.
- It is unsigned and not notarized.
- It is a debug build, larger than a final release build.
- There is no installer or auto-updater.
- There is no cloud bridge or amara Memory import implementation.
- NotebookLM export conversion remains unsupported.
- Podcasts are documented as a future local module candidate; no podcast import
  or service integration exists in this build.
- Binaries alone cannot be modified or independently audited; keep the matching
  source revision available for development and public distribution.

For a public release, produce signed/notarized architecture-labelled artifacts,
publish reproducible checksums, and provide corresponding source under the GNU
AGPL v3-or-later terms.
