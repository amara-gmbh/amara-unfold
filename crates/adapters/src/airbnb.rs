// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    input,
    model::{date_time_parts, slug, EventInput, OpenEvent},
    record_event, record_skip, AdapterError, ExtractionProgress, ProgressCallback,
};
use chrono::{Datelike, Duration, NaiveDate};
use serde_json::{Map, Value};
use std::{
    collections::HashSet,
    fmt::Write as _,
    fs,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Debug)]
struct CoverageRow {
    file: String,
    records: u64,
    written: u64,
    skipped: u64,
    disposition: String,
}

pub fn extract(
    source: &Path,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    let blobs = input::matching_blobs(source, |path| {
        let lower = path.to_lowercase();
        lower.ends_with(".json")
            && (lower.contains("/json/")
                || lower.ends_with("reservations.json")
                || lower.ends_with("messages.json")
                || lower.ends_with("reviews.json"))
    })?;
    if blobs.is_empty() {
        return Err(AdapterError::Input(
            "No readable Airbnb JSON category files were found".into(),
        ));
    }
    let mut seen = HashSet::new();
    let mut coverage = Vec::new();
    for blob in blobs {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let value: Value = serde_json::from_slice(&blob.bytes)
            .map_err(|error| AdapterError::Parse(error.to_string()))?;
        let lower = blob.raw_path.to_lowercase();
        let file = Path::new(&blob.raw_path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(&blob.raw_path)
            .to_owned();
        let expected_records = airbnb_record_count(&lower, &value);
        progress.total_hint = Some(progress.total_hint.unwrap_or(0) + expected_records);
        callback(progress.clone());
        let records_before = progress.records_seen;
        let written_before = progress.events_written;
        let skipped_before = progress.events_skipped;
        let disposition = if lower.ends_with("reservations.json") {
            process_reservations(
                &value,
                &blob.raw_path,
                package,
                cancel,
                progress,
                callback,
                &mut seen,
            )?;
            "All reservation statuses converted; booking-session containers explicitly excluded as supporting metadata".to_owned()
        } else if lower.ends_with("messages.json") {
            process_messages(
                &value,
                &blob.raw_path,
                package,
                cancel,
                progress,
                callback,
                &mut seen,
            )?;
            "Readable timestamped messages converted, including human-readable Airbnb system-message fallbacks; incomplete message records explicitly counted"
                .to_owned()
        } else if lower.ends_with("reviews.json") {
            process_reviews(
                &value,
                &blob.raw_path,
                package,
                cancel,
                progress,
                callback,
                &mut seen,
            )?;
            "Provided and received reviews converted; incomplete review records explicitly counted"
                .to_owned()
        } else {
            let count = logical_record_count(&value);
            let reason = format!("not_timeline_material_{}", reason_slug(&file));
            record_counted_skips(progress, &reason, count);
            progress.current_item = Some(file.clone());
            progress.message = format!(
                "Inspected {} Airbnb records; non-timeline categories remain in the raw export",
                progress.records_seen
            );
            callback(progress.clone());
            "Inspected and explicitly excluded from Timeline to avoid noisy or sensitive static-account records".to_owned()
        };
        coverage.push(CoverageRow {
            file,
            records: progress.records_seen - records_before,
            written: progress.events_written - written_before,
            skipped: progress.events_skipped - skipped_before,
            disposition,
        });
    }
    write_coverage_report(package.root(), &coverage, progress)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn process_reservations(
    value: &Value,
    raw_path: &str,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
    seen: &mut HashSet<String>,
) -> Result<(), AdapterError> {
    let mut records = Vec::new();
    collect_named_records(value, "reservations", &mut records);
    for record in records {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let raw =
            serde_json::to_vec(record).map_err(|error| AdapterError::Parse(error.to_string()))?;
        match reservation_event(record, raw_path, &raw) {
            Some(event) => write_or_skip(package, event, progress, callback, seen)?,
            None => record_counted_skip(progress, "reservation_missing_date", callback),
        }
    }
    let mut booking_sessions = Vec::new();
    collect_named_records(value, "bookingSessions", &mut booking_sessions);
    record_counted_skips(
        progress,
        "booking_session_supporting_metadata",
        booking_sessions.len() as u64,
    );
    callback(progress.clone());
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn process_messages(
    value: &Value,
    raw_path: &str,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
    seen: &mut HashSet<String>,
) -> Result<(), AdapterError> {
    let mut threads = Vec::new();
    collect_named_records(value, "messageThreads", &mut threads);
    for thread in threads {
        let thread_id = scalar_text(thread.get("id"));
        let Some(messages) = thread.get("messagesAndContents").and_then(Value::as_array) else {
            continue;
        };
        for record in messages.iter().filter_map(Value::as_object) {
            if cancel.load(Ordering::Relaxed) {
                return Ok(());
            }
            let raw = serde_json::to_vec(record)
                .map_err(|error| AdapterError::Parse(error.to_string()))?;
            match message_event(record, thread_id.as_deref(), raw_path, &raw) {
                Some(event) => write_or_skip(package, event, progress, callback, seen)?,
                None => record_counted_skip(
                    progress,
                    "message_missing_readable_body_or_timestamp",
                    callback,
                ),
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn process_reviews(
    value: &Value,
    raw_path: &str,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
    seen: &mut HashSet<String>,
) -> Result<(), AdapterError> {
    for (key, direction) in [
        ("reviewsProvided", ReviewDirection::Provided),
        ("reviewsReceived", ReviewDirection::Received),
    ] {
        let mut records = Vec::new();
        collect_named_records(value, key, &mut records);
        for record in records {
            if cancel.load(Ordering::Relaxed) {
                return Ok(());
            }
            let raw = serde_json::to_vec(record)
                .map_err(|error| AdapterError::Parse(error.to_string()))?;
            match review_event(record, direction, raw_path, &raw) {
                Some(event) => write_or_skip(package, event, progress, callback, seen)?,
                None => {
                    record_counted_skip(progress, "review_missing_comment_or_timestamp", callback)
                }
            }
        }
    }
    Ok(())
}

fn reservation_event(object: &Map<String, Value>, raw_path: &str, raw: &[u8]) -> Option<OpenEvent> {
    let start = text(object, "startDate")?;
    let nights = object.get("nights").and_then(Value::as_i64);
    let computed_end = text(object, "endDate")
        .map(str::to_owned)
        .or_else(|| end_from_nights(start, nights));
    let guests = object.get("numberOfGuests").and_then(Value::as_i64);
    let status = text(object, "status").unwrap_or("accepted");
    let completed = is_completed_stay(object);
    let title = if completed {
        format!(
            "Airbnb stay - {}",
            human_date_range(start, computed_end.as_deref())
        )
    } else {
        format!(
            "Airbnb {} reservation - {}",
            status.to_ascii_lowercase(),
            human_date_range(start, computed_end.as_deref())
        )
    };
    let listing = text(object, "hostingUrl").unwrap_or("not included in export");
    let confirmation = text(object, "confirmationCode").unwrap_or("not included in export");
    let actors = text(object, "hostProfileUrl")
        .and_then(last_path_component)
        .map(|value| slug(&format!("airbnb-host-{value}")))
        .into_iter()
        .collect();
    Some(OpenEvent::new(EventInput {
        date: start,
        time: None,
        end_date: computed_end.as_deref(),
        source: "location",
        module: "airbnb",
        event_type: if completed { "stay" } else { "reservation" },
        title: &title,
        raw_path,
        raw_bytes: raw,
        body: format!(
            "Airbnb stay\n\nDates: {}\n\nNights: {}\n\nGuests: {}\n\nStatus: {}\n\nListing: {}\n\nConfirmation: {}",
            human_date_range(start, computed_end.as_deref()),
            nights
                .map(|value| value.to_string())
                .unwrap_or_else(|| "not included in export".into()),
            guests
                .map(|value| value.to_string())
                .unwrap_or_else(|| "not included in export".into()),
            status,
            listing,
            confirmation
        ),
        actors,
        places: Vec::new(),
        tags: vec![
            "airbnb".into(),
            if completed {
                "stay".into()
            } else {
                "reservation".into()
            },
            slug(status),
        ],
        confidence: "high",
    }))
}

fn message_event(
    object: &Map<String, Value>,
    thread_id: Option<&str>,
    raw_path: &str,
    raw: &[u8],
) -> Option<OpenEvent> {
    let message = object.get("message")?.as_object()?;
    let created = text(message, "createdAt")?;
    let (date, time) = date_time_parts(created)?;
    let content = object.get("messageContent")?;
    let body = content
        .pointer("/textContent/body")
        .or_else(|| content.pointer("/messageContentV2/content"))
        .or_else(|| content.pointer("/textAndReferenceContent/text"))
        .and_then(Value::as_str)
        .and_then(readable_message_body)?;
    let sender_type = text(message, "accountType");
    let actors = sender_type
        .map(|value| slug(&format!("airbnb-{value}")))
        .into_iter()
        .collect();
    let title = format!("Airbnb message - {}", summary(&body, 72));
    let mut tags = vec!["airbnb".into(), "message".into()];
    if let Some(id) = thread_id {
        tags.push(slug(&format!("thread-{id}")));
    }
    Some(OpenEvent::new(EventInput {
        date: &date,
        time: time.as_deref(),
        end_date: None,
        source: "message",
        module: "airbnb",
        event_type: "message",
        title: &title,
        raw_path,
        raw_bytes: raw,
        body: format!(
            "Airbnb message\n\nThread: {}\n\nSender type: {}\n\n{}",
            thread_id.unwrap_or("not included in export"),
            sender_type.unwrap_or("not included in export"),
            body
        ),
        actors,
        places: Vec::new(),
        tags,
        confidence: "high",
    }))
}

/// Airbnb occasionally serializes service messages as an internal Java/Kotlin-style
/// `MessageContent{...}` string. Publishing that implementation detail makes the
/// Timeline unreadable. These records also carry the human-facing text in the first
/// `fallback=` field, so preserve that text and never expose the opaque container.
fn readable_message_body(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if !trimmed.starts_with("MessageContent{") {
        return Some(trimmed.to_owned());
    }
    let fallback = trimmed.split_once("fallback=")?.1;
    let fallback = fallback
        .split_once(", interpolationParameters=")
        .map_or(fallback, |(text, _)| text)
        .trim();
    (!fallback.is_empty() && fallback != "null").then(|| fallback.to_owned())
}

#[derive(Clone, Copy)]
enum ReviewDirection {
    Provided,
    Received,
}

fn review_event(
    object: &Map<String, Value>,
    direction: ReviewDirection,
    raw_path: &str,
    raw: &[u8],
) -> Option<OpenEvent> {
    let review = object.get("review")?.as_object()?;
    let created = text(review, "submittedAt").or_else(|| text(review, "createdAt"))?;
    let (date, time) = date_time_parts(created)?;
    let comment = text(review, "comment")?;
    let (direction_label, actor_key, direction_tag) = match direction {
        ReviewDirection::Provided => ("you wrote", "revieweeId", "provided"),
        ReviewDirection::Received => ("you received", "reviewerId", "received"),
    };
    let title = format!("Airbnb review {direction_label} - {}", human_date(&date));
    let actor_id = scalar_text(review.get(actor_key));
    let actors = actor_id
        .as_deref()
        .map(|value| slug(&format!("airbnb-{value}")))
        .into_iter()
        .collect();
    let rating =
        scalar_text(review.get("rating")).unwrap_or_else(|| "not included in export".into());
    Some(OpenEvent::new(EventInput {
        date: &date,
        time: time.as_deref(),
        end_date: None,
        source: "web",
        module: "airbnb",
        event_type: "review",
        title: &title,
        raw_path,
        raw_bytes: raw,
        body: format!("Airbnb review {direction_label}\n\nRating: {rating}\n\n{comment}"),
        actors,
        places: Vec::new(),
        tags: vec!["airbnb".into(), "review".into(), direction_tag.into()],
        confidence: "high",
    }))
}

fn airbnb_record_count(path: &str, value: &Value) -> u64 {
    if path.ends_with("reservations.json") {
        named_record_count(value, "reservations") + named_record_count(value, "bookingSessions")
    } else if path.ends_with("messages.json") {
        let mut threads = Vec::new();
        collect_named_records(value, "messageThreads", &mut threads);
        threads
            .into_iter()
            .filter_map(|thread| thread.get("messagesAndContents").and_then(Value::as_array))
            .map(|messages| messages.iter().filter(|value| value.is_object()).count() as u64)
            .sum()
    } else if path.ends_with("reviews.json") {
        named_record_count(value, "reviewsProvided") + named_record_count(value, "reviewsReceived")
    } else {
        logical_record_count(value)
    }
}

fn named_record_count(value: &Value, key: &str) -> u64 {
    let mut records = Vec::new();
    collect_named_records(value, key, &mut records);
    records.len() as u64
}

fn logical_record_count(value: &Value) -> u64 {
    let roots = match value {
        Value::Array(values) => values.iter().collect::<Vec<_>>(),
        other => vec![other],
    };
    let mut count = 0_u64;
    for root in roots {
        let Some(object) = root.as_object() else {
            count += 1;
            continue;
        };
        let mut nested_records = 0_u64;
        let mut has_scalar_record = false;
        for child in object.values() {
            nested_records += match child {
                Value::Array(values) => values.len() as u64,
                Value::Object(_) => 1,
                Value::Null => 0,
                _ => {
                    has_scalar_record = true;
                    0
                }
            };
        }
        count += if nested_records > 0 {
            nested_records
        } else if has_scalar_record {
            1
        } else {
            0
        };
    }
    count
}

fn write_or_skip(
    package: &crate::writer::PackageWriter,
    event: OpenEvent,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
    seen: &mut HashSet<String>,
) -> Result<(), AdapterError> {
    if seen.insert(event.id.clone()) {
        record_event(package, &event, progress, callback)
    } else {
        record_counted_skip(progress, "duplicate_source_record", callback);
        Ok(())
    }
}

fn record_counted_skip(
    progress: &mut ExtractionProgress,
    reason: &str,
    callback: &mut ProgressCallback<'_>,
) {
    progress.records_seen += 1;
    record_skip(progress, reason);
    progress.message = format!("Inspected {} Airbnb records", progress.records_seen);
    callback(progress.clone());
}

fn record_counted_skips(progress: &mut ExtractionProgress, reason: &str, count: u64) {
    if count == 0 {
        return;
    }
    progress.records_seen += count;
    progress.events_skipped += count;
    *progress.skip_reasons.entry(reason.into()).or_default() += count;
}

fn write_coverage_report(
    root: &Path,
    rows: &[CoverageRow],
    progress: &ExtractionProgress,
) -> Result<(), AdapterError> {
    let mut report = String::from(
        "# Airbnb takeout coverage\n\nEvery inspected logical source record is either represented by event Markdown or explicitly counted below as not converted. The raw Airbnb export remains unchanged. Static account, telemetry, and supporting-container categories are not added to Timeline because they would create noise or expose sensitive account details as apparent life events. They remain available in the original raw JSON.\n\n",
    );
    writeln!(
        report,
        "- JSON category files inspected: {}\n- Logical records inspected: {}\n- Event Markdown written: {}\n- Records not converted, with reasons: {}\n- Extraction errors: {}\n",
        rows.len(),
        progress.records_seen,
        progress.events_written,
        progress.events_skipped,
        progress.errors
    )
    .expect("writing to String cannot fail");
    report.push_str("| Airbnb JSON category | Records inspected | Markdown events | Not converted | Disposition |\n|---|---:|---:|---:|---|\n");
    for row in rows {
        writeln!(
            report,
            "| `{}` | {} | {} | {} | {} |",
            row.file.replace('|', "\\|"),
            row.records,
            row.written,
            row.skipped,
            row.disposition.replace('|', "\\|")
        )
        .expect("writing to String cannot fail");
    }
    report.push_str("\n## Reasons records were not converted\n\n");
    for (reason, count) in &progress.skip_reasons {
        writeln!(report, "- `{reason}`: {count}").expect("writing to String cannot fail");
    }
    report.push_str("\n`manifest.yaml` carries the same balanced skip-reason totals in machine-readable form.\n");
    fs::write(root.join("AIRBNB-COVERAGE.md"), report)
        .map_err(|error| AdapterError::Output(error.to_string()))
}

fn reason_slug(file: &str) -> String {
    file.trim_end_matches(".json")
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn collect_named_records<'a>(
    value: &'a Value,
    key: &str,
    output: &mut Vec<&'a Map<String, Value>>,
) {
    match value {
        Value::Object(object) => {
            if let Some(records) = object.get(key).and_then(Value::as_array) {
                output.extend(records.iter().filter_map(Value::as_object));
            }
            for child in object.values() {
                collect_named_records(child, key, output);
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_named_records(child, key, output);
            }
        }
        _ => {}
    }
}

fn is_completed_stay(object: &Map<String, Value>) -> bool {
    match text(object, "status").map(|value| value.to_ascii_lowercase()) {
        Some(status) => matches!(status.as_str(), "accepted" | "confirmed" | "completed"),
        None => true,
    }
}

fn end_from_nights(start: &str, nights: Option<i64>) -> Option<String> {
    let start = NaiveDate::parse_from_str(start, "%Y-%m-%d").ok()?;
    let nights = nights.filter(|value| *value >= 0)?;
    start
        .checked_add_signed(Duration::days(nights))
        .map(|date| date.to_string())
}

fn human_date_range(start: &str, end: Option<&str>) -> String {
    let Some(end) = end else {
        return human_date(start);
    };
    if start == end {
        return human_date(start);
    }
    format!("{} to {}", human_date(start), human_date(end))
}

fn human_date(value: &str) -> String {
    let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") else {
        return value.to_owned();
    };
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let month = MONTHS[date.month0() as usize];
    format!("{} {month} {}", date.day(), date.year())
}

fn summary(value: &str, max_chars: usize) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut result = collapsed.chars().take(max_chars).collect::<String>();
    if collapsed.chars().count() > max_chars {
        result.push_str("...");
    }
    result
}

fn text<'a>(object: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

fn scalar_text(value: Option<&Value>) -> Option<String> {
    value.and_then(|value| match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    })
}

fn last_path_component(value: &str) -> Option<&str> {
    value
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|part| !part.is_empty())
}
