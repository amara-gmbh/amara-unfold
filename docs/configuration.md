# Configuration reference

This document maps the public configuration files that control amara Unfold.
It explains ownership and intent; the files themselves remain authoritative.

## Workspace and dependency configuration

| File                        | Purpose                                                      |
| --------------------------- | ------------------------------------------------------------ |
| `package.json`              | JavaScript workspace, version, quality/build/test commands   |
| `package-lock.json`         | Exact JavaScript dependency resolution                       |
| `apps/desktop/package.json` | React/Tauri renderer dependencies and workspace commands     |
| `Cargo.toml`                | Rust workspace members, shared dependencies, release profile |
| `Cargo.lock`                | Exact Rust dependency resolution                             |
| `crates/*/Cargo.toml`       | Per-crate dependencies and library/binary declarations       |
| `rust-toolchain.toml`       | Pinned Rust toolchain and components                         |

Lockfiles are committed intentionally. A recovery build should begin with
`npm ci`, not `npm install`, so the committed dependency graph is respected.

## Desktop and native application configuration

| File                                               | Purpose                                              |
| -------------------------------------------------- | ---------------------------------------------------- |
| `apps/desktop/vite.config.ts`                      | Renderer build and development server                |
| `apps/desktop/tsconfig.json`                       | Renderer TypeScript rules                            |
| `apps/desktop/tsconfig.node.json`                  | Build-tool TypeScript rules                          |
| `apps/desktop/eslint.config.js`                    | JavaScript/TypeScript lint policy                    |
| `apps/desktop/src-tauri/tauri.conf.json`           | Product identity, window, CSP, bundle, icon, version |
| `apps/desktop/src-tauri/capabilities/default.json` | Ordinary native permissions                          |
| `apps/desktop/src-tauri/tauri.e2e.conf.json`       | Test-only Tauri/WebDriver overrides                  |
| `apps/desktop/src-tauri/build.rs`                  | Tauri build integration                              |

The ordinary application uses `tauri.conf.json` and the `default` capability.
The E2E configuration is feature-gated, has a different bundle identifier, and
must never silently enter an ordinary build.

## Product identity and versions

The public product version is currently stored in:

- root `package.json`;
- `apps/desktop/package.json`;
- root `Cargo.toml` workspace package version;
- `apps/desktop/src-tauri/tauri.conf.json`.

The macOS bundle build number is stored as
`bundle.macOS.bundleVersion` in `tauri.conf.json`. The bundle identifier is
`com.amaramemory.unfold`; the E2E identifier is separate.

When changing a release version, update all version locations together, run
format/compiler/tests, build the app, and inspect the resulting `Info.plist`.

## Adapter and data contracts

| File                           | Purpose                                        |
| ------------------------------ | ---------------------------------------------- |
| `adapters/catalog.json`        | Public route IDs, labels, formats, recognition |
| `adapters/catalog.schema.json` | Validation schema for the adapter catalogue    |
| `docs/schema-v1.md`            | Portable event Markdown contract               |
| `crates/db/migrations/*.sql`   | Ordered local SQLite index schema              |

The catalogue helps explain and recognize routes. Executable behavior and
tests remain decisive. A parser must not be advertised as user-ready until its
full installed-app route has passed acceptance.

## Test configuration

| File                                         | Purpose                                   |
| -------------------------------------------- | ----------------------------------------- |
| `tests/browser/playwright.config.ts`         | Browser journeys and local preview server |
| `tests/native/wdio.conf.ts`                  | Native Tauri/WebdriverIO acceptance       |
| `apps/desktop/src/testing/browserBridge.ts`  | Deterministic browser-only desktop bridge |
| `apps/desktop/src-tauri/tauri.e2e.conf.json` | Native test-only capabilities             |

Generated test reports, screenshots, traces, videos, compiled binaries, and
temporary fixtures are ignored. Public fixtures must be synthetic.

## Formatting and repository policy

| File                                      | Purpose                                                          |
| ----------------------------------------- | ---------------------------------------------------------------- |
| `.gitignore`                              | First-line public/private and generated-file boundary            |
| `.gitattributes`                          | Cross-platform Git text behavior                                 |
| `.prettierignore`                         | Root formatting exclusions                                       |
| `apps/desktop/.prettierignore`            | Desktop formatting exclusions                                    |
| `apps/desktop/src-tauri/.gitignore`       | Tauri-generated schema/target exclusions                         |
| `scripts/audit-public-source.mjs`         | Content-aware public-source release gate                         |
| `scripts/audit-git-history.mjs`           | All-ref history, company-identity, secret, and private-path gate |
| `scripts/public-export-boundary.test.mjs` | Positive export and negative private-tool/root-allowlist fixture |

`.gitignore` is not a security boundary for already tracked files. Both audit
scripts must pass before a public push or release.

## Continuous integration

Hosted CI is currently disabled for this repository.
`.github/workflows/ci.yml` is a reference configuration for public quality gates
on macOS and Windows, with push-to-`main` and pull-request triggers defined in
the file. If enabled, its `contents: read` permission allows it to test an
uploaded commit, not push source changes. The file alone proves no execution.

The configured Rust advisory job would use the repository-provided
`GITHUB_TOKEN` only during a workflow run. Application runtime behavior does
not use that token.

## Environment and secrets

The ordinary application does not require a `.env` file, API key, cloud token,
or account. Environment files, private keys, signing certificates, and local
credentials are rejected from public source.

Developer ID signing and notarization are release operations outside public
source. Their credentials must come from a protected local keychain or a
reviewed CI secret store, never a committed file.

## Command index

| Command                        | Purpose                                                     |
| ------------------------------ | ----------------------------------------------------------- |
| `npm ci`                       | Restore exact JavaScript dependencies                       |
| `npm run dev`                  | Start the ordinary local development app                    |
| `npm run build`                | Type-check and build the production renderer                |
| `npm run check`                | TypeScript and Rust compiler checks                         |
| `npm run test`                 | Frontend and Rust tests                                     |
| `npm run lint`                 | ESLint and Clippy with warnings denied                      |
| `npm run test:browser`         | Deterministic browser acceptance                            |
| `npm run test:native`          | Real Tauri IPC/filesystem/database acceptance               |
| `npm run audit:public`         | Public-tree content audit plus export-boundary regression   |
| `npm run test:public-boundary` | Focused private-continuity exclusion regression             |
| `npm run audit:history`        | Reachable Git-history audit                                 |
| `npm run audit:licenses`       | Dependency-licence inventory check                          |
| `npm run build:app:debug`      | Build and strict-verify an ad-hoc signed debug macOS bundle |
| `npm run test:deep`            | Complete local release-quality gate                         |

See [Build from source](build-from-source.md) and [Testing](testing.md) for the
operating sequence and acceptance meaning.
