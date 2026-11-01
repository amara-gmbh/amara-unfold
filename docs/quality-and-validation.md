# Data quality and validation

Quality evidence is generated automatically for every adapter run. It is part
of the output contract, not an optional developer dashboard.

## Per-adapter accounting

Each package records `recordsSeen`, `eventsWritten`, `eventsSkipped`, `errors`,
and `skipReasons` in `manifest.yaml`. A run is balanced only when:

```text
recordsSeen = eventsWritten + eventsSkipped
eventsSkipped = sum(skipReasons)
```

Known skip reasons include `duplicate_event`, `unchanged_rerun`,
`missing_date`, `missing_timestamp`, and `missing_coordinates`. The manifest
must name the reason; a raw skipped count is not enough for auditability.

## Automatic report

The validator reads every emitted event and records:

- required-schema and provenance failures;
- invalid UTF-8 replacement characters and visible source-normalization marks;
- missing mail bodies, residual HTML/CSS/JSON-LD, and implausible PST direction
  classification;
- noisy banking counterparties, including bare `Ga`/`merchant-ga` ATM residue;
- implausible banking years before 1990 or after the current year plus one;
- adapter extraction errors and unbalanced record counts.

`quality-report.json` is machine-readable. `quality-report.md` is the compact
human audit. The desktop result screen shows a neutral source glyph, status,
score, counts, and expandable warning categories for every completed adapter.

The current acceptance threshold is 99.50% clean events, with zero hard
invariant failures. Irrecoverable damage already present in a source is kept
visible as `[source character unavailable]`; it is never silently replaced with
a guessed personal value.

A PST record whose body properties are genuinely absent is counted separately
as `sourceEmptyBodyRecords` and does not fail extraction quality. A body
property that exists but cannot be decoded remains a hard mail-body warning.

Two-digit banking years are expanded before event IDs and output paths are
created (`24` becomes `2024`; `99` becomes `1999`). ATM/cash-withdrawal machine
labels stay in the descriptive evidence but do not become counterparties,
actors, or merchant tags.

Unfold also requires two consecutive identical quality scans before it writes
the final report. If three scans do not produce a stable result, completion
fails visibly and asks the user to retry in an isolated destination. This
prevents a report from describing files that were still settling on disk.

## Release validation

Public tests use only synthetic fixtures. Private pre-release validation may
use a local corpus, but only aggregate counts and redacted findings may leave
that machine. A route is not called verified merely because it was recognized:
it must extract, balance its manifest, pass schema/provenance checks, and have
its domain-specific warnings reviewed.

The independent black-box QA validator is intentionally stored outside the
public repository so a private corpus, owner-address rules, and reports cannot
leak into source history.

## Failed, cancelled, and mixed-format runs

Each adapter writes into a temporary package beside its final destination.
On a rerun, this starts as a copy of the last completed package. Parsing and
quality checks finish before the new package replaces that destination.
A parsing failure or Stop extraction discards the temporary work; prior event
files, manifest, and quality reports remain unchanged. The output folder must
be outside a selected source directory so generated files cannot be re-ingested.

Cancellation applies to the current export and prevents later batch exports
from starting. Earlier completed exports remain saved. A batch with successful
and failed exports shows “Some exports need attention”; review the named failures
and rerun those sources after correcting them. Banking CSV and workbook exports
are separate batch routes, even when both occur in the same input folder.
An input with no readable records is not counted as a completed export.

This is per-package rollback for handled errors and cancellation, not a
whole-batch transaction or a power-loss recovery guarantee. Staging directories
beginning `.unfold-staging-` are excluded from Timeline discovery. If publication
and restoration both fail, the error names the retained previous package for
manual recovery. Do not delete that recovery directory before inspecting it.

## Loaded archive counts

Home and All events count the same currently loaded catalog entries, deduplicated
by event ID. An empty readable catalog counts as zero. During loading, the UI
labels counts as loading; unreadable catalogs are reported separately rather
than silently replaced by cached manifest totals. Cached day-page totals from
overlapping archives are not shown as a unique global day count.
