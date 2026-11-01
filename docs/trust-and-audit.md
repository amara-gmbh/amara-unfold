# Trust and audit

Trust is not a slogan for Amara Unfold. Each public claim needs a narrow scope,
inspectable implementation, a test or audit where practical, and a documented
limitation where the claim stops.

Amara Unfold’s declared publisher is
[amara information security GmbH](https://amara.gmbh). The publisher is
declared in the JavaScript and Rust package metadata, the Tauri bundle metadata,
and `NOTICE.md` so source and binaries can present the same identity.

## Claims and evidence

| Claim                                                       | Inspectable evidence                                                                                                                                                                               |
| ----------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The current build is local-only                             | `crates/core/src/privacy.rs` denies outbound bridge operations; `docs/privacy.md` defines the boundary; runtime manifests contain no cloud SDK or HTTP client.                                     |
| A scan starts with explicit user action                     | The Tauri dialog capability and desktop command flow are visible under `apps/desktop/src-tauri/` and `apps/desktop/src/lib/desktop.ts`.                                                            |
| Originals are not changed                                   | Scanner and adapter code open sources for reading, output is written to a separate selected destination, and tests exercise the pipeline with temporary synthetic data.                            |
| Folder traversal does not escape through symlinks           | `crates/scanner` rejects symbolic-link traversal and has focused tests.                                                                                                                            |
| Derived records retain provenance                           | Adapter and Markdown-writer code emit source paths, hashes, deterministic IDs, manifests, and evidence; end-to-end tests verify stable output.                                                     |
| Public source contains no working memory or personal corpus | `scripts/audit-public-source.mjs` fails on private-data directories, raw export formats, runtime databases, archives, logs, secrets, user paths, sidecars, symlinks, and unreviewed binary assets. |
| Published dependencies and permissions are reviewable       | Both lockfiles are included; Tauri capabilities are explicit; CI is configured for macOS and Windows.                                                                                              |
| The project can be independently modified                   | The release candidate includes build instructions, the AGPL-3.0-or-later license, and an audited public-source export.                                                                             |
| The publisher is explicit                                   | Package manifests, Tauri bundle metadata, `NOTICE.md`, and this document identify amara information security GmbH and amaramemory.com.                                                             |

These are claims about the reviewed source. A downloaded binary also requires
provenance, checksum, signing, and release-process evidence before a reviewer
can conclude that it corresponds to that source.

## Known limitations

- Version `2026.9.21` is a limited-feature release candidate; see
  [release scope](release-scope.md).
- Local development bundles have a verified ad-hoc seal, but no Developer ID
  signature or Apple notarization for public distribution.
- Local checks apply to the recorded source and environment. CI configuration
  does not establish a current remote run or bit-for-bit reproducible binaries.
- Known dependency findings and concrete release actions are recorded in
  [Build from source](build-from-source.md#source-package-and-release-decisions--2026-09-07).
- The local SQLite index is not application-level encrypted. Device account
  security and full-disk encryption protect it today.
- Format support is deliberately limited. PST, Immich, Messages, Notes, and
  Journal are outside the registered release routes; missing metadata is
  reported as unavailable rather than inferred.
- A future network bridge is an architectural boundary, not a current feature.
  Adding it requires explicit payload review, per-operation consent, denial
  tests, a local audit record, and updated privacy documentation.

Limitations should be removed from this list only when implementation,
documentation, and verification change together.

## Public-source release gate

Run the audit from the repository root:

```sh
npm run audit:public
```

The audit walks the public source and fails closed. It rejects:

- any top-level path outside the reviewed public-root allowlist;
- private working-memory and raw-data directory names;
- mail archives, databases, office documents, media, tabular exports, and
  compressed archives;
- environment files, credentials, private keys, non-placeholder email
  addresses other than the explicitly reviewed public project contact,
  absolute user paths, logs, and macOS sidecars;
- symbolic links, unexpected binary files, and files too large for ordinary
  source review;
- canonical-only continuity tooling and private package commands;
- missing license, security, lockfile, privacy, architecture, or trust docs.

Reviewed application icons, including the canonical root icon master, are the
only allowed binary assets. Synthetic test inputs are generated inside
temporary directories instead of being committed as real-looking export files.

CI runs this audit before dependency installation. A pull request cannot rely
on `.gitignore` alone to hide a sensitive tracked file.

## Creating a clean public repository

Private development notes or corpora can remain outside the public project. To
create a history-free public tree in an empty directory:

```sh
npm run export:public -- /path/to/empty/amaraUnfold
```

The export command excludes Git history, private working directories,
dependencies, compiler output, test artifacts, local databases, raw export
formats, archives, logs, environment files, sidecars, and canonical-only
continuity tooling. It copies only the reviewed top-level allowlist and removes
private-only package commands before running the same public-source audit against
the result. It refuses to overwrite a non-empty destination, except that an
empty pre-initialized `.git` directory is allowed.

Before the first push, review what Git would publish:

```sh
git status --short
git add --dry-run .
npm run audit:public
```

Starting a fresh public history is essential when an earlier private history
ever contained personal material. Removing a file in a later commit does not
remove it from earlier commits.

## Reviewing changes

For every privacy- or security-relevant change, reviewers should be able to
answer:

1. What new data can the code read, derive, retain, or write?
2. Which user action authorizes it?
3. Can the operation cross the selected folder or use the network?
4. Where is the behavior implemented, surfaced, and tested?
5. What fails safely, and what limitation remains?

If those answers are not clear from source, tests, and documentation, the
change is not ready to merge.
