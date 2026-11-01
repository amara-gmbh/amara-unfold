// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    input,
    model::{date_time_parts, place_id, slug, EventInput, OpenEvent},
    record_event, AdapterError, ExtractionProgress, ProgressCallback,
};
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
    let blobs = input::matching_blobs(source, is_supported_path)?;
    for blob in blobs {
        let root: Value = serde_json::from_slice(&blob.bytes)
            .map_err(|error| AdapterError::Parse(error.to_string()))?;
        let values = records(root);
        progress.total_hint = Some(progress.total_hint.unwrap_or(0) + values.len() as u64);
        callback(progress.clone());

        for value in values {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let place = SavedPlace::from_value(&value);
            let raw = serde_json::to_vec(&value)
                .map_err(|error| AdapterError::Parse(error.to_string()))?;
            let (date, time) = place
                .published
                .as_deref()
                .or(place.updated.as_deref())
                .and_then(date_time_parts)
                .unwrap_or_else(|| ("0000-00-00".into(), None));
            let coordinates = match (place.latitude, place.longitude) {
                (Some(latitude), Some(longitude)) => {
                    format!("`{latitude:.6}, {longitude:.6}`")
                }
                _ => "not included in export".into(),
            };
            let places = match (place.latitude, place.longitude) {
                (Some(latitude), Some(longitude)) => vec![place_id(latitude, longitude)],
                _ => Vec::new(),
            };
            let mut tags = vec!["google-maps".into(), "saved-place".into()];
            if date == "0000-00-00" {
                tags.push("reference".into());
            }
            if !place.collection.is_empty() {
                tags.push(slug(&place.collection));
            }
            let body = format!(
                "Google Maps saved place\n\nTitle: **{}**\n\nCollection: {}\n\nCoordinates: {}\n\nAddress: {}\n\nCountry code: {}\n\nGoogle Maps URL: {}\n\nPublished: {}\n\nUpdated: {}\n\nNote: {}",
                place.name,
                present(&place.collection),
                coordinates,
                present(&place.address),
                present(&place.country_code),
                present(&place.url),
                present_option(place.published.as_deref()),
                present_option(place.updated.as_deref()),
                present(&place.note),
            );
            let mut event = OpenEvent::new(EventInput {
                date: &date,
                time: time.as_deref(),
                end_date: None,
                source: "location",
                module: "google-maps",
                event_type: "saved-place",
                title: &place.name,
                raw_path: &blob.raw_path,
                raw_bytes: &raw,
                body,
                actors: Vec::new(),
                places,
                tags,
                confidence: if place.latitude.is_some() && place.longitude.is_some() {
                    "high"
                } else {
                    "medium"
                },
            });
            if !place.url.is_empty() {
                event.links.push(place.url.clone());
            }
            record_event(package, &event, progress, callback)?;
        }
    }
    Ok(())
}

#[derive(Debug)]
struct SavedPlace {
    name: String,
    latitude: Option<f64>,
    longitude: Option<f64>,
    collection: String,
    address: String,
    country_code: String,
    url: String,
    published: Option<String>,
    updated: Option<String>,
    note: String,
}

impl SavedPlace {
    fn from_value(value: &Value) -> Self {
        let properties = value.get("properties").unwrap_or(&Value::Null);
        let location = properties.get("Location").unwrap_or(&Value::Null);
        let geo = location.get("Geo Coordinates").unwrap_or(&Value::Null);
        let geometry = value.get("geometry").unwrap_or(&Value::Null);
        let geometry_coordinates = geometry.get("coordinates").and_then(Value::as_array);

        let name = first_string(&[
            value.get("name"),
            value.get("title"),
            properties.get("Title"),
            location.get("Business Name"),
        ])
        .unwrap_or("Saved place")
        .to_owned();
        let latitude = first_number(&[
            value.get("lat"),
            value.get("latitude"),
            geometry_coordinates.and_then(|coordinates| coordinates.get(1)),
            geo.get("Latitude"),
        ]);
        let longitude = first_number(&[
            value.get("lon"),
            value.get("longitude"),
            geometry_coordinates.and_then(|coordinates| coordinates.first()),
            geo.get("Longitude"),
        ]);
        Self {
            name,
            latitude: latitude.filter(|value| value.is_finite() && (-90.0..=90.0).contains(value)),
            longitude: longitude
                .filter(|value| value.is_finite() && (-180.0..=180.0).contains(value)),
            collection: first_string(&[
                value.get("collection"),
                properties.get("Collection"),
                properties.get("List"),
            ])
            .unwrap_or("Saved places")
            .to_owned(),
            address: first_string(&[value.get("address"), location.get("Address")])
                .unwrap_or("")
                .to_owned(),
            country_code: first_string(&[
                value.get("countryCode"),
                value.get("country_code"),
                location.get("Country Code"),
            ])
            .unwrap_or("")
            .to_owned(),
            url: first_string(&[
                value.get("googleMapsUrl"),
                value.get("url"),
                properties.get("Google Maps URL"),
            ])
            .unwrap_or("")
            .to_owned(),
            published: first_string(&[value.get("published"), properties.get("Published")])
                .map(str::to_owned),
            updated: first_string(&[value.get("updated"), properties.get("Updated")])
                .map(str::to_owned),
            note: first_string(&[
                value.get("note"),
                value.get("description"),
                properties.get("Description"),
                properties.get("Comment"),
            ])
            .unwrap_or("")
            .to_owned(),
        }
    }
}

fn records(root: Value) -> Vec<Value> {
    match root {
        Value::Array(values) => values,
        Value::Object(mut object) => object
            .remove("features")
            .and_then(|features| features.as_array().cloned())
            .unwrap_or_else(|| vec![Value::Object(object)]),
        _ => Vec::new(),
    }
}

pub(crate) fn is_supported_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let lower = lower
        .strip_suffix(".geojson")
        .map(|stem| format!("{stem}.json"))
        .unwrap_or(lower);
    lower.ends_with("all-pois.json")
        || lower.ends_with("saved places.json")
        || lower.ends_with("gespeicherte orte.json")
        || (lower.ends_with(".json")
            && (lower.contains("maps (your places)") || lower.contains("maps (meine orte)")))
}

fn first_string<'a>(values: &[Option<&'a Value>]) -> Option<&'a str> {
    values.iter().flatten().find_map(|value| {
        value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    })
}

fn first_number(values: &[Option<&Value>]) -> Option<f64> {
    values.iter().flatten().find_map(|value| {
        value
            .as_f64()
            .or_else(|| value.as_str().and_then(|value| value.trim().parse().ok()))
    })
}

fn present(value: &str) -> &str {
    if value.trim().is_empty() {
        "not included in export"
    } else {
        value
    }
}

fn present_option(value: Option<&str>) -> &str {
    value
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("not included in export")
}
