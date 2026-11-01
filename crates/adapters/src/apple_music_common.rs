// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    model::{date_time_parts, sha256, slug, EventInput, OpenEvent},
    record_event, record_skip, AdapterError, ExtractionProgress, ProgressCallback,
};
use chrono::NaiveDate;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

const FIRST_REAL_ITUNES_DAY: &str = "2001-01-09";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MusicSourceKind {
    MusicApp,
    TabSeparated,
    LocalAudio,
}

impl MusicSourceKind {
    fn label(self) -> &'static str {
        match self {
            Self::MusicApp => "Music.app local library metadata",
            Self::TabSeparated => "Apple Music tab-separated track metadata",
            Self::LocalAudio => "Local audio files with embedded metadata where available",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct MusicTrack {
    pub name: String,
    pub artist: String,
    pub album_artist: String,
    pub album: String,
    pub genre: String,
    pub year: String,
    pub track_number: String,
    pub disc_number: String,
    pub duration_seconds: String,
    pub play_count: String,
    pub date_added: String,
    pub last_played: String,
    pub purchase_date: String,
    pub persistent_id: String,
    pub format: String,
    pub file_name: String,
    pub raw_path: String,
    pub raw_record: Vec<u8>,
    pub content_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct MusicPlaylist {
    pub name: String,
    pub persistent_id: String,
    pub parent_name: String,
    pub parent_persistent_id: String,
    pub playlist_kind: String,
    pub smart: bool,
    pub genius: bool,
    pub shared: bool,
    pub visible: bool,
    pub favorited: bool,
    pub description: String,
    pub duration_seconds: String,
    pub member_persistent_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct MusicLibrarySnapshot {
    pub tracks: Vec<MusicTrack>,
    pub playlists: Vec<MusicPlaylist>,
}

#[derive(Debug, Default)]
struct PlaylistCoverage {
    playlists: usize,
    memberships: usize,
    linked_memberships: usize,
    unmatched_memberships: usize,
    files: Vec<String>,
}

impl MusicTrack {
    fn stable_key(&self) -> String {
        let persistent_id = self.persistent_id.trim();
        if !persistent_id.is_empty() {
            return format!("persistent-id:{}", persistent_id.to_ascii_lowercase());
        }
        if !self.content_hash.trim().is_empty() {
            return format!("content-sha256:{}", self.content_hash.trim());
        }
        let fingerprint = [
            self.name.trim(),
            self.artist.trim(),
            self.album_artist.trim(),
            self.album.trim(),
            self.year.trim(),
            self.track_number.trim(),
            self.disc_number.trim(),
            self.duration_seconds.trim(),
        ]
        .join("\u{001f}")
        .to_lowercase();
        format!("metadata-sha256:{}", sha256(fingerprint.as_bytes()))
    }

    fn display_artist(&self) -> &str {
        first_present(&[&self.artist, &self.album_artist], "Unknown artist")
    }

    fn display_album(&self) -> &str {
        first_present(&[&self.album], "Unknown album")
    }

    fn merge_missing_from(&mut self, other: &Self) {
        macro_rules! merge {
            ($field:ident) => {
                if self.$field.trim().is_empty() && !other.$field.trim().is_empty() {
                    self.$field = other.$field.clone();
                }
            };
        }
        merge!(name);
        merge!(artist);
        merge!(album_artist);
        merge!(album);
        merge!(genre);
        merge!(year);
        merge!(track_number);
        merge!(disc_number);
        merge!(duration_seconds);
        merge!(play_count);
        merge!(date_added);
        merge!(last_played);
        merge!(purchase_date);
        merge!(persistent_id);
        merge!(format);
        merge!(file_name);
        merge!(raw_path);
        merge!(content_hash);
        if self.raw_record.is_empty() && !other.raw_record.is_empty() {
            self.raw_record = other.raw_record.clone();
        }
    }
}

#[derive(Debug, Clone)]
struct ActivityItem {
    stable_key: String,
    time: Option<String>,
    name: String,
    artist: String,
    album: String,
}

#[derive(Debug, Default)]
struct DailyActivity {
    added: Vec<ActivityItem>,
}

#[derive(Debug, Default)]
struct ActivityCoverage {
    valid_added: usize,
    valid_latest_played: usize,
    missing_added: usize,
    missing_latest_played: usize,
    invalid_added: usize,
    invalid_latest_played: usize,
}

struct ExportSummary<'a> {
    source_kind: MusicSourceKind,
    source_records: usize,
    duplicate_tracks: usize,
    source_playlists: usize,
    duplicate_playlists: usize,
    tracks: &'a [(String, MusicTrack)],
    daily: &'a BTreeMap<String, DailyActivity>,
    activity: &'a ActivityCoverage,
}

#[derive(Debug, PartialEq, Eq)]
enum ActivityDate {
    Missing,
    Invalid,
    Valid(String, Option<String>),
}

pub(crate) fn export_tracks(
    source: &Path,
    source_kind: MusicSourceKind,
    tracks: Vec<MusicTrack>,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    export_tracks_and_playlists(
        source,
        source_kind,
        MusicLibrarySnapshot {
            tracks,
            playlists: Vec::new(),
        },
        package,
        cancel,
        progress,
        callback,
    )
}

pub(crate) fn export_tracks_and_playlists(
    source: &Path,
    source_kind: MusicSourceKind,
    library: MusicLibrarySnapshot,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    let MusicLibrarySnapshot { tracks, playlists } = library;
    progress.total_hint = Some(tracks.len() as u64);
    progress.message = format!(
        "Organizing {} Apple Music tracks, playlists, and Date Added activity",
        tracks.len()
    );
    callback(progress.clone());

    let source_records = tracks.len();
    let mut unique = BTreeMap::<String, MusicTrack>::new();
    let mut duplicate_tracks = 0_usize;
    for track in tracks {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        if track.name.trim().is_empty() {
            record_skip(progress, "music_track_without_name");
            continue;
        }
        let key = track.stable_key();
        if let Some(existing) = unique.get_mut(&key) {
            existing.merge_missing_from(&track);
            duplicate_tracks += 1;
            record_skip(progress, "duplicate_music_track");
        } else {
            unique.insert(key, track);
        }
    }

    let tracks = unique.into_iter().collect::<Vec<_>>();
    let source_playlists = playlists.len();
    let (playlists, duplicate_playlists) = deduplicate_playlists(playlists);
    progress.source_records_included += tracks.len() as u64;
    let (daily, activity_coverage) = daily_activity(&tracks);
    let bucket_files = write_catalog(package, source_kind, &tracks)?;
    let playlist_coverage = write_playlists(package, &playlists, &tracks)?;
    let summary = ExportSummary {
        source_kind,
        source_records,
        duplicate_tracks,
        source_playlists,
        duplicate_playlists,
        tracks: &tracks,
        daily: &daily,
        activity: &activity_coverage,
    };
    write_overview(package, &summary, &bucket_files, &playlist_coverage)?;
    write_coverage(package, &summary, &playlist_coverage)?;

    if cancel.load(Ordering::Relaxed) {
        return Ok(());
    }
    for (date, activity) in daily {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let event = daily_event(source, &date, activity);
        record_event(package, &event, progress, callback)?;
    }
    Ok(())
}

fn deduplicate_playlists(playlists: Vec<MusicPlaylist>) -> (Vec<MusicPlaylist>, usize) {
    let mut unique = BTreeMap::<String, MusicPlaylist>::new();
    let mut duplicates = 0;
    for playlist in playlists {
        let key = if playlist.persistent_id.trim().is_empty() {
            format!(
                "metadata:{}",
                sha256(
                    format!(
                        "{}\n{}",
                        playlist.parent_name.trim().to_lowercase(),
                        playlist.name.trim().to_lowercase()
                    )
                    .as_bytes()
                )
            )
        } else {
            format!(
                "persistent-id:{}",
                playlist.persistent_id.trim().to_ascii_lowercase()
            )
        };
        if let Some(existing) = unique.get_mut(&key) {
            duplicates += 1;
            merge_playlist(existing, playlist);
        } else {
            unique.insert(key, playlist);
        }
    }
    (unique.into_values().collect(), duplicates)
}

fn merge_playlist(existing: &mut MusicPlaylist, other: MusicPlaylist) {
    macro_rules! fill {
        ($field:ident) => {
            if existing.$field.trim().is_empty() && !other.$field.trim().is_empty() {
                existing.$field = other.$field;
            }
        };
    }
    fill!(name);
    fill!(persistent_id);
    fill!(parent_name);
    fill!(parent_persistent_id);
    fill!(playlist_kind);
    fill!(description);
    fill!(duration_seconds);
    existing.smart |= other.smart;
    existing.genius |= other.genius;
    existing.shared |= other.shared;
    existing.visible |= other.visible;
    existing.favorited |= other.favorited;
    let mut members = existing
        .member_persistent_ids
        .iter()
        .map(|id| id.trim().to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    for member in other.member_persistent_ids {
        if members.insert(member.trim().to_ascii_lowercase()) {
            existing.member_persistent_ids.push(member);
        }
    }
}

fn daily_activity(
    tracks: &[(String, MusicTrack)],
) -> (BTreeMap<String, DailyActivity>, ActivityCoverage) {
    let mut days = BTreeMap::<String, DailyActivity>::new();
    let mut coverage = ActivityCoverage::default();
    for (stable_key, track) in tracks {
        match activity_date(&track.date_added) {
            ActivityDate::Valid(date, time) => {
                coverage.valid_added += 1;
                days.entry(date)
                    .or_default()
                    .added
                    .push(activity_item(stable_key, time, track));
            }
            ActivityDate::Missing => coverage.missing_added += 1,
            ActivityDate::Invalid => coverage.invalid_added += 1,
        }
        match activity_date(&track.last_played) {
            ActivityDate::Valid(date, time) => {
                coverage.valid_latest_played += 1;
                let _ = (date, time);
            }
            ActivityDate::Missing => coverage.missing_latest_played += 1,
            ActivityDate::Invalid => coverage.invalid_latest_played += 1,
        }
    }
    for activity in days.values_mut() {
        activity.added.sort_by(activity_sort);
    }
    (days, coverage)
}

fn activity_item(stable_key: &str, time: Option<String>, track: &MusicTrack) -> ActivityItem {
    ActivityItem {
        stable_key: stable_key.into(),
        time,
        name: track.name.trim().into(),
        artist: track.display_artist().into(),
        album: track.display_album().into(),
    }
}

fn activity_sort(left: &ActivityItem, right: &ActivityItem) -> std::cmp::Ordering {
    left.time
        .cmp(&right.time)
        .then_with(|| left.artist.to_lowercase().cmp(&right.artist.to_lowercase()))
        .then_with(|| left.album.to_lowercase().cmp(&right.album.to_lowercase()))
        .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
}

fn activity_date(value: &str) -> ActivityDate {
    if value.trim().is_empty() {
        return ActivityDate::Missing;
    }
    let Some((date, time)) = date_time_parts(value) else {
        return ActivityDate::Invalid;
    };
    let Ok(parsed) = NaiveDate::parse_from_str(&date, "%Y-%m-%d") else {
        return ActivityDate::Invalid;
    };
    let first_real = NaiveDate::parse_from_str(FIRST_REAL_ITUNES_DAY, "%Y-%m-%d").unwrap();
    if parsed < first_real {
        ActivityDate::Invalid
    } else {
        ActivityDate::Valid(date, time)
    }
}

fn write_catalog(
    package: &crate::writer::PackageWriter,
    source_kind: MusicSourceKind,
    tracks: &[(String, MusicTrack)],
) -> Result<Vec<String>, AdapterError> {
    let mut buckets = BTreeMap::<String, Vec<&MusicTrack>>::new();
    for (_, track) in tracks {
        buckets
            .entry(catalog_bucket(track.display_artist()))
            .or_default()
            .push(track);
    }
    let mut files = Vec::new();
    for (bucket, mut bucket_tracks) in buckets {
        bucket_tracks.sort_by(|left, right| {
            left.display_artist()
                .to_lowercase()
                .cmp(&right.display_artist().to_lowercase())
                .then_with(|| {
                    left.display_album()
                        .to_lowercase()
                        .cmp(&right.display_album().to_lowercase())
                })
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
        });
        let file_name = format!("{}.md", bucket.to_ascii_lowercase());
        let mut output = format!(
            "# Apple Music library — {bucket}\n\nSource format: **{}**  \nTracks in this catalog section: **{}**\n\nTrack release years and file timestamps are metadata only; they are never treated as personal Timeline dates.\n\n",
            source_kind.label(),
            bucket_tracks.len()
        );
        let mut current_artist = String::new();
        let mut current_album = String::new();
        for track in bucket_tracks {
            let artist = track.display_artist();
            let album = track.display_album();
            if artist != current_artist {
                current_artist = artist.into();
                current_album.clear();
                output.push_str(&format!("## {}\n\n", heading(artist)));
            }
            if album != current_album {
                current_album = album.into();
                output.push_str(&format!(
                    "### {}\n\n| Track | Year | Genre | Added | Latest known play | Plays | Track / disc | Duration | Format | Stable source ID |\n|---|---:|---|---|---|---:|---|---:|---|---|\n",
                    heading(album)
                ));
            }
            output.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                cell(&track.name),
                cell_or_dash(&track.year),
                cell_or_dash(&track.genre),
                catalog_activity_value(&track.date_added),
                catalog_activity_value(&track.last_played),
                cell_or_dash(&track.play_count),
                track_disc(track),
                duration(track),
                cell_or_dash(&track.format),
                cell(stable_source_id(track)),
            ));
        }
        let relative = PathBuf::from("library").join(&file_name);
        package.write_markdown(&relative, &output)?;
        files.push(file_name);
    }
    Ok(files)
}

fn write_playlists(
    package: &crate::writer::PackageWriter,
    playlists: &[MusicPlaylist],
    tracks: &[(String, MusicTrack)],
) -> Result<PlaylistCoverage, AdapterError> {
    let tracks_by_id = tracks
        .iter()
        .filter_map(|(_, track)| {
            let id = track.persistent_id.trim();
            (!id.is_empty()).then_some((id.to_ascii_lowercase(), track))
        })
        .collect::<BTreeMap<_, _>>();
    let mut sorted = playlists.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        left.parent_name
            .to_lowercase()
            .cmp(&right.parent_name.to_lowercase())
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.persistent_id.cmp(&right.persistent_id))
    });

    let mut coverage = PlaylistCoverage {
        playlists: sorted.len(),
        ..PlaylistCoverage::default()
    };
    let mut overview = format!(
        "# Apple Music playlists\n\nPlaylists preserved: **{}**\n\nThis is a local snapshot of playlist metadata and membership. Music.app does not expose playlist creation dates through its scripting interface, so no playlist date is invented. Each linked member shows the track's library `Date Added`; that value is not claimed as a playlist-membership date.\n\n",
        sorted.len()
    );
    if sorted.is_empty() {
        overview.push_str("No playlist metadata was available from this source.\n");
    }
    for playlist in sorted {
        let identity = if playlist.persistent_id.trim().is_empty() {
            sha256(format!("{}\n{}", playlist.parent_name.trim(), playlist.name.trim()).as_bytes())
        } else {
            playlist.persistent_id.trim().to_ascii_lowercase()
        };
        let name_slug = {
            let value = slug(&playlist.name);
            if value.is_empty() {
                "unnamed-playlist".to_string()
            } else {
                value
            }
        };
        let short_id = identity.chars().take(8).collect::<String>();
        let file_name = format!("{name_slug}-{short_id}.md");
        let relative = PathBuf::from("playlists").join(&file_name);
        let mut body = format!(
            "# {}\n\n| Playlist field | Value |\n|---|---|\n| Parent folder | {} |\n| Kind | {} |\n| Smart playlist | {} |\n| Genius playlist | {} |\n| Shared | {} |\n| Visible in Music | {} |\n| Favorited | {} |\n| Duration | {} |\n| Stable source ID | {} |\n| Source member count | {} |\n\n",
            heading(&playlist.name),
            cell_or_dash(&playlist.parent_name),
            cell_or_dash(&playlist.playlist_kind),
            playlist.smart,
            playlist.genius,
            playlist.shared,
            playlist.visible,
            playlist.favorited,
            duration_value(&playlist.duration_seconds),
            cell_or_dash(&playlist.persistent_id),
            playlist.member_persistent_ids.len(),
        );
        if !playlist.description.trim().is_empty() {
            body.push_str(&format!(
                "## Description\n\n{}\n\n",
                heading(&playlist.description)
            ));
        }
        body.push_str("## Tracks in this playlist\n\n");
        if playlist.member_persistent_ids.is_empty() {
            body.push_str("This playlist contains no tracks in the current Music.app snapshot.\n");
        } else {
            body.push_str("| Track | Artist | Album | Library Date Added | Stable source ID |\n|---|---|---|---|---|\n");
            for member_id in &playlist.member_persistent_ids {
                coverage.memberships += 1;
                if let Some(track) = tracks_by_id.get(&member_id.trim().to_ascii_lowercase()) {
                    coverage.linked_memberships += 1;
                    body.push_str(&format!(
                        "| {} | {} | {} | {} | {} |\n",
                        cell(&track.name),
                        cell(track.display_artist()),
                        cell(track.display_album()),
                        catalog_activity_value(&track.date_added),
                        cell(stable_source_id(track)),
                    ));
                } else {
                    coverage.unmatched_memberships += 1;
                    body.push_str(&format!(
                        "| Track not present in library catalog | — | — | — | {} |\n",
                        cell(member_id),
                    ));
                }
            }
        }
        package.write_markdown(&relative, &body)?;
        overview.push_str(&format!(
            "- [{}](playlists/{file_name}) — {} {}\n",
            inline(&playlist.name),
            playlist.member_persistent_ids.len(),
            if playlist.member_persistent_ids.len() == 1 {
                "track"
            } else {
                "tracks"
            }
        ));
        coverage.files.push(file_name);
    }
    package.write_markdown(Path::new("PLAYLISTS-OVERVIEW.md"), &overview)?;
    Ok(coverage)
}

fn duration_value(seconds: &str) -> String {
    seconds
        .trim()
        .parse::<u64>()
        .ok()
        .map(|total| {
            format!(
                "{}:{:02}:{:02}",
                total / 3600,
                (total % 3600) / 60,
                total % 60
            )
        })
        .unwrap_or_else(|| "—".into())
}

fn write_overview(
    package: &crate::writer::PackageWriter,
    summary: &ExportSummary<'_>,
    bucket_files: &[String],
    playlist_coverage: &PlaylistCoverage,
) -> Result<(), AdapterError> {
    let source_records = summary.source_records;
    let duplicate_tracks = summary.duplicate_tracks;
    let source_playlists = summary.source_playlists;
    let duplicate_playlists = summary.duplicate_playlists;
    let links = bucket_files
        .iter()
        .map(|file| format!("- [{}](library/{file})", file.trim_end_matches(".md")))
        .collect::<Vec<_>>()
        .join("\n");
    let content = format!(
        "# Apple Music overview\n\n## Recognized source\n\n**{}**\n\n## Result\n\n- Source track records inspected: **{source_records}**\n- Unique tracks preserved: **{}**\n- Duplicate source tracks consolidated: **{duplicate_tracks}**\n- Source playlists inspected: **{source_playlists}**\n- Unique playlists preserved: **{}**\n- Duplicate source playlists consolidated: **{duplicate_playlists}**\n- Playlist memberships preserved: **{}**\n- Daily `Date Added` Timeline files: **{}**\n- Tracks with a genuine source `Date Added`: **{}**\n- Tracks with a genuine source `Last Played`: **{}**\n- Invalid/sentinel activity dates excluded: **{}**\n- Network used: **No**\n\n## Library catalog\n\n{links}\n\n## Playlists\n\n[Open the playlist catalog](PLAYLISTS-OVERVIEW.md)\n\nEach playlist file preserves its visible Music.app metadata and track membership. Every linked member includes the track's genuine library `Date Added`; that date is not misrepresented as the playlist's creation date.\n\n## Timeline meaning\n\nEach dated Apple Music event is a daily summary of tracks whose explicit Music.app **Date Added** falls on that day. `Last Played` remains secondary catalog metadata and never creates or reorders Timeline events. Music.app does not expose playlist creation dates through its local scripting interface, so playlists remain timeless metadata. Track release years, folder names, file creation dates, file modification dates, and import time are never promoted into Timeline dates.\n\nThe catalog and playlist files remain outside Timeline; Apple Music Timeline entries contain `Date Added` evidence only.\n\n## More detail\n\n- [Coverage and date-accounting report](APPLE-MUSIC-COVERAGE.md)\n- [Portable package guide](README.md)\n",
        summary.source_kind.label(),
        summary.tracks.len(),
        playlist_coverage.playlists,
        playlist_coverage.memberships,
        summary.daily.len(),
        summary.activity.valid_added,
        summary.activity.valid_latest_played,
        summary.activity.invalid_added + summary.activity.invalid_latest_played,
    );
    package.write_markdown(Path::new("APPLE-MUSIC-OVERVIEW.md"), &content)
}

fn write_coverage(
    package: &crate::writer::PackageWriter,
    summary: &ExportSummary<'_>,
    playlist_coverage: &PlaylistCoverage,
) -> Result<(), AdapterError> {
    let source_records = summary.source_records;
    let duplicate_tracks = summary.duplicate_tracks;
    let source_playlists = summary.source_playlists;
    let duplicate_playlists = summary.duplicate_playlists;
    let content = format!(
        "# Apple Music coverage and date accounting\n\nSource format: **{}**\n\n| Check | Count |\n|---|---:|\n| Source track records inspected | {source_records} |\n| Unique tracks preserved in Markdown | {} |\n| Duplicate source tracks consolidated | {duplicate_tracks} |\n| Source playlists inspected | {source_playlists} |\n| Unique playlists preserved | {} |\n| Duplicate source playlists consolidated | {duplicate_playlists} |\n| Playlist memberships preserved | {} |\n| Playlist memberships linked to catalog tracks | {} |\n| Playlist memberships not present in the library catalog | {} |\n| Daily Date Added Timeline files | {} |\n| Valid source Date Added values | {} |\n| Missing Date Added values | {} |\n| Invalid/sentinel Date Added values excluded | {} |\n| Valid source Last Played values (catalog only) | {} |\n| Missing Last Played values | {} |\n| Invalid/sentinel Last Played values excluded | {} |\n\n## Rules applied\n\n- Persistent ID is the primary track identity when present.\n- Otherwise Unfold uses a stable content hash or normalized metadata fingerprint.\n- Apple Music/iTunes activity dates earlier than `{FIRST_REAL_ITUNES_DAY}` are invalid for this domain and are not placed on the Timeline. This removes the known `1904-01-01` empty-date sentinel without inventing a replacement.\n- Only explicit Music.app `Date Added` creates Timeline events.\n- `Last Played` means the latest value in this library snapshot; it remains secondary catalog metadata and never creates or reorders Timeline events.\n- Catalog and playlist Markdown are not Timeline events.\n- Playlist membership is preserved as a snapshot. Music.app exposes no playlist creation date through its local scripting interface, so no playlist date is invented.\n- A playlist member's `Date Added` is the track's library date, not a claimed playlist-membership date.\n- Release year is catalog metadata only.\n- Filesystem creation/modification times, folder names, archive names, and extraction time never become event dates.\n- Local audio tags are read through local macOS metadata where available; filename/folder fallbacks remain visibly labelled as fallbacks.\n- All output is local and `networkUsed` remains `false`.\n",
        summary.source_kind.label(),
        summary.tracks.len(),
        playlist_coverage.playlists,
        playlist_coverage.memberships,
        playlist_coverage.linked_memberships,
        playlist_coverage.unmatched_memberships,
        summary.daily.len(),
        summary.activity.valid_added,
        summary.activity.missing_added,
        summary.activity.invalid_added,
        summary.activity.valid_latest_played,
        summary.activity.missing_latest_played,
        summary.activity.invalid_latest_played,
    );
    package.write_markdown(Path::new("APPLE-MUSIC-COVERAGE.md"), &content)
}

fn daily_event(source: &Path, date: &str, activity: DailyActivity) -> OpenEvent {
    let added_count = activity.added.len();
    let mut body = format!(
        "Tracks added to the Apple Music library on this day, preserved from Music.app's explicit `Date Added` field.\n\n- Added to library: {added_count}\n"
    );
    if !activity.added.is_empty() {
        body.push_str("\n## Added to library\n\n");
        for item in &activity.added {
            body.push_str(&activity_line(item));
        }
    }
    body.push_str("\n## Evidence rule\n\nThis date comes only from the track's explicit Music.app `Date Added` value. `Last Played`, release year, filesystem timestamps, folder names, and extraction time are not used as Timeline dates.\n");
    let raw_record = body.as_bytes().to_vec();
    let identity = activity
        .added
        .iter()
        .map(|item| format!("added:{}", item.stable_key))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join("\n");
    let raw_path = format!("{}#Apple-Music/date-added/{date}", source.display());
    OpenEvent::new_with_identity(
        EventInput {
            date,
            time: None,
            end_date: None,
            source: "audio",
            module: "apple-music",
            event_type: "daily_music_additions",
            title: &format!("Apple Music · tracks added — {date}"),
            raw_path: &raw_path,
            raw_bytes: &raw_record,
            body,
            actors: Vec::new(),
            places: Vec::new(),
            tags: vec!["music".into(), "apple-music".into(), "date-added".into()],
            confidence: "high",
        },
        format!("apple-music-date-added\n{date}\n{identity}").as_bytes(),
    )
}

fn activity_line(item: &ActivityItem) -> String {
    let time = item
        .time
        .as_deref()
        .map(|value| format!("{value} · "))
        .unwrap_or_default();
    format!(
        "- {time}**{}** — **{}** · {}\n",
        inline(&item.name),
        inline(&item.artist),
        inline(&item.album)
    )
}

fn catalog_bucket(artist: &str) -> String {
    artist
        .chars()
        .find(|character| character.is_alphanumeric())
        .map(|character| {
            let upper = character.to_ascii_uppercase();
            if upper.is_ascii_alphabetic() {
                upper.to_string()
            } else {
                "0-9-and-other".into()
            }
        })
        .unwrap_or_else(|| "0-9-and-other".into())
}

fn stable_source_id(track: &MusicTrack) -> &str {
    first_present(
        &[&track.persistent_id, &track.content_hash],
        "metadata fingerprint",
    )
}

fn track_disc(track: &MusicTrack) -> String {
    match (track.track_number.trim(), track.disc_number.trim()) {
        ("", "") => "—".into(),
        (track, "") => cell(track),
        ("", disc) => format!("— / {}", cell(disc)),
        (track, disc) => format!("{} / {}", cell(track), cell(disc)),
    }
}

fn duration(track: &MusicTrack) -> String {
    track
        .duration_seconds
        .trim()
        .parse::<f64>()
        .ok()
        .map(|seconds| {
            let rounded = seconds.round() as u64;
            format!("{}:{:02}", rounded / 60, rounded % 60)
        })
        .unwrap_or_else(|| "—".into())
}

fn catalog_activity_value(value: &str) -> String {
    match activity_date(value) {
        ActivityDate::Missing => "—".into(),
        ActivityDate::Invalid => "invalid/sentinel — excluded".into(),
        ActivityDate::Valid(_, _) => cell(value),
    }
}

fn first_present<'a>(values: &[&'a String], fallback: &'a str) -> &'a str {
    values
        .iter()
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
        .unwrap_or(fallback)
}

fn heading(value: &str) -> String {
    value.replace(['\n', '\r'], " ").trim().into()
}

fn inline(value: &str) -> String {
    value
        .replace(['\n', '\r'], " ")
        .replace('*', "\\*")
        .trim()
        .into()
}

fn cell(value: &str) -> String {
    value
        .replace(['\n', '\r'], " ")
        .replace('|', "\\|")
        .trim()
        .into()
}

fn cell_or_dash(value: &str) -> String {
    if value.trim().is_empty() {
        "—".into()
    } else {
        cell(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(name: &str, id: &str, added: &str, played: &str) -> MusicTrack {
        MusicTrack {
            name: name.into(),
            artist: "Open Artist".into(),
            album: "Open Album".into(),
            persistent_id: id.into(),
            date_added: added.into(),
            last_played: played.into(),
            ..MusicTrack::default()
        }
    }

    #[test]
    fn apple_1904_empty_date_sentinel_never_becomes_timeline_activity() {
        assert_eq!(activity_date("1904-01-01T00:00"), ActivityDate::Invalid);
        assert_eq!(activity_date(""), ActivityDate::Missing);
        assert_eq!(
            activity_date("2026-08-05T12:34"),
            ActivityDate::Valid("2026-08-05".into(), Some("12:34".into()))
        );
    }

    #[test]
    fn daily_activity_uses_date_added_only_without_track_spam() {
        let tracks = vec![
            (
                "persistent-id:one".into(),
                track("First", "ONE", "2024-01-02T10:00", "2024-01-03T11:00"),
            ),
            (
                "persistent-id:two".into(),
                track("Second", "TWO", "2024-01-02T12:00", "1904-01-01T00:00"),
            ),
        ];
        let (days, coverage) = daily_activity(&tracks);

        assert_eq!(days.len(), 1);
        assert_eq!(days["2024-01-02"].added.len(), 2);
        assert!(!days.contains_key("2024-01-03"));
        assert_eq!(coverage.valid_latest_played, 1);
        assert_eq!(coverage.invalid_latest_played, 1);
    }

    #[test]
    fn persistent_id_is_cross_input_identity_and_dates_do_not_change_it() {
        let first = track("First", "ABC123", "2024-01-02", "2024-01-03");
        let second = track("First", "abc123", "2025-02-03", "2025-02-04");

        assert_eq!(first.stable_key(), second.stable_key());
    }

    #[test]
    fn playlist_duplicates_merge_memberships_by_stable_source_id() {
        let first = MusicPlaylist {
            name: "Open Mix".into(),
            persistent_id: "PLAYLIST01".into(),
            member_persistent_ids: vec!["TRACK01".into()],
            ..MusicPlaylist::default()
        };
        let second = MusicPlaylist {
            name: "Open Mix".into(),
            persistent_id: "playlist01".into(),
            shared: true,
            member_persistent_ids: vec!["track01".into(), "TRACK02".into()],
            ..MusicPlaylist::default()
        };

        let (playlists, duplicates) = deduplicate_playlists(vec![first, second]);

        assert_eq!(duplicates, 1);
        assert_eq!(playlists.len(), 1);
        assert!(playlists[0].shared);
        assert_eq!(playlists[0].member_persistent_ids, ["TRACK01", "TRACK02"]);
    }

    #[test]
    fn playlist_markdown_links_tracks_and_labels_library_date_added() {
        let destination = tempfile::tempdir().unwrap();
        let source_root = tempfile::tempdir().unwrap();
        let source = source_root.path().join("Music Library.musiclibrary");
        std::fs::create_dir(&source).unwrap();
        let package = crate::writer::PackageWriter::new(
            destination.path(),
            &source,
            "apple-music-local-library",
        )
        .unwrap();
        let tracks = vec![(
            "persistent-id:track01".into(),
            track(
                "First Song",
                "TRACK01",
                "2024-01-02T10:00",
                "2026-08-05T12:00",
            ),
        )];
        let playlists = vec![MusicPlaylist {
            name: "Open Mix".into(),
            persistent_id: "PLAYLIST01".into(),
            member_persistent_ids: vec!["TRACK01".into()],
            ..MusicPlaylist::default()
        }];

        let coverage = write_playlists(&package, &playlists, &tracks).unwrap();
        let markdown =
            std::fs::read_to_string(package.root().join("playlists/open-mix-playlist.md")).unwrap();

        assert_eq!(coverage.playlists, 1);
        assert_eq!(coverage.linked_memberships, 1);
        assert!(markdown.contains("| Library Date Added |"));
        assert!(markdown.contains("2024-01-02T10:00"));
        assert!(!markdown.contains("2026-08-05T12:00"));
    }
}
