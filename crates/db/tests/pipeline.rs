// SPDX-License-Identifier: AGPL-3.0-or-later

use amara_unfold_core::IndexRepository;
use amara_unfold_db::SqliteIndex;
use amara_unfold_memory::{CandidateEngine, DateClusterEngine};
use amara_unfold_scanner::{ArchiveScanner, LocalArchiveScanner};
use std::sync::atomic::AtomicBool;

// This is a repository-owned application icon, never personal fixture data.
const SAMPLE_PNG: &[u8] = include_bytes!("../../../apps/desktop/src-tauri/icons/32x32.png");

#[test]
fn archive_pipeline_creates_a_queryable_sqlite_index() {
    let temporary = tempfile::tempdir().unwrap();
    let archive = temporary.path().join("synthetic-archive");
    std::fs::create_dir(&archive).unwrap();
    std::fs::write(archive.join("photo-a.png"), SAMPLE_PNG).unwrap();
    std::fs::write(archive.join("photo-a-copy.png"), SAMPLE_PNG).unwrap();
    std::fs::write(archive.join("clip.mov"), b"synthetic video placeholder").unwrap();

    let scanner = LocalArchiveScanner::default();
    let output = scanner
        .scan(&archive, &AtomicBool::new(false), &mut |_, _, _, _| {})
        .unwrap();
    assert_eq!(output.assets.len(), 3);
    assert!(
        output
            .assets
            .iter()
            .filter(|asset| asset.metadata.width == Some(32) && asset.metadata.height == Some(32))
            .count()
            >= 2
    );

    let candidates = DateClusterEngine.build_candidates(&output.assets);
    let database_path = temporary.path().join("index/amara-unfold.db");
    let mut database = SqliteIndex::open(&database_path).unwrap();
    let source = archive.to_string_lossy();
    let (source_id, scan_id) = database.begin_scan(&source).unwrap();
    database
        .finish_scan(
            source_id,
            scan_id,
            &output.assets,
            &candidates,
            false,
            &output.errors,
        )
        .unwrap();

    let snapshot = database.dashboard().unwrap();
    assert!(database_path.is_file());
    assert_eq!(snapshot.counts.files, 3);
    assert_eq!(snapshot.counts.photos, 2);
    assert_eq!(snapshot.counts.videos, 1);
    assert_eq!(snapshot.counts.duplicates, 1);
    assert!(!snapshot.timeline.is_empty());
    assert!(!snapshot.candidates.is_empty());
    let candidate = &snapshot.candidates[0];
    assert!(!candidate.sample_assets.is_empty());
    assert_eq!(candidate.distinct_files, 2);
    let preview_asset = candidate.sample_assets.first().unwrap();
    assert_eq!(
        database.asset_path(preview_asset.id).unwrap().as_deref(),
        Some(preview_asset.path.as_str())
    );

    assert!(database.remove_source(source_id).unwrap());
    let empty = database.dashboard().unwrap();
    assert!(empty.sources.is_empty());
    assert_eq!(empty.counts.files, 0);
    assert!(empty.timeline.is_empty());
    assert!(empty.candidates.is_empty());
    assert!(archive.join("photo-a.png").is_file());
    assert!(!database.remove_source(source_id).unwrap());
}
