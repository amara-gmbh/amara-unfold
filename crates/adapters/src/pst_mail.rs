// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::text::{clean_mail_text, html_to_text, sanitize_text};
use crate::{
    input,
    model::{slug, EventInput, OpenEvent},
    record_event, AdapterError, ExtractionProgress, ProgressCallback,
};
use chrono::{TimeZone, Utc};
use outlook_pst::{
    ltp::prop_context::PropertyValue,
    messaging::{folder::Folder, store::Store},
    ndb::node_id::NodeId,
};
use std::{
    path::Path,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

pub fn extract(
    source: &Path,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    let paths = input::files_with_extensions(source, &["pst"]);
    let mut owner_addresses = paths
        .iter()
        .flat_map(|path| email_addresses(&path.to_string_lossy()))
        .collect::<Vec<_>>();
    owner_addresses.sort();
    owner_addresses.dedup();
    for path in &paths {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let store = outlook_pst::open_store(path)
            .map_err(|error| AdapterError::Parse(format!("{}: {error}", path.display())))?;
        let root_id = store
            .properties()
            .ipm_sub_tree_entry_id()
            .map_err(|error| AdapterError::Parse(error.to_string()))?;
        let root = store
            .open_folder(&root_id)
            .map_err(|error| AdapterError::Parse(error.to_string()))?;
        collect_sent_sender_addresses(store, root, "Mailbox", &mut owner_addresses, cancel)?;
    }
    owner_addresses.sort();
    owner_addresses.dedup();
    for path in paths {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        input::validate_file_size(&path, 16 * 1024 * 1024 * 1024, "PST file")?;
        let store = outlook_pst::open_store(&path)
            .map_err(|error| AdapterError::Parse(format!("{}: {error}", path.display())))?;
        let root_id = store
            .properties()
            .ipm_sub_tree_entry_id()
            .map_err(|error| AdapterError::Parse(error.to_string()))?;
        let root = store
            .open_folder(&root_id)
            .map_err(|error| AdapterError::Parse(error.to_string()))?;
        walk_folder(
            &path,
            store,
            root,
            "Mailbox",
            &owner_addresses,
            package,
            cancel,
            progress,
            callback,
        )?;
    }
    Ok(())
}

fn collect_sent_sender_addresses(
    store: Rc<dyn Store>,
    folder: Rc<dyn Folder>,
    parent_path: &str,
    owner_addresses: &mut Vec<String>,
    cancel: &AtomicBool,
) -> Result<(), AdapterError> {
    if cancel.load(Ordering::Relaxed) {
        return Ok(());
    }
    let name = folder
        .properties()
        .display_name()
        .unwrap_or_else(|_| "Folder".into());
    let folder_path = format!("{parent_path}/{name}");
    if folder_indicates_sent(&folder_path) {
        if let Some(table) = folder.contents_table() {
            for row in table.rows_matrix() {
                let node = NodeId::from(u32::from(row.id()));
                let Ok(entry_id) = store.properties().make_entry_id(node) else {
                    continue;
                };
                let Ok(message) = store.open_message(&entry_id, None) else {
                    continue;
                };
                let properties = message.properties();
                let code_page = message_code_page(properties);
                let sender = property_text(properties.get(0x0C1F), code_page)
                    .or_else(|| property_text(properties.get(0x0C1A), code_page))
                    .unwrap_or_default();
                owner_addresses.extend(email_addresses(&sender));
            }
        }
    }
    if let Some(table) = folder.hierarchy_table() {
        for row in table.rows_matrix() {
            let node = NodeId::from(u32::from(row.id()));
            let entry_id = store
                .properties()
                .make_entry_id(node)
                .map_err(|error| AdapterError::Parse(error.to_string()))?;
            let child = store
                .open_folder(&entry_id)
                .map_err(|error| AdapterError::Parse(error.to_string()))?;
            collect_sent_sender_addresses(
                store.clone(),
                child,
                &folder_path,
                owner_addresses,
                cancel,
            )?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn walk_folder(
    pst_path: &Path,
    store: Rc<dyn Store>,
    folder: Rc<dyn Folder>,
    parent_path: &str,
    owner_addresses: &[String],
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    if cancel.load(Ordering::Relaxed) {
        return Ok(());
    }
    let name = folder
        .properties()
        .display_name()
        .unwrap_or_else(|_| "Folder".into());
    let folder_path = format!("{parent_path}/{name}");
    if let Some(table) = folder.contents_table() {
        let rows = table.rows_matrix().collect::<Vec<_>>();
        progress.total_hint = Some(progress.total_hint.unwrap_or(0) + rows.len() as u64);
        callback(progress.clone());
        for row in rows {
            if cancel.load(Ordering::Relaxed) {
                return Ok(());
            }
            let node = NodeId::from(u32::from(row.id()));
            let entry_id = match store.properties().make_entry_id(node) {
                Ok(value) => value,
                Err(_) => {
                    progress.errors += 1;
                    continue;
                }
            };
            let message = match store.open_message(&entry_id, None) {
                Ok(value) => value,
                Err(_) => {
                    progress.errors += 1;
                    continue;
                }
            };
            let properties = message.properties();
            let code_page = message_code_page(properties);
            let subject = property_subject(properties.get(0x0037), code_page)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "Email without subject".into());
            let sender = property_text(properties.get(0x0C1F), code_page)
                .or_else(|| property_text(properties.get(0x0C1A), code_page))
                .unwrap_or_default();
            let to = property_text(properties.get(0x0E04), code_page).unwrap_or_default();
            let cc = property_text(properties.get(0x0E03), code_page).unwrap_or_default();
            let message_id = property_text(properties.get(0x1035), code_page).unwrap_or_default();
            let timestamp = property_time(properties.get(0x0039))
                .or_else(|| property_time(properties.get(0x0E06)))
                .or_else(|| property_time(properties.get(0x3007)));
            let timestamp_key = timestamp
                .as_ref()
                .map(chrono::DateTime::to_rfc3339)
                .unwrap_or_default();
            let (date, time) = timestamp
                .map(|value| {
                    (
                        value.format("%Y-%m-%d").to_string(),
                        Some(value.format("%H:%M").to_string()),
                    )
                })
                .unwrap_or_else(|| ("0000-00-00".into(), None));
            let body = match message_body(properties, code_page) {
                MessageBody::Readable(body) => body,
                MessageBody::Absent =>
                    "[No message body was present in this PST record.]".into(),
                MessageBody::Undecodable => "[A message body was present but could not be decoded; raw provenance is preserved.]".into(),
            };
            let direction = classify_direction(&sender, &folder_path, owner_addresses);
            let actors = if direction == "sent" {
                correspondent_slugs([to.as_str(), cc.as_str()])
            } else {
                correspondent_slugs([sender.as_str()])
            };
            let identity = format!("{message_id}\n{timestamp_key}\n{sender}\n{to}\n{subject}");
            let canonical = format!("{identity}\n{cc}\n{body}");
            let raw_path = format!(
                "{}#{}#node-{}",
                pst_path.display(),
                folder_path,
                u32::from(node)
            );
            let event = OpenEvent::new_with_identity(
                EventInput {
                    date: &date,
                    time: time.as_deref(),
                    end_date: None,
                    source: "email",
                    module: "mail-pst",
                    event_type: "email",
                    title: &subject,
                    raw_path: &raw_path,
                    raw_bytes: canonical.as_bytes(),
                    body: format!(
                        "From: {sender}\n\nTo: {to}\n\nCc: {cc}\n\nDirection: {direction}\n\nMailbox folder: {folder_path}\n\n{body}"
                    ),
                    actors,
                    places: Vec::new(),
                    tags: vec!["email".into(), direction.into()],
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
    if let Some(table) = folder.hierarchy_table() {
        for row in table.rows_matrix() {
            let node = NodeId::from(u32::from(row.id()));
            let entry_id = store
                .properties()
                .make_entry_id(node)
                .map_err(|error| AdapterError::Parse(error.to_string()))?;
            let child = store
                .open_folder(&entry_id)
                .map_err(|error| AdapterError::Parse(error.to_string()))?;
            walk_folder(
                pst_path,
                store.clone(),
                child,
                &folder_path,
                owner_addresses,
                package,
                cancel,
                progress,
                callback,
            )?;
        }
    }
    Ok(())
}

fn property_text(value: Option<&PropertyValue>, code_page: u16) -> Option<String> {
    match value? {
        PropertyValue::String8(value) => decode_bytes(value.buffer(), code_page),
        PropertyValue::Unicode(value) => Some(value.to_string()),
        _ => None,
    }
    .map(clean_text)
}

fn property_subject(value: Option<&PropertyValue>, code_page: u16) -> Option<String> {
    match value? {
        PropertyValue::String8(value) => {
            let offset = usize::from(value.buffer().first() == Some(&1)) * 2;
            decode_bytes(
                &value.buffer()[offset.min(value.buffer().len())..],
                code_page,
            )
        }
        PropertyValue::Unicode(value) => {
            let offset = usize::from(value.buffer().first() == Some(&1)) * 2;
            Some(String::from_utf16_lossy(
                &value.buffer()[offset.min(value.buffer().len())..],
            ))
        }
        _ => None,
    }
    .map(clean_text)
}

fn property_time(value: Option<&PropertyValue>) -> Option<chrono::DateTime<Utc>> {
    let PropertyValue::Time(file_time) = value? else {
        return None;
    };
    let seconds = file_time.div_euclid(10_000_000) - 11_644_473_600;
    let nanos = file_time.rem_euclid(10_000_000) as u32 * 100;
    Utc.timestamp_opt(seconds, nanos).single()
}

enum MessageBody {
    Readable(String),
    Absent,
    Undecodable,
}

fn message_body(
    properties: &outlook_pst::messaging::message::MessageProperties,
    code_page: u16,
) -> MessageBody {
    let decoded = property_text(properties.get(0x1000), code_page)
        .map(|value| clean_mail_text(&value))
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            property_text(properties.get(0x1013), code_page)
                .map(|value| html_to_text(&value))
                .filter(|value| !value.trim().is_empty())
        })
        .or_else(|| match properties.get(0x1013) {
            Some(PropertyValue::Binary(value)) => decode_bytes(value.buffer(), code_page)
                .map(|value| html_to_text(&value))
                .filter(|value| !value.trim().is_empty()),
            _ => None,
        })
        .or_else(|| match properties.get(0x1009) {
            Some(PropertyValue::Binary(value)) => decode_compressed_rtf(value.buffer(), code_page),
            _ => None,
        })
        .map(|value| clean_mail_text(&value))
        .map(|value| value.chars().take(250_000).collect());
    if let Some(body) = decoded {
        MessageBody::Readable(body)
    } else if plain_body_has_content(properties.get(0x1000), code_page)
        || html_body_has_content(properties.get(0x1013), code_page)
        || rtf_body_has_content(properties.get(0x1009), code_page)
    {
        MessageBody::Undecodable
    } else {
        MessageBody::Absent
    }
}

fn plain_body_has_content(value: Option<&PropertyValue>, code_page: u16) -> bool {
    value.is_some_and(|value| match value {
        PropertyValue::String8(_) | PropertyValue::Unicode(_) => {
            property_text(Some(value), code_page).is_some_and(|text| !text.trim().is_empty())
        }
        PropertyValue::Binary(value) => decode_bytes(value.buffer(), code_page)
            .map(clean_text)
            .is_some_and(|text| !text.trim().is_empty()),
        _ => false,
    })
}

fn html_body_has_content(value: Option<&PropertyValue>, code_page: u16) -> bool {
    value.is_some_and(|value| match value {
        PropertyValue::String8(_) | PropertyValue::Unicode(_) => {
            property_text(Some(value), code_page)
                .map(|text| html_to_text(&text))
                .is_some_and(|text| !text.trim().is_empty())
        }
        PropertyValue::Binary(value) => decode_bytes(value.buffer(), code_page)
            .map(|text| html_to_text(&text))
            .is_some_and(|text| !text.trim().is_empty()),
        _ => false,
    })
}

fn rtf_body_has_content(value: Option<&PropertyValue>, code_page: u16) -> bool {
    value.is_some_and(|value| match value {
        PropertyValue::Binary(value) => decode_compressed_rtf(value.buffer(), code_page)
            .is_some_and(|text| !text.trim().is_empty()),
        _ => plain_body_has_content(Some(value), code_page),
    })
}

const MAX_COMPRESSED_RTF_BYTES: usize = 16 * 1024 * 1024;

fn decode_compressed_rtf(bytes: &[u8], code_page: u16) -> Option<String> {
    if bytes.len() < 16 || bytes.len() > MAX_COMPRESSED_RTF_BYTES {
        return None;
    }
    let raw_size = u32::from_le_bytes(bytes.get(4..8)?.try_into().ok()?) as usize;
    if raw_size > MAX_COMPRESSED_RTF_BYTES {
        return None;
    }
    let rtf = compressed_rtf::decompress_rtf(bytes).ok()?;
    let text = rtf_to_text(&rtf, code_page);
    (!text.trim().is_empty()).then_some(text)
}

#[derive(Clone, Copy)]
struct RtfState {
    skip: bool,
    unicode_fallback: usize,
}

fn rtf_to_text(rtf: &str, code_page: u16) -> String {
    let bytes = rtf.as_bytes();
    let mut output = String::new();
    let mut state = RtfState {
        skip: false,
        unicode_fallback: 1,
    };
    let mut stack = Vec::new();
    let mut index = 0;
    let mut skip_plain = 0_usize;
    while index < bytes.len() {
        match bytes[index] {
            b'{' => {
                stack.push(state);
                index += 1;
            }
            b'}' => {
                state = stack.pop().unwrap_or(state);
                index += 1;
            }
            b'\\' => {
                index += 1;
                if index >= bytes.len() {
                    break;
                }
                match bytes[index] {
                    b'\\' | b'{' | b'}' => {
                        if !state.skip && skip_plain == 0 {
                            output.push(bytes[index] as char);
                        } else {
                            skip_plain = skip_plain.saturating_sub(1);
                        }
                        index += 1;
                    }
                    b'\'' if index + 2 < bytes.len() => {
                        if let Ok(value) = u8::from_str_radix(&rtf[index + 1..index + 3], 16) {
                            if !state.skip && skip_plain == 0 {
                                output.push_str(
                                    &decode_bytes(&[value], code_page).unwrap_or_default(),
                                );
                            } else {
                                skip_plain = skip_plain.saturating_sub(1);
                            }
                        }
                        index += 3;
                    }
                    b'*' => {
                        state.skip = true;
                        index += 1;
                    }
                    symbol if !symbol.is_ascii_alphabetic() => {
                        if !state.skip {
                            match symbol {
                                b'~' => output.push(' '),
                                b'_' => output.push('—'),
                                _ => {}
                            }
                        }
                        index += 1;
                    }
                    _ => {
                        let word_start = index;
                        while index < bytes.len() && bytes[index].is_ascii_alphabetic() {
                            index += 1;
                        }
                        let word = &rtf[word_start..index];
                        let sign = if bytes.get(index) == Some(&b'-') {
                            index += 1;
                            -1_i32
                        } else {
                            1
                        };
                        let number_start = index;
                        while index < bytes.len() && bytes[index].is_ascii_digit() {
                            index += 1;
                        }
                        let number = (number_start < index)
                            .then(|| rtf[number_start..index].parse::<i32>().ok())
                            .flatten()
                            .map(|number| number * sign);
                        if bytes.get(index) == Some(&b' ') {
                            index += 1;
                        }
                        if matches!(
                            word,
                            "fonttbl"
                                | "colortbl"
                                | "stylesheet"
                                | "info"
                                | "pict"
                                | "object"
                                | "header"
                                | "footer"
                                | "filetbl"
                                | "datastore"
                                | "themedata"
                                | "xmlnstbl"
                        ) {
                            state.skip = true;
                        } else if !state.skip {
                            match word {
                                "par" | "line" => output.push('\n'),
                                "tab" => output.push('\t'),
                                "emdash" => output.push('—'),
                                "endash" => output.push('–'),
                                "bullet" => output.push('•'),
                                "lquote" | "rquote" => output.push('\''),
                                "ldblquote" | "rdblquote" => output.push('"'),
                                "uc" => {
                                    state.unicode_fallback = number.unwrap_or(1).max(0) as usize;
                                }
                                "u" => {
                                    if let Some(number) = number {
                                        let unit = number as i16 as u16;
                                        if let Some(character) = char::decode_utf16([unit]).next() {
                                            output.push(character.unwrap_or('�'));
                                        }
                                        skip_plain = state.unicode_fallback;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            b'\r' | b'\n' => index += 1,
            byte => {
                if !state.skip && skip_plain == 0 {
                    output.push(byte as char);
                } else {
                    skip_plain = skip_plain.saturating_sub(1);
                }
                index += 1;
            }
        }
    }
    let cleaned = output.lines().map(str::trim).collect::<Vec<_>>().join("\n");
    sanitize_text(cleaned.trim())
}

fn message_code_page(properties: &outlook_pst::messaging::message::MessageProperties) -> u16 {
    match properties.get(0x3FDE) {
        Some(PropertyValue::Integer32(value)) => u16::try_from(*value).unwrap_or(1252),
        _ => 1252,
    }
}

fn decode_bytes(bytes: &[u8], code_page: u16) -> Option<String> {
    let bytes = bytes.strip_suffix(&[0]).unwrap_or(bytes);
    codepage_strings::Coding::new(code_page)
        .or_else(|_| codepage_strings::Coding::new(1252))
        .ok()?
        .decode(bytes)
        .ok()
        .map(|value| value.to_string())
}

fn clean_text(value: String) -> String {
    sanitize_text(&value)
        .trim_matches(|character: char| character == '\0' || character.is_control())
        .trim()
        .to_owned()
}

fn classify_direction(sender: &str, folder_path: &str, owner_addresses: &[String]) -> &'static str {
    let in_sent_folder = folder_indicates_sent(folder_path);
    let sent_by_owner = email_addresses(sender)
        .iter()
        .any(|sender_address| address_matches_owner(sender_address, owner_addresses));
    if in_sent_folder || sent_by_owner {
        "sent"
    } else {
        "received"
    }
}

pub(crate) fn address_matches_owner(address: &str, owner_addresses: &[String]) -> bool {
    if owner_addresses.iter().any(|owner| owner == address) {
        return true;
    }
    let Some((local, domain)) = address.split_once('@') else {
        return false;
    };
    let domain_is_owned = owner_addresses.iter().any(|owner| {
        owner
            .split_once('@')
            .is_some_and(|(_, owner_domain)| owner_domain.eq_ignore_ascii_case(domain))
    });
    let local_is_owned = owner_addresses.iter().any(|owner| {
        owner.split_once('@').is_some_and(|(owner_local, _)| {
            owner_local.eq_ignore_ascii_case(local)
                || alias_local(owner_local) == alias_local(local)
        })
    });
    domain_is_owned && local_is_owned
}

fn alias_local(local: &str) -> String {
    local
        .trim_end_matches(|character: char| character.is_ascii_digit())
        .to_lowercase()
}

fn folder_indicates_sent(folder_path: &str) -> bool {
    let folder_key = folder_path.to_lowercase();
    ["sent", "gesendet", "gesendete", "postausgang", "outbox"]
        .iter()
        .any(|needle| folder_key.contains(needle))
}

fn email_addresses(value: &str) -> Vec<String> {
    let mut addresses = value
        .split(|character: char| {
            character.is_whitespace()
                || matches!(character, '<' | '>' | ',' | ';' | '(' | ')' | '"' | '\'')
                || matches!(character, '/' | '\\')
        })
        .map(|part| {
            part.trim_matches(|character: char| {
                !character.is_alphanumeric() && !matches!(character, '@' | '.' | '_' | '-' | '+')
            })
            .to_lowercase()
        })
        .filter(|part| {
            let Some((local, domain)) = part.split_once('@') else {
                return false;
            };
            !local.is_empty()
                && domain.contains('.')
                && !domain.ends_with('.')
                && part.matches('@').count() == 1
        })
        .collect::<Vec<_>>();
    addresses.sort();
    addresses.dedup();
    addresses
}

fn correspondent_slugs<'a>(values: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut actors = values
        .into_iter()
        .flat_map(|value| value.split(';'))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(slug)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    actors.sort();
    actors.dedup();
    actors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filetime_epoch_converts_to_unix_epoch() {
        let value = PropertyValue::Time(116_444_736_000_000_000);
        assert_eq!(property_time(Some(&value)).unwrap().timestamp(), 0);
    }

    #[test]
    fn html_fallback_is_plain_text() {
        assert_eq!(
            html_to_text("<p>Hello&nbsp; <b>there</b> &#8226;</p>"),
            "Hello there •"
        );
    }

    #[test]
    fn own_sender_or_sent_folder_classifies_as_sent() {
        let owners = email_addresses("/mail/person@example.com/Inbox/archive.pst");
        assert_eq!(
            owners,
            email_addresses(r"C:\mail\person@example.com\Inbox\archive.pst")
        );
        assert_eq!(
            classify_direction("Person <person@example.com>", "Mailbox/Inbox", &owners),
            "sent"
        );
        assert_eq!(
            classify_direction("sender@example.com", "Mailbox/Gesendete Elemente", &owners),
            "sent"
        );
        assert_eq!(
            classify_direction("sender@example.com", "Mailbox/Posteingang", &owners),
            "received"
        );
        let aliases =
            email_addresses("/mail/person@example.com/alex74@example.invalid/archive.pst");
        assert!(address_matches_owner("alex@example.com", &aliases));
        assert!(!address_matches_owner("other@example.com", &aliases));
    }

    #[test]
    fn compressed_rtf_body_becomes_plain_text() {
        let compressed = compressed_rtf::compress_rtf(
            r#"{\rtf1\ansi\ansicpg1252{\fonttbl{\f0 Arial;}}\pard Hello \u252? world\par Second line}"#,
        )
        .unwrap();
        assert_eq!(
            decode_compressed_rtf(&compressed, 1252).unwrap(),
            "Hello ü world\nSecond line"
        );
    }

    #[test]
    fn whitespace_only_pst_body_is_not_reported_as_undecodable_content() {
        let value = PropertyValue::Binary(outlook_pst::ltp::prop_context::BinaryValue::new(
            b"\r\n\r\n\0".to_vec(),
        ));
        assert!(!plain_body_has_content(Some(&value), 1252));
    }

    #[test]
    fn html_shell_pst_body_is_not_reported_as_undecodable_content() {
        let value = PropertyValue::Binary(outlook_pst::ltp::prop_context::BinaryValue::new(
            br#"<html><head><meta http-equiv="Content-Type"></head><body><br></body></html>"#
                .to_vec(),
        ));
        assert!(!html_body_has_content(Some(&value), 1252));
    }
}
