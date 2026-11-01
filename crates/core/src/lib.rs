// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub mod privacy;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Image,
    Video,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedMetadata {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_seconds: Option<f64>,
    pub captured_at: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub device_make: Option<String>,
    pub device_model: Option<String>,
    pub extractor: String,
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredAsset {
    pub path: PathBuf,
    pub filename: String,
    pub extension: String,
    pub media_kind: MediaKind,
    pub size_bytes: u64,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub content_hash: Option<String>,
    pub metadata: ExtractedMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetRecord {
    pub id: i64,
    pub source_id: i64,
    pub path: String,
    pub filename: String,
    pub extension: String,
    pub media_kind: MediaKind,
    pub size_bytes: u64,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub captured_at: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_seconds: Option<f64>,
    pub content_hash: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub device_make: Option<String>,
    pub device_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRecord {
    pub id: i64,
    pub path: String,
    pub display_name: String,
    pub last_scanned_at: Option<String>,
    pub asset_count: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanCounts {
    pub files: u64,
    pub photos: u64,
    pub videos: u64,
    pub unknown: u64,
    pub duplicates: u64,
    pub errors: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineGroup {
    pub key: String,
    pub label: String,
    pub count: u64,
    pub photos: u64,
    pub videos: u64,
    pub total_bytes: u64,
    pub sample_assets: Vec<AssetRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateEvidence {
    pub path: String,
    pub captured_at: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCandidate {
    pub id: Option<i64>,
    pub cluster_key: String,
    pub title: String,
    pub date_label: String,
    pub place_hint: Option<String>,
    pub asset_count: u64,
    pub photo_count: u64,
    pub video_count: u64,
    pub evidence: Vec<CandidateEvidence>,
    #[serde(default)]
    pub sample_assets: Vec<AssetRecord>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub location_spread_meters: Option<f64>,
    #[serde(default)]
    pub device_labels: Vec<String>,
    pub distinct_files: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSnapshot {
    pub counts: ScanCounts,
    pub sources: Vec<SourceRecord>,
    pub timeline: Vec<TimelineGroup>,
    pub candidates: Vec<MemoryCandidate>,
    pub recent_scan_errors: Vec<ScanIssue>,
    pub database_path: String,
    pub last_scan_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub phase: String,
    pub current_path: Option<String>,
    pub files_seen: u64,
    pub media_found: u64,
    pub errors: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub scan_id: i64,
    pub source_id: i64,
    pub cancelled: bool,
    pub counts: ScanCounts,
    pub dashboard: DashboardSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanIssue {
    pub scan_id: i64,
    pub source_name: String,
    pub message: String,
    pub occurred_at: String,
}

#[derive(Debug, thiserror::Error)]
pub enum UnfoldError {
    #[error("invalid source folder: {0}")]
    InvalidSource(String),
    #[error("scan cancelled")]
    Cancelled,
    #[error("storage error: {0}")]
    Storage(String),
    #[error("scan error: {0}")]
    Scan(String),
}

pub trait IndexRepository {
    fn dashboard(&self) -> Result<DashboardSnapshot, UnfoldError>;
    fn remove_source(&mut self, source_id: i64) -> Result<bool, UnfoldError>;
    fn begin_scan(&mut self, source_path: &str) -> Result<(i64, i64), UnfoldError>;
    fn finish_scan(
        &mut self,
        source_id: i64,
        scan_id: i64,
        assets: &[DiscoveredAsset],
        candidates: &[MemoryCandidate],
        cancelled: bool,
        errors: &[String],
    ) -> Result<ScanCounts, UnfoldError>;
    fn fail_scan(&mut self, source_id: i64, scan_id: i64, error: &str) -> Result<(), UnfoldError>;
}

/// A source importer describes how an archive enters Unfold without coupling
/// that archive's format or authentication model to the desktop interface.
///
/// The first implementation is a local-folder importer. Apple Photos, Google
/// Takeout, or other sources can implement this contract later without
/// changing scan orchestration or storage.
pub trait SourceImporter: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn supports(&self, source: &str) -> bool;
}
