// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    input,
    model::{slug, EventInput, OpenEvent},
    record_skip, AdapterError, ExtractionProgress, ProgressCallback,
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

const MAX_ICS_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Default)]
struct CalendarEvent {
    uid: String,
    summary: String,
    description: String,
    location: String,
    dtstart: String,
    dtend: String,
    start_timezone: String,
    end_timezone: String,
    organizer: String,
    attendees: Vec<String>,
    recurrence: Vec<String>,
}

pub fn extract(
    source: &Path,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    let files = input::files_with_extensions(source, &["ics"]);
    if files.is_empty() {
        return Err(AdapterError::Input(format!(
            "No readable Apple Calendar .ics files were found in {}. On iCloud/EventKit-based Macs, Calendar may not keep exportable .ics files there; export calendars as .ics or choose a readable .ics folder.",
            source.display()
        )));
    }
    progress.total_hint = Some(files.len() as u64);
    callback(progress.clone());

    for path in files {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        input::validate_file_size(&path, MAX_ICS_BYTES, "Apple Calendar .ics file")?;
        progress.current_item = Some(path.to_string_lossy().into_owned());
        progress.message = format!("Reading Apple Calendar file {}", path.display());
        callback(progress.clone());
        let raw = fs::read(&path).map_err(|error| AdapterError::Input(error.to_string()))?;
        let text = crate::text::decode_text(&raw);
        let events = parse_ics_events(&text);
        progress.total_hint = Some(events.len() as u64);
        callback(progress.clone());
        for (index, item) in events.iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            progress.records_seen += 1;
            let Some((date, time)) = parse_ics_date_time(&item.dtstart) else {
                record_skip(progress, "calendar_event_without_start_date");
                callback(progress.clone());
                continue;
            };
            let end_date = inclusive_end_date(&item.dtend).filter(|end| end >= &date);
            let title = if item.summary.trim().is_empty() {
                "Calendar event"
            } else {
                item.summary.trim()
            };
            let attendees = item
                .attendees
                .iter()
                .filter(|value| !value.trim().is_empty())
                .cloned()
                .collect::<Vec<_>>();
            let mut actors = attendees
                .iter()
                .chain((!item.organizer.is_empty()).then_some(&item.organizer))
                .map(|value| slug(value))
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>();
            actors.sort();
            actors.dedup();
            let mut body = format!(
                "Calendar: Apple Calendar\n\nStart: {}\n\nEnd: {}\n\nSummary: {}\n\nLocation: {}\n\nOrganizer: {}\n\nAttendees: {}\n\nDescription:\n{}",
                calendar_date_label(&item.dtstart, &item.start_timezone, false),
                calendar_date_label(&item.dtend, &item.end_timezone, true),
                if item.summary.is_empty() { "not included" } else { &item.summary },
                if item.location.is_empty() { "not included" } else { &item.location },
                if item.organizer.is_empty() { "not included" } else { &item.organizer },
                if attendees.is_empty() {
                    "not included".into()
                } else {
                    attendees.join(", ")
                },
                if item.description.is_empty() {
                    "not included"
                } else {
                    &item.description
                }
            );
            if !item.recurrence.is_empty() {
                body.push_str("\n\nRecurrence source fields:\n");
                body.push_str(&item.recurrence.join("\n"));
                body.push_str("\n\nRecurring dates are not expanded. Only explicit source events are exported; rules and exceptions above remain source evidence.");
            }
            let identity = if item.uid.trim().is_empty() {
                format!("{}#VEVENT-{}", path.display(), index + 1)
            } else {
                format!("{}#{}", path.display(), item.uid.trim())
            };
            let event = OpenEvent::new_with_identity(
                EventInput {
                    date: &date,
                    time: time.as_deref(),
                    end_date: end_date.as_deref(),
                    source: "calendar",
                    module: "apple-calendar",
                    event_type: "calendar_event",
                    title,
                    raw_path: &format!("{}#VEVENT-{}", path.display(), index + 1),
                    raw_bytes: raw.as_slice(),
                    body,
                    actors,
                    places: Vec::new(),
                    tags: vec!["calendar".into(), "apple-calendar".into()],
                    confidence: "high",
                },
                identity.as_bytes(),
            );
            match package.write_event(&event)? {
                crate::writer::WriteOutcome::Written => progress.events_written += 1,
                crate::writer::WriteOutcome::Unchanged => record_skip(progress, "unchanged_rerun"),
                crate::writer::WriteOutcome::Duplicate => record_skip(progress, "duplicate_event"),
            }
            progress.current_item = Some(title.to_owned());
            progress.message = format!("Normalized {} calendar events", progress.records_seen);
            callback(progress.clone());
        }
    }
    Ok(())
}

fn parse_ics_events(text: &str) -> Vec<CalendarEvent> {
    let lines = unfold_lines(text);
    let mut events = Vec::new();
    let mut active: Option<HashMap<String, Vec<String>>> = None;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("BEGIN:VEVENT") {
            active = Some(HashMap::new());
            continue;
        }
        if trimmed.eq_ignore_ascii_case("END:VEVENT") {
            if let Some(fields) = active.take() {
                events.push(event_from_fields(fields));
            }
            continue;
        }
        let Some(fields) = active.as_mut() else {
            continue;
        };
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let mut parts = key.split(';');
        let key = parts.next().unwrap_or(key).to_ascii_uppercase();
        if matches!(
            key.as_str(),
            "RRULE" | "RDATE" | "EXDATE" | "RECURRENCE-ID" | "EXRULE"
        ) {
            fields
                .entry("RECURRENCE_SOURCE".into())
                .or_default()
                .push(trimmed.to_owned());
        }
        if key == "DTSTART" || key == "DTEND" {
            for parameter in parts {
                if let Some((name, zone)) = parameter.split_once('=') {
                    if name.eq_ignore_ascii_case("TZID") {
                        fields
                            .entry(format!("{key}_TZID"))
                            .or_default()
                            .push(zone.trim_matches('"').to_owned());
                    }
                }
            }
        }
        fields.entry(key).or_default().push(unescape_ics(value));
    }
    events
}

fn event_from_fields(fields: HashMap<String, Vec<String>>) -> CalendarEvent {
    CalendarEvent {
        uid: first(&fields, "UID"),
        summary: first(&fields, "SUMMARY"),
        description: first(&fields, "DESCRIPTION"),
        location: first(&fields, "LOCATION"),
        dtstart: first(&fields, "DTSTART"),
        dtend: first(&fields, "DTEND"),
        start_timezone: first(&fields, "DTSTART_TZID"),
        end_timezone: first(&fields, "DTEND_TZID"),
        organizer: first(&fields, "ORGANIZER"),
        attendees: fields.get("ATTENDEE").cloned().unwrap_or_default(),
        recurrence: fields.get("RECURRENCE_SOURCE").cloned().unwrap_or_default(),
    }
}

fn first(fields: &HashMap<String, Vec<String>>, key: &str) -> String {
    fields
        .get(key)
        .and_then(|values| values.first())
        .cloned()
        .unwrap_or_default()
}

fn unfold_lines(text: &str) -> Vec<String> {
    let mut output: Vec<String> = Vec::new();
    for raw in text.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
        if raw.starts_with(' ') || raw.starts_with('\t') {
            if let Some(last) = output.last_mut() {
                last.push_str(raw.get(1..).unwrap_or_default());
            }
        } else {
            output.push(raw.to_owned());
        }
    }
    output
}

fn unescape_ics(value: &str) -> String {
    let mut output = String::new();
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            match character {
                'n' | 'N' => output.push('\n'),
                ',' => output.push(','),
                ';' => output.push(';'),
                '\\' => output.push('\\'),
                other => output.push(other),
            }
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            output.push(character);
        }
    }
    if escaped {
        output.push('\\');
    }
    output
}

// Timeline ranges include their last day; iCalendar DTEND is exclusive.
fn inclusive_end_date(value: &str) -> Option<String> {
    let (date, time) = parse_ics_date_time(value)?;
    let date = NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok()?;
    if time.is_none() || time.as_deref() == Some("00:00") {
        // Only exact midnight excludes the end day, not 00:00:30.
        if time.is_none()
            || value.trim_end_matches('Z').ends_with("T000000")
            || value.ends_with("T0000")
        {
            return date.pred_opt().map(|date| date.to_string());
        }
    }
    Some(date.to_string())
}

fn calendar_date_label(value: &str, timezone: &str, exclusive: bool) -> String {
    let Some((date, time)) = parse_ics_date_time(value) else {
        return if value.is_empty() {
            "not included".into()
        } else {
            format!("unrecognized source value: {value}")
        };
    };
    let suffix = if exclusive { "; exclusive end" } else { "" };
    if let Some(time) = time {
        let zone = if value.ends_with('Z') {
            "UTC"
        } else if timezone.is_empty() {
            "floating local time; timezone not included"
        } else {
            timezone
        };
        format!("{date} {time} ({zone}{suffix}) · source: {value}")
    } else {
        format!("{date} (all-day date{suffix})")
    }
}

fn parse_ics_date_time(value: &str) -> Option<(String, Option<String>)> {
    let value = value.trim();
    if value.len() == 8 {
        return NaiveDate::parse_from_str(value, "%Y%m%d")
            .ok()
            .map(|date| (date.to_string(), None));
    }
    if let Some(utc_value) = value.strip_suffix('Z') {
        if let Ok(date_time) = NaiveDateTime::parse_from_str(utc_value, "%Y%m%dT%H%M%S") {
            let utc = DateTime::<Utc>::from_naive_utc_and_offset(date_time, Utc);
            return Some((
                utc.date_naive().to_string(),
                Some(utc.format("%H:%M").to_string()),
            ));
        }
    }
    for format in ["%Y%m%dT%H%M%S", "%Y%m%dT%H%M"] {
        if let Ok(date_time) = NaiveDateTime::parse_from_str(value, format) {
            return Some((
                date_time.date().to_string(),
                Some(date_time.format("%H:%M").to_string()),
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_folded_escaped_calendar_events() {
        let events = parse_ics_events(
            "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:one\r\nDTSTART:20240721T103000Z\r\nSUMMARY:Coffee\\, walk\\nand notes\r\nDESCRIPTION:Line one\\n line two\r\nATTENDEE:mailto:a@example.test\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
        );

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].summary, "Coffee, walk\nand notes");
        assert_eq!(events[0].description, "Line one\n line two");
        assert_eq!(
            parse_ics_date_time(&events[0].dtstart).unwrap().0,
            "2024-07-21"
        );
    }
}
