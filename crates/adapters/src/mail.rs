// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::text::{clean_mail_text, html_to_text};
use crate::{
    model::{slug, EventInput, OpenEvent},
    record_event, AdapterError, ExtractionProgress, ProgressCallback,
};
use chrono::{TimeZone, Utc};
use mailparse::{MailHeaderMap, ParsedMail};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use walkdir::WalkDir;
use zip::ZipArchive;

const MAX_MBOX_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const MAX_MESSAGE_BYTES: usize = 256 * 1024 * 1024;

pub fn extract(
    source: &Path,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    let _ = extract_mbox_sources(source, "gmail-mbox", package, cancel, progress, callback)?;
    Ok(())
}

pub(crate) fn extract_mbox_sources(
    source: &Path,
    module: &str,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<u64, AdapterError> {
    let mut sources_seen = 0_u64;
    for path in mbox_source_files(source) {
        sources_seen += 1;
        crate::input::validate_file_size(&path, MAX_MBOX_BYTES, "MBOX file")?;
        let file = File::open(&path).map_err(|error| AdapterError::Input(error.to_string()))?;
        process_mbox(
            BufReader::new(file),
            &path.to_string_lossy(),
            module,
            package,
            cancel,
            progress,
            callback,
        )?;
    }
    for path in crate::input::files_with_extensions(source, &["zip"]) {
        let file = File::open(&path).map_err(|error| AdapterError::Input(error.to_string()))?;
        let mut archive =
            ZipArchive::new(file).map_err(|error| AdapterError::Input(error.to_string()))?;
        if archive.len() > crate::input::MAX_ARCHIVE_ENTRIES {
            return Err(AdapterError::Input(format!(
                "{} contains too many ZIP entries",
                path.display()
            )));
        }
        let mut mbox_total = 0_u64;
        for index in 0..archive.len() {
            let entry = archive
                .by_index(index)
                .map_err(|error| AdapterError::Input(error.to_string()))?;
            let entry_name = entry
                .name()
                .map_err(|error| AdapterError::Input(error.to_string()))?
                .into_owned();
            if !entry_name.to_lowercase().ends_with(".mbox") {
                continue;
            }
            crate::input::validate_entry_limits(
                &entry_name,
                entry.size(),
                entry.compressed_size(),
                MAX_MBOX_BYTES,
            )?;
            mbox_total = mbox_total
                .checked_add(entry.size())
                .ok_or_else(|| AdapterError::Input("MBOX size overflowed".into()))?;
            if mbox_total > MAX_MBOX_BYTES {
                return Err(AdapterError::Input(format!(
                    "{} expands beyond the MBOX limit",
                    path.display()
                )));
            }
            let raw_path = format!("{}!{}", path.display(), entry_name);
            sources_seen += 1;
            process_mbox(
                BufReader::new(entry),
                &raw_path,
                module,
                package,
                cancel,
                progress,
                callback,
            )?;
        }
    }
    Ok(sources_seen)
}

fn mbox_source_files(source: &Path) -> Vec<PathBuf> {
    let mut seen = BTreeSet::new();
    let mut paths = Vec::new();
    for path in crate::input::files_with_extensions(source, &["mbox"]) {
        if seen.insert(path.clone()) {
            paths.push(path);
        }
    }
    if source.is_dir() {
        for entry in WalkDir::new(source).follow_links(false) {
            let Ok(entry) = entry else {
                continue;
            };
            if !entry.file_type().is_file() || entry.file_name() != "mbox" {
                continue;
            }
            let path = entry.into_path();
            let parent_is_mbox_package = path
                .parent()
                .and_then(|parent| parent.extension())
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("mbox"));
            if parent_is_mbox_package && seen.insert(path.clone()) {
                paths.push(path);
            }
        }
    }
    paths
}

fn process_mbox<R: Read>(
    reader: BufReader<R>,
    raw_path: &str,
    module: &str,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    let mut message = Vec::new();
    let mut index = 0_u64;
    for line in reader.split(b'\n') {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let line = line.map_err(|error| AdapterError::Input(error.to_string()))?;
        if line.starts_with(b"From ") && !message.is_empty() {
            index += 1;
            normalize_mail_message(
                &message, raw_path, index, module, package, progress, callback,
            )?;
            message.clear();
        } else {
            message.extend_from_slice(&line);
            message.push(b'\n');
            if message.len() > MAX_MESSAGE_BYTES {
                return Err(AdapterError::Input(format!(
                    "message {index} in {raw_path} exceeds the {} byte limit",
                    MAX_MESSAGE_BYTES
                )));
            }
        }
    }
    if !message.is_empty() && !cancel.load(Ordering::Relaxed) {
        normalize_mail_message(
            &message,
            raw_path,
            index + 1,
            module,
            package,
            progress,
            callback,
        )?;
    }
    Ok(())
}

pub(crate) fn normalize_mail_message(
    bytes: &[u8],
    raw_path: &str,
    index: u64,
    module: &str,
    package: &crate::writer::PackageWriter,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    let parsed =
        mailparse::parse_mail(bytes).map_err(|error| AdapterError::Parse(error.to_string()))?;
    let date_header = parsed.headers.get_first_value("Date").unwrap_or_default();
    let timestamp = mailparse::dateparse(&date_header).ok();
    let Some(date_time) = timestamp.and_then(|value| Utc.timestamp_opt(value, 0).single()) else {
        progress.errors += 1;
        return Ok(());
    };
    let subject = parsed
        .headers
        .get_first_value("Subject")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Email without subject".into());
    let from = parsed.headers.get_first_value("From").unwrap_or_default();
    let to = parsed.headers.get_first_value("To").unwrap_or_default();
    let direction = infer_mail_direction(raw_path, &from);
    let body = extract_message_body(&parsed).unwrap_or_else(|| {
        "[No readable message body was present; raw message provenance is preserved.]".into()
    });
    let actor = email_address(&from)
        .map(|value| slug(&value))
        .into_iter()
        .collect();
    let event = OpenEvent::new(EventInput {
        date: &date_time.format("%Y-%m-%d").to_string(),
        time: Some(&date_time.format("%H:%M").to_string()),
        end_date: None,
        source: "email",
        module,
        event_type: "email",
        title: &subject,
        raw_path: &format!("{raw_path}#message-{index}"),
        raw_bytes: bytes,
        body: format!("Direction: {direction}\n\nFrom: {from}\n\nTo: {to}\n\n{body}"),
        actors: actor,
        places: Vec::new(),
        tags: vec!["email".into()],
        confidence: "high",
    });
    record_event(package, &event, progress, callback)
}

fn infer_mail_direction(raw_path: &str, from: &str) -> &'static str {
    let sent_mailbox = raw_path.split(['/', '\\', '!', '#']).any(|part| {
        let lower = part.to_ascii_lowercase();
        let mailbox = lower.strip_suffix(".mbox").unwrap_or(&lower).trim();
        matches!(
            mailbox,
            "sent"
                | "sent mail"
                | "sent messages"
                | "sent items"
                | "gesendet"
                | "gesendete"
                | "gesendete elemente"
                | "gesendete nachrichten"
        )
    });
    if sent_mailbox {
        return "sent";
    }
    if email_address(from).is_some() {
        "received"
    } else {
        "unknown"
    }
}

fn email_address(value: &str) -> Option<String> {
    value
        .split(['<', '>', ' ', ','])
        .find(|part| part.contains('@'))
        .map(|value| value.trim().to_lowercase())
}

fn extract_message_body(parsed: &ParsedMail<'_>) -> Option<String> {
    let mut plain = Vec::new();
    let mut html = Vec::new();
    collect_body_parts(parsed, &mut plain, &mut html);
    let body = if !plain.is_empty() {
        plain
            .into_iter()
            .map(|value| clean_mail_text(&value))
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    } else {
        html.into_iter()
            .map(|value| html_to_text(&value))
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    let body = body.trim();
    (!body.is_empty()).then(|| body.chars().take(250_000).collect())
}

fn collect_body_parts(parsed: &ParsedMail<'_>, plain: &mut Vec<String>, html: &mut Vec<String>) {
    if !parsed.subparts.is_empty() {
        for subpart in &parsed.subparts {
            collect_body_parts(subpart, plain, html);
        }
        return;
    }

    let disposition = parsed
        .headers
        .get_first_value("Content-Disposition")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let content_type_header = parsed
        .headers
        .get_first_value("Content-Type")
        .unwrap_or_default()
        .to_ascii_lowercase();
    if disposition.starts_with("attachment")
        || disposition.contains("filename=")
        || content_type_header.contains("name=")
    {
        return;
    }

    let mime_type = parsed.ctype.mimetype.to_ascii_lowercase();
    let Ok(body) = parsed.get_body() else {
        return;
    };
    if body.trim().is_empty() {
        return;
    }
    match mime_type.as_str() {
        "text/plain" => plain.push(body),
        "text/html" => html.push(body),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apple_mail_exported_mbox_package_is_readable_source() {
        let directory = tempfile::tempdir().unwrap();
        let package = directory.path().join("Inbox.mbox");
        std::fs::create_dir(&package).unwrap();
        std::fs::write(package.join("mbox"), b"From sender@example.com\n").unwrap();

        let sources = mbox_source_files(directory.path());

        assert_eq!(sources, vec![package.join("mbox")]);
    }

    #[test]
    fn multipart_prefers_decoded_plain_text_and_excludes_attachments() {
        let raw = b"Content-Type: multipart/mixed; boundary=outer\r\n\r\n--outer\r\nContent-Type: multipart/alternative; boundary=inner\r\n\r\n--inner\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: quoted-printable\r\n\r\nGr=C3=BC=C3=9Fe from plain text.\r\n--inner\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<p>HTML fallback</p>\r\n--inner--\r\n--outer\r\nContent-Type: text/plain; name=private.txt\r\nContent-Disposition: attachment; filename=private.txt\r\n\r\nAttachment content\r\n--outer--\r\n";
        let parsed = mailparse::parse_mail(raw).unwrap();
        let body = extract_message_body(&parsed).unwrap();
        assert_eq!(body, "Grüße from plain text.");
        assert!(!body.contains("Attachment"));
    }

    #[test]
    fn html_only_base64_message_becomes_plain_text() {
        let raw = b"Content-Type: text/html; charset=utf-8\r\nContent-Transfer-Encoding: base64\r\n\r\nPHA+SGVsbG8gJmFtcDsgd2VsY29tZTwvcD4=\r\n";
        let parsed = mailparse::parse_mail(raw).unwrap();
        assert_eq!(extract_message_body(&parsed).unwrap(), "Hello & welcome");
    }

    #[test]
    fn html_mail_discards_style_and_json_ld_payloads() {
        let raw = b"Content-Type: text/html; charset=utf-8\r\n\r\n<style>[data-ogsc] { color: green; } [class=rating] { display:block; }</style><script type=\"application/ld+json\">{\"@context\":\"https://schema.org\"}</script><p>Readable message</p>\r\n";
        let parsed = mailparse::parse_mail(raw).unwrap();
        let body = extract_message_body(&parsed).unwrap();
        assert_eq!(body, "Readable message");
        assert!(!body.contains("data-ogsc"));
        assert!(!body.contains("class=rating"));
        assert!(!body.contains("@context"));
    }

    #[test]
    fn sent_direction_requires_a_mailbox_name_not_an_ancestor_substring() {
        assert_eq!(
            infer_mail_direction(
                "/tmp/essential/consent/Takeout/Gmail/All mail.mbox",
                "Sender <sender@example.test>"
            ),
            "received"
        );
        assert_eq!(
            infer_mail_direction(
                "/tmp/export.zip!Takeout/Gmail/Sent.mbox",
                "Sender <sender@example.test>"
            ),
            "sent"
        );
        assert_eq!(
            infer_mail_direction("/tmp/Presentations/Inbox.mbox", ""),
            "unknown"
        );
    }

    #[test]
    fn local_mail_direction_is_visible_from_folder_context() {
        assert_eq!(
            infer_mail_direction(
                "/Users/example/Library/Mail/V10/Inbox/messages/message.emlx",
                "Friend <friend@example.test>"
            ),
            "received"
        );
        assert_eq!(
            infer_mail_direction(
                "/Users/example/Library/Mail/V10/Sent Messages.mbox/message.emlx",
                "Me <me@example.test>"
            ),
            "sent"
        );
        assert_eq!(infer_mail_direction("/tmp/message.emlx", ""), "unknown");
    }
}
