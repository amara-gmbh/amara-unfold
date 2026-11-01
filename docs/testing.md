# Testing and acceptance strategy

amara Unfold is tested as four connected layers. No single green browser test
is allowed to stand in for the native filesystem and database pipeline.

## Select checks for the change

Start with the normal visible user journey affected by a change. Add focused
checks where they protect meaningful extraction, persistence, cancellation,
privacy or security behavior. Reuse accepted results for unchanged inputs.
Documentation and receipt changes do not trigger full builds or test suites.
Do not write tests that simply reproduce implementation details. Once checks
pass, continue to delivery unless a new failure or changed input warrants more.

`test:deep` is an optional broad diagnostic run, not a mandatory change or release
ritual. Public source still needs a clean-payload/privacy audit, applicable history
and license proof. Binary distribution still needs signature/notarization checks;
recovery ZIPs still need integrity and restore verification. State why an expensive
check is needed and keep the evidence compact. Group related changes and reuse
one verified baseline instead of creating a backup for every edit.

## Acceptance evidence hierarchy

Application acceptance is always a visible UI journey through normal user
controls. Record the full visible result and expected behavior, plus the
environment/version, tester role, viewport, and screenshot/CSV provenance.
Label synthetic/local fixtures and distinguish them from live UI evidence.

Unit, API, backend, IPC, headless, static, and automated browser/native checks
are **supporting automated evidence**. They find regressions and validate
implementation boundaries, but cannot independently close an application UI
case or establish production readiness. Headed automation counts only when it
drives the actual visible interface using normal user actions and inspects the
visible result; direct API/IPC calls, forced DOM/state injection, and hidden
navigation do not replace workflow steps. If a required journey cannot be
completed, record it as BLOCKED or PARTIAL rather than substituting background
evidence.

## What runs where

| Layer                   | Command                | What it proves                                                                                                    |
| ----------------------- | ---------------------- | ----------------------------------------------------------------------------------------------------------------- |
| TypeScript/Rust units   | `npm run test`         | formatting helpers, privacy denial, extractors, candidate rules, scanner boundaries, migrations, atomic indexing  |
| Browser journeys        | `npm run test:browser` | five-section UX, successful scan, cancellation, errors, no external requests, WCAG checks, layout overflow        |
| Native Tauri acceptance | `npm run test:native`  | real macOS WebView, real Tauri IPC, real Rust scan, real isolated SQLite persistence, timeline/evidence rendering |
| Optional broad check    | `npm run test:deep`    | formatting, builds, all tests/lints, dependency audit, and a final ordinary debug `.app` bundle                   |

## Why browser automation is still useful

Most of amara Unfold's interface is HTML/CSS/React inside a Tauri WebView. The
browser suite starts Vite with a test-only desktop adapter and drives installed
Google Chrome with Playwright. Synthetic scenarios make slow or rare states
repeatable:

- empty first run;
- completed 333-file archive;
- in-progress cancellation;
- permission failure with a local audit record;
- populated timeline and evidence.

This is the same surface that the Codex in-app browser can inspect when a local
browser connection is available:

```sh
npm run dev:e2e
```

Then open `http://localhost:1420/?scenario=populated`. This preview never gets
native filesystem access; it exists for deterministic UI review only.

Run Playwright interactively when diagnosing a UI test:

```sh
npm run test:browser:ui
```

The suite saves traces, screenshots, and video under `test-results/` only when
a test fails. Those artifacts are ignored by Git.

## How the actual desktop app is automated

The native suite uses the current WebdriverIO Tauri service with its embedded
WebDriver provider. It launches `target/debug/amara-unfold-desktop`, executes
the real Tauri commands, and interacts with the real WKWebView on macOS.

Test-only code is isolated in three ways:

1. Rust WebDriver plugins are optional dependencies behind Cargo feature `e2e`.
2. Test permissions and `withGlobalTauri` exist only in `tauri.e2e.conf.json`.
3. The test app identifier is `com.amaramemory.unfold.e2e`, so its database is
   separate from the real app and is cleared before every native run.

The native test creates four synthetic files in the OS temporary directory:
two identical PNGs, a MOV placeholder, and an unknown text file. It proves:

- 4 files, 2 photos, 1 video, and 1 unknown are indexed;
- identical content produces one duplicate;
- the August 2019 timeline is persisted;
- candidate evidence appears after a real application reload;
- privacy reports local-only with network and telemetry disabled.

Normal builds do not compile or register the WebDriver plugins. `test:deep`
finishes by rebuilding the ordinary debug app, ensuring the handed-off bundle
is not the E2E binary.

## Rust edge cases covered

- cancellation before and during traversal;
- symbolic links never cross the selected source boundary;
- recursive recognized/unknown file classification;
- dimensions from a real generated image;
- malformed/unsupported metadata degrades without losing filesystem evidence;
- EXIF/file dates form deterministic candidates while unknown files are skipped;
- duplicate hashes and counts;
- completed rescans replace rather than accumulate a source snapshot;
- cancellation preserves the previous complete snapshot;
- failed scans remain queryable as local issues;
- schema migration reopening is idempotent;
- a full synthetic archive creates and queries a physical SQLite database.

## Public CI

Hosted CI is currently disabled for this repository. The checked-in reference
workflow describes macOS and Windows build/test jobs; it does not establish
that those jobs have run. It specifies browser journeys using installed Chrome
on both platforms, feature-gated native WebDriver acceptance on macOS, and
Rust/frontend checks on Windows. Run the relevant commands locally and retain
actual results; do not describe workflow configuration as executed acceptance.

For a release candidate, record the source fingerprint and reuse relevant
accepted build/test results. Run focused checks for any unverified changed
boundaries. Install dependencies only when the lockfile or execution environment
requires it. A signed public binary additionally needs signed/notarized bundle
validation and a normal visible synthetic user journey. Never use a personal
library as a generic test fixture.

Local-app intake has additional route-specific synthetic checks and bounded
format variants in [Local-app intake testing](local-app-uat.md). Personal
exports and their results remain outside public source and Git history.

## Upstream references

- [Tauri testing overview](https://v2.tauri.app/develop/tests/)
- [Tauri WebDriver testing](https://v2.tauri.app/develop/tests/webdriver/)
- [WebdriverIO Tauri service](https://webdriver.io/docs/wdio-tauri-service/)
- [Playwright running and debugging tests](https://playwright.dev/docs/running-tests)

## Native harness dependency compatibility

WDIO components must move together with their expectation-library peer. Current
minimums select the patched WDIO 9 line and expect-webdriverio 6. The scoped
Tauri-service webdriverio override selects 9.31.6 because the service's exact
9.30.1 pin otherwise reintroduces deepmerge-ts 7. Do not bypass npm peer checks
with force or legacy-peer-deps. A focused existing launch test is sufficient
for this toolchain integration change; it is supporting evidence, separate
from a normal native workbook import and full readback.

If old peer-lock entries prevent a coordinated upgrade, preserve the original
lock, resolve the affected WDIO/WebDriver/expect entries in a disposable local
workspace, inspect the resulting delta, then verify npm ci. Do not accept a
fresh broad graph that silently updates unrelated runtime or frontend tools.
The current override versions and remaining compatibility qualifications are
documented in [Build from source](build-from-source.md). The checked-in
lockfile no longer contains extract-zip; this does not replace a fresh advisory
audit or native compatibility checks.
