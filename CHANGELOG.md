# Changelog

All notable public changes to Amara Unfold will be documented here. The format
follows Keep a Changelog. Pre-release app versions use `YYYY.M.D`; macOS bundle
build numbers use `YYYYMMDDHHMM`.

## [Unreleased]

### 2026.9.21 limited-feature release candidate

- Froze the release promise to the currently accepted local export journeys;
  no new importer, account, cloud service, background access, or amara Memory
  bridge was added.
- Removed Outlook PST and Immich from registered release routes and public
  recognition. Their dormant internal code is not part of the release promise.
- General reruns now distinguish newly written events from unchanged retained
  events instead of reporting retained output as a misleading zero-file result.
- Retained transactional package staging and final-index cancellation behavior:
  handled failures do not replace the last completed package, and cancelled
  Timeline rebuilding preserves the prior complete index.
- Added an exact release-scope document, a private security-reporting email,
  explicit source-first/ad-hoc macOS distribution wording, and an in-app Help
  view focused on the frozen feature set instead of speculative modules.
- Settings no longer promises a future bridge or describes repository
  publication as complete before the privacy and history gates pass.
- JavaScript release gates fail on high-severity production advisories and on
  critical advisories anywhere in the development graph. Known high-severity
  development-tool findings remain documented until upstream WDIO support
  removes the affected downloader dependency.
- Product version advanced to `2026.9.21`; macOS bundle build advanced to
  `202609210800`.

### 2026.8.7 beta build

- Added a public documentation spine for first-time readers: the root README
  now routes users by intent, `docs/README.md` provides the complete table of
  contents, and new system, configuration, and design-decision guides explain
  Unfold from source selection through Markdown, Timeline, quality, testing,
  and distribution.
- Clarified the public-history boundary. Private raw development conversations
  and working memory remain outside the public repository; decisions that
  affect public behavior must be restated in the changelog, decision record,
  public contracts, tests, or source comments.
- The public-source audit now requires the documentation index, A-to-Z system
  guide, configuration reference, and decision record. Older component and
  architecture claims were aligned with the current `2026.8.7` build and the
  export-based Apple Photos route.
- Corrected the local-application availability contract: Immich is now shown
  only under **Not available yet** and exposes no selectable route or local
  path. Its internal adapter remains future implementation work.
- Revalidated every application shown under **Available now** through the
  actual installed Mac app: Apple Photos, Apple Calendar, Apple Contacts,
  Apple Mail, and Apple Music all recognize their advertised input and produce
  local portable Markdown.
- Apple Mail `.mbox` output preserves readable sender, recipient, subject,
  direction, timestamp, and body without background Mail access. Calendar and
  Contacts preserve source-backed event/reference Markdown without inventing
  dates or exposing vCard note bodies.
- Apple Music regression checks preserve complete track, playlist, and
  membership catalogues while limiting Timeline output to source-explicit
  `Date Added` evidence. Personal-library counts remain outside public history.
- Genuinely undated events can now be assigned an exact day, month, or year,
  with optional time for exact-day precision. Later correction and one-step
  undo rebuild the open Markdown Timeline while preserving stable event IDs,
  original bodies, raw-source paths, and raw hashes. Every change is recorded
  as an auditable Markdown revision; timeless reference notes cannot be dated
  through this workflow.
- The Timeline editor uses one compact, symmetric row for precision, date,
  optional time, and the primary action. It avoids nested cards and oversized
  action bars; Undo remains a quiet secondary control.
- Scan now states the supported raw-input families inside the generic
  file/folder card, followed by an equally styled accepted-file line. The
  promise is deliberately limited to the four completed raw routes: Diary
  DOCX, Banking Takeout, Airbnb Takeout, and Google Maps. Airbnb is supported
  through its extracted Takeout folder; no direct Airbnb-app connection is
  implied.
  Banking CSV and spreadsheet adapters share the user-facing name **Banking
  Takeout**, while recognition still names the exact file format.
- Both supported-input lines now use the same cream body-copy size, color, and
  weight as the card's “Choose one raw export file…” description; accepted
  files no longer look like a subordinate footnote.
- macOS bundle build advanced to `202608071154`.

### 2026.8.6 beta build

- Product messaging now presents Unfold as both genuinely useful free/open
  source software and the trusted open first step into Amara Memory. Home and
  Help gently invite users to continue into the richer optional experience
  while keeping the current no-account/no-bridge boundary explicit.
- Apple Music now preserves the full track library and all available playlists
  in separate readable Markdown catalogs. Playlist files retain their track
  membership and show each linked track's genuine library `Date Added`.
- Apple Music Timeline contains only combined daily `Date Added` records. A
  track's `Last Played` remains secondary catalog metadata and can never create
  or reorder Timeline entries. Catalog and playlist files remain outside
  Timeline.
- Apple Music dates are source-truth only. The known `1904-01-01` empty-date
  sentinel is reported and excluded, release year remains catalog metadata,
  and loose audio-file creation/modification timestamps never become events.
  Music persistent IDs, content hashes, and metadata fingerprints provide
  stable rerun and cross-input identity.
- Selecting the standard Music.app route now confirms the known format without
  traversing the protected library. The actual local read—and any macOS Music
  or Automation prompt—is deferred until the user chooses Create Markdown.
- macOS bundle build advanced to `202608061158`.
- Apple Music reruns now report retained unchanged Timeline files explicitly instead of presenting an ambiguous zero-file completion message.
- Apple Music completion messages explicitly distinguish the separate track and
  playlist Markdown catalogs from the daily track `Date Added` files added to
  Timeline.
- The Scan application's expand/collapse action now uses the same squared,
  bordered secondary-button treatment as the rest of Unfold.
- Timeline previews and complete-event text now keep normal reading copy in the
  cream default and reserve near-white emphasis for Markdown facts. Apple Music
  marks the song and artist—not the surrounding explanatory copy—as facts.
- Timeline behaves as a deliberate year accordion: all visible years start
  collapsed, opening another year closes the previous one, and closing the open
  year returns to the compact all-years overview.
- The Scan confirmation action “Change source” now uses Unfold's standard
  outlined secondary-button geometry instead of browser-native styling.
- The application source route now mirrors the raw-data route's icon tile,
  eyebrow, title typography, description hierarchy, padding, and closed-card
  height so the two intake paths read as one symmetrical choice.
- Scan now presents raw file and raw folder as two equal-width source routes.
  Application intake lives once in its dedicated section below instead of
  being repeated in the raw-data card. The application list has only two honest
  groups: available now and not available yet; supported Calendar and Contacts
  routes are no longer hidden behind a third “more sources” tier.
- Application availability is shown through two symmetric stream controls.
  “Available now” opens by default, while “Not available yet” stays minimized
  until the user asks to see planned routes.
- Application rows now keep one visible action and one consistent compact
  rhythm. Apple Mail preparation controls and its three-step explanation live
  inside “How access works” instead of stretching one card out of alignment.
- The extraction pipeline is transient progress UI and disappears after a
  successful export, leaving the completion summary and data-quality result.
- macOS bundle build advanced to `202608061501`.

### 2026.8.5 beta build

- Google Maps is now one user-facing source and Timeline provider regardless of
  whether the selected input is a complete Takeout ZIP/folder, standalone KMZ,
  KMZ folder, saved-place CSV collection, or saved-places GeoJSON.
- Recognition names the actual Maps input format before conversion. CSV files
  can remain inside their ZIP and localized/variant title, note, URL, comment,
  latitude, and longitude headers are mapped by meaning rather than position.
- Google Maps GeoJSON preserves explicit `Published`/`Updated` timestamps,
  addresses, country codes, URLs, and coordinates. KMZ `<when>` timestamps are
  retained. Missing dates remain honestly timeless instead of inheriting ZIP,
  folder, filesystem, or import dates.
- Timeless saved places are classified as searchable reference notes rather
  than flooding the date-review queue. Old `KMZ` and `Google Maps POIs` module
  labels are folded into the single Google Maps provider in All events.
- Cross-format Maps deduplication uses stable Maps URLs and coordinate-plus-name
  evidence. It records `duplicate_event` skips and never merges by title alone.
- Airbnb extraction now inspects every JSON category instead of opening only
  three filenames. Every logical source record balances as written or not
  converted with an exact reason in `manifest.yaml` and
  `AIRBNB-COVERAGE.md`.
- Cancelled and denied reservations are preserved as readable reservation
  events alongside completed stays. Message and review records missing the
  minimum readable body/date fields are counted explicitly rather than
  silently disappearing.
- Static account, security, telemetry, search, payment-instrument, and
  supporting-container records remain in the untouched raw takeout and are
  deliberately kept out of Timeline to prevent noise and sensitive account
  details from masquerading as life events.
- Scan completion and quality details now show why records were not written;
  they no longer describe every skip as an unchanged rerun.
- macOS bundle build advanced to `202608051842`.
- Airbnb service/system messages now preserve their human-facing fallback text instead of exposing Airbnb's internal `MessageContent{...}` serialization in Timeline previews and complete-event views.

### 2026.8.4 beta build

- Banking CSV/XLSX now emits one readable exact-date Markdown movement file per
  active day, with one compact month-precision overview as a secondary aid.
  Flexible English/German columns, title rows, common delimiters, and separate
  debit/credit fields are recognized, while the manifest separately reports
  included source rows, daily files, monthly summaries, and skipped records.
- All events loads complete archive catalogs instead of silently limiting each
  archive to 2,000 rows. Markdown previews remove structural markers, and
  catalog review reasons now distinguish date, identity, and data review.
- All event-summary categories now derive from that same event-ID-deduplicated
  catalog at every archive size. `All events` therefore reconciles exactly to
  `Dated + Undated + Reference notes`, including repeated archive layers.
- All events now displays archive-layer and provider counts together, explains
  their different meanings, and exposes reversible per-layer removal that does
  not delete raw data or generated Markdown.
- Diary DOCX chronology now recognizes year/month heading structures across
  German, English, abbreviated, and numeric month labels. Non-year topical
  sections remain truthful timeless reference notes instead of appearing as
  undated events or inheriting an invented date.
- Event filenames now combine sortable date/time, source, readable title, and a
  stable short ID. Existing event IDs are recognized across the older cryptic
  filename format so upgrade reruns do not create duplicate Markdown.
- All events separates Dated, genuinely Undated, and Reference notes; reference
  cards remain searchable and source-backed without pretending to be timeline
  dates.
- Matched Evidence disclosure typography to Read complete event and replaced
  the expanding native scrollbar with one stable thin custom scrollbar.
- Multi-archive summaries now derive distinct month groups from the merged,
  event-ID-deduplicated catalog instead of summing each archive's page count.
  Re-exporting the same source can no longer inflate the Months metric.
- Dark-mode typography now keeps ordinary text on the historic warm cream
  `#ded8cc`; near-white `#faf9f5` is reserved for page titles, headline
  metrics, the current next action, and recognized-format headings. Secondary
  copy remains muted cream `#b8b2a4`.
- Large Timeline year numerals now use the standard Segoe UI interface family
  instead of the legacy display serif.
- The All events filter toolbar now gives Search the same visible
  label-plus-control structure as Provider, Date, and Status. All four columns
  share the same top edge, control baseline, and outer vertical spacing.
- At production width, the Home promise is deliberately three lines and keeps
  “Conserve your memories in open files.” intact as the third line. Narrower
  windows release the no-wrap rule to prevent overflow.
- Home's archive and event-summary cards now use equal columns and content-sized
  height. The duplicate event-count hero and technical destination path were
  removed in favor of one compact browse row.
- Made the warmer coffee dark (`#181713`) the default appearance and removed
  the near-black page variant so the native and production-faithful web UI use
  one consistent dark surface.
- All events now remembers and merges distinct completed export archives instead
  of replacing the previous archive when a later source is converted. Nearby
  timeline siblings are restored locally, and exact reruns with the same event
  IDs are suppressed.
- Timeline event cards now lead with the human provider—Diary, Airbnb, Google
  Maps, and so on—before event-type and source details.
- Simplified Scan into two explicit source-entry routes and the four visible
  steps: choose source, confirm format, choose destination, create Markdown.
- Added single-file selection alongside folder selection and a prominent
  recognized-format announcement before export.
- Replaced the oversized application-card grid with a compact supported-app
  list; unavailable application routes remain clearly separated.
- Added event previews in All events and on-demand expansion of each complete
  Markdown body, including diary entries and messages.
- Changed the timeline filter from ambiguous event channel to provider/module,
  clarified validation versus identity-review labels, and made generated
  timeline sidecars Markdown-first by removing redundant JSON connection and
  quality files.
- macOS bundle build advanced to `202608041710`. Local Tauri release builds use
  ad-hoc signing so the generated application receives a complete resource
  seal automatically.

### 2026.8.1 beta build

- Calendar version advanced to `2026.8.1` with macOS bundle build
  `202608011823` and recompiled from the current validated source tree.
- Includes the quiet-start local-app permission hotfix, path-neutral portable
  UAT metadata, refreshed dependency lock, and clean public-source gate from
  the final `2026.7.27` hotfix cycle.
- Added a first-class Help module to the left navigation with an end-user quick
  start, supported-source truth, output explanation, privacy guarantees,
  troubleshooting, FAQs, and current limitations.
- Help now explains Unfold's community data-freedom purpose, zero-outbound-
  traffic local runtime, AGPL-3.0-or-later license, contribution address, and
  invitation to create privacy-preserving submodules and adapters.
- Home and Help now lead with the central memory-conservation promise and pair
  it with explicit human choice: Unfold is not surveillance, scoring, or a
  hidden dossier; the user chooses every source, module, and retained output.
- Podcasts are explicitly documented as a future fully local module candidate,
  not a working capability or release promise.
- Home and Help now answer why a person would use Unfold and what becomes
  possible after conversion: direct open-file use, optional exploration with a
  separately chosen truly local LLM, or an optional later Amara Memory layer.
- The app explains why Amara Memory must be more than document chat: a
  drillable Book, visual Life Map layers, evidence-based cross-source
  connections, and grounded answers that reveal supporting records. It also
  states honestly that this Unfold build contains no LLM or live Amara Memory
  bridge.
- Home, Help, and strategy documentation explicitly distinguish Unfold from
  “metadata extractor number 27”: extraction is only intake; the product value
  is cross-source memory normalization, provenance, quality visibility, shared
  timelines/indexes, and portable downstream use.

### 2026.7.27 beta package

- Version bumped to `2026.7.27` / macOS bundle build `202607271720`.
- README, how-to, transfer, and adapter docs now split sources into working
  primary paths, careful export routes, catalog-only recognition routes, and
  sources not available in this build.
- Apple Messages, Apple Notes, and Apple Journal are explicitly labelled as not
  available in this build, not as "coming soon" launch promises.
- Native UAT now saves a visual Scan/local-app screenshot artifact.

### Added

- Initial Tauri 2 desktop foundation with React and TypeScript.
- Cancellable local folder scanner and pluggable metadata extractors.
- SQLite index, organized migration, timeline views, and evidence candidates.
- Explicit local-only privacy surface and deny-by-default future bridge guard.
- Public architecture, privacy, repository-structure, trust, contribution, and
  security documentation.
- A fail-closed public-source audit and clean export command that reject private
  working memory, personal exports, secrets, archives, databases, logs, and
  generated build output.
- Playwright browser journeys and feature-gated native Tauri WebDriver acceptance tests.
- User and automated-testing guides.
- A generated, human-readable timeline with day, month, year, undated, entity,
  catalog, connection, and data-quality indexes over exported event Markdown.
- Evidence-only cross-source linking by date, shared entity ID, event spans, and
  direct actor/place metadata, available from both the native app and CLI.
- A self-contained second-Mac transfer kit with binary-first instructions for
  an assisting LLM, component/runtime documentation, SHA-256 verification, and
  reproducible packaging of the desktop app and CLI.
- Eighteen direct offline extraction routes for Airbnb, Apple Music, banking
  CSV and XLSX, diary DOCX, Gmail MBOX, PST mail, Google Maps POI/CSV/KMZ,
  photos, Google Photos Takeout, YouTube Takeout, Apple Mail `.mbox`, Apple
  Calendar `.ics`, Apple Contacts `.vcf`, and Immich local originals.
- The standalone Amara Unfold Open Event Schema v1, automatic per-package
  quality reports, balanced skip-reason ledgers, a 99.50% acceptance gate, and
  data-quality cards in the desktop interface.
- AGPL-3.0-or-later licensing, SPDX source headers, dependency-license report,
  contribution/security templates, and fail-closed source/history audits.

### Changed

- Home's primary folder action now opens the native picker directly.
- Quiet UI text colors now meet automated WCAG AA contrast checks.
- Event frontmatter quotes date-like YAML scalars for portable downstream
  parsing and visibly normalizes damaged source characters.
- Light and dark themes now use only the Amara Memory paper, ink, and warm
  neutral values; colored semantic accents and the active-navigation halo were
  removed.
- The native application icon now uses the same near-black background and
  cream `a` while preserving its approved six-pixel-left alignment.
- The native icon's cream `a` is five percent smaller, retaining the same
  dark background and left-shifted visual center.
- Text inputs now decode UTF-8 or Windows-1252, normalize line endings, repair
  common mojibake, and visibly mark unrecoverable source characters instead of
  guessing personal values.
- Archive and document readers now enforce entry-count, expanded-size,
  compression-ratio, message-size, and path-traversal limits.

### Fixed

- Gmail MBOX extraction now decodes recursive multipart and transfer encodings,
  prefers plain text, converts HTML fallback to text, and excludes attachments.
- PST extraction now infers sent mail from mailbox folders and owner aliases,
  recovers plain/HTML/compressed-RTF bodies, and distinguishes absent source
  bodies from present-but-undecodable bodies.
- Banking CSV and XLSX routes no longer cross-read each other's formats;
  counterparties are normalized without discarding the original transaction
  purpose and all skipped rows receive an explicit reason.
- KMZ, banking, mail, and photo skip totals are now auditable and balance their
  manifests; macOS AppleDouble files are excluded and removed from packages.
