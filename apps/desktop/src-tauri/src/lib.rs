// SPDX-License-Identifier: AGPL-3.0-or-later

use amara_unfold_adapters::{
    apply_manual_date, manual_date_is_active, undo_manual_date, ExtractionProgress,
    ExtractionRequest, ExtractionResult, ManualDateResult, OpenEvent, TimelineBuildResult,
};
use amara_unfold_core::{
    privacy::{NetworkGuard, PrivacyStatus},
    DashboardSnapshot, IndexRepository, ScanProgress, ScanResult,
};
use amara_unfold_db::SqliteIndex;
use amara_unfold_memory::{CandidateEngine, DateClusterEngine};
use amara_unfold_scanner::{ArchiveScanner, LocalArchiveScanner};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::{Emitter, Manager, State};

mod dataset_recognition;
mod local_app_sources;
mod markdown_export;

#[derive(Clone)]
struct AppState {
    database: Arc<Mutex<SqliteIndex>>,
    cancel_scan: Arc<AtomicBool>,
    scan_running: Arc<AtomicBool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalAccessReleaseResult {
    adapter_id: String,
    attempted: bool,
    status: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct TimelineCatalogRow {
    id: String,
    date: String,
    source: String,
    module: String,
    #[serde(rename = "type")]
    event_type: String,
    title: String,
    event_path: String,
    raw_path: String,
    review_status: String,
    #[serde(default)]
    review_reasons: String,
    #[serde(default)]
    tags: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimelineCatalogEntry {
    id: String,
    date: String,
    date_precision: String,
    time: Option<String>,
    manual_date: bool,
    source: String,
    module: String,
    event_type: String,
    title: String,
    preview: String,
    timeline_folder: String,
    event_path: String,
    raw_path: String,
    review_status: String,
    review_reasons: Vec<String>,
    tags: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimelineEventContent {
    body: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimelineDateEditResult {
    timeline: TimelineBuildResult,
    correction: ManualDateResult,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TimelineManifestFile {
    kind: String,
    generated_at: String,
    events_indexed: u64,
    duplicate_events: u64,
    rejected_events: u64,
    day_pages: u64,
    month_pages: u64,
    entity_anchors: u64,
    cross_source_days: u64,
    strict_portable: bool,
}

#[tauri::command]
fn get_dashboard(state: State<'_, AppState>) -> Result<DashboardSnapshot, String> {
    state
        .database
        .lock()
        .map_err(|_| "local index lock was poisoned".to_owned())?
        .dashboard()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn privacy_status() -> PrivacyStatus {
    NetworkGuard.status()
}

#[tauri::command]
fn local_app_sources(
    include_user_locations: Option<bool>,
) -> Vec<local_app_sources::LocalAppSource> {
    local_app_sources::list(include_user_locations.unwrap_or(false))
}

#[tauri::command]
fn open_apple_mail() -> Result<String, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "could not determine the current user's home folder".to_owned())?;
    let export_folder = home.join("Desktop").join("Amara Unfold Mail Exports");
    fs::create_dir_all(&export_folder).map_err(|error| {
        format!(
            "could not create Mail export folder at {}: {error}",
            export_folder.display()
        )
    })?;
    let instructions_path = export_folder.join("READ ME - Apple Mail Export.txt");
    fs::write(
        &instructions_path,
        "Apple Mail export for Amara Unfold\n\
         \n\
         1. In Apple Mail, select the mailbox you want to unfold.\n\
         2. In the menu bar, choose Mailbox > Export Mailbox...\n\
         3. Save the exported folder here. It should end with .mbox.\n\
         4. Return to Amara Unfold. The Mail card should show Export found;\n\
            otherwise click Refresh app exports or Choose Mail export.\n\
         \n\
         Privacy note: Amara Unfold reads only the exported .mbox folder you choose.\n\
         It does not need Full Disk Access, does not read Mail's private live store,\n\
         and keeps no ongoing Apple Mail permission.\n",
    )
    .map_err(|error| {
        format!(
            "could not write Mail export instructions at {}: {error}",
            instructions_path.display()
        )
    })?;
    let _ = Command::new("open").arg(&export_folder).status();
    let status = Command::new("open")
        .args(["-a", "Mail"])
        .status()
        .map_err(|error| format!("could not open Apple Mail: {error}"))?;
    if status.success() {
        Ok(export_folder.to_string_lossy().into_owned())
    } else {
        Err("macOS could not open Apple Mail".into())
    }
}

#[tauri::command]
fn release_local_app_access(app: tauri::AppHandle, adapter_id: String) -> LocalAccessReleaseResult {
    if adapter_id != "apple-music-local-library" {
        return LocalAccessReleaseResult {
            adapter_id,
            attempted: false,
            status: "not_needed".into(),
            message: "This source uses a file/folder export and keeps no app Automation approval."
                .into(),
        };
    }

    let bundle_identifier = app.config().identifier.clone();
    match Command::new("tccutil")
        .args(["reset", "AppleEvents", &bundle_identifier])
        .output()
    {
        Ok(output) if output.status.success() => LocalAccessReleaseResult {
            adapter_id,
            attempted: true,
            status: "released".into(),
            message: "Music Automation approval was released after this export; macOS should ask again next time."
                .into(),
        },
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            let detail = if !stderr.is_empty() {
                stderr
            } else if !stdout.is_empty() {
                stdout
            } else {
                format!("tccutil exited with status {}", output.status)
            };
            LocalAccessReleaseResult {
                adapter_id,
                attempted: true,
                status: "failed".into(),
                message: format!(
                    "Unfold finished the one-time Music read, but macOS did not confirm Automation reset ({detail}). Revoke it manually in System Settings › Privacy & Security › Automation."
                ),
            }
        }
        Err(error) => LocalAccessReleaseResult {
            adapter_id,
            attempted: true,
            status: "unavailable".into(),
            message: format!(
                "Unfold finished the one-time Music read, but could not ask macOS to reset Automation approval ({error}). Revoke it manually in System Settings › Privacy & Security › Automation."
            ),
        },
    }
}

#[tauri::command]
async fn inspect_dataset(path: String) -> Result<dataset_recognition::DatasetInspection, String> {
    tauri::async_runtime::spawn_blocking(move || dataset_recognition::inspect(PathBuf::from(path)))
        .await
        .map_err(|error| format!("dataset inspection stopped unexpectedly: {error}"))?
}

#[tauri::command]
async fn extract_dataset(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    adapter_id: String,
    source: String,
    destination: String,
) -> Result<ExtractionResult, String> {
    if state
        .scan_running
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
        .is_err()
    {
        return Err("another local extraction is already running".into());
    }
    let source = PathBuf::from(source);
    let destination = PathBuf::from(destination);
    if !source.exists() || !destination.is_dir() {
        state.scan_running.store(false, Ordering::Release);
        return Err("choose a readable source and writable local output folder".into());
    }
    state.cancel_scan.store(false, Ordering::Relaxed);
    let cancel = state.cancel_scan.clone();
    let running = state.scan_running.clone();
    let worker_result = tauri::async_runtime::spawn_blocking(move || {
        amara_unfold_adapters::extract(
            ExtractionRequest {
                adapter_id,
                source,
                destination,
            },
            &cancel,
            &mut |progress: ExtractionProgress| {
                let _ = app.emit("extraction-progress", progress);
            },
        )
        .map_err(|error| error.to_string())
    })
    .await;
    running.store(false, Ordering::Release);
    worker_result.map_err(|error| format!("adapter worker stopped unexpectedly: {error}"))?
}

#[tauri::command]
async fn build_timeline_index(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    destination: String,
) -> Result<TimelineBuildResult, String> {
    if state
        .scan_running
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
        .is_err()
    {
        return Err("another local extraction is already running".into());
    }
    let destination = PathBuf::from(destination);
    if !destination.is_dir() {
        state.scan_running.store(false, Ordering::Release);
        return Err("choose a writable local output folder".into());
    }
    let cancel = state.cancel_scan.clone();
    let running = state.scan_running.clone();
    let worker_result = tauri::async_runtime::spawn_blocking(move || {
        amara_unfold_adapters::build_timeline_cancellable(
            &destination,
            &cancel,
            &mut |seen, total, message| {
                let _ = app.emit(
                    "extraction-progress",
                    ExtractionProgress {
                        adapter_id: "timeline-index".into(),
                        phase: if seen == total && message.contains("ready") {
                            "complete".into()
                        } else {
                            "indexing".into()
                        },
                        current_item: None,
                        records_seen: seen,
                        source_records_included: 0,
                        events_written: seen,
                        events_skipped: 0,
                        errors: 0,
                        skip_reasons: Default::default(),
                        total_hint: Some(total),
                        message: message.into(),
                    },
                );
            },
        )
        .map_err(|error| error.to_string())
    })
    .await;
    running.store(false, Ordering::Release);
    worker_result.map_err(|error| format!("timeline worker stopped unexpectedly: {error}"))?
}

#[tauri::command]
async fn read_timeline_catalog(folder: String) -> Result<Vec<TimelineCatalogEntry>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let folder = verified_timeline_folder(&folder)?;
        let mut reader = csv::Reader::from_path(folder.join("catalog.csv"))
            .map_err(|error| format!("could not read the timeline catalog: {error}"))?;
        reader
            .deserialize::<TimelineCatalogRow>()
            .map(|row| {
                let row = row.map_err(|error| format!("invalid timeline catalog row: {error}"))?;
                let event_file = resolve_timeline_event(&folder, &row.event_path).ok();
                let metadata = event_file
                    .as_deref()
                    .and_then(|path| read_event_frontmatter(path).ok());
                Ok(TimelineCatalogEntry {
                    id: row.id,
                    date: row.date,
                    date_precision: metadata
                        .as_ref()
                        .map(|event| event.date_precision.clone())
                        .unwrap_or_else(|| "unknown".into()),
                    time: metadata.as_ref().and_then(|event| event.time.clone()),
                    manual_date: metadata.as_ref().is_some_and(manual_date_is_active),
                    source: row.source,
                    module: row.module,
                    event_type: row.event_type,
                    title: row.title,
                    preview: event_file
                        .as_deref()
                        .ok_or_else(|| "event Markdown is unavailable".to_owned())
                        .and_then(read_markdown_body)
                        .map(|body| event_preview(&body, 240))
                        .unwrap_or_default(),
                    timeline_folder: folder.to_string_lossy().into_owned(),
                    event_path: row.event_path,
                    raw_path: row.raw_path,
                    review_status: row.review_status,
                    review_reasons: row
                        .review_reasons
                        .split(';')
                        .map(str::trim)
                        .filter(|reason| !reason.is_empty())
                        .map(str::to_owned)
                        .collect(),
                    tags: row
                        .tags
                        .split(';')
                        .map(str::trim)
                        .filter(|tag| !tag.is_empty())
                        .map(str::to_owned)
                        .collect(),
                })
            })
            .collect()
    })
    .await
    .map_err(|error| format!("timeline catalog reader stopped unexpectedly: {error}"))?
}

#[tauri::command]
async fn set_timeline_event_date(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    folder: String,
    event_path: String,
    precision: String,
    value: String,
    time: Option<String>,
) -> Result<TimelineDateEditResult, String> {
    edit_timeline_event_date(
        app,
        state,
        folder,
        event_path,
        Some((precision, value, time)),
    )
    .await
}

#[tauri::command]
async fn undo_timeline_event_date(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    folder: String,
    event_path: String,
) -> Result<TimelineDateEditResult, String> {
    edit_timeline_event_date(app, state, folder, event_path, None).await
}

async fn edit_timeline_event_date(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    folder: String,
    event_path: String,
    assignment: Option<(String, String, Option<String>)>,
) -> Result<TimelineDateEditResult, String> {
    if state
        .scan_running
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
        .is_err()
    {
        return Err("another local extraction is already running".into());
    }
    let running = state.scan_running.clone();
    let worker_result = tauri::async_runtime::spawn_blocking(move || {
        let timeline_folder = verified_timeline_folder(&folder)?;
        let export_root = timeline_folder
            .parent()
            .ok_or_else(|| "the timeline has no export root".to_owned())?
            .to_path_buf();
        let event = resolve_timeline_event(&timeline_folder, &event_path)?;
        let correction = if let Some((precision, value, time)) = assignment {
            apply_manual_date(&event, &precision, &value, time.as_deref())
        } else {
            undo_manual_date(&event)
        }
        .map_err(|error| error.to_string())?;
        let timeline =
            amara_unfold_adapters::build_timeline(&export_root, &mut |seen, total, message| {
                let _ = app.emit(
                    "extraction-progress",
                    ExtractionProgress {
                        adapter_id: "timeline-index".into(),
                        phase: if seen == total && message.contains("ready") {
                            "complete".into()
                        } else {
                            "indexing".into()
                        },
                        current_item: None,
                        records_seen: seen,
                        source_records_included: 0,
                        events_written: seen,
                        events_skipped: 0,
                        errors: 0,
                        skip_reasons: Default::default(),
                        total_hint: Some(total),
                        message: message.into(),
                    },
                );
            })
            .map_err(|error| error.to_string())?;
        Ok(TimelineDateEditResult {
            timeline,
            correction,
        })
    })
    .await;
    running.store(false, Ordering::Release);
    worker_result
        .map_err(|error| format!("timeline correction worker stopped unexpectedly: {error}"))?
}

#[tauri::command]
async fn discover_timeline_archives(folder: String) -> Result<Vec<TimelineBuildResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let primary = verified_timeline_folder(&folder)?;
        Ok(discover_timeline_archives_from(&primary))
    })
    .await
    .map_err(|error| format!("timeline archive discovery stopped unexpectedly: {error}"))?
}

#[tauri::command]
async fn read_timeline_event(
    folder: String,
    event_path: String,
) -> Result<TimelineEventContent, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let folder = verified_timeline_folder(&folder)?;
        let event = resolve_timeline_event(&folder, &event_path)?;
        Ok(TimelineEventContent {
            body: read_markdown_body(&event)?,
        })
    })
    .await
    .map_err(|error| format!("timeline event reader stopped unexpectedly: {error}"))?
}

fn verified_timeline_folder(folder: &str) -> Result<PathBuf, String> {
    let folder = PathBuf::from(folder);
    if !folder.is_absolute() || !folder.is_dir() {
        return Err("the saved timeline folder is no longer available".into());
    }
    let folder = folder
        .canonicalize()
        .map_err(|error| format!("could not resolve the timeline folder: {error}"))?;
    let manifest = fs::read_to_string(folder.join("manifest.yaml"))
        .map_err(|error| format!("could not read the timeline manifest: {error}"))?;
    if !manifest.contains("kind: amara-unfold-timeline-index") {
        return Err("the selected folder is not an Amara Unfold timeline".into());
    }
    Ok(folder)
}

fn timeline_archive_candidates(primary: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![primary.to_path_buf()];
    let Some(collection) = primary.parent().and_then(Path::parent) else {
        return candidates;
    };
    let Ok(children) = fs::read_dir(collection) else {
        return candidates;
    };
    for child in children.filter_map(Result::ok).take(128) {
        let candidate = child.path().join("Amara-Unfold-Timeline");
        if candidate.is_dir() {
            candidates.push(candidate);
        }
    }
    candidates.sort();
    candidates.dedup();
    candidates
}

fn discover_timeline_archives_from(primary: &Path) -> Vec<TimelineBuildResult> {
    let mut archives = timeline_archive_candidates(primary)
        .into_iter()
        .filter_map(|candidate| read_timeline_archive(&candidate).ok())
        .collect::<Vec<_>>();
    archives.sort_by(|left, right| right.1.cmp(&left.1));

    let mut signatures = BTreeSet::new();
    let mut results = Vec::new();
    for (result, _, signature) in archives {
        if signatures.insert(signature) {
            results.push(result);
        }
    }
    results
}

fn read_timeline_archive(folder: &Path) -> Result<(TimelineBuildResult, String, String), String> {
    let folder = verified_timeline_folder(&folder.to_string_lossy())?;
    let manifest_text = fs::read_to_string(folder.join("manifest.yaml"))
        .map_err(|error| format!("could not read the timeline manifest: {error}"))?;
    let manifest: TimelineManifestFile = serde_yaml::from_str(&manifest_text)
        .map_err(|error| format!("invalid timeline manifest: {error}"))?;
    if manifest.kind != "amara-unfold-timeline-index" {
        return Err("the selected folder is not an Amara Unfold timeline".into());
    }

    let mut reader = csv::Reader::from_path(folder.join("catalog.csv"))
        .map_err(|error| format!("could not read the timeline catalog: {error}"))?;
    let mut hasher = Sha256::new();
    let mut dated_events = 0_u64;
    let mut undated_events = 0_u64;
    let mut rows = 0_u64;
    for row in reader.deserialize::<TimelineCatalogRow>() {
        let row = row.map_err(|error| format!("invalid timeline catalog row: {error}"))?;
        hasher.update(row.id.as_bytes());
        hasher.update([0]);
        let is_reference = row
            .tags
            .split(';')
            .any(|tag| tag.trim().eq_ignore_ascii_case("reference"));
        if row.date.starts_with("0000-") && !is_reference {
            undated_events += 1;
        } else if !row.date.starts_with("0000-") {
            dated_events += 1;
        }
        rows += 1;
    }
    if rows != manifest.events_indexed {
        return Err(format!(
            "timeline catalog contains {rows} rows but its manifest records {}",
            manifest.events_indexed
        ));
    }

    Ok((
        TimelineBuildResult {
            folder: folder.to_string_lossy().into_owned(),
            events_indexed: manifest.events_indexed,
            duplicate_events: manifest.duplicate_events,
            rejected_events: manifest.rejected_events,
            dated_events,
            undated_events,
            day_pages: manifest.day_pages,
            month_pages: manifest.month_pages,
            entity_anchors: manifest.entity_anchors,
            cross_source_days: manifest.cross_source_days,
            strict_portable: manifest.strict_portable,
        },
        manifest.generated_at,
        format!("{:x}", hasher.finalize()),
    ))
}

fn resolve_timeline_event(folder: &Path, event_path: &str) -> Result<PathBuf, String> {
    let relative = Path::new(event_path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || !relative
            .extension()
            .is_some_and(|extension| extension == "md")
    {
        return Err("the event path is not a safe relative Markdown path".into());
    }
    let export_root = folder
        .parent()
        .ok_or_else(|| "the timeline has no export root".to_owned())?
        .canonicalize()
        .map_err(|error| format!("could not resolve the export root: {error}"))?;
    let event = export_root
        .join(relative)
        .canonicalize()
        .map_err(|error| format!("could not resolve the event Markdown: {error}"))?;
    if !event.starts_with(&export_root) || !event.is_file() {
        return Err("the event Markdown is outside the export root".into());
    }
    Ok(event)
}

fn read_markdown_body(path: &Path) -> Result<String, String> {
    const MAX_EVENT_BYTES: u64 = 4 * 1024 * 1024;
    let metadata =
        fs::metadata(path).map_err(|error| format!("could not inspect event Markdown: {error}"))?;
    if metadata.len() > MAX_EVENT_BYTES {
        return Err("the event Markdown is larger than the 4 MiB display limit".into());
    }
    let markdown = fs::read_to_string(path)
        .map_err(|error| format!("could not read event Markdown: {error}"))?;
    Ok(markdown_body(&markdown).trim().to_owned())
}

fn read_event_frontmatter(path: &Path) -> Result<OpenEvent, String> {
    let markdown = fs::read_to_string(path)
        .map_err(|error| format!("could not read event Markdown: {error}"))?;
    let yaml = markdown
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n").map(|(yaml, _)| yaml))
        .ok_or_else(|| "event Markdown has no valid YAML frontmatter".to_owned())?;
    serde_yaml::from_str(yaml).map_err(|error| format!("invalid event frontmatter: {error}"))
}

fn markdown_body(markdown: &str) -> &str {
    markdown
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n").map(|(_, body)| body))
        .unwrap_or(markdown)
}

fn event_preview(body: &str, limit: usize) -> String {
    let flattened = body
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with("# ") {
                return None;
            }
            let line = line.trim_start_matches('#').trim_start();
            let line = line
                .strip_prefix("- ")
                .or_else(|| line.strip_prefix("* "))
                .or_else(|| line.strip_prefix("> "))
                .unwrap_or(line);
            let mut line = line.replace('`', "");
            if !line.ends_with(['.', '!', '?', ':', ';']) {
                line.push('.');
            }
            Some(line)
        })
        .collect::<Vec<_>>()
        .join(" ");
    let mut chars = flattened.chars();
    let preview = chars.by_ref().take(limit).collect::<String>();
    if chars.next().is_some() {
        format!("{preview}…")
    } else {
        preview
    }
}

#[tauri::command]
async fn export_memory_markdown(
    state: State<'_, AppState>,
    candidate_id: i64,
    folder: String,
) -> Result<markdown_export::MarkdownExportResult, String> {
    let destination = PathBuf::from(folder);
    if !destination.is_absolute() || !destination.is_dir() {
        return Err("choose a writable local folder for the Markdown archive".into());
    }
    let (candidate, assets) = {
        let database = state
            .database
            .lock()
            .map_err(|_| "local index lock was poisoned".to_owned())?;
        let candidate = database
            .dashboard()
            .map_err(|error| error.to_string())?
            .candidates
            .into_iter()
            .find(|candidate| candidate.id == Some(candidate_id))
            .ok_or_else(|| "that memory draft is no longer in the local index".to_owned())?;
        let assets = database
            .candidate_assets(candidate_id)
            .map_err(|error| error.to_string())?;
        (candidate, assets)
    };
    tauri::async_runtime::spawn_blocking(move || {
        markdown_export::export_package(&destination, &candidate, &assets)
    })
    .await
    .map_err(|error| format!("Markdown export stopped unexpectedly: {error}"))?
}

#[tauri::command]
fn cancel_scan(state: State<'_, AppState>) {
    state.cancel_scan.store(true, Ordering::Relaxed);
}

#[tauri::command]
async fn remove_source(
    state: State<'_, AppState>,
    source_id: i64,
) -> Result<DashboardSnapshot, String> {
    if state
        .scan_running
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
        .is_err()
    {
        return Err("wait for the current scan to finish before removing a source".into());
    }

    let database = state.database.clone();
    let worker_result = tauri::async_runtime::spawn_blocking(move || {
        let mut database = database
            .lock()
            .map_err(|_| "local index lock was poisoned".to_owned())?;
        let removed = database
            .remove_source(source_id)
            .map_err(|error| error.to_string())?;
        if !removed {
            return Err("that source is no longer in the local index".into());
        }
        database.dashboard().map_err(|error| error.to_string())
    })
    .await;

    state.scan_running.store(false, Ordering::Release);
    worker_result.map_err(|error| format!("source removal stopped unexpectedly: {error}"))?
}

#[tauri::command]
async fn scan_folder(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    folder: String,
) -> Result<ScanResult, String> {
    if state
        .scan_running
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
        .is_err()
    {
        return Err("a scan is already running".into());
    }

    let source_path = canonical_source(&folder).inspect_err(|_| {
        state.scan_running.store(false, Ordering::Release);
    })?;
    let source_text = source_path.to_string_lossy().into_owned();
    state.cancel_scan.store(false, Ordering::Relaxed);

    let scan_record = state
        .database
        .lock()
        .map_err(|_| "local index lock was poisoned".to_owned())
        .and_then(|mut database| {
            database
                .begin_scan(&source_text)
                .map_err(|error| error.to_string())
        });
    let (source_id, scan_id) = match scan_record {
        Ok(record) => record,
        Err(error) => {
            state.scan_running.store(false, Ordering::Release);
            return Err(error);
        }
    };

    let cancel = state.cancel_scan.clone();
    let database = state.database.clone();
    let running = state.scan_running.clone();
    let scan_app = app.clone();
    let worker_result = tauri::async_runtime::spawn_blocking(move || {
        let scanner = LocalArchiveScanner::default();
        let mut last_emitted = 0_u64;
        let mut progress =
            |path: &std::path::Path, files_seen: u64, media_found: u64, errors: u64| {
                // Emitting every file overwhelms the WebView on large archives.
                // The first item and regular batches still provide responsive UI.
                if files_seen == 1 || files_seen >= last_emitted + 20 {
                    last_emitted = files_seen;
                    let _ = scan_app.emit(
                        "scan-progress",
                        ScanProgress {
                            phase: "scanning".into(),
                            current_path: Some(path.to_string_lossy().into_owned()),
                            files_seen,
                            media_found,
                            errors,
                            message: format!("Extracting metadata from {files_seen} files"),
                        },
                    );
                }
            };
        let output = match scanner.scan(&source_path, &cancel, &mut progress) {
            Ok(output) => output,
            Err(error) => {
                database
                    .lock()
                    .map_err(|_| "local index lock was poisoned".to_owned())?
                    .fail_scan(source_id, scan_id, &error)
                    .map_err(|storage_error| storage_error.to_string())?;
                return Err(error);
            }
        };
        let candidates = DateClusterEngine.build_candidates(&output.assets);
        let counts = database
            .lock()
            .map_err(|_| "local index lock was poisoned".to_owned())?
            .finish_scan(
                source_id,
                scan_id,
                &output.assets,
                &candidates,
                output.cancelled,
                &output.errors,
            )
            .map_err(|error| error.to_string())?;
        let dashboard = database
            .lock()
            .map_err(|_| "local index lock was poisoned".to_owned())?
            .dashboard()
            .map_err(|error| error.to_string())?;
        Ok::<_, String>(ScanResult {
            scan_id,
            source_id,
            cancelled: output.cancelled,
            counts,
            dashboard,
        })
    })
    .await;

    let result = match worker_result {
        Ok(scan) => scan,
        Err(error) => Err(format!("scan worker stopped unexpectedly: {error}")),
    };

    running.store(false, Ordering::Release);
    let message = match &result {
        Ok(scan) if scan.cancelled => {
            "Extraction cancelled; the previous complete index was preserved."
        }
        Ok(_) => "Metadata extraction complete. The local index is ready.",
        Err(_) => "Metadata extraction stopped with an error.",
    };
    let _ = app.emit(
        "scan-progress",
        ScanProgress {
            phase: if result.is_ok() { "complete" } else { "error" }.into(),
            current_path: None,
            files_seen: result
                .as_ref()
                .map(|scan| scan.counts.files)
                .unwrap_or_default(),
            media_found: result
                .as_ref()
                .map(|scan| scan.counts.photos + scan.counts.videos)
                .unwrap_or_default(),
            errors: result.as_ref().map(|scan| scan.counts.errors).unwrap_or(1),
            message: message.into(),
        },
    );
    result
}

fn canonical_source(folder: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(folder);
    if !path.is_dir() {
        return Err(format!("{} is not a readable folder", path.display()));
    }
    path.canonicalize()
        .map_err(|error| format!("could not resolve {}: {error}", path.display()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());
    // Preserve the native default menu and add the company site as an explicit
    // user action; the app does not contact the site by itself.
    #[cfg(target_os = "macos")]
    let builder = builder
        .menu(|app| {
            use tauri::menu::{Menu, MenuItem, MenuItemKind};
            let menu = Menu::default(app)?;
            if let Some(MenuItemKind::Submenu(application)) = menu.items()?.into_iter().next() {
                let website = MenuItem::with_id(
                    app,
                    "amara-company-website",
                    "amara.gmbh",
                    true,
                    None::<&str>,
                )?;
                application.insert(&website, 1)?;
            }
            Ok(menu)
        })
        .on_menu_event(|_app, event| {
            if event.id() == "amara-company-website" {
                let _ = std::process::Command::new("open")
                    .arg("https://amara.gmbh")
                    .spawn();
            }
        });
    // The embedded WebDriver and execution bridge exist only in explicit E2E
    // builds. Ordinary debug/release binaries do not compile or register them.
    #[cfg(feature = "e2e")]
    let builder = builder
        .plugin(tauri_plugin_wdio::init())
        .plugin(tauri_plugin_wdio_webdriver::init());

    builder
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let database = SqliteIndex::open(data_dir.join("amara-unfold.db"))
                .map_err(|error| Box::<dyn std::error::Error>::from(error.to_string()))?;
            app.manage(AppState {
                database: Arc::new(Mutex::new(database)),
                cancel_scan: Arc::new(AtomicBool::new(false)),
                scan_running: Arc::new(AtomicBool::new(false)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_dashboard,
            privacy_status,
            local_app_sources,
            open_apple_mail,
            release_local_app_access,
            inspect_dataset,
            extract_dataset,
            build_timeline_index,
            discover_timeline_archives,
            read_timeline_catalog,
            read_timeline_event,
            set_timeline_event_date,
            undo_timeline_event_date,
            export_memory_markdown,
            scan_folder,
            cancel_scan,
            remove_source
        ])
        .run(tauri::generate_context!())
        .expect("Amara Unfold could not start");
}

#[cfg(test)]
mod timeline_event_tests {
    use super::*;

    #[test]
    fn markdown_body_removes_frontmatter_and_preview_is_bounded() {
        let markdown = "---\nid: evt-1\ntitle: Test\n---\n## Message\n\nA complete diary message with readable words.";
        let body = markdown_body(markdown);
        assert_eq!(
            body,
            "## Message\n\nA complete diary message with readable words."
        );
        assert_eq!(event_preview(body, 18), "Message. A complet…");
        assert_eq!(
            event_preview(
                "# Repeated title\n\nTwo movements.\n\n## Movement 1\n- Amount: `12 EUR`",
                80
            ),
            "Two movements. Movement 1. Amount: 12 EUR."
        );
    }

    #[test]
    fn timeline_event_resolution_stays_inside_export_root() {
        let temporary = tempfile::tempdir().unwrap();
        let export_root = temporary.path().join("output");
        let timeline = export_root.join("Amara-Unfold-Timeline");
        let events = export_root.join("airbnb/events/2024/01");
        fs::create_dir_all(&timeline).unwrap();
        fs::create_dir_all(&events).unwrap();
        fs::write(
            timeline.join("manifest.yaml"),
            "kind: amara-unfold-timeline-index\n",
        )
        .unwrap();
        fs::write(events.join("event.md"), "# Event\n\nComplete body.\n").unwrap();
        let timeline = verified_timeline_folder(&timeline.to_string_lossy()).unwrap();
        assert!(resolve_timeline_event(&timeline, "airbnb/events/2024/01/event.md").is_ok());
        assert!(resolve_timeline_event(&timeline, "../outside.md").is_err());
        assert!(resolve_timeline_event(&timeline, "/tmp/outside.md").is_err());
    }

    #[test]
    fn sibling_timeline_discovery_restores_distinct_archives_and_deduplicates_reruns() {
        let temporary = tempfile::tempdir().unwrap();
        let exports = temporary.path().join("exports");
        let old_diary = exports.join("01-diary-old/Amara-Unfold-Timeline");
        let fixed_diary = exports.join("01-diary-fixed/Amara-Unfold-Timeline");
        let airbnb = exports.join("02-airbnb/Amara-Unfold-Timeline");
        write_test_archive(
            &old_diary,
            "2026-08-03T15:00:00+02:00",
            &[("diary-1", "2019-08-29"), ("diary-2", "0000-00-00")],
        );
        write_test_archive(
            &fixed_diary,
            "2026-08-03T16:00:00+02:00",
            &[("diary-1", "2019-08-29"), ("diary-2", "0000-00-00")],
        );
        write_test_archive(
            &airbnb,
            "2026-08-03T17:00:00+02:00",
            &[("airbnb-1", "2024-06-01")],
        );

        let primary = verified_timeline_folder(&airbnb.to_string_lossy()).unwrap();
        let archives = discover_timeline_archives_from(&primary);
        assert_eq!(archives.len(), 2);
        assert_eq!(
            archives
                .iter()
                .map(|archive| archive.events_indexed)
                .sum::<u64>(),
            3
        );
        assert!(archives
            .iter()
            .any(|archive| archive.folder.contains("01-diary-fixed")));
        assert!(!archives
            .iter()
            .any(|archive| archive.folder.contains("01-diary-old")));
    }

    fn write_test_archive(folder: &Path, generated_at: &str, events: &[(&str, &str)]) {
        fs::create_dir_all(folder).unwrap();
        fs::write(
            folder.join("manifest.yaml"),
            format!(
                "kind: amara-unfold-timeline-index\nschemaVersion: 1\ngeneratedAt: {generated_at}\neventsIndexed: {}\nduplicateEvents: 0\nrejectedEvents: 0\ndayPages: 1\nmonthPages: 1\nentityAnchors: 0\ncrossSourceDays: 0\nstrictPortable: true\nnetworkUsed: false\n",
                events.len()
            ),
        )
        .unwrap();
        let mut catalog =
            String::from("id,date,source,module,type,title,event_path,raw_path,review_status\n");
        for (id, date) in events {
            catalog.push_str(&format!(
                "{id},{date},diary,diary,diary_entry,Test {id},diary/events/{id}.md,/raw/diary.docx,ready\n"
            ));
        }
        fs::write(folder.join("catalog.csv"), catalog).unwrap();
    }
}
