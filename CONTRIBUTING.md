# Contributing

Thank you for helping build Amara Unfold. This repository may handle the most
personal files people own, so clarity and restraint matter as much as features.

Unfold exists to help people move personal history out of digital fenced
gardens and into open files they control. It is licensed under the GNU Affero
General Public License v3.0 or later. Questions, module proposals, and offers to
contribute are welcome at `info@amara.gmbh`.

## Before opening a change

- preserve fully local operation and zero outbound traffic—do not add network
  calls, telemetry, update checks, or cloud APIs;
- use synthetic fixtures—never commit real exports, metadata, names, addresses,
  account identifiers, archive paths, databases, screenshots, or logs;
- preserve the boundaries described in `docs/architecture.md`;
- explain every new native permission, dependency, and data flow;
- add focused tests when changed cancellation, persistence, extraction or privacy
  behavior needs regression protection; avoid tests that mirror implementation;
- update documentation when a guarantee or limitation changes;
- run the public-source audit for the submitted payload and select relevant
  checks from `docs/testing.md`; reuse accepted results for unchanged code and
  do not rebuild or rerun full suites for documentation-only changes.

The public-source audit is intentionally fail-closed. Do not weaken it to make a
convenient fixture pass. Generate the smallest synthetic fixture in a test
temporary directory, or document and review a narrowly scoped source asset.

Comments should explain invariants and non-obvious decisions. Avoid narrating
syntax that the code already makes clear.

## New modules and adapters

The current stabilization phase freezes new importers and prioritizes reliable
existing advertised routes. Proposed future submodules can be discussed before
implementation. Useful future contributions include new
export adapters, local metadata extractors, validators, timeline/index views,
and accessible end-user workflows. A new module must:

- operate locally and preserve zero outbound traffic by default;
- read only data the user deliberately selects;
- leave originals untouched and write portable output;
- use synthetic fixtures and focused automated tests;
- document its source format, limitations, permissions, and output contract;
- avoid creating a new proprietary dependency or data lock-in.

Write to `info@amara.gmbh` if you want to discuss a format or module before
implementing it.

## Style

Rust is formatted with `rustfmt` and linted with Clippy warnings denied.
TypeScript is strict, formatted with Prettier, and linted with ESLint.
User-facing language should distinguish observed metadata from inference.

## Commits

Use focused commits with an imperative summary. Do not include generated
databases, local logs, secrets, or real filesystem paths.

Before pushing, inspect the exact payload:

```sh
npm run audit:public
git status --short
git diff --check
```

If the project was developed alongside private notes or test corpora, export a
fresh public tree with `npm run export:public -- <empty-directory>`. Never
publish a private development history merely because the current checkout looks
clean.
