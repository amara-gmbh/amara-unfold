#!/usr/bin/env node
// SPDX-License-Identifier: AGPL-3.0-or-later

import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import fsp from "node:fs/promises";
import os from "node:os";
import path from "node:path";

const args = parseArgs(process.argv.slice(2));
const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const binary = path.resolve(
  repoRoot,
  args.bin ?? process.env.AMARA_UNFOLD_BIN ?? "target/debug/amara-unfold",
);
const sourceOverride =
  args.source ?? process.env.AMARA_UNFOLD_MAIL_EXPORT_SOURCE;
const failIfMissing = Boolean(args["fail-if-missing"]);
const asJson = Boolean(args.json);
const recommendedExportFolder = path.join(
  os.homedir(),
  "Desktop",
  "Amara Unfold Mail Exports",
);
const missingExportNextStep = [
  "In Unfold, click Prepare Mail export to open Apple Mail and the recommended save folder.",
  "In Apple Mail, select a small mailbox, choose Mailbox › Export Mailbox…, and save the .mbox package to the prepared folder.",
  "Return to Scan; Unfold refreshes on focus, or click Refresh app exports.",
  "Then rerun npm run uat:mail-export:run.",
].join(" ");

const preflight = sourceOverride
  ? await inspectExplicitSource(sourceOverride)
  : runPreflight();
const readyCandidate = preflight.candidates.find(
  (candidate) => candidate.ready,
);

if (!readyCandidate) {
  const result = {
    status: "waiting_for_exported_mbox",
    contentRead: false,
    extractionRun: false,
    validationRun: false,
    recommendedExportFolder,
    nextStep: missingExportNextStep,
    preflight,
  };
  printResult(result, asJson);
  if (failIfMissing) process.exitCode = 2;
  process.exit();
}

if (!fs.existsSync(binary)) {
  exitWithError(
    `Missing CLI binary: ${binary}\nBuild it first with: cargo build -p amara-unfold-adapters --bin amara-unfold`,
    asJson,
  );
}

const startedAt = new Date();
const outputRoot = path.resolve(
  repoRoot,
  args.output ??
    path.join("dist", "uat", `apple-mail-export-${timestamp(startedAt)}`),
);
await fsp.mkdir(outputRoot, { recursive: true });

const extract = spawnSync(
  binary,
  [
    "extract",
    "apple-mail-local",
    "--source",
    readyCandidate.path,
    "--output",
    outputRoot,
  ],
  { cwd: repoRoot, encoding: "utf8" },
);

if (extract.status !== 0) {
  await writeReport(outputRoot, {
    status: "extract_failed",
    startedAt: startedAt.toISOString(),
    finishedAt: new Date().toISOString(),
    source: readyCandidate.path,
    outputRoot,
    contentRead: true,
    extractionRun: true,
    validationRun: false,
    stderr: extract.stderr,
    stdout: extract.stdout,
  });
  exitWithError(
    `Apple Mail export UAT extraction failed.\n${extract.stderr || extract.stdout}`,
    asJson,
  );
}

const packagePath = extract.stdout.trim().split(/\r?\n/).at(-1);
if (!packagePath || !fs.existsSync(packagePath)) {
  exitWithError(
    `Extraction did not return a readable package path.\nstdout:\n${extract.stdout}\nstderr:\n${extract.stderr}`,
    asJson,
  );
}

const validate = spawnSync(binary, ["validate", packagePath], {
  cwd: repoRoot,
  encoding: "utf8",
});
const eventCount = parseEventCount(validate.stdout);
const status = validate.status === 0 ? "pass" : "validate_failed";
const report = {
  status,
  startedAt: startedAt.toISOString(),
  finishedAt: new Date().toISOString(),
  source: readyCandidate.path,
  outputRoot,
  packagePath,
  eventCount,
  contentRead: true,
  contentReadNote:
    "Preflight reads only package shape. This UAT runner reads the user-selected exported .mbox because extraction necessarily converts its messages to Markdown.",
  extractionRun: true,
  validationRun: true,
  networkUsed: false,
  extractStdout: extract.stdout.trim(),
  extractStderr: extract.stderr.trim(),
  validateStdout: validate.stdout.trim(),
  validateStderr: validate.stderr.trim(),
};
await writeReport(outputRoot, report);

if (validate.status !== 0) {
  exitWithError(
    `Apple Mail export UAT validation failed.\n${validate.stderr || validate.stdout}`,
    asJson,
  );
}

printResult(report, asJson);

function parseArgs(rawArgs) {
  const parsed = {};
  for (let index = 0; index < rawArgs.length; index += 1) {
    const item = rawArgs[index];
    if (!item.startsWith("--")) continue;
    const key = item.slice(2);
    const next = rawArgs[index + 1];
    if (!next || next.startsWith("--")) {
      parsed[key] = true;
      continue;
    }
    parsed[key] = next;
    index += 1;
  }
  return parsed;
}

function runPreflight() {
  const stdout = execFileSync(
    process.execPath,
    [path.join(repoRoot, "scripts", "check-mail-export-uat.mjs"), "--json"],
    { cwd: repoRoot, encoding: "utf8" },
  );
  return JSON.parse(stdout);
}

async function inspectExplicitSource(sourcePath) {
  const resolved = path.resolve(sourcePath.replace(/^~/, os.homedir()));
  const entries = await fsp
    .readdir(resolved, { withFileTypes: true })
    .catch(() => []);
  const names = new Set(entries.map((entry) => entry.name));
  const ready =
    resolved.toLowerCase().endsWith(".mbox") &&
    (names.has("mbox") || names.has("Messages"));
  return {
    status: ready ? "ready" : "waiting_for_exported_mbox",
    searchRoots: [resolved],
    candidates: [
      {
        path: resolved,
        ready,
        evidence: [
          ...(resolved.toLowerCase().endsWith(".mbox")
            ? ["directory name ends with .mbox"]
            : []),
          ...(names.has("mbox") ? ["contains mbox file"] : []),
          ...(names.has("Messages") ? ["contains Messages directory"] : []),
        ],
      },
    ],
    contentRead: false,
    recommendedExportFolder,
    nextStep: ready
      ? "Run the apple-mail-local adapter against this exported .mbox and validate the generated package."
      : "Choose a folder ending in .mbox that contains an Apple Mail exported mailbox, or use Prepare Mail export in Unfold and save the Mail export to the prepared Desktop folder.",
  };
}

function parseEventCount(stdout) {
  const match = stdout.match(/valid open Markdown events:\s*(\d+)/);
  return match ? Number(match[1]) : null;
}

async function writeReport(outputRoot, report) {
  await fsp.mkdir(outputRoot, { recursive: true });
  const jsonPath = path.join(outputRoot, "apple-mail-export-uat-report.json");
  const markdownPath = path.join(outputRoot, "APPLE-MAIL-EXPORT-UAT.md");
  await fsp.writeFile(jsonPath, `${JSON.stringify(report, null, 2)}\n`);
  await fsp.writeFile(markdownPath, markdownReport(report));
}

function markdownReport(report) {
  return `# Apple Mail export UAT

- Status: ${report.status}
- Started: ${report.startedAt}
- Finished: ${report.finishedAt}
- Source: ${report.source ?? "not found yet"}
- Output root: ${report.outputRoot ?? "not created"}
- Package: ${report.packagePath ?? "not created"}
- Valid events: ${report.eventCount ?? "not validated"}
- Network used: ${report.networkUsed ?? false}
- Content read: ${report.contentRead}

${report.contentReadNote ?? "Preflight only checked exported .mbox package shape and did not read message contents."}
`;
}

function printResult(result, json) {
  if (json) {
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    return;
  }
  if (result.status === "waiting_for_exported_mbox") {
    process.stdout.write("Apple Mail export UAT: waiting_for_exported_mbox\n");
    process.stdout.write("Content read: no\n");
    process.stdout.write(`Next: ${result.nextStep}\n`);
    return;
  }
  process.stdout.write(`Apple Mail export UAT: ${result.status}\n`);
  process.stdout.write(`Package: ${result.packagePath}\n`);
  process.stdout.write(`Valid events: ${result.eventCount}\n`);
  process.stdout.write(
    `Report: ${path.join(result.outputRoot, "APPLE-MAIL-EXPORT-UAT.md")}\n`,
  );
}

function exitWithError(message, json) {
  if (json) {
    process.stdout.write(
      `${JSON.stringify({ status: "error", message }, null, 2)}\n`,
    );
  } else {
    process.stderr.write(`${message}\n`);
  }
  process.exit(1);
}

function timestamp(date) {
  const pad = (value) => String(value).padStart(2, "0");
  return [
    date.getFullYear(),
    pad(date.getMonth() + 1),
    pad(date.getDate()),
    "-",
    pad(date.getHours()),
    pad(date.getMinutes()),
    pad(date.getSeconds()),
  ].join("");
}
