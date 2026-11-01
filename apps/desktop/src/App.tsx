// SPDX-License-Identifier: AGPL-3.0-or-later

import { AlertCircle, Check, ShieldCheck, X } from "lucide-react";
import {
  type KeyboardEvent as ReactKeyboardEvent,
  type PointerEvent as ReactPointerEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import "./App.css";
import { ExtractionConsentDialog } from "./components/ExtractionConsentDialog";
import { useExtractionConsent } from "./lib/useExtractionConsent";
import { Sidebar } from "./components/Sidebar";
import { isCatalogOnlyExtraction } from "./lib/extraction";
import { desktop } from "./lib/desktop";
import { batchWriteSummary, userFacingPath } from "./lib/format";
import { summarizeTimelineEntries } from "./lib/timeline";
import type {
  AdapterExtractionProgress,
  AdapterExtractionResult,
  DashboardSnapshot,
  DatasetDetection,
  DatasetInspection,
  LocalAppSource,
  LocalAccessReleaseResult,
  PrivacyStatus,
  Section,
  ThemeMode,
  TimelineBuildResult,
  TimelineCatalogEntry,
} from "./types";
import { HomeView } from "./views/HomeView";
import { HelpView } from "./views/HelpView";
import { MemoriesView } from "./views/MemoriesView";
import { ScanView } from "./views/ScanView";
import { SettingsView } from "./views/SettingsView";
import { TimelineView } from "./views/TimelineView";

const initialDashboard: DashboardSnapshot = {
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
  databasePath: "Loading local index…",
  lastScanAt: null,
};

const initialPrivacy: PrivacyStatus = {
  mode: "local_only",
  networkEnabled: false,
  telemetryEnabled: false,
  message: "Files and metadata stay on this device.",
};

const initialExtractionProgress: AdapterExtractionProgress = {
  adapterId: "",
  phase: "extracting",
  currentItem: null,
  recordsSeen: 0,
  sourceRecordsIncluded: 0,
  eventsWritten: 0,
  eventsSkipped: 0,
  errors: 0,
  skipReasons: {},
  totalHint: null,
  message: "Preparing local extraction",
};

const TIMELINE_CANCELLED =
  "timeline indexing cancelled; previous complete index preserved";
const MIN_PROGRESS_VISIBLE_MS = 1600;
const LATEST_TIMELINE_STORAGE_KEY = "amara-unfold-latest-timeline";
const TIMELINE_ARCHIVES_STORAGE_KEY = "amara-unfold-timeline-archives";
const HIDDEN_TIMELINE_ARCHIVES_STORAGE_KEY =
  "amara-unfold-hidden-timeline-archives";

function isTimelineBuildResult(
  value: Partial<TimelineBuildResult>,
): value is TimelineBuildResult {
  return (
    typeof value.folder === "string" &&
    typeof value.eventsIndexed === "number" &&
    typeof value.duplicateEvents === "number" &&
    typeof value.rejectedEvents === "number" &&
    typeof value.datedEvents === "number" &&
    typeof value.undatedEvents === "number" &&
    typeof value.dayPages === "number" &&
    typeof value.monthPages === "number" &&
    typeof value.entityAnchors === "number" &&
    typeof value.crossSourceDays === "number" &&
    typeof value.strictPortable === "boolean"
  );
}

function mergeTimelineArchives(
  timelines: TimelineBuildResult[],
): TimelineBuildResult[] {
  const byFolder = new Map<string, TimelineBuildResult>();
  for (const timeline of timelines) {
    if (!byFolder.has(timeline.folder)) byFolder.set(timeline.folder, timeline);
  }
  return [...byFolder.values()];
}

function saveTimelineArchives(timelines: TimelineBuildResult[]) {
  if (timelines.length === 0) {
    window.localStorage.removeItem(TIMELINE_ARCHIVES_STORAGE_KEY);
    window.localStorage.removeItem(LATEST_TIMELINE_STORAGE_KEY);
    return;
  }
  window.localStorage.setItem(
    TIMELINE_ARCHIVES_STORAGE_KEY,
    JSON.stringify(timelines),
  );
  if (timelines[0]) {
    window.localStorage.setItem(
      LATEST_TIMELINE_STORAGE_KEY,
      JSON.stringify(timelines[0]),
    );
  }
}

function loadHiddenTimelineFolders(): string[] {
  try {
    const saved = window.localStorage.getItem(
      HIDDEN_TIMELINE_ARCHIVES_STORAGE_KEY,
    );
    if (!saved) return [];
    const values = JSON.parse(saved) as unknown;
    return Array.isArray(values)
      ? values.filter((value): value is string => typeof value === "string")
      : [];
  } catch {
    return [];
  }
}

function saveHiddenTimelineFolders(folders: string[]) {
  if (folders.length === 0) {
    window.localStorage.removeItem(HIDDEN_TIMELINE_ARCHIVES_STORAGE_KEY);
  } else {
    window.localStorage.setItem(
      HIDDEN_TIMELINE_ARCHIVES_STORAGE_KEY,
      JSON.stringify([...new Set(folders)]),
    );
  }
}

function loadSavedTimelines(): TimelineBuildResult[] {
  try {
    const archived = window.localStorage.getItem(TIMELINE_ARCHIVES_STORAGE_KEY);
    if (archived) {
      const values = JSON.parse(archived) as Partial<TimelineBuildResult>[];
      if (Array.isArray(values)) {
        const timelines = values.filter(isTimelineBuildResult);
        if (timelines.length > 0) return mergeTimelineArchives(timelines);
      }
    }
    const latest = window.localStorage.getItem(LATEST_TIMELINE_STORAGE_KEY);
    if (!latest) return [];
    const value = JSON.parse(latest) as Partial<TimelineBuildResult>;
    return isTimelineBuildResult(value) ? [value] : [];
  } catch {
    return [];
  }
}

function ensureLocalAppDetection(
  inspection: DatasetInspection,
  source: LocalAppSource,
  folder: string,
): DatasetInspection {
  if (
    !source.adapterId ||
    inspection.detections.some(
      ({ adapterId }) => adapterId === source.adapterId,
    )
  ) {
    return inspection;
  }
  return {
    ...inspection,
    detections: [
      localAppDetection(source.adapterId, source, folder),
      ...inspection.detections,
    ],
  };
}

function localAppDetection(
  adapterId: string,
  source: LocalAppSource,
  folder: string,
): DatasetDetection {
  const metadata = localAppAdapterMetadata(adapterId, source);
  return {
    adapterId,
    label: metadata.label,
    provider: source.provider || source.label,
    moduleId: metadata.moduleId,
    stage: "raw_export",
    format: metadata.format,
    readiness: "ready",
    evidence:
      adapterId === "apple-music-local-library" && folder === source.path
        ? [
            "Standard Music.app library selected; track metadata has not been read yet",
          ]
        : source.recordCount > 0
          ? [`${source.recordCount} readable records found`]
          : [
              folder === source.path
                ? `Default app data location: ${userFacingPath(folder)}`
                : `Selected app data location: ${userFacingPath(folder)}`,
            ],
    commandPreview: `amara-unfold extract ${adapterId} --source "${folder}" --output <folder>`,
    nextStep: metadata.nextStep,
  };
}

function deferredLocalAppInspection(
  source: LocalAppSource,
  folder: string,
): DatasetInspection {
  if (!source.adapterId) {
    throw new Error(`${source.label} has no executable local adapter.`);
  }
  return {
    sourcePath: folder,
    fileCount: 0,
    inspectedPathCount: 0,
    truncated: false,
    catalogVersion: 1,
    outputContract:
      "Amara Unfold Open Event Schema v1 Markdown with YAML frontmatter",
    detections: [localAppDetection(source.adapterId, source, folder)],
  };
}

function localAppAdapterMetadata(
  adapterId: string,
  source: LocalAppSource,
): Pick<DatasetDetection, "label" | "moduleId" | "format" | "nextStep"> {
  switch (adapterId) {
    case "photo-media-folder":
      return {
        label:
          source.id === "apple-photos"
            ? "Apple Photos exported media"
            : "Photo and video folder",
        moduleId: "photos",
        format: "Local photo/video files",
        nextStep:
          "Read local photo/video files and emit dated Markdown events from file metadata.",
      };
    case "apple-mail-local":
      return {
        label: "Apple Mail exported mailbox",
        moduleId: "mail",
        format: "Exported Apple Mail .mbox mailbox",
        nextStep:
          "Read an Apple Mail exported mailbox locally and emit cleaned email Markdown.",
      };
    case "apple-music-local-library":
      return {
        label: "Apple Music local library",
        moduleId: "apple-music",
        format: "Music.app library metadata (one-time read)",
        nextStep:
          "Create Markdown for tracks, playlists, playlist membership, and track Date Added. Timeline uses Date Added only. Music metadata is read only after Create Markdown.",
      };
    case "immich-local-library":
      return {
        label: "Immich local media library",
        moduleId: "immich",
        format: "Local image and video files",
        nextStep:
          "Read local Immich media files without network calls and emit photo/video Markdown.",
      };
    case "apple-calendar-local":
      return {
        label: "Apple Calendar local calendars",
        moduleId: "calendar",
        format: "Local iCalendar (.ics) files",
        nextStep:
          "Read Apple Calendar .ics events locally and emit dated timeline Markdown.",
      };
    case "apple-contacts-local":
      return {
        label: "Apple Contacts local cards",
        moduleId: "contacts",
        format: "Local vCard (.vcf) files",
        nextStep:
          "Read Apple Contacts vCards locally and emit people/contact Markdown.",
      };
    default:
      return {
        label: source.label,
        moduleId: source.id,
        format: "Local app data",
        nextStep: source.nextStep,
      };
  }
}

function hasDetectedAdapter(
  inspection: DatasetInspection,
  adapterId: string,
): boolean {
  return inspection.detections.some(
    (detection) => detection.adapterId === adapterId,
  );
}

function App() {
  const { consent, requestConsent, decideConsent } = useExtractionConsent();
  const [section, setSection] = useState<Section>("home");
  const [dashboard, setDashboard] = useState(initialDashboard);
  const [timelineArchives, setTimelineArchives] =
    useState<TimelineBuildResult[]>(loadSavedTimelines);
  const [hiddenTimelineFolders, setHiddenTimelineFolders] = useState<string[]>(
    loadHiddenTimelineFolders,
  );
  const [loadedCatalogs, setLoadedCatalogs] = useState<{
    archives: TimelineBuildResult[];
    entries: TimelineCatalogEntry[];
    failures: number;
  } | null>(null);
  const catalogsLoading =
    timelineArchives.length > 0 &&
    loadedCatalogs?.archives !== timelineArchives;
  const timelineEntries = useMemo(
    () =>
      loadedCatalogs?.archives === timelineArchives
        ? loadedCatalogs.entries
        : [],
    [loadedCatalogs, timelineArchives],
  );
  const catalogFailures = catalogsLoading ? 0 : (loadedCatalogs?.failures ?? 0);
  const [privacy, setPrivacy] = useState(initialPrivacy);
  const [selectedFolder, setSelectedFolder] = useState<string | null>(null);
  const [outputFolder, setOutputFolder] = useState<string | null>(null);
  const [datasetInspection, setDatasetInspection] =
    useState<DatasetInspection | null>(null);
  const [localAppSources, setLocalAppSources] = useState<LocalAppSource[]>([]);
  const [refreshingLocalApps, setRefreshingLocalApps] = useState(false);
  const mailExportDiscoveryArmed = useRef(false);
  const [mailExportFolder, setMailExportFolder] = useState<string | null>(null);
  const [mailExportAutoChecking, setMailExportAutoChecking] = useState(false);
  const [selectedAdapterId, setSelectedAdapterId] = useState<string | null>(
    null,
  );
  const [inspectingDataset, setInspectingDataset] = useState(false);
  const [extracting, setExtracting] = useState(false);
  const [activeAdapterId, setActiveAdapterId] = useState<string | null>(null);
  const [extractionProgress, setExtractionProgress] =
    useState<AdapterExtractionProgress | null>(null);
  const [extractionResults, setExtractionResults] = useState<
    AdapterExtractionResult[]
  >([]);
  const cancelExtractionRequested = useRef(false);
  const [batchProgress, setBatchProgress] = useState<{
    current: number;
    total: number;
  } | null>(null);
  const workspaceScroll = useRef<HTMLDivElement>(null);
  const scrollbarDrag = useRef<{
    startY: number;
    startScrollTop: number;
  } | null>(null);
  const [scrollbar, setScrollbar] = useState({
    visible: false,
    thumbPercent: 100,
    topPercent: 0,
    valueNow: 0,
    valueMax: 0,
  });
  const [notice, setNotice] = useState<{
    tone: "success" | "error";
    message: string;
  } | null>(null);
  const [themeMode, setThemeMode] = useState<ThemeMode>(() => {
    const saved = window.localStorage.getItem("amara-unfold-theme");
    return saved === "light" || saved === "dark" || saved === "system"
      ? saved
      : "dark";
  });

  const rememberTimeline = useCallback((timeline: TimelineBuildResult) => {
    setHiddenTimelineFolders((current) => {
      const next = current.filter((folder) => folder !== timeline.folder);
      saveHiddenTimelineFolders(next);
      return next;
    });
    setTimelineArchives((current) => {
      const next = mergeTimelineArchives([
        timeline,
        ...current.filter(({ folder }) => folder !== timeline.folder),
      ]);
      saveTimelineArchives(next);
      return next;
    });
  }, []);

  const removeTimelineArchive = useCallback((folder: string) => {
    setHiddenTimelineFolders((current) => {
      const next = [...new Set([...current, folder])];
      saveHiddenTimelineFolders(next);
      return next;
    });
    setTimelineArchives((current) => {
      const next = current.filter((timeline) => timeline.folder !== folder);
      saveTimelineArchives(next);
      return next;
    });
    setNotice({
      tone: "success",
      message:
        "Archive layer removed from All events. Its Markdown files and original source were not deleted.",
    });
  }, []);

  const setTimelineEventDate = useCallback(
    async (
      entry: TimelineCatalogEntry,
      precision: "day" | "month" | "year",
      value: string,
      time: string | null,
    ) => {
      const result = await desktop.setTimelineEventDate(
        entry.timelineFolder,
        entry.eventPath,
        precision,
        value,
        time,
      );
      rememberTimeline(result.timeline);
      setNotice({
        tone: "success",
        message: `Date recorded as ${result.correction.date}${result.correction.time ? ` at ${result.correction.time}` : ""}. Timeline rebuilt; the stable event ID, raw source, and original evidence were preserved.`,
      });
    },
    [rememberTimeline],
  );

  const undoTimelineEventDate = useCallback(
    async (entry: TimelineCatalogEntry) => {
      const result = await desktop.undoTimelineEventDate(
        entry.timelineFolder,
        entry.eventPath,
      );
      rememberTimeline(result.timeline);
      setNotice({
        tone: "success",
        message:
          "Manual date assignment undone. The event returned to the undated review queue and the revision remains auditable in its Markdown file.",
      });
    },
    [rememberTimeline],
  );

  const refreshLocalAppSources = useCallback(
    async (quiet = false, includeUserLocations = false) => {
      setRefreshingLocalApps(true);
      try {
        const sources = await desktop.localAppSources({ includeUserLocations });
        setLocalAppSources(sources);
        if (!quiet) {
          const foundMail = sources.some(
            (source) => source.id === "apple-mail" && source.path,
          );
          setNotice({
            tone: "success",
            message: foundMail
              ? "App exports refreshed. Apple Mail export found — use it when you are ready."
              : "App exports refreshed. No Apple Mail .mbox export found yet.",
          });
        }
      } catch (error) {
        if (!quiet) setNotice({ tone: "error", message: errorMessage(error) });
      } finally {
        setRefreshingLocalApps(false);
      }
    },
    [],
  );

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const applyTheme = () => {
      const resolved =
        themeMode === "system" ? (media.matches ? "dark" : "light") : themeMode;
      document.documentElement.dataset.theme = resolved;
      document.documentElement.dataset.themeMode = themeMode;
      document.documentElement.style.colorScheme = resolved;
    };
    window.localStorage.setItem("amara-unfold-theme", themeMode);
    applyTheme();
    media.addEventListener("change", applyTheme);
    return () => media.removeEventListener("change", applyTheme);
  }, [themeMode]);

  useEffect(() => {
    if (timelineArchives.length === 0) return;
    let disposed = false;
    void Promise.all(
      timelineArchives.map((timeline) =>
        desktop
          .discoverTimelineArchives(timeline.folder)
          .catch(() => [] as TimelineBuildResult[]),
      ),
    ).then((groups) => {
      if (disposed) return;
      const hidden = new Set(hiddenTimelineFolders);
      const discovered = mergeTimelineArchives([
        ...groups.flat(),
        ...timelineArchives,
      ]).filter(({ folder }) => !hidden.has(folder));
      const unchanged =
        discovered.length === timelineArchives.length &&
        discovered.every(
          ({ folder }, index) => folder === timelineArchives[index]?.folder,
        );
      if (!unchanged) {
        setTimelineArchives(discovered);
        saveTimelineArchives(discovered);
      }
    });
    return () => {
      disposed = true;
    };
  }, [hiddenTimelineFolders, timelineArchives]);

  useEffect(() => {
    if (timelineArchives.length === 0) {
      // Reset stale results when the last archive is removed.
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setLoadedCatalogs({
        archives: timelineArchives,
        entries: [],
        failures: 0,
      });
      return;
    }
    let disposed = false;
    void Promise.all(
      timelineArchives.map((timeline) =>
        desktop
          .timelineCatalog(timeline.folder)
          .then((entries) => ({ entries, failed: false }))
          .catch(() => ({ entries: [], failed: true })),
      ),
    ).then((groups) => {
      if (disposed) return;
      const byId = new Map<string, TimelineCatalogEntry>();
      for (const entry of groups.flatMap((group) => group.entries)) {
        if (!byId.has(entry.id)) byId.set(entry.id, entry);
      }
      setLoadedCatalogs({
        archives: timelineArchives,
        entries: [...byId.values()],
        failures: groups.filter((group) => group.failed).length,
      });
    });
    return () => {
      disposed = true;
    };
  }, [timelineArchives]);

  const combinedTimeline = useMemo<TimelineBuildResult | null>(() => {
    if (timelineArchives.length === 0) return null;
    const sum = (select: (timeline: TimelineBuildResult) => number) =>
      timelineArchives.reduce((total, timeline) => total + select(timeline), 0);
    const catalogSummary = summarizeTimelineEntries(timelineEntries);
    return {
      folder: timelineArchives[0].folder,
      eventsIndexed: catalogSummary.eventsIndexed,
      duplicateEvents: sum(({ duplicateEvents }) => duplicateEvents),
      rejectedEvents: sum(({ rejectedEvents }) => rejectedEvents),
      datedEvents: catalogSummary.datedEvents,
      undatedEvents: catalogSummary.undatedEvents,
      dayPages: sum(({ dayPages }) => dayPages),
      monthPages: catalogSummary.monthPages,
      entityAnchors: sum(({ entityAnchors }) => entityAnchors),
      crossSourceDays: sum(({ crossSourceDays }) => crossSourceDays),
      strictPortable:
        !catalogsLoading &&
        catalogFailures === 0 &&
        timelineArchives.every(({ strictPortable }) => Boolean(strictPortable)),
    };
  }, [timelineArchives, timelineEntries, catalogsLoading, catalogFailures]);

  useEffect(() => {
    const refreshWhenVisible = () => {
      if (!document.hidden) {
        void refreshLocalAppSources(true, mailExportDiscoveryArmed.current);
      }
    };
    window.addEventListener("focus", refreshWhenVisible);
    document.addEventListener("visibilitychange", refreshWhenVisible);
    return () => {
      window.removeEventListener("focus", refreshWhenVisible);
      document.removeEventListener("visibilitychange", refreshWhenVisible);
    };
  }, [refreshLocalAppSources]);

  const mailExportReady = localAppSources.some(
    (source) => source.id === "apple-mail" && source.path,
  );

  useEffect(() => {
    if (!mailExportFolder || mailExportReady) {
      // Reflect that the external polling lifecycle has stopped.
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setMailExportAutoChecking(false);
      return;
    }
    setMailExportAutoChecking(true);
    let attempts = 0;
    const interval = window.setInterval(() => {
      attempts += 1;
      void refreshLocalAppSources(true, true);
      if (attempts >= 24) {
        window.clearInterval(interval);
        setMailExportAutoChecking(false);
      }
    }, 5000);
    return () => window.clearInterval(interval);
  }, [mailExportFolder, mailExportReady, refreshLocalAppSources]);

  useEffect(() => {
    void Promise.all([
      desktop.dashboard(),
      desktop.privacy(),
      desktop.localAppSources({ includeUserLocations: false }),
    ])
      .then(([snapshot, status, sources]) => {
        setDashboard(snapshot);
        setPrivacy(status);
        setLocalAppSources(sources);
      })
      .catch((error: unknown) => {
        setNotice({ tone: "error", message: errorMessage(error) });
      });
    let disposed = false;
    let unsubscribeExtraction: () => void = () => undefined;
    void desktop
      .onExtractionProgress((next) => setExtractionProgress(next))
      .then((unlisten) => {
        if (disposed) unlisten();
        else unsubscribeExtraction = unlisten;
      });
    return () => {
      disposed = true;
      unsubscribeExtraction();
    };
  }, []);

  const updateScrollbar = useCallback(() => {
    const scroller = workspaceScroll.current;
    if (!scroller) return;
    const valueMax = Math.max(0, scroller.scrollHeight - scroller.clientHeight);
    const visible = valueMax > 1;
    const thumbPercent = visible
      ? Math.max(8, (scroller.clientHeight / scroller.scrollHeight) * 100)
      : 100;
    const topPercent = visible
      ? (scroller.scrollTop / valueMax) * (100 - thumbPercent)
      : 0;
    setScrollbar({
      visible,
      thumbPercent,
      topPercent,
      valueNow: Math.round(scroller.scrollTop),
      valueMax: Math.round(valueMax),
    });
  }, []);

  useEffect(() => {
    const scroller = workspaceScroll.current;
    if (!scroller) return;
    let frame = 0;
    const schedule = () => {
      window.cancelAnimationFrame(frame);
      frame = window.requestAnimationFrame(updateScrollbar);
    };
    const resize = new ResizeObserver(schedule);
    const mutations = new MutationObserver(schedule);
    resize.observe(scroller);
    mutations.observe(scroller, { childList: true, subtree: true });
    scroller.addEventListener("scroll", schedule, { passive: true });
    schedule();
    return () => {
      window.cancelAnimationFrame(frame);
      resize.disconnect();
      mutations.disconnect();
      scroller.removeEventListener("scroll", schedule);
    };
  }, [updateScrollbar]);

  const moveScrollbar = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      const drag = scrollbarDrag.current;
      const scroller = workspaceScroll.current;
      if (!drag || !scroller) return;
      const trackHeight = event.currentTarget.getBoundingClientRect().height;
      const thumbHeight = (trackHeight * scrollbar.thumbPercent) / 100;
      const travel = Math.max(1, trackHeight - thumbHeight);
      scroller.scrollTop = Math.max(
        0,
        Math.min(
          scrollbar.valueMax,
          drag.startScrollTop +
            (event.clientY - drag.startY) * (scrollbar.valueMax / travel),
        ),
      );
    },
    [scrollbar.thumbPercent, scrollbar.valueMax],
  );

  const startScrollbar = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      const scroller = workspaceScroll.current;
      if (!scroller) return;
      const track = event.currentTarget;
      const trackRect = track.getBoundingClientRect();
      const thumbHeight = (trackRect.height * scrollbar.thumbPercent) / 100;
      const thumbTop = (trackRect.height * scrollbar.topPercent) / 100;
      const pointerY = event.clientY - trackRect.top;

      if (pointerY < thumbTop || pointerY > thumbTop + thumbHeight) {
        const travel = Math.max(1, trackRect.height - thumbHeight);
        const ratio = Math.max(
          0,
          Math.min(1, (pointerY - thumbHeight / 2) / travel),
        );
        scroller.scrollTop = ratio * scrollbar.valueMax;
      }
      scrollbarDrag.current = {
        startY: event.clientY,
        startScrollTop: scroller.scrollTop,
      };
      track.setPointerCapture(event.pointerId);
      event.preventDefault();
    },
    [scrollbar.thumbPercent, scrollbar.topPercent, scrollbar.valueMax],
  );

  const stopScrollbar = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      scrollbarDrag.current = null;
      if (event.currentTarget.hasPointerCapture(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId);
      }
    },
    [],
  );

  const keyScroll = useCallback(
    (event: ReactKeyboardEvent<HTMLDivElement>) => {
      const scroller = workspaceScroll.current;
      if (!scroller) return;
      const increments: Record<string, number> = {
        ArrowDown: 48,
        ArrowUp: -48,
        PageDown: scroller.clientHeight * 0.85,
        PageUp: -scroller.clientHeight * 0.85,
      };
      if (event.key === "Home")
        scroller.scrollTo({ top: 0, behavior: "smooth" });
      else if (event.key === "End") {
        scroller.scrollTo({ top: scrollbar.valueMax, behavior: "smooth" });
      } else if (event.key in increments) {
        scroller.scrollBy({ top: increments[event.key], behavior: "smooth" });
      } else return;
      event.preventDefault();
    },
    [scrollbar.valueMax],
  );

  const inspectChosenSource = useCallback(async (sourcePath: string) => {
    setSelectedFolder(sourcePath);
    setOutputFolder(null);
    setSelectedAdapterId(null);
    setDatasetInspection(null);
    setExtractionResults([]);
    setSection("scan");
    setInspectingDataset(true);
    const inspection = await desktop.inspectDataset(sourcePath);
    setDatasetInspection(inspection);
    setSelectedAdapterId(
      inspection.detections.find(({ readiness }) => readiness === "ready")
        ?.adapterId ??
        inspection.detections[0]?.adapterId ??
        null,
    );
    setInspectingDataset(false);
  }, []);

  const chooseFolder = useCallback(async () => {
    try {
      const folder = await desktop.chooseFolder();
      if (folder) await inspectChosenSource(folder);
    } catch (error) {
      setInspectingDataset(false);
      setNotice({ tone: "error", message: errorMessage(error) });
    }
  }, [inspectChosenSource]);

  const chooseFile = useCallback(async () => {
    try {
      const file = await desktop.chooseFile();
      if (file) await inspectChosenSource(file);
    } catch (error) {
      setInspectingDataset(false);
      setNotice({ tone: "error", message: errorMessage(error) });
    }
  }, [inspectChosenSource]);

  const chooseLocalAppSource = useCallback(
    async (source: LocalAppSource) => {
      if (!source.adapterId) {
        setNotice({
          tone: "error",
          message: `${source.label} is on the local-app roadmap, but it is not export-ready yet: ${source.detail}`,
        });
        return;
      }
      try {
        const folder =
          source.path ??
          (source.id === "apple-mail"
            ? await desktop.chooseMailExportFolder(mailExportFolder)
            : await desktop.chooseFolder());
        if (!folder) return;
        setSelectedFolder(folder);
        setOutputFolder(null);
        setSelectedAdapterId(null);
        setDatasetInspection(null);
        setExtractionResults([]);
        setSection("scan");
        setInspectingDataset(true);
        const inspection =
          source.id === "apple-music" && folder === source.path
            ? deferredLocalAppInspection(source, folder)
            : await desktop.inspectDataset(folder);
        if (
          source.id === "apple-mail" &&
          !hasDetectedAdapter(inspection, "apple-mail-local")
        ) {
          setSelectedFolder(null);
          setInspectingDataset(false);
          setNotice({
            tone: "error",
            message:
              "That does not look like an Apple Mail export. In Apple Mail, choose Mailbox › Export Mailbox…, then select the folder ending .mbox or the parent folder containing it.",
          });
          return;
        }
        const pinnedInspection = ensureLocalAppDetection(
          inspection,
          source,
          folder,
        );
        const preferred =
          pinnedInspection.detections.find(
            ({ adapterId }) => adapterId === source.adapterId,
          ) ??
          pinnedInspection.detections.find(
            ({ readiness }) => readiness === "ready",
          );
        setDatasetInspection(pinnedInspection);
        setSelectedAdapterId(preferred?.adapterId ?? source.adapterId);
        setInspectingDataset(false);
      } catch (error) {
        setInspectingDataset(false);
        setNotice({ tone: "error", message: errorMessage(error) });
      }
    },
    [mailExportFolder],
  );

  const openAppleMail = useCallback(async () => {
    try {
      const exportFolder = await desktop.openAppleMail();
      setMailExportFolder(exportFolder);
      mailExportDiscoveryArmed.current = true;
      await refreshLocalAppSources(true, true);
      setNotice({
        tone: "success",
        message: exportFolder
          ? `Prepared ${userFacingPath(exportFolder)} with a short READ ME and opened Apple Mail. In Mail, use Mailbox › Export Mailbox… and save the .mbox there; Unfold will show it as Export found.`
          : "Apple Mail opened. Use Mailbox › Export Mailbox… there, then return to Unfold and choose the exported .mbox folder.",
      });
    } catch (error) {
      setNotice({ tone: "error", message: errorMessage(error) });
    }
  }, [refreshLocalAppSources]);

  const chooseOutputFolder = useCallback(async () => {
    try {
      const folder = await desktop.chooseOutputFolder();
      if (folder) setOutputFolder(folder);
    } catch (error) {
      setNotice({ tone: "error", message: errorMessage(error) });
    }
  }, []);

  const backToSources = useCallback(() => {
    setSelectedFolder(null);
    setOutputFolder(null);
    setSelectedAdapterId(null);
    setDatasetInspection(null);
    setExtractionResults([]);
    setExtractionProgress(null);
    setActiveAdapterId(null);
    setBatchProgress(null);
    setNotice(null);
  }, []);

  const startDatasetExtraction = useCallback(
    async (adapterId: string) => {
      if (!selectedFolder || !outputFolder || extracting) return;
      if (isDesktopRuntime() && localAppExtractionNeedsConsent(adapterId)) {
        const proceed = await requestConsent({
          description: localAppExtractionConsentCopy(adapterId),
          sourcePath: selectedFolder,
          destinationPath: outputFolder,
        });
        if (!proceed) return;
      }
      cancelExtractionRequested.current = false;
      const progressStartedAt = performance.now();
      setExtracting(true);
      setBatchProgress(null);
      setActiveAdapterId(adapterId);
      setExtractionProgress({ ...initialExtractionProgress, adapterId });
      setExtractionResults([]);
      setNotice(null);
      try {
        const result = await desktop.extractDataset(
          adapterId,
          selectedFolder,
          outputFolder,
        );
        if (!result) {
          await waitForMinimumProgress(progressStartedAt);
          setExtractionProgress(null);
          return;
        }
        if (result.cancelled) {
          await waitForMinimumProgress(progressStartedAt);
          setExtractionResults([result]);
          setNotice({
            tone: "success",
            message:
              "Extraction stopped safely. No new files were committed; previous exports are unchanged.",
          });
          return;
        }
        if (result.recordsSeen === 0 && result.sourceRecordsIncluded === 0) {
          await waitForMinimumProgress(progressStartedAt);
          setExtractionProgress(null);
          setNotice({
            tone: "error",
            message: `${adapterDisplayName(adapterId)} did not find readable records in this source. No timeline was rebuilt. Check the source folder and macOS permissions, then try again.`,
          });
          return;
        }
        setExtractionResults([result]);
        const releaseResult = await releaseLocalAppAccessAfterExport(adapterId);
        if (isCatalogOnlyExtraction(result)) {
          await waitForMinimumProgress(progressStartedAt);
          setExtractionProgress({
            ...initialExtractionProgress,
            adapterId,
            phase: "complete",
            sourceRecordsIncluded: result.sourceRecordsIncluded,
            message: "Markdown catalog saved; no dated activity to index",
          });
          setNotice({
            tone: "success",
            message: `${adapterDisplayName(adapterId)} catalog saved: ${result.sourceRecordsIncluded.toLocaleString()} tracks in ${userFacingPath(result.folder)}. No source Date Added activity was available, so no Timeline was created or changed. Timeline event validation is not applicable.${localAccessReleaseNote(adapterId, releaseResult)}`,
          });
          return;
        }
        setActiveAdapterId("timeline-index");
        setExtractionProgress({
          ...initialExtractionProgress,
          adapterId: "timeline-index",
          phase: "indexing",
          message: "Building the human-readable timeline and connections",
        });
        let timeline;
        try {
          if (cancelExtractionRequested.current)
            throw new Error(TIMELINE_CANCELLED);
          timeline = await desktop.buildTimeline(outputFolder);
          rememberTimeline(timeline);
        } catch (error) {
          await waitForMinimumProgress(progressStartedAt);
          const cancelled = errorMessage(error) === TIMELINE_CANCELLED;
          setExtractionProgress((current) => ({
            ...(current ?? initialExtractionProgress),
            adapterId: "timeline-index",
            phase: cancelled ? "cancelled" : "error",
            message: cancelled
              ? "Timeline rebuild stopped; previous complete index preserved"
              : "Timeline index needs attention",
          }));
          setNotice({
            tone: cancelled ? "success" : "error",
            message: cancelled
              ? "Timeline rebuild stopped safely. The previous complete index is unchanged; completed Markdown exports remain saved. Run extraction again to refresh the Timeline."
              : `The ${result.eventsWritten.toLocaleString()} event files are safe, but the timeline index needs attention: ${errorMessage(error)}`,
          });
          return;
        }
        await waitForMinimumProgress(progressStartedAt);
        setExtractionProgress({
          adapterId: "timeline-index",
          phase: "complete",
          currentItem: null,
          recordsSeen: timeline.eventsIndexed,
          sourceRecordsIncluded: result.sourceRecordsIncluded,
          eventsWritten: result.eventsWritten,
          eventsSkipped: result.eventsSkipped,
          errors: result.errors,
          skipReasons: result.skipReasons ?? {},
          totalHint: timeline.eventsIndexed,
          message: "Human-readable timeline and Markdown files are ready",
        });
        setNotice({
          tone:
            timeline.strictPortable && result.quality.status === "pass"
              ? "success"
              : "error",
          message: `${adapterDisplayName(adapterId)} done: saved ${extractionOutputSummary(adapterId, result)} to ${userFacingPath(result.folder)}. Quality ${result.quality.status === "pass" ? "validated" : "needs review"} at ${(result.quality.scoreBasisPoints / 100).toFixed(2)}%. Timeline refreshed after this run: ${timeline.eventsIndexed.toLocaleString()} events, ${timeline.dayPages.toLocaleString()} day pages, ${timeline.crossSourceDays.toLocaleString()} cross-source dates.${localAccessReleaseNote(adapterId, releaseResult)}${timeline.strictPortable ? "" : " The timeline quality report also needs review."}`,
        });
        const latestDashboard = await desktop.dashboard().catch(() => null);
        if (latestDashboard) setDashboard(latestDashboard);
      } catch (error) {
        await waitForMinimumProgress(progressStartedAt);
        setExtractionProgress((current) => ({
          ...(current ?? { ...initialExtractionProgress, adapterId }),
          phase: "error",
          message: "Open Markdown extraction could not finish.",
        }));
        setNotice({ tone: "error", message: errorMessage(error) });
        const latestDashboard = await desktop.dashboard().catch(() => null);
        if (latestDashboard) setDashboard(latestDashboard);
      } finally {
        setExtracting(false);
      }
    },
    [
      extracting,
      outputFolder,
      rememberTimeline,
      requestConsent,
      selectedFolder,
    ],
  );

  const startDatasetBatch = useCallback(
    async (adapterIds: string[]) => {
      if (!selectedFolder || !outputFolder || extracting) return;
      const routes = [...new Set(adapterIds)];
      if (routes.length === 0) return;
      const consentRoutes = routes.filter(localAppExtractionNeedsConsent);
      if (isDesktopRuntime() && consentRoutes.length > 0) {
        const proceed = await requestConsent({
          description: consentRoutes
            .map(localAppExtractionConsentCopy)
            .join("\n\n"),
          sourcePath: selectedFolder,
          destinationPath: outputFolder,
        });
        if (!proceed) return;
      }
      cancelExtractionRequested.current = false;
      const progressStartedAt = performance.now();
      setExtracting(true);
      setNotice(null);
      setExtractionResults([]);
      let eventsWritten = 0;
      let unchangedEvents = 0;
      let completed = 0;
      let cancelled = false;
      let qualityNeedsReview = false;
      let timelineNeeded = false;
      let timeline: TimelineBuildResult | null = null;
      const failures: string[] = [];
      const releaseNotes: string[] = [];
      try {
        for (const [index, adapterId] of routes.entries()) {
          if (cancelExtractionRequested.current) {
            cancelled = true;
            break;
          }
          setBatchProgress({ current: index + 1, total: routes.length });
          setActiveAdapterId(adapterId);
          setExtractionProgress({
            ...initialExtractionProgress,
            adapterId,
            message: `Preparing export ${index + 1} of ${routes.length}`,
          });
          try {
            const result = await desktop.extractDataset(
              adapterId,
              selectedFolder,
              outputFolder,
            );
            if (!result) continue;
            if (result.cancelled) {
              cancelled = true;
              break;
            }
            if (
              result.recordsSeen === 0 &&
              result.sourceRecordsIncluded === 0
            ) {
              failures.push(
                `${adapterId}: no readable records found; check the selected export`,
              );
              continue;
            }
            setExtractionResults((current) => [...current, result]);
            qualityNeedsReview ||=
              result.quality.status !== "pass" &&
              !isCatalogOnlyExtraction(result);
            timelineNeeded ||= !isCatalogOnlyExtraction(result);
            eventsWritten += result.eventsWritten;
            unchangedEvents += result.skipReasons?.unchanged_rerun ?? 0;
            completed += 1;
            if (result.recordsSeen > 0 || result.eventsWritten > 0) {
              const releaseResult =
                await releaseLocalAppAccessAfterExport(adapterId);
              const releaseNote = localAccessReleaseNote(
                adapterId,
                releaseResult,
              ).trim();
              if (releaseNote && !releaseNotes.includes(releaseNote)) {
                releaseNotes.push(releaseNote);
              }
            }
          } catch (error) {
            failures.push(`${adapterId}: ${errorMessage(error)}`);
          }
        }
        if (completed > 0 && !cancelled && timelineNeeded) {
          setActiveAdapterId("timeline-index");
          setExtractionProgress({
            ...initialExtractionProgress,
            adapterId: "timeline-index",
            phase: "indexing",
            message: "Mapping every export into one human-readable timeline",
          });
          try {
            if (cancelExtractionRequested.current)
              throw new Error(TIMELINE_CANCELLED);
            timeline = await desktop.buildTimeline(outputFolder);
            rememberTimeline(timeline);
          } catch (error) {
            if (errorMessage(error) === TIMELINE_CANCELLED) cancelled = true;
            else failures.push(`timeline-index: ${errorMessage(error)}`);
          }
        }
        const latestDashboard = await desktop.dashboard().catch(() => null);
        if (latestDashboard) setDashboard(latestDashboard);
        await waitForMinimumProgress(progressStartedAt);
        if (completed > 0 && !cancelled) {
          const completedAdapterId = timelineNeeded
            ? "timeline-index"
            : routes[0];
          setActiveAdapterId(completedAdapterId);
          setExtractionProgress({
            adapterId: completedAdapterId,
            phase: failures.length ? "error" : "complete",
            currentItem: null,
            recordsSeen: timeline?.eventsIndexed ?? eventsWritten,
            sourceRecordsIncluded: 0,
            eventsWritten,
            eventsSkipped: 0,
            errors: failures.length,
            skipReasons: {},
            totalHint: timeline?.eventsIndexed ?? eventsWritten,
            message: failures.length
              ? "Markdown files were created; some exports need attention"
              : timelineNeeded
                ? "Human-readable timeline and Markdown files are ready"
                : "Markdown catalogs are ready; no Timeline activity was created",
          });
        }
        if (cancelled) {
          setExtractionProgress((previous) => ({
            ...(previous ?? initialExtractionProgress),
            phase: "cancelled",
            message:
              "Extraction stopped safely; completed exports remain saved",
          }));
        }
        setNotice({
          tone:
            failures.length > 0 ||
            qualityNeedsReview ||
            timeline?.strictPortable === false
              ? "error"
              : "success",
          message: cancelled
            ? `Extraction stopped safely after ${completed} completed exports. The previous complete Timeline index is unchanged; completed exports remain on disk. ${eventsWritten.toLocaleString()} events from completed exports remain saved.`
            : failures.length > 0
              ? `Saved ${eventsWritten.toLocaleString()} Markdown events from ${completed} exports. ${failures.length} exports need attention: ${failures.join("; ")}`
              : !timelineNeeded && completed > 0
                ? `Saved ${completed} Markdown catalogs to ${userFacingPath(outputFolder)}. No dated activity was available; no Timeline was created or changed. Timeline event validation is not applicable.`
                : `${batchWriteSummary(eventsWritten, unchangedEvents, completed, outputFolder)} Adapter quality ${qualityNeedsReview ? "needs review" : "validated"}. Timeline refreshed after this batch: ${timeline?.eventsIndexed.toLocaleString() ?? "all"} events, ${timeline?.dayPages.toLocaleString() ?? "the available"} day pages, ${timeline?.crossSourceDays.toLocaleString() ?? "the detected"} cross-source dates.${releaseNotes.length ? ` ${releaseNotes.join(" ")}` : ""}${timeline?.strictPortable === false ? " The timeline quality report also needs review." : ""}`,
        });
      } finally {
        setBatchProgress(null);
        setExtracting(false);
      }
    },
    [
      extracting,
      outputFolder,
      rememberTimeline,
      requestConsent,
      selectedFolder,
    ],
  );

  const cancelScan = useCallback(async () => {
    cancelExtractionRequested.current = true;
    await desktop.cancelScan();
    setExtractionProgress((current) =>
      current
        ? { ...current, message: "Stopping safely after the current record…" }
        : current,
    );
  }, []);

  return (
    <div className="app-shell">
      <Sidebar active={section} onNavigate={setSection} />
      <main className="workspace">
        <div className="workspace-titlebar" data-tauri-drag-region="deep">
          <span>{section === "home" ? "Archive overview" : section}</span>
          <div className="titlebar-status">
            <ShieldCheck size={14} /> Local only
          </div>
        </div>
        <div
          aria-label="Workspace content"
          className="workspace-scroll"
          id="workspace-scroll"
          ref={workspaceScroll}
          tabIndex={0}
        >
          {(section === "home" || section === "timeline") &&
            catalogsLoading && (
              <p role="status">
                Loading saved event catalogs. Counts reflect loaded events only.
              </p>
            )}
          {(section === "home" || section === "timeline") &&
            catalogFailures > 0 && (
              <p role="status">
                {catalogFailures} saved{" "}
                {catalogFailures === 1 ? "archive could" : "archives could"} not
                be loaded. Counts include readable catalogs only; original files
                are unchanged.
              </p>
            )}
          {section === "home" && (
            <HomeView
              dashboard={dashboard}
              timeline={combinedTimeline}
              onNavigate={setSection}
              onChooseFolder={() => void chooseFolder()}
            />
          )}
          {section === "scan" && (
            <ScanView
              selectedFolder={selectedFolder}
              outputFolder={outputFolder}
              selectedAdapterId={selectedAdapterId}
              extracting={extracting}
              activeAdapterId={activeAdapterId}
              extractionProgress={extractionProgress}
              extractionResults={extractionResults}
              batchProgress={batchProgress}
              inspectingDataset={inspectingDataset}
              datasetInspection={datasetInspection}
              recentIssues={dashboard.recentScanErrors}
              localAppSources={localAppSources}
              preparedMailExportFolder={mailExportFolder}
              mailExportAutoChecking={mailExportAutoChecking}
              refreshingLocalApps={refreshingLocalApps}
              onChooseFolder={() => void chooseFolder()}
              onChooseFile={() => void chooseFile()}
              onChooseLocalAppSource={(source) =>
                void chooseLocalAppSource(source)
              }
              onOpenAppleMail={() => void openAppleMail()}
              onRefreshLocalAppSources={() =>
                void refreshLocalAppSources(false, true)
              }
              onChooseOutputFolder={() => void chooseOutputFolder()}
              onSelectAdapter={setSelectedAdapterId}
              onExtractDataset={(adapterId) =>
                void startDatasetExtraction(adapterId)
              }
              onExtractAllDatasets={(adapterIds) =>
                void startDatasetBatch(adapterIds)
              }
              onBackToSources={backToSources}
              onOpenTimeline={() => setSection("timeline")}
              onCancel={() => void cancelScan()}
            />
          )}
          {section === "memories" && <MemoriesView dashboard={dashboard} />}
          {section === "timeline" && (
            <TimelineView
              archiveCount={timelineArchives.length}
              archives={timelineArchives}
              timeline={combinedTimeline}
              entries={timelineEntries}
              onRemoveArchive={removeTimelineArchive}
              onSetEventDate={setTimelineEventDate}
              onUndoEventDate={undoTimelineEventDate}
              onReadEvent={(timelineFolder, eventPath) => {
                if (!combinedTimeline) {
                  return Promise.reject(
                    new Error("No generated timeline is open."),
                  );
                }
                return desktop.timelineEvent(timelineFolder, eventPath);
              }}
            />
          )}
          {section === "help" && <HelpView onNavigate={setSection} />}
          {section === "settings" && (
            <SettingsView
              dashboard={dashboard}
              privacy={privacy}
              themeMode={themeMode}
              onThemeModeChange={setThemeMode}
            />
          )}
        </div>
        <div
          aria-controls="workspace-scroll"
          aria-label="Page position"
          aria-orientation="vertical"
          aria-valuemax={scrollbar.valueMax}
          aria-valuemin={0}
          aria-valuenow={scrollbar.valueNow}
          className={`workspace-scrollbar${scrollbar.visible ? " visible" : ""}`}
          onKeyDown={keyScroll}
          onPointerCancel={stopScrollbar}
          onPointerDown={startScrollbar}
          onPointerMove={moveScrollbar}
          onPointerUp={stopScrollbar}
          role="scrollbar"
          tabIndex={scrollbar.visible ? 0 : -1}
        >
          <span
            style={{
              height: `${scrollbar.thumbPercent}%`,
              top: `${scrollbar.topPercent}%`,
            }}
          />
        </div>
      </main>

      <ExtractionConsentDialog consent={consent} onDecision={decideConsent} />
      {notice && (
        <div className={`toast ${notice.tone}`} role="status">
          {notice.tone === "success" ? (
            <Check size={17} />
          ) : (
            <AlertCircle size={17} />
          )}
          <span>{notice.message}</span>
          <button
            onClick={() => setNotice(null)}
            aria-label="Dismiss"
            type="button"
          >
            <X size={15} />
          </button>
        </div>
      )}
    </div>
  );
}

async function releaseLocalAppAccessAfterExport(
  adapterId: string,
): Promise<LocalAccessReleaseResult | null> {
  if (adapterId !== "apple-music-local-library") return null;
  try {
    return await desktop.releaseLocalAppAccess(adapterId);
  } catch (error) {
    return {
      adapterId,
      attempted: true,
      status: "unavailable",
      message: `Unfold finished the one-time Music read, but could not confirm Automation reset (${errorMessage(error)}). Revoke it manually in System Settings › Privacy & Security › Automation.`,
    };
  }
}

function localAccessReleaseNote(
  adapterId: string,
  releaseResult?: LocalAccessReleaseResult | null,
): string {
  if (
    ![
      "apple-mail-local",
      "apple-music-local-library",
      "apple-calendar-local",
      "apple-contacts-local",
      "immich-local-library",
    ].includes(adapterId)
  ) {
    return "";
  }
  if (adapterId === "apple-music-local-library") {
    if (releaseResult?.message) return ` ${releaseResult.message}`;
    return " Unfold has finished the one-time local Music read and keeps no ongoing Music.app connection. macOS Automation reset was not checked; revoke remembered approval in System Settings › Privacy & Security › Automation if needed.";
  }
  if (adapterId === "apple-mail-local") {
    return " Unfold read the exported mailbox as a local file/folder and keeps no Mail.app permission or background access.";
  }
  return " Unfold has finished the one-time local read and stores no ongoing access.";
}

function localAppExtractionNeedsConsent(adapterId: string): boolean {
  return [
    "apple-mail-local",
    "apple-music-local-library",
    "apple-calendar-local",
    "apple-contacts-local",
    "immich-local-library",
  ].includes(adapterId);
}

function localAppExtractionConsentCopy(adapterId: string): string {
  const name = adapterDisplayName(adapterId);
  if (adapterId === "apple-mail-local") {
    return `${name}\n\nUnfold will now read the exported mailbox folder once and create Markdown in the destination you chose.\n\nNothing is uploaded. Apple Mail is not contacted. Originals are not changed. Unfold keeps no Mail.app permission or background access.`;
  }
  if (adapterId === "apple-music-local-library") {
    return `${name}\n\nUnfold will now do a one-time, read-only local Music export. macOS may ask whether Unfold can access Music.app automation for this run.\n\nNothing is uploaded. Originals are not changed. Unfold keeps no background Music.app connection. After the export, Unfold will ask macOS to forget the Music Automation approval where supported. If macOS keeps it, you can still revoke it in System Settings › Privacy & Security › Automation.`;
  }
  if (adapterId === "immich-local-library") {
    return `${name}\n\nUnfold will now read the selected local Immich originals folder once and create Markdown in the destination you chose.\n\nNothing is uploaded. No Immich server or network API is contacted. Originals are not changed. Unfold keeps no background access.`;
  }
  if (adapterId === "apple-calendar-local") {
    return `${name}\n\nUnfold will now read the selected Calendar .ics export folder once and create Markdown in the destination you chose.\n\nNothing is uploaded. iCloud is not contacted. Calendar.app's private live database is not inspected. Originals are not changed. Unfold keeps no background Calendar access.`;
  }
  if (adapterId === "apple-contacts-local") {
    return `${name}\n\nUnfold will now read the selected Contacts vCard export folder once and create Markdown in the destination you chose.\n\nNothing is uploaded. iCloud is not contacted. Contacts.app's private live database is not inspected. Originals are not changed. Unfold keeps no background Contacts access.`;
  }
  return `${name}\n\nUnfold will now do a one-time, read-only local export and create Markdown in the destination you chose.\n\nNothing is uploaded. Originals are not changed. Unfold keeps no background access.`;
}

function adapterDisplayName(adapterId: string): string {
  switch (adapterId) {
    case "apple-mail-local":
      return "Apple Mail";
    case "photo-media-folder":
      return "Apple Photos / media folder";
    case "apple-music-local-library":
      return "Apple Music";
    case "apple-calendar-local":
      return "Apple Calendar";
    case "apple-contacts-local":
      return "Apple Contacts";
    case "immich-local-library":
      return "Immich";
    case "google-maps":
      return "Google Maps";
    default:
      return adapterId;
  }
}

function extractionOutputSummary(
  adapterId: string,
  result: AdapterExtractionResult,
): string {
  const unchanged = result.skipReasons?.unchanged_rerun ?? 0;
  if (adapterId.startsWith("banking-") && result.sourceRecordsIncluded > 0) {
    if (result.eventsWritten === 0 && unchanged > 0) {
      return `${result.sourceRecordsIncluded.toLocaleString()} ${result.sourceRecordsIncluded === 1 ? "transaction was" : "transactions were"} processed; ${unchanged.toLocaleString()} unchanged banking Markdown ${unchanged === 1 ? "file remains" : "files remain"} saved`;
    }
    return `${result.eventsWritten.toLocaleString()} banking Markdown ${result.eventsWritten === 1 ? "file" : "files"} containing ${result.sourceRecordsIncluded.toLocaleString()} ${result.sourceRecordsIncluded === 1 ? "transaction" : "transactions"}, organized as daily movements with monthly overviews`;
  }
  if (adapterId === "airbnb-json-takeout") {
    const skipped = result.eventsSkipped.toLocaleString();
    return `${result.eventsWritten.toLocaleString()} open Markdown ${result.eventsWritten === 1 ? "event" : "events"}; ${skipped} ${result.eventsSkipped === 1 ? "record was" : "records were"} inspected but not added to Timeline, with exact reasons in AIRBNB-COVERAGE.md`;
  }
  if (
    ["apple-music-tsv", "apple-music-local-library"].includes(adapterId) &&
    result.sourceRecordsIncluded > 0
  ) {
    if (result.eventsWritten === 0 && unchanged > 0) {
      return `${result.sourceRecordsIncluded.toLocaleString()} tracks and all playlists remain saved in separate Markdown catalogs, with playlist membership preserved; no duplicates were created and ${unchanged.toLocaleString()} unchanged daily Date Added Timeline ${unchanged === 1 ? "file was" : "files were"} retained`;
    }
    return `${result.sourceRecordsIncluded.toLocaleString()} tracks and all playlists were saved in separate Markdown catalogs, with playlist membership preserved. ${result.eventsWritten.toLocaleString()} daily track Date Added ${result.eventsWritten === 1 ? "file was" : "files were"} added to Timeline${unchanged > 0 ? `; ${unchanged.toLocaleString()} unchanged ${unchanged === 1 ? "file was" : "files were"} retained` : ""}. Last Played remains catalog metadata only`;
  }
  if (result.eventsWritten === 0 && unchanged > 0) {
    return `${unchanged.toLocaleString()} unchanged open Markdown ${unchanged === 1 ? "event remains" : "events remain"} saved; no duplicate files were created`;
  }
  return `${result.eventsWritten.toLocaleString()} open Markdown ${result.eventsWritten === 1 ? "event" : "events"}`;
}

function isDesktopRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

function waitForMinimumProgress(startedAt: number): Promise<void> {
  const remaining = MIN_PROGRESS_VISIBLE_MS - (performance.now() - startedAt);
  if (remaining <= 0) return Promise.resolve();
  return new Promise((resolve) => window.setTimeout(resolve, remaining));
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export default App;
