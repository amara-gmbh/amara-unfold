// SPDX-License-Identifier: AGPL-3.0-or-later

export type Section =
  "home" | "scan" | "memories" | "timeline" | "settings" | "help";
export type ThemeMode = "system" | "light" | "dark";
export type MediaKind = "image" | "video" | "unknown";

export interface ScanCounts {
  files: number;
  photos: number;
  videos: number;
  unknown: number;
  duplicates: number;
  errors: number;
  bytes: number;
}

export interface AssetRecord {
  id: number;
  sourceId: number;
  path: string;
  filename: string;
  extension: string;
  mediaKind: MediaKind;
  sizeBytes: number;
  createdAt: string | null;
  modifiedAt: string | null;
  capturedAt: string | null;
  width: number | null;
  height: number | null;
  durationSeconds: number | null;
  contentHash: string | null;
  latitude: number | null;
  longitude: number | null;
  deviceMake: string | null;
  deviceModel: string | null;
}

export interface SourceRecord {
  id: number;
  path: string;
  displayName: string;
  lastScannedAt: string | null;
  assetCount: number;
}

export interface TimelineGroup {
  key: string;
  label: string;
  count: number;
  photos: number;
  videos: number;
  totalBytes: number;
  sampleAssets: AssetRecord[];
}

export interface CandidateEvidence {
  path: string;
  capturedAt: string | null;
  reason: string;
}

export interface MemoryCandidate {
  id: number | null;
  clusterKey: string;
  title: string;
  dateLabel: string;
  placeHint: string | null;
  assetCount: number;
  photoCount: number;
  videoCount: number;
  evidence: CandidateEvidence[];
  sampleAssets: AssetRecord[];
  startedAt: string | null;
  endedAt: string | null;
  latitude: number | null;
  longitude: number | null;
  locationSpreadMeters: number | null;
  deviceLabels: string[];
  distinctFiles: number;
}

export interface DashboardSnapshot {
  counts: ScanCounts;
  sources: SourceRecord[];
  timeline: TimelineGroup[];
  candidates: MemoryCandidate[];
  recentScanErrors: ScanIssue[];
  databasePath: string;
  lastScanAt: string | null;
}

export interface ScanIssue {
  scanId: number;
  sourceName: string;
  message: string;
  occurredAt: string;
}

export interface ScanProgress {
  phase: "idle" | "scanning" | "complete" | "error";
  currentPath: string | null;
  filesSeen: number;
  mediaFound: number;
  errors: number;
  message: string;
}

export interface ScanResult {
  scanId: number;
  sourceId: number;
  cancelled: boolean;
  counts: ScanCounts;
  dashboard: DashboardSnapshot;
}

export interface PrivacyStatus {
  mode: string;
  networkEnabled: boolean;
  telemetryEnabled: boolean;
  message: string;
}

export interface MarkdownExportResult {
  folder: string;
  eventsWritten: number;
  eventsSkipped: number;
}

export type DatasetStage = "raw_export" | "open_markdown" | "mixed";
export type DatasetReadiness = "ready" | "needs_preparation" | "already_open";

export interface DatasetDetection {
  adapterId: string;
  label: string;
  provider: string;
  moduleId: string;
  stage: DatasetStage;
  format: string;
  readiness: DatasetReadiness;
  evidence: string[];
  commandPreview: string | null;
  nextStep: string;
}

export interface DatasetInspection {
  sourcePath: string;
  fileCount: number;
  inspectedPathCount: number;
  truncated: boolean;
  catalogVersion: number;
  outputContract: string;
  detections: DatasetDetection[];
}

export type LocalAppSourceStatus =
  "ready" | "needs_preparation" | "needs_permission" | "not_found";

export interface LocalAppSource {
  id: string;
  label: string;
  tier: string;
  provider: string;
  adapterId: string | null;
  path: string | null;
  status: LocalAppSourceStatus;
  recordCount: number;
  detail: string;
  nextStep: string;
  privacyNote: string;
  evidence: string[];
}

export interface AdapterExtractionProgress {
  adapterId: string;
  phase: "extracting" | "indexing" | "complete" | "cancelled" | "error";
  currentItem: string | null;
  recordsSeen: number;
  sourceRecordsIncluded: number;
  eventsWritten: number;
  eventsSkipped: number;
  errors: number;
  skipReasons?: Record<string, number>;
  totalHint: number | null;
  message: string;
}

export interface AdapterQualitySummary {
  status: "pass" | "review";
  scoreBasisPoints: number;
  acceptanceThresholdBasisPoints: number;
  eventsChecked: number;
  flaggedEvents: number;
  schemaErrors: number;
  provenanceErrors: number;
  encodingWarnings: number;
  sourceNormalizationWarnings: number;
  sourceEmptyBodyRecords: number;
  mailBodyWarnings: number;
  mailDirectionWarnings: number;
  implausibleDateWarnings: number;
  counterpartyWarnings: number;
  extractionErrors: number;
  accountingBalanced: boolean;
  skipReasonsBalanced: boolean;
}

export interface AdapterExtractionResult {
  adapterId: string;
  folder: string;
  recordsSeen: number;
  sourceRecordsIncluded: number;
  eventsWritten: number;
  eventsSkipped: number;
  skipReasons?: Record<string, number>;
  errors: number;
  cancelled: boolean;
  quality: AdapterQualitySummary;
}

export interface LocalAccessReleaseResult {
  adapterId: string;
  attempted: boolean;
  status: "released" | "failed" | "unavailable" | "not_needed";
  message: string;
}

export interface TimelineBuildResult {
  folder: string;
  eventsIndexed: number;
  duplicateEvents: number;
  rejectedEvents: number;
  datedEvents: number;
  undatedEvents: number;
  dayPages: number;
  monthPages: number;
  entityAnchors: number;
  crossSourceDays: number;
  strictPortable: boolean;
}

export interface TimelineCatalogEntry {
  id: string;
  date: string;
  datePrecision?: "minute" | "day" | "month" | "year" | "unknown";
  time?: string | null;
  manualDate?: boolean;
  source: string;
  module: string;
  eventType: string;
  title: string;
  preview: string;
  timelineFolder: string;
  eventPath: string;
  rawPath: string;
  reviewStatus: string;
  reviewReasons?: string[];
  tags?: string[];
}

export interface TimelineEventContent {
  body: string;
}

export interface TimelineDateEditResult {
  timeline: TimelineBuildResult;
  correction: {
    eventId: string;
    previousDate: string;
    date: string;
    datePrecision: string;
    time: string | null;
    previousPath: string;
    eventPath: string;
    action: "set" | "update" | "undo";
  };
}
