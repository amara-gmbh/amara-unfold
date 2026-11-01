// SPDX-License-Identifier: AGPL-3.0-or-later

use amara_unfold_core::{AssetRecord, MediaKind, MemoryCandidate};
use chrono::Utc;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownExportResult {
    pub folder: String,
    pub events_written: usize,
    pub events_skipped: usize,
}

pub fn export_package(
    parent: &Path,
    candidate: &MemoryCandidate,
    assets: &[AssetRecord],
) -> Result<MarkdownExportResult, String> {
    let folder_name = format!("amara-unfold-{}", candidate.date_label);
    let destination = available_path(parent, &folder_name);
    let staging = parent.join(format!(".{folder_name}-{}-tmp", std::process::id()));
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(export_error)?;
    }
    let result = export_into(&staging, candidate, assets);
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    fs::rename(&staging, &destination).map_err(export_error)?;
    Ok(MarkdownExportResult {
        folder: destination.to_string_lossy().into_owned(),
        events_written: assets.len(),
        events_skipped: 0,
    })
}

fn export_into(
    destination: &Path,
    candidate: &MemoryCandidate,
    assets: &[AssetRecord],
) -> Result<(), String> {
    let [year, month, day] = date_parts(&candidate.date_label)?;
    let library = destination
        .join("photos")
        .join("library")
        .join(year)
        .join(month)
        .join(day);
    fs::create_dir_all(&library).map_err(export_error)?;
    fs::write(
        destination.join("README.md"),
        readme(candidate, assets.len()),
    )
    .map_err(export_error)?;

    for asset in assets {
        let raw_hash = sha256_file(Path::new(&asset.path))?;
        let source = match asset.media_kind {
            MediaKind::Video => "video",
            _ => "photo",
        };
        let event_id = event_id(asset, &candidate.date_label, source, &raw_hash);
        let stem = safe_stem(&asset.filename);
        let output = library.join(format!("{stem}-{}.md", &event_id[event_id.len() - 6..]));
        fs::write(output, event_markdown(asset, &event_id, source, &raw_hash))
            .map_err(export_error)?;
    }
    Ok(())
}

fn readme(candidate: &MemoryCandidate, event_count: usize) -> String {
    format!(
        "# Amara Unfold metadata export\n\n\
         This is an open, local Markdown archive. It can be read with any text editor and does not require Amara Unfold or Amara Memory. Original media remains at the paths recorded in each event file.\n\n\
         ## Export contents\n\n\
         - Event date: {}\n\
         - Event files: {}\n\
         - Photos: {}\n\
         - Videos: {}\n\
         - Distinct files: {}\n\
         - Capture window: {} to {}\n\
         - Device: {}\n\
         - GPS centre: {}\n\n\
         ## Portability\n\n\
         Each file under `photos/library/YYYY/MM/DD/` follows the Amara Unfold Open Event Schema v1 photo-event shape. Markdown is the portable source material; Amara Memory is an optional destination.\n\n\
         ## Interpretation boundary\n\n\
         Unfold grouped these files from timestamps and metadata. It did not identify the event, people, or meaning of the photos.\n",
        candidate.date_label,
        event_count,
        candidate.photo_count,
        candidate.video_count,
        candidate.distinct_files,
        candidate.started_at.as_deref().unwrap_or("not recorded"),
        candidate.ended_at.as_deref().unwrap_or("not recorded"),
        if candidate.device_labels.is_empty() {
            "not recorded".into()
        } else {
            candidate.device_labels.join(", ")
        },
        match (candidate.latitude, candidate.longitude) {
            (Some(lat), Some(lon)) => format!("{lat:.6}, {lon:.6}"),
            _ => "not recorded".into(),
        }
    )
}

fn event_markdown(asset: &AssetRecord, event_id: &str, source: &str, raw_hash: &str) -> String {
    let date = asset
        .captured_at
        .as_deref()
        .or(asset.modified_at.as_deref())
        .or(asset.created_at.as_deref())
        .and_then(|value| value.get(..10))
        .unwrap_or("0000-00-00");
    let time = asset
        .captured_at
        .as_deref()
        .or(asset.modified_at.as_deref())
        .and_then(|value| value.get(11..16));
    let event_type = if asset.media_kind == MediaKind::Video {
        "video_clip"
    } else {
        "image"
    };
    let camera = device(asset);
    let title = format!("{} ({camera})", asset.filename);
    let place = place_bucket(asset.latitude, asset.longitude);
    let places = place
        .as_deref()
        .map(|value| format!("[{value}]"))
        .unwrap_or_else(|| "[]".into());
    let dimensions = match (asset.width, asset.height) {
        (None, None) => "unknown size".into(),
        (width, height) => format!("{}x{}", optional_number(width), optional_number(height)),
    };
    let coordinates = match (asset.latitude, asset.longitude) {
        (Some(lat), Some(lon)) => format!("{lat:.6}, {lon:.6}"),
        _ => "no GPS".into(),
    };
    let mut lines = vec![
        "---".into(),
        "schema_version: 1".into(),
        format!("id: {event_id}"),
        format!("date: {date}"),
        format!(
            "date_precision: {}",
            if time.is_some() { "minute" } else { "day" }
        ),
        time.map(|value| format!("time: \"{value}\""))
            .unwrap_or_else(|| "time:".into()),
        "end_date:".into(),
        format!("source: {source}"),
        "module: amara-unfold/0.1".into(),
        format!("type: {event_type}"),
        "actors: []".into(),
        format!("places: {places}"),
        format!("title: \"{}\"", yaml_escape(&title)),
        "tags: []".into(),
        "confidence: high".into(),
        "provenance:".into(),
        format!("  raw_path: \"{}\"", yaml_escape(&asset.path)),
        format!("  raw_hash: \"sha256:{raw_hash}\""),
        format!("  ingested_at: \"{}\"", Utc::now().to_rfc3339()),
        "  module_version: \"amara-unfold/0.1\"".into(),
    ];
    if let (Some(latitude), Some(longitude)) = (asset.latitude, asset.longitude) {
        lines.push(format!("gps_lat: {latitude:.6}"));
        lines.push(format!("gps_lon: {longitude:.6}"));
    }
    lines.extend([
        "links: []".into(),
        "revisions: []".into(),
        "---".into(),
        String::new(),
        format!("{dimensions} · {camera} · {coordinates}"),
        String::new(),
        "## Time".into(),
        format!(
            "- CapturedAt: {}",
            asset.captured_at.as_deref().unwrap_or("not recorded")
        ),
        format!(
            "- CreatedAt: {}",
            asset.created_at.as_deref().unwrap_or("not recorded")
        ),
        format!(
            "- ModifiedAt: {}",
            asset.modified_at.as_deref().unwrap_or("not recorded")
        ),
        String::new(),
        "## Camera".into(),
        format!(
            "- Make: {}",
            asset.device_make.as_deref().unwrap_or("not recorded")
        ),
        format!(
            "- Model: {}",
            asset.device_model.as_deref().unwrap_or("not recorded")
        ),
        String::new(),
        "## Image".into(),
        format!("- ImageWidth: {}", optional_number(asset.width)),
        format!("- ImageHeight: {}", optional_number(asset.height)),
        format!("- FileType: {}", asset.extension.to_ascii_uppercase()),
        format!("- FileSize: {}", asset.size_bytes),
        String::new(),
        "## GPS".into(),
        format!("- GPSLatitude: {}", display_decimal(asset.latitude)),
        format!("- GPSLongitude: {}", display_decimal(asset.longitude)),
        String::new(),
        "## Integrity".into(),
        format!("- SHA256: {raw_hash}"),
        format!(
            "- BLAKE3: {}",
            asset.content_hash.as_deref().unwrap_or("not recorded")
        ),
        String::new(),
    ]);
    lines.join("\n")
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path)
        .map_err(|error| format!("could not read {} during export: {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(export_error)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn event_id(asset: &AssetRecord, date: &str, source: &str, raw_hash: &str) -> String {
    let seed = format!("{raw_hash}{}{source}", asset.path);
    let digest = format!("{:x}", Sha256::digest(seed.as_bytes()));
    format!("{date}-{source}-{}", &digest[..12])
}

fn place_bucket(latitude: Option<f64>, longitude: Option<f64>) -> Option<String> {
    let (latitude, longitude) = (latitude?, longitude?);
    let latitude = (latitude * 100.0).round() / 100.0;
    let longitude = (longitude * 100.0).round() / 100.0;
    let digest = format!(
        "{:x}",
        Sha256::digest(format!("{latitude},{longitude}").as_bytes())
    );
    Some(format!("pg-{}", &digest[..6]))
}

fn date_parts(value: &str) -> Result<[&str; 3], String> {
    let mut parts = value.split('-');
    let result = [parts.next(), parts.next(), parts.next()];
    match result {
        [Some(year), Some(month), Some(day)] if parts.next().is_none() => Ok([year, month, day]),
        _ => Err("memory draft has an invalid date".into()),
    }
}

fn available_path(parent: &Path, name: &str) -> PathBuf {
    let initial = parent.join(name);
    if !initial.exists() {
        return initial;
    }
    for suffix in 'A'..='Z' {
        let candidate = parent.join(format!("{name}{suffix}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{name}-{}", Utc::now().format("%H%M%S")))
}

fn safe_stem(filename: &str) -> String {
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("asset");
    let value: String = stem
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect();
    value.trim_matches('-').to_owned()
}

fn device(asset: &AssetRecord) -> String {
    let label = format!(
        "{} {}",
        asset.device_make.as_deref().unwrap_or_default(),
        asset.device_model.as_deref().unwrap_or_default()
    );
    let label = label.trim();
    if label.is_empty() {
        "unknown camera".into()
    } else {
        label.into()
    }
}

fn optional_number(value: Option<u32>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn display_decimal(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.6}"))
        .unwrap_or_else(|| "not recorded".into())
}

fn yaml_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn export_error(error: impl std::fmt::Display) -> String {
    format!("could not write the Markdown archive: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn place_ids_are_deterministic() {
        assert_eq!(
            place_bucket(Some(48.0724), Some(11.2476)),
            place_bucket(Some(48.0724), Some(11.2476))
        );
        assert!(place_bucket(None, Some(11.2476)).is_none());
    }
}
