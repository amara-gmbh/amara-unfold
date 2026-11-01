# Adapter status

All executable routes run locally through the same open Rust adapter crate used
by the CLI and desktop app. Recognition alone does not count as extraction.

## Direct executable routes

These are the fourteen registered `amara-unfold extract <adapter>` routes in
this build. Registration describes executable code, not installed-app acceptance
of every input variant. Native recognition and application-card availability
are separate gates.

| Route                       | Input                                                             | Output focus                                                           | Expected skips                                                                       |
| --------------------------- | ----------------------------------------------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `photo-media-folder`        | Image/video directory                                             | Capture metadata and hashes                                            | Duplicate or unchanged events                                                        |
| `banking-xlsx`              | XLSX/XLS/ODS                                                      | Daily movements + monthly overviews                                    | Missing dates, duplicates, reruns                                                    |
| `banking-csv`               | Delimited transaction export                                      | Daily movements + monthly overviews                                    | Missing dates, duplicates, reruns                                                    |
| `airbnb-json-takeout`       | Airbnb JSON directory                                             | All reservation statuses, messages, reviews, and category coverage     | Supporting/static/telemetry records with explicit reasons; duplicates or reruns      |
| `diary-docx`                | Structured DOCX                                                   | Dated verbatim entries                                                 | Duplicates or reruns                                                                 |
| `gmail-takeout`             | MBOX or Takeout ZIP                                               | Decoded message bodies                                                 | Duplicates or reruns                                                                 |
| `apple-mail-local`          | Apple Mail `.mbox` export                                         | Cleaned message bodies                                                 | Duplicates, unreadable stores, reruns                                                |
| `apple-calendar-local`      | `.ics` calendar files                                             | Calendar events                                                        | Missing dates, duplicates, reruns                                                    |
| `apple-contacts-local`      | `.vcf` contacts                                                   | Contact/person events                                                  | Empty cards, duplicates, reruns                                                      |
| `youtube-takeout`           | YouTube CSV/JSON or ZIP                                           | Subscription/history events                                            | Duplicates or reruns                                                                 |
| `google-photos-takeout`     | Takeout ZIP                                                       | Supplemental photo metadata                                            | Missing timestamps, duplicates, reruns                                               |
| `google-maps`               | Maps Takeout, GeoJSON/JSON, saved-place CSV, KMZ or supported ZIP | Saved places with unified provider and cross-format identity           | Missing coordinates, duplicates, reruns                                              |
| `apple-music-tsv`           | Track-library TSV                                                 | Track Markdown catalog plus Date Added-only daily Timeline files       | Duplicate tracks/reruns; missing or sentinel activity dates                          |
| `apple-music-local-library` | Music.app metadata or audio folder                                | Separate track/playlist catalogs, membership, Date Added-only Timeline | Duplicate tracks/playlists/reruns; missing/sentinel dates; loose-file dates excluded |

The CLI currently reports fourteen direct executable offline routes. Every
registered release route has a portable synthetic fixture or an accepted
export-shaped local case; no private mail or music corpus ships in source.
Dormant PST and Immich implementation code is not registered or advertised in
this release. Local-app routes use synthetic export-shaped fixtures and
explicit-folder UAT so private app stores never enter the public repository.
See [Open adapter contract](open-adapters.md) for commands and
[Data quality](quality-and-validation.md) for acceptance rules.

Banking routes are intentionally navigable: every usable transaction remains
readable in an exact-date daily Markdown movement file, while one compact
month-precision overview per calendar month provides secondary navigation.
Daily files use readable movement blocks instead of raw tables. The manifest
separately reports output events and included source records. CSV recognition
supports common English/German aliases, title rows before the header, comma,
semicolon, tab, or pipe delimiters, and either one amount column or separate
debit/credit columns.

## Catalog recognition / normalization routes

These entries exist in the adapter catalog for recognition or prepared working
sets. They are not direct executable extract commands in this build.

| Route                           | Purpose                                      |
| ------------------------------- | -------------------------------------------- |
| `foreverlane-photo-markdown-v1` | Recognize already-open photo-event Markdown. |
| `apple-music-markdown`          | Recognize prepared Music Markdown libraries. |

The catalog contains sixteen entries: fourteen ready extraction definitions and
the two already-open recognition entries above. `google-maps-pois-working-set`
is no longer in the catalog. The library retains dispatch branches for
`google-drive-kmz`, `google-maps-pois-json`, and `google-maps-saved-csv`, but these
are not registered CLI commands or independent native recognition routes. Use
the unified `google-maps` route.

## Native support boundary

The Scan raw-input card advertises Diary DOCX, Banking Takeout, Airbnb Takeout
and Google Maps. Airbnb requires an extracted JSON export directory; direct
Airbnb ZIP intake is outside the current contract. Five application cards are
selectable: Apple Photos exported originals, Calendar ICS, Contacts vCard,
Mail exported MBOX, and Music metadata/local audio. These are exported or
explicitly selected sources, not unrestricted access to private application
stores. New importers are frozen while existing routes receive reliability
and acceptance work.

The generic raw recognizer can expose additional registered adapters when the
catalog signatures match. This is broader than the four advertised raw routes
and does not establish current installed-app acceptance of every input variant.
PST and Immich are absent from the release catalog and registered CLI list.

Known recognition limits requiring existing-route closure:

- Workbook recognition and parsing accept XLSX, XLS and ODS. Native acceptance
  of additional XLS/ODS content variants remains separate from recognition.
- Gmail and YouTube native recognition requires matching Takeout archive paths;
  standalone MBOX or loose CSV/JSON support in the extractor does not guarantee
  the same native selection path.
- Music TSV recognition expects `raw/library_tracks.tsv` beneath the selected
  root. A bare TSV is not an equivalent recognized input.
- Diary recognition accepts an existing diary path/name hint or bounded content
  matching the supported chronological heading structure. Ordinary Word reports
  without that structure are not diary imports.

Historical installed-app evidence remains bounded to its input and build.
Fresh candidate acceptance must cover normal selection, recognition, extraction,
balanced quality, complete Markdown readback, provenance, duplicate reruns and
safe cancellation/error behavior. Automated fixtures alone do not close these
native cases.

Messages, Notes, Journal, Outlook PST, and Immich are not executable release
routes. They must not be described as launch features.

For `apple-mail-local`, the recommended source is Apple's documented Mail export
path: **Mailbox › Export Mailbox…**, which creates `.mbox` packages. See
[Apple Mail User Guide: Import or export mailboxes](https://support.apple.com/guide/mail/mlhlp1030/mac).

## Existing-route reliability update

Diary DOCX recognition also checks bounded document content for the existing
chronological heading structure, so a filename such as `memories.docx` can be
selected normally. An arbitrary Word document is not a supported diary.
Workbook intake recognizes the existing XLSX, XLS, and ODS parser extensions;
recognition alone does not validate a workbook or certify every variant.
Banking CSV recognition and parsing share transaction-header detection across
semicolon, comma, tab, and pipe delimiters, including title rows and quoted
punctuation. Unrelated CSV files in a selected folder are skipped by Banking.

See [failure and cancellation behavior](quality-and-validation.md#failed-cancelled-and-mixed-format-runs).

Saved-place GeoJSON supports the existing saved-place filenames with either
`.json` or `.geojson`, loose or inside the existing Maps ZIP intake. Native
recognition and extraction share this filename check. General GeoJSON geometries
outside the existing saved-place contract are not a new supported importer.

## Local audio metadata boundary

MP3 input reads ordinary ID3v2.3/v2.4 text tags directly, ahead of Spotlight
metadata and filename/folder fallbacks. Tags larger than 8 MiB, compressed,
encrypted, unsynchronised, extended-header and unsupported tag versions fall
back safely. Other audio formats still depend on available Spotlight metadata;
complete embedded metadata coverage is not claimed. Release year, custom ID3
DateAdded, file dates and Last Played do not date loose-audio Timeline events.

Prepared Music TSV native acceptance used three synthetic tracks: one valid
Date Added produced one daily event; missing and 1904 sentinel dates produced
none, and Last Played remained catalog metadata. Live Music.app reads require
separate explicit user authority and macOS consent; synthetic TSV/audio proof
does not establish current live-library acceptance.

## Typed workbook dates

Banking XLS/XLSX and ODS intake preserves typed Excel/ISO date cells, including
the workbook 1900/1904 epoch. Numeric amounts remain numeric text for the shared
parser; arbitrary unformatted numbers are not guessed to be dates. Missing or
invalid dates retain explicit skip accounting. Native variant acceptance is
input/build-specific and is recorded separately from parser checks.

### Calendar recurrence evidence

Calendar Markdown preserves the original unfolded RRULE, RDATE, EXDATE,
RECURRENCE-ID and legacy EXRULE lines, including timezone/value parameters.
It exports explicit VEVENT records, including a separately supplied moved
occurrence, and labels recurrence rules as source evidence. It does not expand
a rule into generated occurrences or resolve a complete recurrence schedule.
This limitation is stated beside the preserved fields rather than silently
dropping the recurrence information. Existing source dates remain unchanged.

Mail direction checks known sent-mail path components (for example Sent.mbox
or Sent Messages.mbox), not arbitrary ancestor substrings such as “essential”
or “consent.” Other messages retain the existing sender-based received/unknown
fallback; this is a folder heuristic, not verified account identity. Multipart
plain text is preferred over HTML alternatives, and attachment payloads are not
inserted into the readable message body. This does not promise attachment export.
