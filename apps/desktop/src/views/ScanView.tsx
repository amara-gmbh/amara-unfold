import { isCatalogOnlyExtraction } from "../lib/extraction";
// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  AlertTriangle,
  ArrowRight,
  BedDouble,
  BookOpen,
  Check,
  ChevronDown,
  CircleDashed,
  Database,
  FileCode2,
  FileOutput,
  Folder,
  FolderOpen,
  Image,
  Landmark,
  LoaderCircle,
  Mail,
  MapPinned,
  Music2,
  Play,
  ShieldCheck,
  SkipForward,
  X,
} from "lucide-react";
import { useState, type ReactNode } from "react";
import { PageHeader } from "../components/PageHeader";
import { compactPath, formatCount, userFacingPath } from "../lib/format";
import type {
  AdapterExtractionProgress,
  AdapterExtractionResult,
  DatasetInspection,
  DatasetReadiness,
  DatasetStage,
  LocalAppSource,
  ScanIssue,
} from "../types";

interface ScanViewProps {
  selectedFolder: string | null;
  outputFolder: string | null;
  selectedAdapterId: string | null;
  extracting: boolean;
  activeAdapterId: string | null;
  extractionProgress: AdapterExtractionProgress | null;
  extractionResults: AdapterExtractionResult[];
  batchProgress: { current: number; total: number } | null;
  inspectingDataset: boolean;
  datasetInspection: DatasetInspection | null;
  recentIssues: ScanIssue[];
  localAppSources: LocalAppSource[];
  preparedMailExportFolder: string | null;
  mailExportAutoChecking: boolean;
  refreshingLocalApps: boolean;
  onChooseFile: () => void;
  onChooseFolder: () => void;
  onChooseLocalAppSource: (source: LocalAppSource) => void;
  onOpenAppleMail: () => void;
  onRefreshLocalAppSources: () => void;
  onChooseOutputFolder: () => void;
  onSelectAdapter: (adapterId: string) => void;
  onExtractDataset: (adapterId: string) => void;
  onExtractAllDatasets: (adapterIds: string[]) => void;
  onBackToSources: () => void;
  onOpenTimeline: () => void;
  onCancel: () => void;
}

export function ScanView({
  selectedFolder,
  outputFolder,
  selectedAdapterId,
  extracting,
  activeAdapterId,
  extractionProgress,
  extractionResults,
  batchProgress,
  inspectingDataset,
  datasetInspection,
  recentIssues,
  localAppSources,
  preparedMailExportFolder,
  mailExportAutoChecking,
  refreshingLocalApps,
  onChooseFile,
  onChooseFolder,
  onChooseLocalAppSource,
  onOpenAppleMail,
  onRefreshLocalAppSources,
  onChooseOutputFolder,
  onSelectAdapter,
  onExtractDataset,
  onExtractAllDatasets,
  onBackToSources,
  onOpenTimeline,
  onCancel,
}: ScanViewProps) {
  const [applicationGroup, setApplicationGroup] = useState<
    "available" | "future" | null
  >("available");
  const selectedDetection =
    datasetInspection?.detections.find(
      ({ adapterId }) => adapterId === selectedAdapterId,
    ) ?? datasetInspection?.detections[0];
  const alreadyOpen = selectedDetection?.readiness === "already_open";
  const extractionComplete = extractionProgress?.phase === "complete";
  const readyDetections =
    datasetInspection?.detections.filter(
      ({ readiness, stage }) =>
        readiness === "ready" && stage !== "open_markdown",
    ) ?? [];
  const batchCandidates = readyDetections.some(
    ({ adapterId }) => adapterId !== "photo-media-folder",
  )
    ? readyDetections.filter(
        ({ adapterId }) => adapterId !== "photo-media-folder",
      )
    : readyDetections;
  const batchAdapterIds = batchCandidates
    .filter((detection, index, detections) => {
      const key = detection.adapterId.startsWith("banking-")
        ? detection.adapterId
        : `${detection.provider}:${detection.moduleId}`;
      return (
        detections.findIndex(
          (candidate) =>
            (candidate.adapterId.startsWith("banking-")
              ? candidate.adapterId
              : `${candidate.provider}:${candidate.moduleId}`) === key,
        ) === index
      );
    })
    .map(({ adapterId }) => adapterId);
  const completedResults = extractionResults.filter(
    ({ cancelled, recordsSeen, sourceRecordsIncluded }) =>
      !cancelled && (recordsSeen > 0 || sourceRecordsIncluded > 0),
  );
  const completedEventCount = completedResults.reduce(
    (total, result) => total + result.eventsWritten,
    0,
  );
  const completedSourceRecordCount = completedResults.reduce(
    (total, result) => total + result.sourceRecordsIncluded,
    0,
  );
  const completedUnchangedRerunCount = completedResults.reduce(
    (total, result) => total + (result.skipReasons?.unchanged_rerun ?? 0),
    0,
  );
  const completedBankingOnly =
    completedResults.length > 0 &&
    completedResults.every(({ adapterId }) => adapterId.startsWith("banking-"));
  const completedAppleMusicOnly =
    completedResults.length > 0 &&
    completedResults.every(({ adapterId }) =>
      ["apple-music-tsv", "apple-music-local-library"].includes(adapterId),
    );
  const exportReady = completedResults.length > 0 && !extracting;
  const exportNeedsReview =
    exportReady &&
    (extractionProgress?.phase === "error" ||
      extractionProgress?.phase === "cancelled" ||
      (extractionProgress?.errors ?? 0) > 0 ||
      completedResults.some(
        (result) =>
          result.quality.status !== "pass" && !isCatalogOnlyExtraction(result),
      ));
  const noRecognizedSource =
    Boolean(selectedFolder) &&
    !inspectingDataset &&
    datasetInspection?.detections.length === 0;
  const exportableLocalApps = localAppSources.filter((source) =>
    Boolean(source.adapterId),
  );
  const futureLocalApps = localAppSources.filter((source) => !source.adapterId);
  return (
    <div className="view scan-view">
      <PageHeader
        eyebrow="Open data intake"
        title="Create open Markdown."
        description="Choose a source. Unfold recognizes it locally and shows one clear next step."
      />

      <div className="intake-steps" aria-label="Takeout to Markdown workflow">
        <WorkflowStep
          number="1"
          label="Choose source"
          detail={selectedFolder ? "Done" : "Now"}
          state={selectedFolder ? "complete" : "active"}
        />
        <WorkflowStep
          number="2"
          label="Confirm format"
          detail={
            inspectingDataset ? "Checking" : selectedDetection ? "Done" : "Next"
          }
          state={
            inspectingDataset
              ? "active"
              : selectedDetection
                ? "complete"
                : "pending"
          }
        />
        <WorkflowStep
          number="3"
          label="Choose destination"
          detail={
            alreadyOpen
              ? "Skipped"
              : outputFolder
                ? "Done"
                : selectedDetection
                  ? "Now"
                  : "Later"
          }
          state={
            alreadyOpen
              ? "skipped"
              : outputFolder
                ? "complete"
                : selectedDetection
                  ? "active"
                  : "pending"
          }
        />
        <WorkflowStep
          number="4"
          label="Create Markdown"
          detail={
            alreadyOpen || extractionComplete
              ? "Done"
              : extracting
                ? "Writing"
                : "Later"
          }
          state={
            alreadyOpen || extractionComplete
              ? "complete"
              : extracting
                ? "active"
                : "pending"
          }
        />
      </div>

      <div className="scan-layout simplified">
        <div className="raw-source-entry">
          <section
            className="next-action-card"
            aria-labelledby="next-action-title"
          >
            <div className="next-action-copy">
              <span className="next-action-icon" aria-hidden="true">
                {exportReady ? (
                  <Check size={22} />
                ) : extracting || inspectingDataset ? (
                  <LoaderCircle className="spin" size={22} />
                ) : selectedFolder ? (
                  <FileOutput size={22} />
                ) : (
                  <FolderOpen size={22} />
                )}
              </span>
              <div>
                <span className="eyebrow">
                  {exportNeedsReview
                    ? "Review needed"
                    : exportReady
                      ? "Complete"
                      : "Next step"}
                </span>
                <h2 id="next-action-title">
                  {exportReady
                    ? exportNeedsReview
                      ? "Some exports need attention"
                      : "Your Markdown is ready"
                    : extracting
                      ? "Creating your Markdown"
                      : inspectingDataset
                        ? "Recognizing this folder locally"
                        : !selectedFolder
                          ? "Choose raw data"
                          : noRecognizedSource
                            ? "Choose a different export"
                            : alreadyOpen
                              ? "This folder is already open Markdown"
                              : !outputFolder
                                ? "Choose where to save the Markdown"
                                : batchAdapterIds.length > 1
                                  ? `Create ${batchAdapterIds.length} Markdown exports`
                                  : "Create your Markdown files"}
                </h2>
                <p>
                  {exportReady
                    ? completedBankingOnly && completedSourceRecordCount > 0
                      ? completedEventCount === 0 &&
                        completedUnchangedRerunCount > 0
                        ? `${formatCount(completedSourceRecordCount)} ${completedSourceRecordCount === 1 ? "transaction was" : "transactions were"} processed; ${formatCount(completedUnchangedRerunCount)} unchanged banking Markdown ${completedUnchangedRerunCount === 1 ? "file remains" : "files remain"} saved. No duplicate files were created. Review the automatic quality check below.`
                        : `${formatCount(completedEventCount)} banking Markdown ${completedEventCount === 1 ? "file was" : "files were"} created from ${formatCount(completedSourceRecordCount)} ${completedSourceRecordCount === 1 ? "transaction" : "transactions"}${completedUnchangedRerunCount > 0 ? `; ${formatCount(completedUnchangedRerunCount)} unchanged ${completedUnchangedRerunCount === 1 ? "file was" : "files were"} retained` : ""}, organized as daily movements with monthly overviews. Review the automatic quality check below.`
                      : completedAppleMusicOnly &&
                          completedSourceRecordCount > 0
                        ? completedEventCount === 0 &&
                          completedUnchangedRerunCount > 0
                          ? `${formatCount(completedSourceRecordCount)} tracks and all playlists remain saved in separate Markdown catalogs, with playlist membership preserved. No duplicate files were created; ${formatCount(completedUnchangedRerunCount)} unchanged daily Date Added Timeline ${completedUnchangedRerunCount === 1 ? "file was" : "files were"} retained. Review the automatic quality check below.`
                          : `${formatCount(completedSourceRecordCount)} tracks and all playlists were saved in separate Markdown catalogs, with playlist membership preserved. ${formatCount(completedEventCount)} daily track Date Added ${completedEventCount === 1 ? "file was" : "files were"} added to Timeline${completedUnchangedRerunCount > 0 ? ` and ${formatCount(completedUnchangedRerunCount)} unchanged ${completedUnchangedRerunCount === 1 ? "file was" : "files were"} retained` : ""}. Last Played remains catalog metadata only. Review the automatic quality check below.`
                        : completedEventCount === 0 &&
                            completedUnchangedRerunCount > 0
                          ? `${formatCount(completedUnchangedRerunCount)} unchanged ${completedUnchangedRerunCount === 1 ? "event remains" : "events remain"} saved across ${completedResults.length} ${completedResults.length === 1 ? "export" : "exports"}. No duplicate files were created. Review the automatic quality check below.`
                          : `${formatCount(completedEventCount)} ${completedEventCount === 1 ? "event was" : "events were"} created across ${completedResults.length} ${completedResults.length === 1 ? "export" : "exports"}${completedUnchangedRerunCount > 0 ? `; ${formatCount(completedUnchangedRerunCount)} unchanged ${completedUnchangedRerunCount === 1 ? "event was" : "events were"} retained` : ""}. Review the automatic quality check below.`
                    : extracting
                      ? "Unfold is writing portable files locally. Your originals remain unchanged."
                      : inspectingDataset
                        ? "This normally takes only a moment. Nothing is uploaded or changed."
                        : !selectedFolder
                          ? "Choose one raw export file or a folder containing an export."
                          : noRecognizedSource
                            ? "Unfold did not find a supported format in this folder."
                            : alreadyOpen
                              ? "Nothing needs to be converted. The files are already portable and user-owned."
                              : !outputFolder
                                ? `${selectedDetection?.label ?? "Source"} — ${selectedDetection?.format ?? "supported format"} recognized locally. Now select an empty folder for the new files.`
                                : batchAdapterIds.length > 1
                                  ? "Everything is ready. Create every recognized export together, or use Confirm format below to choose one."
                                  : "Everything is ready. This creates portable Markdown without changing the source."}
                </p>
              </div>
            </div>
            <div className="next-action-controls">
              {exportReady ? (
                <div className="markdown-actions">
                  <button
                    className="secondary-button"
                    onClick={() =>
                      document
                        .getElementById("export-quality")
                        ?.scrollIntoView({ behavior: "smooth", block: "start" })
                    }
                    type="button"
                  >
                    Review data quality <ArrowRight size={16} />
                  </button>
                  <button
                    className="primary-button"
                    onClick={onOpenTimeline}
                    type="button"
                  >
                    Open all events <ArrowRight size={16} />
                  </button>
                </div>
              ) : extracting ? (
                <button
                  className="secondary-button danger-hover"
                  onClick={onCancel}
                  type="button"
                >
                  <X size={16} /> Stop extraction
                </button>
              ) : !selectedFolder ? (
                <div className="source-entry-actions">
                  <button
                    className="primary-button"
                    onClick={onChooseFile}
                    disabled={inspectingDataset}
                    type="button"
                  >
                    <FileCode2 size={16} /> Choose raw file
                  </button>
                  <button
                    className="secondary-button"
                    onClick={onChooseFolder}
                    disabled={inspectingDataset}
                    type="button"
                  >
                    <FolderOpen size={16} /> Choose raw folder
                  </button>
                </div>
              ) : noRecognizedSource ? (
                <button
                  className="primary-button"
                  onClick={onChooseFolder}
                  disabled={inspectingDataset}
                  type="button"
                >
                  <FolderOpen size={16} /> Choose a different source
                </button>
              ) : inspectingDataset ? null : alreadyOpen ? (
                <span className="route-ready">
                  <Check size={15} /> No conversion needed
                </span>
              ) : !outputFolder ? (
                <button
                  className="primary-button"
                  disabled={!selectedDetection}
                  onClick={onChooseOutputFolder}
                  type="button"
                >
                  <Folder size={16} /> Choose output folder
                </button>
              ) : (
                <div className="markdown-actions">
                  {batchAdapterIds.length > 1 && (
                    <button
                      className="primary-button create-markdown-button"
                      onClick={() => onExtractAllDatasets(batchAdapterIds)}
                      type="button"
                    >
                      <FileOutput size={16} /> Create all{" "}
                      {batchAdapterIds.length} exports
                    </button>
                  )}
                  <button
                    className={
                      batchAdapterIds.length > 1
                        ? "secondary-button create-markdown-button"
                        : "primary-button create-markdown-button"
                    }
                    disabled={!selectedAdapterId}
                    onClick={() =>
                      selectedAdapterId && onExtractDataset(selectedAdapterId)
                    }
                    type="button"
                  >
                    <FileOutput size={16} />
                    {batchAdapterIds.length > 1
                      ? "Create selected export"
                      : "Create Markdown files"}
                  </button>
                </div>
              )}
            </div>
            {!selectedFolder && (
              <div className="raw-support-note">
                <p>
                  Supported now: Diary DOCX · Banking Takeout · Airbnb Takeout ·
                  Google Maps
                </p>
                <p>
                  Accepted files: DOCX, CSV, spreadsheet, GeoJSON and KMZ. For
                  Airbnb, choose the extracted Takeout folder.
                </p>
              </div>
            )}
          </section>
        </div>

        {(inspectingDataset || extracting) && (
          <DatasetPipeline
            inspection={datasetInspection}
            inspecting={inspectingDataset}
            extractionProgress={extractionProgress}
            extracting={extracting}
            activeAdapterId={activeAdapterId}
          />
        )}

        {completedResults.length > 0 && (
          <section
            className="quality-results"
            id="export-quality"
            aria-label="Export quality"
          >
            <div className="quality-results-heading">
              <div>
                <span className="eyebrow">
                  Automatic post-extract validation
                </span>
                <h3>Data quality by source</h3>
              </div>
              <span>{completedResults.length} checked</span>
            </div>
            <QualityResultList results={completedResults} />
          </section>
        )}

        {!selectedFolder && (
          <details
            className="panel scan-card local-app-intake"
            id="application-source-routes"
            onToggle={(event) => {
              if (event.currentTarget.open) setApplicationGroup("available");
            }}
          >
            <summary>
              <span className="next-action-copy application-route-copy">
                <span className="next-action-icon" aria-hidden="true">
                  <Database size={22} />
                </span>
                <span>
                  <span className="eyebrow">Application source</span>
                  <h2>Choose from an application</h2>
                  <p>Only supported routes can be selected</p>
                </span>
              </span>
              <span className="summary-action secondary-button application-summary-action">
                <span className="when-closed">Show applications</span>
                <span className="when-open">Hide applications</span>
                <ChevronDown size={16} />
              </span>
            </summary>
            <div className="application-route-intro">
              <p>Choose one of the two application paths.</p>
            </div>
            <div
              aria-label="Application availability"
              className="application-group-choices"
              role="group"
            >
              <button
                aria-pressed={applicationGroup === "available"}
                className={`application-group-choice${applicationGroup === "available" ? " active" : ""}`}
                onClick={() =>
                  setApplicationGroup((current) =>
                    current === "available" ? null : "available",
                  )
                }
                type="button"
              >
                <Check size={18} />
                <span>
                  <strong>Available now</strong>
                  <small>
                    {exportableLocalApps.length} supported applications
                  </small>
                </span>
                <ChevronDown size={16} />
              </button>
              <button
                aria-pressed={applicationGroup === "future"}
                className={`application-group-choice${applicationGroup === "future" ? " active" : ""}`}
                onClick={() =>
                  setApplicationGroup((current) =>
                    current === "future" ? null : "future",
                  )
                }
                type="button"
              >
                <CircleDashed size={18} />
                <span>
                  <strong>Not available yet</strong>
                  <small>{futureLocalApps.length} planned applications</small>
                </span>
                <ChevronDown size={16} />
              </button>
            </div>
            {applicationGroup === "available" && (
              <div className="application-group-panel">
                <div className="application-group-panel-heading">
                  <span>
                    <strong>Available now</strong>
                    <small>Choose an application or its exported data.</small>
                  </span>
                  <button
                    className="secondary-button refresh-local-apps"
                    disabled={
                      refreshingLocalApps || extracting || inspectingDataset
                    }
                    onClick={onRefreshLocalAppSources}
                    type="button"
                  >
                    {refreshingLocalApps
                      ? "Refreshing…"
                      : "Refresh app exports"}
                  </button>
                </div>
                <div
                  className="local-app-list"
                  aria-label="Supported applications"
                >
                  {exportableLocalApps.map((source) => (
                    <LocalAppCard
                      key={source.id}
                      source={source}
                      disabled={extracting || inspectingDataset}
                      onChoose={() => onChooseLocalAppSource(source)}
                      onOpenMail={
                        source.id === "apple-mail" ? onOpenAppleMail : undefined
                      }
                      preparedMailExportFolder={preparedMailExportFolder}
                      mailExportAutoChecking={mailExportAutoChecking}
                    />
                  ))}
                </div>
              </div>
            )}
            {applicationGroup === "future" && futureLocalApps.length > 0 && (
              <div className="application-group-panel future-apps">
                <div className="application-group-panel-heading">
                  <span>
                    <strong>Not available yet</strong>
                    <small>
                      These routes are planned, but cannot be selected.
                    </small>
                  </span>
                </div>
                <div className="future-app-list">
                  {futureLocalApps.map((source) => (
                    <span key={source.id}>{source.label}</span>
                  ))}
                </div>
              </div>
            )}
          </details>
        )}

        {selectedFolder && (
          <section
            aria-labelledby="recognized-format-heading"
            className="panel scan-card scan-setup"
          >
            <div className="scan-stage-heading">
              <span className="summary-main">
                <FolderOpen size={18} />
                <span>
                  <strong id="recognized-format-heading">Confirm format</strong>
                  <small>
                    {datasetInspection
                      ? `${datasetInspection.detections.length} recognized`
                      : "Checking this source"}
                  </small>
                </span>
              </span>
              <button
                className="secondary-button"
                disabled={extracting || inspectingDataset}
                onClick={onBackToSources}
                type="button"
              >
                Change source
              </button>
            </div>
            <div className="scan-setup-body">
              <div className="selected-source-path" title={selectedFolder}>
                <FolderOpen size={17} />
                <span>
                  <small>Selected source</small>
                  <strong>{compactPath(selectedFolder, 4)}</strong>
                </span>
              </div>

              {datasetInspection && datasetInspection.detections.length > 0 && (
                <div className="recognition-announcement" role="status">
                  <span className="recognition-check">
                    <Check size={17} />
                  </span>
                  <div>
                    <span className="eyebrow">Recognized format</span>
                    <h2>{selectedDetection?.label}</h2>
                    <strong>{selectedDetection?.format}</strong>
                    <p>
                      {selectedDetection?.adapterId ===
                        "apple-music-local-library" &&
                      datasetInspection.fileCount === 0
                        ? "The standard library is selected. Track metadata remains unread until you choose Create Markdown."
                        : `${formatCount(datasetInspection.fileCount)} ${datasetInspection.fileCount === 1 ? "file" : "files"} inspected locally. The export will contain readable Markdown event files.`}
                    </p>
                  </div>
                </div>
              )}

              {datasetInspection && datasetInspection.detections.length > 1 && (
                <div className="recognized-list-heading">
                  <strong>More than one format was found</strong>
                  <span>
                    Select the format to create, or create all supported
                    exports.
                  </span>
                </div>
              )}

              {datasetInspection && datasetInspection.detections.length > 1 && (
                <div className="dataset-detection-list">
                  {datasetInspection?.detections.map((detection) => (
                    <section
                      className={`dataset-detection compact${detection.adapterId === selectedAdapterId ? " selected" : ""}`}
                      aria-label={detection.label}
                      key={detection.adapterId}
                    >
                      <div className="dataset-detection-main">
                        <span className="source-glyph" aria-hidden="true">
                          <SourceGlyph adapterId={detection.adapterId} />
                        </span>
                        <div className="dataset-detection-copy">
                          <h3>{detection.label}</h3>
                          <span>
                            {detection.format} ·{" "}
                            {formatCount(datasetInspection.fileCount)} files ·{" "}
                            {readinessLabel(detection.readiness)}
                          </span>
                        </div>
                        {detection.readiness === "already_open" ? (
                          <span className="route-ready">
                            <Check size={15} /> Open Markdown
                          </span>
                        ) : (
                          <button
                            aria-pressed={
                              detection.adapterId === selectedAdapterId
                            }
                            className="secondary-button detection-action"
                            disabled={
                              detection.readiness !== "ready" || extracting
                            }
                            onClick={() => onSelectAdapter(detection.adapterId)}
                            type="button"
                          >
                            {detection.adapterId === selectedAdapterId ? (
                              <>
                                <Check size={15} /> Selected
                              </>
                            ) : (
                              <>Select</>
                            )}
                          </button>
                        )}
                      </div>
                      <details className="dataset-details">
                        <summary>What Unfold found</summary>
                        <p>{detection.nextStep}</p>
                        <dl className="dataset-contract">
                          <div>
                            <dt>Input state</dt>
                            <dd>{stageLabel(detection.stage)}</dd>
                          </div>
                          <div>
                            <dt>Offline adapter</dt>
                            <dd>{detection.moduleId}</dd>
                          </div>
                          <div>
                            <dt>Files inspected</dt>
                            <dd>{formatCount(datasetInspection.fileCount)}</dd>
                          </div>
                          <div>
                            <dt>Open output</dt>
                            <dd>{datasetInspection.outputContract}</dd>
                          </div>
                        </dl>
                        <div className="recognition-evidence">
                          {detection.evidence.map((evidence) => (
                            <span key={evidence}>
                              <Check size={12} /> {evidence}
                            </span>
                          ))}
                        </div>
                        {detection.commandPreview && (
                          <code>{detection.commandPreview}</code>
                        )}
                      </details>
                    </section>
                  ))}
                </div>
              )}

              {selectedDetection &&
                datasetInspection?.detections.length === 1 && (
                  <details className="dataset-details single-detection-details">
                    <summary>Recognition details</summary>
                    <p>{selectedDetection.nextStep}</p>
                    <div className="recognition-evidence">
                      {selectedDetection.evidence.map((evidence) => (
                        <span key={evidence}>
                          <Check size={12} /> {evidence}
                        </span>
                      ))}
                    </div>
                  </details>
                )}

              {selectedDetection && !alreadyOpen && (
                <div className="output-summary">
                  <div>
                    <span className="eyebrow">Output folder</span>
                    <strong>
                      {outputFolder
                        ? compactPath(outputFolder, 3)
                        : "Not selected yet"}
                    </strong>
                  </div>
                  <button
                    className="secondary-button output-folder-button"
                    disabled={extracting}
                    onClick={onChooseOutputFolder}
                    type="button"
                  >
                    <Folder size={16} /> {outputFolder ? "Change" : "Choose"}
                  </button>
                </div>
              )}
            </div>
          </section>
        )}

        {extractionProgress &&
          (extractionProgress.phase === "error" ||
            extractionProgress.phase === "cancelled") && (
            <div
              className={`progress-card ${extractionProgress.phase}`}
              role="status"
            >
              <div className="progress-icon">
                {extracting ? (
                  <LoaderCircle className="spin" size={19} />
                ) : extractionProgress.phase === "error" ? (
                  <AlertTriangle size={19} />
                ) : (
                  <Check size={19} />
                )}
              </div>
              <div className="progress-copy">
                {batchProgress && (
                  <span className="batch-progress-label">
                    Export {batchProgress.current} of {batchProgress.total}
                  </span>
                )}
                <strong>{extractionProgress.message}</strong>
                {extractionProgress.currentItem && (
                  <span title={extractionProgress.currentItem}>
                    {compactPath(extractionProgress.currentItem, 4)}
                  </span>
                )}
                <div className="progress-stats">
                  <span>
                    {formatCount(extractionProgress.recordsSeen)} records
                  </span>
                  {extractionProgress.adapterId !== "timeline-index" && (
                    <span>
                      {formatCount(extractionProgress.eventsWritten)} Markdown
                      files
                    </span>
                  )}
                  {extractionProgress.eventsSkipped > 0 && (
                    <span>
                      {formatCount(extractionProgress.eventsSkipped)} not
                      written — see reasons
                    </span>
                  )}
                  {extractionProgress.errors > 0 && (
                    <span>{formatCount(extractionProgress.errors)} errors</span>
                  )}
                </div>
              </div>
            </div>
          )}

        <details className="trust-details">
          <summary>
            <ShieldCheck size={17} /> Local, read-only, and open
          </summary>
          <div className="trust-details-grid">
            <p>
              Recognition is local and read-only. Selecting a folder never
              changes originals or makes a network request.
            </p>
            <ul>
              <li>Readable detection rules</li>
              <li>Markdown with YAML frontmatter</li>
              <li>Amara Memory remains optional</li>
            </ul>
          </div>
        </details>

        {recentIssues.length > 0 && (
          <section className="scan-issues">
            <h3>
              <AlertTriangle size={14} /> Recent scan issues
            </h3>
            <p>Kept locally so you can see what was skipped.</p>
            <div>
              {recentIssues.slice(0, 5).map((issue, index) => (
                <span key={`${issue.scanId}-${index}`} title={issue.message}>
                  {compactPath(issue.message, 4)}
                </span>
              ))}
            </div>
          </section>
        )}
      </div>
    </div>
  );
}

function LocalAppCard({
  source,
  disabled,
  onChoose,
  onOpenMail,
  preparedMailExportFolder,
  mailExportAutoChecking,
}: {
  source: LocalAppSource;
  disabled: boolean;
  onChoose: () => void;
  onOpenMail?: () => void;
  preparedMailExportFolder?: string | null;
  mailExportAutoChecking?: boolean;
}) {
  return (
    <section className={`local-app-card ${source.status}`} key={source.id}>
      <div className="local-app-card-main">
        <span className="source-glyph" aria-hidden="true">
          <LocalAppGlyph sourceId={source.id} />
        </span>
        <div>
          <h3>{source.label}</h3>
          <p>{source.nextStep}</p>
        </div>
      </div>
      <div className="local-app-meta">
        <span>{localAppStatusLabel(source)}</span>
        {source.recordCount > 0 && (
          <span>{formatCount(source.recordCount)} records</span>
        )}
      </div>
      <details className="local-app-details">
        <summary>How access works</summary>
        <p>{source.detail}</p>
        {source.id === "apple-mail" && (
          <div className="mail-export-flow" aria-label="Apple Mail export flow">
            <span>1. Export in Mail</span>
            <span>2. Save .mbox</span>
            <span>3. Choose Mail export</span>
          </div>
        )}
        {source.id === "apple-mail" &&
          preparedMailExportFolder &&
          !source.path && (
            <div className="mail-export-prepared" role="status">
              <strong>Prepared folder</strong>
              <span>{userFacingPath(preparedMailExportFolder)}</span>
              <em>Waiting for a Mail export ending in .mbox.</em>
              <em>
                No Full Disk Access needed — Unfold reads only the exported
                folder you choose.
              </em>
              {mailExportAutoChecking && (
                <em>Auto-checking this folder now.</em>
              )}
            </div>
          )}
        {localAppPreparationSteps(source).length > 0 && (
          <ol className="local-app-steps">
            {localAppPreparationSteps(source).map((step) => (
              <li key={step}>{step}</li>
            ))}
          </ol>
        )}
        <p>{source.privacyNote}</p>
        {source.path && <p>Default location: {userFacingPath(source.path)}</p>}
        {source.id === "apple-mail" && onOpenMail && (
          <button
            className="ghost-button mail-prepare-action"
            disabled={disabled}
            onClick={onOpenMail}
            type="button"
          >
            {preparedMailExportFolder
              ? "Open Mail + folder again"
              : "Prepare Mail export"}
          </button>
        )}
      </details>
      <div className="local-app-actions">
        <button
          className={
            source.status === "ready" || source.adapterId
              ? "secondary-button detection-action"
              : "ghost-button detection-action"
          }
          disabled={!source.adapterId || disabled}
          onClick={onChoose}
          type="button"
        >
          {localAppActionLabel(source)}
        </button>
      </div>
    </section>
  );
}

function SourceGlyph({ adapterId }: { adapterId: string }) {
  if (adapterId.startsWith("banking-")) return <Landmark size={19} />;
  if (adapterId === "airbnb-json-takeout") return <BedDouble size={19} />;
  if (
    adapterId === "gmail-takeout" ||
    adapterId === "pst-mail" ||
    adapterId === "apple-mail-local"
  )
    return <Mail size={19} />;
  if (
    adapterId === "apple-music-tsv" ||
    adapterId === "apple-music-local-library"
  )
    return <Music2 size={19} />;
  if (
    adapterId === "photo-media-folder" ||
    adapterId === "google-photos-takeout" ||
    adapterId === "immich-local-library"
  )
    return <Image size={19} />;
  if (
    adapterId === "google-maps" ||
    adapterId.startsWith("google-maps-") ||
    adapterId === "google-drive-kmz"
  )
    return <MapPinned size={15} />;
  if (adapterId === "youtube-takeout") return <Play size={19} />;
  if (adapterId === "diary-docx") return <BookOpen size={19} />;
  if (adapterId === "timeline-index") return <Database size={19} />;
  if (adapterId === "apple-calendar-local") return <Database size={19} />;
  if (adapterId === "apple-contacts-local") return <BookOpen size={19} />;
  return <FileCode2 size={19} />;
}

function LocalAppGlyph({ sourceId }: { sourceId: string }) {
  if (sourceId.includes("mail") || sourceId.includes("messages"))
    return <Mail size={19} />;
  if (sourceId.includes("music")) return <Music2 size={19} />;
  if (sourceId.includes("photos") || sourceId.includes("immich"))
    return <Image size={19} />;
  if (sourceId.includes("contacts")) return <BookOpen size={19} />;
  if (sourceId.includes("calendar") || sourceId.includes("journal"))
    return <Database size={19} />;
  return <FileCode2 size={19} />;
}

function localAppStatusLabel(source: LocalAppSource): string {
  if (source.id === "apple-mail" && source.path) return "Export found";
  if (source.id === "apple-mail" && source.adapterId)
    return "Export recommended";
  if (source.id === "apple-photos" && source.adapterId) return "Choose export";
  if (source.id === "apple-music" && source.adapterId) {
    return source.status === "ready" ? "Ready" : "Ask on export";
  }
  if (
    (source.id === "apple-calendar" || source.id === "apple-contacts") &&
    source.adapterId
  ) {
    return source.status === "ready" ? "Ready" : "Choose export";
  }
  if (source.status === "ready") return "Ready";
  if (source.status === "needs_permission") return "Ask on export";
  if (source.adapterId) return "Choose data";
  if (source.status === "not_found") return "Not found";
  return "Not in this build";
}

function localAppActionLabel(source: LocalAppSource): string {
  if (!source.adapterId) return "Not in this build";
  if (source.id === "apple-photos") return "Choose Photos export";
  if (source.id === "apple-mail" && source.path) return "Use found Mail export";
  if (source.id === "apple-mail") return "Choose Mail export";
  if (source.id === "apple-music" && source.path) return "Use Music library";
  if (source.id === "immich" && source.path) return "Use Immich folder";
  if (source.id === "apple-calendar") return "Choose Calendar export";
  if (source.id === "apple-contacts") return "Choose Contacts export";
  return "Choose data folder";
}

function localAppPreparationSteps(source: LocalAppSource): string[] {
  if (source.id === "apple-mail") {
    return [
      "In Apple Mail, select the mailbox you want to unfold.",
      "Click Prepare Mail export to open Mail and a Desktop folder named Amara Unfold Mail Exports.",
      "Use Mailbox › Export Mailbox… and save the exported .mbox folder there.",
      source.path
        ? "Return here and click Use found Mail export, or use Choose Mail export if you want a different .mbox export."
        : "Return here; Unfold can auto-detect the saved export, or you can click Choose Mail export and choose it manually.",
    ];
  }
  if (source.id === "immich") {
    return [
      "Choose an Immich folder that contains original photos or videos.",
      "Usually that is an Immich upload/ or library/ originals folder, not thumbs/ or encoded-video/.",
      "Unfold reads files locally and does not call the Immich server or network API.",
    ];
  }
  if (source.id === "apple-photos") {
    return [
      "In Photos.app, select the photos/videos you want, then use File › Export › Export Unmodified Originals… when you want the cleanest source files.",
      "Save the export into a normal folder, then choose that folder here.",
      "Unfold reads only that chosen folder. It does not open Photos.app's private live library or keep Photos permission.",
    ];
  }
  if (source.id === "apple-music") {
    return [
      "Use the standard Music.app library when available, or choose a folder of local audio files.",
      "macOS may ask for Music/Automation access only when you create the export.",
      "Unfold preserves tracks, playlists, playlist membership, and each track's Date Added in Markdown. Timeline uses only explicit Date Added values; Last Played and loose-file timestamps never become Timeline dates.",
      "After the export, Unfold asks macOS to forget the Music Automation approval where supported; if macOS keeps it, revoke it in System Settings.",
    ];
  }
  if (source.id === "apple-calendar") {
    return [
      "In Calendar.app, export the calendar as an .ics file.",
      "Save one or more .ics files into a normal folder, then choose that folder here.",
      "Unfold reads only those exported files. It does not inspect Calendar.app's private live database or keep Calendar permission.",
    ];
  }
  if (source.id === "apple-contacts") {
    return [
      "In Contacts.app, select the contacts you want and export them as vCards (.vcf).",
      "Save the .vcf files into a normal folder, then choose that folder here.",
      "Unfold reads only those exported vCards. It does not inspect Contacts.app's private live database or keep Contacts permission.",
    ];
  }
  return [];
}

function QualityResultList({
  results,
}: {
  results: AdapterExtractionResult[];
}) {
  return (
    <div className="quality-result-list">
      {results.map((result) => {
        const quality = result.quality;
        const catalogOnly = isCatalogOnlyExtraction(result);
        const score = (quality.scoreBasisPoints / 100).toFixed(2);
        const skipReasons = Object.entries(result.skipReasons ?? {}).sort(
          ([, left], [, right]) => right - left,
        );
        return (
          <details className="quality-result" key={result.adapterId}>
            <summary>
              <span className="source-glyph" aria-hidden="true">
                <SourceGlyph adapterId={result.adapterId} />
              </span>
              <span className="quality-result-name">
                <strong>{adapterLabel(result.adapterId)}</strong>
                <small>
                  {catalogOnly
                    ? "No Timeline events to validate"
                    : `${score}% clean`}
                </small>
              </span>
              <span className={`quality-chip ${quality.status}`}>
                {catalogOnly ? (
                  <Music2 size={13} />
                ) : quality.status === "pass" ? (
                  <Check size={13} />
                ) : (
                  <AlertTriangle size={13} />
                )}
                {catalogOnly
                  ? "Not applicable"
                  : quality.status === "pass"
                    ? "Validated"
                    : "Review"}
              </span>
            </summary>
            <dl className="quality-grid">
              <QualityMetric label="Records seen" value={result.recordsSeen} />
              {result.sourceRecordsIncluded > 0 && (
                <QualityMetric
                  label="Source records included"
                  value={result.sourceRecordsIncluded}
                />
              )}
              <QualityMetric
                label="Markdown written"
                value={result.eventsWritten}
              />
              <QualityMetric
                label="Skipped with reason"
                value={result.eventsSkipped}
              />
              <QualityMetric label="Extraction errors" value={result.errors} />
              <QualityMetric
                label="Flagged events"
                value={quality.flaggedEvents}
              />
              <QualityMetric
                label="Encoding warnings"
                value={
                  quality.encodingWarnings + quality.sourceNormalizationWarnings
                }
              />
              <QualityMetric
                label="Source records without a body"
                value={quality.sourceEmptyBodyRecords}
              />
              <QualityMetric
                label="Mail body warnings"
                value={quality.mailBodyWarnings}
              />
              <QualityMetric
                label="Direction warnings"
                value={quality.mailDirectionWarnings}
              />
              <QualityMetric
                label="Date warnings"
                value={quality.implausibleDateWarnings}
              />
              <QualityMetric
                label="Counterparty warnings"
                value={quality.counterpartyWarnings}
              />
              <QualityMetric
                label="Schema/provenance"
                value={quality.schemaErrors + quality.provenanceErrors}
              />
            </dl>
            {skipReasons.length > 0 && (
              <div className="quality-skip-reasons">
                <strong>Why records were not written</strong>
                <ul>
                  {skipReasons.map(([reason, count]) => (
                    <li key={reason}>
                      <span>{humanizeSkipReason(reason)}</span>
                      <strong>{formatCount(count)}</strong>
                    </li>
                  ))}
                </ul>
              </div>
            )}
            <p>
              Accounting{" "}
              {quality.accountingBalanced ? "balances" : "does not balance"};
              skip reasons{" "}
              {quality.skipReasonsBalanced ? "balance" : "need review"}. Full
              reports are saved beside the Markdown.
            </p>
          </details>
        );
      })}
    </div>
  );
}

function QualityMetric({ label, value }: { label: string; value: number }) {
  return (
    <div>
      <dt>{label}</dt>
      <dd>{formatCount(value)}</dd>
    </div>
  );
}

function humanizeSkipReason(reason: string): string {
  return reason
    .replace(/^not_timeline_material_/, "Not Timeline material: ")
    .replace(/_/g, " ")
    .replace(/\b\w/g, (character: string) => character.toUpperCase());
}

function adapterLabel(adapterId: string): string {
  return adapterId
    .split("-")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function DatasetPipeline({
  inspection,
  inspecting,
  extractionProgress,
  extracting,
  activeAdapterId,
}: {
  inspection: DatasetInspection | null;
  inspecting: boolean;
  extractionProgress: AdapterExtractionProgress | null;
  extracting: boolean;
  activeAdapterId: string | null;
}) {
  const detection =
    inspection?.detections.find((item) => item.adapterId === activeAdapterId) ??
    inspection?.detections[0];
  const alreadyOpen = detection?.readiness === "already_open";
  const fileCount = inspection?.fileCount ?? 0;
  const inspectionDeferred =
    detection?.adapterId === "apple-music-local-library" && fileCount === 0;
  const total = extractionProgress?.totalHint ?? null;
  const extractionRatio = total
    ? Math.min(extractionProgress!.recordsSeen / total, 1)
    : 0;
  const extractionComplete = extractionProgress?.phase === "complete";
  const pipelinePercent = alreadyOpen
    ? 100
    : extractionComplete
      ? 100
      : extracting
        ? 25 + extractionRatio * 75
        : 25;
  const extractionPercent = Math.round(extractionRatio * 100);
  const extractionState = alreadyOpen
    ? "skipped"
    : extracting
      ? "active"
      : extractionComplete
        ? "complete"
        : "pending";
  const extractionIcon = alreadyOpen ? (
    <SkipForward size={14} />
  ) : extracting ? (
    <CircleDashed size={14} />
  ) : extractionComplete ? (
    <Check size={14} />
  ) : (
    <CircleDashed size={14} />
  );
  return (
    <section className="dataset-pipeline" aria-label="Extraction progress">
      <div className="pipeline-heading">
        <div>
          <span className="eyebrow">Open extraction pipeline</span>
          <strong>
            {inspecting
              ? "Recognizing dataset"
              : extractionComplete
                ? extractionProgress?.adapterId === "timeline-index"
                  ? "Timeline and Markdown ready"
                  : "Markdown ready"
                : extractionProgress?.phase === "indexing"
                  ? "Building readable timeline"
                  : extracting
                    ? total
                      ? `Extracting metadata · ${extractionPercent}%`
                      : `Extracting metadata · ${formatCount(extractionProgress?.recordsSeen ?? 0)} records`
                    : alreadyOpen
                      ? "Open Markdown recognized"
                      : extractionComplete
                        ? "Metadata extracted"
                        : detection
                          ? "Ready for review"
                          : "No known adapter found"}
          </strong>
        </div>
        {extractionProgress && extracting ? (
          <span>
            {formatCount(extractionProgress.recordsSeen)}
            {total ? ` of ${formatCount(total)}` : ""} records
          </span>
        ) : inspection ? (
          <span>
            {inspectionDeferred
              ? "Library read deferred until export"
              : `${formatCount(fileCount)} files found`}
          </span>
        ) : null}
      </div>
      <div
        className={`pipeline-track${inspecting || (extracting && !total) ? " active" : ""}`}
        role="progressbar"
        aria-label="Open extraction progress"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={
          inspecting || (extracting && !total)
            ? undefined
            : Math.round(pipelinePercent)
        }
      >
        <span
          style={
            inspecting || (extracting && !total)
              ? undefined
              : { width: `${pipelinePercent}%` }
          }
        />
      </div>
      <div className="pipeline-stages">
        <PipelineStage
          icon={inspecting ? <CircleDashed size={14} /> : <Check size={14} />}
          label="Recognize"
          state={inspecting ? "active" : "complete"}
        />
        <ArrowRight size={13} />
        <PipelineStage
          icon={extractionIcon}
          label="Extract"
          state={extractionState}
        />
        <ArrowRight size={13} />
        <PipelineStage
          icon={
            alreadyOpen || extractionComplete ? (
              <Check size={14} />
            ) : (
              <CircleDashed size={14} />
            )
          }
          label="Markdown"
          state={alreadyOpen || extractionComplete ? "complete" : "pending"}
        />
        <ArrowRight size={13} />
        <PipelineStage
          icon={
            alreadyOpen || extractionComplete ? (
              <Check size={14} />
            ) : (
              <CircleDashed size={14} />
            )
          }
          label="Structure"
          state={alreadyOpen || extractionComplete ? "complete" : "pending"}
        />
      </div>
    </section>
  );
}

function WorkflowStep({
  number,
  label,
  detail,
  state,
}: {
  number: string;
  label: string;
  detail: string;
  state: "active" | "complete" | "pending" | "skipped";
}) {
  return (
    <div className={`intake-step ${state}`}>
      <span className="workflow-step-marker">
        {state === "complete" ? (
          <Check size={14} />
        ) : state === "skipped" ? (
          <SkipForward size={14} />
        ) : (
          number
        )}
      </span>
      <span className="workflow-step-copy">
        <strong>{label}</strong>
        <small>{detail}</small>
      </span>
    </div>
  );
}

function PipelineStage({
  icon,
  label,
  state,
}: {
  icon: ReactNode;
  label: string;
  state: "active" | "complete" | "skipped" | "pending";
}) {
  return (
    <span className={`pipeline-stage ${state}`}>
      {icon} {label}
    </span>
  );
}

function readinessLabel(readiness: DatasetReadiness): string {
  if (readiness === "already_open") return "Already open";
  if (readiness === "ready") return "Adapter ready";
  return "Preparation needed";
}

function stageLabel(stage: DatasetStage): string {
  if (stage === "open_markdown") return "Existing open Markdown";
  if (stage === "mixed") return "Raw and derived files";
  return "Raw provider export";
}
