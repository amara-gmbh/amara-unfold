// SPDX-License-Identifier: AGPL-3.0-or-later

import path from "node:path";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import type { Options } from "@wdio/types";

const binary = path.resolve("target/debug/amara-unfold-desktop");

function isolatedAppData(): string {
  if (process.platform === "darwin") {
    return path.join(
      homedir(),
      "Library/Application Support/com.amaramemory.unfold.e2e",
    );
  }
  if (process.platform === "win32") {
    return path.join(
      process.env.APPDATA ?? path.join(homedir(), "AppData/Roaming"),
      "com.amaramemory.unfold.e2e",
    );
  }
  return path.join(
    process.env.XDG_DATA_HOME ?? path.join(homedir(), ".local/share"),
    "com.amaramemory.unfold.e2e",
  );
}

export const config: Options.Testrunner = {
  runner: "local",
  specs: [path.resolve("tests/native/**/*.e2e.ts")],
  maxInstances: 1,
  capabilities: [
    {
      browserName: "tauri",
      "tauri:options": { application: binary },
    },
  ],
  services: [
    [
      "@wdio/tauri-service",
      {
        appBinaryPath: binary,
        driverProvider: "embedded",
        embeddedPort: 4445,
        captureBackendLogs: true,
        captureFrontendLogs: true,
        startTimeout: 60_000,
      },
    ],
  ],
  framework: "jasmine",
  jasmineOpts: { defaultTimeoutInterval: 60_000 },
  reporters: ["spec"],
  logLevel: "warn",
  waitforTimeout: 10_000,
  connectionRetryTimeout: 90_000,
  connectionRetryCount: 1,
  outputDir: path.resolve("test-results/native/logs"),
  onPrepare() {
    // Never erase an existing test profile: it may contain data from another run.
    if (existsSync(isolatedAppData())) {
      throw new Error(
        "Native E2E profile already exists; preserve it and use a fresh test identifier.",
      );
    }
  },
};
