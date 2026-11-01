#!/usr/bin/env node
// SPDX-License-Identifier: AGPL-3.0-or-later

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(scriptDirectory, "..");
const now = new Date();
const stamp = [
  now.getFullYear(),
  String(now.getMonth() + 1).padStart(2, "0"),
  String(now.getDate()).padStart(2, "0"),
  "-",
  String(now.getHours()).padStart(2, "0"),
  String(now.getMinutes()).padStart(2, "0"),
].join("");
const destination = path.resolve(
  process.argv[2] ??
    path.join(root, "dist", `Amara-Unfold-Public-Source-${stamp}`),
);

const publicRootAllowlist = new Set([
  ".gitattributes",
  ".github",
  ".gitignore",
  ".prettierignore",
  "CHANGELOG.md",
  "CODE_OF_CONDUCT.md",
  "CONTRIBUTING.md",
  "Cargo.lock",
  "Cargo.toml",
  "LICENSE",
  "NOTICE.md",
  "README.md",
  "SECURITY.md",
  "adapters",
  "apps",
  "assets",
  "crates",
  "docs",
  "package-lock.json",
  "package.json",
  "rust-toolchain.toml",
  "scripts",
  "tests",
]);

const excludedDirectories = new Set([
  ".git",
  ".idea",
  ".vscode",
  "coverage",
  "dist",
  "backups",
  "local-data",
  "node_modules",
  "personal-data",
  "playwright-report",
  "private-data",
  "project-memory",
  "raw-data",
  "takeouts",
  "target",
  "test-results",
]);

const privateOnlyFiles = new Set([
  ["scripts", ["verify-project", "memory.mjs"].join("-")].join("/"),
]);

const privateOnlyPackageScripts = [
  ["verify", "memory"].join(":"),
  ["check", "private"].join(":"),
];

const excludedExtensions = new Set([
  ".7z",
  ".csv",
  ".db",
  ".docx",
  ".eml",
  ".gz",
  ".heic",
  ".jpeg",
  ".jpg",
  ".key",
  ".kml",
  ".kmz",
  ".log",
  ".m4a",
  ".mbox",
  ".mov",
  ".mp3",
  ".mp4",
  ".ods",
  ".ost",
  ".p12",
  ".pdf",
  ".pem",
  ".pst",
  ".rar",
  ".sqlite",
  ".sqlite3",
  ".tar",
  ".tsv",
  ".wav",
  ".xls",
  ".xlsx",
  ".zip",
]);

function relative(sourcePath) {
  return path.relative(root, sourcePath).split(path.sep).join("/");
}

function includeSource(sourcePath) {
  const relativePath = relative(sourcePath);
  if (relativePath === "") {
    return true;
  }

  const segments = relativePath.split("/");
  if (!publicRootAllowlist.has(segments[0])) {
    return false;
  }

  if (
    segments.some((segment) => excludedDirectories.has(segment.toLowerCase()))
  ) {
    return false;
  }

  if (privateOnlyFiles.has(relativePath)) {
    return false;
  }

  if (relativePath.startsWith("apps/desktop/src-tauri/gen/")) {
    return false;
  }

  const basename = path.basename(sourcePath);
  if (relativePath === "AGENTS.md") {
    return false;
  }
  if (basename === ".DS_Store" || basename.startsWith("._")) {
    return false;
  }
  if (
    basename === ".env" ||
    (basename.startsWith(".env.") && basename !== ".env.example")
  ) {
    return false;
  }

  return !excludedExtensions.has(path.extname(basename).toLowerCase());
}

if (
  destination === root ||
  (destination.startsWith(`${root}${path.sep}`) &&
    !destination.startsWith(`${root}${path.sep}dist${path.sep}`))
) {
  console.error(
    "Destination must be outside the working source tree or under its ignored dist folder.",
  );
  process.exit(2);
}

if (fs.existsSync(destination)) {
  const existing = fs
    .readdirSync(destination)
    .filter((name) => name !== ".git" && name !== ".DS_Store");
  if (existing.length > 0) {
    console.error(`Destination is not empty: ${destination}`);
    process.exit(2);
  }

  if (fs.existsSync(path.join(destination, ".git"))) {
    const history = spawnSync("git", ["rev-parse", "--verify", "HEAD"], {
      cwd: destination,
      encoding: "utf8",
    });
    if (history.status === 0) {
      console.error(`Destination already has Git history: ${destination}`);
      process.exit(2);
    }
  }
} else {
  fs.mkdirSync(destination, { recursive: true });
}

for (const entry of fs.readdirSync(root, { withFileTypes: true })) {
  const sourcePath = path.join(root, entry.name);
  if (!includeSource(sourcePath)) {
    continue;
  }
  fs.cpSync(sourcePath, path.join(destination, entry.name), {
    recursive: true,
    errorOnExist: false,
    force: true,
    filter: includeSource,
    preserveTimestamps: true,
  });
}

sanitizePublicPackage(destination);
deleteMacOsSidecars(destination);

const audit = spawnSync(
  process.execPath,
  [path.join(destination, "scripts", "audit-public-source.mjs"), destination],
  {
    cwd: destination,
    encoding: "utf8",
  },
);

process.stdout.write(audit.stdout);
process.stderr.write(audit.stderr);
if (audit.status !== 0) {
  process.exit(audit.status ?? 1);
}

console.log(`Public-repository-ready source: ${destination}`);

function sanitizePublicPackage(directory) {
  const packagePath = path.join(directory, "package.json");
  const packageData = JSON.parse(fs.readFileSync(packagePath, "utf8"));
  for (const scriptName of privateOnlyPackageScripts) {
    delete packageData.scripts?.[scriptName];
  }
  fs.writeFileSync(packagePath, `${JSON.stringify(packageData, null, 2)}\n`);
}

function deleteMacOsSidecars(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const entryPath = path.join(directory, entry.name);
    if (entry.name === ".DS_Store" || entry.name.startsWith("._")) {
      fs.rmSync(entryPath, { force: true, recursive: entry.isDirectory() });
      continue;
    }
    if (entry.isDirectory()) {
      deleteMacOsSidecars(entryPath);
    }
  }
}
