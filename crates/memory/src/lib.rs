// SPDX-License-Identifier: AGPL-3.0-or-later

use amara_unfold_core::{CandidateEvidence, DiscoveredAsset, MediaKind, MemoryCandidate};
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Utc};
use std::collections::{BTreeMap, BTreeSet};

pub trait CandidateEngine: Send + Sync {
    fn build_candidates(&self, assets: &[DiscoveredAsset]) -> Vec<MemoryCandidate>;
}

#[derive(Debug, Default)]
pub struct DateClusterEngine;

impl CandidateEngine for DateClusterEngine {
    fn build_candidates(&self, assets: &[DiscoveredAsset]) -> Vec<MemoryCandidate> {
        let mut groups: BTreeMap<String, Vec<&DiscoveredAsset>> = BTreeMap::new();
        for asset in assets
            .iter()
            .filter(|asset| asset.media_kind != MediaKind::Unknown)
        {
            if let Some(day) = asset_day(asset) {
                groups.entry(day).or_default().push(asset);
            }
        }

        let mut candidates: Vec<_> = groups
            .into_iter()
            .filter(|(_, items)| !items.is_empty())
            .map(|(day, items)| make_candidate(day, items))
            .collect();
        candidates.sort_by(|left, right| right.cluster_key.cmp(&left.cluster_key));
        candidates
    }
}

fn make_candidate(day: String, assets: Vec<&DiscoveredAsset>) -> MemoryCandidate {
    let photos = assets
        .iter()
        .filter(|asset| asset.media_kind == MediaKind::Image)
        .count() as u64;
    let videos = assets
        .iter()
        .filter(|asset| asset.media_kind == MediaKind::Video)
        .count() as u64;
    let place_hint = common_parent_hint(&assets);
    let moments: Vec<_> = assets
        .iter()
        .filter_map(|asset| asset_moment(asset))
        .collect();
    let started_at = moments.iter().min().map(DateTime::to_rfc3339);
    let ended_at = moments.iter().max().map(DateTime::to_rfc3339);
    let coordinates: Vec<_> = assets
        .iter()
        .filter_map(|asset| Some((asset.metadata.latitude?, asset.metadata.longitude?)))
        .collect();
    let (latitude, longitude, location_spread_meters) = coordinate_summary(&coordinates);
    let device_labels = assets
        .iter()
        .filter_map(|asset| {
            let make = asset.metadata.device_make.as_deref().unwrap_or_default();
            let model = asset.metadata.device_model.as_deref().unwrap_or_default();
            let label = format!("{make} {model}").trim().to_owned();
            (!label.is_empty()).then_some(label)
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let distinct_files = assets
        .iter()
        .map(|asset| {
            asset
                .content_hash
                .clone()
                .unwrap_or_else(|| format!("path:{}", asset.path.display()))
        })
        .collect::<BTreeSet<_>>()
        .len() as u64;
    let title = match &place_hint {
        Some(place) => format!("Possible memory · {day} · {place}"),
        None => format!("Possible memory · {day}"),
    };
    let evidence = assets
        .iter()
        .map(|asset| CandidateEvidence {
            path: asset.path.to_string_lossy().into_owned(),
            captured_at: asset
                .metadata
                .captured_at
                .clone()
                .or_else(|| asset.modified_at.clone()),
            reason: if asset.metadata.captured_at.is_some() {
                "EXIF capture date matches this cluster".into()
            } else {
                "File timestamp matches this cluster".into()
            },
        })
        .collect();
    MemoryCandidate {
        id: None,
        cluster_key: day.clone(),
        title,
        date_label: day,
        place_hint,
        asset_count: assets.len() as u64,
        photo_count: photos,
        video_count: videos,
        evidence,
        sample_assets: Vec::new(),
        started_at,
        ended_at,
        latitude,
        longitude,
        location_spread_meters,
        device_labels,
        distinct_files,
    }
}

fn asset_moment(asset: &DiscoveredAsset) -> Option<DateTime<Utc>> {
    let value = asset
        .metadata
        .captured_at
        .as_deref()
        .or(asset.modified_at.as_deref())
        .or(asset.created_at.as_deref())?;
    DateTime::parse_from_rfc3339(value)
        .map(|date| date.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            ["%Y:%m:%d %H:%M:%S", "%Y-%m-%d %H:%M:%S"]
                .into_iter()
                .find_map(|format| NaiveDateTime::parse_from_str(value, format).ok())
                .map(|date| date.and_utc())
        })
}

fn coordinate_summary(coordinates: &[(f64, f64)]) -> (Option<f64>, Option<f64>, Option<f64>) {
    if coordinates.is_empty() {
        return (None, None, None);
    }
    let latitude = coordinates.iter().map(|item| item.0).sum::<f64>() / coordinates.len() as f64;
    let longitude = coordinates.iter().map(|item| item.1).sum::<f64>() / coordinates.len() as f64;
    let spread = coordinates
        .iter()
        .map(|&(lat, lon)| distance_meters(latitude, longitude, lat, lon))
        .fold(0.0, f64::max);
    (Some(latitude), Some(longitude), Some(spread))
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

fn asset_day(asset: &DiscoveredAsset) -> Option<String> {
    let raw = asset
        .metadata
        .captured_at
        .as_deref()
        .or(asset.modified_at.as_deref())
        .or(asset.created_at.as_deref())?;
    parse_day(raw).map(|date| format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day()))
}

fn parse_day(value: &str) -> Option<NaiveDate> {
    DateTime::parse_from_rfc3339(value)
        .map(|date| date.date_naive())
        .ok()
        .or_else(|| NaiveDate::parse_from_str(value.get(..10)?, "%Y-%m-%d").ok())
        .or_else(|| NaiveDate::parse_from_str(value.get(..10)?, "%Y:%m:%d").ok())
        .or_else(|| {
            value
                .parse::<DateTime<Utc>>()
                .ok()
                .map(|date| date.date_naive())
        })
}

fn common_parent_hint(assets: &[&DiscoveredAsset]) -> Option<String> {
    let first = assets
        .first()?
        .path
        .parent()?
        .file_name()?
        .to_string_lossy()
        .trim()
        .to_owned();
    if first.is_empty()
        || first.starts_with('.')
        || first.eq_ignore_ascii_case("photos")
        || first.chars().all(|character| character.is_ascii_digit())
    {
        return None;
    }
    let all_match = assets.iter().all(|asset| {
        asset
            .path
            .parent()
            .and_then(|parent| parent.file_name())
            .is_some_and(|name| name.to_string_lossy() == first)
    });
    all_match.then_some(first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use amara_unfold_core::ExtractedMetadata;
    use std::path::PathBuf;

    fn asset(path: &str, captured_at: &str, media_kind: MediaKind) -> DiscoveredAsset {
        DiscoveredAsset {
            path: PathBuf::from(path),
            filename: "asset.jpg".into(),
            extension: "jpg".into(),
            media_kind,
            size_bytes: 12,
            created_at: None,
            modified_at: None,
            content_hash: None,
            metadata: ExtractedMetadata {
                captured_at: Some(captured_at.into()),
                ..ExtractedMetadata::default()
            },
        }
    }

    #[test]
    fn candidates_group_by_day_and_keep_evidence() {
        let assets = vec![
            asset(
                "/archive/Jersey City/a.jpg",
                "2019:08:30 10:00:00",
                MediaKind::Image,
            ),
            asset(
                "/archive/Jersey City/b.mov",
                "2019:08:30 11:00:00",
                MediaKind::Video,
            ),
        ];
        let candidates = DateClusterEngine.build_candidates(&assets);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].asset_count, 2);
        assert_eq!(candidates[0].place_hint.as_deref(), Some("Jersey City"));
        assert_eq!(candidates[0].evidence.len(), 2);
    }

    #[test]
    fn candidates_skip_unknown_files_and_separate_days() {
        let assets = vec![
            asset("/archive/a.jpg", "2019:08:30 10:00:00", MediaKind::Image),
            asset("/archive/b.jpg", "2019:08:31 10:00:00", MediaKind::Image),
            asset(
                "/archive/notes.txt",
                "2019:08:30 10:00:00",
                MediaKind::Unknown,
            ),
        ];
        let candidates = DateClusterEngine.build_candidates(&assets);
        assert_eq!(candidates.len(), 2);
        assert!(candidates
            .iter()
            .all(|candidate| candidate.asset_count == 1));
        assert_eq!(candidates[0].cluster_key, "2019-08-31");
    }

    #[test]
    fn numeric_folders_are_not_reported_as_places() {
        let assets = vec![asset(
            "/archive/2016/10/13/IMG_0001.jpg",
            "2016:10:13 09:26:46",
            MediaKind::Image,
        )];

        let candidates = DateClusterEngine.build_candidates(&assets);

        assert_eq!(candidates[0].place_hint, None);
        assert_eq!(candidates[0].distinct_files, 1);
        assert_eq!(
            candidates[0].started_at.as_deref(),
            Some("2016-10-13T09:26:46+00:00")
        );
    }

    #[test]
    fn candidate_exposes_time_location_and_camera_facts() {
        let mut first = asset(
            "/archive/event/a.jpg",
            "2016:10:13 09:26:46",
            MediaKind::Image,
        );
        first.content_hash = Some("first".into());
        first.metadata.latitude = Some(48.0724);
        first.metadata.longitude = Some(11.2476);
        first.metadata.device_make = Some("Apple".into());
        first.metadata.device_model = Some("iPhone 6".into());
        let mut second = asset(
            "/archive/event/b.jpg",
            "2016:10:13 09:39:06",
            MediaKind::Image,
        );
        second.content_hash = Some("second".into());
        second.metadata.latitude = Some(48.0725);
        second.metadata.longitude = Some(11.2477);
        second.metadata.device_make = Some("Apple".into());
        second.metadata.device_model = Some("iPhone 6".into());

        let candidates = DateClusterEngine.build_candidates(&[first, second]);
        let candidate = &candidates[0];

        assert_eq!(
            candidate.started_at.as_deref(),
            Some("2016-10-13T09:26:46+00:00")
        );
        assert_eq!(
            candidate.ended_at.as_deref(),
            Some("2016-10-13T09:39:06+00:00")
        );
        assert_eq!(candidate.device_labels, ["Apple iPhone 6"]);
        assert_eq!(candidate.distinct_files, 2);
        assert!(candidate
            .location_spread_meters
            .is_some_and(|spread| spread < 10.0));
    }
}
