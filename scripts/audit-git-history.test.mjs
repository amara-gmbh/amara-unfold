// SPDX-License-Identifier: AGPL-3.0-or-later

import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const auditScript = fileURLToPath(
  new URL("audit-git-history.mjs", import.meta.url),
);
const company = {
  name: "amara information security GmbH",
  email: "info@amara.gmbh",
};

function git(repository, args, options = {}) {
  return execFileSync("git", args, {
    cwd: repository,
    encoding: "utf8",
    ...options,
  }).trim();
}

function createRepository() {
  const repository = mkdtempSync(
    path.join(os.tmpdir(), "unfold-history-audit-"),
  );
  git(repository, ["init", "-b", "main"]);
  git(repository, ["config", "user.name", company.name]);
  git(repository, ["config", "user.email", company.email]);
  writeFileSync(path.join(repository, "README.md"), "# Synthetic fixture\n");
  git(repository, ["add", "README.md"]);
  git(repository, ["commit", "-m", "Create synthetic fixture"]);
  return repository;
}

test("accepts company-authored commits and annotated tags", () => {
  const repository = createRepository();
  git(repository, ["tag", "-a", "v1", "-m", "Synthetic release"]);
  const result = spawnSync(process.execPath, [auditScript], {
    cwd: repository,
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /GIT HISTORY AUDIT: PASS/);
});

test("rejects a personal author on an old ref and a personal annotated tagger", () => {
  const repository = createRepository();
  const companyRoot = git(repository, ["rev-parse", "HEAD"]);

  writeFileSync(path.join(repository, "private.txt"), "synthetic old state\n");
  git(repository, ["add", "private.txt"]);
  git(repository, [
    "-c",
    "user.name=Example Person",
    "-c",
    "user.email=person@example.com",
    "commit",
    "-m",
    "Synthetic pre-scrub commit",
  ]);
  const personalCommit = git(repository, ["rev-parse", "HEAD"]);
  git(repository, ["update-ref", "refs/original/pre-scrub", personalCommit]);
  git(repository, ["reset", "--hard", companyRoot]);
  git(repository, [
    "-c",
    "user.name=Example Person",
    "-c",
    "user.email=person@example.com",
    "tag",
    "-a",
    "personal-tagger",
    "-m",
    "Synthetic tag fixture",
    companyRoot,
  ]);

  const result = spawnSync(process.execPath, [auditScript], {
    cwd: repository,
    encoding: "utf8",
  });
  assert.equal(result.status, 1, result.stdout);
  assert.match(result.stderr, /author identity is not company-approved/);
  assert.match(result.stderr, /tagger identity is not company-approved/);
  assert.match(result.stderr, /refs\/original\/pre-scrub|commit [0-9a-f]{40}/);
});

test("rejects canonical-only continuity tooling in company-authored history", () => {
  const repository = createRepository();
  const continuityTool = [
    "scripts",
    ["verify-project", "memory.mjs"].join("-"),
  ].join(path.sep);
  const privateScriptNames = [
    ["verify", "memory"].join(":"),
    ["check", "private"].join(":"),
  ];

  mkdirSync(path.join(repository, "scripts"), { recursive: true });
  writeFileSync(
    path.join(repository, continuityTool),
    "// SPDX-License-Identifier: AGPL-3.0-or-later\n",
  );
  writeFileSync(
    path.join(repository, "package.json"),
    `${JSON.stringify(
      {
        name: "synthetic-history-fixture",
        scripts: Object.fromEntries(
          privateScriptNames.map((name) => [name, "node fixture.mjs"]),
        ),
      },
      null,
      2,
    )}\n`,
  );
  git(repository, ["add", "scripts", "package.json"]);
  git(repository, ["commit", "-m", "Add synthetic continuity leak"]);

  const result = spawnSync(process.execPath, [auditScript], {
    cwd: repository,
    encoding: "utf8",
  });
  assert.equal(result.status, 1, result.stdout);
  assert.match(result.stderr, /private continuity tool/);
  assert.match(result.stderr, /private-only package script/);
});
