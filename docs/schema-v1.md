# amara Unfold Open Event Schema v1

This document defines a standalone, open Markdown event format. Any tool may
emit, read, validate, or extend it. amara Unfold is one implementation. amara
Memory is planned and not available yet; no Memory application or import bridge
is included or required to use these files.

## File shape

Each event is one UTF-8 Markdown file. YAML frontmatter is the machine-readable
record; the Markdown body preserves human-readable source content.

The stable identity lives in frontmatter `id`, not in the filename. amara
Unfold filenames use `YYYYMMDD-HHMM-source-readable-title-shortid.md` so files
sort predictably and remain recognizable outside the application. Consumers
must not derive event identity or semantics from a filename.

```markdown
---
schema_version: 1
id: 2026-07-19-example-a1b2c3
date: "2026-07-19"
date_precision: day
time: "18:07"
source: example
module: example
type: example_event
actors: []
places: []
title: Synthetic example
tags:
  - example
confidence: high
provenance:
  raw_path: synthetic/example.json
  raw_hash: sha256:0000000000000000000000000000000000000000000000000000000000000000
  ingested_at: "2026-07-19T18:07:00Z"
  module_version: example/1.0
links: []
revisions: []
---

Synthetic body text.
```

## Required fields

| Field            | Rule                                                                    |
| ---------------- | ----------------------------------------------------------------------- |
| `schema_version` | Integer `1` for this specification.                                     |
| `id`             | Stable event identifier; deterministic emitters are strongly preferred. |
| `date`           | Quoted `YYYY-MM-DD`; unknown components use `00`.                       |
| `date_precision` | `minute`, `day`, `month`, `year`, or `unknown`.                         |
| `source`         | Portable source/adaptor identifier.                                     |
| `module`         | Normalization domain such as `mail` or `banking`.                       |
| `type`           | Event type within that domain.                                          |
| `title`          | Short human-readable label.                                             |
| `provenance`     | Source path, SHA-256 digest, ingestion time, and module version.        |

`actors`, `places`, `tags`, `links`, and `revisions` are arrays and should be
present even when empty. `confidence` describes the evidence level and must not
turn inference into fact.

## Date and precision rules

- A known day is `2026-07-19` with `date_precision: day`.
- A known month is `2026-07-00` with `date_precision: month`.
- A known year is `2026-00-00` with `date_precision: year`.
- An undated event is `0000-00-00` with `date_precision: unknown`.
- `time` is present only when the source supports it; current emitters store
  minute precision as `HH:MM`.
- Missing dates or times are never invented for sorting convenience.

### Manual dating of genuinely undated events

An owner may optionally assign a defensible year, month, day, or day-and-time
to an event whose original date is `0000-00-00`. amara Unfold records each set,
later correction, and undo operation in `revisions` using a versioned
`manual-date-v1` entry. The first entry preserves the original unknown date;
undo restores that state. Consumers that do not understand the revision entry
may treat it as an opaque string.

Manual dating must never change `id`, `provenance`, the Markdown evidence body,
or the raw source. It must not be offered for timeless reference notes or used
to replace a date already supplied by the source. The event file may move to a
different `events/YYYY/MM/` folder and receive a new meaningful filename, but
its frontmatter `id` remains the durable identity.

## Provenance rules

`provenance.raw_path` identifies the source record or container path as seen by
the local adapter. `raw_hash` uses `sha256:<hex>`. `ingested_at` is an RFC 3339
timestamp for processing, not the event date. `module_version` identifies the
normalizer that emitted the event.

Public fixtures must use synthetic portable paths. A user's absolute path may
appear in their private generated output, but must never be committed to the
public source repository.

## Compatibility and extensions

Readers must ignore unknown frontmatter fields they do not understand. Writers
must not change the meaning of fields defined here. A breaking semantic change
requires a new `schema_version`. Markdown bodies remain readable even when a
consumer does not understand every structured field.

## Validation

Every adapter package contains `manifest.yaml`, `quality-report.md`, and
`quality-report.json`. The report checks schema, provenance, encoding, record
accounting, and domain-specific invariants. The CLI can validate a package with:

```sh
amara-unfold validate ./path/to/package
```
