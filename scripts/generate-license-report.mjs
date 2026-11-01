#!/usr/bin/env node
// SPDX-License-Identifier: AGPL-3.0-or-later

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const root = process.cwd();
const reportPath = path.join(root, "docs", "third-party-licenses.md");
const checkOnly = process.argv.includes("--check");
const permissiveOrCompatible = new Set([
  "0BSD",
  "Apache-2.0",
  "BlueOak-1.0.0",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "BSL-1.0",
  "CC-BY-3.0",
  "CC-BY-4.0",
  "CC0-1.0",
  "ISC",
  "MIT",
  "MIT-0",
  "MPL-2.0",
  "Python-2.0",
  "Unicode-3.0",
  "Unlicense",
  "Zlib",
]);

function cargoMetadata() {
  const result = spawnSync("cargo", ["metadata", "--format-version", "1"], {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 128 * 1024 * 1024,
  });
  if (result.status !== 0) {
    console.error(result.stderr);
    process.exit(2);
  }
  return JSON.parse(result.stdout);
}

function hasCompatibleChoice(expression) {
  const identifiers = expression.match(/[A-Za-z0-9.-]+/g) ?? [];
  return identifiers.some((identifier) =>
    permissiveOrCompatible.has(identifier),
  );
}

const cargo = cargoMetadata();
const rust = [
  ...new Map(
    cargo.packages
      .filter((pkg) => pkg.source)
      .map((pkg) => [
        `${pkg.name}@${pkg.version}`,
        {
          name: pkg.name,
          version: pkg.version,
          license: pkg.license ?? "UNKNOWN",
        },
      ]),
  ).values(),
];

const lock = JSON.parse(
  fs.readFileSync(path.join(root, "package-lock.json"), "utf8"),
);
const npm = Object.entries(lock.packages)
  .filter(
    ([location, pkg]) => location.startsWith("node_modules/") && pkg.version,
  )
  .map(([location, pkg]) => ({
    name: location.replace(/^node_modules\//, ""),
    version: pkg.version,
    license:
      pkg.license ??
      (location === "node_modules/css-value"
        ? "MIT (declared in Readme.md)"
        : "UNKNOWN"),
  }));

const unresolved = [...rust, ...npm].filter(
  (item) => item.license === "UNKNOWN" || !hasCompatibleChoice(item.license),
);
if (unresolved.length > 0) {
  console.error("DEPENDENCY LICENSE AUDIT: FAIL");
  for (const item of unresolved) {
    console.error(`- ${item.name}@${item.version}: ${item.license}`);
  }
  process.exit(1);
}

function rows(items) {
  return items
    .sort((left, right) =>
      `${left.name}@${left.version}`.localeCompare(
        `${right.name}@${right.version}`,
      ),
    )
    .map(
      (item) =>
        `| \`${item.name}\` | \`${item.version}\` | ${item.license.replaceAll("|", "\\|")} |`,
    )
    .join("\n");
}

const report = `# Third-party dependency licenses

This inventory is generated from the exact Rust and JavaScript lockfiles by
\`npm run licenses:generate\`. It records the complete resolved dependency graph,
including development and build tooling. It is attribution evidence, not a
substitute for the license texts distributed by each dependency.

The audit fails on missing license metadata or when an expression has no
reviewed permissive/compatible choice. \`css-value@0.0.1\` is the sole package
whose npm metadata omits the license field; its included \`Readme.md\` contains
the MIT license, which is recorded explicitly below.

## Rust crates (${rust.length})

| Package | Version | Declared license |
| --- | --- | --- |
${rows(rust)}

## JavaScript packages (${npm.length})

| Package | Version | Declared license |
| --- | --- | --- |
${rows(npm)}
`;

if (checkOnly) {
  if (
    !fs.existsSync(reportPath) ||
    fs.readFileSync(reportPath, "utf8") !== report
  ) {
    console.error(
      "DEPENDENCY LICENSE AUDIT: FAIL (generated report is missing or stale)",
    );
    process.exit(1);
  }
  console.log(
    `DEPENDENCY LICENSE AUDIT: PASS (${rust.length} Rust, ${npm.length} JavaScript)`,
  );
} else {
  fs.writeFileSync(reportPath, report);
  console.log(
    `Wrote ${path.relative(root, reportPath)} (${rust.length} Rust, ${npm.length} JavaScript)`,
  );
}
