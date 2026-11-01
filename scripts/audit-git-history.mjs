#!/usr/bin/env node
// SPDX-License-Identifier: AGPL-3.0-or-later

import { spawnSync } from "node:child_process";
import process from "node:process";

const companyIdentities = new Set([
  identityKey("amara information security GmbH", "info@amara.gmbh"),
]);

function git(args) {
  return spawnSync("git", args, {
    encoding: "utf8",
    maxBuffer: 512 * 1024 * 1024,
  });
}

function identityKey(name, email) {
  return `${name.trim()}\0${email.trim().toLowerCase()}`;
}

function identityAllowed(name, email) {
  return companyIdentities.has(identityKey(name, email));
}

function addIdentityFinding(findings, object, role, name, email) {
  if (!identityAllowed(name, email)) {
    findings.push(
      `${object} ${role} identity is not company-approved: ${name} <${email}>`,
    );
  }
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

const inside = git(["rev-parse", "--is-inside-work-tree"]);
if (inside.status !== 0) {
  console.log("GIT HISTORY AUDIT: NOT APPLICABLE (no Git repository)");
  process.exit(0);
}

const head = git(["rev-parse", "--verify", "HEAD"]);
if (head.status !== 0) {
  console.log("GIT HISTORY AUDIT: PASS (repository has no commits)");
  process.exit(0);
}

const history = git([
  "log",
  "--all",
  "--no-ext-diff",
  "--text",
  "--format=commit:%H%n%an <%ae>%n%cn <%ce>%n%B",
  "-p",
  "--full-history",
  "-m",
]);
if (history.status !== 0) {
  console.error("GIT HISTORY AUDIT: ERROR");
  console.error(history.stderr.trim());
  process.exit(2);
}

const commitMetadata = git([
  "log",
  "--all",
  "--format=%H%x1f%an%x1f%ae%x1f%cn%x1f%ce%x1f%B%x1e",
]);
if (commitMetadata.status !== 0) {
  console.error("GIT HISTORY AUDIT: ERROR");
  console.error(commitMetadata.stderr.trim());
  process.exit(2);
}

const refs = git([
  "for-each-ref",
  "--format=%(refname)%09%(objecttype)%09%(objectname)",
]);
if (refs.status !== 0) {
  console.error("GIT HISTORY AUDIT: ERROR");
  console.error(refs.stderr.trim());
  process.exit(2);
}

const findings = [];
for (const record of commitMetadata.stdout.split("\x1e")) {
  const [oid, authorName, authorEmail, committerName, committerEmail] = record
    .replace(/^\s+/, "")
    .split("\x1f");
  if (!oid) continue;
  addIdentityFinding(
    findings,
    `commit ${oid}`,
    "author",
    authorName,
    authorEmail,
  );
  addIdentityFinding(
    findings,
    `commit ${oid}`,
    "committer",
    committerName,
    committerEmail,
  );
}

const tagPayloads = [];
const inspectedTagObjects = new Set();
const pendingTagObjects = [];
for (const line of refs.stdout.split("\n")) {
  const [refName, objectType, oid] = line.split("\t");
  if (!refName || objectType !== "tag") continue;
  pendingTagObjects.push({ refName, oid });
}

while (pendingTagObjects.length > 0) {
  const { refName, oid } = pendingTagObjects.pop();
  if (inspectedTagObjects.has(oid)) continue;
  inspectedTagObjects.add(oid);
  const tag = git(["cat-file", "-p", oid]);
  if (tag.status !== 0) {
    findings.push(`${refName} annotated tag object ${oid} could not be read`);
    continue;
  }
  tagPayloads.push(tag.stdout);
  const tagger = tag.stdout.match(/^tagger (.*) <([^<>]+)> \d+ [+-]\d{4}$/m);
  if (!tagger) {
    findings.push(`${refName} annotated tag object ${oid} has no valid tagger`);
    continue;
  }
  addIdentityFinding(
    findings,
    `${refName} tag ${oid}`,
    "tagger",
    tagger[1],
    tagger[2],
  );
  const nestedTag = tag.stdout.match(/^object ([0-9a-f]{40,64})\ntype tag$/m);
  if (nestedTag) {
    pendingTagObjects.push({ refName: `${refName}^{tag}`, oid: nestedTag[1] });
  }
}

const auditedText = `${history.stdout}\n${tagPayloads.join("\n")}`;
const privateContinuityTool = [
  "scripts",
  ["verify-project", "memory.mjs"].join("-"),
].join("/");
const privatePackageScripts = [
  ["verify", "memory"].join(":"),
  ["check", "private"].join(":"),
];
const checks = [
  ["absolute macOS user path", /\/Users\/(?!example\/)[A-Za-z0-9._-]+\//],
  ["mounted-volume path", /\/Volumes\//],
  ["absolute Windows user path", /[A-Za-z]:\\Users\\[^\\]+\\/],
  ["private key", /-----BEGIN (?:RSA |OPENSSH |EC )?PRIVATE KEY-----/],
  ["AWS access key", /AKIA[0-9A-Z]{16}/],
  ["GitHub-style access token", /gh[pousr]_[A-Za-z0-9_]{20,}/],
  ["OpenAI-style secret key", /sk-(?:proj-)?[A-Za-z0-9_-]{20,}/],
  ["private continuity tool", new RegExp(escapeRegExp(privateContinuityTool))],
  ...privatePackageScripts.map((scriptName) => [
    "private-only package script",
    new RegExp(`"${escapeRegExp(scriptName)}"\\s*:`),
  ]),
  [
    "assigned secret",
    /^\+?\s*(?:api[_-]?key|client_secret|password|access_token|refresh_token)\s*[:=]\s*["']?[^\s"']{8,}/im,
  ],
];

for (const [label, pattern] of checks) {
  if (pattern.test(auditedText)) findings.push(label);
}

if (findings.length > 0) {
  console.error("GIT HISTORY AUDIT: FAIL");
  for (const finding of [...new Set(findings)]) console.error(`- ${finding}`);
  console.error(
    "Replace affected refs or history, then rerun this audit before publishing.",
  );
  process.exit(1);
}

console.log("GIT HISTORY AUDIT: PASS");
console.log(
  "All commits and annotated tags reachable from every ref use the approved company identity; reachable patches, messages, and tag payloads contain no configured private-continuity, private-path, or secret signatures.",
);
