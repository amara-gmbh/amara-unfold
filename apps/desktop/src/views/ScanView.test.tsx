// @vitest-environment jsdom
// SPDX-License-Identifier: AGPL-3.0-or-later
import { render, screen, fireEvent } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { cleanup } from "@testing-library/react";
import { ScanView } from "./ScanView";
import type { AdapterExtractionResult, DatasetDetection } from "../types";
afterEach(cleanup);
const noop = () => {};
const base: Parameters<typeof ScanView>[0] = {
  selectedFolder: "/synthetic",
  outputFolder: "/output",
  selectedAdapterId: "banking-csv",
  extracting: false,
  activeAdapterId: null,
  extractionProgress: null,
  extractionResults: [],
  batchProgress: null,
  inspectingDataset: false,
  datasetInspection: null,
  recentIssues: [],
  localAppSources: [],
  preparedMailExportFolder: null,
  mailExportAutoChecking: false,
  refreshingLocalApps: false,
  onChooseFile: noop,
  onChooseFolder: noop,
  onChooseLocalAppSource: noop,
  onOpenAppleMail: noop,
  onRefreshLocalAppSources: noop,
  onChooseOutputFolder: noop,
  onSelectAdapter: noop,
  onExtractDataset: noop,
  onExtractAllDatasets: noop,
  onBackToSources: noop,
  onOpenTimeline: noop,
  onCancel: noop,
};
const result: AdapterExtractionResult = {
  adapterId: "banking-csv",
  folder: "/output",
  recordsSeen: 2,
  sourceRecordsIncluded: 1,
  eventsWritten: 2,
  eventsSkipped: 0,
  errors: 0,
  cancelled: false,
  quality: {
    status: "pass",
    scoreBasisPoints: 10000,
    acceptanceThresholdBasisPoints: 10000,
    eventsChecked: 2,
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
  },
};
it("keeps both supported banking formats in the batch action", () => {
  const onExtractAllDatasets = vi.fn();
  const detections: DatasetDetection[] = ["banking-csv", "banking-xlsx"].map(
    (adapterId) => ({
      adapterId,
      label: adapterId,
      provider: "banking",
      moduleId: "banking",
      stage: "raw_export",
      format: adapterId,
      readiness: "ready",
      evidence: [],
      commandPreview: null,
      nextStep: "Create Markdown",
    }),
  );
  render(
    <ScanView
      {...base}
      onExtractAllDatasets={onExtractAllDatasets}
      datasetInspection={{
        sourcePath: "/synthetic",
        fileCount: 2,
        inspectedPathCount: 2,
        truncated: false,
        catalogVersion: 1,
        outputContract: "Markdown",
        detections,
      }}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: /Create.*2 exports/ }));
  expect(onExtractAllDatasets).toHaveBeenCalledWith([
    "banking-csv",
    "banking-xlsx",
  ]);
});
it.each(["error", "cancelled"] as const)(
  "labels a partially completed %s batch for review",
  (phase) => {
    render(
      <ScanView
        {...base}
        extractionResults={[result]}
        extractionProgress={{
          adapterId: "diary-docx",
          phase,
          currentItem: null,
          recordsSeen: 2,
          sourceRecordsIncluded: 1,
          eventsWritten: 2,
          eventsSkipped: 0,
          errors: phase === "error" ? 1 : 0,
          totalHint: null,
          message: "Interrupted synthetic batch",
        }}
      />,
    );
    expect(screen.getByText("Some exports need attention")).toBeTruthy();
    expect(screen.queryByText("Your Markdown is ready")).toBeNull();
  },
);
it("does not present a cancelled or empty extraction as a completed export", () => {
  render(
    <ScanView
      {...base}
      extractionResults={[
        { ...result, cancelled: true },
        {
          ...result,
          adapterId: "diary-docx",
          recordsSeen: 0,
          sourceRecordsIncluded: 0,
          eventsWritten: 0,
        },
      ]}
    />,
  );
  expect(screen.queryByText("Your Markdown is ready")).toBeNull();
  expect(screen.queryByText("Data quality by source")).toBeNull();
});

it("reports retained events on a general rerun instead of claiming zero were created", () => {
  render(
    <ScanView
      {...base}
      extractionResults={[
        {
          ...result,
          adapterId: "diary-docx",
          recordsSeen: 2,
          sourceRecordsIncluded: 2,
          eventsWritten: 0,
          eventsSkipped: 2,
          skipReasons: { unchanged_rerun: 2 },
        },
      ]}
    />,
  );
  expect(screen.queryByText(/0 events were created/)).toBeNull();
  expect(screen.getByText(/2 unchanged events remain saved/)).toBeTruthy();
  expect(screen.getByText(/No duplicate files were created/)).toBeTruthy();
});

it("shows a Music catalog as complete without inventing dated activity", () => {
  render(
    <ScanView
      {...base}
      extractionResults={[
        {
          ...result,
          adapterId: "apple-music-local-library",
          recordsSeen: 0,
          sourceRecordsIncluded: 1,
          eventsWritten: 0,
          quality: {
            ...result.quality,
            status: "review",
            eventsChecked: 0,
            scoreBasisPoints: 0,
          },
        },
      ]}
    />,
  );
  expect(screen.getByText("Your Markdown is ready")).toBeTruthy();
  expect(screen.getByText("No Timeline events to validate")).toBeTruthy();
  expect(screen.queryByText("0.00% clean")).toBeNull();
  expect(
    screen.getByText(
      /1 tracks and all playlists were saved in separate Markdown catalogs/,
    ),
  ).toBeTruthy();
  expect(
    screen.getByText(/0 daily track Date Added files were added to Timeline/),
  ).toBeTruthy();
});

it("keeps actual catalog extraction errors visible", () => {
  render(
    <ScanView
      {...base}
      extractionResults={[
        {
          ...result,
          adapterId: "apple-music-local-library",
          recordsSeen: 0,
          sourceRecordsIncluded: 1,
          eventsWritten: 0,
          errors: 1,
          quality: {
            ...result.quality,
            status: "review",
            eventsChecked: 0,
            scoreBasisPoints: 0,
            extractionErrors: 1,
          },
        },
      ]}
    />,
  );
  expect(screen.getByText("Some exports need attention")).toBeTruthy();
});

it.each(["error", "cancelled"] as const)(
  "does not label indexed records as written Markdown files after %s",
  (phase) => {
    render(
      <ScanView
        {...base}
        extractionProgress={{
          adapterId: "timeline-index",
          phase,
          currentItem: null,
          recordsSeen: 3500,
          sourceRecordsIncluded: 0,
          eventsWritten: 3500,
          eventsSkipped: 0,
          errors: 0,
          totalHint: 60002,
          message:
            "Timeline rebuild stopped; previous complete index preserved",
        }}
      />,
    );
    expect(screen.getByText(/3,500 records/)).toBeTruthy();
    expect(screen.queryByText(/3,500 Markdown files/)).toBeNull();
  },
);
