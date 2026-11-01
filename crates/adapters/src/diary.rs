// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    input,
    model::{EventInput, OpenEvent},
    record_event, AdapterError, ExtractionProgress, ProgressCallback,
};
use roxmltree::Document;
use std::{
    fs::File,
    io::Read,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};
use zip::ZipArchive;

#[derive(Debug)]
struct Paragraph {
    text: String,
    heading_level: Option<u8>,
    index: usize,
}

#[derive(Debug)]
struct DiaryEntry {
    title: String,
    body: Vec<String>,
    order: usize,
    paragraph_start: usize,
    paragraph_end: usize,
    year: Option<String>,
    month: Option<String>,
    section: Option<String>,
    parent: Option<String>,
}

pub fn extract(
    source: &Path,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    for path in input::files_with_extensions(source, &["docx"]) {
        let paragraphs = read_paragraphs(&path, 256 * 1024 * 1024)?;
        let entries = parse_entries(&paragraphs)?;
        progress.total_hint = Some(progress.total_hint.unwrap_or(0) + entries.len() as u64);
        callback(progress.clone());
        for entry in entries {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let raw = format!(
                "{}\n{}\n{}",
                entry.title,
                entry.body.join("\n\n"),
                entry.order
            );
            let date = entry_date(&entry);
            let context = [
                entry.year.as_deref(),
                entry.month.as_deref(),
                entry.section.as_deref(),
                entry.parent.as_deref(),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" / ");
            let body = format!(
                "Diary entry\n\nDocument context: {}\n\nParagraphs: {}-{}\n\n{}",
                if context.is_empty() {
                    "undated"
                } else {
                    &context
                },
                entry.paragraph_start,
                entry.paragraph_end,
                entry.body.join("\n\n")
            );
            let raw_path = format!(
                "{}#paragraphs-{}-{}",
                path.display(),
                entry.paragraph_start,
                entry.paragraph_end
            );
            let event = OpenEvent::new(EventInput {
                date: &date,
                time: None,
                end_date: None,
                source: "diary",
                module: "diary",
                event_type: "diary_entry",
                title: &entry.title,
                raw_path: &raw_path,
                raw_bytes: raw.as_bytes(),
                body,
                actors: Vec::new(),
                places: Vec::new(),
                tags: vec!["diary".into(), entry_kind_tag(&entry).into()],
                confidence: "high",
            });
            record_event(package, &event, progress, callback)?;
        }
    }
    Ok(())
}

fn read_paragraphs(path: &Path, max_xml_bytes: u64) -> Result<Vec<Paragraph>, AdapterError> {
    input::validate_file_size(path, 1024 * 1024 * 1024, "DOCX file")?;
    input::validate_zip_container(path, 20_000, 256 * 1024 * 1024, 1024 * 1024 * 1024)?;
    let file = File::open(path).map_err(|error| AdapterError::Input(error.to_string()))?;
    let mut archive =
        ZipArchive::new(file).map_err(|error| AdapterError::Parse(error.to_string()))?;
    let mut entry = archive
        .by_name("word/document.xml")
        .map_err(|error| AdapterError::Parse(error.to_string()))?;
    if entry.size() > max_xml_bytes {
        return Err(AdapterError::Input(
            "DOCX document.xml exceeds the 256 MiB limit".into(),
        ));
    }
    let mut xml = String::new();
    entry
        .read_to_string(&mut xml)
        .map_err(|error| AdapterError::Parse(error.to_string()))?;
    let document = Document::parse(&xml).map_err(|error| AdapterError::Parse(error.to_string()))?;
    let mut paragraphs = Vec::new();
    for node in document
        .descendants()
        .filter(|node| node.tag_name().name() == "p")
    {
        let text = node
            .descendants()
            .filter(|child| child.tag_name().name() == "t")
            .filter_map(|child| child.text())
            .collect::<String>()
            .trim()
            .to_owned();
        if text.is_empty() {
            continue;
        }
        let style = node
            .descendants()
            .find(|child| child.tag_name().name() == "pStyle")
            .and_then(|child| {
                child
                    .attributes()
                    .find(|attribute| attribute.name() == "val")
                    .map(|attribute| attribute.value())
            });
        paragraphs.push(Paragraph {
            text,
            heading_level: heading_level(style.unwrap_or_default()),
            index: paragraphs.len(),
        });
    }
    Ok(paragraphs)
}

fn heading_level(style: &str) -> Option<u8> {
    let normalized = style.to_ascii_lowercase();
    for (needle, level) in [
        ("berschrift1", 1),
        ("heading1", 1),
        ("berschrift3", 3),
        ("heading3", 3),
        ("berschrift4", 4),
        ("heading4", 4),
    ] {
        if normalized == needle {
            return Some(level);
        }
    }
    None
}

fn parse_entries(paragraphs: &[Paragraph]) -> Result<Vec<DiaryEntry>, AdapterError> {
    let start = paragraphs
        .iter()
        .position(|paragraph| paragraph.heading_level == Some(1) && is_year(&paragraph.text))
        .ok_or_else(|| AdapterError::Parse("diary has no chronological year heading".into()))?;
    let mut year = None;
    let mut month = None;
    let mut section = None;
    let mut parent = None;
    let mut current: Option<DiaryEntry> = None;
    let mut entries = Vec::new();

    for paragraph in &paragraphs[start..] {
        match paragraph.heading_level {
            Some(1) => {
                close_entry(
                    &mut current,
                    paragraph.index.saturating_sub(1),
                    &mut entries,
                );
                if is_year(&paragraph.text) {
                    year = Some(paragraph.text.clone());
                    month = None;
                    section = None;
                    parent = None;
                } else {
                    section = Some(paragraph.text.clone());
                    parent = None;
                }
            }
            Some(3) => {
                close_entry(
                    &mut current,
                    paragraph.index.saturating_sub(1),
                    &mut entries,
                );
                if section.is_some() {
                    parent = Some(paragraph.text.clone());
                    current = Some(new_entry(
                        paragraph,
                        entries.len() + 1,
                        None,
                        None,
                        section.clone(),
                        None,
                    ));
                } else {
                    month = Some(paragraph.text.clone());
                }
            }
            Some(4) => {
                close_entry(
                    &mut current,
                    paragraph.index.saturating_sub(1),
                    &mut entries,
                );
                current = Some(new_entry(
                    paragraph,
                    entries.len() + 1,
                    section.is_none().then(|| year.clone()).flatten(),
                    section.is_none().then(|| month.clone()).flatten(),
                    section.clone(),
                    parent.clone(),
                ));
            }
            _ => {
                if section.is_some() && current.is_none() {
                    current = Some(DiaryEntry {
                        title: section.clone().unwrap_or_else(|| "Diary entry".into()),
                        body: Vec::new(),
                        order: entries.len() + 1,
                        paragraph_start: paragraph.index,
                        paragraph_end: paragraph.index,
                        year: None,
                        month: None,
                        section: section.clone(),
                        parent: parent.clone(),
                    });
                }
                if let Some(entry) = &mut current {
                    entry.body.push(paragraph.text.clone());
                }
            }
        }
    }
    if let Some(last) = paragraphs.last() {
        close_entry(&mut current, last.index, &mut entries);
    }
    Ok(entries)
}

fn new_entry(
    paragraph: &Paragraph,
    order: usize,
    year: Option<String>,
    month: Option<String>,
    section: Option<String>,
    parent: Option<String>,
) -> DiaryEntry {
    DiaryEntry {
        title: paragraph.text.clone(),
        body: Vec::new(),
        order,
        paragraph_start: paragraph.index,
        paragraph_end: paragraph.index,
        year,
        month,
        section,
        parent,
    }
}

fn close_entry(
    current: &mut Option<DiaryEntry>,
    paragraph_end: usize,
    entries: &mut Vec<DiaryEntry>,
) {
    if let Some(mut entry) = current.take() {
        entry.paragraph_end = paragraph_end;
        entries.push(entry);
    }
}

fn entry_date(entry: &DiaryEntry) -> String {
    let Some(year) = entry.year.as_deref() else {
        return "0000-00-00".into();
    };
    match entry.month.as_deref().and_then(month_number) {
        Some(month) => format!("{year}-{month}"),
        None => year.to_owned(),
    }
}

fn entry_kind_tag(entry: &DiaryEntry) -> &'static str {
    if entry.year.is_some() {
        "chronological"
    } else if entry.section.is_some() {
        "reference"
    } else {
        "undated"
    }
}

fn month_number(value: &str) -> Option<String> {
    let normalized = value.trim().trim_end_matches('.').to_lowercase();
    let named = match normalized.as_str() {
        "januar" | "january" | "jan" => Some(1),
        "februar" | "february" | "feb" => Some(2),
        "märz" | "maerz" | "march" | "mar" => Some(3),
        "april" | "apr" => Some(4),
        "mai" | "may" => Some(5),
        "juni" | "june" | "jun" => Some(6),
        "juli" | "july" | "jul" => Some(7),
        "august" | "aug" => Some(8),
        "september" | "sep" | "sept" => Some(9),
        "oktober" | "october" | "okt" | "oct" => Some(10),
        "november" | "nov" => Some(11),
        "dezember" | "december" | "dez" | "dec" => Some(12),
        _ => None,
    };
    let number = named.or_else(|| normalized.parse::<u8>().ok())?;
    (1..=12).contains(&number).then(|| format!("{number:02}"))
}

fn is_year(value: &str) -> bool {
    value.len() == 4 && value.chars().all(|character| character.is_ascii_digit())
}

pub(crate) fn recognizes(path: &Path) -> bool {
    read_paragraphs(path, 8 * 1024 * 1024)
        .and_then(|paragraphs| parse_entries(&paragraphs))
        .is_ok_and(|entries| !entries.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_heading_dates_preserve_precision() {
        let entry = DiaryEntry {
            title: "First computer".into(),
            body: Vec::new(),
            order: 1,
            paragraph_start: 1,
            paragraph_end: 1,
            year: Some("1994".into()),
            month: Some("März".into()),
            section: None,
            parent: None,
        };
        assert_eq!(entry_date(&entry), "1994-03");
        assert_eq!(crate::model::slug(&entry.title), "first-computer");
    }

    #[test]
    fn heading_rules_generalize_across_month_languages_and_topical_sections() {
        let paragraphs = vec![
            Paragraph {
                text: "2031".into(),
                heading_level: Some(1),
                index: 0,
            },
            Paragraph {
                text: "September".into(),
                heading_level: Some(3),
                index: 1,
            },
            Paragraph {
                text: "Project launch".into(),
                heading_level: Some(4),
                index: 2,
            },
            Paragraph {
                text: "A dated memory".into(),
                heading_level: None,
                index: 3,
            },
            Paragraph {
                text: "Reading notes".into(),
                heading_level: Some(1),
                index: 4,
            },
            Paragraph {
                text: "A future document topic".into(),
                heading_level: Some(3),
                index: 5,
            },
            Paragraph {
                text: "A timeless quotation".into(),
                heading_level: None,
                index: 6,
            },
        ];

        let entries = parse_entries(&paragraphs).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entry_date(&entries[0]), "2031-09");
        assert_eq!(entry_kind_tag(&entries[0]), "chronological");
        assert_eq!(entry_date(&entries[1]), "0000-00-00");
        assert_eq!(entry_kind_tag(&entries[1]), "reference");
        assert_eq!(month_number("03"), Some("03".into()));
        assert_eq!(month_number("Okt."), Some("10".into()));
    }
}
