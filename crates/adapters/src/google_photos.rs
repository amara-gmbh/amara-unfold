// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    input,
    model::{place_id, EventInput, OpenEvent},
    record_event, record_skip, AdapterError, ExtractionProgress, ProgressCallback,
};
use chrono::{TimeZone, Utc};
use serde_json::Value;
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

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
            && (lower.contains("google fotos/") || lower.contains("google photos/"))
    })?;
    progress.total_hint = Some(blobs.len() as u64);
    callback(progress.clone());
    for blob in blobs {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let value: Value = match serde_json::from_slice(&blob.bytes) {
            Ok(value) => value,
            Err(_) => {
                progress.records_seen += 1;
                progress.errors += 1;
                continue;
            }
        };
        let Some(timestamp) = value
            .pointer("/photoTakenTime/timestamp")
            .or_else(|| value.pointer("/creationTime/timestamp"))
            .and_then(Value::as_str)
            .and_then(|value| value.parse::<i64>().ok())
        else {
            progress.records_seen += 1;
            record_skip(progress, "missing_timestamp");
            continue;
        };
        let Some(captured) = Utc.timestamp_opt(timestamp, 0).single() else {
            progress.records_seen += 1;
            progress.errors += 1;
            continue;
        };
        let title = value
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("Google Photos item");
        let latitude = value
            .pointer("/geoData/latitude")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let longitude = value
            .pointer("/geoData/longitude")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let places = (latitude != 0.0 || longitude != 0.0)
            .then(|| place_id(latitude, longitude))
            .into_iter()
            .collect();
        let description = value
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("");
        let body = format!(
            "{}{}{}",
            if description.is_empty() {
                ""
            } else {
                description
            },
            if description.is_empty() { "" } else { "\n\n" },
            if latitude != 0.0 || longitude != 0.0 {
                format!("Coordinates: `{latitude:.6}, {longitude:.6}`")
            } else {
                "No coordinates supplied by Google Photos.".into()
            }
        );
        let event = OpenEvent::new(EventInput {
            date: &captured.format("%Y-%m-%d").to_string(),
            time: Some(&captured.format("%H:%M").to_string()),
            end_date: None,
            source: "photo",
            module: "google-photos",
            event_type: "image",
            title,
            raw_path: &blob.raw_path,
            raw_bytes: &blob.bytes,
            body,
            actors: Vec::new(),
            places,
            tags: vec!["google-photos".into()],
            confidence: "high",
        });
        record_event(package, &event, progress, callback)?;
    }
    Ok(())
}
