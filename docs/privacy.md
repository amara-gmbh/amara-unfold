# Privacy model

Amara Unfold is the trust surface before Amara Memory. Privacy claims are
implemented as architecture and permissions, not only interface copy.

## Current guarantees

- no HTTP client is included in the runtime dependency graph;
- no analytics, telemetry, crash upload, auto-update, or cloud SDK is configured;
- the network guard denies every outbound intent;
- folder access begins with a native, user-initiated selection;
- the scanner does not follow symbolic links;
- originals are opened read-only and never copied, renamed, or deleted;
- index writes go to a local SQLite database in the OS app-data directory;
- scan traversal errors are stored locally and surfaced in the Scan workspace;
- cancelled scans preserve the previous complete index;
- CSP and Tauri capabilities are explicit and narrow;
- local databases and environment files are excluded from version control.

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
