#!/usr/bin/env node
// SPDX-License-Identifier: AGPL-3.0-or-later

import { execFileSync, spawnSync } from "node:child_process";
import path from "node:path";

const args = parseArgs(process.argv.slice(2));
const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const timeoutMs = Number(args["timeout-ms"] ?? 10 * 60 * 1000);
const intervalMs = Number(args["interval-ms"] ?? 5000);
const asJson = Boolean(args.json);
const startedAt = Date.now();

if (!Number.isFinite(timeoutMs) || timeoutMs < 0) {
  exitWithUsage("--timeout-ms must be a non-negative number.");
}

if (!Number.isFinite(intervalMs) || intervalMs <= 0) {
  exitWithUsage("--interval-ms must be a positive number.");
}

let lastPreflight = null;
let attempt = 0;

while (Date.now() - startedAt <= timeoutMs) {
  attempt += 1;
  lastPreflight = runPreflight();
  const ready = (lastPreflight.candidates ?? []).filter(
    (candidate) => candidate.ready,
  );

  if (ready.length > 0) {
    const run = spawnSync(
      process.execPath,
      [path.join(repoRoot, "scripts", "run-mail-export-uat.mjs")],
      { cwd: repoRoot, encoding: "utf8" },
    );

    if (asJson) {
      process.stdout.write(
        `${JSON.stringify(
          {
            status: run.status === 0 ? "pass" : "run_failed",
            attempts: attempt,
            readyCandidate: ready[0],
            runStdout: run.stdout.trim(),
            runStderr: run.stderr.trim(),
          },
          null,
          2,
        )}\n`,
      );
    } else {
      process.stdout.write(
        `Apple Mail export appeared after ${attempt} check${attempt === 1 ? "" : "s"}: ${ready[0].path}\n`,
      );
      process.stdout.write(run.stdout);
      if (run.stderr) process.stderr.write(run.stderr);
    }

    process.exitCode = run.status ?? 1;
    process.exit();
  }

  if (!asJson) printWaiting(attempt, lastPreflight);
  await sleep(
    Math.min(intervalMs, Math.max(0, timeoutMs - (Date.now() - startedAt))),
  );
}

const result = {
  status: "timeout_waiting_for_exported_mbox",
  attempts: attempt,
  timeoutMs,
  intervalMs,
  preparedExportFolder: lastPreflight?.preparedExportFolder ?? null,
  userExportCandidates: lastPreflight?.candidates ?? [],
  contentRead: false,
  nextStep:
    lastPreflight?.nextStep ??
    "Export a mailbox from Apple Mail as .mbox, then rerun this command.",
};

if (asJson) {
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
} else {
  process.stdout.write("Apple Mail export watcher: timed out\n");
  process.stdout.write("Content read while waiting: no\n");
  if (result.preparedExportFolder) {
    process.stdout.write(
      `Prepared folder: ${result.preparedExportFolder.exists ? "yes" : "no"} ${result.preparedExportFolder.path}\n`,
    );
  }
  process.stdout.write(`Next: ${result.nextStep}\n`);
}
process.exitCode = 2;

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

function printWaiting(attempt, preflight) {
  const prepared = preflight.preparedExportFolder;
  const candidateCount = preflight.candidates?.length ?? 0;
  const ignoredFixtureCount = preflight.ignoredFixtureCandidates?.length ?? 0;
  process.stdout.write(
    `Waiting for Apple Mail .mbox export (${attempt}) — prepared folder: ${
      prepared?.exists ? "yes" : "no"
    }, README: ${prepared?.instructionsFileExists ? "yes" : "no"}, user-export candidates: ${candidateCount}, ignored fixtures: ${ignoredFixtureCount}\n`,
  );
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function exitWithUsage(message) {
  process.stderr.write(`${message}\n`);
  process.stderr.write(
    "Usage: npm run uat:mail-export:watch -- [--timeout-ms 600000] [--interval-ms 5000] [--json]\n",
  );
  process.exit(1);
}
