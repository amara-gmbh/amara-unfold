// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    input,
    model::{date_time_parts, slug, EventInput, OpenEvent},
    record_event, AdapterError, ExtractionProgress, ProgressCallback,
};
use std::{
    collections::HashMap,
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
        lower.ends_with(".csv")
            && (lower.contains("youtube")
                || lower.contains("abos/")
                || lower.contains("subscriptions"))
    })?;
    for blob in blobs {
        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .from_reader(blob.bytes.as_slice());
        let headers = reader
            .headers()
            .map_err(|error| AdapterError::Parse(error.to_string()))?
            .clone();
        let rows = reader
            .records()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| AdapterError::Parse(error.to_string()))?;
        progress.total_hint = Some(rows.len() as u64);
        callback(progress.clone());
        for row in rows {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let fields = headers.iter().zip(row.iter()).collect::<HashMap<_, _>>();
            let title = first(
                &fields,
                &[
                    "Channel title",
                    "Channel Title",
                    "Kanalname",
                    "Kanaltitel",
                    "Title",
                    "Video Title",
                ],
            )
            .or_else(|| row.get(0))
            .unwrap_or("YouTube item");
            let url = first(
                &fields,
                &[
                    "Channel URL",
                    "Channel Url",
                    "Kanal-URL",
                    "URL",
                    "Video URL",
                ],
            )
            .unwrap_or("");
            let date_value = first(
                &fields,
                &["Time", "Date", "Subscription Date", "Abonniert am"],
            )
            .unwrap_or("");
            let (date, time) =
                date_time_parts(date_value).unwrap_or_else(|| ("0000-00-00".into(), None));
            let raw = row.as_slice().as_bytes();
            let event = OpenEvent::new(EventInput {
                date: &date,
                time: time.as_deref(),
                end_date: None,
                source: "web",
                module: "youtube",
                event_type: "subscription",
                title,
                raw_path: &blob.raw_path,
                raw_bytes: raw,
                body: format!("YouTube item: **{title}**\n\nSource URL: {url}"),
                actors: vec![format!("channel-{}", slug(title))],
                places: Vec::new(),
                tags: vec!["youtube".into()],
                confidence: if date == "0000-00-00" {
                    "medium"
                } else {
                    "high"
                },
            });
            record_event(package, &event, progress, callback)?;
        }
    }
    Ok(())
}

fn first<'a>(fields: &HashMap<&str, &'a str>, names: &[&str]) -> Option<&'a str> {
    names
        .iter()
        .find_map(|name| fields.get(name).copied())
        .filter(|value| !value.is_empty())
}
