# Architecture

## Goals

Amara Unfold must be understandable by a reviewer who has never seen the
project, responsive with large archives, and conservative with personal data.
The first release therefore separates traversal, extraction, storage,
candidate rules, UI, and future outbound behavior.

## Runtime flow

```text
Native folder picker
        │ explicit user selection
        ▼
Tauri command ── emits progress/cancellation ──► React workspace
        │
        ▼
LocalArchiveScanner
        │ one file at a time, no symlink following
        ├──► ExtractorRegistry ──► image/EXIF metadata
        │
        ├──► DateClusterEngine ──► candidates + evidence
        │
        ▼ atomic completed snapshot
IndexRepository ──► SqliteIndex ──► local app-data database
```

Scanning runs on Tauri's blocking worker pool rather than the WebView thread.
Progress is throttled before crossing the IPC boundary. An atomic cancellation
flag is checked between filesystem entries.

## Modules

### `crates/core`

Owns serializable domain models and contracts shared across native modules.
`IndexRepository`, `SourceImporter`, and `AmaraMemoryBridge` are boundaries,
not implementations. The `NetworkGuard` denies every outbound intent in this
release.

### `crates/scanner`

Owns recursive traversal and content hashing. It does not know about SQLite or
the UI. Symbolic links are intentionally not followed because they can cross
the folder boundary the user selected.

### `crates/extractors`

`MetadataExtractor` implementations advertise support and return optional
facts. Extraction failure does not discard a file: the scanner keeps
filesystem evidence and records the error. The first registry supports common
image dimensions and EXIF. A future video extractor can use an optional local
helper without changing scan orchestration.

### `crates/db`

Implements `IndexRepository` with bundled SQLite, so users do not need a system
database installation. Schema changes are ordered SQL migrations. A completed
scan replaces one source snapshot inside a transaction. A cancelled scan does
not replace the last complete snapshot.

### `crates/memory`

Creates deterministic candidates from capture/filesystem dates and conservative
folder hints. Every candidate carries supporting paths and reasons. This module
does not generate prose or infer events.

### `apps/desktop`

The Tauri host owns native capabilities, the application data path, shared
state, and IPC commands. React owns display and interaction only. The UI uses a
single typed desktop bridge and cannot access arbitrary filesystem paths.

## Importer extension

The local folder is the first source, not a special case embedded in the UI.
A future importer should:

1. implement `SourceImporter` and define the explicit source selection UX;
2. produce local paths or a documented read-only stream for the scanner;
3. require only narrowly scoped Tauri capabilities;
4. add fixtures containing synthetic data only;
5. preserve cancellation, transparent errors, and atomic indexing.

Application routes should prefer explicit user-controlled exports when a
platform keeps its live library behind protected storage. Apple Photos, for
example, is supported through exported originals or a chosen media folder; the
current release does not read its protected private library database directly.

## Future Amara Memory bridge

The bridge trait is present; a bridge implementation is not. Adding one
requires a review screen that enumerates selected assets and metadata, an
explicit destination, a single-use consent token, and an auditable local event.
It must not turn on background synchronization or change local-only defaults.

The open Markdown can also be used independently with a separate locally
running LLM or document tool chosen by the user; Unfold does not bundle or
control that tool. Unfold is the intended open preparation and intake layer for
the optional higher Amara Memory product, which can add a drillable Book,
visual Life Map layers, evidence-based cross-source connections, and grounded
navigation back to supporting records. Those experiences must never make Amara
Memory the owner or sole reader of the files.

## Cross-platform choices

- `PathBuf` is retained until serialization/storage boundaries.
- no shell commands or macOS-only metadata APIs are required for scanning;
- SQLite is bundled through `rusqlite`;
- native selection uses the Tauri dialog plugin;
- Windows builds are checked in CI from the initial public baseline.

## Package publication boundary

The adapter writer uses a sibling temporary directory for each extraction.
It copies a prior package into staging for idempotent reruns, validates that
staged package, and publishes it only on normal completion. Handled publication
errors restore the prior package; failed restoration retains its path in the
error. Timeline discovery excludes staging directories. This does not provide
whole-batch atomicity, concurrent-writer isolation, or power-loss recovery.

## Public-source packaging boundary

The public source tree is generated from an explicit reviewed top-level
allowlist rather than by copying every new repository-root path. The exporter
also applies narrow file/type exclusions, removes canonical-only continuity
commands from public package metadata, and runs the same public-tree audit on
the result. A negative regression deliberately reinjects canonical-only tooling,
private commands and an unapproved top-level path and must be rejected.

Reachable public Git history is a separate gate. Every reachable commit author
and committer and every annotated tagger must use the exact approved company
identity; reachable patches, messages and tag payloads must remain free of the
configured private signatures. A clean source tree does not make a previously
contaminated Git object database clean, so a standalone publication history must
start from a never-imported object database.

## Timeline rebuild cancellation

The native final-index worker shares the extraction cancellation flag. The
adapter keeps the previous index while writing a temporary replacement, checks
cancellation before promotion, and removes its temporary tree on cancellation
or handled write failure. Promotion renames the old index aside, installs the
complete replacement, then removes the old copy; a failed replacement rename
restores the old index. This is a handled-error/cancellation contract, not a
power-loss or concurrent-writer guarantee. Already committed source packages
remain saved even when the derived Timeline rebuild is cancelled.

## Native product branding

The permanent written brand is `amara`, lowercase even at the beginning of a
sentence or heading. Use `amara Unfold` for new copy, metadata and product
display names. Preserve exact technical paths/identifiers, historical records
and legally required registered names; do not bulk-rename existing artifacts.

The sidebar renders the supplied complete amara Unfold artwork, including its
frame and product lettering, as one visible SVG per theme. Original cream and
mocca variants live in `assets/brand/` and are served through Vite's configured
`../../assets` public directory. The cream variant is dark ink for light mode;
the mocca variant is light ink for dark mode. Keep the original 166:63 viewBox
proportions and colors; do not combine a base mark with a separate product label
or apply the old recoloring filter. Native bundled rendering is the acceptance
surface because an asset in an unused public directory will not ship.

Settings section headings and setting-row labels share a 38px icon column and
12px text gap, so their icon centers and text starts align vertically in both
themes. The appearance selector keeps its separate horizontal button layout.
