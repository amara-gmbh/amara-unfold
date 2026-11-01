// SPDX-License-Identifier: AGPL-3.0-or-later
// Exercise the interfaces used by the test tools across security overrides.
import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { glob } from "glob";
import { minimatch, Minimatch } from "minimatch";
import * as browsers from "@puppeteer/browsers";
import * as tauriService from "@wdio/tauri-service";

const root = await mkdtemp(path.join(tmpdir(), "amara-override-check-"));
try {
  await mkdir(path.join(root, "tests", "nested"), { recursive: true });
  await writeFile(
    path.join(root, "tests", "nested", "sample.e2e.ts"),
    "export {};\n",
  );
  await writeFile(
    path.join(root, "tests", "nested", "ignored.txt"),
    "ignored\n",
  );
  assert.deepEqual(await glob("tests/**/*.e2e.ts", { cwd: root }), [
    "tests/nested/sample.e2e.ts",
  ]);
  assert.equal(
    minimatch("tests/nested/sample.e2e.ts", "tests/**/*.e2e.ts"),
    true,
  );
  assert.equal(
    new Minimatch("*.e2e.ts", { matchBase: true }).match(
      "nested/sample.e2e.ts",
    ),
    true,
  );
  for (const key of [
    "install",
    "canDownload",
    "resolveBuildId",
    "detectBrowserPlatform",
    "computeExecutablePath",
  ]) {
    assert.equal(
      typeof browsers[key],
      "function",
      `Missing browser interface ${key}`,
    );
  }
  assert.equal(typeof browsers.Browser, "object");
  assert.equal(typeof browsers.ChromeReleaseChannel, "object");
  assert.equal(typeof tauriService.default, "function");
  assert.equal(typeof tauriService.launcher, "function");
  console.log("Dependency override compatibility checks passed.");
} finally {
  // root was exclusively created by mkdtemp for this check.
  await rm(root, { recursive: true, force: true });
}
