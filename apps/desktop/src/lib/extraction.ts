// SPDX-License-Identifier: AGPL-3.0-or-later
import type { AdapterExtractionResult } from "../types";

// Music catalogs can contain useful tracks without any source Date Added.
// Zero inspected Timeline events is not a zero-percent catalog quality score.
export function isCatalogOnlyExtraction(
  result: AdapterExtractionResult,
): boolean {
  return (
    ["apple-music-tsv", "apple-music-local-library"].includes(
      result.adapterId,
    ) &&
    !result.cancelled &&
    result.sourceRecordsIncluded > 0 &&
    result.eventsWritten === 0 &&
    result.quality.eventsChecked === 0 &&
    result.errors === 0 &&
    result.quality.accountingBalanced &&
    result.quality.skipReasonsBalanced &&
    Object.entries(result.quality).every(
      ([key, value]) =>
        !(
          key.endsWith("Errors") ||
          key.endsWith("Warnings") ||
          key === "flaggedEvents" ||
          key === "sourceEmptyBodyRecords"
        ) || value === 0,
    )
  );
}
