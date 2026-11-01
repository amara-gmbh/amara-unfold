# Security policy

Amara Unfold is maintained by
[amara information security GmbH](https://amara.gmbh).

Please do not post sensitive archive paths, metadata, images, personal exports,
or exploit details in a public issue. Send private reports to
`info@amara.gmbh` with `[amara Unfold security]` in the subject. Do not attach a
real personal archive; use the smallest synthetic reproducer that demonstrates
the issue. GitHub private vulnerability reporting may be added later, but it is
not required to use this private email channel.

A useful private report identifies the affected version, expected and observed
behavior, impact, and a minimal synthetic reproduction. Maintainers should
acknowledge receipt, keep the reporter informed, and publish a clear remediation
record once disclosure is safe.

Supported security fixes currently target the latest source on `main`. The
project is pre-release and does not yet distribute signed production binaries.

## Untrusted input boundary

Unfold parses attacker-controlled or malformed PST, MBOX, ZIP, KMZ/KML, DOCX,
XLSX/ODS, CSV, JSON, TSV, images, and videos. Parsers must assume filenames,
headers, archive metadata, text encodings, and claimed sizes are untrusted.

Current safeguards include read-only source access, no symbolic-link traversal,
archive path validation, entry-count and expanded-size ceilings, compression
ratio checks, bounded message/document reads, local-only operation, and a
fail-closed quality report. These controls reduce risk; they are not a claim
that third-party parsers are free of vulnerabilities.

Reports are especially useful when they cover path traversal, decompression
bombs, memory exhaustion, parser panics, unsafe native behavior, provenance
confusion, silent record loss, or any unexpected network access. Use only a
minimal synthetic reproducer.
