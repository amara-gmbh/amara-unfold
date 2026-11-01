// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, it } from "vitest";
import type { TimelineCatalogEntry } from "../types";
import { summarizeTimelineEntries } from "./timeline";

function entry(
  id: string,
  date: string,
  tags?: string[],
): TimelineCatalogEntry {
  return {
    id,
    date,
    source: "test",
    module: "test",
    eventType: "test",
    title: id,
    preview: "",
    timelineFolder: "/Synthetic/Timeline",
    eventPath: `${id}.md`,
    rawPath: `/Synthetic/${id}`,
    reviewStatus: "ready",
    reviewReasons: [],
    tags,
  };
}

describe("summarizeTimelineEntries", () => {
  it("reconciles a catalog larger than the former 2,000-row boundary", () => {
    const entries = [
      ...Array.from({ length: 2_500 }, (_, index) =>
        entry(`dated-${index}`, index % 2 ? "2025-03-14" : "2026-02-01"),
      ),
      entry("undated", "0000-00-00"),
      entry("reference-1", "0000-00-00", ["reference"]),
      entry("reference-2", "0000-00-00", ["diary", "REFERENCE"]),
    ];

    const summary = summarizeTimelineEntries(entries);

    expect(summary).toEqual({
      eventsIndexed: 2_503,
      datedEvents: 2_500,
      undatedEvents: 1,
      referenceNotes: 2,
      monthPages: 2,
    });
    expect(summary.eventsIndexed).toBe(
      summary.datedEvents + summary.undatedEvents + summary.referenceNotes,
    );
  });
});
