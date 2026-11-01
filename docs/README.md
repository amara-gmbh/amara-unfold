# amara Unfold documentation

This is the public table of contents for amara Unfold. Start with the reading
path that matches what you want to do, then use the complete catalogue below.

## First reads

| You want to…                             | Start here                                        |
| ---------------------------------------- | ------------------------------------------------- |
| Understand the product in five minutes   | [Project README](../README.md)                    |
| Understand the complete system           | [A-to-Z system guide](system-guide.md)            |
| Use the desktop application              | [User guide](how-to-use.md)                       |
| Build the application                    | [Build from source](build-from-source.md)         |
| Understand why it is designed this way   | [Design and decisions](decisions.md)              |
| Find a configuration or version setting  | [Configuration reference](configuration.md)       |
| Review privacy and security claims       | [Trust and audit](trust-and-audit.md)             |
| Add or review an extraction route        | [Open adapter contract](open-adapters.md)         |
| Understand the generated Markdown format | [Open Event Schema v1](schema-v1.md)              |
| See what changed                         | [Public changelog](../CHANGELOG.md)               |
| Check the frozen release boundary        | [Limited-feature release scope](release-scope.md) |

## The system from top to bottom

1. [Project README](../README.md) — mission, current promise, supported paths,
   repository shape, and build entry points.
2. [A-to-Z system guide](system-guide.md) — source selection through Markdown,
   Timeline, local indexing, application modules, and release boundaries.
3. [Architecture](architecture.md) — runtime flow and boundaries between Rust,
   Tauri, React, SQLite, and future bridges.
4. [Component manifest](component-manifest.md) — concrete source components,
   runtime storage, deliverables, and data formats.
5. [Configuration reference](configuration.md) — where each build, runtime,
   security, test, and CI setting lives.
6. [Design and decisions](decisions.md) — the reasoning behind the major
   product and engineering choices.

## Use and output

- [How to use amara Unfold](how-to-use.md) — source selection, recognition,
  extraction, Timeline, All events, Photo drafts, settings, and troubleshooting.
- [Open Event Schema v1](schema-v1.md) — portable Markdown frontmatter,
  date precision, provenance, manual dating, and compatibility rules.
- [Data quality and validation](quality-and-validation.md) — accounting,
  automatic checks, quality labels, and release acceptance.
- [Adapter status](adapters.md) — executable routes and recognition-only or
  normalization routes.
- [Limited-feature release scope](release-scope.md) — included journeys,
  exclusions, cancellation guarantees, and distribution status.
- [Open adapter contract](open-adapters.md) — recognition-first extension
  model and CLI contract.

## Privacy, security, and trust

- [Privacy model](privacy.md) — what is read, written, stored, and deliberately
  not transmitted.
- [Trust and audit](trust-and-audit.md) — public claims, evidence, limitations,
  source audit, and review workflow.
- [Security policy](../SECURITY.md) — reporting security issues safely.
- [Repository structure](repository-structure.md) — strict separation of
  editable source, generated builds, the installed app, and user data.
- [Third-party licences](third-party-licenses.md) — generated dependency
  licence inventory.

## Development and testing

- [Build from source](build-from-source.md) — pinned environment and clean
  build procedure.
- [Configuration reference](configuration.md) — JavaScript, Rust, Tauri,
  capabilities, browser/native test, and CI files.
- [Automated testing strategy](testing.md) — unit, browser, native Tauri, and
  deep-gate responsibilities.
- [Local-app intake testing](local-app-uat.md) — public synthetic and visible
  candidate checks for application routes.
- [Contributing](../CONTRIBUTING.md) — contribution standards and synthetic
  fixture requirements.
- [Code of conduct](../CODE_OF_CONDUCT.md) — community expectations.

## Distribution and transfer

- [Component manifest](component-manifest.md) — what belongs in source and in a
  distributed binary.
- [Limited-feature release scope](release-scope.md) — the exact public promise
  for version `2026.9.21`.
- [Transfer package guide](transfer-readme-first.md) — verifying and operating
  a binary transfer package.
- [Second-Mac LLM instructions](second-mac-llm-instructions.md) — constrained
  operating contract for an assisting local agent.
- [Independent local-app acceptance checklist](claude-local-app-approval.md) —
  tool-neutral candidate review retained under its compatibility filename.

## History and rationale

- [Public changelog](../CHANGELOG.md) — release-facing additions, changes, and
  fixes.
- [Design and decision record](decisions.md) — curated explanations of the
  choices that materially shape public behavior.
- [Licence](../LICENSE) and [notice](../NOTICE.md) — legal terms, publisher, and
  attribution.

Raw development conversations and private working memory are not published.
They may contain local paths, recovery context, or private test evidence.
Every decision that affects public behavior must instead be restated in the
public changelog, decision record, relevant contract, or source comments so the
public repository remains independently understandable.

## Authority and precedence

When two public documents appear to disagree, use this order:

1. executable behavior and its passing tests;
2. the Open Event Schema and explicit privacy/security contracts;
3. the current README, system guide, adapter status, and user guide;
4. the component/configuration references;
5. older changelog entries and historical transfer documents.

An outdated claim is a documentation defect. Please report it without attaching
personal archives, local paths, or secrets.
