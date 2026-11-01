// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{text::SOURCE_CHARACTER_UNAVAILABLE, AdapterError, ExtractionProgress};
use chrono::{Datelike, Utc};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::{fs, path::Path};

pub const ACCEPTANCE_THRESHOLD_BASIS_POINTS: u16 = 9_950;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QualitySummary {
    pub status: String,
    pub score_basis_points: u16,
    pub acceptance_threshold_basis_points: u16,
    pub events_checked: u64,
    pub flagged_events: u64,
    pub schema_errors: u64,
    pub provenance_errors: u64,
    pub encoding_warnings: u64,
    pub source_normalization_warnings: u64,
    pub source_empty_body_records: u64,
    pub mail_body_warnings: u64,
    pub mail_direction_warnings: u64,
    pub implausible_date_warnings: u64,
    pub counterparty_warnings: u64,
    pub extraction_errors: u64,
    pub accounting_balanced: bool,
    pub skip_reasons_balanced: bool,
}

impl QualitySummary {
    pub fn status_label(&self) -> &str {
        if self.status == "pass" {
            "passed"
        } else {
            "review required"
        }
    }
}

pub fn validate_package(
    root: &Path,
    adapter_id: &str,
    source: &Path,
    progress: &ExtractionProgress,
) -> Result<QualitySummary, AdapterError> {
    let mut summary = QualitySummary {
        status: "review".into(),
        score_basis_points: 0,
        acceptance_threshold_basis_points: ACCEPTANCE_THRESHOLD_BASIS_POINTS,
        events_checked: 0,
        flagged_events: 0,
        schema_errors: 0,
        provenance_errors: 0,
        encoding_warnings: 0,
        source_normalization_warnings: 0,
        source_empty_body_records: 0,
        mail_body_warnings: 0,
        mail_direction_warnings: 0,
        implausible_date_warnings: 0,
        counterparty_warnings: 0,
        extraction_errors: progress.errors,
        accounting_balanced: progress.records_seen
            == progress.events_written + progress.events_skipped,
        skip_reasons_balanced: progress.skip_reasons.values().sum::<u64>()
            == progress.events_skipped,
    };
    let mut owner_addresses = if adapter_id == "pst-mail" {
        let mut addresses = crate::input::files_with_extensions(source, &["pst"])
            .iter()
            .flat_map(|path| email_addresses(&path.to_string_lossy()))
            .collect::<Vec<_>>();
        addresses.sort();
        addresses.dedup();
        addresses
    } else {
        email_addresses(&source.to_string_lossy())
    };
    if adapter_id == "pst-mail" {
        for entry in walkdir::WalkDir::new(root.join("events"))
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
        {
            let Ok(text) = fs::read_to_string(entry.path()) else {
                continue;
            };
            let Some((_, body)) = split_markdown(&text) else {
                continue;
            };
            if field_segment(body, "Direction:").eq_ignore_ascii_case("sent") {
                owner_addresses.extend(email_addresses(field_segment(body, "From:")));
            }
        }
        owner_addresses.sort();
        owner_addresses.dedup();
    }
    for entry in walkdir::WalkDir::new(root.join("events"))
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            let file_name = entry.file_name().to_string_lossy();
            entry.file_type().is_file()
                && !file_name.starts_with("._")
                && file_name != ".DS_Store"
                && entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "md")
        })
    {
        summary.events_checked += 1;
        let text = fs::read_to_string(entry.path()).map_err(|error| {
            AdapterError::Output(format!(
                "quality check could not read emitted Markdown: {error}"
            ))
        })?;
        let mut flagged = false;
        if text.contains('\u{fffd}') {
            summary.encoding_warnings += 1;
            flagged = true;
        }
        if text.contains(SOURCE_CHARACTER_UNAVAILABLE) {
            summary.source_normalization_warnings += 1;
            flagged = true;
        }
        let Some((frontmatter, body)) = split_markdown(&text) else {
            summary.schema_errors += 1;
            summary.flagged_events += 1;
            continue;
        };
        let value: Value = match serde_yaml::from_str(frontmatter) {
            Ok(value) => value,
            Err(_) => {
                summary.schema_errors += 1;
                summary.flagged_events += 1;
                continue;
            }
        };
        if !required_fields_present(&value) {
            summary.schema_errors += 1;
            flagged = true;
        }
        if !provenance_present(&value) {
            summary.provenance_errors += 1;
            flagged = true;
        }
        if matches!(
            adapter_id,
            "gmail-takeout" | "pst-mail" | "apple-mail-local"
        ) {
            match mail_message_body(body) {
                MailBodyQuality::Readable => {}
                MailBodyQuality::SourceEmpty => summary.source_empty_body_records += 1,
                MailBodyQuality::Warning => {
                    summary.mail_body_warnings += 1;
                    flagged = true;
                }
            }
        }
        if adapter_id == "pst-mail" && own_sender_marked_received(body, &owner_addresses) {
            summary.mail_direction_warnings += 1;
            flagged = true;
        }
        if matches!(adapter_id, "banking-xlsx" | "banking-csv") {
            if banking_date_is_implausible(&value, Utc::now().year()) {
                summary.implausible_date_warnings += 1;
                flagged = true;
            }
            if banking_counterparty_is_noise(body) || banking_identity_contains_noise(&value) {
                summary.counterparty_warnings += 1;
                flagged = true;
            }
        }
        summary.flagged_events += u64::from(flagged);
    }

    summary.score_basis_points = if summary.events_checked == 0 {
        0
    } else {
        let clean = summary.events_checked - summary.flagged_events;
        ((clean.saturating_mul(10_000) / summary.events_checked).min(10_000)) as u16
    };
    let hard_invariants_pass = summary.schema_errors == 0
        && summary.provenance_errors == 0
        && summary.encoding_warnings == 0
        && summary.mail_body_warnings == 0
        && summary.mail_direction_warnings == 0
        && summary.implausible_date_warnings == 0
        && summary.counterparty_warnings == 0
        && summary.extraction_errors == 0
        && summary.accounting_balanced
        && summary.skip_reasons_balanced;
    if hard_invariants_pass
        && summary.score_basis_points >= summary.acceptance_threshold_basis_points
    {
        summary.status = "pass".into();
    }
    Ok(summary)
}

pub fn write_reports(root: &Path, summary: &QualitySummary) -> Result<(), AdapterError> {
    let json = serde_json::to_string_pretty(summary)
        .map_err(|error| AdapterError::Output(error.to_string()))?;
    fs::write(root.join("quality-report.json"), format!("{json}\n"))
        .map_err(|error| AdapterError::Output(error.to_string()))?;
    let score = f64::from(summary.score_basis_points) / 100.0;
    let threshold = f64::from(summary.acceptance_threshold_basis_points) / 100.0;
    let markdown = format!(
        "# Adapter quality report\n\n**Status: {}** · **Score: {score:.2}%** · acceptance threshold: {threshold:.2}%\n\n| Check | Count / result |\n|---|---:|\n| Events checked | {} |\n| Flagged events | {} |\n| Schema errors | {} |\n| Provenance errors | {} |\n| Unicode replacement warnings | {} |\n| Explicit source-normalization warnings | {} |\n| Source records without a body | {} |\n| Mail body warnings | {} |\n| Mail direction warnings | {} |\n| Implausible date warnings | {} |\n| Counterparty warnings | {} |\n| Extraction errors | {} |\n| Record accounting balanced | {} |\n| Skip reasons balanced | {} |\n\nA `review` result preserves the output and makes the issue visible; it is not silently promoted to a clean export. No network service is used by this validation.\n",
        summary.status,
        summary.events_checked,
        summary.flagged_events,
        summary.schema_errors,
        summary.provenance_errors,
        summary.encoding_warnings,
        summary.source_normalization_warnings,
        summary.source_empty_body_records,
        summary.mail_body_warnings,
        summary.mail_direction_warnings,
        summary.implausible_date_warnings,
        summary.counterparty_warnings,
        summary.extraction_errors,
        summary.accounting_balanced,
        summary.skip_reasons_balanced,
    );
    fs::write(root.join("quality-report.md"), markdown)
        .map_err(|error| AdapterError::Output(error.to_string()))
}

fn split_markdown(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix("---\n")?;
    rest.split_once("\n---\n")
}

fn required_fields_present(value: &Value) -> bool {
    [
        "schema_version",
        "id",
        "date",
        "date_precision",
        "source",
        "module",
        "type",
        "title",
    ]
    .iter()
    .all(|key| value.get(*key).is_some_and(nonempty_yaml_value))
}

fn provenance_present(value: &Value) -> bool {
    let Some(provenance) = value.get("provenance") else {
        return false;
    };
    ["raw_path", "raw_hash", "ingested_at"]
        .iter()
        .all(|key| provenance.get(*key).is_some_and(nonempty_yaml_value))
}

fn nonempty_yaml_value(value: &Value) -> bool {
    !value.is_null() && value.as_str().is_none_or(|text| !text.trim().is_empty())
}

#[derive(Debug, PartialEq, Eq)]
enum MailBodyQuality {
    Readable,
    SourceEmpty,
    Warning,
}

fn mail_message_body(body: &str) -> MailBodyQuality {
    let mut segments = body
        .split("\n\n")
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments
        .first()
        .is_some_and(|segment| segment.starts_with("# "))
    {
        segments.remove(0);
    }
    let kept_segments = segments
        .into_iter()
        .filter(|segment| {
            !["From:", "To:", "Cc:", "Direction:", "Mailbox folder:"]
                .iter()
                .any(|prefix| segment.starts_with(prefix))
        })
        .collect::<Vec<_>>();
    let content = kept_segments.join("\n\n");
    if content.starts_with("[No message body was present in this PST record.]") {
        MailBodyQuality::SourceEmpty
    } else if content.is_empty()
        || content.starts_with("[No readable message body")
        || content.starts_with("[Body could not be decoded")
        || content.starts_with("[A message body was present but could not be decoded")
        || mail_body_has_markup_residue(&content)
    {
        MailBodyQuality::Warning
    } else {
        MailBodyQuality::Readable
    }
}

fn mail_body_has_markup_residue(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    [
        "<script",
        "<style",
        "[data-ogsc]",
        "[class=rating]",
        "\"@context\"",
        "\"@type\"",
        "application/ld+json",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn own_sender_marked_received(body: &str, owner_addresses: &[String]) -> bool {
    let direction = field_segment(body, "Direction:");
    if !direction.eq_ignore_ascii_case("received") {
        return false;
    }
    let sender = field_segment(body, "From:");
    email_addresses(sender).iter().any(|sender_address| {
        crate::pst_mail::address_matches_owner(sender_address, owner_addresses)
    })
}

fn banking_date_is_implausible(frontmatter: &Value, current_year: i32) -> bool {
    let Some(date) = frontmatter.get("date").and_then(Value::as_str) else {
        return true;
    };
    let Some(year) = date.get(..4).and_then(|value| value.parse::<i32>().ok()) else {
        return true;
    };
    !(1990..=current_year + 1).contains(&year)
}

fn banking_identity_contains_noise(frontmatter: &Value) -> bool {
    ["actors", "tags"].iter().any(|field| {
        frontmatter
            .get(*field)
            .and_then(Value::as_sequence)
            .is_some_and(|values| {
                values.iter().any(|value| {
                    value.as_str().is_some_and(|value| {
                        matches!(
                            value.trim().to_ascii_lowercase().as_str(),
                            "ga" | "merchant-ga"
                        )
                    })
                })
            })
    })
}

fn banking_counterparty_is_noise(body: &str) -> bool {
    let counterparty = field_segment(body, "Counterparty:").trim();
    if counterparty.is_empty() {
        return false;
    }
    let upper = counterparty.to_uppercase();
    if upper == "GA"
        || upper == "MERCHANT-GA"
        || upper.starts_with("GA NR")
        || upper.contains(" BLZ")
        || upper.contains("SAGT DANKE")
        || counterparty
            .chars()
            .all(|character| character.is_ascii_digit() || character.is_ascii_punctuation())
    {
        return true;
    }
    let characters = counterparty.chars().collect::<Vec<_>>();
    if characters.len() >= 8 && characters.len() % 2 == 0 {
        let middle = characters.len() / 2;
        return characters[..middle]
            .iter()
            .collect::<String>()
            .eq_ignore_ascii_case(&characters[middle..].iter().collect::<String>());
    }
    false
}

fn field_segment<'a>(body: &'a str, prefix: &str) -> &'a str {
    body.split("\n\n")
        .map(str::trim)
        .find_map(|segment| segment.strip_prefix(prefix).map(str::trim))
        .unwrap_or("")
}

fn email_addresses(value: &str) -> Vec<String> {
    let mut addresses = value
        .split(|character: char| {
            character.is_whitespace()
                || matches!(character, '<' | '>' | ',' | ';' | '(' | ')' | '"' | '\'')
                || character == std::path::MAIN_SEPARATOR
        })
        .map(|part| {
            part.trim_matches(|character: char| {
                !character.is_alphanumeric() && !matches!(character, '@' | '.' | '_' | '-' | '+')
            })
            .to_lowercase()
        })
        .filter(|part| {
            part.split_once('@').is_some_and(|(local, domain)| {
                !local.is_empty()
                    && domain.contains('.')
                    && !domain.ends_with('.')
                    && part.matches('@').count() == 1
            })
        })
        .collect::<Vec<_>>();
    addresses.sort();
    addresses.dedup();
    addresses
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mail_body_check_looks_after_structured_headers() {
        assert_eq!(
            mail_message_body("# Mail\n\nFrom: a@example.com\n\nTo: b@example.com\n\nHello"),
            MailBodyQuality::Readable
        );
        assert_eq!(
            mail_message_body("# Mail\n\nFrom: a@example.com\n\nTo: b@example.com\n\n"),
            MailBodyQuality::Warning
        );
        assert_eq!(
            mail_message_body(
                "# Mail\n\nFrom: a@example.com\n\nTo: b@example.com\n\n#outlook a { padding: 0; }"
            ),
            MailBodyQuality::Readable
        );
        assert_eq!(
            mail_message_body(
                "# Mail\n\nFrom: a@example.com\n\nTo: b@example.com\n\n[data-ogsc] { color: green; }"
            ),
            MailBodyQuality::Warning
        );
        assert_eq!(
            mail_message_body(
                "# Mail\n\nFrom: a@example.com\n\nTo: b@example.com\n\n{\"@context\":\"https://schema.org\"}"
            ),
            MailBodyQuality::Warning
        );
        assert_eq!(
            mail_message_body("# Mail\n\n[No message body was present in this PST record.]"),
            MailBodyQuality::SourceEmpty
        );
    }

    #[test]
    fn banking_noise_rules_match_machine_values_not_empty_values() {
        assert!(banking_counterparty_is_noise(
            "Counterparty: GA NR123 BLZ456"
        ));
        assert!(banking_counterparty_is_noise("Counterparty: Ga"));
        assert!(banking_counterparty_is_noise("Counterparty: 99999999"));
        assert!(!banking_counterparty_is_noise("Counterparty: "));
        assert!(!banking_counterparty_is_noise("Counterparty: Corner Cafe"));
    }

    #[test]
    fn banking_quality_rejects_implausible_years_and_ga_identities() {
        let malformed: Value = serde_yaml::from_str(
            "date: 0024-03-04\nactors:\n  - merchant-ga\ntags:\n  - banking\n  - ga\n",
        )
        .unwrap();
        assert!(banking_date_is_implausible(&malformed, 2026));
        assert!(banking_identity_contains_noise(&malformed));

        let valid: Value = serde_yaml::from_str(
            "date: 2024-03-04\nactors:\n  - merchant-corner-cafe\ntags:\n  - banking\n",
        )
        .unwrap();
        assert!(!banking_date_is_implausible(&valid, 2026));
        assert!(!banking_identity_contains_noise(&valid));
    }
}
