// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::apple_music_common::{
    export_tracks, export_tracks_and_playlists, MusicLibrarySnapshot, MusicPlaylist,
    MusicSourceKind, MusicTrack,
};
use crate::{model::sha256_file, AdapterError, ExtractionProgress, ProgressCallback};
use id3::TagLike;
use roxmltree::Document;
use std::{
    collections::HashMap,
    fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
};
use walkdir::WalkDir;

const AUDIO_EXTENSIONS: &[&str] = &["mp3", "m4a", "aac", "wav", "aiff", "aif", "flac", "alac"];
const MUSIC_APP_HEADER: &str = "Name\tArtist\tAlbumArtist\tAlbum\tGenre\tYear\tTrackNumber\tDiscNumber\tDurationSeconds\tPlayCount\tDateAdded\tLastPlayed\tPersistentID\tKind";
const MUSIC_APP_PLAYLIST_MARKER: &str = "__AMARA_PLAYLISTS__";
const MUSIC_APP_PLAYLIST_HEADER: &str = "Name\tPersistentID\tParentName\tParentPersistentID\tKind\tSmart\tGenius\tShared\tVisible\tFavorited\tDescription\tDurationSeconds\tMemberPersistentIDs";

pub fn extract(
    source: &Path,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    let audio = audio_files(source);
    if audio.is_empty() && is_music_library_bundle(source) {
        return extract_music_app_library(source, package, cancel, progress, callback);
    }

    progress.total_hint = Some(audio.len() as u64);
    callback(progress.clone());
    let mut tracks = Vec::with_capacity(audio.len());
    for path in audio {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        progress.current_item = Some(path.to_string_lossy().into_owned());
        progress.message = format!("Reading embedded audio metadata · {}", path.display());
        callback(progress.clone());
        tracks.push(local_audio_track(source, &path)?);
    }
    export_tracks(
        source,
        MusicSourceKind::LocalAudio,
        tracks,
        package,
        cancel,
        progress,
        callback,
    )
}

fn extract_music_app_library(
    source: &Path,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    progress.message = "Reading Apple Music library metadata via Music.app".into();
    callback(progress.clone());
    let export = music_app_export()?;
    progress.total_hint = Some(export.track_rows.len() as u64);
    callback(progress.clone());
    let tracks = export
        .track_rows
        .into_iter()
        .map(|row| {
            let parsed = parse_music_app_row(&row);
            let raw_path = if parsed.persistent_id.trim().is_empty() {
                format!("{}#Music.app/library/metadata-row", source.display())
            } else {
                format!(
                    "{}#Music.app/library/{}",
                    source.display(),
                    parsed.persistent_id.trim()
                )
            };
            MusicTrack {
                name: parsed.name.into(),
                artist: parsed.artist.into(),
                album_artist: parsed.album_artist.into(),
                album: parsed.album.into(),
                genre: parsed.genre.into(),
                year: parsed.year.into(),
                track_number: parsed.track_number.into(),
                disc_number: parsed.disc_number.into(),
                duration_seconds: parsed.duration_seconds.into(),
                play_count: parsed.play_count.into(),
                date_added: parsed.date_added.into(),
                last_played: parsed.last_played.into(),
                persistent_id: parsed.persistent_id.into(),
                format: parsed.kind.into(),
                raw_path,
                raw_record: row.into_bytes(),
                ..MusicTrack::default()
            }
        })
        .collect::<Vec<_>>();
    export_tracks_and_playlists(
        source,
        MusicSourceKind::MusicApp,
        MusicLibrarySnapshot {
            tracks,
            playlists: export.playlists,
        },
        package,
        cancel,
        progress,
        callback,
    )
}

fn local_audio_track(source: &Path, path: &Path) -> Result<MusicTrack, AdapterError> {
    let metadata = fs::metadata(path).map_err(|error| AdapterError::Input(error.to_string()))?;
    let mut spotlight = spotlight_metadata(path);
    let embedded = mp3_metadata(path);
    let embedded_available = !embedded.is_empty();
    // Direct tags take precedence over potentially stale or absent Spotlight data.
    spotlight.extend(embedded);
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Audio file");
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(filename);
    let folder_parts = path
        .parent()
        .and_then(|parent| parent.strip_prefix(source).ok())
        .map(|relative| {
            relative
                .components()
                .filter_map(|part| part.as_os_str().to_str())
                .filter(|part| !part.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let artist_fallback = folder_parts.first().cloned().unwrap_or_default();
    let album_fallback = folder_parts.get(1).cloned().unwrap_or_default();
    let content_hash = sha256_file(path).map_err(|error| AdapterError::Input(error.to_string()))?;
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let raw_record = format!(
        "content_sha256={content_hash}\nsize={}\nspotlight_or_embedded_metadata_available={}\nembedded_id3_metadata_available={embedded_available}",
        metadata.len(),
        !spotlight.is_empty()
    )
    .into_bytes();
    Ok(MusicTrack {
        name: metadata_value(&spotlight, "kMDItemTitle")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(stem)
            .into(),
        artist: metadata_value(&spotlight, "kMDItemAuthors")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&artist_fallback)
            .into(),
        album_artist: metadata_value(&spotlight, "album_artist")
            .unwrap_or_default()
            .into(),
        disc_number: metadata_value(&spotlight, "disc_number")
            .unwrap_or_default()
            .into(),
        album: metadata_value(&spotlight, "kMDItemAlbum")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&album_fallback)
            .into(),
        genre: metadata_value(&spotlight, "kMDItemMusicalGenre")
            .unwrap_or_default()
            .into(),
        year: metadata_value(&spotlight, "kMDItemRecordingYear")
            .unwrap_or_default()
            .into(),
        track_number: metadata_value(&spotlight, "kMDItemAudioTrackNumber")
            .unwrap_or_default()
            .into(),
        duration_seconds: metadata_value(&spotlight, "kMDItemDurationSeconds")
            .unwrap_or_default()
            .into(),
        purchase_date: metadata_value(&spotlight, "kMDItemPurchaseDate")
            .unwrap_or_default()
            .into(),
        format: extension,
        file_name: filename.into(),
        raw_path: path.to_string_lossy().into_owned(),
        raw_record,
        content_hash,
        // Spotlight's DateAdded/creation/modification fields often describe the
        // filesystem copy, not a truthful Music-library activity. They remain
        // deliberately absent from Timeline dating for loose audio folders.
        date_added: String::new(),
        last_played: String::new(),
        ..MusicTrack::default()
    })
}

// Bound untrusted tag input before decoding. Compressed/encrypted/unsynchronised
// tags fall back to Spotlight/path metadata; never decompress arbitrary input.
fn mp3_metadata(path: &Path) -> HashMap<String, Vec<String>> {
    fn decode(path: &Path) -> Option<id3::Tag> {
        if !path.extension()?.to_str()?.eq_ignore_ascii_case("mp3") {
            return None;
        }
        let mut file = fs::File::open(path).ok()?;
        let mut header = [0_u8; 10];
        file.read_exact(&mut header).ok()?;
        if &header[..3] != b"ID3" || !matches!(header[3], 3 | 4) || header[5] != 0 {
            return None;
        }
        let synchsafe = |bytes: &[u8]| -> Option<usize> {
            bytes.iter().try_fold(0_usize, |n, &b| {
                (b < 128).then_some((n << 7) | usize::from(b))
            })
        };
        let size = synchsafe(&header[6..10])?;
        if size > 8 * 1024 * 1024 {
            return None;
        }
        let mut bytes = vec![0; 10 + size];
        bytes[..10].copy_from_slice(&header);
        file.read_exact(&mut bytes[10..]).ok()?;
        let mut offset = 10;
        while offset < bytes.len() {
            let rest = &bytes[offset..];
            if rest[0] == 0 {
                if rest.iter().any(|&byte| byte != 0) {
                    return None;
                }
                break;
            }
            if rest.len() < 10
                || rest[9] != 0
                || !rest[..4]
                    .iter()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
            {
                return None;
            }
            let frame_size = if header[3] == 4 {
                synchsafe(&rest[4..8])?
            } else {
                u32::from_be_bytes(rest[4..8].try_into().ok()?) as usize
            };
            if frame_size == 0 || frame_size > rest.len() - 10 {
                return None;
            }
            offset += 10 + frame_size;
        }
        id3::Tag::read_from2(Cursor::new(bytes)).ok()
    }
    let Some(tag) = decode(path) else {
        return HashMap::new();
    };
    let mut values = HashMap::new();
    for (key, value) in [
        ("kMDItemTitle", tag.title()),
        ("kMDItemAuthors", tag.artist()),
        ("kMDItemAlbum", tag.album()),
        ("album_artist", tag.album_artist()),
        ("kMDItemMusicalGenre", tag.genre()),
    ] {
        if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
            values.insert(key.into(), vec![value.to_owned()]);
        }
    }
    for (key, value) in [
        (
            "kMDItemRecordingYear",
            tag.date_recorded()
                .map(|date| date.year)
                .or_else(|| tag.year())
                .map(|n| n.to_string()),
        ),
        (
            "kMDItemAudioTrackNumber",
            tag.track().map(|n| n.to_string()),
        ),
        ("disc_number", tag.disc().map(|n| n.to_string())),
        (
            "kMDItemDurationSeconds",
            tag.duration()
                .map(|ms| (f64::from(ms) / 1000.0).to_string()),
        ),
    ] {
        if let Some(value) = value {
            values.insert(key.into(), vec![value]);
        }
    }
    // Generic ID3 timestamps (including custom DateAdded) are not library activity.
    values
}

fn spotlight_metadata(path: &Path) -> HashMap<String, Vec<String>> {
    let Ok(output) = Command::new("mdls")
        .arg("-plist")
        .arg("-")
        .arg(path)
        .output()
    else {
        return HashMap::new();
    };
    if !output.status.success() {
        return HashMap::new();
    }
    parse_spotlight_plist(&String::from_utf8_lossy(&output.stdout))
}

fn parse_spotlight_plist(xml: &str) -> HashMap<String, Vec<String>> {
    let Ok(document) = Document::parse(xml) else {
        return HashMap::new();
    };
    let Some(dictionary) = document
        .descendants()
        .find(|node| node.has_tag_name("dict"))
    else {
        return HashMap::new();
    };
    let elements = dictionary
        .children()
        .filter(|node| node.is_element())
        .collect::<Vec<_>>();
    let mut output = HashMap::new();
    let mut index = 0;
    while index + 1 < elements.len() {
        let key = elements[index];
        let value = elements[index + 1];
        if key.has_tag_name("key") {
            let values = if value.has_tag_name("array") {
                value
                    .children()
                    .filter(|node| node.is_element())
                    .filter_map(|node| node.text())
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            } else {
                value
                    .text()
                    .map(|value| vec![value.into()])
                    .unwrap_or_default()
            };
            output.insert(key.text().unwrap_or_default().into(), values);
        }
        index += 2;
    }
    output
}

fn metadata_value<'a>(metadata: &'a HashMap<String, Vec<String>>, key: &str) -> Option<&'a str> {
    metadata
        .get(key)
        .and_then(|values| values.first())
        .map(String::as_str)
}

fn audio_files(source: &Path) -> Vec<PathBuf> {
    WalkDir::new(source)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| {
            path.extension()
                .and_then(|value| value.to_str())
                .is_some_and(|extension| {
                    AUDIO_EXTENSIONS
                        .iter()
                        .any(|candidate| extension.eq_ignore_ascii_case(candidate))
                })
        })
        .collect()
}

fn is_music_library_bundle(source: &Path) -> bool {
    source
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("musiclibrary"))
        || source.join("Library.musicdb").is_file()
        || source.join("Application.musicdb").is_file()
}

#[derive(Debug, Default)]
struct MusicAppExport {
    track_rows: Vec<String>,
    playlists: Vec<MusicPlaylist>,
}

fn music_app_export() -> Result<MusicAppExport, AdapterError> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(MUSIC_APP_SCRIPT)
        .output()
        .map_err(|error| AdapterError::Input(format!("could not run Music.app reader: {error}")))?;
    if !output.status.success() {
        return Err(AdapterError::Input(music_app_failure_message(
            &String::from_utf8_lossy(&output.stderr),
        )));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut export = MusicAppExport::default();
    let mut playlists = false;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        if line == MUSIC_APP_PLAYLIST_MARKER {
            playlists = true;
            continue;
        }
        if line == MUSIC_APP_HEADER || line == MUSIC_APP_PLAYLIST_HEADER {
            continue;
        }
        if playlists {
            export.playlists.push(parse_music_app_playlist_row(line));
        } else {
            export.track_rows.push(line.to_owned());
        }
    }
    Ok(export)
}

fn parse_music_app_playlist_row(row: &str) -> MusicPlaylist {
    let columns = row.split('\t').collect::<Vec<_>>();
    MusicPlaylist {
        name: columns.first().copied().unwrap_or("").into(),
        persistent_id: columns.get(1).copied().unwrap_or("").into(),
        parent_name: columns.get(2).copied().unwrap_or("").into(),
        parent_persistent_id: columns.get(3).copied().unwrap_or("").into(),
        playlist_kind: columns.get(4).copied().unwrap_or("").into(),
        smart: parse_music_bool(columns.get(5).copied().unwrap_or("")),
        genius: parse_music_bool(columns.get(6).copied().unwrap_or("")),
        shared: parse_music_bool(columns.get(7).copied().unwrap_or("")),
        visible: parse_music_bool(columns.get(8).copied().unwrap_or("")),
        favorited: parse_music_bool(columns.get(9).copied().unwrap_or("")),
        description: columns.get(10).copied().unwrap_or("").into(),
        duration_seconds: columns.get(11).copied().unwrap_or("").into(),
        member_persistent_ids: columns
            .get(12)
            .copied()
            .unwrap_or("")
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect(),
    }
}

fn parse_music_bool(value: &str) -> bool {
    value.eq_ignore_ascii_case("true")
}

#[derive(Debug, Default, PartialEq, Eq)]
struct MusicAppTrack<'a> {
    name: &'a str,
    artist: &'a str,
    album_artist: &'a str,
    album: &'a str,
    genre: &'a str,
    year: &'a str,
    track_number: &'a str,
    disc_number: &'a str,
    duration_seconds: &'a str,
    play_count: &'a str,
    date_added: &'a str,
    last_played: &'a str,
    persistent_id: &'a str,
    kind: &'a str,
}

fn parse_music_app_row(row: &str) -> MusicAppTrack<'_> {
    let columns = row.split('\t').collect::<Vec<_>>();
    MusicAppTrack {
        name: columns.first().copied().unwrap_or(""),
        artist: columns.get(1).copied().unwrap_or(""),
        album_artist: columns.get(2).copied().unwrap_or(""),
        album: columns.get(3).copied().unwrap_or(""),
        genre: columns.get(4).copied().unwrap_or(""),
        year: columns.get(5).copied().unwrap_or(""),
        track_number: columns.get(6).copied().unwrap_or(""),
        disc_number: columns.get(7).copied().unwrap_or(""),
        duration_seconds: columns.get(8).copied().unwrap_or(""),
        play_count: columns.get(9).copied().unwrap_or(""),
        date_added: columns.get(10).copied().unwrap_or(""),
        last_played: columns.get(11).copied().unwrap_or(""),
        persistent_id: columns.get(12).copied().unwrap_or(""),
        kind: columns.get(13).copied().unwrap_or(""),
    }
}

fn music_app_failure_message(stderr: &str) -> String {
    let detail = stderr.trim();
    let explanation = if detail.contains("not authorized")
        || detail.contains("Not authorized")
        || detail.contains("Operation not permitted")
        || detail.contains("-1743")
    {
        "macOS denied access to Music.app automation"
    } else {
        "Music.app could not provide library metadata"
    };
    format!(
        "{explanation}. Unfold only needs one-time, read-only Music Automation for this export. Open System Settings › Privacy & Security › Automation, allow Amara Unfold to control Music, then run the export again. After a successful export, Unfold asks macOS to forget that approval where supported; if macOS keeps it, revoke it manually. Detail: {}",
        if detail.is_empty() { "no system detail returned" } else { detail }
    )
}

const MUSIC_APP_SCRIPT: &str = r#"
on pad2(n)
	set s to n as text
	if (count of s) is 1 then return "0" & s
	return s
end pad2

on isoDateTime(d)
	try
		set y to year of d as integer
		set m to month of d as integer
		set dd to day of d as integer
		set hh to hours of d as integer
		set mm to minutes of d as integer
		return (y as text) & "-" & my pad2(m) & "-" & my pad2(dd) & "T" & my pad2(hh) & ":" & my pad2(mm)
	on error
		return ""
	end try
end isoDateTime

on replaceText(sourceText, findText, replaceText)
	set oldDelimiters to AppleScript's text item delimiters
	set AppleScript's text item delimiters to findText
	set textItems to text items of sourceText
	set AppleScript's text item delimiters to replaceText
	set joinedText to textItems as text
	set AppleScript's text item delimiters to oldDelimiters
	return joinedText
end replaceText

on clean(value)
	try
		set output to value as text
	on error
		set output to ""
	end try
	set output to my replaceText(output, tab, " ")
	set output to my replaceText(output, return, " ")
	set output to my replaceText(output, linefeed, " ")
	return output
end clean

on joinClean(values, delimiterText)
	set cleanedValues to {}
	repeat with value in values
		set end of cleanedValues to my clean(value)
	end repeat
	set oldDelimiters to AppleScript's text item delimiters
	set AppleScript's text item delimiters to delimiterText
	set joinedText to cleanedValues as text
	set AppleScript's text item delimiters to oldDelimiters
	return joinedText
end joinClean

set outputLines to {"Name" & tab & "Artist" & tab & "AlbumArtist" & tab & "Album" & tab & "Genre" & tab & "Year" & tab & "TrackNumber" & tab & "DiscNumber" & tab & "DurationSeconds" & tab & "PlayCount" & tab & "DateAdded" & tab & "LastPlayed" & tab & "PersistentID" & tab & "Kind"}
tell application "Music"
	set trackCount to count of tracks of library playlist 1
	set trackNames to name of every track of library playlist 1
	set trackArtists to artist of every track of library playlist 1
	set trackAlbumArtists to album artist of every track of library playlist 1
	set trackAlbums to album of every track of library playlist 1
	set trackGenres to genre of every track of library playlist 1
	set trackYears to year of every track of library playlist 1
	set trackNumbers to track number of every track of library playlist 1
	set trackDiscNumbers to disc number of every track of library playlist 1
	set trackDurations to duration of every track of library playlist 1
	set trackPlayCounts to played count of every track of library playlist 1
	set trackDateAdded to date added of every track of library playlist 1
	set trackLastPlayed to played date of every track of library playlist 1
	set trackPersistentIds to persistent ID of every track of library playlist 1
	set trackKinds to kind of every track of library playlist 1
end tell
repeat with i from 1 to trackCount
	set fields to {my clean(item i of trackNames), my clean(item i of trackArtists), my clean(item i of trackAlbumArtists), my clean(item i of trackAlbums), my clean(item i of trackGenres), my clean(item i of trackYears), my clean(item i of trackNumbers), my clean(item i of trackDiscNumbers), my clean(item i of trackDurations), my clean(item i of trackPlayCounts), my isoDateTime(item i of trackDateAdded), my isoDateTime(item i of trackLastPlayed), my clean(item i of trackPersistentIds), my clean(item i of trackKinds)}
	set AppleScript's text item delimiters to tab
	set end of outputLines to fields as text
end repeat
set end of outputLines to "__AMARA_PLAYLISTS__"
set end of outputLines to "Name" & tab & "PersistentID" & tab & "ParentName" & tab & "ParentPersistentID" & tab & "Kind" & tab & "Smart" & tab & "Genius" & tab & "Shared" & tab & "Visible" & tab & "Favorited" & tab & "Description" & tab & "DurationSeconds" & tab & "MemberPersistentIDs"
tell application "Music"
	set candidatePlaylists to (every user playlist) & (every subscription playlist)
end tell
repeat with playlistItem in candidatePlaylists
	set playlistName to ""
	set playlistId to ""
	set parentName to ""
	set parentId to ""
	set playlistKind to "playlist"
	set isSmart to false
	set isGenius to false
	set isShared to false
	set isVisible to false
	set isFavorited to false
	set playlistDescription to ""
	set playlistDuration to ""
	set memberIds to {}
	tell application "Music"
		try
			set playlistName to name of playlistItem
		end try
		try
			set playlistId to persistent ID of playlistItem
		end try
		try
			set parentName to name of parent of playlistItem
			set parentId to persistent ID of parent of playlistItem
		end try
		try
			set playlistKind to class of playlistItem as text
		end try
		try
			set isSmart to smart of playlistItem
		end try
		try
			set isGenius to genius of playlistItem
		end try
		try
			set isShared to shared of playlistItem
		end try
		try
			set isVisible to visible of playlistItem
		end try
		try
			set isFavorited to favorited of playlistItem
		end try
		try
			set playlistDescription to description of playlistItem
		end try
		try
			set playlistDuration to duration of playlistItem
		end try
		try
			set memberIds to persistent ID of every track of playlistItem
		end try
	end tell
	set fields to {my clean(playlistName), my clean(playlistId), my clean(parentName), my clean(parentId), my clean(playlistKind), my clean(isSmart), my clean(isGenius), my clean(isShared), my clean(isVisible), my clean(isFavorited), my clean(playlistDescription), my clean(playlistDuration), my joinClean(memberIds, ",")}
	set AppleScript's text item delimiters to tab
	set end of outputLines to fields as text
end repeat
set AppleScript's text item delimiters to linefeed
return outputLines as text
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actual_id3_tags_override_decoy_paths_without_creating_activity_dates() {
        let directory = tempfile::tempdir().unwrap();
        let folder = directory.path().join("Decoy Artist/Decoy Album");
        fs::create_dir_all(&folder).unwrap();
        for version in [id3::Version::Id3v23, id3::Version::Id3v24] {
            let path = folder.join("decoy.mp3");
            let mut tag = id3::Tag::new();
            tag.set_title("Tagged Café Song");
            tag.set_artist("Embedded Artist");
            tag.set_album("Embedded Album");
            tag.set_year(2007);
            tag.set_genre("Synthetic");
            tag.set_track(3);
            tag.add_frame(id3::frame::ExtendedText {
                description: "DateAdded".into(),
                value: "2099-01-01".into(),
            });
            let mut bytes = Vec::new();
            tag.write_to(&mut bytes, version).unwrap();
            fs::write(&path, bytes).unwrap();
            let track = local_audio_track(directory.path(), &path).unwrap();
            assert_eq!(track.name, "Tagged Café Song");
            assert_eq!(track.artist, "Embedded Artist");
            assert_eq!(track.album, "Embedded Album");
            assert_eq!(track.year, "2007");
            assert_eq!(track.genre, "Synthetic");
            assert_eq!(track.track_number, "3");
            assert!(track.date_added.is_empty() && track.last_played.is_empty());
        }
    }

    #[test]
    fn hostile_or_truncated_id3_tags_fall_back_without_allocation_or_decompression() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("fallback.mp3");
        for bytes in [
            b"ID3\x03\x00\x00\x7f\x7f\x7f\x7f".as_slice(),
            b"ID3\x03\x00\x00\x00\x00\x00\x0bTIT2\x00\x00\x00\x01\x00\x80x",
            b"ID3\x04\x00\x00\x00\x00\x00\x0bTIT2\x00\x00\x00\x7f\x00\x00x",
            b"ID3\x03\x00\x80\x00\x00\x00\x00",
        ] {
            fs::write(&path, bytes).unwrap();
            assert!(mp3_metadata(&path).is_empty());
        }
    }

    #[test]
    fn local_music_route_finds_audio_files_only() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join("Artist/Album")).unwrap();
        fs::write(directory.path().join("Artist/Album/song.m4a"), b"audio").unwrap();
        fs::write(directory.path().join("cover.jpg"), b"image").unwrap();

        let files = audio_files(directory.path());

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("song.m4a"));
    }

    #[test]
    fn music_library_bundle_is_supported_without_audio_files() {
        let directory = tempfile::tempdir().unwrap();
        let library = directory.path().join("Music Library.musiclibrary");
        fs::create_dir_all(&library).unwrap();
        fs::write(library.join("Library.musicdb"), b"opaque music db").unwrap();

        assert!(is_music_library_bundle(&library));
    }

    #[test]
    fn music_app_rows_are_parsed_as_tab_separated_metadata() {
        let track = parse_music_app_row(
            "Synthetic Track\tExample Artist\t\tExample Album\tElectronic\t2020\t1\t1\t180.0\t7\t2020-01-02T03:04\t2020-02-03T04:05\tSYNTHETIC-TRACK-ID\tApple Music AAC audio file",
        );

        assert_eq!(track.name, "Synthetic Track");
        assert_eq!(track.artist, "Example Artist");
        assert_eq!(track.album, "Example Album");
        assert_eq!(track.date_added, "2020-01-02T03:04");
        assert_eq!(track.persistent_id, "SYNTHETIC-TRACK-ID");
    }

    #[test]
    fn music_app_playlist_rows_preserve_membership_without_inventing_dates() {
        let playlist = parse_music_app_playlist_row(
            "Open Mix\tPLAYLIST01\tFolder\tFOLDER01\tuser playlist\tfalse\tfalse\ttrue\ttrue\tfalse\tA portable mix\t3600\tTRACK01,TRACK02",
        );

        assert_eq!(playlist.name, "Open Mix");
        assert_eq!(playlist.parent_name, "Folder");
        assert!(playlist.shared);
        assert!(playlist.visible);
        assert_eq!(playlist.member_persistent_ids, ["TRACK01", "TRACK02"]);
    }

    #[test]
    fn spotlight_plist_exposes_embedded_audio_tags_without_filesystem_dates() {
        let metadata = parse_spotlight_plist(
            r#"<?xml version="1.0"?><plist><dict><key>kMDItemTitle</key><string>Open Song</string><key>kMDItemAuthors</key><array><string>Open Artist</string></array><key>kMDItemRecordingYear</key><real>2024</real><key>kMDItemContentModificationDate</key><date>2026-08-06T10:00:00Z</date></dict></plist>"#,
        );

        assert_eq!(metadata_value(&metadata, "kMDItemTitle"), Some("Open Song"));
        assert_eq!(
            metadata_value(&metadata, "kMDItemAuthors"),
            Some("Open Artist")
        );
        assert_eq!(
            metadata_value(&metadata, "kMDItemRecordingYear"),
            Some("2024")
        );
    }

    #[test]
    fn music_app_permission_error_is_actionable() {
        let message =
            music_app_failure_message("Not authorized to send Apple events to Music. (-1743)");

        assert!(message.contains("macOS denied access"));
        assert!(message.contains("System Settings"));
        assert!(message.contains("Privacy & Security"));
        assert!(message.contains("Automation"));
        assert!(message.contains("revoke"));
    }
}
