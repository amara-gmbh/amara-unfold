// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    input,
    model::{slug, EventInput, OpenEvent},
    record_event, record_skip, AdapterError, ExtractionProgress, ProgressCallback,
};
use chrono::NaiveDate;
use std::{collections::HashMap, fs, path::Path, sync::atomic::AtomicBool};

const MAX_VCARD_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Default)]
struct ContactCard {
    uid: String,
    full_name: String,
    structured_name: String,
    organization: String,
    title: String,
    birthday: String,
    emails: Vec<String>,
    phones: Vec<String>,
    addresses: Vec<String>,
    notes: Vec<String>,
}

pub fn extract(
    source: &Path,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    let files = input::files_with_extensions(source, &["vcf", "vcard"]);
    progress.total_hint = Some(files.len() as u64);
    callback(progress.clone());

    for path in files {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        input::validate_file_size(&path, MAX_VCARD_BYTES, "Apple Contacts vCard file")?;
        progress.current_item = Some(path.to_string_lossy().into_owned());
        progress.message = format!("Reading Apple Contacts vCard {}", path.display());
        callback(progress.clone());

        let raw = fs::read(&path).map_err(|error| AdapterError::Input(error.to_string()))?;
        let text = crate::text::decode_text(&raw);
        let contacts = parse_vcards(&text);
        progress.total_hint = Some(contacts.len() as u64);
        callback(progress.clone());

        for (index, contact) in contacts.iter().enumerate() {
            if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            let name = contact.display_name();
            if name.is_empty() {
                progress.records_seen += 1;
                record_skip(progress, "contact_without_name");
                callback(progress.clone());
                continue;
            }
            let date = contact_date(&contact.birthday).unwrap_or_else(|| "0000-00-00".into());
            let birthday = contact_birthday_label(&contact.birthday);
            let body = format!(
                "Contact: **{}**\n\nOrganization: {}\n\nRole: {}\n\nBirthday: {}\n\nEmails: {}\n\nPhones: {}\n\nAddresses: {}\n\nNotes: {}",
                name,
                empty_label(&contact.organization),
                empty_label(&contact.title),
                birthday,
                list_label(&contact.emails),
                list_label(&contact.phones),
                address_list_label(&contact.addresses),
                if contact.notes.is_empty() {
                    "not included".into()
                } else {
                    format!(
                        "private notes present: {} note field(s); note bodies are not emitted by default.",
                        contact.notes.len()
                    )
                }
            );
            let identity = if contact.uid.is_empty() {
                format!("{}#VCARD-{}#{name}", path.display(), index + 1)
            } else {
                format!("{}#{}", path.display(), contact.uid.trim())
            };
            let event = OpenEvent::new_with_identity(
                EventInput {
                    date: &date,
                    time: None,
                    end_date: None,
                    source: "contact",
                    module: "apple-contacts",
                    event_type: "person_contact",
                    title: &name,
                    raw_path: &format!("{}#VCARD-{}", path.display(), index + 1),
                    raw_bytes: raw.as_slice(),
                    body,
                    actors: vec![slug(&format!("person-{name}"))],
                    places: Vec::new(),
                    tags: vec!["contacts".into(), "apple-contacts".into()],
                    confidence: if date == "0000-00-00" {
                        "medium"
                    } else {
                        "high"
                    },
                },
                identity.as_bytes(),
            );
            record_event(package, &event, progress, callback)?;
        }
    }
    Ok(())
}

impl ContactCard {
    fn display_name(&self) -> String {
        if !self.full_name.trim().is_empty() {
            return self.full_name.trim().to_owned();
        }
        self.structured_name
            .split(';')
            .filter(|part| !part.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_owned()
    }
}

fn parse_vcards(text: &str) -> Vec<ContactCard> {
    let lines = unfold_lines(text);
    let mut contacts = Vec::new();
    let mut active: Option<HashMap<String, Vec<String>>> = None;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("BEGIN:VCARD") {
            active = Some(HashMap::new());
            continue;
        }
        if trimmed.eq_ignore_ascii_case("END:VCARD") {
            if let Some(fields) = active.take() {
                contacts.push(contact_from_fields(fields));
            }
            continue;
        }
        let Some(fields) = active.as_mut() else {
            continue;
        };
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.split(';').next().unwrap_or(key).to_ascii_uppercase();
        fields.entry(key).or_default().push(unescape_vcard(value));
    }
    contacts
}

fn contact_from_fields(fields: HashMap<String, Vec<String>>) -> ContactCard {
    ContactCard {
        uid: first(&fields, "UID"),
        full_name: first(&fields, "FN"),
        structured_name: first(&fields, "N"),
        organization: first(&fields, "ORG"),
        title: first(&fields, "TITLE"),
        birthday: first(&fields, "BDAY"),
        emails: fields.get("EMAIL").cloned().unwrap_or_default(),
        phones: fields.get("TEL").cloned().unwrap_or_default(),
        addresses: fields.get("ADR").cloned().unwrap_or_default(),
        notes: fields.get("NOTE").cloned().unwrap_or_default(),
    }
}

fn first(fields: &HashMap<String, Vec<String>>, key: &str) -> String {
    fields
        .get(key)
        .and_then(|values| values.first())
        .cloned()
        .unwrap_or_default()
}

fn unfold_lines(text: &str) -> Vec<String> {
    let mut output: Vec<String> = Vec::new();
    for raw in text.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
        if raw.starts_with(' ') || raw.starts_with('\t') {
            if let Some(last) = output.last_mut() {
                last.push_str(raw.get(1..).unwrap_or_default());
            }
        } else {
            output.push(raw.to_owned());
        }
    }
    output
}

fn unescape_vcard(value: &str) -> String {
    let mut output = String::new();
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            match character {
                'n' | 'N' => output.push('\n'),
                ',' => output.push(','),
                ';' => output.push(';'),
                '\\' => output.push('\\'),
                other => output.push(other),
            }
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            output.push(character);
        }
    }
    if escaped {
        output.push('\\');
    }
    output
}

fn contact_date(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value.starts_with("--") {
        return None;
    }
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        return Some(date.to_string());
    }
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y%m%d") {
        return Some(date.to_string());
    }
    None
}

fn contact_birthday_label(value: &str) -> String {
    if let Some(date) = contact_date(value) {
        date
    } else {
        empty_label(value).to_owned()
    }
}

fn empty_label(value: &str) -> &str {
    if value.trim().is_empty() {
        "not included"
    } else {
        value
    }
}

fn list_label(values: &[String]) -> String {
    if values.is_empty() {
        "not included".into()
    } else {
        values.join(", ")
    }
}

fn address_list_label(values: &[String]) -> String {
    let addresses = values
        .iter()
        .map(|value| {
            value
                .split(';')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if addresses.is_empty() {
        "not included".into()
    } else {
        addresses.join(" · ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_folded_vcards_and_hides_note_body_by_default() {
        let cards = parse_vcards(
            "BEGIN:VCARD\r\nVERSION:3.0\r\nUID:one\r\nFN:Alex Example\r\nORG:Amara\\, Local\r\nBDAY:19800102\r\nEMAIL:alex@example.test\r\nNOTE:Private note line one\\nline two\r\nEND:VCARD\r\n",
        );

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].full_name, "Alex Example");
        assert_eq!(cards[0].organization, "Amara, Local");
        assert_eq!(cards[0].notes.len(), 1);
        assert_eq!(contact_date(&cards[0].birthday).unwrap(), "1980-01-02");
    }

    #[test]
    fn contact_addresses_are_readable_not_raw_vcard_segments() {
        assert_eq!(
            address_list_label(&[";;Main Street 1;Munich;;80331;Germany".into()]),
            "Main Street 1, Munich, 80331, Germany"
        );
    }
}
