#!/usr/bin/env node
// SPDX-License-Identifier: AGPL-3.0-or-later

import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";

const args = new Set(process.argv.slice(2));
const asJson = args.has("--json");
const failIfMissing = args.has("--fail-if-missing");
const maxDepth = 6;

const home = os.homedir();
const searchRoots = [
  path.join(home, "Desktop"),
  path.join(home, "Documents"),
  path.join(home, "Downloads"),
  path.join(home, "Library", "Mobile Documents", "com~apple~CloudDocs"),
];

const diagnosticRoots = [
  ...(process.env.AMARA_UNFOLD_MAIL_UAT_ROOT
    ? [path.resolve(process.env.AMARA_UNFOLD_MAIL_UAT_ROOT)]
    : []),
  path.join(home, ".amara-unfold", "uat", "mail"),
];

const skippedNames = new Set([
  ".git",
  ".Trash",
  "node_modules",
  "target",
  "dist",
  "Library",
]);

const candidates = [];
const ignoredFixtureCandidates = [];
const errors = [];
const recommendedExportFolder = path.join(
  home,
  "Desktop",
  "Amara Unfold Mail Exports",
);
const recommendedInstructionsFile = path.join(
  recommendedExportFolder,
  "READ ME - Apple Mail Export.txt",
);
const preparedExportFolder = await inspectPreparedExportFolder();
const missingExportNextStep = [
  "No user-exported Apple Mail mailbox was found.",
  "In Unfold, click Prepare Mail export to open Apple Mail and the recommended save folder.",
  "In Apple Mail, select a small mailbox, choose Mailbox › Export Mailbox…, and save the .mbox package to the prepared folder.",
  "Return to Scan; Unfold refreshes on focus, or click Refresh app exports.",
  "Then rerun npm run uat:mail-export:run.",
].join(" ");

for (const root of searchRoots) {
  await walk(root, 0, "user-export");
}

for (const root of diagnosticRoots) {
  await walk(root, 0, "diagnostic-fixture");
}

const ready = candidates.filter((candidate) => candidate.ready);
const result = {
  status: ready.length > 0 ? "ready" : "waiting_for_exported_mbox",
  searchRoots,
  userExportCandidates: candidates,
  candidates,
  ignoredFixtureCandidates,
  recommendedExportFolder,
  preparedExportFolder,
  contentRead: false,
  nextStep:
    ready.length > 0
      ? "Run the apple-mail-local adapter against one candidate and validate the generated package."
      : missingExportNextStep,
};

if (asJson) {
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
} else {
  process.stdout.write(`Apple Mail export UAT preflight: ${result.status}\n`);
  process.stdout.write("Content read: no\n");
  process.stdout.write(
    `Prepared folder: ${preparedExportFolder.exists ? "yes" : "no"} ${preparedExportFolder.path}\n`,
  );
  if (preparedExportFolder.instructionsFileExists) {
    process.stdout.write(
      `Instructions: yes ${preparedExportFolder.instructionsFile}\n`,
    );
  }
  if (errors.length > 0) {
    process.stdout.write(`Skipped unreadable search paths: ${errors.length}\n`);
  }
  if (candidates.length > 0) {
    process.stdout.write("\nUser-exported mailbox candidates:\n");
    for (const candidate of candidates) {
      process.stdout.write(
        `- ${candidate.ready ? "ready" : "incomplete"} ${candidate.path}\n`,
      );
    }
  }
  if (ignoredFixtureCandidates.length > 0) {
    process.stdout.write("\nIgnored test fixtures:\n");
    for (const candidate of ignoredFixtureCandidates) {
      process.stdout.write(`- ${candidate.path}\n`);
    }
  }
  process.stdout.write(`\nNext: ${result.nextStep}\n`);
  if (ready[0]) {
    process.stdout.write("\nExample:\n");
    process.stdout.write(
      `target/debug/amara-unfold extract apple-mail-local --source ${JSON.stringify(ready[0].path)} --output "<empty-output-folder>"\n`,
    );
  }
}

if (failIfMissing && ready.length === 0) {
  process.exitCode = 2;
}

async function walk(directory, depth, mode) {
  let entries;
  try {
    entries = await fs.readdir(directory, { withFileTypes: true });
  } catch (error) {
    if (error?.code !== "ENOENT") {
      errors.push({ path: directory, message: error.message });
    }
    return;
  }

  if (directory.toLowerCase().endsWith(".mbox")) {
    const inspected = await inspectMboxPackage(directory, entries);
    if (mode === "diagnostic-fixture" || isLikelyFixture(directory)) {
      ignoredFixtureCandidates.push({
        ...inspected,
        ignoredReason:
          "Synthetic/test fixture; useful for adapter tests but not accepted as a user-export preflight result.",
      });
    } else {
      candidates.push(inspected);
    }
    return;
  }

  if (depth >= maxDepth) return;

  for (const entry of entries) {
    if (!entry.isDirectory() || skippedNames.has(entry.name)) continue;
    if (entry.name.startsWith(".") && entry.name !== ".mbox") continue;
    await walk(path.join(directory, entry.name), depth + 1, mode);
  }
}

async function inspectPreparedExportFolder() {
  let entries = [];
  let exists = false;
  let instructionsFileExists = false;
  try {
    entries = await fs.readdir(recommendedExportFolder, {
      withFileTypes: true,
    });
    exists = true;
  } catch (error) {
    if (error?.code !== "ENOENT") {
      errors.push({ path: recommendedExportFolder, message: error.message });
    }
  }
  try {
    const stat = await fs.stat(recommendedInstructionsFile);
    instructionsFileExists = stat.isFile();
  } catch (error) {
    if (error?.code !== "ENOENT") {
      errors.push({
        path: recommendedInstructionsFile,
        message: error.message,
      });
    }
  }
  return {
    path: recommendedExportFolder,
    exists,
    instructionsFile: recommendedInstructionsFile,
    instructionsFileExists,
    entryCount: entries.length,
    entries: entries
      .map((entry) => ({
        name: entry.name,
        kind: entry.isDirectory()
          ? "directory"
          : entry.isFile()
            ? "file"
            : "other",
      }))
      .slice(0, 20),
  };
}

function isLikelyFixture(directory) {
  const normalized = directory.split(path.sep).join("/");
  return (
    normalized.includes("/fixtures/") ||
    normalized.includes("/test-fixtures/") ||
    normalized.includes("/synthetic/") ||
    normalized.includes("/.amara-unfold/uat/")
  );
}

async function inspectMboxPackage(directory, entries) {
  const childNames = new Set(entries.map((entry) => entry.name));
  const hasMboxFile = childNames.has("mbox");
  const hasMessagesDir = childNames.has("Messages");
  return {
    path: directory,
    ready: hasMboxFile || hasMessagesDir,
    evidence: [
      "directory name ends with .mbox",
      ...(hasMboxFile ? ["contains mbox file"] : []),
      ...(hasMessagesDir ? ["contains Messages directory"] : []),
    ],
  };
}
