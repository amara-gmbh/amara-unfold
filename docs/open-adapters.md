# Open adapter contract

Amara Unfold turns raw personal exports into user-owned Markdown. Amara Memory
is an optional consumer of that Markdown, not a required runtime or storage
service.

## Recognition first

Selecting a dataset performs a read-only local inspection. Recognition uses the
versioned catalog in `adapters/catalog.json`, validated by
`adapters/catalog.schema.json`. Rules identify datasets from ordinary source
names, paths, extensions, and ZIP member paths.

Recognition never runs an adapter. It reports:

- the provider and dataset type;
- whether the input is raw, mixed, or already open Markdown;
- the existing adapter/module ID;
- evidence that caused the match;
- readiness and any preparation gate;
- the open output contract;
- the exact command template when an adapter is ready.

## Adapter boundary

Adapters are implemented in the open-source `crates/adapters` Rust crate. The
desktop app and bundled `amara-unfold` command use the same library. Timeline
modules informed normalization compatibility, but Timeline, Python, Amara
Memory, provider accounts, and network access are not runtime dependencies.

An adapter must:

1. read the selected source without modifying it;
2. produce Markdown with YAML frontmatter;
3. preserve original paths and deterministic content hashes;
4. be idempotent where the source format permits;
5. expose progress, writes, skips, warnings, and errors;
6. never require Amara Memory to read the result.

## Direct executable offline routes

| Adapter                     | Input                                                 | Normalized event output                                           |
| --------------------------- | ----------------------------------------------------- | ----------------------------------------------------------------- |
| `photo-media-folder`        | Local image/video directory                           | Photo/video metadata                                              |
| `banking-xlsx`              | XLSX, XLS, or ODS workbook                            | Transactions                                                      |
| `banking-csv`               | Provider transaction CSV                              | Transactions                                                      |
| `airbnb-json-takeout`       | Airbnb JSON directory                                 | Reservations, messages, reviews, and complete category accounting |
| `diary-docx`                | Structured Word diary                                 | Verbatim diary entries                                            |
| `gmail-takeout`             | MBOX file or MBOX inside Takeout ZIP                  | Emails                                                            |
| `apple-mail-local`          | Apple Mail `.mbox` export                             | Emails                                                            |
| `apple-calendar-local`      | `.ics` calendar files                                 | Calendar events                                                   |
| `apple-contacts-local`      | `.vcf` contacts                                       | Contact/person events                                             |
| `youtube-takeout`           | YouTube CSV inside or outside ZIP                     | Subscriptions/history rows                                        |
| `google-photos-takeout`     | Supplemental JSON inside Takeout ZIP                  | Photo metadata                                                    |
| `google-maps`               | Maps Takeout, GeoJSON/JSON, CSV, KMZ or supported ZIP | Unified saved places and collections, cross-format identity       |
| `apple-music-tsv`           | `library_tracks.tsv`                                  | Track catalog and Date Added-only daily Timeline files            |
| `apple-music-local-library` | Music.app metadata or audio folder                    | Separate track/playlist catalogs, membership, Date Added Timeline |

The current registered CLI list contains fourteen direct executable offline
routes. Portable synthetic fixtures and accepted export-shaped local cases cover
the registered release surface without putting private app stores or personal
mail in source control. PST and Immich are not registered release routes. The
same compiled library powers the CLI and native app.

The catalog also contains two recognition-only entries for already-open photo
Markdown and prepared Apple Music Markdown. Those are not direct executable
extract commands. Legacy Maps library dispatch aliases are not registered CLI
commands; use `google-maps`. The native recognizer and application cards have
narrower input and availability limits than the extraction library. Consult
[Adapter status](adapters.md#native-support-boundary) before advertising an input
variant as a supported native workflow. Existing-route reliability is the current
scope; this contract does not authorize new importers.

Apple Messages, Apple Notes, and Apple Journal are not executable adapter
routes in this build. They must be described as not available in this build.

For `apple-mail-local`, the recommended source is Apple’s documented Mail export
path: **Mailbox › Export Mailbox…**, which creates `.mbox` packages. See
[Apple Mail User Guide: Import or export mailboxes](https://support.apple.com/guide/mail/mlhlp1030/mac).

## Command line

The CLI is useful without the desktop app and makes the adapter behavior
inspectable:

```sh
amara-unfold adapters
amara-unfold extract banking-xlsx --source ./takeout --output ./my-markdown
amara-unfold validate ./my-markdown/takeout-banking-xlsx-unfold
```

Output packages live under `<destination>/<service>/<export-date>/` and contain
`README.md`, `manifest.yaml`, and one event per file under `events/YYYY/MM/`.
The filenames combine sortable date/time, source, a readable title, and a stable
short ID. The manifest explicitly records `networkUsed: false`.

Airbnb packages additionally contain `AIRBNB-COVERAGE.md`. It names every JSON
category inspected, balances every logical source record as written or not
converted, and explains why static account, telemetry, or supporting-container
records were kept out of Timeline. Cancelled and denied reservations remain
events rather than disappearing from the record.

The desktop app then derives `<destination>/Amara-Unfold-Timeline/` across all
packages. Event files remain truth. The timeline adds open Markdown navigation,
CSV, and JSON indexes without creating a database or proprietary dependency.
Its connection rules are deliberately narrow:

- exact shared source date;
- active multi-day event span;
- exact shared unresolved actor identifier;
- exact shared deterministic place identifier;
- actor and place evidence contained in the same event.

Actor/place records found only on the same day are review candidates, not claims
that a person was physically present. Month-only and undated events are never
assigned invented days.

## Already-open data

If a source already contains a recognized Markdown substrate, Unfold must not
extract it again. It identifies the structure, shows extraction as skipped, and
offers validation/import workflows separately.

A photo metadata library is an example: `manifest.yaml`, `README.md`, and
`library/` identify an existing photo metadata library.

## Progress stages

Long operations use stable stages:

1. Recognize
2. Extract
3. Markdown
4. Structure/validate

Stages may be complete, active, pending, or explicitly skipped. Progress must be
grounded in adapter events and counts; the UI must not imply work occurred when
it did not.

The CLI can rebuild the same index without the desktop app:

```sh
amara-unfold timeline ./my-markdown
```
