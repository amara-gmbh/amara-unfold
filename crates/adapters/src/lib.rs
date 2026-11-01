// SPDX-License-Identifier: AGPL-3.0-or-later

mod airbnb;
mod apple_calendar;
mod apple_contacts;
mod apple_mail;
mod apple_music;
mod apple_music_common;
mod apple_music_local;
mod banking;
mod diary;
mod google_maps;
mod google_photos;
mod immich;
mod input;
mod kmz;
mod mail;
mod manual_date;
mod maps_csv;
mod model;
mod photos;
mod pois;
mod pst_mail;
mod quality;
mod text;
mod timeline;
mod writer;
mod youtube;

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, sync::atomic::AtomicBool};
use thiserror::Error;

pub use manual_date::{
    apply_manual_date, manual_date_is_active, undo_manual_date, ManualDateResult,
};
pub use model::OpenEvent;
pub use quality::QualitySummary;
pub use timeline::{
    build_timeline, build_timeline_cancellable, TimelineBuildResult, TimelineProgressCallback,
};

pub const EXECUTABLE_ADAPTERS: &[&str] = &[
    "photo-media-folder",
    "banking-xlsx",
    "banking-csv",
    "airbnb-json-takeout",
    "diary-docx",
    "gmail-takeout",
    "apple-mail-local",
    "apple-calendar-local",
    "apple-contacts-local",
    "youtube-takeout",
    "google-photos-takeout",
    "google-maps",
    "apple-music-tsv",
    "apple-music-local-library",
];

#[derive(Debug, Clone)]
pub struct ExtractionRequest {
    pub adapter_id: String,
    pub source: PathBuf,
    pub destination: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionProgress {
    pub adapter_id: String,
    pub phase: String,
    pub current_item: Option<String>,
    pub records_seen: u64,
    #[serde(default)]
    pub source_records_included: u64,
    pub events_written: u64,
    pub events_skipped: u64,
    pub errors: u64,
    pub skip_reasons: BTreeMap<String, u64>,
    pub total_hint: Option<u64>,
    pub message: String,
}

impl ExtractionProgress {
    fn starting(adapter_id: &str) -> Self {
        Self {
            adapter_id: adapter_id.into(),
            phase: "extracting".into(),
            current_item: None,
            records_seen: 0,
            source_records_included: 0,
            events_written: 0,
            events_skipped: 0,
            errors: 0,
            skip_reasons: BTreeMap::new(),
            total_hint: None,
            message: "Preparing local extraction".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionResult {
    pub adapter_id: String,
    pub folder: String,
    pub records_seen: u64,
    #[serde(default)]
    pub source_records_included: u64,
    pub events_written: u64,
    pub events_skipped: u64,
    #[serde(default)]
    pub skip_reasons: BTreeMap<String, u64>,
    pub errors: u64,
    pub cancelled: bool,
    pub quality: QualitySummary,
}

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("timeline indexing cancelled; previous complete index preserved")]
    Cancelled,
    #[error("unsupported adapter: {0}")]
    Unsupported(String),
    #[error("invalid local path: {0}")]
    InvalidPath(String),
    #[error("could not read input: {0}")]
    Input(String),
    #[error("could not write open Markdown: {0}")]
    Output(String),
    #[error("could not parse dataset: {0}")]
    Parse(String),
}

pub type ProgressCallback<'a> = dyn FnMut(ExtractionProgress) + 'a;

pub fn extract(
    request: ExtractionRequest,
    cancel: &AtomicBool,
    progress_callback: &mut ProgressCallback<'_>,
) -> Result<ExtractionResult, AdapterError> {
    if !request.source.exists() {
        return Err(AdapterError::InvalidPath(
            request.source.display().to_string(),
        ));
    }
    if !request.destination.is_dir() {
        return Err(AdapterError::InvalidPath(
            request.destination.display().to_string(),
        ));
    }

    if request.source.is_dir() {
        let source = request
            .source
            .canonicalize()
            .map_err(|error| AdapterError::Input(error.to_string()))?;
        let destination = request
            .destination
            .canonicalize()
            .map_err(|error| AdapterError::Output(error.to_string()))?;
        if destination.starts_with(source) {
            return Err(AdapterError::Output("choose an output folder outside the selected source; originals must remain unchanged".into()));
        }
    }
    let mut package =
        writer::PackageWriter::new(&request.destination, &request.source, &request.adapter_id)?;
    let mut progress = ExtractionProgress::starting(&request.adapter_id);
    progress_callback(progress.clone());

    match request.adapter_id.as_str() {
        "photo-media-folder" => photos::extract(
            &request.source,
            &package,
            cancel,
            &mut progress,
            progress_callback,
        )?,
        "banking-xlsx" => banking::extract_workbooks(
            &request.source,
            &package,
            cancel,
            &mut progress,
            progress_callback,
        )?,
        "banking-csv" => banking::extract_csvs(
            &request.source,
            &package,
            cancel,
            &mut progress,
            progress_callback,
        )?,
        "diary-docx" => diary::extract(
            &request.source,
            &package,
            cancel,
            &mut progress,
            progress_callback,
        )?,
        "airbnb-json-takeout" => airbnb::extract(
            &request.source,
            &package,
            cancel,
            &mut progress,
            progress_callback,
        )?,
        "gmail-takeout" => mail::extract(
            &request.source,
            &package,
            cancel,
            &mut progress,
            progress_callback,
        )?,
        "pst-mail" => pst_mail::extract(
            &request.source,
            &package,
            cancel,
            &mut progress,
            progress_callback,
        )?,
        "apple-mail-local" => apple_mail::extract(
            &request.source,
            &package,
            cancel,
            &mut progress,
            progress_callback,
        )?,
        "apple-calendar-local" => apple_calendar::extract(
            &request.source,
            &package,
            cancel,
            &mut progress,
            progress_callback,
        )?,
        "apple-contacts-local" => apple_contacts::extract(
            &request.source,
            &package,
            cancel,
            &mut progress,
            progress_callback,
        )?,
        "youtube-takeout" => youtube::extract(
            &request.source,
            &package,
            cancel,
            &mut progress,
            progress_callback,
        )?,
        "google-photos-takeout" => google_photos::extract(
            &request.source,
            &package,
            cancel,
            &mut progress,
            progress_callback,
        )?,
        "immich-local-library" => immich::extract(
            &request.source,
            &package,
            cancel,
            &mut progress,
            progress_callback,
        )?,
        "google-maps" => google_maps::extract(
            &request.source,
            &package,
            cancel,
            &mut progress,
            progress_callback,
        )?,
        "google-drive-kmz" => kmz::extract(
            &request.source,
            &package,
            cancel,
            &mut progress,
            progress_callback,
        )?,
        "apple-music-tsv" => apple_music::extract(
            &request.source,
            &package,
            cancel,
            &mut progress,
            progress_callback,
        )?,
        "apple-music-local-library" => apple_music_local::extract(
            &request.source,
            &package,
            cancel,
            &mut progress,
            progress_callback,
        )?,
        "google-maps-pois-json" => pois::extract(
            &request.source,
            &package,
            cancel,
            &mut progress,
            progress_callback,
        )?,
        "google-maps-saved-csv" => maps_csv::extract(
            &request.source,
            &package,
            cancel,
            &mut progress,
            progress_callback,
        )?,
        other => return Err(AdapterError::Unsupported(other.into())),
    }

    let quality = package.finish(&progress)?;
    progress.phase = if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        "cancelled".into()
    } else {
        "complete".into()
    };
    if progress.phase != "cancelled" {
        package.commit()?;
    } else {
        progress.events_written = 0;
    }
    progress.current_item = None;
    progress.message = if progress.phase == "cancelled" {
        "Extraction stopped safely; no new files committed and previous exports unchanged".into()
    } else {
        format!(
            "Open Markdown ready: {} written, {} skipped · quality {}",
            progress.events_written,
            progress.events_skipped,
            quality.status_label()
        )
    };
    progress_callback(progress.clone());

    Ok(ExtractionResult {
        adapter_id: request.adapter_id,
        folder: package.final_root().to_string_lossy().into_owned(),
        records_seen: progress.records_seen,
        source_records_included: progress.source_records_included,
        events_written: progress.events_written,
        events_skipped: progress.events_skipped,
        skip_reasons: progress.skip_reasons.clone(),
        errors: progress.errors,
        cancelled: progress.phase == "cancelled",
        quality,
    })
}

pub(crate) fn record_event(
    package: &writer::PackageWriter,
    event: &OpenEvent,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    progress.records_seen += 1;
    match package.write_event(event)? {
        writer::WriteOutcome::Written => progress.events_written += 1,
        writer::WriteOutcome::Unchanged => record_skip(progress, "unchanged_rerun"),
        writer::WriteOutcome::Duplicate => record_skip(progress, "duplicate_event"),
    }
    progress.current_item = Some(event.title.clone());
    progress.message = format!("Normalized {} records", progress.records_seen);
    callback(progress.clone());
    Ok(())
}

pub(crate) fn record_skip(progress: &mut ExtractionProgress, reason: &str) {
    progress.events_skipped += 1;
    *progress.skip_reasons.entry(reason.into()).or_default() += 1;
}

/// Bounded native recognition uses the same transaction-header rules as extraction.
pub fn recognizes_banking_csv(text: &str) -> bool {
    banking::header_delimiter(text).is_some()
}

/// Recognize existing structured Diary DOCX by headings rather than filename.
pub fn recognizes_diary_docx(path: &std::path::Path) -> bool {
    diary::recognizes(path)
}

/// Use the same saved-place filename variants for native recognition and intake.
pub fn recognizes_maps_json_path(path: &str) -> bool {
    pois::is_supported_path(path)
}
