// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  BadgeInfo,
  Check,
  Database,
  Globe2,
  LockKeyhole,
  Radio,
  ShieldCheck,
  SunMoon,
} from "lucide-react";
import desktopPackage from "../../package.json";
import { compactPath } from "../lib/format";
import type { DashboardSnapshot, PrivacyStatus, ThemeMode } from "../types";
import { PageHeader } from "../components/PageHeader";

export function SettingsView({
  dashboard,
  privacy,
  themeMode,
  onThemeModeChange,
}: {
  dashboard: DashboardSnapshot;
  privacy: PrivacyStatus;
  themeMode: ThemeMode;
  onThemeModeChange: (mode: ThemeMode) => void;
}) {
  return (
    <div className="view settings-view">
      <PageHeader
        eyebrow="Transparent by design"
        title="Settings and privacy"
        description="The current release has no cloud bridge, telemetry, analytics, or background network service."
      />

      <div className="settings-layout">
        <section className="settings-section panel">
          <div className="settings-heading">
            <div className="settings-icon">
              <SunMoon size={20} />
            </div>
            <div>
              <h2>Appearance</h2>
              <p>
                Warm coffee dark is the default; light and system modes remain
                available.
              </p>
            </div>
          </div>
          <div className="setting-row appearance-row">
            <div>
              <SunMoon size={16} />
              <span>
                <strong>Theme</strong>
                <small>System follows the current macOS appearance</small>
              </span>
            </div>
            <div className="theme-segmented" aria-label="Appearance theme">
              {(["system", "light", "dark"] as ThemeMode[]).map((mode) => (
                <button
                  aria-pressed={themeMode === mode}
                  className={themeMode === mode ? "active" : ""}
                  key={mode}
                  onClick={() => onThemeModeChange(mode)}
                  type="button"
                >
                  {mode[0].toUpperCase() + mode.slice(1)}
                </button>
              ))}
            </div>
          </div>
        </section>

        <section className="settings-section panel">
          <div className="settings-heading">
            <div className="settings-icon safe">
              <ShieldCheck size={20} />
            </div>
            <div>
              <h2>Privacy state</h2>
              <p>The application reports its effective runtime policy here.</p>
            </div>
          </div>
          <div className="setting-row">
            <div>
              <LockKeyhole size={16} />
              <span>
                <strong>Operating mode</strong>
                <small>All scanning and indexing happen on this device</small>
              </span>
            </div>
            <span className="setting-value safe">
              <Check size={13} />{" "}
              {privacy.mode === "local_only" ? "Local only" : privacy.mode}
            </span>
          </div>
          <div className="setting-row">
            <div>
              <Globe2 size={16} />
              <span>
                <strong>Network access</strong>
                <small>No upload or sync implementation is included</small>
              </span>
            </div>
            <span className="setting-value">
              {privacy.networkEnabled ? "Enabled" : "Off"}
            </span>
          </div>
          <div className="setting-row">
            <div>
              <Radio size={16} />
              <span>
                <strong>Analytics and telemetry</strong>
                <small>No usage events or crash reports are transmitted</small>
              </span>
            </div>
            <span className="setting-value">
              {privacy.telemetryEnabled ? "Enabled" : "Off"}
            </span>
          </div>
        </section>

        <section className="settings-section panel">
          <div className="settings-heading">
            <div className="settings-icon">
              <Database size={20} />
            </div>
            <div>
              <h2>Local index</h2>
              <p>
                Metadata is stored in a replaceable SQLite database in the app
                data directory.
              </p>
            </div>
          </div>
          <div className="database-path">
            <span>Database location</span>
            <code title={dashboard.databasePath}>
              {compactPath(dashboard.databasePath, 5)}
            </code>
          </div>
          <div className="database-note">
            <Check size={14} /> Database files and personal paths are excluded
            from source control by default.
          </div>
        </section>

        <section className="settings-section panel future-bridge">
          <div className="settings-heading">
            <div className="settings-icon">
              <Globe2 size={20} />
            </div>
            <div>
              <h2>amara Memory bridge</h2>
              <p>
                No amara Memory bridge is included in this release. Any later
                proposal would require a review screen and explicit, single-use
                consent before sending selected material.
              </p>
            </div>
          </div>
          <button className="disabled-connection" disabled type="button">
            Not included in this release
          </button>
        </section>

        <section className="settings-section panel">
          <div className="settings-heading">
            <div className="settings-icon">
              <BadgeInfo size={20} />
            </div>
            <div>
              <h2>About amara Unfold</h2>
              <p>
                Open-source software published by amara information security
                GmbH.
              </p>
            </div>
          </div>
          <div className="setting-row">
            <div>
              <BadgeInfo size={16} />
              <span>
                <strong>Version {desktopPackage.version}</strong>
                <small>amaramemory.com · AGPL source release candidate</small>
              </span>
            </div>
            <span className="setting-value">AGPL-3.0-or-later</span>
          </div>
        </section>
      </div>
    </div>
  );
}
