#!/usr/bin/env node
// SPDX-License-Identifier: AGPL-3.0-or-later

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import fsp from "node:fs/promises";
import os from "node:os";
import path from "node:path";

const args = parseArgs(process.argv.slice(2));
const asJson = Boolean(args.json);
const portable = Boolean(args.portable);
const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const evidenceRoot = path.resolve(
  args.evidenceRoot ??
    process.env.AMARA_UNFOLD_LOCAL_APP_UAT_ROOT ??
    path.join(os.homedir(), ".amara-unfold", "uat", "local-apps"),
);

const packages = fs.existsSync(evidenceRoot)
  ? await findPackages(evidenceRoot)
  : [];

const music = latestPackage(packages, "apple-music-local-library");
const mailReport = await latestMailReport([
  path.join(repoRoot, "dist", "uat"),
  evidenceRoot,
]);
const mailPreflight = runMailPreflight();

const result = {
  status: aggregateStatus({ music, mailReport, mailPreflight }),
  generatedAt: new Date().toISOString(),
  evidenceRoot,
  routes: {
    appleMail: mailStatus(mailReport, mailPreflight),
    appleMusic: packageStatus(music),
  },
  completionGate:
    "This optional helper covers only Mail and Music reports it can inspect. It does not prove all release routes and must not publish personal exports or generated Markdown.",
};

const outputResult = portable ? portableResult(result) : result;

if (asJson) {
  process.stdout.write(`${JSON.stringify(outputResult, null, 2)}\n`);
} else {
  printText(outputResult);
}

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

async function findPackages(root) {
  const found = [];
  await walk(root, async (directory, entries) => {
    const names = new Set(entries.map((entry) => entry.name));
    if (!names.has("manifest.yaml") || !names.has("quality-report.json")) {
      return;
    }
    const manifestPath = path.join(directory, "manifest.yaml");
    const qualityPath = path.join(directory, "quality-report.json");
    const [manifestText, qualityText, stat] = await Promise.all([
      fsp.readFile(manifestPath, "utf8"),
      fsp.readFile(qualityPath, "utf8"),
      fsp.stat(qualityPath),
    ]);
    found.push({
      path: directory,
      manifestPath,
      qualityPath,
      modifiedAt: stat.mtime.toISOString(),
      manifest: parseManifest(manifestText),
      quality: JSON.parse(qualityText),
    });
  });
  return found;
}

async function walk(directory, visitor, depth = 0) {
  if (depth > 12) return;
  let entries;
  try {
    entries = await fsp.readdir(directory, { withFileTypes: true });
  } catch {
    return;
  }
  await visitor(directory, entries);
  for (const entry of entries) {
    if (!entry.isDirectory() || skipName(entry.name)) continue;
    await walk(path.join(directory, entry.name), visitor, depth + 1);
  }
}

function skipName(name) {
  return (
    name === ".git" ||
    name === "node_modules" ||
    name === "target" ||
    name.startsWith("._")
  );
}

function parseManifest(text) {
  const manifest = {};
  for (const line of text.split(/\r?\n/)) {
    const match = line.match(/^([A-Za-z][A-Za-z0-9]*):\s*(.*)$/);
    if (!match) continue;
    const value = match[2].trim();
    manifest[match[1]] = /^\d+$/.test(value) ? Number(value) : value;
  }
  return manifest;
}

function latestPackage(packages, adapterId) {
  return packages
    .filter((item) => item.manifest.adapterId === adapterId)
    .sort((left, right) => right.modifiedAt.localeCompare(left.modifiedAt))[0];
}

async function latestMailReport(roots) {
  const reports = [];
  for (const root of roots) {
    if (!fs.existsSync(root)) continue;
    await walk(root, async (directory, entries) => {
      if (
        !entries.some(
          (entry) => entry.name === "apple-mail-export-uat-report.json",
        )
      ) {
        return;
      }
      const reportPath = path.join(
        directory,
        "apple-mail-export-uat-report.json",
      );
      const [text, stat] = await Promise.all([
        fsp.readFile(reportPath, "utf8"),
        fsp.stat(reportPath),
      ]);
      reports.push({
        path: reportPath,
        modifiedAt: stat.mtime.toISOString(),
        report: JSON.parse(text),
      });
    });
  }
  return reports.sort((left, right) =>
    right.modifiedAt.localeCompare(left.modifiedAt),
  )[0];
}

function runMailPreflight() {
  const stdout = execFileSync(
    process.execPath,
    [path.join(repoRoot, "scripts", "check-mail-export-uat.mjs"), "--json"],
    { cwd: repoRoot, encoding: "utf8" },
  );
  return JSON.parse(stdout);
}

function packageStatus(item) {
  if (!item) {
    return {
      status: "missing",
      evidence: null,
      reason: "No validated package report found.",
    };
  }
  const pass =
    item.quality.status === "pass" &&
    item.manifest.networkUsed === "false" &&
    item.manifest.errors === 0 &&
    item.quality.schemaErrors === 0 &&
    item.quality.provenanceErrors === 0 &&
    item.quality.extractionErrors === 0 &&
    item.quality.accountingBalanced === true &&
    item.quality.skipReasonsBalanced === true &&
    Number(item.quality.eventsChecked) > 0;
  return {
    status: pass ? "pass" : "review",
    evidence: item.path,
    adapterId: item.manifest.adapterId,
    recordsSeen: item.manifest.recordsSeen,
    eventsWritten: item.manifest.eventsWritten,
    eventsChecked: item.quality.eventsChecked,
    qualityStatus: item.quality.status,
    networkUsed:
      item.manifest.networkUsed === "false" ? false : item.manifest.networkUsed,
    modifiedAt: item.modifiedAt,
  };
}

function mailStatus(mailReport, mailPreflight) {
  if (mailReport?.report?.status === "pass") {
    return {
      status: "pass",
      evidence: mailReport.report.packagePath,
      report: mailReport.path,
      eventCount: mailReport.report.eventCount,
      networkUsed: mailReport.report.networkUsed,
      modifiedAt: mailReport.modifiedAt,
    };
  }
  return {
    status:
      mailPreflight.status === "ready"
        ? "ready_for_user_export_check"
        : "waiting_for_exported_mbox",
    evidence: null,
    candidates: mailPreflight.userExportCandidates ?? mailPreflight.candidates,
    ignoredFixtureCandidates: mailPreflight.ignoredFixtureCandidates ?? [],
    preparedExportFolder: mailPreflight.preparedExportFolder ?? null,
    contentRead: false,
    nextStep: mailPreflight.nextStep,
  };
}

function aggregateStatus({ music, mailReport, mailPreflight }) {
  const musicStatus = packageStatus(music).status;
  const mail = mailStatus(mailReport, mailPreflight).status;
  if (musicStatus === "pass" && mail === "pass") {
    return "mail_and_music_checks_pass";
  }
  if (musicStatus === "pass") {
    return "waiting_for_user_exported_apple_mail";
  }
  return "incomplete";
}

function portableResult(source) {
  return {
    status: source.status,
    generatedAt: source.generatedAt,
    routes: Object.fromEntries(
      Object.entries(source.routes).map(([name, route]) => {
        const sanitized = { ...route };
        delete sanitized.evidence;
        delete sanitized.report;
        if (Array.isArray(sanitized.candidates)) {
          sanitized.candidateCount = sanitized.candidates.length;
          delete sanitized.candidates;
        }
        if (Array.isArray(sanitized.ignoredFixtureCandidates)) {
          sanitized.ignoredFixtureCandidateCount =
            sanitized.ignoredFixtureCandidates.length;
          delete sanitized.ignoredFixtureCandidates;
        }
        if (sanitized.preparedExportFolder) {
          sanitized.preparedExportFolder = {
            exists: sanitized.preparedExportFolder.exists,
            instructionsFileExists:
              sanitized.preparedExportFolder.instructionsFileExists,
          };
        }
        return [name, sanitized];
      }),
    ),
    completionGate: source.completionGate,
  };
}

function printText(result) {
  process.stdout.write(`Local app UAT status: ${result.status}\n`);
  if (result.evidenceRoot) {
    process.stdout.write(`Evidence root: ${result.evidenceRoot}\n`);
  }
  process.stdout.write("\n");
  for (const [label, route] of Object.entries(result.routes)) {
    process.stdout.write(`${label}: ${route.status}\n`);
    if (route.evidence) process.stdout.write(`  evidence: ${route.evidence}\n`);
    if (route.report) process.stdout.write(`  report: ${route.report}\n`);
    if (route.eventsChecked != null) {
      process.stdout.write(`  events checked: ${route.eventsChecked}\n`);
    }
    if (route.eventCount != null) {
      process.stdout.write(`  valid events: ${route.eventCount}\n`);
    }
    if (route.ignoredFixtureCandidates?.length > 0) {
      process.stdout.write(
        `  ignored test fixtures: ${route.ignoredFixtureCandidates.length}\n`,
      );
    }
    if (route.preparedExportFolder) {
      process.stdout.write(
        `  prepared folder: ${route.preparedExportFolder.exists ? "yes" : "no"} ${route.preparedExportFolder.path}\n`,
      );
      process.stdout.write(
        `  instructions readme: ${
          route.preparedExportFolder.instructionsFileExists ? "yes" : "no"
        }\n`,
      );
    }
    if (route.nextStep) process.stdout.write(`  next: ${route.nextStep}\n`);
  }
  process.stdout.write(`\nGate: ${result.completionGate}\n`);
}
