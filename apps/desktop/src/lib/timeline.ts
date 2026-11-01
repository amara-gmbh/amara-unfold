// SPDX-License-Identifier: AGPL-3.0-or-later

import type { TimelineCatalogEntry } from "../types";

export interface TimelineCatalogSummary {
  eventsIndexed: number;
  datedEvents: number;
  undatedEvents: number;
  referenceNotes: number;
  monthPages: number;
}

export function isReferenceTimelineEntry(entry: TimelineCatalogEntry): boolean {
  return Boolean(
    entry.tags?.some((tag) => tag.toLocaleLowerCase() === "reference"),
  );
}

export function summarizeTimelineEntries(
  entries: TimelineCatalogEntry[],
): TimelineCatalogSummary {
  let datedEvents = 0;
  let undatedEvents = 0;
  let referenceNotes = 0;
  const months = new Set<string>();

  for (const entry of entries) {
    if (isReferenceTimelineEntry(entry)) {
      referenceNotes += 1;
      continue;
    }
    if (!entry.date || entry.date.startsWith("0000-")) {
      undatedEvents += 1;
      continue;
    }

    datedEvents += 1;
    const yearMonth = entry.date.slice(0, 7);
    if (
      yearMonth.length === 7 &&
      !yearMonth.startsWith("0000-") &&
      !yearMonth.endsWith("-00")
    ) {
      months.add(yearMonth);
    }
  }

  return {
    eventsIndexed: entries.length,
    datedEvents,
    undatedEvents,
    referenceNotes,
    monthPages: months.size,
  };
}
