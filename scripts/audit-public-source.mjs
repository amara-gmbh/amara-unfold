#!/usr/bin/env node
// SPDX-License-Identifier: AGPL-3.0-or-later

import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const root = path.resolve(process.argv[2] ?? process.cwd());
const findings = [];
let fileCount = 0;
let byteCount = 0;

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

const skippedDirectories = new Set([
  ".git",
  "node_modules",
  "target",
  "dist",
  "test-results",
  "playwright-report",
  "coverage",
]);

const forbiddenDirectories = new Set([
  "project-memory",
  "private-data",
  "personal-data",
  "raw-data",
  "takeouts",
  "backups",
  "local-data",
]);

const privateOnlyFiles = new Set([
  ["scripts", ["verify-project", "memory.mjs"].join("-")].join("/"),
]);

const privateOnlyPackageScripts = [
  ["verify", "memory"].join(":"),
  ["check", "private"].join(":"),
];

const forbiddenExtensions = new Set([
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

const requiredPaths = [
  "README.md",
  "LICENSE",
  "NOTICE.md",
  "SECURITY.md",
  "CONTRIBUTING.md",
  "CODE_OF_CONDUCT.md",
  "Cargo.lock",
  "package-lock.json",
  "docs/architecture.md",
  "docs/adapters.md",
  "docs/build-from-source.md",
  "docs/configuration.md",
  "docs/decisions.md",
  "docs/privacy.md",
  "docs/quality-and-validation.md",
  "docs/README.md",
  "docs/repository-structure.md",
  "docs/release-scope.md",
  "docs/schema-v1.md",
  "docs/system-guide.md",
  "docs/third-party-licenses.md",
  "docs/trust-and-audit.md",
  ".github/ISSUE_TEMPLATE/bug_report.yml",
  ".github/pull_request_template.md",
];

const approvedPublicEmails = new Set(["info@amara.gmbh"]);

const textChecks = [
  {
    label: "absolute macOS user path",
    pattern: /\/Users\/(?!example\/)[A-Za-z0-9._-]+\//,
  },
  {
    label: "mounted-volume path",
    pattern: /\/Volumes\//,
  },
  {
    label: "absolute Windows user path",
    pattern: /[A-Za-z]:\\Users\\[^\\]+\\/,
  },
  {
    label: "private key",
    pattern: new RegExp(
      `-----BEGIN (?:RSA |OPENSSH |EC )?PRIVATE ${"KEY"}-----`,
    ),
  },
  {
    label: "AWS access key",
    pattern: /AKIA[0-9A-Z]{16}/,
  },
  {
    label: "GitHub access token",
    pattern: /gh[pousr]_[A-Za-z0-9_]{20,}/,
  },
  {
    label: "OpenAI-style secret key",
    pattern: /sk-(?:proj-)?[A-Za-z0-9_-]{20,}/,
  },
  {
    label: "assigned secret",
    pattern:
      /^\s*(?:api[_-]?key|client_secret|password|access_token|refresh_token)\s*[:=]\s*["']?[^\s"']{8,}/im,
  },
];

function relative(filePath) {
  return path.relative(root, filePath).split(path.sep).join("/");
}

function addFinding(filePath, message) {
  findings.push(`${relative(filePath)}: ${message}`);
}

function isApprovedBinaryAsset(relativePath, extension) {
  return (
    (relativePath === "assets/app-icon.png" && extension === ".png") ||
    ([".icns", ".ico", ".png"].includes(extension) &&
      relativePath.startsWith("apps/desktop/src-tauri/icons/"))
  );
}

function inspectFile(filePath) {
  const relativePath = relative(filePath);
  const basename = path.basename(filePath);
  const extension = path.extname(basename).toLowerCase();
  const stat = fs.statSync(filePath);

  fileCount += 1;
  byteCount += stat.size;

  if (privateOnlyFiles.has(relativePath)) {
    addFinding(filePath, "private continuity tool is not public source");
  }

  if (relativePath === "AGENTS.md") {
    addFinding(filePath, "private agent configuration is not public source");
  }

  if (basename === ".DS_Store" || basename.startsWith("._")) {
    addFinding(filePath, "macOS sidecar metadata is not public source");
  }

  if (
    basename === ".env" ||
    (basename.startsWith(".env.") && basename !== ".env.example")
  ) {
    addFinding(filePath, "environment file may contain credentials");
  }

  if (basename.endsWith(".log")) {
    addFinding(filePath, "runtime log is not public source");
  }

  if (forbiddenExtensions.has(extension)) {
    addFinding(filePath, `forbidden private/generated file type ${extension}`);
  }

  if ([".icns", ".ico", ".png"].includes(extension)) {
    if (!isApprovedBinaryAsset(relativePath, extension)) {
      addFinding(
        filePath,
        "binary image is outside the reviewed application-icon folder",
      );
    }
    return;
  }

  if (stat.size > 5 * 1024 * 1024) {
    addFinding(filePath, "file exceeds the 5 MiB public-source review limit");
    return;
  }

  const content = fs.readFileSync(filePath);
  if (content.includes(0)) {
    addFinding(filePath, "unreviewed binary file");
    return;
  }

  const text = content.toString("utf8");

  if (
    [".js", ".mjs", ".py", ".rs", ".sh", ".ts", ".tsx"].includes(extension) &&
    !text.slice(0, 500).includes("SPDX-License-Identifier: AGPL-3.0-or-later")
  ) {
    addFinding(
      filePath,
      "first-party source file is missing its SPDX license header",
    );
  }
  for (const check of textChecks) {
    if (check.pattern.test(text)) {
      addFinding(filePath, check.label);
    }
  }

  if (basename !== "package-lock.json") {
    const emails =
      text.match(/[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/g) ?? [];
    for (const email of emails) {
      if (
        approvedPublicEmails.has(email.toLowerCase()) ||
        /^\d+x\d+@\d+x\.[a-z]+$/i.test(email)
      ) {
        continue;
      }
      const domain = email.split("@").at(-1)?.toLowerCase();
      if (
        !["example.com", "example.invalid", "example.test"].includes(
          domain ?? "",
        )
      ) {
        addFinding(filePath, "non-placeholder email address");
        break;
      }
    }
  }
}

function walk(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const entryPath = path.join(directory, entry.name);

    if (entry.isSymbolicLink()) {
      addFinding(
        entryPath,
        "symbolic links are not allowed in the public source package",
      );
      continue;
    }

    if (entry.isDirectory()) {
      if (forbiddenDirectories.has(entry.name.toLowerCase())) {
        addFinding(entryPath, "private-data directory is forbidden");
        continue;
      }
      if (skippedDirectories.has(entry.name)) {
        continue;
      }
      walk(entryPath);
      continue;
    }

    if (entry.isFile()) {
      inspectFile(entryPath);
    }
  }
}

if (!fs.existsSync(root) || !fs.statSync(root).isDirectory()) {
  console.error(`Public source root does not exist: ${root}`);
  process.exit(2);
}

for (const entry of fs.readdirSync(root)) {
  if (!publicRootAllowlist.has(entry)) {
    findings.push(`${entry}: top-level path is not in the public allowlist`);
  }
}

for (const requiredPath of requiredPaths) {
  if (!fs.existsSync(path.join(root, requiredPath))) {
    findings.push(
      `${requiredPath}: required public documentation/source file is missing`,
    );
  }
}

const packagePath = path.join(root, "package.json");
if (fs.existsSync(packagePath)) {
  try {
    const packageData = JSON.parse(fs.readFileSync(packagePath, "utf8"));
    for (const scriptName of privateOnlyPackageScripts) {
      if (Object.hasOwn(packageData.scripts ?? {}, scriptName)) {
        findings.push(
          `package.json: private-only package script is forbidden (${scriptName})`,
        );
      }
    }
  } catch (error) {
    findings.push(
      `package.json: could not parse package metadata (${error.message})`,
    );
  }
}

const license = path.join(root, "LICENSE");
if (
  fs.existsSync(license) &&
  !fs
    .readFileSync(license, "utf8")
    .startsWith("GNU AFFERO GENERAL PUBLIC LICENSE\nVersion 3")
) {
  findings.push("LICENSE: canonical AGPL-3.0 text header is malformed");
}

walk(root);

if (findings.length > 0) {
  console.error("PUBLIC SOURCE AUDIT: FAIL");
  for (const finding of findings.sort()) {
    console.error(`- ${finding}`);
  }
  process.exit(1);
}

console.log("PUBLIC SOURCE AUDIT: PASS");
console.log(`Files reviewed: ${fileCount}`);
console.log(`Bytes reviewed: ${byteCount}`);
console.log(
  "Private working memory, raw exports, runtime databases, secrets, archives, logs, and build output: absent",
);
