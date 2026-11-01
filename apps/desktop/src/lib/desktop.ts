// SPDX-License-Identifier: AGPL-3.0-or-later

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  AdapterExtractionProgress,
  AdapterExtractionResult,
  DashboardSnapshot,
  DatasetInspection,
  LocalAppSource,
  LocalAccessReleaseResult,
  MarkdownExportResult,
  PrivacyStatus,
  ScanProgress,
  ScanResult,
  TimelineBuildResult,
  TimelineCatalogEntry,
  TimelineDateEditResult,
  TimelineEventContent,
} from "../types";
import { createBrowserTestBridge } from "../testing/browserBridge";

const emptyDashboard: DashboardSnapshot = {
  counts: {
    files: 0,
    photos: 0,
    videos: 0,
    unknown: 0,
    duplicates: 0,
    errors: 0,
    bytes: 0,
  },
  sources: [],
  timeline: [],
  candidates: [],
  recentScanErrors: [],
  databasePath: "Created when the desktop app starts",
  lastScanAt: null,
};

const localPrivacy: PrivacyStatus = {
  mode: "local_only",
  networkEnabled: false,
  telemetryEnabled: false,
  message: "Files and metadata stay on this device.",
};

function isDesktopRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

export interface DesktopBridge {
  dashboard(): Promise<DashboardSnapshot>;
  privacy(): Promise<PrivacyStatus>;
  localAppSources(options?: {
    includeUserLocations?: boolean;
  }): Promise<LocalAppSource[]>;
  inspectDataset(path: string): Promise<DatasetInspection>;
  extractDataset(
    adapterId: string,
    source: string,
    destination: string,
  ): Promise<AdapterExtractionResult | null>;
  releaseLocalAppAccess(adapterId: string): Promise<LocalAccessReleaseResult>;
  buildTimeline(destination: string): Promise<TimelineBuildResult>;
  discoverTimelineArchives(folder: string): Promise<TimelineBuildResult[]>;
  timelineCatalog(folder: string): Promise<TimelineCatalogEntry[]>;
  timelineEvent(
    folder: string,
    eventPath: string,
  ): Promise<TimelineEventContent>;
  setTimelineEventDate(
    folder: string,
    eventPath: string,
    precision: "day" | "month" | "year",
    value: string,
    time?: string | null,
  ): Promise<TimelineDateEditResult>;
  undoTimelineEventDate(
    folder: string,
    eventPath: string,
  ): Promise<TimelineDateEditResult>;
  openAppleMail(): Promise<string | null>;
  chooseFile(): Promise<string | null>;
  chooseFolder(): Promise<string | null>;
  chooseMailExportFolder(defaultPath?: string | null): Promise<string | null>;
  chooseOutputFolder(): Promise<string | null>;
  scan(folder: string): Promise<ScanResult>;
  cancelScan(): Promise<void>;
  removeSource(sourceId: number): Promise<DashboardSnapshot>;
  exportMemoryMarkdown(
    candidateId: number,
  ): Promise<MarkdownExportResult | null>;
  onScanProgress(
    handler: (progress: ScanProgress) => void,
  ): Promise<UnlistenFn>;
  onExtractionProgress(
    handler: (progress: AdapterExtractionProgress) => void,
  ): Promise<UnlistenFn>;
}

const tauriBridge: DesktopBridge = {
  dashboard: () => invoke("get_dashboard"),
  privacy: () => invoke("privacy_status"),
  localAppSources: (options = {}) =>
    invoke("local_app_sources", {
      includeUserLocations: options.includeUserLocations ?? false,
    }),
  inspectDataset: (path) => invoke("inspect_dataset", { path }),
  extractDataset: (adapterId, source, destination) =>
    invoke("extract_dataset", { adapterId, source, destination }),
  releaseLocalAppAccess: (adapterId) =>
    invoke("release_local_app_access", { adapterId }),
  buildTimeline: (destination) =>
    invoke("build_timeline_index", { destination }),
  discoverTimelineArchives: (folder) =>
    invoke("discover_timeline_archives", { folder }),
  timelineCatalog: (folder) => invoke("read_timeline_catalog", { folder }),
  timelineEvent: (folder, eventPath) =>
    invoke("read_timeline_event", { folder, eventPath }),
  setTimelineEventDate: (folder, eventPath, precision, value, time = null) =>
    invoke("set_timeline_event_date", {
      folder,
      eventPath,
      precision,
      value,
      time,
    }),
  undoTimelineEventDate: (folder, eventPath) =>
    invoke("undo_timeline_event_date", { folder, eventPath }),
  openAppleMail: () => invoke<string>("open_apple_mail"),
  async chooseOutputFolder() {
    const destination = await open({
      directory: true,
      multiple: false,
      title: "Choose where your open Markdown should live",
    });
    return typeof destination === "string" ? destination : null;
  },
  async chooseFolder() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Choose an archive to inspect locally",
    });
    return typeof selected === "string" ? selected : null;
  },
  async chooseFile() {
    const selected = await open({
      directory: false,
      multiple: false,
      title: "Choose one raw export file",
    });
    return typeof selected === "string" ? selected : null;
  },
  async chooseMailExportFolder(defaultPath?: string | null) {
    const selected = await open({
      ...(defaultPath ? { defaultPath } : {}),
      directory: true,
      multiple: false,
      title: "Choose the Apple Mail .mbox export folder",
    });
    return typeof selected === "string" ? selected : null;
  },
  scan: (folder) => invoke("scan_folder", { folder }),
  cancelScan: () => invoke("cancel_scan"),
  removeSource: (sourceId) => invoke("remove_source", { sourceId }),
  async exportMemoryMarkdown(candidateId) {
    const folder = await open({
      directory: true,
      multiple: false,
      title: "Choose where to save the open Markdown archive",
    });
    if (typeof folder !== "string") return null;
    return invoke("export_memory_markdown", { candidateId, folder });
  },
  onScanProgress: (handler) =>
    listen<ScanProgress>("scan-progress", (event) => handler(event.payload)),
  onExtractionProgress: (handler) =>
    listen<AdapterExtractionProgress>("extraction-progress", (event) =>
      handler(event.payload),
    ),
};

const browserPreviewBridge: DesktopBridge = {
  dashboard: async () => emptyDashboard,
  privacy: async () => localPrivacy,
  localAppSources: async () => [],
  inspectDataset: async () => ({
    sourcePath: "",
    fileCount: 0,
    inspectedPathCount: 0,
    truncated: false,
    catalogVersion: 1,
    outputContract:
      "Amara Unfold Open Event Schema v1 Markdown with YAML frontmatter",
    detections: [],
  }),
  extractDataset: async () => {
    throw new Error("Dataset extraction requires the desktop app.");
  },
  releaseLocalAppAccess: async (adapterId) => ({
    adapterId,
    attempted: false,
    status: "not_needed",
    message: "No local app access is held in browser preview.",
  }),
  buildTimeline: async () => {
    throw new Error("Timeline generation requires the desktop app.");
  },
  discoverTimelineArchives: async () => [],
  timelineCatalog: async () => [],
  timelineEvent: async () => ({ body: "" }),
  setTimelineEventDate: async () => {
    throw new Error("Manual Timeline dating requires the desktop app.");
  },
  undoTimelineEventDate: async () => {
    throw new Error("Manual Timeline dating requires the desktop app.");
  },
  openAppleMail: async () => null,
  chooseFile: async () => null,
  chooseFolder: async () => null,
  chooseMailExportFolder: async () => null,
  chooseOutputFolder: async () => null,
  scan: async () => {
    throw new Error("Folder scanning requires the desktop app.");
  },
  cancelScan: async () => undefined,
  removeSource: async () => {
    throw new Error("Source removal requires the desktop app.");
  },
  exportMemoryMarkdown: async () => ({
    folder: "/Synthetic/amara-unfold-2019-08-30",
    eventsWritten: 2,
    eventsSkipped: 0,
  }),
  onScanProgress: async () => () => undefined,
  onExtractionProgress: async () => () => undefined,
};

// Vite replaces this condition at build time. Rollup then removes the test
// adapter and its synthetic fixture data from ordinary production bundles.
const usesBrowserScenario =
  !isDesktopRuntime() &&
  (import.meta.env.VITE_E2E === "1" ||
    (import.meta.env.DEV && window.location.search.includes("scenario=")));

const activeBridge: DesktopBridge = isDesktopRuntime()
  ? tauriBridge
  : usesBrowserScenario
    ? createBrowserTestBridge(window.location.search)
    : browserPreviewBridge;

/**
 * The UI talks to one narrow bridge. Browser preview stays useful for visual
 * work, while all filesystem access remains exclusively in the desktop host.
 */
export const desktop: DesktopBridge = activeBridge;
