// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  Camera,
  ChevronDown,
  ChevronRight,
  Clock3,
  Download,
  Files,
  FileImage,
  Film,
  MapPin,
  Smartphone,
  Sparkles,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { PageHeader } from "../components/PageHeader";
import { desktop } from "../lib/desktop";
import { fileName, formatBytes, formatCount } from "../lib/format";
import type { DashboardSnapshot, MemoryCandidate } from "../types";

export function MemoriesView({ dashboard }: { dashboard: DashboardSnapshot }) {
  const [expanded, setExpanded] = useState<string | null>(null);
  const candidateCards = useRef(new Map<string, HTMLElement>());
  const pendingScroll = useRef<string | null>(null);
  const [exporting, setExporting] = useState<number | null>(null);
  const [exportMessage, setExportMessage] = useState<string | null>(null);
  const [exportError, setExportError] = useState<string | null>(null);

  async function exportMarkdown(candidate: MemoryCandidate) {
    if (candidate.id == null) return;
    setExporting(candidate.id);
    setExportMessage(null);
    setExportError(null);
    try {
      const result = await desktop.exportMemoryMarkdown(candidate.id);
      if (result) {
        setExportMessage(
          `Saved ${result.eventsWritten} open Markdown event files to ${result.folder}`,
        );
      }
    } catch (error) {
      setExportError(
        error instanceof Error ? error.message : "The Markdown export failed.",
      );
    } finally {
      setExporting(null);
    }
  }

  useEffect(() => {
    const clusterKey = pendingScroll.current;
    if (!clusterKey || expanded !== clusterKey) return;

    const frame = window.requestAnimationFrame(() => {
      pendingScroll.current = null;
      const card = candidateCards.current.get(clusterKey);
      const scroller = card?.closest<HTMLElement>(".workspace-scroll");
      if (!card || !scroller) return;

      const cardRect = card.getBoundingClientRect();
      const scrollerRect = scroller.getBoundingClientRect();
      const inset = 16;
      const availableHeight = scrollerRect.height - inset * 2;
      let target = scroller.scrollTop;

      if (cardRect.height > availableHeight) {
        target += cardRect.top - scrollerRect.top - inset;
      } else if (cardRect.bottom > scrollerRect.bottom - inset) {
        target += cardRect.bottom - scrollerRect.bottom + inset;
      } else if (cardRect.top < scrollerRect.top + inset) {
        target += cardRect.top - scrollerRect.top - inset;
      }

      if (Math.abs(target - scroller.scrollTop) > 1) {
        scroller.scrollTo({ top: Math.max(0, target), behavior: "smooth" });
      }
    });

    return () => window.cancelAnimationFrame(frame);
  }, [expanded]);

  return (
    <div className="view memories-view">
      <PageHeader
        eyebrow="Photos and videos only"
        title="Photo event drafts"
        description="Draft groups created from scanned photo and video metadata. Diary, Airbnb, and other completed Markdown exports appear in All events."
        action={
          <div className="result-count">
            <Sparkles size={15} /> {formatCount(dashboard.candidates.length)}{" "}
            {dashboard.candidates.length === 1 ? "photo draft" : "photo drafts"}
          </div>
        }
      />

      {dashboard.timeline.length === 0 ? (
        <section className="large-empty panel">
          <div className="empty-orbit large" aria-hidden="true">
            <span />
          </div>
          <h2>No photo event drafts yet</h2>
          <p>
            Scan a folder containing photos or videos and Unfold will group the
            media by EXIF capture time, then filesystem time. Converted Diary
            and other source events are available in All events.
          </p>
        </section>
      ) : (
        <div className="timeline">
          {dashboard.timeline.map((group) => {
            const monthCandidates = dashboard.candidates.filter((candidate) =>
              candidate.dateLabel.startsWith(group.key),
            );
            return (
              <section className="timeline-month" key={group.key}>
                <div className="timeline-marker">
                  <span />
                </div>
                <div className="timeline-month-header">
                  <div>
                    <h2>{group.label}</h2>
                    <span>
                      {formatCount(group.count)} files ·{" "}
                      {formatBytes(group.totalBytes)}
                    </span>
                  </div>
                  <div className="media-pills">
                    <span>
                      <FileImage size={13} /> {formatCount(group.photos)}
                    </span>
                    <span>
                      <Film size={13} /> {formatCount(group.videos)}
                    </span>
                  </div>
                </div>

                <div className="candidate-stack">
                  {monthCandidates.map((candidate) => {
                    const isExpanded = expanded === candidate.clusterKey;
                    return (
                      <article
                        className="candidate-card"
                        key={candidate.clusterKey}
                        ref={(element) => {
                          if (element) {
                            candidateCards.current.set(
                              candidate.clusterKey,
                              element,
                            );
                          } else {
                            candidateCards.current.delete(candidate.clusterKey);
                          }
                        }}
                      >
                        <button
                          aria-controls={`evidence-${candidate.clusterKey}`}
                          aria-expanded={isExpanded}
                          className="candidate-summary"
                          onClick={() => {
                            pendingScroll.current = isExpanded
                              ? null
                              : candidate.clusterKey;
                            setExpanded(
                              isExpanded ? null : candidate.clusterKey,
                            );
                          }}
                          type="button"
                        >
                          <span className="sr-only">
                            {isExpanded ? "Collapse" : "Expand"} memory
                          </span>
                          <div className="candidate-date">
                            <strong>{candidate.dateLabel.slice(8, 10)}</strong>
                            <span>
                              {new Date(
                                `${candidate.dateLabel}T12:00:00`,
                              ).toLocaleDateString(undefined, {
                                weekday: "short",
                              })}
                            </span>
                          </div>
                          <div className="candidate-title">
                            <span className="candidate-kicker">
                              Photo event
                            </span>
                            <h3>
                              {candidate.placeHint ??
                                formatLongDate(candidate.dateLabel)}
                            </h3>
                            <p>{candidateSummary(candidate)}</p>
                          </div>
                          {isExpanded ? (
                            <ChevronDown size={18} />
                          ) : (
                            <ChevronRight size={18} />
                          )}
                        </button>
                        {isExpanded && (
                          <div
                            className="evidence-panel"
                            id={`evidence-${candidate.clusterKey}`}
                          >
                            <section className="memory-finding">
                              <div>
                                <span className="candidate-kicker">
                                  What Unfold can say
                                </span>
                                <h4>
                                  These files form one photo event on{" "}
                                  {formatLongDate(candidate.dateLabel)}.
                                </h4>
                                <p>
                                  The grouping comes from capture times and file
                                  metadata. Unfold has not named the event or
                                  identified the people in it.
                                </p>
                              </div>
                              {candidate.id != null && (
                                <button
                                  className="secondary-button export-markdown-button"
                                  disabled={exporting === candidate.id}
                                  onClick={() => void exportMarkdown(candidate)}
                                  type="button"
                                >
                                  <Download size={15} />
                                  {exporting === candidate.id
                                    ? "Exporting metadata…"
                                    : "Export open Markdown"}
                                </button>
                              )}
                            </section>
                            {(exportMessage || exportError) && (
                              <div
                                className={`export-result ${exportError ? "error" : "success"}`}
                                role="status"
                              >
                                {exportError ?? exportMessage}
                              </div>
                            )}
                            <section className="observed-section">
                              <div className="section-label">
                                <strong>Observed from file metadata</strong>
                                <span>Facts found without uploading files</span>
                              </div>
                              <div className="observed-grid">
                                <ObservedFact
                                  icon={<Camera size={16} />}
                                  label="Media"
                                  value={mediaSummary(candidate)}
                                />
                                <ObservedFact
                                  icon={<Clock3 size={16} />}
                                  label="Captured"
                                  value={timeSummary(candidate)}
                                />
                                <ObservedFact
                                  icon={<MapPin size={16} />}
                                  label="Location evidence"
                                  value={locationSummary(candidate)}
                                />
                                <ObservedFact
                                  icon={<Smartphone size={16} />}
                                  label="Camera"
                                  value={
                                    candidate.deviceLabels.join(", ") ||
                                    "Not recorded"
                                  }
                                />
                                <ObservedFact
                                  icon={<Files size={16} />}
                                  label="Distinct files"
                                  value={`${formatCount(candidate.distinctFiles)} of ${formatCount(candidate.assetCount)}`}
                                />
                              </div>
                              {candidate.latitude != null &&
                                candidate.longitude != null && (
                                  <span className="coordinate-note">
                                    GPS centre {candidate.latitude.toFixed(4)},{" "}
                                    {candidate.longitude.toFixed(4)}
                                  </span>
                                )}
                              <p className="metadata-scope">
                                Exported per file: schema version, stable event
                                ID, date and time, media type, original path,
                                SHA-256, BLAKE3, dimensions, byte size, GPS,
                                device make/model, confidence, and provenance.
                              </p>
                            </section>
                            <div className="evidence-heading">
                              <div>
                                <strong>Files behind this draft</strong>
                                <span>
                                  Each grouping remains traceable to its local
                                  source files.
                                </span>
                              </div>
                              {candidate.placeHint && (
                                <span className="place-hint">
                                  <MapPin size={13} /> Folder hint:{" "}
                                  {candidate.placeHint}
                                </span>
                              )}
                            </div>
                            <div className="evidence-list">
                              {candidate.evidence.map((evidence) => (
                                <div
                                  className="evidence-row"
                                  key={evidence.path}
                                >
                                  <FileImage size={15} />
                                  <div>
                                    <strong>{fileName(evidence.path)}</strong>
                                    <span>
                                      {fileMetadataSummary(
                                        candidate,
                                        evidence.path,
                                      ) ?? evidence.reason}
                                    </span>
                                  </div>
                                  <time>
                                    {evidence.capturedAt?.slice(0, 10) ??
                                      "No date"}
                                  </time>
                                </div>
                              ))}
                            </div>
                          </div>
                        )}
                      </article>
                    );
                  })}
                </div>
              </section>
            );
          })}
        </div>
      )}
    </div>
  );
}

function ObservedFact({
  icon,
  label,
  value,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
}) {
  return (
    <div className="observed-fact">
      <span>{icon}</span>
      <div>
        <small>{label}</small>
        <strong>{value}</strong>
      </div>
    </div>
  );
}

function formatLongDate(dateLabel: string): string {
  return new Date(`${dateLabel}T12:00:00`).toLocaleDateString(undefined, {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  });
}

function mediaSummary(candidate: MemoryCandidate): string {
  const parts = [];
  if (candidate.photoCount) {
    parts.push(`${formatCount(candidate.photoCount)} photos`);
  }
  if (candidate.videoCount) {
    parts.push(`${formatCount(candidate.videoCount)} videos`);
  }
  return parts.join(" · ") || `${formatCount(candidate.assetCount)} files`;
}

function candidateSummary(candidate: MemoryCandidate): string {
  return [
    mediaSummary(candidate),
    timeSummary(candidate),
    locationSummary(candidate),
  ].join(" · ");
}

function timeSummary(candidate: MemoryCandidate): string {
  const start = candidate.startedAt?.slice(11, 16);
  const end = candidate.endedAt?.slice(11, 16);
  if (!start) return "Time not recorded";
  if (!end || end === start) return start;
  return `${start}–${end}`;
}

function locationSummary(candidate: MemoryCandidate): string {
  if (candidate.latitude == null || candidate.longitude == null) {
    return "No GPS recorded";
  }
  if (candidate.locationSpreadMeters == null) return "GPS recorded";
  if (candidate.locationSpreadMeters < 100) return "One GPS area";
  if (candidate.locationSpreadMeters < 1_000) {
    return `Within ${Math.round(candidate.locationSpreadMeters)} m`;
  }
  return `Within ${(candidate.locationSpreadMeters / 1_000).toFixed(1)} km`;
}

function fileMetadataSummary(
  candidate: MemoryCandidate,
  path: string,
): string | null {
  const asset = candidate.sampleAssets.find((item) => item.path === path);
  if (!asset) return null;
  const values = [
    asset.width || asset.height
      ? `${asset.width ?? "?"}×${asset.height ?? "?"}`
      : null,
    [asset.deviceMake, asset.deviceModel].filter(Boolean).join(" ") || null,
    asset.latitude != null && asset.longitude != null ? "GPS" : null,
    asset.contentHash ? `BLAKE3 ${asset.contentHash.slice(0, 10)}…` : null,
  ].filter(Boolean);
  return values.join(" · ") || null;
}
