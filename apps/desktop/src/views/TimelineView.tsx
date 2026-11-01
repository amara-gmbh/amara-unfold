// SPDX-License-Identifier: AGPL-3.0-or-later

import {
  CalendarClock,
  CalendarRange,
  Check,
  ChevronDown,
  FileText,
  Filter,
  FolderOpen,
  Layers3,
  Link2,
  LoaderCircle,
  RotateCcw,
  Search,
  Trash2,
  X,
} from "lucide-react";
import { type FormEvent, useMemo, useState } from "react";
import { PageHeader } from "../components/PageHeader";
import { compactPath, formatCount } from "../lib/format";
import type {
  TimelineBuildResult,
  TimelineCatalogEntry,
  TimelineEventContent,
} from "../types";
import { isReferenceTimelineEntry } from "../lib/timeline";

interface TimelineViewProps {
  timeline: TimelineBuildResult | null;
  archiveCount: number;
  archives: TimelineBuildResult[];
  entries: TimelineCatalogEntry[];
  onRemoveArchive: (folder: string) => void;
  onReadEvent: (
    timelineFolder: string,
    eventPath: string,
  ) => Promise<TimelineEventContent>;
  onSetEventDate: (
    entry: TimelineCatalogEntry,
    precision: "day" | "month" | "year",
    value: string,
    time: string | null,
  ) => Promise<void>;
  onUndoEventDate: (entry: TimelineCatalogEntry) => Promise<void>;
}

type PrecisionFilter =
  "all" | "day" | "month" | "year" | "undated" | "reference";
type ReviewFilter = "all" | "ready" | "review";

interface MonthGroup {
  key: string;
  label: string;
  entries: TimelineCatalogEntry[];
}

interface YearGroup {
  year: string;
  count: number;
  months: MonthGroup[];
}

export function TimelineView({
  timeline,
  archiveCount,
  archives,
  entries,
  onRemoveArchive,
  onReadEvent,
  onSetEventDate,
  onUndoEventDate,
}: TimelineViewProps) {
  const [query, setQuery] = useState("");
  const [provider, setProvider] = useState("all");
  const [precision, setPrecision] = useState<PrecisionFilter>("all");
  const [review, setReview] = useState<ReviewFilter>("all");
  const [expandedYears, setExpandedYears] = useState<Set<string>>(new Set());
  const [undatedOpen, setUndatedOpen] = useState(false);
  const [referencesOpen, setReferencesOpen] = useState(false);

  const providerOptions = useMemo(
    () =>
      [
        ...new Set(
          entries.map((entry) => providerKey(entry.module)).filter(Boolean),
        ),
      ].sort((left, right) => left.localeCompare(right)),
    [entries],
  );

  const filteredEntries = useMemo(() => {
    const needle = query.trim().toLocaleLowerCase();
    return entries.filter((entry) => {
      if (provider !== "all" && providerKey(entry.module) !== provider)
        return false;
      if (precision !== "all" && entryPrecision(entry) !== precision) {
        return false;
      }
      const isReady = entryIsReady(entry);
      if (review === "ready" && !isReady) return false;
      if (review === "review" && isReady) return false;
      if (!needle) return true;
      return [
        entry.title,
        entry.source,
        entry.module,
        entry.eventType,
        entry.eventPath,
        entry.rawPath,
      ].some((value) => value.toLocaleLowerCase().includes(needle));
    });
  }, [entries, precision, provider, query, review]);

  const datedEntries = useMemo(
    () =>
      filteredEntries.filter(
        (entry) => !["undated", "reference"].includes(entryPrecision(entry)),
      ),
    [filteredEntries],
  );
  const undatedEntries = useMemo(
    () =>
      filteredEntries.filter((entry) => entryPrecision(entry) === "undated"),
    [filteredEntries],
  );
  const referenceEntries = useMemo(
    () =>
      filteredEntries.filter((entry) => entryPrecision(entry) === "reference"),
    [filteredEntries],
  );
  const allReferenceEntries = useMemo(
    () => entries.filter((entry) => entryPrecision(entry) === "reference"),
    [entries],
  );
  const yearGroups = useMemo(() => groupByYear(datedEntries), [datedEntries]);

  if (!timeline) {
    return (
      <div className="view timeline-view">
        <PageHeader
          eyebrow="Your open archive"
          title="All events"
          description="Every completed export appears here after Unfold creates its open Markdown catalog."
        />
        <section className="large-empty panel">
          <div className="empty-orbit large" aria-hidden="true">
            <span />
          </div>
          <h2>No exported events yet</h2>
          <p>
            Create Markdown from a recognized source. The resulting events will
            appear here without changing or moving the original files.
          </p>
        </section>
      </div>
    );
  }

  const filtersActive =
    query.trim() !== "" ||
    provider !== "all" ||
    precision !== "all" ||
    review !== "all";

  function clearFilters() {
    setQuery("");
    setProvider("all");
    setPrecision("all");
    setReview("all");
  }

  function toggleYear(year: string) {
    setExpandedYears((current) =>
      current.has(year) ? new Set() : new Set([year]),
    );
  }

  return (
    <div className="view timeline-view">
      <PageHeader
        eyebrow="Your open archive"
        title="All events"
        description="Browse every generated event in one place. Photo drafts are separate until they are exported; every event here remains traceable to its Markdown and raw source."
        action={
          <div className="result-count">
            <CalendarRange size={15} /> {formatCount(timeline.eventsIndexed)}{" "}
            {timeline.eventsIndexed === 1 ? "event" : "events"}
          </div>
        }
      />

      <section
        className="metric-grid timeline-metrics"
        aria-label="Event summary"
      >
        <TimelineMetric label="All events" value={timeline.eventsIndexed} />
        <TimelineMetric label="Dated" value={timeline.datedEvents} />
        <TimelineMetric label="Undated" value={timeline.undatedEvents} />
        <TimelineMetric
          label="Reference notes"
          value={allReferenceEntries.length}
        />
        <TimelineMetric label="Months" value={timeline.monthPages} />
      </section>

      <details className="panel timeline-layers">
        <summary>
          <Layers3 aria-hidden="true" size={20} />
          <div>
            <strong>
              {archiveCount === 1
                ? "1 active archive layer"
                : `${archiveCount} active archive layers`}{" "}
              · {providerOptions.length}{" "}
              {providerOptions.length === 1 ? "provider" : "providers"}
            </strong>
            <span>
              Each export is one layer. Provider groups repeat exports by source
              type. Open to inspect or remove a layer; files stay on disk.
            </span>
          </div>
          <span className="route-ready">
            <Check size={14} />{" "}
            {timeline.strictPortable ? "Files validated" : "File review needed"}
          </span>
          <ChevronDown aria-hidden="true" className="layer-chevron" size={18} />
        </summary>
        <div className="timeline-layer-list">
          {archives.map((archive) => (
            <div className="timeline-layer" key={archive.folder}>
              <FolderOpen aria-hidden="true" size={18} />
              <div>
                <strong>{archiveLabel(archive.folder, entries)}</strong>
                <span>
                  {formatCount(archive.eventsIndexed)}{" "}
                  {archive.eventsIndexed === 1 ? "event" : "events"}
                </span>
                <code title={archive.folder}>
                  {compactPath(archive.folder, 6)}
                </code>
              </div>
              <button
                aria-label={`Remove ${archiveLabel(archive.folder, entries)} from All events`}
                className="remove-timeline-layer"
                onClick={() => onRemoveArchive(archive.folder)}
                type="button"
              >
                <Trash2 aria-hidden="true" size={15} /> Remove layer
              </button>
            </div>
          ))}
        </div>
      </details>

      <section
        className="panel event-filters"
        aria-label="Search and filter events"
      >
        <label className="event-filter event-search-filter">
          <span>Search</span>
          <div className="event-search">
            <Search aria-hidden="true" size={16} />
            <input
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search titles, sources, or paths"
              type="search"
              value={query}
            />
          </div>
        </label>
        <label className="event-filter">
          <span>Provider</span>
          <select
            onChange={(event) => setProvider(event.target.value)}
            value={provider}
          >
            <option value="all">All providers</option>
            {providerOptions.map((option) => (
              <option key={option} value={option}>
                {providerLabel(option)}
              </option>
            ))}
          </select>
        </label>
        <label className="event-filter">
          <span>Date</span>
          <select
            onChange={(event) =>
              setPrecision(event.target.value as PrecisionFilter)
            }
            value={precision}
          >
            <option value="all">Any precision</option>
            <option value="day">Exact date</option>
            <option value="month">Month only</option>
            <option value="year">Year only</option>
            <option value="undated">Undated</option>
            <option value="reference">Reference notes</option>
          </select>
        </label>
        <label className="event-filter">
          <span>Status</span>
          <select
            onChange={(event) => setReview(event.target.value as ReviewFilter)}
            value={review}
          >
            <option value="all">Any status</option>
            <option value="ready">Ready</option>
            <option value="review">Needs review</option>
          </select>
        </label>
        {filtersActive && (
          <button
            className="clear-event-filters"
            onClick={clearFilters}
            type="button"
          >
            <X size={14} /> Clear
          </button>
        )}
      </section>

      <div className="event-results-summary" aria-live="polite">
        <span>
          <Filter size={14} /> Showing {formatCount(filteredEntries.length)} of{" "}
          {formatCount(entries.length)} loaded events
        </span>
        {entries.length < timeline.eventsIndexed && (
          <span>
            The complete {formatCount(timeline.eventsIndexed)}-event catalog is
            preserved in the generated timeline folder.
          </span>
        )}
      </div>

      {undatedEntries.length > 0 && (
        <section className="panel undated-queue">
          <button
            aria-expanded={undatedOpen}
            className="undated-queue-toggle"
            onClick={() => setUndatedOpen((open) => !open)}
            type="button"
          >
            <div>
              <strong>Undated — review queue</strong>
              <span>
                {countLabel(undatedEntries.length, "event")}{" "}
                {undatedEntries.length === 1 ? "needs" : "need"} a usable date
                before joining the chronological view.
              </span>
            </div>
            <ChevronDown className={undatedOpen ? "expanded" : ""} size={18} />
          </button>
          {undatedOpen && (
            <div className="timeline-event-list undated-event-list">
              {undatedEntries.map((entry) => (
                <EventCard
                  entry={entry}
                  key={entry.id}
                  onReadEvent={onReadEvent}
                  onSetEventDate={onSetEventDate}
                  onUndoEventDate={onUndoEventDate}
                />
              ))}
            </div>
          )}
        </section>
      )}

      {referenceEntries.length > 0 && (
        <section className="panel undated-queue reference-queue">
          <button
            aria-expanded={referencesOpen}
            className="undated-queue-toggle"
            onClick={() => setReferencesOpen((open) => !open)}
            type="button"
          >
            <div>
              <strong>Timeless reference notes</strong>
              <span>
                {countLabel(referenceEntries.length, "topical source note")}{" "}
                {referenceEntries.length === 1 ? "has" : "have"} no
                chronological date. The content remains searchable without being
                placed on an invented date.
              </span>
            </div>
            <ChevronDown
              className={referencesOpen ? "expanded" : ""}
              size={18}
            />
          </button>
          {referencesOpen && (
            <div className="timeline-event-list undated-event-list">
              {referenceEntries.map((entry) => (
                <EventCard
                  entry={entry}
                  key={entry.id}
                  onReadEvent={onReadEvent}
                  onSetEventDate={onSetEventDate}
                  onUndoEventDate={onUndoEventDate}
                />
              ))}
            </div>
          )}
        </section>
      )}

      {yearGroups.length > 0 ? (
        <div className="event-year-list">
          {yearGroups.map((group) => {
            const isExpanded = expandedYears.has(group.year);
            return (
              <section className="event-year" key={group.year}>
                <button
                  aria-expanded={isExpanded}
                  className="event-year-toggle"
                  onClick={() => toggleYear(group.year)}
                  type="button"
                >
                  <span className="event-year-number">{group.year}</span>
                  <span>
                    {countLabel(group.count, "event")} ·{" "}
                    {countLabel(group.months.length, "date group")}
                  </span>
                  <ChevronDown
                    className={isExpanded ? "expanded" : ""}
                    size={19}
                  />
                </button>
                {isExpanded && (
                  <div className="event-month-list">
                    {group.months.map((month) => (
                      <EventMonth
                        group={month}
                        key={month.key}
                        onReadEvent={onReadEvent}
                        onSetEventDate={onSetEventDate}
                        onUndoEventDate={onUndoEventDate}
                      />
                    ))}
                  </div>
                )}
              </section>
            );
          })}
        </div>
      ) : undatedEntries.length === 0 && referenceEntries.length === 0 ? (
        <section className="large-empty panel filtered-events-empty">
          <h2>No events match these filters</h2>
          <p>
            Clear one or more filters to return to the complete event archive.
          </p>
          <button
            className="secondary-button"
            onClick={clearFilters}
            type="button"
          >
            Clear filters
          </button>
        </section>
      ) : null}
    </div>
  );
}

function TimelineMetric({ label, value }: { label: string; value: number }) {
  return (
    <div className="metric-card">
      <div className="metric-label">{label}</div>
      <strong>{formatCount(value)}</strong>
    </div>
  );
}

function countLabel(count: number, singular: string): string {
  return `${formatCount(count)} ${count === 1 ? singular : `${singular}s`}`;
}

function EventMonth({
  group,
  onReadEvent,
  onSetEventDate,
  onUndoEventDate,
}: {
  group: MonthGroup;
  onReadEvent: (
    timelineFolder: string,
    eventPath: string,
  ) => Promise<TimelineEventContent>;
  onSetEventDate: TimelineViewProps["onSetEventDate"];
  onUndoEventDate: TimelineViewProps["onUndoEventDate"];
}) {
  const [open, setOpen] = useState(false);
  return (
    <section className="event-month">
      <button
        aria-expanded={open}
        className="event-month-toggle"
        onClick={() => setOpen((value) => !value)}
        type="button"
      >
        <span>{group.label}</span>
        <small>{countLabel(group.entries.length, "event")}</small>
        <ChevronDown className={open ? "expanded" : ""} size={17} />
      </button>
      {open && (
        <div className="timeline-event-list">
          {group.entries.map((entry) => (
            <EventCard
              entry={entry}
              key={entry.id}
              onReadEvent={onReadEvent}
              onSetEventDate={onSetEventDate}
              onUndoEventDate={onUndoEventDate}
            />
          ))}
        </div>
      )}
    </section>
  );
}

function EventCard({
  entry,
  onReadEvent,
  onSetEventDate,
  onUndoEventDate,
}: {
  entry: TimelineCatalogEntry;
  onReadEvent: (
    timelineFolder: string,
    eventPath: string,
  ) => Promise<TimelineEventContent>;
  onSetEventDate: TimelineViewProps["onSetEventDate"];
  onUndoEventDate: TimelineViewProps["onUndoEventDate"];
}) {
  const [body, setBody] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function loadEvent() {
    if (body !== null || loading) return;
    setLoading(true);
    setError(null);
    try {
      const content = await onReadEvent(entry.timelineFolder, entry.eventPath);
      setBody(content.body);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  }

  return (
    <article className="timeline-event-card panel">
      <div className="timeline-event-date">
        <CalendarRange size={15} />
        <span>{eventDateLabel(entry)}</span>
      </div>
      <div className="timeline-event-copy">
        <span className="candidate-kicker">{eventKicker(entry)}</span>
        <h3>{entry.title}</h3>
        {entry.preview && (
          <p className="event-preview">
            <MarkdownFacts text={entry.preview} />
          </p>
        )}
        <details
          className="event-content"
          onToggle={(event) => {
            if (event.currentTarget.open) void loadEvent();
          }}
        >
          <summary>Read complete event</summary>
          {loading ? (
            <span className="event-content-state">
              <LoaderCircle className="spin" size={14} /> Loading Markdown…
            </span>
          ) : error ? (
            <span className="event-content-error">{error}</span>
          ) : (
            body !== null && (
              <pre>
                <MarkdownFacts
                  text={body || "This event has no readable body."}
                />
              </pre>
            )
          )}
        </details>
        <details className="event-evidence">
          <summary>Evidence</summary>
          <span title={entry.eventPath}>
            <FileText size={13} /> {compactPath(entry.eventPath, 4)}
          </span>
          <span title={entry.rawPath}>
            <Link2 size={13} /> {compactPath(entry.rawPath, 4)}
          </span>
        </details>
        {!isReferenceEntry(entry) &&
          (entryPrecision(entry) === "undated" || entry.manualDate) && (
            <DateEditor
              entry={entry}
              onSave={onSetEventDate}
              onUndo={onUndoEventDate}
            />
          )}
      </div>
      <span className="quiet-label">{reviewLabel(entry)}</span>
    </article>
  );
}

function DateEditor({
  entry,
  onSave,
  onUndo,
}: {
  entry: TimelineCatalogEntry;
  onSave: TimelineViewProps["onSetEventDate"];
  onUndo: TimelineViewProps["onUndoEventDate"];
}) {
  const [open, setOpen] = useState(false);
  const initialPrecision =
    entry.datePrecision === "year"
      ? "year"
      : entry.datePrecision === "month"
        ? "month"
        : "day";
  const [precision, setPrecision] = useState<"day" | "month" | "year">(
    initialPrecision,
  );
  const [value, setValue] = useState(() =>
    dateInputValue(entry, initialPrecision),
  );
  const [time, setTime] = useState(entry.time ?? "");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function changePrecision(next: "day" | "month" | "year") {
    setPrecision(next);
    setValue(dateInputValue(entry, next));
    if (next !== "day") setTime("");
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!value || saving) return;
    setSaving(true);
    setError(null);
    try {
      await onSave(
        entry,
        precision,
        value,
        precision === "day" && time ? time : null,
      );
      setSaving(false);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
      setSaving(false);
    }
  }

  async function undo() {
    if (saving) return;
    setSaving(true);
    setError(null);
    try {
      await onUndo(entry);
      setSaving(false);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
      setSaving(false);
    }
  }

  return (
    <div className={`manual-date-editor${open ? " open" : ""}`}>
      <button
        aria-expanded={open}
        className="manual-date-toggle"
        onClick={() => setOpen((current) => !current)}
        type="button"
      >
        <CalendarClock aria-hidden="true" size={14} />
        {entry.manualDate ? "Edit assigned date" : "Add a date"}
        <ChevronDown className={open ? "expanded" : ""} size={14} />
      </button>
      {open && (
        <form className="manual-date-form" onSubmit={submit}>
          <div className="manual-date-heading">
            <div className="manual-date-heading-row">
              <strong>
                {entry.manualDate
                  ? "Correct the assigned date"
                  : "Place this event on the Timeline"}
              </strong>
              {entry.manualDate && (
                <button
                  className="manual-date-undo"
                  onClick={() => void undo()}
                  type="button"
                >
                  <RotateCcw size={13} /> Undo assignment
                </button>
              )}
            </div>
            <span>
              Use only a date you can verify. The stable ID, original evidence,
              and raw source remain unchanged; the revision is recorded in the
              Markdown file.
            </span>
          </div>
          <div
            className={`manual-date-fields ${precision === "day" ? "with-time" : "without-time"}`}
          >
            <label>
              <span>Precision</span>
              <select
                onChange={(event) =>
                  changePrecision(
                    event.target.value as "day" | "month" | "year",
                  )
                }
                value={precision}
              >
                <option value="day">Exact day</option>
                <option value="month">Month only</option>
                <option value="year">Year only</option>
              </select>
            </label>
            <label>
              <span>
                {precision === "day"
                  ? "Date"
                  : precision === "month"
                    ? "Month"
                    : "Year"}
              </span>
              <input
                aria-label="Assigned date"
                max={precision === "year" ? "9999" : undefined}
                min={precision === "year" ? "1" : undefined}
                onChange={(event) => setValue(event.target.value)}
                placeholder={precision === "year" ? "YYYY" : undefined}
                required
                type={
                  precision === "day"
                    ? "date"
                    : precision === "month"
                      ? "month"
                      : "number"
                }
                value={value}
              />
            </label>
            {precision === "day" && (
              <label>
                <span>
                  Time <small>optional</small>
                </span>
                <input
                  aria-label="Assigned time"
                  onChange={(event) => setTime(event.target.value)}
                  type="time"
                  value={time}
                />
              </label>
            )}
            <button
              className="primary-button manual-date-save"
              disabled={saving || !value}
              type="submit"
            >
              {saving ? (
                <LoaderCircle className="spin" size={14} />
              ) : (
                <CalendarClock size={14} />
              )}
              {entry.manualDate ? "Save correction" : "Add to Timeline"}
            </button>
          </div>
          {error && <span className="manual-date-error">{error}</span>}
        </form>
      )}
    </div>
  );
}

function dateInputValue(
  entry: TimelineCatalogEntry,
  precision: "day" | "month" | "year",
): string {
  if (!entry.manualDate) return "";
  if (precision === "year") return entry.date.slice(0, 4);
  if (precision === "month") return entry.date.slice(0, 7);
  return entry.date.endsWith("-00") ? "" : entry.date;
}

function MarkdownFacts({ text }: { text: string }) {
  const parts = text.split(/(\*\*[^*]+\*\*)/g);
  return parts.map((part, index) =>
    part.startsWith("**") && part.endsWith("**") ? (
      <strong className="event-fact" key={`${index}-${part}`}>
        {part.slice(2, -2)}
      </strong>
    ) : (
      part
    ),
  );
}

function groupByYear(entries: TimelineCatalogEntry[]): YearGroup[] {
  const years = new Map<string, Map<string, TimelineCatalogEntry[]>>();
  for (const entry of entries) {
    const year = entry.date.slice(0, 4);
    const month =
      entry.date.slice(5, 7) === "00" ? "year" : entry.date.slice(0, 7);
    const months = years.get(year) ?? new Map<string, TimelineCatalogEntry[]>();
    months.set(month, [...(months.get(month) ?? []), entry]);
    years.set(year, months);
  }

  return [...years.entries()]
    .sort(([left], [right]) => right.localeCompare(left))
    .map(([year, months]) => {
      const monthGroups = [...months.entries()]
        .sort(([left], [right]) => {
          if (left === "year") return 1;
          if (right === "year") return -1;
          return right.localeCompare(left);
        })
        .map(([key, monthEntries]) => ({
          key,
          label: key === "year" ? "Year only" : monthLabel(key),
          entries: monthEntries,
        }));
      return {
        year,
        count: monthGroups.reduce(
          (total, month) => total + month.entries.length,
          0,
        ),
        months: monthGroups,
      };
    });
}

function datePrecision(date: string): Exclude<PrecisionFilter, "all"> {
  if (!date || date.startsWith("0000")) return "undated";
  if (date.slice(5, 7) === "00") return "year";
  if (date.endsWith("-00")) return "month";
  return "day";
}

function isReferenceEntry(entry: TimelineCatalogEntry): boolean {
  return isReferenceTimelineEntry(entry);
}

function entryPrecision(
  entry: TimelineCatalogEntry,
): Exclude<PrecisionFilter, "all"> {
  return isReferenceEntry(entry) ? "reference" : datePrecision(entry.date);
}

function entryIsReady(entry: TimelineCatalogEntry): boolean {
  return (
    isReferenceEntry(entry) ||
    entry.reviewStatus.toLocaleLowerCase() === "ready"
  );
}

function monthLabel(key: string): string {
  return new Date(`${key}-01T12:00:00`).toLocaleDateString(undefined, {
    month: "long",
  });
}

function dateLabel(date: string): string {
  const precision = datePrecision(date);
  if (precision === "undated") return "Undated";
  if (precision === "year") return date.slice(0, 4);
  if (precision === "month") {
    return new Date(`${date.slice(0, 7)}-01T12:00:00`).toLocaleDateString(
      undefined,
      { month: "long", year: "numeric" },
    );
  }
  return new Date(`${date}T12:00:00`).toLocaleDateString(undefined, {
    day: "numeric",
    month: "short",
    year: "numeric",
  });
}

function eventDateLabel(entry: TimelineCatalogEntry): string {
  if (isReferenceEntry(entry)) return "Timeless";
  const label = dateLabel(entry.date);
  return entry.time ? `${label} · ${entry.time}` : label;
}

function reviewLabel(entry: TimelineCatalogEntry): string {
  if (isReferenceEntry(entry)) return "Reference note";
  if (entry.manualDate) return "Manually dated";
  if (entry.reviewStatus.toLocaleLowerCase() === "ready") return "Ready";
  if (entry.reviewReasons?.includes("identity_resolution_pending")) {
    return "Identity review";
  }
  if (
    entry.reviewReasons?.some((reason) =>
      [
        "undated",
        "month_precision",
        "year_precision",
        "unquoted_date_scalar",
      ].includes(reason),
    )
  ) {
    return "Date review";
  }
  return "Data review";
}

function archiveLabel(folder: string, entries: TimelineCatalogEntry[]): string {
  const counts = new Map<string, number>();
  for (const entry of entries.filter(
    (candidate) => candidate.timelineFolder === folder,
  )) {
    if (entry.module) {
      counts.set(entry.module, (counts.get(entry.module) ?? 0) + 1);
    }
  }
  const normalizedFolder = folder.toLocaleLowerCase();
  const pathProvider = [...counts.keys()].find((provider) =>
    normalizedFolder.includes(provider.toLocaleLowerCase().replace(/_/g, "-")),
  );
  if (pathProvider) return `${providerLabel(pathProvider)} archive`;
  const dominantProvider = [...counts.entries()].sort(
    ([leftProvider, leftCount], [rightProvider, rightCount]) =>
      rightCount - leftCount || leftProvider.localeCompare(rightProvider),
  )[0]?.[0];
  if (dominantProvider) return `${providerLabel(dominantProvider)} archive`;
  return "Generated Markdown archive";
}

function providerLabel(provider: string): string {
  const branded: Record<string, string> = {
    airbnb: "Airbnb",
    diary: "Diary",
    gmail: "Gmail",
    google: "Google",
    "google-maps": "Google Maps",
    maps: "Google Maps",
    youtube: "YouTube",
  };
  const normalizedProvider = providerKey(provider);
  const key = normalizedProvider.toLocaleLowerCase();
  if (branded[key]) return branded[key];
  return normalizedProvider
    .split(/[-_]/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toLocaleUpperCase() + part.slice(1))
    .join(" ");
}

function providerKey(provider: string): string {
  const key = provider.toLocaleLowerCase().replace(/_/g, "-");
  if (["google-maps", "google-maps-pois", "kmz", "maps"].includes(key)) {
    return "google-maps";
  }
  return provider;
}

function eventKicker(entry: TimelineCatalogEntry): string {
  const provider = providerLabel(entry.module);
  const details = [providerLabel(entry.eventType)];
  if (
    entry.source.toLocaleLowerCase() !== entry.module.toLocaleLowerCase() &&
    entry.source.toLocaleLowerCase() !== entry.eventType.toLocaleLowerCase()
  ) {
    details.push(providerLabel(entry.source));
  }
  return [provider, ...details].join(" · ");
}
