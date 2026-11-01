// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::apple_music_common::{export_tracks, MusicSourceKind, MusicTrack};
use crate::text::decode_text;
use crate::{input, AdapterError, ExtractionProgress, ProgressCallback};
use std::{
    collections::HashMap,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

pub fn extract(
    source: &Path,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    let mut blobs = input::matching_blobs(source, |path| {
        path.to_lowercase().ends_with("library_tracks.tsv")
    })?;
    if blobs
        .iter()
        .any(|blob| file_name(&blob.raw_path) == "live_library_tracks.tsv")
    {
        blobs.retain(|blob| file_name(&blob.raw_path) == "live_library_tracks.tsv");
    } else {
        blobs.retain(|blob| file_name(&blob.raw_path) == "library_tracks.tsv");
    }

    let mut tracks = Vec::new();
    for blob in blobs {
        let text = decode_text(&blob.bytes);
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b'\t')
            .flexible(true)
            .from_reader(text.as_bytes());
        let headers = reader
            .headers()
            .map_err(|error| AdapterError::Parse(error.to_string()))?
            .clone();
        for (row_index, row) in reader.records().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let row = row.map_err(|error| AdapterError::Parse(error.to_string()))?;
            let fields = headers
                .iter()
                .zip(row.iter())
                .map(|(header, value)| (normalized_header(header), value))
                .collect::<HashMap<_, _>>();
            let raw_record = row.as_slice().as_bytes().to_vec();
            tracks.push(MusicTrack {
                name: field(&fields, &["name", "track", "trackname"]).into(),
                artist: field(&fields, &["artist", "trackartist"]).into(),
                album_artist: field(&fields, &["albumartist"]).into(),
                album: field(&fields, &["album", "albumname"]).into(),
                genre: field(&fields, &["genre"]).into(),
                year: field(&fields, &["year", "releaseyear"]).into(),
                track_number: field(&fields, &["tracknumber", "trackno"]).into(),
                disc_number: field(&fields, &["discnumber", "discno"]).into(),
                duration_seconds: field(&fields, &["durationseconds", "duration", "totaltime"])
                    .into(),
                play_count: field(&fields, &["playcount", "playedcount"]).into(),
                date_added: field(&fields, &["dateadded", "addeddate"]).into(),
                last_played: field(&fields, &["lastplayed", "playeddate"]).into(),
                purchase_date: field(&fields, &["purchasedate"]).into(),
                persistent_id: field(&fields, &["persistentid", "trackpersistentid"]).into(),
                format: field(&fields, &["kind", "format", "filetype"]).into(),
                file_name: field(&fields, &["filename", "location"]).into(),
                raw_path: format!("{}#row-{}", blob.raw_path, row_index + 2),
                raw_record,
                content_hash: String::new(),
            });
        }
    }
    export_tracks(
        source,
        MusicSourceKind::TabSeparated,
        tracks,
        package,
        cancel,
        progress,
        callback,
    )
}

fn file_name(path: &str) -> &str {
    path.rsplit(['/', '!']).next().unwrap_or(path)
}

fn normalized_header(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn field<'a>(fields: &HashMap<String, &'a str>, names: &[&str]) -> &'a str {
    names
        .iter()
        .find_map(|name| fields.get(*name).copied())
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_headers_are_matched_by_meaning() {
        assert_eq!(normalized_header("Date Added"), "dateadded");
        assert_eq!(normalized_header("Persistent_ID"), "persistentid");
    }
}
