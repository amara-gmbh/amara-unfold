// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::text::decode_text;
use crate::{
    input,
    model::{date_time_parts, place_id, EventInput, OpenEvent},
    record_event, record_skip, AdapterError, ExtractionProgress, ProgressCallback,
};
use std::{
    io::{Cursor, Read},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};
use zip::ZipArchive;

pub fn extract(
    source: &Path,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    let blobs = input::matching_blobs(source, |path| path.to_lowercase().ends_with(".kmz"))?;
    for blob in blobs {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let mut archive = ZipArchive::new(Cursor::new(&blob.bytes))
            .map_err(|error| AdapterError::Parse(error.to_string()))?;
        if archive.len() > 10_000 {
            return Err(AdapterError::Input(format!(
                "{} contains too many nested KMZ entries",
                blob.raw_path
            )));
        }
        let mut kml_total = 0_u64;
        for index in 0..archive.len() {
            let entry = archive
                .by_index(index)
                .map_err(|error| AdapterError::Parse(error.to_string()))?;
            let entry_name = entry
                .name()
                .map_err(|error| AdapterError::Parse(error.to_string()))?
                .into_owned();
            if !entry_name.to_lowercase().ends_with(".kml") {
                continue;
            }
            input::validate_entry_limits(
                &entry_name,
                entry.size(),
                entry.compressed_size(),
                256 * 1024 * 1024,
            )?;
            kml_total = kml_total
                .checked_add(entry.size())
                .ok_or_else(|| AdapterError::Input("KMZ size overflowed".into()))?;
            if kml_total > 512 * 1024 * 1024 {
                return Err(AdapterError::Input(format!(
                    "{} expands beyond the nested KML limit",
                    blob.raw_path
                )));
            }
            let mut xml_bytes = Vec::new();
            entry
                .take(256 * 1024 * 1024 + 1)
                .read_to_end(&mut xml_bytes)
                .map_err(|error| AdapterError::Parse(error.to_string()))?;
            if xml_bytes.len() > 256 * 1024 * 1024 {
                return Err(AdapterError::Input(
                    "nested KML exceeded the read limit".into(),
                ));
            }
            let xml = decode_text(&xml_bytes);
            let document = roxmltree::Document::parse(&xml)
                .map_err(|error| AdapterError::Parse(error.to_string()))?;
            let placemarks = document
                .descendants()
                .filter(|node| node.has_tag_name("Placemark"))
                .collect::<Vec<_>>();
            progress.total_hint = Some(progress.total_hint.unwrap_or(0) + placemarks.len() as u64);
            callback(progress.clone());
            for placemark in placemarks {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let name = child_text(placemark, "name").unwrap_or("Map place");
                let coordinates = placemark
                    .descendants()
                    .find(|node| node.has_tag_name("coordinates"))
                    .and_then(|node| node.text())
                    .unwrap_or("");
                let Some((longitude, latitude)) = coordinate_pair(coordinates) else {
                    progress.records_seen += 1;
                    record_skip(progress, "missing_coordinates");
                    continue;
                };
                let when = placemark
                    .descendants()
                    .find(|node| node.has_tag_name("when"))
                    .and_then(|node| node.text())
                    .unwrap_or("");
                let (date, time) =
                    date_time_parts(when).unwrap_or_else(|| ("0000-00-00".into(), None));
                let raw = format!("{name}\n{coordinates}\n{when}");
                let description = child_text(placemark, "description").unwrap_or("");
                let mut tags = vec!["google-maps".into(), "saved-place".into(), "kmz".into()];
                if date == "0000-00-00" {
                    tags.push("reference".into());
                }
                let mut event = OpenEvent::new(EventInput {
                    date: &date,
                    time: time.as_deref(),
                    end_date: None,
                    source: "location",
                    module: "google-maps",
                    event_type: "saved-place",
                    title: name,
                    raw_path: &blob.raw_path,
                    raw_bytes: raw.as_bytes(),
                    body: format!(
                        "Google Maps place: **{name}**\n\nCoordinates: `{latitude:.6}, {longitude:.6}`\n\n{}",
                        if description.trim().is_empty() {
                            "Description: not included in export"
                        } else {
                            description
                        }
                    ),
                    actors: Vec::new(),
                    places: vec![place_id(latitude, longitude)],
                    tags,
                    confidence: if date == "0000-00-00" { "medium" } else { "high" },
                });
                event.links.extend(google_maps_links(description));
                record_event(package, &event, progress, callback)?;
            }
        }
    }
    Ok(())
}

fn child_text<'a>(node: roxmltree::Node<'a, 'a>, name: &str) -> Option<&'a str> {
    node.children()
        .find(|child| child.has_tag_name(name))
        .and_then(|child| child.text())
}
fn coordinate_pair(value: &str) -> Option<(f64, f64)> {
    let mut parts = value.trim().split(',');
    Some((
        parts.next()?.trim().parse().ok()?,
        parts.next()?.trim().parse().ok()?,
    ))
}

fn google_maps_links(value: &str) -> Vec<String> {
    value
        .split(|character: char| {
            character.is_whitespace() || ['\"', '\'', '<', '>', '(', ')'].contains(&character)
        })
        .map(|part| part.trim_matches(['.', ',', ';']))
        .filter(|part| {
            let lower = part.to_ascii_lowercase();
            (lower.starts_with("http://") || lower.starts_with("https://"))
                && (lower.contains("google.com/maps") || lower.contains("maps.google."))
        })
        .map(str::to_owned)
        .collect()
}
