// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    model::{date_time_parts, place_id, sha256_file, EventInput, OpenEvent},
    record_event, AdapterError, ExtractionProgress, ProgressCallback,
};
use amara_unfold_core::MediaKind;
use amara_unfold_scanner::{ArchiveScanner, LocalArchiveScanner};
use std::{path::Path, sync::atomic::AtomicBool};

pub fn extract(
    source: &Path,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    let mut scan_progress = |path: &Path, files: u64, media: u64, errors: u64| {
        progress.total_hint = None;
        progress.current_item = Some(path.to_string_lossy().into_owned());
        progress.message = format!("Reading metadata · {files} files · {media} media");
        progress.errors = errors;
        callback(progress.clone());
    };
    let output = LocalArchiveScanner::default()
        .scan(source, cancel, &mut scan_progress)
        .map_err(AdapterError::Input)?;
    progress.total_hint = Some(
        output
            .assets
            .iter()
            .filter(|asset| asset.media_kind != MediaKind::Unknown)
            .count() as u64,
    );
    progress.errors = output.errors.len() as u64;
    for asset in output
        .assets
        .into_iter()
        .filter(|asset| asset.media_kind != MediaKind::Unknown)
    {
        let capture = asset
            .metadata
            .captured_at
            .as_deref()
            .and_then(date_time_parts);
        let (date, time) = capture
            .clone()
            .unwrap_or_else(|| ("0000-00-00".into(), None));
        let places = asset
            .metadata
            .latitude
            .zip(asset.metadata.longitude)
            .map(|(lat, lon)| place_id(lat, lon))
            .into_iter()
            .collect();
        let raw_hash =
            sha256_file(&asset.path).map_err(|error| AdapterError::Input(error.to_string()))?;
        let event_type = if asset.media_kind == MediaKind::Image {
            "image"
        } else {
            "video_clip"
        };
        let source_token = if asset.media_kind == MediaKind::Image {
            "photo"
        } else {
            "video"
        };
        let body = format!(
            "File: `{}`\n\nDimensions: {} × {}\n\nDevice: {} {}\n\nSize: {} bytes\n\nCapture date: {}",
            asset.filename,
            asset
                .metadata
                .width
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".into()),
            asset
                .metadata
                .height
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".into()),
            asset.metadata.device_make.as_deref().unwrap_or(""),
            asset.metadata.device_model.as_deref().unwrap_or(""),
            asset.size_bytes,
            asset.metadata.captured_at.as_deref().unwrap_or("not included; filesystem dates are not capture evidence")
        );
        let event = OpenEvent::from_raw_hash(
            EventInput {
                date: &date,
                time: time.as_deref(),
                end_date: None,
                source: source_token,
                module: "photos",
                event_type,
                title: &asset.filename,
                raw_path: &asset.path.to_string_lossy(),
                raw_bytes: &[],
                body,
                actors: Vec::new(),
                places,
                tags: Vec::new(),
                confidence: if capture.is_some() { "high" } else { "medium" },
            },
            &raw_hash,
        );
        record_event(package, &event, progress, callback)?;
    }
    Ok(())
}
