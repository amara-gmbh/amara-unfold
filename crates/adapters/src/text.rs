// SPDX-License-Identifier: AGPL-3.0-or-later

use codepage_strings::Coding;

pub const SOURCE_CHARACTER_UNAVAILABLE: &str = "[source character unavailable]";

/// Decode a byte-oriented text export without assuming UTF-8. UTF-8 remains
/// the first choice; Windows-1252 is the conservative fallback used by the
/// bank, mail, music, and mapping exports commonly seen by Unfold.
pub fn decode_text(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(value) => sanitize_text(value),
        Err(_) => Coding::new(1252)
            .ok()
            .and_then(|coding| coding.decode(bytes).ok())
            .map(|value| sanitize_text(&value))
            .unwrap_or_else(|| sanitize_text(&String::from_utf8_lossy(bytes))),
    }
}

/// Keep output valid UTF-8, repair common double-decoding artifacts, and make
/// genuinely unrecoverable source bytes visible rather than guessing.
pub fn sanitize_text(value: &str) -> String {
    let repaired = repair_common_mojibake(value)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let mut output = String::with_capacity(repaired.len());
    for character in repaired.chars() {
        match character {
            '\u{fffd}' => output.push_str(SOURCE_CHARACTER_UNAVAILABLE),
            '\n' | '\r' | '\t' => output.push(character),
            character if character.is_control() => {
                output.push_str("[source control character removed]")
            }
            character => output.push(character),
        }
    }
    output
}

/// Remove non-readable HTML payloads (CSS, scripts, and JSON-LD) before
/// converting the remaining message markup to compact plain text.
pub fn html_to_text(value: &str) -> String {
    let mut readable = remove_html_comments(value);
    for element in ["head", "style", "script"] {
        readable = remove_html_element_blocks(&readable, element);
    }
    readable = remove_json_ld_blocks(&readable);

    let mut output = String::with_capacity(readable.len());
    let mut in_tag = false;
    for character in readable.chars() {
        match character {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                if !output.ends_with([' ', '\n']) {
                    output.push(' ');
                }
            }
            _ if !in_tag => output.push(character),
            _ => {}
        }
    }

    let mut text = html_escape::decode_html_entities(&output)
        .lines()
        .filter(|line| !is_residual_markup_line(line))
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ");
    for (spaced, compact) in [
        (" .", "."),
        (" ,", ","),
        (" ;", ";"),
        (" :", ":"),
        (" !", "!"),
        (" ?", "?"),
    ] {
        text = text.replace(spaced, compact);
    }
    text
}

/// Preserve ordinary plain text, but route obvious markup leakage through the
/// same cleanup used for HTML MIME parts.
pub fn clean_mail_text(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let contains_markup = [
        "<!doctype",
        "<html",
        "<head",
        "<body",
        "<div",
        "<p",
        "<br",
        "<table",
        "<span",
        "<style",
        "<script",
        "[data-ogsc]",
        "[class=rating]",
        "\"@context\"",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    if contains_markup {
        html_to_text(value)
    } else {
        value.trim().to_owned()
    }
}

fn remove_html_comments(value: &str) -> String {
    let mut output = value.to_owned();
    while let Some(start) = output.find("<!--") {
        let end = output[start + 4..]
            .find("-->")
            .map(|offset| start + 4 + offset + 3)
            .unwrap_or(output.len());
        output.replace_range(start..end, " ");
    }
    output
}

fn remove_html_element_blocks(value: &str, element: &str) -> String {
    let mut output = value.to_owned();
    let opening = format!("<{element}");
    let closing = format!("</{element}>");
    loop {
        let lower = output.to_ascii_lowercase();
        let Some(start) = lower.find(&opening) else {
            break;
        };
        let Some(open_end_offset) = lower[start..].find('>') else {
            output.truncate(start);
            break;
        };
        let open_end = start + open_end_offset + 1;
        let end = lower[open_end..]
            .find(&closing)
            .map(|offset| open_end + offset + closing.len())
            .unwrap_or(output.len());
        output.replace_range(start..end, " ");
    }
    output
}

fn remove_json_ld_blocks(value: &str) -> String {
    let mut output = value.to_owned();
    loop {
        let lower = output.to_ascii_lowercase();
        let Some(marker) = lower.find("\"@context\"") else {
            break;
        };
        let Some(start) = lower[..marker].rfind('{') else {
            break;
        };
        let mut depth = 0_i32;
        let mut end = output.len();
        for (offset, byte) in output.as_bytes()[start..].iter().enumerate() {
            match byte {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = start + offset + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        output.replace_range(start..end, " ");
    }
    output
}

fn is_residual_markup_line(line: &str) -> bool {
    let lower = line.trim().to_ascii_lowercase();
    lower.starts_with("[data-")
        || lower.starts_with("[class=")
        || lower.starts_with("@media")
        || lower.contains("\"@context\"")
}

fn repair_common_mojibake(value: &str) -> String {
    const REPLACEMENTS: &[(&str, &str)] = &[
        ("Ã¤", "ä"),
        ("Ã¶", "ö"),
        ("Ã¼", "ü"),
        ("Ã„", "Ä"),
        ("Ã–", "Ö"),
        ("Ãœ", "Ü"),
        ("ÃŸ", "ß"),
        ("â€“", "–"),
        ("â€”", "—"),
        ("â€˜", "‘"),
        ("â€™", "’"),
        ("â€œ", "“"),
        ("â€", "”"),
        ("â€¦", "…"),
        ("Â ", " "),
        ("Â", ""),
    ];
    let mut output = value.to_owned();
    for (damaged, repaired) in REPLACEMENTS {
        output = output.replace(damaged, repaired);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_windows_1252_and_repairs_double_decoding() {
        assert_eq!(decode_text(b"M\xfcnchen"), "München");
        assert_eq!(sanitize_text("GebÃ¼hr"), "Gebühr");
    }

    #[test]
    fn unavailable_source_bytes_are_visible_but_not_re_emitted() {
        let cleaned = sanitize_text("M\u{fffd}nchen\u{0007}");
        assert_eq!(
            cleaned,
            "M[source character unavailable]nchen[source control character removed]"
        );
        assert!(!cleaned.contains('\u{fffd}'));
    }

    #[test]
    fn normalizes_source_line_endings_for_portable_markdown() {
        assert_eq!(sanitize_text("one\r\ntwo\rthree\n"), "one\ntwo\nthree\n");
    }

    #[test]
    fn mail_html_removes_css_and_json_ld_before_extracting_text() {
        let html = r#"<html><head><style>[data-ogsc] { color: green; } [class=rating] { display:block; }</style><script type="application/ld+json">{"@context":"https://schema.org","@type":"EmailMessage"}</script></head><body><p>Hello <strong>there</strong>.</p></body></html>"#;
        let text = html_to_text(html);
        assert_eq!(text, "Hello there.");
        assert!(!text.contains("data-ogsc"));
        assert!(!text.contains("class=rating"));
        assert!(!text.contains("@context"));
    }

    #[test]
    fn plain_mail_text_without_markup_is_preserved() {
        assert_eq!(
            clean_mail_text("Two values: 2 < 3 and 4 > 1"),
            "Two values: 2 < 3 and 4 > 1"
        );
    }
}
