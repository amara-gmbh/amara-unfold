// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{writer::event_yaml, AdapterError, OpenEvent};
use chrono::{Datelike, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

const REVISION_PREFIX: &str = "manual-date-v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManualDateResult {
    pub event_id: String,
    pub previous_date: String,
    pub date: String,
    pub date_precision: String,
    pub time: Option<String>,
    pub previous_path: String,
    pub event_path: String,
    pub action: String,
}

#[derive(Debug)]
struct EventDocument {
    event: OpenEvent,
    markdown: String,
}

#[derive(Debug, Clone)]
struct DateState {
    date: String,
    precision: String,
    time: Option<String>,
}

pub fn manual_date_is_active(event: &OpenEvent) -> bool {
    has_manual_date_history(event) && event.date != "0000-00-00"
}

pub fn apply_manual_date(
    event_path: &Path,
    precision: &str,
    value: &str,
    time: Option<&str>,
) -> Result<ManualDateResult, AdapterError> {
    let mut document = read_document(event_path)?;
    ensure_editable(&document.event)?;
    let previous = current_state(&document.event);
    let next = normalize_requested_date(precision, value, time)?;
    if previous.date == next.date
        && previous.precision == next.precision
        && previous.time == next.time
    {
        return Err(AdapterError::Input(
            "the selected date is already assigned to this event".into(),
        ));
    }
    let action = if previous.date == "0000-00-00" {
        "set"
    } else {
        "update"
    };
    document
        .event
        .revisions
        .push(revision_line(action, &previous, &next));
    document.event.date = next.date.clone();
    document.event.date_precision = next.precision.clone();
    document.event.time = next.time.clone();
    persist_document(event_path, document, action, previous)
}

pub fn undo_manual_date(event_path: &Path) -> Result<ManualDateResult, AdapterError> {
    let mut document = read_document(event_path)?;
    if !manual_date_is_active(&document.event) {
        return Err(AdapterError::Input(
            "this event has no active manual date assignment to undo".into(),
        ));
    }
    let original = original_state(&document.event)?;
    if original.date != "0000-00-00" || original.precision != "unknown" {
        return Err(AdapterError::Input(
            "manual date history does not originate from a genuinely undated event".into(),
        ));
    }
    let previous = current_state(&document.event);
    document
        .event
        .revisions
        .push(revision_line("undo", &previous, &original));
    document.event.date = original.date.clone();
    document.event.date_precision = original.precision.clone();
    document.event.time = original.time.clone();
    persist_document(event_path, document, "undo", previous)
}

fn ensure_editable(event: &OpenEvent) -> Result<(), AdapterError> {
    if event
        .tags
        .iter()
        .any(|tag| tag.eq_ignore_ascii_case("reference"))
    {
        return Err(AdapterError::Input(
            "timeless reference notes cannot be assigned a Timeline date".into(),
        ));
    }
    if event.date == "0000-00-00" || manual_date_is_active(event) {
        return Ok(());
    }
    Err(AdapterError::Input(
        "only genuinely undated or previously manually dated events can be edited".into(),
    ))
}

fn normalize_requested_date(
    precision: &str,
    value: &str,
    time: Option<&str>,
) -> Result<DateState, AdapterError> {
    let value = value.trim();
    let time = time.map(str::trim).filter(|value| !value.is_empty());
    match precision {
        "day" => {
            let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map_err(|_| AdapterError::Input("enter a valid calendar day".into()))?;
            let normalized_time = time
                .map(|value| {
                    NaiveTime::parse_from_str(value, "%H:%M")
                        .map(|parsed| parsed.format("%H:%M").to_string())
                        .map_err(|_| AdapterError::Input("enter a valid time".into()))
                })
                .transpose()?;
            Ok(DateState {
                date: date.format("%Y-%m-%d").to_string(),
                precision: if normalized_time.is_some() {
                    "minute"
                } else {
                    "day"
                }
                .into(),
                time: normalized_time,
            })
        }
        "month" => {
            if time.is_some() {
                return Err(AdapterError::Input(
                    "time can only be assigned with day precision".into(),
                ));
            }
            let date = NaiveDate::parse_from_str(&format!("{value}-01"), "%Y-%m-%d")
                .map_err(|_| AdapterError::Input("enter a valid calendar month".into()))?;
            Ok(DateState {
                date: format!("{}-{:02}-00", date.year(), date.month()),
                precision: "month".into(),
                time: None,
            })
        }
        "year" => {
            if time.is_some() {
                return Err(AdapterError::Input(
                    "time can only be assigned with day precision".into(),
                ));
            }
            let year = value
                .parse::<i32>()
                .ok()
                .filter(|year| (1..=9999).contains(year))
                .ok_or_else(|| AdapterError::Input("enter a valid four-digit year".into()))?;
            Ok(DateState {
                date: format!("{year:04}-00-00"),
                precision: "year".into(),
                time: None,
            })
        }
        _ => Err(AdapterError::Input(
            "date precision must be day, month, or year".into(),
        )),
    }
}

fn current_state(event: &OpenEvent) -> DateState {
    DateState {
        date: event.date.clone(),
        precision: event.date_precision.clone(),
        time: event.time.clone(),
    }
}

fn has_manual_date_history(event: &OpenEvent) -> bool {
    event
        .revisions
        .iter()
        .any(|revision| revision.starts_with(&format!("{REVISION_PREFIX}|")))
}

fn original_state(event: &OpenEvent) -> Result<DateState, AdapterError> {
    let revision = event
        .revisions
        .iter()
        .find(|revision| revision.starts_with(&format!("{REVISION_PREFIX}|")))
        .ok_or_else(|| AdapterError::Input("manual date history is missing".into()))?;
    let fields = revision.split('|').collect::<Vec<_>>();
    if fields.len() != 9 {
        return Err(AdapterError::Parse(
            "manual date history has an unsupported format".into(),
        ));
    }
    Ok(DateState {
        date: fields[3].to_owned(),
        precision: fields[4].to_owned(),
        time: nonempty(fields[5]),
    })
}

fn revision_line(action: &str, from: &DateState, to: &DateState) -> String {
    format!(
        "{REVISION_PREFIX}|{}|{action}|{}|{}|{}|{}|{}|{}",
        Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        from.date,
        from.precision,
        from.time.as_deref().unwrap_or_default(),
        to.date,
        to.precision,
        to.time.as_deref().unwrap_or_default(),
    )
}

fn nonempty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

fn read_document(path: &Path) -> Result<EventDocument, AdapterError> {
    let content =
        fs::read_to_string(path).map_err(|error| AdapterError::Input(error.to_string()))?;
    let content = content
        .strip_prefix("---\n")
        .ok_or_else(|| AdapterError::Parse("event Markdown has no YAML frontmatter".into()))?;
    let (yaml, markdown) = content
        .split_once("\n---\n")
        .ok_or_else(|| AdapterError::Parse("event Markdown frontmatter is incomplete".into()))?;
    let event = serde_yaml::from_str::<OpenEvent>(yaml)
        .map_err(|error| AdapterError::Parse(error.to_string()))?;
    Ok(EventDocument {
        event,
        markdown: markdown.to_owned(),
    })
}

fn persist_document(
    previous_path: &Path,
    document: EventDocument,
    action: &str,
    previous: DateState,
) -> Result<ManualDateResult, AdapterError> {
    let events_root = previous_path
        .ancestors()
        .find(|path| path.file_name().and_then(|name| name.to_str()) == Some("events"))
        .ok_or_else(|| AdapterError::InvalidPath("event is not inside an events folder".into()))?;
    let (year, month) = document.event.year_month();
    let destination = events_root
        .join(year)
        .join(month)
        .join(document.event.filename());
    let yaml = event_yaml(&document.event)?;
    let content = format!("---\n{yaml}---\n{}", document.markdown);
    write_and_move(previous_path, &destination, &content)?;
    remove_empty_parents(previous_path.parent(), events_root);
    Ok(ManualDateResult {
        event_id: document.event.id,
        previous_date: previous.date,
        date: document.event.date,
        date_precision: document.event.date_precision,
        time: document.event.time,
        previous_path: previous_path.to_string_lossy().into_owned(),
        event_path: destination.to_string_lossy().into_owned(),
        action: action.into(),
    })
}

fn write_and_move(previous: &Path, destination: &Path, content: &str) -> Result<(), AdapterError> {
    let parent = destination
        .parent()
        .ok_or_else(|| AdapterError::InvalidPath(destination.display().to_string()))?;
    fs::create_dir_all(parent).map_err(|error| AdapterError::Output(error.to_string()))?;
    if destination != previous && destination.exists() {
        return Err(AdapterError::Output(format!(
            "corrected event path already exists: {}",
            destination.display()
        )));
    }
    let temporary = destination.with_extension("md.tmp");
    fs::write(&temporary, content).map_err(|error| AdapterError::Output(error.to_string()))?;
    fs::rename(&temporary, destination).map_err(|error| AdapterError::Output(error.to_string()))?;
    if destination != previous {
        if let Err(error) = fs::remove_file(previous) {
            let _ = fs::remove_file(destination);
            return Err(AdapterError::Output(error.to_string()));
        }
    }
    Ok(())
}

fn remove_empty_parents(mut parent: Option<&Path>, events_root: &Path) {
    while let Some(path) = parent {
        if path == events_root || !path.starts_with(events_root) {
            break;
        }
        if fs::remove_dir(path).is_err() {
            break;
        }
        parent = path.parent();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EventInput, OpenEvent};
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn fixture() -> (tempfile::TempDir, PathBuf, Vec<u8>) {
        let root = tempdir().unwrap();
        let raw = root.path().join("raw.eml");
        let raw_bytes = b"Subject: undated\n\nOriginal evidence".to_vec();
        fs::write(&raw, &raw_bytes).unwrap();
        let event = OpenEvent::new(EventInput {
            date: "0000-00-00",
            time: None,
            end_date: None,
            source: "mail",
            module: "test-mail",
            event_type: "message",
            title: "An undated message",
            raw_path: raw.to_str().unwrap(),
            raw_bytes: &raw_bytes,
            body: "Original evidence".into(),
            actors: vec![],
            places: vec![],
            tags: vec![],
            confidence: "high",
        });
        let path = root
            .path()
            .join("package/events/0000/00")
            .join(event.filename());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let yaml = event_yaml(&event).unwrap();
        fs::write(
            &path,
            format!("---\n{yaml}---\n\n# An undated message\n\nOriginal evidence\n"),
        )
        .unwrap();
        (root, path, raw_bytes)
    }

    #[test]
    fn assigns_updates_and_undoes_without_changing_identity_or_evidence() {
        let (root, initial, raw_bytes) = fixture();
        let raw = root.path().join("raw.eml");
        let assigned = apply_manual_date(&initial, "day", "2024-03-02", Some("09:45")).unwrap();
        assert_eq!(assigned.date_precision, "minute");
        assert!(Path::new(&assigned.event_path)
            .parent()
            .unwrap()
            .ends_with(Path::new("events").join("2024").join("03")));
        assert!(!initial.exists());
        let assigned_path = PathBuf::from(&assigned.event_path);
        let assigned_document = read_document(&assigned_path).unwrap();
        assert_eq!(assigned_document.event.id, assigned.event_id);
        assert_eq!(
            assigned_document.markdown,
            "\n# An undated message\n\nOriginal evidence\n"
        );
        assert_eq!(fs::read(&raw).unwrap(), raw_bytes);
        let assigned_timeline =
            crate::build_timeline(&root.path().join("package"), &mut |_, _, _| {}).unwrap();
        assert_eq!(assigned_timeline.dated_events, 1);
        assert_eq!(assigned_timeline.undated_events, 0);

        let updated = apply_manual_date(&assigned_path, "month", "2023-11", None).unwrap();
        assert_eq!(updated.date, "2023-11-00");
        let updated_path = PathBuf::from(&updated.event_path);
        let undone = undo_manual_date(&updated_path).unwrap();
        assert_eq!(undone.date, "0000-00-00");
        assert_eq!(undone.date_precision, "unknown");
        assert!(Path::new(&undone.event_path)
            .parent()
            .unwrap()
            .ends_with(Path::new("events").join("0000").join("00")));
        let final_document = read_document(Path::new(&undone.event_path)).unwrap();
        assert_eq!(final_document.event.id, assigned.event_id);
        assert_eq!(final_document.event.revisions.len(), 3);
        assert_eq!(
            final_document.markdown,
            "\n# An undated message\n\nOriginal evidence\n"
        );
        assert_eq!(fs::read(&raw).unwrap(), raw_bytes);
        let undone_timeline =
            crate::build_timeline(&root.path().join("package"), &mut |_, _, _| {}).unwrap();
        assert_eq!(undone_timeline.dated_events, 0);
        assert_eq!(undone_timeline.undated_events, 1);
    }

    #[test]
    fn supports_year_precision_and_rejects_reference_notes() {
        let (_root, initial, _) = fixture();
        let year = apply_manual_date(&initial, "year", "1998", None).unwrap();
        assert_eq!(year.date, "1998-00-00");
        assert_eq!(year.date_precision, "year");

        let mut document = read_document(Path::new(&year.event_path)).unwrap();
        document.event.tags.push("reference".into());
        let yaml = event_yaml(&document.event).unwrap();
        fs::write(
            &year.event_path,
            format!("---\n{yaml}---\n{}", document.markdown),
        )
        .unwrap();
        let error =
            apply_manual_date(Path::new(&year.event_path), "year", "2000", None).unwrap_err();
        assert!(error.to_string().contains("reference"));
    }
}
