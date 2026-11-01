// @vitest-environment jsdom
// SPDX-License-Identifier: AGPL-3.0-or-later
import {
  cleanup,
  render,
  screen,
  waitFor,
  within,
  fireEvent,
} from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import App from "./App";
import { desktop } from "./lib/desktop";

vi.mock("./lib/desktop", () => ({
  desktop: {
    dashboard: vi.fn().mockResolvedValue({
      counts: { files: 0, photos: 0, videos: 0, bytes: 0 },
      sources: [],
      candidates: [],
      recentScanErrors: [],
    }),
    privacy: vi.fn().mockResolvedValue({ mode: "local_only" }),
    localAppSources: vi.fn().mockResolvedValue([]),
    discoverTimelineArchives: vi.fn().mockResolvedValue([]),
    timelineCatalog: vi.fn(),
    onExtractionProgress: vi.fn().mockResolvedValue(() => {}),
    onScanProgress: vi.fn().mockResolvedValue(() => {}),
  },
}));
const archive = {
  folder: "/Synthetic/Timeline",
  eventsIndexed: 6766,
  datedEvents: 6766,
  undatedEvents: 0,
  duplicateEvents: 0,
  rejectedEvents: 0,
  dayPages: 4842,
  monthPages: 100,
  entityAnchors: 0,
  crossSourceDays: 0,
  strictPortable: true,
};
beforeEach(() => {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      disconnect() {}
    },
  );
  window.localStorage.clear();
  window.localStorage.setItem(
    "amara-unfold-timeline-archives",
    JSON.stringify([archive]),
  );
  window.matchMedia = vi.fn().mockReturnValue({
    matches: false,
    addEventListener() {},
    removeEventListener() {},
  });
});
afterEach(cleanup);
it("never presents cached event totals while the catalog is loading", () => {
  vi.mocked(desktop.timelineCatalog).mockReturnValue(new Promise(() => {}));
  render(<App />);
  expect(
    within(screen.getByLabelText("Archive summary")).queryByText("6,766"),
  ).toBeNull();
  expect(screen.getByText(/Loading saved event catalogs/)).toBeTruthy();
});
it("uses zero loaded entries after an empty catalog and reconciles both views", async () => {
  vi.mocked(desktop.timelineCatalog).mockResolvedValue([]);
  render(<App />);
  await waitFor(() =>
    expect(screen.queryByText(/Loading saved event catalogs/)).toBeNull(),
  );
  expect(
    within(screen.getByLabelText("Archive summary")).queryByText("6,766"),
  ).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "All events" }));
  expect(
    within(screen.getByLabelText("Event summary")).queryByText("6,766"),
  ).toBeNull();
});
it("reports an unavailable catalog without treating cached totals as loaded events", async () => {
  vi.mocked(desktop.timelineCatalog).mockRejectedValue(
    new Error("synthetic missing catalog"),
  );
  render(<App />);
  await waitFor(() =>
    expect(
      screen.getByText(/1 saved archive could not be loaded/),
    ).toBeTruthy(),
  );
  expect(
    within(screen.getByLabelText("Archive summary")).queryByText("6,766"),
  ).toBeNull();
});

it("counts a repeated event once across two loaded archives", async () => {
  window.localStorage.setItem(
    "amara-unfold-timeline-archives",
    JSON.stringify([archive, { ...archive, folder: "/Synthetic/Second" }]),
  );
  const event = {
    id: "same-event",
    date: "2024-04-05",
    source: "synthetic",
    module: "synthetic",
    eventType: "fixture",
    title: "Synthetic record",
    preview: "",
    timelineFolder: archive.folder,
    eventPath: "events/record.md",
    rawPath: "/Synthetic/raw.csv",
    reviewStatus: "ready",
  };
  vi.mocked(desktop.timelineCatalog).mockResolvedValue([event]);
  render(<App />);
  await waitFor(() =>
    expect(screen.queryByText(/Loading saved event catalogs/)).toBeNull(),
  );
  expect(
    within(screen.getByLabelText("Archive summary")).getByText("1"),
  ).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "All events" }));
  expect(screen.getByText(/Showing/).textContent).toContain("1");
  expect(screen.queryByText("13,532")).toBeNull();
});
