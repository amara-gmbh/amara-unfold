// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, it } from "vitest";
import {
  batchWriteSummary,
  compactPath,
  fileName,
  formatBytes,
  userFacingPath,
} from "./format";

describe("archive display formatting", () => {
  it("uses human-readable binary units", () => {
    expect(formatBytes(1024)).toBe("1.0 KB");
    expect(formatBytes(12 * 1024 * 1024)).toBe("12 MB");
  });

  it("handles macOS and Windows paths without changing stored values", () => {
    expect(fileName("/Photos/2019/Jersey/a.jpg")).toBe("a.jpg");
    expect(fileName("C:\\Photos\\a.jpg")).toBe("a.jpg");
    expect(compactPath("/Photos/2019/Jersey/a.jpg", 2)).toBe("…/Jersey/a.jpg");
  });

  it("hides the local macOS account name in user-facing default paths", () => {
    expect(
      userFacingPath("/Users/example/Pictures/Photos Library.photoslibrary"),
    ).toBe("~/Pictures/Photos Library.photoslibrary");
    expect(
      userFacingPath("/Users/example/Music/Music/Music Library.musiclibrary"),
    ).toBe("~/Music/Music/Music Library.musiclibrary");
    expect(userFacingPath("/External/T9/Takeout")).toBe("/External/T9/Takeout");
  });

  it("reports unchanged batch output as retained instead of saved zero", () => {
    const summary = batchWriteSummary(0, 12, 5, "/Synthetic/Output");
    expect(summary).toContain("12 unchanged Markdown events remain saved");
    expect(summary).toContain("no duplicate files were created");
    expect(summary).not.toContain("saved 0");
  });
});
