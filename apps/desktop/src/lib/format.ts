// SPDX-License-Identifier: AGPL-3.0-or-later

export function formatCount(value: number): string {
  return new Intl.NumberFormat().format(value);
}

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const unitIndex = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    units.length - 1,
  );
  const value = bytes / 1024 ** unitIndex;
  return `${value >= 10 || unitIndex === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[unitIndex]}`;
}

export function formatDate(
  value: string | null,
  fallback = "Not scanned yet",
): string {
  if (!value) return fallback;
  const date = new Date(value);
  if (Number.isNaN(date.valueOf())) return value;
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

export function compactPath(path: string, segments = 3): string {
  const separator = path.includes("\\") ? "\\" : "/";
  const parts = path.split(separator).filter(Boolean);
  if (parts.length <= segments) return path;
  return `…${separator}${parts.slice(-segments).join(separator)}`;
}

export function userFacingPath(path: string): string {
  return path.replace(/^\/Users\/[^/]+(?=\/|$)/, "~");
}

export function batchWriteSummary(
  eventsWritten: number,
  unchangedEvents: number,
  completed: number,
  outputFolder: string,
): string {
  const destination = userFacingPath(outputFolder);
  if (eventsWritten === 0 && unchangedEvents > 0) {
    return `Done: ${unchangedEvents.toLocaleString()} unchanged Markdown ${unchangedEvents === 1 ? "event remains" : "events remain"} saved across ${completed} recognized exports to ${destination}; no duplicate files were created.`;
  }
  const retained =
    unchangedEvents > 0
      ? ` ${unchangedEvents.toLocaleString()} unchanged ${unchangedEvents === 1 ? "event remains" : "events remain"} saved; no duplicate files were created.`
      : "";
  return `Done: saved ${eventsWritten.toLocaleString()} new Markdown ${eventsWritten === 1 ? "event" : "events"} across ${completed} recognized exports to ${destination}.${retained}`;
}

export function fileName(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}
