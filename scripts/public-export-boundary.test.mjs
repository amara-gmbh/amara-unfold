// SPDX-License-Identifier: AGPL-3.0-or-later

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

const sourceRoot = path.resolve(new URL("..", import.meta.url).pathname);
const exportScript = path.join(
  sourceRoot,
  "scripts",
  "create-public-source.mjs",
);
const auditScript = path.join(sourceRoot, "scripts", "audit-public-source.mjs");
const continuityTool = [
  "scripts",
  ["verify-project", "memory.mjs"].join("-"),
].join(path.sep);
const privateScriptNames = [
  ["verify", "memory"].join(":"),
  ["check", "private"].join(":"),
];

test("public export strips canonical-only continuity tooling and commands", (t) => {
  const fixtureRoot = mkdtempSync(
    path.join(os.tmpdir(), "unfold-public-boundary-"),
  );
  t.after(() => rmSync(fixtureRoot, { recursive: true, force: true }));

  const exported = path.join(fixtureRoot, "public");
  const created = spawnSync(process.execPath, [exportScript, exported], {
    cwd: sourceRoot,
    encoding: "utf8",
  });
  assert.equal(created.status, 0, created.stderr || created.stdout);
  assert.equal(existsSync(path.join(exported, continuityTool)), false);

  const publicPackage = JSON.parse(
    readFileSync(path.join(exported, "package.json"), "utf8"),
  );
  for (const scriptName of privateScriptNames) {
    assert.equal(Object.hasOwn(publicPackage.scripts ?? {}, scriptName), false);
  }

  const negative = path.join(fixtureRoot, "negative");
  cpSync(exported, negative, { recursive: true });
  const forbiddenToolPath = path.join(negative, continuityTool);
  mkdirSync(path.dirname(forbiddenToolPath), { recursive: true });
  writeFileSync(
    forbiddenToolPath,
    "// SPDX-License-Identifier: AGPL-3.0-or-later\n",
  );
  const negativePackagePath = path.join(negative, "package.json");
  const negativePackage = JSON.parse(readFileSync(negativePackagePath, "utf8"));
  for (const scriptName of privateScriptNames) {
    negativePackage.scripts[scriptName] = "node private-only-fixture.mjs";
  }
  writeFileSync(
    negativePackagePath,
    `${JSON.stringify(negativePackage, null, 2)}\n`,
  );
  writeFileSync(
    path.join(negative, "internal-operations.txt"),
    "synthetic unapproved top-level fixture\n",
  );

  const audited = spawnSync(process.execPath, [auditScript, negative], {
    cwd: negative,
    encoding: "utf8",
  });
  assert.equal(audited.status, 1, audited.stdout);
  assert.match(audited.stderr, /private continuity tool is not public source/);
  assert.match(audited.stderr, /private-only package script is forbidden/);
  assert.match(audited.stderr, /top-level path is not in the public allowlist/);
});
