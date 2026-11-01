// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    input,
    model::{place_id, slug, EventInput, OpenEvent},
    record_event, AdapterError, ExtractionProgress, ProgressCallback,
};
use std::{
    collections::{BTreeSet, HashMap},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Default)]
struct SavedPlace {
    title: String,
    note: String,
    url: String,
    comment: String,
    latitude: Option<f64>,
    longitude: Option<f64>,
    collections: BTreeSet<String>,
    source_paths: BTreeSet<String>,
}

pub fn extract(
    source: &Path,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    let files = input::matching_blobs(source, |path| path.to_ascii_lowercase().ends_with(".csv"))?;
    let Some(master) = files
        .iter()
        .find(|blob| raw_file_name(&blob.raw_path).eq_ignore_ascii_case("ALL_Locations.csv"))
    else {
        return Ok(());
    };
    let master_parent = raw_parent(&master.raw_path);
    let mut places = HashMap::new();
    ingest_blob(&master.bytes, &master.raw_path, None, &mut places)?;
    for blob in files.iter().filter(|blob| {
        blob.raw_path != master.raw_path && raw_parent(&blob.raw_path) == master_parent
    }) {
        let collection = raw_file_stem(&blob.raw_path);
        ingest_blob(&blob.bytes, &blob.raw_path, Some(&collection), &mut places)?;
    }
    let mut places = places.into_values().collect::<Vec<_>>();
    places.sort_by(|left, right| left.title.cmp(&right.title).then(left.url.cmp(&right.url)));
    progress.total_hint = Some(progress.total_hint.unwrap_or(0) + places.len() as u64);
    callback(progress.clone());
    for place in places {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let title = if place.title.trim().is_empty() {
            "Untitled saved place"
        } else {
            place.title.trim()
        };
        let coordinates = match (place.latitude, place.longitude) {
            (Some(latitude), Some(longitude)) => {
                format!("{latitude:.6}, {longitude:.6}")
            }
            _ => "not included in export".into(),
        };
        let collections = place
            .collections
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let body = format!(
            "Saved Google Maps place\n\nTitle: **{title}**\n\nCollections: {}\n\nCoordinates: `{coordinates}`\n\nNote: {}\n\nComment: {}\n\nGoogle Maps URL: {}",
            if collections.is_empty() { "none" } else { &collections },
            if place.note.is_empty() { "none" } else { &place.note },
            if place.comment.is_empty() { "none" } else { &place.comment },
            if place.url.is_empty() { "not included in export" } else { &place.url }
        );
        let raw = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            place.title, place.note, place.url, place.comment, coordinates, collections
        );
        let master_path = master.raw_path.clone();
        let raw_path = if place.source_paths.contains(&master_path) {
            master_path
        } else {
            place
                .source_paths
                .iter()
                .next()
                .cloned()
                .unwrap_or_else(|| master.raw_path.clone())
        };
        let places = match (place.latitude, place.longitude) {
            (Some(latitude), Some(longitude)) => vec![place_id(latitude, longitude)],
            _ => vec![format!("place-{}", slug(title))],
        };
        let mut tags = vec![
            "google-maps".into(),
            "saved-place".into(),
            "reference".into(),
        ];
        tags.extend(place.collections.iter().map(|value| slug(value)));
        let mut event = OpenEvent::new(EventInput {
            date: "0000-00-00",
            time: None,
            end_date: None,
            source: "location",
            module: "google-maps",
            event_type: "saved-place",
            title,
            raw_path: &raw_path,
            raw_bytes: raw.as_bytes(),
            body,
            actors: Vec::new(),
            places,
            tags,
            confidence: "high",
        });
        if !place.url.is_empty() {
            event.links.push(place.url.clone());
        }
        record_event(package, &event, progress, callback)?;
    }
    Ok(())
}

fn ingest_blob(
    bytes: &[u8],
    raw_path: &str,
    collection: Option<&str>,
    places: &mut HashMap<String, SavedPlace>,
) -> Result<(), AdapterError> {
    let mut reader = csv::ReaderBuilder::new().flexible(true).from_reader(bytes);
    let headers = reader
        .headers()
        .map_err(|error| AdapterError::Parse(error.to_string()))?
        .clone();
    let columns = CsvColumns::from_headers(&headers);
    for row in reader.records() {
        let row = row.map_err(|error| AdapterError::Parse(error.to_string()))?;
        let title = field(&row, columns.title);
        let note = field(&row, columns.note);
        let url = field(&row, columns.url);
        let comment = field(&row, columns.comment);
        if title.is_empty() && url.is_empty() {
            continue;
        }
        let key = if url.is_empty() {
            format!("title:{}", title.to_lowercase())
        } else {
            format!("url:{url}")
        };
        let place = places.entry(key).or_default();
        if place.title.is_empty() {
            place.title = title.to_owned();
        }
        if place.note.is_empty() {
            place.note = note.to_owned();
        }
        if place.url.is_empty() {
            place.url = url.to_owned();
        }
        if place.comment.is_empty() {
            place.comment = comment.to_owned();
        }
        if place.latitude.is_none() {
            place.latitude = columns
                .latitude
                .and_then(|index| field(&row, index).parse().ok());
            place.longitude = columns
                .longitude
                .and_then(|index| field(&row, index).parse().ok());
        }
        if let Some(collection) = collection {
            place.collections.insert(collection.to_owned());
        }
        place.source_paths.insert(raw_path.to_owned());
    }
    Ok(())
}

struct CsvColumns {
    title: usize,
    note: usize,
    url: usize,
    comment: usize,
    latitude: Option<usize>,
    longitude: Option<usize>,
}

impl CsvColumns {
    fn from_headers(headers: &csv::StringRecord) -> Self {
        Self {
            title: find_header(headers, &["title", "titel", "name", "place"]).unwrap_or(0),
            note: find_header(
                headers,
                &["note", "notes", "notiz", "description", "beschreibung"],
            )
            .unwrap_or(1),
            url: find_header(headers, &["url", "google maps url", "maps url", "link"]).unwrap_or(2),
            comment: find_header(headers, &["comment", "comments", "kommentar"]).unwrap_or(3),
            latitude: find_header(headers, &["latitude", "lat", "breitengrad"]),
            longitude: find_header(
                headers,
                &["longitude", "lon", "lng", "längengrad", "laengengrad"],
            ),
        }
    }
}

fn find_header(headers: &csv::StringRecord, aliases: &[&str]) -> Option<usize> {
    headers.iter().position(|header| {
        let normalized = header
            .trim()
            .trim_matches('\u{feff}')
            .to_lowercase()
            .replace(['_', '-'], " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        aliases.iter().any(|alias| normalized == *alias)
    })
}

fn field(row: &csv::StringRecord, index: usize) -> &str {
    row.get(index).unwrap_or("").trim()
}

fn raw_file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn raw_file_stem(path: &str) -> String {
    raw_file_name(path)
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or("Saved places")
        .to_owned()
}

fn raw_parent(path: &str) -> &str {
    path.rsplit_once('/')
        .map(|(parent, _)| parent)
        .unwrap_or("")
}
