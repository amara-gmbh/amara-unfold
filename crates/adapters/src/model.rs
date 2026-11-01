// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::text::sanitize_text;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub raw_path: String,
    pub raw_hash: String,
    pub ingested_at: String,
    pub module_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenEvent {
    pub schema_version: u8,
    pub id: String,
    pub date: String,
    pub date_precision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    pub source: String,
    pub module: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub actors: Vec<String>,
    pub places: Vec<String>,
    pub title: String,
    pub tags: Vec<String>,
    pub confidence: String,
    pub provenance: Provenance,
    pub links: Vec<String>,
    pub revisions: Vec<String>,
    #[serde(skip)]
    pub body: String,
}

pub struct EventInput<'a> {
    pub date: &'a str,
    pub time: Option<&'a str>,
    pub end_date: Option<&'a str>,
    pub source: &'a str,
    pub module: &'a str,
    pub event_type: &'a str,
    pub title: &'a str,
    pub raw_path: &'a str,
    pub raw_bytes: &'a [u8],
    pub body: String,
    pub actors: Vec<String>,
    pub places: Vec<String>,
    pub tags: Vec<String>,
    pub confidence: &'a str,
}

impl OpenEvent {
    pub fn new(input: EventInput<'_>) -> Self {
        let raw_hash = sha256(input.raw_bytes);
        Self::from_hashes(input, &raw_hash, &raw_hash)
    }

    pub fn from_raw_hash(input: EventInput<'_>, raw_hash: &str) -> Self {
        Self::from_hashes(input, raw_hash, raw_hash)
    }

    pub fn new_with_identity(input: EventInput<'_>, identity_bytes: &[u8]) -> Self {
        let raw_hash = sha256(input.raw_bytes);
        let identity_hash = sha256(identity_bytes);
        Self::from_hashes(input, &raw_hash, &identity_hash)
    }

    fn from_hashes(input: EventInput<'_>, raw_hash: &str, identity_hash: &str) -> Self {
        let mut id_hasher = Sha256::new();
        id_hasher.update(identity_hash.as_bytes());
        id_hasher.update(input.source.as_bytes());
        let id_hash = format!("{:x}", id_hasher.finalize());
        let date = normalize_date(input.date);
        let precision = if date == "0000-00-00" {
            "unknown"
        } else if input.time.is_some() {
            "minute"
        } else if date.ends_with("-00-00") {
            "year"
        } else if date.ends_with("-00") {
            "month"
        } else {
            "day"
        };
        let title = clean_text(input.title)
            .replace(['\r', '\n'], " ")
            .chars()
            .take(100)
            .collect::<String>();
        let title = if title.trim().is_empty() {
            format!("Untitled {} event", input.source)
        } else {
            title
        };
        Self {
            schema_version: 1,
            id: format!("{}-{}-{}", date, input.source, &id_hash[..6]),
            date,
            date_precision: precision.into(),
            time: input.time.map(|value| value.chars().take(5).collect()),
            end_date: input.end_date.map(normalize_date),
            source: input.source.into(),
            module: input.module.into(),
            event_type: input.event_type.into(),
            actors: input.actors,
            places: input.places,
            title,
            tags: input.tags,
            confidence: input.confidence.into(),
            provenance: Provenance {
                raw_path: sanitize_text(input.raw_path),
                raw_hash: format!("sha256:{raw_hash}"),
                ingested_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                module_version: format!("{}/1.0", input.module),
            },
            links: Vec::new(),
            revisions: Vec::new(),
            body: clean_text(&input.body),
        }
    }

    pub fn filename(&self) -> String {
        let compact = self.date.replace('-', "");
        let time = self.time.as_deref().unwrap_or("0000").replace(':', "");
        let hash = self.id.rsplit('-').next().unwrap_or("000000");
        let source = slug(&self.source);
        let title = filename_slug(&self.title);
        format!("{compact}-{time}-{source}-{title}-{hash}.md")
    }

    pub fn year_month(&self) -> (&str, &str) {
        let year = self.date.get(0..4).unwrap_or("0000");
        let month = self.date.get(5..7).unwrap_or("00");
        (year, month)
    }
}

/// Preserve visible evidence when a source decoder could not recover a byte,
/// while keeping emitted Markdown valid UTF-8 and free of control characters.
fn clean_text(value: &str) -> String {
    sanitize_text(value)
}

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn sha256_file(path: &Path) -> Result<String, std::io::Error> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn date_time_parts(value: &str) -> Option<(String, Option<String>)> {
    let trimmed = value.trim();
    if let Ok(parsed) = DateTime::parse_from_rfc3339(trimmed) {
        return Some((
            parsed.date_naive().to_string(),
            Some(parsed.format("%H:%M").to_string()),
        ));
    }
    if trimmed.len() >= 10 && NaiveDate::parse_from_str(&trimmed[..10], "%Y-%m-%d").is_ok() {
        let time = trimmed
            .get(11..16)
            .filter(|time| time.chars().nth(2) == Some(':'))
            .map(str::to_owned);
        return Some((trimmed[..10].to_owned(), time));
    }
    for format in ["%d.%m.%Y", "%m/%d/%Y", "%d/%m/%Y", "%Y/%m/%d"] {
        if let Ok(date) = NaiveDate::parse_from_str(trimmed, format) {
            return Some((date.to_string(), None));
        }
    }
    None
}

pub fn slug(value: &str) -> String {
    let mut output = String::new();
    let mut separator = false;
    for character in value.to_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character);
            separator = false;
        } else if !separator && !output.is_empty() {
            output.push('-');
            separator = true;
        }
    }
    output.trim_matches('-').chars().take(64).collect()
}

/// Keep exported event filenames readable without sacrificing filesystem safety.
/// Unlike metadata slugs, filenames may preserve Unicode letters and numbers so
/// personal names and non-English titles remain recognizable to their owner.
fn filename_slug(value: &str) -> String {
    let mut output = String::new();
    let mut separator = false;
    for character in value.to_lowercase().chars() {
        if character.is_alphanumeric() {
            output.push(character);
            separator = false;
        } else if !separator && !output.is_empty() {
            output.push('-');
            separator = true;
        }
    }
    let output = output
        .trim_matches('-')
        .chars()
        .take(64)
        .collect::<String>();
    if output.is_empty() {
        "untitled".into()
    } else {
        output
    }
}

pub fn place_id(latitude: f64, longitude: f64) -> String {
    let digest = Sha256::digest(format!("{latitude:.4},{longitude:.4}").as_bytes());
    let number = u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]) % 1_000_000_000;
    format!("pg-{number:09}")
}

fn normalize_date(value: &str) -> String {
    date_time_parts(value)
        .map(|parts| parts.0)
        .unwrap_or_else(|| {
            if value.len() == 7 {
                format!("{value}-00")
            } else if value.len() == 4 {
                format!("{value}-00-00")
            } else {
                "0000-00-00".into()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_event(title: &str) -> OpenEvent {
        OpenEvent::new(EventInput {
            date: "0000-00-00",
            time: None,
            end_date: None,
            source: "diary",
            module: "diary",
            event_type: "diary_entry",
            title,
            raw_path: "/source/diary.docx",
            raw_bytes: b"stable source record",
            body: "Verbatim memory".into(),
            actors: Vec::new(),
            places: Vec::new(),
            tags: vec!["diary".into()],
            confidence: "high",
        })
    }

    #[test]
    fn source_encoding_damage_stays_visible_without_invalid_characters() {
        let cleaned = clean_text("M\u{fffd}nchen\u{0007}");
        assert_eq!(
            cleaned,
            "M[source character unavailable]nchen[source control character removed]"
        );
        assert!(!cleaned.chars().any(char::is_control));
    }

    #[test]
    fn event_filename_combines_sortable_date_readable_title_and_stable_id() {
        let event = test_event("All humans being unique");

        assert_eq!(event.date_precision, "unknown");
        assert_eq!(
            event.filename(),
            format!(
                "00000000-0000-diary-all-humans-being-unique-{}.md",
                event.id.rsplit('-').next().unwrap()
            )
        );
    }

    #[test]
    fn event_filename_preserves_meaningful_unicode_and_has_safe_fallback() {
        let unicode = test_event("Über Café — 東京");
        let punctuation = test_event("... / \\ ...");

        assert!(unicode.filename().contains("-über-café-東京-"));
        assert!(punctuation.filename().contains("-untitled-"));
        for forbidden in ['/', '\\', ':', '\n', '\r'] {
            assert!(!unicode.filename().contains(forbidden));
            assert!(!punctuation.filename().contains(forbidden));
        }
    }
}
