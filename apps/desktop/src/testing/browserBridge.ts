// SPDX-License-Identifier: AGPL-3.0-or-later

import type { UnlistenFn } from "@tauri-apps/api/event";
import type { DesktopBridge } from "../lib/desktop";
import type {
  AdapterExtractionProgress,
  AdapterQualitySummary,
  DashboardSnapshot,
  DatasetDetection,
  LocalAppSource,
  PrivacyStatus,
  ScanProgress,
  ScanResult,
  TimelineCatalogEntry,
} from "../types";

const syntheticQuality: AdapterQualitySummary = {
  status: "pass",
  scoreBasisPoints: 10_000,
  acceptanceThresholdBasisPoints: 9_950,
  eventsChecked: 333,
  flaggedEvents: 0,
  schemaErrors: 0,
  provenanceErrors: 0,
  encodingWarnings: 0,
  sourceNormalizationWarnings: 0,
  sourceEmptyBodyRecords: 0,
  mailBodyWarnings: 0,
  mailDirectionWarnings: 0,
  implausibleDateWarnings: 0,
  counterpartyWarnings: 0,
  extractionErrors: 0,
  accountingBalanced: true,
  skipReasonsBalanced: true,
};

const selectedFolder = "/Synthetic Archive/Jersey City";
const syntheticAppleMail = "/Synthetic Local Apps/Apple Mail";
const syntheticWrongAppleMail = "/Synthetic Local Apps/Wrong Mail Folder";
const syntheticApplePhotos = "/Synthetic Local Apps/Apple Photos";
const syntheticAppleMusic = "/Synthetic Local Apps/Apple Music";
const syntheticImmich = "/Synthetic Local Apps/Immich";
const syntheticAppleCalendar = "/Synthetic Local Apps/Apple Calendar";
const syntheticAppleContacts = "/Synthetic Local Apps/Apple Contacts";
const syntheticAirbnb = "/Synthetic Archive/Airbnb JSON Export";

const localPrivacy: PrivacyStatus = {
  mode: "local_only",
  networkEnabled: false,
  telemetryEnabled: false,
  message: "Files and metadata stay on this device.",
};

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
  databasePath: "/Synthetic/AppData/amara-unfold.db",
  lastScanAt: null,
};

const samplePhoto = {
  id: 1,
  sourceId: 1,
  path: `${selectedFolder}/IMG_4021.jpg`,
  filename: "IMG_4021.jpg",
  extension: "jpg",
  mediaKind: "image" as const,
  sizeBytes: 4_200_000,
  createdAt: "2019-08-30T12:42:00Z",
  modifiedAt: "2019-08-30T12:42:00Z",
  capturedAt: "2019-08-30T12:41:58Z",
  width: 4032,
  height: 3024,
  durationSeconds: null,
  contentHash: "synthetic-a",
  latitude: 40.7178,
  longitude: -74.0431,
  deviceMake: "Apple",
  deviceModel: "iPhone 11",
};

const sampleVideo = {
  id: 2,
  sourceId: 1,
  path: `${selectedFolder}/IMG_4022.mov`,
  filename: "IMG_4022.mov",
  extension: "mov",
  mediaKind: "video" as const,
  sizeBytes: 23_000_000,
  createdAt: "2019-08-30T13:02:00Z",
  modifiedAt: "2019-08-30T13:02:00Z",
  capturedAt: "2019-08-30T13:01:12Z",
  width: null,
  height: null,
  durationSeconds: null,
  contentHash: "synthetic-b",
  latitude: 40.718,
  longitude: -74.043,
  deviceMake: "Apple",
  deviceModel: "iPhone 11",
};

const populatedDashboard: DashboardSnapshot = {
  counts: {
    files: 333,
    photos: 330,
    videos: 2,
    unknown: 1,
    duplicates: 3,
    errors: 0,
    bytes: 8_423_489_536,
  },
  sources: [
    {
      id: 1,
      path: selectedFolder,
      displayName: "Jersey City",
      lastScannedAt: "2026-07-16T08:00:00Z",
      assetCount: 333,
    },
  ],
  timeline: [
    {
      key: "2019-08",
      label: "August 2019",
      count: 332,
      photos: 330,
      videos: 2,
      totalBytes: 8_400_000_000,
      sampleAssets: [samplePhoto, sampleVideo],
    },
  ],
  candidates: [
    {
      id: 1,
      clusterKey: "2019-08-30",
      title: "Possible memory · 2019-08-30 · Jersey City",
      dateLabel: "2019-08-30",
      placeHint: "Jersey City",
      assetCount: 332,
      photoCount: 330,
      videoCount: 2,
      sampleAssets: [
        samplePhoto,
        {
          ...samplePhoto,
          id: 3,
          path: `${selectedFolder}/IMG_4023.jpg`,
          filename: "IMG_4023.jpg",
          contentHash: "synthetic-c",
        },
      ],
      startedAt: "2019-08-30T12:41:58Z",
      endedAt: "2019-08-30T13:01:12Z",
      latitude: 40.7179,
      longitude: -74.04305,
      locationSpreadMeters: 24,
      deviceLabels: ["Apple iPhone 11"],
      distinctFiles: 332,
      evidence: [
        {
          path: `${selectedFolder}/IMG_4021.jpg`,
          capturedAt: "2019-08-30T12:41:58Z",
          reason: "EXIF capture date matches this cluster",
        },
        {
          path: `${selectedFolder}/IMG_4022.mov`,
          capturedAt: "2019-08-30T13:01:12Z",
          reason: "File timestamp matches this cluster",
        },
      ],
    },
  ],
  recentScanErrors: [],
  databasePath: "/Synthetic/AppData/amara-unfold.db",
  lastScanAt: "2026-07-16T08:00:00Z",
};

const manyMemoriesDashboard: DashboardSnapshot = {
  ...populatedDashboard,
  candidates: [
    ...populatedDashboard.candidates,
    ...Array.from({ length: 12 }, (_, index) => {
      const day = String(28 - index).padStart(2, "0");
      return {
        id: index + 2,
        clusterKey: `2019-08-${day}`,
        title: `Possible memory · 2019-08-${day} · Jersey City`,
        dateLabel: `2019-08-${day}`,
        placeHint: `Cluster ${index + 2}`,
        assetCount: 13,
        photoCount: 12,
        videoCount: 1,
        sampleAssets: [
          {
            ...samplePhoto,
            id: index + 20,
            filename: `IMG_${4100 + index}.jpg`,
            path: `${selectedFolder}/cluster-${index + 2}/IMG_${4100 + index}.jpg`,
            capturedAt: `2019-08-${day}T12:00:00Z`,
            contentHash: `synthetic-${index + 2}`,
          },
        ],
        startedAt: `2019-08-${day}T12:00:00Z`,
        endedAt: `2019-08-${day}T12:19:00Z`,
        latitude: 40.7179,
        longitude: -74.04305,
        locationSpreadMeters: 18,
        deviceLabels: ["Apple iPhone 11"],
        distinctFiles: 13,
        evidence: [
          {
            path: `${selectedFolder}/cluster-${index + 2}/IMG_${4100 + index}.jpg`,
            capturedAt: `2019-08-${day}T12:00:00Z`,
            reason: "EXIF capture date matches this cluster",
          },
        ],
      };
    }),
  ],
};

/**
 * Deterministic browser-only implementation for renderer automation.
 * It cannot be activated in a normal build and never touches the filesystem.
 */
export function createBrowserTestBridge(search: string): DesktopBridge {
  const scenario = new URLSearchParams(search).get("scenario") ?? "empty";
  const listeners = new Set<(progress: ScanProgress) => void>();
  const extractionListeners = new Set<
    (progress: AdapterExtractionProgress) => void
  >();
  let dashboard =
    scenario === "many-memories"
      ? manyMemoriesDashboard
      : scenario === "populated"
        ? populatedDashboard
        : emptyDashboard;
  let cancelled = false;
  const manualDates = new Map<
    string,
    {
      date: string;
      datePrecision: "minute" | "day" | "month" | "year";
      time: string | null;
    }
  >();

  const withManualDates = (entries: TimelineCatalogEntry[]) =>
    entries.map((entry) => {
      const correction = manualDates.get(entry.eventPath);
      return correction
        ? { ...entry, ...correction, manualDate: true }
        : {
            ...entry,
            datePrecision:
              entry.datePrecision ??
              (entry.date === "0000-00-00" ? "unknown" : "day"),
          };
    });

  const emit = (progress: ScanProgress) => {
    for (const listener of listeners) listener(progress);
  };

  return {
    async dashboard() {
      return structuredClone(dashboard);
    },
    async privacy() {
      return localPrivacy;
    },
    async localAppSources() {
      return localAppSourcesForScenario(scenario);
    },
    async inspectDataset(path) {
      if (scenario === "airbnb-coverage") {
        return {
          sourcePath: path,
          fileCount: 17,
          inspectedPathCount: 17,
          truncated: false,
          catalogVersion: 1,
          outputContract:
            "Amara Unfold Open Event Schema v1 Markdown with YAML frontmatter",
          detections: [
            {
              adapterId: "airbnb-json-takeout",
              label: "Airbnb data export",
              provider: "Airbnb",
              moduleId: "airbnb",
              stage: "raw_export",
              format: "Airbnb JSON directory",
              readiness: "ready",
              evidence: ["17 Airbnb JSON category files"],
              commandPreview:
                'amara-unfold extract airbnb-json-takeout --source "/Synthetic Archive/Airbnb JSON Export" --output <folder>',
              nextStep:
                "Convert reservations, messages, and reviews and account for every inspected category.",
            },
          ],
        };
      }
      const openMarkdown = scenario === "open-markdown";
      const localAppleMail = path === syntheticAppleMail;
      const localApplePhotos = path === syntheticApplePhotos;
      const localAppleMusic = path === syntheticAppleMusic;
      const localImmich = path === syntheticImmich;
      const localAppleCalendar = path === syntheticAppleCalendar;
      const localAppleContacts = path === syntheticAppleContacts;
      const detections: DatasetDetection[] = [
        {
          adapterId: localAppleContacts
            ? "apple-contacts-local"
            : localAppleCalendar
              ? "apple-calendar-local"
              : localAppleMail
                ? "apple-mail-local"
                : localApplePhotos
                  ? "photo-media-folder"
                  : localAppleMusic
                    ? "apple-music-local-library"
                    : localImmich
                      ? "immich-local-library"
                      : openMarkdown
                        ? "foreverlane-photo-markdown-v1"
                        : "photo-media-folder",
          label: localAppleContacts
            ? "Apple Contacts local cards"
            : localAppleCalendar
              ? "Apple Calendar local calendars"
              : localAppleMail
                ? "Apple Mail exported mailbox"
                : localApplePhotos
                  ? "Apple Photos exported media"
                  : localAppleMusic
                    ? "Apple Music local library"
                    : localImmich
                      ? "Immich local media library"
                      : openMarkdown
                        ? "Photo metadata Markdown"
                        : "Photo and video folder",
          provider: localAppleContacts
            ? "Apple Contacts"
            : localAppleCalendar
              ? "Apple Calendar"
              : localAppleMail
                ? "Apple Mail"
                : localApplePhotos
                  ? "Apple Photos"
                  : localAppleMusic
                    ? "Apple Music"
                    : localImmich
                      ? "Immich"
                      : openMarkdown
                        ? "Open Markdown"
                        : "Local files",
          moduleId: localAppleContacts
            ? "contacts"
            : localAppleCalendar
              ? "calendar"
              : localAppleMail
                ? "mail"
                : localApplePhotos
                  ? "photos"
                  : localAppleMusic
                    ? "apple-music"
                    : localImmich
                      ? "immich"
                      : "photos",
          stage: openMarkdown ? "open_markdown" : "raw_export",
          format: localAppleContacts
            ? "Local vCard (.vcf) files"
            : localAppleCalendar
              ? "Local iCalendar (.ics) files"
              : localAppleMail
                ? "Exported Apple Mail .mbox mailbox"
                : localApplePhotos
                  ? "Exported Apple Photos originals or media folder"
                  : localAppleMusic
                    ? "Music.app library metadata (one-time read)"
                    : localImmich
                      ? "Local image and video files"
                      : openMarkdown
                        ? "Markdown + YAML frontmatter"
                        : "Image and video files",
          readiness: openMarkdown ? "already_open" : "ready",
          evidence: localAppleContacts
            ? ["8 readable records found"]
            : localAppleCalendar
              ? ["12 readable records found"]
              : localAppleMail
                ? ["42 readable records found"]
                : localApplePhotos
                  ? ["10 readable records found"]
                  : localAppleMusic
                    ? ["16 readable records found"]
                    : localImmich
                      ? ["24 readable records found"]
                      : openMarkdown
                        ? ["contains manifest.yaml", "contains library/"]
                        : ["contains .jpg files"],
          commandPreview: openMarkdown
            ? null
            : localAppleContacts
              ? `amara-unfold extract apple-contacts-local --source "${syntheticAppleContacts}" --output <folder>`
              : localAppleCalendar
                ? `amara-unfold extract apple-calendar-local --source "${syntheticAppleCalendar}" --output <folder>`
                : localAppleMail
                  ? `amara-unfold extract apple-mail-local --source "${syntheticAppleMail}" --output <folder>`
                  : localApplePhotos
                    ? `amara-unfold extract photo-media-folder --source "${syntheticApplePhotos}" --output <folder>`
                    : localAppleMusic
                      ? `amara-unfold extract apple-music-local-library --source "${syntheticAppleMusic}" --output <folder>`
                      : localImmich
                        ? `amara-unfold extract immich-local-library --source "${syntheticImmich}" --output <folder>`
                        : './ingest.sh --source "/Synthetic Archive/Jersey City"',
          nextStep: openMarkdown
            ? "Validate the Markdown substrate and keep it portable; do not extract it again."
            : localAppleContacts
              ? "Read Apple Contacts vCards locally and emit people/contact Markdown."
              : localAppleCalendar
                ? "Read Apple Calendar .ics events locally and emit dated timeline Markdown."
                : localAppleMail
                  ? "Read an Apple Mail exported mailbox locally and emit cleaned email Markdown."
                  : localApplePhotos
                    ? "Read exported Apple Photos originals or media files and emit dated photo/video Markdown."
                    : localAppleMusic
                      ? "Create Markdown for tracks, playlists, playlist membership, and track Date Added. Timeline uses Date Added only. Music metadata is read only after Create Markdown."
                      : localImmich
                        ? "Read local Immich media files without network calls and emit photo/video Markdown."
                        : "Use the existing photo adapter to emit open Markdown metadata events.",
        },
      ];
      if (scenario === "multi-takeout") {
        detections.push(
          syntheticDetection("apple-music-tsv", "Apple Music", "apple-music"),
          syntheticDetection("pst-mail", "Microsoft Outlook", "mail"),
          syntheticDetection("youtube-takeout", "Google Takeout", "media"),
        );
      }
      return {
        sourcePath: path,
        fileCount: openMarkdown ? 55_517 : 333,
        inspectedPathCount: openMarkdown ? 56_000 : 340,
        truncated: false,
        catalogVersion: 1,
        outputContract:
          "Amara Unfold Open Event Schema v1 Markdown with YAML frontmatter",
        detections,
      };
    },
    async chooseFolder() {
      return scenario === "airbnb-coverage" ? syntheticAirbnb : selectedFolder;
    },
    async chooseFile() {
      return selectedFolder;
    },
    async chooseMailExportFolder() {
      if (scenario === "mail-wrong-export") return syntheticWrongAppleMail;
      return syntheticAppleMail;
    },
    async chooseOutputFolder() {
      return "/Synthetic/output";
    },
    async extractDataset(adapterId) {
      cancelled = false;
      if (scenario === "airbnb-coverage") {
        const skipReasons = {
          booking_session_supporting_metadata: 49,
          message_missing_readable_body_or_timestamp: 46,
          not_timeline_material_search_history: 751,
          not_timeline_material_activity_log: 429,
          not_timeline_material_profile_information: 16,
          not_timeline_material_search_telemetry_signals: 40,
          not_timeline_material_account_security: 24,
          not_timeline_material_other_categories: 155,
        };
        const complete: AdapterExtractionProgress = {
          adapterId,
          phase: "complete",
          currentItem: null,
          recordsSeen: 1_659,
          sourceRecordsIncluded: 0,
          eventsWritten: 149,
          eventsSkipped: 1_510,
          errors: 0,
          skipReasons,
          totalHint: 1_659,
          message:
            "Open Markdown ready: 149 written, 1510 skipped · quality passed",
        };
        for (const listener of extractionListeners) listener(complete);
        return {
          adapterId,
          folder: "/Synthetic/output/airbnb-json-takeout-unfold",
          recordsSeen: 1_659,
          sourceRecordsIncluded: 0,
          eventsWritten: 149,
          eventsSkipped: 1_510,
          skipReasons,
          errors: 0,
          cancelled: false,
          quality: { ...syntheticQuality, eventsChecked: 149 },
        };
      }
      if (scenario === "scan-error") {
        dashboard = {
          ...emptyDashboard,
          recentScanErrors: [
            {
              scanId: 9,
              sourceName: "Unreadable Archive",
              message: "/Synthetic Archive/locked: permission denied",
              occurredAt: "2026-07-16T08:00:00Z",
            },
          ],
        };
        throw new Error("The selected archive could not be read.");
      }
      if (
        scenario === "music-rerun" &&
        adapterId === "apple-music-local-library"
      ) {
        const unchanged = 17;
        const complete: AdapterExtractionProgress = {
          adapterId,
          phase: "complete",
          currentItem: null,
          recordsSeen: unchanged,
          sourceRecordsIncluded: 42,
          eventsWritten: 0,
          eventsSkipped: unchanged,
          skipReasons: { unchanged_rerun: unchanged },
          errors: 0,
          totalHint: unchanged,
          message: "Open Markdown ready: 0 new, 17 unchanged",
        };
        for (const listener of extractionListeners) listener(complete);
        return {
          adapterId,
          folder: `/Synthetic/output/${adapterId}-unfold`,
          recordsSeen: unchanged,
          sourceRecordsIncluded: 42,
          eventsWritten: 0,
          eventsSkipped: unchanged,
          skipReasons: { unchanged_rerun: unchanged },
          errors: 0,
          cancelled: false,
          quality: { ...syntheticQuality, eventsChecked: unchanged },
        };
      }
      for (const recordsSeen of [1, 120, 333]) {
        await wait(scenario === "slow-cancel" ? 180 : 20);
        if (cancelled) {
          const stopped: AdapterExtractionProgress = {
            adapterId,
            phase: "cancelled",
            currentItem: null,
            recordsSeen,
            sourceRecordsIncluded: recordsSeen,
            eventsWritten: recordsSeen,
            eventsSkipped: 0,
            errors: 0,
            totalHint: 333,
            message:
              "Extraction stopped safely; completed Markdown remains valid",
          };
          for (const listener of extractionListeners) listener(stopped);
          return {
            adapterId,
            folder: `/Synthetic/output/${adapterId}-unfold`,
            recordsSeen,
            sourceRecordsIncluded: recordsSeen,
            eventsWritten: recordsSeen,
            eventsSkipped: 0,
            errors: 0,
            cancelled: true,
            quality: { ...syntheticQuality, eventsChecked: recordsSeen },
          };
        }
        const payload: AdapterExtractionProgress = {
          adapterId,
          phase: "extracting",
          currentItem: `record-${recordsSeen}`,
          recordsSeen,
          sourceRecordsIncluded: recordsSeen,
          eventsWritten: recordsSeen,
          eventsSkipped: 0,
          errors: 0,
          totalHint: 333,
          message: `Normalized ${recordsSeen} records`,
        };
        for (const listener of extractionListeners) listener(payload);
      }
      const complete: AdapterExtractionProgress = {
        adapterId,
        phase: "complete",
        currentItem: null,
        recordsSeen: 333,
        sourceRecordsIncluded: 333,
        eventsWritten: 333,
        eventsSkipped: 0,
        errors: 0,
        totalHint: 333,
        message: "Open Markdown ready: 333 written, 0 unchanged",
      };
      for (const listener of extractionListeners) listener(complete);
      if (adapterId === "photo-media-folder") dashboard = populatedDashboard;
      return {
        adapterId,
        folder: `/Synthetic/output/${adapterId}-unfold`,
        recordsSeen: 333,
        sourceRecordsIncluded: 333,
        eventsWritten: 333,
        eventsSkipped: 0,
        errors: 0,
        cancelled: false,
        quality: syntheticQuality,
      };
    },
    async releaseLocalAppAccess(adapterId) {
      if (adapterId === "apple-music-local-library") {
        return {
          adapterId,
          attempted: true,
          status: "released",
          message:
            "Music Automation approval was released after this export; macOS should ask again next time.",
        };
      }
      return {
        adapterId,
        attempted: false,
        status: "not_needed",
        message:
          "This source uses a file/folder export and keeps no app Automation approval.",
      };
    },
    async buildTimeline() {
      const progress: AdapterExtractionProgress = {
        adapterId: "timeline-index",
        phase: "complete",
        currentItem: null,
        recordsSeen:
          scenario === "music-rerun"
            ? 17
            : scenario === "airbnb-coverage"
              ? 149
              : scenario === "multi-takeout"
                ? 999
                : 333,
        sourceRecordsIncluded: 0,
        eventsWritten:
          scenario === "music-rerun"
            ? 17
            : scenario === "airbnb-coverage"
              ? 149
              : scenario === "multi-takeout"
                ? 999
                : 333,
        eventsSkipped: 0,
        errors: 0,
        totalHint:
          scenario === "music-rerun"
            ? 17
            : scenario === "airbnb-coverage"
              ? 149
              : scenario === "multi-takeout"
                ? 999
                : 333,
        message: "Human-readable timeline and connections are ready",
      };
      for (const listener of extractionListeners) listener(progress);
      return {
        folder:
          scenario === "multi-archive"
            ? "/Synthetic/output/02-airbnb/Amara-Unfold-Timeline"
            : "/Synthetic/output/Amara-Unfold-Timeline",
        eventsIndexed:
          scenario === "music-rerun"
            ? 17
            : scenario === "airbnb-coverage"
              ? 149
              : scenario === "multi-takeout"
                ? 999
                : 333,
        duplicateEvents: 0,
        rejectedEvents: 0,
        datedEvents:
          scenario === "music-rerun"
            ? 17
            : scenario === "airbnb-coverage"
              ? 149
              : scenario === "multi-takeout"
                ? 990
                : 330,
        undatedEvents: scenario === "multi-takeout" ? 9 : 0,
        dayPages: 30,
        monthPages: 2,
        entityAnchors: 12,
        crossSourceDays: scenario === "multi-takeout" ? 8 : 0,
        strictPortable: true,
      };
    },
    async discoverTimelineArchives(folder) {
      if (scenario !== "multi-archive") return [];
      return [
        {
          folder,
          eventsIndexed: 3,
          duplicateEvents: 0,
          rejectedEvents: 0,
          datedEvents: 2,
          undatedEvents: 1,
          dayPages: 104,
          monthPages: 18,
          entityAnchors: 40,
          crossSourceDays: 17,
          strictPortable: true,
        },
        {
          folder: "/Synthetic/output/01-diary/Amara-Unfold-Timeline",
          eventsIndexed: 2,
          duplicateEvents: 0,
          rejectedEvents: 0,
          datedEvents: 1,
          undatedEvents: 1,
          dayPages: 390,
          monthPages: 24,
          entityAnchors: 0,
          crossSourceDays: 0,
          strictPortable: true,
        },
      ];
    },
    async timelineCatalog(folder) {
      if (scenario === "year-accordion") {
        return withManualDates(
          [
            ["2026", "Newest year event"],
            ["2025", "Middle year event"],
            ["2024", "Oldest year event"],
          ].map(([year, title]) => ({
            id: `evt-year-${year}`,
            date: `${year}-04-08`,
            source: "Local files",
            module: "photos",
            eventType: "photo",
            title,
            preview: `A dated event from ${year}.`,
            timelineFolder: folder,
            eventPath: `photos/events/${year}/04/evt-year-${year}.md`,
            rawPath: `/Synthetic Archive/${year}/event.jpg`,
            reviewStatus: "ready",
          })),
        );
      }
      if (scenario === "multi-archive" && folder.includes("01-diary")) {
        return withManualDates([
          {
            id: "evt-diary-001",
            date: "2019-08-29",
            source: "diary",
            module: "diary",
            eventType: "diary_entry",
            title: "A day worth remembering",
            preview:
              "The complete diary entry remains readable inside the chronological archive.",
            timelineFolder: folder,
            eventPath: "diary/events/2019/08/evt-diary-001.md",
            rawPath: "/Synthetic Sources/Diary.docx#paragraphs-10-14",
            reviewStatus: "ready",
          },
          {
            id: "evt-diary-undated",
            date: "0000-00-00",
            source: "diary",
            module: "diary",
            eventType: "diary_entry",
            title: "A timeless diary reference",
            preview:
              "A topical Diary note whose source contains no chronological date.",
            timelineFolder: folder,
            eventPath: "diary/events/0000/00/evt-diary-undated.md",
            rawPath: "/Synthetic Sources/Diary.docx#paragraphs-20-22",
            reviewStatus: "review",
            tags: ["diary", "reference"],
          },
        ]);
      }
      const entries: TimelineCatalogEntry[] = [
        {
          id: "evt-synthetic-001",
          date: "2019-08-30",
          source: scenario === "multi-archive" ? "location" : "Local files",
          module: scenario === "multi-archive" ? "airbnb" : "photos",
          eventType: scenario === "multi-archive" ? "stay" : "photo",
          title:
            scenario === "multi-archive"
              ? "Airbnb stay in Jersey City"
              : "Jersey City photo event",
          preview:
            "A **Jersey City photo** recorded on August 30, 2019. Camera metadata and the original path remain attached as evidence.",
          timelineFolder: folder,
          eventPath: "photos/events/2019/08/evt-synthetic-001.md",
          rawPath: "/Synthetic Archive/Jersey City/IMG_4021.jpg",
          reviewStatus: "ready",
        },
        {
          id: "evt-synthetic-002",
          date: "2019-08-17",
          source: "Local files",
          module: "photos",
          eventType: "photo",
          title: "August archive event",
          preview:
            "A second event from the August archive with its original media evidence.",
          timelineFolder: folder,
          eventPath: "photos/events/2019/08/evt-synthetic-002.md",
          rawPath: "/Synthetic Archive/Jersey City/IMG_4101.jpg",
          reviewStatus: "ready",
        },
        {
          id: "evt-synthetic-003",
          date: "0000-00-00",
          source: "Local files",
          module: "photos",
          eventType: "photo",
          title: "Undated archive event",
          preview:
            "This event has no defensible source date and remains in the undated review queue.",
          timelineFolder: folder,
          eventPath: "photos/events/undated/evt-synthetic-003.md",
          rawPath: "/Synthetic Archive/Jersey City/undated.jpg",
          reviewStatus: "ready",
        },
      ];
      if (scenario !== "multi-archive") {
        entries.push(
          ...Array.from({ length: 328 }, (_, index) => ({
            id: `evt-synthetic-archive-${String(index + 1).padStart(3, "0")}`,
            date: "2019-07-01",
            source: "Local files",
            module: "photos",
            eventType: "photo",
            title: `July archive event ${index + 1}`,
            preview:
              "A catalogued photo event retained to exercise complete archive accounting.",
            timelineFolder: folder,
            eventPath: `photos/events/2019/07/evt-synthetic-archive-${String(index + 1).padStart(3, "0")}.md`,
            rawPath: `/Synthetic Archive/Jersey City/archive-${index + 1}.jpg`,
            reviewStatus: "ready",
          })),
          ...Array.from({ length: 2 }, (_, index) => ({
            id: `evt-synthetic-undated-${index + 4}`,
            date: "0000-00-00",
            source: "Local files",
            module: "photos",
            eventType: "photo",
            title: `Undated archive event ${index + 2}`,
            preview:
              "This event remains in the review queue because its source has no defensible date.",
            timelineFolder: folder,
            eventPath: `photos/events/undated/evt-synthetic-undated-${index + 4}.md`,
            rawPath: `/Synthetic Archive/Jersey City/undated-${index + 2}.jpg`,
            reviewStatus: "review",
          })),
        );
      }
      return withManualDates(entries);
    },
    async timelineEvent(folder, eventPath) {
      if (folder.includes("01-diary")) {
        return {
          body: "## Diary entry\n\nThe complete diary entry remains readable inside the chronological archive.",
        };
      }
      if (eventPath.includes("evt-synthetic-001")) {
        return {
          body: "## Event\n\nA **Jersey City photo** recorded on August 30, 2019.\n\nThe complete event body stays readable here, while source paths remain under Evidence.",
        };
      }
      return {
        body: "## Event\n\nThe complete Markdown event body is available here.",
      };
    },
    async setTimelineEventDate(
      folder,
      eventPath,
      precision,
      value,
      time = null,
    ) {
      const date =
        precision === "year"
          ? `${value}-00-00`
          : precision === "month"
            ? `${value}-00`
            : value;
      const datePrecision = precision === "day" && time ? "minute" : precision;
      manualDates.set(eventPath, { date, datePrecision, time });
      return {
        timeline: {
          folder,
          eventsIndexed: 333,
          duplicateEvents: 0,
          rejectedEvents: 0,
          datedEvents: 331,
          undatedEvents: 2,
          dayPages: 31,
          monthPages: 3,
          entityAnchors: 12,
          crossSourceDays: 0,
          strictPortable: true,
        },
        correction: {
          eventId: eventPath,
          previousDate: "0000-00-00",
          date,
          datePrecision,
          time,
          previousPath: eventPath,
          eventPath,
          action: "set",
        },
      };
    },
    async undoTimelineEventDate(folder, eventPath) {
      const previous = manualDates.get(eventPath);
      manualDates.delete(eventPath);
      return {
        timeline: {
          folder,
          eventsIndexed: 333,
          duplicateEvents: 0,
          rejectedEvents: 0,
          datedEvents: 330,
          undatedEvents: 3,
          dayPages: 30,
          monthPages: 2,
          entityAnchors: 12,
          crossSourceDays: 0,
          strictPortable: true,
        },
        correction: {
          eventId: eventPath,
          previousDate: previous?.date ?? "0000-00-00",
          date: "0000-00-00",
          datePrecision: "unknown",
          time: null,
          previousPath: eventPath,
          eventPath,
          action: "undo",
        },
      };
    },
    async openAppleMail() {
      return "/Synthetic/Desktop/Amara Unfold Mail Exports";
    },
    async scan(): Promise<ScanResult> {
      cancelled = false;
      if (scenario === "scan-error") {
        await wait(30);
        dashboard = {
          ...emptyDashboard,
          recentScanErrors: [
            {
              scanId: 9,
              sourceName: "Unreadable Archive",
              message: "/Synthetic Archive/locked: permission denied",
              occurredAt: "2026-07-16T08:00:00Z",
            },
          ],
        };
        throw new Error("The selected archive could not be read.");
      }

      for (const [index, filesSeen] of [1, 120, 333].entries()) {
        await wait(scenario === "slow-cancel" ? 180 : 25);
        if (cancelled) {
          emit({
            phase: "complete",
            currentPath: null,
            filesSeen,
            mediaFound: filesSeen,
            errors: 0,
            message:
              "Scan cancelled; the previous complete index was preserved.",
          });
          return result(emptyDashboard, true);
        }
        emit({
          phase: "scanning",
          currentPath: `${selectedFolder}/batch-${index + 1}/IMG_${4000 + filesSeen}.jpg`,
          filesSeen,
          mediaFound: Math.max(1, filesSeen - 1),
          errors: 0,
          message: `Extracting metadata from ${filesSeen} files`,
        });
      }
      dashboard = populatedDashboard;
      emit({
        phase: "complete",
        currentPath: null,
        filesSeen: 333,
        mediaFound: 332,
        errors: 0,
        message: "Scan complete. The local index is ready.",
      });
      return result(populatedDashboard, false);
    },
    async cancelScan() {
      cancelled = true;
    },
    async removeSource(sourceId) {
      if (!dashboard.sources.some((source) => source.id === sourceId)) {
        throw new Error("that source is no longer in the local index");
      }
      dashboard = emptyDashboard;
      return structuredClone(dashboard);
    },
    async exportMemoryMarkdown() {
      return {
        folder: "/Synthetic/amara-unfold-2019-08-30",
        eventsWritten: 2,
        eventsSkipped: 0,
      };
    },
    async onScanProgress(handler): Promise<UnlistenFn> {
      listeners.add(handler);
      return () => listeners.delete(handler);
    },
    async onExtractionProgress(handler): Promise<UnlistenFn> {
      extractionListeners.add(handler);
      return () => extractionListeners.delete(handler);
    },
  };
}

function syntheticDetection(
  adapterId: string,
  provider: string,
  moduleId: string,
): DatasetDetection {
  return {
    adapterId,
    label: `${provider} export`,
    provider,
    moduleId,
    stage: "raw_export",
    format: "Provider takeout",
    readiness: "ready",
    evidence: ["recognized fixture"],
    commandPreview: `amara-unfold extract ${adapterId}`,
    nextStep: "Create open Markdown locally.",
  };
}

function localAppSourcesForScenario(scenario: string): LocalAppSource[] {
  const appleMusicReady = scenario === "local-apps";
  const applePhotosReady = scenario === "local-apps";
  const appleCalendarReady = scenario === "local-apps";
  const appleContactsReady = scenario === "local-apps";
  return [
    localApp(
      "apple-photos",
      "Apple Photos",
      "Photos export",
      "photo-media-folder",
      applePhotosReady ? syntheticApplePhotos : null,
      applePhotosReady ? "ready" : "needs_preparation",
      applePhotosReady ? 10 : 0,
    ),
    localApp(
      "apple-calendar",
      "Apple Calendar",
      "Calendar",
      "apple-calendar-local",
      appleCalendarReady ? syntheticAppleCalendar : null,
      appleCalendarReady ? "ready" : "needs_preparation",
      appleCalendarReady ? 12 : 0,
    ),
    localApp(
      "apple-contacts",
      "Apple Contacts",
      "Contacts",
      "apple-contacts-local",
      appleContactsReady ? syntheticAppleContacts : null,
      appleContactsReady ? "ready" : "needs_preparation",
      appleContactsReady ? 8 : 0,
    ),
    localApp(
      "apple-mail",
      "Apple Mail",
      "Mail export",
      "apple-mail-local",
      scenario === "local-apps" ? syntheticAppleMail : null,
      scenario === "local-apps" ? "ready" : "needs_preparation",
      0,
    ),
    localApp(
      "apple-music",
      "Apple Music",
      "Music",
      "apple-music-local-library",
      appleMusicReady ? syntheticAppleMusic : null,
      appleMusicReady ? "ready" : "needs_preparation",
      appleMusicReady ? 16 : 0,
    ),
    localApp("immich", "Immich", "Planned", null, null, "needs_preparation"),
    localApp(
      "apple-messages",
      "Apple Messages / iMessage",
      "Explicit opt-in",
      null,
      null,
      "needs_preparation",
    ),
    localApp(
      "apple-notes",
      "Apple Notes",
      "Explicit opt-in",
      null,
      null,
      "needs_preparation",
    ),
    localApp(
      "apple-journal",
      "Apple Journal",
      "Explicit opt-in",
      null,
      null,
      "not_found",
    ),
  ];
}

function localApp(
  id: string,
  label: string,
  tier: string,
  adapterId: string | null,
  path: string | null,
  status: LocalAppSource["status"],
  recordCount = 0,
): LocalAppSource {
  return {
    id,
    label,
    tier,
    provider: label,
    adapterId,
    path,
    status,
    recordCount,
    detail:
      status === "ready"
        ? `${recordCount.toLocaleString()} readable local records found.`
        : id === "apple-mail"
          ? "Export a mailbox from Apple Mail first, then choose the exported .mbox folder here."
          : path
            ? "Default app data location is known; use it to test whether readable records are available."
            : "Source is listed for native local-app intake; adapter/readable records are not ready in this scenario.",
    nextStep:
      status === "ready"
        ? readyLocalAppNextStep(id)
        : pendingLocalAppNextStep(id),
    privacyNote: "Local-only; never auto-run.",
    evidence:
      status === "ready" ? [`${recordCount} readable records found`] : [],
  };
}

function readyLocalAppNextStep(id: string): string {
  switch (id) {
    case "apple-calendar":
      return "Read Apple Calendar .ics events locally and emit clean timeline Markdown.";
    case "apple-contacts":
      return "Read local vCards and emit people/contact Markdown. Note bodies are not emitted by default.";
    case "apple-mail":
      return "Export a mailbox in Apple Mail, then choose the saved .mbox folder.";
    case "apple-music":
      return "Read local audio-file metadata and emit portable track events.";
    case "immich":
      return "Read local images/videos without network calls and emit photo/video Markdown events.";
    case "apple-photos":
      return "Read exported Apple Photos originals or a normal media folder and emit photo/video Markdown events.";
    default:
      return "Select this source and create open Markdown locally.";
  }
}

function pendingLocalAppNextStep(id: string): string {
  switch (id) {
    case "apple-mail":
      return "Export a mailbox in Apple Mail, then choose the saved .mbox folder.";
    case "apple-photos":
      return "Export originals from Apple Photos, then choose the folder containing photos/videos.";
    case "apple-messages":
      return "Messages is not available in this build. Use a ready source or exported folder.";
    case "apple-notes":
      return "Notes is not available in this build. Use a ready source or exported folder.";
    case "apple-journal":
      return "Journal is not available in this build. Use a ready source or exported folder.";
    default:
      return "This source is not available in this build. Use a ready source or exported folder.";
  }
}

function result(dashboard: DashboardSnapshot, cancelled: boolean): ScanResult {
  return {
    scanId: 1,
    sourceId: 1,
    cancelled,
    counts: dashboard.counts,
    dashboard: structuredClone(dashboard),
  };
}

function wait(milliseconds: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, milliseconds));
}
