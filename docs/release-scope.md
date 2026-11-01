# Limited-feature release scope

Version `2026.9.21` freezes the currently accepted feature set. This release is
for local conversion of explicitly selected exports into open Markdown. It does
not add importers, cloud services, background access, or an amara Memory bridge.

## Included user journeys

The desktop release presents these primary paths:

- Diary DOCX with supported chronological headings.
- Banking CSV, XLS, XLSX, and ODS transaction exports.
- Extracted Airbnb JSON export folders.
- Google Maps saved places in supported CSV, JSON/GeoJSON, KMZ, and recognized
  Takeout shapes.
- Exported Apple Photos/media folders, Calendar ICS, Contacts vCard, and Apple
  Mail MBOX packages.
- Apple Music local metadata and prepared TSV/audio inputs, with Timeline dates
  limited to explicit `Date Added` evidence.
- Recognized Google Takeout Photos, Gmail, and YouTube archive shapes.

The CLI exposes the same fourteen registered release routes. Two additional
catalog entries only recognize Markdown that is already open and portable; they
do not run extraction again.

## Reliability boundary

- Raw inputs remain unchanged.
- Adapter output is written in private staging and published only after parsing
  and validation finish.
- A cancelled or failed run does not replace the last completed package.
- Cancelling final Timeline indexing preserves the previous complete Timeline;
  successfully completed Markdown exports remain saved and are identified as
  newer than that retained index.
- Reruns report newly written and unchanged retained events separately instead
  of presenting retained output as a zero-file success.

These guarantees cover handled errors and cancellation. They are not a claim of
power-loss atomicity or concurrent writers targeting the same destination.

## Excluded from this release

- Outlook PST and Immich are not registered or advertised release routes.
- Apple Messages, Notes, and Journal are unavailable.
- Direct Airbnb ZIP intake, live Photos database access, attachment export,
  recurrence expansion, and new importers are outside the frozen scope.
- No account, sync, telemetry, hosted extraction, LLM, or live amara Memory
  bridge is included.

Internal source may retain dormant experimental code for later review. It is not
part of the release promise unless registered here and exposed by the current
catalog.

## Source and macOS distribution

The GitHub source is licensed under AGPL-3.0-or-later and is sufficient to audit,
test, and build the application. The repository excludes private project memory,
raw exports, generated personal Markdown, runtime databases, secrets, build
output, and signing credentials.

The current macOS candidate is ad-hoc signed, not Developer ID signed, and not
notarized. Building from source is the supported route until a separately
approved binary is published. Any unsigned/ad-hoc download must be labelled as
such and must never be presented as a notarized production application.
