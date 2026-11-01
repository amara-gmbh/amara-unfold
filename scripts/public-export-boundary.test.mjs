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
import { fileURLToPath } from "node:url";

const sourceRoot = fileURLToPath(new URL("..", import.meta.url));
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

  // Packaged source must not contain Git history. A deliberate CI worktree
  // audit can skip the root .git directory only while history is checked separately.
  mkdirSync(path.join(exported, ".git"));
  const packagedGit = spawnSync(process.execPath, [auditScript, exported], {
    encoding: "utf8",
    env: { ...process.env, AMARA_AUDIT_GIT_WORKTREE: "0" },
  });
  assert.equal(packagedGit.status, 1, packagedGit.stdout);
  assert.match(
    packagedGit.stderr,
    /top-level path is not in the public allowlist/,
  );
  const worktree = spawnSync(process.execPath, [auditScript, exported], {
    encoding: "utf8",
    env: { ...process.env, AMARA_AUDIT_GIT_WORKTREE: "1" },
  });
  assert.equal(worktree.status, 0, worktree.stderr || worktree.stdout);

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

  writeFileSync(
    path.join(negative, "assets", "screenshots", "unreviewed.png"),
    "synthetic unreviewed image",
  );

  const audited = spawnSync(process.execPath, [auditScript, negative], {
    cwd: negative,
    encoding: "utf8",
    env: { ...process.env, AMARA_AUDIT_GIT_WORKTREE: "1" },
  });
  assert.equal(audited.status, 1, audited.stdout);
  assert.match(audited.stderr, /private continuity tool is not public source/);
  assert.match(audited.stderr, /private-only package script is forbidden/);
  assert.match(audited.stderr, /top-level path is not in the public allowlist/);
  assert.match(
    audited.stderr,
    /binary image is outside the reviewed asset allowlist/,
  );
});
