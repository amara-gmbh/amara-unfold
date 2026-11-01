// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    model::{date_time_parts, place_id, sha256_file, EventInput, OpenEvent},
    record_event, record_skip, AdapterError, ExtractionProgress, ProgressCallback,
};
use amara_unfold_core::MediaKind;
use amara_unfold_scanner::{ArchiveScanner, LocalArchiveScanner};
use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

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
        progress.message = format!("Reading Immich local media · {files} files · {media} media");
        progress.errors = errors;
        callback(progress.clone());
    };
    let scanner = LocalArchiveScanner::default();
    let media_roots = immich_media_roots(source);
    let mut output = amara_unfold_scanner::ScanOutput::default();
    for root in media_roots {
        let scanned = scanner
            .scan(&root, cancel, &mut scan_progress)
            .map_err(AdapterError::Input)?;
        output.assets.extend(scanned.assets);
        output.errors.extend(scanned.errors);
        output.cancelled |= scanned.cancelled;
        if output.cancelled {
            break;
        }
    }
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
        let timestamp = asset
            .metadata
            .captured_at
            .as_deref()
            .or(asset.created_at.as_deref())
            .or(asset.modified_at.as_deref());
        let Some((date, time)) = timestamp.and_then(date_time_parts) else {
            progress.records_seen += 1;
            record_skip(progress, "missing_timestamp");
            callback(progress.clone());
            continue;
        };
        let places = asset
            .metadata
            .latitude
            .zip(asset.metadata.longitude)
            .map(|(lat, lon)| place_id(lat, lon))
            .into_iter()
            .collect();
        let raw_hash =
            sha256_file(&asset.path).map_err(|error| AdapterError::Input(error.to_string()))?;
        let (event_type, source_token) = if asset.media_kind == MediaKind::Image {
            ("image", "photo")
        } else {
            ("video_clip", "video")
        };
        let device = readable_device(
            asset.metadata.device_make.as_deref(),
            asset.metadata.device_model.as_deref(),
        );
        let body = format!(
            "Immich local file: `{}`\n\nDimensions: {} × {}\n\nDevice: {}\n\nSize: {} bytes\n\nSource path remains local and is preserved only as provenance.",
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
            device,
            asset.size_bytes
        );
        let event = OpenEvent::from_raw_hash(
            EventInput {
                date: &date,
                time: time.as_deref(),
                end_date: None,
                source: source_token,
                module: "immich",
                event_type,
                title: &asset.filename,
                raw_path: &asset.path.to_string_lossy(),
                raw_bytes: &[],
                body,
                actors: Vec::new(),
                places,
                tags: vec!["immich".into()],
                confidence: if asset.metadata.captured_at.is_some() {
                    "high"
                } else {
                    "medium"
                },
            },
            &raw_hash,
        );
        record_event(package, &event, progress, callback)?;
    }
    Ok(())
}

fn immich_media_roots(source: &Path) -> Vec<PathBuf> {
    let upload = source.join("upload");
    let library = source.join("library");
    let roots = [upload, library]
        .into_iter()
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    if roots.is_empty() {
        vec![source.to_path_buf()]
    } else {
        roots
    }
}

fn readable_device(make: Option<&str>, model: Option<&str>) -> String {
    let make = make.unwrap_or("").trim();
    let model = model.unwrap_or("").trim();
    if make.is_empty() && model.is_empty() {
        "not available".into()
    } else {
        [make, model]
            .into_iter()
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::immich_media_roots;

    #[test]
    fn immich_root_narrows_to_user_media_subfolders() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("upload")).unwrap();
        std::fs::create_dir(root.path().join("library")).unwrap();
        std::fs::create_dir(root.path().join("thumbs")).unwrap();
        std::fs::create_dir(root.path().join("encoded-video")).unwrap();
        std::fs::create_dir(root.path().join("backups")).unwrap();

        let roots = immich_media_roots(root.path());
        assert_eq!(roots.len(), 2);
        assert!(roots.iter().any(|path| path.ends_with("upload")));
        assert!(roots.iter().any(|path| path.ends_with("library")));
        assert!(!roots.iter().any(|path| path.ends_with("thumbs")));
        assert!(!roots.iter().any(|path| path.ends_with("encoded-video")));
        assert!(!roots.iter().any(|path| path.ends_with("backups")));
    }

    #[test]
    fn plain_folder_remains_supported() {
        let root = tempfile::tempdir().unwrap();
        let roots = immich_media_roots(root.path());
        assert_eq!(roots, vec![root.path().to_path_buf()]);
    }
}
