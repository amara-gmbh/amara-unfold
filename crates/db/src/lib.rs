// SPDX-License-Identifier: AGPL-3.0-or-later

use amara_unfold_core::{
    AssetRecord, CandidateEvidence, DashboardSnapshot, DiscoveredAsset, IndexRepository, MediaKind,
    MemoryCandidate, ScanCounts, ScanIssue, SourceRecord, TimelineGroup, UnfoldError,
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use std::{collections::HashMap, path::Path};

const INITIAL_MIGRATION: &str = include_str!("../migrations/0001_initial.sql");

/// SQLite is the only persistence boundary in the first release. Keeping it
/// behind `IndexRepository` lets tests use temporary databases and leaves room
/// for a different local store without changing scanning or candidate logic.
pub struct SqliteIndex {
    connection: Connection,
    path: String,
}

impl SqliteIndex {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, UnfoldError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(storage_error)?;
        }
        let connection = Connection::open(path).map_err(storage_error)?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(storage_error)?;
        connection
            .execute_batch(INITIAL_MIGRATION)
            .map_err(storage_error)?;
        Ok(Self {
            connection,
            path: path.to_string_lossy().into_owned(),
        })
    }

    pub fn open_in_memory() -> Result<Self, UnfoldError> {
        let connection = Connection::open_in_memory().map_err(storage_error)?;
        connection
            .execute_batch(INITIAL_MIGRATION)
            .map_err(storage_error)?;
        Ok(Self {
            connection,
            path: ":memory:".into(),
        })
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn asset_path(&self, asset_id: i64) -> Result<Option<String>, UnfoldError> {
        self.connection
            .query_row("SELECT path FROM assets WHERE id = ?1", [asset_id], |row| {
                row.get(0)
            })
            .optional()
            .map_err(storage_error)
    }

    pub fn candidate_assets(&self, candidate_id: i64) -> Result<Vec<AssetRecord>, UnfoldError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT a.id, a.source_id, a.path, a.filename, a.extension, a.media_kind,
                    a.size_bytes, a.created_at, a.modified_at, a.captured_at,
                    m.width, m.height, m.duration_seconds, a.content_hash,
                    m.latitude, m.longitude, m.device_make, m.device_model
                 FROM candidate_assets ca
                 JOIN assets a ON a.id = ca.asset_id
                 LEFT JOIN metadata m ON m.asset_id = a.id
                 WHERE ca.candidate_id = ?1
                 ORDER BY a.effective_date, a.id",
            )
            .map_err(storage_error)?;
        let assets = statement
            .query_map([candidate_id], asset_from_row)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(assets)
    }
}

impl IndexRepository for SqliteIndex {
    fn dashboard(&self) -> Result<DashboardSnapshot, UnfoldError> {
        let counts = self
            .connection
            .query_row(
                "SELECT
                COUNT(*),
                COALESCE(SUM(media_kind = 'image'), 0),
                COALESCE(SUM(media_kind = 'video'), 0),
                COALESCE(SUM(media_kind = 'unknown'), 0),
                COALESCE(SUM(size_bytes), 0),
                COALESCE((SELECT SUM(items - 1) FROM (
                    SELECT COUNT(*) AS items FROM assets
                    WHERE content_hash IS NOT NULL GROUP BY content_hash HAVING COUNT(*) > 1
                )), 0)
             FROM assets",
                [],
                |row| {
                    Ok(ScanCounts {
                        files: row.get::<_, i64>(0)? as u64,
                        photos: row.get::<_, i64>(1)? as u64,
                        videos: row.get::<_, i64>(2)? as u64,
                        unknown: row.get::<_, i64>(3)? as u64,
                        bytes: row.get::<_, i64>(4)? as u64,
                        duplicates: row.get::<_, i64>(5)? as u64,
                        errors: 0,
                    })
                },
            )
            .map_err(storage_error)?;

        let sources = query_sources(&self.connection)?;
        let timeline = query_timeline(&self.connection)?;
        let candidates = query_candidates(&self.connection)?;
        let recent_scan_errors = query_scan_errors(&self.connection)?;
        let last_scan_at = self.connection
            .query_row(
                "SELECT finished_at FROM scans WHERE status = 'completed' ORDER BY finished_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;

        Ok(DashboardSnapshot {
            counts,
            sources,
            timeline,
            candidates,
            recent_scan_errors,
            database_path: self.path.clone(),
            last_scan_at,
        })
    }

    fn remove_source(&mut self, source_id: i64) -> Result<bool, UnfoldError> {
        let transaction = self.connection.transaction().map_err(storage_error)?;
        let exists = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sources WHERE id = ?1)",
                [source_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(storage_error)?;
        if !exists {
            return Ok(false);
        }

        // Delete index-owned snapshots before their scans. The source folder
        // itself is never opened or modified by this operation.
        transaction
            .execute(
                "DELETE FROM memory_candidates WHERE source_id = ?1",
                [source_id],
            )
            .map_err(storage_error)?;
        transaction
            .execute("DELETE FROM assets WHERE source_id = ?1", [source_id])
            .map_err(storage_error)?;
        transaction
            .execute("DELETE FROM scans WHERE source_id = ?1", [source_id])
            .map_err(storage_error)?;
        transaction
            .execute("DELETE FROM sources WHERE id = ?1", [source_id])
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)?;
        Ok(true)
    }

    fn begin_scan(&mut self, source_path: &str) -> Result<(i64, i64), UnfoldError> {
        let now = Utc::now().to_rfc3339();
        let display_name = Path::new(source_path)
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or(source_path);
        self.connection
            .execute(
                "INSERT INTO sources(path, display_name, created_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(path) DO UPDATE SET display_name = excluded.display_name",
                params![source_path, display_name, now],
            )
            .map_err(storage_error)?;
        let source_id: i64 = self
            .connection
            .query_row(
                "SELECT id FROM sources WHERE path = ?1",
                [source_path],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        self.connection
            .execute(
                "INSERT INTO scans(source_id, status, started_at) VALUES (?1, 'running', ?2)",
                params![source_id, now],
            )
            .map_err(storage_error)?;
        Ok((source_id, self.connection.last_insert_rowid()))
    }

    fn finish_scan(
        &mut self,
        source_id: i64,
        scan_id: i64,
        assets: &[DiscoveredAsset],
        candidates: &[MemoryCandidate],
        cancelled: bool,
        errors: &[String],
    ) -> Result<ScanCounts, UnfoldError> {
        let counts = count_assets(assets, errors.len() as u64);
        let transaction = self.connection.transaction().map_err(storage_error)?;

        // Cancellation leaves the last complete index untouched. The scan and
        // its transparent errors are still recorded for auditability.
        if !cancelled {
            replace_source_snapshot(&transaction, source_id, scan_id, assets, candidates)?;
        }
        for error in errors {
            transaction
                .execute(
                    "INSERT INTO scan_errors(scan_id, message) VALUES (?1, ?2)",
                    params![scan_id, error],
                )
                .map_err(storage_error)?;
        }
        let now = Utc::now().to_rfc3339();
        transaction
            .execute(
                "UPDATE scans SET status = ?2, finished_at = ?3, files_count = ?4,
                    photos_count = ?5, videos_count = ?6, unknown_count = ?7,
                    duplicates_count = ?8, errors_count = ?9, bytes_count = ?10
                 WHERE id = ?1",
                params![
                    scan_id,
                    if cancelled { "cancelled" } else { "completed" },
                    now,
                    counts.files as i64,
                    counts.photos as i64,
                    counts.videos as i64,
                    counts.unknown as i64,
                    counts.duplicates as i64,
                    counts.errors as i64,
                    counts.bytes as i64,
                ],
            )
            .map_err(storage_error)?;
        if !cancelled {
            transaction
                .execute(
                    "UPDATE sources SET last_scanned_at = ?2 WHERE id = ?1",
                    params![source_id, now],
                )
                .map_err(storage_error)?;
        }
        transaction.commit().map_err(storage_error)?;
        Ok(counts)
    }

    fn fail_scan(&mut self, source_id: i64, scan_id: i64, error: &str) -> Result<(), UnfoldError> {
        let transaction = self.connection.transaction().map_err(storage_error)?;
        transaction
            .execute(
                "INSERT INTO scan_errors(scan_id, message) VALUES (?1, ?2)",
                params![scan_id, error],
            )
            .map_err(storage_error)?;
        transaction
            .execute(
                "UPDATE scans SET status = 'failed', finished_at = ?2, errors_count = 1 WHERE id = ?1 AND source_id = ?3",
                params![scan_id, Utc::now().to_rfc3339(), source_id],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)
    }
}

fn replace_source_snapshot(
    transaction: &Transaction<'_>,
    source_id: i64,
    scan_id: i64,
    assets: &[DiscoveredAsset],
    candidates: &[MemoryCandidate],
) -> Result<(), UnfoldError> {
    transaction
        .execute(
            "DELETE FROM memory_candidates WHERE source_id = ?1",
            [source_id],
        )
        .map_err(storage_error)?;
    transaction
        .execute("DELETE FROM assets WHERE source_id = ?1", [source_id])
        .map_err(storage_error)?;

    let mut asset_ids = HashMap::new();
    for asset in assets {
        let path = asset.path.to_string_lossy();
        let captured_at = normalize_date(asset.metadata.captured_at.as_deref());
        let effective_date = captured_at
            .clone()
            .or_else(|| normalize_date(asset.modified_at.as_deref()))
            .or_else(|| normalize_date(asset.created_at.as_deref()));
        transaction
            .execute(
                "INSERT INTO assets(source_id, last_scan_id, path, filename, extension, media_kind,
                    size_bytes, created_at, modified_at, captured_at, effective_date, content_hash)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    source_id,
                    scan_id,
                    path.as_ref(),
                    asset.filename,
                    asset.extension,
                    media_kind_string(asset.media_kind),
                    asset.size_bytes as i64,
                    asset.created_at,
                    asset.modified_at,
                    captured_at,
                    effective_date,
                    asset.content_hash,
                ],
            )
            .map_err(storage_error)?;
        let asset_id = transaction.last_insert_rowid();
        asset_ids.insert(path.into_owned(), asset_id);
        transaction
            .execute(
                "INSERT INTO metadata(asset_id, width, height, duration_seconds, latitude, longitude,
                    device_make, device_model, extractor, extra_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    asset_id,
                    asset.metadata.width.map(i64::from),
                    asset.metadata.height.map(i64::from),
                    asset.metadata.duration_seconds,
                    asset.metadata.latitude,
                    asset.metadata.longitude,
                    asset.metadata.device_make,
                    asset.metadata.device_model,
                    asset.metadata.extractor,
                    asset.metadata.extra.to_string(),
                ],
            )
            .map_err(storage_error)?;
    }

    for candidate in candidates {
        transaction
            .execute(
                "INSERT INTO memory_candidates(source_id, scan_id, cluster_key, title, date_label,
                    place_hint, asset_count, photo_count, video_count, evidence_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    source_id,
                    scan_id,
                    candidate.cluster_key,
                    candidate.title,
                    candidate.date_label,
                    candidate.place_hint,
                    candidate.asset_count as i64,
                    candidate.photo_count as i64,
                    candidate.video_count as i64,
                    serde_json::to_string(&candidate.evidence).map_err(storage_error)?,
                    Utc::now().to_rfc3339(),
                ],
            )
            .map_err(storage_error)?;
        let candidate_id = transaction.last_insert_rowid();
        for evidence in &candidate.evidence {
            if let Some(asset_id) = asset_ids.get(&evidence.path) {
                transaction
                    .execute(
                        "INSERT INTO candidate_assets(candidate_id, asset_id, reason) VALUES (?1, ?2, ?3)",
                        params![candidate_id, asset_id, evidence.reason],
                    )
                    .map_err(storage_error)?;
            }
        }
    }
    Ok(())
}

fn query_sources(connection: &Connection) -> Result<Vec<SourceRecord>, UnfoldError> {
    let mut statement = connection
        .prepare(
            "SELECT s.id, s.path, s.display_name, s.last_scanned_at, COUNT(a.id)
             FROM sources s LEFT JOIN assets a ON a.source_id = s.id
             GROUP BY s.id ORDER BY COALESCE(s.last_scanned_at, s.created_at) DESC",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok(SourceRecord {
                id: row.get(0)?,
                path: row.get(1)?,
                display_name: row.get(2)?,
                last_scanned_at: row.get(3)?,
                asset_count: row.get::<_, i64>(4)? as u64,
            })
        })
        .map_err(storage_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
}

fn query_timeline(connection: &Connection) -> Result<Vec<TimelineGroup>, UnfoldError> {
    let mut statement = connection
        .prepare(
            "SELECT substr(effective_date, 1, 7) AS month, COUNT(*),
                SUM(media_kind = 'image'), SUM(media_kind = 'video'), SUM(size_bytes)
             FROM assets WHERE effective_date IS NOT NULL
             GROUP BY month ORDER BY month DESC",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| {
            let key: String = row.get(0)?;
            Ok((
                key.clone(),
                TimelineGroup {
                    label: month_label(&key),
                    key,
                    count: row.get::<_, i64>(1)? as u64,
                    photos: row.get::<_, i64>(2)? as u64,
                    videos: row.get::<_, i64>(3)? as u64,
                    total_bytes: row.get::<_, i64>(4)? as u64,
                    sample_assets: Vec::new(),
                },
            ))
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;

    rows.into_iter()
        .map(|(key, mut group)| {
            let mut samples = connection
                .prepare(
                    "SELECT a.id, a.source_id, a.path, a.filename, a.extension, a.media_kind,
                        a.size_bytes, a.created_at, a.modified_at, a.captured_at,
                        m.width, m.height, m.duration_seconds, a.content_hash,
                        m.latitude, m.longitude, m.device_make, m.device_model
                     FROM assets a LEFT JOIN metadata m ON m.asset_id = a.id
                     WHERE substr(a.effective_date, 1, 7) = ?1
                     ORDER BY a.effective_date DESC LIMIT 8",
                )
                .map_err(storage_error)?;
            group.sample_assets = samples
                .query_map([key], asset_from_row)
                .map_err(storage_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage_error)?;
            Ok(group)
        })
        .collect()
}

fn query_candidates(connection: &Connection) -> Result<Vec<MemoryCandidate>, UnfoldError> {
    let mut statement = connection
        .prepare(
            "SELECT id, cluster_key, title, date_label, place_hint, asset_count,
                photo_count, video_count, evidence_json
             FROM memory_candidates ORDER BY date_label DESC LIMIT 200",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| {
            let evidence_json: String = row.get(8)?;
            let place_hint: Option<String> = row.get(4)?;
            Ok(MemoryCandidate {
                id: Some(row.get(0)?),
                cluster_key: row.get(1)?,
                title: row.get(2)?,
                date_label: row.get(3)?,
                place_hint: place_hint.filter(|value| meaningful_folder_hint(value)),
                asset_count: row.get::<_, i64>(5)? as u64,
                photo_count: row.get::<_, i64>(6)? as u64,
                video_count: row.get::<_, i64>(7)? as u64,
                evidence: serde_json::from_str::<Vec<CandidateEvidence>>(&evidence_json)
                    .unwrap_or_default(),
                sample_assets: Vec::new(),
                started_at: None,
                ended_at: None,
                latitude: None,
                longitude: None,
                location_spread_meters: None,
                device_labels: Vec::new(),
                distinct_files: 0,
            })
        })
        .map_err(storage_error)?;
    let candidates = rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)?;
    candidates
        .into_iter()
        .map(|mut candidate| {
            populate_candidate_facts(connection, &mut candidate)?;
            Ok(candidate)
        })
        .collect()
}

fn populate_candidate_facts(
    connection: &Connection,
    candidate: &mut MemoryCandidate,
) -> Result<(), UnfoldError> {
    let Some(candidate_id) = candidate.id else {
        return Ok(());
    };
    let mut statement = connection
        .prepare(
            "SELECT a.id, a.source_id, a.path, a.filename, a.extension, a.media_kind,
                a.size_bytes, a.created_at, a.modified_at, a.captured_at,
                m.width, m.height, m.duration_seconds, a.content_hash,
                m.latitude, m.longitude, m.device_make, m.device_model
             FROM candidate_assets ca
             JOIN assets a ON a.id = ca.asset_id
             LEFT JOIN metadata m ON m.asset_id = a.id
             WHERE ca.candidate_id = ?1
             ORDER BY a.effective_date, a.id",
        )
        .map_err(storage_error)?;
    let assets = statement
        .query_map([candidate_id], asset_from_row)
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;

    candidate.started_at = assets
        .iter()
        .filter_map(|asset| asset.captured_at.clone().or(asset.modified_at.clone()))
        .min();
    candidate.ended_at = assets
        .iter()
        .filter_map(|asset| asset.captured_at.clone().or(asset.modified_at.clone()))
        .max();
    let coordinates: Vec<_> = assets
        .iter()
        .filter_map(|asset| Some((asset.latitude?, asset.longitude?)))
        .collect();
    if !coordinates.is_empty() {
        let latitude =
            coordinates.iter().map(|item| item.0).sum::<f64>() / coordinates.len() as f64;
        let longitude =
            coordinates.iter().map(|item| item.1).sum::<f64>() / coordinates.len() as f64;
        candidate.latitude = Some(latitude);
        candidate.longitude = Some(longitude);
        candidate.location_spread_meters = Some(
            coordinates
                .iter()
                .map(|&(lat, lon)| distance_meters(latitude, longitude, lat, lon))
                .fold(0.0, f64::max),
        );
    }
    candidate.device_labels = assets
        .iter()
        .filter_map(|asset| {
            let label = format!(
                "{} {}",
                asset.device_make.as_deref().unwrap_or_default(),
                asset.device_model.as_deref().unwrap_or_default()
            )
            .trim()
            .to_owned();
            (!label.is_empty()).then_some(label)
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    candidate.distinct_files = assets
        .iter()
        .map(|asset| {
            asset
                .content_hash
                .clone()
                .unwrap_or_else(|| format!("path:{}", asset.path))
        })
        .collect::<std::collections::BTreeSet<_>>()
        .len() as u64;
    candidate.sample_assets = assets.into_iter().take(8).collect();
    Ok(())
}

fn query_scan_errors(connection: &Connection) -> Result<Vec<ScanIssue>, UnfoldError> {
    let mut statement = connection
        .prepare(
            "SELECT e.scan_id, s.display_name, e.message, sc.started_at
             FROM scan_errors e
             JOIN scans sc ON sc.id = e.scan_id
             JOIN sources s ON s.id = sc.source_id
             ORDER BY e.id DESC LIMIT 50",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok(ScanIssue {
                scan_id: row.get(0)?,
                source_name: row.get(1)?,
                message: row.get(2)?,
                occurred_at: row.get(3)?,
            })
        })
        .map_err(storage_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
}

fn asset_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AssetRecord> {
    let kind: String = row.get(5)?;
    Ok(AssetRecord {
        id: row.get(0)?,
        source_id: row.get(1)?,
        path: row.get(2)?,
        filename: row.get(3)?,
        extension: row.get(4)?,
        media_kind: media_kind_from_string(&kind),
        size_bytes: row.get::<_, i64>(6)? as u64,
        created_at: row.get(7)?,
        modified_at: row.get(8)?,
        captured_at: row.get(9)?,
        width: row.get::<_, Option<i64>>(10)?.map(|value| value as u32),
        height: row.get::<_, Option<i64>>(11)?.map(|value| value as u32),
        duration_seconds: row.get(12)?,
        content_hash: row.get(13)?,
        latitude: row.get(14)?,
        longitude: row.get(15)?,
        device_make: row.get(16)?,
        device_model: row.get(17)?,
    })
}

fn meaningful_folder_hint(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && !value.chars().all(|character| character.is_ascii_digit())
}

fn distance_meters(lat_a: f64, lon_a: f64, lat_b: f64, lon_b: f64) -> f64 {
    let radius = 6_371_000.0;
    let lat_delta = (lat_b - lat_a).to_radians();
    let lon_delta = (lon_b - lon_a).to_radians();
    let lat_a = lat_a.to_radians();
    let lat_b = lat_b.to_radians();
    let haversine = (lat_delta / 2.0).sin().powi(2)
        + lat_a.cos() * lat_b.cos() * (lon_delta / 2.0).sin().powi(2);
    radius * 2.0 * haversine.sqrt().asin()
}

fn count_assets(assets: &[DiscoveredAsset], errors: u64) -> ScanCounts {
    let mut counts = ScanCounts {
        files: assets.len() as u64,
        errors,
        ..ScanCounts::default()
    };
    let mut hashes: HashMap<&str, u64> = HashMap::new();
    for asset in assets {
        counts.bytes += asset.size_bytes;
        match asset.media_kind {
            MediaKind::Image => counts.photos += 1,
            MediaKind::Video => counts.videos += 1,
            MediaKind::Unknown => counts.unknown += 1,
        }
        if let Some(hash) = asset.content_hash.as_deref() {
            *hashes.entry(hash).or_default() += 1;
        }
    }
    counts.duplicates = hashes.values().map(|count| count.saturating_sub(1)).sum();
    counts
}

fn normalize_date(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if let Ok(date) = DateTime::parse_from_rfc3339(value) {
        return Some(date.to_rfc3339());
    }
    for format in ["%Y:%m:%d %H:%M:%S", "%Y-%m-%d %H:%M:%S"] {
        if let Ok(date) = NaiveDateTime::parse_from_str(value, format) {
            return Some(date.and_utc().to_rfc3339());
        }
    }
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        return date
            .and_hms_opt(0, 0, 0)
            .map(|date| date.and_utc().to_rfc3339());
    }
    None
}

fn month_label(key: &str) -> String {
    NaiveDate::parse_from_str(&format!("{key}-01"), "%Y-%m-%d")
        .map(|date| date.format("%B %Y").to_string())
        .unwrap_or_else(|_| key.to_owned())
}

fn media_kind_string(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Image => "image",
        MediaKind::Video => "video",
        MediaKind::Unknown => "unknown",
    }
}

fn media_kind_from_string(value: &str) -> MediaKind {
    match value {
        "image" => MediaKind::Image,
        "video" => MediaKind::Video,
        _ => MediaKind::Unknown,
    }
}

fn storage_error(error: impl std::fmt::Display) -> UnfoldError {
    UnfoldError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use amara_unfold_core::ExtractedMetadata;
    use std::path::PathBuf;

    fn discovered(path: &str, kind: MediaKind, hash: &str) -> DiscoveredAsset {
        DiscoveredAsset {
            path: PathBuf::from(path),
            filename: Path::new(path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            extension: "jpg".into(),
            media_kind: kind,
            size_bytes: 128,
            created_at: None,
            modified_at: Some("2019-08-30T10:00:00Z".into()),
            content_hash: Some(hash.into()),
            metadata: ExtractedMetadata {
                extractor: "test".into(),
                ..ExtractedMetadata::default()
            },
        }
    }

    #[test]
    fn complete_scan_is_queryable_and_counts_duplicates() {
        let mut index = SqliteIndex::open_in_memory().unwrap();
        let (source_id, scan_id) = index.begin_scan("/archive/Jersey City").unwrap();
        let assets = vec![
            discovered("/archive/Jersey City/a.jpg", MediaKind::Image, "same"),
            discovered("/archive/Jersey City/b.jpg", MediaKind::Image, "same"),
        ];
        index
            .finish_scan(source_id, scan_id, &assets, &[], false, &[])
            .unwrap();
        let dashboard = index.dashboard().unwrap();
        assert_eq!(dashboard.counts.files, 2);
        assert_eq!(dashboard.counts.duplicates, 1);
        assert_eq!(dashboard.timeline[0].key, "2019-08");
    }

    #[test]
    fn cancelled_scan_does_not_replace_last_complete_snapshot() {
        let mut index = SqliteIndex::open_in_memory().unwrap();
        let (source_id, scan_id) = index.begin_scan("/archive").unwrap();
        let first = vec![discovered("/archive/a.jpg", MediaKind::Image, "a")];
        index
            .finish_scan(source_id, scan_id, &first, &[], false, &[])
            .unwrap();
        let (_, cancelled_scan) = index.begin_scan("/archive").unwrap();
        index
            .finish_scan(source_id, cancelled_scan, &[], &[], true, &[])
            .unwrap();
        assert_eq!(index.dashboard().unwrap().counts.files, 1);
    }

    #[test]
    fn rescan_replaces_source_snapshot_instead_of_accumulating() {
        let mut index = SqliteIndex::open_in_memory().unwrap();
        let (source_id, first_scan) = index.begin_scan("/archive").unwrap();
        let first = vec![
            discovered("/archive/a.jpg", MediaKind::Image, "a"),
            discovered("/archive/b.jpg", MediaKind::Image, "b"),
        ];
        index
            .finish_scan(source_id, first_scan, &first, &[], false, &[])
            .unwrap();

        let (_, second_scan) = index.begin_scan("/archive").unwrap();
        let second = vec![discovered("/archive/c.jpg", MediaKind::Image, "c")];
        index
            .finish_scan(source_id, second_scan, &second, &[], false, &[])
            .unwrap();
        let dashboard = index.dashboard().unwrap();
        assert_eq!(dashboard.counts.files, 1);
        assert_eq!(dashboard.sources[0].asset_count, 1);
    }

    #[test]
    fn failed_scan_is_transparently_queryable() {
        let mut index = SqliteIndex::open_in_memory().unwrap();
        let (source_id, scan_id) = index.begin_scan("/locked/archive").unwrap();
        index
            .fail_scan(source_id, scan_id, "permission denied")
            .unwrap();
        let dashboard = index.dashboard().unwrap();
        assert_eq!(dashboard.recent_scan_errors.len(), 1);
        assert_eq!(dashboard.recent_scan_errors[0].message, "permission denied");
        assert_eq!(dashboard.counts.files, 0);
    }

    #[test]
    fn migrations_are_idempotent_when_database_is_reopened() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("index.db");
        drop(SqliteIndex::open(&path).unwrap());
        let reopened = SqliteIndex::open(&path).unwrap();
        assert!(reopened.dashboard().is_ok());
    }
}
