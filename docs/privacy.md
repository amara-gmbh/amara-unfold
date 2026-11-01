# Privacy model

amara Unfold processes selected sources locally. amara Memory is planned and
not available yet; this release contains no Memory application or import
bridge. Privacy statements below distinguish implemented behavior from status
declarations and future extension points.

## Current behavior and boundaries

- no HTTP client is included in the runtime dependency graph;
- no analytics, telemetry, crash upload, auto-update, or cloud SDK is configured;
- `NetworkGuard` reports the local-only status and provides a placeholder for
  future outbound policy; it is not an active network firewall;
- source-content scanning and extraction begin with an explicit selection;
  local-app discovery can check other known locations as described below;
- the scanner does not follow symbolic links;
- originals are opened read-only and never copied, renamed, or deleted;
- index writes go to a local SQLite database in the OS app-data directory;
- scan traversal errors are stored locally and surfaced in the Scan workspace;
- cancelled scans preserve the previous complete index;
- CSP and Tauri capabilities are explicit and narrow;
- local databases and environment files are excluded from version control.

### Local-app discovery is separate from content import

At startup, the app requests discovery without user-location inspection, while
standard installed-application checks can still occur. Choosing **Refresh app
exports**, or preparing Mail export discovery, enables checks of known
Calendar/Contacts export folders and the prepared Mail export folder. These
checks can inspect directory names, file extensions, and MBOX package shape.
The same discovery call also checks whether known Messages, Notes, and Journal
locations exist, even when a different app export is the one of interest. It
does not import their contents or enable those unavailable import routes.

After Mail export preparation, discovery can repeat while waiting for the
export or when the app regains focus. Reading source records for conversion
still requires choosing the relevant source and starting extraction.

### Network status is not network enforcement

The application calls `NetworkGuard.status()`. Its `authorize()` method always
returns a denial, but that method is a placeholder exercised by a unit test,
not a runtime interception point for outbound traffic. The local-only
description rests on the configured runtime features, dependencies, CSP and
Tauri capabilities; a reported off state is not a general firewall guarantee.

## Data stored locally

For every file in a selected folder, the index may contain:

- absolute path and filename;
- extension and media classification;
- byte size and filesystem timestamps;
- BLAKE3 content hash for supported media;
- available image dimensions and duration;
- available EXIF capture date, GPS coordinates, and device make/model;
- extractor name and local extraction errors;
- deterministic cluster/candidate evidence.

Absolute paths and metadata can be sensitive even when original media never
moves. Treat the app database as personal data.

## Deliberate limitations

The first release does not encrypt the SQLite database. It relies on operating
system account security and full-disk encryption such as FileVault or BitLocker.
Application-level encryption and a clear index-deletion interface are release
requirements to evaluate before handling shared-device threat models.

Content hashing reads recognized media in full. Unknown file types are counted
and their filesystem metadata indexed, but their contents are not opened for
extraction.

## Network changes

Any future network feature must be separately reviewable and must not weaken
local-only behavior. At minimum it needs:

1. a visible destination and exact payload preview;
2. affirmative, per-operation user consent;
3. no preselected upload or sync option;
4. cancellation and a local audit record;
5. tests proving denial without authorization;
6. updated capability, dependency, and privacy documentation.

Report privacy vulnerabilities through the private process in
[SECURITY.md](../SECURITY.md), not a public issue containing archive details.
