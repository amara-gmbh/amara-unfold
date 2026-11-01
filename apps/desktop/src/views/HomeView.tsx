// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  ArrowRight,
  BookOpen,
  BrainCircuit,
  CalendarRange,
  FolderSearch2,
  Heart,
  Image,
  Layers3,
  Sparkles,
  Video,
} from "lucide-react";
import type { ReactNode } from "react";
import { formatBytes, formatCount, formatDate } from "../lib/format";
import type { DashboardSnapshot, Section, TimelineBuildResult } from "../types";
import { PageHeader } from "../components/PageHeader";

interface HomeViewProps {
  dashboard: DashboardSnapshot;
  timeline: TimelineBuildResult | null;
  onNavigate: (section: Section) => void;
  onChooseFolder: () => void;
}

export function HomeView({
  dashboard,
  timeline,
  onNavigate,
  onChooseFolder,
}: HomeViewProps) {
  const hasTimeline = Boolean(timeline);
  const hasArchive = dashboard.counts.files > 0 || hasTimeline;
  return (
    <div className="view home-view">
      <PageHeader
        eyebrow="Your memories, back in your hands"
        title={
          <>
            Discover your digital traces. <br />
            Understand what matters. <br />
            <span className="home-conserve-line">
              Conserve your memories in open files.
            </span>
          </>
        }
        description={
          hasArchive
            ? "You can now see which traces were found, where they came from, and what may be worth conserving. Everything stays on this device."
            : "Unfold helps you discover your digital traces, understand which ones matter, and conserve the memories you choose as open files."
        }
        action={
          <button
            className="primary-button"
            onClick={onChooseFolder}
            type="button"
          >
            <FolderSearch2 size={17} />
            {hasArchive ? "Scan another folder" : "Choose a folder"}
          </button>
        }
      />

      <section className="trust-strip" aria-label="Privacy state">
        <span className="status-dot" />
        <strong>Local only</strong>
        <span>
          Your files remain where they are. The index lives privately on this
          device.
        </span>
      </section>

      <section
        aria-labelledby="memory-guidance-title"
        className="memory-guidance panel"
      >
        <div className="memory-guidance-icon">
          <Heart aria-hidden="true" size={21} />
        </div>
        <div>
          <span className="eyebrow">
            We show you. We guide you. You choose.
          </span>
          <h2 id="memory-guidance-title">
            A date, a place, a photograph, a message, or a song can carry the
            shape of a life.
          </h2>
          <p>
            Unfold makes those traces visible and explains where they came from.
            Unfold is not surveillance, scoring, or a hidden dossier. You choose
            every source, every module, and every output; seeing a trace never
            means you must conserve it.
          </p>
        </div>
      </section>

      <section aria-labelledby="home-why-title" className="home-why panel">
        <div className="home-why-heading">
          <div className="home-why-icon">
            <BookOpen aria-hidden="true" size={20} />
          </div>
          <div>
            <span className="eyebrow">Why use Amara Unfold?</span>
            <h2 id="home-why-title">
              Because an export is not yet a memory you can use.
            </h2>
            <p>
              Platform exports are fragmented, technical, and difficult to see
              as a life. Unfold turns the traces you choose into one readable,
              traceable, open memory layer—so their meaning can become visible.
            </p>
          </div>
        </div>
        <div className="home-outcome-grid">
          <article>
            <BookOpen aria-hidden="true" size={18} />
            <strong>Read and keep it</strong>
            <p>
              Search, edit, move, and preserve ordinary open files without
              depending on Unfold or a company account.
            </p>
          </article>
          <article>
            <BrainCircuit aria-hidden="true" size={18} />
            <strong>Ask it—locally</strong>
            <p>
              A locally running LLM of your choice can explore selected
              Markdown. You may be surprised by the richness already present.
            </p>
          </article>
          <article>
            <Layers3 aria-hidden="true" size={18} />
            <strong>Continue with Amara Memory</strong>
            <p>
              Unfold is the open first step. When you want more, Amara Memory
              can add visual time and life layers, evidence-based connections,
              and grounded exploration.
            </p>
          </article>
        </div>
        <div className="home-not-extractor" role="note">
          <strong>Not metadata extractor number 27.</strong>
          <p>
            Extraction is only the intake step. Unfold normalizes different
            sources into one human-readable event structure, keeps provenance,
            builds a shared timeline and indexes, reports quality, and leaves
            you with an open memory foundation other tools can use.
          </p>
        </div>
        <p className="home-why-boundary">
          This build contains no LLM or live Amara Memory bridge. It prepares
          the open foundation for that optional next step, while your files
          remain useful on their own.
        </p>
      </section>

      <section className="metric-grid" aria-label="Archive summary">
        <Metric
          label={hasTimeline ? "Open events" : "Files indexed"}
          value={formatCount(
            hasTimeline
              ? (timeline?.eventsIndexed ?? 0)
              : dashboard.counts.files,
          )}
          note={
            hasTimeline
              ? `${formatCount(timeline?.monthPages ?? 0)} month pages`
              : `${formatBytes(dashboard.counts.bytes)} indexed`
          }
          icon={hasTimeline ? <CalendarRange size={16} /> : undefined}
        />
        <Metric
          label="Photos"
          value={formatCount(dashboard.counts.photos)}
          icon={<Image size={16} />}
        />
        <Metric
          label="Videos"
          value={formatCount(dashboard.counts.videos)}
          icon={<Video size={16} />}
        />
        <Metric
          label="Photo events found"
          value={formatCount(dashboard.candidates.length)}
          icon={<Sparkles size={16} />}
        />
      </section>

      <div className="home-columns">
        <section className="panel recent-panel">
          <div className="panel-heading">
            <div>
              <span className="eyebrow">Archive activity</span>
              <h2>{hasArchive ? "Recently indexed" : "Ready when you are"}</h2>
            </div>
            {hasArchive && (
              <button
                className="text-button"
                onClick={() => onNavigate("scan")}
                type="button"
              >
                Scan more <ArrowRight size={15} />
              </button>
            )}
          </div>
          {dashboard.sources.length > 0 ? (
            <div className="source-list compact">
              {dashboard.sources.slice(0, 3).map((source) => (
                <div className="source-row" key={source.id}>
                  <div className="source-icon">
                    <FolderSearch2 size={18} />
                  </div>
                  <div className="source-copy">
                    <strong>{source.displayName}</strong>
                    <span>
                      {formatCount(source.assetCount)} files ·{" "}
                      {formatDate(source.lastScannedAt)}
                    </span>
                  </div>
                  <span className="quiet-label">Local</span>
                </div>
              ))}
            </div>
          ) : timeline ? (
            <div className="source-list compact">
              <div className="source-row">
                <div className="source-icon">
                  <CalendarRange size={18} />
                </div>
                <div className="source-copy">
                  <strong>Latest open timeline</strong>
                  <span>
                    {formatCount(timeline.eventsIndexed)} events ·{" "}
                    {formatCount(timeline.datedEvents)} dated ·{" "}
                    {formatCount(timeline.undatedEvents)} undated
                  </span>
                </div>
                <span className="quiet-label">Ready</span>
              </div>
            </div>
          ) : (
            <EmptyMessage
              title="No folders indexed"
              text="Scanning reads metadata and builds a private catalog. It does not move or alter your files."
            />
          )}
        </section>

        <section className="panel candidates-preview">
          <div className="panel-heading">
            <div>
              <span className="eyebrow">What Unfold found</span>
              <h2>
                {timeline && dashboard.candidates.length === 0
                  ? "All events"
                  : "Photo event drafts"}
              </h2>
            </div>
            {dashboard.candidates.length > 0 && (
              <button
                className="text-button"
                onClick={() => onNavigate("memories")}
                type="button"
              >
                Open photo drafts <ArrowRight size={15} />
              </button>
            )}
            {dashboard.candidates.length === 0 && timeline && (
              <button
                className="text-button"
                onClick={() => onNavigate("timeline")}
                type="button"
              >
                Open all events <ArrowRight size={15} />
              </button>
            )}
          </div>
          {dashboard.candidates.length > 0 ? (
            <div className="candidate-mini-list">
              {dashboard.candidates.slice(0, 3).map((candidate) => (
                <div className="candidate-mini" key={candidate.clusterKey}>
                  <div className="date-tile">
                    <strong>{candidate.dateLabel.slice(8, 10)}</strong>
                    <span>{candidate.dateLabel.slice(0, 7)}</span>
                  </div>
                  <div>
                    <strong>
                      {candidate.placeHint ??
                        formatCandidateDate(candidate.dateLabel)}
                    </strong>
                    <span>
                      {formatCount(candidate.photoCount)} photos
                      {candidate.videoCount > 0 &&
                        ` · ${formatCount(candidate.videoCount)} videos`}
                      {candidate.startedAt &&
                        ` · ${formatTimeRange(candidate.startedAt, candidate.endedAt)}`}
                    </span>
                  </div>
                </div>
              ))}
            </div>
          ) : timeline ? (
            <div className="timeline-home-preview">
              <div className="source-icon">
                <CalendarRange size={18} />
              </div>
              <div className="source-copy">
                <strong>Browse your complete open archive</strong>
                <span>
                  {formatCount(timeline.monthPages)} months
                  {timeline.undatedEvents > 0 &&
                    ` · ${formatCount(timeline.undatedEvents)} undated`}
                </span>
              </div>
            </div>
          ) : (
            <EmptyMessage
              title="Photo event drafts will appear here"
              text="Photo drafts come from scanned photos and videos. Completed Markdown exports appear separately in All events."
            />
          )}
        </section>
      </div>

      {dashboard.lastScanAt && (
        <p className="last-updated">
          Last complete scan {formatDate(dashboard.lastScanAt)}
        </p>
      )}
    </div>
  );
}

function formatCandidateDate(dateLabel: string): string {
  return new Date(`${dateLabel}T12:00:00`).toLocaleDateString(undefined, {
    day: "numeric",
    month: "long",
    year: "numeric",
  });
}

function formatTimeRange(startedAt: string, endedAt: string | null): string {
  const start = startedAt.slice(11, 16);
  const end = endedAt?.slice(11, 16);
  return end && end !== start ? `${start}–${end}` : start;
}

function Metric({
  label,
  value,
  note,
  icon,
}: {
  label: string;
  value: string;
  note?: string;
  icon?: ReactNode;
}) {
  return (
    <div className="metric-card">
      <div className="metric-label">
        {icon}
        {label}
      </div>
      <strong>{value}</strong>
      {note && <span>{note}</span>}
    </div>
  );
}

function EmptyMessage({ title, text }: { title: string; text: string }) {
  return (
    <div className="empty-message">
      <div className="empty-orbit" aria-hidden="true">
        <span />
      </div>
      <strong>{title}</strong>
      <p>{text}</p>
    </div>
  );
}
